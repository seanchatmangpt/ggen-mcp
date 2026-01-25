//! DoD Remediation Suggestion Generator
//!
//! Generates actionable remediation suggestions based on check results.
//! Provides category-specific guidance with automation commands where applicable.

use crate::dod::types::*;

/// Priority for remediation suggestions
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Critical = 0, // Must fix before shipping
    High = 1,     // Should fix soon
    Medium = 2,   // Fix when convenient
    Low = 3,      // Nice to have
}

/// A remediation suggestion with steps and optional automation
#[derive(Debug, Clone)]
pub struct RemediationSuggestion {
    pub check_id: String,
    pub priority: Priority,
    pub title: String,
    pub steps: Vec<String>,
    pub automation: Option<String>,
}

/// Remediation generator
pub struct RemediationGenerator;

impl RemediationGenerator {
    /// Generate remediation suggestions from check results
    pub fn generate(check_results: &[DodCheckResult]) -> Vec<RemediationSuggestion> {
        let mut suggestions = vec![];

        for result in check_results {
            // Only generate suggestions for failures and warnings
            if result.status != CheckStatus::Fail && result.status != CheckStatus::Warn {
                continue;
            }

            let category_suggestions = match result.category {
                CheckCategory::WorkspaceIntegrity => Self::workspace_remediation(result),
                CheckCategory::IntentAlignment => Self::intent_remediation(result),
                CheckCategory::ToolRegistry => Self::tool_registry_remediation(result),
                CheckCategory::BuildCorrectness => Self::build_remediation(result),
                CheckCategory::TestTruth => Self::test_remediation(result),
                CheckCategory::GgenPipeline => Self::ggen_remediation(result),
                CheckCategory::SafetyInvariants => Self::safety_remediation(result),
                CheckCategory::DeploymentReadiness => Self::deployment_remediation(result),
            };

            suggestions.extend(category_suggestions);
        }

        // Deduplicate and prioritize
        Self::deduplicate_and_prioritize(suggestions)
    }

    /// Workspace integrity remediation
    fn workspace_remediation(result: &DodCheckResult) -> Vec<RemediationSuggestion> {
        let priority = if result.severity == CheckSeverity::Fatal {
            Priority::Critical
        } else {
            Priority::High
        };

        let (title, steps, automation) = match result.id.as_str() {
            "GIT_CLEAN" => (
                "Clean git workspace".to_string(),
                vec![
                    "Commit or stash uncommitted changes".to_string(),
                    "Run: git status".to_string(),
                    "Ensure working directory is clean".to_string(),
                ],
                Some("git status".to_string()),
            ),
            "SUBMODULES_INIT" => (
                "Initialize git submodules".to_string(),
                vec![
                    "Run: git submodule update --init --recursive".to_string(),
                    "Verify submodules are present".to_string(),
                ],
                Some("git submodule update --init --recursive".to_string()),
            ),
            "CARGO_LOCK" => (
                "Update Cargo.lock".to_string(),
                vec![
                    "Run: cargo update".to_string(),
                    "Commit Cargo.lock changes".to_string(),
                ],
                Some("cargo update".to_string()),
            ),
            _ => (
                format!("Fix {}", result.id),
                result.remediation.clone(),
                None,
            ),
        };

        vec![RemediationSuggestion {
            check_id: result.id.clone(),
            priority,
            title,
            steps,
            automation,
        }]
    }

    /// Intent alignment remediation
    fn intent_remediation(result: &DodCheckResult) -> Vec<RemediationSuggestion> {
        vec![RemediationSuggestion {
            check_id: result.id.clone(),
            priority: Priority::Medium,
            title: "Document intent (WHY)".to_string(),
            steps: vec![
                "Create docs/PRD.md or docs/ADR.md".to_string(),
                "Explain WHY this change is being made".to_string(),
                "Reference related issues or requirements".to_string(),
                "Document decision rationale".to_string(),
            ],
            automation: Some("mkdir -p docs && touch docs/PRD.md".to_string()),
        }]
    }

    /// Tool registry remediation
    fn tool_registry_remediation(result: &DodCheckResult) -> Vec<RemediationSuggestion> {
        vec![RemediationSuggestion {
            check_id: result.id.clone(),
            priority: Priority::High,
            title: "Align tool registry with OpenAPI".to_string(),
            steps: result.remediation.clone(),
            automation: None,
        }]
    }

    /// Build correctness remediation
    fn build_remediation(result: &DodCheckResult) -> Vec<RemediationSuggestion> {
        let (priority, title, automation) = match result.id.as_str() {
            "BUILD_FMT" => (
                Priority::Critical,
                "Fix code formatting".to_string(),
                Some("cargo fmt".to_string()),
            ),
            "BUILD_CLIPPY" => (
                Priority::High,
                "Fix clippy warnings".to_string(),
                Some("cargo clippy --fix".to_string()),
            ),
            "BUILD_CHECK" => (
                Priority::Critical,
                "Fix compilation errors".to_string(),
                Some("cargo check".to_string()),
            ),
            _ => (Priority::High, format!("Fix {}", result.id), None),
        };

        let mut steps = result.remediation.clone();
        if steps.is_empty() {
            steps = vec![
                format!(
                    "Run: {}",
                    automation.as_ref().unwrap_or(&"cargo build".to_string())
                ),
                "Fix all errors and warnings".to_string(),
                "Verify with: cargo check".to_string(),
            ];
        }

        vec![RemediationSuggestion {
            check_id: result.id.clone(),
            priority,
            title,
            steps,
            automation,
        }]
    }

    /// Test truth remediation
    fn test_remediation(result: &DodCheckResult) -> Vec<RemediationSuggestion> {
        let (title, steps, automation) = match result.id.as_str() {
            "TEST_UNIT" => (
                "Fix failing unit tests".to_string(),
                vec![
                    "Run: cargo test".to_string(),
                    "Fix test failures".to_string(),
                    "Ensure all tests pass".to_string(),
                ],
                Some("cargo test".to_string()),
            ),
            "TEST_INTEGRATION" => (
                "Fix integration tests".to_string(),
                vec![
                    "Run: cargo test --test '*'".to_string(),
                    "Fix integration test failures".to_string(),
                ],
                Some("cargo test --test '*'".to_string()),
            ),
            "TEST_PROPERTY" => (
                "Fix property tests".to_string(),
                vec![
                    "Run property tests".to_string(),
                    "Fix property violations".to_string(),
                ],
                Some("cargo test property".to_string()),
            ),
            _ => (
                format!("Fix {}", result.id),
                result.remediation.clone(),
                Some("cargo test".to_string()),
            ),
        };

        vec![RemediationSuggestion {
            check_id: result.id.clone(),
            priority: Priority::Critical,
            title,
            steps,
            automation,
        }]
    }

    /// Ggen pipeline remediation
    fn ggen_remediation(result: &DodCheckResult) -> Vec<RemediationSuggestion> {
        let (title, steps, automation) = match result.id.as_str() {
            "GGEN_DRY_RUN" => (
                "Fix ggen dry-run validation".to_string(),
                vec![
                    "Run: cargo make sync".to_string(),
                    "Review preview output".to_string(),
                    "Fix ontology/template issues".to_string(),
                ],
                Some("cargo make sync".to_string()),
            ),
            "GGEN_RENDER" => (
                "Fix ggen rendering".to_string(),
                vec![
                    "Run: cargo make sync --no-preview".to_string(),
                    "Verify generated code compiles".to_string(),
                    "Check for TODOs in generated code".to_string(),
                ],
                Some("cargo make sync --no-preview".to_string()),
            ),
            "GGEN_VALIDATE" => (
                "Fix ggen validation".to_string(),
                vec![
                    "Ensure generated code has no TODOs".to_string(),
                    "Verify all validate() functions are implemented".to_string(),
                    "Check file sizes > 100 bytes".to_string(),
                ],
                Some("cargo make sync-validate".to_string()),
            ),
            _ => (
                format!("Fix {}", result.id),
                result.remediation.clone(),
                Some("cargo make sync".to_string()),
            ),
        };

        vec![RemediationSuggestion {
            check_id: result.id.clone(),
            priority: Priority::High,
            title,
            steps,
            automation,
        }]
    }

    /// Safety invariants remediation
    fn safety_remediation(result: &DodCheckResult) -> Vec<RemediationSuggestion> {
        let priority = if result.id == "G8_SECRETS" {
            Priority::Critical
        } else {
            Priority::Medium
        };

        let (title, steps, automation) = match result.id.as_str() {
            "G8_SECRETS" => (
                "Remove exposed secrets".to_string(),
                vec![
                    "Scan code for API keys, passwords, tokens".to_string(),
                    "Move secrets to .env or secure vault".to_string(),
                    "Add .env to .gitignore".to_string(),
                    "Rotate exposed credentials".to_string(),
                ],
                Some("git-secrets --scan".to_string()),
            ),
            "G8_BOUNDS" => (
                "Fix bounds validation".to_string(),
                vec![
                    "Review input validation code".to_string(),
                    "Ensure all inputs are validated".to_string(),
                    "Add bounds checks for numeric inputs".to_string(),
                ],
                None,
            ),
            _ => (
                format!("Address {}", result.id),
                result.remediation.clone(),
                None,
            ),
        };

        vec![RemediationSuggestion {
            check_id: result.id.clone(),
            priority,
            title,
            steps,
            automation,
        }]
    }

    /// Deployment readiness remediation
    fn deployment_remediation(result: &DodCheckResult) -> Vec<RemediationSuggestion> {
        let (title, steps, automation) = match result.id.as_str() {
            "DEPLOY_BUILD_RELEASE" => (
                "Fix release build".to_string(),
                vec![
                    "Run: cargo build --release --locked".to_string(),
                    "Fix any release-specific issues".to_string(),
                    "Verify binary is created".to_string(),
                ],
                Some("cargo build --release --locked".to_string()),
            ),
            "DEPLOY_DOCKER_BUILD" => (
                "Fix Docker build".to_string(),
                vec![
                    "Run: docker build -t app:latest .".to_string(),
                    "Fix Dockerfile issues".to_string(),
                    "Verify image builds successfully".to_string(),
                ],
                Some("docker build -t app:latest .".to_string()),
            ),
            _ => (
                format!("Fix {}", result.id),
                result.remediation.clone(),
                None,
            ),
        };

        vec![RemediationSuggestion {
            check_id: result.id.clone(),
            priority: Priority::High,
            title,
            steps,
            automation,
        }]
    }

    /// Deduplicate and prioritize suggestions
    fn deduplicate_and_prioritize(
        mut suggestions: Vec<RemediationSuggestion>,
    ) -> Vec<RemediationSuggestion> {
        // Sort by priority: Critical > High > Medium > Low
        suggestions.sort_by_key(|s| s.priority.clone());

        // Could add deduplication logic here if needed
        // For now, just return sorted list

        suggestions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_failing_check(id: &str, category: CheckCategory) -> DodCheckResult {
        DodCheckResult {
            id: id.to_string(),
            category,
            status: CheckStatus::Fail,
            severity: CheckSeverity::Fatal,
            message: "Check failed".to_string(),
            evidence: vec![],
            remediation: vec!["Fix the issue".to_string()],
            duration_ms: 100,
            check_hash: "test".to_string(),
        }
    }

    #[test]
    fn generate_produces_suggestions_for_failures() {
        let checks = vec![
            create_failing_check("BUILD_FMT", CheckCategory::BuildCorrectness),
            create_failing_check("TEST_UNIT", CheckCategory::TestTruth),
        ];

        let suggestions = RemediationGenerator::generate(&checks);

        assert_eq!(suggestions.len(), 2);
        assert!(suggestions.iter().any(|s| s.check_id == "BUILD_FMT"));
        assert!(suggestions.iter().any(|s| s.check_id == "TEST_UNIT"));
    }

    #[test]
    fn generate_prioritizes_critical_issues() {
        let checks = vec![
            create_failing_check("INTENT_PRD", CheckCategory::IntentAlignment),
            create_failing_check("BUILD_FMT", CheckCategory::BuildCorrectness),
        ];

        let suggestions = RemediationGenerator::generate(&checks);

        // BUILD_FMT (Critical) should come before INTENT_PRD (Medium)
        assert_eq!(suggestions[0].check_id, "BUILD_FMT");
        assert_eq!(suggestions[0].priority, Priority::Critical);
    }

    #[test]
    fn generate_includes_automation_commands() {
        let checks = vec![create_failing_check(
            "BUILD_FMT",
            CheckCategory::BuildCorrectness,
        )];

        let suggestions = RemediationGenerator::generate(&checks);

        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].automation, Some("cargo fmt".to_string()));
    }

    #[test]
    fn generate_category_specific_suggestions() {
        let checks = vec![
            create_failing_check("BUILD_FMT", CheckCategory::BuildCorrectness),
            create_failing_check("TEST_UNIT", CheckCategory::TestTruth),
            create_failing_check("GGEN_DRY_RUN", CheckCategory::GgenPipeline),
            create_failing_check("G8_SECRETS", CheckCategory::SafetyInvariants),
        ];

        let suggestions = RemediationGenerator::generate(&checks);

        assert_eq!(suggestions.len(), 4);

        // Verify automation commands are category-specific
        let fmt_suggestion = suggestions
            .iter()
            .find(|s| s.check_id == "BUILD_FMT")
            .unwrap();
        assert_eq!(fmt_suggestion.automation, Some("cargo fmt".to_string()));

        let test_suggestion = suggestions
            .iter()
            .find(|s| s.check_id == "TEST_UNIT")
            .unwrap();
        assert_eq!(test_suggestion.automation, Some("cargo test".to_string()));

        let ggen_suggestion = suggestions
            .iter()
            .find(|s| s.check_id == "GGEN_DRY_RUN")
            .unwrap();
        assert_eq!(
            ggen_suggestion.automation,
            Some("cargo make sync".to_string())
        );
    }

    #[test]
    fn generate_skips_passing_checks() {
        let mut passing = create_failing_check("BUILD_FMT", CheckCategory::BuildCorrectness);
        passing.status = CheckStatus::Pass;

        let checks = vec![passing];

        let suggestions = RemediationGenerator::generate(&checks);

        assert_eq!(suggestions.len(), 0);
    }
}
