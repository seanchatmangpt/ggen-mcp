use super::types::*;
use std::collections::HashMap;

/// Scorer for calculating readiness scores
pub struct Scorer;

impl Scorer {
    pub fn new() -> Self {
        Self
    }

    /// Calculate overall readiness score (0-100) as u8
    pub fn calculate_score(&self, check_results: &[DodCheckResult]) -> u8 {
        // Group by category and compute category scores
        let mut category_scores = HashMap::new();
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
            let score = compute_category_score(category, check_results);
            category_scores.insert(category, score);
        }

        // Compute overall readiness score
        let readiness = compute_readiness_score(&category_scores);
        readiness.round() as u8
    }
}

/// Default category weights (must sum to 1.0)
/// From PRD: Build 25%, Tests 25%, ggen 20%, alignment 15%, safety 10%, why 5%
pub fn default_category_weights() -> HashMap<CheckCategory, f64> {
    let mut weights = HashMap::new();
    weights.insert(CheckCategory::BuildCorrectness, 0.25);
    weights.insert(CheckCategory::TestTruth, 0.25);
    weights.insert(CheckCategory::GgenPipeline, 0.20);
    weights.insert(CheckCategory::ToolRegistry, 0.15);
    weights.insert(CheckCategory::SafetyInvariants, 0.10);
    weights.insert(CheckCategory::IntentAlignment, 0.05);
    weights.insert(CheckCategory::WorkspaceIntegrity, 0.0); // Gating only, not scored
    weights.insert(CheckCategory::DeploymentReadiness, 0.0); // Gating only
    weights
}

/// Compute category score (0-100)
pub fn compute_category_score(
    category: CheckCategory,
    check_results: &[DodCheckResult],
) -> CategoryScore {
    let category_checks: Vec<_> = check_results
        .iter()
        .filter(|c| c.category == category)
        .collect();

    if category_checks.is_empty() {
        return CategoryScore {
            category,
            score: 0.0,
            weight: 0.0,
            checks_passed: 0,
            checks_failed: 0,
            checks_warned: 0,
            checks_skipped: 0,
        };
    }

    let passed = category_checks
        .iter()
        .filter(|c| c.status == CheckStatus::Pass)
        .count();
    let failed = category_checks
        .iter()
        .filter(|c| c.status == CheckStatus::Fail)
        .count();
    let warned = category_checks
        .iter()
        .filter(|c| c.status == CheckStatus::Warn)
        .count();
    let skipped = category_checks
        .iter()
        .filter(|c| c.status == CheckStatus::Skip)
        .count();

    // Score = (passed / (passed + failed)) * 100
    // Warnings reduce score by 2% each, skipped not counted
    let total_evaluated = passed + failed;
    let base_score = if total_evaluated > 0 {
        (passed as f64 / total_evaluated as f64) * 100.0
    } else {
        100.0
    };

    // Penalty: 2 points per warning (max 20% reduction)
    let warning_penalty = (warned as f64 * 2.0).min(20.0);
    let score = (base_score - warning_penalty).max(0.0);

    let weights = default_category_weights();
    let weight = weights.get(&category).copied().unwrap_or(0.0);

    CategoryScore {
        category,
        score,
        weight,
        checks_passed: passed,
        checks_failed: failed,
        checks_warned: warned,
        checks_skipped: skipped,
    }
}

/// Compute overall readiness score (0-100) using weighted average
pub fn compute_readiness_score(category_scores: &HashMap<CheckCategory, CategoryScore>) -> f64 {
    let weighted_sum: f64 = category_scores
        .values()
        .map(|cs| cs.score * cs.weight)
        .sum();

    weighted_sum.min(100.0).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_weights_sum_to_one() {
        let weights = default_category_weights();
        let sum: f64 = weights.values().sum();
        assert!(
            (sum - 1.0).abs() < 0.001,
            "Weights sum to {}, expected 1.0",
            sum
        );
    }

    #[test]
    fn category_score_perfect() {
        let checks = vec![
            create_check(CheckCategory::BuildCorrectness, CheckStatus::Pass),
            create_check(CheckCategory::BuildCorrectness, CheckStatus::Pass),
        ];
        let score = compute_category_score(CheckCategory::BuildCorrectness, &checks);
        assert_eq!(score.score, 100.0);
        assert_eq!(score.checks_passed, 2);
        assert_eq!(score.checks_failed, 0);
    }

    #[test]
    fn category_score_with_failure() {
        let checks = vec![
            create_check(CheckCategory::BuildCorrectness, CheckStatus::Pass),
            create_check(CheckCategory::BuildCorrectness, CheckStatus::Fail),
        ];
        let score = compute_category_score(CheckCategory::BuildCorrectness, &checks);
        assert_eq!(score.score, 50.0);
    }

    #[test]
    fn category_score_with_warnings() {
        let checks = vec![
            create_check(CheckCategory::BuildCorrectness, CheckStatus::Pass),
            create_check(CheckCategory::BuildCorrectness, CheckStatus::Pass),
            create_check(CheckCategory::BuildCorrectness, CheckStatus::Warn),
        ];
        let score = compute_category_score(CheckCategory::BuildCorrectness, &checks);
        // Base: 100%, Warning penalty: 2 points
        assert_eq!(score.score, 98.0);
    }

    #[test]
    fn category_score_empty() {
        let checks = vec![];
        let score = compute_category_score(CheckCategory::BuildCorrectness, &checks);
        assert_eq!(score.score, 0.0);
        assert_eq!(score.checks_passed, 0);
    }

    #[test]
    fn readiness_score_weighted() {
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
        category_scores.insert(
            CheckCategory::TestTruth,
            CategoryScore {
                category: CheckCategory::TestTruth,
                score: 80.0,
                weight: 0.25,
                checks_passed: 4,
                checks_failed: 1,
                checks_warned: 0,
                checks_skipped: 0,
            },
        );
        let readiness = compute_readiness_score(&category_scores);
        // (100 * 0.25) + (80 * 0.25) = 25 + 20 = 45
        assert_eq!(readiness, 45.0);
    }

    fn create_check(category: CheckCategory, status: CheckStatus) -> DodCheckResult {
        DodCheckResult {
            id: "TEST".to_string(),
            category,
            status,
            severity: CheckSeverity::Fatal,
            message: "test".to_string(),
            evidence: vec![],
            remediation: vec![],
            duration_ms: 0,
            check_hash: "test".to_string(),
        }
    }
}
