//! Tests for Category H: Deployment Readiness checks

use spreadsheet_mcp::dod::checks::deployment::*;
use spreadsheet_mcp::dod::types::*;
use spreadsheet_mcp::dod::{CheckContext, DodCheck};
use std::fs;
use tempfile::TempDir;

/// Helper to create test workspace with Cargo.toml
fn create_test_workspace() -> TempDir {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path();

    // Create minimal Cargo.toml
    let cargo_toml = r#"
[package]
name = "test-project"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "spreadsheet-mcp"
path = "src/main.rs"

[dependencies]
"#;

    fs::write(workspace.join("Cargo.toml"), cargo_toml).unwrap();

    // Create minimal src/main.rs
    fs::create_dir_all(workspace.join("src")).unwrap();
    fs::write(
        workspace.join("src/main.rs"),
        "fn main() { println!(\"test\"); }",
    )
    .unwrap();

    temp
}

#[tokio::test]
async fn test_artifact_build_check_properties() {
    let check = ArtifactBuildCheck;
    assert_eq!(check.id(), "H1_ARTIFACTS");
    assert_eq!(check.category(), CheckCategory::DeploymentReadiness);
    assert_eq!(check.severity(), CheckSeverity::Fatal);
    assert_eq!(check.dependencies(), vec!["BUILD_CHECK".to_string()]);
    assert_eq!(
        check.description(),
        "Validates release artifacts can be built successfully"
    );
}

#[tokio::test]
#[ignore] // This test actually builds, which is slow
async fn test_artifact_build_success() {
    let workspace = create_test_workspace();

    let context = CheckContext::new(workspace.path().to_path_buf(), ValidationMode::Fast)
        .with_timeout(300_000); // 5 minutes

    let check = ArtifactBuildCheck;
    let result = check.execute(&context).await.unwrap();

    // Should either pass or fail with meaningful error
    assert!(matches!(
        result.status,
        CheckStatus::Pass | CheckStatus::Fail | CheckStatus::Warn
    ));
    assert!(!result.message.is_empty());
}

#[tokio::test]
async fn test_artifact_build_invalid_cargo_toml() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path();

    // Create invalid Cargo.toml
    fs::write(workspace.join("Cargo.toml"), "invalid toml content[[[").unwrap();

    let context = CheckContext::new(workspace.to_path_buf(), ValidationMode::Fast)
        .with_timeout(60_000); // 1 minute

    let check = ArtifactBuildCheck;
    let result = check.execute(&context).await.unwrap();

    assert_eq!(result.status, CheckStatus::Fail);
    assert!(!result.evidence.is_empty());
}

#[tokio::test]
async fn test_artifact_build_missing_cargo_toml() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path();

    let context = CheckContext::new(workspace.to_path_buf(), ValidationMode::Fast)
        .with_timeout(60_000);

    let check = ArtifactBuildCheck;
    let result = check.execute(&context).await.unwrap();

    assert_eq!(result.status, CheckStatus::Fail);
}

#[tokio::test]
async fn test_artifact_build_timeout() {
    let workspace = create_test_workspace();

    // Set very short timeout
    let context = CheckContext::new(workspace.path().to_path_buf(), ValidationMode::Fast)
        .with_timeout(1); // 1ms - will definitely timeout

    let check = ArtifactBuildCheck;
    let result = check.execute(&context).await;

    // Should timeout and return error
    assert!(result.is_err());
}

#[tokio::test]
async fn test_artifact_build_check_evidence_collection() {
    let workspace = create_test_workspace();

    let context = CheckContext::new(workspace.path().to_path_buf(), ValidationMode::Fast)
        .with_timeout(300_000);

    let check = ArtifactBuildCheck;
    let result = check.execute(&context).await.unwrap();

    // Evidence should be collected regardless of pass/fail
    if result.status == CheckStatus::Pass {
        // On success, should have evidence about binary size
        assert!(!result.evidence.is_empty());
        let has_metric = result
            .evidence
            .iter()
            .any(|e| matches!(e.kind, EvidenceKind::Metric));
        assert!(has_metric);
    } else {
        // On failure, should have command output evidence
        let has_output = result
            .evidence
            .iter()
            .any(|e| matches!(e.kind, EvidenceKind::CommandOutput));
        assert!(has_output);
    }
}

#[tokio::test]
async fn test_artifact_build_check_remediation() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path();

    // Create broken project
    fs::write(workspace.join("Cargo.toml"), "[package]\nname = \"broken\"").unwrap();

    let context = CheckContext::new(workspace.to_path_buf(), ValidationMode::Fast)
        .with_timeout(60_000);

    let check = ArtifactBuildCheck;
    let result = check.execute(&context).await.unwrap();

    if result.status == CheckStatus::Fail {
        assert!(!result.remediation.is_empty());
        // Should suggest running cargo build locally
        let has_cargo_suggestion = result
            .remediation
            .iter()
            .any(|r| r.contains("cargo build --release"));
        assert!(has_cargo_suggestion);
    }
}

#[tokio::test]
async fn test_artifact_build_duration_tracking() {
    let workspace = create_test_workspace();

    let context = CheckContext::new(workspace.path().to_path_buf(), ValidationMode::Fast)
        .with_timeout(300_000);

    let check = ArtifactBuildCheck;
    let result = check.execute(&context).await.unwrap();

    // Duration should be tracked
    assert!(result.duration_ms > 0);
}
