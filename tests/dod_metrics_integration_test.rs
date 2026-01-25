//! DoD Metrics Integration Test
//!
//! Validates that metrics collection integrates properly with the DoD system.

use anyhow::Result;
use spreadsheet_mcp::dod::*;
use std::path::PathBuf;
use std::time::Duration;

#[tokio::test]
async fn test_metrics_integration_end_to_end() -> Result<()> {
    // Create full DoD validation pipeline
    let mut registry = CheckRegistry::new();

    // Register workspace checks
    registry.register_all(&spreadsheet_mcp::dod::checks::workspace::get_workspace_checks());

    // Register build checks
    registry.register_all(&spreadsheet_mcp::dod::checks::build::get_build_checks());

    let profile = DodProfile::default_dev();
    let executor = CheckExecutor::new(registry, profile);

    let context = CheckContext {
        workspace_root: PathBuf::from("."),
        mode: ValidationMode::Fast,
        timeout_ms: 60_000,
    };

    // Execute validation
    let check_results = executor.execute_all(&context).await?;

    // Build validation result
    let summary = ValidationSummary {
        checks_total: check_results.len(),
        checks_passed: check_results
            .iter()
            .filter(|r| r.status == CheckStatus::Pass)
            .count(),
        checks_failed: check_results
            .iter()
            .filter(|r| r.status == CheckStatus::Fail)
            .count(),
        checks_warned: check_results
            .iter()
            .filter(|r| r.status == CheckStatus::Warn)
            .count(),
        checks_skipped: check_results
            .iter()
            .filter(|r| r.status == CheckStatus::Skip)
            .count(),
    };

    let total_duration: u64 = check_results.iter().map(|r| r.duration_ms).sum();

    let validation_result = DodValidationResult {
        verdict: if summary.checks_failed == 0 {
            OverallVerdict::Ready
        } else {
            OverallVerdict::NotReady
        },
        readiness_score: (summary.checks_passed as f64 / summary.checks_total as f64) * 100.0,
        profile: "dev".to_string(),
        mode: ValidationMode::Fast,
        summary,
        category_scores: std::collections::HashMap::new(),
        check_results,
        artifacts: ArtifactPaths {
            receipt_path: PathBuf::from("./ggen.out/receipts/latest.json"),
            report_path: PathBuf::from("./ggen.out/reports/latest.md"),
            bundle_path: Some(PathBuf::from("./ggen.out/bundles/latest.tar.gz")),
        },
        duration_ms: total_duration,
    };

    // Collect metrics
    let metrics = DodMetrics::from_validation_result(&validation_result);

    // Verify metrics are populated
    assert!(metrics.checks_executed > 0, "Should have executed checks");
    assert!(
        metrics.total_duration.as_millis() > 0,
        "Should have non-zero duration"
    );
    assert_eq!(
        metrics.checks_executed,
        validation_result.check_results.len(),
        "Check count should match"
    );

    // Verify all checks have durations
    for result in &validation_result.check_results {
        assert!(
            metrics.check_durations.contains_key(&result.id),
            "Should track duration for {}",
            result.id
        );
    }

    // Test metrics exports
    let summary = metrics.format_summary();
    assert!(summary.contains("DoD Metrics Summary"));
    assert!(summary.contains("Total Duration"));
    assert!(summary.contains("Success Rate"));

    let prom = metrics.to_prometheus();
    assert!(prom.contains("dod_validation_duration_seconds"));
    assert!(prom.contains("dod_checks_total"));
    assert!(prom.contains("dod_success_rate"));

    println!("\n{}", summary);
    println!("\nPrometheus metrics:\n{}", prom);

    Ok(())
}

#[tokio::test]
async fn test_metrics_recorder_integration() -> Result<()> {
    let mut recorder = MetricsRecorder::new();

    // Simulate multiple validation runs
    for _ in 0..10 {
        recorder.record_check(
            "BUILD_CHECK",
            Duration::from_millis(1000 + rand::random::<u64>() % 500),
        );
        recorder.record_check(
            "TEST_UNIT",
            Duration::from_millis(500 + rand::random::<u64>() % 300),
        );
        recorder.record_verdict(OverallVerdict::Ready);
    }

    // Calculate percentiles
    let p50 = recorder.get_percentile("BUILD_CHECK", 50.0);
    let p95 = recorder.get_percentile("BUILD_CHECK", 95.0);
    let p99 = recorder.get_percentile("BUILD_CHECK", 99.0);

    assert!(p50.is_some(), "Should calculate p50");
    assert!(p95.is_some(), "Should calculate p95");
    assert!(p99.is_some(), "Should calculate p99");

    // Verify percentile ordering
    let p50_val = p50.unwrap();
    let p95_val = p95.unwrap();
    let p99_val = p99.unwrap();

    assert!(p50_val <= p95_val, "p50 should be <= p95");
    assert!(p95_val <= p99_val, "p95 should be <= p99");

    println!("BUILD_CHECK percentiles:");
    println!("  p50: {:.2}ms", p50_val.as_millis());
    println!("  p95: {:.2}ms", p95_val.as_millis());
    println!("  p99: {:.2}ms", p99_val.as_millis());

    Ok(())
}

#[test]
fn test_opentelemetry_span_creation() {
    // Test span creation (doesn't require full OTel setup)
    let span1 = DodSpan::validation_run("dev");
    assert_eq!(span1.metadata().name(), "dod_validation");

    let span2 = DodSpan::check_execution("BUILD_CHECK", CheckCategory::BuildCorrectness);
    assert_eq!(span2.metadata().name(), "dod_check");

    let span3 = DodSpan::evidence_collection("TEST_UNIT");
    assert_eq!(span3.metadata().name(), "dod_evidence");

    let span4 = DodSpan::report_generation("markdown");
    assert_eq!(span4.metadata().name(), "dod_report");

    let span5 = DodSpan::receipt_generation();
    assert_eq!(span5.metadata().name(), "dod_receipt");
}
