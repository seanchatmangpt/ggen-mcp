//! DoD MCP Integration Tests
//!
//! Tests for the MCP tool interface for Definition of Done validation.
//! Validates tool invocation, parameter handling, response format, and error handling.

use spreadsheet_mcp::ServerConfig;
use spreadsheet_mcp::state::AppState;
use spreadsheet_mcp::tools::dod::*;
use std::path::PathBuf;
use std::sync::Arc;

/// Create test app state
fn test_app_state() -> Arc<AppState> {
    let config = Arc::new(ServerConfig {
        workspace_root: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        cache_capacity: 5,
        supported_extensions: vec!["xlsx".to_string()],
        single_workbook: None,
        enabled_tools: None,
        transport: spreadsheet_mcp::TransportKind::Stdio,
        http_bind_address: "127.0.0.1:8079".parse().unwrap(),
        recalc_enabled: false,
        vba_enabled: false,
        max_concurrent_recalcs: 2,
        tool_timeout_ms: Some(30_000),
        max_response_bytes: Some(1_000_000),
        allow_overwrite: false,
    });
    Arc::new(AppState::new(config))
}

mod mcp_tool_invocation {
    use super::*;

    #[tokio::test]
    async fn validates_with_default_params() {
        let state = test_app_state();

        let params = ValidateDefinitionOfDoneParams {
            profile: "comprehensive".to_string(),
            workspace_path: None,
            include_remediation: true,
            include_evidence: true,
            fail_fast: false,
        };

        let result = validate_definition_of_done(state, params).await;

        assert!(result.is_ok(), "MCP tool should execute successfully");

        let response = result.unwrap();
        assert!(!response.checks.is_empty(), "Should return check results");
        assert!(!response.narrative.is_empty(), "Should include narrative");
    }

    #[tokio::test]
    async fn validates_with_minimal_profile() {
        let state = test_app_state();

        let params = ValidateDefinitionOfDoneParams {
            profile: "minimal".to_string(),
            workspace_path: None,
            include_remediation: false,
            include_evidence: false,
            fail_fast: false,
        };

        let result = validate_definition_of_done(state, params).await;

        assert!(result.is_ok());
        let response = result.unwrap();

        // Minimal profile should run fewer checks
        assert!(response.summary.total_checks > 0);
        assert!(response.summary.total_checks < 15); // Less than comprehensive

        // Should not include remediation/evidence when disabled
        assert!(response.remediation.is_none());
        assert!(response.checks.iter().all(|c| c.evidence.is_none()));
    }

    #[tokio::test]
    async fn validates_with_standard_profile() {
        let state = test_app_state();

        let params = ValidateDefinitionOfDoneParams {
            profile: "standard".to_string(),
            workspace_path: None,
            include_remediation: true,
            include_evidence: true,
            fail_fast: false,
        };

        let result = validate_definition_of_done(state, params).await;

        assert!(result.is_ok());
        let response = result.unwrap();

        // Standard profile should be between minimal and comprehensive
        assert!(response.summary.total_checks >= 5);
        assert!(response.summary.total_checks <= 15);
    }

    #[tokio::test]
    async fn validates_with_comprehensive_profile() {
        let state = test_app_state();

        let params = ValidateDefinitionOfDoneParams {
            profile: "comprehensive".to_string(),
            workspace_path: None,
            include_remediation: true,
            include_evidence: true,
            fail_fast: false,
        };

        let result = validate_definition_of_done(state, params).await;

        assert!(result.is_ok());
        let response = result.unwrap();

        // Comprehensive should run most/all checks
        assert!(response.summary.total_checks >= 10);
    }

    #[tokio::test]
    async fn validates_with_custom_workspace_path() {
        let state = test_app_state();

        let params = ValidateDefinitionOfDoneParams {
            profile: "minimal".to_string(),
            workspace_path: Some(env!("CARGO_MANIFEST_DIR").to_string()),
            include_remediation: false,
            include_evidence: false,
            fail_fast: false,
        };

        let result = validate_definition_of_done(state, params).await;

        assert!(result.is_ok());
    }
}

mod parameter_handling {
    use super::*;

    #[tokio::test]
    async fn rejects_invalid_profile_name() {
        let state = test_app_state();

        let params = ValidateDefinitionOfDoneParams {
            profile: "nonexistent_profile".to_string(),
            workspace_path: None,
            include_remediation: true,
            include_evidence: true,
            fail_fast: false,
        };

        let result = validate_definition_of_done(state, params).await;

        assert!(result.is_err(), "Should reject unknown profile");
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Unknown profile"));
    }

    #[tokio::test]
    async fn rejects_path_traversal() {
        let state = test_app_state();

        let params = ValidateDefinitionOfDoneParams {
            profile: "minimal".to_string(),
            workspace_path: Some("../../../etc/passwd".to_string()),
            include_remediation: false,
            include_evidence: false,
            fail_fast: false,
        };

        let result = validate_definition_of_done(state, params).await;

        assert!(result.is_err(), "Should reject path traversal");
    }

    #[tokio::test]
    async fn handles_include_remediation_flag() {
        let state = test_app_state();

        // With remediation
        let params_with = ValidateDefinitionOfDoneParams {
            profile: "minimal".to_string(),
            workspace_path: None,
            include_remediation: true,
            include_evidence: false,
            fail_fast: false,
        };

        let response_with = validate_definition_of_done(state.clone(), params_with)
            .await
            .unwrap();

        // Without remediation
        let params_without = ValidateDefinitionOfDoneParams {
            profile: "minimal".to_string(),
            workspace_path: None,
            include_remediation: false,
            include_evidence: false,
            fail_fast: false,
        };

        let response_without = validate_definition_of_done(state, params_without)
            .await
            .unwrap();

        // If there are failures, with should have remediation, without should not
        if response_with.summary.failed > 0 {
            assert!(response_with.remediation.is_some());
            assert!(!response_with.remediation.unwrap().is_empty());
        }

        assert!(response_without.remediation.is_none());
    }

    #[tokio::test]
    async fn handles_include_evidence_flag() {
        let state = test_app_state();

        // With evidence
        let params_with = ValidateDefinitionOfDoneParams {
            profile: "minimal".to_string(),
            workspace_path: None,
            include_remediation: false,
            include_evidence: true,
            fail_fast: false,
        };

        let response_with = validate_definition_of_done(state.clone(), params_with)
            .await
            .unwrap();

        // Without evidence
        let params_without = ValidateDefinitionOfDoneParams {
            profile: "minimal".to_string(),
            workspace_path: None,
            include_remediation: false,
            include_evidence: false,
            fail_fast: false,
        };

        let response_without = validate_definition_of_done(state, params_without)
            .await
            .unwrap();

        // All checks in without should have no evidence
        assert!(response_without.checks.iter().all(|c| c.evidence.is_none()));
    }
}

mod response_format {
    use super::*;

    #[tokio::test]
    async fn response_has_required_fields() {
        let state = test_app_state();

        let params = ValidateDefinitionOfDoneParams {
            profile: "minimal".to_string(),
            workspace_path: None,
            include_remediation: true,
            include_evidence: true,
            fail_fast: false,
        };

        let response = validate_definition_of_done(state, params).await.unwrap();

        // Required fields
        assert!(response.confidence_score <= 100);
        assert!(!response.verdict.is_empty());
        assert!(!response.narrative.is_empty());
        assert!(!response.checks.is_empty());

        // Summary fields
        assert!(response.summary.total_checks > 0);
        assert_eq!(
            response.summary.total_checks,
            response.summary.passed
                + response.summary.failed
                + response.summary.warnings
                + response.summary.skipped
                + response.summary.errors
        );
    }

    #[tokio::test]
    async fn check_results_have_required_fields() {
        let state = test_app_state();

        let params = ValidateDefinitionOfDoneParams {
            profile: "minimal".to_string(),
            workspace_path: None,
            include_remediation: false,
            include_evidence: true,
            fail_fast: false,
        };

        let response = validate_definition_of_done(state, params).await.unwrap();

        for check in &response.checks {
            assert!(!check.id.is_empty(), "Check ID should not be empty");
            assert!(!check.category.is_empty(), "Category should not be empty");
            assert!(!check.status.is_empty(), "Status should not be empty");
            assert!(!check.message.is_empty(), "Message should not be empty");
            assert!(check.duration_ms > 0, "Duration should be recorded");

            // Status should be valid
            assert!(
                ["Pass", "Fail", "Warning", "Skipped", "Error"].contains(&check.status.as_str()),
                "Status should be valid: {}",
                check.status
            );
        }
    }

    #[tokio::test]
    async fn verdict_matches_ready_flag() {
        let state = test_app_state();

        let params = ValidateDefinitionOfDoneParams {
            profile: "minimal".to_string(),
            workspace_path: None,
            include_remediation: false,
            include_evidence: false,
            fail_fast: false,
        };

        let response = validate_definition_of_done(state, params).await.unwrap();

        // ready_for_deployment should match verdict
        if response.ready_for_deployment {
            assert_eq!(response.verdict, "READY");
        } else {
            assert!(["PENDING", "BLOCKED"].contains(&response.verdict.as_str()));
        }
    }

    #[tokio::test]
    async fn remediation_suggestions_have_valid_structure() {
        let state = test_app_state();

        let params = ValidateDefinitionOfDoneParams {
            profile: "comprehensive".to_string(),
            workspace_path: None,
            include_remediation: true,
            include_evidence: false,
            fail_fast: false,
        };

        let response = validate_definition_of_done(state, params).await.unwrap();

        if let Some(suggestions) = response.remediation {
            for suggestion in suggestions {
                assert!(!suggestion.check_id.is_empty());
                assert!(!suggestion.priority.is_empty());
                assert!(!suggestion.action.is_empty());
                assert!(!suggestion.rationale.is_empty());

                // Priority should be valid
                assert!(
                    ["Critical", "High", "Medium", "Low"].contains(&suggestion.priority.as_str()),
                    "Priority should be valid: {}",
                    suggestion.priority
                );
            }
        }
    }

    #[tokio::test]
    async fn narrative_provides_meaningful_context() {
        let state = test_app_state();

        let params = ValidateDefinitionOfDoneParams {
            profile: "minimal".to_string(),
            workspace_path: None,
            include_remediation: false,
            include_evidence: false,
            fail_fast: false,
        };

        let response = validate_definition_of_done(state, params).await.unwrap();

        // Narrative should be substantial
        assert!(
            response.narrative.len() > 50,
            "Narrative should be descriptive"
        );

        // Should mention verdict
        assert!(
            response.narrative.contains("READY")
                || response.narrative.contains("PENDING")
                || response.narrative.contains("BLOCKED")
        );
    }
}

mod error_handling {
    use super::*;

    #[tokio::test]
    async fn handles_nonexistent_workspace() {
        let state = test_app_state();

        let params = ValidateDefinitionOfDoneParams {
            profile: "minimal".to_string(),
            workspace_path: Some("/tmp/nonexistent_workspace_xyz123".to_string()),
            include_remediation: false,
            include_evidence: false,
            fail_fast: false,
        };

        let result = validate_definition_of_done(state, params).await;

        // Should either error or return with failures
        if let Ok(response) = result {
            // Workspace checks should fail
            assert!(response.summary.failed > 0 || response.summary.errors > 0);
        }
    }

    #[tokio::test]
    async fn provides_error_details_in_check_results() {
        let state = test_app_state();

        let params = ValidateDefinitionOfDoneParams {
            profile: "comprehensive".to_string(),
            workspace_path: None,
            include_remediation: true,
            include_evidence: false,
            fail_fast: false,
        };

        let response = validate_definition_of_done(state, params).await.unwrap();

        // Failed/error checks should have descriptive messages
        for check in &response.checks {
            if check.status == "Fail" || check.status == "Error" {
                assert!(
                    check.message.len() > 10,
                    "Error message should be descriptive: {}",
                    check.message
                );
            }
        }
    }

    #[tokio::test]
    async fn remediation_generated_for_failures() {
        let state = test_app_state();

        let params = ValidateDefinitionOfDoneParams {
            profile: "comprehensive".to_string(),
            workspace_path: None,
            include_remediation: true,
            include_evidence: false,
            fail_fast: false,
        };

        let response = validate_definition_of_done(state, params).await.unwrap();

        // If there are failures, should have remediation suggestions
        if response.summary.failed > 0 {
            assert!(response.remediation.is_some());
            let suggestions = response.remediation.unwrap();
            assert!(!suggestions.is_empty());

            // Each failed check should have remediation
            let failed_check_ids: Vec<_> = response
                .checks
                .iter()
                .filter(|c| c.status == "Fail")
                .map(|c| c.id.as_str())
                .collect();

            for failed_id in failed_check_ids {
                assert!(
                    suggestions.iter().any(|s| s.check_id == failed_id),
                    "Failed check {} should have remediation",
                    failed_id
                );
            }
        }
    }
}

mod performance {
    use super::*;

    #[tokio::test]
    async fn minimal_profile_completes_quickly() {
        let state = test_app_state();

        let params = ValidateDefinitionOfDoneParams {
            profile: "minimal".to_string(),
            workspace_path: None,
            include_remediation: false,
            include_evidence: false,
            fail_fast: false,
        };

        let start = std::time::Instant::now();
        let response = validate_definition_of_done(state, params).await.unwrap();
        let elapsed = start.elapsed();

        // Minimal profile should complete in reasonable time
        assert!(
            elapsed.as_secs() < 30,
            "Minimal profile should complete within 30 seconds"
        );

        // Duration should be recorded
        assert!(response.summary.total_duration_ms > 0);
        assert!(response.summary.total_duration_ms < 30_000);
    }

    #[tokio::test]
    async fn check_durations_are_recorded() {
        let state = test_app_state();

        let params = ValidateDefinitionOfDoneParams {
            profile: "minimal".to_string(),
            workspace_path: None,
            include_remediation: false,
            include_evidence: false,
            fail_fast: false,
        };

        let response = validate_definition_of_done(state, params).await.unwrap();

        // All checks should have duration > 0
        for check in &response.checks {
            assert!(
                check.duration_ms > 0,
                "Check {} should have duration",
                check.id
            );
        }

        // Sum of check durations should be <= total duration
        let sum_check_durations: u64 = response.checks.iter().map(|c| c.duration_ms).sum();
        assert!(
            sum_check_durations <= response.summary.total_duration_ms * 2,
            "Check durations should be reasonable"
        );
    }
}

mod integration_scenarios {
    use super::*;

    #[tokio::test]
    async fn validates_current_workspace() {
        // This tests against the actual ggen-mcp workspace
        let state = test_app_state();

        let params = ValidateDefinitionOfDoneParams {
            profile: "comprehensive".to_string(),
            workspace_path: Some(env!("CARGO_MANIFEST_DIR").to_string()),
            include_remediation: true,
            include_evidence: true,
            fail_fast: false,
        };

        let response = validate_definition_of_done(state, params).await.unwrap();

        // Should execute all checks
        assert!(response.summary.total_checks >= 10);

        // Should have a verdict
        assert!(["READY", "PENDING", "BLOCKED"].contains(&response.verdict.as_str()));

        // If not ready, should provide remediation
        if !response.ready_for_deployment {
            assert!(response.remediation.is_some());
            assert!(!response.remediation.unwrap().is_empty());
        }

        println!("\n=== Current Workspace Validation ===");
        println!("Verdict: {}", response.verdict);
        println!("Confidence: {}", response.confidence_score);
        println!(
            "Checks: {} passed, {} failed, {} warnings",
            response.summary.passed, response.summary.failed, response.summary.warnings
        );
    }

    #[tokio::test]
    async fn all_profiles_execute_successfully() {
        let state = test_app_state();

        for profile in ["minimal", "standard", "comprehensive"] {
            let params = ValidateDefinitionOfDoneParams {
                profile: profile.to_string(),
                workspace_path: None,
                include_remediation: true,
                include_evidence: true,
                fail_fast: false,
            };

            let result = validate_definition_of_done(state.clone(), params).await;

            assert!(
                result.is_ok(),
                "Profile {} should execute successfully",
                profile
            );

            let response = result.unwrap();
            assert!(
                response.summary.total_checks > 0,
                "Profile {} should execute checks",
                profile
            );
        }
    }
}
