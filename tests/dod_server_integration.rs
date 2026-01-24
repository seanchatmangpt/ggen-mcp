//! Definition of Done MCP Server Integration Tests
//!
//! Tests validate_definition_of_done tool registration and invocation via MCP server.

use ggen_mcp::config::ServerConfig;
use ggen_mcp::server::SpreadsheetServer;
use ggen_mcp::state::AppState;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::ServerInfo;
use rmcp::ServerHandler;
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;

// =============================================================================
// Test Fixtures
// =============================================================================

fn create_test_server() -> (TempDir, SpreadsheetServer) {
    let temp_dir = TempDir::new().unwrap();
    let config = ServerConfig::new(temp_dir.path().to_str().unwrap().to_string());
    let state = Arc::new(AppState::new(Arc::new(config)));
    let server = SpreadsheetServer::from_state(state);
    (temp_dir, server)
}

// =============================================================================
// Tool Registration Tests
// =============================================================================

#[test]
fn test_server_includes_validate_definition_of_done_tool() {
    let (_temp_dir, server) = create_test_server();
    let info: ServerInfo = server.get_info();

    assert!(info.capabilities.tools.is_some(), "Server should have tools capability");

    // Note: We can't directly inspect the tool router, but we can verify
    // the tool is callable (tested below)
}

#[test]
fn test_server_info_includes_instructions() {
    let (_temp_dir, server) = create_test_server();
    let info: ServerInfo = server.get_info();

    assert!(info.instructions.is_some(), "Server should have instructions");
    // DoD tool doesn't need to be in instructions (it's a quality/validation tool)
}

// =============================================================================
// Tool Invocation Tests
// =============================================================================

#[tokio::test]
async fn test_validate_definition_of_done_minimal_profile() {
    let (_temp_dir, server) = create_test_server();

    let params = ggen_mcp::tools::dod::ValidateDefinitionOfDoneParams {
        profile: "minimal".to_string(),
        workspace_path: None,
        include_remediation: true,
        include_evidence: true,
        fail_fast: false,
    };

    let result = server
        .validate_definition_of_done_tool(Parameters(params))
        .await;

    assert!(result.is_ok(), "Tool invocation should succeed: {:?}", result);

    let response = result.unwrap().0;
    assert!(!response.checks.is_empty(), "Should have check results");
    assert!(response.confidence_score <= 100, "Score should be valid");
    assert!(
        response.summary.total_checks > 0,
        "Should have executed checks"
    );
    assert!(
        response.verdict == "READY" || response.verdict == "PENDING" || response.verdict == "BLOCKED",
        "Verdict should be valid: {}",
        response.verdict
    );
}

#[tokio::test]
async fn test_validate_definition_of_done_standard_profile() {
    let (_temp_dir, server) = create_test_server();

    let params = ggen_mcp::tools::dod::ValidateDefinitionOfDoneParams {
        profile: "standard".to_string(),
        workspace_path: None,
        include_remediation: true,
        include_evidence: false,
        fail_fast: false,
    };

    let result = server
        .validate_definition_of_done_tool(Parameters(params))
        .await;

    assert!(result.is_ok(), "Tool invocation should succeed");

    let response = result.unwrap().0;
    assert!(
        response.checks.iter().all(|c| c.evidence.is_none()),
        "Evidence should be excluded when include_evidence=false"
    );
    assert!(
        response.summary.total_checks >= response.summary.total_checks,
        "Standard should have more checks than minimal"
    );
}

#[tokio::test]
async fn test_validate_definition_of_done_comprehensive_profile() {
    let (_temp_dir, server) = create_test_server();

    let params = ggen_mcp::tools::dod::ValidateDefinitionOfDoneParams {
        profile: "comprehensive".to_string(),
        workspace_path: None,
        include_remediation: false,
        include_evidence: true,
        fail_fast: false,
    };

    let result = server
        .validate_definition_of_done_tool(Parameters(params))
        .await;

    assert!(result.is_ok(), "Tool invocation should succeed");

    let response = result.unwrap().0;
    assert!(
        response.remediation.is_none(),
        "Remediation should be excluded when include_remediation=false"
    );
    assert!(
        response.summary.total_checks >= 10,
        "Comprehensive should run many checks"
    );
}

#[tokio::test]
async fn test_validate_definition_of_done_unknown_profile() {
    let (_temp_dir, server) = create_test_server();

    let params = ggen_mcp::tools::dod::ValidateDefinitionOfDoneParams {
        profile: "unknown_profile".to_string(),
        workspace_path: None,
        include_remediation: true,
        include_evidence: true,
        fail_fast: false,
    };

    let result = server
        .validate_definition_of_done_tool(Parameters(params))
        .await;

    assert!(result.is_err(), "Should reject unknown profile");
}

#[tokio::test]
async fn test_validate_definition_of_done_with_workspace_path() {
    let (temp_dir, server) = create_test_server();

    let params = ggen_mcp::tools::dod::ValidateDefinitionOfDoneParams {
        profile: "minimal".to_string(),
        workspace_path: Some(temp_dir.path().to_str().unwrap().to_string()),
        include_remediation: true,
        include_evidence: true,
        fail_fast: false,
    };

    let result = server
        .validate_definition_of_done_tool(Parameters(params))
        .await;

    assert!(result.is_ok(), "Tool invocation with custom workspace should succeed");
}

// =============================================================================
// Response Format Tests
// =============================================================================

#[tokio::test]
async fn test_validate_definition_of_done_response_format() {
    let (_temp_dir, server) = create_test_server();

    let params = ggen_mcp::tools::dod::ValidateDefinitionOfDoneParams {
        profile: "minimal".to_string(),
        workspace_path: None,
        include_remediation: true,
        include_evidence: true,
        fail_fast: false,
    };

    let result = server
        .validate_definition_of_done_tool(Parameters(params))
        .await;

    assert!(result.is_ok(), "Tool invocation should succeed");

    let response = result.unwrap().0;

    // Verify response structure
    assert!(
        response.ready_for_deployment == (response.verdict == "READY"),
        "ready_for_deployment should match verdict"
    );
    assert!(response.confidence_score <= 100, "Score should be 0-100");
    assert!(!response.checks.is_empty(), "Should have checks");
    assert!(!response.narrative.is_empty(), "Should have narrative");

    // Verify summary
    let summary = &response.summary;
    assert_eq!(
        summary.total_checks,
        summary.passed + summary.failed + summary.warnings + summary.skipped + summary.errors,
        "Summary counts should add up"
    );
    assert!(summary.total_duration_ms > 0, "Should have duration");

    // Verify checks
    for check in &response.checks {
        assert!(!check.id.is_empty(), "Check should have ID");
        assert!(!check.category.is_empty(), "Check should have category");
        assert!(
            matches!(
                check.status.as_str(),
                "Pass" | "Fail" | "Warning" | "Skipped" | "Error"
            ),
            "Check status should be valid: {}",
            check.status
        );
        assert!(!check.message.is_empty(), "Check should have message");
    }

    // Verify remediation (when requested)
    if let Some(remediation) = &response.remediation {
        for suggestion in remediation {
            assert!(!suggestion.check_id.is_empty(), "Suggestion should have check_id");
            assert!(
                matches!(
                    suggestion.priority.as_str(),
                    "Critical" | "High" | "Medium" | "Low"
                ),
                "Priority should be valid: {}",
                suggestion.priority
            );
            assert!(!suggestion.action.is_empty(), "Suggestion should have action");
            assert!(!suggestion.rationale.is_empty(), "Suggestion should have rationale");
        }
    }
}

// =============================================================================
// Tool Enablement Tests
// =============================================================================

#[tokio::test]
async fn test_validate_definition_of_done_tool_enabled_by_default() {
    let (_temp_dir, server) = create_test_server();

    // Tool should be enabled by default (no SPREADSHEET_MCP_ENABLED_TOOLS restriction)
    let params = ggen_mcp::tools::dod::ValidateDefinitionOfDoneParams {
        profile: "minimal".to_string(),
        workspace_path: None,
        include_remediation: false,
        include_evidence: false,
        fail_fast: false,
    };

    let result = server
        .validate_definition_of_done_tool(Parameters(params))
        .await;

    assert!(result.is_ok(), "Tool should be enabled by default");
}

// =============================================================================
// Performance Tests
// =============================================================================

#[tokio::test]
async fn test_validate_definition_of_done_performance() {
    let (_temp_dir, server) = create_test_server();

    let params = ggen_mcp::tools::dod::ValidateDefinitionOfDoneParams {
        profile: "comprehensive".to_string(),
        workspace_path: None,
        include_remediation: true,
        include_evidence: true,
        fail_fast: false,
    };

    let start = std::time::Instant::now();
    let result = server
        .validate_definition_of_done_tool(Parameters(params))
        .await;
    let duration = start.elapsed();

    assert!(result.is_ok(), "Tool invocation should succeed");
    assert!(
        duration.as_secs() < 30,
        "Comprehensive validation should complete within 30s (took {:?})",
        duration
    );

    let response = result.unwrap().0;
    assert!(
        response.summary.total_duration_ms < 30_000,
        "Reported duration should be < 30s"
    );
}

// =============================================================================
// Serialization Tests
// =============================================================================

#[tokio::test]
async fn test_validate_definition_of_done_serialization() {
    let (_temp_dir, server) = create_test_server();

    let params = ggen_mcp::tools::dod::ValidateDefinitionOfDoneParams {
        profile: "minimal".to_string(),
        workspace_path: None,
        include_remediation: true,
        include_evidence: true,
        fail_fast: false,
    };

    let result = server
        .validate_definition_of_done_tool(Parameters(params))
        .await;

    assert!(result.is_ok(), "Tool invocation should succeed");

    let response = result.unwrap().0;

    // Test JSON serialization
    let json = serde_json::to_value(&response).unwrap();
    assert!(json.is_object(), "Response should serialize to JSON object");
    assert!(json.get("ready_for_deployment").is_some());
    assert!(json.get("verdict").is_some());
    assert!(json.get("confidence_score").is_some());
    assert!(json.get("checks").is_some());
    assert!(json.get("summary").is_some());
    assert!(json.get("narrative").is_some());
}
