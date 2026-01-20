//! Comprehensive audit trail logging for all MCP operations
//!
//! This module provides structured logging and audit trail capabilities for:
//! - Tool invocations with parameters
//! - Fork lifecycle events (create, edit, recalc, save, discard)
//! - File operations and modifications
//! - Checkpoint and staged change operations
//!
//! # Architecture
//!
//! The audit system uses the `tracing` crate for structured logging with spans,
//! combined with a persistent audit log that can be queried and analyzed.
//!
//! # Usage
//!
//! ```rust
//! use crate::audit::{AuditLogger, ToolInvocation, audit_tool_call};
//!
//! // Initialize the audit logger
//! let logger = AuditLogger::new(AuditConfig::default())?;
//!
//! // Log a tool invocation
//! let span = audit_tool_call("list_workbooks", &params);
//! // ... perform operation ...
//! drop(span);
//! ```

pub mod integration;
#[cfg(test)]
mod examples;

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::VecDeque;
use std::fs::{File, OpenOptions, self};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{Span, info, warn, error, info_span};

/// Maximum number of events to keep in memory
const DEFAULT_MEMORY_BUFFER_SIZE: usize = 10_000;

/// Maximum size of a log file before rotation (100 MB)
const DEFAULT_MAX_LOG_FILE_SIZE: u64 = 100 * 1024 * 1024;

/// Maximum number of rotated log files to keep
const DEFAULT_MAX_LOG_FILES: usize = 10;

/// Maximum age of log files in days before deletion
const DEFAULT_MAX_LOG_AGE_DAYS: i64 = 30;

/// Configuration for the audit logger
#[derive(Debug, Clone)]
pub struct AuditConfig {
    /// Directory where audit logs are stored
    pub log_dir: PathBuf,

    /// Maximum number of events to keep in memory
    pub memory_buffer_size: usize,

    /// Maximum size of a log file before rotation
    pub max_log_file_size: u64,

    /// Maximum number of rotated log files to keep
    pub max_log_files: usize,

    /// Maximum age of log files in days
    pub max_log_age_days: i64,

    /// Whether to enable persistent logging to disk
    pub persistent_logging: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            log_dir: PathBuf::from("/tmp/mcp-audit-logs"),
            memory_buffer_size: DEFAULT_MEMORY_BUFFER_SIZE,
            max_log_file_size: DEFAULT_MAX_LOG_FILE_SIZE,
            max_log_files: DEFAULT_MAX_LOG_FILES,
            max_log_age_days: DEFAULT_MAX_LOG_AGE_DAYS,
            persistent_logging: true,
        }
    }
}

/// Type of audit event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    /// Tool invocation
    ToolInvocation,

    /// Fork lifecycle events
    ForkCreate,
    ForkEdit,
    ForkRecalc,
    ForkSave,
    ForkDiscard,

    /// Checkpoint operations
    CheckpointCreate,
    CheckpointRestore,
    CheckpointDelete,

    /// Staged change operations
    StagedChangeCreate,
    StagedChangeApply,
    StagedChangeDiscard,

    /// File operations
    FileRead,
    FileWrite,
    FileCopy,
    FileDelete,
    DirectoryCreate,
    DirectoryDelete,

    /// Workbook operations
    WorkbookOpen,
    WorkbookClose,
    WorkbookList,

    /// Error events
    Error,
}

/// Outcome of an audited operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    Success,
    Failure,
    Partial,
}

/// Core audit event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID
    pub event_id: String,

    /// Event timestamp
    pub timestamp: DateTime<Utc>,

    /// Type of event
    pub event_type: AuditEventType,

    /// Event outcome
    pub outcome: AuditOutcome,

    /// User or session identifier (if applicable)
    pub principal: Option<String>,

    /// Resource affected (e.g., file path, fork ID)
    pub resource: Option<String>,

    /// Operation details as JSON
    pub details: JsonValue,

    /// Error message (if outcome is Failure)
    pub error: Option<String>,

    /// Duration in milliseconds
    pub duration_ms: Option<u64>,

    /// Parent span ID for correlation
    pub parent_span_id: Option<String>,
}

impl AuditEvent {
    /// Create a new audit event
    pub fn new(event_type: AuditEventType) -> Self {
        Self {
            event_id: crate::utils::make_short_random_id("evt", 16),
            timestamp: Utc::now(),
            event_type,
            outcome: AuditOutcome::Success,
            principal: None,
            resource: None,
            details: JsonValue::Null,
            error: None,
            duration_ms: None,
            parent_span_id: None,
        }
    }

    /// Set the outcome
    pub fn with_outcome(mut self, outcome: AuditOutcome) -> Self {
        self.outcome = outcome;
        self
    }

    /// Set the principal
    pub fn with_principal(mut self, principal: impl Into<String>) -> Self {
        self.principal = Some(principal.into());
        self
    }

    /// Set the resource
    pub fn with_resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into());
        self
    }

    /// Set the details
    pub fn with_details(mut self, details: JsonValue) -> Self {
        self.details = details;
        self
    }

    /// Set the error message
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self.outcome = AuditOutcome::Failure;
        self
    }

    /// Set the duration
    pub fn with_duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Set the parent span ID
    pub fn with_parent_span(mut self, span_id: impl Into<String>) -> Self {
        self.parent_span_id = Some(span_id.into());
        self
    }
}

/// Audit logger implementation
pub struct AuditLogger {
    config: AuditConfig,
    buffer: Arc<RwLock<VecDeque<AuditEvent>>>,
    log_file: Arc<RwLock<Option<BufWriter<File>>>>,
    current_log_path: Arc<RwLock<Option<PathBuf>>>,
    current_log_size: Arc<RwLock<u64>>,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(config: AuditConfig) -> Result<Self> {
        if config.persistent_logging {
            fs::create_dir_all(&config.log_dir)
                .context("failed to create audit log directory")?;
        }

        let logger = Self {
            config,
            buffer: Arc::new(RwLock::new(VecDeque::new())),
            log_file: Arc::new(RwLock::new(None)),
            current_log_path: Arc::new(RwLock::new(None)),
            current_log_size: Arc::new(RwLock::new(0)),
        };

        if logger.config.persistent_logging {
            logger.rotate_log_if_needed()?;
        }

        Ok(logger)
    }

    /// Log an audit event
    pub fn log(&self, event: AuditEvent) {
        // Add to memory buffer
        {
            let mut buffer = self.buffer.write();
            buffer.push_back(event.clone());

            // Trim buffer if it exceeds max size
            while buffer.len() > self.config.memory_buffer_size {
                buffer.pop_front();
            }
        }

        // Log to tracing
        match event.outcome {
            AuditOutcome::Success => {
                info!(
                    event_id = %event.event_id,
                    event_type = ?event.event_type,
                    resource = ?event.resource,
                    duration_ms = ?event.duration_ms,
                    "audit event"
                );
            }
            AuditOutcome::Failure => {
                error!(
                    event_id = %event.event_id,
                    event_type = ?event.event_type,
                    resource = ?event.resource,
                    error = ?event.error,
                    "audit event failed"
                );
            }
            AuditOutcome::Partial => {
                warn!(
                    event_id = %event.event_id,
                    event_type = ?event.event_type,
                    resource = ?event.resource,
                    duration_ms = ?event.duration_ms,
                    "audit event partial"
                );
            }
        }

        // Persist to disk
        if self.config.persistent_logging {
            if let Err(e) = self.persist_event(&event) {
                error!("failed to persist audit event: {}", e);
            }
        }
    }

    /// Persist an event to disk
    fn persist_event(&self, event: &AuditEvent) -> Result<()> {
        // Check if rotation is needed
        self.rotate_log_if_needed()?;

        let mut log_file = self.log_file.write();
        if let Some(writer) = log_file.as_mut() {
            let json = serde_json::to_string(event)?;
            writeln!(writer, "{}", json)?;
            writer.flush()?;

            // Update current size
            let mut size = self.current_log_size.write();
            *size += json.len() as u64 + 1; // +1 for newline
        }

        Ok(())
    }

    /// Rotate log file if needed
    fn rotate_log_if_needed(&self) -> Result<()> {
        let current_size = *self.current_log_size.read();

        // Check if rotation is needed
        let needs_rotation = {
            let current_path = self.current_log_path.read();
            current_path.is_none() || current_size >= self.config.max_log_file_size
        };

        if !needs_rotation {
            return Ok(());
        }

        // Close current log file
        {
            let mut log_file = self.log_file.write();
            *log_file = None;
        }

        // Create new log file
        let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
        let new_log_path = self.config.log_dir.join(format!("audit-{}.jsonl", timestamp));

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&new_log_path)
            .context("failed to create new audit log file")?;

        let writer = BufWriter::new(file);

        {
            let mut log_file = self.log_file.write();
            *log_file = Some(writer);
        }

        {
            let mut current_log_path = self.current_log_path.write();
            *current_log_path = Some(new_log_path.clone());
        }

        {
            let mut current_log_size = self.current_log_size.write();
            *current_log_size = 0;
        }

        info!("rotated audit log to {}", new_log_path.display());

        // Clean up old log files
        self.cleanup_old_logs()?;

        Ok(())
    }

    /// Clean up old log files based on retention policy
    fn cleanup_old_logs(&self) -> Result<()> {
        let entries = fs::read_dir(&self.config.log_dir)?;

        let mut log_files: Vec<(PathBuf, DateTime<Utc>)> = Vec::new();

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                if filename.starts_with("audit-") && filename.ends_with(".jsonl") {
                    if let Ok(metadata) = fs::metadata(&path) {
                        if let Ok(modified) = metadata.modified() {
                            let datetime: DateTime<Utc> = modified.into();
                            log_files.push((path, datetime));
                        }
                    }
                }
            }
        }

        // Sort by modification time (newest first)
        log_files.sort_by(|a, b| b.1.cmp(&a.1));

        let now = Utc::now();
        let max_age = chrono::Duration::days(self.config.max_log_age_days);

        // Delete files that are too old or exceed max count
        for (i, (path, modified)) in log_files.iter().enumerate() {
            let age = now - *modified;

            if i >= self.config.max_log_files || age > max_age {
                if let Err(e) = fs::remove_file(path) {
                    warn!("failed to delete old audit log {}: {}", path.display(), e);
                } else {
                    info!("deleted old audit log {}", path.display());
                }
            }
        }

        Ok(())
    }

    /// Query events from the in-memory buffer
    pub fn query_events(&self, filter: AuditFilter) -> Vec<AuditEvent> {
        let buffer = self.buffer.read();

        buffer
            .iter()
            .filter(|event| filter.matches(event))
            .take(filter.limit.unwrap_or(usize::MAX))
            .cloned()
            .collect()
    }

    /// Get recent events
    pub fn recent_events(&self, limit: usize) -> Vec<AuditEvent> {
        let buffer = self.buffer.read();
        buffer.iter().rev().take(limit).cloned().collect()
    }

    /// Get event count in memory buffer
    pub fn event_count(&self) -> usize {
        self.buffer.read().len()
    }

    /// Export all events from memory buffer
    pub fn export_events(&self) -> Vec<AuditEvent> {
        self.buffer.read().iter().cloned().collect()
    }
}

/// Filter for querying audit events
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    pub event_type: Option<AuditEventType>,
    pub outcome: Option<AuditOutcome>,
    pub resource: Option<String>,
    pub principal: Option<String>,
    pub after: Option<DateTime<Utc>>,
    pub before: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

impl AuditFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_event_type(mut self, event_type: AuditEventType) -> Self {
        self.event_type = Some(event_type);
        self
    }

    pub fn with_outcome(mut self, outcome: AuditOutcome) -> Self {
        self.outcome = Some(outcome);
        self
    }

    pub fn with_resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into());
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    fn matches(&self, event: &AuditEvent) -> bool {
        if let Some(event_type) = &self.event_type {
            if &event.event_type != event_type {
                return false;
            }
        }

        if let Some(outcome) = &self.outcome {
            if &event.outcome != outcome {
                return false;
            }
        }

        if let Some(resource) = &self.resource {
            if event.resource.as_ref() != Some(resource) {
                return false;
            }
        }

        if let Some(principal) = &self.principal {
            if event.principal.as_ref() != Some(principal) {
                return false;
            }
        }

        if let Some(after) = &self.after {
            if &event.timestamp < after {
                return false;
            }
        }

        if let Some(before) = &self.before {
            if &event.timestamp > before {
                return false;
            }
        }

        true
    }
}

/// Helper to create a tracing span for tool invocations
pub fn audit_tool_span(tool_name: &str, params: &impl Serialize) -> Span {
    let params_json = serde_json::to_value(params)
        .unwrap_or(JsonValue::Null);

    info_span!(
        "tool_invocation",
        tool = tool_name,
        params = ?params_json,
        outcome = tracing::field::Empty,
        duration_ms = tracing::field::Empty,
    )
}

/// Helper to create a tracing span for fork operations
pub fn audit_fork_span(operation: &str, fork_id: &str) -> Span {
    info_span!(
        "fork_operation",
        operation = operation,
        fork_id = fork_id,
        outcome = tracing::field::Empty,
        duration_ms = tracing::field::Empty,
    )
}

/// Helper to create a tracing span for file operations
pub fn audit_file_span(operation: &str, path: &Path) -> Span {
    info_span!(
        "file_operation",
        operation = operation,
        path = %path.display(),
        outcome = tracing::field::Empty,
        duration_ms = tracing::field::Empty,
    )
}

/// Helper to create a tracing span for checkpoint operations
pub fn audit_checkpoint_span(operation: &str, fork_id: &str, checkpoint_id: Option<&str>) -> Span {
    info_span!(
        "checkpoint_operation",
        operation = operation,
        fork_id = fork_id,
        checkpoint_id = ?checkpoint_id,
        outcome = tracing::field::Empty,
        duration_ms = tracing::field::Empty,
    )
}

/// Helper to create a tracing span for staged change operations
pub fn audit_staged_change_span(operation: &str, fork_id: &str, change_id: Option<&str>) -> Span {
    info_span!(
        "staged_change_operation",
        operation = operation,
        fork_id = fork_id,
        change_id = ?change_id,
        outcome = tracing::field::Empty,
        duration_ms = tracing::field::Empty,
    )
}

/// Scoped audit event that logs on drop
pub struct AuditScope {
    logger: Arc<AuditLogger>,
    event: AuditEvent,
    start_time: std::time::Instant,
}

impl AuditScope {
    /// Create a new audit scope
    pub fn new(logger: Arc<AuditLogger>, event_type: AuditEventType) -> Self {
        Self {
            logger,
            event: AuditEvent::new(event_type),
            start_time: std::time::Instant::now(),
        }
    }

    /// Set the resource
    pub fn resource(mut self, resource: impl Into<String>) -> Self {
        self.event.resource = Some(resource.into());
        self
    }

    /// Set the principal
    pub fn principal(mut self, principal: impl Into<String>) -> Self {
        self.event.principal = Some(principal.into());
        self
    }

    /// Set the details
    pub fn details(mut self, details: JsonValue) -> Self {
        self.event.details = details;
        self
    }

    /// Mark as failed with error message
    pub fn fail(mut self, error: impl Into<String>) -> Self {
        self.event.error = Some(error.into());
        self.event.outcome = AuditOutcome::Failure;
        self
    }

    /// Mark as partial success
    pub fn partial(mut self) -> Self {
        self.event.outcome = AuditOutcome::Partial;
        self
    }
}

impl Drop for AuditScope {
    fn drop(&mut self) {
        let duration_ms = self.start_time.elapsed().as_millis() as u64;
        self.event.duration_ms = Some(duration_ms);
        self.logger.log(self.event.clone());
    }
}

/// Global audit logger instance
static AUDIT_LOGGER: once_cell::sync::OnceCell<Arc<AuditLogger>> = once_cell::sync::OnceCell::new();

/// Initialize the global audit logger
pub fn init_audit_logger(config: AuditConfig) -> Result<()> {
    let logger = Arc::new(AuditLogger::new(config)?);
    AUDIT_LOGGER
        .set(logger)
        .map_err(|_| anyhow!("audit logger already initialized"))?;
    info!("audit logger initialized");
    Ok(())
}

/// Get the global audit logger
pub fn get_audit_logger() -> Option<Arc<AuditLogger>> {
    AUDIT_LOGGER.get().cloned()
}

/// Log an audit event to the global logger
pub fn audit_event(event: AuditEvent) {
    if let Some(logger) = get_audit_logger() {
        logger.log(event);
    }
}

/// Create an audit scope that logs on drop
pub fn audit_scope(event_type: AuditEventType) -> Option<AuditScope> {
    get_audit_logger().map(|logger| AuditScope::new(logger, event_type))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent::new(AuditEventType::ToolInvocation)
            .with_resource("test-resource")
            .with_outcome(AuditOutcome::Success)
            .with_duration_ms(100);

        assert_eq!(event.event_type, AuditEventType::ToolInvocation);
        assert_eq!(event.outcome, AuditOutcome::Success);
        assert_eq!(event.resource, Some("test-resource".to_string()));
        assert_eq!(event.duration_ms, Some(100));
    }

    #[test]
    fn test_audit_filter() {
        let event = AuditEvent::new(AuditEventType::ForkCreate)
            .with_resource("fork-123")
            .with_outcome(AuditOutcome::Success);

        let filter = AuditFilter::new()
            .with_event_type(AuditEventType::ForkCreate);

        assert!(filter.matches(&event));

        let filter = AuditFilter::new()
            .with_event_type(AuditEventType::ForkEdit);

        assert!(!filter.matches(&event));
    }

    #[test]
    fn test_audit_logger() -> Result<()> {
        let config = AuditConfig {
            persistent_logging: false,
            memory_buffer_size: 10,
            ..Default::default()
        };

        let logger = AuditLogger::new(config)?;

        for i in 0..15 {
            let event = AuditEvent::new(AuditEventType::ToolInvocation)
                .with_resource(format!("resource-{}", i));
            logger.log(event);
        }

        // Should only keep last 10 events
        assert_eq!(logger.event_count(), 10);

        let events = logger.recent_events(5);
        assert_eq!(events.len(), 5);

        Ok(())
    }
}
