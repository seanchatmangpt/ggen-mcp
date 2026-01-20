//! Category F: ggen Pipeline Checks
//!
//! Validates ontology-driven code generation pipeline.

use crate::dod::check::{CheckContext, DodCheck};
use crate::dod::types::*;
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Command;
use std::time::Duration;

/// GGEN_DRY_RUN: Validate ggen sync dry-run
pub struct GgenDryRunCheck;

#[async_trait]
impl DodCheck for GgenDryRunCheck {
    fn id(&self) -> &str {
        "GGEN_DRY_RUN"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::GgenPipeline
    }

    fn severity(&self) -> CheckSeverity {
        CheckSeverity::Fatal
    }

    fn description(&self) -> &str {
        "Validates ggen pipeline with sync --preview (dry-run)"
    }

    async fn execute(&self, context: &CheckContext) -> Result<DodCheckResult> {
        let start = std::time::Instant::now();

        // Run: cargo make sync (default preview mode)
        let output = tokio::time::timeout(
            Duration::from_millis(context.timeout_ms),
            tokio::task::spawn_blocking({
                let workspace = context.workspace_root.clone();
                move || {
                    Command::new("cargo")
                        .args(&["make", "sync"])
                        .current_dir(&workspace)
                        .env("GGEN_PREVIEW", "true")
                        .output()
                }
            }),
        )
        .await
        .context("Timeout waiting for ggen sync")??
        .context("Failed to execute ggen sync")?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            // Check if First Light Report was generated
            let report_path = context.workspace_root.join("ggen.out/reports/latest.md");
            let report_exists = report_path.exists();

            let mut evidence = vec![];
            if report_exists {
                evidence.push(Evidence {
                    kind: EvidenceKind::FileContent,
                    content: "First Light Report generated".to_string(),
                    file_path: Some(report_path),
                    line_number: None,
                    hash: String::new(),
                });
            }

            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Pass,
                severity: self.severity(),
                message: "ggen sync dry-run passed".to_string(),
                evidence,
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
                message: "ggen sync dry-run failed".to_string(),
                evidence: vec![Evidence {
                    kind: EvidenceKind::CommandOutput,
                    content: format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr),
                    file_path: None,
                    line_number: None,
                    hash: String::new(),
                }],
                remediation: vec![
                    "Review ggen sync errors above".to_string(),
                    "Check ggen.toml configuration".to_string(),
                    "Verify ontology parses: cargo test --test test_ontology".to_string(),
                    "Verify templates render: cargo test --test test_templates".to_string(),
                ],
                duration_ms,
                check_hash: String::new(),
            })
        }
    }
}

/// GGEN_RENDER: Validate template rendering with fixtures
pub struct GgenRenderCheck;

#[async_trait]
impl DodCheck for GgenRenderCheck {
    fn id(&self) -> &str {
        "GGEN_RENDER"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::GgenPipeline
    }

    fn severity(&self) -> CheckSeverity {
        CheckSeverity::Warning
    }

    fn description(&self) -> &str {
        "Validates templates render with fixture data"
    }

    async fn execute(&self, context: &CheckContext) -> Result<DodCheckResult> {
        let start = std::time::Instant::now();

        // Run template tests
        let output = tokio::time::timeout(
            Duration::from_millis(context.timeout_ms),
            tokio::task::spawn_blocking({
                let workspace = context.workspace_root.clone();
                move || {
                    Command::new("cargo")
                        .args(&["test", "--test", "test_templates"])
                        .current_dir(&workspace)
                        .output()
                }
            }),
        )
        .await
        .context("Timeout waiting for template tests")??
        .context("Failed to execute template tests")?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Pass,
                severity: self.severity(),
                message: "All templates render successfully".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms,
                check_hash: String::new(),
            })
        } else {
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Warn,
                severity: self.severity(),
                message: "Template rendering issues detected".to_string(),
                evidence: vec![Evidence {
                    kind: EvidenceKind::CommandOutput,
                    content: stderr.to_string(),
                    file_path: None,
                    line_number: None,
                    hash: String::new(),
                }],
                remediation: vec![
                    "Fix template syntax errors".to_string(),
                    "Update fixture data if schema changed".to_string(),
                ],
                duration_ms,
                check_hash: String::new(),
            })
        }
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["GGEN_DRY_RUN".to_string()]
    }
}

/// GGEN_ONTOLOGY: Validate ontology parses and validates
pub struct GgenOntologyCheck;

#[async_trait]
impl DodCheck for GgenOntologyCheck {
    fn id(&self) -> &str {
        "GGEN_ONTOLOGY"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::GgenPipeline
    }

    fn severity(&self) -> CheckSeverity {
        CheckSeverity::Fatal
    }

    fn description(&self) -> &str {
        "Validates ontology/mcp-domain.ttl parses correctly"
    }

    async fn execute(&self, context: &CheckContext) -> Result<DodCheckResult> {
        let start = std::time::Instant::now();

        // Run ontology tests
        let output = tokio::time::timeout(
            Duration::from_millis(context.timeout_ms),
            tokio::task::spawn_blocking({
                let workspace = context.workspace_root.clone();
                move || {
                    Command::new("cargo")
                        .args(&["test", "--test", "test_ontology"])
                        .current_dir(&workspace)
                        .output()
                }
            }),
        )
        .await
        .context("Timeout waiting for ontology tests")??
        .context("Failed to execute ontology tests")?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Pass,
                severity: self.severity(),
                message: "Ontology parses and validates successfully".to_string(),
                evidence: vec![],
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
                message: "Ontology validation failed".to_string(),
                evidence: vec![Evidence {
                    kind: EvidenceKind::CommandOutput,
                    content: stderr.to_string(),
                    file_path: Some(context.workspace_root.join("ontology/mcp-domain.ttl")),
                    line_number: None,
                    hash: String::new(),
                }],
                remediation: vec![
                    "Fix Turtle syntax errors in ontology/mcp-domain.ttl".to_string(),
                    "Run `cargo test --test test_ontology` to reproduce".to_string(),
                ],
                duration_ms,
                check_hash: String::new(),
            })
        }
    }
}

/// GGEN_SPARQL: Validate SPARQL queries execute without errors
pub struct GgenSparqlCheck;

#[async_trait]
impl DodCheck for GgenSparqlCheck {
    fn id(&self) -> &str {
        "GGEN_SPARQL"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::GgenPipeline
    }

    fn severity(&self) -> CheckSeverity {
        CheckSeverity::Fatal
    }

    fn description(&self) -> &str {
        "Validates SPARQL queries execute successfully"
    }

    async fn execute(&self, context: &CheckContext) -> Result<DodCheckResult> {
        let start = std::time::Instant::now();

        // Run SPARQL query tests
        let output = tokio::time::timeout(
            Duration::from_millis(context.timeout_ms),
            tokio::task::spawn_blocking({
                let workspace = context.workspace_root.clone();
                move || {
                    Command::new("cargo")
                        .args(&["test", "--test", "test_sparql"])
                        .current_dir(&workspace)
                        .output()
                }
            }),
        )
        .await
        .context("Timeout waiting for SPARQL tests")??
        .context("Failed to execute SPARQL tests")?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Pass,
                severity: self.severity(),
                message: "All SPARQL queries execute successfully".to_string(),
                evidence: vec![],
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
                message: "SPARQL query validation failed".to_string(),
                evidence: vec![Evidence {
                    kind: EvidenceKind::CommandOutput,
                    content: stderr.to_string(),
                    file_path: Some(context.workspace_root.join("queries")),
                    line_number: None,
                    hash: String::new(),
                }],
                remediation: vec![
                    "Fix SPARQL syntax errors in queries/*.rq".to_string(),
                    "Run `cargo test --test test_sparql` to reproduce".to_string(),
                ],
                duration_ms,
                check_hash: String::new(),
            })
        }
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["GGEN_ONTOLOGY".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn ggen_dry_run_check_metadata() {
        let check = GgenDryRunCheck;
        assert_eq!(check.id(), "GGEN_DRY_RUN");
        assert_eq!(check.category(), CheckCategory::GgenPipeline);
        assert_eq!(check.severity(), CheckSeverity::Fatal);
    }

    #[test]
    fn ggen_render_check_dependencies() {
        let check = GgenRenderCheck;
        let deps = check.dependencies();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], "GGEN_DRY_RUN");
    }

    #[test]
    fn ggen_ontology_check_metadata() {
        let check = GgenOntologyCheck;
        assert_eq!(check.id(), "GGEN_ONTOLOGY");
        assert_eq!(check.severity(), CheckSeverity::Fatal);
    }

    #[test]
    fn ggen_sparql_check_dependencies() {
        let check = GgenSparqlCheck;
        let deps = check.dependencies();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], "GGEN_ONTOLOGY");
    }
}
