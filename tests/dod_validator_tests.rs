//! Comprehensive tests for DoD Validator Orchestrator
//!
//! Tests cover:
//! - All-pass scenarios
//! - Failure scenarios (fatal and non-fatal)
//! - Partial-pass scenarios
//! - Scoring algorithm
//! - Category aggregation
//! - Profile weight application
//! - Verdict computation logic

use ggen_mcp::dod::*;
use async_trait::async_trait;
use std::path::PathBuf;

// ============================================================================
// Test Helpers
// ============================================================================

/// Mock check that returns predetermined result
struct MockCheck {
    id: String,
    name: String,
    category: CheckCategory,
    severity: CheckSeverity,
    status: CheckStatus,
    dependencies: Vec<String>,
}

impl MockCheck {
    fn new(
        id: &str,
        category: CheckCategory,
        severity: CheckSeverity,
        status: CheckStatus,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: format!("Mock {}", id),
            category,
            severity,
            status,
            dependencies: vec![],
        }
    }

    fn with_deps(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }
}

#[async_trait]
impl DodCheck for MockCheck {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn category(&self) -> CheckCategory {
        self.category
    }

    fn severity(&self) -> CheckSeverity {
        self.severity
    }

    fn dependencies(&self) -> Vec<String> {
        self.dependencies.clone()
    }

    async fn execute(&self, _context: &CheckContext) -> anyhow::Result<DodCheckResult> {
        Ok(DodCheckResult {
            id: self.id.clone(),
            category: self.category,
            status: self.status.clone(),
            severity: self.severity,
            message: format!("{} executed", self.name),
            evidence: vec![],
            remediation: vec![],
            duration_ms: 10,
            check_hash: format!("hash_{}", self.id),
        })
    }
}

fn test_workspace() -> PathBuf {
    std::env::current_dir().unwrap()
}

// ============================================================================
// All-Pass Scenarios
// ============================================================================

#[tokio::test]
async fn all_pass_perfect_score() {
    let mut registry = CheckRegistry::new();

    // Register all passing checks across categories
    registry.register(Box::new(MockCheck::new(
        "BUILD_FMT",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));
    registry.register(Box::new(MockCheck::new(
        "BUILD_CHECK",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));
    registry.register(Box::new(MockCheck::new(
        "TEST_UNIT",
        CheckCategory::TestTruth,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));
    registry.register(Box::new(MockCheck::new(
        "GGEN_DRY_RUN",
        CheckCategory::GgenPipeline,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));

    let validator = DodValidator::with_default_profile(registry);
    let result = validator.validate(test_workspace()).await.unwrap();

    assert_eq!(result.verdict, Verdict::Pass);
    assert_eq!(result.score, 100.0);
    assert_eq!(result.max_score, 100.0);
    assert_eq!(result.summary().passed, 4);
    assert_eq!(result.summary().failed, 0);
    assert!(result.failed_checks().is_empty());
    assert!(result.fatal_failures().is_empty());
}

#[tokio::test]
async fn all_pass_with_warnings() {
    let mut registry = CheckRegistry::new();

    // Passing checks
    registry.register(Box::new(MockCheck::new(
        "BUILD_CHECK",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));

    // Warning (non-fatal)
    registry.register(Box::new(MockCheck::new(
        "OPTIONAL_CHECK",
        CheckCategory::TestTruth,
        CheckSeverity::Warning,
        CheckStatus::Warn,
    )));

    let validator = DodValidator::with_default_profile(registry);
    let result = validator.validate(test_workspace()).await.unwrap();

    // Should pass despite warning
    assert_eq!(result.verdict, Verdict::Pass);
    assert_eq!(result.summary().warned, 1);
    assert!(result.fatal_failures().is_empty());
    // Score should be slightly reduced by warning
    assert!(result.score < 100.0);
    assert!(result.score > 90.0);
}

#[tokio::test]
async fn all_pass_with_skipped_checks() {
    let mut registry = CheckRegistry::new();

    registry.register(Box::new(MockCheck::new(
        "BUILD_CHECK",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));

    registry.register(Box::new(MockCheck::new(
        "SKIPPED_CHECK",
        CheckCategory::TestTruth,
        CheckSeverity::Info,
        CheckStatus::Skip,
    )));

    let validator = DodValidator::with_default_profile(registry);
    let result = validator.validate(test_workspace()).await.unwrap();

    assert_eq!(result.verdict, Verdict::Pass);
    assert_eq!(result.summary().skipped, 1);
    // Skipped checks don't affect score
    assert_eq!(result.score, 100.0);
}

// ============================================================================
// Failure Scenarios
// ============================================================================

#[tokio::test]
async fn single_fatal_failure() {
    let mut registry = CheckRegistry::new();

    registry.register(Box::new(MockCheck::new(
        "BUILD_CHECK",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Fail,
    )));

    registry.register(Box::new(MockCheck::new(
        "TEST_UNIT",
        CheckCategory::TestTruth,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));

    let validator = DodValidator::with_default_profile(registry);
    let result = validator.validate(test_workspace()).await.unwrap();

    assert_eq!(result.verdict, Verdict::Fail);
    assert_eq!(result.summary().failed, 1);
    assert_eq!(result.fatal_failures().len(), 1);
    assert_eq!(result.fatal_failures()[0].id, "BUILD_CHECK");
}

#[tokio::test]
async fn multiple_fatal_failures() {
    let mut registry = CheckRegistry::new();

    registry.register(Box::new(MockCheck::new(
        "BUILD_CHECK",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Fail,
    )));

    registry.register(Box::new(MockCheck::new(
        "TEST_UNIT",
        CheckCategory::TestTruth,
        CheckSeverity::Fatal,
        CheckStatus::Fail,
    )));

    registry.register(Box::new(MockCheck::new(
        "GGEN_DRY_RUN",
        CheckCategory::GgenPipeline,
        CheckSeverity::Fatal,
        CheckStatus::Fail,
    )));

    let validator = DodValidator::with_default_profile(registry);
    let result = validator.validate(test_workspace()).await.unwrap();

    assert_eq!(result.verdict, Verdict::Fail);
    assert_eq!(result.summary().failed, 3);
    assert_eq!(result.fatal_failures().len(), 3);
    assert!(result.score < 50.0);
}

#[tokio::test]
async fn non_fatal_failures_dont_fail_verdict() {
    let mut registry = CheckRegistry::new();

    // Fatal check passes
    registry.register(Box::new(MockCheck::new(
        "BUILD_CHECK",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));

    // Non-fatal check fails
    registry.register(Box::new(MockCheck::new(
        "OPTIONAL_CHECK",
        CheckCategory::TestTruth,
        CheckSeverity::Warning,
        CheckStatus::Fail,
    )));

    let validator = DodValidator::with_default_profile(registry);
    let result = validator.validate(test_workspace()).await.unwrap();

    // Should pass (or partial) - no fatal failures
    assert_ne!(result.verdict, Verdict::Fail);
    assert!(result.fatal_failures().is_empty());
    assert_eq!(result.summary().failed, 1);
}

#[tokio::test]
async fn all_checks_fail() {
    let mut registry = CheckRegistry::new();

    registry.register(Box::new(MockCheck::new(
        "BUILD_CHECK",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Fail,
    )));

    registry.register(Box::new(MockCheck::new(
        "TEST_UNIT",
        CheckCategory::TestTruth,
        CheckSeverity::Fatal,
        CheckStatus::Fail,
    )));

    let validator = DodValidator::with_default_profile(registry);
    let result = validator.validate(test_workspace()).await.unwrap();

    assert_eq!(result.verdict, Verdict::Fail);
    assert_eq!(result.score, 0.0);
    assert_eq!(result.summary().failed, 2);
    assert_eq!(result.summary().passed, 0);
}

// ============================================================================
// Partial-Pass Scenarios
// ============================================================================

#[tokio::test]
async fn partial_pass_below_threshold() {
    let mut registry = CheckRegistry::new();

    // All checks pass (no fatal failures)
    registry.register(Box::new(MockCheck::new(
        "BUILD_CHECK",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));

    // But add a non-fatal failure that lowers score
    registry.register(Box::new(MockCheck::new(
        "TEST_UNIT",
        CheckCategory::TestTruth,
        CheckSeverity::Warning,
        CheckStatus::Fail,
    )));

    let mut profile = DodProfile::default_dev();
    // Set high threshold (95%)
    profile.thresholds.min_readiness_score = 95.0;
    profile.required_checks = vec!["BUILD_CHECK".to_string(), "TEST_UNIT".to_string()];

    let validator = DodValidator::new(registry, profile);
    let result = validator.validate(test_workspace()).await.unwrap();

    // No fatal failures but score below threshold
    assert!(result.fatal_failures().is_empty());
    assert!(result.score < 95.0);
    assert_eq!(result.verdict, Verdict::PartialPass);
}

#[tokio::test]
async fn partial_pass_exactly_at_threshold() {
    let mut registry = CheckRegistry::new();

    // Build: 2 pass (100%)
    registry.register(Box::new(MockCheck::new(
        "BUILD_CHECK_1",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));
    registry.register(Box::new(MockCheck::new(
        "BUILD_CHECK_2",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));

    // Test: 1 pass, 1 fail (50%)
    registry.register(Box::new(MockCheck::new(
        "TEST_PASS",
        CheckCategory::TestTruth,
        CheckSeverity::Warning,
        CheckStatus::Pass,
    )));
    registry.register(Box::new(MockCheck::new(
        "TEST_FAIL",
        CheckCategory::TestTruth,
        CheckSeverity::Warning,
        CheckStatus::Fail,
    )));

    let mut profile = DodProfile::default_dev();
    // Build weight 0.25, Test weight 0.25
    // Score = 100 * 0.25 + 50 * 0.25 = 37.5
    profile.thresholds.min_readiness_score = 37.5;
    profile.required_checks = vec![
        "BUILD_CHECK_1".to_string(),
        "BUILD_CHECK_2".to_string(),
        "TEST_PASS".to_string(),
        "TEST_FAIL".to_string(),
    ];

    let validator = DodValidator::new(registry, profile);
    let result = validator.validate(test_workspace()).await.unwrap();

    // Should pass when exactly at threshold
    assert_eq!(result.score, 37.5);
    assert_eq!(result.verdict, Verdict::Pass);
}

// ============================================================================
// Scoring Algorithm Tests
// ============================================================================

#[tokio::test]
async fn scoring_respects_category_weights() {
    let mut registry = CheckRegistry::new();

    // Build (weight 0.25): 0% pass
    registry.register(Box::new(MockCheck::new(
        "BUILD_CHECK",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Warning,
        CheckStatus::Fail,
    )));

    // Test (weight 0.25): 100% pass
    registry.register(Box::new(MockCheck::new(
        "TEST_UNIT",
        CheckCategory::TestTruth,
        CheckSeverity::Warning,
        CheckStatus::Pass,
    )));

    let mut profile = DodProfile::default_dev();
    profile.required_checks = vec!["BUILD_CHECK".to_string(), "TEST_UNIT".to_string()];

    let validator = DodValidator::new(registry, profile);
    let result = validator.validate(test_workspace()).await.unwrap();

    // Score = 0 * 0.25 + 100 * 0.25 = 25.0
    assert_eq!(result.score, 25.0);
}

#[tokio::test]
async fn scoring_category_scores_computed() {
    let mut registry = CheckRegistry::new();

    // Build category: 2 pass, 1 fail
    registry.register(Box::new(MockCheck::new(
        "BUILD_1",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));
    registry.register(Box::new(MockCheck::new(
        "BUILD_2",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));
    registry.register(Box::new(MockCheck::new(
        "BUILD_3",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Warning,
        CheckStatus::Fail,
    )));

    let validator = DodValidator::with_default_profile(registry);
    let result = validator.validate(test_workspace()).await.unwrap();

    let build_score = result
        .category_scores
        .get(&CheckCategory::BuildCorrectness)
        .unwrap();

    assert_eq!(build_score.checks_passed, 2);
    assert_eq!(build_score.checks_failed, 1);
    // Score = 2/3 * 100 = 66.67%
    assert!((build_score.score - 66.67).abs() < 0.1);
}

#[tokio::test]
async fn scoring_warning_penalty_applied() {
    let mut registry = CheckRegistry::new();

    // All pass
    registry.register(Box::new(MockCheck::new(
        "CHECK_1",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));
    registry.register(Box::new(MockCheck::new(
        "CHECK_2",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));

    // One warning
    registry.register(Box::new(MockCheck::new(
        "CHECK_WARN",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Warning,
        CheckStatus::Warn,
    )));

    let validator = DodValidator::with_default_profile(registry);
    let result = validator.validate(test_workspace()).await.unwrap();

    let build_score = result
        .category_scores
        .get(&CheckCategory::BuildCorrectness)
        .unwrap();

    assert_eq!(build_score.checks_warned, 1);
    // Base: 100%, Warning penalty: -2 points
    assert_eq!(build_score.score, 98.0);
}

#[tokio::test]
async fn scoring_multiple_categories() {
    let mut registry = CheckRegistry::new();

    // Build: 100%
    registry.register(Box::new(MockCheck::new(
        "BUILD_CHECK",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));

    // Test: 100%
    registry.register(Box::new(MockCheck::new(
        "TEST_UNIT",
        CheckCategory::TestTruth,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));

    // ggen: 100%
    registry.register(Box::new(MockCheck::new(
        "GGEN_DRY_RUN",
        CheckCategory::GgenPipeline,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));

    let validator = DodValidator::with_default_profile(registry);
    let result = validator.validate(test_workspace()).await.unwrap();

    // Weight: Build 0.25 + Test 0.25 + ggen 0.20 = 0.70
    // Score = 100 * 0.70 = 70.0
    assert_eq!(result.score, 70.0);

    assert_eq!(result.category_scores.len(), 8); // All categories present
    assert!(result
        .category_scores
        .contains_key(&CheckCategory::BuildCorrectness));
    assert!(result
        .category_scores
        .contains_key(&CheckCategory::TestTruth));
    assert!(result
        .category_scores
        .contains_key(&CheckCategory::GgenPipeline));
}

// ============================================================================
// Verdict Computation Logic
// ============================================================================

#[tokio::test]
async fn verdict_logic_fatal_failure_overrides_score() {
    let mut registry = CheckRegistry::new();

    // One fatal failure
    registry.register(Box::new(MockCheck::new(
        "FATAL_CHECK",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Fail,
    )));

    // Many passing checks (high score)
    for i in 0..10 {
        registry.register(Box::new(MockCheck::new(
            &format!("PASS_{}", i),
            CheckCategory::TestTruth,
            CheckSeverity::Warning,
            CheckStatus::Pass,
        )));
    }

    let mut profile = DodProfile::default_dev();
    profile.thresholds.min_readiness_score = 50.0; // Low threshold

    let validator = DodValidator::new(registry, profile);
    let result = validator.validate(test_workspace()).await.unwrap();

    // Despite high score, fatal failure causes Fail verdict
    assert!(result.score > 50.0);
    assert_eq!(result.verdict, Verdict::Fail);
}

#[tokio::test]
async fn verdict_logic_threshold_checked_when_no_fatals() {
    let mut registry = CheckRegistry::new();

    // All non-fatal checks
    registry.register(Box::new(MockCheck::new(
        "CHECK_1",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Warning,
        CheckStatus::Pass,
    )));

    let mut profile = DodProfile::default_dev();

    // Test both above and below threshold
    profile.thresholds.min_readiness_score = 50.0;
    profile.required_checks = vec!["CHECK_1".to_string()];

    let validator = DodValidator::new(registry, profile);
    let result = validator.validate(test_workspace()).await.unwrap();

    // No fatal failures, score 100 > 50 threshold
    assert_eq!(result.verdict, Verdict::Pass);
}

#[tokio::test]
async fn verdict_logic_different_validation_modes() {
    let mut registry = CheckRegistry::new();

    registry.register(Box::new(MockCheck::new(
        "BUILD_CHECK",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));

    let validator = DodValidator::with_default_profile(registry);

    for mode in [
        ValidationMode::Fast,
        ValidationMode::Strict,
        ValidationMode::Paranoid,
    ] {
        let result = validator
            .validate_with_mode(test_workspace(), mode)
            .await
            .unwrap();

        assert_eq!(result.mode, mode);
        // Verdict logic same across modes (mode affects check behavior, not verdict)
        assert_eq!(result.verdict, Verdict::Pass);
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test]
async fn integration_full_validation_flow() {
    let mut registry = CheckRegistry::new();

    // Realistic set of checks
    registry.register(Box::new(MockCheck::new(
        "G0_WORKSPACE",
        CheckCategory::WorkspaceIntegrity,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));
    registry.register(Box::new(MockCheck::new(
        "BUILD_FMT",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));
    registry.register(Box::new(MockCheck::new(
        "BUILD_CHECK",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));
    registry.register(Box::new(MockCheck::new(
        "TEST_UNIT",
        CheckCategory::TestTruth,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));
    registry.register(Box::new(MockCheck::new(
        "GGEN_DRY_RUN",
        CheckCategory::GgenPipeline,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));

    let validator = DodValidator::with_default_profile(registry);
    let result = validator.validate(test_workspace()).await.unwrap();

    // Verify complete result structure
    assert_eq!(result.verdict, Verdict::Pass);
    assert_eq!(result.max_score, 100.0);
    assert!(!result.checks.is_empty());
    assert!(!result.category_scores.is_empty());
    assert!(result.execution_time.as_millis() > 0);
    assert!(!result.profile_name.is_empty());

    let summary = result.summary();
    assert_eq!(summary.total, 5);
    assert_eq!(summary.passed, 5);
    assert_eq!(summary.pass_rate(), 1.0);
}

#[tokio::test]
async fn integration_result_summary_methods() {
    let mut registry = CheckRegistry::new();

    registry.register(Box::new(MockCheck::new(
        "PASS_CHECK",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));
    registry.register(Box::new(MockCheck::new(
        "FAIL_CHECK",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Fail,
    )));
    registry.register(Box::new(MockCheck::new(
        "WARN_CHECK",
        CheckCategory::TestTruth,
        CheckSeverity::Warning,
        CheckStatus::Warn,
    )));

    let validator = DodValidator::with_default_profile(registry);
    let result = validator.validate(test_workspace()).await.unwrap();

    // Test all helper methods
    assert_eq!(result.failed_checks().len(), 1);
    assert_eq!(result.fatal_failures().len(), 1);
    assert_eq!(result.warned_checks().len(), 1);
    assert!(!result.execution_time_str().is_empty());

    let summary = result.summary();
    assert_eq!(summary.total, 3);
    assert_eq!(summary.passed, 1);
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.warned, 1);
}

#[tokio::test]
async fn integration_custom_profile() {
    let mut registry = CheckRegistry::new();

    registry.register(Box::new(MockCheck::new(
        "BUILD_CHECK",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));

    let profile = DodProfile::enterprise_strict();
    let validator = DodValidator::new(registry, profile);

    let result = validator.validate(test_workspace()).await.unwrap();

    assert_eq!(result.profile_name, "enterprise-strict");
    // Enterprise profile has higher threshold
    assert!(validator.profile().thresholds.min_readiness_score >= 90.0);
}

#[tokio::test]
async fn integration_timestamp_tracking() {
    let mut registry = CheckRegistry::new();

    registry.register(Box::new(MockCheck::new(
        "BUILD_CHECK",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));

    let validator = DodValidator::with_default_profile(registry);
    let before = chrono::Utc::now();
    let result = validator.validate(test_workspace()).await.unwrap();
    let after = chrono::Utc::now();

    assert!(result.timestamp >= before);
    assert!(result.timestamp <= after);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test]
async fn edge_case_no_checks_registered() {
    let registry = CheckRegistry::new();
    let validator = DodValidator::with_default_profile(registry);

    let result = validator.validate(test_workspace()).await.unwrap();

    // No checks = perfect score (nothing to fail)
    assert_eq!(result.checks.len(), 0);
    assert_eq!(result.score, 0.0);
}

#[tokio::test]
async fn edge_case_only_skipped_checks() {
    let mut registry = CheckRegistry::new();

    registry.register(Box::new(MockCheck::new(
        "SKIP_1",
        CheckCategory::BuildCorrectness,
        CheckSeverity::Info,
        CheckStatus::Skip,
    )));
    registry.register(Box::new(MockCheck::new(
        "SKIP_2",
        CheckCategory::TestTruth,
        CheckSeverity::Info,
        CheckStatus::Skip,
    )));

    let validator = DodValidator::with_default_profile(registry);
    let result = validator.validate(test_workspace()).await.unwrap();

    assert_eq!(result.summary().skipped, 2);
    // Skipped checks treated as 100% (no failures)
    assert_eq!(result.score, 100.0);
    assert_eq!(result.verdict, Verdict::Pass);
}

#[tokio::test]
async fn edge_case_invalid_workspace() {
    let registry = CheckRegistry::new();
    let validator = DodValidator::with_default_profile(registry);

    let invalid = PathBuf::from("/definitely/does/not/exist");
    let result = validator.validate(invalid).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn edge_case_zero_weight_categories() {
    let mut registry = CheckRegistry::new();

    // WorkspaceIntegrity has 0 weight (gating only)
    registry.register(Box::new(MockCheck::new(
        "G0_WORKSPACE",
        CheckCategory::WorkspaceIntegrity,
        CheckSeverity::Fatal,
        CheckStatus::Pass,
    )));

    let validator = DodValidator::with_default_profile(registry);
    let result = validator.validate(test_workspace()).await.unwrap();

    let ws_score = result
        .category_scores
        .get(&CheckCategory::WorkspaceIntegrity)
        .unwrap();

    // Weight is 0 (gating category)
    assert_eq!(ws_score.weight, 0.0);
    // But check still executes
    assert_eq!(ws_score.checks_passed, 1);
}
