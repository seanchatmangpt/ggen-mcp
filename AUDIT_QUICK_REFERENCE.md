# Audit Trail Quick Reference

## Import

```rust
use crate::audit::integration::*;
```

## Common Patterns

### Tool Handler

```rust
pub async fn my_tool(state: Arc<AppState>, params: Params) -> Result<Response> {
    let _audit = audit_tool("my_tool", &params);
    // ... implementation ...
    Ok(response)
}
```

### With Error Handling

```rust
pub async fn my_tool(state: Arc<AppState>, params: Params) -> Result<Response> {
    let audit = audit_tool("my_tool", &params);
    match perform_operation(&params) {
        Ok(response) => Ok(response),
        Err(e) => {
            let _audit = audit.fail(e.to_string());
            Err(e)
        }
    }
}
```

### Fork Operations

```rust
// Create
let _audit = audit_fork_create(&fork_id, base_path);

// Edit
let _audit = audit_fork_edit(&fork_id, sheet, edit_count);

// Recalc
let _audit = audit_fork_recalc(&fork_id);

// Save
let _audit = audit_fork_save(&fork_id, target_path, drop_fork);

// Discard
let _audit = audit_fork_discard(&fork_id);
```

### Checkpoint Operations

```rust
// Create
let _audit = audit_checkpoint_create(&fork_id, &cp_id, label);

// Restore
let _audit = audit_checkpoint_restore(&fork_id, &cp_id);

// Delete
let _audit = audit_checkpoint_delete(&fork_id, &cp_id);
```

### Staged Changes

```rust
// Create
let _audit = audit_staged_change_create(&fork_id, &change_id, op_count);

// Apply
let _audit = audit_staged_change_apply(&fork_id, &change_id);

// Discard
let _audit = audit_staged_change_discard(&fork_id, &change_id);
```

### File Operations

```rust
// Read
audit_file_read(path);

// Write
audit_file_write(path, Some(size_bytes));

// Copy
audit_file_copy(src, dst);

// Delete
audit_file_delete(path);

// Create directory
audit_dir_create(path);
```

### Workbook Operations

```rust
// Open
audit_workbook_open(&workbook_id, path);

// Close
audit_workbook_close(&workbook_id);
```

## Querying Events

```rust
use crate::audit::{get_audit_logger, AuditFilter, AuditEventType, AuditOutcome};

if let Some(logger) = get_audit_logger() {
    // Get fork creates
    let events = logger.query_events(
        AuditFilter::new()
            .with_event_type(AuditEventType::ForkCreate)
            .with_limit(100)
    );

    // Get failures
    let failures = logger.query_events(
        AuditFilter::new()
            .with_outcome(AuditOutcome::Failure)
            .with_limit(50)
    );

    // Get recent events
    let recent = logger.recent_events(20);

    // Get event count
    let count = logger.event_count();
}
```

## Configuration

```rust
use crate::audit::{AuditConfig, init_audit_logger};

let config = AuditConfig {
    log_dir: PathBuf::from("/var/log/mcp-audit"),
    memory_buffer_size: 10_000,
    max_log_file_size: 100 * 1024 * 1024,
    max_log_files: 10,
    max_log_age_days: 30,
    persistent_logging: true,
};

init_audit_logger(config)?;
```

## Event Types

- `ToolInvocation` - Tool calls
- `ForkCreate`, `ForkEdit`, `ForkRecalc`, `ForkSave`, `ForkDiscard` - Fork lifecycle
- `CheckpointCreate`, `CheckpointRestore`, `CheckpointDelete` - Checkpoints
- `StagedChangeCreate`, `StagedChangeApply`, `StagedChangeDiscard` - Staged changes
- `FileRead`, `FileWrite`, `FileCopy`, `FileDelete` - File operations
- `DirectoryCreate`, `DirectoryDelete` - Directory operations
- `WorkbookOpen`, `WorkbookClose`, `WorkbookList` - Workbook operations
- `Error` - Error events

## Outcomes

- `Success` - Operation completed successfully
- `Failure` - Operation failed
- `Partial` - Operation partially succeeded

## Best Practices

1. **Always use guards**: Let them log automatically on drop
2. **Name guards with `_`**: `let _audit = ...` (unless error handling needed)
3. **Mark failures**: `let _audit = audit.fail(error)`
4. **Keep guards alive**: Don't drop them before operation completes
5. **Sanitize sensitive data**: Don't log passwords, tokens, etc.
6. **Use consistent IDs**: fork_id, checkpoint_id, etc.

## Common Mistakes

### ❌ Guard dropped immediately
```rust
audit_tool("operation", &params);
perform_operation()?;
```

### ✅ Guard lives until function end
```rust
let _audit = audit_tool("operation", &params);
perform_operation()?;
```

### ❌ Error not marked as failure
```rust
let _audit = audit_tool("operation", &params);
operation()?; // Error doesn't mark audit as failed
```

### ✅ Explicitly mark errors
```rust
let audit = audit_tool("operation", &params);
match operation() {
    Ok(result) => Ok(result),
    Err(e) => {
        let _audit = audit.fail(e.to_string());
        Err(e)
    }
}
```

## Analyzing Logs

### Command Line

```bash
# Count events by type
jq -r '.event_type' audit-*.jsonl | sort | uniq -c

# Find failures
jq 'select(.outcome == "failure")' audit-*.jsonl

# Average duration
jq '.duration_ms' audit-*.jsonl | awk '{sum+=$1; n++} END {print sum/n}'

# Events for a fork
jq 'select(.resource | contains("fork-abc"))' audit-*.jsonl
```

### In Code

```rust
if let Some(logger) = get_audit_logger() {
    let filter = AuditFilter::new()
        .with_resource("fork-abc123")
        .with_limit(100);

    for event in logger.query_events(filter) {
        println!("{:?} at {}: {:?}",
            event.event_type, event.timestamp, event.outcome);
    }
}
```

## Tracing Integration

View structured logs:

```bash
RUST_LOG=info cargo run 2>&1 | grep "audit event"
```

## Performance

- Tool overhead: < 1ms per call
- Memory: ~1KB per event
- Disk I/O: Buffered, minimal impact
- CPU: < 1% for typical workloads
