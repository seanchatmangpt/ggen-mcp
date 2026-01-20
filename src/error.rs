//! Comprehensive error handling for the spreadsheet MCP server
//!
//! This module provides:
//! - Expanded MCP error codes (JSON-RPC standard + custom codes)
//! - Rich error context with operation details
//! - Error telemetry and metrics
//! - Actionable error messages with suggestions
//! - Error recovery hints
//! - Builder pattern for constructing errors

use anyhow::{Context as _, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

// =============================================================================
// MCP ERROR CODES
// =============================================================================

/// MCP error codes following JSON-RPC 2.0 specification plus custom codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(i32)]
pub enum ErrorCode {
    // Standard JSON-RPC errors (-32700 to -32603)
    /// Invalid JSON was received by the server
    ParseError = -32700,
    /// The JSON sent is not a valid Request object
    InvalidRequest = -32600,
    /// The method does not exist / is not available
    MethodNotFound = -32601,
    /// Invalid method parameter(s)
    InvalidParams = -32602,
    /// Internal JSON-RPC error
    InternalError = -32603,

    // Custom application errors (-32000 to -32099)
    /// Workbook file not found or not accessible
    WorkbookNotFound = -32001,
    /// Fork not found or expired
    ForkNotFound = -32002,
    /// Recalculation operation timed out
    RecalcTimeout = -32003,
    /// Parameter validation failed
    ValidationError = -32004,
    /// Resource limits exceeded (memory, size, etc.)
    ResourceExhausted = -32005,
    /// Sheet not found in workbook
    SheetNotFound = -32006,
    /// Range address is invalid or out of bounds
    InvalidRange = -32007,
    /// Named range or table not found
    NamedRangeNotFound = -32008,
    /// VBA operation failed
    VbaError = -32009,
    /// SPARQL query failed
    SparqlError = -32010,
    /// Template rendering failed
    TemplateError = -32011,
    /// File I/O error
    IoError = -32012,
    /// Permission denied
    PermissionDenied = -32013,
    /// Tool disabled by configuration
    ToolDisabled = -32014,
    /// Response too large
    ResponseTooLarge = -32015,
    /// Checkpoint not found
    CheckpointNotFound = -32016,
    /// Staged change not found
    StagedChangeNotFound = -32017,
    /// Region not found
    RegionNotFound = -32018,
    /// Formula parse error
    FormulaParseError = -32019,
    /// Entitlement required for capability
    EntitlementRequired = -32020,
}

impl ErrorCode {
    /// Get the integer code
    pub fn code(&self) -> i32 {
        *self as i32
    }

    /// Check if this error type is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ErrorCode::InternalError
                | ErrorCode::RecalcTimeout
                | ErrorCode::ResourceExhausted
                | ErrorCode::IoError
        )
    }

    /// Get the error category for metrics
    pub fn category(&self) -> &'static str {
        match self {
            ErrorCode::ParseError | ErrorCode::InvalidRequest | ErrorCode::InvalidParams => {
                "client_error"
            }
            ErrorCode::MethodNotFound | ErrorCode::ToolDisabled => "not_found",
            ErrorCode::InternalError => "server_error",
            ErrorCode::WorkbookNotFound
            | ErrorCode::ForkNotFound
            | ErrorCode::SheetNotFound
            | ErrorCode::NamedRangeNotFound
            | ErrorCode::CheckpointNotFound
            | ErrorCode::StagedChangeNotFound
            | ErrorCode::RegionNotFound => "resource_not_found",
            ErrorCode::RecalcTimeout => "timeout",
            ErrorCode::ValidationError | ErrorCode::InvalidRange | ErrorCode::FormulaParseError => {
                "validation_error"
            }
            ErrorCode::EntitlementRequired => "entitlement_error",
            ErrorCode::ResourceExhausted | ErrorCode::ResponseTooLarge => "resource_limit",
            ErrorCode::VbaError | ErrorCode::SparqlError | ErrorCode::TemplateError => {
                "subsystem_error"
            }
            ErrorCode::IoError | ErrorCode::PermissionDenied => "io_error",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}({})", self, self.code())
    }
}

// =============================================================================
// ERROR CONTEXT
// =============================================================================

/// Rich context information for errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Operation that was being performed
    pub operation: Option<String>,
    /// Workbook ID if relevant
    pub workbook_id: Option<String>,
    /// Fork ID if relevant
    pub fork_id: Option<String>,
    /// Sheet name if relevant
    pub sheet_name: Option<String>,
    /// Cell range if relevant
    pub range: Option<String>,
    /// Additional parameters
    pub params: HashMap<String, serde_json::Value>,
    /// Suggestions for fixing the error
    pub suggestions: Vec<String>,
    /// Related errors or context
    pub related_errors: Vec<String>,
    /// Documentation link
    pub doc_link: Option<String>,
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self {
            operation: None,
            workbook_id: None,
            fork_id: None,
            sheet_name: None,
            range: None,
            params: HashMap::new(),
            suggestions: Vec::new(),
            related_errors: Vec::new(),
            doc_link: None,
        }
    }
}

// =============================================================================
// ERROR RECOVERY HINTS
// =============================================================================

/// Information about how to recover from an error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryHints {
    /// Whether the operation can be retried
    pub is_retryable: bool,
    /// Suggested delay before retry (seconds)
    pub retry_after: Option<u32>,
    /// Expected fix description
    pub expected_fix: Option<String>,
    /// Alternative approaches
    pub alternatives: Vec<String>,
}

impl Default for RecoveryHints {
    fn default() -> Self {
        Self {
            is_retryable: false,
            retry_after: None,
            expected_fix: None,
            alternatives: Vec::new(),
        }
    }
}

// =============================================================================
// MCP ERROR TYPE
// =============================================================================

/// Main error type for MCP operations
#[derive(Debug, Clone, Serialize)]
pub struct McpError {
    /// Error code
    pub code: ErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Unique error ID for tracking
    pub error_id: String,
    /// Rich context information
    pub context: ErrorContext,
    /// Recovery hints
    pub recovery: RecoveryHints,
    /// Timestamp when error occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl McpError {
    /// Create a new error with the given code and message
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            error_id: Self::generate_error_id(),
            context: ErrorContext::default(),
            recovery: RecoveryHints {
                is_retryable: code.is_retryable(),
                ..Default::default()
            },
            timestamp: chrono::Utc::now(),
        }
    }

    /// Start building an error with the builder pattern
    pub fn builder(code: ErrorCode) -> ErrorBuilder {
        ErrorBuilder::new(code)
    }

    /// Create a validation error
    pub fn validation() -> ErrorBuilder {
        ErrorBuilder::new(ErrorCode::ValidationError)
    }

    /// Create an invalid params error
    pub fn invalid_params() -> ErrorBuilder {
        ErrorBuilder::new(ErrorCode::InvalidParams)
    }

    /// Create a not found error
    pub fn not_found() -> ErrorBuilder {
        ErrorBuilder::new(ErrorCode::WorkbookNotFound)
    }

    /// Create an internal error
    pub fn internal() -> ErrorBuilder {
        ErrorBuilder::new(ErrorCode::InternalError)
    }

    /// Add this error to telemetry
    pub fn track(&self) {
        ERROR_METRICS.record_error(&self.code, self.context.operation.as_deref());
    }

    /// Generate a unique error ID
    fn generate_error_id() -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let count = COUNTER.fetch_add(1, Ordering::Relaxed);
        let timestamp = chrono::Utc::now().timestamp_millis();
        format!("err_{:x}_{:x}", timestamp, count)
    }

    /// Convert to anyhow::Error
    pub fn into_anyhow(self) -> anyhow::Error {
        anyhow::anyhow!("{}", self.message)
    }
}

impl fmt::Display for McpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)?;
        if !self.context.suggestions.is_empty() {
            write!(f, "\nSuggestions:\n")?;
            for (i, suggestion) in self.context.suggestions.iter().enumerate() {
                write!(f, "  {}. {}\n", i + 1, suggestion)?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for McpError {}

// =============================================================================
// ERROR BUILDER
// =============================================================================

/// Builder for constructing rich errors
pub struct ErrorBuilder {
    error: McpError,
}

impl ErrorBuilder {
    fn new(code: ErrorCode) -> Self {
        Self {
            error: McpError::new(code, ""),
        }
    }

    /// Set the error message
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.error.message = message.into();
        self
    }

    /// Set the operation context
    pub fn operation(mut self, operation: impl Into<String>) -> Self {
        self.error.context.operation = Some(operation.into());
        self
    }

    /// Set the workbook ID context
    pub fn workbook_id(mut self, workbook_id: impl Into<String>) -> Self {
        self.error.context.workbook_id = Some(workbook_id.into());
        self
    }

    /// Set the fork ID context
    pub fn fork_id(mut self, fork_id: impl Into<String>) -> Self {
        self.error.context.fork_id = Some(fork_id.into());
        self
    }

    /// Set the sheet name context
    pub fn sheet_name(mut self, sheet_name: impl Into<String>) -> Self {
        self.error.context.sheet_name = Some(sheet_name.into());
        self
    }

    /// Set the range context
    pub fn range(mut self, range: impl Into<String>) -> Self {
        self.error.context.range = Some(range.into());
        self
    }

    /// Add a parameter to the context
    pub fn param(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.error.context.params.insert(key.into(), json_value);
        }
        self
    }

    /// Add a suggestion for fixing the error
    pub fn suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.error.context.suggestions.push(suggestion.into());
        self
    }

    /// Add multiple suggestions
    pub fn suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.error.context.suggestions.extend(suggestions);
        self
    }

    /// Add a related error message
    pub fn related_error(mut self, error: impl Into<String>) -> Self {
        self.error.context.related_errors.push(error.into());
        self
    }

    /// Set the documentation link
    pub fn doc_link(mut self, link: impl Into<String>) -> Self {
        self.error.context.doc_link = Some(link.into());
        self
    }

    /// Set whether the error is retryable
    pub fn retryable(mut self, retryable: bool) -> Self {
        self.error.recovery.is_retryable = retryable;
        self
    }

    /// Set the retry delay
    pub fn retry_after(mut self, seconds: u32) -> Self {
        self.error.recovery.retry_after = Some(seconds);
        self.error.recovery.is_retryable = true;
        self
    }

    /// Set the expected fix
    pub fn expected_fix(mut self, fix: impl Into<String>) -> Self {
        self.error.recovery.expected_fix = Some(fix.into());
        self
    }

    /// Add an alternative approach
    pub fn alternative(mut self, alternative: impl Into<String>) -> Self {
        self.error.recovery.alternatives.push(alternative.into());
        self
    }

    /// Build the error
    pub fn build(self) -> McpError {
        self.error
    }

    /// Build and track the error
    pub fn build_and_track(self) -> McpError {
        let error = self.error;
        error.track();
        error
    }
}

// =============================================================================
// ERROR TELEMETRY
// =============================================================================

/// Error metrics for telemetry
#[derive(Debug)]
pub struct ErrorMetrics {
    /// Total error count by error code
    error_counts: RwLock<HashMap<ErrorCode, AtomicU64>>,
    /// Error count by tool/operation
    tool_errors: RwLock<HashMap<String, AtomicU64>>,
    /// Error count by category
    category_counts: RwLock<HashMap<String, AtomicU64>>,
}

impl ErrorMetrics {
    pub fn new() -> Self {
        Self {
            error_counts: RwLock::new(HashMap::new()),
            tool_errors: RwLock::new(HashMap::new()),
            category_counts: RwLock::new(HashMap::new()),
        }
    }

    /// Record an error occurrence
    pub fn record_error(&self, code: &ErrorCode, tool: Option<&str>) {
        // Increment error code counter
        {
            let map = self.error_counts.read();
            if let Some(counter) = map.get(code) {
                counter.fetch_add(1, Ordering::Relaxed);
            } else {
                drop(map);
                let mut map = self.error_counts.write();
                map.entry(*code)
                    .or_insert_with(|| AtomicU64::new(0))
                    .fetch_add(1, Ordering::Relaxed);
            }
        }

        // Increment tool counter if provided
        if let Some(tool_name) = tool {
            let map = self.tool_errors.read();
            if let Some(counter) = map.get(tool_name) {
                counter.fetch_add(1, Ordering::Relaxed);
            } else {
                drop(map);
                let mut map = self.tool_errors.write();
                map.entry(tool_name.to_string())
                    .or_insert_with(|| AtomicU64::new(0))
                    .fetch_add(1, Ordering::Relaxed);
            }
        }

        // Increment category counter
        let category = code.category();
        {
            let map = self.category_counts.read();
            if let Some(counter) = map.get(category) {
                counter.fetch_add(1, Ordering::Relaxed);
            } else {
                drop(map);
                let mut map = self.category_counts.write();
                map.entry(category.to_string())
                    .or_insert_with(|| AtomicU64::new(0))
                    .fetch_add(1, Ordering::Relaxed);
            }
        }

        tracing::debug!(
            error_code = %code,
            tool = tool,
            category = category,
            "error recorded"
        );
    }

    /// Get error count for a specific code
    pub fn get_error_count(&self, code: &ErrorCode) -> u64 {
        self.error_counts
            .read()
            .get(code)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Get error count for a specific tool
    pub fn get_tool_error_count(&self, tool: &str) -> u64 {
        self.tool_errors
            .read()
            .get(tool)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Get error count for a category
    pub fn get_category_count(&self, category: &str) -> u64 {
        self.category_counts
            .read()
            .get(category)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Get all error statistics
    pub fn get_stats(&self) -> ErrorStats {
        let error_counts = self
            .error_counts
            .read()
            .iter()
            .map(|(code, counter)| (*code, counter.load(Ordering::Relaxed)))
            .collect();

        let tool_errors = self
            .tool_errors
            .read()
            .iter()
            .map(|(tool, counter)| (tool.clone(), counter.load(Ordering::Relaxed)))
            .collect();

        let category_counts = self
            .category_counts
            .read()
            .iter()
            .map(|(category, counter)| (category.clone(), counter.load(Ordering::Relaxed)))
            .collect();

        ErrorStats {
            error_counts,
            tool_errors,
            category_counts,
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.error_counts.write().clear();
        self.tool_errors.write().clear();
        self.category_counts.write().clear();
    }
}

impl Default for ErrorMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Error statistics snapshot
#[derive(Debug, Clone, Serialize)]
pub struct ErrorStats {
    pub error_counts: HashMap<ErrorCode, u64>,
    pub tool_errors: HashMap<String, u64>,
    pub category_counts: HashMap<String, u64>,
}

/// Global error metrics instance
pub static ERROR_METRICS: once_cell::sync::Lazy<ErrorMetrics> =
    once_cell::sync::Lazy::new(ErrorMetrics::new);

// =============================================================================
// CONTEXT HELPERS
// =============================================================================

/// Extension trait for adding context to Results
pub trait ResultExt<T> {
    /// Add operation context
    fn with_operation(self, operation: &str) -> Result<T>;

    /// Add workbook context
    fn with_workbook(self, workbook_id: &str) -> Result<T>;

    /// Add sheet context
    fn with_sheet(self, sheet_name: &str) -> Result<T>;

    /// Add range context
    fn with_range(self, range: &str) -> Result<T>;
}

impl<T, E> ResultExt<T> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn with_operation(self, operation: &str) -> Result<T> {
        self.with_context(|| format!("Operation '{}' failed", operation))
    }

    fn with_workbook(self, workbook_id: &str) -> Result<T> {
        self.with_context(|| format!("Error in workbook '{}'", workbook_id))
    }

    fn with_sheet(self, sheet_name: &str) -> Result<T> {
        self.with_context(|| format!("Error in sheet '{}'", sheet_name))
    }

    fn with_range(self, range: &str) -> Result<T> {
        self.with_context(|| format!("Error in range '{}'", range))
    }
}

// =============================================================================
// CONVERSION FROM COMMON ERROR TYPES
// =============================================================================

/// Convert anyhow::Error to McpError
pub fn to_mcp_error(error: anyhow::Error) -> McpError {
    // Check error message for common patterns to determine appropriate code
    let error_msg = error.to_string().to_lowercase();

    let code = if error_msg.contains("workbook") && error_msg.contains("not found") {
        ErrorCode::WorkbookNotFound
    } else if error_msg.contains("fork") && error_msg.contains("not found") {
        ErrorCode::ForkNotFound
    } else if error_msg.contains("sheet") && error_msg.contains("not found") {
        ErrorCode::SheetNotFound
    } else if error_msg.contains("range") && error_msg.contains("invalid") {
        ErrorCode::InvalidRange
    } else if error_msg.contains("timeout") || error_msg.contains("timed out") {
        ErrorCode::RecalcTimeout
    } else if error_msg.contains("validation")
        || error_msg.contains("invalid")
        || error_msg.contains("malformed")
    {
        ErrorCode::ValidationError
    } else if error_msg.contains("too large") || error_msg.contains("exceeds limit") {
        ErrorCode::ResponseTooLarge
    } else if error_msg.contains("permission denied") || error_msg.contains("access denied") {
        ErrorCode::PermissionDenied
    } else if error_msg.contains("disabled") {
        ErrorCode::ToolDisabled
    } else if error_msg.contains("parse") {
        ErrorCode::ParseError
    } else if error_msg.contains("entitlement") || error_msg.contains("capability") {
        ErrorCode::EntitlementRequired
    } else {
        ErrorCode::InternalError
    };

    let mut builder = McpError::builder(code).message(error.to_string());

    // Extract context from error chain
    for (i, cause) in error.chain().enumerate().skip(1) {
        if i < 3 {
            // Limit to 3 related errors
            builder = builder.related_error(cause.to_string());
        }
    }

    // Add suggestions based on error type
    match code {
        ErrorCode::WorkbookNotFound => {
            builder = builder
                .suggestion("Check that the workbook path is correct")
                .suggestion("Use list_workbooks to see available workbooks")
                .suggestion("Ensure the file has a supported extension (.xlsx, .xlsm)");
        }
        ErrorCode::ForkNotFound => {
            builder = builder
                .suggestion("The fork may have expired (1 hour timeout)")
                .suggestion("Use list_forks to see active forks")
                .suggestion("Create a new fork with create_fork");
        }
        ErrorCode::SheetNotFound => {
            builder = builder
                .suggestion("Use list_sheets to see available sheets")
                .suggestion("Check for typos in the sheet name")
                .suggestion("Sheet names are case-sensitive");
        }
        ErrorCode::InvalidRange => {
            builder = builder
                .suggestion("Use A1 notation (e.g., A1:C10)")
                .suggestion("Ensure the range is within sheet bounds")
                .suggestion("Use sheet_overview to see sheet dimensions");
        }
        ErrorCode::RecalcTimeout => {
            builder = builder
                .suggestion("Try reducing the scope of the recalculation")
                .suggestion("The workbook may have circular references")
                .retryable(true)
                .retry_after(5);
        }
        ErrorCode::ResponseTooLarge => {
            builder = builder
                .suggestion("Use limit and offset parameters for pagination")
                .suggestion("Narrow the range or use filters")
                .suggestion("Try summary_only=true for changesets");
        }
        _ => {}
    }

    let mcp_error = builder.build_and_track();
    mcp_error
}

/// Convert McpError to rmcp::ErrorData
pub fn to_rmcp_error(error: McpError) -> rmcp::ErrorData {
    let data = serde_json::to_value(&error).ok();

    match error.code {
        ErrorCode::InvalidRequest | ErrorCode::ToolDisabled | ErrorCode::ResponseTooLarge => {
            rmcp::ErrorData::invalid_request(error.message, data)
        }
        ErrorCode::InvalidParams
        | ErrorCode::ValidationError
        | ErrorCode::InvalidRange
        | ErrorCode::FormulaParseError => rmcp::ErrorData::invalid_params(error.message, data),
        ErrorCode::MethodNotFound => {
            // Use ErrorData::new directly since method_not_found is a generic function
            rmcp::ErrorData::new(rmcp::model::ErrorCode::METHOD_NOT_FOUND, error.message, data)
        }
        _ => rmcp::ErrorData::internal_error(error.message, data),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_values() {
        assert_eq!(ErrorCode::ParseError.code(), -32700);
        assert_eq!(ErrorCode::InvalidRequest.code(), -32600);
        assert_eq!(ErrorCode::MethodNotFound.code(), -32601);
        assert_eq!(ErrorCode::InvalidParams.code(), -32602);
        assert_eq!(ErrorCode::InternalError.code(), -32603);
        assert_eq!(ErrorCode::WorkbookNotFound.code(), -32001);
    }

    #[test]
    fn test_error_builder() {
        let error = McpError::validation()
            .message("Invalid row number")
            .operation("read_table")
            .workbook_id("test.xlsx")
            .sheet_name("Sheet1")
            .range("A1:Z1000000")
            .param("row", 2000000)
            .suggestion("Row must be between 1 and 1,048,576")
            .build();

        assert_eq!(error.code, ErrorCode::ValidationError);
        assert_eq!(error.message, "Invalid row number");
        assert_eq!(error.context.operation, Some("read_table".to_string()));
        assert_eq!(error.context.workbook_id, Some("test.xlsx".to_string()));
        assert_eq!(error.context.sheet_name, Some("Sheet1".to_string()));
        assert_eq!(error.context.suggestions.len(), 1);
    }

    #[test]
    fn test_error_categories() {
        assert_eq!(ErrorCode::InvalidParams.category(), "client_error");
        assert_eq!(ErrorCode::InternalError.category(), "server_error");
        assert_eq!(ErrorCode::WorkbookNotFound.category(), "resource_not_found");
        assert_eq!(ErrorCode::RecalcTimeout.category(), "timeout");
    }

    #[test]
    fn test_error_metrics() {
        let metrics = ErrorMetrics::new();

        metrics.record_error(&ErrorCode::ValidationError, Some("read_table"));
        metrics.record_error(&ErrorCode::ValidationError, Some("read_table"));
        metrics.record_error(&ErrorCode::WorkbookNotFound, Some("describe_workbook"));

        assert_eq!(metrics.get_error_count(&ErrorCode::ValidationError), 2);
        assert_eq!(metrics.get_error_count(&ErrorCode::WorkbookNotFound), 1);
        assert_eq!(metrics.get_tool_error_count("read_table"), 2);
        assert_eq!(metrics.get_category_count("validation_error"), 2);
    }

    #[test]
    fn test_retryable_errors() {
        assert!(ErrorCode::InternalError.is_retryable());
        assert!(ErrorCode::RecalcTimeout.is_retryable());
        assert!(ErrorCode::ResourceExhausted.is_retryable());
        assert!(!ErrorCode::ValidationError.is_retryable());
        assert!(!ErrorCode::InvalidParams.is_retryable());
    }

    #[test]
    fn test_error_display() {
        let error = McpError::validation()
            .message("Invalid parameter")
            .suggestion("Check the documentation")
            .suggestion("Verify parameter types")
            .build();

        let display = format!("{}", error);
        assert!(display.contains("ValidationError"));
        assert!(display.contains("Invalid parameter"));
        assert!(display.contains("Suggestions"));
        assert!(display.contains("Check the documentation"));
    }

    #[test]
    fn test_to_mcp_error_conversion() {
        let anyhow_err = anyhow::anyhow!("Workbook 'test.xlsx' not found");
        let mcp_err = to_mcp_error(anyhow_err);

        assert_eq!(mcp_err.code, ErrorCode::WorkbookNotFound);
        assert!(mcp_err.context.suggestions.len() > 0);
    }
}
