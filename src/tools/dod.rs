//! Definition of Done Validation MCP Tool
//!
//! Validates Definition of Done: 15 checks across 5 categories.
//! Returns verdict with evidence bundle for deployment readiness assessment.
//!
//! ## 15 DoD Checks (5 Categories)
//! - Workspace: Git status, uncommitted changes, branch check
//! - Build: Compilation success, warnings, dependency audit
//! - Tests: Unit/integration pass, coverage thresholds
//! - Ggen: Ontology validation, template syntax, SPARQL safety, receipt verification
//! - Safety: Input validation, SPARQL injection prevention, poka-yoke completeness

use crate::audit::integration::audit_tool;
use crate::dod::check::CheckRegistry;
use crate::dod::executor::CheckExecutor;
use crate::dod::profile::DodProfile;
use crate::dod::remediation::RemediationGenerator;
use crate::dod::scoring::Scorer;
use crate::dod::types::{CheckContext, DodCheckResult, CheckStatus};
use crate::dod::verdict::VerdictRenderer;
use crate::state::AppState;
use crate::validation::validate_path_safe;
use anyhow::{Context, Result, anyhow};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// =============================================================================
// Public API
// =============================================================================

/// Validate Definition of Done
pub async fn validate_definition_of_done(
    _state: Arc<AppState>,
    params: ValidateDefinitionOfDoneParams,
) -> Result<ValidateDefinitionOfDoneResponse> {
    let _span = audit_tool("validate_definition_of_done", &params);

    // Validate workspace path if provided
    if let Some(ref ws_path) = params.workspace_path {
        validate_path_safe(ws_path)?;
    }

    // Execute validation
    DodValidator::validate(params).await
}

// =============================================================================
// Parameters & Response
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidateDefinitionOfDoneParams {
    /// Profile name (default: "comprehensive")
    /// Options: "minimal", "standard", "comprehensive"
    #[serde(default = "default_profile")]
    pub profile: String,

    /// Workspace path (defaults to cwd)
    #[serde(default)]
    pub workspace_path: Option<String>,

    /// Include remediation suggestions (default: true)
    #[serde(default = "default_true")]
    pub include_remediation: bool,

    /// Include detailed evidence (default: true)
    #[serde(default = "default_true")]
    pub include_evidence: bool,

    /// Fail fast on first error (default: false)
    #[serde(default)]
    pub fail_fast: bool,
}

fn default_profile() -> String {
    "comprehensive".to_string()
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ValidateDefinitionOfDoneResponse {
    /// Overall validation result
    pub ready_for_deployment: bool,

    /// Deployment verdict (READY, PENDING, BLOCKED)
    pub verdict: String,

    /// Overall confidence score (0-100)
    pub confidence_score: u8,

    /// Individual check results
    pub checks: Vec<CheckResult>,

    /// Summary statistics
    pub summary: ValidationSummary,

    /// Remediation suggestions (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<Vec<RemediationSuggestion>>,

    /// Verdict narrative
    pub narrative: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CheckResult {
    /// Check ID
    pub id: String,

    /// Check category (workspace/build/tests/ggen/safety)
    pub category: String,

    /// Check status (Pass/Fail/Warning/Skipped/Error)
    pub status: String,

    /// Check message
    pub message: String,

    /// Execution time (ms)
    pub duration_ms: u64,

    /// Detailed evidence (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ValidationSummary {
    /// Total checks run
    pub total_checks: usize,

    /// Checks passed
    pub passed: usize,

    /// Checks failed
    pub failed: usize,

    /// Checks with warnings
    pub warnings: usize,

    /// Checks skipped
    pub skipped: usize,

    /// Checks with errors
    pub errors: usize,

    /// Total execution time (ms)
    pub total_duration_ms: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemediationSuggestion {
    /// Check ID this remediation applies to
    pub check_id: String,

    /// Priority (Critical/High/Medium/Low)
    pub priority: String,

    /// Suggested action
    pub action: String,

    /// Rationale
    pub rationale: String,

    /// Automation script (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automation_script: Option<String>,
}

// =============================================================================
// DoD Validator
// =============================================================================

pub struct DodValidator;

impl DodValidator {
    /// Validate Definition of Done
    pub async fn validate(
        params: ValidateDefinitionOfDoneParams,
    ) -> Result<ValidateDefinitionOfDoneResponse> {
        let start = std::time::Instant::now();

        // 1. Load profile
        let profile = Self::load_profile(&params.profile)?;

        // 2. Build check registry
        let registry = CheckRegistry::default_registry();

        // 3. Create executor
        let executor = CheckExecutor::new(registry, profile);

        // 4. Create check context
        let workspace_path = params
            .workspace_path
            .clone()
            .unwrap_or_else(|| std::env::current_dir()
                .expect("Failed to get current directory")
                .to_string_lossy()
                .to_string());

        let context = CheckContext::new(&workspace_path)?;

        // 5. Execute checks
        let check_results = executor
            .execute_all(&context)
            .await
            .context("Failed to execute DoD checks")?;

        let total_duration_ms = start.elapsed().as_millis() as u64;

        // 6. Calculate score
        let scorer = Scorer::new();
        let score = scorer.calculate_score(&check_results);

        // 7. Generate verdict
        let verdict_renderer = VerdictRenderer;
        let verdict = verdict_renderer.render_verdict(&check_results, score);

        // 8. Generate remediation (if requested)
        let remediation = if params.include_remediation {
            let generator = RemediationGenerator;
            let suggestions = generator.generate_remediation(&check_results);
            Some(
                suggestions
                    .into_iter()
                    .map(|s| RemediationSuggestion {
                        check_id: s.check_id,
                        priority: format!("{:?}", s.priority),
                        action: s.action,
                        rationale: s.rationale,
                        automation_script: s.automation_script,
                    })
                    .collect(),
            )
        } else {
            None
        };

        // 9. Build check results
        let checks: Vec<CheckResult> = check_results
            .iter()
            .map(|r| CheckResult {
                id: r.check_id.clone(),
                category: r.category.clone(),
                status: Self::format_status(&r.status),
                message: r.message.clone(),
                duration_ms: r.duration_ms,
                evidence: if params.include_evidence {
                    r.evidence.clone()
                } else {
                    None
                },
            })
            .collect();

        // 10. Calculate summary
        let summary = Self::calculate_summary(&check_results, total_duration_ms);

        // 11. Generate narrative
        let narrative = verdict_renderer.render_narrative(&check_results, score, &verdict);

        Ok(ValidateDefinitionOfDoneResponse {
            ready_for_deployment: verdict == "READY",
            verdict: verdict.to_string(),
            confidence_score: score,
            checks,
            summary,
            remediation,
            narrative,
        })
    }

    /// Load profile by name
    fn load_profile(name: &str) -> Result<DodProfile> {
        match name {
            "minimal" => Ok(DodProfile::minimal()),
            "standard" => Ok(DodProfile::standard()),
            "comprehensive" => Ok(DodProfile::comprehensive()),
            _ => Err(anyhow!("Unknown profile: {}. Valid options: minimal, standard, comprehensive", name)),
        }
    }

    /// Format check status for API
    fn format_status(status: &CheckStatus) -> String {
        match status {
            CheckStatus::Pass => "Pass".to_string(),
            CheckStatus::Fail => "Fail".to_string(),
            CheckStatus::Warning => "Warning".to_string(),
            CheckStatus::Skipped => "Skipped".to_string(),
            CheckStatus::Error => "Error".to_string(),
        }
    }

    /// Calculate validation summary
    fn calculate_summary(results: &[DodCheckResult], total_duration_ms: u64) -> ValidationSummary {
        let total_checks = results.len();
        let passed = results
            .iter()
            .filter(|r| matches!(r.status, CheckStatus::Pass))
            .count();
        let failed = results
            .iter()
            .filter(|r| matches!(r.status, CheckStatus::Fail))
            .count();
        let warnings = results
            .iter()
            .filter(|r| matches!(r.status, CheckStatus::Warning))
            .count();
        let skipped = results
            .iter()
            .filter(|r| matches!(r.status, CheckStatus::Skipped))
            .count();
        let errors = results
            .iter()
            .filter(|r| matches!(r.status, CheckStatus::Error))
            .count();

        ValidationSummary {
            total_checks,
            passed,
            failed,
            warnings,
            skipped,
            errors,
            total_duration_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validate_minimal_profile() {
        let params = ValidateDefinitionOfDoneParams {
            profile: "minimal".to_string(),
            workspace_path: None,
            include_remediation: true,
            include_evidence: true,
            fail_fast: false,
        };

        let result = DodValidator::validate(params).await;
        assert!(result.is_ok(), "Validation should succeed: {:?}", result);

        let response = result.unwrap();
        assert!(!response.checks.is_empty(), "Should have check results");
        assert!(response.confidence_score <= 100, "Score should be valid");
        assert!(
            response.summary.total_checks > 0,
            "Should have executed checks"
        );
    }

    #[tokio::test]
    async fn test_validate_standard_profile() {
        let params = ValidateDefinitionOfDoneParams {
            profile: "standard".to_string(),
            workspace_path: None,
            include_remediation: true,
            include_evidence: false,
            fail_fast: false,
        };

        let result = DodValidator::validate(params).await;
        assert!(result.is_ok(), "Validation should succeed");

        let response = result.unwrap();
        assert!(response.checks.iter().all(|c| c.evidence.is_none()));
    }

    #[tokio::test]
    async fn test_validate_comprehensive_profile() {
        let params = ValidateDefinitionOfDoneParams {
            profile: "comprehensive".to_string(),
            workspace_path: None,
            include_remediation: false,
            include_evidence: true,
            fail_fast: false,
        };

        let result = DodValidator::validate(params).await;
        assert!(result.is_ok(), "Validation should succeed");

        let response = result.unwrap();
        assert!(response.remediation.is_none());
        assert!(response.summary.total_checks >= 10, "Comprehensive should run many checks");
    }

    #[tokio::test]
    async fn test_validate_unknown_profile() {
        let params = ValidateDefinitionOfDoneParams {
            profile: "unknown_profile".to_string(),
            workspace_path: None,
            include_remediation: true,
            include_evidence: true,
            fail_fast: false,
        };

        let result = DodValidator::validate(params).await;
        assert!(result.is_err(), "Should reject unknown profile");
        assert!(result.unwrap_err().to_string().contains("Unknown profile"));
    }

    #[test]
    fn test_format_status() {
        assert_eq!(DodValidator::format_status(&CheckStatus::Pass), "Pass");
        assert_eq!(DodValidator::format_status(&CheckStatus::Fail), "Fail");
        assert_eq!(DodValidator::format_status(&CheckStatus::Warning), "Warning");
        assert_eq!(DodValidator::format_status(&CheckStatus::Skipped), "Skipped");
        assert_eq!(DodValidator::format_status(&CheckStatus::Error), "Error");
    }

    #[test]
    fn test_calculate_summary() {
        let results = vec![
            DodCheckResult {
                check_id: "test1".to_string(),
                category: "workspace".to_string(),
                status: CheckStatus::Pass,
                message: "OK".to_string(),
                duration_ms: 10,
                evidence: None,
            },
            DodCheckResult {
                check_id: "test2".to_string(),
                category: "build".to_string(),
                status: CheckStatus::Fail,
                message: "Failed".to_string(),
                duration_ms: 20,
                evidence: None,
            },
            DodCheckResult {
                check_id: "test3".to_string(),
                category: "tests".to_string(),
                status: CheckStatus::Warning,
                message: "Warning".to_string(),
                duration_ms: 5,
                evidence: None,
            },
        ];

        let summary = DodValidator::calculate_summary(&results, 100);

        assert_eq!(summary.total_checks, 3);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.warnings, 1);
        assert_eq!(summary.skipped, 0);
        assert_eq!(summary.errors, 0);
        assert_eq!(summary.total_duration_ms, 100);
    }
}
