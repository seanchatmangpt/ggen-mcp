# Rust Resource Management for MCP Servers

## Table of Contents

1. [Overview](#overview)
2. [RAII Patterns for MCP](#raii-patterns-for-mcp)
3. [Lifetime Management](#lifetime-management)
4. [Shared State Patterns](#shared-state-patterns)
5. [Memory Management](#memory-management)
6. [Resource Limits](#resource-limits)
7. [Cleanup Strategies](#cleanup-strategies)
8. [TPS Waste Elimination Principles](#tps-waste-elimination-principles)

## Overview

This document outlines Rust resource management best practices specifically for MCP (Model Context Protocol) servers, with patterns extracted from the ggen-mcp codebase. These patterns ensure:

- **Zero resource leaks** (aligned with TPS waste elimination)
- **Predictable cleanup** through RAII
- **Safe concurrent access** to shared state
- **Bounded memory usage** for long-running servers
- **Graceful degradation** under resource pressure

### Core Philosophy

MCP servers are long-running processes that must handle:
- Multiple concurrent client requests
- External process management (LibreOffice instances)
- File system resources (temporary files, workbooks)
- Memory constraints (caching, indexing)
- Connection pooling and limits

Rust's ownership system makes this manageable, but requires careful design patterns.

## RAII Patterns for MCP

### What is RAII?

Resource Acquisition Is Initialization (RAII) is a Rust idiom where:
1. Resources are acquired in a constructor (new/create)
2. Resources are released in the Drop implementation
3. The type system guarantees cleanup

### Pattern: Temporary File Guard

**Problem:** Temporary files must be cleaned up even if operations fail.

**Solution:** Use a guard that implements Drop for automatic cleanup.

```rust
// From src/fork.rs
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
```

**Usage:**

```rust
// Create backup with automatic cleanup
let backup_guard = TempFileGuard::new(backup_path.clone());
fs::copy(&work_path, &backup_path)?;

// ... perform operations ...

if operation_succeeded {
    // Disarm guard - keep the file
    backup_guard.disarm();
} else {
    // Guard dropped - file automatically deleted
}
```

### Pattern: Transaction Guard

**Problem:** Multi-step operations need rollback on failure.

**Solution:** Use a guard that tracks transaction state and rolls back in Drop.

```rust
// From src/fork.rs
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
```

**Usage:**

```rust
pub fn create_fork(&self, base_path: &Path) -> Result<String> {
    let fork_id = allocate_unique_id()?;
    let work_path = self.fork_dir.join(format!("{}.xlsx", fork_id));

    // Create RAII guard for rollback on error
    let guard = ForkCreationGuard::new(fork_id.clone(), work_path.clone(), self);

    // Perform operations (any error triggers rollback)
    fs::copy(base_path, &work_path)?;
    let context = ForkContext::new(fork_id.clone(), base_path.to_path_buf(), work_path)?;
    self.forks.write().insert(fork_id.clone(), context);

    // Commit the transaction
    guard.commit();

    Ok(fork_id)
}
```

### Pattern: Checkpoint Guard

**Problem:** Snapshot operations need cleanup if they fail partway through.

**Solution:** Similar to transaction guard but for checkpoint files.

```rust
// From src/fork.rs
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
```

### Pattern: Automatic Resource Cleanup

**Problem:** Complex types with multiple resources need guaranteed cleanup.

**Solution:** Implement Drop to clean up all resources.

```rust
// From src/fork.rs
impl Drop for ForkContext {
    fn drop(&mut self) {
        debug!(fork_id = %self.fork_id, "fork context dropped, cleaning up files");
        self.cleanup_files();
    }
}

impl ForkContext {
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
```

### RAII Best Practices

1. **Use guards for cleanup:** Create guard types for any multi-step operation
2. **Disarm on success:** Guards should have a way to prevent cleanup (disarm/commit)
3. **Log cleanup failures:** Use debug/warn logging when cleanup fails
4. **Validate paths:** Before cleanup, ensure paths are in expected locations
5. **Use best-effort cleanup:** Don't panic in Drop; log and continue

## Lifetime Management

### When to Use Lifetimes vs Owned Types

**Use lifetimes when:**
- Borrowing from a larger structure temporarily
- Implementing views or iterators over existing data
- Performance-critical paths where allocation would be costly

**Use owned types (Arc/Box) when:**
- Data needs to outlive the creator
- Multiple ownership is required
- Shared across async tasks or threads
- Simplicity is more important than micro-optimization

### Lifetime Elision Rules

Rust can infer lifetimes in many cases:

```rust
// Lifetime elision - compiler infers 'a
fn get_sheet(&self, name: &str) -> Result<&Worksheet> {
    // Implicitly: fn get_sheet<'a>(&'a self, name: &str) -> Result<&'a Worksheet>
    // Return type lifetime tied to self
}

// No elision needed for owned return types
fn load_workbook(path: &Path) -> Result<WorkbookContext> {
    // Returns owned data - no lifetime annotations needed
}
```

### When Lifetimes Are Required

**Guards with references:**

```rust
// From src/fork.rs
pub struct ForkCreationGuard<'a> {
    fork_id: String,
    work_path: PathBuf,
    registry: &'a ForkRegistry,  // Borrowed from caller
    committed: bool,
}

impl<'a> ForkCreationGuard<'a> {
    fn new(fork_id: String, work_path: PathBuf, registry: &'a ForkRegistry) -> Self {
        // Lifetime 'a ensures guard doesn't outlive the registry
        Self { fork_id, work_path, registry, committed: false }
    }
}
```

### Avoiding Lifetime Complexity

**Problem:** Complex lifetime annotations make code hard to maintain.

**Solution:** Use Arc for shared ownership instead.

```rust
// Instead of this (complex lifetimes):
pub struct WorkbookView<'a> {
    sheet: &'a Worksheet,
    config: &'a ServerConfig,
}

// Do this (simple ownership):
pub struct WorkbookContext {
    spreadsheet: Arc<RwLock<Spreadsheet>>,
    config: Arc<ServerConfig>,
}

impl WorkbookContext {
    pub fn with_sheet<T, F>(&self, sheet_name: &str, func: F) -> Result<T>
    where
        F: FnOnce(&Worksheet) -> T,
    {
        let book = self.spreadsheet.read();
        let sheet = book.get_sheet_by_name(sheet_name)
            .ok_or_else(|| anyhow!("sheet {} not found", sheet_name))?;
        Ok(func(sheet))
    }
}
```

This pattern:
- Avoids lifetime annotations
- Allows flexible borrowing through closures
- Keeps locks scoped and short-lived

### Struct Lifetime Annotations

When you must use lifetimes in structs:

```rust
pub struct RegionAnalyzer<'a> {
    sheet: &'a Worksheet,
    metrics: &'a SheetMetrics,
}

impl<'a> RegionAnalyzer<'a> {
    pub fn new(sheet: &'a Worksheet, metrics: &'a SheetMetrics) -> Self {
        Self { sheet, metrics }
    }

    pub fn detect_regions(&self) -> Vec<Region> {
        // Analysis using borrowed data
    }
}

// Usage - lifetime is scoped to function
fn analyze_sheet(ctx: &WorkbookContext, sheet_name: &str) -> Result<Vec<Region>> {
    ctx.with_sheet(sheet_name, |sheet| {
        let metrics = compute_metrics(sheet);
        let analyzer = RegionAnalyzer::new(sheet, &metrics);
        analyzer.detect_regions()
    })
}
```

### Generic Lifetime Bounds

When combining generics with lifetimes:

```rust
// Closure that borrows from context
pub fn with_fork_mut<F, R>(&self, fork_id: &str, f: F) -> Result<R>
where
    F: FnOnce(&mut ForkContext) -> Result<R>,
{
    let mut forks = self.forks.write();
    let ctx = forks.get_mut(fork_id)
        .ok_or_else(|| anyhow!("fork not found: {}", fork_id))?;
    ctx.touch();
    let result = f(ctx)?;
    ctx.increment_version();
    Ok(result)
}
```

### Lifetime Best Practices

1. **Prefer owned types:** Use Arc/Box unless profiling shows a problem
2. **Keep lifetimes simple:** Avoid multiple lifetime parameters if possible
3. **Use closures for scoped access:** Better than returning references
4. **Document lifetime relationships:** When lifetimes are needed, explain why
5. **Leverage elision:** Let the compiler infer when possible

## Shared State Patterns

### RwLock vs Mutex Selection

**Use RwLock when:**
- Many readers, few writers
- Read operations are common and should not block each other
- Write operations are relatively rare

**Use Mutex when:**
- Reads and writes are equally common
- Critical sections are very short
- Simpler semantics needed

### Pattern: Read-Heavy State (RwLock)

```rust
// From src/state.rs
pub struct AppState {
    config: Arc<ServerConfig>,
    /// Workbook cache with RwLock for concurrent read access
    cache: RwLock<LruCache<WorkbookId, Arc<WorkbookContext>>>,
    /// Workbook ID to path index with RwLock for concurrent reads
    index: RwLock<HashMap<WorkbookId, PathBuf>>,
    /// Alias to workbook ID mapping with RwLock for concurrent reads
    alias_index: RwLock<HashMap<String, WorkbookId>>,
    /// Cache operation counter for monitoring
    cache_ops: AtomicU64,
}

impl AppState {
    pub async fn open_workbook(&self, workbook_id: &WorkbookId) -> Result<Arc<WorkbookContext>> {
        self.cache_ops.fetch_add(1, Ordering::Relaxed);

        // First, try to get from cache with read lock only
        {
            let mut cache = self.cache.write();
            if let Some(entry) = cache.get(&canonical) {
                self.cache_hits.fetch_add(1, Ordering::Relaxed);
                return Ok(entry.clone());
            }
        }

        // Load outside of locks to avoid blocking
        let workbook = load_workbook_blocking()?;

        // Insert with write lock - short duration
        {
            let mut cache = self.cache.write();
            cache.put(canonical, workbook.clone());
        }

        Ok(workbook)
    }
}
```

**Key principles:**
1. **Minimize lock duration:** Perform expensive operations outside locks
2. **Prefer read locks:** Use read() when possible
3. **Clone Arc references:** Cheap to clone, allows early lock release
4. **Separate concerns:** Different RwLocks for different data structures

### Pattern: Per-Resource Locks

```rust
// From src/fork.rs
pub struct ForkRegistry {
    /// RwLock for better read concurrency on fork access
    forks: RwLock<HashMap<String, ForkContext>>,
    /// Per-fork locks for recalc operations
    recalc_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    config: ForkConfig,
}

impl ForkRegistry {
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
}

// Usage
async fn recalculate_fork(registry: &ForkRegistry, fork_id: &str) -> Result<()> {
    let lock = registry.acquire_recalc_lock(fork_id);
    let _guard = lock.lock();

    // Only one recalc operation per fork at a time
    perform_recalc(fork_id).await?;

    registry.release_recalc_lock(fork_id);
    Ok(())
}
```

**Benefits:**
- Fine-grained locking per resource
- Prevents concurrent modification of same fork
- Other forks can be recalculated in parallel

### Pattern: Optimistic Locking with Versioning

```rust
// From src/fork.rs
pub struct ForkContext {
    pub fork_id: String,
    pub work_path: PathBuf,
    pub edits: Vec<EditOp>,
    /// Version counter for optimistic locking
    version: AtomicU64,
}

impl ForkContext {
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::SeqCst)
    }

    pub fn increment_version(&self) -> u64 {
        self.version.fetch_add(1, Ordering::SeqCst) + 1
    }

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
}

// Registry provides version-checked mutation
impl ForkRegistry {
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
        let ctx = forks.get_mut(fork_id)
            .ok_or_else(|| anyhow!("fork not found: {}", fork_id))?;
        ctx.validate_version(expected_version)?;
        ctx.touch();
        let result = f(ctx)?;
        ctx.increment_version();
        Ok(result)
    }
}
```

**Use cases:**
- Detecting concurrent modifications
- Implementing retry logic
- Ensuring consistency across operations

### Lock-Free Patterns with Atomics

```rust
// From src/state.rs
pub struct AppState {
    /// Cache operation counter for monitoring
    cache_ops: AtomicU64,
    /// Cache hit counter for statistics
    cache_hits: AtomicU64,
    /// Cache miss counter for statistics
    cache_misses: AtomicU64,
}

impl AppState {
    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            operations: self.cache_ops.load(Ordering::Relaxed),
            hits: self.cache_hits.load(Ordering::Relaxed),
            misses: self.cache_misses.load(Ordering::Relaxed),
            size: self.cache.read().len(),
            capacity: self.cache.read().cap().get(),
        }
    }
}
```

**When to use atomics:**
- Simple counters and flags
- Statistics gathering
- Non-critical metadata
- High-contention scenarios where locks would be bottleneck

**Ordering guidelines:**
- `Relaxed`: Counters, statistics (no synchronization needed)
- `Acquire/Release`: Flagging completion of operations
- `SeqCst`: When in doubt, or for correctness-critical operations

### Pattern: Lazy Initialization with RwLock

```rust
// From src/workbook.rs
pub struct SheetCacheEntry {
    pub metrics: SheetMetrics,
    pub style_tags: Vec<String>,
    detected_regions: RwLock<Option<Vec<DetectedRegion>>>,
    region_notes: RwLock<Vec<String>>,
}

impl SheetCacheEntry {
    pub fn detected_regions(&self) -> Vec<DetectedRegion> {
        self.detected_regions
            .read()
            .as_ref()
            .cloned()
            .unwrap_or_default()
    }

    pub fn has_detected_regions(&self) -> bool {
        self.detected_regions.read().is_some()
    }

    pub fn set_detected_regions(&self, regions: Vec<DetectedRegion>) {
        let mut guard = self.detected_regions.write();
        if guard.is_none() {
            *guard = Some(regions);
        }
    }
}
```

**Benefits:**
- Compute expensive values only when needed
- Thread-safe lazy initialization
- Multiple readers can check status without blocking

### Shared State Best Practices

1. **Use parking_lot:** Faster than std locks for most use cases
2. **Keep critical sections short:** Minimize time holding locks
3. **Clone Arc references:** Release locks early by cloning Arc pointers
4. **Separate data structures:** Use different locks for independent data
5. **Document lock ordering:** Prevent deadlocks by establishing order
6. **Consider message passing:** Use channels for some concurrent patterns

## Memory Management

### Zero-Copy Patterns

**Problem:** Avoid unnecessary allocations when working with string data.

**Solution:** Use `&str` for borrowed strings, `Cow` for conditional allocation.

```rust
use std::borrow::Cow;

// Good: Accept borrowed strings
fn process_sheet_name(name: &str) -> Result<()> {
    // Work with borrowed data
}

// Better: Use Cow when you might need to modify
fn normalize_sheet_name(name: &str) -> Cow<str> {
    if name.chars().all(|c| c.is_alphanumeric()) {
        Cow::Borrowed(name)
    } else {
        Cow::Owned(name.replace(|c: char| !c.is_alphanumeric(), "_"))
    }
}
```

### String Handling: String vs &str vs Cow

**Use `&str` when:**
- Reading/viewing string data
- Passing to functions that don't need ownership
- Working with string literals

**Use `String` when:**
- Building strings dynamically
- Storing strings in structs
- Returning owned data

**Use `Cow<str>` when:**
- Sometimes need to modify, sometimes not
- Want to avoid allocation when possible
- Building APIs that are allocation-flexible

```rust
// From ggen-mcp patterns
pub fn make_short_workbook_id(slug: &str, full_id: &str) -> String {
    // Return String - always allocating anyway
    format!("{}_{}", slug, &full_id[..8])
}

pub fn column_number_to_name(col: u32) -> String {
    // Return String - building new value
    let mut name = String::new();
    let mut n = col;
    while n > 0 {
        n -= 1;
        name.push((b'A' + (n % 26) as u8) as char);
        n /= 26;
    }
    name.chars().rev().collect()
}
```

### Buffer Reuse Patterns

**Problem:** Repeated allocations in hot loops.

**Solution:** Reuse buffers when possible.

```rust
// Example for processing many cells
pub fn process_cells(sheet: &Worksheet) -> Result<Vec<ProcessedCell>> {
    let mut results = Vec::with_capacity(1000);  // Pre-allocate
    let mut buffer = String::new();  // Reuse for string building

    for cell in sheet.get_cell_collection() {
        buffer.clear();  // Clear but keep capacity

        // Build string in reused buffer
        write!(&mut buffer, "{}:{}", cell.row(), cell.col())?;

        results.push(ProcessedCell {
            address: buffer.clone(),  // Clone only when storing
            value: cell.value(),
        });
    }

    Ok(results)
}
```

### Pattern: Arc-based Sharing

```rust
// From src/state.rs and src/workbook.rs
pub struct AppState {
    config: Arc<ServerConfig>,  // Config shared across all operations
    cache: RwLock<LruCache<WorkbookId, Arc<WorkbookContext>>>,  // Cache Arc references
}

pub struct WorkbookContext {
    pub id: WorkbookId,
    pub path: PathBuf,
    spreadsheet: Arc<RwLock<Spreadsheet>>,  // Shared access to spreadsheet
    sheet_cache: RwLock<HashMap<String, Arc<SheetCacheEntry>>>,  // Cached entries shared
    formula_atlas: Arc<FormulaAtlas>,  // Shared formula index
}

impl AppState {
    pub async fn open_workbook(&self, id: &WorkbookId) -> Result<Arc<WorkbookContext>> {
        // Check cache
        if let Some(entry) = self.cache.write().get(id) {
            return Ok(entry.clone());  // Clone Arc - cheap!
        }

        // Load and wrap in Arc
        let workbook = Arc::new(WorkbookContext::load(id)?);
        self.cache.write().put(id.clone(), workbook.clone());
        Ok(workbook)
    }
}
```

**Benefits:**
1. **Cheap cloning:** `Arc::clone` is just pointer + atomic increment
2. **Shared ownership:** Multiple references to same data
3. **Memory efficiency:** One copy shared by many consumers
4. **Thread-safe:** Arc can be sent across threads

### Pattern: Bounded Caches (LRU)

```rust
// From src/state.rs
use lru::LruCache;
use std::num::NonZeroUsize;

pub struct AppState {
    cache: RwLock<LruCache<WorkbookId, Arc<WorkbookContext>>>,
}

impl AppState {
    pub fn new(config: Arc<ServerConfig>) -> Self {
        let capacity = NonZeroUsize::new(config.cache_capacity.max(1)).unwrap();

        Self {
            config,
            cache: RwLock::new(LruCache::new(capacity)),
            // ...
        }
    }

    pub async fn open_workbook(&self, id: &WorkbookId) -> Result<Arc<WorkbookContext>> {
        // LRU cache automatically evicts least-recently-used entries
        let mut cache = self.cache.write();
        if let Some(entry) = cache.get(id) {
            return Ok(entry.clone());
        }

        let workbook = Arc::new(load_workbook(id)?);
        cache.put(id.clone(), workbook.clone());  // May evict old entry
        Ok(workbook)
    }
}
```

**Configuration:**

```rust
// From src/config.rs
const DEFAULT_CACHE_CAPACITY: usize = 5;
const MAX_CACHE_CAPACITY: usize = 1000;
const MIN_CACHE_CAPACITY: usize = 1;

pub struct ServerConfig {
    pub cache_capacity: usize,
    // ...
}

impl ServerConfig {
    pub fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            self.cache_capacity >= MIN_CACHE_CAPACITY,
            "cache_capacity must be at least {} (got {})",
            MIN_CACHE_CAPACITY,
            self.cache_capacity
        );
        anyhow::ensure!(
            self.cache_capacity <= MAX_CACHE_CAPACITY,
            "cache_capacity must not exceed {} (got {})",
            MAX_CACHE_CAPACITY,
            self.cache_capacity
        );
        Ok(())
    }
}
```

### Memory Best Practices

1. **Pre-allocate when size is known:** Use `Vec::with_capacity`
2. **Reuse buffers in hot paths:** Clear instead of reallocating
3. **Use Arc for shared data:** Cheap cloning, automatic cleanup
4. **Implement bounded caches:** Prevent unbounded growth
5. **Profile before optimizing:** Measure actual allocations
6. **Consider smallvec:** Stack-allocated small vectors

## Resource Limits

### File Descriptor Management

```rust
// From src/fork.rs
const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB

pub fn create_fork(&self, base_path: &Path) -> Result<String> {
    // Validate file size before copying
    let metadata = fs::metadata(base_path)?;
    if metadata.len() > MAX_FILE_SIZE {
        return Err(anyhow!(
            "base file too large: {} bytes (max {} MB)",
            metadata.len(),
            MAX_FILE_SIZE / 1024 / 1024
        ));
    }

    // Check fork count before creating
    if self.forks.read().len() >= self.config.max_forks {
        return Err(anyhow!(
            "max forks ({}) reached, discard existing forks first",
            self.config.max_forks
        ));
    }

    // ... create fork
}
```

### Connection/Process Pooling

```rust
// From src/recalc/mod.rs and src/state.rs
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct GlobalRecalcLock(pub Arc<Semaphore>);

impl GlobalRecalcLock {
    pub fn new(permits: usize) -> Self {
        Self(Arc::new(Semaphore::new(permits)))
    }
}

pub struct AppState {
    recalc_semaphore: Option<GlobalRecalcLock>,
    screenshot_semaphore: Option<GlobalScreenshotLock>,
}

// Usage
async fn perform_recalc(state: &AppState, workbook_path: &Path) -> Result<()> {
    let semaphore = state.recalc_semaphore()
        .ok_or_else(|| anyhow!("recalc not enabled"))?;

    // Acquire permit - blocks if max concurrent recalcs reached
    let _permit = semaphore.0.acquire().await?;

    // Only max_concurrent_recalcs LibreOffice instances run at once
    run_libreoffice_recalc(workbook_path).await?;

    // Permit automatically released on drop
    Ok(())
}
```

**Configuration:**

```rust
// From src/config.rs
const DEFAULT_MAX_RECALCS: usize = 2;
const MAX_CONCURRENT_RECALCS: usize = 100;
const MIN_CONCURRENT_RECALCS: usize = 1;

pub struct ServerConfig {
    pub max_concurrent_recalcs: usize,
}

impl ServerConfig {
    pub fn validate(&self) -> Result<()> {
        if self.recalc_enabled {
            anyhow::ensure!(
                self.max_concurrent_recalcs >= MIN_CONCURRENT_RECALCS,
                "max_concurrent_recalcs must be at least {} (got {})",
                MIN_CONCURRENT_RECALCS,
                self.max_concurrent_recalcs
            );
            anyhow::ensure!(
                self.max_concurrent_recalcs <= MAX_CONCURRENT_RECALCS,
                "max_concurrent_recalcs must not exceed {} (got {})",
                MAX_CONCURRENT_RECALCS,
                self.max_concurrent_recalcs
            );
        }
        Ok(())
    }
}
```

### Memory Budgets

```rust
// From src/fork.rs - Checkpoint limits
const DEFAULT_MAX_CHECKPOINTS_PER_FORK: usize = 10;
const DEFAULT_MAX_CHECKPOINT_TOTAL_BYTES: u64 = 500 * 1024 * 1024;  // 500MB

fn enforce_checkpoint_limits(ctx: &mut ForkContext) -> Result<()> {
    // Limit by count
    while ctx.checkpoints.len() > DEFAULT_MAX_CHECKPOINTS_PER_FORK {
        let removed = ctx.checkpoints.remove(0);
        let _ = fs::remove_file(&removed.snapshot_path);
    }

    // Limit by total size
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
```

### Operation Limits (Detection Caps)

```rust
// From src/workbook.rs - Detection limits to prevent runaway computation
const DETECT_MAX_ROWS: u32 = 10_000;
const DETECT_MAX_COLS: u32 = 500;
const DETECT_MAX_AREA: u64 = 5_000_000;
const DETECT_MAX_CELLS: usize = 200_000;
const DETECT_MAX_LEAVES: usize = 200;
const DETECT_MAX_DEPTH: u32 = 12;
const DETECT_MAX_MS: u64 = 200;

struct DetectLimits {
    start: Instant,
    max_ms: u64,
    max_leaves: usize,
    max_depth: u32,
    leaves: usize,
    exceeded_time: bool,
    exceeded_leaves: bool,
}

impl DetectLimits {
    fn should_stop(&mut self) -> bool {
        if !self.exceeded_time && self.start.elapsed().as_millis() as u64 >= self.max_ms {
            self.exceeded_time = true;
        }
        self.exceeded_time || self.exceeded_leaves
    }
}

fn detect_regions(sheet: &Worksheet, metrics: &SheetMetrics) -> DetectRegionsResult {
    let area = (metrics.row_count as u64) * (metrics.column_count as u64);
    let exceeds_caps = metrics.row_count > DETECT_MAX_ROWS
        || metrics.column_count > DETECT_MAX_COLS
        || area > DETECT_MAX_AREA
        || occupancy.cells.len() > DETECT_MAX_CELLS;

    if exceeds_caps {
        // Return fallback result instead of attempting full detection
        return fallback_detection(metrics);
    }

    let mut limits = DetectLimits::new();
    let mut regions = Vec::new();

    // Recursive analysis with limit checking
    analyze_recursively(&mut regions, &mut limits);

    if limits.exceeded_time || limits.exceeded_leaves {
        // Add note about truncation
        result.notes.push("Region detection truncated due to time/complexity caps.".to_string());
    }

    result
}
```

### Resource Limit Best Practices

1. **Validate inputs early:** Check sizes before processing
2. **Use semaphores for process limits:** Tokio semaphores for async control
3. **Implement memory budgets:** Both count and size limits
4. **Set computation timeouts:** Prevent runaway operations
5. **Provide fallback behavior:** Graceful degradation when limits exceeded
6. **Make limits configurable:** Allow tuning for different deployments

## Cleanup Strategies

### Drop Implementation Patterns

**Pattern: Comprehensive cleanup**

```rust
// From src/fork.rs
impl Drop for ForkContext {
    fn drop(&mut self) {
        debug!(fork_id = %self.fork_id, "fork context dropped, cleaning up files");
        self.cleanup_files();
    }
}

impl ForkContext {
    fn cleanup_files(&self) {
        // Clean up work file
        let _ = fs::remove_file(&self.work_path);

        // Clean up staged changes
        for staged in &self.staged_changes {
            remove_staged_snapshot(staged);
        }

        // Clean up checkpoint directory
        let checkpoint_dir = self.checkpoint_dir();
        if checkpoint_dir.starts_with(CHECKPOINT_DIR) {
            let _ = fs::remove_dir_all(&checkpoint_dir);
        }
    }
}
```

**Best practices:**
1. **Don't panic in Drop:** Use `let _ =` to ignore errors
2. **Log failures:** Use debug/warn to track cleanup issues
3. **Validate paths:** Ensure cleanup doesn't delete wrong files
4. **Use constants for directories:** Prevent path manipulation bugs

### Graceful Shutdown

**Pattern: Background cleanup tasks**

```rust
// From src/fork.rs
impl ForkRegistry {
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

        for id in expired {
            if let Some(ctx) = forks.remove(&id) {
                ctx.cleanup_files();
                debug!(fork_id = %id, "evicted expired fork");
            }
            // Clean up recalc lock
            self.recalc_locks.lock().remove(&id);
        }
    }
}
```

**For server shutdown:**

```rust
use tokio::signal;

async fn run_server(state: Arc<AppState>) -> Result<()> {
    let server = start_mcp_server(state.clone());

    tokio::select! {
        _ = server => {
            info!("Server completed");
        }
        _ = signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
    }

    // Graceful cleanup
    shutdown_gracefully(state).await?;
    Ok(())
}

async fn shutdown_gracefully(state: Arc<AppState>) -> Result<()> {
    info!("Starting graceful shutdown");

    // Close all forks
    if let Some(registry) = state.fork_registry() {
        let fork_ids: Vec<String> = registry.list_forks()
            .into_iter()
            .map(|info| info.fork_id)
            .collect();

        for fork_id in fork_ids {
            let _ = registry.discard_fork(&fork_id);
        }
    }

    // Clear cache (triggers Drop on workbooks)
    {
        let mut cache = state.cache.write();
        cache.clear();
    }

    info!("Graceful shutdown complete");
    Ok(())
}
```

### Resource Leak Detection

**Pattern: Reference counting checks**

```rust
// From src/fork.rs
pub fn release_recalc_lock(&self, fork_id: &str) {
    let mut locks = self.recalc_locks.lock();

    if let Some(lock) = locks.get(fork_id) {
        // Check if we're the last reference
        if Arc::strong_count(lock) == 1 {
            locks.remove(fork_id);
        } else {
            // Leak detection - someone still holds a reference
            warn!(
                fork_id = %fork_id,
                ref_count = Arc::strong_count(lock),
                "recalc lock still has references"
            );
        }
    }
}
```

**Pattern: Explicit validation**

```rust
// From src/fork.rs - Checkpoint validation before use
fn validate_checkpoint(&self, checkpoint: &Checkpoint) -> Result<()> {
    if !checkpoint.snapshot_path.exists() {
        return Err(anyhow!(
            "checkpoint file does not exist: {:?}",
            checkpoint.snapshot_path
        ));
    }

    let metadata = fs::metadata(&checkpoint.snapshot_path)?;

    if metadata.len() == 0 {
        return Err(anyhow!("checkpoint file is empty"));
    }

    if metadata.len() > MAX_FILE_SIZE {
        return Err(anyhow!("checkpoint file exceeds maximum size"));
    }

    // Verify XLSX magic bytes
    let mut file = fs::File::open(&checkpoint.snapshot_path)?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)?;

    if &magic != b"PK\x03\x04" {
        return Err(anyhow!("checkpoint file is not a valid XLSX file"));
    }

    Ok(())
}
```

### Cleanup Ordering

**Pattern: Cleanup in reverse dependency order**

```rust
impl ForkContext {
    fn cleanup_files(&self) {
        // 1. Remove staged changes first (depend on work file existing)
        for staged in &self.staged_changes {
            if let Some(path) = staged.fork_path_snapshot.as_ref() {
                let _ = fs::remove_file(path);
            }
        }

        // 2. Remove checkpoints (independent)
        let checkpoint_dir = self.checkpoint_dir();
        if checkpoint_dir.starts_with(CHECKPOINT_DIR) {
            let _ = fs::remove_dir_all(&checkpoint_dir);
        }

        // 3. Finally remove work file (last because others may reference it)
        let _ = fs::remove_file(&self.work_path);
    }
}
```

### Cleanup Best Practices

1. **Implement Drop for all resources:** Ensure automatic cleanup
2. **Use background tasks for periodic cleanup:** Async cleanup tasks
3. **Handle shutdown gracefully:** Respond to signals properly
4. **Validate before cleanup:** Ensure files/resources are what you expect
5. **Clean up in correct order:** Dependencies first, then dependents
6. **Log all cleanup operations:** Debug-level logs for tracking

## TPS Waste Elimination Principles

The Toyota Production System identifies seven types of waste. In Rust MCP servers, these translate to:

### 1. Overproduction (Computing Too Much)

**Waste:** Computing values that might not be needed.

**Solution:** Lazy evaluation with RwLock.

```rust
// From src/workbook.rs
pub struct SheetCacheEntry {
    pub metrics: SheetMetrics,
    detected_regions: RwLock<Option<Vec<DetectedRegion>>>,  // Lazy
}

impl SheetCacheEntry {
    pub fn has_detected_regions(&self) -> bool {
        self.detected_regions.read().is_some()
    }

    pub fn set_detected_regions(&self, regions: Vec<DetectedRegion>) {
        let mut guard = self.detected_regions.write();
        if guard.is_none() {  // Only compute once
            *guard = Some(regions);
        }
    }
}
```

### 2. Waiting (Blocking)

**Waste:** Holding locks while doing expensive work.

**Solution:** Minimize lock duration.

```rust
// Bad: Long lock duration
let mut cache = self.cache.write();
let workbook = load_workbook()?;  // Expensive!
cache.put(id, workbook);

// Good: Load outside lock
let workbook = load_workbook()?;  // Outside lock
let mut cache = self.cache.write();
cache.put(id, workbook);  // Short lock
```

### 3. Transportation (Unnecessary Copies)

**Waste:** Copying data that could be borrowed or shared.

**Solution:** Use Arc for shared ownership.

```rust
// Bad: Clone entire workbook
pub fn get_workbook(&self, id: &str) -> Result<WorkbookContext> {
    self.cache.get(id).cloned()  // Full clone!
}

// Good: Share via Arc
pub fn get_workbook(&self, id: &str) -> Result<Arc<WorkbookContext>> {
    Ok(self.cache.get(id)?.clone())  // Arc clone - cheap!
}
```

### 4. Over-Processing (Doing More Than Needed)

**Waste:** Processing entire dataset when subset is sufficient.

**Solution:** Implement caps and limits.

```rust
// From src/workbook.rs
const DETECT_MAX_MS: u64 = 200;

fn detect_regions(sheet: &Worksheet) -> Result<Vec<Region>> {
    let mut limits = DetectLimits::new();

    // Stop if taking too long
    if limits.should_stop() {
        return Ok(partial_results);
    }

    // Continue processing...
}
```

### 5. Inventory (Unbounded Caches)

**Waste:** Accumulating unlimited cached data.

**Solution:** Use bounded LRU caches.

```rust
// From src/state.rs
use lru::LruCache;

pub struct AppState {
    cache: RwLock<LruCache<WorkbookId, Arc<WorkbookContext>>>,
}

impl AppState {
    pub fn new(config: Arc<ServerConfig>) -> Self {
        let capacity = NonZeroUsize::new(config.cache_capacity.max(1)).unwrap();
        Self {
            cache: RwLock::new(LruCache::new(capacity)),  // Bounded!
        }
    }
}
```

### 6. Motion (Inefficient Access Patterns)

**Waste:** Repeatedly acquiring/releasing locks.

**Solution:** Batch operations under single lock.

```rust
// Bad: Multiple lock acquisitions
for item in items {
    let mut cache = self.cache.write();
    cache.update(item);
}  // Lock/unlock each iteration

// Good: Single lock for batch
{
    let mut cache = self.cache.write();
    for item in items {
        cache.update(item);
    }
}  // Lock once for all
```

### 7. Defects (Resource Leaks)

**Waste:** Resources not properly cleaned up.

**Solution:** RAII with Drop implementations.

```rust
// From src/fork.rs
impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.cleanup_on_drop {
            let _ = fs::remove_file(&self.path);  // Always cleaned up
        }
    }
}
```

### TPS Resource Management Summary

| TPS Principle | Rust Pattern | Example |
|---------------|--------------|---------|
| Overproduction | Lazy evaluation | RwLock<Option<T>> |
| Waiting | Minimize lock duration | Load outside locks |
| Transportation | Shared ownership | Arc instead of Clone |
| Over-processing | Computation limits | Timeouts, caps |
| Inventory | Bounded caches | LRU with max size |
| Motion | Batch operations | Single lock for batch |
| Defects | RAII cleanup | Drop implementations |

## Conclusion

Resource management in Rust MCP servers requires:

1. **RAII patterns** for guaranteed cleanup
2. **Careful lifetime management** (prefer Arc when in doubt)
3. **Appropriate lock selection** (RwLock for read-heavy, Mutex for balanced)
4. **Memory-efficient patterns** (Arc sharing, bounded caches)
5. **Resource limits** (files, processes, memory, computation)
6. **Comprehensive cleanup** (Drop, background tasks, graceful shutdown)
7. **TPS waste elimination** (lazy evaluation, batching, limits)

By following these patterns, MCP servers can run reliably for extended periods without resource leaks, with predictable performance characteristics, and graceful behavior under load.

## References

- [src/state.rs](/home/user/ggen-mcp/src/state.rs) - Application state with concurrent caching
- [src/fork.rs](/home/user/ggen-mcp/src/fork.rs) - Fork management with RAII guards
- [src/workbook.rs](/home/user/ggen-mcp/src/workbook.rs) - Workbook context and lazy caching
- [src/config.rs](/home/user/ggen-mcp/src/config.rs) - Resource limit configuration
- [docs/TPS_WASTE_ELIMINATION.md](/home/user/ggen-mcp/docs/TPS_WASTE_ELIMINATION.md) - TPS principles
