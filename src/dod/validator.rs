//! DoD Validator Orchestrator
//!
//! Main orchestrator that aggregates check results and computes final verdict.
//! Uses CheckExecutor from phase 4, applies profile weights, returns final result.

use super::check::{CheckContext, CheckRegistry};
use super::executor::CheckExecutor;
use super::profile::DodProfile;
use super::result::{DodResult, Verdict};
use super::scoring::{compute_category_score, compute_readiness_score};
use super::types::*;
use super::verdict::compute_verdict;
use anyhow::{Context as _, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

/// DoD validator orchestrator
///
/// Coordinates check execution, scoring, and verdict computation.
/// Uses profile configuration to determine which checks run and how they're weighted.
pub struct DodValidator {
    executor: CheckExecutor,
    profile: Arc<DodProfile>,
}

impl DodValidator {
    /// Create new validator with registry and profile
    pub fn new(registry: CheckRegistry, profile: DodProfile) -> Self {
        let profile_arc = Arc::new(profile.clone());
        let executor = CheckExecutor::new(registry, profile);

        Self {
            executor,
            profile: profile_arc,
        }
    }

    /// Create validator with default dev profile
    pub fn with_default_profile(registry: CheckRegistry) -> Self {
        Self::new(registry, DodProfile::default_dev())
    }

    /// Validate workspace and return aggregated result
    ///
    /// # Errors
    /// Returns error if check execution fails or workspace is invalid
    pub async fn validate(&self, workspace_root: PathBuf) -> Result<DodResult> {
        let start = Instant::now();
        let timestamp = Utc::now();

        tracing::info!(
            profile = %self.profile.name,
            workspace = ?workspace_root,
            "Starting DoD validation"
        );

        // Create check context
        let context = self.create_context(workspace_root)?;

        // Execute all checks
        let check_results = self
            .executor
            .execute_all(&context)
            .await
            .context("Failed to execute checks")?;

        if check_results.is_empty() {
            tracing::warn!("No checks executed - validation may be incomplete");
        }

        tracing::info!(
            check_count = check_results.len(),
            "Check execution completed"
        );

        // Compute category scores
        let category_scores = self.compute_all_category_scores(&check_results);

        // Compute weighted readiness score
        let readiness_score = compute_readiness_score(&category_scores);

        // Compute final verdict
        let verdict = self.compute_final_verdict(&check_results, readiness_score);

        let execution_time = start.elapsed();

        tracing::info!(
            verdict = ?verdict,
            score = readiness_score,
            duration_ms = execution_time.as_millis(),
            "DoD validation completed"
        );

        Ok(DodResult {
            verdict,
            score: readiness_score,
            max_score: 100.0,
            checks: check_results,
            category_scores,
            execution_time,
            timestamp,
            profile_name: self.profile.name.clone(),
            mode: context.mode,
        })
    }

    /// Validate with custom validation mode
    pub async fn validate_with_mode(
        &self,
        workspace_root: PathBuf,
        mode: ValidationMode,
    ) -> Result<DodResult> {
        let start = Instant::now();
        let timestamp = Utc::now();

        let context = CheckContext {
            workspace_root,
            mode,
            timeout_ms: self.profile.timeouts_ms.default,
        };

        let check_results = self.executor.execute_all(&context).await?;
        let category_scores = self.compute_all_category_scores(&check_results);
        let readiness_score = compute_readiness_score(&category_scores);
        let verdict = self.compute_final_verdict(&check_results, readiness_score);

        Ok(DodResult {
            verdict,
            score: readiness_score,
            max_score: 100.0,
            checks: check_results,
            category_scores,
            execution_time: start.elapsed(),
            timestamp,
            profile_name: self.profile.name.clone(),
            mode,
        })
    }

    /// Execute single check by ID
    pub async fn validate_single(
        &self,
        workspace_root: PathBuf,
        check_id: &str,
    ) -> Result<DodCheckResult> {
        let context = self.create_context(workspace_root)?;
        self.executor
            .execute_one(check_id, &context)
            .await
            .context(format!("Failed to execute check: {}", check_id))
    }

    /// Get profile being used
    pub fn profile(&self) -> &DodProfile {
        &self.profile
    }

    /// Create check context from workspace
    fn create_context(&self, workspace_root: PathBuf) -> Result<CheckContext> {
        // Validate workspace exists
        if !workspace_root.exists() {
            anyhow::bail!("Workspace root does not exist: {:?}", workspace_root);
        }

        if !workspace_root.is_dir() {
            anyhow::bail!("Workspace root is not a directory: {:?}", workspace_root);
        }

        Ok(CheckContext {
            workspace_root,
            mode: ValidationMode::Fast,
            timeout_ms: self.profile.timeouts_ms.default,
        })
    }

    /// Compute scores for all categories
    fn compute_all_category_scores(
        &self,
        check_results: &[DodCheckResult],
    ) -> HashMap<CheckCategory, CategoryScore> {
        let mut scores = HashMap::new();

        // Compute score for each category that has checks
        for category in [
            CheckCategory::WorkspaceIntegrity,
            CheckCategory::IntentAlignment,
            CheckCategory::ToolRegistry,
            CheckCategory::BuildCorrectness,
            CheckCategory::TestTruth,
            CheckCategory::GgenPipeline,
            CheckCategory::SafetyInvariants,
            CheckCategory::DeploymentReadiness,
        ] {
            let category_score = compute_category_score(category, check_results);
            scores.insert(category, category_score);
        }

        scores
    }

    /// Compute final verdict using severity-first logic + threshold
    fn compute_final_verdict(
        &self,
        check_results: &[DodCheckResult],
        readiness_score: f64,
    ) -> Verdict {
        // First check: any fatal failures?
        let overall_verdict = compute_verdict(check_results);

        match overall_verdict {
            OverallVerdict::NotReady => {
                // Fatal failure present
                Verdict::Fail
            }
            OverallVerdict::Ready => {
                // No fatal failures - check threshold
                if readiness_score >= self.profile.thresholds.min_readiness_score {
                    Verdict::Pass
                } else {
                    Verdict::PartialPass
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dod::checks::DodCheck;
    use async_trait::async_trait;

    // Mock check for testing
    struct MockCheck {
        id: String,
        category: CheckCategory,
        severity: CheckSeverity,
        status: CheckStatus,
    }

    #[async_trait]
    impl DodCheck for MockCheck {
        fn id(&self) -> &str {
            &self.id
        }

        fn name(&self) -> &str {
            "Mock Check"
        }

        fn category(&self) -> CheckCategory {
            self.category
        }

        fn severity(&self) -> CheckSeverity {
            self.severity
        }

        fn dependencies(&self) -> Vec<String> {
            vec![]
        }

        async fn execute(&self, _context: &CheckContext) -> Result<DodCheckResult> {
            Ok(DodCheckResult {
                id: self.id.clone(),
                category: self.category,
                status: self.status.clone(),
                severity: self.severity,
                message: format!("Mock check {}", self.id),
                evidence: vec![],
                remediation: vec![],
                duration_ms: 1,
                check_hash: "mock".to_string(),
            })
        }
    }

    fn create_temp_workspace() -> PathBuf {
        std::env::current_dir().unwrap()
    }

    #[tokio::test]
    async fn validator_all_pass_scenario() {
        let mut registry = CheckRegistry::new();

        // Add passing checks
        registry.register(Box::new(MockCheck {
            id: "BUILD_CHECK".to_string(),
            category: CheckCategory::BuildCorrectness,
            severity: CheckSeverity::Fatal,
            status: CheckStatus::Pass,
        }));
        registry.register(Box::new(MockCheck {
            id: "TEST_UNIT".to_string(),
            category: CheckCategory::TestTruth,
            severity: CheckSeverity::Fatal,
            status: CheckStatus::Pass,
        }));

        let validator = DodValidator::with_default_profile(registry);
        let workspace = create_temp_workspace();

        let result = validator.validate(workspace).await.unwrap();

        assert_eq!(result.verdict, Verdict::Pass);
        assert_eq!(result.score, 100.0);
        assert!(result.failed_checks().is_empty());
        assert_eq!(result.summary().passed, 2);
    }

    #[tokio::test]
    async fn validator_fatal_failure_scenario() {
        let mut registry = CheckRegistry::new();

        registry.register(Box::new(MockCheck {
            id: "BUILD_CHECK".to_string(),
            category: CheckCategory::BuildCorrectness,
            severity: CheckSeverity::Fatal,
            status: CheckStatus::Fail,
        }));
        registry.register(Box::new(MockCheck {
            id: "TEST_UNIT".to_string(),
            category: CheckCategory::TestTruth,
            severity: CheckSeverity::Fatal,
            status: CheckStatus::Pass,
        }));

        let validator = DodValidator::with_default_profile(registry);
        let workspace = create_temp_workspace();

        let result = validator.validate(workspace).await.unwrap();

        assert_eq!(result.verdict, Verdict::Fail);
        assert_eq!(result.failed_checks().len(), 1);
        assert_eq!(result.fatal_failures().len(), 1);
        assert_eq!(result.fatal_failures()[0].id, "BUILD_CHECK");
    }

    #[tokio::test]
    async fn validator_partial_pass_scenario() {
        let mut registry = CheckRegistry::new();

        // All fatal checks pass, but score below threshold
        registry.register(Box::new(MockCheck {
            id: "BUILD_CHECK".to_string(),
            category: CheckCategory::BuildCorrectness,
            severity: CheckSeverity::Fatal,
            status: CheckStatus::Pass,
        }));
        registry.register(Box::new(MockCheck {
            id: "OPTIONAL_CHECK".to_string(),
            category: CheckCategory::TestTruth,
            severity: CheckSeverity::Warning,
            status: CheckStatus::Fail,
        }));

        let mut profile = DodProfile::default_dev();
        // Set high threshold that won't be met
        profile.thresholds.min_readiness_score = 95.0;
        profile.required_checks = vec!["BUILD_CHECK".to_string()];
        profile.optional_checks = vec!["OPTIONAL_CHECK".to_string()];

        let validator = DodValidator::new(registry, profile);
        let workspace = create_temp_workspace();

        let result = validator.validate(workspace).await.unwrap();

        // No fatal failures, but score < threshold
        assert!(result.fatal_failures().is_empty());
        // With 1 pass and 1 fail, we get 50% in different categories
        // Weighted score depends on category weights, should be low
        assert!(result.score < 95.0);
        // Should be PartialPass (no fatal fails, but below threshold)
        assert_eq!(result.verdict, Verdict::PartialPass);
    }

    #[tokio::test]
    async fn validator_computes_category_scores() {
        let mut registry = CheckRegistry::new();

        // Build checks
        registry.register(Box::new(MockCheck {
            id: "BUILD_CHECK_1".to_string(),
            category: CheckCategory::BuildCorrectness,
            severity: CheckSeverity::Fatal,
            status: CheckStatus::Pass,
        }));
        registry.register(Box::new(MockCheck {
            id: "BUILD_CHECK_2".to_string(),
            category: CheckCategory::BuildCorrectness,
            severity: CheckSeverity::Fatal,
            status: CheckStatus::Pass,
        }));

        // Test checks
        registry.register(Box::new(MockCheck {
            id: "TEST_UNIT".to_string(),
            category: CheckCategory::TestTruth,
            severity: CheckSeverity::Fatal,
            status: CheckStatus::Pass,
        }));

        let validator = DodValidator::with_default_profile(registry);
        let workspace = create_temp_workspace();

        let result = validator.validate(workspace).await.unwrap();

        // Should have category scores
        assert!(result
            .category_scores
            .contains_key(&CheckCategory::BuildCorrectness));
        assert!(result
            .category_scores
            .contains_key(&CheckCategory::TestTruth));

        let build_score = &result.category_scores[&CheckCategory::BuildCorrectness];
        assert_eq!(build_score.checks_passed, 2);
        assert_eq!(build_score.score, 100.0);

        let test_score = &result.category_scores[&CheckCategory::TestTruth];
        assert_eq!(test_score.checks_passed, 1);
        assert_eq!(test_score.score, 100.0);
    }

    #[tokio::test]
    async fn validator_respects_profile_weights() {
        let mut registry = CheckRegistry::new();

        // Build fails (weight 0.25)
        registry.register(Box::new(MockCheck {
            id: "BUILD_CHECK".to_string(),
            category: CheckCategory::BuildCorrectness,
            severity: CheckSeverity::Warning,
            status: CheckStatus::Fail,
        }));

        // Test passes (weight 0.25)
        registry.register(Box::new(MockCheck {
            id: "TEST_UNIT".to_string(),
            category: CheckCategory::TestTruth,
            severity: CheckSeverity::Warning,
            status: CheckStatus::Pass,
        }));

        let mut profile = DodProfile::default_dev();
        profile.required_checks = vec!["BUILD_CHECK".to_string(), "TEST_UNIT".to_string()];

        let validator = DodValidator::new(registry, profile);
        let workspace = create_temp_workspace();

        let result = validator.validate(workspace).await.unwrap();

        // Build: 0/1 = 0% * 0.25 = 0
        // Test: 1/1 = 100% * 0.25 = 25
        // Total = 25.0
        assert_eq!(result.score, 25.0);
    }

    #[tokio::test]
    async fn validator_with_validation_modes() {
        let mut registry = CheckRegistry::new();

        registry.register(Box::new(MockCheck {
            id: "BUILD_CHECK".to_string(),
            category: CheckCategory::BuildCorrectness,
            severity: CheckSeverity::Fatal,
            status: CheckStatus::Pass,
        }));

        let validator = DodValidator::with_default_profile(registry);
        let workspace = create_temp_workspace();

        for mode in [
            ValidationMode::Fast,
            ValidationMode::Strict,
            ValidationMode::Paranoid,
        ] {
            let result = validator
                .validate_with_mode(workspace.clone(), mode)
                .await
                .unwrap();
            assert_eq!(result.mode, mode);
        }
    }

    #[tokio::test]
    async fn validator_single_check_execution() {
        let mut registry = CheckRegistry::new();

        registry.register(Box::new(MockCheck {
            id: "BUILD_CHECK".to_string(),
            category: CheckCategory::BuildCorrectness,
            severity: CheckSeverity::Fatal,
            status: CheckStatus::Pass,
        }));

        let validator = DodValidator::with_default_profile(registry);
        let workspace = create_temp_workspace();

        let result = validator
            .validate_single(workspace, "BUILD_CHECK")
            .await
            .unwrap();

        assert_eq!(result.id, "BUILD_CHECK");
        assert_eq!(result.status, CheckStatus::Pass);
    }

    #[tokio::test]
    async fn validator_invalid_workspace_fails() {
        let registry = CheckRegistry::new();
        let validator = DodValidator::with_default_profile(registry);

        let invalid_workspace = PathBuf::from("/nonexistent/path");
        let result = validator.validate(invalid_workspace).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("does not exist"));
    }

    #[tokio::test]
    async fn validator_tracks_execution_time() {
        let mut registry = CheckRegistry::new();

        registry.register(Box::new(MockCheck {
            id: "BUILD_CHECK".to_string(),
            category: CheckCategory::BuildCorrectness,
            severity: CheckSeverity::Fatal,
            status: CheckStatus::Pass,
        }));

        let validator = DodValidator::with_default_profile(registry);
        let workspace = create_temp_workspace();

        let result = validator.validate(workspace).await.unwrap();

        assert!(result.execution_time.as_millis() > 0);
        assert!(!result.execution_time_str().is_empty());
    }

    #[tokio::test]
    async fn validator_mixed_severity_failures() {
        let mut registry = CheckRegistry::new();

        // Fatal failure
        registry.register(Box::new(MockCheck {
            id: "FATAL_CHECK".to_string(),
            category: CheckCategory::BuildCorrectness,
            severity: CheckSeverity::Fatal,
            status: CheckStatus::Fail,
        }));

        // Warning failure (should not affect verdict)
        registry.register(Box::new(MockCheck {
            id: "WARNING_CHECK".to_string(),
            category: CheckCategory::TestTruth,
            severity: CheckSeverity::Warning,
            status: CheckStatus::Fail,
        }));

        let validator = DodValidator::with_default_profile(registry);
        let workspace = create_temp_workspace();

        let result = validator.validate(workspace).await.unwrap();

        // Should fail due to fatal failure
        assert_eq!(result.verdict, Verdict::Fail);
        assert_eq!(result.failed_checks().len(), 2);
        assert_eq!(result.fatal_failures().len(), 1);
    }
}
