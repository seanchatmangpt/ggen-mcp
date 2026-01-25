//! DoD Markdown Report Generator
//!
//! Generates formatted markdown reports from DoD validation results.
//! Reports include: summary, scores, checks by category, and remediation.

use crate::dod::remediation::{Priority, RemediationGenerator};
use crate::dod::types::*;
use anyhow::Result;

/// Report generator for DoD validation results
pub struct ReportGenerator;

impl ReportGenerator {
    /// Generate formatted markdown report from validation result
    pub fn generate_markdown(result: &DodValidationResult) -> Result<String> {
        let mut report = String::new();

        // Header section
        Self::write_header(&mut report, result);

        // Summary section
        Self::write_summary(&mut report, result);

        // Category sections
        Self::write_categories(&mut report, result);

        // Remediation section (only if there are failures/warnings)
        if Self::has_issues(result) {
            Self::write_remediation(&mut report, result);
        }

        Ok(report)
    }

    /// Write report header with verdict and score
    fn write_header(report: &mut String, result: &DodValidationResult) {
        report.push_str("# Definition of Done Report\n\n");

        let verdict_emoji = match result.verdict {
            OverallVerdict::Ready => "✅",
            OverallVerdict::NotReady => "❌",
        };

        let verdict_text = match result.verdict {
            OverallVerdict::Ready => "PASS",
            OverallVerdict::NotReady => "FAIL",
        };

        report.push_str(&format!(
            "**Verdict**: {} {}\n",
            verdict_emoji, verdict_text
        ));
        report.push_str(&format!("**Score**: {:.1}/100.0\n", result.readiness_score));
        report.push_str(&format!("**Profile**: {}\n", result.profile));
        report.push_str(&format!("**Mode**: {:?}\n", result.mode));
        report.push_str(&format!("**Duration**: {}ms\n\n", result.duration_ms));
    }

    /// Write summary statistics
    fn write_summary(report: &mut String, result: &DodValidationResult) {
        report.push_str("## Summary\n\n");
        report.push_str(&format!(
            "- **Total Checks**: {}\n",
            result.summary.checks_total
        ));
        report.push_str(&format!(
            "- **Passed**: {} ✅\n",
            result.summary.checks_passed
        ));
        report.push_str(&format!(
            "- **Failed**: {} ❌\n",
            result.summary.checks_failed
        ));
        report.push_str(&format!(
            "- **Warnings**: {} ⚠️\n",
            result.summary.checks_warned
        ));
        report.push_str(&format!(
            "- **Skipped**: {} ⏭️\n\n",
            result.summary.checks_skipped
        ));
    }

    /// Write checks grouped by category
    fn write_categories(report: &mut String, result: &DodValidationResult) {
        report.push_str("## Checks by Category\n\n");

        // Define category order and labels
        let categories = vec![
            (
                CheckCategory::WorkspaceIntegrity,
                "A. Workspace Integrity (G0)",
            ),
            (CheckCategory::IntentAlignment, "B. Intent Alignment (WHY)"),
            (CheckCategory::ToolRegistry, "C. Tool Registry (WHAT)"),
            (CheckCategory::BuildCorrectness, "D. Build Correctness"),
            (CheckCategory::TestTruth, "E. Test Truth"),
            (CheckCategory::GgenPipeline, "F. Ggen Pipeline"),
            (CheckCategory::SafetyInvariants, "G. Safety Invariants"),
            (
                CheckCategory::DeploymentReadiness,
                "H. Deployment Readiness",
            ),
        ];

        for (category, label) in categories {
            Self::write_category_section(report, result, category, label);
        }
    }

    /// Write single category section
    fn write_category_section(
        report: &mut String,
        result: &DodValidationResult,
        category: CheckCategory,
        label: &str,
    ) {
        // Get checks for this category
        let category_checks: Vec<_> = result
            .check_results
            .iter()
            .filter(|c| c.category == category)
            .collect();

        if category_checks.is_empty() {
            return;
        }

        report.push_str(&format!("### {}\n\n", label));

        // Add category score if available
        if let Some(score) = result.category_scores.get(&category) {
            if score.weight > 0.0 {
                report.push_str(&format!(
                    "**Score**: {:.1}/100.0 (weight: {:.0}%)\n\n",
                    score.score,
                    score.weight * 100.0
                ));
            }
        }

        // Table header
        report.push_str("| Check | Verdict | Severity | Message |\n");
        report.push_str("|-------|---------|----------|----------|\n");

        // Table rows
        for check in category_checks {
            let verdict_emoji = Self::status_emoji(&check.status);
            let severity_text = Self::severity_text(check.severity);

            report.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                check.id,
                verdict_emoji,
                severity_text,
                Self::escape_markdown(&check.message)
            ));
        }

        report.push_str("\n");
    }

    /// Write remediation section
    fn write_remediation(report: &mut String, result: &DodValidationResult) {
        report.push_str("## Remediation\n\n");

        let suggestions = RemediationGenerator::generate(&result.check_results);

        if suggestions.is_empty() {
            report.push_str("*No remediation needed - all checks passed!*\n\n");
            return;
        }

        report.push_str("Address the following issues to pass all checks:\n\n");

        // Group by priority
        let mut critical: Vec<_> = suggestions
            .iter()
            .filter(|s| s.priority == Priority::Critical)
            .collect();
        let mut high: Vec<_> = suggestions
            .iter()
            .filter(|s| s.priority == Priority::High)
            .collect();
        let mut medium: Vec<_> = suggestions
            .iter()
            .filter(|s| s.priority == Priority::Medium)
            .collect();
        let mut low: Vec<_> = suggestions
            .iter()
            .filter(|s| s.priority == Priority::Low)
            .collect();

        if !critical.is_empty() {
            report.push_str("### 🚨 Critical Priority\n\n");
            for suggestion in critical {
                Self::write_suggestion(report, suggestion);
            }
        }

        if !high.is_empty() {
            report.push_str("### ⚠️ High Priority\n\n");
            for suggestion in high {
                Self::write_suggestion(report, suggestion);
            }
        }

        if !medium.is_empty() {
            report.push_str("### 📋 Medium Priority\n\n");
            for suggestion in medium {
                Self::write_suggestion(report, suggestion);
            }
        }

        if !low.is_empty() {
            report.push_str("### 💡 Low Priority\n\n");
            for suggestion in low {
                Self::write_suggestion(report, suggestion);
            }
        }
    }

    /// Write single remediation suggestion
    fn write_suggestion(
        report: &mut String,
        suggestion: &crate::dod::remediation::RemediationSuggestion,
    ) {
        report.push_str(&format!("#### {}\n\n", suggestion.title));
        report.push_str(&format!("**Check**: `{}`\n\n", suggestion.check_id));

        if !suggestion.steps.is_empty() {
            report.push_str("**Steps**:\n");
            for step in &suggestion.steps {
                report.push_str(&format!("- {}\n", step));
            }
            report.push_str("\n");
        }

        if let Some(ref automation) = suggestion.automation {
            report.push_str(&format!("**Quick Fix**: `{}`\n\n", automation));
        }
    }

    /// Get emoji for check status
    fn status_emoji(status: &CheckStatus) -> &'static str {
        match status {
            CheckStatus::Pass => "✅ Pass",
            CheckStatus::Fail => "❌ Fail",
            CheckStatus::Warn => "⚠️ Warning",
            CheckStatus::Skip => "⏭️ Skip",
        }
    }

    /// Get text for severity
    fn severity_text(severity: CheckSeverity) -> &'static str {
        match severity {
            CheckSeverity::Fatal => "Fatal",
            CheckSeverity::Warning => "Warning",
            CheckSeverity::Info => "Info",
        }
    }

    /// Escape markdown special characters in text
    fn escape_markdown(text: &str) -> String {
        text.replace('|', "\\|")
            .replace('\n', " ")
            .replace('\r', "")
    }

    /// Check if result has any issues (failures or warnings)
    fn has_issues(result: &DodValidationResult) -> bool {
        result.summary.checks_failed > 0 || result.summary.checks_warned > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_result() -> DodValidationResult {
        let mut category_scores = HashMap::new();
        category_scores.insert(
            CheckCategory::BuildCorrectness,
            CategoryScore {
                category: CheckCategory::BuildCorrectness,
                score: 100.0,
                weight: 0.25,
                checks_passed: 2,
                checks_failed: 0,
                checks_warned: 0,
                checks_skipped: 0,
            },
        );

        DodValidationResult {
            verdict: OverallVerdict::Ready,
            readiness_score: 95.0,
            profile: "dev".to_string(),
            mode: ValidationMode::Fast,
            summary: ValidationSummary {
                checks_total: 2,
                checks_passed: 2,
                checks_failed: 0,
                checks_warned: 0,
                checks_skipped: 0,
            },
            category_scores,
            check_results: vec![
                DodCheckResult {
                    id: "BUILD_CHECK".to_string(),
                    category: CheckCategory::BuildCorrectness,
                    status: CheckStatus::Pass,
                    severity: CheckSeverity::Fatal,
                    message: "Build successful".to_string(),
                    evidence: vec![],
                    remediation: vec![],
                    duration_ms: 100,
                    check_hash: "abc123".to_string(),
                },
                DodCheckResult {
                    id: "BUILD_FMT".to_string(),
                    category: CheckCategory::BuildCorrectness,
                    status: CheckStatus::Pass,
                    severity: CheckSeverity::Fatal,
                    message: "Code formatted correctly".to_string(),
                    evidence: vec![],
                    remediation: vec![],
                    duration_ms: 50,
                    check_hash: "def456".to_string(),
                },
            ],
            artifacts: ArtifactPaths {
                receipt_path: PathBuf::from("./receipts/latest.json"),
                report_path: PathBuf::from("./reports/latest.md"),
                bundle_path: None,
            },
            duration_ms: 150,
        }
    }

    #[test]
    fn generates_basic_report() {
        let result = create_test_result();
        let report = ReportGenerator::generate_markdown(&result).unwrap();

        assert!(report.contains("# Definition of Done Report"));
        assert!(report.contains("✅ PASS"));
        assert!(report.contains("95.0/100.0"));
    }

    #[test]
    fn includes_summary_section() {
        let result = create_test_result();
        let report = ReportGenerator::generate_markdown(&result).unwrap();

        assert!(report.contains("## Summary"));
        assert!(report.contains("Total Checks**: 2"));
        assert!(report.contains("Passed**: 2 ✅"));
        assert!(report.contains("Failed**: 0 ❌"));
    }

    #[test]
    fn includes_category_sections() {
        let result = create_test_result();
        let report = ReportGenerator::generate_markdown(&result).unwrap();

        assert!(report.contains("## Checks by Category"));
        assert!(report.contains("### D. Build Correctness"));
        assert!(report.contains("BUILD_CHECK"));
        assert!(report.contains("BUILD_FMT"));
    }

    #[test]
    fn shows_table_with_checks() {
        let result = create_test_result();
        let report = ReportGenerator::generate_markdown(&result).unwrap();

        assert!(report.contains("| Check | Verdict | Severity | Message |"));
        assert!(report.contains("| BUILD_CHECK | ✅ Pass | Fatal | Build successful |"));
    }

    #[test]
    fn escapes_markdown_in_messages() {
        let mut result = create_test_result();
        result.check_results[0].message = "Error | with | pipes".to_string();

        let report = ReportGenerator::generate_markdown(&result).unwrap();
        assert!(report.contains("Error \\| with \\| pipes"));
    }

    #[test]
    fn shows_category_scores() {
        let result = create_test_result();
        let report = ReportGenerator::generate_markdown(&result).unwrap();

        assert!(report.contains("**Score**: 100.0/100.0 (weight: 25%)"));
    }

    #[test]
    fn verdict_not_ready_shows_fail() {
        let mut result = create_test_result();
        result.verdict = OverallVerdict::NotReady;
        result.check_results[0].status = CheckStatus::Fail;
        result.summary.checks_passed = 1;
        result.summary.checks_failed = 1;

        let report = ReportGenerator::generate_markdown(&result).unwrap();

        assert!(report.contains("❌ FAIL"));
        assert!(report.contains("❌ Fail"));
    }

    #[test]
    fn includes_remediation_for_failures() {
        let mut result = create_test_result();
        result.verdict = OverallVerdict::NotReady;
        result.check_results[0].status = CheckStatus::Fail;
        result.check_results[0].remediation = vec!["Fix the build".to_string()];
        result.summary.checks_passed = 1;
        result.summary.checks_failed = 1;

        let report = ReportGenerator::generate_markdown(&result).unwrap();

        assert!(report.contains("## Remediation"));
    }

    #[test]
    fn no_remediation_when_all_pass() {
        let result = create_test_result();
        let report = ReportGenerator::generate_markdown(&result).unwrap();

        // Should not have remediation section for passing checks
        assert!(!report.contains("## Remediation"));
    }

    #[test]
    fn handles_warnings() {
        let mut result = create_test_result();
        result.check_results[0].status = CheckStatus::Warn;
        result.check_results[0].severity = CheckSeverity::Warning;
        result.summary.checks_passed = 1;
        result.summary.checks_warned = 1;

        let report = ReportGenerator::generate_markdown(&result).unwrap();

        assert!(report.contains("⚠️ Warning"));
        assert!(report.contains("Warnings**: 1 ⚠️"));
    }

    #[test]
    fn handles_skipped_checks() {
        let mut result = create_test_result();
        result.check_results.push(DodCheckResult {
            id: "OPTIONAL_CHECK".to_string(),
            category: CheckCategory::BuildCorrectness,
            status: CheckStatus::Skip,
            severity: CheckSeverity::Info,
            message: "Skipped - not required".to_string(),
            evidence: vec![],
            remediation: vec![],
            duration_ms: 0,
            check_hash: "skip123".to_string(),
        });
        result.summary.checks_total = 3;
        result.summary.checks_skipped = 1;

        let report = ReportGenerator::generate_markdown(&result).unwrap();

        assert!(report.contains("⏭️ Skip"));
        assert!(report.contains("Skipped**: 1 ⏭️"));
    }

    #[test]
    fn shows_all_categories() {
        let mut result = create_test_result();

        // Add checks for each category
        let categories = vec![
            (CheckCategory::WorkspaceIntegrity, "WORKSPACE_CHECK"),
            (CheckCategory::IntentAlignment, "INTENT_CHECK"),
            (CheckCategory::ToolRegistry, "TOOL_CHECK"),
            (CheckCategory::TestTruth, "TEST_CHECK"),
            (CheckCategory::GgenPipeline, "GGEN_CHECK"),
            (CheckCategory::SafetyInvariants, "SAFETY_CHECK"),
            (CheckCategory::DeploymentReadiness, "DEPLOY_CHECK"),
        ];

        for (category, id) in categories {
            result.check_results.push(DodCheckResult {
                id: id.to_string(),
                category,
                status: CheckStatus::Pass,
                severity: CheckSeverity::Fatal,
                message: "Passed".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms: 10,
                check_hash: "test".to_string(),
            });
        }

        let report = ReportGenerator::generate_markdown(&result).unwrap();

        assert!(report.contains("### A. Workspace Integrity (G0)"));
        assert!(report.contains("### B. Intent Alignment (WHY)"));
        assert!(report.contains("### C. Tool Registry (WHAT)"));
        assert!(report.contains("### D. Build Correctness"));
        assert!(report.contains("### E. Test Truth"));
        assert!(report.contains("### F. Ggen Pipeline"));
        assert!(report.contains("### G. Safety Invariants"));
        assert!(report.contains("### H. Deployment Readiness"));
    }
}
