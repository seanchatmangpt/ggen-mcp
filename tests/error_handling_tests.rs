//! Comprehensive tests for error handling system

use anyhow::anyhow;
use chicago_tdd_tools::prelude::*;
use spreadsheet_mcp::error::{
    ErrorCode, ErrorMetrics, McpError, to_mcp_error, to_rmcp_error, ERROR_METRICS,
};

test!(test_error_code_values, {
    // Arrange: Define expected error codes

    // Act & Assert: Verify standard JSON-RPC error codes
    assert_eq!(ErrorCode::ParseError.code(), -32700);
    assert_eq!(ErrorCode::InvalidRequest.code(), -32600);
    assert_eq!(ErrorCode::MethodNotFound.code(), -32601);
    assert_eq!(ErrorCode::InvalidParams.code(), -32602);
    assert_eq!(ErrorCode::InternalError.code(), -32603);

    // Act & Assert: Verify custom application error codes
    assert_eq!(ErrorCode::WorkbookNotFound.code(), -32001);
    assert_eq!(ErrorCode::ForkNotFound.code(), -32002);
    assert_eq!(ErrorCode::RecalcTimeout.code(), -32003);
    assert_eq!(ErrorCode::ValidationError.code(), -32004);
    assert_eq!(ErrorCode::ResourceExhausted.code(), -32005);
    assert_eq!(ErrorCode::SheetNotFound.code(), -32006);
    assert_eq!(ErrorCode::InvalidRange.code(), -32007);
});

test!(test_error_categories, {
    // Arrange: Define error codes to test

    // Act & Assert: Verify client error categories
    assert_eq!(ErrorCode::InvalidParams.category(), "client_error");
    assert_eq!(ErrorCode::InvalidRequest.category(), "client_error");

    // Act & Assert: Verify server error categories
    assert_eq!(ErrorCode::InternalError.category(), "server_error");

    // Act & Assert: Verify resource not found categories
    assert_eq!(
        ErrorCode::WorkbookNotFound.category(),
        "resource_not_found"
    );
    assert_eq!(ErrorCode::ForkNotFound.category(), "resource_not_found");

    // Act & Assert: Verify other categories
    assert_eq!(ErrorCode::RecalcTimeout.category(), "timeout");
    assert_eq!(ErrorCode::ValidationError.category(), "validation_error");
    assert_eq!(ErrorCode::ResourceExhausted.category(), "resource_limit");
    assert_eq!(ErrorCode::VbaError.category(), "subsystem_error");
    assert_eq!(ErrorCode::IoError.category(), "io_error");
});

test!(test_retryable_errors, {
    // Arrange: Define error codes to test

    // Act & Assert: Verify retryable errors
    assert!(ErrorCode::InternalError.is_retryable());
    assert!(ErrorCode::RecalcTimeout.is_retryable());
    assert!(ErrorCode::ResourceExhausted.is_retryable());
    assert!(ErrorCode::IoError.is_retryable());

    // Act & Assert: Verify non-retryable errors
    assert!(!ErrorCode::ValidationError.is_retryable());
    assert!(!ErrorCode::InvalidParams.is_retryable());
    assert!(!ErrorCode::WorkbookNotFound.is_retryable());
    assert!(!ErrorCode::InvalidRange.is_retryable());
});

test!(test_error_builder_basic, {
    // Arrange: Build a simple validation error
    let error = McpError::validation().message("Invalid row number").build();

    // Assert: Verify error properties
    assert_eq!(error.code, ErrorCode::ValidationError);
    assert_eq!(error.message, "Invalid row number");
    assert!(error.error_id.starts_with("err_"));
});

test!(test_error_builder_with_context, {
    // Arrange: Build an error with detailed context
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

    // Assert: Verify error code and message
    assert_eq!(error.code, ErrorCode::ValidationError);
    assert_eq!(error.message, "Invalid row number");

    // Assert: Verify context fields
    assert_eq!(error.context.operation, Some("read_table".to_string()));
    assert_eq!(error.context.workbook_id, Some("test.xlsx".to_string()));
    assert_eq!(error.context.sheet_name, Some("Sheet1".to_string()));
    assert_eq!(error.context.range, Some("A1:Z1000000".to_string()));
    assert_eq!(error.context.suggestions.len(), 2);
    assert!(error.context.params.contains_key("row"));
});

test!(test_error_builder_recovery_hints, {
    // Arrange: Build an error with recovery information
    let error = McpError::internal()
        .message("Temporary failure")
        .retryable(true)
        .retry_after(5)
        .expected_fix("Wait for system to recover")
        .alternative("Try a smaller request")
        .alternative("Use a different workbook")
        .build();

    // Assert: Verify error code
    assert_eq!(error.code, ErrorCode::InternalError);

    // Assert: Verify recovery information
    assert!(error.recovery.is_retryable);
    assert_eq!(error.recovery.retry_after, Some(5));
    assert_eq!(
        error.recovery.expected_fix,
        Some("Wait for system to recover".to_string())
    );
    assert_eq!(error.recovery.alternatives.len(), 2);
});

test!(test_error_display, {
    // Arrange: Build an error with suggestions
    let error = McpError::validation()
        .message("Invalid parameter")
        .suggestion("Check the documentation")
        .suggestion("Verify parameter types")
        .build();

    // Act: Format error for display
    let display = format!("{}", error);

    // Assert: Verify display includes key information
    assert!(display.contains("ValidationError"));
    assert!(display.contains("Invalid parameter"));
    assert!(display.contains("Suggestions"));
    assert!(display.contains("Check the documentation"));
    assert!(display.contains("Verify parameter types"));
});

test!(test_error_metrics_recording, {
    // Arrange: Create a new metrics instance
    let metrics = ErrorMetrics::new();

    // Act: Record various errors
    metrics.record_error(&ErrorCode::ValidationError, Some("read_table"));
    metrics.record_error(&ErrorCode::ValidationError, Some("read_table"));
    metrics.record_error(&ErrorCode::ValidationError, Some("sheet_page"));
    metrics.record_error(&ErrorCode::WorkbookNotFound, Some("describe_workbook"));
    metrics.record_error(&ErrorCode::InternalError, None);

    // Assert: Verify error counts by error code
    assert_eq!(metrics.get_error_count(&ErrorCode::ValidationError), 3);
    assert_eq!(metrics.get_error_count(&ErrorCode::WorkbookNotFound), 1);
    assert_eq!(metrics.get_error_count(&ErrorCode::InternalError), 1);

    // Assert: Verify tool-specific error counts
    assert_eq!(metrics.get_tool_error_count("read_table"), 2);
    assert_eq!(metrics.get_tool_error_count("sheet_page"), 1);
    assert_eq!(metrics.get_tool_error_count("describe_workbook"), 1);

    // Assert: Verify category counts
    assert_eq!(metrics.get_category_count("validation_error"), 3);
    assert_eq!(metrics.get_category_count("resource_not_found"), 1);
    assert_eq!(metrics.get_category_count("server_error"), 1);
});

test!(test_error_metrics_stats, {
    // Arrange: Create metrics and record some errors
    let metrics = ErrorMetrics::new();
    metrics.record_error(&ErrorCode::ValidationError, Some("read_table"));
    metrics.record_error(&ErrorCode::WorkbookNotFound, Some("describe_workbook"));

    // Act: Get statistics
    let stats = metrics.get_stats();

    // Assert: Verify error counts
    assert_eq!(
        stats.error_counts.get(&ErrorCode::ValidationError),
        Some(&1)
    );
    assert_eq!(
        stats.error_counts.get(&ErrorCode::WorkbookNotFound),
        Some(&1)
    );

    // Assert: Verify tool error counts
    assert_eq!(stats.tool_errors.get("read_table"), Some(&1));
    assert_eq!(stats.tool_errors.get("describe_workbook"), Some(&1));
});

test!(test_error_metrics_reset, {
    // Arrange: Create metrics and record an error
    let metrics = ErrorMetrics::new();
    metrics.record_error(&ErrorCode::ValidationError, Some("read_table"));

    // Act: Verify error was recorded, then reset
    assert_eq!(metrics.get_error_count(&ErrorCode::ValidationError), 1);
    metrics.reset();

    // Assert: Verify metrics were reset
    assert_eq!(metrics.get_error_count(&ErrorCode::ValidationError), 0);
});

test!(test_to_mcp_error_workbook_not_found, {
    // Arrange: Create an anyhow error for workbook not found
    let anyhow_err = anyhow!("Workbook 'test.xlsx' not found");

    // Act: Convert to MCP error
    let mcp_err = to_mcp_error(anyhow_err);

    // Assert: Verify correct error code and suggestions
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
});

test!(test_to_mcp_error_fork_not_found, {
    // Arrange: Create an anyhow error for fork not found
    let anyhow_err = anyhow!("Fork 'abc123' not found");

    // Act: Convert to MCP error
    let mcp_err = to_mcp_error(anyhow_err);

    // Assert: Verify correct error code and suggestions
    assert_eq!(mcp_err.code, ErrorCode::ForkNotFound);
    assert!(mcp_err.context.suggestions.len() > 0);
    assert!(
        mcp_err
            .context
            .suggestions
            .iter()
            .any(|s| s.contains("list_forks"))
    );
});

test!(test_to_mcp_error_sheet_not_found, {
    // Arrange: Create an anyhow error for sheet not found
    let anyhow_err = anyhow!("Sheet 'Sheet1' not found");

    // Act: Convert to MCP error
    let mcp_err = to_mcp_error(anyhow_err);

    // Assert: Verify correct error code and suggestions
    assert_eq!(mcp_err.code, ErrorCode::SheetNotFound);
    assert!(mcp_err.context.suggestions.len() > 0);
    assert!(
        mcp_err
            .context
            .suggestions
            .iter()
            .any(|s| s.contains("list_sheets"))
    );
});

test!(test_to_mcp_error_invalid_range, {
    // Arrange: Create an anyhow error for invalid range
    let anyhow_err = anyhow!("Range 'ZZZZ999999' is invalid");

    // Act: Convert to MCP error
    let mcp_err = to_mcp_error(anyhow_err);

    // Assert: Verify correct error code and suggestions
    assert_eq!(mcp_err.code, ErrorCode::InvalidRange);
    assert!(mcp_err.context.suggestions.len() > 0);
    assert!(
        mcp_err
            .context
            .suggestions
            .iter()
            .any(|s| s.contains("A1 notation"))
    );
});

test!(test_to_mcp_error_timeout, {
    // Arrange: Create an anyhow error for timeout
    let anyhow_err = anyhow!("Operation timed out after 30s");

    // Act: Convert to MCP error
    let mcp_err = to_mcp_error(anyhow_err);

    // Assert: Verify timeout error with retry information
    assert_eq!(mcp_err.code, ErrorCode::RecalcTimeout);
    assert!(mcp_err.recovery.is_retryable);
    assert_eq!(mcp_err.recovery.retry_after, Some(5));
});

test!(test_to_mcp_error_response_too_large, {
    // Arrange: Create an anyhow error for response too large
    let anyhow_err = anyhow!("Response too large: 10MB exceeds limit of 5MB");

    // Act: Convert to MCP error
    let mcp_err = to_mcp_error(anyhow_err);

    // Assert: Verify correct error code and pagination suggestions
    assert_eq!(mcp_err.code, ErrorCode::ResponseTooLarge);
    assert!(
        mcp_err
            .context
            .suggestions
            .iter()
            .any(|s| s.contains("pagination"))
    );
});

test!(test_to_mcp_error_validation, {
    // Arrange: Create an anyhow error for validation failure
    let anyhow_err = anyhow!("Validation failed: invalid parameter type");

    // Act: Convert to MCP error
    let mcp_err = to_mcp_error(anyhow_err);

    // Assert: Verify validation error code
    assert_eq!(mcp_err.code, ErrorCode::ValidationError);
});

test!(test_to_mcp_error_permission_denied, {
    // Arrange: Create an anyhow error for permission denied
    let anyhow_err = anyhow!("Permission denied: cannot write to file");

    // Act: Convert to MCP error
    let mcp_err = to_mcp_error(anyhow_err);

    // Assert: Verify permission denied error code
    assert_eq!(mcp_err.code, ErrorCode::PermissionDenied);
});

test!(test_to_mcp_error_tool_disabled, {
    // Arrange: Create an anyhow error for disabled tool
    let anyhow_err = anyhow!("Tool is disabled by configuration");

    // Act: Convert to MCP error
    let mcp_err = to_mcp_error(anyhow_err);

    // Assert: Verify tool disabled error code
    assert_eq!(mcp_err.code, ErrorCode::ToolDisabled);
});

test!(test_to_mcp_error_parse_error, {
    // Arrange: Create an anyhow error for parse failure
    let anyhow_err = anyhow!("Failed to parse JSON request");

    // Act: Convert to MCP error
    let mcp_err = to_mcp_error(anyhow_err);

    // Assert: Verify parse error code
    assert_eq!(mcp_err.code, ErrorCode::ParseError);
});

test!(test_to_mcp_error_generic, {
    // Arrange: Create a generic anyhow error
    let anyhow_err = anyhow!("Something went wrong");

    // Act: Convert to MCP error
    let mcp_err = to_mcp_error(anyhow_err);

    // Assert: Verify generic errors become internal errors
    assert_eq!(mcp_err.code, ErrorCode::InternalError);
});

test!(test_error_context_preservation, {
    // Arrange: Create an anyhow error with nested context
    let anyhow_err = anyhow!("Base error")
        .context("First context layer")
        .context("Second context layer");

    // Act: Convert to MCP error
    let mcp_err = to_mcp_error(anyhow_err);

    // Assert: Verify context chain is preserved in related errors
    assert!(mcp_err.context.related_errors.len() > 0);
});

test!(test_error_unique_ids, {
    // Arrange & Act: Create two errors
    let error1 = McpError::validation().message("Test 1").build();
    let error2 = McpError::validation().message("Test 2").build();

    // Assert: Verify each error has a unique ID
    assert_ne!(error1.error_id, error2.error_id);
});

test!(test_error_timestamp, {
    // Arrange & Act: Create an error and capture current time
    let error = McpError::validation().message("Test").build();
    let now = chrono::Utc::now();

    // Assert: Error timestamp should be very recent (within 1 second)
    let diff = (now - error.timestamp).num_seconds().abs();
    assert!(diff < 1);
});

test!(test_multiple_suggestions, {
    // Arrange: Create an error with multiple suggestions
    let error = McpError::validation()
        .message("Multiple issues")
        .suggestions(vec![
            "First suggestion".to_string(),
            "Second suggestion".to_string(),
            "Third suggestion".to_string(),
        ])
        .build();

    // Assert: Verify all suggestions are stored
    assert_eq!(error.context.suggestions.len(), 3);
});

test!(test_multiple_related_errors, {
    // Arrange: Create an error with multiple related errors
    let error = McpError::validation()
        .message("Cascading failure")
        .related_error("Database connection failed")
        .related_error("Retry limit exceeded")
        .build();

    // Assert: Verify all related errors are stored
    assert_eq!(error.context.related_errors.len(), 2);
});

test!(test_doc_link, {
    // Arrange: Create an error with documentation link
    let error = McpError::validation()
        .message("Complex validation error")
        .doc_link("https://docs.example.com/validation")
        .build();

    // Assert: Verify doc link is stored correctly
    assert_eq!(
        error.context.doc_link,
        Some("https://docs.example.com/validation".to_string())
    );
});

test!(test_error_serialization, {
    // Arrange: Build an error with various fields
    let error = McpError::validation()
        .message("Test error")
        .operation("test_operation")
        .workbook_id("test.xlsx")
        .suggestion("Try again")
        .build();

    // Act: Serialize to JSON
    let json_result = serde_json::to_string(&error);

    // Assert: Verify serialization succeeded and contains expected fields
    assert_ok!(&json_result, "Serialization should succeed");
    if let Ok(json) = json_result {
        assert!(json.contains("ValidationError"));
        assert!(json.contains("Test error"));
        assert!(json.contains("test_operation"));
    }
});

test!(test_global_error_metrics, {
    // Arrange: Use global ERROR_METRICS singleton

    // Act: Record an error using global metrics
    ERROR_METRICS.record_error(&ErrorCode::ValidationError, Some("test_tool"));

    // Assert: Verify error was recorded
    let count = ERROR_METRICS.get_error_count(&ErrorCode::ValidationError);
    assert!(count > 0);
});

test!(test_builder_chaining, {
    // Arrange & Act: Test fluent builder interface with all methods chained
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

    // Assert: Verify builder correctly set all fields
    assert_eq!(error.message, "Test");
    assert_eq!(error.context.operation, Some("test_op".to_string()));
    assert!(error.recovery.is_retryable);
});

test!(test_error_code_display, {
    // Arrange: Get an error code
    let code = ErrorCode::ValidationError;

    // Act: Format error code for display
    let display = format!("{}", code);

    // Assert: Verify display includes both name and code
    assert!(display.contains("ValidationError"));
    assert!(display.contains("-32004"));
});

test!(test_to_rmcp_error_conversion, {
    // Arrange: Create a custom MCP error
    let custom_error = McpError::validation()
        .message("Test validation error")
        .build();

    // Act: Convert to RMCP error
    let rmcp_error = to_rmcp_error(custom_error);

    // Assert: Verify RMCP error can be formatted without panicking
    // (ErrorData fields are not directly inspectable without additional traits)
    let debug_string = format!("{:?}", rmcp_error);
    assert!(!debug_string.is_empty());
});
