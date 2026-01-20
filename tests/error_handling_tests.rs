//! Comprehensive tests for error handling system

use anyhow::anyhow;
use spreadsheet_mcp::error::{
    ERROR_METRICS, ErrorCode, ErrorMetrics, McpError, to_mcp_error, to_rmcp_error,
};

#[test]
fn test_error_code_values() {
    // Standard JSON-RPC error codes
    assert_eq!(ErrorCode::ParseError.code(), -32700);
    assert_eq!(ErrorCode::InvalidRequest.code(), -32600);
    assert_eq!(ErrorCode::MethodNotFound.code(), -32601);
    assert_eq!(ErrorCode::InvalidParams.code(), -32602);
    assert_eq!(ErrorCode::InternalError.code(), -32603);

    // Custom application error codes
    assert_eq!(ErrorCode::WorkbookNotFound.code(), -32001);
    assert_eq!(ErrorCode::ForkNotFound.code(), -32002);
    assert_eq!(ErrorCode::RecalcTimeout.code(), -32003);
    assert_eq!(ErrorCode::ValidationError.code(), -32004);
    assert_eq!(ErrorCode::ResourceExhausted.code(), -32005);
    assert_eq!(ErrorCode::SheetNotFound.code(), -32006);
    assert_eq!(ErrorCode::InvalidRange.code(), -32007);
}

#[test]
fn test_error_categories() {
    assert_eq!(ErrorCode::InvalidParams.category(), "client_error");
    assert_eq!(ErrorCode::InvalidRequest.category(), "client_error");
    assert_eq!(ErrorCode::InternalError.category(), "server_error");
    assert_eq!(ErrorCode::WorkbookNotFound.category(), "resource_not_found");
    assert_eq!(ErrorCode::ForkNotFound.category(), "resource_not_found");
    assert_eq!(ErrorCode::RecalcTimeout.category(), "timeout");
    assert_eq!(ErrorCode::ValidationError.category(), "validation_error");
    assert_eq!(ErrorCode::ResourceExhausted.category(), "resource_limit");
    assert_eq!(ErrorCode::VbaError.category(), "subsystem_error");
    assert_eq!(ErrorCode::IoError.category(), "io_error");
}

#[test]
fn test_retryable_errors() {
    // Retryable errors
    assert!(ErrorCode::InternalError.is_retryable());
    assert!(ErrorCode::RecalcTimeout.is_retryable());
    assert!(ErrorCode::ResourceExhausted.is_retryable());
    assert!(ErrorCode::IoError.is_retryable());

    // Non-retryable errors
    assert!(!ErrorCode::ValidationError.is_retryable());
    assert!(!ErrorCode::InvalidParams.is_retryable());
    assert!(!ErrorCode::WorkbookNotFound.is_retryable());
    assert!(!ErrorCode::InvalidRange.is_retryable());
}

#[test]
fn test_error_builder_basic() {
    let error = McpError::validation().message("Invalid row number").build();

    assert_eq!(error.code, ErrorCode::ValidationError);
    assert_eq!(error.message, "Invalid row number");
    assert!(error.error_id.starts_with("err_"));
}

#[test]
fn test_error_builder_with_context() {
    let error = McpError::validation()
        .message("Invalid row number")
        .operation("read_table")
        .workbook_id("test.xlsx")
        .sheet_name("Sheet1")
        .range("A1:Z1000000")
        .param("row", 2000000)
        .suggestion("Row must be between 1 and 1,048,576")
        .suggestion("Use sheet_overview to check sheet dimensions")
        .build();

    assert_eq!(error.code, ErrorCode::ValidationError);
    assert_eq!(error.message, "Invalid row number");
    assert_eq!(error.context.operation, Some("read_table".to_string()));
    assert_eq!(error.context.workbook_id, Some("test.xlsx".to_string()));
    assert_eq!(error.context.sheet_name, Some("Sheet1".to_string()));
    assert_eq!(error.context.range, Some("A1:Z1000000".to_string()));
    assert_eq!(error.context.suggestions.len(), 2);
    assert!(error.context.params.contains_key("row"));
}

#[test]
fn test_error_builder_recovery_hints() {
    let error = McpError::internal()
        .message("Temporary failure")
        .retryable(true)
        .retry_after(5)
        .expected_fix("Wait for system to recover")
        .alternative("Try a smaller request")
        .alternative("Use a different workbook")
        .build();

    assert_eq!(error.code, ErrorCode::InternalError);
    assert!(error.recovery.is_retryable);
    assert_eq!(error.recovery.retry_after, Some(5));
    assert_eq!(
        error.recovery.expected_fix,
        Some("Wait for system to recover".to_string())
    );
    assert_eq!(error.recovery.alternatives.len(), 2);
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
    assert!(display.contains("Verify parameter types"));
}

#[test]
fn test_error_metrics_recording() {
    let metrics = ErrorMetrics::new();

    // Record some errors
    metrics.record_error(&ErrorCode::ValidationError, Some("read_table"));
    metrics.record_error(&ErrorCode::ValidationError, Some("read_table"));
    metrics.record_error(&ErrorCode::ValidationError, Some("sheet_page"));
    metrics.record_error(&ErrorCode::WorkbookNotFound, Some("describe_workbook"));
    metrics.record_error(&ErrorCode::InternalError, None);

    // Check error counts
    assert_eq!(metrics.get_error_count(&ErrorCode::ValidationError), 3);
    assert_eq!(metrics.get_error_count(&ErrorCode::WorkbookNotFound), 1);
    assert_eq!(metrics.get_error_count(&ErrorCode::InternalError), 1);

    // Check tool error counts
    assert_eq!(metrics.get_tool_error_count("read_table"), 2);
    assert_eq!(metrics.get_tool_error_count("sheet_page"), 1);
    assert_eq!(metrics.get_tool_error_count("describe_workbook"), 1);

    // Check category counts
    assert_eq!(metrics.get_category_count("validation_error"), 3);
    assert_eq!(metrics.get_category_count("resource_not_found"), 1);
    assert_eq!(metrics.get_category_count("server_error"), 1);
}

#[test]
fn test_error_metrics_stats() {
    let metrics = ErrorMetrics::new();

    metrics.record_error(&ErrorCode::ValidationError, Some("read_table"));
    metrics.record_error(&ErrorCode::WorkbookNotFound, Some("describe_workbook"));

    let stats = metrics.get_stats();

    assert_eq!(
        stats.error_counts.get(&ErrorCode::ValidationError),
        Some(&1)
    );
    assert_eq!(
        stats.error_counts.get(&ErrorCode::WorkbookNotFound),
        Some(&1)
    );
    assert_eq!(stats.tool_errors.get("read_table"), Some(&1));
    assert_eq!(stats.tool_errors.get("describe_workbook"), Some(&1));
}

#[test]
fn test_error_metrics_reset() {
    let metrics = ErrorMetrics::new();

    metrics.record_error(&ErrorCode::ValidationError, Some("read_table"));
    assert_eq!(metrics.get_error_count(&ErrorCode::ValidationError), 1);

    metrics.reset();
    assert_eq!(metrics.get_error_count(&ErrorCode::ValidationError), 0);
}

#[test]
fn test_to_mcp_error_workbook_not_found() {
    let anyhow_err = anyhow!("Workbook 'test.xlsx' not found");
    let mcp_err = to_mcp_error(anyhow_err);

    assert_eq!(mcp_err.code, ErrorCode::WorkbookNotFound);
    assert!(mcp_err.message.contains("not found"));
    assert!(mcp_err.context.suggestions.len() > 0);
    assert!(
        mcp_err
            .context
            .suggestions
            .iter()
            .any(|s| s.contains("list_workbooks"))
    );
}

#[test]
fn test_to_mcp_error_fork_not_found() {
    let anyhow_err = anyhow!("Fork 'abc123' not found");
    let mcp_err = to_mcp_error(anyhow_err);

    assert_eq!(mcp_err.code, ErrorCode::ForkNotFound);
    assert!(mcp_err.context.suggestions.len() > 0);
    assert!(
        mcp_err
            .context
            .suggestions
            .iter()
            .any(|s| s.contains("list_forks"))
    );
}

#[test]
fn test_to_mcp_error_sheet_not_found() {
    let anyhow_err = anyhow!("Sheet 'Sheet1' not found");
    let mcp_err = to_mcp_error(anyhow_err);

    assert_eq!(mcp_err.code, ErrorCode::SheetNotFound);
    assert!(mcp_err.context.suggestions.len() > 0);
    assert!(
        mcp_err
            .context
            .suggestions
            .iter()
            .any(|s| s.contains("list_sheets"))
    );
}

#[test]
fn test_to_mcp_error_invalid_range() {
    let anyhow_err = anyhow!("Range 'ZZZZ999999' is invalid");
    let mcp_err = to_mcp_error(anyhow_err);

    assert_eq!(mcp_err.code, ErrorCode::InvalidRange);
    assert!(mcp_err.context.suggestions.len() > 0);
    assert!(
        mcp_err
            .context
            .suggestions
            .iter()
            .any(|s| s.contains("A1 notation"))
    );
}

#[test]
fn test_to_mcp_error_timeout() {
    let anyhow_err = anyhow!("Operation timed out after 30s");
    let mcp_err = to_mcp_error(anyhow_err);

    assert_eq!(mcp_err.code, ErrorCode::RecalcTimeout);
    assert!(mcp_err.recovery.is_retryable);
    assert_eq!(mcp_err.recovery.retry_after, Some(5));
}

#[test]
fn test_to_mcp_error_response_too_large() {
    let anyhow_err = anyhow!("Response too large: 10MB exceeds limit of 5MB");
    let mcp_err = to_mcp_error(anyhow_err);

    assert_eq!(mcp_err.code, ErrorCode::ResponseTooLarge);
    assert!(
        mcp_err
            .context
            .suggestions
            .iter()
            .any(|s| s.contains("pagination"))
    );
}

#[test]
fn test_to_mcp_error_validation() {
    let anyhow_err = anyhow!("Validation failed: invalid parameter type");
    let mcp_err = to_mcp_error(anyhow_err);

    assert_eq!(mcp_err.code, ErrorCode::ValidationError);
}

#[test]
fn test_to_mcp_error_permission_denied() {
    let anyhow_err = anyhow!("Permission denied: cannot write to file");
    let mcp_err = to_mcp_error(anyhow_err);

    assert_eq!(mcp_err.code, ErrorCode::PermissionDenied);
}

#[test]
fn test_to_mcp_error_tool_disabled() {
    let anyhow_err = anyhow!("Tool is disabled by configuration");
    let mcp_err = to_mcp_error(anyhow_err);

    assert_eq!(mcp_err.code, ErrorCode::ToolDisabled);
}

#[test]
fn test_to_mcp_error_parse_error() {
    let anyhow_err = anyhow!("Failed to parse JSON request");
    let mcp_err = to_mcp_error(anyhow_err);

    assert_eq!(mcp_err.code, ErrorCode::ParseError);
}

#[test]
fn test_to_mcp_error_generic() {
    let anyhow_err = anyhow!("Something went wrong");
    let mcp_err = to_mcp_error(anyhow_err);

    assert_eq!(mcp_err.code, ErrorCode::InternalError);
}

#[test]
fn test_error_context_preservation() {
    let anyhow_err = anyhow!("Base error")
        .context("First context layer")
        .context("Second context layer");

    let mcp_err = to_mcp_error(anyhow_err);

    // Related errors should contain context from error chain
    assert!(mcp_err.context.related_errors.len() > 0);
}

#[test]
fn test_error_unique_ids() {
    let error1 = McpError::validation().message("Test 1").build();
    let error2 = McpError::validation().message("Test 2").build();

    assert_ne!(error1.error_id, error2.error_id);
}

#[test]
fn test_error_timestamp() {
    let error = McpError::validation().message("Test").build();
    let now = chrono::Utc::now();

    // Error timestamp should be very recent (within 1 second)
    let diff = (now - error.timestamp).num_seconds().abs();
    assert!(diff < 1);
}

#[test]
fn test_multiple_suggestions() {
    let error = McpError::validation()
        .message("Multiple issues")
        .suggestions(vec![
            "First suggestion".to_string(),
            "Second suggestion".to_string(),
            "Third suggestion".to_string(),
        ])
        .build();

    assert_eq!(error.context.suggestions.len(), 3);
}

#[test]
fn test_multiple_related_errors() {
    let error = McpError::validation()
        .message("Cascading failure")
        .related_error("Database connection failed")
        .related_error("Retry limit exceeded")
        .build();

    assert_eq!(error.context.related_errors.len(), 2);
}

#[test]
fn test_doc_link() {
    let error = McpError::validation()
        .message("Complex validation error")
        .doc_link("https://docs.example.com/validation")
        .build();

    assert_eq!(
        error.context.doc_link,
        Some("https://docs.example.com/validation".to_string())
    );
}

#[test]
fn test_error_serialization() {
    let error = McpError::validation()
        .message("Test error")
        .operation("test_operation")
        .workbook_id("test.xlsx")
        .suggestion("Try again")
        .build();

    let json = serde_json::to_string(&error).unwrap();
    assert!(json.contains("ValidationError"));
    assert!(json.contains("Test error"));
    assert!(json.contains("test_operation"));
}

#[test]
fn test_global_error_metrics() {
    // Test that global ERROR_METRICS is accessible
    ERROR_METRICS.record_error(&ErrorCode::ValidationError, Some("test_tool"));
    let count = ERROR_METRICS.get_error_count(&ErrorCode::ValidationError);
    assert!(count > 0);
}

#[test]
fn test_builder_chaining() {
    // Test that builder methods can be chained fluently
    let error = McpError::builder(ErrorCode::ValidationError)
        .message("Test")
        .operation("test_op")
        .workbook_id("wb")
        .fork_id("fork")
        .sheet_name("sheet")
        .range("A1:B2")
        .param("key", "value")
        .suggestion("suggestion 1")
        .related_error("related 1")
        .doc_link("http://example.com")
        .retryable(true)
        .retry_after(10)
        .expected_fix("fix it")
        .alternative("alt 1")
        .build();

    assert_eq!(error.message, "Test");
    assert_eq!(error.context.operation, Some("test_op".to_string()));
    assert!(error.recovery.is_retryable);
}

#[test]
fn test_error_code_display() {
    let code = ErrorCode::ValidationError;
    let display = format!("{}", code);
    assert!(display.contains("ValidationError"));
    assert!(display.contains("-32004"));
}

#[test]
fn test_to_rmcp_error_conversion() {
    let custom_error = McpError::validation()
        .message("Test validation error")
        .build();

    let rmcp_error = to_rmcp_error(custom_error);

    // The rmcp error should be properly formatted
    // We can't directly inspect ErrorData fields without implementing additional traits,
    // but we can test that it doesn't panic
    let _ = format!("{:?}", rmcp_error);
}
