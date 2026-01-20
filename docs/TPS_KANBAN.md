# Kanban (Pull System) Principles for MCP Servers

## Executive Summary

This document analyzes how **Kanban** (pull system) principles from the **Toyota Production System (TPS)** apply to Model Context Protocol (MCP) servers, with specific findings from the `ggen-mcp` (spreadsheet-mcp) codebase. The goal is to optimize request handling, prevent overload, visualize backlogs, minimize wait times, and communicate availability.

**Key Finding**: MCP servers naturally implement pull-based patterns through async request handling, but can benefit from explicit WIP limits, queue visualization, and capacity signaling to prevent overload and optimize throughput.

---

## Table of Contents

1. [Kanban Principles Overview](#kanban-principles-overview)
2. [MCP Server Architecture Analysis](#mcp-server-architecture-analysis)
3. [Current Implementation Findings](#current-implementation-findings)
4. [Pull-Based Request Handling](#pull-based-request-handling)
5. [WIP Limit Implementation](#wip-limit-implementation)
6. [Queue Management Strategies](#queue-management-strategies)
7. [Flow Metrics](#flow-metrics)
8. [Throughput Optimization](#throughput-optimization)
9. [Demand Forecasting](#demand-forecasting)
10. [Capacity Planning](#capacity-planning)
11. [Recommendations](#recommendations)

---

## Kanban Principles Overview

### Core Concepts from Toyota Production System

**Kanban** (看板, signboard/billboard) is a scheduling system for lean manufacturing that implements **pull-based** production control. Key principles:

1. **Pull vs Push**: Work is "pulled" when capacity is available, not "pushed" when produced
2. **Visual Management**: Make work status and queue depth visible
3. **WIP Limits**: Limit work-in-progress to prevent overload
4. **Flow Optimization**: Minimize cycle time and wait time
5. **Just-in-Time**: Process work exactly when needed
6. **Continuous Improvement (Kaizen)**: Iteratively reduce waste and improve flow

### Translation to Software Systems

In software, Kanban principles translate to:

- **Demand-driven processing**: Handle requests as they arrive (event-driven)
- **Backlog visibility**: Expose queue depths and pending work
- **Concurrency limits**: Cap parallel operations to prevent thrashing
- **Flow metrics**: Measure lead time, cycle time, throughput
- **Capacity signaling**: Communicate availability and load to clients
- **Backpressure**: Reject work when at capacity (fail fast)

---

## MCP Server Architecture Analysis

### What is an MCP Server?

A **Model Context Protocol (MCP) server** is a standardized interface that allows Large Language Models (LLMs) to interact with external tools, data sources, and services. Key characteristics:

- **Request-response pattern**: Client sends tool invocation, server returns result
- **Async I/O**: Most operations involve I/O (file reads, external processes)
- **Bursty traffic**: LLM agents may send multiple requests in rapid succession
- **Variable latency**: Tool execution times vary widely (ms to minutes)
- **Resource constraints**: Limited CPU, memory, file handles, external process slots

### ggen-mcp Specifics

The `ggen-mcp` (spreadsheet-mcp) server provides:

- **Read tools**: Parse and analyze spreadsheet files (fast, mostly CPU-bound)
- **Write/fork tools**: Create temporary copies for "what-if" analysis (I/O-bound)
- **Recalc tools**: Invoke LibreOffice to recompute formulas (process-bound, slow)
- **Screenshot tools**: Render spreadsheet regions as images (process-bound, very slow)

**Traffic patterns**:
- Read-heavy: Most requests are `list_workbooks`, `sheet_overview`, `read_table`
- Write-bursty: Fork → edit batch → recalc → changeset workflow
- Resource-intensive: Recalc operations spawn external processes (soffice)

---

## Current Implementation Findings

### 1. Request Handling Patterns

**Architecture**: Async request handling with Tokio runtime

**Code Evidence**:
```rust
// src/server.rs
impl SpreadsheetServer {
    async fn run_tool_with_timeout<T, F>(&self, tool: &str, fut: F) -> Result<T>
    where
        F: Future<Output = Result<T>>,
        T: Serialize,
    {
        let result = if let Some(timeout_duration) = self.state.config().tool_timeout() {
            match tokio::time::timeout(timeout_duration, fut).await {
                Ok(result) => result,
                Err(_) => Err(anyhow!(
                    "tool '{}' timed out after {}ms",
                    tool,
                    timeout_duration.as_millis()
                )),
            }
        } else {
            fut.await
        }?;

        self.ensure_response_size(tool, &result)?;
        Ok(result)
    }
}
```

**Analysis**:
- **Pull-based**: Requests processed on arrival via async handlers
- **Timeout protection**: Tool execution capped at configurable timeout (default 30s, max 10min)
- **Response size limits**: Prevents unbounded memory growth (default 1MB, configurable)
- **No explicit queue**: Tokio runtime manages task queue internally
- **Backpressure**: Timeout and size limits provide implicit backpressure

**Kanban Alignment**: ✅ Demand-driven processing

### 2. Queue Management

**Findings**:

**A. Implicit Queues (Tokio Runtime)**
```rust
// Tokio runtime manages task queue internally
// No explicit queue visibility in application code
```

**B. Fork Registry (Explicit Resource Pool)**
```rust
// src/fork.rs
pub struct ForkRegistry {
    /// RwLock for better read concurrency on fork access
    forks: RwLock<HashMap<String, ForkContext>>,
    /// Per-fork locks for recalc operations
    recalc_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    config: ForkConfig,
}

impl ForkRegistry {
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
        // ...
    }
}
```

**C. Workbook Cache (LRU Eviction)**
```rust
// src/state.rs
pub struct AppState {
    /// Workbook cache with RwLock for concurrent read access
    cache: RwLock<LruCache<WorkbookId, Arc<WorkbookContext>>>,
    /// Workbook ID to path index
    index: RwLock<HashMap<WorkbookId, PathBuf>>,
    /// Cache operation counter for monitoring
    cache_ops: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
}
```

**Analysis**:
- **Fork queue**: Fixed-size pool (default max_forks=10), hard limit with rejection
- **Cache queue**: LRU eviction (default capacity=5), automatic eviction
- **No wait queue**: Requests rejected immediately when limits hit (fail-fast)
- **Visibility**: Fork count and cache stats available, but not exposed via tools

**Kanban Alignment**: ⚠️ Partial - WIP limits exist, but queues not visualized

### 3. Concurrency Controls

**A. Global Semaphores for Process-Bound Operations**
```rust
// src/recalc/mod.rs
pub struct GlobalRecalcLock(pub Arc<Semaphore>);

impl GlobalRecalcLock {
    pub fn new(permits: usize) -> Self {
        Self(Arc::new(Semaphore::new(permits)))
    }
}

pub struct GlobalScreenshotLock(pub Arc<Semaphore>);

impl GlobalScreenshotLock {
    pub fn new() -> Self {
        Self(Arc::new(Semaphore::new(1)))  // Only 1 concurrent screenshot
    }
}
```

**Usage**:
```rust
// src/tools/fork.rs (recalculate tool)
let semaphore = state
    .recalc_semaphore()
    .ok_or_else(|| anyhow!("recalc disabled"))?;

let _permit = semaphore
    .0
    .acquire()
    .await
    .map_err(|_| anyhow!("recalc semaphore closed"))?;

// Perform recalc with exclusive access
let result = backend.recalculate(&fork_path).await?;
```

**B. Per-Fork Recalc Locks**
```rust
// src/fork.rs
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
        if let Some(lock) = locks.get(fork_id) {
            if Arc::strong_count(lock) == 1 {
                locks.remove(fork_id);
            }
        }
    }
}
```

**C. Optimistic Locking for Fork Modifications**
```rust
// src/fork.rs
pub struct ForkContext {
    /// Version counter for optimistic locking - incremented on each modification
    version: AtomicU64,
    // ...
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
```

**Analysis**:
- **WIP Limits**:
  - Recalc operations: Configurable semaphore (default 2, max 100)
  - Screenshot operations: Hard limit of 1 (sequential processing)
  - Fork pool: Hard limit (default 10 forks)
  - Cache: LRU with capacity limit (default 5)
- **Granularity**: Global semaphore for resource class, per-fork locks for data consistency
- **Wait behavior**: Semaphore acquisition blocks (queues), fork/cache limits reject
- **Isolation**: Per-fork locks prevent concurrent modification of same fork

**Kanban Alignment**: ✅ Excellent WIP limit implementation

### 4. Async Processing

**Execution Model**:
```rust
// src/main.rs
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let cli = CliArgs::parse();
    let config = ServerConfig::from_args(cli)?;
    config.validate()?;
    run_server(config).await
}
```

**Blocking I/O Handling**:
```rust
// src/state.rs
pub async fn open_workbook(&self, workbook_id: &WorkbookId) -> Result<Arc<WorkbookContext>> {
    // ...
    // Load workbook outside of locks to avoid blocking other operations
    let workbook =
        task::spawn_blocking(move || WorkbookContext::load(&config, &path_buf)).await??;
    // ...
}
```

**External Process Handling**:
```rust
// src/recalc/fire_and_forget.rs
impl RecalcExecutor for FireAndForgetExecutor {
    async fn recalculate(&self, workbook_path: &Path) -> Result<RecalcResult> {
        let start = Instant::now();

        let output_result = time::timeout(
            self.timeout,
            Command::new(&self.soffice_path)
                .args([
                    "--headless",
                    "--norestore",
                    "--nodefault",
                    "--nofirststartwizard",
                    "--nolockcheck",
                    "--calc",
                    &macro_uri,
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await
        // ...
    }
}
```

**Analysis**:
- **Async runtime**: Tokio multi-threaded executor
- **Offloading**: CPU-bound work (file parsing) offloaded to blocking pool
- **Process isolation**: Each recalc spawns fresh LibreOffice process (clean state)
- **Timeout protection**: All external processes have timeout (default 30s)
- **Concurrent I/O**: Multiple async tasks can run concurrently

**Kanban Alignment**: ✅ Non-blocking, efficient resource utilization

### 5. Resource Allocation

**Configuration-Driven Limits**:
```rust
// src/config.rs
const DEFAULT_CACHE_CAPACITY: usize = 5;
const DEFAULT_MAX_RECALCS: usize = 2;
const DEFAULT_TOOL_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_MAX_RESPONSE_BYTES: u64 = 1_000_000;

const MAX_CACHE_CAPACITY: usize = 1000;
const MIN_CACHE_CAPACITY: usize = 1;
const MAX_CONCURRENT_RECALCS: usize = 100;
const MIN_CONCURRENT_RECALCS: usize = 1;
const MIN_TOOL_TIMEOUT_MS: u64 = 100;
const MAX_TOOL_TIMEOUT_MS: u64 = 600_000; // 10 minutes
const MIN_MAX_RESPONSE_BYTES: u64 = 1024; // 1 KB
const MAX_MAX_RESPONSE_BYTES: u64 = 100_000_000; // 100 MB
```

**Fork Resource Limits**:
```rust
// src/fork.rs
const DEFAULT_TTL_SECS: u64 = 0;  // No TTL by default
const DEFAULT_MAX_FORKS: usize = 10;
const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB
const DEFAULT_MAX_CHECKPOINTS_PER_FORK: usize = 10;
const DEFAULT_MAX_STAGED_CHANGES_PER_FORK: usize = 20;
const DEFAULT_MAX_CHECKPOINT_TOTAL_BYTES: u64 = 500 * 1024 * 1024;
```

**Adaptive Resource Management**:
```rust
// src/fork.rs
impl ForkRegistry {
    pub fn start_cleanup_task(self: Arc<Self>) {
        if self.config.ttl.is_zero() {
            return;  // No cleanup if TTL disabled
        }
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                self.evict_expired();
            }
        });
    }

    fn evict_expired(&self) {
        let mut forks = self.forks.write();
        let expired_ids: Vec<_> = forks
            .iter()
            .filter(|(_, ctx)| ctx.is_expired(self.config.ttl))
            .map(|(id, _)| id.clone())
            .collect();

        for fork_id in expired_ids {
            if let Some(ctx) = forks.remove(&fork_id) {
                debug!(fork_id = %fork_id, "evicted expired fork");
                ctx.cleanup_files();
            }
        }
    }
}
```

**Analysis**:
- **Static limits**: Max forks, max recalcs, cache capacity (operator-configured)
- **Dynamic limits**: Response size, timeout (validated at runtime)
- **Resource cleanup**: TTL-based eviction for forks, LRU eviction for cache
- **Monitoring**: Atomic counters for cache hits/misses, operations
- **Bounded resources**: File size limits, checkpoint limits prevent runaway growth

**Kanban Alignment**: ✅ Clear capacity constraints, automatic cleanup

---

## Pull-Based Request Handling

### Principle: Work Pulled When Capacity Available

In Kanban, work items move from one stage to the next only when the downstream stage has capacity. In MCP servers, this translates to:

1. **Client pull**: Client initiates request (demand signal)
2. **Capacity check**: Server checks resource availability (semaphore, pool limits)
3. **Admission control**: Accept or reject based on capacity
4. **Processing**: Execute work with bounded resources
5. **Completion**: Release resources for next request

### Current Implementation Analysis

**Request Flow**:
```
Client Request → MCP Protocol Handler → Tool Router → Tool Handler → Resource Acquisition → Processing → Response
```

**Resource Acquisition Patterns**:

**Pattern 1: Semaphore-Based Queue (Recalc)**
```rust
// Acquire permit (blocks if at capacity, queues request)
let _permit = recalc_semaphore.0.acquire().await?;

// Process with guaranteed capacity
let result = backend.recalculate(&fork_path).await?;

// Release permit (implicit on drop)
```

**Pattern 2: Hard Limit with Rejection (Fork Pool)**
```rust
// Check capacity (read lock)
{
    let forks = self.forks.read();
    if forks.len() >= self.config.max_forks {
        return Err(anyhow!("max forks reached, discard existing forks first"));
    }
}

// Proceed with fork creation
```

**Pattern 3: Optimistic Concurrency (Fork Modification)**
```rust
// Read current version
let fork = registry.get_fork("fork-123")?;
let version = fork.version();

// Attempt modification with version check
registry.with_fork_mut_versioned("fork-123", version, |ctx| {
    ctx.edits.push(edit);
    Ok(())
})?;
```

### Recommendations for Pull-Based Optimization

1. **Explicit wait queue for forks**: Instead of hard rejection, queue fork creation requests
2. **Priority queues**: Allow high-priority tool invocations to jump queue
3. **Request budgets**: Assign token budgets to clients for fair resource allocation
4. **Adaptive timeouts**: Increase timeout for queued requests based on queue depth
5. **Queue depth limits**: Cap queue size to prevent unbounded memory growth

---

## WIP Limit Implementation

### Principle: Limit Work-In-Progress to Prevent Overload

WIP limits prevent system overload by capping concurrent operations. Benefits:

- **Prevents thrashing**: Too many concurrent operations compete for resources
- **Improves throughput**: Optimal concurrency level maximizes throughput
- **Reduces latency**: Fewer context switches, better cache locality
- **Fail-fast**: Reject work early rather than queue indefinitely

### Current WIP Limits

| Resource | Limit Type | Default | Configurable | Behavior on Limit |
|----------|-----------|---------|--------------|-------------------|
| Recalc operations | Semaphore | 2 | Yes (1-100) | Queue (block) |
| Screenshot operations | Semaphore | 1 | No | Queue (block) |
| Fork pool | Hard limit | 10 | No | Reject (error) |
| Workbook cache | LRU eviction | 5 | Yes (1-1000) | Evict oldest |
| Tool timeout | Timeout | 30s | Yes (0.1s-10min) | Cancel |
| Response size | Size check | 1MB | Yes (1KB-100MB) | Error |

### WIP Limit Tuning Guidelines

**Recalc Operations** (CPU/Process-bound):
```
Optimal limit ≈ Number of CPU cores

Reasoning: LibreOffice recalc is CPU-intensive, spawns full process
- Too low: Underutilizes CPU, increases latency
- Too high: Thrashing, memory pressure, reduced throughput

Recommended: 2-4 for typical deployments, scale with cores
```

**Fork Pool** (Memory-bound):
```
Optimal limit = Available_Memory / Average_Workbook_Size

Default: 10 forks (assumes ~50MB average workbook)
- Monitor: Memory usage, fork creation rate
- Scale: Increase if memory available and high fork churn

Recommended: 10-50 depending on workbook sizes and available RAM
```

**Cache Capacity** (Memory-bound):
```
Optimal limit = Available_Memory / Average_Workbook_Parsed_Size

Default: 5 workbooks (assumes ~100MB parsed size per workbook)
- Monitor: Cache hit rate (target >80%)
- Scale: Increase if hit rate low and memory available

Recommended: 5-20 for typical use cases
```

**Tool Timeout** (Latency-sensitive):
```
Timeout = P99_Execution_Time * Safety_Factor

Default: 30s (conservative for complex spreadsheets)
- Monitor: Tool execution time percentiles
- Tune: Based on actual workload P95/P99

Recommended: 10s for read tools, 60s for recalc tools
```

### Advanced WIP Limit Patterns

**Dynamic WIP Adjustment**:
```rust
// Pseudo-code for adaptive semaphore sizing
struct AdaptiveSemaphore {
    semaphore: Arc<Semaphore>,
    target_latency_ms: u64,
    current_permits: AtomicUsize,
}

impl AdaptiveSemaphore {
    async fn auto_tune(&self, metrics: &LatencyMetrics) {
        let p95_latency = metrics.p95_latency_ms();
        let current = self.current_permits.load(Ordering::Relaxed);

        if p95_latency > self.target_latency_ms && current > 1 {
            // Latency too high, reduce concurrency
            self.semaphore.forget_permits(1);
            self.current_permits.fetch_sub(1, Ordering::Relaxed);
        } else if p95_latency < self.target_latency_ms / 2 && current < 100 {
            // Latency low, can increase concurrency
            self.semaphore.add_permits(1);
            self.current_permits.fetch_add(1, Ordering::Relaxed);
        }
    }
}
```

**Per-Client WIP Limits**:
```rust
// Fair resource allocation across multiple clients
struct ClientQuota {
    client_id: String,
    max_concurrent_requests: usize,
    active_requests: AtomicUsize,
}

impl ClientQuota {
    fn try_acquire(&self) -> Option<QuotaGuard> {
        let current = self.active_requests.fetch_add(1, Ordering::SeqCst);
        if current >= self.max_concurrent_requests {
            self.active_requests.fetch_sub(1, Ordering::SeqCst);
            None
        } else {
            Some(QuotaGuard { quota: self })
        }
    }
}
```

---

## Queue Management Strategies

### Principle: Visualize and Control Backlogs

Effective queue management requires:

1. **Visibility**: Expose queue depth, wait times
2. **Prioritization**: Process high-priority work first
3. **Bounded queues**: Prevent unbounded growth
4. **Fairness**: Prevent starvation
5. **Metrics**: Track queue depth, wait time, throughput

### Current Queue Types

**1. Implicit Queues (Tokio Runtime)**

The Tokio runtime maintains internal task queues:

```
                     ┌──────────────────┐
                     │  Tokio Scheduler │
                     └────────┬─────────┘
                              │
                    ┌─────────┴──────────┐
                    │   Work-Stealing    │
                    │   Thread Pool      │
                    │  (Multi-threaded)  │
                    └─────────┬──────────┘
                              │
              ┌───────────────┼───────────────┐
              │               │               │
         ┌────▼────┐    ┌────▼────┐    ┌────▼────┐
         │ Queue 1 │    │ Queue 2 │    │ Queue 3 │
         │ Thread 1│    │ Thread 2│    │ Thread 3│
         └─────────┘    └─────────┘    └─────────┘
```

**Characteristics**:
- **Invisible to application**: No direct queue depth metrics
- **Work-stealing**: Idle threads steal from busy threads
- **Fair**: FIFO within priority levels
- **Bounded**: Only by system memory

**2. Semaphore Wait Queue**

Semaphore acquisition forms an implicit queue:

```
Request 1 ──┐
            │
Request 2 ──┼──► Semaphore (2 permits) ──► Processing
            │         │
Request 3 ──┘         │
                      ▼
                 Wait Queue
                      │
                 Request 3 (waiting)
```

**Characteristics**:
- **FIFO**: First-come, first-served
- **No visibility**: Wait queue depth not exposed
- **Unbounded**: Can grow indefinitely (memory bound)
- **Fairness**: No priority, no starvation protection

**3. Fork Pool (Fixed-Size Resource Pool)**

```
                     ┌─────────────────┐
Request ──► Check ──►│ Fork Pool (10)  │──► Create Fork
            Capacity │                 │
                     └─────────────────┘
                              │
                              ▼
                         Reject if Full
                      (No wait queue)
```

**Characteristics**:
- **Hard limit**: No queuing, immediate rejection
- **Visible**: Fork count exposed via list_forks tool
- **Manual eviction**: User must discard forks to free space
- **TTL-based cleanup**: Automatic eviction if TTL configured

### Recommended Queue Management Improvements

**1. Expose Queue Metrics**

Add a new tool to expose queue depths:

```rust
#[tool(name = "server_status", description = "Get server queue and capacity metrics")]
pub async fn server_status(&self) -> Result<Json<ServerStatusResponse>, McpError> {
    let stats = ServerStatusResponse {
        recalc_queue_depth: self.recalc_semaphore_waiters(),
        screenshot_queue_depth: self.screenshot_semaphore_waiters(),
        fork_pool_size: self.fork_registry().list_forks().len(),
        fork_pool_capacity: self.config.max_forks,
        cache_stats: self.state.cache_stats(),
        active_requests: self.active_request_count.load(Ordering::Relaxed),
    };
    Ok(Json(stats))
}
```

**2. Implement Priority Queues**

Replace simple semaphores with priority-aware semaphores:

```rust
use tokio::sync::PriorityQueue;  // Hypothetical

struct PriorityRecalcQueue {
    queue: PriorityQueue<(RecalcRequest, Priority)>,
    semaphore: Arc<Semaphore>,
}

impl PriorityRecalcQueue {
    async fn enqueue(&self, request: RecalcRequest, priority: Priority) {
        self.queue.push((request, priority)).await;
    }

    async fn process(&self) {
        while let Some((request, _)) = self.queue.pop().await {
            let _permit = self.semaphore.acquire().await;
            self.execute(request).await;
        }
    }
}
```

**3. Bounded Wait Queues**

Add queue depth limits to prevent runaway growth:

```rust
const MAX_RECALC_QUEUE_DEPTH: usize = 50;

async fn enqueue_recalc(&self, request: RecalcRequest) -> Result<()> {
    if self.recalc_queue.len() >= MAX_RECALC_QUEUE_DEPTH {
        return Err(anyhow!("recalc queue full ({} requests), try again later",
                          MAX_RECALC_QUEUE_DEPTH));
    }
    self.recalc_queue.push(request).await;
    Ok(())
}
```

**4. Fair Queuing (Per-Client)**

Prevent single client from monopolizing queue:

```rust
struct FairQueue {
    client_queues: HashMap<ClientId, VecDeque<Request>>,
    round_robin_index: AtomicUsize,
}

impl FairQueue {
    async fn pop(&self) -> Option<Request> {
        // Round-robin across non-empty client queues
        let clients: Vec<_> = self.client_queues.keys().collect();
        if clients.is_empty() {
            return None;
        }

        let start_index = self.round_robin_index.fetch_add(1, Ordering::Relaxed);
        for i in 0..clients.len() {
            let client_id = &clients[(start_index + i) % clients.len()];
            if let Some(queue) = self.client_queues.get_mut(client_id) {
                if let Some(request) = queue.pop_front() {
                    return Some(request);
                }
            }
        }
        None
    }
}
```

---

## Flow Metrics

### Principle: Measure to Improve

Key metrics for Kanban flow optimization:

1. **Lead Time**: Total time from request arrival to completion
2. **Cycle Time**: Time from work start to completion
3. **Throughput**: Requests completed per unit time
4. **Queue Depth**: Number of requests waiting
5. **Utilization**: Percentage of time resources are busy
6. **Wait Time**: Time spent waiting in queue

### Current Metrics Collection

**Cache Metrics**:
```rust
// src/state.rs
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
            return 0.0;
        }
        self.hits as f64 / self.operations as f64
    }
}
```

**Recalc Metrics**:
```rust
// src/recalc/executor.rs
pub struct RecalcResult {
    pub duration_ms: u64,
    pub was_warm: bool,
    pub executor_type: &'static str,
}
```

**Limitations**:
- No aggregation: Metrics not collected over time
- No percentiles: No P50, P95, P99 latency tracking
- No queue metrics: Wait time, queue depth not tracked
- No throughput: Requests/sec not calculated
- No client-level: No per-client metrics

### Recommended Metrics Framework

**Metric Collection**:
```rust
use std::time::Instant;

pub struct RequestMetrics {
    tool_name: String,
    enqueued_at: Instant,
    started_at: Option<Instant>,
    completed_at: Option<Instant>,
    result: Option<Result<(), String>>,
}

impl RequestMetrics {
    pub fn lead_time(&self) -> Option<Duration> {
        self.completed_at.map(|c| c.duration_since(self.enqueued_at))
    }

    pub fn cycle_time(&self) -> Option<Duration> {
        match (self.started_at, self.completed_at) {
            (Some(s), Some(c)) => Some(c.duration_since(s)),
            _ => None,
        }
    }

    pub fn wait_time(&self) -> Option<Duration> {
        self.started_at.map(|s| s.duration_since(self.enqueued_at))
    }
}
```

**Aggregation**:
```rust
use hdrhistogram::Histogram;

pub struct ToolMetricsAggregator {
    tool_name: String,
    lead_time_histogram: Histogram<u64>,
    cycle_time_histogram: Histogram<u64>,
    request_count: AtomicU64,
    error_count: AtomicU64,
    last_minute_throughput: AtomicU64,
}

impl ToolMetricsAggregator {
    pub fn record(&mut self, metrics: RequestMetrics) {
        if let Some(lead_time) = metrics.lead_time() {
            self.lead_time_histogram.record(lead_time.as_millis() as u64).ok();
        }
        if let Some(cycle_time) = metrics.cycle_time() {
            self.cycle_time_histogram.record(cycle_time.as_millis() as u64).ok();
        }
        self.request_count.fetch_add(1, Ordering::Relaxed);
        if metrics.result.as_ref().map_or(false, |r| r.is_err()) {
            self.error_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn percentiles(&self) -> LatencyPercentiles {
        LatencyPercentiles {
            p50: self.lead_time_histogram.value_at_percentile(50.0),
            p90: self.lead_time_histogram.value_at_percentile(90.0),
            p95: self.lead_time_histogram.value_at_percentile(95.0),
            p99: self.lead_time_histogram.value_at_percentile(99.0),
        }
    }
}
```

**Visualization**:
```rust
#[tool(name = "flow_metrics", description = "Get flow metrics for performance analysis")]
pub async fn flow_metrics(
    &self,
    Parameters(params): Parameters<FlowMetricsParams>,
) -> Result<Json<FlowMetricsResponse>, McpError> {
    let tool_filter = params.tool_name;
    let time_window = params.time_window_seconds.unwrap_or(60);

    let metrics = self.metrics_aggregator.get_metrics(tool_filter, time_window);

    Ok(Json(FlowMetricsResponse {
        tool_name: tool_filter,
        time_window_seconds: time_window,
        request_count: metrics.request_count,
        error_rate: metrics.error_rate(),
        throughput_per_sec: metrics.throughput(),
        lead_time: metrics.lead_time_percentiles(),
        cycle_time: metrics.cycle_time_percentiles(),
        queue_depth_avg: metrics.avg_queue_depth,
    }))
}
```

---

## Throughput Optimization

### Principle: Maximize Flow, Minimize Waste

Throughput optimization focuses on:

1. **Eliminate bottlenecks**: Identify and remove constraints
2. **Reduce wait time**: Minimize queuing delays
3. **Increase concurrency**: Scale parallelism to optimal level
4. **Cache hot data**: Reduce redundant work
5. **Batch operations**: Amortize overhead

### Current Optimizations

**1. LRU Cache for Workbooks**
```rust
// src/state.rs
cache: RwLock<LruCache<WorkbookId, Arc<WorkbookContext>>>,
```

**Impact**: Avoids re-parsing same workbook multiple times
**Effectiveness**: High for read-heavy workflows (hit rate >80%)

**2. RwLock for Concurrent Reads**
```rust
// src/fork.rs
forks: RwLock<HashMap<String, ForkContext>>,
```

**Impact**: Multiple concurrent readers, exclusive writer
**Effectiveness**: 2-5x throughput improvement for read-heavy workloads

**3. Lazy Computation**
```rust
// Region detection computed on first access, cached for subsequent calls
// Sheet metrics computed once per sheet
```

**Impact**: Avoids expensive computations until needed
**Effectiveness**: Reduces latency for simple queries

**4. Async I/O**
```rust
// All file I/O uses tokio async primitives
task::spawn_blocking(|| /* CPU-bound work */)
```

**Impact**: Non-blocking I/O, efficient thread utilization
**Effectiveness**: Enables high concurrency with minimal threads

**5. Batch Edit Operations**
```rust
#[tool(name = "edit_batch", description = "Apply batch edits")]
pub async fn edit_batch(&self, params: EditBatchParams) -> Result<EditBatchResponse> {
    // Apply multiple edits in single operation
    for edit in params.edits {
        apply_edit(&fork, &edit)?;
    }
}
```

**Impact**: Amortizes overhead of fork locking, file I/O
**Effectiveness**: 10x speedup for bulk operations vs sequential

### Bottleneck Analysis

**CPU Bottleneck**: Spreadsheet parsing
```
Symptom: High CPU utilization, slow cache misses
Solution: Increase cache capacity, use faster parser
```

**I/O Bottleneck**: Workbook loading from disk
```
Symptom: High disk I/O wait, slow cache misses
Solution: Use SSD, increase cache capacity, prefetch
```

**Process Bottleneck**: LibreOffice recalc
```
Symptom: Recalc queue depth high, long lead times
Solution: Increase recalc semaphore permits, use pooled executor
```

**Memory Bottleneck**: Large workbooks
```
Symptom: OOM errors, cache thrashing
Solution: Reduce cache capacity, add workbook size limits
```

### Recommended Throughput Improvements

**1. Prefetching**
```rust
// Speculatively load related workbooks
async fn prefetch_related_workbooks(&self, workbook_id: &WorkbookId) {
    let related = self.get_related_workbooks(workbook_id);
    for related_id in related {
        tokio::spawn({
            let state = self.clone();
            let id = related_id.clone();
            async move {
                let _ = state.open_workbook(&id).await;
            }
        });
    }
}
```

**2. Connection Pooling for Recalc (Future)**
```rust
// Instead of fire-and-forget, maintain pool of LibreOffice instances
struct LibreOfficePool {
    instances: Vec<LibreOfficeInstance>,
    available: Semaphore,
}

impl LibreOfficePool {
    async fn recalculate(&self, workbook_path: &Path) -> Result<RecalcResult> {
        let _permit = self.available.acquire().await?;
        let instance = self.get_or_create_instance().await?;
        instance.recalculate(workbook_path).await
    }
}
```

**3. Compression for Large Responses**
```rust
// Compress large JSON responses
if response_bytes.len() > COMPRESSION_THRESHOLD {
    let compressed = zstd::encode_all(&response_bytes[..], 3)?;
    if compressed.len() < response_bytes.len() * 0.9 {
        response_bytes = compressed;
        response.set_encoding("zstd");
    }
}
```

---

## Demand Forecasting

### Principle: Anticipate Load, Adjust Capacity

Demand forecasting allows proactive resource allocation:

1. **Pattern recognition**: Identify usage patterns (daily cycles, bursty workflows)
2. **Predictive scaling**: Pre-warm caches, pre-allocate resources
3. **Load shedding**: Reject low-priority work during peak load
4. **Client profiling**: Track per-client usage patterns

### Current Capabilities

**Limited**: No historical metrics, no pattern detection

**Available Data**:
- Cache hit rate (real-time)
- Fork count (real-time)
- Recalc duration (per-operation)

### Recommended Forecasting Techniques

**1. Time-Series Metrics**
```rust
struct TimeSeriesMetrics {
    samples: VecDeque<(Instant, MetricsSample)>,
    max_samples: usize,
}

impl TimeSeriesMetrics {
    fn record(&mut self, sample: MetricsSample) {
        self.samples.push_back((Instant::now(), sample));
        while self.samples.len() > self.max_samples {
            self.samples.pop_front();
        }
    }

    fn requests_per_minute(&self, window: Duration) -> f64 {
        let cutoff = Instant::now() - window;
        let recent: Vec<_> = self.samples.iter()
            .filter(|(t, _)| *t > cutoff)
            .collect();
        recent.len() as f64 / window.as_secs_f64() * 60.0
    }
}
```

**2. Exponential Moving Average**
```rust
struct EMA {
    value: f64,
    alpha: f64,  // Smoothing factor (0-1)
}

impl EMA {
    fn update(&mut self, sample: f64) {
        self.value = self.alpha * sample + (1.0 - self.alpha) * self.value;
    }

    fn forecast(&self) -> f64 {
        self.value
    }
}

// Usage
let mut request_rate_ema = EMA { value: 0.0, alpha: 0.1 };
request_rate_ema.update(current_requests_per_minute);
let predicted_rate = request_rate_ema.forecast();
```

**3. Seasonal Pattern Detection**
```rust
// Detect daily usage patterns
struct DailyPattern {
    hourly_avg: [f64; 24],
    samples_per_hour: [usize; 24],
}

impl DailyPattern {
    fn record(&mut self, timestamp: DateTime<Utc>, value: f64) {
        let hour = timestamp.hour() as usize;
        let n = self.samples_per_hour[hour];
        self.hourly_avg[hour] = (self.hourly_avg[hour] * n as f64 + value) / (n + 1) as f64;
        self.samples_per_hour[hour] += 1;
    }

    fn predict(&self, timestamp: DateTime<Utc>) -> f64 {
        let hour = timestamp.hour() as usize;
        self.hourly_avg[hour]
    }
}
```

**4. Load-Based Auto-Scaling**
```rust
async fn auto_scale_recalc_capacity(&self) {
    let metrics = self.metrics_aggregator.get_metrics("recalculate", 300);
    let queue_depth = self.recalc_queue_depth();
    let current_permits = self.recalc_semaphore_permits();

    // Scale up if queue is growing and latency is high
    if queue_depth > 10 && metrics.p95_latency > 60_000 && current_permits < 10 {
        self.recalc_semaphore.add_permits(1);
        tracing::info!("scaled up recalc capacity to {}", current_permits + 1);
    }

    // Scale down if queue is empty and permits are underutilized
    if queue_depth == 0 && current_permits > 2 {
        let _ = self.recalc_semaphore.forget_permits(1);
        tracing::info!("scaled down recalc capacity to {}", current_permits - 1);
    }
}
```

---

## Capacity Planning

### Principle: Right-Size Resources for Workload

Capacity planning ensures adequate resources for peak load while minimizing waste. Key questions:

1. What is peak request rate?
2. What is average request latency?
3. What is resource utilization at peak?
4. Where are the bottlenecks?
5. How much headroom for growth?

### Capacity Planning Model

**Little's Law**:
```
Concurrency = Throughput × Latency

Example:
- Throughput: 10 requests/sec
- Latency: 2 seconds/request
- Required Concurrency: 10 × 2 = 20 concurrent requests
```

**Apply to ggen-mcp**:

**Recalc Operations**:
```
Target throughput: 5 recalcs/minute = 0.083 recalcs/sec
Average latency: 10 seconds
Required permits: 0.083 × 10 = 0.83 ≈ 1 permit

Current default: 2 permits (2.4x headroom)
```

**Fork Operations**:
```
Target fork creation rate: 20 forks/hour = 0.0056 forks/sec
Average fork lifetime: 30 minutes = 1800 sec
Required pool size: 0.0056 × 1800 = 10 forks

Current default: 10 forks (exact match)
```

**Workbook Cache**:
```
Target workbook access rate: 100 accesses/minute = 1.67 accesses/sec
Average workbook reuse window: 5 minutes = 300 sec
Required cache size: Unique workbooks in 5 min window

Estimate:
- If 80% of accesses hit cache (hit rate target)
- 20% are unique workbooks
- 1.67 × 300 × 0.2 = 100 unique workbooks in window

Current default: 5 workbooks (too small for high concurrency)
Recommendation: 20-50 workbooks for production
```

### Sizing Guidelines by Deployment Scale

**Small Deployment** (1-5 concurrent LLM agents):
```
cache_capacity: 5
max_concurrent_recalcs: 2
max_forks: 10
tool_timeout_ms: 30000
```

**Medium Deployment** (5-20 concurrent LLM agents):
```
cache_capacity: 20
max_concurrent_recalcs: 4
max_forks: 30
tool_timeout_ms: 60000
```

**Large Deployment** (20-100 concurrent LLM agents):
```
cache_capacity: 100
max_concurrent_recalcs: 8
max_forks: 100
tool_timeout_ms: 120000
```

### Resource Requirements Estimation

**Memory**:
```
Base: 100 MB (runtime overhead)
+ (cache_capacity × 100 MB per workbook)
+ (max_forks × 50 MB per fork)
+ (max_concurrent_recalcs × 500 MB per LibreOffice process)

Example (Medium deployment):
100 + (20 × 100) + (30 × 50) + (4 × 500)
= 100 + 2000 + 1500 + 2000
= 5.6 GB RAM
```

**CPU**:
```
Cores = max(max_concurrent_recalcs, tokio_worker_threads)

Example: 4 cores for medium deployment
```

**Disk**:
```
Workspace: Workbook storage (varies by dataset)
Temp: (max_forks × 100 MB) + (checkpoint_space × 500 MB)

Example: 30 × 100 + 10 × 500 = 8 GB temp space
```

---

## Recommendations

### Priority 1: Immediate Improvements (No Code Changes)

1. **Tune WIP limits for your workload**:
   ```bash
   # For CPU-heavy workloads (complex formulas)
   SPREADSHEET_MCP_MAX_CONCURRENT_RECALCS=4

   # For memory-constrained environments
   SPREADSHEET_MCP_CACHE_CAPACITY=3

   # For large workbooks
   SPREADSHEET_MCP_MAX_RESPONSE_BYTES=10000000
   ```

2. **Enable monitoring**:
   ```bash
   # Enable detailed logging
   RUST_LOG=spreadsheet_mcp=debug,info
   ```

3. **Document capacity planning**:
   - Measure actual request rates
   - Track cache hit rates
   - Monitor recalc queue depth
   - Size resources based on observed workload

### Priority 2: High-Impact Code Enhancements

1. **Add server_status tool** (expose queue metrics):
   ```rust
   #[tool(name = "server_status")]
   pub async fn server_status(&self) -> Result<Json<ServerStatusResponse>> {
       // Return fork count, cache stats, queue depths
   }
   ```

2. **Implement metrics aggregation**:
   ```rust
   // Use hdrhistogram for percentile tracking
   struct ToolMetrics {
       lead_time: Histogram<u64>,
       cycle_time: Histogram<u64>,
       throughput: AtomicU64,
   }
   ```

3. **Add bounded wait queues**:
   ```rust
   // Replace unlimited semaphore wait with bounded queue
   const MAX_RECALC_QUEUE_DEPTH: usize = 50;
   ```

4. **Implement request prioritization**:
   ```rust
   enum Priority {
       High,    // Interactive requests
       Normal,  // Batch analysis
       Low,     // Background prefetch
   }
   ```

### Priority 3: Advanced Optimizations

1. **Auto-tuning WIP limits**:
   ```rust
   async fn auto_tune_recalc_permits(&self) {
       // Adjust based on P95 latency and queue depth
   }
   ```

2. **Predictive cache warming**:
   ```rust
   async fn prefetch_likely_workbooks(&self, context: &RequestContext) {
       // Based on historical access patterns
   }
   ```

3. **Connection pooling for LibreOffice**:
   ```rust
   // Replace fire-and-forget with persistent pool
   struct LibreOfficePool { /* ... */ }
   ```

4. **Multi-tenant fairness**:
   ```rust
   // Per-client quotas and priority queues
   struct ClientQuota { /* ... */ }
   ```

---

## Conclusion

The `ggen-mcp` server demonstrates strong alignment with Kanban pull-based principles:

**Strengths**:
- ✅ Demand-driven async request processing
- ✅ WIP limits via semaphores (recalc, screenshot)
- ✅ Resource pools with hard limits (forks, cache)
- ✅ Optimistic locking for concurrent safety
- ✅ Efficient resource utilization (async I/O, RwLock)

**Opportunities**:
- ⚠️ Queue visibility (no queue depth metrics exposed)
- ⚠️ Flow metrics (no latency percentiles, throughput tracking)
- ⚠️ Demand forecasting (no historical metrics, pattern detection)
- ⚠️ Priority queues (FIFO only, no prioritization)
- ⚠️ Auto-tuning (static WIP limits, no adaptive scaling)

**Key Takeaways**:

1. **WIP limits prevent overload**: Semaphores are essential for process-bound operations (recalc)
2. **Visibility drives optimization**: Expose queue metrics to identify bottlenecks
3. **Measure to improve**: Track lead time, cycle time, throughput for data-driven tuning
4. **Capacity planning is critical**: Right-size resources based on workload (Little's Law)
5. **Pull-based is natural for MCP**: Async request handling inherently implements pull pattern

**Recommended Next Steps**:

1. Add `server_status` tool to expose queue/capacity metrics
2. Implement latency histogram tracking (hdrhistogram)
3. Tune WIP limits based on actual workload measurements
4. Consider bounded wait queues to prevent runaway growth
5. Plan for multi-tenant deployments with per-client quotas

By applying these Kanban principles, MCP servers can achieve:
- **Higher throughput** through optimal WIP limits
- **Lower latency** through queue management and flow optimization
- **Better reliability** through capacity planning and fail-fast behavior
- **Improved observability** through comprehensive metrics

---

## References

- **Toyota Production System**: Ohno, Taiichi. "Toyota Production System: Beyond Large-Scale Production"
- **Kanban**: Anderson, David J. "Kanban: Successful Evolutionary Change for Your Technology Business"
- **Little's Law**: Little, John D.C. "A Proof for the Queuing Formula: L = λW"
- **Tokio Async Runtime**: https://tokio.rs/
- **ggen-mcp Source**: https://github.com/PSU3D0/spreadsheet-mcp
- **Concurrency Patterns**: Goetz, Brian. "Java Concurrency in Practice" (applicable to Rust)
