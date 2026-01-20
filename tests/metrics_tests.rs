/// Integration tests for Prometheus metrics using chicago-tdd-tools framework
use chicago_tdd_tools::prelude::*;
use spreadsheet_mcp::metrics::{METRICS, MetricsCollector, RequestMetrics, classify_error};
use std::time::Duration;

test!(test_metrics_collector_initialization, {
    // Arrange: Create a new metrics collector
    let collector = MetricsCollector::new();

    // Act: Encode metrics to Prometheus format
    let output = collector.encode();

    // Assert: Verify all metric types are registered
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
});

test!(test_request_success_metrics, {
    // Arrange: Create collector and define test parameters
    let collector = MetricsCollector::new();
    let duration = Duration::from_millis(150);

    // Act: Record successful requests
    collector.record_request_success("list_workbooks", duration);
    collector.record_request_success("read_table", duration);
    let output = collector.encode();

    // Assert: Verify success counter incremented
    assert!(output.contains("mcp_requests_total"));
    assert!(output.contains("list_workbooks"));
    assert!(output.contains("read_table"));
    assert!(output.contains("success"));

    // Assert: Verify histogram recorded duration
    assert!(output.contains("mcp_request_duration_seconds"));
});

test!(test_request_error_metrics, {
    // Arrange: Create collector and define test duration
    let collector = MetricsCollector::new();
    let duration = Duration::from_millis(50);

    // Act: Record error requests
    collector.record_request_error("list_workbooks", duration, "not_found");
    collector.record_request_error("read_table", duration, "timeout");
    let output = collector.encode();

    // Assert: Verify error counter incremented
    assert!(output.contains("mcp_requests_total"));
    assert!(output.contains("error"));
    assert!(output.contains("mcp_errors_total"));
    assert!(output.contains("not_found"));
    assert!(output.contains("timeout"));
});

test!(test_request_timeout_metrics, {
    // Arrange: Create collector and define long duration
    let collector = MetricsCollector::new();
    let duration = Duration::from_secs(30);

    // Act: Record timeout
    collector.record_request_timeout("long_operation", duration);
    let output = collector.encode();

    // Assert: Verify timeout is recorded
    assert!(output.contains("timeout"));
    assert!(output.contains("long_operation"));
});

test!(test_cache_hit_miss_metrics, {
    // Arrange: Create collector
    let collector = MetricsCollector::new();

    // Act: Record cache operations
    collector.record_cache_hit();
    collector.record_cache_hit();
    collector.record_cache_hit();
    collector.record_cache_miss();
    let output = collector.encode();

    // Assert: Check cache hit counter
    assert!(output.contains("mcp_cache_hits_total 3"));

    // Assert: Check cache miss counter
    assert!(output.contains("mcp_cache_misses_total 1"));
});

test!(test_cache_stats_update, {
    // Arrange: Create collector and define cache stats
    let collector = MetricsCollector::new();
    let workbook_count = 10;
    let cache_size_bytes = 10_485_760; // 10MB

    // Act: Update cache stats
    collector.update_cache_stats(workbook_count, cache_size_bytes);
    let output = collector.encode();

    // Assert: Verify cache stats are recorded
    assert!(output.contains("mcp_workbooks_total 10"));
    assert!(output.contains("mcp_cache_size_bytes 10485760"));
});

test!(test_fork_count_metrics, {
    // Arrange: Create collector and define fork count
    let collector = MetricsCollector::new();
    let fork_count = 5;

    // Act: Update fork count
    collector.update_fork_count(fork_count);
    let output = collector.encode();

    // Assert: Verify fork count is recorded
    assert!(output.contains("mcp_forks_total 5"));
});

test!(test_recalc_duration_metrics, {
    // Arrange: Create collector
    let collector = MetricsCollector::new();

    // Act: Record recalculation durations
    collector.record_recalc_duration(Duration::from_secs(3));
    collector.record_recalc_duration(Duration::from_secs(5));
    let output = collector.encode();

    // Assert: Verify recalc duration histogram is recorded
    assert!(output.contains("mcp_recalc_duration_seconds"));
    // Assert: Should have histogram buckets
    assert!(output.contains("bucket"));
});

test!(test_libreoffice_process_count, {
    // Arrange: Create collector and define process count
    let collector = MetricsCollector::new();
    let process_count = 2;

    // Act: Update LibreOffice process count
    collector.update_libreoffice_processes(process_count);
    let output = collector.encode();

    // Assert: Verify process count is recorded
    assert!(output.contains("mcp_libreoffice_processes_active 2"));
});

test!(test_request_metrics_guard_success, {
    // Arrange: Create collector
    let collector = MetricsCollector::new();

    // Act: Create and complete a successful request metrics guard
    {
        let metrics = RequestMetrics::new("test_tool");
        std::thread::sleep(Duration::from_millis(10));
        metrics.success();
    }
    let output = collector.encode();

    // Assert: Verify success metrics recorded
    assert!(output.contains("test_tool"));
    assert!(output.contains("success"));
});

test!(test_request_metrics_guard_error, {
    // Arrange: Create collector
    let collector = MetricsCollector::new();

    // Act: Create and complete an error request metrics guard
    {
        let metrics = RequestMetrics::new("test_tool_error");
        metrics.error("validation_error");
    }
    let output = collector.encode();

    // Assert: Verify error metrics recorded
    assert!(output.contains("test_tool_error"));
    assert!(output.contains("error"));
    assert!(output.contains("validation_error"));
});

test!(test_request_metrics_guard_timeout, {
    // Arrange: Create collector
    let collector = MetricsCollector::new();

    // Act: Create and complete a timeout request metrics guard
    {
        let metrics = RequestMetrics::new("test_tool_timeout");
        metrics.timeout();
    }
    let output = collector.encode();

    // Assert: Verify timeout metrics recorded
    assert!(output.contains("test_tool_timeout"));
    assert!(output.contains("timeout"));
});

test!(test_request_metrics_guard_drop_without_completion, {
    // Arrange: Create collector
    let collector = MetricsCollector::new();

    // Act: Create guard and drop without calling success/error/timeout
    {
        let _metrics = RequestMetrics::new("test_tool_drop");
        // Guard dropped without calling success/error/timeout
        // Should record as unknown error
    }
    let output = collector.encode();

    // Assert: Verify unknown error recorded
    assert!(output.contains("test_tool_drop"));
    assert!(output.contains("unknown"));
});

test!(test_classify_error_not_found, {
    use anyhow::anyhow;

    // Arrange: Create a "not found" error
    let error = anyhow!("workbook not found");

    // Act: Classify the error
    let classification = classify_error(&error);

    // Assert: Verify correct classification
    assert_eq!(classification, "not_found");
});

test!(test_classify_error_timeout, {
    use anyhow::anyhow;

    // Arrange: Create timeout errors with different messages
    let error = anyhow!("operation timed out");
    let error2 = anyhow!("request timeout exceeded");

    // Act: Classify the errors
    let classification1 = classify_error(&error);
    let classification2 = classify_error(&error2);

    // Assert: Verify both are classified as timeout
    assert_eq!(classification1, "timeout");
    assert_eq!(classification2, "timeout");
});

test!(test_classify_error_permission, {
    use anyhow::anyhow;

    // Arrange: Create a permission denied error
    let error = anyhow!("permission denied");

    // Act: Classify the error
    let classification = classify_error(&error);

    // Assert: Verify correct classification
    assert_eq!(classification, "permission_denied");
});

test!(test_classify_error_invalid_input, {
    use anyhow::anyhow;

    // Arrange: Create an invalid input error
    let error = anyhow!("invalid cell reference");

    // Act: Classify the error
    let classification = classify_error(&error);

    // Assert: Verify correct classification
    assert_eq!(classification, "invalid_input");
});

test!(test_classify_error_parse, {
    use anyhow::anyhow;

    // Arrange: Create a parse error
    let error = anyhow!("parsing formula failed");

    // Act: Classify the error
    let classification = classify_error(&error);

    // Assert: Verify correct classification
    assert_eq!(classification, "parse_error");
});

test!(test_classify_error_io, {
    use anyhow::anyhow;

    // Arrange: Create an I/O error
    let error = anyhow!("io error reading file");

    // Act: Classify the error
    let classification = classify_error(&error);

    // Assert: Verify correct classification
    assert_eq!(classification, "io_error");
});

test!(test_classify_error_capacity, {
    use anyhow::anyhow;

    // Arrange: Create capacity errors with different messages
    let error = anyhow!("capacity exceeded");
    let error2 = anyhow!("limit reached");

    // Act: Classify the errors
    let classification1 = classify_error(&error);
    let classification2 = classify_error(&error2);

    // Assert: Verify both are classified as capacity_exceeded
    assert_eq!(classification1, "capacity_exceeded");
    assert_eq!(classification2, "capacity_exceeded");
});

test!(test_classify_error_fork, {
    use anyhow::anyhow;

    // Arrange: Create a fork error
    let error = anyhow!("fork creation failed");

    // Act: Classify the error
    let classification = classify_error(&error);

    // Assert: Verify correct classification
    assert_eq!(classification, "fork_error");
});

test!(test_classify_error_recalc, {
    use anyhow::anyhow;

    // Arrange: Create a recalc error
    let error = anyhow!("recalc failed");

    // Act: Classify the error
    let classification = classify_error(&error);

    // Assert: Verify correct classification
    assert_eq!(classification, "recalc_error");
});

test!(test_classify_error_cache, {
    use anyhow::anyhow;

    // Arrange: Create a cache error
    let error = anyhow!("cache error");

    // Act: Classify the error
    let classification = classify_error(&error);

    // Assert: Verify correct classification
    assert_eq!(classification, "cache_error");
});

test!(test_classify_error_unknown, {
    use anyhow::anyhow;

    // Arrange: Create an error with an unrecognized message
    let error = anyhow!("some weird error");

    // Act: Classify the error
    let classification = classify_error(&error);

    // Assert: Verify it's classified as unknown
    assert_eq!(classification, "unknown");
});

test!(test_multiple_tools_metrics, {
    // Arrange: Create collector
    let collector = MetricsCollector::new();

    // Act: Simulate multiple tool calls with mixed success/error
    collector.record_request_success("list_workbooks", Duration::from_millis(50));
    collector.record_request_success("read_table", Duration::from_millis(100));
    collector.record_request_success("list_workbooks", Duration::from_millis(45));
    collector.record_request_error("read_table", Duration::from_millis(30), "timeout");
    let output = collector.encode();

    // Assert: Both tools should be present
    assert!(output.contains("list_workbooks"));
    assert!(output.contains("read_table"));

    // Note: Multiple metrics for same tool should be aggregated
    // list_workbooks should have 2 successful requests
    // read_table should have 1 success and 1 error
});

test!(test_concurrent_metrics_updates, {
    use std::sync::Arc;
    use std::thread;

    // Arrange: Create shared collector and thread handles
    let collector = Arc::new(MetricsCollector::new());
    let mut handles = vec![];

    // Act: Spawn multiple threads updating metrics concurrently
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
        if let Err(e) = handle.join() {
            panic!("Thread panicked: {:?}", e);
        }
    }
    let output = collector.encode();

    // Assert: Verify all tools are present
    assert!(output.contains("tool_0"));
    assert!(output.contains("tool_1"));
    assert!(output.contains("tool_2"));

    // Assert: Verify cache hits (should be 10)
    assert!(output.contains("mcp_cache_hits_total 10"));
});

test!(test_metrics_encode_format, {
    // Arrange: Create collector and record a request
    let collector = MetricsCollector::new();

    // Act: Record a request and encode
    collector.record_request_success("test", Duration::from_millis(100));
    let output = collector.encode();

    // Assert: Verify Prometheus text format
    assert!(output.contains("# HELP"));
    assert!(output.contains("# TYPE"));

    // Assert: Should have metric names and values
    assert!(output.contains("mcp_"));
});

test!(test_global_metrics_instance, {
    // Arrange: Use the global METRICS singleton

    // Act: Record cache operations
    METRICS.record_cache_hit();
    METRICS.record_cache_miss();
    let output = METRICS.encode();

    // Assert: Should contain the recorded metrics
    // Note: This test shares state with other tests using METRICS,
    // so we just verify the structure is correct
    assert!(output.contains("mcp_cache_hits_total"));
    assert!(output.contains("mcp_cache_misses_total"));
});

test!(test_histogram_buckets, {
    // Arrange: Create collector
    let collector = MetricsCollector::new();

    // Act: Record various durations to test histogram buckets
    collector.record_request_success("fast", Duration::from_millis(5));
    collector.record_request_success("medium", Duration::from_millis(100));
    collector.record_request_success("slow", Duration::from_secs(2));
    let output = collector.encode();

    // Assert: Verify histogram has buckets
    assert!(output.contains("mcp_request_duration_seconds_bucket"));
    assert!(output.contains("le="));
    assert!(output.contains("mcp_request_duration_seconds_sum"));
    assert!(output.contains("mcp_request_duration_seconds_count"));
});

test!(test_active_requests_gauge, {
    // Arrange: Create collector

    let collector = MetricsCollector::new();

    // Act: Create multiple active request guards
    let _guard1 = RequestMetrics::new("tool_a");
    let _guard2 = RequestMetrics::new("tool_b");
    let _guard3 = RequestMetrics::new("tool_a");
    let output = collector.encode();

    // Assert: Active requests should be tracked
    assert!(output.contains("mcp_active_requests"));

    // Note: When guards are dropped, active requests should decrease
});

test!(test_error_types_tracking, {
    // Arrange: Create collector
    let collector = MetricsCollector::new();

    // Act: Record various error types
    collector.record_request_error("tool", Duration::from_millis(10), "not_found");
    collector.record_request_error("tool", Duration::from_millis(20), "timeout");
    collector.record_request_error("tool", Duration::from_millis(15), "invalid_input");
    collector.record_request_error("tool", Duration::from_millis(5), "not_found");
    let output = collector.encode();

    // Assert: All error types should be tracked separately
    assert!(output.contains("not_found"));
    assert!(output.contains("timeout"));
    assert!(output.contains("invalid_input"));

    // Note: not_found should have count of 2
});

test!(test_metrics_labels, {
    // Arrange: Create collector
    let collector = MetricsCollector::new();

    // Act: Record a successful request
    collector.record_request_success("list_workbooks", Duration::from_millis(50));
    let output = collector.encode();

    // Assert: Verify labels are present and correctly formatted
    assert!(output.contains("tool=\"list_workbooks\""));
    assert!(output.contains("status=\"success\""));
});

test!(test_zero_metrics_initial_state, {
    // Arrange: Create a new collector
    let collector = MetricsCollector::new();

    // Act: Encode without recording any metrics
    let output = collector.encode();

    // Assert: New collector should have metrics registered but with zero/empty values
    // Just verify structure exists
    assert!(output.contains("mcp_requests_total"));
    assert!(output.contains("mcp_cache_hits_total"));
});
