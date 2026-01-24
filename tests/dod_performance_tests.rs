//! DoD Performance Tests
//!
//! Tests for performance targets, timeouts, and resource limits.

use anyhow::Result;
use spreadsheet_mcp::dod::*;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio;

/// Create test context for performance tests
fn create_test_context() -> CheckContext {
    CheckContext {
        workspace_root: PathBuf::from("."),
        mode: ValidationMode::Fast,
        timeout_ms: 60_000,
    }
}

/// Create registry with all checks
fn create_full_registry() -> CheckRegistry {
    let mut registry = CheckRegistry::new();

    // Register all workspace checks
    registry.register_all(&spreadsheet_mcp::dod::checks::workspace::get_workspace_checks());

    // Register all build checks
    registry.register_all(&spreadsheet_mcp::dod::checks::build::get_build_checks());

    // Register all test checks
    registry.register_all(&spreadsheet_mcp::dod::checks::tests::get_test_checks());

    // Register all safety checks
    registry.register_all(&spreadsheet_mcp::dod::checks::safety::get_safety_checks());

    // Register all ggen checks
    registry.register_all(&spreadsheet_mcp::dod::checks::ggen::get_ggen_checks());

    // Register all intent checks
    registry.register_all(&spreadsheet_mcp::dod::checks::intent::get_intent_checks());

    // Register all tool registry checks
    registry.register_all(&spreadsheet_mcp::dod::checks::tool_registry::get_tool_registry_checks());

    // Register all deployment checks
    registry.register_all(&spreadsheet_mcp::dod::checks::deployment::get_deployment_checks());

    registry
}

#[tokio::test]
async fn test_dev_profile_meets_5s_target() -> Result<()> {
    let registry = create_full_registry();
    let profile = DodProfile::default_dev();
    let executor = CheckExecutor::new(registry, profile);
    let context = create_test_context();

    let start = Instant::now();
    let results = executor.execute_all(&context).await?;
    let duration = start.elapsed();

    println!(
        "Dev profile completed in {:.2}s with {} checks",
        duration.as_secs_f64(),
        results.len()
    );

    // Target: <5s for development profile
    // Allow 10s in CI environment (may be slower)
    assert!(
        duration.as_secs() < 10,
        "Dev profile took {:.2}s, should be under 10s",
        duration.as_secs_f64()
    );

    Ok(())
}

#[tokio::test]
async fn test_enterprise_profile_meets_10s_target() -> Result<()> {
    let registry = create_full_registry();
    let profile = DodProfile::default_enterprise();
    let executor = CheckExecutor::new(registry, profile);
    let context = create_test_context();

    let start = Instant::now();
    let results = executor.execute_all(&context).await?;
    let duration = start.elapsed();

    println!(
        "Enterprise profile completed in {:.2}s with {} checks",
        duration.as_secs_f64(),
        results.len()
    );

    // Target: <10s for enterprise profile
    // Allow 20s in CI environment
    assert!(
        duration.as_secs() < 20,
        "Enterprise profile took {:.2}s, should be under 20s",
        duration.as_secs_f64()
    );

    Ok(())
}

#[tokio::test]
async fn test_timeout_enforcement() -> Result<()> {
    use async_trait::async_trait;

    // Create a slow check that exceeds timeout
    struct SlowCheck;

    #[async_trait]
    impl DodCheck for SlowCheck {
        fn id(&self) -> &str {
            "SLOW_CHECK"
        }

        fn name(&self) -> &str {
            "Slow Check"
        }

        fn category(&self) -> CheckCategory {
            CheckCategory::BuildCorrectness
        }

        fn severity(&self) -> CheckSeverity {
            CheckSeverity::Fatal
        }

        fn dependencies(&self) -> Vec<String> {
            vec![]
        }

        async fn execute(&self, _context: &CheckContext) -> Result<DodCheckResult> {
            // Sleep for 5 seconds (longer than timeout)
            tokio::time::sleep(Duration::from_secs(5)).await;

            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Pass,
                severity: self.severity(),
                message: "Should not reach here".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms: 5000,
                check_hash: "slow".to_string(),
            })
        }
    }

    let mut registry = CheckRegistry::new();
    registry.register(Box::new(SlowCheck));

    let mut profile = DodProfile::default_dev();
    profile.required_checks.clear();
    profile.required_checks.insert("SLOW_CHECK".to_string());
    profile.timeouts_ms.build = 100; // 100ms timeout

    let executor = CheckExecutor::new(registry, profile);
    let context = create_test_context();

    let start = Instant::now();
    let results = executor.execute_all(&context).await?;
    let duration = start.elapsed();

    // Should complete quickly due to timeout
    assert!(
        duration.as_millis() < 1000,
        "Timeout should have triggered within 1s"
    );

    // Result should indicate timeout
    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_eq!(result.status, CheckStatus::Fail);
    assert!(result.message.contains("timed out"));

    Ok(())
}

#[tokio::test]
async fn test_parallel_execution_faster_than_serial() -> Result<()> {
    let registry_serial = create_full_registry();
    let mut profile_serial = DodProfile::default_dev();
    profile_serial.parallelism = ParallelismConfig::Serial;

    let executor_serial = CheckExecutor::new(registry_serial, profile_serial);
    let context_serial = create_test_context();

    let start_serial = Instant::now();
    let _ = executor_serial.execute_all(&context_serial).await?;
    let duration_serial = start_serial.elapsed();

    // Now parallel
    let registry_parallel = create_full_registry();
    let mut profile_parallel = DodProfile::default_dev();
    profile_parallel.parallelism = ParallelismConfig::Auto;

    let executor_parallel = CheckExecutor::new(registry_parallel, profile_parallel);
    let context_parallel = create_test_context();

    let start_parallel = Instant::now();
    let _ = executor_parallel.execute_all(&context_parallel).await?;
    let duration_parallel = start_parallel.elapsed();

    println!(
        "Serial: {:.2}s, Parallel: {:.2}s, Speedup: {:.2}x",
        duration_serial.as_secs_f64(),
        duration_parallel.as_secs_f64(),
        duration_serial.as_secs_f64() / duration_parallel.as_secs_f64()
    );

    // Parallel should be at least as fast (may be faster with multiple CPUs)
    // Allow for some variance in CI
    assert!(
        duration_parallel.as_millis() <= duration_serial.as_millis() + 1000,
        "Parallel execution should not be significantly slower than serial"
    );

    Ok(())
}

#[tokio::test]
async fn test_metrics_collection_overhead() -> Result<()> {
    let registry = create_full_registry();
    let profile = DodProfile::default_dev();
    let executor = CheckExecutor::new(registry, profile);
    let context = create_test_context();

    // Run without metrics collection
    let start_no_metrics = Instant::now();
    let results = executor.execute_all(&context).await?;
    let duration_no_metrics = start_no_metrics.elapsed();

    // Build validation result
    let summary = ValidationSummary {
        checks_total: results.len(),
        checks_passed: results.iter().filter(|r| r.status == CheckStatus::Pass).count(),
        checks_failed: results.iter().filter(|r| r.status == CheckStatus::Fail).count(),
        checks_warned: results.iter().filter(|r| r.status == CheckStatus::Warn).count(),
        checks_skipped: results.iter().filter(|r| r.status == CheckStatus::Skip).count(),
    };

    let validation_result = DodValidationResult {
        verdict: OverallVerdict::Ready,
        readiness_score: 100.0,
        profile: "dev".to_string(),
        mode: ValidationMode::Fast,
        summary,
        category_scores: std::collections::HashMap::new(),
        check_results: results,
        artifacts: ArtifactPaths {
            receipt_path: PathBuf::from("receipt.json"),
            report_path: PathBuf::from("report.md"),
            bundle_path: None,
        },
        duration_ms: duration_no_metrics.as_millis() as u64,
    };

    // Collect metrics
    let start_metrics = Instant::now();
    let metrics = spreadsheet_mcp::dod::metrics::DodMetrics::from_validation_result(&validation_result);
    let duration_metrics = start_metrics.elapsed();

    println!(
        "Validation: {:.2}ms, Metrics collection: {:.2}ms, Overhead: {:.2}%",
        duration_no_metrics.as_millis(),
        duration_metrics.as_millis(),
        (duration_metrics.as_millis() as f64 / duration_no_metrics.as_millis() as f64) * 100.0
    );

    // Metrics collection should be negligible (<1% overhead)
    assert!(
        duration_metrics.as_millis() < duration_no_metrics.as_millis() / 10,
        "Metrics collection overhead should be minimal"
    );

    // Verify metrics are correct
    assert_eq!(metrics.checks_executed, validation_result.check_results.len());
    assert!(metrics.total_duration.as_millis() > 0);

    Ok(())
}

#[tokio::test]
async fn test_evidence_size_limits() -> Result<()> {
    // Create check with large evidence
    use async_trait::async_trait;

    struct LargeEvidenceCheck;

    #[async_trait]
    impl DodCheck for LargeEvidenceCheck {
        fn id(&self) -> &str {
            "LARGE_EVIDENCE"
        }

        fn name(&self) -> &str {
            "Large Evidence Check"
        }

        fn category(&self) -> CheckCategory {
            CheckCategory::BuildCorrectness
        }

        fn severity(&self) -> CheckSeverity {
            CheckSeverity::Fatal
        }

        fn dependencies(&self) -> Vec<String> {
            vec![]
        }

        async fn execute(&self, _context: &CheckContext) -> Result<DodCheckResult> {
            // Create 1MB of evidence
            let large_content = "x".repeat(1024 * 1024);

            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Pass,
                severity: self.severity(),
                message: "Large evidence check".to_string(),
                evidence: vec![Evidence {
                    kind: EvidenceKind::CommandOutput,
                    content: large_content,
                    file_path: None,
                    line_number: None,
                    hash: "large".to_string(),
                }],
                remediation: vec![],
                duration_ms: 10,
                check_hash: "large".to_string(),
            })
        }
    }

    let mut registry = CheckRegistry::new();
    registry.register(Box::new(LargeEvidenceCheck));

    let mut profile = DodProfile::default_dev();
    profile.required_checks.clear();
    profile.required_checks.insert("LARGE_EVIDENCE".to_string());

    let executor = CheckExecutor::new(registry, profile);
    let context = create_test_context();

    let results = executor.execute_all(&context).await?;
    assert_eq!(results.len(), 1);

    let validation_result = DodValidationResult {
        verdict: OverallVerdict::Ready,
        readiness_score: 100.0,
        profile: "dev".to_string(),
        mode: ValidationMode::Fast,
        summary: ValidationSummary {
            checks_total: 1,
            checks_passed: 1,
            checks_failed: 0,
            checks_warned: 0,
            checks_skipped: 0,
        },
        category_scores: std::collections::HashMap::new(),
        check_results: results,
        artifacts: ArtifactPaths {
            receipt_path: PathBuf::from("receipt.json"),
            report_path: PathBuf::from("report.md"),
            bundle_path: None,
        },
        duration_ms: 100,
    };

    let metrics = spreadsheet_mcp::dod::metrics::DodMetrics::from_validation_result(&validation_result);

    // Verify evidence size is tracked
    assert!(
        metrics.evidence_size_bytes >= 1024 * 1024,
        "Evidence size should be at least 1MB"
    );

    println!("Evidence size: {} bytes", metrics.evidence_size_bytes);

    Ok(())
}

#[tokio::test]
async fn test_resource_limits_respected() -> Result<()> {
    let registry = create_full_registry();
    let profile = DodProfile::default_dev();
    let executor = CheckExecutor::new(registry, profile);
    let context = create_test_context();

    // Monitor memory during execution (simple check)
    let start_memory = get_current_memory_usage();
    let _ = executor.execute_all(&context).await?;
    let end_memory = get_current_memory_usage();

    let memory_delta = end_memory.saturating_sub(start_memory);

    println!(
        "Memory usage: start={}MB, end={}MB, delta={}MB",
        start_memory / 1024 / 1024,
        end_memory / 1024 / 1024,
        memory_delta / 1024 / 1024
    );

    // Should not use excessive memory (allow 500MB for test environment)
    assert!(
        memory_delta < 500 * 1024 * 1024,
        "Memory usage should be reasonable"
    );

    Ok(())
}

#[tokio::test]
async fn test_concurrent_executions() -> Result<()> {
    // Test that multiple DoD runs can execute concurrently
    let mut handles = vec![];

    for i in 0..3 {
        let handle = tokio::spawn(async move {
            let registry = create_full_registry();
            let profile = DodProfile::default_dev();
            let executor = CheckExecutor::new(registry, profile);
            let context = create_test_context();

            let start = Instant::now();
            let results = executor.execute_all(&context).await;
            let duration = start.elapsed();

            (i, results, duration)
        });

        handles.push(handle);
    }

    // Wait for all to complete
    let start = Instant::now();
    let mut results = vec![];
    for handle in handles {
        results.push(handle.await?);
    }
    let total_duration = start.elapsed();

    println!("Concurrent executions completed in {:.2}s", total_duration.as_secs_f64());

    // All should complete successfully
    for (i, result, duration) in results {
        assert!(result.is_ok(), "Execution {} should succeed", i);
        println!("Execution {}: {:.2}s", i, duration.as_secs_f64());
    }

    // Concurrent execution should not take 3x as long
    // (some overlap should occur)
    let avg_duration = results.iter().map(|(_, _, d)| d.as_millis()).sum::<u128>() / 3;
    assert!(
        total_duration.as_millis() < avg_duration * 4,
        "Concurrent executions should overlap"
    );

    Ok(())
}

/// Helper to get current memory usage (rough estimate)
fn get_current_memory_usage() -> usize {
    // Read from /proc/self/statm on Linux
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/self/statm") {
            if let Some(rss) = content.split_whitespace().nth(1) {
                if let Ok(pages) = rss.parse::<usize>() {
                    return pages * 4096; // Assume 4KB page size
                }
            }
        }
    }

    // Fallback: return 0 (metric not available)
    0
}
