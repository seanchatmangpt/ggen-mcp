# Rust Async/Await Patterns for MCP Servers

## Executive Summary

This guide documents async/await best practices for MCP (Model Context Protocol) servers in Rust, specifically for the ggen-mcp project. It synthesizes real-world patterns from the codebase with industry best practices and Toyota Production System (TPS) principles.

**Key Principles:**
- **Just-In-Time (JIT)**: Minimize async overhead, only go async when needed
- **Waste Elimination (Muda)**: Avoid blocking the async runtime, use spawn_blocking appropriately
- **Continuous Flow (Nagare)**: Keep the async runtime responsive with proper concurrency control

## Table of Contents

1. [Async Runtime Best Practices](#1-async-runtime-best-practices)
2. [Tool Handler Patterns](#2-tool-handler-patterns)
3. [Blocking Operations](#3-blocking-operations)
4. [Performance Patterns](#4-performance-patterns)
5. [Common Pitfalls](#5-common-pitfalls)
6. [Testing Async Code](#6-testing-async-code)
7. [TPS Principles in Async Design](#7-tps-principles-in-async-design)

---

## 1. Async Runtime Best Practices

### 1.1 Tokio Runtime Configuration

**Pattern**: Use the multi-threaded runtime with feature flags for precise control.

```rust
// Cargo.toml
tokio = {
    version = "1.37",
    features = [
        "macros",           // #[tokio::main] and #[tokio::test]
        "rt-multi-thread",  // Multi-threaded runtime
        "sync",             // Synchronization primitives (Mutex, Semaphore)
        "time",             // Timeout and sleep
        "fs",               // Async file operations
        "signal",           // Signal handling (Ctrl+C)
        "net",              // TCP/UDP networking
        "process"           // Async process spawning
    ]
}
```

**Main Entry Point**:

```rust
// src/main.rs - Current implementation
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let cli = CliArgs::parse();
    let config = ServerConfig::from_args(cli)?;

    // Validate configuration before server startup (fail-fast)
    config.validate()?;

    run_server(config).await
}
```

**TPS Principle**: **Jidoka (Autonomation)** - Validate early to stop the line on defects.

### 1.2 Worker Thread Sizing

**Default Behavior**: Tokio automatically sizes the thread pool based on available CPU cores.

**Current Approach**: Let tokio auto-configure unless specific needs arise.

```rust
// For CPU-bound workloads, tokio defaults to:
// - Worker threads = number of CPU cores
// - Blocking thread pool = 512 threads (grows as needed)

// Explicit configuration (if needed):
use tokio::runtime::Builder;

let runtime = Builder::new_multi_thread()
    .worker_threads(4)  // Override if needed
    .max_blocking_threads(512)
    .enable_all()
    .build()?;
```

**Observation**: The ggen-mcp codebase relies on tokio's defaults, which is appropriate for most MCP servers.

### 1.3 Blocking Task Handling

**Rule**: Use `spawn_blocking` for any operation that:
1. Performs synchronous I/O (file reads/writes)
2. Is CPU-intensive (parsing, compression, image processing)
3. Calls blocking C libraries (LibreOffice, umya-spreadsheet)

**Pattern from codebase** (src/state.rs:190):

```rust
// Loading workbooks is blocking I/O + CPU-intensive parsing
let workbook = task::spawn_blocking(move || {
    WorkbookContext::load(&config, &path_buf)
}).await??;
```

**Why**: The umya-spreadsheet library performs synchronous XML parsing that can take 100ms+ for large files. Running this directly would block all other async tasks.

**Error Handling**: Note the double `??` - one for the join error, one for the inner Result.

### 1.4 Runtime Lifecycle Management

**Pattern**: Use tokio::select! for graceful shutdown.

```rust
// src/lib.rs:119-131 - HTTP transport shutdown
tokio::select! {
    result = server_future.as_mut() => {
        tracing::info!("http transport stopped");
        result.map_err(anyhow::Error::from)?;
        return Ok(());
    }
    ctrl = tokio::signal::ctrl_c() => {
        match ctrl {
            Ok(_) => tracing::info!("shutdown signal received"),
            Err(error) => tracing::warn!(?error, "ctrl_c listener exited unexpectedly"),
        };
    }
}

// Graceful shutdown timeout
if timeout(Duration::from_secs(5), server_future.as_mut())
    .await
    .is_err()
{
    tracing::warn!("forcing http transport shutdown after timeout");
    return Ok(());
}
```

**TPS Principle**: **Andon (Problem Visualization)** - Signal handlers make problems visible and allow orderly shutdown.

---

## 2. Tool Handler Patterns

### 2.1 Async Function Design for MCP Tools

**Standard Pattern**: All tool handlers follow this structure.

```rust
// src/server.rs - Example tool handler
#[tool(
    name = "list_workbooks",
    description = "List spreadsheet files in the workspace"
)]
pub async fn list_workbooks(
    &self,
    Parameters(params): Parameters<tools::ListWorkbooksParams>,
) -> Result<Json<WorkbookListResponse>, McpError> {
    // 1. Authorization check
    self.ensure_tool_enabled("list_workbooks")
        .map_err(to_mcp_error)?;

    // 2. Execute with timeout
    self.run_tool_with_timeout(
        "list_workbooks",
        tools::list_workbooks(self.state.clone(), params),
    )
    .await
    .map(Json)
    .map_err(to_mcp_error)
}
```

**Key Components**:
1. **Parameter extraction**: `Parameters<T>` wrapper for JSON deserialization
2. **Authorization**: Check if tool is enabled
3. **Timeout wrapper**: Prevent runaway operations
4. **Error conversion**: Convert anyhow::Error to McpError

### 2.2 Error Handling in Async Contexts

**Pattern**: Use anyhow for error propagation, convert to MCP errors at boundaries.

```rust
// Internal function - use anyhow::Result
pub async fn edit_batch(
    state: Arc<AppState>,
    params: EditBatchParams,
) -> Result<EditBatchResponse> {
    let registry = state
        .fork_registry()
        .ok_or_else(|| anyhow!("fork registry not available"))?;

    // ... rest of implementation
}

// MCP boundary - convert to McpError
fn to_mcp_error(error: anyhow::Error) -> McpError {
    if error.downcast_ref::<ToolDisabledError>().is_some() {
        McpError::invalid_request(error.to_string(), None)
    } else if error.downcast_ref::<ResponseTooLargeError>().is_some() {
        McpError::invalid_request(error.to_string(), None)
    } else {
        McpError::internal_error(error.to_string(), None)
    }
}
```

**TPS Principle**: **Poka-Yoke (Error Proofing)** - Type system prevents wrong error types from crossing boundaries.

### 2.3 Timeout Patterns with tokio::time

**Pattern**: Centralized timeout handling with configurable duration.

```rust
// src/server.rs:221-241 - Timeout wrapper
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

    // Also check response size
    self.ensure_response_size(tool, &result)?;
    Ok(result)
}
```

**Usage in LibreOffice integration** (src/recalc/fire_and_forget.rs:43-61):

```rust
let output_result = time::timeout(
    self.timeout,  // Default: 30 seconds
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
.map_err(|_| anyhow!("soffice timed out after {:?}", self.timeout))
.and_then(|res| res.map_err(|e| anyhow!("failed to spawn soffice: {}", e)));
```

**Configuration**:

```rust
// src/recalc/mod.rs - Recalc configuration
#[derive(Debug, Clone)]
pub struct RecalcConfig {
    pub soffice_path: Option<PathBuf>,
    pub timeout_ms: Option<u64>,  // Default: 30_000ms
    pub strategy: ExecutorStrategy,
}
```

### 2.4 Cancellation and Graceful Shutdown

**Pattern**: Use tokio::select! to handle cancellation signals.

**Observation**: The codebase currently doesn't implement per-tool cancellation, but the HTTP transport handles global shutdown gracefully.

**Recommended Enhancement** (not yet implemented):

```rust
use tokio_util::sync::CancellationToken;

pub struct SpreadsheetServer {
    state: Arc<AppState>,
    tool_router: ToolRouter<SpreadsheetServer>,
    shutdown_token: CancellationToken,  // Add this
}

async fn run_tool_with_cancellation<T, F>(
    &self,
    tool: &str,
    fut: F
) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    tokio::select! {
        result = fut => result,
        _ = self.shutdown_token.cancelled() => {
            Err(anyhow!("tool '{}' cancelled", tool))
        }
    }
}
```

---

## 3. Blocking Operations

### 3.1 When to Use spawn_blocking

**Decision Tree**:

```
Is this operation async-native (tokio::fs, tokio::net)?
├─ YES → Use directly, no spawn_blocking needed
└─ NO → Is it I/O or CPU-bound?
    ├─ I/O-bound (std::fs, sync file operations)
    │   └─ Use spawn_blocking
    │
    └─ CPU-bound (parsing, compression, image processing)
        ├─ Takes < 10-100 microseconds?
        │   └─ NO spawn_blocking needed (acceptable inline cost)
        └─ Takes > 100 microseconds?
            └─ Use spawn_blocking
```

### 3.2 CPU-Bound vs I/O-Bound Tasks

**CPU-Bound Example** (src/tools/fork.rs:104-109):

```rust
// Editing spreadsheet cells involves:
// 1. Loading entire workbook into memory (I/O)
// 2. Modifying cell data structures (CPU)
// 3. Writing back to disk (I/O)
tokio::task::spawn_blocking({
    let sheet_name = params.sheet_name.clone();
    let edits = params.edits.clone();
    move || apply_edits_to_file(&work_path, &sheet_name, &edits)
})
.await??;

fn apply_edits_to_file(path: &Path, sheet_name: &str, edits: &[CellEdit]) -> Result<()> {
    let mut book = umya_spreadsheet::reader::xlsx::read(path)?;  // Blocking I/O
    let sheet = book.get_sheet_by_name_mut(sheet_name)
        .ok_or_else(|| anyhow!("sheet '{}' not found", sheet_name))?;

    for edit in edits {  // CPU-bound loop
        let cell = sheet.get_cell_mut(edit.address.as_str());
        if edit.is_formula {
            cell.set_formula(edit.value.clone());
        } else {
            cell.set_value(edit.value.clone());
        }
    }

    umya_spreadsheet::writer::xlsx::write(&book, path)?;  // Blocking I/O
    Ok(())
}
```

**I/O-Bound Example** (src/state.rs:189-190):

```rust
// Loading a workbook is dominated by file I/O
let workbook = task::spawn_blocking(move ||
    WorkbookContext::load(&config, &path_buf)
).await??;
```

### 3.3 LibreOffice Process Management

**Pattern**: Use tokio::process::Command for async process spawning.

```rust
// src/recalc/fire_and_forget.rs:45-57
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
    .output()  // Returns a Future
```

**Why async**: Process spawning and waiting is I/O-bound (waiting for OS), so use tokio's async process support.

**Concurrency Control** (src/recalc/mod.rs:29-36):

```rust
#[derive(Clone)]
pub struct GlobalRecalcLock(pub Arc<Semaphore>);

impl GlobalRecalcLock {
    pub fn new(permits: usize) -> Self {
        Self(Arc::new(Semaphore::new(permits)))
    }
}
```

**Usage Pattern**:

```rust
// Acquire semaphore permit before spawning LibreOffice
let _permit = self.recalc_semaphore()
    .ok_or_else(|| anyhow!("recalc semaphore not available"))?
    .0
    .acquire()
    .await?;

// Now spawn LibreOffice process
let result = self.recalc_backend()
    .ok_or_else(|| anyhow!("recalc backend not available"))?
    .recalculate(&work_path)
    .await?;
```

**TPS Principle**: **Heijunka (Production Leveling)** - Semaphore limits concurrent LibreOffice processes to prevent resource exhaustion.

### 3.4 File System Operations

**Pattern**: Use spawn_blocking for std::fs, or tokio::fs for truly async needs.

**Current Approach** (consistent throughout codebase):

```rust
// Use spawn_blocking when combined with CPU work
tokio::task::spawn_blocking(move || {
    let mut book = umya_spreadsheet::reader::xlsx::read(path)?;
    // ... modify ...
    umya_spreadsheet::writer::xlsx::write(&book, path)?;
    Ok(())
}).await??
```

**Tokio::fs Usage** (src/recalc/screenshot.rs:124-131):

```rust
// Simple file metadata check - use tokio::fs
fs::metadata(&pdf_output_path).await.map_err(|_| {
    anyhow!(
        "screenshot PDF output file not created at {}",
        pdf_output_path.display()
    )
})?;
```

**Rule of Thumb**:
- **spawn_blocking**: When combining std::fs with CPU work or complex parsing
- **tokio::fs**: For simple file operations (metadata, read_to_string, write) when not combined with blocking work

---

## 4. Performance Patterns

### 4.1 Avoiding Async Overhead

**TPS Principle**: **Muda (Waste Elimination)** - Don't go async unless you need it.

**Anti-Pattern**:

```rust
// ✗ BAD: Unnecessary async for pure computation
pub async fn calculate_sum(values: Vec<i32>) -> i32 {
    values.iter().sum()  // Pure CPU work, no I/O
}
```

**Correct Pattern**:

```rust
// ✓ GOOD: Keep it synchronous
pub fn calculate_sum(values: Vec<i32>) -> i32 {
    values.iter().sum()
}

// If called from async context:
pub async fn process_data(state: Arc<AppState>) -> Result<i32> {
    let workbook = state.open_workbook(&id).await?;  // Async needed

    // Extract values (sync)
    let values = extract_values(&workbook);

    // Calculate (sync, inline)
    let sum = calculate_sum(values);

    Ok(sum)
}
```

### 4.2 Future Composition

**tokio::join! - Run futures concurrently**:

```rust
use tokio::join;

// Run multiple independent queries in parallel
let (metrics, formulas, styles) = join!(
    sheet.compute_metrics(),
    sheet.analyze_formulas(),
    sheet.extract_styles()
);
```

**tokio::try_join! - Early termination on first error**:

```rust
use tokio::try_join;

// Stop all if any fails
let (result1, result2, result3) = try_join!(
    async { sheet1.validate()? },
    async { sheet2.validate()? },
    async { sheet3.validate()? }
)?;
```

**tokio::select! - Race futures**:

```rust
use tokio::select;

// First one to complete wins
select! {
    result = primary_source.fetch_data() => {
        tracing::info!("primary source responded");
        result
    }
    result = backup_source.fetch_data() => {
        tracing::warn!("using backup source");
        result
    }
    _ = tokio::time::sleep(Duration::from_secs(5)) => {
        Err(anyhow!("timeout"))
    }
}
```

**Real Example from Codebase** (src/lib.rs:119-131):

```rust
tokio::select! {
    result = server_future.as_mut() => {
        tracing::info!("http transport stopped");
        result.map_err(anyhow::Error::from)?;
        return Ok(());
    }
    ctrl = tokio::signal::ctrl_c() => {
        match ctrl {
            Ok(_) => tracing::info!("shutdown signal received"),
            Err(error) => tracing::warn!(?error, "ctrl_c listener exited unexpectedly"),
        };
    }
}
```

### 4.3 Stream Processing

**Pattern**: Use tokio-stream for processing async sequences.

**Example** (not yet in codebase, recommended for future pagination work):

```rust
use tokio_stream::{self as stream, StreamExt};

// Process workbook sheets as a stream
let sheet_names = workbook.sheet_names();
let mut stream = stream::iter(sheet_names)
    .map(|name| async move {
        // Load sheet asynchronously
        workbook.load_sheet(&name).await
    })
    .buffer_unordered(4);  // Process up to 4 sheets concurrently

while let Some(result) = stream.next().await {
    match result {
        Ok(sheet) => process_sheet(sheet).await?,
        Err(e) => tracing::error!("failed to load sheet: {}", e),
    }
}
```

### 4.4 Buffering and Batching

**Pattern**: Batch operations to reduce overhead.

**Example from codebase** (src/tools/fork.rs:58-124):

```rust
pub struct EditBatchParams {
    pub fork_id: String,
    pub sheet_name: String,
    pub edits: Vec<CellEdit>,  // Batch multiple edits
}

pub async fn edit_batch(
    state: Arc<AppState>,
    params: EditBatchParams,
) -> Result<EditBatchResponse> {
    // Prepare all edits
    let edits_to_apply: Vec<_> = params
        .edits
        .iter()
        .map(|e| EditOp { /* ... */ })
        .collect();

    // Apply entire batch in one spawn_blocking call
    tokio::task::spawn_blocking({
        let sheet_name = params.sheet_name.clone();
        let edits = params.edits.clone();
        move || apply_edits_to_file(&work_path, &sheet_name, &edits)
    })
    .await??;

    Ok(EditBatchResponse {
        edits_applied: edit_count,
        total_edits: total,
        /* ... */
    })
}
```

**TPS Principle**: **Batch Processing** - Reduce setup/teardown overhead by processing multiple items together.

---

## 5. Common Pitfalls

### 5.1 Holding Locks Across .await Points

**Anti-Pattern**:

```rust
// ✗ BAD: Lock held across await
let mut cache = self.cache.lock().await;  // Assume async mutex
let entry = cache.get(&id);
let data = fetch_data().await;  // Other tasks blocked!
cache.insert(id, data);
```

**Correct Pattern**:

```rust
// ✓ GOOD: Minimize lock scope
let entry = {
    let cache = self.cache.read();  // Use parking_lot for sync locks
    cache.get(&id).cloned()
};

if entry.is_none() {
    let data = fetch_data().await;  // No lock held
    let mut cache = self.cache.write();
    cache.insert(id, data);
}
```

**Real Example** (src/state.rs:171-178):

```rust
// ✓ GOOD: Read lock held briefly, no await
{
    let mut cache = self.cache.write();
    if let Some(entry) = cache.get(&canonical) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
        debug!(workbook_id = %canonical, "cache hit");
        return Ok(entry.clone());
    }
}
// Lock released before expensive operations

// Load workbook outside of locks
let workbook = task::spawn_blocking(move ||
    WorkbookContext::load(&config, &path_buf)
).await??;
```

**TPS Principle**: **Flow (Nagare)** - Minimize wait time by releasing resources quickly.

### 5.2 Blocking in Async Contexts

**Anti-Pattern**:

```rust
// ✗ BAD: Blocking call in async function
pub async fn process_workbook(path: PathBuf) -> Result<Workbook> {
    let data = std::fs::read(&path)?;  // Blocks the runtime!
    parse_workbook(data)
}
```

**Correct Pattern**:

```rust
// ✓ GOOD: Use spawn_blocking
pub async fn process_workbook(path: PathBuf) -> Result<Workbook> {
    tokio::task::spawn_blocking(move || {
        let data = std::fs::read(&path)?;
        parse_workbook(data)
    }).await?
}
```

**Detection**: Use `tokio-console` or `tracing` to identify blocking:

```rust
// Add tracing to identify slow operations
let start = std::time::Instant::now();
let result = operation().await;
let duration = start.elapsed();
if duration > Duration::from_millis(10) {
    tracing::warn!(duration_ms = duration.as_millis(), "slow operation");
}
```

### 5.3 Accidental Cloning

**Anti-Pattern**:

```rust
// ✗ BAD: Unnecessary clones in hot path
pub async fn process_sheets(workbook: Arc<WorkbookContext>) -> Result<()> {
    for name in workbook.sheet_names() {
        let workbook_clone = workbook.clone();  // Arc clone is cheap but unnecessary
        process_sheet(workbook_clone, name).await?;
    }
    Ok(())
}
```

**Correct Pattern**:

```rust
// ✓ GOOD: Borrow when possible, clone only when moving into spawn_blocking
pub async fn process_sheets(workbook: Arc<WorkbookContext>) -> Result<()> {
    for name in workbook.sheet_names() {
        // Borrow workbook, clone name (small)
        process_sheet(&workbook, name).await?;
    }
    Ok(())
}

// If you need to move into spawn_blocking:
pub async fn process_sheet_blocking(
    workbook: Arc<WorkbookContext>,
    name: String
) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        // workbook moved here (Arc clone is acceptable)
        compute_metrics(&workbook, &name)
    }).await?
}
```

**Real Example** (src/state.rs:189-190):

```rust
// ✓ GOOD: Arc clone only when moving into spawn_blocking
let config = self.config.clone();  // Arc clone
let path_buf = path.clone();
let workbook_id_clone = canonical.clone();

let workbook = task::spawn_blocking(move ||
    WorkbookContext::load(&config, &path_buf)  // Values moved here
).await??;
```

### 5.4 Stack Size Issues

**Pitfall**: Large data structures on the stack can cause issues in async contexts.

**Problem**:

```rust
// ✗ RISKY: Large buffer on stack
pub async fn process_large_data() -> Result<()> {
    let buffer = [0u8; 1024 * 1024];  // 1MB on stack!
    // ...
}
```

**Solution**:

```rust
// ✓ GOOD: Heap allocate large structures
pub async fn process_large_data() -> Result<()> {
    let buffer = vec![0u8; 1024 * 1024];  // On heap
    // ...
}

// Or use Box for non-Vec types
pub async fn process_complex_state() -> Result<()> {
    let state = Box::new(LargeState::default());
    // ...
}
```

---

## 6. Testing Async Code

### 6.1 #[tokio::test] Patterns

**Basic Test**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workbook_loading() {
        let config = Arc::new(ServerConfig::default());
        let state = Arc::new(AppState::new(config));

        let result = state.open_workbook(&WorkbookId("test".into())).await;
        assert!(result.is_ok());
    }
}
```

**Test with Timeout**:

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_concurrent_access() {
    let state = Arc::new(AppState::new(config));

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let state = state.clone();
            tokio::spawn(async move {
                state.open_workbook(&WorkbookId(format!("wb{}", i))).await
            })
        })
        .collect();

    for handle in handles {
        handle.await.unwrap().unwrap();
    }
}
```

**Test with Mock** (recommended pattern):

```rust
use mockall::predicate::*;
use mockall::mock;

#[async_trait]
pub trait RecalcExecutor: Send + Sync {
    async fn recalculate(&self, workbook_path: &Path) -> Result<RecalcResult>;
    fn is_available(&self) -> bool;
}

mock! {
    pub RecalcExecutor {}

    #[async_trait]
    impl RecalcExecutor for RecalcExecutor {
        async fn recalculate(&self, workbook_path: &Path) -> Result<RecalcResult>;
        fn is_available(&self) -> bool;
    }
}

#[tokio::test]
async fn test_recalc_timeout() {
    let mut mock = MockRecalcExecutor::new();
    mock.expect_recalculate()
        .returning(|_| {
            Box::pin(async {
                tokio::time::sleep(Duration::from_secs(100)).await;
                Ok(RecalcResult { /* ... */ })
            })
        });

    // Test timeout logic
    let result = tokio::time::timeout(
        Duration::from_millis(100),
        mock.recalculate(Path::new("/test.xlsx"))
    ).await;

    assert!(result.is_err());  // Should timeout
}
```

### 6.2 Mocking Async Dependencies

**Pattern**: Use `async_trait` with mockall.

```rust
use async_trait::async_trait;
use mockall::automock;

#[automock]
#[async_trait]
pub trait WorkbookLoader: Send + Sync {
    async fn load(&self, path: &Path) -> Result<WorkbookContext>;
}

#[tokio::test]
async fn test_with_mock_loader() {
    let mut mock = MockWorkbookLoader::new();
    mock.expect_load()
        .with(predicate::eq(Path::new("/test.xlsx")))
        .times(1)
        .returning(|_| {
            Box::pin(async {
                Ok(WorkbookContext::test_fixture())
            })
        });

    let result = mock.load(Path::new("/test.xlsx")).await;
    assert!(result.is_ok());
}
```

### 6.3 Testing Timeouts and Cancellation

**Timeout Test**:

```rust
#[tokio::test]
async fn test_tool_timeout() {
    let config = Arc::new(ServerConfig {
        tool_timeout_ms: Some(100),  // 100ms timeout
        ..Default::default()
    });
    let state = Arc::new(AppState::new(config.clone()));
    let server = SpreadsheetServer::from_state(state);

    // Create a future that takes longer than timeout
    let slow_op = async {
        tokio::time::sleep(Duration::from_secs(1)).await;
        Ok(WorkbookListResponse::default())
    };

    let result = server.run_tool_with_timeout("test", slow_op).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("timed out"));
}
```

**Cancellation Test**:

```rust
#[tokio::test]
async fn test_graceful_shutdown() {
    let (tx, rx) = tokio::sync::oneshot::channel();

    let task = tokio::spawn(async move {
        tokio::select! {
            _ = long_running_operation() => {
                panic!("should have been cancelled");
            }
            _ = rx => {
                // Graceful shutdown
            }
        }
    });

    // Send cancellation signal
    tx.send(()).unwrap();

    // Wait for task to complete
    task.await.unwrap();
}
```

---

## 7. TPS Principles in Async Design

### 7.1 Just-In-Time (JIT) Execution

**Principle**: Execute work only when needed, minimize speculative computation.

**Implementation**:

```rust
// ✓ GOOD: Lazy loading with cache
pub async fn get_sheet_metrics(
    &self,
    sheet_name: &str
) -> Result<Arc<SheetCacheEntry>> {
    // Check cache first (JIT: only compute if needed)
    {
        let cache = self.sheet_cache.read();
        if let Some(entry) = cache.get(sheet_name) {
            return Ok(entry.clone());
        }
    }

    // Compute only on cache miss
    let metrics = self.compute_sheet_metrics(sheet_name)?;
    let entry = Arc::new(SheetCacheEntry { metrics, /* ... */ });

    {
        let mut cache = self.sheet_cache.write();
        cache.insert(sheet_name.to_string(), entry.clone());
    }

    Ok(entry)
}
```

**Anti-Pattern**:

```rust
// ✗ BAD: Eager computation (waste)
pub async fn new(path: PathBuf) -> Result<Self> {
    let workbook = load_workbook(&path).await?;

    // Compute all metrics upfront (may never be used!)
    let mut metrics = HashMap::new();
    for sheet in workbook.sheets() {
        metrics.insert(sheet.name(), compute_metrics(sheet));
    }

    Ok(Self { workbook, metrics })
}
```

### 7.2 Waste Elimination (Muda)

**Seven Wastes in Async Code**:

1. **Overprocessing**: Unnecessary async overhead
   - Solution: Keep pure computation synchronous

2. **Waiting**: Holding locks across await points
   - Solution: Minimize lock scope (see section 5.1)

3. **Transportation**: Excessive cloning/copying
   - Solution: Use Arc for shared state, borrow when possible

4. **Inventory**: Large response payloads
   - Solution: Pagination and size limits (see run_tool_with_timeout)

5. **Motion**: Context switching between threads
   - Solution: Use spawn_blocking appropriately, batch operations

6. **Defects**: Unhandled errors cascading
   - Solution: Comprehensive error handling with anyhow

7. **Overproduction**: Spawning unnecessary tasks
   - Solution: Use join!/try_join! instead of spawning separate tasks

**Example - Response Size Limits** (src/server.rs:243-253):

```rust
fn ensure_response_size<T: Serialize>(&self, tool: &str, value: &T) -> Result<()> {
    let Some(limit) = self.state.config().max_response_bytes() else {
        return Ok(());
    };
    let payload = serde_json::to_vec(value)
        .map_err(|e| anyhow!("failed to serialize response for {}: {}", tool, e))?;
    if payload.len() > limit {
        return Err(ResponseTooLargeError::new(tool, payload.len(), limit).into());
    }
    Ok(())
}
```

### 7.3 Continuous Flow (Nagare)

**Principle**: Keep work flowing smoothly without blockages.

**Implementation** - Semaphore for flow control:

```rust
// src/recalc/mod.rs - Limit concurrent LibreOffice processes
#[derive(Clone)]
pub struct GlobalRecalcLock(pub Arc<Semaphore>);

impl GlobalRecalcLock {
    pub fn new(permits: usize) -> Self {
        Self(Arc::new(Semaphore::new(permits)))
    }
}

// Usage:
let _permit = recalc_semaphore.0.acquire().await?;
// Only N recalc operations can run concurrently
// Others wait smoothly without overwhelming the system
```

**Benefit**: Prevents resource exhaustion while maintaining steady throughput.

### 7.4 Jidoka (Autonomation)

**Principle**: Build quality into the process, stop on defects.

**Implementation** - Validation at boundaries:

```rust
// src/main.rs:11-12 - Fail fast on invalid config
config.validate()?;

// src/server.rs:266-267 - Check authorization before execution
self.ensure_tool_enabled("list_workbooks")
    .map_err(to_mcp_error)?;

// src/server.rs:239 - Check response size
self.ensure_response_size(tool, &result)?;
```

**Async-Specific**: Use timeouts as automatic defect detection:

```rust
// Timeout acts as a circuit breaker
match tokio::time::timeout(timeout_duration, fut).await {
    Ok(result) => result,
    Err(_) => {
        // Automatic detection of "stuck" operations
        tracing::error!(tool = tool, "operation timed out");
        Err(anyhow!("tool '{}' timed out", tool))
    }
}
```

### 7.5 Kaizen (Continuous Improvement)

**Observability for Improvement**:

```rust
// src/state.rs:134-142 - Cache statistics
pub fn cache_stats(&self) -> CacheStats {
    CacheStats {
        operations: self.cache_ops.load(Ordering::Relaxed),
        hits: self.cache_hits.load(Ordering::Relaxed),
        misses: self.cache_misses.load(Ordering::Relaxed),
        size: self.cache.read().len(),
        capacity: self.cache.read().cap().get(),
    }
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
```

**Use Metrics to Guide Optimization**:
- Low cache hit rate → Increase cache size or change eviction policy
- High spawn_blocking wait times → Add more blocking thread pool capacity
- Frequent timeouts → Adjust timeout durations or optimize operations

---

## Appendix A: Quick Reference

### Common Patterns Cheat Sheet

| Pattern | Use Case | Example |
|---------|----------|---------|
| `spawn_blocking` | Sync I/O, CPU work | `tokio::task::spawn_blocking(move \|\| read_file(path)).await??` |
| `tokio::fs` | Simple async I/O | `tokio::fs::read_to_string(path).await?` |
| `tokio::process::Command` | External processes | `Command::new("soffice").output().await?` |
| `tokio::time::timeout` | Prevent runaway ops | `timeout(Duration::from_secs(30), fut).await?` |
| `tokio::select!` | Cancellation, racing | `select! { result = fut => ..., _ = signal => ... }` |
| `Arc<RwLock<T>>` | Shared state | `let data = state.cache.read(); data.get(key)` |
| `Semaphore` | Concurrency limits | `let _permit = sem.acquire().await?; do_work()` |

### Error Handling Patterns

```rust
// Double ?? for spawn_blocking
let result = tokio::task::spawn_blocking(|| operation()).await??;
//                                                       ^^ ^^
//                                                       |  |
//                                                       |  Inner Result
//                                                       Join error
```

### Performance Checklist

- [ ] Async functions only when needed (I/O or concurrency)
- [ ] spawn_blocking for sync I/O and CPU work
- [ ] Locks held for minimal time, never across await
- [ ] Appropriate use of Arc vs Clone
- [ ] Timeouts on external operations
- [ ] Response size limits enforced
- [ ] Concurrency limits via Semaphore
- [ ] Metrics/logging for observability

---

## Appendix B: Further Reading

**Tokio Documentation**:
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Async Book](https://rust-lang.github.io/async-book/)
- [tokio::task::spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)

**TPS Resources**:
- `docs/TPS_RESEARCH_COMPLETE.md` - TPS principles for software
- `docs/TPS_WASTE_ELIMINATION.md` - Identifying waste in code
- `docs/TPS_STANDARDIZED_WORK.md` - Standardized async patterns

**Codebase Examples**:
- `src/server.rs` - Tool handler patterns
- `src/state.rs` - State management with locks
- `src/recalc/fire_and_forget.rs` - Process spawning
- `src/tools/fork.rs` - spawn_blocking usage
- `examples/async_mcp_patterns.rs` - Comprehensive examples

---

## Version History

- **v1.0** (2026-01-20): Initial comprehensive guide
  - Documented existing patterns from ggen-mcp codebase
  - Integrated TPS principles
  - Added examples and anti-patterns
  - Created testing section
