//! Category D: Build Correctness Checks
//!
//! Validates build hygiene:
//! - BUILD_FMT: cargo fmt --check
//! - BUILD_CLIPPY: cargo clippy
//! - BUILD_CHECK: cargo check

use crate::dod::check::{CheckContext, DodCheck};
use crate::dod::types::*;
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Command;
use std::time::Duration;

/// BUILD_FMT: cargo fmt --check
pub struct BuildFmtCheck;

#[async_trait]
impl DodCheck for BuildFmtCheck {
    fn id(&self) -> &str {
        "BUILD_FMT"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::BuildCorrectness
    }

    fn severity(&self) -> CheckSeverity {
        CheckSeverity::Fatal
    }

    fn description(&self) -> &str {
        "Validates code formatting with cargo fmt"
    }

    async fn execute(&self, context: &CheckContext) -> Result<DodCheckResult> {
        let start = std::time::Instant::now();

        let output = tokio::time::timeout(
            Duration::from_millis(context.timeout_ms),
            tokio::task::spawn_blocking({
                let workspace = context.workspace_root.clone();
                move || {
                    Command::new("cargo")
                        .args(&["fmt", "--", "--check"])
                        .current_dir(&workspace)
                        .output()
                }
            }),
        )
        .await
        .context("Timeout waiting for cargo fmt")??
        .context("Failed to execute cargo fmt")?;

        let duration_ms = start.elapsed().as_millis() as u64;

        if output.status.success() {
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Pass,
                severity: self.severity(),
                message: "All code is properly formatted".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms,
                check_hash: String::new(),
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Fail,
                severity: self.severity(),
                message: "Code formatting issues detected".to_string(),
                evidence: vec![Evidence {
                    kind: EvidenceKind::CommandOutput,
                    content: stderr.to_string(),
                    file_path: None,
                    line_number: None,
                    hash: String::new(),
                }],
                remediation: vec!["Run `cargo fmt` to fix formatting".to_string()],
                duration_ms,
                check_hash: String::new(),
            })
        }
    }
}

/// BUILD_CLIPPY: cargo clippy
pub struct BuildClippyCheck;

#[async_trait]
impl DodCheck for BuildClippyCheck {
    fn id(&self) -> &str {
        "BUILD_CLIPPY"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::BuildCorrectness
    }

    fn severity(&self) -> CheckSeverity {
        CheckSeverity::Warning // Configurable via profile
    }

    fn description(&self) -> &str {
        "Validates code quality with cargo clippy"
    }

    async fn execute(&self, context: &CheckContext) -> Result<DodCheckResult> {
        let start = std::time::Instant::now();

        let output = tokio::time::timeout(
            Duration::from_millis(context.timeout_ms),
            tokio::task::spawn_blocking({
                let workspace = context.workspace_root.clone();
                move || {
                    Command::new("cargo")
                        .args(&[
                            "clippy",
                            "--all-targets",
                            "--all-features",
                            "--",
                            "-D",
                            "warnings",
                        ])
                        .current_dir(&workspace)
                        .output()
                }
            }),
        )
        .await
        .context("Timeout waiting for cargo clippy")??
        .context("Failed to execute cargo clippy")?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let warning_count = count_clippy_warnings(&stdout, &stderr);

        if output.status.success() {
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Pass,
                severity: self.severity(),
                message: "No clippy warnings".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms,
                check_hash: String::new(),
            })
        } else if warning_count > 0 {
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Warn,
                severity: self.severity(),
                message: format!("{} clippy warning(s) detected", warning_count),
                evidence: vec![Evidence {
                    kind: EvidenceKind::CommandOutput,
                    content: format!("{}\n{}", stdout, stderr),
                    file_path: None,
                    line_number: None,
                    hash: String::new(),
                }],
                remediation: vec![
                    "Review clippy warnings and fix issues".to_string(),
                    "Run `cargo clippy --fix` for automatic fixes".to_string(),
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
                message: "Clippy check failed".to_string(),
                evidence: vec![Evidence {
                    kind: EvidenceKind::CommandOutput,
                    content: stderr.to_string(),
                    file_path: None,
                    line_number: None,
                    hash: String::new(),
                }],
                remediation: vec!["Fix clippy errors".to_string()],
                duration_ms,
                check_hash: String::new(),
            })
        }
    }
}

/// BUILD_CHECK: cargo check
pub struct BuildCheckCheck;

#[async_trait]
impl DodCheck for BuildCheckCheck {
    fn id(&self) -> &str {
        "BUILD_CHECK"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::BuildCorrectness
    }

    fn severity(&self) -> CheckSeverity {
        CheckSeverity::Fatal
    }

    fn description(&self) -> &str {
        "Validates code compiles with cargo check"
    }

    async fn execute(&self, context: &CheckContext) -> Result<DodCheckResult> {
        let start = std::time::Instant::now();

        let output = tokio::time::timeout(
            Duration::from_millis(context.timeout_ms),
            tokio::task::spawn_blocking({
                let workspace = context.workspace_root.clone();
                move || {
                    Command::new("cargo")
                        .args(&["check", "--all-targets", "--all-features"])
                        .current_dir(&workspace)
                        .output()
                }
            }),
        )
        .await
        .context("Timeout waiting for cargo check")??
        .context("Failed to execute cargo check")?;

        let duration_ms = start.elapsed().as_millis() as u64;

        if output.status.success() {
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Pass,
                severity: self.severity(),
                message: "Code compiles successfully".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms,
                check_hash: String::new(),
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Fail,
                severity: self.severity(),
                message: "Compilation errors detected".to_string(),
                evidence: vec![Evidence {
                    kind: EvidenceKind::CommandOutput,
                    content: stderr.to_string(),
                    file_path: None,
                    line_number: None,
                    hash: String::new(),
                }],
                remediation: vec!["Fix compilation errors shown above".to_string()],
                duration_ms,
                check_hash: String::new(),
            })
        }
    }
}

fn count_clippy_warnings(stdout: &str, stderr: &str) -> usize {
    let combined = format!("{}\n{}", stdout, stderr);
    combined
        .lines()
        .filter(|line| line.contains("warning:"))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_build_fmt_check_structure() {
        let check = BuildFmtCheck;
        assert_eq!(check.id(), "BUILD_FMT");
        assert_eq!(check.category(), CheckCategory::BuildCorrectness);
        assert_eq!(check.severity(), CheckSeverity::Fatal);
    }

    #[tokio::test]
    async fn test_build_clippy_check_structure() {
        let check = BuildClippyCheck;
        assert_eq!(check.id(), "BUILD_CLIPPY");
        assert_eq!(check.category(), CheckCategory::BuildCorrectness);
    }

    #[tokio::test]
    async fn test_build_check_check_structure() {
        let check = BuildCheckCheck;
        assert_eq!(check.id(), "BUILD_CHECK");
        assert_eq!(check.severity(), CheckSeverity::Fatal);
    }

    #[test]
    fn test_count_clippy_warnings() {
        let output = "warning: unused variable\nwarning: dead code\n";
        assert_eq!(count_clippy_warnings(output, ""), 2);
    }
}
