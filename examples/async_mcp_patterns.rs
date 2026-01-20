/*!
# Async MCP Patterns - Comprehensive Examples

This file demonstrates async/await best practices for MCP servers in Rust,
specifically tailored for the ggen-mcp project. Each example is self-contained
and demonstrates a specific pattern or principle.

## Running Examples

```bash
# Run a specific example
cargo run --example async_mcp_patterns -- basic_tool_handler

# List all examples
cargo run --example async_mcp_patterns -- --list
```

## Categories

1. Basic Patterns
   - Tool handler structure
   - Error handling
   - Timeout patterns

2. Blocking Operations
   - spawn_blocking usage
   - File I/O patterns
   - CPU-bound work

3. Concurrency Control
   - Semaphores
   - Rate limiting
   - Resource pooling

4. State Management
   - Arc + RwLock patterns
   - Cache patterns
   - Lock scope minimization

5. Testing
   - Async test patterns
   - Mocking
   - Timeout testing
*/

use anyhow::{Result, anyhow};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::timeout;

// ============================================================================
// EXAMPLE 1: Basic Tool Handler Pattern
// ============================================================================

/// Example parameter structure for an MCP tool
#[derive(Debug, Deserialize)]
struct BasicToolParams {
    workbook_id: String,
    sheet_name: String,
}

/// Example response structure
#[derive(Debug, Serialize)]
struct BasicToolResponse {
    success: bool,
    row_count: u32,
    message: String,
}

/// Basic tool handler following MCP patterns
///
/// Key principles:
/// 1. Async function signature
/// 2. Return Result<T> for error handling
/// 3. Use Arc for shared state
/// 4. Minimal async overhead - only async when needed
async fn basic_tool_handler(
    state: Arc<ExampleState>,
    params: BasicToolParams,
) -> Result<BasicToolResponse> {
    // Step 1: Validation (sync, inline)
    if params.sheet_name.is_empty() {
        return Err(anyhow!("sheet_name cannot be empty"));
    }

    // Step 2: Load resource (async, may be cached)
    let workbook = state.get_workbook(&params.workbook_id).await?;

    // Step 3: CPU-bound work in spawn_blocking
    let sheet_name = params.sheet_name.clone();
    let row_count =
        tokio::task::spawn_blocking(move || count_rows(&workbook, &sheet_name)).await??;

    // Step 4: Return response
    Ok(BasicToolResponse {
        success: true,
        row_count,
        message: format!("Processed sheet '{}'", params.sheet_name),
    })
}

fn count_rows(_workbook: &ExampleWorkbook, _sheet_name: &str) -> Result<u32> {
    // Simulate CPU work
    std::thread::sleep(Duration::from_millis(10));
    Ok(42)
}

// ============================================================================
// EXAMPLE 2: Error Handling and Conversion
// ============================================================================

/// Custom error type for MCP boundaries
#[derive(Debug, thiserror::Error)]
enum McpError {
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Operation timed out after {0}ms")]
    Timeout(u64),
}

/// Convert anyhow::Error to McpError at boundaries
fn to_mcp_error(error: anyhow::Error) -> McpError {
    // Check for specific error types
    if let Some(not_found) = error.downcast_ref::<NotFoundError>() {
        return McpError::NotFound(not_found.0.clone());
    }

    // Default to internal error
    McpError::Internal(error.to_string())
}

#[derive(Debug, thiserror::Error)]
#[error("Not found: {0}")]
struct NotFoundError(String);

/// Tool handler with comprehensive error handling
async fn error_handling_example(
    state: Arc<ExampleState>,
    workbook_id: String,
) -> Result<BasicToolResponse, McpError> {
    // Internal functions use anyhow::Result
    let result = internal_operation(state, workbook_id)
        .await
        .map_err(to_mcp_error)?;

    Ok(result)
}

async fn internal_operation(
    state: Arc<ExampleState>,
    workbook_id: String,
) -> Result<BasicToolResponse> {
    let workbook = state
        .get_workbook(&workbook_id)
        .await
        .map_err(|_| NotFoundError(workbook_id))?;

    Ok(BasicToolResponse {
        success: true,
        row_count: count_rows(&workbook, "Sheet1")?,
        message: "Success".to_string(),
    })
}

// ============================================================================
// EXAMPLE 3: Timeout Patterns
// ============================================================================

/// Wrapper that applies timeout to any future
async fn run_with_timeout<T>(
    operation_name: &str,
    timeout_ms: u64,
    fut: impl std::future::Future<Output = Result<T>>,
) -> Result<T, McpError> {
    match timeout(Duration::from_millis(timeout_ms), fut).await {
        Ok(result) => result.map_err(to_mcp_error),
        Err(_) => Err(McpError::Timeout(timeout_ms)),
    }
}

/// Tool handler with timeout
async fn tool_with_timeout(
    state: Arc<ExampleState>,
    params: BasicToolParams,
) -> Result<BasicToolResponse, McpError> {
    run_with_timeout(
        "basic_tool",
        5000, // 5 second timeout
        basic_tool_handler(state, params),
    )
    .await
}

// ============================================================================
// EXAMPLE 4: spawn_blocking for File I/O
// ============================================================================

/// Load a file using spawn_blocking (correct pattern)
async fn load_file_blocking(path: PathBuf) -> Result<Vec<u8>> {
    tokio::task::spawn_blocking(move || {
        std::fs::read(&path).map_err(|e| anyhow!("failed to read {:?}: {}", path, e))
    })
    .await?
}

/// Parse a workbook file (blocking I/O + CPU work)
async fn load_workbook_file(path: PathBuf) -> Result<ExampleWorkbook> {
    tokio::task::spawn_blocking(move || {
        // Step 1: Blocking file I/O
        let data = std::fs::read(&path).map_err(|e| anyhow!("failed to read workbook: {}", e))?;

        // Step 2: CPU-intensive parsing
        parse_workbook_data(&data)
    })
    .await?
}

fn parse_workbook_data(data: &[u8]) -> Result<ExampleWorkbook> {
    // Simulate parsing work
    std::thread::sleep(Duration::from_millis(50));

    Ok(ExampleWorkbook {
        id: format!("wb_{}", data.len()),
        path: PathBuf::from("/tmp/example.xlsx"),
        sheets: vec!["Sheet1".to_string()],
    })
}

/// Edit a workbook file (blocking I/O + CPU work)
async fn edit_workbook_file(path: PathBuf, sheet_name: String, edits: Vec<CellEdit>) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        // Load workbook (blocking I/O)
        let mut workbook = load_workbook_sync(&path)?;

        // Apply edits (CPU work)
        apply_edits_sync(&mut workbook, &sheet_name, &edits)?;

        // Save workbook (blocking I/O)
        save_workbook_sync(&workbook, &path)?;

        Ok(())
    })
    .await?
}

// ============================================================================
// EXAMPLE 5: CPU-Bound Work in spawn_blocking
// ============================================================================

/// Compute-intensive operation that should use spawn_blocking
async fn compute_sheet_statistics(workbook: Arc<ExampleWorkbook>) -> Result<SheetStats> {
    // Clone Arc to move into spawn_blocking
    tokio::task::spawn_blocking(move || {
        let mut stats = SheetStats::default();

        // Simulate CPU-intensive computation
        for _ in 0..1000 {
            std::thread::sleep(Duration::from_micros(100));
            stats.cell_count += 1;
        }

        Ok(stats)
    })
    .await?
}

/// Anti-pattern: DON'T do this
#[allow(dead_code)]
async fn compute_sheet_statistics_bad(workbook: Arc<ExampleWorkbook>) -> Result<SheetStats> {
    // ✗ BAD: Blocking the async runtime!
    let mut stats = SheetStats::default();
    for _ in 0..1000 {
        std::thread::sleep(Duration::from_micros(100)); // Blocks!
        stats.cell_count += 1;
    }
    Ok(stats)
}

// ============================================================================
// EXAMPLE 6: Concurrency Control with Semaphores
// ============================================================================

/// Semaphore-based rate limiting for expensive operations
struct RateLimiter {
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
}

impl RateLimiter {
    fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
        }
    }

    /// Execute operation with rate limiting
    async fn execute<T, F>(&self, op: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        // Acquire permit (may wait if at capacity)
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| anyhow!("failed to acquire permit: {}", e))?;

        // Execute operation while holding permit
        op.await
    }
}

/// Example: Limit concurrent LibreOffice processes
async fn recalculate_with_rate_limit(
    limiter: Arc<RateLimiter>,
    workbook_path: PathBuf,
) -> Result<()> {
    limiter
        .execute(async move {
            // Only N of these can run concurrently
            spawn_libreoffice_process(workbook_path).await
        })
        .await
}

async fn spawn_libreoffice_process(path: PathBuf) -> Result<()> {
    // Simulate LibreOffice process
    tokio::time::sleep(Duration::from_millis(100)).await;
    println!("Recalculated: {:?}", path);
    Ok(())
}

// ============================================================================
// EXAMPLE 7: State Management with Arc + RwLock
// ============================================================================

/// Shared application state
struct ExampleState {
    /// Workbook cache with RwLock for concurrent reads
    cache: RwLock<HashMap<String, Arc<ExampleWorkbook>>>,

    /// Rate limiter for expensive operations
    rate_limiter: Arc<RateLimiter>,

    /// Configuration
    config: ExampleConfig,
}

#[derive(Clone)]
struct ExampleConfig {
    max_concurrent_ops: usize,
    cache_capacity: usize,
}

impl ExampleState {
    fn new(config: ExampleConfig) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            rate_limiter: Arc::new(RateLimiter::new(config.max_concurrent_ops)),
            config,
        }
    }

    /// Get workbook from cache or load it
    async fn get_workbook(&self, id: &str) -> Result<Arc<ExampleWorkbook>> {
        // Step 1: Try cache (read lock only)
        {
            let cache = self.cache.read();
            if let Some(workbook) = cache.get(id) {
                return Ok(workbook.clone());
            }
        } // Lock released here

        // Step 2: Cache miss - load workbook
        let path = self.resolve_path(id)?;
        let workbook = load_workbook_file(path).await?;
        let workbook = Arc::new(workbook);

        // Step 3: Insert into cache (write lock)
        {
            let mut cache = self.cache.write();
            cache.insert(id.to_string(), workbook.clone());
        }

        Ok(workbook)
    }

    fn resolve_path(&self, id: &str) -> Result<PathBuf> {
        // Simulate path resolution
        Ok(PathBuf::from(format!("/tmp/{}.xlsx", id)))
    }

    /// Evict workbook from cache
    fn evict_workbook(&self, id: &str) {
        let mut cache = self.cache.write();
        cache.remove(id);
    }
}

// ============================================================================
// EXAMPLE 8: Lock Scope Minimization
// ============================================================================

/// Anti-pattern: Lock held too long
#[allow(dead_code)]
async fn bad_lock_pattern(state: Arc<ExampleState>, id: &str) -> Result<u32> {
    // ✗ BAD: Lock held across await point
    let cache = state.cache.write();
    let workbook = cache.get(id).cloned();

    if let Some(wb) = workbook {
        // Still holding lock!
        let count = compute_sheet_statistics(wb).await?;
        return Ok(count.cell_count);
    }
    // Lock held until here!

    Ok(0)
}

/// Good pattern: Minimal lock scope
async fn good_lock_pattern(state: Arc<ExampleState>, id: &str) -> Result<u32> {
    // ✓ GOOD: Lock held briefly
    let workbook = {
        let cache = state.cache.read();
        cache.get(id).cloned()
    }; // Lock released immediately

    if let Some(wb) = workbook {
        let count = compute_sheet_statistics(wb).await?; // No lock held
        return Ok(count.cell_count);
    }

    Ok(0)
}

// ============================================================================
// EXAMPLE 9: Future Composition
// ============================================================================

/// Use join! to run independent operations concurrently
async fn parallel_analysis(workbook: Arc<ExampleWorkbook>) -> Result<WorkbookAnalysis> {
    use tokio::join;

    // All three run concurrently
    let (stats, formulas, styles) = join!(
        compute_sheet_statistics(workbook.clone()),
        analyze_formulas(workbook.clone()),
        extract_styles(workbook.clone())
    );

    Ok(WorkbookAnalysis {
        stats: stats?,
        formula_count: formulas?,
        style_count: styles?,
    })
}

/// Use try_join! for early termination on error
async fn parallel_validation(workbooks: Vec<Arc<ExampleWorkbook>>) -> Result<()> {
    use tokio::try_join;

    if workbooks.len() >= 3 {
        // All must succeed, or first error stops all
        try_join!(
            validate_workbook(workbooks[0].clone()),
            validate_workbook(workbooks[1].clone()),
            validate_workbook(workbooks[2].clone())
        )?;
    }

    Ok(())
}

/// Use select! to race futures or handle cancellation
async fn operation_with_cancellation(
    workbook: Arc<ExampleWorkbook>,
    cancel_token: tokio::sync::oneshot::Receiver<()>,
) -> Result<SheetStats> {
    use tokio::select;

    select! {
        result = compute_sheet_statistics(workbook) => {
            result
        }
        _ = cancel_token => {
            Err(anyhow!("operation cancelled"))
        }
    }
}

// ============================================================================
// EXAMPLE 10: Process Management
// ============================================================================

/// Spawn external process asynchronously
async fn run_external_tool(
    tool_path: &str,
    args: Vec<String>,
    timeout_secs: u64,
) -> Result<String> {
    use tokio::process::Command;

    let output = tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        Command::new(tool_path).args(&args).output(),
    )
    .await
    .map_err(|_| anyhow!("process timed out after {}s", timeout_secs))?
    .map_err(|e| anyhow!("failed to spawn process: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "process failed with exit code {}: {}",
            output.status.code().unwrap_or(-1),
            stderr
        ));
    }

    let stdout =
        String::from_utf8(output.stdout).map_err(|e| anyhow!("invalid UTF-8 in output: {}", e))?;

    Ok(stdout)
}

// ============================================================================
// EXAMPLE 11: Batching Operations
// ============================================================================

/// Batch edits to reduce overhead
struct EditBatch {
    fork_id: String,
    sheet_name: String,
    edits: Vec<CellEdit>,
}

#[derive(Clone)]
struct CellEdit {
    address: String,
    value: String,
    is_formula: bool,
}

/// Apply batch of edits in single spawn_blocking call
async fn apply_edit_batch(batch: EditBatch) -> Result<usize> {
    let edit_count = batch.edits.len();

    tokio::task::spawn_blocking(move || {
        // Load workbook once
        let path = PathBuf::from(format!("/tmp/{}.xlsx", batch.fork_id));
        let mut workbook = load_workbook_sync(&path)?;

        // Apply all edits
        apply_edits_sync(&mut workbook, &batch.sheet_name, &batch.edits)?;

        // Save once
        save_workbook_sync(&workbook, &path)?;

        Ok(edit_count)
    })
    .await?
}

/// Anti-pattern: Individual operations
#[allow(dead_code)]
async fn apply_edits_individually(batch: EditBatch) -> Result<usize> {
    // ✗ BAD: Multiple spawn_blocking calls, loading workbook each time
    let mut count = 0;
    for edit in batch.edits {
        let fork_id = batch.fork_id.clone();
        let sheet_name = batch.sheet_name.clone();

        tokio::task::spawn_blocking(move || {
            let path = PathBuf::from(format!("/tmp/{}.xlsx", fork_id));
            let mut workbook = load_workbook_sync(&path)?; // Load every time!
            apply_edits_sync(&mut workbook, &sheet_name, &[edit])?;
            save_workbook_sync(&workbook, &path)?; // Save every time!
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        count += 1;
    }
    Ok(count)
}

// ============================================================================
// EXAMPLE 12: Testing Patterns
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Basic async test
    #[tokio::test]
    async fn test_basic_tool_handler() {
        let config = ExampleConfig {
            max_concurrent_ops: 2,
            cache_capacity: 10,
        };
        let state = Arc::new(ExampleState::new(config));

        let params = BasicToolParams {
            workbook_id: "test".to_string(),
            sheet_name: "Sheet1".to_string(),
        };

        let result = basic_tool_handler(state, params).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.success);
    }

    /// Test with timeout
    #[tokio::test]
    async fn test_timeout_handling() {
        let config = ExampleConfig {
            max_concurrent_ops: 2,
            cache_capacity: 10,
        };
        let state = Arc::new(ExampleState::new(config));

        let params = BasicToolParams {
            workbook_id: "test".to_string(),
            sheet_name: "Sheet1".to_string(),
        };

        // Should complete within timeout
        let result = run_with_timeout("test", 5000, basic_tool_handler(state, params)).await;

        assert!(result.is_ok());
    }

    /// Test concurrent access
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_concurrent_cache_access() {
        let config = ExampleConfig {
            max_concurrent_ops: 2,
            cache_capacity: 10,
        };
        let state = Arc::new(ExampleState::new(config));

        // Spawn multiple tasks accessing cache
        let mut handles = vec![];
        for i in 0..10 {
            let state = state.clone();
            let handle = tokio::spawn(async move {
                let id = format!("wb_{}", i % 3); // Access 3 workbooks
                state.get_workbook(&id).await
            });
            handles.push(handle);
        }

        // All should succeed
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }
    }

    /// Test semaphore rate limiting
    #[tokio::test]
    async fn test_rate_limiting() {
        let limiter = Arc::new(RateLimiter::new(2));

        // Track concurrent executions
        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let mut handles = vec![];
        for i in 0..5 {
            let limiter = limiter.clone();
            let counter = counter.clone();

            let handle = tokio::spawn(async move {
                limiter
                    .execute(async {
                        let current = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        assert!(
                            current < 2,
                            "More than 2 concurrent operations at task {}",
                            i
                        );

                        tokio::time::sleep(Duration::from_millis(50)).await;

                        counter.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                        Ok::<_, anyhow::Error>(())
                    })
                    .await
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap().unwrap();
        }
    }
}

// ============================================================================
// Supporting Types and Functions
// ============================================================================

#[derive(Debug, Clone)]
struct ExampleWorkbook {
    id: String,
    path: PathBuf,
    sheets: Vec<String>,
}

#[derive(Debug, Default)]
struct SheetStats {
    cell_count: u32,
    formula_count: u32,
    style_count: u32,
}

#[derive(Debug)]
struct WorkbookAnalysis {
    stats: SheetStats,
    formula_count: u32,
    style_count: u32,
}

async fn analyze_formulas(_workbook: Arc<ExampleWorkbook>) -> Result<u32> {
    tokio::time::sleep(Duration::from_millis(10)).await;
    Ok(5)
}

async fn extract_styles(_workbook: Arc<ExampleWorkbook>) -> Result<u32> {
    tokio::time::sleep(Duration::from_millis(10)).await;
    Ok(3)
}

async fn validate_workbook(_workbook: Arc<ExampleWorkbook>) -> Result<()> {
    tokio::time::sleep(Duration::from_millis(10)).await;
    Ok(())
}

fn load_workbook_sync(_path: &Path) -> Result<ExampleWorkbook> {
    std::thread::sleep(Duration::from_millis(10));
    Ok(ExampleWorkbook {
        id: "wb_123".to_string(),
        path: PathBuf::from("/tmp/test.xlsx"),
        sheets: vec!["Sheet1".to_string()],
    })
}

fn apply_edits_sync(
    _workbook: &mut ExampleWorkbook,
    _sheet_name: &str,
    _edits: &[CellEdit],
) -> Result<()> {
    std::thread::sleep(Duration::from_millis(10));
    Ok(())
}

fn save_workbook_sync(_workbook: &ExampleWorkbook, _path: &Path) -> Result<()> {
    std::thread::sleep(Duration::from_millis(10));
    Ok(())
}

// ============================================================================
// Main Function - Example Runner
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("Async MCP Patterns - Example Runner");
    println!("====================================\n");

    // Run example demonstrations
    demonstrate_basic_patterns().await?;
    demonstrate_concurrency().await?;
    demonstrate_error_handling().await?;

    println!("\n✓ All examples completed successfully");

    Ok(())
}

async fn demonstrate_basic_patterns() -> Result<()> {
    println!("1. Basic Tool Handler Pattern");
    println!("------------------------------");

    let config = ExampleConfig {
        max_concurrent_ops: 2,
        cache_capacity: 10,
    };
    let state = Arc::new(ExampleState::new(config));

    let params = BasicToolParams {
        workbook_id: "demo".to_string(),
        sheet_name: "Sheet1".to_string(),
    };

    let response = basic_tool_handler(state, params).await?;
    println!("✓ Tool executed: {}", response.message);
    println!("  Row count: {}", response.row_count);
    println!();

    Ok(())
}

async fn demonstrate_concurrency() -> Result<()> {
    println!("2. Concurrency Control");
    println!("----------------------");

    let limiter = Arc::new(RateLimiter::new(2));
    println!("✓ Created rate limiter with 2 concurrent permits");

    // Spawn multiple operations
    let mut handles = vec![];
    for i in 0..5 {
        let limiter = limiter.clone();
        let handle = tokio::spawn(async move {
            limiter
                .execute(async move {
                    println!("  Task {} started", i);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    println!("  Task {} completed", i);
                    Ok::<_, anyhow::Error>(())
                })
                .await
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await??;
    }

    println!("✓ All tasks completed with rate limiting\n");

    Ok(())
}

async fn demonstrate_error_handling() -> Result<()> {
    println!("3. Error Handling");
    println!("-----------------");

    let config = ExampleConfig {
        max_concurrent_ops: 2,
        cache_capacity: 10,
    };
    let state = Arc::new(ExampleState::new(config));

    // Test successful case
    match error_handling_example(state.clone(), "valid_id".to_string()).await {
        Ok(response) => println!("✓ Success: {}", response.message),
        Err(e) => println!("✗ Error: {}", e),
    }

    // Test error conversion
    match error_handling_example(state, "invalid_id".to_string()).await {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("✓ Error handled correctly: {}", e),
    }

    println!();
    Ok(())
}
