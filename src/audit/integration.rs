//! Integration helpers for audit logging throughout the codebase
//!
//! This module provides convenient functions and macros to integrate audit logging
//! into existing tool handlers, fork operations, and file operations.

use super::{
    AuditEvent, AuditEventType, AuditScope, audit_checkpoint_span, audit_event,
    audit_file_span, audit_fork_span, audit_scope, audit_staged_change_span,
    audit_tool_span, get_audit_logger,
};
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::path::Path;
use tracing::Span;

/// Audit a tool invocation
///
/// # Example
///
/// ```rust
/// use crate::audit::integration::audit_tool;
///
/// async fn list_workbooks(params: ListWorkbooksParams) -> Result<WorkbookListResponse> {
///     let _audit = audit_tool("list_workbooks", &params);
///     // ... implementation ...
/// }
/// ```
pub fn audit_tool<P: Serialize>(tool_name: &str, params: &P) -> ToolAuditGuard {
    let span = audit_tool_span(tool_name, params);

    let scope = if let Some(logger) = get_audit_logger() {
        let params_json = serde_json::to_value(params).unwrap_or(JsonValue::Null);
        Some(
            AuditScope::new(logger, AuditEventType::ToolInvocation)
                .resource(tool_name.to_string())
                .details(params_json),
        )
    } else {
        None
    };

    ToolAuditGuard {
        scope,
        _span: span,
        tool_name: tool_name.to_string(),
    }
}

/// Guard that logs tool invocation on drop
pub struct ToolAuditGuard {
    scope: Option<AuditScope>,
    _span: Span,
    tool_name: String,
}

impl ToolAuditGuard {
    /// Mark the tool invocation as failed
    pub fn fail(mut self, error: impl Into<String>) -> Self {
        if let Some(scope) = self.scope.take() {
            self.scope = Some(scope.fail(error));
        }
        self
    }

    /// Mark the tool invocation as partial success
    pub fn partial(mut self) -> Self {
        if let Some(scope) = self.scope.take() {
            self.scope = Some(scope.partial());
        }
        self
    }

    /// Get the tool name
    pub fn tool_name(&self) -> &str {
        &self.tool_name
    }
}

/// Audit a fork creation
pub fn audit_fork_create(fork_id: &str, base_path: &Path) -> ForkAuditGuard {
    let span = audit_fork_span("create", fork_id);

    let scope = if let Some(logger) = get_audit_logger() {
        let details = serde_json::json!({
            "fork_id": fork_id,
            "base_path": base_path.display().to_string(),
        });
        Some(
            AuditScope::new(logger, AuditEventType::ForkCreate)
                .resource(fork_id.to_string())
                .details(details),
        )
    } else {
        None
    };

    ForkAuditGuard {
        scope,
        _span: span,
        fork_id: fork_id.to_string(),
    }
}

/// Audit a fork edit operation
pub fn audit_fork_edit(fork_id: &str, sheet: &str, edit_count: usize) -> ForkAuditGuard {
    let span = audit_fork_span("edit", fork_id);

    let scope = if let Some(logger) = get_audit_logger() {
        let details = serde_json::json!({
            "fork_id": fork_id,
            "sheet": sheet,
            "edit_count": edit_count,
        });
        Some(
            AuditScope::new(logger, AuditEventType::ForkEdit)
                .resource(fork_id.to_string())
                .details(details),
        )
    } else {
        None
    };

    ForkAuditGuard {
        scope,
        _span: span,
        fork_id: fork_id.to_string(),
    }
}

/// Audit a fork recalculation
pub fn audit_fork_recalc(fork_id: &str) -> ForkAuditGuard {
    let span = audit_fork_span("recalc", fork_id);

    let scope = if let Some(logger) = get_audit_logger() {
        let details = serde_json::json!({
            "fork_id": fork_id,
        });
        Some(
            AuditScope::new(logger, AuditEventType::ForkRecalc)
                .resource(fork_id.to_string())
                .details(details),
        )
    } else {
        None
    };

    ForkAuditGuard {
        scope,
        _span: span,
        fork_id: fork_id.to_string(),
    }
}

/// Audit a fork save operation
pub fn audit_fork_save(fork_id: &str, target_path: &Path, drop_fork: bool) -> ForkAuditGuard {
    let span = audit_fork_span("save", fork_id);

    let scope = if let Some(logger) = get_audit_logger() {
        let details = serde_json::json!({
            "fork_id": fork_id,
            "target_path": target_path.display().to_string(),
            "drop_fork": drop_fork,
        });
        Some(
            AuditScope::new(logger, AuditEventType::ForkSave)
                .resource(fork_id.to_string())
                .details(details),
        )
    } else {
        None
    };

    ForkAuditGuard {
        scope,
        _span: span,
        fork_id: fork_id.to_string(),
    }
}

/// Audit a fork discard operation
pub fn audit_fork_discard(fork_id: &str) -> ForkAuditGuard {
    let span = audit_fork_span("discard", fork_id);

    let scope = if let Some(logger) = get_audit_logger() {
        let details = serde_json::json!({
            "fork_id": fork_id,
        });
        Some(
            AuditScope::new(logger, AuditEventType::ForkDiscard)
                .resource(fork_id.to_string())
                .details(details),
        )
    } else {
        None
    };

    ForkAuditGuard {
        scope,
        _span: span,
        fork_id: fork_id.to_string(),
    }
}

/// Guard for fork operations
pub struct ForkAuditGuard {
    scope: Option<AuditScope>,
    _span: Span,
    fork_id: String,
}

impl ForkAuditGuard {
    /// Mark the fork operation as failed
    pub fn fail(mut self, error: impl Into<String>) -> Self {
        if let Some(scope) = self.scope.take() {
            self.scope = Some(scope.fail(error));
        }
        self
    }

    /// Mark the fork operation as partial success
    pub fn partial(mut self) -> Self {
        if let Some(scope) = self.scope.take() {
            self.scope = Some(scope.partial());
        }
        self
    }

    /// Get the fork ID
    pub fn fork_id(&self) -> &str {
        &self.fork_id
    }
}

/// Audit a checkpoint creation
pub fn audit_checkpoint_create(
    fork_id: &str,
    checkpoint_id: &str,
    label: Option<&str>,
) -> CheckpointAuditGuard {
    let span = audit_checkpoint_span("create", fork_id, Some(checkpoint_id));

    let scope = if let Some(logger) = get_audit_logger() {
        let details = serde_json::json!({
            "fork_id": fork_id,
            "checkpoint_id": checkpoint_id,
            "label": label,
        });
        Some(
            AuditScope::new(logger, AuditEventType::CheckpointCreate)
                .resource(format!("{}/{}", fork_id, checkpoint_id))
                .details(details),
        )
    } else {
        None
    };

    CheckpointAuditGuard { scope, _span: span }
}

/// Audit a checkpoint restoration
pub fn audit_checkpoint_restore(fork_id: &str, checkpoint_id: &str) -> CheckpointAuditGuard {
    let span = audit_checkpoint_span("restore", fork_id, Some(checkpoint_id));

    let scope = if let Some(logger) = get_audit_logger() {
        let details = serde_json::json!({
            "fork_id": fork_id,
            "checkpoint_id": checkpoint_id,
        });
        Some(
            AuditScope::new(logger, AuditEventType::CheckpointRestore)
                .resource(format!("{}/{}", fork_id, checkpoint_id))
                .details(details),
        )
    } else {
        None
    };

    CheckpointAuditGuard { scope, _span: span }
}

/// Audit a checkpoint deletion
pub fn audit_checkpoint_delete(fork_id: &str, checkpoint_id: &str) -> CheckpointAuditGuard {
    let span = audit_checkpoint_span("delete", fork_id, Some(checkpoint_id));

    let scope = if let Some(logger) = get_audit_logger() {
        let details = serde_json::json!({
            "fork_id": fork_id,
            "checkpoint_id": checkpoint_id,
        });
        Some(
            AuditScope::new(logger, AuditEventType::CheckpointDelete)
                .resource(format!("{}/{}", fork_id, checkpoint_id))
                .details(details),
        )
    } else {
        None
    };

    CheckpointAuditGuard { scope, _span: span }
}

/// Guard for checkpoint operations
pub struct CheckpointAuditGuard {
    scope: Option<AuditScope>,
    _span: Span,
}

impl CheckpointAuditGuard {
    /// Mark the checkpoint operation as failed
    pub fn fail(mut self, error: impl Into<String>) -> Self {
        if let Some(scope) = self.scope.take() {
            self.scope = Some(scope.fail(error));
        }
        self
    }
}

/// Audit a staged change creation
pub fn audit_staged_change_create(
    fork_id: &str,
    change_id: &str,
    op_count: usize,
) -> StagedChangeAuditGuard {
    let span = audit_staged_change_span("create", fork_id, Some(change_id));

    let scope = if let Some(logger) = get_audit_logger() {
        let details = serde_json::json!({
            "fork_id": fork_id,
            "change_id": change_id,
            "op_count": op_count,
        });
        Some(
            AuditScope::new(logger, AuditEventType::StagedChangeCreate)
                .resource(format!("{}/{}", fork_id, change_id))
                .details(details),
        )
    } else {
        None
    };

    StagedChangeAuditGuard { scope, _span: span }
}

/// Audit a staged change application
pub fn audit_staged_change_apply(fork_id: &str, change_id: &str) -> StagedChangeAuditGuard {
    let span = audit_staged_change_span("apply", fork_id, Some(change_id));

    let scope = if let Some(logger) = get_audit_logger() {
        let details = serde_json::json!({
            "fork_id": fork_id,
            "change_id": change_id,
        });
        Some(
            AuditScope::new(logger, AuditEventType::StagedChangeApply)
                .resource(format!("{}/{}", fork_id, change_id))
                .details(details),
        )
    } else {
        None
    };

    StagedChangeAuditGuard { scope, _span: span }
}

/// Audit a staged change discard
pub fn audit_staged_change_discard(fork_id: &str, change_id: &str) -> StagedChangeAuditGuard {
    let span = audit_staged_change_span("discard", fork_id, Some(change_id));

    let scope = if let Some(logger) = get_audit_logger() {
        let details = serde_json::json!({
            "fork_id": fork_id,
            "change_id": change_id,
        });
        Some(
            AuditScope::new(logger, AuditEventType::StagedChangeDiscard)
                .resource(format!("{}/{}", fork_id, change_id))
                .details(details),
        )
    } else {
        None
    };

    StagedChangeAuditGuard { scope, _span: span }
}

/// Guard for staged change operations
pub struct StagedChangeAuditGuard {
    scope: Option<AuditScope>,
    _span: Span,
}

impl StagedChangeAuditGuard {
    /// Mark the staged change operation as failed
    pub fn fail(mut self, error: impl Into<String>) -> Self {
        if let Some(scope) = self.scope.take() {
            self.scope = Some(scope.fail(error));
        }
        self
    }
}

/// Audit a file read operation
pub fn audit_file_read(path: &Path) {
    let _span = audit_file_span("read", path);

    if let Some(scope) = audit_scope(AuditEventType::FileRead) {
        let details = serde_json::json!({
            "path": path.display().to_string(),
        });
        drop(scope.resource(path.display().to_string()).details(details));
    }
}

/// Audit a file write operation
pub fn audit_file_write(path: &Path, size_bytes: Option<u64>) {
    let _span = audit_file_span("write", path);

    if let Some(scope) = audit_scope(AuditEventType::FileWrite) {
        let mut details = serde_json::json!({
            "path": path.display().to_string(),
        });
        if let Some(size) = size_bytes {
            details["size_bytes"] = serde_json::json!(size);
        }
        drop(scope.resource(path.display().to_string()).details(details));
    }
}

/// Audit a file copy operation
pub fn audit_file_copy(src: &Path, dst: &Path) {
    let _span = audit_file_span("copy", src);

    if let Some(scope) = audit_scope(AuditEventType::FileCopy) {
        let details = serde_json::json!({
            "source": src.display().to_string(),
            "destination": dst.display().to_string(),
        });
        drop(
            scope
                .resource(format!("{} -> {}", src.display(), dst.display()))
                .details(details),
        );
    }
}

/// Audit a file delete operation
pub fn audit_file_delete(path: &Path) {
    let _span = audit_file_span("delete", path);

    if let Some(scope) = audit_scope(AuditEventType::FileDelete) {
        let details = serde_json::json!({
            "path": path.display().to_string(),
        });
        drop(scope.resource(path.display().to_string()).details(details));
    }
}

/// Audit a directory creation
pub fn audit_dir_create(path: &Path) {
    let _span = audit_file_span("mkdir", path);

    if let Some(scope) = audit_scope(AuditEventType::DirectoryCreate) {
        let details = serde_json::json!({
            "path": path.display().to_string(),
        });
        drop(scope.resource(path.display().to_string()).details(details));
    }
}

/// Audit a workbook open operation
pub fn audit_workbook_open(workbook_id: &str, path: &Path) {
    let scope = if let Some(logger) = get_audit_logger() {
        let details = serde_json::json!({
            "workbook_id": workbook_id,
            "path": path.display().to_string(),
        });
        Some(
            AuditScope::new(logger, AuditEventType::WorkbookOpen)
                .resource(workbook_id.to_string())
                .details(details),
        )
    } else {
        None
    };

    drop(scope);
}

/// Audit a workbook close operation
pub fn audit_workbook_close(workbook_id: &str) {
    let scope = if let Some(logger) = get_audit_logger() {
        let details = serde_json::json!({
            "workbook_id": workbook_id,
        });
        Some(
            AuditScope::new(logger, AuditEventType::WorkbookClose)
                .resource(workbook_id.to_string())
                .details(details),
        )
    } else {
        None
    };

    drop(scope);
}

/// Audit an error event
pub fn audit_error(context: &str, error: &str) {
    let event = AuditEvent::new(AuditEventType::Error)
        .with_resource(context.to_string())
        .with_error(error);

    audit_event(event);
}
