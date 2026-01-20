# Heijunka (Production Leveling) for MCP Servers

## Executive Summary

This document applies **Heijunka** (平準化, "production leveling") principles from the Toyota Production System to MCP (Model Context Protocol) servers. Heijunka focuses on smoothing production flows to eliminate waste, reduce variability, and optimize resource utilization. In the context of MCP servers, this translates to leveling request loads, managing concurrency, and preventing resource exhaustion.

**Key Findings from ggen-mcp Analysis:**
- ✅ **Existing Controls**: Semaphore-based concurrency limiting (recalc operations)
- ✅ **Resource Pooling**: LRU cache for workbooks with configurable capacity
- ✅ **Timeout Management**: Configurable tool timeouts prevent resource starvation
- ⚠️ **Missing**: Request queuing, rate limiting, backpressure signaling
- ⚠️ **Missing**: Load balancing across different request types
- ⚠️ **Missing**: Batch size optimization and takt time management

---

## Table of Contents

1. [Heijunka Principles Overview](#heijunka-principles-overview)
2. [Current Implementation Analysis](#current-implementation-analysis)
3. [Request Rate Limiting](#request-rate-limiting)
4. [Queue Management](#queue-management)
5. [Resource Pooling](#resource-pooling)
6. [Batch Size Optimization](#batch-size-optimization)
7. [Backpressure Mechanisms](#backpressure-mechanisms)
8. [Fair Scheduling Algorithms](#fair-scheduling-algorithms)
9. [Capacity Planning](#capacity-planning)
10. [Implementation Recommendations](#implementation-recommendations)
11. [References](#references)

---

## Heijunka Principles Overview

### What is Heijunka?

Heijunka is a core Toyota Production System principle that **levels production volume and variety** over time to reduce:
- **Mura** (unevenness/variability)
- **Muri** (overburden/strain)
- **Muda** (waste)

In traditional manufacturing, Heijunka smooths customer demand by producing different products in a mixed sequence at steady rates, rather than batch-processing large orders.

### Heijunka Applied to MCP Servers

| Manufacturing Concept | MCP Server Equivalent |
|----------------------|----------------------|
| **Product Variety** | Different tool types (read_table, recalculate, screenshot_sheet) |
| **Production Rate** | Request throughput (requests/second) |
| **Batch Size** | Number of requests processed together |
| **Work-in-Progress (WIP)** | Active concurrent operations |
| **Takt Time** | Target time between request completions |
| **Pull System** | Backpressure signaling when capacity reached |
| **Kanban** | Request queue with size limits |

### Core Principles

1. **Smooth Flow**: Eliminate spikes and valleys in request load
2. **Predictable Capacity**: Maintain consistent processing capability
3. **Balanced Mix**: Distribute expensive and cheap operations evenly
4. **Right-Sized Batches**: Balance latency vs throughput
5. **Visual Management**: Make system load and capacity visible

---

## Current Implementation Analysis

### Codebase Review: ggen-mcp Spreadsheet MCP Server

#### 1. Concurrency Controls

**File:** `src/state.rs`

**Current Implementation:**
```rust
pub struct AppState {
    cache: RwLock<LruCache<WorkbookId, Arc<WorkbookContext>>>,
    index: RwLock<HashMap<WorkbookId, PathBuf>>,
    alias_index: RwLock<HashMap<String, WorkbookId>>,
    #[cfg(feature = "recalc")]
    recalc_semaphore: Option<GlobalRecalcLock>,  // Semaphore-based concurrency limit
    #[cfg(feature = "recalc")]
    screenshot_semaphore: Option<GlobalScreenshotLock>,
}
```

**Analysis:**
- ✅ **RwLock** for cache allows multiple concurrent readers
- ✅ **Semaphore** limits concurrent recalculation operations
- ✅ **Atomic counters** for cache statistics (lock-free monitoring)
- ⚠️ **No global request rate limiting** - relies on client-side throttling
- ⚠️ **No fair scheduling** - requests processed in arrival order

**From `src/recalc/mod.rs`:**
```rust
pub struct GlobalRecalcLock(pub Arc<Semaphore>);

impl GlobalRecalcLock {
    pub fn new(permits: usize) -> Self {
        Self(Arc::new(Semaphore::new(permits)))
    }
}
```

**Configuration:**
- Default: `max_concurrent_recalcs = 2`
- Range: 1-100 concurrent operations
- Prevents resource exhaustion from LibreOffice processes

#### 2. Request Handling Patterns

**File:** `src/server.rs`

**Current Implementation:**
```rust
async fn run_tool_with_timeout<T, F>(&self, tool: &str, fut: F) -> Result<T>
where
    F: Future<Output = Result<T>>,
    T: Serialize,
{
    let result = if let Some(timeout_duration) = self.state.config().tool_timeout() {
        match tokio::time::timeout(timeout_duration, fut).await {
            Ok(result) => result,
            Err(_) => Err(anyhow!("tool '{}' timed out after {}ms", tool, timeout_duration.as_millis())),
        }
    } else {
        fut.await
    }?;

    self.ensure_response_size(tool, &result)?;
    Ok(result)
}
```

**Analysis:**
- ✅ **Per-request timeouts** prevent indefinite blocking (default: 30s, max: 10 minutes)
- ✅ **Response size limits** prevent memory exhaustion (default: 1MB, max: 100MB)
- ⚠️ **No request queueing** - relies on TCP backlog and OS-level buffering
- ⚠️ **No admission control** - all requests accepted up to TCP limits

#### 3. Resource Contention Points

**Identified Bottlenecks:**

1. **Workbook Cache Eviction**
   - LRU cache with fixed capacity (default: 5, max: 1000)
   - Write lock required for eviction (blocks all readers during eviction)
   - **Impact**: Cache thrashing under high concurrency

2. **Recalculation Operations**
   - Spawns LibreOffice process per operation
   - Limited by semaphore (default: 2 concurrent)
   - **Impact**: Queue builds up during recalc-heavy workloads

3. **Fork Registry Operations**
   - RwLock protects fork metadata
   - Per-fork locks prevent concurrent recalculation
   - **Impact**: Uneven load if multiple forks being modified

4. **File I/O Operations**
   - Workbook loading done in `spawn_blocking` (blocking thread pool)
   - No I/O rate limiting
   - **Impact**: Thread pool exhaustion under high load

#### 4. Uneven Load Patterns

**Observed Scenarios:**

| Request Pattern | Impact | Current Mitigation |
|----------------|--------|-------------------|
| **Burst of recalculations** | Semaphore queue builds up | Timeout prevents indefinite blocking |
| **Cache miss storm** | Concurrent workbook loads exhaust blocking pool | LRU eviction + capacity limits |
| **Mixed read/write** | Write locks block readers | RwLock allows concurrent reads |
| **Large response payloads** | Memory pressure | Response size limits |

**Missing Mitigations:**
- No request prioritization (all requests equal)
- No adaptive timeout based on system load
- No graceful degradation under overload
- No request shedding when capacity exceeded

#### 5. Opportunities for Leveling

**High-Priority Improvements:**

1. **Request Queue with Fair Scheduling**
   - Bounded queue prevents unbounded memory growth
   - Priority queues for read vs write operations
   - Weighted fair queuing for different tool types

2. **Adaptive Rate Limiting**
   - Token bucket for global request rate
   - Per-client rate limiting (if client identification available)
   - Exponential backoff signaling to clients

3. **Batch Processing for Similar Operations**
   - Group similar operations (e.g., multiple read_table requests)
   - Amortize setup costs (workbook loading, cache warming)

4. **Predictive Resource Allocation**
   - Pre-warm cache for frequently accessed workbooks
   - Reserve capacity for high-priority operations

5. **Load Shedding**
   - Reject new requests when queue is full
   - Return HTTP 503 Service Unavailable with Retry-After header
   - Health check endpoint for load balancer integration

---

## Request Rate Limiting

### Overview

Rate limiting prevents **Muri** (overburden) by controlling the flow of requests into the system. Instead of accepting all requests and failing under load, we **level the load** to match system capacity.

### Rate Limiting Strategies

#### 1. Token Bucket Algorithm

**Concept:**
- Tokens added to bucket at steady rate (refill rate)
- Each request consumes one token
- If bucket empty, request is rejected or queued

**Configuration:**
```rust
pub struct RateLimitConfig {
    /// Maximum tokens in bucket (burst capacity)
    pub bucket_capacity: usize,
    /// Tokens added per second (sustained rate)
    pub refill_rate: f64,
    /// Maximum wait time for token (0 = reject immediately)
    pub max_wait_ms: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            bucket_capacity: 100,      // Allow bursts up to 100 requests
            refill_rate: 10.0,          // Sustain 10 req/sec
            max_wait_ms: 1000,          // Wait up to 1s for token
        }
    }
}
```

**Implementation Pattern:**
```rust
use tokio::sync::Semaphore;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct TokenBucket {
    tokens: Arc<Semaphore>,
    refill_rate: f64,
    last_refill: Arc<Mutex<Instant>>,
    capacity: usize,
}

impl TokenBucket {
    pub fn new(capacity: usize, refill_rate: f64) -> Self {
        Self {
            tokens: Arc::new(Semaphore::new(capacity)),
            refill_rate,
            last_refill: Arc::new(Mutex::new(Instant::now())),
            capacity,
        }
    }

    pub async fn acquire(&self, max_wait: Duration) -> Result<(), RateLimitError> {
        // Refill tokens based on elapsed time
        self.refill();

        // Try to acquire token with timeout
        match tokio::time::timeout(max_wait, self.tokens.acquire()).await {
            Ok(Ok(_permit)) => Ok(()),
            Ok(Err(_)) => Err(RateLimitError::Closed),
            Err(_) => Err(RateLimitError::Timeout),
        }
    }

    fn refill(&self) {
        let mut last = self.last_refill.lock();
        let now = Instant::now();
        let elapsed = now.duration_since(*last).as_secs_f64();

        let tokens_to_add = (elapsed * self.refill_rate) as usize;
        if tokens_to_add > 0 {
            self.tokens.add_permits(tokens_to_add.min(self.capacity));
            *last = now;
        }
    }
}
```

**Benefits:**
- Allows bursts while controlling sustained rate
- Simple to implement and reason about
- Well-tested in production systems

**Drawbacks:**
- All requests treated equally (no prioritization)
- Fixed rate doesn't adapt to system load

#### 2. Leaky Bucket Algorithm

**Concept:**
- Requests enter bucket at any rate
- Requests leave bucket at fixed rate
- If bucket full, new requests rejected

**Use Case:** More appropriate when you want **strict rate limiting** without bursts.

```rust
pub struct LeakyBucket {
    queue: Arc<Mutex<VecDeque<Instant>>>,
    capacity: usize,
    leak_rate: f64,
}

impl LeakyBucket {
    pub async fn enqueue(&self, request: Request) -> Result<(), RateLimitError> {
        let mut queue = self.queue.lock();

        // Remove old requests (leak)
        let now = Instant::now();
        while let Some(&front) = queue.front() {
            let age = now.duration_since(front).as_secs_f64();
            if age >= 1.0 / self.leak_rate {
                queue.pop_front();
            } else {
                break;
            }
        }

        // Check capacity
        if queue.len() >= self.capacity {
            return Err(RateLimitError::QueueFull);
        }

        queue.push_back(now);
        Ok(())
    }
}
```

#### 3. Adaptive Rate Limiting

**Concept:** Adjust rate limits based on system metrics (CPU, memory, queue depth).

```rust
pub struct AdaptiveRateLimiter {
    base_rate: f64,
    current_rate: Arc<Mutex<f64>>,
    metrics: Arc<SystemMetrics>,
}

impl AdaptiveRateLimiter {
    pub fn adjust_rate(&self) {
        let metrics = self.metrics.snapshot();
        let mut rate = self.current_rate.lock();

        // Reduce rate if system under stress
        if metrics.cpu_usage > 0.8 || metrics.queue_depth > 100 {
            *rate = (*rate * 0.9).max(self.base_rate * 0.1);
        }
        // Increase rate if system healthy
        else if metrics.cpu_usage < 0.5 && metrics.queue_depth < 10 {
            *rate = (*rate * 1.1).min(self.base_rate * 2.0);
        }
    }
}
```

### Per-Client Rate Limiting

**Challenge:** MCP servers typically don't have strong client identification.

**Solutions:**

1. **IP-based limiting** (if HTTP transport)
   ```rust
   pub struct PerClientLimiter {
       limiters: Arc<Mutex<HashMap<IpAddr, Arc<TokenBucket>>>>,
       default_config: RateLimitConfig,
   }
   ```

2. **Session-based limiting** (if authenticated)
   ```rust
   pub struct PerSessionLimiter {
       limiters: Arc<Mutex<HashMap<SessionId, Arc<TokenBucket>>>>,
       // ...
   }
   ```

3. **Tool-based limiting** (different limits for expensive operations)
   ```rust
   pub struct PerToolLimiter {
       read_limiter: Arc<TokenBucket>,       // High rate for reads
       write_limiter: Arc<TokenBucket>,      // Medium rate for writes
       recalc_limiter: Arc<TokenBucket>,     // Low rate for recalculations
   }
   ```

### Recommended Configuration for ggen-mcp

```yaml
rate_limits:
  global:
    bucket_capacity: 100
    refill_rate: 50.0        # 50 req/sec baseline
    max_wait_ms: 5000

  per_tool:
    # Cheap read operations
    list_workbooks:
      refill_rate: 100.0
    list_sheets:
      refill_rate: 100.0

    # Moderate operations
    read_table:
      refill_rate: 50.0
    sheet_overview:
      refill_rate: 50.0

    # Expensive operations
    recalculate:
      refill_rate: 2.0       # Already limited by semaphore
      bucket_capacity: 5
    screenshot_sheet:
      refill_rate: 1.0
      bucket_capacity: 2

    # Write operations
    edit_batch:
      refill_rate: 20.0
    transform_batch:
      refill_rate: 20.0
```

---

## Queue Management

### Overview

Queues smooth demand variability (**Mura**) and prevent overload. In Heijunka, queues have strict size limits to maintain **flow** and prevent **inventory buildup** (work-in-progress).

### Queue Design Principles

1. **Bounded Queues**: Reject when full (fail-fast)
2. **Fair Scheduling**: Prevent starvation
3. **Priority Levels**: Critical operations jump queue
4. **Visibility**: Queue depth metrics for monitoring

### Queue Types

#### 1. FIFO Queue (First-In-First-Out)

**Use Case:** Default for most operations where fairness is important.

```rust
use tokio::sync::mpsc;

pub struct FifoRequestQueue {
    sender: mpsc::Sender<Request>,
    receiver: Arc<Mutex<mpsc::Receiver<Request>>>,
    capacity: usize,
}

impl FifoRequestQueue {
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = mpsc::channel(capacity);
        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
            capacity,
        }
    }

    pub async fn enqueue(&self, request: Request) -> Result<(), QueueError> {
        self.sender.send(request).await
            .map_err(|_| QueueError::Closed)
    }

    pub async fn dequeue(&self) -> Option<Request> {
        self.receiver.lock().await.recv().await
    }

    pub fn len(&self) -> usize {
        // Approximate - channel doesn't expose exact length
        self.capacity - self.sender.capacity()
    }
}
```

**Benefits:**
- Simple and predictable
- Good for uniform workloads

**Drawbacks:**
- No prioritization
- Expensive operations block cheap ones

#### 2. Priority Queue

**Use Case:** When some operations are more critical than others.

```rust
use std::cmp::Ordering;
use tokio::sync::Notify;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    Critical = 0,   // Health checks, cleanup
    High = 1,       // Read operations
    Normal = 2,     // Write operations
    Low = 3,        // Background recalculations
}

pub struct PriorityRequest {
    priority: Priority,
    request: Request,
    enqueued_at: Instant,
}

impl Ord for PriorityRequest {
    fn cmp(&self, other: &Self) -> Ordering {
        // Lower priority value = higher priority
        self.priority.cmp(&other.priority)
            .then_with(|| self.enqueued_at.cmp(&other.enqueued_at))
    }
}

pub struct PriorityRequestQueue {
    queue: Arc<Mutex<BinaryHeap<Reverse<PriorityRequest>>>>,
    notify: Arc<Notify>,
    capacity: usize,
}

impl PriorityRequestQueue {
    pub async fn enqueue(&self, request: Request, priority: Priority) -> Result<(), QueueError> {
        let mut queue = self.queue.lock();
        if queue.len() >= self.capacity {
            return Err(QueueError::Full);
        }

        queue.push(Reverse(PriorityRequest {
            priority,
            request,
            enqueued_at: Instant::now(),
        }));

        self.notify.notify_one();
        Ok(())
    }

    pub async fn dequeue(&self) -> Option<Request> {
        loop {
            {
                let mut queue = self.queue.lock();
                if let Some(Reverse(req)) = queue.pop() {
                    return Some(req.request);
                }
            }
            self.notify.notified().await;
        }
    }
}
```

**Priority Assignment for ggen-mcp:**
```rust
fn assign_priority(tool_name: &str) -> Priority {
    match tool_name {
        // Critical operations
        "close_workbook" | "discard_fork" => Priority::Critical,

        // High priority (cheap reads)
        "list_workbooks" | "list_sheets" | "describe_workbook" => Priority::High,

        // Normal priority (most operations)
        "read_table" | "sheet_overview" | "find_value" => Priority::Normal,

        // Low priority (expensive operations)
        "recalculate" | "screenshot_sheet" | "workbook_summary" => Priority::Low,

        _ => Priority::Normal,
    }
}
```

#### 3. Weighted Fair Queuing (WFQ)

**Use Case:** Balance different request types proportionally.

```rust
pub struct WeightedFairQueue {
    queues: HashMap<RequestType, VecDeque<Request>>,
    weights: HashMap<RequestType, f64>,
    counters: HashMap<RequestType, f64>,
}

impl WeightedFairQueue {
    pub fn dequeue(&mut self) -> Option<Request> {
        // Find queue with minimum counter value
        let next_type = self.queues.iter()
            .filter(|(_, q)| !q.is_empty())
            .min_by(|(t1, _), (t2, _)| {
                let c1 = self.counters.get(t1).unwrap_or(&0.0);
                let c2 = self.counters.get(t2).unwrap_or(&0.0);
                c1.partial_cmp(c2).unwrap()
            })
            .map(|(t, _)| *t)?;

        // Dequeue from selected queue
        let request = self.queues.get_mut(&next_type)?.pop_front()?;

        // Increment counter by inverse of weight
        let weight = self.weights.get(&next_type).unwrap_or(&1.0);
        *self.counters.entry(next_type).or_insert(0.0) += 1.0 / weight;

        Some(request)
    }
}
```

**Weight Configuration:**
```rust
let weights = hashmap! {
    RequestType::Read => 10.0,        // 10x weight (process 10 reads for every...)
    RequestType::Write => 3.0,        // 3 writes
    RequestType::Recalc => 1.0,       // 1 recalc
};
```

### Queue Depth Monitoring

**Metrics to Track:**
```rust
pub struct QueueMetrics {
    pub depth: usize,               // Current queue size
    pub enqueued_total: u64,        // Total requests enqueued
    pub dequeued_total: u64,        // Total requests processed
    pub rejected_total: u64,        // Total requests rejected (full)
    pub avg_wait_time_ms: f64,      // Average time in queue
    pub p95_wait_time_ms: f64,      // 95th percentile wait time
    pub p99_wait_time_ms: f64,      // 99th percentile wait time
}

impl QueueMetrics {
    pub fn utilization(&self) -> f64 {
        self.depth as f64 / self.capacity as f64
    }

    pub fn throughput(&self, window_secs: f64) -> f64 {
        self.dequeued_total as f64 / window_secs
    }
}
```

### Recommended Queue Configuration

```yaml
queues:
  default:
    type: "priority"
    capacity: 1000
    reject_policy: "drop_oldest"  # When full, drop oldest low-priority item

  recalc:
    type: "fifo"
    capacity: 50               # Limited due to semaphore anyway
    reject_policy: "reject"    # Fail fast when full

  read_only:
    type: "fifo"
    capacity: 5000             # Large buffer for cheap operations
    reject_policy: "reject"
```

---

## Resource Pooling

### Overview

Resource pooling reduces **Muda** (waste) by reusing expensive resources instead of repeatedly creating/destroying them. This is a core Heijunka technique for **leveling resource utilization**.

### Current Implementation Analysis

#### Workbook Cache (LRU)

**File:** `src/state.rs`

```rust
pub struct AppState {
    cache: RwLock<LruCache<WorkbookId, Arc<WorkbookContext>>>,
    // ...
}
```

**Analysis:**
- ✅ **Least Recently Used (LRU)** eviction policy
- ✅ **Arc-wrapped** contexts for cheap cloning
- ✅ **RwLock** for concurrent reads
- ⚠️ **Fixed capacity** - no adaptive sizing
- ⚠️ **No warming** - all loads are cold

**Metrics:**
```rust
pub struct CacheStats {
    pub operations: u64,
    pub hits: u64,
    pub misses: u64,
    pub size: usize,
    pub capacity: usize,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.operations == 0 { 0.0 }
        else { self.hits as f64 / self.operations as f64 }
    }
}
```

### Resource Pool Patterns

#### 1. Object Pool (Generic)

**Use Case:** Reuse expensive-to-create objects (e.g., LibreOffice connections, temp files).

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

pub struct Pool<T> {
    objects: Arc<Mutex<Vec<T>>>,
    semaphore: Arc<Semaphore>,
    factory: Arc<dyn Fn() -> Result<T> + Send + Sync>,
    capacity: usize,
}

impl<T: Send + 'static> Pool<T> {
    pub fn new<F>(capacity: usize, factory: F) -> Self
    where
        F: Fn() -> Result<T> + Send + Sync + 'static
    {
        Self {
            objects: Arc::new(Mutex::new(Vec::new())),
            semaphore: Arc::new(Semaphore::new(capacity)),
            factory: Arc::new(factory),
            capacity,
        }
    }

    pub async fn acquire(&self) -> Result<PoolGuard<T>> {
        // Wait for available slot
        let permit = self.semaphore.acquire().await?;

        // Try to get existing object
        let object = {
            let mut objects = self.objects.lock();
            objects.pop()
        };

        // Create new if none available
        let object = match object {
            Some(obj) => obj,
            None => (self.factory())?,
        };

        Ok(PoolGuard {
            object: Some(object),
            pool: self.objects.clone(),
            _permit: permit,
        })
    }
}

pub struct PoolGuard<T> {
    object: Option<T>,
    pool: Arc<Mutex<Vec<T>>>,
    _permit: SemaphorePermit<'static>,
}

impl<T> Deref for PoolGuard<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.object.as_ref().unwrap()
    }
}

impl<T> Drop for PoolGuard<T> {
    fn drop(&mut self) {
        if let Some(object) = self.object.take() {
            self.pool.lock().push(object);
        }
    }
}
```

**Usage Example:**
```rust
// Pool of pre-opened LibreOffice connections
let pool = Pool::new(5, || {
    LibreOfficeConnection::new("/usr/bin/soffice")
});

// Acquire connection from pool
let conn = pool.acquire().await?;
conn.recalculate("/tmp/workbook.xlsx").await?;
// Connection automatically returned to pool when dropped
```

#### 2. Thread Pool (Existing: Tokio Runtime)

**Current Usage:**
```rust
// Blocking I/O operations use spawn_blocking
let workbook = task::spawn_blocking(move ||
    WorkbookContext::load(&config, &path_buf)
).await??;
```

**Analysis:**
- ✅ Tokio runtime provides thread pool for blocking operations
- ✅ Prevents blocking async executor
- ⚠️ No visibility into blocking pool utilization
- ⚠️ Default pool size: CPU cores × 512 (can be excessive)

**Recommendation:** Configure blocking pool size based on I/O characteristics:
```rust
tokio::runtime::Builder::new_multi_thread()
    .worker_threads(4)                    // Async worker threads
    .max_blocking_threads(50)             // Blocking I/O threads (for workbook loading)
    .thread_name("mcp-worker")
    .build()?
```

#### 3. Connection Pool (Future Enhancement)

**Use Case:** Pooled LibreOffice instances instead of fire-and-forget.

**Current:** Fire-and-forget executor spawns new soffice process per request
```rust
pub struct FireAndForgetExecutor {
    soffice_path: PathBuf,
    timeout: Duration,
}

impl RecalcExecutor for FireAndForgetExecutor {
    async fn recalculate(&self, workbook_path: &Path) -> Result<RecalcResult> {
        // Spawns new process every time
        Command::new(&self.soffice_path)
            .args(["--headless", "--norestore", ...])
            .output()
            .await?;
    }
}
```

**Improved:** Pooled executor with long-lived connections
```rust
pub struct PooledLibreOfficeExecutor {
    pool: Pool<LibreOfficeInstance>,
}

pub struct LibreOfficeInstance {
    socket_path: PathBuf,
    process: Child,
    connection: UnoConnection,
}

impl RecalcExecutor for PooledLibreOfficeExecutor {
    async fn recalculate(&self, workbook_path: &Path) -> Result<RecalcResult> {
        let instance = self.pool.acquire().await?;
        instance.connection.recalculate(workbook_path).await?;
        // Instance returned to pool automatically
    }
}
```

**Benefits:**
- **~10x faster** - no process spawn overhead
- **Lower resource usage** - reuse process memory
- **Better capacity control** - fixed number of instances

### Cache Warming Strategies

#### 1. Predictive Pre-Loading

**Concept:** Load frequently accessed workbooks before they're requested.

```rust
pub struct CacheWarmer {
    state: Arc<AppState>,
    access_log: Arc<Mutex<HashMap<WorkbookId, AccessStats>>>,
}

struct AccessStats {
    count: u64,
    last_access: Instant,
    access_pattern: Vec<Instant>,
}

impl CacheWarmer {
    pub async fn warm_frequently_accessed(&self) {
        let candidates = {
            let log = self.access_log.lock();
            log.iter()
                .filter(|(_, stats)| {
                    stats.count > 10 &&
                    stats.last_access.elapsed() < Duration::from_hours(1)
                })
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>()
        };

        for workbook_id in candidates {
            if !self.state.is_cached(&workbook_id) {
                let _ = self.state.open_workbook(&workbook_id).await;
            }
        }
    }
}
```

#### 2. Time-Based Pre-Loading

**Concept:** Load workbooks during low-traffic periods.

```rust
pub struct ScheduledWarmer {
    state: Arc<AppState>,
    schedule: CronSchedule,
}

impl ScheduledWarmer {
    pub async fn run(&self) {
        // Run every night at 2 AM
        if self.schedule.should_run() {
            let workbooks = self.state.list_all_workbooks()?;
            for wb in workbooks.iter().take(self.state.cache_capacity()) {
                let _ = self.state.open_workbook(&wb.workbook_id).await;
            }
        }
    }
}
```

### Resource Utilization Leveling

**Goal:** Keep resource usage steady, avoiding spikes and idle periods.

```rust
pub struct ResourceMetrics {
    pub cpu_usage: f64,           // 0.0-1.0
    pub memory_usage: f64,        // 0.0-1.0
    pub cache_hit_rate: f64,      // 0.0-1.0
    pub active_requests: usize,
    pub blocking_pool_active: usize,
}

pub struct AdaptiveResourceManager {
    metrics: Arc<Mutex<ResourceMetrics>>,
    cache_capacity: Arc<Mutex<usize>>,
}

impl AdaptiveResourceManager {
    pub fn adjust_cache_capacity(&self) {
        let metrics = self.metrics.lock();
        let mut capacity = self.cache_capacity.lock();

        // Increase cache if memory available and hit rate low
        if metrics.memory_usage < 0.6 && metrics.cache_hit_rate < 0.7 {
            *capacity = (*capacity * 11 / 10).min(1000);
        }

        // Decrease cache if memory pressure
        if metrics.memory_usage > 0.85 {
            *capacity = (*capacity * 9 / 10).max(1);
        }
    }
}
```

### Recommended Resource Pool Configuration

```yaml
resource_pools:
  workbook_cache:
    type: "lru"
    initial_capacity: 10
    max_capacity: 100
    adaptive_sizing: true
    warm_on_startup: true

  blocking_threads:
    min_threads: 10
    max_threads: 50
    idle_timeout_secs: 60

  libreoffice_instances:  # Future
    pool_size: 5
    max_idle_secs: 300
    health_check_interval_secs: 30
```

---

## Batch Size Optimization

### Overview

**Batch size** is the number of items processed together before switching contexts. In Heijunka, the goal is to find the optimal batch size that balances:
- **Setup cost** (time to start/stop operations)
- **Latency** (time from request to completion)
- **Throughput** (total items processed per second)

### Batching Opportunities in ggen-mcp

#### 1. Workbook Loading Batch

**Current:** Each workbook loaded individually
```rust
pub async fn open_workbook(&self, workbook_id: &WorkbookId) -> Result<Arc<WorkbookContext>> {
    // Load one workbook at a time
    let workbook = task::spawn_blocking(move ||
        WorkbookContext::load(&config, &path_buf)
    ).await??;
    // ...
}
```

**Optimized:** Batch load multiple workbooks in single blocking task
```rust
pub async fn open_workbooks_batch(&self, workbook_ids: &[WorkbookId])
    -> Result<Vec<Arc<WorkbookContext>>>
{
    // Clone needed data
    let ids = workbook_ids.to_vec();
    let config = self.config.clone();
    let paths: Vec<_> = ids.iter()
        .map(|id| self.resolve_workbook_path(id))
        .collect::<Result<_>>()?;

    // Load all workbooks in single blocking task
    let workbooks = task::spawn_blocking(move || {
        paths.into_iter()
            .map(|path| WorkbookContext::load(&config, &path))
            .collect::<Result<Vec<_>>>()
    }).await??;

    // Cache results
    let results = workbooks.into_iter().map(Arc::new).collect::<Vec<_>>();
    for (id, wb) in ids.iter().zip(&results) {
        self.cache.write().put(id.clone(), wb.clone());
    }

    Ok(results)
}
```

**Benefits:**
- Amortize task spawn overhead
- Better CPU cache locality
- Potential parallel I/O

#### 2. Edit Operation Batching

**Current:** Already batched via `edit_batch` tool
```rust
pub struct EditBatchParams {
    pub fork_id: String,
    pub sheet_name: String,
    pub edits: Vec<Edit>,  // ✅ Already supports batching
}
```

**Analysis:**
- ✅ API supports batching
- ⚠️ No guidance on optimal batch size
- ⚠️ Large batches can timeout

**Recommendation:** Add batch size guidance to documentation:
```markdown
### Batch Size Guidelines

- **Small batches (< 10 edits)**: Low latency, use for interactive operations
- **Medium batches (10-100 edits)**: Optimal for most use cases
- **Large batches (100-1000 edits)**: Higher throughput, but may timeout
- **Very large batches (> 1000 edits)**: Split into multiple requests

**Timeout considerations:**
- Default timeout: 30 seconds
- Each edit: ~10ms processing time
- Max recommended batch size: 2000 edits
```

#### 3. Recalculation Batching

**Current:** One recalculation per request
```rust
pub async fn recalculate(&self, fork_id: &str) -> Result<RecalcResult> {
    // Recalculates entire workbook
}
```

**Opportunity:** Batch multiple fork recalculations
```rust
pub async fn recalculate_batch(&self, fork_ids: &[String]) -> Result<Vec<RecalcResult>> {
    // Acquire semaphore permits for all at once
    let permits: Vec<_> = fork_ids.iter()
        .map(|_| self.recalc_semaphore.acquire())
        .collect();

    let permits = try_join_all(permits).await?;

    // Execute recalculations in parallel
    let results = fork_ids.iter()
        .map(|id| self.recalc_backend.recalculate(id))
        .collect::<Vec<_>>();

    try_join_all(results).await
}
```

**Trade-offs:**
- **Pro**: Higher throughput (parallel execution)
- **Con**: Higher latency (wait for all to complete)
- **Con**: All-or-nothing (one failure fails batch)

### Dynamic Batch Sizing

**Concept:** Adjust batch size based on system load.

```rust
pub struct DynamicBatcher {
    min_batch: usize,
    max_batch: usize,
    target_latency_ms: u64,
    metrics: Arc<SystemMetrics>,
}

impl DynamicBatcher {
    pub fn optimal_batch_size(&self) -> usize {
        let metrics = self.metrics.snapshot();

        // Under heavy load: smaller batches (lower latency)
        if metrics.queue_depth > 100 || metrics.cpu_usage > 0.8 {
            return self.min_batch;
        }

        // Under light load: larger batches (higher throughput)
        if metrics.queue_depth < 10 && metrics.cpu_usage < 0.5 {
            return self.max_batch;
        }

        // Adaptive sizing based on recent latency
        let observed_latency = metrics.avg_latency_ms;
        if observed_latency > self.target_latency_ms {
            // Latency too high, reduce batch size
            (metrics.recent_batch_size * 9 / 10).max(self.min_batch)
        } else {
            // Latency good, increase batch size
            (metrics.recent_batch_size * 11 / 10).min(self.max_batch)
        }
    }
}
```

### Batch Processing Patterns

#### 1. Time-Based Batching

**Concept:** Collect requests for fixed time window, then process batch.

```rust
pub struct TimeBatcher {
    requests: Arc<Mutex<Vec<Request>>>,
    window_ms: u64,
    max_batch_size: usize,
}

impl TimeBatcher {
    pub async fn run(&self) {
        let mut interval = tokio::time::interval(Duration::from_millis(self.window_ms));

        loop {
            interval.tick().await;

            // Collect batch
            let batch = {
                let mut requests = self.requests.lock();
                let batch_size = requests.len().min(self.max_batch_size);
                requests.drain(0..batch_size).collect::<Vec<_>>()
            };

            if !batch.is_empty() {
                self.process_batch(batch).await;
            }
        }
    }
}
```

#### 2. Size-Based Batching

**Concept:** Process batch when it reaches target size.

```rust
pub struct SizeBatcher {
    requests: Arc<Mutex<Vec<Request>>>,
    batch_size: usize,
    notify: Arc<Notify>,
}

impl SizeBatcher {
    pub async fn enqueue(&self, request: Request) {
        let should_process = {
            let mut requests = self.requests.lock();
            requests.push(request);
            requests.len() >= self.batch_size
        };

        if should_process {
            self.notify.notify_one();
        }
    }

    pub async fn run(&self) {
        loop {
            self.notify.notified().await;

            let batch = {
                let mut requests = self.requests.lock();
                if requests.len() >= self.batch_size {
                    requests.drain(0..self.batch_size).collect::<Vec<_>>()
                } else {
                    continue;
                }
            };

            self.process_batch(batch).await;
        }
    }
}
```

### Recommended Batch Configurations

```yaml
batching:
  workbook_loading:
    enabled: true
    max_batch_size: 10
    max_wait_ms: 100           # Don't wait more than 100ms to fill batch

  edit_operations:
    recommended_batch_size: 50
    max_batch_size: 2000
    timeout_per_edit_ms: 10

  recalculations:
    enabled: false             # Too expensive for batching
    max_parallel: 2            # Use semaphore instead
```

---

## Backpressure Mechanisms

### Overview

**Backpressure** is the system's way of signaling to clients: "Slow down, I'm overloaded!" This prevents **Muri** (overburden) by rejecting new work when capacity is reached.

### Current State: Limited Backpressure

**File:** `src/server.rs`

```rust
async fn run_tool_with_timeout<T, F>(&self, tool: &str, fut: F) -> Result<T> {
    // Timeout on individual request
    match tokio::time::timeout(timeout_duration, fut).await {
        Ok(result) => result,
        Err(_) => Err(anyhow!("tool '{}' timed out", tool)),  // ⚠️ Client doesn't know if overload or slow operation
    }
}
```

**Issues:**
- No distinction between slow operation and overload
- No signal to client to retry later
- No graceful degradation

### Backpressure Signaling Mechanisms

#### 1. HTTP Status Codes (for HTTP transport)

**Pattern:** Return 503 Service Unavailable when overloaded

```rust
pub enum BackpressureResponse {
    Ok(Response),
    RateLimited { retry_after_ms: u64 },
    Overloaded { queue_depth: usize },
}

impl BackpressureResponse {
    pub fn to_http_response(self) -> HttpResponse {
        match self {
            Self::Ok(response) => HttpResponse::ok(response),

            Self::RateLimited { retry_after_ms } => {
                HttpResponse::builder()
                    .status(429)  // Too Many Requests
                    .header("Retry-After", format!("{}", retry_after_ms / 1000))
                    .header("X-RateLimit-Retry-After-Ms", retry_after_ms.to_string())
                    .body("Rate limit exceeded")
            }

            Self::Overloaded { queue_depth } => {
                HttpResponse::builder()
                    .status(503)  // Service Unavailable
                    .header("Retry-After", "5")  // Retry after 5 seconds
                    .header("X-Queue-Depth", queue_depth.to_string())
                    .body("Service overloaded, please retry")
            }
        }
    }
}
```

#### 2. MCP Error Codes

**Pattern:** Use structured MCP errors with metadata

```rust
use rmcp::ErrorData as McpError;

pub fn create_backpressure_error(queue_depth: usize, capacity: usize) -> McpError {
    McpError::internal_error(
        format!(
            "Server overloaded: queue depth {}/{} ({}% full). Please retry in 5 seconds.",
            queue_depth, capacity, (queue_depth * 100 / capacity)
        ),
        Some(serde_json::json!({
            "error_type": "backpressure",
            "queue_depth": queue_depth,
            "queue_capacity": capacity,
            "retry_after_seconds": 5,
            "suggestion": "Reduce request rate or wait for queue to drain"
        }))
    )
}
```

#### 3. Exponential Backoff Hints

**Pattern:** Suggest increasing retry delays

```rust
pub struct BackoffPolicy {
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub multiplier: f64,
    pub jitter: bool,
}

impl BackoffPolicy {
    pub fn retry_delay(&self, attempt: u32) -> Duration {
        let delay = self.initial_delay_ms as f64
            * self.multiplier.powi(attempt as i32);
        let delay = delay.min(self.max_delay_ms as f64) as u64;

        if self.jitter {
            // Add random jitter (0-25%)
            let jitter = rand::random::<f64>() * 0.25;
            Duration::from_millis((delay as f64 * (1.0 + jitter)) as u64)
        } else {
            Duration::from_millis(delay)
        }
    }

    pub fn to_json(&self, current_attempt: u32) -> serde_json::Value {
        serde_json::json!({
            "retry_after_ms": self.retry_delay(current_attempt).as_millis(),
            "backoff_policy": {
                "initial_delay_ms": self.initial_delay_ms,
                "multiplier": self.multiplier,
                "max_delay_ms": self.max_delay_ms,
            },
            "current_attempt": current_attempt,
        })
    }
}
```

### Load Shedding Strategies

#### 1. Admission Control

**Concept:** Reject requests when system is at capacity.

```rust
pub struct AdmissionController {
    max_concurrent_requests: usize,
    active_requests: Arc<AtomicUsize>,
    queue_capacity: usize,
    queue_depth: Arc<AtomicUsize>,
}

impl AdmissionController {
    pub fn try_admit(&self) -> Result<AdmissionGuard, BackpressureError> {
        // Check concurrent request limit
        let active = self.active_requests.fetch_add(1, Ordering::SeqCst);
        if active >= self.max_concurrent_requests {
            self.active_requests.fetch_sub(1, Ordering::SeqCst);
            return Err(BackpressureError::TooManyConcurrentRequests {
                active,
                limit: self.max_concurrent_requests,
            });
        }

        // Check queue capacity
        let depth = self.queue_depth.load(Ordering::SeqCst);
        if depth >= self.queue_capacity {
            self.active_requests.fetch_sub(1, Ordering::SeqCst);
            return Err(BackpressureError::QueueFull {
                depth,
                capacity: self.queue_capacity,
            });
        }

        Ok(AdmissionGuard {
            controller: self.active_requests.clone(),
        })
    }
}

pub struct AdmissionGuard {
    controller: Arc<AtomicUsize>,
}

impl Drop for AdmissionGuard {
    fn drop(&mut self) {
        self.controller.fetch_sub(1, Ordering::SeqCst);
    }
}
```

**Usage:**
```rust
pub async fn handle_request(&self, request: Request) -> Result<Response> {
    // Try to admit request
    let _guard = self.admission_controller.try_admit()
        .map_err(|e| create_backpressure_error(e))?;

    // Process request
    self.process_request(request).await
}
```

#### 2. Selective Shedding

**Concept:** Drop low-priority requests first when overloaded.

```rust
pub struct SelectiveShedder {
    queue: Arc<Mutex<PriorityQueue<Request>>>,
    shed_threshold: f64,  // Shed when queue > threshold (0.0-1.0)
}

impl SelectiveShedder {
    pub fn should_shed(&self, request: &Request) -> bool {
        let queue = self.queue.lock();
        let utilization = queue.len() as f64 / queue.capacity() as f64;

        if utilization < self.shed_threshold {
            return false;  // Below threshold, accept all
        }

        // Shed based on priority and utilization
        match request.priority {
            Priority::Critical => false,  // Never shed critical
            Priority::High => utilization > 0.95,  // Shed high only when almost full
            Priority::Normal => utilization > 0.85,
            Priority::Low => utilization > 0.70,  // Shed low-priority first
        }
    }
}
```

#### 3. Graceful Degradation

**Concept:** Reduce functionality when overloaded instead of failing.

```rust
pub struct DegradationPolicy {
    cpu_threshold: f64,
    memory_threshold: f64,
}

impl DegradationPolicy {
    pub fn get_response_mode(&self, metrics: &SystemMetrics) -> ResponseMode {
        if metrics.cpu_usage > self.cpu_threshold
            || metrics.memory_usage > self.memory_threshold
        {
            ResponseMode::Minimal  // Return minimal data
        } else {
            ResponseMode::Full  // Return full data
        }
    }
}

pub enum ResponseMode {
    Full,      // Return all data
    Minimal,   // Return summary only
    Cached,    // Return cached data (may be stale)
}

// Example: sheet_overview with degradation
pub async fn sheet_overview(&self, params: SheetOverviewParams) -> Result<Response> {
    let mode = self.degradation_policy.get_response_mode(&self.metrics());

    match mode {
        ResponseMode::Full => {
            // Run full region detection
            self.full_sheet_overview(params).await
        }
        ResponseMode::Minimal => {
            // Skip expensive region detection
            self.minimal_sheet_overview(params).await
        }
        ResponseMode::Cached => {
            // Return cached response if available
            self.cached_sheet_overview(params).await
                .or_else(|_| self.minimal_sheet_overview(params).await)
        }
    }
}
```

### Circuit Breaker Pattern

**Concept:** Temporarily stop calling failing service to prevent cascading failures.

**File:** `src/recovery/circuit_breaker.rs` (already exists!)

```rust
pub struct CircuitBreaker {
    state: Arc<Mutex<CircuitState>>,
    failure_threshold: u32,
    timeout: Duration,
}

pub enum CircuitState {
    Closed,      // Normal operation
    Open,        // Failing, reject all requests
    HalfOpen,    // Testing if service recovered
}

impl CircuitBreaker {
    pub async fn call<F, T>(&self, f: F) -> Result<T>
    where
        F: Future<Output = Result<T>>,
    {
        let mut state = self.state.lock();

        match *state {
            CircuitState::Open => {
                // Check if timeout expired
                if state.should_attempt_reset() {
                    *state = CircuitState::HalfOpen;
                } else {
                    return Err(anyhow!("Circuit breaker is open"));
                }
            }
            CircuitState::HalfOpen | CircuitState::Closed => {}
        }
        drop(state);

        // Execute function
        match f.await {
            Ok(result) => {
                self.record_success();
                Ok(result)
            }
            Err(e) => {
                self.record_failure();
                Err(e)
            }
        }
    }
}
```

**Usage:**
```rust
pub struct RecalcService {
    backend: Arc<dyn RecalcBackend>,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl RecalcService {
    pub async fn recalculate(&self, path: &Path) -> Result<RecalcResult> {
        self.circuit_breaker.call(async {
            self.backend.recalculate(path).await
        }).await
    }
}
```

### Recommended Backpressure Configuration

```yaml
backpressure:
  admission_control:
    max_concurrent_requests: 100
    queue_capacity: 1000
    reject_policy: "fail_fast"  # vs "wait" vs "shed_low_priority"

  circuit_breaker:
    enabled: true
    failure_threshold: 5      # Open after 5 consecutive failures
    timeout_secs: 30          # Stay open for 30 seconds
    half_open_requests: 1     # Try 1 request in half-open state

  graceful_degradation:
    enabled: true
    cpu_threshold: 0.85
    memory_threshold: 0.90
    degraded_response_mode: "minimal"

  load_shedding:
    enabled: true
    shed_threshold: 0.80      # Start shedding at 80% queue capacity
    priorities:
      critical: 1.00          # Never shed
      high: 0.95
      normal: 0.85
      low: 0.70
```

---

## Fair Scheduling Algorithms

### Overview

Fair scheduling ensures that no client or operation type **starves** while others consume all resources. This prevents **Mura** (unevenness) in resource allocation.

### Scheduling Goals

1. **Fairness**: All clients get proportional access
2. **Starvation Prevention**: No request waits indefinitely
3. **Priority Support**: Critical operations jump queue
4. **Resource Efficiency**: Minimize context switching

### Scheduling Algorithms

#### 1. Round Robin

**Concept:** Take turns serving each client/request type.

```rust
pub struct RoundRobinScheduler {
    queues: Vec<VecDeque<Request>>,
    current_index: AtomicUsize,
}

impl RoundRobinScheduler {
    pub fn next_request(&self) -> Option<Request> {
        let start_index = self.current_index.load(Ordering::SeqCst);

        for i in 0..self.queues.len() {
            let index = (start_index + i) % self.queues.len();
            if let Some(request) = self.queues[index].pop_front() {
                self.current_index.store((index + 1) % self.queues.len(), Ordering::SeqCst);
                return Some(request);
            }
        }

        None
    }
}
```

**Benefits:**
- Simple and predictable
- Perfect fairness (each queue gets equal turns)

**Drawbacks:**
- Doesn't account for different operation costs
- Can waste time checking empty queues

#### 2. Weighted Fair Queuing (WFQ)

**Concept:** Allocate bandwidth proportional to weights.

```rust
pub struct WeightedFairScheduler {
    queues: HashMap<RequestType, VecDeque<Request>>,
    weights: HashMap<RequestType, f64>,
    virtual_time: HashMap<RequestType, f64>,
}

impl WeightedFairScheduler {
    pub fn next_request(&mut self) -> Option<Request> {
        // Find queue with minimum virtual time (most starved)
        let next_type = self.queues.iter()
            .filter(|(_, q)| !q.is_empty())
            .min_by(|(t1, _), (t2, _)| {
                let vt1 = self.virtual_time.get(t1).unwrap_or(&0.0);
                let vt2 = self.virtual_time.get(t2).unwrap_or(&0.0);
                vt1.partial_cmp(vt2).unwrap()
            })
            .map(|(t, _)| *t)?;

        // Dequeue request
        let request = self.queues.get_mut(&next_type)?.pop_front()?;

        // Advance virtual time by 1/weight
        let weight = self.weights.get(&next_type).unwrap_or(&1.0);
        *self.virtual_time.entry(next_type).or_insert(0.0) += 1.0 / weight;

        Some(request)
    }
}
```

**Weight Assignment for ggen-mcp:**
```rust
let weights = hashmap! {
    RequestType::Read => 10.0,        // Cheap operations get more slots
    RequestType::Write => 5.0,
    RequestType::Recalc => 1.0,       // Expensive operations get fewer slots
    RequestType::Screenshot => 0.5,
};
```

**Benefits:**
- Proportional fairness
- Configurable resource allocation
- Prevents expensive operations from dominating

#### 3. Deficit Round Robin (DRR)

**Concept:** Track "deficit" (unused quota) and carry forward.

```rust
pub struct DeficitRoundRobin {
    queues: HashMap<RequestType, VecDeque<Request>>,
    quantum: HashMap<RequestType, usize>,  // Credits per round
    deficit: HashMap<RequestType, isize>,   // Accumulated credits
}

impl DeficitRoundRobin {
    pub fn next_request(&mut self) -> Option<Request> {
        for (req_type, queue) in &mut self.queues {
            if queue.is_empty() {
                continue;
            }

            // Add quantum to deficit
            let quantum = *self.quantum.get(req_type).unwrap_or(&1);
            let deficit = self.deficit.entry(*req_type).or_insert(0);
            *deficit += quantum as isize;

            // Dequeue if deficit allows
            if let Some(request) = queue.front() {
                let cost = request.estimated_cost();
                if *deficit >= cost as isize {
                    *deficit -= cost as isize;
                    return queue.pop_front();
                }
            }
        }

        None
    }
}
```

**Benefits:**
- Handles variable-size requests
- Prevents credit waste
- More flexible than strict round-robin

#### 4. Completely Fair Scheduler (CFS-inspired)

**Concept:** Track actual runtime and schedule based on "lag" (difference from fair share).

```rust
pub struct CompletelyFairScheduler {
    queues: HashMap<ClientId, VecDeque<Request>>,
    runtime: HashMap<ClientId, Duration>,  // Actual CPU time used
    weights: HashMap<ClientId, f64>,
}

impl CompletelyFairScheduler {
    pub fn next_request(&mut self) -> Option<Request> {
        let total_weight: f64 = self.weights.values().sum();

        // Calculate fair share for each client
        let fair_runtime = |client_id: &ClientId| -> Duration {
            let weight = self.weights.get(client_id).unwrap_or(&1.0);
            let share = weight / total_weight;
            let total_runtime: Duration = self.runtime.values().sum();
            Duration::from_secs_f64(total_runtime.as_secs_f64() * share)
        };

        // Find most starved client (actual runtime < fair share)
        let most_starved = self.queues.iter()
            .filter(|(_, q)| !q.is_empty())
            .map(|(id, _)| {
                let actual = self.runtime.get(id).unwrap_or(&Duration::ZERO);
                let fair = fair_runtime(id);
                let lag = fair.saturating_sub(*actual);
                (id, lag)
            })
            .max_by_key(|(_, lag)| *lag)
            .map(|(id, _)| *id)?;

        // Dequeue from most starved client
        self.queues.get_mut(&most_starved)?.pop_front()
    }

    pub fn record_runtime(&mut self, client_id: ClientId, duration: Duration) {
        *self.runtime.entry(client_id).or_insert(Duration::ZERO) += duration;
    }
}
```

**Benefits:**
- Guarantees proportional fairness
- Self-correcting (automatically balances over time)
- Proven in Linux kernel

**Drawbacks:**
- More complex
- Requires runtime tracking

### Multi-Level Feedback Queue (MLFQ)

**Concept:** Start requests in high-priority queue, demote if they take too long.

```rust
pub struct MultiLevelFeedbackQueue {
    queues: Vec<VecDeque<Request>>,
    time_slices: Vec<Duration>,  // Time allowed at each level
    boosts: Arc<Mutex<HashMap<RequestId, usize>>>,  // Track priority boosts
}

impl MultiLevelFeedbackQueue {
    pub fn enqueue(&self, request: Request) {
        // New requests start at highest priority
        self.queues[0].push_back(request);
    }

    pub async fn process(&self) {
        // Always process highest non-empty queue first
        for (level, queue) in self.queues.iter().enumerate() {
            if let Some(mut request) = queue.pop_front() {
                let time_slice = self.time_slices[level];

                // Process with timeout
                let start = Instant::now();
                let result = tokio::time::timeout(time_slice, request.process()).await;

                match result {
                    Ok(Ok(_)) => {
                        // Completed successfully
                        request.complete();
                    }
                    Ok(Err(_)) | Err(_) => {
                        // Didn't complete in time slice
                        if level < self.queues.len() - 1 {
                            // Demote to next level
                            self.queues[level + 1].push_back(request);
                        } else {
                            // Already at lowest level, keep there
                            self.queues[level].push_back(request);
                        }
                    }
                }

                return;
            }
        }
    }

    pub fn priority_boost(&self, request_id: RequestId) {
        // Move request back to highest priority queue
        // (prevents starvation of long-running requests)
    }
}
```

**Benefits:**
- Short requests complete quickly (high priority)
- Long requests don't block short ones
- Adaptive to workload

### Recommended Scheduling Configuration

```yaml
scheduling:
  algorithm: "weighted_fair_queuing"

  weights:
    read_operations: 10.0
    write_operations: 5.0
    recalc_operations: 1.0
    screenshot_operations: 0.5

  fairness:
    enable_starvation_prevention: true
    max_wait_time_ms: 30000       # Boost priority after 30s wait
    boost_factor: 2.0              # Double weight when boosted

  context_switching:
    min_batch_size: 1              # Min requests before switching queues
    max_batch_size: 10             # Max requests before forced switch
```

---

## Capacity Planning

### Overview

Capacity planning ensures the system has **sufficient resources** to handle expected load with acceptable performance. This prevents both **Muri** (overburden) and **Muda** (waste from over-provisioning).

### Capacity Metrics

#### 1. Throughput Metrics

**Requests per Second (RPS):**
```rust
pub struct ThroughputMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub window_duration: Duration,
}

impl ThroughputMetrics {
    pub fn requests_per_second(&self) -> f64 {
        self.total_requests as f64 / self.window_duration.as_secs_f64()
    }

    pub fn success_rate(&self) -> f64 {
        self.successful_requests as f64 / self.total_requests as f64
    }
}
```

**Tool-Specific Throughput:**
```rust
pub struct PerToolMetrics {
    pub tool_name: String,
    pub requests: u64,
    pub avg_duration_ms: f64,
    pub p50_duration_ms: f64,
    pub p95_duration_ms: f64,
    pub p99_duration_ms: f64,
}

impl PerToolMetrics {
    pub fn max_theoretical_rps(&self) -> f64 {
        1000.0 / self.avg_duration_ms
    }
}
```

#### 2. Latency Metrics

**Service Level Objectives (SLOs):**
```rust
pub struct LatencySLO {
    pub p50_target_ms: f64,    // Median
    pub p95_target_ms: f64,    // 95th percentile
    pub p99_target_ms: f64,    // 99th percentile
}

impl LatencySLO {
    pub fn is_meeting_target(&self, observed: &LatencyMetrics) -> bool {
        observed.p50 <= self.p50_target_ms &&
        observed.p95 <= self.p95_target_ms &&
        observed.p99 <= self.p99_target_ms
    }
}
```

**Recommended SLOs for ggen-mcp:**
```yaml
slos:
  read_operations:
    p50: 100      # 100ms median
    p95: 500      # 500ms 95th percentile
    p99: 1000     # 1s 99th percentile

  write_operations:
    p50: 200
    p95: 1000
    p99: 2000

  recalc_operations:
    p50: 5000     # 5s median
    p95: 15000    # 15s 95th percentile
    p99: 25000    # 25s 99th percentile
```

#### 3. Resource Utilization

**System Resource Metrics:**
```rust
pub struct ResourceMetrics {
    pub cpu_usage: f64,           // 0.0-1.0
    pub memory_usage: f64,        // 0.0-1.0
    pub disk_io_read_mbps: f64,
    pub disk_io_write_mbps: f64,
    pub network_rx_mbps: f64,
    pub network_tx_mbps: f64,
}

impl ResourceMetrics {
    pub fn is_healthy(&self) -> bool {
        self.cpu_usage < 0.8 &&
        self.memory_usage < 0.85
    }
}
```

### Capacity Modeling

#### 1. Little's Law

**Formula:** `L = λ × W`
- L = Average number of requests in system
- λ = Average arrival rate (requests/second)
- W = Average time in system (seconds)

**Example:**
```rust
pub struct LittlesLaw;

impl LittlesLaw {
    /// Calculate required concurrency for target throughput and latency
    pub fn required_concurrency(
        target_rps: f64,
        avg_latency_secs: f64
    ) -> usize {
        (target_rps * avg_latency_secs).ceil() as usize
    }

    /// Calculate max throughput for given concurrency and latency
    pub fn max_throughput(
        max_concurrent: usize,
        avg_latency_secs: f64
    ) -> f64 {
        max_concurrent as f64 / avg_latency_secs
    }
}
```

**Application to ggen-mcp:**
```rust
// Recalculation capacity
let recalc_concurrency = 2;  // max_concurrent_recalcs
let recalc_latency = 5.0;     // 5 seconds average
let recalc_max_rps = LittlesLaw::max_throughput(recalc_concurrency, recalc_latency);
// = 2 / 5.0 = 0.4 requests/second max

// Read operation capacity
let read_concurrency = 100;  // Assume 100 concurrent reads
let read_latency = 0.1;      // 100ms average
let read_max_rps = LittlesLaw::max_throughput(read_concurrency, read_latency);
// = 100 / 0.1 = 1000 requests/second max
```

#### 2. Amdahl's Law

**Formula:** `Speedup = 1 / (S + P/N)`
- S = Serial fraction of work
- P = Parallel fraction of work
- N = Number of processors

**Application:** Determine concurrency benefits
```rust
pub struct AmdahlsLaw;

impl AmdahlsLaw {
    pub fn speedup(serial_fraction: f64, cores: usize) -> f64 {
        1.0 / (serial_fraction + ((1.0 - serial_fraction) / cores as f64))
    }
}

// Example: Workbook loading
let serial_fraction = 0.2;  // 20% is serial (file I/O)
let parallel_fraction = 0.8; // 80% is parallel (parsing)
let cores = 8;

let speedup = AmdahlsLaw::speedup(serial_fraction, cores);
// = 1 / (0.2 + 0.8/8) = 3.33x speedup with 8 cores
```

#### 3. Queue Theory (M/M/c)

**Model:** Multiple servers, Poisson arrivals, exponential service time

```rust
pub struct MMCQueue {
    pub arrival_rate: f64,      // λ (lambda)
    pub service_rate: f64,      // μ (mu)
    pub servers: usize,         // c
}

impl MMCQueue {
    pub fn utilization(&self) -> f64 {
        self.arrival_rate / (self.service_rate * self.servers as f64)
    }

    pub fn avg_queue_length(&self) -> f64 {
        // Simplified formula (exact calculation is complex)
        let rho = self.utilization();
        if rho >= 1.0 {
            return f64::INFINITY;  // Unstable
        }

        let c = self.servers as f64;
        let lambda = self.arrival_rate;
        let mu = self.service_rate;

        (rho.powi(self.servers as i32 + 1)) /
        ((1.0 - rho) * c)
    }

    pub fn avg_wait_time(&self) -> Duration {
        let lq = self.avg_queue_length();
        Duration::from_secs_f64(lq / self.arrival_rate)
    }
}
```

**Example for recalc queue:**
```rust
let recalc_queue = MMCQueue {
    arrival_rate: 0.3,      // 0.3 requests/second
    service_rate: 0.2,      // 5 seconds per request = 0.2 requests/second
    servers: 2,             // max_concurrent_recalcs
};

let utilization = recalc_queue.utilization();
// = 0.3 / (0.2 * 2) = 0.75 (75% utilized)

let avg_wait = recalc_queue.avg_wait_time();
// ~10 seconds average wait
```

### Load Testing

#### 1. Baseline Performance Test

**Goal:** Establish single-user performance

```rust
#[tokio::test]
async fn baseline_read_performance() {
    let server = setup_server().await;

    let iterations = 100;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = server.read_table(ReadTableParams {
            workbook_id: "wb-test".to_string(),
            sheet_name: "Sheet1".to_string(),
            limit: Some(100),
            ..Default::default()
        }).await;
    }

    let duration = start.elapsed();
    let rps = iterations as f64 / duration.as_secs_f64();

    println!("Baseline RPS: {:.2}", rps);
    // Expected: 50-100 RPS for read_table
}
```

#### 2. Concurrency Test

**Goal:** Find maximum throughput under concurrent load

```rust
use futures::future::join_all;

#[tokio::test]
async fn concurrent_load_test() {
    let server = Arc::new(setup_server().await);

    for concurrency in [1, 5, 10, 20, 50, 100] {
        let tasks: Vec<_> = (0..concurrency)
            .map(|_| {
                let server = server.clone();
                tokio::spawn(async move {
                    let start = Instant::now();
                    for _ in 0..10 {
                        let _ = server.read_table(/* ... */).await;
                    }
                    start.elapsed()
                })
            })
            .collect();

        let results = join_all(tasks).await;
        let total_duration: Duration = results.iter()
            .map(|r| r.as_ref().unwrap())
            .sum();
        let avg_duration = total_duration / concurrency as u32;

        println!("Concurrency: {}, Avg Duration: {:?}", concurrency, avg_duration);
    }
}
```

#### 3. Sustained Load Test

**Goal:** Verify stability under sustained load

```rust
#[tokio::test]
async fn sustained_load_test() {
    let server = setup_server().await;
    let duration = Duration::from_secs(300);  // 5 minutes
    let target_rps = 50.0;

    let start = Instant::now();
    let mut requests = 0u64;
    let mut errors = 0u64;

    while start.elapsed() < duration {
        let result = server.read_table(/* ... */).await;

        requests += 1;
        if result.is_err() {
            errors += 1;
        }

        // Maintain target rate
        let elapsed = start.elapsed().as_secs_f64();
        let expected_requests = (elapsed * target_rps) as u64;
        if requests > expected_requests {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    let actual_rps = requests as f64 / duration.as_secs_f64();
    let error_rate = errors as f64 / requests as f64;

    println!("Sustained RPS: {:.2}, Error Rate: {:.2}%", actual_rps, error_rate * 100.0);
    assert!(error_rate < 0.01);  // < 1% errors
}
```

### Capacity Recommendations for ggen-mcp

#### Current Configuration Analysis

Based on codebase review:

| Component | Current Limit | Bottleneck | Recommended Limit |
|-----------|--------------|------------|------------------|
| **Workbook Cache** | 5 (default) | Memory | 20-50 (depending on workbook sizes) |
| **Max Concurrent Recalcs** | 2 | CPU/LibreOffice | 2-5 (depending on CPU cores) |
| **Tool Timeout** | 30s | LibreOffice | 30s (read), 60s (recalc) |
| **Response Size** | 1MB | Network/Memory | 1-10MB (depending on use case) |
| **Blocking Threads** | CPU cores × 512 | Thread pool | 50-100 (for workbook I/O) |

#### Sizing Guide

**Small Deployment (1-10 users):**
```yaml
capacity:
  workbook_cache: 10
  max_concurrent_recalcs: 2
  max_concurrent_requests: 50
  queue_capacity: 100
  blocking_threads: 20
```

**Medium Deployment (10-50 users):**
```yaml
capacity:
  workbook_cache: 50
  max_concurrent_recalcs: 5
  max_concurrent_requests: 200
  queue_capacity: 500
  blocking_threads: 50
```

**Large Deployment (50+ users):**
```yaml
capacity:
  workbook_cache: 100
  max_concurrent_recalcs: 10
  max_concurrent_requests: 500
  queue_capacity: 2000
  blocking_threads: 100
```

#### Scaling Strategies

**Vertical Scaling (Single Server):**
- Increase cache capacity (more RAM)
- Increase concurrent recalcs (more CPU cores)
- Use faster disk (NVMe SSD for workbook I/O)

**Horizontal Scaling (Multiple Servers):**
- Load balancer distributes requests
- Shared file system for workbooks (NFS, S3)
- Cache coordination (Redis for shared cache)
- Session affinity (sticky sessions for fork operations)

```
                    ┌─────────────────┐
                    │ Load Balancer   │
                    │  (Round Robin)  │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
         ┌────▼────┐    ┌────▼────┐    ┌────▼────┐
         │ Server 1│    │ Server 2│    │ Server 3│
         └────┬────┘    └────┬────┘    └────┬────┘
              │              │              │
              └──────────────┼──────────────┘
                             │
                    ┌────────▼────────┐
                    │  Shared Storage │
                    │  (Workbooks)    │
                    └─────────────────┘
```

---

## Implementation Recommendations

### Phase 1: Foundation (Week 1-2)

**Priority: High-impact, low-risk improvements**

1. **Add Request Queue**
   - Implement bounded FIFO queue (capacity: 1000)
   - Add queue depth metrics
   - Return 503 when queue full

2. **Implement Token Bucket Rate Limiter**
   - Global rate limit: 50 req/sec sustained, 100 burst
   - Per-tool rate limits for expensive operations
   - Return 429 with Retry-After header

3. **Enhance Metrics Collection**
   - Track per-tool latency percentiles (p50, p95, p99)
   - Track queue depth over time
   - Track cache hit rate
   - Export Prometheus metrics

**Files to Modify:**
```
src/server.rs          # Add rate limiter and queue
src/state.rs           # Add metrics collection
src/config.rs          # Add rate limit configuration
```

**New Files to Create:**
```
src/rate_limit.rs      # Token bucket implementation
src/queue.rs           # Request queue implementation
src/metrics.rs         # Metrics collection and export
```

### Phase 2: Fairness (Week 3-4)

**Priority: Prevent starvation and improve scheduling**

1. **Implement Priority Queue**
   - Assign priorities based on tool type
   - Critical > High > Normal > Low
   - Add priority boost after max wait time

2. **Add Weighted Fair Queuing**
   - Configure weights per request type
   - Read: 10x, Write: 5x, Recalc: 1x

3. **Implement Admission Control**
   - Max concurrent requests limit
   - Graceful degradation when near capacity
   - Circuit breaker for failing operations

**Files to Modify:**
```
src/queue.rs           # Add priority queue
src/server.rs          # Integrate admission control
```

**New Files:**
```
src/scheduler.rs       # Fair scheduling algorithms
src/admission.rs       # Admission control logic
```

### Phase 3: Optimization (Week 5-6)

**Priority: Performance optimization**

1. **Implement LibreOffice Connection Pool**
   - Replace fire-and-forget with pooled connections
   - Pool size: 5 instances
   - Health checks every 30 seconds

2. **Add Batch Processing**
   - Batch workbook loading (up to 10 at once)
   - Dynamic batch sizing based on load

3. **Implement Cache Warming**
   - Pre-load frequently accessed workbooks
   - Scheduled warming during low-traffic periods

**Files to Modify:**
```
src/recalc/pooled.rs   # Implement pooled executor
src/state.rs           # Add batch loading
```

**New Files:**
```
src/cache_warmer.rs    # Cache warming logic
```

### Phase 4: Monitoring (Week 7-8)

**Priority: Visibility and observability**

1. **Add Health Check Endpoint**
   - `/health` returns 200 if healthy, 503 if overloaded
   - Include queue depth, cache utilization, error rate

2. **Implement Structured Logging**
   - Request ID tracking
   - Latency logging
   - Error correlation

3. **Create Grafana Dashboard**
   - RPS over time
   - Latency percentiles
   - Queue depth
   - Cache hit rate
   - Resource utilization

**New Files:**
```
src/health.rs          # Health check endpoint
dashboards/grafana.json # Grafana dashboard definition
```

### Testing Strategy

**Unit Tests:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_rate_limit() {
        let bucket = TokenBucket::new(10, 5.0);

        // Burst capacity
        for _ in 0..10 {
            assert!(bucket.try_acquire().is_ok());
        }

        // Exceeded capacity
        assert!(bucket.try_acquire().is_err());

        // Refill after time
        std::thread::sleep(Duration::from_millis(200));
        assert!(bucket.try_acquire().is_ok());
    }

    #[test]
    fn test_priority_queue_ordering() {
        let queue = PriorityQueue::new(100);

        queue.enqueue(request1, Priority::Low);
        queue.enqueue(request2, Priority::High);
        queue.enqueue(request3, Priority::Critical);

        // Dequeue in priority order
        assert_eq!(queue.dequeue().unwrap().id, request3.id);
        assert_eq!(queue.dequeue().unwrap().id, request2.id);
        assert_eq!(queue.dequeue().unwrap().id, request1.id);
    }
}
```

**Integration Tests:**
```rust
#[tokio::test]
async fn test_backpressure_under_load() {
    let server = setup_server_with_small_queue(10).await;

    // Fill queue
    let mut tasks = vec![];
    for _ in 0..15 {
        let handle = tokio::spawn(server.recalculate(/* slow operation */));
        tasks.push(handle);
    }

    // New request should be rejected
    let result = server.recalculate(/* ... */).await;
    assert!(matches!(result, Err(BackpressureError::QueueFull { .. })));
}
```

**Load Tests:**
```bash
# Use k6 or similar load testing tool
k6 run --vus 50 --duration 5m load_test.js
```

### Configuration Example

**config.yaml:**
```yaml
server:
  workspace_root: "/data/workbooks"
  cache_capacity: 50
  max_concurrent_recalcs: 5
  tool_timeout_ms: 30000
  max_response_bytes: 10000000

rate_limits:
  global:
    enabled: true
    bucket_capacity: 100
    refill_rate: 50.0
    max_wait_ms: 5000

  per_tool:
    recalculate:
      bucket_capacity: 10
      refill_rate: 2.0
    screenshot_sheet:
      bucket_capacity: 5
      refill_rate: 1.0

queues:
  default:
    type: "priority"
    capacity: 1000
    reject_policy: "fail_fast"

admission_control:
  max_concurrent_requests: 200
  enable_graceful_degradation: true
  cpu_threshold: 0.85
  memory_threshold: 0.90

scheduling:
  algorithm: "weighted_fair_queuing"
  weights:
    read: 10.0
    write: 5.0
    recalc: 1.0

  starvation_prevention:
    enabled: true
    max_wait_ms: 30000
    boost_multiplier: 2.0

backpressure:
  circuit_breaker:
    enabled: true
    failure_threshold: 5
    timeout_secs: 30

metrics:
  enabled: true
  prometheus_port: 9090
  export_interval_secs: 10
```

---

## References

### Toyota Production System

1. **Heijunka (Production Leveling)**
   - Liker, J. K. (2004). *The Toyota Way*. McGraw-Hill.
   - Ohno, T. (1988). *Toyota Production System: Beyond Large-Scale Production*. Productivity Press.

2. **Kanban (Pull System)**
   - Anderson, D. J. (2010). *Kanban: Successful Evolutionary Change for Your Technology Business*. Blue Hole Press.

3. **Just-in-Time (JIT)**
   - Womack, J. P., & Jones, D. T. (1996). *Lean Thinking*. Simon & Schuster.

### Computer Science

4. **Rate Limiting Algorithms**
   - Token Bucket: Wikipedia - https://en.wikipedia.org/wiki/Token_bucket
   - Leaky Bucket: Wikipedia - https://en.wikipedia.org/wiki/Leaky_bucket

5. **Queueing Theory**
   - Kleinrock, L. (1975). *Queueing Systems, Volume 1: Theory*. Wiley.
   - M/M/c Queue: https://en.wikipedia.org/wiki/M/M/c_queue

6. **Fair Scheduling**
   - Weighted Fair Queueing (WFQ): Demers, A., et al. (1989). "Analysis and Simulation of a Fair Queueing Algorithm"
   - Completely Fair Scheduler: https://www.kernel.org/doc/html/latest/scheduler/sched-design-CFS.html

7. **Backpressure and Flow Control**
   - Reactive Streams: https://www.reactive-streams.org/
   - TCP Flow Control: Stevens, W. R. (1994). *TCP/IP Illustrated, Volume 1*

8. **Capacity Planning**
   - Little's Law: Little, J. D. C. (1961). "A Proof for the Queuing Formula: L = λW"
   - Universal Scalability Law: Gunther, N. J. (2007). *Guerrilla Capacity Planning*

### Software Engineering

9. **Circuit Breaker Pattern**
   - Nygard, M. T. (2007). *Release It!* Pragmatic Bookshelf.
   - Netflix Hystrix: https://github.com/Netflix/Hystrix/wiki

10. **Load Shedding**
    - Google SRE Book: Beyer, B., et al. (2016). *Site Reliability Engineering*. O'Reilly.
    - Chapter 21: Handling Overload

### Rust-Specific

11. **Tokio Async Runtime**
    - https://tokio.rs/tokio/tutorial
    - Semaphore: https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html

12. **Parking Lot Synchronization**
    - https://docs.rs/parking_lot/
    - RwLock: https://docs.rs/parking_lot/latest/parking_lot/struct.RwLock.html

---

## Appendix: Heijunka Glossary

| Japanese Term | English | MCP Server Equivalent |
|--------------|---------|----------------------|
| **Heijunka (平準化)** | Production Leveling | Load balancing and rate limiting |
| **Mura (斑)** | Unevenness | Request spikes and variability |
| **Muri (無理)** | Overburden | System overload and resource exhaustion |
| **Muda (無駄)** | Waste | Inefficiency, idle resources, context switching |
| **Kanban (看板)** | Signal Card | Queue depth, backpressure signals |
| **Takt Time (タクトタイム)** | Customer Demand Rate | Target request processing rate |
| **Jidoka (自働化)** | Automation with Human Touch | Graceful degradation, error handling |
| **Kaizen (改善)** | Continuous Improvement | Monitoring, metrics, iterative optimization |
| **Andon (行灯)** | Visual Management | Health checks, dashboards, alerts |

---

**Document Version:** 1.0
**Last Updated:** 2026-01-20
**Authors:** Research analysis of ggen-mcp codebase
**Status:** Research Documentation (No Implementation)
