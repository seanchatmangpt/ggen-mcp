use super::types::*;

/// Compute overall verdict using severity-first logic
/// Rule: Any fatal fail → NOT_READY
pub fn compute_verdict(check_results: &[DodCheckResult]) -> OverallVerdict {
    let has_fatal_fail = check_results.iter().any(|check| {
        check.severity == CheckSeverity::Fatal && check.status == CheckStatus::Fail
    });

    if has_fatal_fail {
        OverallVerdict::NotReady
    } else {
        OverallVerdict::Ready
    }
}

/// Get all fatal failures
pub fn get_fatal_failures(check_results: &[DodCheckResult]) -> Vec<&DodCheckResult> {
    check_results
        .iter()
        .filter(|check| {
            check.severity == CheckSeverity::Fatal && check.status == CheckStatus::Fail
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_ready_when_no_fatal_failures() {
        let checks = vec![
            create_check("CHECK1", CheckStatus::Pass, CheckSeverity::Fatal),
            create_check("CHECK2", CheckStatus::Warn, CheckSeverity::Warning),
        ];
        assert_eq!(compute_verdict(&checks), OverallVerdict::Ready);
    }

    #[test]
    fn verdict_not_ready_when_fatal_failure() {
        let checks = vec![
            create_check("CHECK1", CheckStatus::Pass, CheckSeverity::Fatal),
            create_check("CHECK2", CheckStatus::Fail, CheckSeverity::Fatal),
        ];
        assert_eq!(compute_verdict(&checks), OverallVerdict::NotReady);
    }

    #[test]
    fn verdict_ready_with_non_fatal_failures() {
        let checks = vec![
            create_check("CHECK1", CheckStatus::Fail, CheckSeverity::Warning),
            create_check("CHECK2", CheckStatus::Pass, CheckSeverity::Fatal),
        ];
        assert_eq!(compute_verdict(&checks), OverallVerdict::Ready);
    }

    #[test]
    fn get_fatal_failures_filters_correctly() {
        let checks = vec![
            create_check("CHECK1", CheckStatus::Pass, CheckSeverity::Fatal),
            create_check("CHECK2", CheckStatus::Fail, CheckSeverity::Fatal),
            create_check("CHECK3", CheckStatus::Fail, CheckSeverity::Warning),
        ];
        let fatal = get_fatal_failures(&checks);
        assert_eq!(fatal.len(), 1);
        assert_eq!(fatal[0].id, "CHECK2");
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
