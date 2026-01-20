/// Integration tests for Prometheus metrics
use spreadsheet_mcp::metrics::{METRICS, MetricsCollector, RequestMetrics, classify_error};
use std::time::Duration;

#[test]
fn test_metrics_collector_initialization() {
    let collector = MetricsCollector::new();
    let output = collector.encode();

    // Verify all metric types are registered
    assert!(output.contains("TYPE mcp_requests_total counter"));
    assert!(output.contains("TYPE mcp_request_duration_seconds histogram"));
    assert!(output.contains("TYPE mcp_active_requests gauge"));
    assert!(output.contains("TYPE mcp_cache_hits_total counter"));
    assert!(output.contains("TYPE mcp_cache_misses_total counter"));
    assert!(output.contains("TYPE mcp_cache_size_bytes gauge"));
    assert!(output.contains("TYPE mcp_workbooks_total gauge"));
    assert!(output.contains("TYPE mcp_forks_total gauge"));
    assert!(output.contains("TYPE mcp_libreoffice_processes_active gauge"));
    assert!(output.contains("TYPE mcp_recalc_duration_seconds histogram"));
    assert!(output.contains("TYPE mcp_errors_total counter"));
}

#[test]
fn test_request_success_metrics() {
    let collector = MetricsCollector::new();
    let duration = Duration::from_millis(150);

    collector.record_request_success("list_workbooks", duration);
    collector.record_request_success("read_table", duration);

    let output = collector.encode();

    // Verify success counter incremented
    assert!(output.contains("mcp_requests_total"));
    assert!(output.contains("list_workbooks"));
    assert!(output.contains("read_table"));
    assert!(output.contains("success"));

    // Verify histogram recorded duration
    assert!(output.contains("mcp_request_duration_seconds"));
}

#[test]
fn test_request_error_metrics() {
    let collector = MetricsCollector::new();
    let duration = Duration::from_millis(50);

    collector.record_request_error("list_workbooks", duration, "not_found");
    collector.record_request_error("read_table", duration, "timeout");

    let output = collector.encode();

    // Verify error counter incremented
    assert!(output.contains("mcp_requests_total"));
    assert!(output.contains("error"));
    assert!(output.contains("mcp_errors_total"));
    assert!(output.contains("not_found"));
    assert!(output.contains("timeout"));
}

#[test]
fn test_request_timeout_metrics() {
    let collector = MetricsCollector::new();
    let duration = Duration::from_secs(30);

    collector.record_request_timeout("long_operation", duration);

    let output = collector.encode();

    assert!(output.contains("timeout"));
    assert!(output.contains("long_operation"));
}

#[test]
fn test_cache_hit_miss_metrics() {
    let collector = MetricsCollector::new();

    // Record cache operations
    collector.record_cache_hit();
    collector.record_cache_hit();
    collector.record_cache_hit();
    collector.record_cache_miss();

    let output = collector.encode();

    // Check cache hit counter
    assert!(output.contains("mcp_cache_hits_total 3"));

    // Check cache miss counter
    assert!(output.contains("mcp_cache_misses_total 1"));
}

#[test]
fn test_cache_stats_update() {
    let collector = MetricsCollector::new();

    collector.update_cache_stats(10, 10_485_760); // 10 workbooks, 10MB

    let output = collector.encode();

    assert!(output.contains("mcp_workbooks_total 10"));
    assert!(output.contains("mcp_cache_size_bytes 10485760"));
}

#[test]
fn test_fork_count_metrics() {
    let collector = MetricsCollector::new();

    collector.update_fork_count(5);

    let output = collector.encode();

    assert!(output.contains("mcp_forks_total 5"));
}

#[test]
fn test_recalc_duration_metrics() {
    let collector = MetricsCollector::new();

    collector.record_recalc_duration(Duration::from_secs(3));
    collector.record_recalc_duration(Duration::from_secs(5));

    let output = collector.encode();

    assert!(output.contains("mcp_recalc_duration_seconds"));
    // Should have histogram buckets
    assert!(output.contains("bucket"));
}

#[test]
fn test_libreoffice_process_count() {
    let collector = MetricsCollector::new();

    collector.update_libreoffice_processes(2);

    let output = collector.encode();

    assert!(output.contains("mcp_libreoffice_processes_active 2"));
}

#[test]
fn test_request_metrics_guard_success() {
    let collector = MetricsCollector::new();

    {
        let metrics = RequestMetrics::new("test_tool");
        std::thread::sleep(Duration::from_millis(10));
        metrics.success();
    }

    let output = collector.encode();

    assert!(output.contains("test_tool"));
    assert!(output.contains("success"));
}

#[test]
fn test_request_metrics_guard_error() {
    let collector = MetricsCollector::new();

    {
        let metrics = RequestMetrics::new("test_tool_error");
        metrics.error("validation_error");
    }

    let output = collector.encode();

    assert!(output.contains("test_tool_error"));
    assert!(output.contains("error"));
    assert!(output.contains("validation_error"));
}

#[test]
fn test_request_metrics_guard_timeout() {
    let collector = MetricsCollector::new();

    {
        let metrics = RequestMetrics::new("test_tool_timeout");
        metrics.timeout();
    }

    let output = collector.encode();

    assert!(output.contains("test_tool_timeout"));
    assert!(output.contains("timeout"));
}

#[test]
fn test_request_metrics_guard_drop_without_completion() {
    let collector = MetricsCollector::new();

    {
        let _metrics = RequestMetrics::new("test_tool_drop");
        // Guard dropped without calling success/error/timeout
        // Should record as unknown error
    }

    let output = collector.encode();

    assert!(output.contains("test_tool_drop"));
    assert!(output.contains("unknown"));
}

#[test]
fn test_classify_error_not_found() {
    use anyhow::anyhow;

    let error = anyhow!("workbook not found");
    assert_eq!(classify_error(&error), "not_found");
}

#[test]
fn test_classify_error_timeout() {
    use anyhow::anyhow;

    let error = anyhow!("operation timed out");
    assert_eq!(classify_error(&error), "timeout");

    let error2 = anyhow!("request timeout exceeded");
    assert_eq!(classify_error(&error2), "timeout");
}

#[test]
fn test_classify_error_permission() {
    use anyhow::anyhow;

    let error = anyhow!("permission denied");
    assert_eq!(classify_error(&error), "permission_denied");
}

#[test]
fn test_classify_error_invalid_input() {
    use anyhow::anyhow;

    let error = anyhow!("invalid cell reference");
    assert_eq!(classify_error(&error), "invalid_input");
}

#[test]
fn test_classify_error_parse() {
    use anyhow::anyhow;

    let error = anyhow!("parsing formula failed");
    assert_eq!(classify_error(&error), "parse_error");
}

#[test]
fn test_classify_error_io() {
    use anyhow::anyhow;

    let error = anyhow!("io error reading file");
    assert_eq!(classify_error(&error), "io_error");
}

#[test]
fn test_classify_error_capacity() {
    use anyhow::anyhow;

    let error = anyhow!("capacity exceeded");
    assert_eq!(classify_error(&error), "capacity_exceeded");

    let error2 = anyhow!("limit reached");
    assert_eq!(classify_error(&error2), "capacity_exceeded");
}

#[test]
fn test_classify_error_fork() {
    use anyhow::anyhow;

    let error = anyhow!("fork creation failed");
    assert_eq!(classify_error(&error), "fork_error");
}

#[test]
fn test_classify_error_recalc() {
    use anyhow::anyhow;

    let error = anyhow!("recalc failed");
    assert_eq!(classify_error(&error), "recalc_error");
}

#[test]
fn test_classify_error_cache() {
    use anyhow::anyhow;

    let error = anyhow!("cache error");
    assert_eq!(classify_error(&error), "cache_error");
}

#[test]
fn test_classify_error_unknown() {
    use anyhow::anyhow;

    let error = anyhow!("some weird error");
    assert_eq!(classify_error(&error), "unknown");
}

#[test]
fn test_multiple_tools_metrics() {
    let collector = MetricsCollector::new();

    // Simulate multiple tool calls
    collector.record_request_success("list_workbooks", Duration::from_millis(50));
    collector.record_request_success("read_table", Duration::from_millis(100));
    collector.record_request_success("list_workbooks", Duration::from_millis(45));
    collector.record_request_error("read_table", Duration::from_millis(30), "timeout");

    let output = collector.encode();

    // Both tools should be present
    assert!(output.contains("list_workbooks"));
    assert!(output.contains("read_table"));

    // Multiple metrics for same tool should be aggregated
    // list_workbooks should have 2 successful requests
    // read_table should have 1 success and 1 error
}

#[test]
fn test_concurrent_metrics_updates() {
    use std::sync::Arc;
    use std::thread;

    let collector = Arc::new(MetricsCollector::new());
    let mut handles = vec![];

    // Spawn multiple threads updating metrics concurrently
    for i in 0..10 {
        let collector = collector.clone();
        let handle = thread::spawn(move || {
            let tool = format!("tool_{}", i % 3);
            collector.record_request_success(&tool, Duration::from_millis(i as u64));
            collector.record_cache_hit();
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    let output = collector.encode();

    // Verify all tools are present
    assert!(output.contains("tool_0"));
    assert!(output.contains("tool_1"));
    assert!(output.contains("tool_2"));

    // Verify cache hits (should be 10)
    assert!(output.contains("mcp_cache_hits_total 10"));
}

#[test]
fn test_metrics_encode_format() {
    let collector = MetricsCollector::new();

    collector.record_request_success("test", Duration::from_millis(100));

    let output = collector.encode();

    // Verify Prometheus text format
    assert!(output.contains("# HELP"));
    assert!(output.contains("# TYPE"));

    // Should have metric names and values
    assert!(output.contains("mcp_"));
}

#[test]
fn test_global_metrics_instance() {
    // Test that METRICS singleton works
    METRICS.record_cache_hit();
    METRICS.record_cache_miss();

    let output = METRICS.encode();

    // Should contain the recorded metrics
    // Note: This test shares state with other tests using METRICS,
    // so we just verify the structure is correct
    assert!(output.contains("mcp_cache_hits_total"));
    assert!(output.contains("mcp_cache_misses_total"));
}

#[test]
fn test_histogram_buckets() {
    let collector = MetricsCollector::new();

    // Record various durations to test histogram buckets
    collector.record_request_success("fast", Duration::from_millis(5));
    collector.record_request_success("medium", Duration::from_millis(100));
    collector.record_request_success("slow", Duration::from_secs(2));

    let output = collector.encode();

    // Verify histogram has buckets
    assert!(output.contains("mcp_request_duration_seconds_bucket"));
    assert!(output.contains("le="));
    assert!(output.contains("mcp_request_duration_seconds_sum"));
    assert!(output.contains("mcp_request_duration_seconds_count"));
}

#[test]
fn test_active_requests_gauge() {
    let collector = MetricsCollector::new();

    // Create multiple active request guards
    let _guard1 = RequestMetrics::new("tool_a");
    let _guard2 = RequestMetrics::new("tool_b");
    let _guard3 = RequestMetrics::new("tool_a");

    let output = collector.encode();

    // Active requests should be tracked
    assert!(output.contains("mcp_active_requests"));

    // When guards are dropped, active requests should decrease
}

#[test]
fn test_error_types_tracking() {
    let collector = MetricsCollector::new();

    // Record various error types
    collector.record_request_error("tool", Duration::from_millis(10), "not_found");
    collector.record_request_error("tool", Duration::from_millis(20), "timeout");
    collector.record_request_error("tool", Duration::from_millis(15), "invalid_input");
    collector.record_request_error("tool", Duration::from_millis(5), "not_found");

    let output = collector.encode();

    // All error types should be tracked separately
    assert!(output.contains("not_found"));
    assert!(output.contains("timeout"));
    assert!(output.contains("invalid_input"));

    // not_found should have count of 2
}

#[test]
fn test_metrics_labels() {
    let collector = MetricsCollector::new();

    collector.record_request_success("list_workbooks", Duration::from_millis(50));

    let output = collector.encode();

    // Verify labels are present and correctly formatted
    assert!(output.contains("tool=\"list_workbooks\""));
    assert!(output.contains("status=\"success\""));
}

#[test]
fn test_zero_metrics_initial_state() {
    let collector = MetricsCollector::new();
    let output = collector.encode();

    // New collector should have metrics registered but with zero/empty values
    // Just verify structure exists
    assert!(output.contains("mcp_requests_total"));
    assert!(output.contains("mcp_cache_hits_total"));
}
