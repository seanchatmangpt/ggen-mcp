//! Guard Kernel - Poka-Yoke (Error-Proofing) System for ggen-mcp
//!
//! This module implements a 7-guard validation system following TPS principles:
//! - **G1: Path Safety** - Prevent path traversal attacks
//! - **G2: Output Overlap** - Detect duplicate output paths
//! - **G3: Template Compile** - Validate Tera template syntax
//! - **G4: Turtle Parse** - Validate RDF/Turtle ontology syntax
//! - **G5: SPARQL Execute** - Validate SPARQL query syntax
//! - **G6: Determinism** - Verify input determinism via hashing
//! - **G7: Bounds** - Enforce size and time limits
//!
//! ## Design Philosophy
//! - Fail-fast with actionable remediation
//! - Composable guard system (GuardKernel orchestrates)
//! - Each guard is independent and testable
//! - Guard results include diagnostic + remediation
//!
//! ## Usage
//! ```rust,ignore
//! use spreadsheet_mcp::guards::{GuardKernel, SyncContext};
//!
//! let guard_kernel = GuardKernel::default_suite();
//! let sync_context = SyncContext::from_workspace(workspace_root)?;
//! let results = guard_kernel.evaluate(&sync_context)?;
//!
//! if !results.all_passed() {
//!     eprintln!("Guards failed:");
//!     for failure in results.failures() {
//!         eprintln!("  {} - {}", failure.diagnostic, failure.remediation);
//!     }
//!     return Err(anyhow!("Guard failures"));
//! }
//! ```

pub mod bounds;
pub mod determinism;
pub mod output_overlap;
pub mod path_safety;
pub mod sparql_execute;
pub mod template_compile;
pub mod turtle_parse;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// Re-export guards
pub use bounds::BoundsGuard;
pub use determinism::DeterminismGuard;
pub use output_overlap::OutputOverlapGuard;
pub use path_safety::PathSafetyGuard;
pub use sparql_execute::SparqlExecuteGuard;
pub use template_compile::TemplateCompileGuard;
pub use turtle_parse::TurtleParseGuard;

// =============================================================================
// Core Guard Trait
// =============================================================================

/// Guard trait - each guard implements one validation check
pub trait Guard: Send + Sync {
    /// Guard identifier (e.g., "G1: Path Safety")
    fn name(&self) -> &str;

    /// Human-readable description
    fn description(&self) -> &str;

    /// Execute the guard check
    fn check(&self, ctx: &SyncContext) -> GuardResult;
}

// =============================================================================
// Guard Result Types
// =============================================================================

/// Result of a guard check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardResult {
    /// Guard name
    pub guard_name: String,

    /// Verdict: Pass or Fail
    pub verdict: Verdict,

    /// Diagnostic message explaining the result
    pub diagnostic: String,

    /// Remediation steps if failed
    pub remediation: String,

    /// Additional metadata (hashes, counts, etc.)
    pub metadata: HashMap<String, String>,
}

impl GuardResult {
    /// Create a passing result
    pub fn pass(guard_name: impl Into<String>, diagnostic: impl Into<String>) -> Self {
        Self {
            guard_name: guard_name.into(),
            verdict: Verdict::Pass,
            diagnostic: diagnostic.into(),
            remediation: String::new(),
            metadata: HashMap::new(),
        }
    }

    /// Create a passing result with metadata
    pub fn pass_with_metadata(
        guard_name: impl Into<String>,
        diagnostic: impl Into<String>,
        metadata: Vec<(&str, String)>,
    ) -> Self {
        Self {
            guard_name: guard_name.into(),
            verdict: Verdict::Pass,
            diagnostic: diagnostic.into(),
            remediation: String::new(),
            metadata: metadata
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        }
    }

    /// Create a failing result
    pub fn fail(
        guard_name: impl Into<String>,
        diagnostic: impl Into<String>,
        remediation: impl Into<String>,
    ) -> Self {
        Self {
            guard_name: guard_name.into(),
            verdict: Verdict::Fail,
            diagnostic: diagnostic.into(),
            remediation: remediation.into(),
            metadata: HashMap::new(),
        }
    }

    /// Create a failing result with metadata
    pub fn fail_with_metadata(
        guard_name: impl Into<String>,
        diagnostic: impl Into<String>,
        remediation: impl Into<String>,
        metadata: Vec<(&str, String)>,
    ) -> Self {
        Self {
            guard_name: guard_name.into(),
            verdict: Verdict::Fail,
            diagnostic: diagnostic.into(),
            remediation: remediation.into(),
            metadata: metadata
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        }
    }

    /// Check if this result is a pass
    pub fn is_pass(&self) -> bool {
        matches!(self.verdict, Verdict::Pass)
    }

    /// Check if this result is a fail
    pub fn is_fail(&self) -> bool {
        matches!(self.verdict, Verdict::Fail)
    }
}

/// Guard verdict
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    /// Guard check passed
    Pass,
    /// Guard check failed
    Fail,
}

// =============================================================================
// Sync Context (Input to Guards)
// =============================================================================

/// Execution context for guard evaluation
#[derive(Debug, Clone)]
pub struct SyncContext {
    /// Workspace root directory
    pub workspace_root: PathBuf,

    /// Discovered generation rules
    pub generation_rules: Vec<GenerationRule>,

    /// Discovered SPARQL queries
    pub discovered_queries: Vec<PathBuf>,

    /// Discovered Tera templates
    pub discovered_templates: Vec<PathBuf>,

    /// Discovered ontology files
    pub discovered_ontologies: Vec<PathBuf>,

    /// Config file content (if exists)
    pub config_content: String,

    /// Ontology file contents (for hashing)
    pub ontology_contents: Vec<String>,

    /// Query file contents (for hashing)
    pub query_contents: Vec<String>,

    /// Template file contents (for hashing)
    pub template_contents: Vec<String>,
}

impl SyncContext {
    /// Build context from workspace root
    pub fn from_workspace(workspace_root: &Path) -> Result<Self> {
        let workspace_root = workspace_root.to_path_buf();

        // Discover queries
        let queries_dir = workspace_root.join("queries");
        let discovered_queries = if queries_dir.exists() {
            std::fs::read_dir(&queries_dir)
                .context("Failed to read queries directory")?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("rq"))
                .collect()
        } else {
            Vec::new()
        };

        // Discover templates
        let templates_dir = workspace_root.join("templates");
        let discovered_templates = if templates_dir.exists() {
            std::fs::read_dir(&templates_dir)
                .context("Failed to read templates directory")?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|s| s.to_str())
                        .map(|s| s.ends_with(".tera"))
                        .unwrap_or(false)
                })
                .collect()
        } else {
            Vec::new()
        };

        // Discover ontologies
        let ontology_dir = workspace_root.join("ontology");
        let discovered_ontologies = if ontology_dir.exists() {
            std::fs::read_dir(&ontology_dir)
                .context("Failed to read ontology directory")?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("ttl"))
                .collect()
        } else {
            Vec::new()
        };

        // Read config
        let config_path = workspace_root.join("ggen.toml");
        let config_content = if config_path.exists() {
            std::fs::read_to_string(&config_path)
                .context("Failed to read ggen.toml")?
        } else {
            String::new()
        };

        // Read ontology contents
        let ontology_contents = discovered_ontologies
            .iter()
            .filter_map(|p| std::fs::read_to_string(p).ok())
            .collect();

        // Read query contents
        let query_contents = discovered_queries
            .iter()
            .filter_map(|p| std::fs::read_to_string(p).ok())
            .collect();

        // Read template contents
        let template_contents = discovered_templates
            .iter()
            .filter_map(|p| std::fs::read_to_string(p).ok())
            .collect();

        // Build generation rules from discovered resources
        let generation_rules = Self::infer_generation_rules(
            &discovered_queries,
            &discovered_templates,
            &workspace_root,
        );

        Ok(Self {
            workspace_root,
            generation_rules,
            discovered_queries,
            discovered_templates,
            discovered_ontologies,
            config_content,
            ontology_contents,
            query_contents,
            template_contents,
        })
    }

    /// Infer generation rules from discovered queries and templates
    fn infer_generation_rules(
        queries: &[PathBuf],
        templates: &[PathBuf],
        workspace_root: &Path,
    ) -> Vec<GenerationRule> {
        let mut rules = Vec::new();

        for query_path in queries {
            let stem = query_path.file_stem().and_then(|s| s.to_str());
            if let Some(stem) = stem {
                // Find matching template
                let matching_template = templates.iter().find(|t| {
                    t.file_name()
                        .and_then(|s| s.to_str())
                        .map(|s| s.starts_with(stem))
                        .unwrap_or(false)
                });

                if let Some(template_path) = matching_template {
                    // Infer output path: src/generated/{stem}.rs
                    let output_path = workspace_root
                        .join("src/generated")
                        .join(format!("{}.rs", stem));

                    rules.push(GenerationRule {
                        name: stem.to_string(),
                        query_path: query_path.clone(),
                        template_path: template_path.clone(),
                        output_path: output_path.to_string_lossy().to_string(),
                    });
                }
            }
        }

        rules
    }
}

/// Generation rule (query + template → output)
#[derive(Debug, Clone)]
pub struct GenerationRule {
    pub name: String,
    pub query_path: PathBuf,
    pub template_path: PathBuf,
    pub output_path: String,
}

// =============================================================================
// Guard Kernel (Orchestrator)
// =============================================================================

/// Guard kernel - orchestrates all guards
pub struct GuardKernel {
    guards: Vec<Box<dyn Guard>>,
}

impl GuardKernel {
    /// Create a new guard kernel with custom guards
    pub fn new(guards: Vec<Box<dyn Guard>>) -> Self {
        Self { guards }
    }

    /// Create the default suite of 7 guards
    pub fn default_suite() -> Self {
        Self {
            guards: vec![
                Box::new(PathSafetyGuard),
                Box::new(OutputOverlapGuard),
                Box::new(TemplateCompileGuard),
                Box::new(TurtleParseGuard),
                Box::new(SparqlExecuteGuard),
                Box::new(DeterminismGuard),
                Box::new(BoundsGuard),
            ],
        }
    }

    /// Evaluate all guards against the sync context
    pub fn evaluate(&self, ctx: &SyncContext) -> GuardResults {
        let results: Vec<GuardResult> = self.guards.iter().map(|g| g.check(ctx)).collect();

        GuardResults { results }
    }

    /// Get the number of guards
    pub fn guard_count(&self) -> usize {
        self.guards.len()
    }
}

// =============================================================================
// Guard Results Collection
// =============================================================================

/// Collection of guard results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardResults {
    pub results: Vec<GuardResult>,
}

impl GuardResults {
    /// Check if all guards passed
    pub fn all_passed(&self) -> bool {
        self.results.iter().all(|r| r.is_pass())
    }

    /// Get failed guards
    pub fn failures(&self) -> Vec<&GuardResult> {
        self.results.iter().filter(|r| r.is_fail()).collect()
    }

    /// Get passing guards
    pub fn passes(&self) -> Vec<&GuardResult> {
        self.results.iter().filter(|r| r.is_pass()).collect()
    }

    /// Count of failures
    pub fn failure_count(&self) -> usize {
        self.failures().len()
    }

    /// Count of passes
    pub fn pass_count(&self) -> usize {
        self.passes().len()
    }

    /// Generate remediation summary
    pub fn remediation_summary(&self) -> String {
        let failures = self.failures();
        if failures.is_empty() {
            return "All guards passed.".to_string();
        }

        let mut summary = format!("{} guard(s) failed:\n", failures.len());
        for (i, failure) in failures.iter().enumerate() {
            summary.push_str(&format!(
                "{}. {} - {}\n   Remediation: {}\n",
                i + 1,
                failure.guard_name,
                failure.diagnostic,
                failure.remediation
            ));
        }
        summary
    }

    /// Get diagnostic summary (passing + failing)
    pub fn diagnostic_summary(&self) -> String {
        let mut summary = String::new();
        summary.push_str(&format!(
            "Guard Results: {} passed, {} failed\n\n",
            self.pass_count(),
            self.failure_count()
        ));

        for result in &self.results {
            let status = if result.is_pass() { "✓" } else { "✗" };
            summary.push_str(&format!(
                "{} {} - {}\n",
                status, result.guard_name, result.diagnostic
            ));
        }

        summary
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guard_result_pass() {
        let result = GuardResult::pass("TestGuard", "Test passed");
        assert!(result.is_pass());
        assert!(!result.is_fail());
        assert_eq!(result.verdict, Verdict::Pass);
    }

    #[test]
    fn test_guard_result_fail() {
        let result = GuardResult::fail("TestGuard", "Test failed", "Fix it");
        assert!(result.is_fail());
        assert!(!result.is_pass());
        assert_eq!(result.verdict, Verdict::Fail);
    }

    #[test]
    fn test_guard_results_all_passed() {
        let results = GuardResults {
            results: vec![
                GuardResult::pass("G1", "Pass 1"),
                GuardResult::pass("G2", "Pass 2"),
            ],
        };
        assert!(results.all_passed());
        assert_eq!(results.failure_count(), 0);
        assert_eq!(results.pass_count(), 2);
    }

    #[test]
    fn test_guard_results_with_failures() {
        let results = GuardResults {
            results: vec![
                GuardResult::pass("G1", "Pass"),
                GuardResult::fail("G2", "Fail", "Fix"),
            ],
        };
        assert!(!results.all_passed());
        assert_eq!(results.failure_count(), 1);
        assert_eq!(results.pass_count(), 1);
    }

    #[test]
    fn test_guard_kernel_default_suite() {
        let kernel = GuardKernel::default_suite();
        assert_eq!(kernel.guard_count(), 7);
    }
}
