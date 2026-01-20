use crate::utils::make_short_random_id;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use parking_lot::{Mutex, MutexGuard, RwLock};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tracing::{debug, warn};

const FORK_DIR: &str = "/tmp/mcp-forks";
const CHECKPOINT_DIR: &str = "/tmp/mcp-checkpoints";
#[allow(dead_code)]
const STAGED_SNAPSHOT_DIR: &str = "/tmp/mcp-staged";
const DEFAULT_TTL_SECS: u64 = 0;
const DEFAULT_MAX_FORKS: usize = 10;
const CLEANUP_TASK_CHECK_SECS: u64 = 60;
const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB
const DEFAULT_MAX_CHECKPOINTS_PER_FORK: usize = 10;
const DEFAULT_MAX_STAGED_CHANGES_PER_FORK: usize = 20;
const DEFAULT_MAX_CHECKPOINT_TOTAL_BYTES: u64 = 500 * 1024 * 1024;

/// RAII guard for temporary files - ensures cleanup on drop
#[derive(Debug)]
pub struct TempFileGuard {
    path: PathBuf,
    cleanup_on_drop: bool,
}

impl TempFileGuard {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            cleanup_on_drop: true,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Disarm the guard - file will not be deleted on drop
    pub fn disarm(mut self) -> PathBuf {
        self.cleanup_on_drop = false;
        self.path.clone()
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.cleanup_on_drop {
            if let Err(e) = fs::remove_file(&self.path) {
                debug!(path = ?self.path, error = %e, "failed to cleanup temp file");
            } else {
                debug!(path = ?self.path, "cleaned up temp file");
            }
        }
    }
}

/// RAII guard for fork creation - ensures rollback on error
#[derive(Debug)]
pub struct ForkCreationGuard<'a> {
    fork_id: String,
    work_path: PathBuf,
    registry: &'a ForkRegistry,
    committed: bool,
}

impl<'a> ForkCreationGuard<'a> {
    fn new(fork_id: String, work_path: PathBuf, registry: &'a ForkRegistry) -> Self {
        Self {
            fork_id,
            work_path,
            registry,
            committed: false,
        }
    }

    /// Commit the fork creation - prevents rollback on drop
    pub fn commit(mut self) {
        self.committed = true;
    }
}

impl<'a> Drop for ForkCreationGuard<'a> {
    fn drop(&mut self) {
        if !self.committed {
            warn!(fork_id = %self.fork_id, "rolling back failed fork creation");
            // Remove from registry if present
            let _ = self.registry.forks.write().remove(&self.fork_id);
            // Clean up work file
            let _ = fs::remove_file(&self.work_path);
        }
    }
}

/// RAII guard for checkpoint operations - ensures cleanup on error
#[derive(Debug)]
pub struct CheckpointGuard {
    snapshot_path: PathBuf,
    committed: bool,
}

impl CheckpointGuard {
    pub fn new(snapshot_path: PathBuf) -> Self {
        Self {
            snapshot_path,
            committed: false,
        }
    }

    fn path(&self) -> &Path {
        &self.snapshot_path
    }

    /// Commit the checkpoint - prevents cleanup on drop
    fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for CheckpointGuard {
    fn drop(&mut self) {
        if !self.committed {
            debug!(path = ?self.snapshot_path, "rolling back failed checkpoint");
            let _ = fs::remove_file(&self.snapshot_path);
        }
    }
}

#[derive(Debug, Clone)]
pub struct EditOp {
    pub timestamp: DateTime<Utc>,
    pub sheet: String,
    pub address: String,
    pub value: String,
    pub is_formula: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagedOp {
    pub kind: String,
    pub payload: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct ChangeSummary {
    pub op_kinds: Vec<String>,
    pub affected_sheets: Vec<String>,
    pub affected_bounds: Vec<String>,
    pub counts: BTreeMap<String, u64>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StagedChange {
    pub change_id: String,
    pub created_at: DateTime<Utc>,
    pub label: Option<String>,
    pub ops: Vec<StagedOp>,
    pub summary: ChangeSummary,
    pub fork_path_snapshot: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub checkpoint_id: String,
    pub created_at: DateTime<Utc>,
    pub label: Option<String>,
    pub snapshot_path: PathBuf,
}

/// Fork context with version tracking for optimistic locking
#[derive(Debug)]
pub struct ForkContext {
    pub fork_id: String,
    pub base_path: PathBuf,
    pub work_path: PathBuf,
    pub created_at: Instant,
    pub last_accessed: Instant,
    pub edits: Vec<EditOp>,
    pub staged_changes: Vec<StagedChange>,
    pub checkpoints: Vec<Checkpoint>,
    base_hash: String,
    base_modified: std::time::SystemTime,
    /// Version counter for optimistic locking - incremented on each modification
    version: AtomicU64,
}

impl ForkContext {
    fn new(fork_id: String, base_path: PathBuf, work_path: PathBuf) -> Result<Self> {
        let metadata = fs::metadata(&base_path)?;
        let base_modified = metadata.modified()?;
        let base_hash = hash_file(&base_path)?;

        Ok(Self {
            fork_id,
            base_path,
            work_path,
            created_at: Instant::now(),
            last_accessed: Instant::now(),
            edits: Vec::new(),
            staged_changes: Vec::new(),
            checkpoints: Vec::new(),
            base_hash,
            base_modified,
            version: AtomicU64::new(0),
        })
    }

    pub fn is_expired(&self, ttl: Duration) -> bool {
        if ttl.is_zero() {
            return false;
        }
        self.last_accessed.elapsed() > ttl
    }

    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }

    /// Get current version for optimistic locking
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::SeqCst)
    }

    /// Increment version after modification
    pub fn increment_version(&self) -> u64 {
        self.version.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Validate version for optimistic locking
    pub fn validate_version(&self, expected_version: u64) -> Result<()> {
        let current = self.version();
        if current != expected_version {
            return Err(anyhow!(
                "version mismatch: expected {}, got {} (concurrent modification detected)",
                expected_version,
                current
            ));
        }
        Ok(())
    }

    pub fn validate_base_unchanged(&self) -> Result<()> {
        let metadata = fs::metadata(&self.base_path)?;
        let current_modified = metadata.modified()?;

        if current_modified != self.base_modified {
            return Err(anyhow!("base file modified since fork creation"));
        }

        let current_hash = hash_file(&self.base_path)?;
        if current_hash != self.base_hash {
            return Err(anyhow!("base file content changed since fork creation"));
        }

        Ok(())
    }

    fn checkpoint_dir(&self) -> PathBuf {
        PathBuf::from(CHECKPOINT_DIR).join(&self.fork_id)
    }

    fn cleanup_files(&self) {
        let _ = fs::remove_file(&self.work_path);
        for staged in &self.staged_changes {
            remove_staged_snapshot(staged);
        }
        let checkpoint_dir = self.checkpoint_dir();
        if checkpoint_dir.starts_with(CHECKPOINT_DIR) {
            let _ = fs::remove_dir_all(&checkpoint_dir);
        }
    }
}

fn hash_file(path: &Path) -> Result<String> {
    let contents = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&contents);
    Ok(format!("{:x}", hasher.finalize()))
}

#[derive(Debug, Clone)]
pub struct ForkConfig {
    pub ttl: Duration,
    pub max_forks: usize,
    pub fork_dir: PathBuf,
}

impl Default for ForkConfig {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(DEFAULT_TTL_SECS),
            max_forks: DEFAULT_MAX_FORKS,
            fork_dir: PathBuf::from(FORK_DIR),
        }
    }
}

/// Fork registry with enhanced concurrency protection
pub struct ForkRegistry {
    /// RwLock for better read concurrency on fork access
    forks: RwLock<HashMap<String, ForkContext>>,
    /// Per-fork locks for recalc operations to prevent concurrent recalc on same fork
    recalc_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    config: ForkConfig,
}

impl ForkRegistry {
    pub fn new(config: ForkConfig) -> Result<Self> {
        fs::create_dir_all(&config.fork_dir)?;
        fs::create_dir_all(CHECKPOINT_DIR)?;
        Ok(Self {
            forks: RwLock::new(HashMap::new()),
            recalc_locks: Mutex::new(HashMap::new()),
            config,
        })
    }

    pub fn start_cleanup_task(self: Arc<Self>) {
        if self.config.ttl.is_zero() {
            return;
        }
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(CLEANUP_TASK_CHECK_SECS));
            loop {
                interval.tick().await;
                self.evict_expired();
            }
        });
    }

    /// Acquire a per-fork recalc lock to prevent concurrent recalc operations
    pub fn acquire_recalc_lock(&self, fork_id: &str) -> Arc<Mutex<()>> {
        let mut locks = self.recalc_locks.lock();
        locks
            .entry(fork_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Release a per-fork recalc lock (cleanup)
    pub fn release_recalc_lock(&self, fork_id: &str) {
        let mut locks = self.recalc_locks.lock();
        // Only remove if no one else is holding it
        if let Some(lock) = locks.get(fork_id) {
            if Arc::strong_count(lock) == 1 {
                locks.remove(fork_id);
            }
        }
    }

    pub fn create_fork(&self, base_path: &Path, workspace_root: &Path) -> Result<String> {
        self.evict_expired();

        // Check capacity before expensive operations
        {
            let forks = self.forks.read();
            if forks.len() >= self.config.max_forks {
                return Err(anyhow!(
                    "max forks ({}) reached, discard existing forks first",
                    self.config.max_forks
                ));
            }
        }

        // Validate input
        let ext = base_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());

        if ext.as_deref() != Some("xlsx") {
            return Err(anyhow!(
                "only .xlsx files supported for fork/recalc (got {:?})",
                ext
            ));
        }

        if !base_path.starts_with(workspace_root) {
            return Err(anyhow!("base path must be within workspace root"));
        }

        if !base_path.exists() {
            return Err(anyhow!("base file does not exist: {:?}", base_path));
        }

        let metadata = fs::metadata(base_path)?;
        if metadata.len() > MAX_FILE_SIZE {
            return Err(anyhow!(
                "base file too large: {} bytes (max {} MB)",
                metadata.len(),
                MAX_FILE_SIZE / 1024 / 1024
            ));
        }

        // Allocate unique fork ID
        let fork_id = {
            let mut attempts: u32 = 0;
            loop {
                let candidate = make_short_random_id("fork", 12);
                let work_path = self.config.fork_dir.join(format!("{}.xlsx", candidate));
                let exists_in_registry = self.forks.read().contains_key(&candidate);
                if !exists_in_registry && !work_path.exists() {
                    break candidate;
                }
                attempts += 1;
                if attempts > 20 {
                    return Err(anyhow!("failed to allocate unique fork id"));
                }
            }
        };
        let work_path = self.config.fork_dir.join(format!("{}.xlsx", fork_id));

        // Create RAII guard for rollback on error
        let guard = ForkCreationGuard::new(fork_id.clone(), work_path.clone(), self);

        // Copy file
        fs::copy(base_path, &work_path)?;

        // Create context
        let context = ForkContext::new(fork_id.clone(), base_path.to_path_buf(), work_path)?;

        // Insert with write lock
        self.forks.write().insert(fork_id.clone(), context);

        // Commit the fork creation
        guard.commit();

        // Update metrics
        self.update_fork_metrics();

        debug!(fork_id = %fork_id, "created fork");
        Ok(fork_id)
    }

    pub fn get_fork(&self, fork_id: &str) -> Result<Arc<ForkContext>> {
        self.evict_expired();

        let mut forks = self.forks.write();
        let ctx = forks
            .get_mut(fork_id)
            .ok_or_else(|| anyhow!("fork not found: {}", fork_id))?;
        ctx.touch();
        Ok(Arc::new(ctx.clone()))
    }

    pub fn get_fork_path(&self, fork_id: &str) -> Option<PathBuf> {
        let mut forks = self.forks.write();
        if let Some(ctx) = forks.get_mut(fork_id) {
            ctx.touch();
            return Some(ctx.work_path.clone());
        }
        None
    }

    /// Execute a function with mutable access to a fork context
    /// Automatically handles version incrementing for modifications
    pub fn with_fork_mut<F, R>(&self, fork_id: &str, f: F) -> Result<R>
    where
        F: FnOnce(&mut ForkContext) -> Result<R>,
    {
        let mut forks = self.forks.write();
        let ctx = forks
            .get_mut(fork_id)
            .ok_or_else(|| anyhow!("fork not found: {}", fork_id))?;
        ctx.touch();
        let result = f(ctx)?;
        ctx.increment_version();
        Ok(result)
    }

    /// Execute a function with mutable access and version checking for optimistic locking
    pub fn with_fork_mut_versioned<F, R>(
        &self,
        fork_id: &str,
        expected_version: u64,
        f: F,
    ) -> Result<R>
    where
        F: FnOnce(&mut ForkContext) -> Result<R>,
    {
        let mut forks = self.forks.write();
        let ctx = forks
            .get_mut(fork_id)
            .ok_or_else(|| anyhow!("fork not found: {}", fork_id))?;
        ctx.validate_version(expected_version)?;
        ctx.touch();
        let result = f(ctx)?;
        ctx.increment_version();
        Ok(result)
    }

    pub fn discard_fork(&self, fork_id: &str) -> Result<()> {
        let mut forks = self.forks.write();
        if let Some(ctx) = forks.remove(fork_id) {
            ctx.cleanup_files();
            debug!(fork_id = %fork_id, "discarded fork");
        }
        // Clean up recalc lock
        self.recalc_locks.lock().remove(fork_id);
        // Update metrics
        drop(forks); // Release write lock before updating metrics
        self.update_fork_metrics();
        Ok(())
    }

    pub fn save_fork(
        &self,
        fork_id: &str,
        target_path: &Path,
        workspace_root: &Path,
        drop_fork: bool,
    ) -> Result<()> {
        if !target_path.starts_with(workspace_root) {
            return Err(anyhow!("target path must be within workspace root"));
        }

        let ext = target_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());

        if ext.as_deref() != Some("xlsx") {
            return Err(anyhow!("target must be .xlsx"));
        }

        // Create backup of target if it exists (for rollback on error)
        let backup_guard = if target_path.exists() {
            let backup = target_path.with_extension("backup.xlsx");
            if let Ok(_) = fs::copy(target_path, &backup) {
                Some(TempFileGuard::new(backup))
            } else {
                None
            }
        } else {
            None
        };

        let mut forks = self.forks.write();
        let ctx = forks
            .get(fork_id)
            .ok_or_else(|| anyhow!("fork not found: {}", fork_id))?;

        // Validate base hasn't changed
        ctx.validate_base_unchanged()?;

        // Validate work file exists
        if !ctx.work_path.exists() {
            return Err(anyhow!(
                "fork work file does not exist: {:?}",
                ctx.work_path
            ));
        }

        // Attempt to save
        let save_result = fs::copy(&ctx.work_path, target_path);

        if let Err(e) = save_result {
            // Rollback: restore backup if it exists
            if let Some(backup) = backup_guard {
                warn!(fork_id = %fork_id, target = ?target_path, "save failed, restoring backup");
                let _ = fs::copy(backup.path(), target_path);
            }
            return Err(anyhow!("failed to save fork: {}", e));
        }

        // Success - disarm backup guard
        if let Some(backup) = backup_guard {
            backup.disarm();
        }

        if drop_fork {
            if let Some(ctx) = forks.remove(fork_id) {
                ctx.cleanup_files();
                debug!(fork_id = %fork_id, "saved and discarded fork");
            }
            // Clean up recalc lock
            self.recalc_locks.lock().remove(fork_id);
            // Update metrics
            drop(forks); // Release write lock
            self.update_fork_metrics();
        } else {
            debug!(fork_id = %fork_id, "saved fork");
        }

        Ok(())
    }

    pub fn ttl(&self) -> Duration {
        self.config.ttl
    }

    pub fn list_forks(&self) -> Vec<ForkInfo> {
        self.evict_expired();

        let forks = self.forks.read();
        forks
            .values()
            .map(|ctx| ForkInfo {
                fork_id: ctx.fork_id.clone(),
                base_path: ctx.base_path.display().to_string(),
                created_at: ctx.created_at,
                edit_count: ctx.edits.len(),
                version: ctx.version(),
            })
            .collect()
    }

    pub fn create_checkpoint(&self, fork_id: &str, label: Option<String>) -> Result<Checkpoint> {
        self.evict_expired();

        let work_path = {
            let forks = self.forks.read();
            let ctx = forks
                .get(fork_id)
                .ok_or_else(|| anyhow!("fork not found: {}", fork_id))?;
            ctx.work_path.clone()
        };

        let checkpoint_id = make_short_random_id("cp", 12);
        let dir = PathBuf::from(CHECKPOINT_DIR).join(fork_id);
        fs::create_dir_all(&dir)?;
        let snapshot_path = dir.join(format!("{}.xlsx", checkpoint_id));

        // Create guard for rollback on error
        let guard = CheckpointGuard::new(snapshot_path.clone());

        fs::copy(&work_path, guard.path())?;

        let checkpoint = Checkpoint {
            checkpoint_id: checkpoint_id.clone(),
            created_at: Utc::now(),
            label,
            snapshot_path,
        };

        self.with_fork_mut(fork_id, |ctx| {
            ctx.checkpoints.push(checkpoint.clone());
            enforce_checkpoint_limits(ctx)?;
            Ok(())
        })?;

        guard.commit();
        debug!(fork_id = %fork_id, checkpoint_id = %checkpoint_id, "created checkpoint");

        Ok(checkpoint)
    }

    pub fn list_checkpoints(&self, fork_id: &str) -> Result<Vec<Checkpoint>> {
        let ctx = self.get_fork(fork_id)?;
        Ok(ctx.checkpoints.clone())
    }

    pub fn delete_checkpoint(&self, fork_id: &str, checkpoint_id: &str) -> Result<()> {
        self.with_fork_mut(fork_id, |ctx| {
            let index = ctx
                .checkpoints
                .iter()
                .position(|c| c.checkpoint_id == checkpoint_id)
                .ok_or_else(|| anyhow!("checkpoint not found: {}", checkpoint_id))?;
            let removed = ctx.checkpoints.remove(index);
            let _ = fs::remove_file(&removed.snapshot_path);
            debug!(fork_id = %fork_id, checkpoint_id = %checkpoint_id, "deleted checkpoint");
            Ok(())
        })
    }

    pub fn restore_checkpoint(&self, fork_id: &str, checkpoint_id: &str) -> Result<Checkpoint> {
        self.evict_expired();

        let (work_path, checkpoint) = {
            let forks = self.forks.read();
            let ctx = forks
                .get(fork_id)
                .ok_or_else(|| anyhow!("fork not found: {}", fork_id))?;
            let checkpoint = ctx
                .checkpoints
                .iter()
                .find(|c| c.checkpoint_id == checkpoint_id)
                .cloned()
                .ok_or_else(|| anyhow!("checkpoint not found: {}", checkpoint_id))?;
            (ctx.work_path.clone(), checkpoint)
        };

        // Validate checkpoint before restoration
        self.validate_checkpoint(&checkpoint)?;

        // Create backup of current work file in case restoration fails
        let backup_path = work_path.with_extension("backup.xlsx");
        let backup_guard = TempFileGuard::new(backup_path.clone());

        fs::copy(&work_path, &backup_path).map_err(|e| {
            anyhow!(
                "failed to create backup before checkpoint restoration: {}",
                e
            )
        })?;

        // Attempt restoration with rollback on error
        let restore_result = fs::copy(&checkpoint.snapshot_path, &work_path);

        if let Err(e) = restore_result {
            // Rollback: restore from backup
            warn!(fork_id = %fork_id, checkpoint_id = %checkpoint_id, "checkpoint restoration failed, rolling back");
            let _ = fs::copy(&backup_path, &work_path);
            return Err(anyhow!("failed to restore checkpoint: {}", e));
        }

        // Update fork context metadata
        let metadata_result = self.with_fork_mut(fork_id, |ctx| {
            let cutoff = checkpoint.created_at;
            ctx.edits.retain(|e| e.timestamp <= cutoff);
            let mut i = 0;
            while i < ctx.staged_changes.len() {
                if ctx.staged_changes[i].created_at > cutoff {
                    let removed = ctx.staged_changes.remove(i);
                    remove_staged_snapshot(&removed);
                } else {
                    i += 1;
                }
            }
            Ok(())
        });

        if let Err(e) = metadata_result {
            // Rollback: restore from backup
            warn!(fork_id = %fork_id, checkpoint_id = %checkpoint_id, "metadata update failed, rolling back checkpoint");
            let _ = fs::copy(&backup_path, &work_path);
            return Err(e);
        }

        // Success - disarm the backup guard
        backup_guard.disarm();

        debug!(fork_id = %fork_id, checkpoint_id = %checkpoint_id, "checkpoint restored successfully");
        Ok(checkpoint)
    }

    /// Validate that a checkpoint file exists and is readable
    fn validate_checkpoint(&self, checkpoint: &Checkpoint) -> Result<()> {
        if !checkpoint.snapshot_path.exists() {
            return Err(anyhow!(
                "checkpoint file does not exist: {:?}",
                checkpoint.snapshot_path
            ));
        }

        let metadata = fs::metadata(&checkpoint.snapshot_path)
            .map_err(|e| anyhow!("failed to read checkpoint metadata: {}", e))?;

        if metadata.len() == 0 {
            return Err(anyhow!("checkpoint file is empty"));
        }

        if metadata.len() > MAX_FILE_SIZE {
            return Err(anyhow!("checkpoint file exceeds maximum size"));
        }

        // Verify it's a valid xlsx file by checking magic bytes
        let mut file = fs::File::open(&checkpoint.snapshot_path)
            .map_err(|e| anyhow!("failed to open checkpoint file: {}", e))?;

        let mut magic = [0u8; 4];
        use std::io::Read;
        file.read_exact(&mut magic)
            .map_err(|e| anyhow!("failed to read checkpoint file header: {}", e))?;

        // XLSX files are ZIP archives, should start with PK\x03\x04
        if &magic != b"PK\x03\x04" {
            return Err(anyhow!("checkpoint file is not a valid XLSX file"));
        }

        Ok(())
    }

    pub fn add_staged_change(&self, fork_id: &str, staged: StagedChange) -> Result<()> {
        self.with_fork_mut(fork_id, |ctx| {
            ctx.staged_changes.push(staged);
            enforce_staged_limits(ctx);
            Ok(())
        })
    }

    pub fn list_staged_changes(&self, fork_id: &str) -> Result<Vec<StagedChange>> {
        let ctx = self.get_fork(fork_id)?;
        Ok(ctx.staged_changes.clone())
    }

    pub fn take_staged_change(&self, fork_id: &str, change_id: &str) -> Result<StagedChange> {
        self.with_fork_mut(fork_id, |ctx| {
            let index = ctx
                .staged_changes
                .iter()
                .position(|c| c.change_id == change_id)
                .ok_or_else(|| anyhow!("staged change not found: {}", change_id))?;
            Ok(ctx.staged_changes.remove(index))
        })
    }

    pub fn discard_staged_change(&self, fork_id: &str, change_id: &str) -> Result<()> {
        let removed = self.take_staged_change(fork_id, change_id)?;
        remove_staged_snapshot(&removed);
        Ok(())
    }

    fn evict_expired(&self) {
        if self.config.ttl.is_zero() {
            return;
        }
        let mut forks = self.forks.write();
        let expired: Vec<String> = forks
            .iter()
            .filter(|(_, ctx)| ctx.is_expired(self.config.ttl))
            .map(|(id, _)| id.clone())
            .collect();

        let evicted_count = expired.len();
        for id in expired {
            if let Some(ctx) = forks.remove(&id) {
                ctx.cleanup_files();
                debug!(fork_id = %id, "evicted expired fork");
            }
            // Clean up recalc lock
            self.recalc_locks.lock().remove(&id);
        }

        // Update metrics if we evicted any forks
        if evicted_count > 0 {
            drop(forks); // Release write lock
            self.update_fork_metrics();
        }
    }

    /// Update Prometheus fork metrics
    fn update_fork_metrics(&self) {
        let forks = self.forks.read();
        let count = forks.len();
        crate::metrics::METRICS.update_fork_count(count);
    }
}

fn remove_staged_snapshot(staged: &StagedChange) {
    if let Some(path) = staged.fork_path_snapshot.as_ref() {
        let _ = fs::remove_file(path);
    }
}

fn enforce_staged_limits(ctx: &mut ForkContext) {
    while ctx.staged_changes.len() > DEFAULT_MAX_STAGED_CHANGES_PER_FORK {
        let removed = ctx.staged_changes.remove(0);
        remove_staged_snapshot(&removed);
    }
}

fn enforce_checkpoint_limits(ctx: &mut ForkContext) -> Result<()> {
    while ctx.checkpoints.len() > DEFAULT_MAX_CHECKPOINTS_PER_FORK {
        let removed = ctx.checkpoints.remove(0);
        let _ = fs::remove_file(&removed.snapshot_path);
    }

    loop {
        let mut total_bytes = 0u64;
        for cp in &ctx.checkpoints {
            if let Ok(meta) = fs::metadata(&cp.snapshot_path) {
                total_bytes += meta.len();
            }
        }
        if total_bytes <= DEFAULT_MAX_CHECKPOINT_TOTAL_BYTES || ctx.checkpoints.len() <= 1 {
            break;
        }
        let removed = ctx.checkpoints.remove(0);
        let _ = fs::remove_file(&removed.snapshot_path);
    }

    Ok(())
}

impl Clone for ForkContext {
    fn clone(&self) -> Self {
        Self {
            fork_id: self.fork_id.clone(),
            base_path: self.base_path.clone(),
            work_path: self.work_path.clone(),
            created_at: self.created_at,
            last_accessed: self.last_accessed,
            edits: self.edits.clone(),
            staged_changes: self.staged_changes.clone(),
            checkpoints: self.checkpoints.clone(),
            base_hash: self.base_hash.clone(),
            base_modified: self.base_modified,
            version: AtomicU64::new(self.version.load(Ordering::SeqCst)),
        }
    }
}

impl Drop for ForkContext {
    fn drop(&mut self) {
        debug!(fork_id = %self.fork_id, "fork context dropped, cleaning up files");
        self.cleanup_files();
    }
}

#[derive(Debug, Clone)]
pub struct ForkInfo {
    pub fork_id: String,
    pub base_path: String,
    pub created_at: Instant,
    pub edit_count: usize,
    pub version: u64,
}
