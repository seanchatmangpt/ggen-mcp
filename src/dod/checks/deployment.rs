//! Category H: Deployment Readiness
//!
//! H1_ARTIFACTS: Release artifact buildability

use crate::dod::check::{CheckContext, DodCheck};
use crate::dod::types::*;
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Command;
use std::time::Duration;

/// H1_ARTIFACTS: Release artifact buildability
pub struct ArtifactBuildCheck;

#[async_trait]
impl DodCheck for ArtifactBuildCheck {
    fn id(&self) -> &str {
        "H1_ARTIFACTS"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::DeploymentReadiness
    }

    fn severity(&self) -> CheckSeverity {
        CheckSeverity::Fatal
    }

    fn description(&self) -> &str {
        "Validates release artifacts can be built successfully"
    }

    async fn execute(&self, context: &CheckContext) -> Result<DodCheckResult> {
        let start = std::time::Instant::now();
        let mut evidence = vec![];
        let mut remediation = vec![];

        // Build release artifact
        let output = tokio::time::timeout(
            Duration::from_millis(context.timeout_ms),
            tokio::task::spawn_blocking({
                let workspace = context.workspace_root.clone();
                move || {
                    Command::new("cargo")
                        .args(&["build", "--release", "--locked"])
                        .current_dir(&workspace)
                        .output()
                }
            }),
        )
        .await
        .context("Timeout waiting for release build")??
        .context("Failed to execute cargo build --release")?;

        let duration_ms = start.elapsed().as_millis() as u64;

        if output.status.success() {
            // Verify binary exists and has reasonable size
            let binary_path = context.workspace_root.join("target/release/spreadsheet-mcp");

            if !binary_path.exists() {
                return Ok(DodCheckResult {
                    id: self.id().to_string(),
                    category: self.category(),
                    status: CheckStatus::Fail,
                    severity: self.severity(),
                    message: "Release binary not found after build".to_string(),
                    evidence,
                    remediation: vec!["Check Cargo.toml [[bin]] configuration".to_string()],
                    duration_ms,
                    check_hash: "".to_string(),
                });
            }

            let metadata = std::fs::metadata(&binary_path)?;
            let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);

            evidence.push(Evidence {
                kind: EvidenceKind::Metric,
                content: format!("Binary size: {:.2} MB", size_mb),
                file_path: Some(binary_path.clone()),
                line_number: None,
                hash: "".to_string(),
            });

            // Smoke test: run --version
            let version_output = Command::new(&binary_path).arg("--version").output()?;

            if !version_output.status.success() {
                remediation.push("Binary smoke test failed (--version)".to_string());
            }

            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: if remediation.is_empty() {
                    CheckStatus::Pass
                } else {
                    CheckStatus::Warn
                },
                severity: self.severity(),
                message: format!("Release artifact built successfully ({:.2} MB)", size_mb),
                evidence,
                remediation,
                duration_ms,
                check_hash: "".to_string(),
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Fail,
                severity: self.severity(),
                message: "Release build failed".to_string(),
                evidence: vec![Evidence {
                    kind: EvidenceKind::CommandOutput,
                    content: stderr.to_string(),
                    file_path: None,
                    line_number: None,
                    hash: "".to_string(),
                }],
                remediation: vec![
                    "Fix release build errors".to_string(),
                    "Run `cargo build --release --locked` locally".to_string(),
                ],
                duration_ms,
                check_hash: "".to_string(),
            })
        }
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["BUILD_CHECK".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_artifact_build_check_properties() {
        let check = ArtifactBuildCheck;
        assert_eq!(check.id(), "H1_ARTIFACTS");
        assert_eq!(check.category(), CheckCategory::DeploymentReadiness);
        assert_eq!(check.severity(), CheckSeverity::Fatal);
        assert_eq!(check.dependencies(), vec!["BUILD_CHECK".to_string()]);
    }
}
