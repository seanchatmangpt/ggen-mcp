//! Example: Generate DoD Markdown Report
//!
//! Demonstrates how to generate a formatted markdown report from DoD validation results.
//!
//! Run with: cargo run --example dod_report_example

use ggen_mcp::dod::*;
use std::collections::HashMap;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("=== DoD Report Generator Example ===\n");

    // Create a sample validation result with mixed statuses
    let result = create_sample_result();

    // Generate markdown report
    let report = ReportGenerator::generate_markdown(&result)?;

    // Print the report
    println!("{}", report);

    // Optionally save to file
    std::fs::write("./dod_report_example.md", &report)?;
    println!("\nReport saved to: ./dod_report_example.md");

    Ok(())
}

fn create_sample_result() -> DodValidationResult {
    let mut category_scores = HashMap::new();

    // Build category: some failures
    category_scores.insert(
        CheckCategory::BuildCorrectness,
        CategoryScore {
            category: CheckCategory::BuildCorrectness,
            score: 66.7,
            weight: 0.25,
            checks_passed: 2,
            checks_failed: 1,
            checks_warned: 0,
            checks_skipped: 0,
        },
    );

    // Test category: perfect
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

    // Ggen category: warnings
    category_scores.insert(
        CheckCategory::GgenPipeline,
        CategoryScore {
            category: CheckCategory::GgenPipeline,
            score: 96.0,
            weight: 0.20,
            checks_passed: 2,
            checks_failed: 0,
            checks_warned: 2,
            checks_skipped: 0,
        },
    );

    // Safety category: critical failure
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
        readiness_score: 65.7,
        profile: "dev".to_string(),
        mode: ValidationMode::Fast,
        summary: ValidationSummary {
            checks_total: 8,
            checks_passed: 6,
            checks_failed: 2,
            checks_warned: 2,
            checks_skipped: 0,
        },
        category_scores,
        check_results: vec![
            // Build checks
            DodCheckResult {
                id: "BUILD_CHECK".to_string(),
                category: CheckCategory::BuildCorrectness,
                status: CheckStatus::Pass,
                severity: CheckSeverity::Fatal,
                message: "Cargo check passed".to_string(),
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
                message: "Code formatting issues detected".to_string(),
                evidence: vec![],
                remediation: vec!["Run: cargo fmt".to_string()],
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
            // Test checks
            DodCheckResult {
                id: "TEST_UNIT".to_string(),
                category: CheckCategory::TestTruth,
                status: CheckStatus::Pass,
                severity: CheckSeverity::Fatal,
                message: "All 42 unit tests passed".to_string(),
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
                message: "All 12 integration tests passed".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms: 5000,
                check_hash: "mno345".to_string(),
            },
            // Ggen checks
            DodCheckResult {
                id: "GGEN_DRY_RUN".to_string(),
                category: CheckCategory::GgenPipeline,
                status: CheckStatus::Pass,
                severity: CheckSeverity::Fatal,
                message: "Ggen dry-run validation passed".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms: 500,
                check_hash: "pqr678".to_string(),
            },
            DodCheckResult {
                id: "GGEN_RENDER".to_string(),
                category: CheckCategory::GgenPipeline,
                status: CheckStatus::Warn,
                severity: CheckSeverity::Warning,
                message: "Generated code has minor warnings".to_string(),
                evidence: vec![],
                remediation: vec!["Review generated code warnings".to_string()],
                duration_ms: 800,
                check_hash: "stu901".to_string(),
            },
            // Safety checks
            DodCheckResult {
                id: "G8_SECRETS".to_string(),
                category: CheckCategory::SafetyInvariants,
                status: CheckStatus::Fail,
                severity: CheckSeverity::Fatal,
                message: "Hardcoded API key detected in src/config.rs:42".to_string(),
                evidence: vec![],
                remediation: vec![
                    "Remove hardcoded secrets from source code".to_string(),
                    "Move secrets to .env file".to_string(),
                    "Add .env to .gitignore".to_string(),
                    "Rotate exposed credentials immediately".to_string(),
                ],
                duration_ms: 200,
                check_hash: "vwx234".to_string(),
            },
        ],
        artifacts: ArtifactPaths {
            receipt_path: PathBuf::from("./ggen.out/receipts/latest.json"),
            report_path: PathBuf::from("./ggen.out/reports/latest.md"),
            bundle_path: Some(PathBuf::from("./ggen.out/bundles/latest.tar.gz")),
        },
        duration_ms: 11300,
    }
}
