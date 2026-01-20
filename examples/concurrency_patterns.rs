// Concurrency Patterns for Rust MCP Servers
// This file demonstrates various concurrency patterns applicable to MCP servers
// Based on analysis of ggen-mcp implementation
//
// Run examples with: cargo run --example concurrency_patterns

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::{Mutex, RwLock};
use tokio::sync::{mpsc, Semaphore};
use tokio::time;

// ============================================================================
// Example 1: Semaphore-Based WIP Limits
// ============================================================================

/// Demonstrates limiting concurrent operations using semaphores
/// Similar to GlobalRecalcLock and GlobalScreenshotLock in ggen-mcp
pub struct WipLimitedExecutor {
    semaphore: Arc<Semaphore>,
    active_count: AtomicUsize,
}

impl WipLimitedExecutor {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            active_count: AtomicUsize::new(0),
        }
    }

    /// Execute an operation with WIP limit enforcement
    pub async fn execute<F, T>(&self, operation: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        // Acquire permit (blocks if limit reached)
        let _permit = self.semaphore.acquire().await.unwrap();

        self.active_count.fetch_add(1, Ordering::Relaxed);
        println!(
            "[WIP] Active operations: {}",
            self.active_count.load(Ordering::Relaxed)
        );

        let result = operation.await;

        self.active_count.fetch_sub(1, Ordering::Relaxed);
        result
    }

    pub fn active_count(&self) -> usize {
        self.active_count.load(Ordering::Relaxed)
    }
}

// ============================================================================
// Example 2: RwLock-Based Cache
// ============================================================================

/// Demonstrates read-heavy cache pattern used in AppState
pub struct ReadHeavyCache<K, V> {
    data: RwLock<HashMap<K, Arc<V>>>,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl<K, V> ReadHeavyCache<K, V>
where
    K: Eq + std::hash::Hash + Clone,
{
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Get value from cache (concurrent reads possible)
    pub fn get(&self, key: &K) -> Option<Arc<V>> {
        let data = self.data.read();
        if let Some(value) = data.get(key) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(value.clone())
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Insert value into cache (exclusive access required)
    pub fn insert(&self, key: K, value: V) {
        let mut data = self.data.write();
        data.insert(key, Arc::new(value));
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        CacheStats {
            hits,
            misses,
            hit_rate: if hits + misses > 0 {
                hits as f64 / (hits + misses) as f64
            } else {
                0.0
            },
            size: self.data.read().len(),
        }
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub size: usize,
}

// ============================================================================
// Example 3: Task-Based Concurrency with spawn_blocking
// ============================================================================

/// Demonstrates offloading blocking I/O to thread pool
pub struct BlockingIoHandler;

impl BlockingIoHandler {
    /// Simulate CPU-intensive operation (like workbook parsing)
    pub async fn load_workbook(&self, workbook_id: String) -> String {
        println!("[Blocking] Starting load for {}", workbook_id);

        // Offload to blocking thread pool
        let result = tokio::task::spawn_blocking(move || {
            // Simulate expensive parsing
            std::thread::sleep(Duration::from_millis(100));
            format!("Workbook({}) contents", workbook_id)
        })
        .await
        .unwrap();

        println!("[Blocking] Completed load");
        result
    }
}

// ============================================================================
// Example 4: Per-Resource Fine-Grained Locking
// ============================================================================

/// Demonstrates per-fork locking pattern from ForkRegistry
pub struct PerResourceLockRegistry {
    /// Global registry of all resources
    resources: RwLock<HashMap<String, ResourceState>>,

    /// Per-resource locks for exclusive operations
    operation_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

struct ResourceState {
    id: String,
    version: AtomicU64,
}

impl PerResourceLockRegistry {
    pub fn new() -> Self {
        Self {
            resources: RwLock::new(HashMap::new()),
            operation_locks: Mutex::new(HashMap::new()),
        }
    }

    /// Get a resource (concurrent reads)
    pub fn get(&self, id: &str) -> Option<u64> {
        let resources = self.resources.read();
        resources
            .get(id)
            .map(|r| r.version.load(Ordering::SeqCst))
    }

    /// Acquire per-resource lock for exclusive operation
    pub fn acquire_operation_lock(&self, resource_id: &str) -> Arc<Mutex<()>> {
        let mut locks = self.operation_locks.lock();
        locks
            .entry(resource_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Perform exclusive operation on a resource
    pub async fn exclusive_operation(&self, resource_id: String) {
        // Acquire per-resource lock (not global lock)
        let lock = self.acquire_operation_lock(&resource_id);
        let _guard = lock.lock();

        println!("[PerResource] Operating on {}", resource_id);
        time::sleep(Duration::from_millis(50)).await;
        println!("[PerResource] Completed operation on {}", resource_id);
    }
}

// ============================================================================
// Example 5: RAII Transaction Guards
// ============================================================================

/// Demonstrates RAII pattern for rollback on error (like ForkCreationGuard)
pub struct TransactionGuard<T> {
    resource: Option<T>,
    rollback: Box<dyn FnOnce(&T) + Send>,
    committed: bool,
}

impl<T> TransactionGuard<T> {
    pub fn new(resource: T, rollback: impl FnOnce(&T) + Send + 'static) -> Self {
        Self {
            resource: Some(resource),
            rollback: Box::new(rollback),
            committed: false,
        }
    }

    pub fn commit(mut self) -> T {
        self.committed = true;
        self.resource.take().unwrap()
    }
}

impl<T> Drop for TransactionGuard<T> {
    fn drop(&mut self) {
        if !self.committed {
            if let Some(resource) = &self.resource {
                println!("[RAII] Rolling back transaction");
                (self.rollback)(resource);
            }
        }
    }
}

// ============================================================================
// Example 6: Adaptive Semaphore for Dynamic Concurrency
// ============================================================================

/// Demonstrates adaptive concurrency control based on system load
pub struct AdaptiveConcurrencyLimiter {
    semaphore: Arc<Semaphore>,
    current_permits: AtomicUsize,
    max_permits: usize,
    min_permits: usize,
    latency_samples: Mutex<VecDeque<Duration>>,
}

impl AdaptiveConcurrencyLimiter {
    pub fn new(min: usize, max: usize) -> Self {
        let initial = (min + max) / 2;
        Self {
            semaphore: Arc::new(Semaphore::new(initial)),
            current_permits: AtomicUsize::new(initial),
            max_permits: max,
            min_permits: min,
            latency_samples: Mutex::new(VecDeque::with_capacity(100)),
        }
    }

    pub async fn execute<F, T>(&self, operation: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        let _permit = self.semaphore.acquire().await.unwrap();

        let start = Instant::now();
        let result = operation.await;
        let latency = start.elapsed();

        self.record_latency(latency);
        self.adjust_concurrency();

        result
    }

    fn record_latency(&self, latency: Duration) {
        let mut samples = self.latency_samples.lock();
        samples.push_back(latency);
        if samples.len() > 100 {
            samples.pop_front();
        }
    }

    fn adjust_concurrency(&self) {
        let samples = self.latency_samples.lock();
        if samples.len() < 20 {
            return; // Not enough data
        }

        let p95_latency = self.calculate_p95(&samples);
        let current = self.current_permits.load(Ordering::Relaxed);

        if p95_latency > Duration::from_millis(500) && current > self.min_permits {
            // High latency, reduce concurrency
            println!(
                "[Adaptive] P95 latency {}ms, reducing permits from {} to {}",
                p95_latency.as_millis(),
                current,
                current - 1
            );
            self.current_permits.fetch_sub(1, Ordering::Relaxed);
            // Note: In production, would need to close and recreate semaphore
        } else if p95_latency < Duration::from_millis(100) && current < self.max_permits {
            // Low latency, increase concurrency
            println!(
                "[Adaptive] P95 latency {}ms, increasing permits from {} to {}",
                p95_latency.as_millis(),
                current,
                current + 1
            );
            self.semaphore.add_permits(1);
            self.current_permits.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn calculate_p95(&self, samples: &VecDeque<Duration>) -> Duration {
        let mut sorted: Vec<_> = samples.iter().cloned().collect();
        sorted.sort();
        let idx = (sorted.len() as f64 * 0.95) as usize;
        sorted.get(idx).cloned().unwrap_or(Duration::ZERO)
    }
}

// ============================================================================
// Example 7: Heijunka Level Loading Pattern
// ============================================================================

/// Demonstrates production leveling (Heijunka) for smooth flow
pub struct LeveledScheduler {
    buffer: Arc<Mutex<VecDeque<String>>>,
    target_rate: usize,
    interval: Duration,
}

impl LeveledScheduler {
    pub fn new(target_rate: usize, interval: Duration) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            target_rate,
            interval,
        }
    }

    /// Add request to buffer (smooths incoming bursts)
    pub async fn enqueue(&self, request: String) {
        let mut buffer = self.buffer.lock();
        buffer.push_back(request);
        println!(
            "[Heijunka] Buffered request, queue depth: {}",
            buffer.len()
        );
    }

    /// Process requests at steady rate (leveled production)
    pub async fn run_processor(self: Arc<Self>) {
        let mut ticker = time::interval(self.interval);

        loop {
            ticker.tick().await;

            let batch = {
                let mut buffer = self.buffer.lock();
                let batch_size = self.target_rate.min(buffer.len());
                buffer.drain(..batch_size).collect::<Vec<_>>()
            };

            if !batch.is_empty() {
                println!(
                    "[Heijunka] Processing batch of {} requests at steady rate",
                    batch.len()
                );
                for request in batch {
                    self.process_request(request).await;
                }
            }
        }
    }

    async fn process_request(&self, request: String) {
        println!("[Heijunka] Processing: {}", request);
        time::sleep(Duration::from_millis(10)).await;
    }
}

// ============================================================================
// Example 8: Circuit Breaker Pattern
// ============================================================================

/// Demonstrates circuit breaker for fault tolerance
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    state: Arc<Mutex<CircuitState>>,
    failure_count: Arc<AtomicUsize>,
    failure_threshold: usize,
    timeout: Duration,
    last_failure: Arc<Mutex<Option<Instant>>>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: usize, timeout: Duration) -> Self {
        Self {
            state: Arc::new(Mutex::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicUsize::new(0)),
            failure_threshold,
            timeout,
            last_failure: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        let state = *self.state.lock();

        match state {
            CircuitState::Open => {
                // Check if timeout has elapsed
                let last_failure = self.last_failure.lock();
                if let Some(last) = *last_failure {
                    if last.elapsed() > self.timeout {
                        drop(last_failure);
                        *self.state.lock() = CircuitState::HalfOpen;
                        println!("[CircuitBreaker] Moving to HalfOpen state");
                    } else {
                        println!("[CircuitBreaker] Circuit is OPEN, rejecting request");
                        return Err(operation.await.unwrap_err());
                    }
                }
            }
            CircuitState::HalfOpen => {
                println!("[CircuitBreaker] Testing in HalfOpen state");
            }
            CircuitState::Closed => {
                // Normal operation
            }
        }

        match operation.await {
            Ok(result) => {
                self.on_success();
                Ok(result)
            }
            Err(e) => {
                self.on_failure();
                Err(e)
            }
        }
    }

    fn on_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        let mut state = self.state.lock();
        if *state == CircuitState::HalfOpen {
            println!("[CircuitBreaker] Success in HalfOpen, moving to Closed");
            *state = CircuitState::Closed;
        }
    }

    fn on_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        *self.last_failure.lock() = Some(Instant::now());

        if failures >= self.failure_threshold {
            let mut state = self.state.lock();
            *state = CircuitState::Open;
            println!(
                "[CircuitBreaker] Threshold reached ({}), moving to OPEN",
                failures
            );
        }
    }

    pub fn state(&self) -> CircuitState {
        *self.state.lock()
    }
}

// ============================================================================
// Example 9: Sharded Cache for Reduced Contention
// ============================================================================

/// Demonstrates sharding to reduce lock contention
pub struct ShardedCache<K, V> {
    shards: Vec<RwLock<HashMap<K, V>>>,
    shard_count: usize,
}

impl<K, V> ShardedCache<K, V>
where
    K: Eq + std::hash::Hash + Clone,
{
    pub fn new(shard_count: usize) -> Self {
        let shards = (0..shard_count)
            .map(|_| RwLock::new(HashMap::new()))
            .collect();

        Self {
            shards,
            shard_count,
        }
    }

    fn shard_index(&self, key: &K) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() as usize) % self.shard_count
    }

    pub fn get(&self, key: &K) -> Option<V>
    where
        V: Clone,
    {
        let shard_idx = self.shard_index(key);
        let shard = self.shards[shard_idx].read();
        shard.get(key).cloned()
    }

    pub fn insert(&self, key: K, value: V) {
        let shard_idx = self.shard_index(&key);
        let mut shard = self.shards[shard_idx].write();
        shard.insert(key, value);
    }
}

// ============================================================================
// Main: Demonstration of Patterns
// ============================================================================

#[tokio::main]
async fn main() {
    println!("=== Rust MCP Server Concurrency Patterns ===\n");

    // Example 1: WIP Limits
    println!("\n--- Example 1: WIP Limits with Semaphores ---");
    demo_wip_limits().await;

    // Example 2: Read-Heavy Cache
    println!("\n--- Example 2: Read-Heavy Cache with RwLock ---");
    demo_read_heavy_cache().await;

    // Example 3: Blocking I/O Offload
    println!("\n--- Example 3: Offloading Blocking I/O ---");
    demo_blocking_io().await;

    // Example 4: Per-Resource Locking
    println!("\n--- Example 4: Per-Resource Fine-Grained Locking ---");
    demo_per_resource_locks().await;

    // Example 5: RAII Transaction Guards
    println!("\n--- Example 5: RAII Transaction Guards ---");
    demo_transaction_guards().await;

    // Example 6: Adaptive Concurrency
    println!("\n--- Example 6: Adaptive Concurrency Control ---");
    demo_adaptive_concurrency().await;

    // Example 7: Heijunka Level Loading
    println!("\n--- Example 7: Heijunka Level Loading ---");
    demo_heijunka().await;

    // Example 8: Circuit Breaker
    println!("\n--- Example 8: Circuit Breaker ---");
    demo_circuit_breaker().await;

    // Example 9: Sharded Cache
    println!("\n--- Example 9: Sharded Cache ---");
    demo_sharded_cache().await;

    println!("\n=== All Examples Completed ===");
}

async fn demo_wip_limits() {
    let executor = Arc::new(WipLimitedExecutor::new(2));

    let tasks: Vec<_> = (0..5)
        .map(|i| {
            let executor = executor.clone();
            tokio::spawn(async move {
                executor
                    .execute(async move {
                        println!("Task {} executing", i);
                        time::sleep(Duration::from_millis(100)).await;
                        println!("Task {} completed", i);
                    })
                    .await
            })
        })
        .collect();

    for task in tasks {
        task.await.unwrap();
    }
}

async fn demo_read_heavy_cache() {
    let cache = Arc::new(ReadHeavyCache::new());

    // Populate cache
    cache.insert("key1".to_string(), "value1".to_string());
    cache.insert("key2".to_string(), "value2".to_string());

    // Concurrent reads
    let tasks: Vec<_> = (0..10)
        .map(|i| {
            let cache = cache.clone();
            tokio::spawn(async move {
                let key = format!("key{}", (i % 2) + 1);
                let value = cache.get(&key);
                println!("Read {}: {:?}", key, value);
            })
        })
        .collect();

    for task in tasks {
        task.await.unwrap();
    }

    let stats = cache.stats();
    println!("Cache stats: {:?}", stats);
}

async fn demo_blocking_io() {
    let handler = BlockingIoHandler;

    let tasks: Vec<_> = (0..3)
        .map(|i| {
            let handler_clone = handler.clone();
            tokio::spawn(async move {
                let workbook_id = format!("workbook-{}", i);
                handler_clone.load_workbook(workbook_id).await
            })
        })
        .collect();

    for task in tasks {
        let result = task.await.unwrap();
        println!("Loaded: {}", result);
    }
}

impl Clone for BlockingIoHandler {
    fn clone(&self) -> Self {
        Self
    }
}

async fn demo_per_resource_locks() {
    let registry = Arc::new(PerResourceLockRegistry::new());

    let tasks: Vec<_> = vec!["resource-1", "resource-2", "resource-1"]
        .into_iter()
        .map(|id| {
            let registry = registry.clone();
            let id = id.to_string();
            tokio::spawn(async move { registry.exclusive_operation(id).await })
        })
        .collect();

    for task in tasks {
        task.await.unwrap();
    }
}

async fn demo_transaction_guards() {
    struct Resource {
        name: String,
    }

    let resource = Resource {
        name: "test-resource".to_string(),
    };

    // Simulate transaction that fails
    {
        let guard = TransactionGuard::new(resource, |r| {
            println!("Rollback cleanup for {}", r.name);
        });

        // Simulate error - guard will rollback on drop
        println!("Simulating error...");
    }

    println!("Transaction guard dropped, rollback executed");
}

async fn demo_adaptive_concurrency() {
    let limiter = Arc::new(AdaptiveConcurrencyLimiter::new(1, 5));

    // Simulate varying latencies
    for i in 0..20 {
        let limiter = limiter.clone();
        tokio::spawn(async move {
            limiter
                .execute(async move {
                    let latency = if i % 5 == 0 { 600 } else { 50 };
                    time::sleep(Duration::from_millis(latency)).await;
                })
                .await
        })
        .await
        .unwrap();

        time::sleep(Duration::from_millis(100)).await;
    }
}

async fn demo_heijunka() {
    let scheduler = Arc::new(LeveledScheduler::new(2, Duration::from_millis(100)));

    // Start processor
    let scheduler_clone = scheduler.clone();
    tokio::spawn(async move { scheduler_clone.run_processor().await });

    // Enqueue burst of requests
    for i in 0..10 {
        scheduler.enqueue(format!("request-{}", i)).await;
        if i == 4 {
            time::sleep(Duration::from_millis(50)).await;
        }
    }

    // Let processor run
    time::sleep(Duration::from_millis(600)).await;
}

async fn demo_circuit_breaker() {
    let breaker = Arc::new(CircuitBreaker::new(3, Duration::from_millis(200)));

    // Simulate failures
    for i in 0..10 {
        let breaker = breaker.clone();
        let result: Result<(), String> = breaker
            .call(async move {
                if i < 5 {
                    Err(format!("Simulated failure {}", i))
                } else {
                    Ok(())
                }
            })
            .await;

        match result {
            Ok(_) => println!("Request {} succeeded", i),
            Err(e) => println!("Request {} failed: {}", i, e),
        }

        time::sleep(Duration::from_millis(100)).await;
    }

    println!("Final circuit state: {:?}", breaker.state());
}

async fn demo_sharded_cache() {
    let cache = Arc::new(ShardedCache::new(4));

    // Concurrent inserts
    let insert_tasks: Vec<_> = (0..100)
        .map(|i| {
            let cache = cache.clone();
            tokio::spawn(async move {
                cache.insert(format!("key-{}", i), format!("value-{}", i));
            })
        })
        .collect();

    for task in insert_tasks {
        task.await.unwrap();
    }

    // Concurrent reads
    let read_tasks: Vec<_> = (0..100)
        .map(|i| {
            let cache = cache.clone();
            tokio::spawn(async move {
                let _value = cache.get(&format!("key-{}", i));
            })
        })
        .collect();

    for task in read_tasks {
        task.await.unwrap();
    }

    println!("Sharded cache test completed with reduced contention");
}
