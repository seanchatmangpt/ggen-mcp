/// Resource Management Patterns for MCP Servers
///
/// This example demonstrates best practices for resource management in Rust MCP servers,
/// covering RAII guards, shared state, lifetime management, and cleanup strategies.
///
/// Run with: cargo run --example resource_management_patterns

use anyhow::{anyhow, Result};
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

// =============================================================================
// RAII Guard Patterns
// =============================================================================

/// RAII guard for temporary files - ensures cleanup on drop
///
/// Example usage:
/// ```
/// let guard = TempFileGuard::new(temp_path);
/// perform_operations(guard.path())?;
/// guard.disarm(); // Keep file on success
/// ```
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
                eprintln!("Failed to cleanup temp file {:?}: {}", self.path, e);
            } else {
                println!("Cleaned up temp file: {:?}", self.path);
            }
        }
    }
}

/// RAII guard for transactions - rollback on drop unless committed
///
/// Example usage:
/// ```
/// let guard = TransactionGuard::new("txn-123", cleanup_fn);
/// perform_multi_step_operation()?;
/// guard.commit(); // Success - no rollback
/// ```
pub struct TransactionGuard<F>
where
    F: FnOnce(),
{
    transaction_id: String,
    rollback_fn: Option<F>,
    committed: bool,
}

impl<F> TransactionGuard<F>
where
    F: FnOnce(),
{
    pub fn new(transaction_id: String, rollback_fn: F) -> Self {
        Self {
            transaction_id,
            rollback_fn: Some(rollback_fn),
            committed: false,
        }
    }

    pub fn commit(mut self) {
        self.committed = true;
        // Drop will not call rollback
    }
}

impl<F> Drop for TransactionGuard<F>
where
    F: FnOnce(),
{
    fn drop(&mut self) {
        if !self.committed {
            if let Some(rollback_fn) = self.rollback_fn.take() {
                println!("Rolling back transaction: {}", self.transaction_id);
                rollback_fn();
            }
        }
    }
}

// =============================================================================
// Shared State with RwLock and Arc
// =============================================================================

/// Resource with version tracking for optimistic locking
pub struct Resource {
    pub id: String,
    pub data: String,
    version: AtomicU64,
    last_accessed: Mutex<Instant>,
}

impl Resource {
    pub fn new(id: String, data: String) -> Self {
        Self {
            id,
            data,
            version: AtomicU64::new(0),
            last_accessed: Mutex::new(Instant::now()),
        }
    }

    pub fn version(&self) -> u64 {
        self.version.load(Ordering::SeqCst)
    }

    pub fn increment_version(&self) -> u64 {
        self.version.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn validate_version(&self, expected: u64) -> Result<()> {
        let current = self.version();
        if current != expected {
            return Err(anyhow!(
                "Version mismatch: expected {}, got {}",
                expected,
                current
            ));
        }
        Ok(())
    }

    pub fn touch(&self) {
        *self.last_accessed.lock() = Instant::now();
    }

    pub fn last_accessed(&self) -> Instant {
        *self.last_accessed.lock()
    }
}

/// Registry with read-heavy access pattern (RwLock)
pub struct ResourceRegistry {
    resources: RwLock<HashMap<String, Arc<Resource>>>,
    /// Per-resource locks for exclusive operations
    operation_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    /// Statistics using atomics (lock-free)
    access_count: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self {
            resources: RwLock::new(HashMap::new()),
            operation_locks: Mutex::new(HashMap::new()),
            access_count: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
        }
    }

    /// Get resource with read lock only
    pub fn get(&self, id: &str) -> Option<Arc<Resource>> {
        self.access_count.fetch_add(1, Ordering::Relaxed);

        let resources = self.resources.read();
        if let Some(resource) = resources.get(id) {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
            resource.touch();
            Some(resource.clone()) // Clone Arc - cheap!
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Insert resource with write lock
    pub fn insert(&self, resource: Resource) {
        let mut resources = self.resources.write();
        resources.insert(resource.id.clone(), Arc::new(resource));
    }

    /// Modify resource with version checking
    pub fn modify_versioned<F>(&self, id: &str, expected_version: u64, f: F) -> Result<()>
    where
        F: FnOnce(&mut Resource) -> Result<()>,
    {
        // Acquire per-resource lock for exclusive access
        let lock = self.acquire_operation_lock(id);
        let _guard = lock.lock();

        // Get resource with write lock
        let mut resources = self.resources.write();
        let resource = resources
            .get_mut(id)
            .ok_or_else(|| anyhow!("Resource not found: {}", id))?;

        // Get mutable reference to the Arc's data
        let resource = Arc::get_mut(resource)
            .ok_or_else(|| anyhow!("Resource has multiple references"))?;

        // Validate version
        resource.validate_version(expected_version)?;

        // Apply modification
        f(resource)?;

        // Increment version
        resource.increment_version();

        Ok(())
    }

    /// Acquire per-resource operation lock
    fn acquire_operation_lock(&self, id: &str) -> Arc<Mutex<()>> {
        let mut locks = self.operation_locks.lock();
        locks
            .entry(id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Release operation lock (cleanup)
    pub fn release_operation_lock(&self, id: &str) {
        let mut locks = self.operation_locks.lock();
        if let Some(lock) = locks.get(id) {
            // Only remove if no one else is holding it
            if Arc::strong_count(lock) == 1 {
                locks.remove(id);
            }
        }
    }

    /// Get statistics (lock-free reads)
    pub fn stats(&self) -> RegistryStats {
        RegistryStats {
            access_count: self.access_count.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            resource_count: self.resources.read().len(),
        }
    }
}

pub struct RegistryStats {
    pub access_count: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub resource_count: usize,
}

impl RegistryStats {
    pub fn hit_rate(&self) -> f64 {
        if self.access_count == 0 {
            0.0
        } else {
            self.cache_hits as f64 / self.access_count as f64
        }
    }
}

// =============================================================================
// Resource Limits with Semaphores
// =============================================================================

/// Process pool with semaphore-based limiting
pub struct ProcessPool {
    semaphore: Arc<Semaphore>,
    max_processes: usize,
    active_processes: AtomicU64,
}

impl ProcessPool {
    pub fn new(max_processes: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_processes)),
            max_processes,
            active_processes: AtomicU64::new(0),
        }
    }

    /// Acquire process slot - blocks if pool is full
    pub async fn acquire(&self) -> Result<ProcessPermit> {
        let permit = self.semaphore.clone().acquire_owned().await?;
        self.active_processes.fetch_add(1, Ordering::Relaxed);

        Ok(ProcessPermit {
            _permit: permit,
            pool: self,
        })
    }

    pub fn active_count(&self) -> u64 {
        self.active_processes.load(Ordering::Relaxed)
    }

    pub fn max_count(&self) -> usize {
        self.max_processes
    }
}

/// RAII guard for process permit
pub struct ProcessPermit<'a> {
    _permit: tokio::sync::OwnedSemaphorePermit,
    pool: &'a ProcessPool,
}

impl<'a> Drop for ProcessPermit<'a> {
    fn drop(&mut self) {
        self.pool.active_processes.fetch_sub(1, Ordering::Relaxed);
        println!("Released process permit");
    }
}

// =============================================================================
// Lazy Initialization Pattern
// =============================================================================

/// Cache entry with lazy computation
pub struct CacheEntry<T> {
    value: RwLock<Option<T>>,
}

impl<T> CacheEntry<T> {
    pub fn new() -> Self {
        Self {
            value: RwLock::new(None),
        }
    }

    /// Get cached value, computing if needed
    pub fn get_or_compute<F>(&self, compute_fn: F) -> T
    where
        F: FnOnce() -> T,
        T: Clone,
    {
        // Fast path: check with read lock
        {
            let value = self.value.read();
            if let Some(ref v) = *value {
                return v.clone();
            }
        }

        // Slow path: compute with write lock
        let mut value = self.value.write();
        if value.is_none() {
            *value = Some(compute_fn());
        }
        value.as_ref().unwrap().clone()
    }

    pub fn has_value(&self) -> bool {
        self.value.read().is_some()
    }

    pub fn clear(&self) {
        *self.value.write() = None;
    }
}

// =============================================================================
// Memory Budget with Cleanup
// =============================================================================

/// Bounded collection with automatic cleanup
pub struct BoundedCollection<T> {
    items: Vec<T>,
    max_count: usize,
    max_size_bytes: u64,
    current_size_bytes: u64,
}

impl<T> BoundedCollection<T> {
    pub fn new(max_count: usize, max_size_bytes: u64) -> Self {
        Self {
            items: Vec::with_capacity(max_count),
            max_count,
            max_size_bytes,
            current_size_bytes: 0,
        }
    }

    /// Add item, enforcing limits
    pub fn add(&mut self, item: T, item_size: u64) -> Result<()> {
        // Enforce count limit
        while self.items.len() >= self.max_count {
            if let Some(removed) = self.items.first() {
                let removed_size = std::mem::size_of_val(removed) as u64;
                self.current_size_bytes = self.current_size_bytes.saturating_sub(removed_size);
            }
            self.items.remove(0);
        }

        // Enforce size limit
        while self.current_size_bytes + item_size > self.max_size_bytes && !self.items.is_empty() {
            if let Some(removed) = self.items.first() {
                let removed_size = std::mem::size_of_val(removed) as u64;
                self.current_size_bytes = self.current_size_bytes.saturating_sub(removed_size);
            }
            self.items.remove(0);
        }

        // Add new item
        self.items.push(item);
        self.current_size_bytes += item_size;

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn size_bytes(&self) -> u64 {
        self.current_size_bytes
    }
}

// =============================================================================
// Example Usage
// =============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Resource Management Patterns Demo ===\n");

    // Demo 1: RAII Guards
    demo_raii_guards().await?;

    // Demo 2: Shared State
    demo_shared_state().await?;

    // Demo 3: Resource Limits
    demo_resource_limits().await?;

    // Demo 4: Lazy Initialization
    demo_lazy_init().await?;

    // Demo 5: Bounded Collections
    demo_bounded_collections().await?;

    Ok(())
}

async fn demo_raii_guards() -> Result<()> {
    println!("--- Demo 1: RAII Guards ---");

    // Temp file guard
    {
        let temp_path = PathBuf::from("/tmp/test_file.txt");
        fs::write(&temp_path, "test data")?;

        let guard = TempFileGuard::new(temp_path.clone());
        println!("Created temp file: {:?}", guard.path());

        // File will be deleted when guard drops
    }
    println!("Guard dropped - temp file cleaned up\n");

    // Transaction guard with rollback
    {
        let mut data = vec![1, 2, 3];
        let backup = data.clone();

        let guard = TransactionGuard::new("txn-001".to_string(), || {
            data.clone_from(&backup); // Rollback
        });

        data.push(4);
        data.push(5);

        // Simulate error - don't commit
        println!("Simulating error - transaction will rollback");
        drop(guard);
        // Rollback function called
    }

    // Transaction guard with commit
    {
        let mut data = vec![1, 2, 3];
        let backup = data.clone();

        let guard = TransactionGuard::new("txn-002".to_string(), || {
            data.clone_from(&backup);
        });

        data.push(4);
        data.push(5);

        guard.commit(); // Success - no rollback
        println!("Transaction committed successfully\n");
    }

    Ok(())
}

async fn demo_shared_state() -> Result<()> {
    println!("--- Demo 2: Shared State with RwLock ---");

    let registry = Arc::new(ResourceRegistry::new());

    // Insert resources
    registry.insert(Resource::new("res-1".to_string(), "data-1".to_string()));
    registry.insert(Resource::new("res-2".to_string(), "data-2".to_string()));
    registry.insert(Resource::new("res-3".to_string(), "data-3".to_string()));

    // Read operations (concurrent)
    let tasks: Vec<_> = (0..5)
        .map(|i| {
            let registry = registry.clone();
            tokio::spawn(async move {
                if let Some(resource) = registry.get("res-1") {
                    println!("Task {} read resource: {}", i, resource.id);
                }
            })
        })
        .collect();

    for task in tasks {
        task.await?;
    }

    // Modify with version checking
    if let Some(resource) = registry.get("res-1") {
        let version = resource.version();
        println!("Current version: {}", version);

        registry.modify_versioned("res-1", version, |r| {
            r.data = "modified-data".to_string();
            Ok(())
        })?;

        println!("Modified resource, new version: {}", resource.version());
    }

    // Print statistics
    let stats = registry.stats();
    println!(
        "Stats: {} accesses, {} hits, {} misses, hit rate: {:.2}%\n",
        stats.access_count,
        stats.cache_hits,
        stats.cache_misses,
        stats.hit_rate() * 100.0
    );

    Ok(())
}

async fn demo_resource_limits() -> Result<()> {
    println!("--- Demo 3: Resource Limits with Semaphore ---");

    let pool = Arc::new(ProcessPool::new(3));
    println!("Created process pool with max 3 processes");

    let mut handles = vec![];

    // Spawn 6 tasks - only 3 will run concurrently
    for i in 0..6 {
        let pool = pool.clone();
        let handle = tokio::spawn(async move {
            println!("Task {} waiting for permit...", i);
            let _permit = pool.acquire().await.unwrap();
            println!(
                "Task {} acquired permit (active: {})",
                i,
                pool.active_count()
            );

            // Simulate work
            tokio::time::sleep(Duration::from_millis(100)).await;

            println!("Task {} releasing permit", i);
            // Permit automatically released on drop
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await?;
    }

    println!("All tasks completed\n");
    Ok(())
}

async fn demo_lazy_init() -> Result<()> {
    println!("--- Demo 4: Lazy Initialization ---");

    let cache = CacheEntry::<String>::new();

    println!("Has value: {}", cache.has_value());

    // First access - computes value
    let value = cache.get_or_compute(|| {
        println!("Computing expensive value...");
        "computed-result".to_string()
    });
    println!("Got value: {}", value);

    // Second access - returns cached value
    let value2 = cache.get_or_compute(|| {
        println!("This won't be called!");
        "won't-compute".to_string()
    });
    println!("Got cached value: {}", value2);

    println!("Has value: {}\n", cache.has_value());

    Ok(())
}

async fn demo_bounded_collections() -> Result<()> {
    println!("--- Demo 5: Bounded Collections ---");

    let mut collection = BoundedCollection::<String>::new(
        5,                  // Max 5 items
        1024 * 1024 * 10,   // Max 10MB
    );

    println!("Created bounded collection: max 5 items, 10MB");

    // Add items
    for i in 0..8 {
        let item = format!("item-{}", i);
        let size = item.len() as u64;
        collection.add(item, size)?;
        println!(
            "Added item-{}: {} items, {} bytes",
            i,
            collection.len(),
            collection.size_bytes()
        );
    }

    println!("Collection enforced limits - oldest items removed\n");

    Ok(())
}

// =============================================================================
// Additional Patterns
// =============================================================================

/// Example: Cleanup ordering
pub struct ComplexResource {
    id: String,
    temp_files: Vec<PathBuf>,
    work_dir: PathBuf,
}

impl Drop for ComplexResource {
    fn drop(&mut self) {
        println!("Cleaning up complex resource: {}", self.id);

        // 1. Clean up temp files first
        for file in &self.temp_files {
            let _ = fs::remove_file(file);
        }

        // 2. Clean up work directory last
        if self.work_dir.exists() {
            let _ = fs::remove_dir_all(&self.work_dir);
        }

        println!("Cleanup complete for: {}", self.id);
    }
}

/// Example: Graceful shutdown
pub async fn graceful_shutdown(registry: Arc<ResourceRegistry>) -> Result<()> {
    println!("Starting graceful shutdown...");

    // 1. Stop accepting new requests (not shown)

    // 2. Wait for active operations to complete (not shown)

    // 3. Clean up resources
    {
        let resources = registry.resources.write();
        let count = resources.len();
        println!("Cleaning up {} resources", count);
        // Resources will be dropped when write lock is released
    }

    println!("Graceful shutdown complete");
    Ok(())
}
