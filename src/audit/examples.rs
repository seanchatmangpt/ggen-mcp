//! Example integrations showing how to add audit logging to existing code
//!
//! This file demonstrates how to instrument fork operations, tool handlers,
//! and file operations with comprehensive audit trails.

#![allow(dead_code)]

use super::integration::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

// ============================================================================
// Example: Tool Handler with Audit Logging
// ============================================================================

#[derive(Debug, Deserialize, Serialize)]
struct ListWorkbooksParams {
    slug_prefix: Option<String>,
    folder: Option<String>,
}

#[derive(Debug, Serialize)]
struct WorkbookListResponse {
    workbooks: Vec<String>,
}

/// Example of instrumenting a tool handler
async fn list_workbooks_example(params: ListWorkbooksParams) -> Result<WorkbookListResponse> {
    // Create audit guard that logs on drop
    let _audit = audit_tool("list_workbooks", &params);

    // Perform the operation
    let workbooks = vec!["workbook1.xlsx".to_string(), "workbook2.xlsx".to_string()];

    Ok(WorkbookListResponse { workbooks })
    // Guard drops here, logging success with duration
}

/// Example of handling errors in a tool handler
async fn list_workbooks_with_error_example(
    params: ListWorkbooksParams,
) -> Result<WorkbookListResponse> {
    let audit = audit_tool("list_workbooks", &params);

    match perform_list_workbooks(&params).await {
        Ok(response) => {
            // Audit guard logs success on drop
            Ok(response)
        }
        Err(e) => {
            // Explicitly mark as failed before returning
            let _audit = audit.fail(e.to_string());
            Err(e)
        }
    }
}

async fn perform_list_workbooks(
    _params: &ListWorkbooksParams,
) -> Result<WorkbookListResponse> {
    // Implementation...
    Ok(WorkbookListResponse {
        workbooks: vec![],
    })
}

// ============================================================================
// Example: Fork Lifecycle Operations
// ============================================================================

/// Example: Create fork with audit logging
async fn create_fork_example(base_path: &Path) -> Result<String> {
    let fork_id = "fork-abc123";
    let _audit = audit_fork_create(fork_id, base_path);

    // Perform fork creation
    // ...

    Ok(fork_id.to_string())
    // Audit guard logs success on drop
}

/// Example: Edit fork with audit logging
async fn edit_fork_example(fork_id: &str, sheet: &str, edits: Vec<String>) -> Result<()> {
    let audit = audit_fork_edit(fork_id, sheet, edits.len());

    match perform_edits(fork_id, sheet, &edits).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let _audit = audit.fail(e.to_string());
            Err(e)
        }
    }
}

async fn perform_edits(_fork_id: &str, _sheet: &str, _edits: &[String]) -> Result<()> {
    // Implementation...
    Ok(())
}

/// Example: Recalculate fork with audit logging
async fn recalculate_fork_example(fork_id: &str) -> Result<()> {
    let audit = audit_fork_recalc(fork_id);

    match perform_recalculation(fork_id).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let _audit = audit.fail(e.to_string());
            Err(e)
        }
    }
}

async fn perform_recalculation(_fork_id: &str) -> Result<()> {
    // Implementation...
    Ok(())
}

/// Example: Save fork with audit logging
async fn save_fork_example(fork_id: &str, target_path: &Path, drop_fork: bool) -> Result<()> {
    let audit = audit_fork_save(fork_id, target_path, drop_fork);

    match perform_save(fork_id, target_path, drop_fork).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let _audit = audit.fail(e.to_string());
            Err(e)
        }
    }
}

async fn perform_save(_fork_id: &str, _target_path: &Path, _drop_fork: bool) -> Result<()> {
    // Implementation...
    Ok(())
}

/// Example: Discard fork with audit logging
async fn discard_fork_example(fork_id: &str) -> Result<()> {
    let _audit = audit_fork_discard(fork_id);

    // Perform discard
    // ...

    Ok(())
}

// ============================================================================
// Example: Checkpoint Operations
// ============================================================================

/// Example: Create checkpoint with audit logging
async fn create_checkpoint_example(fork_id: &str, label: Option<&str>) -> Result<String> {
    let checkpoint_id = "cp-xyz789";
    let audit = audit_checkpoint_create(fork_id, checkpoint_id, label);

    match perform_checkpoint_creation(fork_id, checkpoint_id, label).await {
        Ok(_) => Ok(checkpoint_id.to_string()),
        Err(e) => {
            let _audit = audit.fail(e.to_string());
            Err(e)
        }
    }
}

async fn perform_checkpoint_creation(
    _fork_id: &str,
    _checkpoint_id: &str,
    _label: Option<&str>,
) -> Result<()> {
    // Implementation...
    Ok(())
}

/// Example: Restore checkpoint with audit logging
async fn restore_checkpoint_example(fork_id: &str, checkpoint_id: &str) -> Result<()> {
    let audit = audit_checkpoint_restore(fork_id, checkpoint_id);

    match perform_checkpoint_restore(fork_id, checkpoint_id).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let _audit = audit.fail(e.to_string());
            Err(e)
        }
    }
}

async fn perform_checkpoint_restore(_fork_id: &str, _checkpoint_id: &str) -> Result<()> {
    // Implementation...
    Ok(())
}

/// Example: Delete checkpoint with audit logging
async fn delete_checkpoint_example(fork_id: &str, checkpoint_id: &str) -> Result<()> {
    let audit = audit_checkpoint_delete(fork_id, checkpoint_id);

    match perform_checkpoint_delete(fork_id, checkpoint_id).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let _audit = audit.fail(e.to_string());
            Err(e)
        }
    }
}

async fn perform_checkpoint_delete(_fork_id: &str, _checkpoint_id: &str) -> Result<()> {
    // Implementation...
    Ok(())
}

// ============================================================================
// Example: Staged Change Operations
// ============================================================================

/// Example: Create staged change with audit logging
async fn create_staged_change_example(
    fork_id: &str,
    change_id: &str,
    ops: Vec<String>,
) -> Result<()> {
    let audit = audit_staged_change_create(fork_id, change_id, ops.len());

    match perform_staged_change_creation(fork_id, change_id, &ops).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let _audit = audit.fail(e.to_string());
            Err(e)
        }
    }
}

async fn perform_staged_change_creation(
    _fork_id: &str,
    _change_id: &str,
    _ops: &[String],
) -> Result<()> {
    // Implementation...
    Ok(())
}

/// Example: Apply staged change with audit logging
async fn apply_staged_change_example(fork_id: &str, change_id: &str) -> Result<()> {
    let audit = audit_staged_change_apply(fork_id, change_id);

    match perform_staged_change_apply(fork_id, change_id).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let _audit = audit.fail(e.to_string());
            Err(e)
        }
    }
}

async fn perform_staged_change_apply(_fork_id: &str, _change_id: &str) -> Result<()> {
    // Implementation...
    Ok(())
}

/// Example: Discard staged change with audit logging
async fn discard_staged_change_example(fork_id: &str, change_id: &str) -> Result<()> {
    let audit = audit_staged_change_discard(fork_id, change_id);

    match perform_staged_change_discard(fork_id, change_id).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let _audit = audit.fail(e.to_string());
            Err(e)
        }
    }
}

async fn perform_staged_change_discard(_fork_id: &str, _change_id: &str) -> Result<()> {
    // Implementation...
    Ok(())
}

// ============================================================================
// Example: File Operations
// ============================================================================

/// Example: Instrumented file copy
fn copy_file_example(src: &Path, dst: &Path) -> Result<()> {
    audit_file_copy(src, dst);

    std::fs::copy(src, dst)?;

    Ok(())
}

/// Example: Instrumented file delete
fn delete_file_example(path: &Path) -> Result<()> {
    audit_file_delete(path);

    std::fs::remove_file(path)?;

    Ok(())
}

/// Example: Instrumented file write
fn write_file_example(path: &Path, contents: &[u8]) -> Result<()> {
    std::fs::write(path, contents)?;

    // Audit after successful write
    audit_file_write(path, Some(contents.len() as u64));

    Ok(())
}

/// Example: Instrumented directory creation
fn create_directory_example(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)?;

    audit_dir_create(path);

    Ok(())
}

// ============================================================================
// Example: Complex Operation with Multiple Audit Points
// ============================================================================

/// Example: Complex operation with multiple audit points
async fn complex_operation_example(fork_id: &str) -> Result<()> {
    // Tool-level audit
    let _tool_audit = audit_tool(
        "complex_operation",
        &serde_json::json!({ "fork_id": fork_id }),
    );

    // Step 1: Create checkpoint
    let checkpoint_id = "cp-before-complex";
    {
        let audit = audit_checkpoint_create(fork_id, checkpoint_id, Some("before complex op"));
        perform_checkpoint_creation(fork_id, checkpoint_id, Some("before complex op"))
            .await
            .map_err(|e| {
                let _audit = audit.fail(e.to_string());
                e
            })?;
    }

    // Step 2: Perform edits
    {
        let audit = audit_fork_edit(fork_id, "Sheet1", 10);
        perform_edits(fork_id, "Sheet1", &vec![])
            .await
            .map_err(|e| {
                let _audit = audit.fail(e.to_string());
                e
            })?;
    }

    // Step 3: Recalculate
    {
        let audit = audit_fork_recalc(fork_id);
        perform_recalculation(fork_id).await.map_err(|e| {
            let _audit = audit.fail(e.to_string());
            e
        })?;
    }

    Ok(())
}

// ============================================================================
// Example: Using Audit Logger Directly
// ============================================================================

use super::{AuditEvent, AuditEventType, get_audit_logger};

/// Example: Creating and logging custom audit events
fn custom_audit_event_example() {
    if let Some(logger) = get_audit_logger() {
        let event = AuditEvent::new(AuditEventType::ToolInvocation)
            .with_resource("custom_tool")
            .with_details(serde_json::json!({
                "custom_field": "custom_value",
                "count": 42,
            }))
            .with_duration_ms(150);

        logger.log(event);
    }
}

/// Example: Querying audit events
fn query_audit_events_example() {
    use super::{AuditFilter, AuditOutcome};

    if let Some(logger) = get_audit_logger() {
        // Get all fork creation events
        let filter = AuditFilter::new()
            .with_event_type(AuditEventType::ForkCreate)
            .with_limit(100);

        let events = logger.query_events(filter);
        println!("Found {} fork creation events", events.len());

        // Get all failed operations
        let filter = AuditFilter::new()
            .with_outcome(AuditOutcome::Failure)
            .with_limit(50);

        let failed_events = logger.query_events(filter);
        println!("Found {} failed operations", failed_events.len());

        // Get recent events
        let recent = logger.recent_events(20);
        println!("Recent {} events", recent.len());
    }
}
