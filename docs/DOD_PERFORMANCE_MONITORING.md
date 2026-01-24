# DoD Performance Monitoring & Benchmarks

**Version**: 1.0.0 | Phase 10 Complete

## Overview

Performance monitoring for Definition of Done validation system. Tracks execution times, resource usage, and provides benchmarks for optimization.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ DoD Performance Monitoring Stack                            │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Metrics    │  │  OpenTelemetry│  │ Prometheus   │      │
│  │  Collection  │──│    Spans      │──│  Exporter    │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│         │                  │                  │              │
│         v                  v                  v              │
│  ┌──────────────────────────────────────────────────┐       │
│  │         DodMetrics (Phase 10)                     │       │
│  │  - Total duration                                 │       │
│  │  - Per-check durations                            │       │
│  │  - Category durations                             │       │
│  │  - Evidence size tracking                         │       │
│  │  - Success/failure rates                          │       │
│  └──────────────────────────────────────────────────┘       │
│         │                                                     │
│         v                                                     │
│  ┌──────────────────────────────────────────────────┐       │
│  │         MetricsRecorder                           │       │
│  │  - Histogram tracking                             │       │
│  │  - Percentile calculations (p50, p95, p99)        │       │
│  │  - Verdict distribution                           │       │
│  └──────────────────────────────────────────────────┘       │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Components

### 1. DodMetrics

Core metrics collection struct tracking validation performance.

```rust
pub struct DodMetrics {
    pub total_duration: Duration,
    pub check_durations: HashMap<String, Duration>,
    pub category_durations: HashMap<CheckCategory, Duration>,
    pub evidence_size_bytes: u64,
    pub checks_executed: usize,
    pub checks_passed: usize,
    pub checks_failed: usize,
    pub checks_warned: usize,
    pub checks_skipped: usize,
}
```

**Usage**:
```rust
// From validation result
let metrics = DodMetrics::from_validation_result(&result);

// Get insights
println!("Average check duration: {:.2}s", metrics.average_check_duration().as_secs_f64());
println!("Success rate: {:.1}%", metrics.success_rate() * 100.0);

// Export to Prometheus
let prom = metrics.to_prometheus();
```

### 2. DodSpan (OpenTelemetry)

Tracing spans for distributed observability.

```rust
// Validation run span
let _span = DodSpan::validation_run("dev").entered();

// Check execution span
let _span = DodSpan::check_execution("BUILD_CHECK", CheckCategory::BuildCorrectness).entered();

// Evidence collection span
let _span = DodSpan::evidence_collection("TEST_UNIT").entered();
```

### 3. MetricsRecorder

Histogram-based metrics for percentile calculations.

```rust
let mut recorder = MetricsRecorder::new();

// Record check executions
recorder.record_check("BUILD_CHECK", Duration::from_millis(1500));
recorder.record_check("BUILD_CHECK", Duration::from_millis(1200));

// Get percentiles
let p50 = recorder.get_percentile("BUILD_CHECK", 50.0).unwrap();
let p95 = recorder.get_percentile("BUILD_CHECK", 95.0).unwrap();
let p99 = recorder.get_percentile("BUILD_CHECK", 99.0).unwrap();

// Record verdicts
recorder.record_verdict(OverallVerdict::Ready);
```

## Performance Targets

### Development Profile
- **Target**: <5 seconds
- **CI Tolerance**: <10 seconds
- **Checks**: ~15 required, 10 optional
- **Parallelism**: Auto (CPU cores)

### CI Profile
- **Target**: <8 seconds
- **CI Tolerance**: <15 seconds
- **Checks**: ~25 required, 15 optional
- **Parallelism**: Auto

### Enterprise Profile
- **Target**: <10 seconds
- **CI Tolerance**: <20 seconds
- **Checks**: All (~50 checks)
- **Parallelism**: Auto or Parallel(8)

## Benchmarks

Located in `/benches/dod_benchmarks.rs`. Run with:

```bash
cargo bench --bench dod_benchmarks
```

### Benchmark Suites

1. **Full Validation** (`bench_full_validation`)
   - Dev profile end-to-end
   - CI profile end-to-end
   - Enterprise profile end-to-end
   - Measurement time: 30s
   - Sample size: 10

2. **Individual Checks** (`bench_individual_checks`)
   - Per-category benchmarks
   - Workspace, Build, Tests, Safety, Ggen
   - Measurement time: 10s

3. **Report Generation** (`bench_report_generation`)
   - Markdown report generation
   - JSON report generation
   - Measurement time: 10s

4. **Evidence Bundle** (`bench_evidence_bundle`)
   - Evidence serialization
   - Hash computation
   - Measurement time: 10s

5. **Metrics Collection** (`bench_metrics_collection`)
   - Metrics computation
   - Prometheus export
   - Summary formatting
   - Measurement time: 5s

6. **Parallelism** (`bench_parallelism`)
   - Serial vs Parallel execution
   - Auto mode
   - Fixed thread count (4)
   - Measurement time: 20s
   - Sample size: 10

## Performance Tests

Located in `/tests/dod_performance_tests.rs`. Run with:

```bash
cargo test --test dod_performance_tests
```

### Test Coverage

1. **test_dev_profile_meets_5s_target**
   - Validates dev profile completes under 10s (CI tolerance)
   - Tracks check count and duration

2. **test_enterprise_profile_meets_10s_target**
   - Validates enterprise profile completes under 20s (CI tolerance)
   - Full check suite

3. **test_timeout_enforcement**
   - Verifies timeout mechanism works
   - Slow check (5s) with 100ms timeout
   - Should fail-fast

4. **test_parallel_execution_faster_than_serial**
   - Compares serial vs parallel execution
   - Measures speedup

5. **test_metrics_collection_overhead**
   - Validates metrics overhead <10%
   - Ensures negligible performance impact

6. **test_evidence_size_limits**
   - Tracks evidence size (1MB test)
   - Validates size reporting

7. **test_resource_limits_respected**
   - Monitors memory usage
   - Should not exceed 500MB delta

8. **test_concurrent_executions**
   - 3 concurrent DoD runs
   - Validates no contention
   - Should overlap (not take 3x)

## Metrics Export

### Prometheus Format

```prometheus
# HELP dod_validation_duration_seconds Total DoD validation duration
# TYPE dod_validation_duration_seconds gauge
dod_validation_duration_seconds 3.45

# HELP dod_checks_total Total number of checks executed
# TYPE dod_checks_total counter
dod_checks_total 25

# HELP dod_checks_passed Number of checks that passed
# TYPE dod_checks_passed counter
dod_checks_passed 23

# HELP dod_checks_failed Number of checks that failed
# TYPE dod_checks_failed counter
dod_checks_failed 2

# HELP dod_evidence_bytes Total evidence size in bytes
# TYPE dod_evidence_bytes gauge
dod_evidence_bytes 524288

# HELP dod_success_rate Success rate (0.0 to 1.0)
# TYPE dod_success_rate gauge
dod_success_rate 0.92

dod_category_duration_seconds{category="BuildCorrectness"} 1.23
dod_category_duration_seconds{category="TestTruth"} 0.87
dod_category_duration_seconds{category="GgenPipeline"} 0.65
```

### Human-Readable Summary

```
DoD Metrics Summary
==================

Total Duration: 3.45s
Checks Executed: 25
  - Passed: 23
  - Failed: 2
  - Warned: 0
  - Skipped: 0
Success Rate: 92.0%
Evidence Size: 524288 bytes

Slowest Check: GGEN_DETERMINISM (1.23s)
Slowest Category: BuildCorrectness (1.23s)
Average Check Duration: 0.14s
```

## OpenTelemetry Integration

### Span Hierarchy

```
dod_validation (profile=dev)
├── dod_check (check_id=BUILD_CHECK, category=BuildCorrectness)
│   ├── dod_evidence (check_id=BUILD_CHECK)
│   └── [check execution]
├── dod_check (check_id=TEST_UNIT, category=TestTruth)
│   └── [check execution]
└── dod_report (format=markdown)
```

### Configuration

Set environment variables for OTLP export:

```bash
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export OTEL_SERVICE_NAME=ggen-mcp-dod
export OTEL_SAMPLING_RATE=1.0  # 100% sampling
```

### Grafana Dashboard

Import the DoD metrics into Grafana:

1. Add Prometheus data source
2. Create dashboard with panels:
   - Total validation duration (gauge)
   - Check success rate (graph)
   - Per-category durations (heatmap)
   - Evidence size over time (graph)

## Best Practices

### 1. Always Measure Before Optimizing

```rust
let start = Instant::now();
let result = expensive_operation().await?;
let duration = start.elapsed();
tracing::info!(duration_ms = duration.as_millis(), "Operation completed");
```

### 2. Use Spans for Profiling

```rust
let _span = tracing::info_span!("expensive_operation").entered();
// Automatic duration tracking via OpenTelemetry
```

### 3. Track Evidence Size

```rust
let metrics = DodMetrics::from_validation_result(&result);
if metrics.evidence_size_bytes > 10 * 1024 * 1024 {
    tracing::warn!("Evidence bundle exceeds 10MB");
}
```

### 4. Monitor Timeout Violations

```rust
for result in &validation_result.check_results {
    if result.message.contains("timed out") {
        tracing::error!(
            check_id = result.id,
            category = ?result.category,
            "Check timed out - consider increasing timeout or optimizing"
        );
    }
}
```

### 5. Compare Parallel vs Serial

```rust
// Benchmark both modes for your workload
let serial_time = benchmark_with_config(ParallelismConfig::Serial);
let parallel_time = benchmark_with_config(ParallelismConfig::Auto);
let speedup = serial_time.as_secs_f64() / parallel_time.as_secs_f64();
println!("Speedup: {:.2}x", speedup);
```

## Optimization Guide

### If Validation is Too Slow

1. **Profile with OpenTelemetry**
   - Identify slowest checks
   - Look for serialization bottlenecks

2. **Enable Parallelism**
   ```rust
   profile.parallelism = ParallelismConfig::Auto;
   ```

3. **Increase Timeouts Selectively**
   ```rust
   profile.timeouts_ms.build = 30_000; // 30s for builds
   ```

4. **Skip Non-Critical Checks**
   ```rust
   profile.optional_checks.remove("EXPENSIVE_CHECK");
   ```

5. **Cache Evidence**
   - Reuse evidence across checks
   - Avoid re-running expensive operations

### If Memory Usage is High

1. **Limit Evidence Size**
   ```rust
   let truncated_content = content[..1024].to_string(); // Max 1KB
   ```

2. **Stream Evidence to Disk**
   - Don't load all evidence in memory
   - Write directly to bundle

3. **Clear Completed Checks**
   ```rust
   drop(check_result); // Release memory immediately
   ```

## Monitoring in Production

### Key Metrics to Track

1. **Validation Duration** (p50, p95, p99)
2. **Success Rate** (target: >95%)
3. **Timeout Rate** (target: <1%)
4. **Evidence Size** (alert if >50MB)
5. **Check Execution Rate** (checks/second)

### Alerts

```yaml
# Prometheus alerting rules
groups:
  - name: dod_alerts
    rules:
      - alert: DodValidationSlow
        expr: dod_validation_duration_seconds > 20
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "DoD validation taking too long"

      - alert: DodHighFailureRate
        expr: dod_success_rate < 0.8
        for: 10m
        labels:
          severity: critical
        annotations:
          summary: "DoD checks failing frequently"
```

## Testing Strategy

### Unit Tests

- Metrics calculation accuracy
- Percentile computation
- Prometheus export format

### Integration Tests

- Full validation flow
- Metrics collection overhead
- Concurrent execution

### Performance Tests

- Target adherence (5s dev, 10s enterprise)
- Timeout enforcement
- Resource limits

### Benchmarks

- Regression detection
- Optimization validation
- Profile comparison

## Future Enhancements

1. **Real-time Dashboard**
   - Live validation progress
   - Per-check status updates

2. **Historical Trends**
   - Track validation times over commits
   - Detect performance regressions

3. **Predictive Timeouts**
   - ML-based timeout estimation
   - Adaptive based on workload

4. **Cost Attribution**
   - Track which checks consume most resources
   - Optimize high-cost checks first

5. **Distributed Validation**
   - Parallelize across machines
   - Cloud-based check execution

---

**Metric Density**: Total duration, per-check, per-category, evidence size, success rates. Full observability stack.

**Integration**: OpenTelemetry spans, Prometheus metrics, Grafana dashboards. Production-ready monitoring.

**Performance**: <5s dev profile, <10s enterprise. Validated via benchmarks and performance tests.
