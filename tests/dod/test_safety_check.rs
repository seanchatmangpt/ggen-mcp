//! Tests for Category G: Safety Invariants checks

use spreadsheet_mcp::dod::checks::safety::*;
use spreadsheet_mcp::dod::types::*;
use spreadsheet_mcp::dod::{CheckContext, DodCheck};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create test workspace
fn create_test_workspace() -> TempDir {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path();

    // Create directory structure
    fs::create_dir_all(workspace.join("src")).unwrap();
    fs::create_dir_all(workspace.join("templates")).unwrap();
    fs::create_dir_all(workspace.join("ontology")).unwrap();
    fs::create_dir_all(workspace.join("queries")).unwrap();

    temp
}

#[tokio::test]
async fn test_secret_detection_no_secrets() {
    let workspace = create_test_workspace();
    let src_file = workspace.path().join("src/main.rs");
    fs::write(&src_file, "fn main() { println!(\"Hello, world!\"); }").unwrap();

    let context = CheckContext::new(workspace.path().to_path_buf(), ValidationMode::Fast);
    let check = SecretDetectionCheck;
    let result = check.execute(&context).await.unwrap();

    assert_eq!(result.status, CheckStatus::Pass);
    assert_eq!(result.id, "G8_SECRETS");
    assert_eq!(result.category, CheckCategory::SafetyInvariants);
    assert!(result.evidence.is_empty());
}

#[tokio::test]
async fn test_secret_detection_aws_key() {
    let workspace = create_test_workspace();
    let src_file = workspace.path().join("src/config.rs");
    fs::write(
        &src_file,
        "const AWS_KEY: &str = \"AKIAIOSFODNN7EXAMPLE\";",
    )
    .unwrap();

    let context = CheckContext::new(workspace.path().to_path_buf(), ValidationMode::Fast);
    let check = SecretDetectionCheck;
    let result = check.execute(&context).await.unwrap();

    assert_eq!(result.status, CheckStatus::Fail);
    assert_eq!(result.severity, CheckSeverity::Fatal);
    assert!(!result.evidence.is_empty());
    assert!(result.message.contains("secret(s) detected"));

    // Verify evidence contains file path and line number
    let evidence = &result.evidence[0];
    assert!(matches!(evidence.kind, EvidenceKind::FileContent));
    assert!(evidence.file_path.is_some());
    assert!(evidence.line_number.is_some());
}

#[tokio::test]
async fn test_secret_detection_github_token() {
    let workspace = create_test_workspace();
    let src_file = workspace.path().join("src/auth.rs");
    fs::write(
        &src_file,
        "let token = \"ghp_1234567890abcdefghijklmnopqrstuvw\";",
    )
    .unwrap();

    let context = CheckContext::new(workspace.path().to_path_buf(), ValidationMode::Fast);
    let check = SecretDetectionCheck;
    let result = check.execute(&context).await.unwrap();

    assert_eq!(result.status, CheckStatus::Fail);
    assert!(!result.remediation.is_empty());
}

#[tokio::test]
async fn test_secret_detection_private_key() {
    let workspace = create_test_workspace();
    let src_file = workspace.path().join("src/keys.rs");
    fs::write(
        &src_file,
        "const KEY: &str = \"-----BEGIN RSA PRIVATE KEY-----\nMIIBogIBAAJBAL...";",
    )
    .unwrap();

    let context = CheckContext::new(workspace.path().to_path_buf(), ValidationMode::Fast);
    let check = SecretDetectionCheck;
    let result = check.execute(&context).await.unwrap();

    assert_eq!(result.status, CheckStatus::Fail);
    assert!(result.message.contains("secret(s) detected"));
}

#[tokio::test]
async fn test_secret_detection_high_entropy() {
    let workspace = create_test_workspace();
    let src_file = workspace.path().join("src/config.rs");
    // High entropy string that looks like a secret
    fs::write(
        &src_file,
        "const SECRET: &str = \"Kd8sH3pL9xQ2mN5vR7tY1wZ4aB6cE9fG\";",
    )
    .unwrap();

    let context = CheckContext::new(workspace.path().to_path_buf(), ValidationMode::Fast);
    let check = SecretDetectionCheck;
    let result = check.execute(&context).await.unwrap();

    assert_eq!(result.status, CheckStatus::Fail);
    assert!(result.message.contains("secret(s) detected"));
}

#[tokio::test]
async fn test_license_header_check_no_generated() {
    let workspace = create_test_workspace();

    let context = CheckContext::new(workspace.path().to_path_buf(), ValidationMode::Fast);
    let check = LicenseHeaderCheck;
    let result = check.execute(&context).await.unwrap();

    assert_eq!(result.status, CheckStatus::Skip);
    assert_eq!(result.id, "G9_LICENSES");
    assert!(result.message.contains("No generated files"));
}

#[tokio::test]
async fn test_license_header_check_with_spdx() {
    let workspace = create_test_workspace();
    fs::create_dir_all(workspace.path().join("src/generated")).unwrap();

    let gen_file = workspace.path().join("src/generated/types.rs");
    fs::write(
        &gen_file,
        "// SPDX-License-Identifier: Apache-2.0\n\npub struct Type;",
    )
    .unwrap();

    let context = CheckContext::new(workspace.path().to_path_buf(), ValidationMode::Fast);
    let check = LicenseHeaderCheck;
    let result = check.execute(&context).await.unwrap();

    assert_eq!(result.status, CheckStatus::Pass);
    assert_eq!(result.severity, CheckSeverity::Warning);
}

#[tokio::test]
async fn test_license_header_check_missing_spdx() {
    let workspace = create_test_workspace();
    fs::create_dir_all(workspace.path().join("src/generated")).unwrap();

    let gen_file = workspace.path().join("src/generated/types.rs");
    fs::write(&gen_file, "pub struct Type;").unwrap();

    let context = CheckContext::new(workspace.path().to_path_buf(), ValidationMode::Fast);
    let check = LicenseHeaderCheck;
    let result = check.execute(&context).await.unwrap();

    assert_eq!(result.status, CheckStatus::Warn);
    assert!(!result.remediation.is_empty());
}

#[tokio::test]
async fn test_dependency_risk_check_no_cargo_audit() {
    let workspace = create_test_workspace();

    let context = CheckContext::new(workspace.path().to_path_buf(), ValidationMode::Fast);
    let check = DependencyRiskCheck;
    let result = check.execute(&context).await.unwrap();

    // Should skip if cargo-audit is not installed
    // This test might pass or skip depending on environment
    assert!(matches!(
        result.status,
        CheckStatus::Skip | CheckStatus::Pass | CheckStatus::Warn
    ));
    assert_eq!(result.id, "G10_DEPENDENCIES");
}

#[test]
fn test_entropy_low() {
    use spreadsheet_mcp::dod::checks::safety::calculate_entropy;
    // Repeating characters have low entropy
    let low = "aaaaaaaaaa";
    assert!(calculate_entropy(low) < 1.0);
}

#[test]
fn test_entropy_high() {
    use spreadsheet_mcp::dod::checks::safety::calculate_entropy;
    // Random-looking strings have high entropy
    let high = "Kd8sH3pL9xQ2mN5v";
    assert!(calculate_entropy(high) > 3.5);
}

#[test]
fn test_entropy_medium() {
    use spreadsheet_mcp::dod::checks::safety::calculate_entropy;
    let medium = "password123";
    let e = calculate_entropy(medium);
    assert!(e > 2.0 && e < 4.0);
}

#[tokio::test]
async fn test_multiple_secrets_in_file() {
    let workspace = create_test_workspace();
    let src_file = workspace.path().join("src/config.rs");
    fs::write(
        &src_file,
        r#"
const AWS_KEY: &str = "AKIAIOSFODNN7EXAMPLE";
const GITHUB_TOKEN: &str = "ghp_1234567890abcdefghijklmnopqrstuvw";
    "#,
    )
    .unwrap();

    let context = CheckContext::new(workspace.path().to_path_buf(), ValidationMode::Fast);
    let check = SecretDetectionCheck;
    let result = check.execute(&context).await.unwrap();

    assert_eq!(result.status, CheckStatus::Fail);
    assert!(result.evidence.len() >= 2);
}

#[tokio::test]
async fn test_secret_check_properties() {
    let check = SecretDetectionCheck;
    assert_eq!(check.id(), "G8_SECRETS");
    assert_eq!(check.category(), CheckCategory::SafetyInvariants);
    assert_eq!(check.severity(), CheckSeverity::Fatal);
    assert_eq!(
        check.description(),
        "Detects hardcoded secrets using pattern matching and entropy analysis"
    );
}

#[tokio::test]
async fn test_license_check_properties() {
    let check = LicenseHeaderCheck;
    assert_eq!(check.id(), "G9_LICENSES");
    assert_eq!(check.category(), CheckCategory::SafetyInvariants);
    assert_eq!(check.severity(), CheckSeverity::Warning);
}

#[tokio::test]
async fn test_dependency_check_properties() {
    let check = DependencyRiskCheck;
    assert_eq!(check.id(), "G10_DEPENDENCIES");
    assert_eq!(check.category(), CheckCategory::SafetyInvariants);
    assert_eq!(check.severity(), CheckSeverity::Warning);
}
