# Recovery Module

This module provides graceful error recovery and fallback mechanisms for the spreadsheet MCP server.

## Features

### 1. Retry Logic for LibreOffice Recalc Operations

Automatic retry with exponential backoff for transient failures:

```rust
use spreadsheet_mcp::recovery::{RetryConfig, ExponentialBackoff, retry_async_with_policy};

let config = RetryConfig::recalc();
let policy = ExponentialBackoff::new(config);

let result = retry_async_with_policy(
    || async { recalc_executor.recalculate(&path).await },
    &policy,
    "recalculate_workbook"
).await?;
```

### 2. Circuit Breaker Pattern

Protect against cascading failures with automatic circuit breaking:

```rust
use spreadsheet_mcp::recovery::{CircuitBreaker, CircuitBreakerConfig};

let config = CircuitBreakerConfig::recalc();
let circuit_breaker = CircuitBreaker::new("recalc_executor", config);

let result = circuit_breaker.execute_async(|| async {
    recalc_executor.recalculate(&path).await
}).await?;
```

### 3. Fallback for Failed Region Detection

Graceful degradation when region detection times out or exceeds complexity limits:

```rust
use spreadsheet_mcp::recovery::RegionDetectionFallback;

let fallback = RegionDetectionFallback::default();

let regions = match detect_regions(sheet, metrics) {
    Ok(detected) => detected,
    Err(err) if fallback.should_use_fallback(metrics.non_empty_cells as usize, Some(&err)) => {
        // Use simplified region as fallback
        let simple = fallback.create_simple_region(
            metrics.row_count,
            metrics.column_count,
            metrics.non_empty_cells
        );
        vec![simple]
    }
    Err(err) => return Err(err),
};
```

### 4. Partial Success for Batch Operations

Handle batch operations where some items succeed and others fail:

```rust
use spreadsheet_mcp::recovery::PartialSuccessHandler;

let handler = PartialSuccessHandler::new()
    .max_errors(10);  // Stop after 10 errors

let result = handler.process_batch_async(edits, |index, edit| async move {
    apply_edit_to_file(&work_path, &edit).await
}).await;

if result.is_partial_success() {
    println!(
        "Applied {}/{} edits successfully",
        result.summary.success_count,
        result.total
    );
}
```

### 5. Workbook State Recovery

Detect and recover from corrupted workbook state:

```rust
use spreadsheet_mcp::recovery::{WorkbookRecoveryStrategy, CorruptionDetector};

let strategy = WorkbookRecoveryStrategy::new(true); // Enable backups

// Check for corruption
let action = strategy.determine_action(&workbook_path)?;

// Execute recovery
match strategy.execute_recovery(&workbook_path, action)? {
    RecoveryResult::Restored { from } => {
        println!("Restored from backup: {}", from);
    }
    RecoveryResult::Corrupted => {
        println!("Workbook is corrupted and cannot be recovered");
    }
    _ => {}
}
```

## Configuration

### Retry Policies

Three preset configurations are available:

- `RetryConfig::recalc()` - For LibreOffice operations (5 attempts, 30s max delay)
- `RetryConfig::file_io()` - For file operations (3 attempts, 5s max delay)
- `RetryConfig::network()` - For network-like operations (4 attempts, 15s max delay)

### Circuit Breaker

- `CircuitBreakerConfig::recalc()` - For recalc operations
  - Failure threshold: 3
  - Success threshold (half-open): 2
  - Timeout: 30 seconds

- `CircuitBreakerConfig::file_io()` - For file operations
  - Failure threshold: 5
  - Success threshold: 3
  - Timeout: 15 seconds

## Error Recovery Strategies

The module automatically determines the appropriate recovery strategy based on error type:

| Error Type | Strategy |
|------------|----------|
| Timeout | Retry with exponential backoff |
| File not found | Fallback to simpler operation |
| Corrupted data | Use backup or fallback |
| Resource exhaustion | Retry with backoff |
| Batch operation | Partial success |

## Integration Examples

### Recalc Executor with Circuit Breaker and Retry

```rust
use spreadsheet_mcp::recovery::{
    CircuitBreaker, CircuitBreakerConfig,
    RetryConfig, ExponentialBackoff, retry_async_with_policy
};

pub struct ResilientRecalcExecutor {
    inner: Arc<dyn RecalcExecutor>,
    circuit_breaker: CircuitBreaker,
}

impl ResilientRecalcExecutor {
    pub fn new(executor: Arc<dyn RecalcExecutor>) -> Self {
        let config = CircuitBreakerConfig::recalc();
        let circuit_breaker = CircuitBreaker::new("recalc", config);

        Self {
            inner: executor,
            circuit_breaker,
        }
    }

    pub async fn recalculate_with_recovery(&self, path: &Path) -> Result<RecalcResult> {
        let executor = self.inner.clone();
        let path = path.to_path_buf();

        // Use circuit breaker + retry
        self.circuit_breaker.execute_async(|| {
            let executor = executor.clone();
            let path = path.clone();

            async move {
                let policy = ExponentialBackoff::new(RetryConfig::recalc());
                retry_async_with_policy(
                    || {
                        let executor = executor.clone();
                        let path = path.clone();
                        async move { executor.recalculate(&path).await }
                    },
                    &policy,
                    "recalculate"
                ).await
            }
        }).await
    }
}
```

### Batch Edit with Partial Success

```rust
use spreadsheet_mcp::recovery::{PartialSuccessHandler, BatchResult};

pub async fn edit_batch_resilient(
    work_path: &Path,
    edits: Vec<CellEdit>,
) -> Result<BatchResult<CellEdit>> {
    let handler = PartialSuccessHandler::new()
        .max_errors(20);  // Allow up to 20 errors

    let result = handler.process_batch_async(edits, |index, edit| {
        let work_path = work_path.to_path_buf();
        async move {
            apply_single_edit(&work_path, &edit).await?;
            Ok(edit)
        }
    }).await;

    Ok(result)
}
```

### Region Detection with Fallback

```rust
use spreadsheet_mcp::recovery::{RegionDetectionFallback, GracefulDegradation};

pub fn detect_regions_with_fallback(
    sheet: &Worksheet,
    metrics: &SheetMetrics,
) -> Result<Vec<DetectedRegion>> {
    let fallback_strategy = RegionDetectionFallback::default();

    GracefulDegradation::new("region_detection")
        .primary(|| detect_regions_complex(sheet, metrics))
        .fallback(|| {
            // Use simple bounds-based fallback
            let simple = fallback_strategy.create_simple_region(
                metrics.row_count,
                metrics.column_count,
                metrics.non_empty_cells
            );
            Ok(vec![convert_to_detected_region(simple)])
        })
        .execute()
}
```

## Testing

Each recovery component includes comprehensive unit tests:

```bash
cargo test --package spreadsheet-mcp --lib recovery
```

## Best Practices

1. **Use Circuit Breakers for External Services**: Protect against cascading failures when calling LibreOffice or other external services.

2. **Retry Transient Failures**: Use exponential backoff for timeout and resource exhaustion errors.

3. **Implement Fallbacks**: Provide degraded functionality rather than complete failure.

4. **Track Partial Success**: For batch operations, continue processing after individual failures.

5. **Monitor Recovery Metrics**: Log retry attempts, circuit breaker state changes, and fallback usage.

6. **Set Appropriate Timeouts**: Configure retry and circuit breaker timeouts based on operation characteristics.

## License

Apache-2.0
