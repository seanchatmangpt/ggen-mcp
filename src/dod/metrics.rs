//! DoD Metrics Collection and Reporting
//!
//! Provides performance metrics for DoD validation system.
//! Integrates with OpenTelemetry for distributed tracing and Prometheus for metrics.

use crate::dod::types::*;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Performance metrics for DoD validation run
#[derive(Debug, Clone)]
pub struct DodMetrics {
    /// Total duration from start to finish
    pub total_duration: Duration,

    /// Per-check execution durations
    pub check_durations: HashMap<String, Duration>,

    /// Per-category aggregate durations
    pub category_durations: HashMap<CheckCategory, Duration>,

    /// Total evidence size in bytes
    pub evidence_size_bytes: u64,

    /// Number of checks executed
    pub checks_executed: usize,

    /// Number of checks that passed
    pub checks_passed: usize,

    /// Number of checks that failed
    pub checks_failed: usize,

    /// Number of checks that warned
    pub checks_warned: usize,

    /// Number of checks that were skipped
    pub checks_skipped: usize,

    /// Timestamp when metrics collection started
    pub start_time: Instant,

    /// Timestamp when metrics collection ended
    pub end_time: Option<Instant>,
}

impl DodMetrics {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            total_duration: Duration::ZERO,
            check_durations: HashMap::new(),
            category_durations: HashMap::new(),
            evidence_size_bytes: 0,
            checks_executed: 0,
            checks_passed: 0,
            checks_failed: 0,
            checks_warned: 0,
            checks_skipped: 0,
            start_time: Instant::now(),
            end_time: None,
        }
    }

    /// Compute metrics from validation result
    pub fn from_validation_result(result: &DodValidationResult) -> Self {
        let mut metrics = Self::new();

        metrics.total_duration = Duration::from_millis(result.duration_ms);
        metrics.checks_executed = result.check_results.len();
        metrics.checks_passed = result.summary.checks_passed;
        metrics.checks_failed = result.summary.checks_failed;
        metrics.checks_warned = result.summary.checks_warned;
        metrics.checks_skipped = result.summary.checks_skipped;

        // Collect per-check durations
        for check_result in &result.check_results {
            metrics.check_durations.insert(
                check_result.id.clone(),
                Duration::from_millis(check_result.duration_ms),
            );

            // Aggregate category durations
            let category_duration = metrics
                .category_durations
                .entry(check_result.category)
                .or_insert(Duration::ZERO);
            *category_duration += Duration::from_millis(check_result.duration_ms);

            // Calculate evidence size
            for evidence in &check_result.evidence {
                metrics.evidence_size_bytes += evidence.content.len() as u64;
                if let Some(path) = &evidence.file_path {
                    metrics.evidence_size_bytes += path.to_string_lossy().len() as u64;
                }
            }
        }

        metrics.end_time = Some(metrics.start_time + metrics.total_duration);
        metrics
    }

    /// Get average check duration
    pub fn average_check_duration(&self) -> Duration {
        if self.checks_executed == 0 {
            return Duration::ZERO;
        }
        self.total_duration / self.checks_executed as u32
    }

    /// Get slowest check
    pub fn slowest_check(&self) -> Option<(String, Duration)> {
        self.check_durations
            .iter()
            .max_by_key(|(_, duration)| *duration)
            .map(|(id, duration)| (id.clone(), *duration))
    }

    /// Get fastest check
    pub fn fastest_check(&self) -> Option<(String, Duration)> {
        self.check_durations
            .iter()
            .min_by_key(|(_, duration)| *duration)
            .map(|(id, duration)| (id.clone(), *duration))
    }

    /// Get slowest category
    pub fn slowest_category(&self) -> Option<(CheckCategory, Duration)> {
        self.category_durations
            .iter()
            .max_by_key(|(_, duration)| *duration)
            .map(|(category, duration)| (*category, *duration))
    }

    /// Get success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.checks_executed == 0 {
            return 0.0;
        }
        self.checks_passed as f64 / self.checks_executed as f64
    }

    /// Get failure rate (0.0 to 1.0)
    pub fn failure_rate(&self) -> f64 {
        if self.checks_executed == 0 {
            return 0.0;
        }
        self.checks_failed as f64 / self.checks_executed as f64
    }

    /// Format metrics as human-readable summary
    pub fn format_summary(&self) -> String {
        let mut summary = String::new();

        summary.push_str(&format!("DoD Metrics Summary\n"));
        summary.push_str(&format!("==================\n\n"));

        summary.push_str(&format!("Total Duration: {:.2}s\n", self.total_duration.as_secs_f64()));
        summary.push_str(&format!("Checks Executed: {}\n", self.checks_executed));
        summary.push_str(&format!("  - Passed: {}\n", self.checks_passed));
        summary.push_str(&format!("  - Failed: {}\n", self.checks_failed));
        summary.push_str(&format!("  - Warned: {}\n", self.checks_warned));
        summary.push_str(&format!("  - Skipped: {}\n", self.checks_skipped));
        summary.push_str(&format!("Success Rate: {:.1}%\n", self.success_rate() * 100.0));
        summary.push_str(&format!("Evidence Size: {} bytes\n\n", self.evidence_size_bytes));

        if let Some((check_id, duration)) = self.slowest_check() {
            summary.push_str(&format!("Slowest Check: {} ({:.2}s)\n", check_id, duration.as_secs_f64()));
        }

        if let Some((category, duration)) = self.slowest_category() {
            summary.push_str(&format!("Slowest Category: {:?} ({:.2}s)\n", category, duration.as_secs_f64()));
        }

        summary.push_str(&format!("Average Check Duration: {:.2}s\n", self.average_check_duration().as_secs_f64()));

        summary
    }

    /// Export metrics in Prometheus format
    pub fn to_prometheus(&self) -> String {
        let mut output = String::new();

        output.push_str("# HELP dod_validation_duration_seconds Total DoD validation duration\n");
        output.push_str("# TYPE dod_validation_duration_seconds gauge\n");
        output.push_str(&format!("dod_validation_duration_seconds {}\n", self.total_duration.as_secs_f64()));

        output.push_str("# HELP dod_checks_total Total number of checks executed\n");
        output.push_str("# TYPE dod_checks_total counter\n");
        output.push_str(&format!("dod_checks_total {}\n", self.checks_executed));

        output.push_str("# HELP dod_checks_passed Number of checks that passed\n");
        output.push_str("# TYPE dod_checks_passed counter\n");
        output.push_str(&format!("dod_checks_passed {}\n", self.checks_passed));

        output.push_str("# HELP dod_checks_failed Number of checks that failed\n");
        output.push_str("# TYPE dod_checks_failed counter\n");
        output.push_str(&format!("dod_checks_failed {}\n", self.checks_failed));

        output.push_str("# HELP dod_evidence_bytes Total evidence size in bytes\n");
        output.push_str("# TYPE dod_evidence_bytes gauge\n");
        output.push_str(&format!("dod_evidence_bytes {}\n", self.evidence_size_bytes));

        output.push_str("# HELP dod_success_rate Success rate (0.0 to 1.0)\n");
        output.push_str("# TYPE dod_success_rate gauge\n");
        output.push_str(&format!("dod_success_rate {}\n", self.success_rate()));

        // Per-category durations
        for (category, duration) in &self.category_durations {
            output.push_str(&format!(
                "dod_category_duration_seconds{{category=\"{:?}\"}} {}\n",
                category,
                duration.as_secs_f64()
            ));
        }

        output
    }
}

impl Default for DodMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// OpenTelemetry span builder for DoD operations
pub struct DodSpan;

impl DodSpan {
    /// Create span for full validation run
    pub fn validation_run(profile: &str) -> tracing::Span {
        tracing::info_span!(
            "dod_validation",
            profile = profile,
            otel.name = "dod.validation",
            otel.kind = "internal"
        )
    }

    /// Create span for check execution
    pub fn check_execution(check_id: &str, category: CheckCategory) -> tracing::Span {
        tracing::debug_span!(
            "dod_check",
            check_id = check_id,
            category = ?category,
            otel.name = "dod.check.execute",
            otel.kind = "internal"
        )
    }

    /// Create span for evidence collection
    pub fn evidence_collection(check_id: &str) -> tracing::Span {
        tracing::debug_span!(
            "dod_evidence",
            check_id = check_id,
            otel.name = "dod.evidence.collect",
            otel.kind = "internal"
        )
    }

    /// Create span for report generation
    pub fn report_generation(format: &str) -> tracing::Span {
        tracing::info_span!(
            "dod_report",
            format = format,
            otel.name = "dod.report.generate",
            otel.kind = "internal"
        )
    }

    /// Create span for receipt generation
    pub fn receipt_generation() -> tracing::Span {
        tracing::info_span!(
            "dod_receipt",
            otel.name = "dod.receipt.generate",
            otel.kind = "internal"
        )
    }
}

/// Metrics recorder that integrates with Prometheus
pub struct MetricsRecorder {
    /// Histogram of check execution durations
    check_durations: HashMap<String, Vec<f64>>,

    /// Counter of verdicts
    verdict_counts: HashMap<String, u64>,
}

impl MetricsRecorder {
    /// Create new metrics recorder
    pub fn new() -> Self {
        Self {
            check_durations: HashMap::new(),
            verdict_counts: HashMap::new(),
        }
    }

    /// Record check execution
    pub fn record_check(&mut self, check_id: &str, duration: Duration) {
        self.check_durations
            .entry(check_id.to_string())
            .or_insert_with(Vec::new)
            .push(duration.as_secs_f64());
    }

    /// Record verdict
    pub fn record_verdict(&mut self, verdict: OverallVerdict) {
        let verdict_str = match verdict {
            OverallVerdict::Ready => "ready",
            OverallVerdict::NotReady => "not_ready",
        };

        *self.verdict_counts.entry(verdict_str.to_string()).or_insert(0) += 1;
    }

    /// Get percentile for check duration (p50, p95, p99)
    pub fn get_percentile(&self, check_id: &str, percentile: f64) -> Option<Duration> {
        let durations = self.check_durations.get(check_id)?;
        if durations.is_empty() {
            return None;
        }

        let mut sorted = durations.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let idx = ((percentile / 100.0) * sorted.len() as f64) as usize;
        let idx = idx.min(sorted.len() - 1);

        Some(Duration::from_secs_f64(sorted[idx]))
    }

    /// Clear all recorded metrics
    pub fn clear(&mut self) {
        self.check_durations.clear();
        self.verdict_counts.clear();
    }
}

impl Default for MetricsRecorder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_validation_result() -> DodValidationResult {
        let check_result1 = DodCheckResult {
            id: "BUILD_CHECK".to_string(),
            category: CheckCategory::BuildCorrectness,
            status: CheckStatus::Pass,
            severity: CheckSeverity::Fatal,
            message: "Build succeeded".to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::CommandOutput,
                content: "cargo build succeeded".to_string(),
                file_path: None,
                line_number: None,
                hash: "abc123".to_string(),
            }],
            remediation: vec![],
            duration_ms: 1000,
            check_hash: "hash1".to_string(),
        };

        let check_result2 = DodCheckResult {
            id: "TEST_UNIT".to_string(),
            category: CheckCategory::TestTruth,
            status: CheckStatus::Pass,
            severity: CheckSeverity::Fatal,
            message: "Tests passed".to_string(),
            evidence: vec![],
            remediation: vec![],
            duration_ms: 2000,
            check_hash: "hash2".to_string(),
        };

        DodValidationResult {
            verdict: OverallVerdict::Ready,
            readiness_score: 100.0,
            profile: "test".to_string(),
            mode: ValidationMode::Fast,
            summary: ValidationSummary {
                checks_total: 2,
                checks_passed: 2,
                checks_failed: 0,
                checks_warned: 0,
                checks_skipped: 0,
            },
            category_scores: HashMap::new(),
            check_results: vec![check_result1, check_result2],
            artifacts: ArtifactPaths {
                receipt_path: PathBuf::from("receipt.json"),
                report_path: PathBuf::from("report.md"),
                bundle_path: None,
            },
            duration_ms: 3000,
        }
    }

    #[test]
    fn metrics_from_validation_result() {
        let result = create_test_validation_result();
        let metrics = DodMetrics::from_validation_result(&result);

        assert_eq!(metrics.checks_executed, 2);
        assert_eq!(metrics.checks_passed, 2);
        assert_eq!(metrics.checks_failed, 0);
        assert_eq!(metrics.total_duration, Duration::from_millis(3000));
        assert!(metrics.check_durations.contains_key("BUILD_CHECK"));
        assert!(metrics.check_durations.contains_key("TEST_UNIT"));
    }

    #[test]
    fn metrics_calculates_success_rate() {
        let result = create_test_validation_result();
        let metrics = DodMetrics::from_validation_result(&result);

        assert_eq!(metrics.success_rate(), 1.0);
        assert_eq!(metrics.failure_rate(), 0.0);
    }

    #[test]
    fn metrics_finds_slowest_check() {
        let result = create_test_validation_result();
        let metrics = DodMetrics::from_validation_result(&result);

        let (slowest_id, duration) = metrics.slowest_check().unwrap();
        assert_eq!(slowest_id, "TEST_UNIT");
        assert_eq!(duration, Duration::from_millis(2000));
    }

    #[test]
    fn metrics_finds_fastest_check() {
        let result = create_test_validation_result();
        let metrics = DodMetrics::from_validation_result(&result);

        let (fastest_id, duration) = metrics.fastest_check().unwrap();
        assert_eq!(fastest_id, "BUILD_CHECK");
        assert_eq!(duration, Duration::from_millis(1000));
    }

    #[test]
    fn metrics_calculates_average_duration() {
        let result = create_test_validation_result();
        let metrics = DodMetrics::from_validation_result(&result);

        assert_eq!(metrics.average_check_duration(), Duration::from_millis(1500));
    }

    #[test]
    fn metrics_tracks_evidence_size() {
        let result = create_test_validation_result();
        let metrics = DodMetrics::from_validation_result(&result);

        // "cargo build succeeded" = 22 bytes
        assert_eq!(metrics.evidence_size_bytes, 22);
    }

    #[test]
    fn metrics_summary_is_formatted() {
        let result = create_test_validation_result();
        let metrics = DodMetrics::from_validation_result(&result);

        let summary = metrics.format_summary();
        assert!(summary.contains("DoD Metrics Summary"));
        assert!(summary.contains("Total Duration"));
        assert!(summary.contains("Checks Executed: 2"));
        assert!(summary.contains("Success Rate: 100.0%"));
    }

    #[test]
    fn metrics_exports_prometheus_format() {
        let result = create_test_validation_result();
        let metrics = DodMetrics::from_validation_result(&result);

        let prom = metrics.to_prometheus();
        assert!(prom.contains("dod_validation_duration_seconds"));
        assert!(prom.contains("dod_checks_total 2"));
        assert!(prom.contains("dod_checks_passed 2"));
        assert!(prom.contains("dod_success_rate 1"));
    }

    #[test]
    fn recorder_tracks_check_durations() {
        let mut recorder = MetricsRecorder::new();

        recorder.record_check("CHECK_1", Duration::from_millis(100));
        recorder.record_check("CHECK_1", Duration::from_millis(200));
        recorder.record_check("CHECK_1", Duration::from_millis(150));

        let p50 = recorder.get_percentile("CHECK_1", 50.0).unwrap();
        assert_eq!(p50, Duration::from_millis(150));
    }

    #[test]
    fn recorder_tracks_verdicts() {
        let mut recorder = MetricsRecorder::new();

        recorder.record_verdict(OverallVerdict::Ready);
        recorder.record_verdict(OverallVerdict::Ready);
        recorder.record_verdict(OverallVerdict::NotReady);

        assert_eq!(recorder.verdict_counts.get("ready"), Some(&2));
        assert_eq!(recorder.verdict_counts.get("not_ready"), Some(&1));
    }

    #[test]
    fn recorder_clears_metrics() {
        let mut recorder = MetricsRecorder::new();

        recorder.record_check("CHECK_1", Duration::from_millis(100));
        recorder.record_verdict(OverallVerdict::Ready);

        recorder.clear();

        assert!(recorder.check_durations.is_empty());
        assert!(recorder.verdict_counts.is_empty());
    }
}
