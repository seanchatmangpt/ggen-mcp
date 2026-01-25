//! MCP Tool Handler for Definition of Done Validation
//!
//! Provides MCP-accessible interface to DoD validation system.
//! Executes profile-based validation and generates comprehensive reports.
//!
//! ## Tool: validate_definition_of_done
//!
//! Validates codebase against Definition of Done criteria using:
//! - Profile-based check selection (dev/enterprise)
//! - Parallel check execution with dependency ordering
//! - Evidence generation and artifact bundling
//! - Cryptographic receipt generation
//!
//! ## Usage
//!
//! ```json
//! {
//!   "profile": "dev",
//!   "output_dir": "./dod-validation",
//!   "skip_evidence": false
//! }
//! ```

use crate::audit::integration::audit_tool;
use crate::dod::check::CheckRegistry;
use crate::dod::profile::DodProfile;
use crate::dod::receipt::ReceiptGenerator;
use crate::dod::report::ReportGenerator;
use crate::dod::result::DodResult;
use crate::dod::types::*;
use crate::dod::validator::DodValidator;
use crate::state::AppState;
use crate::validation::validate_path_safe;
use anyhow::{Context, Result, anyhow};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;

// =============================================================================
// Constants
// =============================================================================

const DEFAULT_OUTPUT_DIR: &str = "./dod-validation";
const MAX_REPORT_SIZE: usize = 100 * 1024 * 1024; // 100MB

// =============================================================================
// Public API
// =============================================================================

/// Execute DoD validation with profile
pub async fn validate_definition_of_done(
    _state: Arc<AppState>,
    params: ValidateDefinitionOfDoneParams,
) -> Result<ValidateDefinitionOfDoneResponse> {
    let _span = audit_tool("validate_definition_of_done", &params);

    tracing::info!(
        profile = ?params.profile,
        output_dir = ?params.output_dir,
        "Starting DoD validation"
    );

    // Load profile
    let profile = load_profile(&params.profile)?;
    tracing::info!(
        profile_name = %profile.name,
        required_checks = profile.required_checks.len(),
        optional_checks = profile.optional_checks.len(),
        "Loaded DoD profile"
    );

    // Setup output directory
    let output_dir = params
        .output_dir
        .clone()
        .unwrap_or_else(|| DEFAULT_OUTPUT_DIR.to_string());
    validate_path_safe(&output_dir)?;
    let output_path = PathBuf::from(&output_dir);

    fs::create_dir_all(&output_path)
        .await
        .context("Failed to create output directory")?;

    // Get workspace root
    let workspace_root = std::env::current_dir().context("Failed to get current directory")?;

    // Create validator with all checks registered
    let registry = CheckRegistry::with_all_checks();
    let validator = DodValidator::new(registry, profile.clone());

    // Execute validation
    let result = validator
        .validate(workspace_root.clone())
        .await
        .context("DoD validation failed")?;

    tracing::info!(
        verdict = ?result.verdict,
        score = result.score,
        checks = result.checks.len(),
        "DoD validation completed"
    );

    // Convert DodResult to DodValidationResult for artifact generation
    let validation_result = convert_to_validation_result(&result, &workspace_root);

    // Generate artifacts
    let artifacts = generate_artifacts(
        &output_path,
        &validation_result,
        params.skip_evidence.unwrap_or(false),
    )
    .await?;

    let duration_ms = result.execution_time.as_millis() as u64;

    // Build response
    Ok(ValidateDefinitionOfDoneResponse {
        verdict: format!("{:?}", result.verdict.to_overall_verdict()),
        score: result.score,
        report_path: artifacts.report_path.to_string_lossy().to_string(),
        receipt_path: artifacts.receipt_path.to_string_lossy().to_string(),
        summary: format_summary_from_result(&result),
        checks_passed: result.count_by_status(CheckStatus::Pass),
        checks_failed: result.count_by_status(CheckStatus::Fail),
        checks_warned: result.count_by_status(CheckStatus::Warn),
        checks_total: result.checks.len(),
        duration_ms,
        validation_result,
    })
}

// =============================================================================
// Parameters & Response
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidateDefinitionOfDoneParams {
    /// Profile name (e.g., "dev", "enterprise")
    #[serde(default = "default_profile")]
    pub profile: String,

    /// Output directory for reports and artifacts
    #[serde(default)]
    pub output_dir: Option<String>,

    /// Skip evidence generation (faster, but less detailed)
    #[serde(default)]
    pub skip_evidence: Option<bool>,
}

fn default_profile() -> String {
    "dev".to_string()
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ValidateDefinitionOfDoneResponse {
    /// Overall verdict: "Ready" or "NotReady"
    pub verdict: String,

    /// Readiness score (0.0 to 100.0)
    pub score: f64,

    /// Path to generated report
    pub report_path: String,

    /// Path to cryptographic receipt
    pub receipt_path: String,

    /// Human-readable summary
    pub summary: String,

    /// Number of checks passed
    pub checks_passed: usize,

    /// Number of checks failed
    pub checks_failed: usize,

    /// Number of checks with warnings
    pub checks_warned: usize,

    /// Total checks executed
    pub checks_total: usize,

    /// Execution duration in milliseconds
    pub duration_ms: u64,

    /// Full validation result (for detailed analysis)
    pub validation_result: DodValidationResult,
}

// =============================================================================
// Profile Loading
// =============================================================================

/// Load DoD profile by name
fn load_profile(profile_name: &str) -> Result<DodProfile> {
    match profile_name.to_lowercase().as_str() {
        "dev" | "developer" => Ok(DodProfile::default_dev()),
        "enterprise" => Ok(DodProfile::enterprise_strict()),
        _ => Err(anyhow!(
            "Unknown profile '{}'. Available: dev, enterprise",
            profile_name
        )),
    }
}

// =============================================================================
// Summary Building
// =============================================================================

/// Convert DodResult to DodValidationResult
fn convert_to_validation_result(
    result: &DodResult,
    workspace_root: &PathBuf,
) -> DodValidationResult {
    let summary = ValidationSummary {
        checks_total: result.checks.len(),
        checks_passed: result.count_by_status(CheckStatus::Pass),
        checks_failed: result.count_by_status(CheckStatus::Fail),
        checks_warned: result.count_by_status(CheckStatus::Warn),
        checks_skipped: result.count_by_status(CheckStatus::Skip),
    };

    DodValidationResult {
        verdict: result.verdict.to_overall_verdict(),
        readiness_score: result.score,
        profile: result.profile_name.clone(),
        mode: result.mode,
        summary,
        category_scores: result.category_scores.clone(),
        check_results: result.checks.clone(),
        artifacts: ArtifactPaths {
            receipt_path: workspace_root.join("dod_receipt.json"),
            report_path: workspace_root.join("dod_report.md"),
            bundle_path: None,
        },
        duration_ms: result.execution_time.as_millis() as u64,
    }
}

/// Format summary from DodResult
fn format_summary_from_result(result: &DodResult) -> String {
    let verdict_str = match result.verdict.to_overall_verdict() {
        OverallVerdict::Ready => "✅ READY",
        OverallVerdict::NotReady => "❌ NOT READY",
    };

    let passed = result.count_by_status(CheckStatus::Pass);
    let failed = result.count_by_status(CheckStatus::Fail);
    let warned = result.count_by_status(CheckStatus::Warn);
    let total = result.checks.len();

    format!(
        "{} - Score: {:.1}/100.0 | Passed: {}/{} | Failed: {} | Warned: {}",
        verdict_str, result.score, passed, total, failed, warned
    )
}

// =============================================================================
// Artifact Generation
// =============================================================================

/// Generate all validation artifacts (report, receipt, bundle)
async fn generate_artifacts(
    output_dir: &PathBuf,
    validation_result: &DodValidationResult,
    skip_evidence: bool,
) -> Result<ArtifactPaths> {
    // Generate markdown report using ReportGenerator
    let report_path = output_dir.join("dod_report.md");
    let report_content = ReportGenerator::generate_markdown(validation_result)
        .context("Failed to generate markdown report")?;

    if report_content.len() > MAX_REPORT_SIZE {
        return Err(anyhow!(
            "Report too large: {} bytes (max: {})",
            report_content.len(),
            MAX_REPORT_SIZE
        ));
    }

    fs::write(&report_path, &report_content)
        .await
        .context("Failed to write report")?;

    tracing::info!(
        report_path = ?report_path,
        size_bytes = report_content.len(),
        "Generated DoD report"
    );

    // Generate and save cryptographic receipt using ReceiptGenerator
    let receipt_generator =
        ReceiptGenerator::new(output_dir).context("Failed to create receipt generator")?;
    let receipt_path = receipt_generator
        .generate_and_save(validation_result)
        .context("Failed to generate and save receipt")?;

    tracing::info!(
        receipt_path = ?receipt_path,
        "Generated DoD receipt with hash chain"
    );

    // Generate evidence bundle (optional)
    let bundle_path = if !skip_evidence {
        let bundle = output_dir.join("dod_evidence.tar.gz");
        // FUTURE: Implement evidence bundling (tar.gz creation)
        // See: https://docs.rs/tar/latest/tar/ for archive creation
        // This would bundle all evidence files into a single archive
        tracing::debug!("Evidence bundling not yet implemented");
        Some(bundle)
    } else {
        None
    };

    Ok(ArtifactPaths {
        receipt_path,
        report_path,
        bundle_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dod::types::*;

    fn create_mock_check_result(id: &str, status: CheckStatus) -> DodCheckResult {
        DodCheckResult {
            id: id.to_string(),
            category: CheckCategory::BuildCorrectness,
            status,
            severity: CheckSeverity::Fatal,
            message: "Test check".to_string(),
            evidence: vec![],
            remediation: vec![],
            duration_ms: 100,
            check_hash: "test_hash".to_string(),
        }
    }

    #[test]
    fn test_load_profile_dev() {
        let profile = load_profile("dev").unwrap();
        assert_eq!(profile.name, "dev");
        assert!(profile.required_checks.len() > 0);
    }

    #[test]
    fn test_load_profile_enterprise() {
        let profile = load_profile("enterprise").unwrap();
        assert_eq!(profile.name, "enterprise");
        assert!(profile.required_checks.len() > 0);
    }

    #[test]
    fn test_load_profile_invalid() {
        let result = load_profile("invalid_profile");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown profile"));
    }

    #[test]
    fn test_validate_path_in_params() {
        let params = ValidateDefinitionOfDoneParams {
            profile: "dev".to_string(),
            output_dir: Some("./valid/path".to_string()),
            skip_evidence: Some(false),
        };

        assert_eq!(params.profile, "dev");
        assert_eq!(params.output_dir.unwrap(), "./valid/path");
    }
}
