//! DoD validation result types
//!
//! Simplified result structure for validator output.
//! Focused on orchestrator aggregation, distinct from full artifact generation.

use super::types::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Final verdict from DoD validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    /// All fatal checks pass, meets threshold
    Pass,
    /// At least one fatal check fails
    Fail,
    /// All fatal checks pass but score below threshold
    PartialPass,
}

impl Verdict {
    /// Is this verdict considered ship-ready?
    pub fn is_ship_ready(&self) -> bool {
        matches!(self, Verdict::Pass)
    }

    /// Convert to overall verdict (binary: ready or not ready)
    pub fn to_overall_verdict(&self) -> OverallVerdict {
        match self {
            Verdict::Pass => OverallVerdict::Ready,
            Verdict::Fail | Verdict::PartialPass => OverallVerdict::NotReady,
        }
    }
}

/// Aggregated result from DoD validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DodResult {
    /// Final verdict (Pass/Fail/PartialPass)
    pub verdict: Verdict,

    /// Weighted readiness score (0.0 - 100.0)
    pub score: f64,

    /// Maximum possible score (always 100.0)
    pub max_score: f64,

    /// Individual check results
    pub checks: Vec<DodCheckResult>,

    /// Category scores (weighted)
    pub category_scores: HashMap<CheckCategory, CategoryScore>,

    /// Total execution time
    pub execution_time: Duration,

    /// Validation timestamp
    pub timestamp: DateTime<Utc>,

    /// Profile used for validation
    pub profile_name: String,

    /// Validation mode
    pub mode: ValidationMode,
}

impl DodResult {
    /// Count checks by status
    pub fn count_by_status(&self, status: CheckStatus) -> usize {
        self.checks.iter().filter(|c| c.status == status).count()
    }

    /// Get all failed checks
    pub fn failed_checks(&self) -> Vec<&DodCheckResult> {
        self.checks
            .iter()
            .filter(|c| c.status == CheckStatus::Fail)
            .collect()
    }

    /// Get all fatal failures
    pub fn fatal_failures(&self) -> Vec<&DodCheckResult> {
        self.checks
            .iter()
            .filter(|c| c.severity == CheckSeverity::Fatal && c.status == CheckStatus::Fail)
            .collect()
    }

    /// Get checks that produced warnings
    pub fn warned_checks(&self) -> Vec<&DodCheckResult> {
        self.checks
            .iter()
            .filter(|c| c.status == CheckStatus::Warn)
            .collect()
    }

    /// Get summary statistics
    pub fn summary(&self) -> ResultSummary {
        ResultSummary {
            total: self.checks.len(),
            passed: self.count_by_status(CheckStatus::Pass),
            failed: self.count_by_status(CheckStatus::Fail),
            warned: self.count_by_status(CheckStatus::Warn),
            skipped: self.count_by_status(CheckStatus::Skip),
        }
    }

    /// Check if result meets profile thresholds
    pub fn meets_threshold(&self, min_score: f64) -> bool {
        self.score >= min_score
    }

    /// Format execution time as human-readable string
    pub fn execution_time_str(&self) -> String {
        let secs = self.execution_time.as_secs();
        if secs < 60 {
            format!("{}s", secs)
        } else {
            let mins = secs / 60;
            let secs = secs % 60;
            format!("{}m {}s", mins, secs)
        }
    }
}

/// Summary statistics for DoD result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub warned: usize,
    pub skipped: usize,
}

impl ResultSummary {
    /// Calculate pass rate (0.0 - 1.0)
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.passed as f64 / self.total as f64
        }
    }

    /// Calculate fail rate (0.0 - 1.0)
    pub fn fail_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.failed as f64 / self.total as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_pass_is_ship_ready() {
        assert!(Verdict::Pass.is_ship_ready());
        assert!(!Verdict::Fail.is_ship_ready());
        assert!(!Verdict::PartialPass.is_ship_ready());
    }

    #[test]
    fn verdict_to_overall_verdict() {
        assert_eq!(Verdict::Pass.to_overall_verdict(), OverallVerdict::Ready);
        assert_eq!(Verdict::Fail.to_overall_verdict(), OverallVerdict::NotReady);
        assert_eq!(
            Verdict::PartialPass.to_overall_verdict(),
            OverallVerdict::NotReady
        );
    }

    #[test]
    fn result_counts_by_status() {
        let result = create_test_result(vec![
            CheckStatus::Pass,
            CheckStatus::Pass,
            CheckStatus::Fail,
            CheckStatus::Warn,
        ]);

        assert_eq!(result.count_by_status(CheckStatus::Pass), 2);
        assert_eq!(result.count_by_status(CheckStatus::Fail), 1);
        assert_eq!(result.count_by_status(CheckStatus::Warn), 1);
        assert_eq!(result.count_by_status(CheckStatus::Skip), 0);
    }

    #[test]
    fn result_summary() {
        let result = create_test_result(vec![
            CheckStatus::Pass,
            CheckStatus::Pass,
            CheckStatus::Fail,
        ]);

        let summary = result.summary();
        assert_eq!(summary.total, 3);
        assert_eq!(summary.passed, 2);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.pass_rate(), 2.0 / 3.0);
    }

    #[test]
    fn result_meets_threshold() {
        let result = DodResult {
            verdict: Verdict::Pass,
            score: 85.0,
            max_score: 100.0,
            checks: vec![],
            category_scores: HashMap::new(),
            execution_time: Duration::from_secs(10),
            timestamp: Utc::now(),
            profile_name: "test".to_string(),
            mode: ValidationMode::Fast,
        };

        assert!(result.meets_threshold(80.0));
        assert!(!result.meets_threshold(90.0));
    }

    #[test]
    fn result_fatal_failures() {
        let checks = vec![
            create_check("C1", CheckStatus::Pass, CheckSeverity::Fatal),
            create_check("C2", CheckStatus::Fail, CheckSeverity::Fatal),
            create_check("C3", CheckStatus::Fail, CheckSeverity::Warning),
        ];

        let result = DodResult {
            verdict: Verdict::Fail,
            score: 50.0,
            max_score: 100.0,
            checks,
            category_scores: HashMap::new(),
            execution_time: Duration::from_secs(5),
            timestamp: Utc::now(),
            profile_name: "test".to_string(),
            mode: ValidationMode::Fast,
        };

        let fatals = result.fatal_failures();
        assert_eq!(fatals.len(), 1);
        assert_eq!(fatals[0].id, "C2");
    }

    #[test]
    fn execution_time_formatting() {
        let result = DodResult {
            verdict: Verdict::Pass,
            score: 100.0,
            max_score: 100.0,
            checks: vec![],
            category_scores: HashMap::new(),
            execution_time: Duration::from_secs(125),
            timestamp: Utc::now(),
            profile_name: "test".to_string(),
            mode: ValidationMode::Fast,
        };

        assert_eq!(result.execution_time_str(), "2m 5s");

        let result_short = DodResult {
            execution_time: Duration::from_secs(45),
            ..result
        };
        assert_eq!(result_short.execution_time_str(), "45s");
    }

    // Test helpers
    fn create_test_result(statuses: Vec<CheckStatus>) -> DodResult {
        let checks = statuses
            .into_iter()
            .enumerate()
            .map(|(i, status)| create_check(&format!("C{}", i), status, CheckSeverity::Fatal))
            .collect();

        DodResult {
            verdict: Verdict::Pass,
            score: 100.0,
            max_score: 100.0,
            checks,
            category_scores: HashMap::new(),
            execution_time: Duration::from_secs(1),
            timestamp: Utc::now(),
            profile_name: "test".to_string(),
            mode: ValidationMode::Fast,
        }
    }

    fn create_check(id: &str, status: CheckStatus, severity: CheckSeverity) -> DodCheckResult {
        DodCheckResult {
            id: id.to_string(),
            category: CheckCategory::BuildCorrectness,
            status,
            severity,
            message: "test".to_string(),
            evidence: vec![],
            remediation: vec![],
            duration_ms: 0,
            check_hash: "test".to_string(),
        }
    }
}
