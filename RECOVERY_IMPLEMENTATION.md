# Recovery Implementation Guide

This document describes the implementation of graceful error recovery and fallback mechanisms for the spreadsheet MCP server.

## Overview

The recovery module (`src/recovery/`) provides comprehensive error handling and recovery strategies:

1. **Retry Logic** - Exponential backoff for transient failures
2. **Circuit Breaker** - Protection against cascading failures
3. **Fallback Strategies** - Graceful degradation when primary operations fail
4. **Partial Success** - Continue batch operations when individual items fail
5. **Workbook Recovery** - Detect and recover from corrupted workbook state

## Module Structure

```
src/recovery/
├── mod.rs                    # Main module with core recovery types
├── retry.rs                  # Retry policies and exponential backoff
├── circuit_breaker.rs        # Circuit breaker pattern implementation
├── fallback.rs              # Fallback strategies for operations
├── partial_success.rs       # Batch operation partial success handling
├── workbook_recovery.rs     # Workbook corruption detection and recovery
└── README.md                # User documentation
```

## Key Components

### 1. Retry Logic (`retry.rs`)

**Purpose**: Automatically retry operations that fail due to transient errors.

**Features**:
- Exponential backoff with jitter
- Configurable retry policies
- Support for both sync and async operations
- Preset configurations for common scenarios (recalc, file I/O, network)

**Key Types**:
- `RetryConfig` - Configuration for retry behavior
- `RetryPolicy` trait - Determines retry behavior
- `ExponentialBackoff` - Exponential backoff implementation
- `retry_with_policy()` - Execute with retry logic
- `retry_async_with_policy()` - Async version

**Usage**:
```rust
let policy = ExponentialBackoff::new(RetryConfig::recalc());
let result = retry_async_with_policy(
    || recalc_executor.recalculate(&path),
    &policy,
    "recalculate"
).await?;
```

### 2. Circuit Breaker (`circuit_breaker.rs`)

**Purpose**: Prevent cascading failures by failing fast when a service is unhealthy.

**Features**:
- Three states: Closed (healthy), Open (failing), Half-Open (testing)
- Automatic state transitions based on failure/success counts
- Configurable thresholds and timeouts
- Statistics tracking
- Support for both sync and async operations

**Key Types**:
- `CircuitBreaker` - Main circuit breaker implementation
- `CircuitBreakerConfig` - Configuration
- `CircuitBreakerState` - Current state enum
- `CircuitBreakerStats` - Statistics

**State Transitions**:
1. **Closed → Open**: After N consecutive failures
2. **Open → Half-Open**: After timeout duration
3. **Half-Open → Closed**: After M consecutive successes
4. **Half-Open → Open**: On any failure

**Usage**:
```rust
let cb = CircuitBreaker::new("recalc", CircuitBreakerConfig::recalc());
let result = cb.execute_async(|| recalc_executor.recalculate(&path)).await?;
```

### 3. Fallback Strategies (`fallback.rs`)

**Purpose**: Provide degraded functionality when primary operations fail.

**Features**:
- Region detection fallback (simple bounds-based)
- Recalc operation fallback (use cached values)
- Generic fallback executor
- Automatic fallback decision based on error type

**Key Types**:
- `RegionDetectionFallback` - Simplified region detection
- `RecalcFallback` - Recalc operation fallback
- `FallbackExecutor` - Generic fallback pattern
- `AsyncFallbackExecutor` - Async version

**Usage**:
```rust
let fallback = RegionDetectionFallback::default();
if fallback.should_use_fallback(cell_count, Some(&err)) {
    let simple = fallback.create_simple_region(rows, cols, cells);
    return Ok(vec![simple]);
}
```

### 4. Partial Success (`partial_success.rs`)

**Purpose**: Handle batch operations where some items succeed and others fail.

**Features**:
- Track successes and failures separately
- Configurable error thresholds
- Fail-fast or continue-on-error modes
- Detailed batch statistics
- Support for both sync and async batch processing

**Key Types**:
- `BatchResult<T>` - Result with partial success tracking
- `PartialSuccessHandler` - Batch processor
- `BatchFailure` - Information about failed items
- `BatchSummary` - Statistics

**Usage**:
```rust
let handler = PartialSuccessHandler::new().max_errors(10);
let result = handler.process_batch_async(edits, |idx, edit| async {
    apply_edit(&edit).await
}).await;

println!("Applied {}/{} edits",
    result.summary.success_count,
    result.total
);
```

### 5. Workbook Recovery (`workbook_recovery.rs`)

**Purpose**: Detect and recover from corrupted workbook state.

**Features**:
- File corruption detection (size, signature validation)
- Automatic backup creation
- Recovery action determination
- Restore from backup capability

**Key Types**:
- `WorkbookRecoveryStrategy` - Main recovery orchestrator
- `CorruptionDetector` - Detect file corruption
- `CorruptionStatus` - Corruption status enum
- `RecoveryAction` - Action to take
- `RecoveryResult` - Result of recovery

**Detection Checks**:
- File existence
- File size (min/max bounds)
- File signature validation (ZIP for XLSX, OLE for XLS)
- Format validation

**Usage**:
```rust
let strategy = WorkbookRecoveryStrategy::new(true);
let action = strategy.determine_action(&path)?;
strategy.execute_recovery(&path, action)?;
```

## Integration Points

### 1. Recalc Executor Enhancement

**Location**: `src/recalc/fire_and_forget.rs` or create new wrapper

**Integration**:
```rust
use crate::recovery::{CircuitBreaker, CircuitBreakerConfig, RetryConfig,
                      ExponentialBackoff, retry_async_with_policy};

pub struct ResilientFireAndForgetExecutor {
    inner: FireAndForgetExecutor,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl RecalcExecutor for ResilientFireAndForgetExecutor {
    async fn recalculate(&self, path: &Path) -> Result<RecalcResult> {
        self.circuit_breaker.execute_async(|| {
            let inner = self.inner.clone();
            let path = path.to_path_buf();
            async move {
                let policy = ExponentialBackoff::new(RetryConfig::recalc());
                retry_async_with_policy(
                    || inner.recalculate(&path),
                    &policy,
                    "recalculate"
                ).await
            }
        }).await
    }
}
```

### 2. Region Detection Fallback

**Location**: `src/workbook.rs` in `detect_regions()` function

**Integration**:
```rust
use crate::recovery::RegionDetectionFallback;

fn detect_regions(sheet: &Worksheet, metrics: &SheetMetrics) -> DetectRegionsResult {
    let fallback = RegionDetectionFallback::default();

    // Existing detection logic...
    if exceeds_caps {
        let mut result = DetectRegionsResult::default();
        if let Some(bounds) = occupancy.dense_bounds() {
            // Use fallback instead of build_fallback_region
            let simple = fallback.create_simple_region(
                metrics.row_count,
                metrics.column_count,
                occupancy.cells.len() as u32
            );
            result.regions.push(convert_to_detected_region(simple));
        }
        return result;
    }
    // ... rest of function
}
```

### 3. Batch Operations Enhancement

**Location**: `src/tools/fork.rs` in `edit_batch()`, `transform_batch()`, etc.

**Integration**:
```rust
use crate::recovery::{PartialSuccessHandler, BatchResult};

pub async fn edit_batch(
    state: Arc<AppState>,
    params: EditBatchParams,
) -> Result<EditBatchResponse> {
    let handler = PartialSuccessHandler::new()
        .max_errors(20);

    let result = handler.process_batch_async(params.edits, |idx, edit| {
        let work_path = fork_ctx.work_path.clone();
        async move {
            apply_single_edit(&work_path, &edit).await?;
            Ok(edit)
        }
    }).await;

    Ok(EditBatchResponse {
        fork_id: params.fork_id,
        edits_applied: result.summary.success_count,
        total_edits: result.total,
        warnings: result.summary.warnings,
        partial_success: result.is_partial_success(),
    })
}
```

### 4. Workbook Loading Recovery

**Location**: `src/workbook.rs` in `WorkbookContext::load()`

**Integration**:
```rust
use crate::recovery::{WorkbookRecoveryStrategy, RecoveryAction};

impl WorkbookContext {
    pub fn load(config: &Arc<ServerConfig>, path: &Path) -> Result<Self> {
        let recovery = WorkbookRecoveryStrategy::new(config.enable_backups);

        // Check for corruption before loading
        let action = recovery.determine_action(path)?;
        match action {
            RecoveryAction::None => {
                // Continue with normal load
            }
            RecoveryAction::RestoreFromBackup { backup_path } => {
                warn!("Restoring workbook from backup: {:?}", backup_path);
                recovery.execute_recovery(path, action)?;
            }
            RecoveryAction::MarkCorrupted => {
                bail!("Workbook is corrupted: {:?}", path);
            }
            _ => {
                // Handle other recovery actions
            }
        }

        // Existing load logic...
        let spreadsheet = xlsx::read(path)
            .with_context(|| format!("failed to parse workbook {:?}", path))?;
        // ...
    }
}
```

### 5. State Management Integration

**Location**: `src/state.rs`

**Integration**:
```rust
use crate::recovery::WorkbookRecoveryStrategy;

pub struct AppState {
    // Existing fields...
    #[cfg(feature = "recalc")]
    workbook_recovery: Option<Arc<WorkbookRecoveryStrategy>>,
}

impl AppState {
    pub fn new(config: Arc<ServerConfig>) -> Self {
        #[cfg(feature = "recalc")]
        let workbook_recovery = if config.recalc_enabled {
            Some(Arc::new(WorkbookRecoveryStrategy::new(true)))
        } else {
            None
        };

        Self {
            // ...existing fields
            #[cfg(feature = "recalc")]
            workbook_recovery,
        }
    }

    pub async fn open_workbook(&self, workbook_id: &WorkbookId) -> Result<Arc<WorkbookContext>> {
        let path = self.resolve_workbook_path(workbook_id)?;

        // Check for corruption if recovery is enabled
        #[cfg(feature = "recalc")]
        if let Some(recovery) = &self.workbook_recovery {
            let action = recovery.determine_action(&path)?;
            if action != RecoveryAction::None {
                recovery.execute_recovery(&path, action)?;
            }
        }

        // Existing load logic...
    }
}
```

## Configuration

### Environment Variables

Add these to your server configuration:

```bash
# Retry configuration
RECALC_MAX_RETRIES=5
RECALC_INITIAL_DELAY_MS=500
RECALC_MAX_DELAY_MS=30000

# Circuit breaker
CIRCUIT_BREAKER_FAILURE_THRESHOLD=3
CIRCUIT_BREAKER_SUCCESS_THRESHOLD=2
CIRCUIT_BREAKER_TIMEOUT_SECS=30

# Batch operations
BATCH_MAX_ERRORS=20
BATCH_FAIL_FAST=false

# Workbook recovery
ENABLE_WORKBOOK_BACKUPS=true
WORKBOOK_MIN_SIZE_BYTES=100
WORKBOOK_MAX_SIZE_BYTES=524288000  # 500MB
```

### ServerConfig Updates

Add recovery configuration fields:

```rust
pub struct ServerConfig {
    // Existing fields...

    #[cfg(feature = "recalc")]
    pub recovery_config: RecoveryConfig,
}

pub struct RecoveryConfig {
    pub retry: RetryConfig,
    pub circuit_breaker: CircuitBreakerConfig,
    pub enable_fallbacks: bool,
    pub enable_partial_success: bool,
    pub enable_workbook_recovery: bool,
}
```

## Testing

### Unit Tests

Each module includes comprehensive unit tests:

```bash
# Test all recovery modules
cargo test --lib recovery

# Test specific module
cargo test --lib recovery::retry
cargo test --lib recovery::circuit_breaker
cargo test --lib recovery::partial_success
```

### Integration Tests

Create integration tests for recovery scenarios:

```rust
#[tokio::test]
async fn test_recalc_with_retry_and_circuit_breaker() {
    // Test that retries work correctly
    // Test that circuit breaker opens after failures
    // Test that circuit breaker recovers
}

#[test]
fn test_region_detection_fallback() {
    // Test that fallback is used for large sheets
    // Test that fallback creates valid regions
}

#[tokio::test]
async fn test_batch_partial_success() {
    // Test that batch continues after failures
    // Test that batch respects max_errors
    // Test that batch statistics are correct
}
```

## Monitoring and Observability

### Metrics to Track

1. **Retry Metrics**:
   - Number of retry attempts per operation
   - Success rate after retries
   - Time spent in retries

2. **Circuit Breaker Metrics**:
   - State transitions (Closed → Open, etc.)
   - Time in each state
   - Number of requests rejected while open

3. **Fallback Metrics**:
   - Fallback usage frequency
   - Fallback success rate
   - Primary vs fallback performance

4. **Batch Metrics**:
   - Partial success rate
   - Average items failed per batch
   - Error distribution

5. **Recovery Metrics**:
   - Corruption detection frequency
   - Recovery success rate
   - Time to recover

### Logging

The recovery module uses `tracing` for structured logging:

```rust
tracing::warn!(
    operation = "recalculate",
    attempt = 3,
    max_attempts = 5,
    error = %err,
    "retrying operation after delay"
);
```

## Best Practices

1. **Always use retry for LibreOffice operations** - They're prone to transient failures
2. **Wrap recalc executor with circuit breaker** - Prevents cascading failures
3. **Use fallback for region detection** - Large sheets can timeout
4. **Enable partial success for all batch operations** - Better user experience
5. **Check workbook health before operations** - Prevents wasted work
6. **Create backups before destructive operations** - Enable recovery
7. **Monitor circuit breaker state** - Early warning of problems
8. **Log all recovery actions** - Essential for debugging

## Performance Impact

The recovery mechanisms are designed to have minimal overhead:

- **Retry**: Only adds delay on failures (exponential backoff)
- **Circuit Breaker**: ~1μs overhead per operation when closed
- **Fallback**: Only executed when primary fails
- **Partial Success**: Minimal overhead (tracking structures)
- **Workbook Recovery**: File check adds ~1-5ms per load

## Future Enhancements

1. **Adaptive retry delays** - Adjust based on failure patterns
2. **Distributed circuit breaker** - Share state across instances
3. **Recovery webhooks** - Notify external systems
4. **Health check endpoint** - Expose circuit breaker state
5. **Automatic backup rotation** - Limit backup storage
6. **Recovery metrics dashboard** - Visualize recovery health

## References

- [Retry Pattern](https://docs.microsoft.com/en-us/azure/architecture/patterns/retry)
- [Circuit Breaker Pattern](https://martinfowler.com/bliki/CircuitBreaker.html)
- [Fallback Pattern](https://docs.microsoft.com/en-us/azure/architecture/patterns/compensating-transaction)
