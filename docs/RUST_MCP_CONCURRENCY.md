# Rust MCP Server Concurrency Guide

**Version:** 1.0
**Last Updated:** 2026-01-20
**Status:** Research & Documentation

This guide documents concurrent request handling patterns for Rust MCP servers, based on analysis of the ggen-mcp implementation and established concurrency best practices.

---

## Table of Contents

1. [Current ggen-mcp Concurrency Analysis](#1-current-ggen-mcp-concurrency-analysis)
2. [Concurrency Models](#2-concurrency-models)
3. [Request Routing](#3-request-routing)
4. [Synchronization Primitives](#4-synchronization-primitives)
5. [Backpressure Management](#5-backpressure-management)
6. [Work Distribution](#6-work-distribution)
7. [Deadlock Prevention](#7-deadlock-prevention)
8. [Testing Concurrency](#8-testing-concurrency)
9. [TPS Heijunka Principles](#9-tps-heijunka-principles)

---

## 1. Current ggen-mcp Concurrency Analysis

### 1.1 Request Processing Pipeline

The ggen-mcp server processes requests through the following pipeline:

```
Client Request → MCP Protocol → Tool Router → Tool Handler → State Access → Response
                     ↓              ↓             ↓              ↓            ↓
                  Tokio RT    Tool Dispatch   Async Fn     RwLock/Mutex   Serialize
```

**Key characteristics:**
- **Asynchronous I/O**: All handlers are `async fn`, using tokio runtime
- **Non-blocking**: Network I/O and disk I/O use async operations
- **Blocking delegation**: CPU-intensive work delegated to `spawn_blocking`
- **Timeout wrapping**: All tools wrapped with configurable timeouts
- **Response size validation**: Prevents memory exhaustion from large responses

### 1.2 Semaphore Usage

**File:** `src/state.rs`, `src/recalc/mod.rs`

#### GlobalRecalcLock (WIP Limit: 2)
```rust
pub struct GlobalRecalcLock(pub Arc<Semaphore>);

impl GlobalRecalcLock {
    pub fn new(permits: usize) -> Self {
        Self(Arc::new(Semaphore::new(permits)))
    }
}
```

**Purpose:** Limit concurrent recalculation operations to prevent resource exhaustion
- Default: 2 concurrent recalcs
- LibreOffice processes are resource-intensive (CPU, memory)
- Prevents system overload from too many soffice instances

**Usage pattern:**
```rust
let _permit = semaphore.0.acquire().await?;
// Perform recalculation
// Permit automatically released on drop
```

#### GlobalScreenshotLock (WIP Limit: 1)
```rust
pub struct GlobalScreenshotLock(pub Arc<Semaphore>);

impl GlobalScreenshotLock {
    pub fn new() -> Self {
        Self(Arc::new(Semaphore::new(1))) // Single permit
    }
}
```

**Purpose:** Serialize screenshot operations
- LibreOffice screenshot rendering is not thread-safe
- Prevents race conditions in PDF/PNG conversion
- Ensures deterministic output ordering

### 1.3 Lock Contention Analysis

**File:** `src/state.rs`

#### Cache Access Patterns
```rust
pub struct AppState {
    /// RwLock for concurrent read access
    cache: RwLock<LruCache<WorkbookId, Arc<WorkbookContext>>>,
    index: RwLock<HashMap<WorkbookId, PathBuf>>,
    alias_index: RwLock<HashMap<String, WorkbookId>>,

    /// Atomics for lock-free counters
    cache_ops: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
}
```

**Lock hierarchy:**
1. **Read-heavy workload**: Most operations are cache reads (workbook lookups)
2. **Write minimization**: Locks held for minimal duration
3. **Separate lock granularity**: Index, alias, and cache have independent locks
4. **Lock-free metrics**: Counters use atomics to avoid contention

**Contention hotspots:**
- `cache.write()` during cache misses (blocking)
- `index.write()` during workbook list updates (infrequent)
- Mitigated by: short critical sections, spawn_blocking for I/O

#### Fork Registry Lock Strategy
```rust
pub struct ForkRegistry {
    /// RwLock for better read concurrency
    forks: RwLock<HashMap<String, ForkContext>>,

    /// Per-fork recalc locks prevent concurrent recalc on same fork
    recalc_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}
```

**Design rationale:**
- **RwLock for forks**: Multiple readers can access different forks concurrently
- **Per-fork Mutex**: Prevents data races during recalculation of same fork
- **Lock nesting avoided**: Recalc lock acquired separately, not while holding forks lock

### 1.4 Work Distribution

**File:** `src/state.rs`

#### Workbook Loading
```rust
async fn open_workbook(&self, workbook_id: &WorkbookId) -> Result<Arc<WorkbookContext>> {
    // 1. Fast path: check cache with read lock
    {
        let mut cache = self.cache.write();
        if let Some(entry) = cache.get(&canonical) {
            return Ok(entry.clone()); // Arc clone, cheap
        }
    }

    // 2. Slow path: load workbook off-thread
    let workbook = task::spawn_blocking(move || {
        WorkbookContext::load(&config, &path_buf)
    }).await??;

    // 3. Insert into cache
    {
        let mut cache = self.cache.write();
        cache.put(workbook_id_clone, workbook.clone());
    }

    Ok(workbook)
}
```

**Key patterns:**
- **Check-then-act**: Read lock → miss → release → blocking load → write lock
- **Offload blocking I/O**: `spawn_blocking` prevents blocking async runtime
- **Arc sharing**: Workbook clones are cheap (reference counted)

### 1.5 Backpressure Handling

**Current implementation:**

1. **Timeout enforcement**: All tools have configurable timeouts
   ```rust
   async fn run_tool_with_timeout<T, F>(&self, tool: &str, fut: F) -> Result<T>
   where F: Future<Output = Result<T>>, T: Serialize
   {
       let result = if let Some(timeout_duration) = self.state.config().tool_timeout() {
           match tokio::time::timeout(timeout_duration, fut).await {
               Ok(result) => result,
               Err(_) => Err(anyhow!("tool '{}' timed out", tool)),
           }
       } else {
           fut.await
       }?;

       self.ensure_response_size(tool, &result)?;
       Ok(result)
   }
   ```

2. **Response size limits**: Prevents memory exhaustion
   ```rust
   fn ensure_response_size<T: Serialize>(&self, tool: &str, value: &T) -> Result<()> {
       let Some(limit) = self.state.config().max_response_bytes() else {
           return Ok(());
       };
       let payload = serde_json::to_vec(value)?;
       if payload.len() > limit {
           return Err(ResponseTooLargeError::new(tool, payload.len(), limit).into());
       }
       Ok(())
   }
   ```

3. **Capacity limits**: Fork registry has max forks limit
   ```rust
   pub fn create_fork(&self, base_path: &Path) -> Result<String> {
       let forks = self.forks.read();
       if forks.len() >= self.config.max_forks {
           return Err(anyhow!(
               "max forks ({}) reached, discard existing forks first",
               self.config.max_forks
           ));
       }
       // ...
   }
   ```

4. **Semaphore-based WIP limits**: As documented in section 1.2

**Gaps:**
- No request queue depth monitoring
- No adaptive concurrency (permits are static)
- No circuit breaker for repeated failures

---

## 2. Concurrency Models

### 2.1 Task-Based Concurrency (Tokio Tasks)

**Best for:** I/O-bound operations, lightweight coordination

```rust
use tokio::task;

// Spawn concurrent tasks
async fn process_multiple_workbooks(ids: Vec<WorkbookId>) -> Result<Vec<WorkbookContext>> {
    let tasks: Vec<_> = ids.into_iter()
        .map(|id| task::spawn(async move {
            load_workbook(id).await
        }))
        .collect();

    // Wait for all tasks
    let results: Result<Vec<_>> = futures::future::try_join_all(tasks).await;
    results
}
```

**Advantages:**
- Lightweight (cheap to spawn)
- Async/await ergonomics
- Works with tokio ecosystem

**When to use:**
- Concurrent I/O operations
- Fan-out/fan-in patterns
- Background cleanup tasks

**ggen-mcp usage:**
```rust
// Background cleanup task
pub fn start_cleanup_task(self: Arc<Self>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            self.evict_expired();
        }
    });
}
```

### 2.2 Thread Pool Patterns

**Best for:** CPU-bound operations, blocking I/O

```rust
// Use tokio's spawn_blocking for blocking operations
async fn load_workbook_blocking(path: PathBuf) -> Result<WorkbookContext> {
    task::spawn_blocking(move || {
        // CPU-intensive parsing
        WorkbookContext::parse_from_disk(&path)
    }).await?
}
```

**Advantages:**
- Doesn't block async runtime
- Automatic thread pool management
- Backpressure via blocking

**When to use:**
- Parsing large files
- Image processing
- Compression/decompression
- Cryptographic operations

**ggen-mcp usage:**
```rust
// Workbook loading (CPU-intensive parsing)
let workbook = task::spawn_blocking(move || {
    WorkbookContext::load(&config, &path_buf)
}).await??;

// Image cropping (CPU-intensive)
async fn crop_png_best_effort(path: &Path) {
    let path = path.to_path_buf();
    let _ = task::spawn_blocking(move || {
        crop_png_in_place(&path)
    }).await;
}
```

### 2.3 Actor Patterns

**Best for:** Stateful entities with message-based coordination

```rust
use tokio::sync::mpsc;

struct WorkbookActor {
    receiver: mpsc::Receiver<WorkbookMessage>,
    state: WorkbookState,
}

enum WorkbookMessage {
    Load { path: PathBuf, reply: oneshot::Sender<Result<()>> },
    Query { range: Range, reply: oneshot::Sender<Result<Vec<Cell>>> },
    Close,
}

impl WorkbookActor {
    async fn run(mut self) {
        while let Some(msg) = self.receiver.recv().await {
            match msg {
                WorkbookMessage::Load { path, reply } => {
                    let result = self.state.load(&path);
                    let _ = reply.send(result);
                }
                WorkbookMessage::Query { range, reply } => {
                    let result = self.state.query(&range);
                    let _ = reply.send(result);
                }
                WorkbookMessage::Close => break,
            }
        }
    }
}
```

**Advantages:**
- Encapsulated state (no shared locks)
- Sequential message processing (no races)
- Natural backpressure via bounded channels

**When to use:**
- Complex stateful entities
- Need for message ordering guarantees
- Avoiding shared lock complexity

**Not currently used in ggen-mcp**, but could be beneficial for:
- Per-workbook request serialization
- Fork lifecycle management
- Recalculation job queue

### 2.4 Work Stealing

**Best for:** Load balancing parallel tasks across threads

```rust
use rayon::prelude::*;

// Process cells in parallel with work stealing
fn process_cells_parallel(cells: Vec<Cell>) -> Vec<ProcessedCell> {
    cells.par_iter()
        .map(|cell| process_cell(cell))
        .collect()
}
```

**Advantages:**
- Automatic load balancing
- Good for uneven workloads
- High throughput

**When to use:**
- Data-parallel operations
- MapReduce-style processing
- Batch processing

**Potential ggen-mcp applications:**
- Parallel formula evaluation
- Batch style processing
- Region detection across sheets

### 2.5 Pipeline Parallelism

**Best for:** Multi-stage processing with different resource requirements

```rust
use tokio::sync::mpsc;

async fn pipeline_workbook_processing() {
    let (tx1, rx1) = mpsc::channel(100);
    let (tx2, rx2) = mpsc::channel(100);

    // Stage 1: Parse
    tokio::spawn(async move {
        while let Some(path) = rx1.recv().await {
            let workbook = parse_workbook(path).await;
            tx2.send(workbook).await.unwrap();
        }
    });

    // Stage 2: Analyze
    tokio::spawn(async move {
        while let Some(workbook) = rx2.recv().await {
            analyze_workbook(workbook).await;
        }
    });

    // Feed pipeline
    tx1.send(path).await.unwrap();
}
```

**Advantages:**
- Stage-specific concurrency
- Different resource profiles per stage
- Natural buffering between stages

**When to use:**
- Multi-step processing
- Different concurrency needs per stage
- I/O → CPU → I/O patterns

---

## 3. Request Routing

### 3.1 Request Multiplexing

**Pattern:** Single async runtime handles multiple concurrent requests

```rust
// MCP server uses rmcp framework with tokio
// Each request is handled as a separate task
#[tool_handler(router = self.tool_router)]
impl ServerHandler for SpreadsheetServer {
    // Each tool is an independent async task
    async fn handle_call_tool(&self, request: CallToolRequest) -> Result<CallToolResult> {
        self.tool_router.handle(self, request).await
    }
}
```

**Characteristics:**
- Non-blocking request handling
- Concurrent request processing (up to runtime limits)
- No request serialization unless explicitly enforced

### 3.2 Priority Queues

**Pattern:** Prioritize certain requests over others

```rust
use std::cmp::Ordering;

#[derive(Eq, PartialEq)]
struct PrioritizedRequest {
    priority: u8,
    request: Request,
    timestamp: Instant,
}

impl Ord for PrioritizedRequest {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first, then FIFO
        other.priority.cmp(&self.priority)
            .then_with(|| self.timestamp.cmp(&other.timestamp))
    }
}

use tokio::sync::Mutex;
use std::collections::BinaryHeap;

struct PriorityRequestQueue {
    queue: Mutex<BinaryHeap<PrioritizedRequest>>,
}
```

**When to use:**
- Differentiate interactive vs batch requests
- Prioritize cache warming over cold queries
- Admin operations over user operations

**Not currently in ggen-mcp**, but could be useful for:
- Prioritizing small queries over large recalcs
- Fast-tracking cache hits
- Deprioritizing screenshots

### 3.3 Load Balancing

**Pattern:** Distribute requests across multiple workers

```rust
struct WorkerPool {
    workers: Vec<mpsc::Sender<Request>>,
    next: AtomicUsize,
}

impl WorkerPool {
    async fn submit(&self, request: Request) -> Result<Response> {
        let idx = self.next.fetch_add(1, Ordering::Relaxed) % self.workers.len();
        let (tx, rx) = oneshot::channel();

        self.workers[idx].send((request, tx)).await?;
        rx.await?
    }
}
```

**When to use:**
- Multiple instances of same service
- Partitioned state across workers
- Horizontal scaling

### 3.4 Request Batching

**Pattern:** Aggregate multiple requests into a batch

```rust
struct RequestBatcher {
    buffer: Arc<Mutex<Vec<Request>>>,
    batch_size: usize,
    timeout: Duration,
}

impl RequestBatcher {
    async fn submit(&self, request: Request) -> Result<Response> {
        let mut buffer = self.buffer.lock().await;
        buffer.push(request);

        if buffer.len() >= self.batch_size {
            let batch = std::mem::take(&mut *buffer);
            drop(buffer);
            process_batch(batch).await
        } else {
            // Wait for timeout
            // ...
        }
    }
}
```

**When to use:**
- Database query batching
- Network request batching
- Amortize setup costs

**Potential ggen-mcp usage:**
- Batch multiple cell queries into single workbook load
- Batch fork operations

### 3.5 Fair Scheduling

**Pattern:** Ensure all clients get fair share of resources

```rust
use std::collections::VecDeque;

struct FairScheduler {
    /// Per-client request queues
    queues: HashMap<ClientId, VecDeque<Request>>,
    /// Round-robin index
    next_client: usize,
}

impl FairScheduler {
    fn next_request(&mut self) -> Option<(ClientId, Request)> {
        // Round-robin across clients
        let clients: Vec<_> = self.queues.keys().cloned().collect();
        if clients.is_empty() {
            return None;
        }

        for _ in 0..clients.len() {
            let idx = self.next_client % clients.len();
            self.next_client += 1;

            if let Some(queue) = self.queues.get_mut(&clients[idx]) {
                if let Some(request) = queue.pop_front() {
                    return Some((clients[idx], request));
                }
            }
        }
        None
    }
}
```

**When to use:**
- Multi-tenant systems
- Prevent single client from monopolizing resources
- Ensure QoS across clients

---

## 4. Synchronization Primitives

### 4.1 Mutex vs RwLock Tradeoffs

#### Mutex (Exclusive Access)
```rust
use parking_lot::Mutex;

struct SharedState {
    data: Mutex<HashMap<String, Value>>,
}

impl SharedState {
    fn update(&self, key: String, value: Value) {
        let mut data = self.data.lock();
        data.insert(key, value);
    }
}
```

**Characteristics:**
- Single writer OR single reader at a time
- Simple, predictable semantics
- Lower overhead than RwLock
- Better for write-heavy workloads

**When to use:**
- Writes are frequent
- Critical sections are short
- Contention is low

**ggen-mcp usage:**
```rust
// Per-fork recalc locks
recalc_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
```

#### RwLock (Concurrent Reads)
```rust
use parking_lot::RwLock;

struct SharedState {
    data: RwLock<HashMap<String, Value>>,
}

impl SharedState {
    fn read(&self, key: &str) -> Option<Value> {
        let data = self.data.read();
        data.get(key).cloned()
    }

    fn write(&self, key: String, value: Value) {
        let mut data = self.data.write();
        data.insert(key, value);
    }
}
```

**Characteristics:**
- Multiple concurrent readers
- Single writer (exclusive)
- Higher overhead than Mutex
- Better for read-heavy workloads

**When to use:**
- Reads significantly outnumber writes (>80%)
- Read critical sections are long
- Many concurrent readers

**ggen-mcp usage:**
```rust
// Read-heavy workbook cache
cache: RwLock<LruCache<WorkbookId, Arc<WorkbookContext>>>,
index: RwLock<HashMap<WorkbookId, PathBuf>>,

// Read-heavy fork registry
forks: RwLock<HashMap<String, ForkContext>>,
```

**Performance comparison:**
| Workload | Mutex | RwLock | Winner |
|----------|-------|--------|--------|
| 90% reads, short critical sections | Fast | Medium | Mutex |
| 90% reads, long critical sections | Slow | Fast | RwLock |
| 50% reads, any length | Fast | Medium | Mutex |
| 10% reads (write-heavy) | Fast | Slow | Mutex |

### 4.2 Semaphore Patterns (WIP Limits)

**Pattern:** Limit concurrent access to a resource

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

struct ResourcePool {
    semaphore: Arc<Semaphore>,
}

impl ResourcePool {
    fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    async fn acquire(&self) -> Result<SemaphoreGuard> {
        let permit = self.semaphore.acquire().await?;
        Ok(SemaphoreGuard { permit })
    }
}

// RAII guard ensures release
struct SemaphoreGuard {
    permit: tokio::sync::SemaphorePermit<'static>,
}

impl Drop for SemaphoreGuard {
    fn drop(&mut self) {
        // Permit automatically released
    }
}
```

**ggen-mcp usage:**
```rust
// Limit concurrent recalculations
pub struct GlobalRecalcLock(pub Arc<Semaphore>);

// Usage:
async fn recalculate(&self, fork_id: &str) -> Result<()> {
    let _permit = self.recalc_semaphore.acquire().await?;
    // Do recalculation
    // Permit released on drop
}
```

**Advantages:**
- Simple concurrency control
- Natural backpressure
- Prevents resource exhaustion

**Best practices:**
- Use RAII guards (automatic release)
- Set limits based on resource constraints (CPU, memory, file descriptors)
- Monitor permit acquisition time for bottleneck detection

### 4.3 Barriers and Latches

**Pattern:** Coordinate multiple tasks at a synchronization point

```rust
use tokio::sync::Barrier;
use std::sync::Arc;

async fn parallel_processing(tasks: Vec<Task>) {
    let barrier = Arc::new(Barrier::new(tasks.len()));

    let handles: Vec<_> = tasks.into_iter().map(|task| {
        let barrier = barrier.clone();
        tokio::spawn(async move {
            // Phase 1
            process_phase1(task).await;

            // Wait for all tasks to complete phase 1
            barrier.wait().await;

            // Phase 2
            process_phase2(task).await;
        })
    }).collect();

    futures::future::join_all(handles).await;
}
```

**When to use:**
- Multi-phase parallel processing
- Synchronization points in parallel algorithms
- Coordinated initialization

### 4.4 Channels (mpsc, broadcast, watch)

#### MPSC (Multiple Producer, Single Consumer)
```rust
use tokio::sync::mpsc;

let (tx, mut rx) = mpsc::channel(100);

// Multiple producers
tokio::spawn(async move {
    tx.send(Message::Data(42)).await.unwrap();
});

// Single consumer
while let Some(msg) = rx.recv().await {
    process(msg);
}
```

**When to use:**
- Task coordination
- Work queue
- Actor message passing

#### Broadcast (Multiple Consumers)
```rust
use tokio::sync::broadcast;

let (tx, _rx) = broadcast::channel(100);

// Multiple consumers
let mut rx1 = tx.subscribe();
let mut rx2 = tx.subscribe();

// Send to all consumers
tx.send(Event::Update).unwrap();
```

**When to use:**
- Event distribution
- Pub/sub patterns
- Notification systems

#### Watch (Latest Value)
```rust
use tokio::sync::watch;

let (tx, rx) = watch::channel(ConfigValue::default());

// Update config
tx.send(ConfigValue::new()).unwrap();

// Receivers get latest value
let value = *rx.borrow();
```

**When to use:**
- Configuration updates
- State broadcasting
- Latest-value semantics (no queue)

### 4.5 Atomics for Lock-Free Patterns

**Pattern:** Lock-free counters and flags

```rust
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};

struct Metrics {
    requests: AtomicU64,
    errors: AtomicU64,
    shutdown: AtomicBool,
}

impl Metrics {
    fn record_request(&self) {
        self.requests.fetch_add(1, Ordering::Relaxed);
    }

    fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Acquire)
    }

    fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }
}
```

**ggen-mcp usage:**
```rust
// Lock-free cache metrics
cache_ops: AtomicU64,
cache_hits: AtomicU64,
cache_misses: AtomicU64,

// Optimistic locking version counter
version: AtomicU64,
```

**Ordering guarantees:**
- `Relaxed`: No synchronization (counters)
- `Acquire/Release`: Synchronizes with other atomic operations
- `SeqCst`: Sequentially consistent (strongest, slowest)

**Best practices:**
- Use for counters and flags
- Avoid for complex state (use locks instead)
- Choose weakest ordering that provides necessary guarantees

---

## 5. Backpressure Management

### 5.1 Bounded Channels

**Pattern:** Apply backpressure via channel capacity

```rust
use tokio::sync::mpsc;

// Bounded channel with capacity 10
let (tx, rx) = mpsc::channel(10);

// Producer blocks when channel is full
async fn producer(tx: mpsc::Sender<Item>) {
    for item in items {
        tx.send(item).await.unwrap(); // Blocks if channel full
    }
}
```

**Benefits:**
- Natural flow control
- Prevents unbounded memory growth
- Producer slows down when consumer is slow

**Sizing guidelines:**
- Small buffers (1-10): Tight coupling, immediate backpressure
- Medium buffers (10-100): Smooth out bursts
- Large buffers (100+): Decouple producer/consumer, risk of memory growth

### 5.2 Adaptive Concurrency

**Pattern:** Dynamically adjust concurrency based on system load

```rust
struct AdaptiveSemaphore {
    semaphore: Arc<Semaphore>,
    current_permits: AtomicUsize,
    max_permits: usize,
    min_permits: usize,
}

impl AdaptiveSemaphore {
    async fn acquire(&self) -> SemaphorePermit {
        self.semaphore.acquire().await.unwrap()
    }

    fn adjust_based_on_latency(&self, latency_ms: u64) {
        let current = self.current_permits.load(Ordering::Relaxed);

        if latency_ms > 1000 && current > self.min_permits {
            // High latency, reduce concurrency
            self.semaphore.close();
            self.current_permits.fetch_sub(1, Ordering::Relaxed);
        } else if latency_ms < 100 && current < self.max_permits {
            // Low latency, increase concurrency
            self.semaphore.add_permits(1);
            self.current_permits.fetch_add(1, Ordering::Relaxed);
        }
    }
}
```

**When to use:**
- Dynamic workloads
- Unknown optimal concurrency
- Multi-tenant systems

**Metrics to monitor:**
- Request latency (P50, P95, P99)
- CPU utilization
- Memory usage
- Error rate

### 5.3 Circuit Breakers

**Pattern:** Prevent cascading failures by stopping requests to failing services

```rust
use std::sync::Arc;
use std::time::{Duration, Instant};

enum CircuitState {
    Closed,  // Normal operation
    Open,    // Failing, reject requests
    HalfOpen, // Testing recovery
}

struct CircuitBreaker {
    state: Arc<Mutex<CircuitState>>,
    failure_threshold: usize,
    success_threshold: usize,
    timeout: Duration,
}

impl CircuitBreaker {
    async fn call<F, T>(&self, f: F) -> Result<T>
    where F: Future<Output = Result<T>>
    {
        let state = self.state.lock().await;
        match *state {
            CircuitState::Open => {
                return Err(anyhow!("circuit breaker open"));
            }
            CircuitState::HalfOpen => {
                drop(state);
                match f.await {
                    Ok(result) => {
                        self.on_success().await;
                        Ok(result)
                    }
                    Err(e) => {
                        self.on_failure().await;
                        Err(e)
                    }
                }
            }
            CircuitState::Closed => {
                drop(state);
                match f.await {
                    Ok(result) => Ok(result),
                    Err(e) => {
                        self.on_failure().await;
                        Err(e)
                    }
                }
            }
        }
    }
}
```

**When to use:**
- External service calls
- Operations with high failure rate
- Prevent resource exhaustion from retries

**Not in ggen-mcp currently**, but could be useful for:
- LibreOffice process failures
- File system errors
- External API calls (future features)

### 5.4 Rate Limiting

**Pattern:** Limit request rate to prevent overload

```rust
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

struct RateLimiter {
    max_requests: usize,
    window: Duration,
    requests: Mutex<Vec<Instant>>,
}

impl RateLimiter {
    async fn acquire(&self) -> Result<()> {
        let mut requests = self.requests.lock().await;
        let now = Instant::now();

        // Remove old requests outside window
        requests.retain(|&time| now.duration_since(time) < self.window);

        if requests.len() >= self.max_requests {
            return Err(anyhow!("rate limit exceeded"));
        }

        requests.push(now);
        Ok(())
    }
}
```

**Algorithms:**
- **Token bucket**: Smooth rate, allows bursts
- **Leaky bucket**: Constant rate, no bursts
- **Fixed window**: Simple, but boundary issues
- **Sliding window**: More accurate, higher overhead

**When to use:**
- API rate limiting
- Prevent DoS
- Fair resource allocation

### 5.5 Queue Depth Monitoring

**Pattern:** Monitor queue depth for system health

```rust
struct MonitoredQueue {
    queue: Arc<Mutex<VecDeque<Request>>>,
    metrics: Arc<Metrics>,
}

impl MonitoredQueue {
    async fn push(&self, request: Request) {
        let mut queue = self.queue.lock().await;
        queue.push_back(request);

        let depth = queue.len();
        self.metrics.record_queue_depth(depth);

        if depth > 1000 {
            tracing::warn!("queue depth high: {}", depth);
        }
    }
}
```

**Metrics to track:**
- Current queue depth
- Max queue depth
- Time in queue (queueing latency)
- Dropped requests

**Alerting thresholds:**
- Warning: 70% capacity
- Critical: 90% capacity
- Emergency: 100% capacity (dropping requests)

---

## 6. Work Distribution

### 6.1 Sharding Strategies

**Pattern:** Partition data across multiple shards

```rust
struct ShardedCache {
    shards: Vec<RwLock<HashMap<String, Value>>>,
}

impl ShardedCache {
    fn new(num_shards: usize) -> Self {
        let shards = (0..num_shards)
            .map(|_| RwLock::new(HashMap::new()))
            .collect();
        Self { shards }
    }

    fn get_shard(&self, key: &str) -> usize {
        let hash = hash(key);
        hash as usize % self.shards.len()
    }

    fn get(&self, key: &str) -> Option<Value> {
        let shard_idx = self.get_shard(key);
        let shard = self.shards[shard_idx].read();
        shard.get(key).cloned()
    }
}
```

**Advantages:**
- Reduces lock contention
- Parallel access to different shards
- Scales with number of shards

**Best practices:**
- Use power-of-2 shards (fast modulo)
- Good hash distribution (avoid hotspots)
- Size based on expected contention

**Potential ggen-mcp usage:**
- Shard workbook cache by workbook ID
- Shard fork registry by fork ID prefix

### 6.2 Partition-Based Concurrency

**Pattern:** Process partitions in parallel

```rust
use rayon::prelude::*;

fn process_sheet_parallel(sheet: &Sheet) -> Result<ProcessedSheet> {
    // Partition cells by row
    let partitions: Vec<_> = sheet.cells()
        .chunks(100)
        .collect();

    // Process partitions in parallel
    let results: Vec<_> = partitions.par_iter()
        .map(|partition| process_partition(partition))
        .collect::<Result<Vec<_>>>()?;

    // Merge results
    merge_results(results)
}
```

**When to use:**
- Large data sets
- Independent partitions
- CPU-bound processing

### 6.3 Reader-Writer Locks (Pattern Refinement)

**Pattern:** Allow multiple readers, single writer

Already covered in Section 4.1, but key design pattern:

```rust
// Pattern: Check-Act-Check for cache updates
async fn update_if_changed(&self, key: &str, fetch_new: impl Fn() -> Value) {
    // 1. Check with read lock (fast path)
    {
        let cache = self.cache.read();
        if cache.get(key).is_some() {
            return; // Already cached
        }
    }

    // 2. Fetch new value (no lock held)
    let value = fetch_new();

    // 3. Update with write lock
    {
        let mut cache = self.cache.write();
        // Double-check in case another thread updated
        cache.entry(key).or_insert(value);
    }
}
```

### 6.4 Fine-Grained Locking

**Pattern:** Use multiple locks for different data

```rust
// Instead of single lock for entire state
struct CoarseGrainedState {
    lock: Mutex<State>,
}

// Use separate locks for independent data
struct FineGrainedState {
    cache: RwLock<Cache>,
    index: RwLock<Index>,
    metrics: Mutex<Metrics>,
}
```

**ggen-mcp example:**
```rust
pub struct AppState {
    cache: RwLock<LruCache<WorkbookId, Arc<WorkbookContext>>>,
    index: RwLock<HashMap<WorkbookId, PathBuf>>,
    alias_index: RwLock<HashMap<String, WorkbookId>>,
    // Each can be accessed independently
}
```

**Advantages:**
- Reduced contention
- Better parallelism
- Finer control

**Disadvantages:**
- More complex code
- Risk of deadlock (lock ordering)
- Higher memory overhead

### 6.5 Lock-Free Data Structures

**Pattern:** Use atomic operations instead of locks

```rust
use std::sync::atomic::{AtomicPtr, Ordering};

struct LockFreeStack<T> {
    head: AtomicPtr<Node<T>>,
}

struct Node<T> {
    value: T,
    next: *mut Node<T>,
}

impl<T> LockFreeStack<T> {
    fn push(&self, value: T) {
        let new_node = Box::into_raw(Box::new(Node {
            value,
            next: std::ptr::null_mut(),
        }));

        loop {
            let head = self.head.load(Ordering::Acquire);
            unsafe { (*new_node).next = head; }

            if self.head.compare_exchange(
                head,
                new_node,
                Ordering::Release,
                Ordering::Acquire,
            ).is_ok() {
                break;
            }
        }
    }
}
```

**Advantages:**
- No blocking
- High performance under contention
- Wait-free progress

**Disadvantages:**
- Complex to implement correctly
- Limited use cases
- ABA problem

**When to use:**
- Very high contention
- Simple data structures
- Performance-critical paths

**Libraries:**
- `crossbeam`: Lock-free queues, stacks
- `dashmap`: Concurrent hash map
- `arc-swap`: Lock-free Arc swapping

---

## 7. Deadlock Prevention

### 7.1 Lock Ordering

**Rule:** Always acquire locks in a consistent global order

```rust
// BAD: Inconsistent lock ordering
async fn transfer_bad(from: &Account, to: &Account, amount: u64) {
    let mut from_balance = from.balance.lock().await;
    let mut to_balance = to.balance.lock().await; // Deadlock possible!
    *from_balance -= amount;
    *to_balance += amount;
}

// GOOD: Consistent lock ordering
async fn transfer_good(from: &Account, to: &Account, amount: u64) {
    let (first, second) = if from.id < to.id {
        (&from.balance, &to.balance)
    } else {
        (&to.balance, &from.balance)
    };

    let mut first_balance = first.lock().await;
    let mut second_balance = second.lock().await;

    // Update balances
}
```

**ggen-mcp lock hierarchy:**
1. `AppState.index` / `AppState.alias_index` (never held together)
2. `AppState.cache`
3. `ForkRegistry.forks`
4. `ForkRegistry.recalc_locks` (per-fork, acquired separately)

**No nested locking observed** - locks are always acquired and released within a single scope.

### 7.2 Timeout Patterns

**Pattern:** Use timeouts to detect deadlocks

```rust
use tokio::time::{timeout, Duration};

async fn acquire_with_timeout<T>(
    lock: &Mutex<T>,
    timeout_duration: Duration,
) -> Result<MutexGuard<'_, T>> {
    match timeout(timeout_duration, lock.lock()).await {
        Ok(guard) => Ok(guard),
        Err(_) => Err(anyhow!("lock acquisition timed out (possible deadlock)")),
    }
}
```

**When to use:**
- Long-running operations
- Potential deadlock scenarios
- Debugging

### 7.3 Try-Lock Patterns

**Pattern:** Attempt to acquire lock, fail fast if unavailable

```rust
async fn try_update(state: &Mutex<State>) -> Result<()> {
    match state.try_lock() {
        Some(mut guard) => {
            update_state(&mut guard)?;
            Ok(())
        }
        None => {
            // Lock is held, skip update or retry later
            Err(anyhow!("state locked, try again later"))
        }
    }
}
```

**When to use:**
- Non-critical updates
- Best-effort operations
- Avoiding blocking

### 7.4 Deadlock Detection

**Pattern:** Detect deadlocks at runtime

```rust
use std::collections::HashMap;
use parking_lot::Mutex;

struct DeadlockDetector {
    /// Thread ID -> locks held
    held_locks: Mutex<HashMap<ThreadId, Vec<LockId>>>,
    /// Lock ID -> waiting thread
    waiting_threads: Mutex<HashMap<LockId, ThreadId>>,
}

impl DeadlockDetector {
    fn check_for_cycle(&self) -> Option<Vec<ThreadId>> {
        // Build dependency graph and detect cycles
        // ...
    }
}
```

**Tools:**
- `parking_lot` deadlock detection: Enable `deadlock_detection` feature
- Rust async deadlock detector: Limited support
- Manual instrumentation

**ggen-mcp mitigation:**
- No nested lock acquisition
- Short critical sections
- Prefer RwLock for read-heavy workloads

### 7.5 Avoiding Nested Locks

**Pattern:** Minimize lock nesting depth

```rust
// BAD: Nested locks
fn bad_pattern(registry: &ForkRegistry) {
    let forks = registry.forks.write();
    let recalc_locks = registry.recalc_locks.lock(); // AVOID!
}

// GOOD: Sequential locks
fn good_pattern(registry: &ForkRegistry) {
    // Acquire, use, release
    {
        let forks = registry.forks.write();
        // Use forks
    }

    // Acquire next lock after releasing previous
    {
        let recalc_locks = registry.recalc_locks.lock();
        // Use recalc_locks
    }
}
```

**ggen-mcp adherence:**
- All lock acquisitions are single-level
- Locks released before calling other methods
- No lock held across await points (mostly)

---

## 8. Testing Concurrency

### 8.1 Loom for Testing

**Pattern:** Use `loom` for systematic concurrency testing

```rust
#[cfg(test)]
mod tests {
    use loom::sync::Arc;
    use loom::sync::atomic::{AtomicUsize, Ordering};
    use loom::thread;

    #[test]
    fn test_concurrent_increment() {
        loom::model(|| {
            let counter = Arc::new(AtomicUsize::new(0));

            let threads: Vec<_> = (0..2).map(|_| {
                let counter = counter.clone();
                thread::spawn(move || {
                    counter.fetch_add(1, Ordering::SeqCst);
                })
            }).collect();

            for t in threads {
                t.join().unwrap();
            }

            assert_eq!(counter.load(Ordering::SeqCst), 2);
        });
    }
}
```

**What loom tests:**
- All possible thread interleavings
- Memory ordering issues
- Deadlocks
- Race conditions

**Limitations:**
- Only small thread counts (2-4)
- Only small numbers of operations
- No async/await support yet

### 8.2 Stress Testing

**Pattern:** High-load concurrent testing

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stress_test_cache() {
    let state = Arc::new(AppState::new(config));
    let num_tasks = 100;
    let ops_per_task = 1000;

    let tasks: Vec<_> = (0..num_tasks).map(|i| {
        let state = state.clone();
        tokio::spawn(async move {
            for j in 0..ops_per_task {
                let workbook_id = format!("workbook-{}", (i + j) % 10);
                let _ = state.open_workbook(&workbook_id).await;
            }
        })
    }).collect();

    for task in tasks {
        task.await.unwrap();
    }

    let stats = state.cache_stats();
    println!("Cache stats: {:?}", stats);
}
```

**Metrics to collect:**
- Throughput (requests/sec)
- Latency (P50, P95, P99)
- Error rate
- Resource usage (CPU, memory)

### 8.3 Race Condition Detection

**Pattern:** Use sanitizers and tools

```bash
# Thread Sanitizer (TSan)
RUSTFLAGS="-Z sanitizer=thread" cargo test

# Miri (experimental)
cargo +nightly miri test
```

**Tools:**
- `cargo-tsan`: Thread sanitizer
- `miri`: Interpreter with UB detection
- `valgrind --tool=helgrind`: Helgrind for C/C++ components

### 8.4 Property-Based Concurrency Tests

**Pattern:** Use `proptest` or `quickcheck` for concurrent properties

```rust
use proptest::prelude::*;
use std::sync::Arc;

proptest! {
    #[test]
    fn test_cache_consistency(
        ops in prop::collection::vec(
            (any::<String>(), any::<String>()),
            1..100
        )
    ) {
        let cache = Arc::new(RwLock::new(HashMap::new()));

        // Apply operations concurrently
        let handles: Vec<_> = ops.into_iter().map(|(key, value)| {
            let cache = cache.clone();
            tokio::spawn(async move {
                cache.write().insert(key, value);
            })
        }).collect();

        // Wait for completion
        futures::future::join_all(handles).await;

        // Check invariants
        // ...
    }
}
```

**Properties to test:**
- No data races
- Invariants maintained
- Eventual consistency
- Linearizability

### 8.5 Testing Best Practices

**Checklist:**
- [ ] Test with different thread counts (1, 2, 4, 8, ...)
- [ ] Test under high load (100x normal)
- [ ] Test with slow operations (simulated delays)
- [ ] Test timeout behavior
- [ ] Test backpressure handling
- [ ] Test graceful shutdown
- [ ] Test resource cleanup
- [ ] Monitor for memory leaks

**ggen-mcp testing gaps:**
- No loom tests for atomic operations
- No systematic concurrency testing
- Limited stress tests
- No deadlock detection in tests

---

## 9. TPS Heijunka Principles

**Heijunka** (平準化) means "leveling" or "smoothing" in the Toyota Production System. It aims to level the production schedule in terms of both volume and variety to reduce waste and improve flow.

### 9.1 Level Loading in MCP Servers

**Principle:** Smooth out workload variations to maintain consistent throughput

**Application to MCP servers:**

```rust
struct LeveledScheduler {
    /// Buffer to smooth incoming requests
    buffer: Arc<Mutex<VecDeque<Request>>>,
    /// Target processing rate
    target_rate: usize,
    /// Processing interval
    interval: Duration,
}

impl LeveledScheduler {
    async fn run(&self) {
        let mut ticker = tokio::time::interval(self.interval);

        loop {
            ticker.tick().await;

            let mut buffer = self.buffer.lock().await;
            let batch_size = self.target_rate;
            let batch: Vec<_> = buffer.drain(..batch_size.min(buffer.len())).collect();
            drop(buffer);

            // Process batch at steady rate
            for request in batch {
                process_request(request).await;
            }
        }
    }
}
```

**Benefits:**
- Prevents resource spikes
- Predictable performance
- Better capacity planning
- Reduced queueing variance

**ggen-mcp application:**
- Batch recalculation requests
- Smooth screenshot generation rate
- Level workbook loading across time

### 9.2 Smooth Flow Patterns

**Principle:** Eliminate variability in processing times

**Techniques:**

1. **Request batching**: Amortize setup costs
   ```rust
   // Instead of processing one-by-one
   for request in requests {
       process(request).await; // Variable latency
   }

   // Batch similar requests
   let batches = group_by_type(requests);
   for batch in batches {
       process_batch(batch).await; // Consistent latency
   }
   ```

2. **Workload classification**: Route by expected processing time
   ```rust
   enum WorkloadClass {
       Fast,    // < 100ms
       Medium,  // 100ms - 1s
       Slow,    // > 1s
   }

   fn classify(request: &Request) -> WorkloadClass {
       match request {
           Request::CacheLookup(_) => WorkloadClass::Fast,
           Request::ReadTable(_) => WorkloadClass::Medium,
           Request::Recalculate(_) => WorkloadClass::Slow,
       }
   }
   ```

3. **Standardized processing**: Minimize variation
   ```rust
   // Use consistent algorithms and data structures
   // Avoid conditional branches in hot paths
   // Pre-allocate resources to reduce allocation variance
   ```

### 9.3 Production Leveling Techniques

**Principle:** Mix different types of work to maintain smooth flow

**EPEI (Every Part Every Interval):**

```rust
struct MixedScheduler {
    /// Work types to process
    work_types: Vec<WorkType>,
    /// Current type index
    current: AtomicUsize,
}

impl MixedScheduler {
    async fn next_work(&self) -> Option<Request> {
        // Round-robin through work types
        let idx = self.current.fetch_add(1, Ordering::Relaxed);
        let work_type = &self.work_types[idx % self.work_types.len()];

        // Get next request of this type
        work_type.queue.pop().await
    }
}
```

**Application to MCP servers:**
- Alternate between read and write operations
- Mix fast and slow requests
- Interleave different tool types

### 9.4 Capacity Buffering

**Principle:** Maintain buffer capacity to absorb demand fluctuations

```rust
struct BufferedPool {
    /// Idle workers ready for work
    idle_workers: Semaphore,
    /// Target idle capacity (buffer)
    buffer_size: usize,
}

impl BufferedPool {
    async fn acquire_worker(&self) -> WorkerPermit {
        // Block if no idle workers
        self.idle_workers.acquire().await
    }

    fn maintain_buffer(&self) {
        // Ensure buffer of idle workers
        let current_idle = self.idle_workers.available_permits();
        if current_idle < self.buffer_size {
            // Spawn more workers to maintain buffer
            self.spawn_worker();
        }
    }
}
```

**Buffer sizing:**
- Too small: No protection against bursts
- Too large: Wasted resources
- Rule of thumb: 20-30% of capacity

**ggen-mcp application:**
- Maintain buffer of parsed workbooks (cache)
- Pre-fork LibreOffice instances (future)
- Pre-allocated response buffers

### 9.5 Takt Time and Pull Systems

**Principle:** Match processing rate to demand rate

**Takt time** = Available time / Customer demand

```rust
struct TaktTimeScheduler {
    /// Time between processing items
    takt_time: Duration,
}

impl TaktTimeScheduler {
    async fn run(&self, mut queue: Receiver<Request>) {
        let mut ticker = tokio::time::interval(self.takt_time);

        loop {
            ticker.tick().await;

            // Process exactly one request per takt
            if let Some(request) = queue.recv().await {
                process_request(request).await;
            }
        }
    }
}
```

**Pull system:**
```rust
// Instead of pushing work to workers (push)
fn push_work(workers: &[Worker], work: Work) {
    workers[0].queue.push(work); // May overload
}

// Let workers pull work when ready (pull)
async fn worker_pull(shared_queue: Arc<Mutex<VecDeque<Work>>>) {
    loop {
        // Worker pulls when ready
        let work = {
            let mut queue = shared_queue.lock().await;
            queue.pop_front()
        };

        if let Some(work) = work {
            process(work).await;
        } else {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}
```

**Benefits:**
- Self-regulating system
- No overload
- Natural backpressure

### 9.6 Implementation Checklist

**For implementing Heijunka in MCP servers:**

- [ ] **Measure current variability**
  - Request arrival rate distribution
  - Processing time distribution
  - Queue depth variation

- [ ] **Classify workload**
  - Identify work types (read vs write, fast vs slow)
  - Measure processing time per type
  - Determine mixing ratios

- [ ] **Buffer design**
  - Size buffers based on variance
  - Monitor buffer utilization
  - Adjust capacity dynamically

- [ ] **Leveling mechanisms**
  - Implement request batching
  - Add workload classification
  - Set up mixed scheduling

- [ ] **Pull system**
  - Convert push to pull where appropriate
  - Implement backpressure signaling
  - Monitor queue depths

- [ ] **Monitoring**
  - Track takt time vs actual time
  - Measure flow efficiency
  - Alert on variance spikes

---

## 10. Conclusion

### Key Takeaways

1. **ggen-mcp has solid concurrency foundations**:
   - Semaphores for WIP limits
   - RwLocks for read-heavy workloads
   - Atomics for lock-free counters
   - RAII guards for transaction safety

2. **Areas for enhancement**:
   - Adaptive concurrency control
   - Circuit breakers for resilience
   - Request queue monitoring
   - Systematic concurrency testing

3. **Best practices**:
   - Choose the right primitive (Mutex vs RwLock vs Atomic)
   - Minimize lock scope
   - Avoid nested locks
   - Use RAII guards
   - Test under load

4. **TPS Heijunka principles apply**:
   - Level loading reduces resource spikes
   - Smooth flow improves predictability
   - Pull systems provide natural backpressure
   - Buffering absorbs demand variation

### Further Reading

- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [The Rust Programming Language - Concurrency](https://doc.rust-lang.org/book/ch16-00-concurrency.html)
- [parking_lot Documentation](https://docs.rs/parking_lot/)
- [Crossbeam Guide](https://docs.rs/crossbeam/)
- [Toyota Production System](https://en.wikipedia.org/wiki/Toyota_Production_System)
- [Little's Law](https://en.wikipedia.org/wiki/Little%27s_law) (for queuing theory)

---

**Document Status:** Research & Documentation Only
**No code changes made.**
