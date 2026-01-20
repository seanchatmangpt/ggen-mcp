use crate::config::ServerConfig;
#[cfg(feature = "recalc")]
use crate::fork::{ForkConfig, ForkRegistry};
use crate::model::{WorkbookId, WorkbookListResponse};
#[cfg(feature = "recalc")]
use crate::recalc::{
    GlobalRecalcLock, GlobalScreenshotLock, LibreOfficeBackend, RecalcBackend, RecalcConfig,
    create_executor,
};
use crate::tools::filters::WorkbookFilter;
use crate::utils::{hash_path_metadata, make_short_workbook_id};
use crate::workbook::{WorkbookContext, build_workbook_list};
use anyhow::{Result, anyhow};
use lru::LruCache;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::task;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

/// Application state with enhanced concurrency protection
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
    /// Cache hit counter for statistics
    cache_hits: AtomicU64,
    /// Cache miss counter for statistics
    cache_misses: AtomicU64,
    #[cfg(feature = "recalc")]
    fork_registry: Option<Arc<ForkRegistry>>,
    #[cfg(feature = "recalc")]
    recalc_backend: Option<Arc<dyn RecalcBackend>>,
    #[cfg(feature = "recalc")]
    recalc_semaphore: Option<GlobalRecalcLock>,
    #[cfg(feature = "recalc")]
    screenshot_semaphore: Option<GlobalScreenshotLock>,
}

/// Cache warming configuration
#[derive(Debug, Clone)]
pub struct CacheWarmingConfig {
    /// Enable cache warming on startup
    pub enabled: bool,
    /// Maximum number of workbooks to pre-load
    pub max_workbooks: usize,
    /// Timeout for warming operation (seconds)
    pub timeout_secs: u64,
    /// Specific workbook IDs to warm (empty = auto-detect frequently used)
    pub workbook_ids: Vec<String>,
}

impl Default for CacheWarmingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_workbooks: 5,
            timeout_secs: 30,
            workbook_ids: vec![],
        }
    }
}

impl AppState {
    pub fn new(config: Arc<ServerConfig>) -> Self {
        let capacity = NonZeroUsize::new(config.cache_capacity.max(1)).unwrap();

        #[cfg(feature = "recalc")]
        let (fork_registry, recalc_backend, recalc_semaphore, screenshot_semaphore) =
            if config.recalc_enabled {
                let fork_config = ForkConfig::default();
                let registry = ForkRegistry::new(fork_config)
                    .map(Arc::new)
                    .map_err(|e| tracing::warn!("failed to init fork registry: {}", e))
                    .ok();

                if let Some(registry) = &registry {
                    registry.clone().start_cleanup_task();
                }

                let executor = create_executor(&RecalcConfig::default());
                let backend: Arc<dyn RecalcBackend> = Arc::new(LibreOfficeBackend::new(executor));
                let backend = if backend.is_available() {
                    Some(backend)
                } else {
                    tracing::warn!("recalc backend not available (soffice not found)");
                    None
                };

                let semaphore = GlobalRecalcLock::new(config.max_concurrent_recalcs);
                let screenshot_semaphore = GlobalScreenshotLock::new();

                (
                    registry,
                    backend,
                    Some(semaphore),
                    Some(screenshot_semaphore),
                )
            } else {
                (None, None, None, None)
            };

        Self {
            config,
            cache: RwLock::new(LruCache::new(capacity)),
            index: RwLock::new(HashMap::new()),
            alias_index: RwLock::new(HashMap::new()),
            cache_ops: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            #[cfg(feature = "recalc")]
            fork_registry,
            #[cfg(feature = "recalc")]
            recalc_backend,
            #[cfg(feature = "recalc")]
            recalc_semaphore,
            #[cfg(feature = "recalc")]
            screenshot_semaphore,
        }
    }

    pub fn config(&self) -> Arc<ServerConfig> {
        self.config.clone()
    }

    #[cfg(feature = "recalc")]
    pub fn fork_registry(&self) -> Option<&Arc<ForkRegistry>> {
        self.fork_registry.as_ref()
    }

    #[cfg(feature = "recalc")]
    pub fn recalc_backend(&self) -> Option<&Arc<dyn RecalcBackend>> {
        self.recalc_backend.as_ref()
    }

    #[cfg(feature = "recalc")]
    pub fn recalc_semaphore(&self) -> Option<&GlobalRecalcLock> {
        self.recalc_semaphore.as_ref()
    }

    #[cfg(feature = "recalc")]
    pub fn screenshot_semaphore(&self) -> Option<&GlobalScreenshotLock> {
        self.screenshot_semaphore.as_ref()
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            operations: self.cache_ops.load(Ordering::Relaxed),
            hits: self.cache_hits.load(Ordering::Relaxed),
            misses: self.cache_misses.load(Ordering::Relaxed),
            size: self.cache.read().len(),
            capacity: self.cache.read().cap().get(),
        }
    }

    /// Update Prometheus cache metrics
    fn update_cache_metrics(&self) {
        let cache = self.cache.read();
        let size = cache.len();
        // Rough estimate: 1MB per workbook average
        let estimated_bytes = (size * 1024 * 1024) as u64;
        crate::metrics::METRICS.update_cache_stats(size, estimated_bytes);
    }

    pub fn list_workbooks(&self, filter: WorkbookFilter) -> Result<WorkbookListResponse> {
        let response = build_workbook_list(&self.config, &filter)?;

        // Use write lock only when actually updating indices
        // This minimizes lock contention
        {
            let mut index = self.index.write();
            let mut aliases = self.alias_index.write();
            for descriptor in &response.workbooks {
                let abs_path = self.config.resolve_path(PathBuf::from(&descriptor.path));
                index.insert(descriptor.workbook_id.clone(), abs_path);
                aliases.insert(
                    descriptor.short_id.to_ascii_lowercase(),
                    descriptor.workbook_id.clone(),
                );
            }
        }

        Ok(response)
    }

    pub async fn open_workbook(&self, workbook_id: &WorkbookId) -> Result<Arc<WorkbookContext>> {
        self.cache_ops.fetch_add(1, Ordering::Relaxed);

        let canonical = self.canonicalize_workbook_id(workbook_id)?;

        // First, try to get from cache with read lock only
        {
            let mut cache = self.cache.write();
            if let Some(entry) = cache.get(&canonical) {
                self.cache_hits.fetch_add(1, Ordering::Relaxed);
                crate::metrics::METRICS.record_cache_hit();
                debug!(workbook_id = %canonical, "cache hit");
                return Ok(entry.clone());
            }
        }

        self.cache_misses.fetch_add(1, Ordering::Relaxed);
        crate::metrics::METRICS.record_cache_miss();
        debug!(workbook_id = %canonical, "cache miss");

        // Load workbook outside of locks to avoid blocking other operations
        let path = self.resolve_workbook_path(&canonical)?;
        let config = self.config.clone();
        let path_buf = path.clone();
        let workbook_id_clone = canonical.clone();

        let workbook =
            task::spawn_blocking(move || WorkbookContext::load(&config, &path_buf)).await??;
        let workbook = Arc::new(workbook);

        // Update alias index with minimal lock holding
        {
            let mut aliases = self.alias_index.write();
            aliases.insert(
                workbook.short_id.to_ascii_lowercase(),
                workbook_id_clone.clone(),
            );
        }

        // Insert into cache with write lock
        {
            let mut cache = self.cache.write();
            cache.put(workbook_id_clone, workbook.clone());
        }
        self.update_cache_metrics();

        debug!(workbook_id = %canonical, "workbook loaded and cached");
        Ok(workbook)
    }

    pub fn close_workbook(&self, workbook_id: &WorkbookId) -> Result<()> {
        let canonical = self.canonicalize_workbook_id(workbook_id)?;

        // Atomic cache operation
        {
            let mut cache = self.cache.write();
            cache.pop(&canonical);
        }
        self.update_cache_metrics();

        debug!(workbook_id = %canonical, "workbook closed");
        Ok(())
    }

    pub fn evict_by_path(&self, path: &Path) {
        // Use read lock to find the workbook ID
        let workbook_id = {
            let index = self.index.read();
            index
                .iter()
                .find(|(_, p)| *p == path)
                .map(|(id, _)| id.clone())
        };

        if let Some(id) = workbook_id {
            let mut cache = self.cache.write();
            cache.pop(&id);
            self.update_cache_metrics();
            debug!(path = ?path, workbook_id = %id, "evicted workbook by path");
        }
    }

    fn resolve_workbook_path(&self, workbook_id: &WorkbookId) -> Result<PathBuf> {
        // Check fork registry first with minimal locking
        #[cfg(feature = "recalc")]
        if let Some(registry) = &self.fork_registry {
            if let Some(fork_path) = registry.get_fork_path(workbook_id.as_str()) {
                debug!(workbook_id = %workbook_id, "resolved to fork path");
                return Ok(fork_path);
            }
        }

        // Check index with read lock
        {
            let index = self.index.read();
            if let Some(path) = index.get(workbook_id).cloned() {
                debug!(workbook_id = %workbook_id, "resolved from index");
                return Ok(path);
            }
        }

        // Scan filesystem if not found
        debug!(workbook_id = %workbook_id, "scanning filesystem");
        let located = self.scan_for_workbook(workbook_id.as_str())?;
        self.register_location(&located);
        Ok(located.path)
    }

    fn canonicalize_workbook_id(&self, workbook_id: &WorkbookId) -> Result<WorkbookId> {
        // Check fork registry first
        #[cfg(feature = "recalc")]
        if let Some(registry) = &self.fork_registry {
            if registry.get_fork_path(workbook_id.as_str()).is_some() {
                return Ok(workbook_id.clone());
            }
        }

        // Check index with read lock
        {
            let index = self.index.read();
            if index.contains_key(workbook_id) {
                return Ok(workbook_id.clone());
            }
        }

        // Check aliases with read lock
        {
            let aliases = self.alias_index.read();
            if let Some(mapped) = aliases.get(workbook_id.as_str()).cloned() {
                return Ok(mapped);
            }

            let lowered = workbook_id.as_str().to_ascii_lowercase();
            if lowered != workbook_id.as_str() {
                if let Some(mapped) = aliases.get(&lowered).cloned() {
                    return Ok(mapped);
                }
            }
        }

        // Scan filesystem if not found
        let located = self.scan_for_workbook(workbook_id.as_str())?;
        let canonical = located.workbook_id.clone();
        self.register_location(&located);
        Ok(canonical)
    }

    fn scan_for_workbook(&self, candidate: &str) -> Result<LocatedWorkbook> {
        let candidate_lower = candidate.to_ascii_lowercase();

        if let Some(single) = self.config.single_workbook() {
            let metadata = fs::metadata(single)?;
            let canonical = WorkbookId(hash_path_metadata(single, &metadata));
            let slug = single
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "workbook".to_string());
            let short_id = make_short_workbook_id(&slug, canonical.as_str());
            if candidate_lower == canonical.as_str() || candidate_lower == short_id {
                return Ok(LocatedWorkbook {
                    workbook_id: canonical,
                    short_id,
                    path: single.to_path_buf(),
                });
            }
            return Err(anyhow!(
                "workbook id {} not found in single-workbook mode (expected {} or {})",
                candidate,
                canonical.as_str(),
                short_id
            ));
        }

        for entry in WalkDir::new(&self.config.workspace_root) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if !has_supported_extension(&self.config.supported_extensions, path) {
                continue;
            }
            let metadata = entry.metadata()?;
            let canonical = WorkbookId(hash_path_metadata(path, &metadata));
            let slug = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "workbook".to_string());
            let short_id = make_short_workbook_id(&slug, canonical.as_str());
            if candidate_lower == canonical.as_str() || candidate_lower == short_id {
                return Ok(LocatedWorkbook {
                    workbook_id: canonical,
                    short_id,
                    path: path.to_path_buf(),
                });
            }
        }
        Err(anyhow!("workbook id {} not found", candidate))
    }

    fn register_location(&self, located: &LocatedWorkbook) {
        // Atomic registration of both index and alias
        let mut index = self.index.write();
        let mut aliases = self.alias_index.write();

        index.insert(located.workbook_id.clone(), located.path.clone());
        aliases.insert(
            located.short_id.to_ascii_lowercase(),
            located.workbook_id.clone(),
        );

        debug!(
            workbook_id = %located.workbook_id,
            short_id = %located.short_id,
            path = ?located.path,
            "registered workbook location"
        );
    }

    /// Warm up the cache by pre-loading frequently used workbooks
    /// This eliminates cold-start latency for common operations
    pub async fn warm_cache(&self, config: CacheWarmingConfig) -> Result<CacheWarmingResult> {
        if !config.enabled {
            debug!("cache warming disabled");
            return Ok(CacheWarmingResult::default());
        }

        let start_time = Instant::now();
        info!(
            max_workbooks = config.max_workbooks,
            timeout_secs = config.timeout_secs,
            "starting cache warming"
        );

        let workbooks_to_warm = if !config.workbook_ids.is_empty() {
            // Use explicitly configured workbooks
            config.workbook_ids.clone()
        } else {
            // Auto-detect workbooks to warm
            self.discover_warmup_candidates(config.max_workbooks)?
        };

        let mut loaded = 0;
        let mut failed = 0;
        let mut errors = Vec::new();

        for workbook_id in workbooks_to_warm.iter().take(config.max_workbooks) {
            // Check timeout
            if start_time.elapsed().as_secs() >= config.timeout_secs {
                warn!(
                    loaded,
                    failed, "cache warming timeout reached, stopping early"
                );
                break;
            }

            info!(workbook_id = %workbook_id, "warming cache for workbook");

            match self.open_workbook(&WorkbookId(workbook_id.clone())).await {
                Ok(wb) => {
                    loaded += 1;
                    debug!(
                        workbook_id = %workbook_id,
                        short_id = %wb.short_id,
                        "workbook loaded into cache"
                    );
                }
                Err(e) => {
                    failed += 1;
                    let error_msg = format!("{}: {}", workbook_id, e);
                    warn!(workbook_id = %workbook_id, error = %e, "failed to warm workbook");
                    errors.push(error_msg);
                }
            }
        }

        let duration = start_time.elapsed();
        let result = CacheWarmingResult {
            loaded,
            failed,
            duration_ms: duration.as_millis() as u64,
            errors,
        };

        info!(
            loaded,
            failed,
            duration_ms = result.duration_ms,
            "cache warming completed"
        );

        Ok(result)
    }

    /// Discover candidate workbooks for cache warming
    /// Heuristic: pick the most recently modified workbooks
    fn discover_warmup_candidates(&self, max_count: usize) -> Result<Vec<String>> {
        let mut candidates: Vec<(String, std::time::SystemTime)> = Vec::new();

        if let Some(single) = self.config.single_workbook() {
            let metadata = fs::metadata(single)?;
            let canonical = WorkbookId(hash_path_metadata(single, &metadata));
            if let Ok(modified) = metadata.modified() {
                candidates.push((canonical.0, modified));
            }
        } else {
            for entry in WalkDir::new(&self.config.workspace_root).max_depth(3) {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                if !entry.file_type().is_file() {
                    continue;
                }

                let path = entry.path();
                if !has_supported_extension(&self.config.supported_extensions, path) {
                    continue;
                }

                let metadata = match entry.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let canonical = WorkbookId(hash_path_metadata(path, &metadata));
                if let Ok(modified) = metadata.modified() {
                    candidates.push((canonical.0, modified));
                }
            }
        }

        // Sort by modification time (most recent first)
        candidates.sort_by(|a, b| b.1.cmp(&a.1));

        Ok(candidates
            .into_iter()
            .take(max_count)
            .map(|(id, _)| id)
            .collect())
    }
}

struct LocatedWorkbook {
    workbook_id: WorkbookId,
    short_id: String,
    path: PathBuf,
}

fn has_supported_extension(allowed: &[String], path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let lower = ext.to_ascii_lowercase();
            allowed.iter().any(|candidate| candidate == &lower)
        })
        .unwrap_or(false)
}

/// Cache statistics for monitoring
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub operations: u64,
    pub hits: u64,
    pub misses: u64,
    pub size: usize,
    pub capacity: usize,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.operations == 0 {
            0.0
        } else {
            self.hits as f64 / self.operations as f64
        }
    }
}

/// Result of cache warming operation
#[derive(Debug, Clone, Default)]
pub struct CacheWarmingResult {
    /// Number of workbooks successfully loaded
    pub loaded: usize,
    /// Number of workbooks that failed to load
    pub failed: usize,
    /// Duration of warming operation in milliseconds
    pub duration_ms: u64,
    /// Error messages for failed workbooks
    pub errors: Vec<String>,
}
