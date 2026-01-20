# Audit Trail System Documentation

## Overview

The audit trail system provides comprehensive logging and tracking for all critical operations in the MCP server. It combines structured logging via the `tracing` crate with a persistent, queryable audit log.

## Architecture

### Components

1. **AuditLogger**: Core component that manages in-memory event buffer and persistent log files
2. **AuditEvent**: Structured event data with metadata (timestamp, type, outcome, duration, etc.)
3. **Integration Helpers**: Convenient functions for instrumenting code with audit logging
4. **Tracing Spans**: Hierarchical context for correlating related operations

### Event Types

The system tracks the following event types:

- **Tool Invocations**: All MCP tool calls with parameters
- **Fork Lifecycle**: create, edit, recalc, save, discard
- **Checkpoint Operations**: create, restore, delete
- **Staged Changes**: create, apply, discard
- **File Operations**: read, write, copy, delete, mkdir
- **Workbook Operations**: open, close, list
- **Errors**: All error events with context

### Storage

- **In-Memory Buffer**: Circular buffer of recent events (default: 10,000 events)
- **Persistent Logs**: JSON-Lines format files with automatic rotation
- **Log Rotation**: Based on file size (default: 100 MB) and retention policy
- **Retention**: Configurable age (default: 30 days) and count (default: 10 files)

## Configuration

### Basic Configuration

```rust
use crate::audit::{AuditConfig, init_audit_logger};

let config = AuditConfig {
    log_dir: PathBuf::from("/var/log/mcp-audit"),
    memory_buffer_size: 10_000,
    max_log_file_size: 100 * 1024 * 1024, // 100 MB
    max_log_files: 10,
    max_log_age_days: 30,
    persistent_logging: true,
};

init_audit_logger(config)?;
```

### Environment-Based Configuration

The audit system can be configured via environment variables or configuration files to support different deployment scenarios.

## Usage Patterns

### 1. Tool Handler Instrumentation

The simplest and most common pattern for instrumenting tool handlers:

```rust
use crate::audit::integration::audit_tool;

pub async fn list_workbooks(
    state: Arc<AppState>,
    params: ListWorkbooksParams,
) -> Result<WorkbookListResponse> {
    // Create audit guard - logs on drop with duration
    let _audit = audit_tool("list_workbooks", &params);

    // Normal implementation
    let filter = params.into_filter()?;
    state.list_workbooks(filter)

    // Guard automatically logs success with duration when dropped
}
```

### 2. Error Handling with Audit

When operations can fail, explicitly mark the audit as failed:

```rust
use crate::audit::integration::audit_tool;

pub async fn create_fork(
    state: Arc<AppState>,
    params: CreateForkParams,
) -> Result<CreateForkResponse> {
    let audit = audit_tool("create_fork", &params);

    match perform_fork_creation(&state, &params).await {
        Ok(response) => {
            // Audit guard logs success on drop
            Ok(response)
        }
        Err(e) => {
            // Explicitly mark as failed with error message
            let _audit = audit.fail(e.to_string());
            Err(e)
        }
    }
}
```

### 3. Fork Lifecycle Operations

Track fork operations throughout their lifecycle:

```rust
use crate::audit::integration::{
    audit_fork_create, audit_fork_edit, audit_fork_recalc,
    audit_fork_save, audit_fork_discard
};

// Fork creation
pub async fn create_fork(base_path: &Path) -> Result<String> {
    let fork_id = generate_fork_id();
    let _audit = audit_fork_create(&fork_id, base_path);

    // ... create fork ...

    Ok(fork_id)
}

// Fork editing
pub async fn edit_batch(fork_id: &str, sheet: &str, edits: Vec<Edit>) -> Result<()> {
    let audit = audit_fork_edit(fork_id, sheet, edits.len());

    match apply_edits(fork_id, sheet, &edits).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let _audit = audit.fail(e.to_string());
            Err(e)
        }
    }
}

// Fork recalculation
pub async fn recalculate(fork_id: &str) -> Result<RecalcResult> {
    let audit = audit_fork_recalc(fork_id);

    match perform_recalc(fork_id).await {
        Ok(result) => Ok(result),
        Err(e) => {
            let _audit = audit.fail(e.to_string());
            Err(e)
        }
    }
}

// Fork save
pub async fn save_fork(fork_id: &str, target: &Path, drop: bool) -> Result<()> {
    let audit = audit_fork_save(fork_id, target, drop);

    match perform_save(fork_id, target, drop).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let _audit = audit.fail(e.to_string());
            Err(e)
        }
    }
}

// Fork discard
pub async fn discard_fork(fork_id: &str) -> Result<()> {
    let _audit = audit_fork_discard(fork_id);

    // ... discard fork ...

    Ok(())
}
```

### 4. Checkpoint Operations

Track checkpoint lifecycle:

```rust
use crate::audit::integration::{
    audit_checkpoint_create, audit_checkpoint_restore, audit_checkpoint_delete
};

pub async fn create_checkpoint(fork_id: &str, label: Option<&str>) -> Result<String> {
    let checkpoint_id = generate_checkpoint_id();
    let audit = audit_checkpoint_create(fork_id, &checkpoint_id, label);

    match perform_checkpoint_creation(fork_id, &checkpoint_id, label).await {
        Ok(_) => Ok(checkpoint_id),
        Err(e) => {
            let _audit = audit.fail(e.to_string());
            Err(e)
        }
    }
}

pub async fn restore_checkpoint(fork_id: &str, checkpoint_id: &str) -> Result<()> {
    let audit = audit_checkpoint_restore(fork_id, checkpoint_id);

    match perform_restore(fork_id, checkpoint_id).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let _audit = audit.fail(e.to_string());
            Err(e)
        }
    }
}
```

### 5. File Operations

Track all file system operations:

```rust
use crate::audit::integration::{
    audit_file_read, audit_file_write, audit_file_copy,
    audit_file_delete, audit_dir_create
};

// File read
fn load_workbook(path: &Path) -> Result<Workbook> {
    audit_file_read(path);
    // ... read file ...
}

// File write
fn save_workbook(path: &Path, data: &[u8]) -> Result<()> {
    fs::write(path, data)?;
    audit_file_write(path, Some(data.len() as u64));
    Ok(())
}

// File copy
fn copy_fork(src: &Path, dst: &Path) -> Result<()> {
    audit_file_copy(src, dst);
    fs::copy(src, dst)?;
    Ok(())
}

// File delete
fn cleanup_fork(path: &Path) -> Result<()> {
    audit_file_delete(path);
    fs::remove_file(path)?;
    Ok(())
}

// Directory creation
fn ensure_fork_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    audit_dir_create(path);
    Ok(())
}
```

### 6. Staged Change Operations

Track staged changes:

```rust
use crate::audit::integration::{
    audit_staged_change_create, audit_staged_change_apply,
    audit_staged_change_discard
};

pub async fn create_staged_change(fork_id: &str, ops: Vec<Op>) -> Result<String> {
    let change_id = generate_change_id();
    let audit = audit_staged_change_create(fork_id, &change_id, ops.len());

    match perform_stage(fork_id, &change_id, &ops).await {
        Ok(_) => Ok(change_id),
        Err(e) => {
            let _audit = audit.fail(e.to_string());
            Err(e)
        }
    }
}

pub async fn apply_staged_change(fork_id: &str, change_id: &str) -> Result<()> {
    let audit = audit_staged_change_apply(fork_id, change_id);

    match perform_apply(fork_id, change_id).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let _audit = audit.fail(e.to_string());
            Err(e)
        }
    }
}
```

### 7. Complex Operations with Multiple Audit Points

For complex operations involving multiple steps:

```rust
pub async fn complex_workflow(fork_id: &str) -> Result<()> {
    // Overall operation audit
    let _tool_audit = audit_tool("complex_workflow",
        &serde_json::json!({ "fork_id": fork_id }));

    // Step 1: Create checkpoint
    {
        let audit = audit_checkpoint_create(fork_id, "cp-1", Some("pre-workflow"));
        create_checkpoint_internal(fork_id, "cp-1").await
            .map_err(|e| {
                let _audit = audit.fail(e.to_string());
                e
            })?;
    }

    // Step 2: Apply edits
    {
        let audit = audit_fork_edit(fork_id, "Sheet1", 10);
        apply_edits_internal(fork_id).await
            .map_err(|e| {
                let _audit = audit.fail(e.to_string());
                e
            })?;
    }

    // Step 3: Recalculate
    {
        let audit = audit_fork_recalc(fork_id);
        recalc_internal(fork_id).await
            .map_err(|e| {
                let _audit = audit.fail(e.to_string());
                e
            })?;
    }

    Ok(())
}
```

## Querying Audit Logs

### In-Memory Queries

```rust
use crate::audit::{get_audit_logger, AuditFilter, AuditEventType, AuditOutcome};

// Get all fork creation events
if let Some(logger) = get_audit_logger() {
    let filter = AuditFilter::new()
        .with_event_type(AuditEventType::ForkCreate)
        .with_limit(100);

    let events = logger.query_events(filter);
    for event in events {
        println!("Fork created: {:?}", event.resource);
    }
}

// Get all failed operations
if let Some(logger) = get_audit_logger() {
    let filter = AuditFilter::new()
        .with_outcome(AuditOutcome::Failure)
        .with_limit(50);

    let failed = logger.query_events(filter);
    for event in failed {
        println!("Failed: {} - {}",
            event.resource.unwrap_or_default(),
            event.error.unwrap_or_default());
    }
}

// Get recent events
if let Some(logger) = get_audit_logger() {
    let recent = logger.recent_events(20);
    for event in recent {
        println!("{:?} at {}: {:?}",
            event.event_type, event.timestamp, event.outcome);
    }
}

// Get event count
if let Some(logger) = get_audit_logger() {
    println!("Total events in buffer: {}", logger.event_count());
}
```

### Persistent Log Analysis

The persistent logs are stored in JSON-Lines format, making them easy to analyze with standard tools:

```bash
# Count events by type
cat /tmp/mcp-audit-logs/audit-*.jsonl | \
  jq -r '.event_type' | sort | uniq -c

# Find all failed operations
cat /tmp/mcp-audit-logs/audit-*.jsonl | \
  jq 'select(.outcome == "failure")'

# Get statistics on operation durations
cat /tmp/mcp-audit-logs/audit-*.jsonl | \
  jq '.duration_ms' | \
  awk '{sum+=$1; count++} END {print "Average:", sum/count, "ms"}'

# Find all operations on a specific fork
cat /tmp/mcp-audit-logs/audit-*.jsonl | \
  jq 'select(.resource | contains("fork-abc123"))'

# Extract events within a time range
cat /tmp/mcp-audit-logs/audit-*.jsonl | \
  jq 'select(.timestamp >= "2024-01-20T10:00:00Z" and
             .timestamp <= "2024-01-20T11:00:00Z")'
```

## Log Rotation and Retention

### Automatic Rotation

Log files are automatically rotated when they reach the configured size limit (default: 100 MB). The rotation process:

1. Closes the current log file
2. Creates a new log file with timestamp
3. Continues logging to the new file
4. Cleans up old files based on retention policy

### Retention Policy

Old log files are automatically cleaned up based on:

- **Age**: Files older than `max_log_age_days` (default: 30 days)
- **Count**: Only `max_log_files` (default: 10) most recent files are kept

### Manual Cleanup

To manually manage log files:

```rust
use crate::audit::get_audit_logger;

if let Some(logger) = get_audit_logger() {
    // Export all events before cleanup
    let events = logger.export_events();

    // Archive or process events as needed
    archive_events(&events)?;
}
```

## Tracing Integration

The audit system integrates with the `tracing` crate for hierarchical logging:

### Viewing Structured Logs

```bash
# Run with tracing enabled
RUST_LOG=info cargo run

# View audit events in logs
RUST_LOG=info cargo run 2>&1 | grep "audit event"

# Filter by event type
RUST_LOG=info cargo run 2>&1 | grep "fork_operation"
```

### Span Context

All audit operations create tracing spans with structured fields:

```rust
// Tool invocation span
info_span!(
    "tool_invocation",
    tool = "list_workbooks",
    params = ?params_json,
    outcome = tracing::field::Empty,
    duration_ms = tracing::field::Empty,
)

// Fork operation span
info_span!(
    "fork_operation",
    operation = "create",
    fork_id = "fork-abc123",
    outcome = tracing::field::Empty,
    duration_ms = tracing::field::Empty,
)
```

## Performance Considerations

### Overhead

The audit system is designed for minimal performance impact:

- **In-Memory Buffer**: O(1) append to circular buffer
- **Async Logging**: File I/O is buffered and flushed periodically
- **Lazy Serialization**: Events are only serialized when persisted
- **Conditional Logging**: Guards check if logger is initialized before creating events

### Best Practices

1. **Use Guards**: Prefer audit guards that log on drop over manual logging
2. **Avoid Excessive Detail**: Keep event details concise; use external references
3. **Batch Operations**: For bulk operations, log summary rather than individual items
4. **Monitor Buffer Size**: Adjust `memory_buffer_size` based on event volume
5. **Monitor Disk Usage**: Configure retention policy based on available disk space

## Security and Compliance

### Sensitive Data

**Important**: The audit system logs operation parameters and details. Ensure sensitive data is not logged:

```rust
// GOOD: Sanitize parameters
let sanitized_params = params.sanitize();
let _audit = audit_tool("update_credentials", &sanitized_params);

// BAD: Logging sensitive data
let _audit = audit_tool("update_credentials", &params); // May include passwords!
```

### Access Control

Audit logs should be:

- **Readable**: Only by administrators and authorized security personnel
- **Writable**: Only by the MCP server process
- **Immutable**: Protected from modification or deletion (use filesystem permissions)

### Compliance

The audit trail system supports compliance requirements by:

- **Comprehensive Logging**: All operations tracked with timestamp and outcome
- **Tamper Evidence**: JSON-Lines format makes modifications detectable
- **Retention Policy**: Configurable retention for regulatory requirements
- **Queryability**: Easy to extract and analyze for audit reports

## Troubleshooting

### Audit Logger Not Initialized

If audit logs are not being written:

```rust
use crate::audit::get_audit_logger;

if get_audit_logger().is_none() {
    eprintln!("Audit logger not initialized!");
}
```

### Log Files Not Rotating

Check configuration:

```rust
use crate::audit::get_audit_logger;

if let Some(logger) = get_audit_logger() {
    // Check current log size vs. max
    // (internal API - see implementation)
}
```

### High Memory Usage

If memory usage is high due to audit buffer:

```rust
let config = AuditConfig {
    memory_buffer_size: 1_000, // Reduce buffer size
    ..Default::default()
};
```

### Missing Events

Ensure guards are not dropped prematurely:

```rust
// GOOD: Guard lives until end of function
let _audit = audit_tool("operation", &params);
perform_operation()?;
// Guard drops here, logging event

// BAD: Guard dropped immediately
audit_tool("operation", &params);
perform_operation()?;
// Event logged before operation completes!
```

## Example Integration Checklist

When adding audit logging to new code:

- [ ] Import appropriate audit helpers
- [ ] Create audit guard at operation start
- [ ] Handle errors by marking audit as failed
- [ ] Ensure guard lives until operation completes
- [ ] Add tracing spans for hierarchical context
- [ ] Sanitize sensitive data in parameters
- [ ] Document audit behavior in function documentation
- [ ] Test audit logging in integration tests

## Future Enhancements

Potential improvements to the audit system:

1. **Remote Logging**: Send audit events to external SIEM systems
2. **Real-time Alerts**: Trigger alerts on specific event patterns
3. **Metrics Integration**: Export audit metrics to Prometheus/Grafana
4. **Compression**: Compress rotated log files
5. **Encryption**: Encrypt audit logs at rest
6. **Search API**: Build index for fast audit log queries
7. **Web UI**: Dashboard for viewing and analyzing audit events

## Resources

- Source: `src/audit/mod.rs` - Core audit system
- Integration: `src/audit/integration.rs` - Helper functions
- Examples: `src/audit/examples.rs` - Usage examples
- Tests: `src/audit/mod.rs` - Unit tests
