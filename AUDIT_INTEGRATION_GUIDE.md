# Audit Trail Integration Guide

## Quick Start: Adding Audit Logging to Existing Code

This guide shows how to add audit logging to the existing fork tools in `src/tools/fork.rs`.

### Step 1: Add Import Statements

At the top of `src/tools/fork.rs`, add:

```rust
use crate::audit::integration::{
    audit_tool, audit_fork_create, audit_fork_edit, audit_fork_recalc,
    audit_fork_save, audit_fork_discard, audit_checkpoint_create,
    audit_checkpoint_restore, audit_checkpoint_delete,
    audit_staged_change_create, audit_staged_change_apply,
    audit_staged_change_discard,
};
```

### Step 2: Instrument Tool Handlers

#### Example 1: create_fork

**Before:**
```rust
pub async fn create_fork(
    state: Arc<AppState>,
    params: CreateForkParams,
) -> Result<CreateForkResponse> {
    let registry = state
        .fork_registry()
        .ok_or_else(|| anyhow!("fork registry not available"))?;

    let fork_id = registry.create_fork(base_path, workspace_root)?;

    Ok(CreateForkResponse {
        fork_id,
        base_path: base_path_str,
        workbook_id: workbook_id.clone(),
    })
}
```

**After:**
```rust
pub async fn create_fork(
    state: Arc<AppState>,
    params: CreateForkParams,
) -> Result<CreateForkResponse> {
    // Add tool-level audit
    let tool_audit = audit_tool("create_fork", &params);

    let registry = state
        .fork_registry()
        .ok_or_else(|| anyhow!("fork registry not available"))?;

    // Add fork-specific audit
    let fork_audit = {
        let fork_id = registry.create_fork(base_path, workspace_root)?;

        // Create fork audit after we have the fork_id
        let audit = audit_fork_create(&fork_id, base_path);

        Ok::<_, anyhow::Error>((fork_id, audit))
    }.map_err(|e| {
        let _tool_audit = tool_audit.fail(e.to_string());
        e
    })?;

    let (fork_id, _fork_audit) = fork_audit;

    Ok(CreateForkResponse {
        fork_id,
        base_path: base_path_str,
        workbook_id: workbook_id.clone(),
    })
}
```

#### Example 2: edit_batch

**Before:**
```rust
pub async fn edit_batch(
    state: Arc<AppState>,
    params: EditBatchParams,
) -> Result<EditBatchResponse> {
    let fork_id = &params.fork_id;
    let sheet_name = &params.sheet_name;
    let edits = &params.edits;

    // Apply edits...

    Ok(EditBatchResponse {
        fork_id: fork_id.clone(),
        applied: edits.len(),
    })
}
```

**After:**
```rust
pub async fn edit_batch(
    state: Arc<AppState>,
    params: EditBatchParams,
) -> Result<EditBatchResponse> {
    let tool_audit = audit_tool("edit_batch", &params);

    let fork_id = &params.fork_id;
    let sheet_name = &params.sheet_name;
    let edits = &params.edits;

    let fork_audit = audit_fork_edit(fork_id, sheet_name, edits.len());

    // Apply edits...
    let result = apply_edits_internal(state, fork_id, sheet_name, edits).await;

    match result {
        Ok(count) => Ok(EditBatchResponse {
            fork_id: fork_id.clone(),
            applied: count,
        }),
        Err(e) => {
            let _tool_audit = tool_audit.fail(e.to_string());
            let _fork_audit = fork_audit.fail(e.to_string());
            Err(e)
        }
    }
}
```

#### Example 3: recalculate

**Before:**
```rust
pub async fn recalculate(
    state: Arc<AppState>,
    params: RecalculateParams,
) -> Result<RecalculateResponse> {
    let fork_id = &params.fork_id;

    // Perform recalculation...

    Ok(RecalculateResponse {
        fork_id: fork_id.clone(),
        status: "success".to_string(),
    })
}
```

**After:**
```rust
pub async fn recalculate(
    state: Arc<AppState>,
    params: RecalculateParams,
) -> Result<RecalculateResponse> {
    let tool_audit = audit_tool("recalculate", &params);
    let fork_id = &params.fork_id;
    let fork_audit = audit_fork_recalc(fork_id);

    match perform_recalculation(state, fork_id).await {
        Ok(status) => Ok(RecalculateResponse {
            fork_id: fork_id.clone(),
            status,
        }),
        Err(e) => {
            let _tool_audit = tool_audit.fail(e.to_string());
            let _fork_audit = fork_audit.fail(e.to_string());
            Err(e)
        }
    }
}
```

#### Example 4: save_fork

**Before:**
```rust
pub async fn save_fork(
    state: Arc<AppState>,
    params: SaveForkParams,
) -> Result<SaveForkResponse> {
    let fork_id = &params.fork_id;
    let target_path = resolve_path(&params.target_path, workspace_root)?;

    registry.save_fork(fork_id, &target_path, workspace_root, params.drop_fork)?;

    Ok(SaveForkResponse {
        fork_id: fork_id.clone(),
        saved_to: target_path.display().to_string(),
    })
}
```

**After:**
```rust
pub async fn save_fork(
    state: Arc<AppState>,
    params: SaveForkParams,
) -> Result<SaveForkResponse> {
    let tool_audit = audit_tool("save_fork", &params);
    let fork_id = &params.fork_id;
    let target_path = resolve_path(&params.target_path, workspace_root)?;

    let fork_audit = audit_fork_save(fork_id, &target_path, params.drop_fork);

    match registry.save_fork(fork_id, &target_path, workspace_root, params.drop_fork) {
        Ok(_) => Ok(SaveForkResponse {
            fork_id: fork_id.clone(),
            saved_to: target_path.display().to_string(),
        }),
        Err(e) => {
            let _tool_audit = tool_audit.fail(e.to_string());
            let _fork_audit = fork_audit.fail(e.to_string());
            Err(e)
        }
    }
}
```

#### Example 5: discard_fork

**Before:**
```rust
pub async fn discard_fork(
    state: Arc<AppState>,
    params: DiscardForkParams,
) -> Result<DiscardForkResponse> {
    let fork_id = &params.fork_id;

    registry.discard_fork(fork_id)?;

    Ok(DiscardForkResponse {
        fork_id: fork_id.clone(),
        discarded: true,
    })
}
```

**After:**
```rust
pub async fn discard_fork(
    state: Arc<AppState>,
    params: DiscardForkParams,
) -> Result<DiscardForkResponse> {
    let tool_audit = audit_tool("discard_fork", &params);
    let fork_id = &params.fork_id;
    let fork_audit = audit_fork_discard(fork_id);

    match registry.discard_fork(fork_id) {
        Ok(_) => Ok(DiscardForkResponse {
            fork_id: fork_id.clone(),
            discarded: true,
        }),
        Err(e) => {
            let _tool_audit = tool_audit.fail(e.to_string());
            let _fork_audit = fork_audit.fail(e.to_string());
            Err(e)
        }
    }
}
```

### Step 3: Instrument Checkpoint Operations

#### checkpoint_fork

**Before:**
```rust
pub async fn checkpoint_fork(
    state: Arc<AppState>,
    params: CheckpointForkParams,
) -> Result<CheckpointForkResponse> {
    let checkpoint = registry.create_checkpoint(fork_id, params.label.clone())?;

    Ok(CheckpointForkResponse {
        fork_id: fork_id.clone(),
        checkpoint_id: checkpoint.checkpoint_id,
    })
}
```

**After:**
```rust
pub async fn checkpoint_fork(
    state: Arc<AppState>,
    params: CheckpointForkParams,
) -> Result<CheckpointForkResponse> {
    let tool_audit = audit_tool("checkpoint_fork", &params);
    let fork_id = &params.fork_id;

    let checkpoint = registry.create_checkpoint(fork_id, params.label.clone())?;

    let _checkpoint_audit = audit_checkpoint_create(
        fork_id,
        &checkpoint.checkpoint_id,
        params.label.as_deref(),
    );

    match checkpoint {
        Ok(cp) => Ok(CheckpointForkResponse {
            fork_id: fork_id.clone(),
            checkpoint_id: cp.checkpoint_id,
        }),
        Err(e) => {
            let _tool_audit = tool_audit.fail(e.to_string());
            Err(e)
        }
    }
}
```

#### restore_checkpoint

**Before:**
```rust
pub async fn restore_checkpoint(
    state: Arc<AppState>,
    params: RestoreCheckpointParams,
) -> Result<RestoreCheckpointResponse> {
    registry.restore_checkpoint(fork_id, checkpoint_id)?;

    Ok(RestoreCheckpointResponse {
        fork_id: fork_id.clone(),
        checkpoint_id: checkpoint_id.clone(),
    })
}
```

**After:**
```rust
pub async fn restore_checkpoint(
    state: Arc<AppState>,
    params: RestoreCheckpointParams,
) -> Result<RestoreCheckpointResponse> {
    let tool_audit = audit_tool("restore_checkpoint", &params);
    let fork_id = &params.fork_id;
    let checkpoint_id = &params.checkpoint_id;

    let checkpoint_audit = audit_checkpoint_restore(fork_id, checkpoint_id);

    match registry.restore_checkpoint(fork_id, checkpoint_id) {
        Ok(_) => Ok(RestoreCheckpointResponse {
            fork_id: fork_id.clone(),
            checkpoint_id: checkpoint_id.clone(),
        }),
        Err(e) => {
            let _tool_audit = tool_audit.fail(e.to_string());
            let _checkpoint_audit = checkpoint_audit.fail(e.to_string());
            Err(e)
        }
    }
}
```

### Step 4: Instrument File Operations in ForkRegistry

In `src/fork.rs`, add file operation auditing:

```rust
use crate::audit::integration::{audit_file_copy, audit_file_delete, audit_dir_create};

impl ForkRegistry {
    pub fn create_fork(&self, base_path: &Path, workspace_root: &Path) -> Result<String> {
        // ... existing validation code ...

        let work_path = self.config.fork_dir.join(format!("{}.xlsx", fork_id));

        // Audit the file copy
        audit_file_copy(base_path, &work_path);
        fs::copy(base_path, &work_path)?;

        // ... rest of implementation ...
    }

    pub fn discard_fork(&self, fork_id: &str) -> Result<()> {
        let mut forks = self.forks.lock();
        if let Some(ctx) = forks.remove(fork_id) {
            // Audit file deletion
            audit_file_delete(&ctx.work_path);
            ctx.cleanup_files();
        }
        Ok(())
    }

    fn checkpoint_dir(&self) -> PathBuf {
        let dir = PathBuf::from(CHECKPOINT_DIR).join(fork_id);

        // Audit directory creation
        if !dir.exists() {
            audit_dir_create(&dir);
            fs::create_dir_all(&dir)?;
        }

        Ok(dir)
    }
}
```

### Step 5: Add Audit Querying Tool (Optional)

Create a new tool for querying audit logs:

```rust
use crate::audit::{get_audit_logger, AuditFilter, AuditEventType, AuditOutcome};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueryAuditParams {
    pub event_type: Option<String>,
    pub outcome: Option<String>,
    pub resource: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct QueryAuditResponse {
    pub events: Vec<AuditEventSummary>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct AuditEventSummary {
    pub event_id: String,
    pub timestamp: String,
    pub event_type: String,
    pub outcome: String,
    pub resource: Option<String>,
    pub duration_ms: Option<u64>,
}

pub async fn query_audit(
    _state: Arc<AppState>,
    params: QueryAuditParams,
) -> Result<QueryAuditResponse> {
    let logger = get_audit_logger()
        .ok_or_else(|| anyhow!("audit logger not available"))?;

    let mut filter = AuditFilter::new();

    if let Some(event_type_str) = &params.event_type {
        // Parse event type string
        let event_type = parse_event_type(event_type_str)?;
        filter = filter.with_event_type(event_type);
    }

    if let Some(outcome_str) = &params.outcome {
        let outcome = parse_outcome(outcome_str)?;
        filter = filter.with_outcome(outcome);
    }

    if let Some(resource) = &params.resource {
        filter = filter.with_resource(resource.clone());
    }

    if let Some(limit) = params.limit {
        filter = filter.with_limit(limit);
    }

    let events = logger.query_events(filter);

    let summaries: Vec<AuditEventSummary> = events
        .iter()
        .map(|e| AuditEventSummary {
            event_id: e.event_id.clone(),
            timestamp: e.timestamp.to_rfc3339(),
            event_type: format!("{:?}", e.event_type),
            outcome: format!("{:?}", e.outcome),
            resource: e.resource.clone(),
            duration_ms: e.duration_ms,
        })
        .collect();

    Ok(QueryAuditResponse {
        total: summaries.len(),
        events: summaries,
    })
}
```

### Step 6: Test the Integration

Create integration tests to verify audit logging:

```rust
#[cfg(test)]
mod audit_integration_tests {
    use super::*;
    use crate::audit::{get_audit_logger, AuditFilter, AuditEventType};

    #[tokio::test]
    async fn test_fork_creation_audit() {
        let state = setup_test_state();
        let params = CreateForkParams {
            workbook_or_fork_id: "test.xlsx".into(),
        };

        let result = create_fork(state, params).await;
        assert!(result.is_ok());

        // Verify audit event was logged
        if let Some(logger) = get_audit_logger() {
            let filter = AuditFilter::new()
                .with_event_type(AuditEventType::ForkCreate)
                .with_limit(1);

            let events = logger.query_events(filter);
            assert_eq!(events.len(), 1);
            assert!(events[0].resource.is_some());
        }
    }

    #[tokio::test]
    async fn test_fork_edit_audit() {
        let state = setup_test_state();
        let fork_id = create_test_fork(&state).await;

        let params = EditBatchParams {
            fork_id,
            sheet_name: "Sheet1".to_string(),
            edits: vec![/* ... */],
        };

        let result = edit_batch(state, params).await;
        assert!(result.is_ok());

        // Verify audit event
        if let Some(logger) = get_audit_logger() {
            let filter = AuditFilter::new()
                .with_event_type(AuditEventType::ForkEdit)
                .with_limit(1);

            let events = logger.query_events(filter);
            assert_eq!(events.len(), 1);
        }
    }

    #[tokio::test]
    async fn test_failed_operation_audit() {
        let state = setup_test_state();
        let params = CreateForkParams {
            workbook_or_fork_id: "nonexistent.xlsx".into(),
        };

        let result = create_fork(state, params).await;
        assert!(result.is_err());

        // Verify failure was audited
        if let Some(logger) = get_audit_logger() {
            let filter = AuditFilter::new()
                .with_outcome(AuditOutcome::Failure)
                .with_limit(1);

            let events = logger.query_events(filter);
            assert_eq!(events.len(), 1);
            assert!(events[0].error.is_some());
        }
    }
}
```

## Migration Checklist

- [ ] Add audit integration imports to affected modules
- [ ] Instrument all tool handlers with `audit_tool`
- [ ] Add fork-specific auditing to fork operations
- [ ] Add checkpoint auditing to checkpoint operations
- [ ] Add staged change auditing to staged change operations
- [ ] Audit file operations in `ForkRegistry`
- [ ] Create integration tests for audit logging
- [ ] Update documentation with audit behavior
- [ ] Configure audit retention policy for production
- [ ] Set up log monitoring and alerting

## Performance Impact

Expected performance impact:

- **Tool Handlers**: < 1ms overhead per invocation
- **Fork Operations**: < 2ms overhead (includes file operation auditing)
- **Memory**: ~1KB per event in buffer (10MB for 10,000 events)
- **Disk I/O**: Buffered writes, minimal impact
- **CPU**: Negligible (< 1% for typical workloads)

## Monitoring

To monitor audit system health:

```rust
use crate::audit::get_audit_logger;

// Add health check endpoint
pub async fn audit_health() -> Result<HealthResponse> {
    let logger = get_audit_logger()
        .ok_or_else(|| anyhow!("audit logger not initialized"))?;

    Ok(HealthResponse {
        initialized: true,
        event_count: logger.event_count(),
        buffer_size: logger.config().memory_buffer_size,
    })
}
```

## Troubleshooting Common Issues

### Issue: Guards dropped too early

**Symptom**: Events logged before operation completes

**Solution**: Ensure guards live until end of function:

```rust
// WRONG
audit_tool("operation", &params);
perform_operation()?;

// RIGHT
let _audit = audit_tool("operation", &params);
perform_operation()?;
```

### Issue: Errors not marked as failed

**Symptom**: Failed operations logged as success

**Solution**: Explicitly mark audit as failed:

```rust
match operation() {
    Ok(result) => Ok(result),
    Err(e) => {
        let _audit = audit.fail(e.to_string());
        Err(e)
    }
}
```

### Issue: Missing context in audit events

**Symptom**: Hard to correlate related events

**Solution**: Use consistent resource naming:

```rust
// Use fork_id consistently
audit_fork_edit(&fork_id, sheet, count);
audit_fork_recalc(&fork_id);
audit_fork_save(&fork_id, path, drop);
```

## Next Steps

1. Review the examples in `src/audit/examples.rs`
2. Start with instrumenting one tool handler
3. Verify audit events in logs and persistent storage
4. Gradually instrument remaining operations
5. Set up monitoring and alerting
6. Document audit behavior for users
