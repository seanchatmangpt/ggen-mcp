//! Integration tests for DoD Markdown Report Generator
//!
//! Tests report formatting, all verdict types, and remediation sections.

use ggen_mcp::dod::*;
use std::collections::HashMap;
use std::path::PathBuf;

// Test helpers

fn create_passing_result() -> DodValidationResult {
    let mut category_scores = HashMap::new();
    category_scores.insert(
        CheckCategory::BuildCorrectness,
        CategoryScore {
            category: CheckCategory::BuildCorrectness,
            score: 100.0,
            weight: 0.25,
            checks_passed: 3,
            checks_failed: 0,
            checks_warned: 0,
            checks_skipped: 0,
        },
    );
    category_scores.insert(
        CheckCategory::TestTruth,
        CategoryScore {
            category: CheckCategory::TestTruth,
            score: 100.0,
            weight: 0.25,
            checks_passed: 2,
            checks_failed: 0,
            checks_warned: 0,
            checks_skipped: 0,
        },
    );

    DodValidationResult {
        verdict: OverallVerdict::Ready,
        readiness_score: 100.0,
        profile: "dev".to_string(),
        mode: ValidationMode::Fast,
        summary: ValidationSummary {
            checks_total: 5,
            checks_passed: 5,
            checks_failed: 0,
            checks_warned: 0,
            checks_skipped: 0,
        },
        category_scores,
        check_results: vec![
            DodCheckResult {
                id: "BUILD_CHECK".to_string(),
                category: CheckCategory::BuildCorrectness,
                status: CheckStatus::Pass,
                severity: CheckSeverity::Fatal,
                message: "Compilation successful".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms: 1200,
                check_hash: "abc123".to_string(),
            },
            DodCheckResult {
                id: "BUILD_FMT".to_string(),
                category: CheckCategory::BuildCorrectness,
                status: CheckStatus::Pass,
                severity: CheckSeverity::Fatal,
                message: "Code formatting correct".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms: 300,
                check_hash: "def456".to_string(),
            },
            DodCheckResult {
                id: "BUILD_CLIPPY".to_string(),
                category: CheckCategory::BuildCorrectness,
                status: CheckStatus::Pass,
                severity: CheckSeverity::Fatal,
                message: "No clippy warnings".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms: 800,
                check_hash: "ghi789".to_string(),
            },
            DodCheckResult {
                id: "TEST_UNIT".to_string(),
                category: CheckCategory::TestTruth,
                status: CheckStatus::Pass,
                severity: CheckSeverity::Fatal,
                message: "All unit tests passed".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms: 2500,
                check_hash: "jkl012".to_string(),
            },
            DodCheckResult {
                id: "TEST_INTEGRATION".to_string(),
                category: CheckCategory::TestTruth,
                status: CheckStatus::Pass,
                severity: CheckSeverity::Fatal,
                message: "All integration tests passed".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms: 5000,
                check_hash: "mno345".to_string(),
            },
        ],
        artifacts: ArtifactPaths {
            receipt_path: PathBuf::from("./receipts/test.json"),
            report_path: PathBuf::from("./reports/test.md"),
            bundle_path: None,
        },
        duration_ms: 9800,
    }
}

fn create_failing_result() -> DodValidationResult {
    let mut category_scores = HashMap::new();
    category_scores.insert(
        CheckCategory::BuildCorrectness,
        CategoryScore {
            category: CheckCategory::BuildCorrectness,
            score: 50.0,
            weight: 0.25,
            checks_passed: 1,
            checks_failed: 1,
            checks_warned: 0,
            checks_skipped: 0,
        },
    );
    category_scores.insert(
        CheckCategory::SafetyInvariants,
        CategoryScore {
            category: CheckCategory::SafetyInvariants,
            score: 0.0,
            weight: 0.10,
            checks_passed: 0,
            checks_failed: 1,
            checks_warned: 0,
            checks_skipped: 0,
        },
    );

    DodValidationResult {
        verdict: OverallVerdict::NotReady,
        readiness_score: 35.0,
        profile: "strict".to_string(),
        mode: ValidationMode::Strict,
        summary: ValidationSummary {
            checks_total: 3,
            checks_passed: 1,
            checks_failed: 2,
            checks_warned: 0,
            checks_skipped: 0,
        },
        category_scores,
        check_results: vec![
            DodCheckResult {
                id: "BUILD_CHECK".to_string(),
                category: CheckCategory::BuildCorrectness,
                status: CheckStatus::Pass,
                severity: CheckSeverity::Fatal,
                message: "Build passed".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms: 1200,
                check_hash: "abc123".to_string(),
            },
            DodCheckResult {
                id: "BUILD_FMT".to_string(),
                category: CheckCategory::BuildCorrectness,
                status: CheckStatus::Fail,
                severity: CheckSeverity::Fatal,
                message: "Code not formatted correctly".to_string(),
                evidence: vec![],
                remediation: vec!["Run: cargo fmt".to_string()],
                duration_ms: 100,
                check_hash: "def456".to_string(),
            },
            DodCheckResult {
                id: "G8_SECRETS".to_string(),
                category: CheckCategory::SafetyInvariants,
                status: CheckStatus::Fail,
                severity: CheckSeverity::Fatal,
                message: "Hardcoded secrets detected".to_string(),
                evidence: vec![],
                remediation: vec![
                    "Remove hardcoded API keys".to_string(),
                    "Move secrets to .env file".to_string(),
                ],
                duration_ms: 200,
                check_hash: "ghi789".to_string(),
            },
        ],
        artifacts: ArtifactPaths {
            receipt_path: PathBuf::from("./receipts/test.json"),
            report_path: PathBuf::from("./reports/test.md"),
            bundle_path: None,
        },
        duration_ms: 1500,
    }
}

fn create_warning_result() -> DodValidationResult {
    let mut category_scores = HashMap::new();
    category_scores.insert(
        CheckCategory::BuildCorrectness,
        CategoryScore {
            category: CheckCategory::BuildCorrectness,
            score: 98.0,
            weight: 0.25,
            checks_passed: 2,
            checks_failed: 0,
            checks_warned: 1,
            checks_skipped: 0,
        },
    );

    DodValidationResult {
        verdict: OverallVerdict::Ready,
        readiness_score: 95.0,
        profile: "dev".to_string(),
        mode: ValidationMode::Fast,
        summary: ValidationSummary {
            checks_total: 3,
            checks_passed: 2,
            checks_failed: 0,
            checks_warned: 1,
            checks_skipped: 0,
        },
        category_scores,
        check_results: vec![
            DodCheckResult {
                id: "BUILD_CHECK".to_string(),
                category: CheckCategory::BuildCorrectness,
                status: CheckStatus::Pass,
                severity: CheckSeverity::Fatal,
                message: "Build passed".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms: 1200,
                check_hash: "abc123".to_string(),
            },
            DodCheckResult {
                id: "BUILD_FMT".to_string(),
                category: CheckCategory::BuildCorrectness,
                status: CheckStatus::Pass,
                severity: CheckSeverity::Fatal,
                message: "Formatting OK".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms: 300,
                check_hash: "def456".to_string(),
            },
            DodCheckResult {
                id: "BUILD_CLIPPY".to_string(),
                category: CheckCategory::BuildCorrectness,
                status: CheckStatus::Warn,
                severity: CheckSeverity::Warning,
                message: "Minor clippy warnings detected".to_string(),
                evidence: vec![],
                remediation: vec!["Run: cargo clippy --fix".to_string()],
                duration_ms: 800,
                check_hash: "ghi789".to_string(),
            },
        ],
        artifacts: ArtifactPaths {
            receipt_path: PathBuf::from("./receipts/test.json"),
            report_path: PathBuf::from("./reports/test.md"),
            bundle_path: None,
        },
        duration_ms: 2300,
    }
}

fn create_mixed_categories_result() -> DodValidationResult {
    let mut category_scores = HashMap::new();
    category_scores.insert(
        CheckCategory::WorkspaceIntegrity,
        CategoryScore {
            category: CheckCategory::WorkspaceIntegrity,
            score: 100.0,
            weight: 0.0,
            checks_passed: 1,
            checks_failed: 0,
            checks_warned: 0,
            checks_skipped: 0,
        },
    );
    category_scores.insert(
        CheckCategory::IntentAlignment,
        CategoryScore {
            category: CheckCategory::IntentAlignment,
            score: 100.0,
            weight: 0.05,
            checks_passed: 1,
            checks_failed: 0,
            checks_warned: 0,
            checks_skipped: 0,
        },
    );
    category_scores.insert(
        CheckCategory::ToolRegistry,
        CategoryScore {
            category: CheckCategory::ToolRegistry,
            score: 100.0,
            weight: 0.15,
            checks_passed: 1,
            checks_failed: 0,
            checks_warned: 0,
            checks_skipped: 0,
        },
    );
    category_scores.insert(
        CheckCategory::GgenPipeline,
        CategoryScore {
            category: CheckCategory::GgenPipeline,
            score: 100.0,
            weight: 0.20,
            checks_passed: 1,
            checks_failed: 0,
            checks_warned: 0,
            checks_skipped: 0,
        },
    );
    category_scores.insert(
        CheckCategory::DeploymentReadiness,
        CategoryScore {
            category: CheckCategory::DeploymentReadiness,
            score: 100.0,
            weight: 0.0,
            checks_passed: 1,
            checks_failed: 0,
            checks_warned: 0,
            checks_skipped: 0,
        },
    );

    DodValidationResult {
        verdict: OverallVerdict::Ready,
        readiness_score: 90.0,
        profile: "comprehensive".to_string(),
        mode: ValidationMode::Paranoid,
        summary: ValidationSummary {
            checks_total: 5,
            checks_passed: 5,
            checks_failed: 0,
            checks_warned: 0,
            checks_skipped: 0,
        },
        category_scores,
        check_results: vec![
            DodCheckResult {
                id: "G0_WORKSPACE".to_string(),
                category: CheckCategory::WorkspaceIntegrity,
                status: CheckStatus::Pass,
                severity: CheckSeverity::Fatal,
                message: "All required paths exist".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms: 10,
                check_hash: "ws1".to_string(),
            },
            DodCheckResult {
                id: "INTENT_PRD".to_string(),
                category: CheckCategory::IntentAlignment,
                status: CheckStatus::Pass,
                severity: CheckSeverity::Warning,
                message: "PRD documentation exists".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms: 5,
                check_hash: "int1".to_string(),
            },
            DodCheckResult {
                id: "TOOL_OPENAPI_ALIGNMENT".to_string(),
                category: CheckCategory::ToolRegistry,
                status: CheckStatus::Pass,
                severity: CheckSeverity::Fatal,
                message: "Tools align with OpenAPI spec".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms: 200,
                check_hash: "tool1".to_string(),
            },
            DodCheckResult {
                id: "GGEN_DRY_RUN".to_string(),
                category: CheckCategory::GgenPipeline,
                status: CheckStatus::Pass,
                severity: CheckSeverity::Fatal,
                message: "Ggen dry-run successful".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms: 500,
                check_hash: "ggen1".to_string(),
            },
            DodCheckResult {
                id: "DEPLOY_BUILD_RELEASE".to_string(),
                category: CheckCategory::DeploymentReadiness,
                status: CheckStatus::Pass,
                severity: CheckSeverity::Fatal,
                message: "Release build successful".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms: 8000,
                check_hash: "deploy1".to_string(),
            },
        ],
        artifacts: ArtifactPaths {
            receipt_path: PathBuf::from("./receipts/test.json"),
            report_path: PathBuf::from("./reports/test.md"),
            bundle_path: None,
        },
        duration_ms: 8715,
    }
}

// Tests

#[test]
fn test_report_generation_succeeds() {
    let result = create_passing_result();
    let report = ReportGenerator::generate_markdown(&result);
    assert!(report.is_ok(), "Report generation should succeed");
}

#[test]
fn test_passing_verdict_format() {
    let result = create_passing_result();
    let report = ReportGenerator::generate_markdown(&result).unwrap();

    assert!(report.contains("# Definition of Done Report"));
    assert!(report.contains("**Verdict**: ✅ PASS"));
    assert!(report.contains("**Score**: 100.0/100.0"));
    assert!(report.contains("**Profile**: dev"));
    assert!(report.contains("**Mode**: Fast"));
}

#[test]
fn test_failing_verdict_format() {
    let result = create_failing_result();
    let report = ReportGenerator::generate_markdown(&result).unwrap();

    assert!(report.contains("**Verdict**: ❌ FAIL"));
    assert!(report.contains("**Score**: 35.0/100.0"));
    assert!(report.contains("**Profile**: strict"));
    assert!(report.contains("**Mode**: Strict"));
}

#[test]
fn test_summary_section_complete() {
    let result = create_passing_result();
    let report = ReportGenerator::generate_markdown(&result).unwrap();

    assert!(report.contains("## Summary"));
    assert!(report.contains("**Total Checks**: 5"));
    assert!(report.contains("**Passed**: 5 ✅"));
    assert!(report.contains("**Failed**: 0 ❌"));
    assert!(report.contains("**Warnings**: 0 ⚠️"));
    assert!(report.contains("**Skipped**: 0 ⏭️"));
}

#[test]
fn test_summary_with_failures() {
    let result = create_failing_result();
    let report = ReportGenerator::generate_markdown(&result).unwrap();

    assert!(report.contains("**Total Checks**: 3"));
    assert!(report.contains("**Passed**: 1 ✅"));
    assert!(report.contains("**Failed**: 2 ❌"));
}

#[test]
fn test_category_sections_present() {
    let result = create_passing_result();
    let report = ReportGenerator::generate_markdown(&result).unwrap();

    assert!(report.contains("## Checks by Category"));
    assert!(report.contains("### D. Build Correctness"));
    assert!(report.contains("### E. Test Truth"));
}

#[test]
fn test_all_category_headers() {
    let result = create_mixed_categories_result();
    let report = ReportGenerator::generate_markdown(&result).unwrap();

    assert!(report.contains("### A. Workspace Integrity (G0)"));
    assert!(report.contains("### B. Intent Alignment (WHY)"));
    assert!(report.contains("### C. Tool Registry (WHAT)"));
    assert!(report.contains("### F. Ggen Pipeline"));
    assert!(report.contains("### H. Deployment Readiness"));
}

#[test]
fn test_category_table_format() {
    let result = create_passing_result();
    let report = ReportGenerator::generate_markdown(&result).unwrap();

    assert!(report.contains("| Check | Verdict | Severity | Message |"));
    assert!(report.contains("|-------|---------|----------|----------|"));
    assert!(report.contains("| BUILD_CHECK | ✅ Pass | Fatal | Compilation successful |"));
    assert!(report.contains("| BUILD_FMT | ✅ Pass | Fatal | Code formatting correct |"));
}

#[test]
fn test_check_status_emojis() {
    let result = create_failing_result();
    let report = ReportGenerator::generate_markdown(&result).unwrap();

    assert!(report.contains("✅ Pass"));
    assert!(report.contains("❌ Fail"));
}

#[test]
fn test_warning_status_emoji() {
    let result = create_warning_result();
    let report = ReportGenerator::generate_markdown(&result).unwrap();

    assert!(report.contains("⚠️ Warning"));
    assert!(report.contains("**Warnings**: 1 ⚠️"));
}

#[test]
fn test_category_scores_displayed() {
    let result = create_passing_result();
    let report = ReportGenerator::generate_markdown(&result).unwrap();

    assert!(report.contains("**Score**: 100.0/100.0 (weight: 25%)"));
}

#[test]
fn test_remediation_section_for_failures() {
    let result = create_failing_result();
    let report = ReportGenerator::generate_markdown(&result).unwrap();

    assert!(report.contains("## Remediation"));
    assert!(report.contains("Address the following issues to pass all checks"));
}

#[test]
fn test_no_remediation_for_passing() {
    let result = create_passing_result();
    let report = ReportGenerator::generate_markdown(&result).unwrap();

    assert!(!report.contains("## Remediation"));
}

#[test]
fn test_remediation_includes_check_ids() {
    let result = create_failing_result();
    let report = ReportGenerator::generate_markdown(&result).unwrap();

    assert!(report.contains("`BUILD_FMT`"));
    assert!(report.contains("`G8_SECRETS`"));
}

#[test]
fn test_remediation_priority_sections() {
    let result = create_failing_result();
    let report = ReportGenerator::generate_markdown(&result).unwrap();

    // Should have critical priority section for G8_SECRETS and BUILD_FMT
    assert!(report.contains("### 🚨 Critical Priority"));
}

#[test]
fn test_remediation_includes_automation_commands() {
    let result = create_failing_result();
    let report = ReportGenerator::generate_markdown(&result).unwrap();

    assert!(report.contains("**Quick Fix**: `cargo fmt`"));
}

#[test]
fn test_markdown_escaping() {
    let mut result = create_passing_result();
    result.check_results[0].message = "Build | test | success".to_string();

    let report = ReportGenerator::generate_markdown(&result).unwrap();
    assert!(report.contains("Build \\| test \\| success"));
}

#[test]
fn test_multiline_message_handling() {
    let mut result = create_passing_result();
    result.check_results[0].message = "Line 1\nLine 2\nLine 3".to_string();

    let report = ReportGenerator::generate_markdown(&result).unwrap();
    assert!(report.contains("Line 1 Line 2 Line 3"));
}

#[test]
fn test_duration_displayed() {
    let result = create_passing_result();
    let report = ReportGenerator::generate_markdown(&result).unwrap();

    assert!(report.contains("**Duration**: 9800ms"));
}

#[test]
fn test_severity_levels_displayed() {
    let result = create_warning_result();
    let report = ReportGenerator::generate_markdown(&result).unwrap();

    assert!(report.contains("| Fatal |"));
    assert!(report.contains("| Warning |"));
}

#[test]
fn test_comprehensive_report_structure() {
    let result = create_failing_result();
    let report = ReportGenerator::generate_markdown(&result).unwrap();

    // Verify complete structure
    assert!(report.contains("# Definition of Done Report"));
    assert!(report.contains("## Summary"));
    assert!(report.contains("## Checks by Category"));
    assert!(report.contains("## Remediation"));

    // Verify ordering
    let header_pos = report.find("# Definition of Done Report").unwrap();
    let summary_pos = report.find("## Summary").unwrap();
    let category_pos = report.find("## Checks by Category").unwrap();
    let remediation_pos = report.find("## Remediation").unwrap();

    assert!(header_pos < summary_pos);
    assert!(summary_pos < category_pos);
    assert!(category_pos < remediation_pos);
}

#[test]
fn test_empty_categories_omitted() {
    let result = create_passing_result();
    let report = ReportGenerator::generate_markdown(&result).unwrap();

    // Categories not in result should not appear
    assert!(!report.contains("### A. Workspace Integrity (G0)"));
    assert!(!report.contains("### G. Safety Invariants"));
}

#[test]
fn test_paranoid_mode_displayed() {
    let result = create_mixed_categories_result();
    let report = ReportGenerator::generate_markdown(&result).unwrap();

    assert!(report.contains("**Mode**: Paranoid"));
}
