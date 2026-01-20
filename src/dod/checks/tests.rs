//! Category E: Test Truth Checks
//!
//! Validates test execution and coverage.

use crate::dod::check::{CheckContext, DodCheck};
use crate::dod::types::*;
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Command;
use std::time::Duration;

/// TEST_UNIT: Run unit tests
pub struct TestUnitCheck;

#[async_trait]
impl DodCheck for TestUnitCheck {
    fn id(&self) -> &str {
        "TEST_UNIT"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::TestTruth
    }

    fn severity(&self) -> CheckSeverity {
        CheckSeverity::Fatal
    }

    fn description(&self) -> &str {
        "Runs unit tests with cargo test --lib"
    }

    async fn execute(&self, context: &CheckContext) -> Result<DodCheckResult> {
        let start = std::time::Instant::now();

        let output = tokio::time::timeout(
            Duration::from_millis(context.timeout_ms),
            tokio::task::spawn_blocking({
                let workspace = context.workspace_root.clone();
                move || {
                    Command::new("cargo")
                        .args(&["test", "--lib", "--no-fail-fast", "--", "--test-threads=1"])
                        .current_dir(&workspace)
                        .output()
                }
            }),
        )
        .await
        .context("Timeout waiting for unit tests")??
        .context("Failed to execute cargo test")?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let test_summary = parse_test_output(&stdout);

        if output.status.success() {
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Pass,
                severity: self.severity(),
                message: format!(
                    "All unit tests passed ({} tests)",
                    test_summary.passed
                ),
                evidence: vec![Evidence {
                    kind: EvidenceKind::Metric,
                    content: format!("Passed: {}, Failed: {}, Ignored: {}",
                        test_summary.passed, test_summary.failed, test_summary.ignored),
                    file_path: None,
                    line_number: None,
                    hash: String::new(),
                }],
                remediation: vec![],
                duration_ms,
                check_hash: String::new(),
            })
        } else {
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Fail,
                severity: self.severity(),
                message: format!("{} unit test(s) failed", test_summary.failed),
                evidence: vec![
                    Evidence {
                        kind: EvidenceKind::CommandOutput,
                        content: format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr),
                        file_path: None,
                        line_number: None,
                        hash: String::new(),
                    },
                ],
                remediation: vec![
                    "Review test failures above".to_string(),
                    "Run `cargo test --lib` to reproduce locally".to_string(),
                ],
                duration_ms,
                check_hash: String::new(),
            })
        }
    }
}

/// TEST_INTEGRATION: Run integration tests
pub struct TestIntegrationCheck;

#[async_trait]
impl DodCheck for TestIntegrationCheck {
    fn id(&self) -> &str {
        "TEST_INTEGRATION"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::TestTruth
    }

    fn severity(&self) -> CheckSeverity {
        CheckSeverity::Fatal
    }

    fn description(&self) -> &str {
        "Runs integration tests"
    }

    async fn execute(&self, context: &CheckContext) -> Result<DodCheckResult> {
        let start = std::time::Instant::now();

        let output = tokio::time::timeout(
            Duration::from_millis(context.timeout_ms),
            tokio::task::spawn_blocking({
                let workspace = context.workspace_root.clone();
                move || {
                    Command::new("cargo")
                        .args(&["test", "--test", "*", "--no-fail-fast"])
                        .current_dir(&workspace)
                        .output()
                }
            }),
        )
        .await
        .context("Timeout waiting for integration tests")??
        .context("Failed to execute integration tests")?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let test_summary = parse_test_output(&stdout);

        if output.status.success() {
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Pass,
                severity: self.severity(),
                message: format!("All integration tests passed ({} tests)", test_summary.passed),
                evidence: vec![Evidence {
                    kind: EvidenceKind::Metric,
                    content: format!("Passed: {}, Failed: {}, Ignored: {}",
                        test_summary.passed, test_summary.failed, test_summary.ignored),
                    file_path: None,
                    line_number: None,
                    hash: String::new(),
                }],
                remediation: vec![],
                duration_ms,
                check_hash: String::new(),
            })
        } else {
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Fail,
                severity: self.severity(),
                message: format!("{} integration test(s) failed", test_summary.failed),
                evidence: vec![Evidence {
                    kind: EvidenceKind::CommandOutput,
                    content: format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr),
                    file_path: None,
                    line_number: None,
                    hash: String::new(),
                }],
                remediation: vec![
                    "Review test failures above".to_string(),
                    "Fix failing integration tests".to_string(),
                ],
                duration_ms,
                check_hash: String::new(),
            })
        }
    }
}

/// TEST_SNAPSHOT: Verify snapshot tests are current
pub struct TestSnapshotCheck;

#[async_trait]
impl DodCheck for TestSnapshotCheck {
    fn id(&self) -> &str {
        "TEST_SNAPSHOT"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::TestTruth
    }

    fn severity(&self) -> CheckSeverity {
        CheckSeverity::Warning
    }

    fn description(&self) -> &str {
        "Verifies snapshot tests are up to date"
    }

    async fn execute(&self, context: &CheckContext) -> Result<DodCheckResult> {
        let start = std::time::Instant::now();

        // Run tests with snapshot verification
        let output = tokio::time::timeout(
            Duration::from_millis(context.timeout_ms),
            tokio::task::spawn_blocking({
                let workspace = context.workspace_root.clone();
                move || {
                    Command::new("cargo")
                        .args(&["test", "--features", "snapshot-testing", "--no-fail-fast"])
                        .current_dir(&workspace)
                        .env("INSTA_FORCE_PASS", "0")
                        .output()
                }
            }),
        )
        .await
        .context("Timeout waiting for snapshot tests")??
        .context("Failed to execute snapshot tests")?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Check for snapshot differences
        let has_snapshot_diffs = stdout.contains("snapshot mismatch")
            || stderr.contains("snapshot mismatch")
            || stdout.contains("to update snapshots run")
            || stderr.contains("to update snapshots run");

        if output.status.success() && !has_snapshot_diffs {
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Pass,
                severity: self.severity(),
                message: "All snapshot tests are current".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms,
                check_hash: String::new(),
            })
        } else if has_snapshot_diffs {
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Warn,
                severity: self.severity(),
                message: "Snapshot tests have mismatches".to_string(),
                evidence: vec![Evidence {
                    kind: EvidenceKind::CommandOutput,
                    content: format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr),
                    file_path: None,
                    line_number: None,
                    hash: String::new(),
                }],
                remediation: vec![
                    "Review snapshot differences".to_string(),
                    "Run `cargo insta review` to accept changes".to_string(),
                    "Or run `cargo insta test` to regenerate snapshots".to_string(),
                ],
                duration_ms,
                check_hash: String::new(),
            })
        } else {
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Fail,
                severity: self.severity(),
                message: "Snapshot tests failed".to_string(),
                evidence: vec![Evidence {
                    kind: EvidenceKind::CommandOutput,
                    content: format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr),
                    file_path: None,
                    line_number: None,
                    hash: String::new(),
                }],
                remediation: vec![
                    "Fix failing snapshot tests".to_string(),
                ],
                duration_ms,
                check_hash: String::new(),
            })
        }
    }
}

#[derive(Debug, Default)]
struct TestSummary {
    passed: usize,
    failed: usize,
    ignored: usize,
}

fn parse_test_output(output: &str) -> TestSummary {
    let mut summary = TestSummary::default();

    for line in output.lines() {
        if line.contains("test result:") {
            // Parse: "test result: ok. 42 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out"
            let parts: Vec<&str> = line.split_whitespace().collect();
            for (i, part) in parts.iter().enumerate() {
                if *part == "passed;" && i > 0 {
                    if let Ok(n) = parts[i - 1].parse::<usize>() {
                        summary.passed = n;
                    }
                }
                if *part == "failed;" && i > 0 {
                    if let Ok(n) = parts[i - 1].parse::<usize>() {
                        summary.failed = n;
                    }
                }
                if *part == "ignored;" && i > 0 {
                    if let Ok(n) = parts[i - 1].parse::<usize>() {
                        summary.ignored = n;
                    }
                }
            }
        }
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_test_output_success() {
        let output = "test result: ok. 42 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out";
        let summary = parse_test_output(output);
        assert_eq!(summary.passed, 42);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.ignored, 3);
    }

    #[test]
    fn parse_test_output_with_failures() {
        let output = "test result: FAILED. 38 passed; 4 failed; 3 ignored; 0 measured; 0 filtered out";
        let summary = parse_test_output(output);
        assert_eq!(summary.passed, 38);
        assert_eq!(summary.failed, 4);
        assert_eq!(summary.ignored, 3);
    }

    #[test]
    fn parse_test_output_empty() {
        let output = "";
        let summary = parse_test_output(output);
        assert_eq!(summary.passed, 0);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.ignored, 0);
    }
}
