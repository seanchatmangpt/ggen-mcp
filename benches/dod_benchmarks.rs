//! DoD Performance Benchmarks
//!
//! Benchmarks for Definition of Done validation system.
//! Targets: <5s development profile, <10s enterprise profile

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use spreadsheet_mcp::dod::*;
use std::path::PathBuf;
use std::time::Duration;
use tokio::runtime::Runtime;

/// Create test context for benchmarks
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

/// Benchmark full validation run
fn bench_full_validation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("dod_full_validation");
    group.measurement_time(Duration::from_secs(30));
    group.sample_size(10); // Reduced sample size for long-running benchmarks

    // Development profile
    group.bench_function("dev_profile", |b| {
        b.to_async(&rt).iter(|| async {
            let registry = create_full_registry();
            let profile = DodProfile::default_dev();
            let executor = CheckExecutor::new(registry, profile);
            let context = create_test_context();

            let results = executor.execute_all(&context).await;
            black_box(results)
        });
    });

    // CI profile
    group.bench_function("ci_profile", |b| {
        b.to_async(&rt).iter(|| async {
            let registry = create_full_registry();
            let profile = DodProfile::default_ci();
            let executor = CheckExecutor::new(registry, profile);
            let context = create_test_context();

            let results = executor.execute_all(&context).await;
            black_box(results)
        });
    });

    // Enterprise profile (strict)
    group.bench_function("enterprise_profile", |b| {
        b.to_async(&rt).iter(|| async {
            let registry = create_full_registry();
            let profile = DodProfile::default_enterprise();
            let executor = CheckExecutor::new(registry, profile);
            let context = create_test_context();

            let results = executor.execute_all(&context).await;
            black_box(results)
        });
    });

    group.finish();
}

/// Benchmark individual check categories
fn bench_individual_checks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("dod_individual_checks");
    group.measurement_time(Duration::from_secs(10));

    let categories = [
        ("workspace", || spreadsheet_mcp::dod::checks::workspace::get_workspace_checks()),
        ("build", || spreadsheet_mcp::dod::checks::build::get_build_checks()),
        ("tests", || spreadsheet_mcp::dod::checks::tests::get_test_checks()),
        ("safety", || spreadsheet_mcp::dod::checks::safety::get_safety_checks()),
        ("ggen", || spreadsheet_mcp::dod::checks::ggen::get_ggen_checks()),
    ];

    for (category_name, get_checks) in categories {
        group.bench_with_input(
            BenchmarkId::new("category", category_name),
            &get_checks,
            |b, get_checks_fn| {
                b.to_async(&rt).iter(|| async {
                    let mut registry = CheckRegistry::new();
                    registry.register_all(&get_checks_fn());

                    let profile = DodProfile::default_dev();
                    let executor = CheckExecutor::new(registry, profile);
                    let context = create_test_context();

                    let results = executor.execute_all(&context).await;
                    black_box(results)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark report generation
fn bench_report_generation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("dod_report_generation");
    group.measurement_time(Duration::from_secs(10));

    // Create sample validation result
    let validation_result = rt.block_on(async {
        let registry = create_full_registry();
        let profile = DodProfile::default_dev();
        let executor = CheckExecutor::new(registry, profile);
        let context = create_test_context();

        let check_results = executor.execute_all(&context).await.unwrap();

        // Build validation result
        let summary = ValidationSummary {
            checks_total: check_results.len(),
            checks_passed: check_results.iter().filter(|r| r.status == CheckStatus::Pass).count(),
            checks_failed: check_results.iter().filter(|r| r.status == CheckStatus::Fail).count(),
            checks_warned: check_results.iter().filter(|r| r.status == CheckStatus::Warn).count(),
            checks_skipped: check_results.iter().filter(|r| r.status == CheckStatus::Skip).count(),
        };

        DodValidationResult {
            verdict: OverallVerdict::Ready,
            readiness_score: 100.0,
            profile: "dev".to_string(),
            mode: ValidationMode::Fast,
            summary,
            category_scores: std::collections::HashMap::new(),
            check_results,
            artifacts: ArtifactPaths {
                receipt_path: PathBuf::from("receipt.json"),
                report_path: PathBuf::from("report.md"),
                bundle_path: None,
            },
            duration_ms: 5000,
        }
    });

    group.bench_function("markdown_report", |b| {
        b.iter(|| {
            // Simulate report generation
            let mut report = String::new();
            report.push_str("# DoD Validation Report\n\n");
            report.push_str(&format!("Verdict: {:?}\n", validation_result.verdict));
            report.push_str(&format!("Score: {:.1}\n\n", validation_result.readiness_score));

            for check in &validation_result.check_results {
                report.push_str(&format!("## {}\n", check.id));
                report.push_str(&format!("Status: {:?}\n", check.status));
                report.push_str(&format!("Message: {}\n\n", check.message));
            }

            black_box(report)
        });
    });

    group.bench_function("json_report", |b| {
        b.iter(|| {
            let json = serde_json::to_string_pretty(&validation_result).unwrap();
            black_box(json)
        });
    });

    group.finish();
}

/// Benchmark evidence bundle creation
fn bench_evidence_bundle(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("dod_evidence_bundle");
    group.measurement_time(Duration::from_secs(10));

    // Create sample check results with evidence
    let check_results: Vec<DodCheckResult> = (0..50)
        .map(|i| DodCheckResult {
            id: format!("CHECK_{}", i),
            category: CheckCategory::BuildCorrectness,
            status: CheckStatus::Pass,
            severity: CheckSeverity::Fatal,
            message: format!("Check {} passed", i),
            evidence: vec![
                Evidence {
                    kind: EvidenceKind::CommandOutput,
                    content: "x".repeat(1000), // 1KB of content
                    file_path: Some(PathBuf::from(format!("/tmp/evidence_{}.txt", i))),
                    line_number: Some(i),
                    hash: format!("hash_{}", i),
                },
            ],
            remediation: vec![],
            duration_ms: 100,
            check_hash: format!("check_hash_{}", i),
        })
        .collect();

    group.bench_function("serialize_evidence", |b| {
        b.iter(|| {
            let serialized = serde_json::to_vec(&check_results).unwrap();
            black_box(serialized)
        });
    });

    group.bench_function("compute_evidence_hashes", |b| {
        b.iter(|| {
            use sha2::{Digest, Sha256};
            let mut hashes = Vec::new();

            for check in &check_results {
                for evidence in &check.evidence {
                    let mut hasher = Sha256::new();
                    hasher.update(evidence.content.as_bytes());
                    let hash = format!("{:x}", hasher.finalize());
                    hashes.push(hash);
                }
            }

            black_box(hashes)
        });
    });

    group.finish();
}

/// Benchmark metrics collection
fn bench_metrics_collection(c: &mut Criterion) {
    let mut group = c.benchmark_group("dod_metrics");
    group.measurement_time(Duration::from_secs(5));

    // Create sample validation result
    let check_results: Vec<DodCheckResult> = (0..100)
        .map(|i| DodCheckResult {
            id: format!("CHECK_{}", i),
            category: CheckCategory::BuildCorrectness,
            status: if i % 10 == 0 {
                CheckStatus::Fail
            } else {
                CheckStatus::Pass
            },
            severity: CheckSeverity::Fatal,
            message: format!("Check {}", i),
            evidence: vec![],
            remediation: vec![],
            duration_ms: (i * 10) % 1000,
            check_hash: format!("hash_{}", i),
        })
        .collect();

    let validation_result = DodValidationResult {
        verdict: OverallVerdict::Ready,
        readiness_score: 90.0,
        profile: "dev".to_string(),
        mode: ValidationMode::Fast,
        summary: ValidationSummary {
            checks_total: 100,
            checks_passed: 90,
            checks_failed: 10,
            checks_warned: 0,
            checks_skipped: 0,
        },
        category_scores: std::collections::HashMap::new(),
        check_results,
        artifacts: ArtifactPaths {
            receipt_path: PathBuf::from("receipt.json"),
            report_path: PathBuf::from("report.md"),
            bundle_path: None,
        },
        duration_ms: 10_000,
    };

    group.bench_function("collect_metrics", |b| {
        b.iter(|| {
            let metrics = spreadsheet_mcp::dod::metrics::DodMetrics::from_validation_result(&validation_result);
            black_box(metrics)
        });
    });

    group.bench_function("format_prometheus", |b| {
        let metrics = spreadsheet_mcp::dod::metrics::DodMetrics::from_validation_result(&validation_result);
        b.iter(|| {
            let prom = metrics.to_prometheus();
            black_box(prom)
        });
    });

    group.bench_function("format_summary", |b| {
        let metrics = spreadsheet_mcp::dod::metrics::DodMetrics::from_validation_result(&validation_result);
        b.iter(|| {
            let summary = metrics.format_summary();
            black_box(summary)
        });
    });

    group.finish();
}

/// Benchmark parallel vs serial execution
fn bench_parallelism(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("dod_parallelism");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(10);

    // Serial execution
    group.bench_function("serial", |b| {
        b.to_async(&rt).iter(|| async {
            let registry = create_full_registry();
            let mut profile = DodProfile::default_dev();
            profile.parallelism = ParallelismConfig::Serial;

            let executor = CheckExecutor::new(registry, profile);
            let context = create_test_context();

            let results = executor.execute_all(&context).await;
            black_box(results)
        });
    });

    // Parallel execution (auto)
    group.bench_function("parallel_auto", |b| {
        b.to_async(&rt).iter(|| async {
            let registry = create_full_registry();
            let mut profile = DodProfile::default_dev();
            profile.parallelism = ParallelismConfig::Auto;

            let executor = CheckExecutor::new(registry, profile);
            let context = create_test_context();

            let results = executor.execute_all(&context).await;
            black_box(results)
        });
    });

    // Parallel execution (4 threads)
    group.bench_function("parallel_4", |b| {
        b.to_async(&rt).iter(|| async {
            let registry = create_full_registry();
            let mut profile = DodProfile::default_dev();
            profile.parallelism = ParallelismConfig::Parallel(4);

            let executor = CheckExecutor::new(registry, profile);
            let context = create_test_context();

            let results = executor.execute_all(&context).await;
            black_box(results)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_full_validation,
    bench_individual_checks,
    bench_report_generation,
    bench_evidence_bundle,
    bench_metrics_collection,
    bench_parallelism,
);

criterion_main!(benches);
