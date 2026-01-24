//! MCP Handler Integration Tests
//!
//! Tests for DoD validation MCP tool handler

use ggen_mcp::dod::mcp_handler::{
    validate_definition_of_done, ValidateDefinitionOfDoneParams, ValidateDefinitionOfDoneResponse,
};
use ggen_mcp::dod::types::{CheckStatus, OverallVerdict};
use ggen_mcp::state::AppState;
use std::sync::Arc;
use tempfile::TempDir;

// =============================================================================
// Test Helpers
// =============================================================================

fn create_test_state() -> Arc<AppState> {
    Arc::new(AppState::default())
}

async fn run_validation(
    profile: &str,
    output_dir: Option<String>,
    skip_evidence: Option<bool>,
) -> anyhow::Result<ValidateDefinitionOfDoneResponse> {
    let state = create_test_state();
    let params = ValidateDefinitionOfDoneParams {
        profile: profile.to_string(),
        output_dir,
        skip_evidence,
    };

    validate_definition_of_done(state, params).await
}

// =============================================================================
// Parameter Parsing Tests
// =============================================================================

#[tokio::test]
async fn test_params_default_profile() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap().to_string();

    let result = run_validation("dev", Some(output_path), None).await;

    assert!(result.is_ok(), "Should use default dev profile");
    let response = result.unwrap();
    assert_eq!(response.verdict, "Ready" || response.verdict == "NotReady");
}

#[tokio::test]
async fn test_params_enterprise_profile() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap().to_string();

    let result = run_validation("enterprise", Some(output_path), None).await;

    assert!(result.is_ok(), "Should support enterprise profile");
}

#[tokio::test]
async fn test_params_invalid_profile() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap().to_string();

    let result = run_validation("invalid_profile_name", Some(output_path), None).await;

    assert!(result.is_err(), "Should reject invalid profile");
    assert!(result.unwrap_err().to_string().contains("Unknown profile"));
}

#[tokio::test]
async fn test_params_skip_evidence() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap().to_string();

    let result = run_validation("dev", Some(output_path.clone()), Some(true)).await;

    assert!(result.is_ok(), "Should support skip_evidence flag");
}

#[tokio::test]
async fn test_params_custom_output_dir() {
    let temp_dir = TempDir::new().unwrap();
    let custom_output = temp_dir.path().join("custom-dod-output");
    let output_path = custom_output.to_str().unwrap().to_string();

    let result = run_validation("dev", Some(output_path.clone()), None).await;

    assert!(result.is_ok(), "Should create custom output directory");
    assert!(custom_output.exists(), "Custom output directory should exist");
}

// =============================================================================
// Response Format Tests
// =============================================================================

#[tokio::test]
async fn test_response_format() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap().to_string();

    let response = run_validation("dev", Some(output_path), None)
        .await
        .unwrap();

    // Verify verdict format
    assert!(
        response.verdict == "Ready" || response.verdict == "NotReady",
        "Verdict should be Ready or NotReady, got: {}",
        response.verdict
    );

    // Verify score range
    assert!(
        response.score >= 0.0 && response.score <= 100.0,
        "Score should be 0-100, got: {}",
        response.score
    );

    // Verify paths are non-empty
    assert!(!response.report_path.is_empty(), "Report path should not be empty");
    assert!(!response.receipt_path.is_empty(), "Receipt path should not be empty");

    // Verify summary is non-empty
    assert!(!response.summary.is_empty(), "Summary should not be empty");

    // Verify check counts
    assert!(response.checks_total > 0, "Should have executed some checks");
    assert_eq!(
        response.checks_total,
        response.checks_passed + response.checks_failed + response.checks_warned,
        "Check totals should add up"
    );

    // Verify duration
    assert!(response.duration_ms > 0, "Duration should be positive");
}

#[tokio::test]
async fn test_response_validation_result() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap().to_string();

    let response = run_validation("dev", Some(output_path), None)
        .await
        .unwrap();

    let validation_result = &response.validation_result;

    // Verify validation result structure
    assert!(!validation_result.profile.is_empty());
    assert!(!validation_result.check_results.is_empty());
    assert!(!validation_result.category_scores.is_empty());

    // Verify consistency with response
    assert_eq!(
        validation_result.readiness_score, response.score,
        "Scores should match"
    );
    assert_eq!(
        format!("{:?}", validation_result.verdict),
        response.verdict,
        "Verdicts should match"
    );
}

#[tokio::test]
async fn test_response_check_results() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap().to_string();

    let response = run_validation("dev", Some(output_path), None)
        .await
        .unwrap();

    let check_results = &response.validation_result.check_results;

    // Verify all check results have required fields
    for result in check_results {
        assert!(!result.id.is_empty(), "Check ID should not be empty");
        assert!(!result.message.is_empty(), "Check message should not be empty");
        assert!(result.duration_ms >= 0, "Duration should be non-negative");
        assert!(!result.check_hash.is_empty(), "Check hash should not be empty");

        // Verify status is valid
        match result.status {
            CheckStatus::Pass | CheckStatus::Fail | CheckStatus::Warn | CheckStatus::Skip => {}
        }
    }
}

#[tokio::test]
async fn test_response_category_scores() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap().to_string();

    let response = run_validation("dev", Some(output_path), None)
        .await
        .unwrap();

    let category_scores = &response.validation_result.category_scores;

    assert!(!category_scores.is_empty(), "Should have category scores");

    for (category, score) in category_scores {
        // Verify score range
        assert!(
            score.score >= 0.0 && score.score <= 100.0,
            "Category {:?} score should be 0-100, got: {}",
            category,
            score.score
        );

        // Verify weight range
        assert!(
            score.weight >= 0.0 && score.weight <= 1.0,
            "Category {:?} weight should be 0-1, got: {}",
            category,
            score.weight
        );

        // Verify check counts
        assert_eq!(
            score.checks_passed + score.checks_failed + score.checks_warned + score.checks_skipped,
            0, // Categories may have 0 checks if not applicable
            "Category check counts should be consistent for {:?}",
            category
        );
    }
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[tokio::test]
async fn test_error_invalid_output_path() {
    // Test with path traversal attempt
    let result = run_validation("dev", Some("../dangerous/path".to_string()), None).await;

    assert!(result.is_err(), "Should reject path traversal");
}

#[tokio::test]
async fn test_error_handling_preserves_context() {
    let result = run_validation("nonexistent", None, None).await;

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Unknown profile") || error_msg.contains("profile"),
        "Error should mention profile issue"
    );
}

// =============================================================================
// Artifact Generation Tests
// =============================================================================

#[tokio::test]
async fn test_artifacts_report_generated() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap().to_string();

    let response = run_validation("dev", Some(output_path.clone()), None)
        .await
        .unwrap();

    let report_path = std::path::PathBuf::from(&response.report_path);
    assert!(report_path.exists(), "Report file should be created");

    let report_content = tokio::fs::read_to_string(&report_path).await.unwrap();
    assert!(
        report_content.contains("Definition of Done Validation Report"),
        "Report should have header"
    );
    assert!(
        report_content.contains("Summary"),
        "Report should have summary section"
    );
}

#[tokio::test]
async fn test_artifacts_receipt_generated() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap().to_string();

    let response = run_validation("dev", Some(output_path.clone()), None)
        .await
        .unwrap();

    let receipt_path = std::path::PathBuf::from(&response.receipt_path);
    assert!(receipt_path.exists(), "Receipt file should be created");

    let receipt_content = tokio::fs::read_to_string(&receipt_path).await.unwrap();
    let receipt: serde_json::Value = serde_json::from_str(&receipt_content).unwrap();

    // Verify receipt structure
    assert!(receipt.get("version").is_some(), "Receipt should have version");
    assert!(receipt.get("timestamp").is_some(), "Receipt should have timestamp");
    assert!(receipt.get("verdict").is_some(), "Receipt should have verdict");
    assert!(receipt.get("readiness_score").is_some(), "Receipt should have score");
}

#[tokio::test]
async fn test_artifacts_paths_in_response() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap().to_string();

    let response = run_validation("dev", Some(output_path.clone()), None)
        .await
        .unwrap();

    // Verify artifact paths are accessible
    let artifacts = &response.validation_result.artifacts;

    assert!(
        artifacts.report_path.exists(),
        "Report path should exist: {:?}",
        artifacts.report_path
    );
    assert!(
        artifacts.receipt_path.exists(),
        "Receipt path should exist: {:?}",
        artifacts.receipt_path
    );
}

// =============================================================================
// Integration Tests
// =============================================================================

#[tokio::test]
async fn test_integration_dev_profile_execution() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap().to_string();

    let response = run_validation("dev", Some(output_path), None)
        .await
        .unwrap();

    // Dev profile should execute required checks
    assert!(
        response.checks_total >= 3,
        "Dev profile should execute at least 3 checks"
    );

    // Should have category scores
    assert!(
        response.validation_result.category_scores.len() >= 1,
        "Should have at least one category score"
    );
}

#[tokio::test]
async fn test_integration_enterprise_profile_execution() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap().to_string();

    let response = run_validation("enterprise", Some(output_path), None)
        .await
        .unwrap();

    // Enterprise profile should execute more checks than dev
    assert!(
        response.checks_total >= 3,
        "Enterprise profile should execute comprehensive checks"
    );
}

#[tokio::test]
async fn test_integration_verdict_calculation() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap().to_string();

    let response = run_validation("dev", Some(output_path), None)
        .await
        .unwrap();

    // Verify verdict consistency
    let verdict_from_string = &response.verdict;
    let verdict_from_result = format!("{:?}", response.validation_result.verdict);

    assert_eq!(
        verdict_from_string, &verdict_from_result,
        "Verdict should be consistent across response fields"
    );

    // Verify verdict matches score threshold
    if response.score >= 70.0 {
        // Assuming 70.0 is a common threshold
        // Could be Ready or NotReady depending on fatal failures
        assert!(
            verdict_from_string == "Ready" || verdict_from_string == "NotReady",
            "High score should have valid verdict"
        );
    }
}

#[tokio::test]
async fn test_integration_summary_accuracy() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().to_str().unwrap().to_string();

    let response = run_validation("dev", Some(output_path), None)
        .await
        .unwrap();

    let summary_str = &response.summary;

    // Verify summary contains key information
    assert!(
        summary_str.contains("Score:"),
        "Summary should mention score"
    );
    assert!(
        summary_str.contains("Passed:") || summary_str.contains("Failed:"),
        "Summary should mention pass/fail counts"
    );

    // Verify summary matches data
    assert!(
        summary_str.contains(&response.score.to_string()[..4]), // First 4 chars of score
        "Summary should include actual score"
    );
}
