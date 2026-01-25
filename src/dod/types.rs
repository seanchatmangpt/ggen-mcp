use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Severity level for check results
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
pub enum CheckSeverity {
    Info,
    Warning,
    Fatal,
}

/// Status of a single check
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum CheckStatus {
    Pass,
    Fail,
    Warn,
    Skip,
}

/// Result of a single DoD check
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DodCheckResult {
    pub id: String,
    pub category: CheckCategory,
    pub status: CheckStatus,
    pub severity: CheckSeverity,
    pub message: String,
    pub evidence: Vec<Evidence>,
    pub remediation: Vec<String>,
    pub duration_ms: u64,
    pub check_hash: String,
}

/// Check category (A-H from PRD)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum CheckCategory {
    WorkspaceIntegrity,  // Category A
    IntentAlignment,     // Category B (WHY)
    ToolRegistry,        // Category C (WHAT)
    BuildCorrectness,    // Category D
    TestTruth,           // Category E
    GgenPipeline,        // Category F
    SafetyInvariants,    // Category G
    DeploymentReadiness, // Category H
}

/// Evidence for a check result
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Evidence {
    pub kind: EvidenceKind,
    pub content: String,
    pub file_path: Option<PathBuf>,
    pub line_number: Option<usize>,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum EvidenceKind {
    FileContent,
    CommandOutput,
    LogEntry,
    Metric,
    Hash,
}

/// Category score
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CategoryScore {
    pub category: CheckCategory,
    pub score: f64,  // 0.0 to 100.0
    pub weight: f64, // 0.0 to 1.0
    pub checks_passed: usize,
    pub checks_failed: usize,
    pub checks_warned: usize,
    pub checks_skipped: usize,
}

/// Overall DoD validation result
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DodValidationResult {
    pub verdict: OverallVerdict,
    pub readiness_score: f64,
    pub profile: String,
    pub mode: ValidationMode,
    pub summary: ValidationSummary,
    pub category_scores: HashMap<CheckCategory, CategoryScore>,
    pub check_results: Vec<DodCheckResult>,
    pub artifacts: ArtifactPaths,
    pub duration_ms: u64,
}

/// Overall verdict: Ready or NotReady
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum OverallVerdict {
    Ready,    // PASS - ship-ready
    NotReady, // FAIL - not ship-ready
}

/// Validation mode determines thoroughness
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ValidationMode {
    Fast,
    Strict,
    Paranoid,
}

/// Summary of validation results
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidationSummary {
    pub checks_total: usize,
    pub checks_passed: usize,
    pub checks_failed: usize,
    pub checks_warned: usize,
    pub checks_skipped: usize,
}

/// Paths to generated artifacts
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ArtifactPaths {
    pub receipt_path: PathBuf,
    pub report_path: PathBuf,
    pub bundle_path: Option<PathBuf>,
}
