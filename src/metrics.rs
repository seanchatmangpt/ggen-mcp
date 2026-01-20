/// Prometheus metrics for production observability
///
/// This module provides comprehensive metrics collection for monitoring
/// the MCP server in production environments.
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use prometheus_client::encoding::{EncodeLabelSet, text::encode};
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::{Histogram, exponential_buckets};
use prometheus_client::registry::Registry;
use std::sync::Arc;
use std::time::Instant;

/// Global metrics registry instance
pub static METRICS: Lazy<Arc<MetricsCollector>> = Lazy::new(|| Arc::new(MetricsCollector::new()));

/// Labels for MCP request metrics
#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct RequestLabels {
    /// Tool name (e.g., "list_workbooks", "read_table")
    pub tool: String,
    /// Request status ("success", "error", "timeout")
    pub status: String,
}

/// Labels for error metrics
#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ErrorLabels {
    /// Tool name
    pub tool: String,
    /// Error type classification
    pub error_type: String,
}

/// Labels for tool-specific metrics
#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ToolLabels {
    /// Tool name
    pub tool: String,
}

/// Central metrics collector with Prometheus registry
pub struct MetricsCollector {
    /// The Prometheus registry
    registry: RwLock<Registry>,

    // Request metrics
    /// Total MCP requests by tool and status
    pub mcp_requests_total: Family<RequestLabels, Counter>,

    /// Request duration in seconds by tool
    pub mcp_request_duration_seconds: Family<ToolLabels, Histogram>,

    /// Currently active requests by tool
    pub mcp_active_requests: Family<ToolLabels, Gauge>,

    // Cache metrics
    /// Total cache hits
    pub mcp_cache_hits_total: Counter,

    /// Total cache misses
    pub mcp_cache_misses_total: Counter,

    /// Current cache size in bytes (estimated)
    pub mcp_cache_size_bytes: Gauge,

    /// Total workbooks currently in cache
    pub mcp_workbooks_total: Gauge,

    // Fork metrics (recalc feature)
    /// Total active forks
    pub mcp_forks_total: Gauge,

    /// LibreOffice processes currently active
    pub mcp_libreoffice_processes_active: Gauge,

    /// Recalculation duration in seconds
    pub mcp_recalc_duration_seconds: Histogram,

    // Error metrics
    /// Total errors by tool and error type
    pub mcp_errors_total: Family<ErrorLabels, Counter>,
}

impl MetricsCollector {
    /// Create a new metrics collector with all metrics registered
    pub fn new() -> Self {
        let mut registry = Registry::default();

        // Request metrics
        let mcp_requests_total = Family::<RequestLabels, Counter>::default();
        registry.register(
            "mcp_requests_total",
            "Total number of MCP requests",
            mcp_requests_total.clone(),
        );

        let mcp_request_duration_seconds =
            Family::<ToolLabels, Histogram>::new_with_constructor(|| {
                // Buckets: 10ms, 50ms, 100ms, 250ms, 500ms, 1s, 2.5s, 5s, 10s, 30s
                Histogram::new(exponential_buckets(0.01, 2.5, 10))
            });
        registry.register(
            "mcp_request_duration_seconds",
            "Request latency histogram in seconds",
            mcp_request_duration_seconds.clone(),
        );

        let mcp_active_requests = Family::<ToolLabels, Gauge>::default();
        registry.register(
            "mcp_active_requests",
            "Number of requests currently being processed",
            mcp_active_requests.clone(),
        );

        // Cache metrics
        let mcp_cache_hits_total = Counter::default();
        registry.register(
            "mcp_cache_hits_total",
            "Total number of workbook cache hits",
            mcp_cache_hits_total.clone(),
        );

        let mcp_cache_misses_total = Counter::default();
        registry.register(
            "mcp_cache_misses_total",
            "Total number of workbook cache misses",
            mcp_cache_misses_total.clone(),
        );

        let mcp_cache_size_bytes = Gauge::default();
        registry.register(
            "mcp_cache_size_bytes",
            "Estimated cache size in bytes",
            mcp_cache_size_bytes.clone(),
        );

        let mcp_workbooks_total = Gauge::default();
        registry.register(
            "mcp_workbooks_total",
            "Total number of workbooks in cache",
            mcp_workbooks_total.clone(),
        );

        // Fork metrics
        let mcp_forks_total = Gauge::default();
        registry.register(
            "mcp_forks_total",
            "Total number of active forks",
            mcp_forks_total.clone(),
        );

        let mcp_libreoffice_processes_active = Gauge::default();
        registry.register(
            "mcp_libreoffice_processes_active",
            "Number of active LibreOffice processes",
            mcp_libreoffice_processes_active.clone(),
        );

        let mcp_recalc_duration_seconds = Histogram::new(exponential_buckets(0.1, 2.0, 12));
        registry.register(
            "mcp_recalc_duration_seconds",
            "Recalculation duration in seconds",
            mcp_recalc_duration_seconds.clone(),
        );

        // Error metrics
        let mcp_errors_total = Family::<ErrorLabels, Counter>::default();
        registry.register(
            "mcp_errors_total",
            "Total number of errors by tool and error type",
            mcp_errors_total.clone(),
        );

        Self {
            registry: RwLock::new(registry),
            mcp_requests_total,
            mcp_request_duration_seconds,
            mcp_active_requests,
            mcp_cache_hits_total,
            mcp_cache_misses_total,
            mcp_cache_size_bytes,
            mcp_workbooks_total,
            mcp_forks_total,
            mcp_libreoffice_processes_active,
            mcp_recalc_duration_seconds,
            mcp_errors_total,
        }
    }

    /// Encode metrics in Prometheus text format
    pub fn encode(&self) -> String {
        let mut buffer = String::new();
        let registry = self.registry.read();
        encode(&mut buffer, &registry).expect("encoding metrics should succeed");
        buffer
    }

    /// Record a successful request
    pub fn record_request_success(&self, tool: &str, duration: std::time::Duration) {
        self.mcp_requests_total
            .get_or_create(&RequestLabels {
                tool: tool.to_string(),
                status: "success".to_string(),
            })
            .inc();

        self.mcp_request_duration_seconds
            .get_or_create(&ToolLabels {
                tool: tool.to_string(),
            })
            .observe(duration.as_secs_f64());
    }

    /// Record a failed request
    pub fn record_request_error(
        &self,
        tool: &str,
        duration: std::time::Duration,
        error_type: &str,
    ) {
        self.mcp_requests_total
            .get_or_create(&RequestLabels {
                tool: tool.to_string(),
                status: "error".to_string(),
            })
            .inc();

        self.mcp_request_duration_seconds
            .get_or_create(&ToolLabels {
                tool: tool.to_string(),
            })
            .observe(duration.as_secs_f64());

        self.mcp_errors_total
            .get_or_create(&ErrorLabels {
                tool: tool.to_string(),
                error_type: error_type.to_string(),
            })
            .inc();
    }

    /// Record a timeout
    pub fn record_request_timeout(&self, tool: &str, duration: std::time::Duration) {
        self.mcp_requests_total
            .get_or_create(&RequestLabels {
                tool: tool.to_string(),
                status: "timeout".to_string(),
            })
            .inc();

        self.mcp_request_duration_seconds
            .get_or_create(&ToolLabels {
                tool: tool.to_string(),
            })
            .observe(duration.as_secs_f64());

        self.mcp_errors_total
            .get_or_create(&ErrorLabels {
                tool: tool.to_string(),
                error_type: "timeout".to_string(),
            })
            .inc();
    }

    /// Record cache hit
    pub fn record_cache_hit(&self) {
        self.mcp_cache_hits_total.inc();
    }

    /// Record cache miss
    pub fn record_cache_miss(&self) {
        self.mcp_cache_misses_total.inc();
    }

    /// Update cache statistics
    pub fn update_cache_stats(&self, size: usize, estimated_bytes: u64) {
        self.mcp_workbooks_total.set(size as i64);
        self.mcp_cache_size_bytes.set(estimated_bytes as i64);
    }

    /// Update fork count
    pub fn update_fork_count(&self, count: usize) {
        self.mcp_forks_total.set(count as i64);
    }

    /// Record recalculation duration
    pub fn record_recalc_duration(&self, duration: std::time::Duration) {
        self.mcp_recalc_duration_seconds
            .observe(duration.as_secs_f64());
    }

    /// Update LibreOffice process count
    pub fn update_libreoffice_processes(&self, count: usize) {
        self.mcp_libreoffice_processes_active.set(count as i64);
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard for automatic request timing and metric recording
///
/// This guard automatically records request metrics when dropped,
/// including duration and success/failure status.
///
/// # Example
///
/// ```no_run
/// use spreadsheet_mcp::metrics::{METRICS, RequestMetrics};
///
/// async fn handle_request(tool: &str) -> anyhow::Result<()> {
///     let _metrics = RequestMetrics::new(tool);
///     // Your request handling logic here
///     Ok(())
/// }
/// ```
pub struct RequestMetrics {
    tool: String,
    start: Instant,
    completed: bool,
}

impl RequestMetrics {
    /// Create a new request metrics guard
    ///
    /// Increments the active requests counter and starts timing.
    pub fn new(tool: &str) -> Self {
        METRICS
            .mcp_active_requests
            .get_or_create(&ToolLabels {
                tool: tool.to_string(),
            })
            .inc();

        Self {
            tool: tool.to_string(),
            start: Instant::now(),
            completed: false,
        }
    }

    /// Mark the request as successful
    ///
    /// Records success metrics immediately. Call this before dropping the guard
    /// to ensure success is recorded even if the guard is dropped during cleanup.
    pub fn success(mut self) {
        let duration = self.start.elapsed();
        METRICS.record_request_success(&self.tool, duration);
        self.completed = true;

        METRICS
            .mcp_active_requests
            .get_or_create(&ToolLabels {
                tool: self.tool.clone(),
            })
            .dec();
    }

    /// Mark the request as failed
    ///
    /// Records error metrics immediately with the provided error type.
    pub fn error(mut self, error_type: &str) {
        let duration = self.start.elapsed();
        METRICS.record_request_error(&self.tool, duration, error_type);
        self.completed = true;

        METRICS
            .mcp_active_requests
            .get_or_create(&ToolLabels {
                tool: self.tool.clone(),
            })
            .dec();
    }

    /// Mark the request as timed out
    pub fn timeout(mut self) {
        let duration = self.start.elapsed();
        METRICS.record_request_timeout(&self.tool, duration);
        self.completed = true;

        METRICS
            .mcp_active_requests
            .get_or_create(&ToolLabels {
                tool: self.tool.clone(),
            })
            .dec();
    }
}

impl Drop for RequestMetrics {
    fn drop(&mut self) {
        if !self.completed {
            // If not explicitly marked as success/error, treat as error
            let duration = self.start.elapsed();
            METRICS.record_request_error(&self.tool, duration, "unknown");

            METRICS
                .mcp_active_requests
                .get_or_create(&ToolLabels {
                    tool: self.tool.clone(),
                })
                .dec();
        }
    }
}

/// Helper macro for instrumenting tool handlers
///
/// Automatically wraps a function with metrics collection.
///
/// # Example
///
/// ```no_run
/// use spreadsheet_mcp::with_metrics;
///
/// async fn my_tool_handler() -> anyhow::Result<String> {
///     with_metrics!("my_tool", {
///         // Your tool logic here
///         Ok("result".to_string())
///     })
/// }
/// ```
#[macro_export]
macro_rules! with_metrics {
    ($tool:expr, $body:expr) => {{
        let _metrics = $crate::metrics::RequestMetrics::new($tool);
        let result = $body;
        match &result {
            Ok(_) => _metrics.success(),
            Err(_) => _metrics.error("execution_error"),
        }
        result
    }};
}

/// Classify error type for metrics
///
/// Maps common error types to metric labels for better error tracking.
pub fn classify_error(error: &anyhow::Error) -> &'static str {
    let error_str = error.to_string().to_lowercase();

    if error_str.contains("not found") {
        "not_found"
    } else if error_str.contains("timeout") || error_str.contains("timed out") {
        "timeout"
    } else if error_str.contains("permission") || error_str.contains("denied") {
        "permission_denied"
    } else if error_str.contains("invalid") {
        "invalid_input"
    } else if error_str.contains("parse") || error_str.contains("parsing") {
        "parse_error"
    } else if error_str.contains("io") || error_str.contains("i/o") {
        "io_error"
    } else if error_str.contains("capacity") || error_str.contains("limit") {
        "capacity_exceeded"
    } else if error_str.contains("fork") {
        "fork_error"
    } else if error_str.contains("recalc") {
        "recalc_error"
    } else if error_str.contains("cache") {
        "cache_error"
    } else {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        let output = collector.encode();

        // Verify all metrics are present in output
        assert!(output.contains("mcp_requests_total"));
        assert!(output.contains("mcp_request_duration_seconds"));
        assert!(output.contains("mcp_active_requests"));
        assert!(output.contains("mcp_cache_hits_total"));
        assert!(output.contains("mcp_cache_misses_total"));
        assert!(output.contains("mcp_cache_size_bytes"));
        assert!(output.contains("mcp_workbooks_total"));
        assert!(output.contains("mcp_forks_total"));
        assert!(output.contains("mcp_libreoffice_processes_active"));
        assert!(output.contains("mcp_recalc_duration_seconds"));
        assert!(output.contains("mcp_errors_total"));
    }

    #[test]
    fn test_record_request_success() {
        let collector = MetricsCollector::new();
        let duration = std::time::Duration::from_millis(100);

        collector.record_request_success("test_tool", duration);

        let output = collector.encode();
        assert!(output.contains("test_tool"));
        assert!(output.contains("success"));
    }

    #[test]
    fn test_record_request_error() {
        let collector = MetricsCollector::new();
        let duration = std::time::Duration::from_millis(50);

        collector.record_request_error("test_tool", duration, "not_found");

        let output = collector.encode();
        assert!(output.contains("test_tool"));
        assert!(output.contains("error"));
        assert!(output.contains("not_found"));
    }

    #[test]
    fn test_cache_metrics() {
        let collector = MetricsCollector::new();

        collector.record_cache_hit();
        collector.record_cache_hit();
        collector.record_cache_miss();
        collector.update_cache_stats(5, 1024 * 1024);

        let output = collector.encode();
        assert!(output.contains("mcp_cache_hits_total 2"));
        assert!(output.contains("mcp_cache_misses_total 1"));
        assert!(output.contains("mcp_workbooks_total 5"));
    }

    #[test]
    fn test_request_metrics_guard_success() {
        let collector = MetricsCollector::new();

        {
            let metrics = RequestMetrics::new("guard_test");
            std::thread::sleep(std::time::Duration::from_millis(10));
            metrics.success();
        }

        let output = collector.encode();
        assert!(output.contains("guard_test"));
        assert!(output.contains("success"));
    }

    #[test]
    fn test_request_metrics_guard_error() {
        let collector = MetricsCollector::new();

        {
            let metrics = RequestMetrics::new("guard_error_test");
            metrics.error("test_error");
        }

        let output = collector.encode();
        assert!(output.contains("guard_error_test"));
        assert!(output.contains("error"));
        assert!(output.contains("test_error"));
    }

    #[test]
    fn test_classify_error() {
        use anyhow::anyhow;

        assert_eq!(classify_error(&anyhow!("file not found")), "not_found");
        assert_eq!(classify_error(&anyhow!("operation timed out")), "timeout");
        assert_eq!(
            classify_error(&anyhow!("permission denied")),
            "permission_denied"
        );
        assert_eq!(classify_error(&anyhow!("invalid input")), "invalid_input");
        assert_eq!(classify_error(&anyhow!("parse error")), "parse_error");
        assert_eq!(classify_error(&anyhow!("io error")), "io_error");
        assert_eq!(
            classify_error(&anyhow!("capacity exceeded")),
            "capacity_exceeded"
        );
        assert_eq!(classify_error(&anyhow!("fork failed")), "fork_error");
        assert_eq!(classify_error(&anyhow!("recalc failed")), "recalc_error");
        assert_eq!(classify_error(&anyhow!("unknown error type")), "unknown");
    }

    #[test]
    fn test_fork_metrics() {
        let collector = MetricsCollector::new();

        collector.update_fork_count(3);
        collector.update_libreoffice_processes(2);
        collector.record_recalc_duration(std::time::Duration::from_secs(5));

        let output = collector.encode();
        assert!(output.contains("mcp_forks_total 3"));
        assert!(output.contains("mcp_libreoffice_processes_active 2"));
        assert!(output.contains("mcp_recalc_duration_seconds"));
    }

    #[test]
    fn test_multiple_tools() {
        let collector = MetricsCollector::new();

        collector.record_request_success("tool_a", std::time::Duration::from_millis(10));
        collector.record_request_success("tool_b", std::time::Duration::from_millis(20));
        collector.record_request_error("tool_a", std::time::Duration::from_millis(15), "timeout");

        let output = collector.encode();
        assert!(output.contains("tool_a"));
        assert!(output.contains("tool_b"));
    }

    #[test]
    fn test_concurrent_metrics() {
        use std::sync::Arc;
        use std::thread;

        let collector = Arc::new(MetricsCollector::new());
        let mut handles = vec![];

        for i in 0..10 {
            let collector = collector.clone();
            let handle = thread::spawn(move || {
                let tool = format!("tool_{}", i % 3);
                collector.record_request_success(&tool, std::time::Duration::from_millis(i as u64));
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let output = collector.encode();
        assert!(output.contains("tool_0"));
        assert!(output.contains("tool_1"));
        assert!(output.contains("tool_2"));
    }
}
