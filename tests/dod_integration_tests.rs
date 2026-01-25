//! DoD End-to-End Integration Tests
//!
//! Comprehensive integration tests for the Definition of Done validation system.
//! Tests full orchestration flow: checks → scoring → verdict → remediation.

use spreadsheet_mcp::dod::*;
use std::path::PathBuf;

/// Create test context pointing to actual workspace
fn test_workspace_context() -> CheckContext {
    CheckContext::new(PathBuf::from(env!("CARGO_MANIFEST_DIR"))).with_timeout(120_000)
}

/// Create test context for fixtures
fn _fixture_context(fixture_name: &str) -> CheckContext {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("dod_test_workspace")
        .join(fixture_name);

    CheckContext::new(fixture_path).with_timeout(60_000)
}

mod e2e_full_validation {
    use super::*;

    #[tokio::test]
    async fn full_validation_flow_with_dev_profile() {
        // ARRANGE: Create registry and profile
        let registry = spreadsheet_mcp::dod::checks::create_registry();
        let profile = DodProfile::default_dev();
        let executor = CheckExecutor::new(registry, profile.clone());
        let context = test_workspace_context();

        // ACT: Execute all checks
        let check_results = executor.execute_all(&context).await.unwrap();

        // ASSERT: Basic validation
        assert!(!check_results.is_empty(), "Should execute some checks");

        // Verify all results have required fields
        for result in &check_results {
            assert!(!result.id.is_empty(), "Check ID should not be empty");
            assert!(!result.message.is_empty(), "Message should not be empty");
            assert!(
                !result.check_hash.is_empty(),
                "Check hash should not be empty"
            );
            assert!(result.duration_ms > 0, "Duration should be recorded");
        }

        // ACT: Compute category scores
        let mut category_scores = std::collections::HashMap::new();
        for category in [
            CheckCategory::BuildCorrectness,
            CheckCategory::TestTruth,
            CheckCategory::GgenPipeline,
            CheckCategory::ToolRegistry,
            CheckCategory::SafetyInvariants,
            CheckCategory::IntentAlignment,
        ] {
            let score = compute_category_score(category, &check_results);
            category_scores.insert(category, score);
        }

        // ACT: Compute overall readiness score
        let readiness_score = compute_readiness_score(&category_scores);

        // ASSERT: Score is in valid range
        assert!(readiness_score >= 0.0 && readiness_score <= 100.0);

        // ACT: Compute verdict
        let verdict = compute_verdict(&check_results);

        // ASSERT: Verdict is consistent with fatal failures
        let fatal_failures = get_fatal_failures(&check_results);
        if fatal_failures.is_empty() {
            assert_eq!(verdict, OverallVerdict::Ready);
        } else {
            assert_eq!(verdict, OverallVerdict::NotReady);
        }

        // ACT: Generate remediation
        let remediations = RemediationGenerator::generate(&check_results);

        // ASSERT: Remediation only for failures/warnings
        let failure_count = check_results
            .iter()
            .filter(|r| r.status == CheckStatus::Fail || r.status == CheckStatus::Warn)
            .count();

        if failure_count > 0 {
            assert!(
                !remediations.is_empty(),
                "Should have remediation suggestions"
            );
        }

        // Log summary for debugging
        println!("\n=== DoD Validation Summary ===");
        println!("Profile: {}", profile.name);
        println!("Checks executed: {}", check_results.len());
        println!("Readiness score: {:.2}", readiness_score);
        println!("Verdict: {:?}", verdict);
        println!("Fatal failures: {}", fatal_failures.len());
        println!("Remediation suggestions: {}", remediations.len());
    }

    #[tokio::test]
    async fn full_validation_flow_with_enterprise_profile() {
        // ARRANGE: Enterprise strict profile
        let registry = spreadsheet_mcp::dod::checks::create_registry();
        let profile = DodProfile::enterprise_strict();
        let executor = CheckExecutor::new(registry, profile.clone());
        let context = test_workspace_context();

        // ACT: Execute all checks
        let check_results = executor.execute_all(&context).await.unwrap();

        // ASSERT: More checks than dev profile
        let dev_profile = DodProfile::default_dev();
        assert!(
            check_results.len() >= dev_profile.required_checks.len(),
            "Enterprise should run at least as many checks as dev"
        );

        // ASSERT: Thresholds are stricter
        assert!(profile.thresholds.min_readiness_score > 80.0);
        assert!(profile.thresholds.max_warnings < 10);
        assert!(profile.thresholds.require_all_tests_pass);
        assert!(profile.thresholds.fail_on_clippy_warnings);
    }

    #[tokio::test]
    async fn validation_respects_check_dependencies() {
        // ARRANGE
        let registry = spreadsheet_mcp::dod::checks::create_registry();
        let profile = DodProfile::default_dev();
        let executor = CheckExecutor::new(registry, profile);
        let context = test_workspace_context();

        // ACT: Execute all checks
        let check_results = executor.execute_all(&context).await.unwrap();

        // ASSERT: Find dependent checks
        // GGEN_RENDER depends on GGEN_DRY_RUN
        let dry_run_idx = check_results.iter().position(|r| r.id == "GGEN_DRY_RUN");
        let render_idx = check_results.iter().position(|r| r.id == "GGEN_RENDER");

        if let (Some(dry_idx), Some(rend_idx)) = (dry_run_idx, render_idx) {
            assert!(
                dry_idx < rend_idx,
                "GGEN_DRY_RUN should execute before GGEN_RENDER (dependency ordering)"
            );
        }
    }

    #[tokio::test]
    async fn validation_generates_evidence() {
        // ARRANGE
        let registry = spreadsheet_mcp::dod::checks::create_registry();
        let profile = DodProfile::default_dev();
        let executor = CheckExecutor::new(registry, profile);
        let context = test_workspace_context();

        // ACT: Execute checks
        let check_results = executor.execute_all(&context).await.unwrap();

        // ASSERT: Some checks should generate evidence
        let checks_with_evidence = check_results
            .iter()
            .filter(|r| !r.evidence.is_empty())
            .count();

        // At least safety and build checks should have evidence
        assert!(
            checks_with_evidence > 0,
            "Some checks should provide evidence"
        );

        // ASSERT: Evidence has valid structure
        for result in &check_results {
            for evidence in &result.evidence {
                assert!(
                    !evidence.content.is_empty(),
                    "Evidence content should not be empty"
                );
                assert!(
                    !evidence.hash.is_empty(),
                    "Evidence hash should not be empty"
                );
            }
        }
    }
}

mod profile_validation {
    use super::*;

    #[test]
    fn default_dev_profile_is_valid() {
        let profile = DodProfile::default_dev();
        assert!(profile.validate().is_ok());
        assert_eq!(profile.name, "ggen-mcp-default");
        assert!(!profile.required_checks.is_empty());
    }

    #[test]
    fn enterprise_strict_profile_is_valid() {
        let profile = DodProfile::enterprise_strict();
        assert!(profile.validate().is_ok());
        assert_eq!(profile.name, "enterprise-strict");
        assert!(profile.required_checks.len() > 10);
    }

    #[test]
    fn profile_weights_sum_to_one() {
        let profile = DodProfile::default_dev();
        let weight_sum: f64 = profile.category_weights.values().sum();
        assert!(
            (weight_sum - 1.0).abs() < 0.001,
            "Weights should sum to 1.0"
        );
    }

    #[test]
    fn profile_thresholds_are_valid() {
        let dev = DodProfile::default_dev();
        assert!(dev.thresholds.min_readiness_score >= 0.0);
        assert!(dev.thresholds.min_readiness_score <= 100.0);
        assert!(dev.thresholds.max_warnings > 0);

        let enterprise = DodProfile::enterprise_strict();
        assert!(enterprise.thresholds.min_readiness_score > dev.thresholds.min_readiness_score);
        assert!(enterprise.thresholds.max_warnings < dev.thresholds.max_warnings);
    }

    #[test]
    fn profile_timeouts_are_reasonable() {
        let profile = DodProfile::default_dev();
        assert!(profile.timeouts_ms.build >= 60_000); // At least 1 minute
        assert!(profile.timeouts_ms.tests >= 60_000);
        assert!(profile.timeouts_ms.ggen >= 60_000);
        assert!(profile.timeouts_ms.default >= 1_000);
    }
}

mod scoring_integration {
    use super::*;

    #[test]
    fn category_score_computes_correctly() {
        // ARRANGE: Mock check results
        let results = vec![
            mock_check_result("CHECK1", CheckCategory::BuildCorrectness, CheckStatus::Pass),
            mock_check_result("CHECK2", CheckCategory::BuildCorrectness, CheckStatus::Pass),
            mock_check_result("CHECK3", CheckCategory::BuildCorrectness, CheckStatus::Fail),
            mock_check_result("CHECK4", CheckCategory::BuildCorrectness, CheckStatus::Warn),
        ];

        // ACT
        let score = compute_category_score(CheckCategory::BuildCorrectness, &results);

        // ASSERT
        assert_eq!(score.category, CheckCategory::BuildCorrectness);
        assert_eq!(score.checks_passed, 2);
        assert_eq!(score.checks_failed, 1);
        assert_eq!(score.checks_warned, 1);

        // Score = (2 / 3) * 100 = 66.67 - 2 (warning penalty) = 64.67
        assert!((score.score - 64.67).abs() < 0.1);
    }

    #[test]
    fn readiness_score_uses_weighted_average() {
        // ARRANGE
        let mut category_scores = std::collections::HashMap::new();

        // Perfect build score (25% weight)
        category_scores.insert(
            CheckCategory::BuildCorrectness,
            CategoryScore {
                category: CheckCategory::BuildCorrectness,
                score: 100.0,
                weight: 0.25,
                checks_passed: 3,
                checks_failed: 0,
                checks_warned: 0,
                checks_skipped: 0,
            },
        );

        // 50% test score (25% weight)
        category_scores.insert(
            CheckCategory::TestTruth,
            CategoryScore {
                category: CheckCategory::TestTruth,
                score: 50.0,
                weight: 0.25,
                checks_passed: 1,
                checks_failed: 1,
                checks_warned: 0,
                checks_skipped: 0,
            },
        );

        // Other categories with 0 weight
        for category in [
            CheckCategory::GgenPipeline,
            CheckCategory::ToolRegistry,
            CheckCategory::SafetyInvariants,
            CheckCategory::IntentAlignment,
        ] {
            category_scores.insert(
                category,
                CategoryScore {
                    category,
                    score: 100.0,
                    weight: 0.50 / 4.0, // Remaining 50% divided
                    checks_passed: 1,
                    checks_failed: 0,
                    checks_warned: 0,
                    checks_skipped: 0,
                },
            );
        }

        // ACT
        let readiness = compute_readiness_score(&category_scores);

        // ASSERT: (100 * 0.25) + (50 * 0.25) + (100 * 0.50) = 87.5
        assert!((readiness - 87.5).abs() < 0.1);
    }

    #[test]
    fn empty_category_scores_zero() {
        let results: Vec<DodCheckResult> = vec![];
        let score = compute_category_score(CheckCategory::BuildCorrectness, &results);

        assert_eq!(score.score, 0.0);
        assert_eq!(score.checks_passed, 0);
        assert_eq!(score.checks_failed, 0);
    }

    fn mock_check_result(id: &str, category: CheckCategory, status: CheckStatus) -> DodCheckResult {
        DodCheckResult {
            id: id.to_string(),
            category,
            status,
            severity: CheckSeverity::Fatal,
            message: "test".to_string(),
            evidence: vec![],
            remediation: vec![],
            duration_ms: 100,
            check_hash: "test_hash".to_string(),
        }
    }
}

mod verdict_integration {
    use super::*;

    #[test]
    fn verdict_ready_when_all_pass() {
        let results = vec![
            mock_result("CHECK1", CheckStatus::Pass, CheckSeverity::Fatal),
            mock_result("CHECK2", CheckStatus::Pass, CheckSeverity::Fatal),
            mock_result("CHECK3", CheckStatus::Warn, CheckSeverity::Warning),
        ];

        assert_eq!(compute_verdict(&results), OverallVerdict::Ready);
    }

    #[test]
    fn verdict_not_ready_on_fatal_failure() {
        let results = vec![
            mock_result("CHECK1", CheckStatus::Pass, CheckSeverity::Fatal),
            mock_result("CHECK2", CheckStatus::Fail, CheckSeverity::Fatal),
        ];

        assert_eq!(compute_verdict(&results), OverallVerdict::NotReady);
    }

    #[test]
    fn verdict_ready_with_non_fatal_failures() {
        let results = vec![
            mock_result("CHECK1", CheckStatus::Fail, CheckSeverity::Warning),
            mock_result("CHECK2", CheckStatus::Fail, CheckSeverity::Info),
            mock_result("CHECK3", CheckStatus::Pass, CheckSeverity::Fatal),
        ];

        assert_eq!(compute_verdict(&results), OverallVerdict::Ready);
    }

    #[test]
    fn get_fatal_failures_filters_correctly() {
        let results = vec![
            mock_result("CHECK1", CheckStatus::Pass, CheckSeverity::Fatal),
            mock_result("CHECK2", CheckStatus::Fail, CheckSeverity::Fatal),
            mock_result("CHECK3", CheckStatus::Fail, CheckSeverity::Warning),
            mock_result("CHECK4", CheckStatus::Fail, CheckSeverity::Fatal),
        ];

        let fatal = get_fatal_failures(&results);
        assert_eq!(fatal.len(), 2);
        assert!(fatal.iter().any(|r| r.id == "CHECK2"));
        assert!(fatal.iter().any(|r| r.id == "CHECK4"));
    }

    fn mock_result(id: &str, status: CheckStatus, severity: CheckSeverity) -> DodCheckResult {
        DodCheckResult {
            id: id.to_string(),
            category: CheckCategory::BuildCorrectness,
            status,
            severity,
            message: "test".to_string(),
            evidence: vec![],
            remediation: vec![],
            duration_ms: 100,
            check_hash: "test".to_string(),
        }
    }
}

mod remediation_integration {
    use super::*;

    #[test]
    fn remediation_generated_for_failures() {
        let results = vec![
            mock_fail("BUILD_FMT", CheckCategory::BuildCorrectness),
            mock_fail("BUILD_CLIPPY", CheckCategory::BuildCorrectness),
            mock_pass("BUILD_CHECK", CheckCategory::BuildCorrectness),
        ];

        let suggestions = RemediationGenerator::generate(&results);

        assert_eq!(suggestions.len(), 2);
        assert!(suggestions.iter().any(|s| s.check_id == "BUILD_FMT"));
        assert!(suggestions.iter().any(|s| s.check_id == "BUILD_CLIPPY"));
    }

    #[test]
    fn remediation_includes_automation() {
        let results = vec![mock_fail("BUILD_FMT", CheckCategory::BuildCorrectness)];

        let suggestions = RemediationGenerator::generate(&results);

        assert_eq!(suggestions.len(), 1);
        let suggestion = &suggestions[0];
        assert!(suggestion.automation.is_some());
        assert!(
            suggestion
                .automation
                .as_ref()
                .unwrap()
                .contains("cargo fmt")
        );
    }

    #[test]
    fn remediation_prioritizes_critical() {
        let results = vec![
            mock_fail_severity(
                "BUILD_CHECK",
                CheckCategory::BuildCorrectness,
                CheckSeverity::Fatal,
            ),
            mock_fail_severity(
                "WHY_INTENT",
                CheckCategory::IntentAlignment,
                CheckSeverity::Warning,
            ),
        ];

        let suggestions = RemediationGenerator::generate(&results);

        assert!(suggestions.len() >= 2);

        // Critical failures should be prioritized
        let build_suggestion = suggestions.iter().find(|s| s.check_id == "BUILD_CHECK");
        assert!(build_suggestion.is_some());
        assert_eq!(build_suggestion.unwrap().priority, Priority::Critical);
    }

    #[test]
    fn no_remediation_for_passing_checks() {
        let results = vec![
            mock_pass("BUILD_FMT", CheckCategory::BuildCorrectness),
            mock_pass("BUILD_CHECK", CheckCategory::BuildCorrectness),
        ];

        let suggestions = RemediationGenerator::generate(&results);
        assert!(suggestions.is_empty());
    }

    fn mock_fail(id: &str, category: CheckCategory) -> DodCheckResult {
        mock_fail_severity(id, category, CheckSeverity::Fatal)
    }

    fn mock_fail_severity(
        id: &str,
        category: CheckCategory,
        severity: CheckSeverity,
    ) -> DodCheckResult {
        DodCheckResult {
            id: id.to_string(),
            category,
            status: CheckStatus::Fail,
            severity,
            message: "Failed".to_string(),
            evidence: vec![],
            remediation: vec!["Fix it".to_string()],
            duration_ms: 100,
            check_hash: "test".to_string(),
        }
    }

    fn mock_pass(id: &str, category: CheckCategory) -> DodCheckResult {
        DodCheckResult {
            id: id.to_string(),
            category,
            status: CheckStatus::Pass,
            severity: CheckSeverity::Fatal,
            message: "Passed".to_string(),
            evidence: vec![],
            remediation: vec![],
            duration_ms: 100,
            check_hash: "test".to_string(),
        }
    }
}

mod failure_scenarios {
    use super::*;

    #[tokio::test]
    async fn handles_missing_cargo_toml() {
        // ARRANGE: Context pointing to non-existent workspace
        let context =
            CheckContext::new(PathBuf::from("/tmp/nonexistent_workspace")).with_timeout(30_000);

        let registry = spreadsheet_mcp::dod::checks::create_registry();
        let profile = DodProfile::default_dev();
        let executor = CheckExecutor::new(registry, profile);

        // ACT: Execute checks
        let check_results = executor.execute_all(&context).await.unwrap();

        // ASSERT: Workspace checks should fail
        let workspace_failures = check_results
            .iter()
            .filter(|r| {
                r.category == CheckCategory::WorkspaceIntegrity && r.status == CheckStatus::Fail
            })
            .count();

        assert!(
            workspace_failures > 0,
            "Workspace integrity checks should fail"
        );
    }

    #[tokio::test]
    async fn handles_timeout_gracefully() {
        use async_trait::async_trait;

        // ARRANGE: Create a slow check
        struct SlowCheck;

        #[async_trait]
        impl DodCheck for SlowCheck {
            fn id(&self) -> &str {
                "SLOW_CHECK"
            }

            fn category(&self) -> CheckCategory {
                CheckCategory::BuildCorrectness
            }

            fn severity(&self) -> CheckSeverity {
                CheckSeverity::Fatal
            }

            fn description(&self) -> &str {
                "A very slow check"
            }

            async fn execute(&self, _context: &CheckContext) -> anyhow::Result<DodCheckResult> {
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                Ok(DodCheckResult {
                    id: self.id().to_string(),
                    category: self.category(),
                    status: CheckStatus::Pass,
                    severity: self.severity(),
                    message: "Should never reach here".to_string(),
                    evidence: vec![],
                    remediation: vec![],
                    duration_ms: 10_000,
                    check_hash: "slow".to_string(),
                })
            }
        }

        let mut registry = CheckRegistry::new();
        registry.register(Box::new(SlowCheck));

        let mut profile = DodProfile::default_dev();
        profile.required_checks.clear();
        profile.required_checks.push("SLOW_CHECK".to_string());
        profile.timeouts_ms.build = 100; // 100ms timeout

        let executor = CheckExecutor::new(registry, profile);
        let context = test_workspace_context();

        // ACT
        let results = executor.execute_all(&context).await.unwrap();

        // ASSERT: Should have timeout result
        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert_eq!(result.status, CheckStatus::Fail);
        assert!(result.message.contains("timed out"));
    }

    #[tokio::test]
    async fn executor_continues_after_check_failure() {
        use async_trait::async_trait;

        // ARRANGE: Create failing and passing checks
        struct FailingCheck;

        #[async_trait]
        impl DodCheck for FailingCheck {
            fn id(&self) -> &str {
                "FAILING_CHECK"
            }

            fn category(&self) -> CheckCategory {
                CheckCategory::BuildCorrectness
            }

            fn severity(&self) -> CheckSeverity {
                CheckSeverity::Fatal
            }

            fn description(&self) -> &str {
                "Always fails"
            }

            async fn execute(&self, _context: &CheckContext) -> anyhow::Result<DodCheckResult> {
                Ok(DodCheckResult {
                    id: self.id().to_string(),
                    category: self.category(),
                    status: CheckStatus::Fail,
                    severity: self.severity(),
                    message: "Intentional failure".to_string(),
                    evidence: vec![],
                    remediation: vec!["Fix the issue".to_string()],
                    duration_ms: 10,
                    check_hash: "fail".to_string(),
                })
            }
        }

        struct PassingCheck;

        #[async_trait]
        impl DodCheck for PassingCheck {
            fn id(&self) -> &str {
                "PASSING_CHECK"
            }

            fn category(&self) -> CheckCategory {
                CheckCategory::BuildCorrectness
            }

            fn severity(&self) -> CheckSeverity {
                CheckSeverity::Fatal
            }

            fn description(&self) -> &str {
                "Always passes"
            }

            async fn execute(&self, _context: &CheckContext) -> anyhow::Result<DodCheckResult> {
                Ok(DodCheckResult {
                    id: self.id().to_string(),
                    category: self.category(),
                    status: CheckStatus::Pass,
                    severity: self.severity(),
                    message: "Success".to_string(),
                    evidence: vec![],
                    remediation: vec![],
                    duration_ms: 10,
                    check_hash: "pass".to_string(),
                })
            }
        }

        let mut registry = CheckRegistry::new();
        registry.register(Box::new(FailingCheck));
        registry.register(Box::new(PassingCheck));

        let mut profile = DodProfile::default_dev();
        profile.required_checks.clear();
        profile.required_checks.push("FAILING_CHECK".to_string());
        profile.required_checks.push("PASSING_CHECK".to_string());

        let executor = CheckExecutor::new(registry, profile);
        let context = test_workspace_context();

        // ACT
        let results = executor.execute_all(&context).await.unwrap();

        // ASSERT: Both checks should execute despite one failing
        assert_eq!(results.len(), 2);
        assert!(
            results
                .iter()
                .any(|r| r.id == "FAILING_CHECK" && r.status == CheckStatus::Fail)
        );
        assert!(
            results
                .iter()
                .any(|r| r.id == "PASSING_CHECK" && r.status == CheckStatus::Pass)
        );
    }
}

mod all_checks_together {
    use super::*;

    #[tokio::test]
    async fn all_15_checks_execute_in_enterprise_mode() {
        // ARRANGE
        let registry = spreadsheet_mcp::dod::checks::create_registry();
        let profile = DodProfile::enterprise_strict();
        let executor = CheckExecutor::new(registry, profile.clone());
        let context = test_workspace_context();

        // ACT
        let results = executor.execute_all(&context).await.unwrap();

        // ASSERT: Should execute required checks from enterprise profile
        assert!(
            results.len() >= profile.required_checks.len(),
            "Should execute all required checks"
        );

        // ASSERT: All categories represented
        let categories: std::collections::HashSet<_> = results.iter().map(|r| r.category).collect();

        assert!(categories.contains(&CheckCategory::BuildCorrectness));
        assert!(categories.contains(&CheckCategory::TestTruth));
        assert!(categories.contains(&CheckCategory::GgenPipeline));
        assert!(categories.contains(&CheckCategory::ToolRegistry));
        assert!(categories.contains(&CheckCategory::SafetyInvariants));
    }

    #[tokio::test]
    async fn check_ids_are_unique() {
        let registry = spreadsheet_mcp::dod::checks::create_registry();
        let all_checks = registry.get_all();

        let mut seen_ids = std::collections::HashSet::new();
        for check in all_checks {
            let id = check.id();
            assert!(!seen_ids.contains(id), "Duplicate check ID found: {}", id);
            seen_ids.insert(id);
        }
    }

    #[tokio::test]
    async fn all_checks_have_valid_metadata() {
        let registry = spreadsheet_mcp::dod::checks::create_registry();
        let all_checks = registry.get_all();

        for check in all_checks {
            assert!(!check.id().is_empty(), "Check ID should not be empty");
            assert!(
                !check.description().is_empty(),
                "Description should not be empty"
            );

            // Category should be valid
            let _category = check.category(); // Should not panic

            // Severity should be valid
            let _severity = check.severity(); // Should not panic
        }
    }
}
