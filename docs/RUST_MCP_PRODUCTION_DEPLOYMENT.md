# Rust MCP Production Deployment Guide

> **Toyota Production System (TPS) Principle**: Production observability follows Gemba (現場) - "go and see" the actual place where work happens. This guide implements comprehensive production monitoring to make problems visible immediately.

## Table of Contents

1. [Configuration Management](#1-configuration-management)
2. [Logging and Tracing](#2-logging-and-tracing)
3. [Metrics and Observability](#3-metrics-and-observability)
4. [Health Checks](#4-health-checks)
5. [Graceful Shutdown](#5-graceful-shutdown)
6. [Container Deployment](#6-container-deployment)
7. [Monitoring and Alerting](#7-monitoring-and-alerting)
8. [Security Hardening](#8-security-hardening)

---

## 1. Configuration Management

### 1.1 Environment Variables

Current ggen-mcp implementation uses environment variables with `clap`:

```rust
#[derive(Parser, Debug)]
pub struct CliArgs {
    #[arg(long, env = "SPREADSHEET_MCP_WORKSPACE")]
    pub workspace_root: Option<PathBuf>,

    #[arg(long, env = "SPREADSHEET_MCP_CACHE_CAPACITY")]
    pub cache_capacity: Option<usize>,

    #[arg(long, env = "SPREADSHEET_MCP_TRANSPORT")]
    pub transport: Option<TransportKind>,
    // ... more fields
}
```

**Best Practices:**
- ✅ Use consistent prefix (e.g., `SPREADSHEET_MCP_*`)
- ✅ Support both CLI args and environment variables
- ✅ Document all environment variables in README
- ⚠️ Consider `12-factor app` principles for configuration

**Recommended Environment Variables:**

```bash
# Core Configuration
SPREADSHEET_MCP_WORKSPACE=/data
SPREADSHEET_MCP_CACHE_CAPACITY=50
SPREADSHEET_MCP_TRANSPORT=http
SPREADSHEET_MCP_HTTP_BIND=0.0.0.0:8079

# Timeouts and Limits
SPREADSHEET_MCP_TOOL_TIMEOUT_MS=30000
SPREADSHEET_MCP_MAX_RESPONSE_BYTES=1000000

# Feature Flags
SPREADSHEET_MCP_RECALC_ENABLED=true
SPREADSHEET_MCP_VBA_ENABLED=false
SPREADSHEET_MCP_ALLOW_OVERWRITE=false

# Observability
RUST_LOG=info,spreadsheet_mcp=debug
RUST_BACKTRACE=1
OTEL_SERVICE_NAME=spreadsheet-mcp
OTEL_EXPORTER_OTLP_ENDPOINT=http://jaeger:4317

# Metrics
METRICS_ENABLED=true
METRICS_PORT=9090
PROMETHEUS_PUSHGATEWAY=http://pushgateway:9091
```

### 1.2 Config File Formats

ggen-mcp supports YAML and JSON configuration files:

```yaml
# config.yaml
workspace_root: "/data"
cache_capacity: 50
extensions:
  - xlsx
  - xlsm
  - xls
  - xlsb
transport: http
http_bind: "0.0.0.0:8079"
recalc_enabled: true
vba_enabled: false
max_concurrent_recalcs: 5
tool_timeout_ms: 60000
max_response_bytes: 5000000
allow_overwrite: false
```

**Best Practices:**
- ✅ Use YAML for human-readable configs
- ✅ Use JSON for programmatic generation
- ✅ Validate config files at startup
- ✅ Provide schema validation (JSON Schema/TOML)

### 1.3 Config Validation at Startup

Current implementation has excellent validation:

```rust
impl ServerConfig {
    pub fn validate(&self) -> Result<()> {
        // 1. Validate workspace_root exists and is readable
        anyhow::ensure!(
            self.workspace_root.exists(),
            "workspace root {:?} does not exist",
            self.workspace_root
        );

        // 2. Validate cache_capacity is reasonable
        anyhow::ensure!(
            self.cache_capacity >= MIN_CACHE_CAPACITY,
            "cache_capacity must be at least {}",
            MIN_CACHE_CAPACITY
        );

        // 3. Validate tool timeout is sane
        if let Some(timeout_ms) = self.tool_timeout_ms {
            anyhow::ensure!(
                timeout_ms >= MIN_TOOL_TIMEOUT_MS,
                "tool_timeout_ms must be at least {}ms",
                MIN_TOOL_TIMEOUT_MS
            );
        }

        Ok(())
    }
}
```

**Enhancement Recommendations:**

```rust
use validator::Validate;
use serde::Deserialize;

#[derive(Debug, Deserialize, Validate)]
pub struct ServerConfig {
    #[validate(custom = "validate_directory_exists")]
    pub workspace_root: PathBuf,

    #[validate(range(min = 1, max = 1000))]
    pub cache_capacity: usize,

    #[validate(range(min = 1024, max = 65535))]
    pub http_port: u16,

    #[validate(email)]
    pub admin_email: Option<String>,
}

fn validate_directory_exists(path: &PathBuf) -> Result<(), ValidationError> {
    if !path.exists() || !path.is_dir() {
        return Err(ValidationError::new("directory_not_found"));
    }
    Ok(())
}
```

### 1.4 Runtime Reconfiguration

**Pattern for Hot Reloading:**

```rust
use tokio::sync::watch;
use notify::{Watcher, RecursiveMode};

pub struct ConfigReloader {
    config_path: PathBuf,
    config_tx: watch::Sender<Arc<ServerConfig>>,
}

impl ConfigReloader {
    pub fn new(config_path: PathBuf, initial: Arc<ServerConfig>) -> Self {
        let (tx, _) = watch::channel(initial);
        Self {
            config_path,
            config_tx: tx,
        }
    }

    pub async fn watch(&mut self) -> Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);

        let mut watcher = notify::recommended_watcher(move |res| {
            if let Ok(event) = res {
                let _ = tx.blocking_send(event);
            }
        })?;

        watcher.watch(&self.config_path, RecursiveMode::NonRecursive)?;

        while let Some(event) = rx.recv().await {
            if matches!(event.kind, notify::EventKind::Modify(_)) {
                match ServerConfig::load(&self.config_path) {
                    Ok(new_config) => {
                        tracing::info!("reloaded configuration from {:?}", self.config_path);
                        self.config_tx.send(Arc::new(new_config)).ok();
                    }
                    Err(e) => {
                        tracing::error!("failed to reload config: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn subscribe(&self) -> watch::Receiver<Arc<ServerConfig>> {
        self.config_tx.subscribe()
    }
}
```

### 1.5 Feature Flags

**Pattern using `unleash` or simple boolean flags:**

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct FeatureFlags {
    flags: HashMap<String, bool>,
}

impl FeatureFlags {
    pub fn is_enabled(&self, flag: &str) -> bool {
        self.flags.get(flag).copied().unwrap_or(false)
    }

    pub fn percentage_rollout(&self, flag: &str, user_id: &str) -> bool {
        // Use consistent hashing for gradual rollout
        let hash = hash_user_flag(user_id, flag);
        let threshold = self.flags.get(&format!("{}_percentage", flag))
            .and_then(|&v| if v { Some(100) } else { None })
            .unwrap_or(0);
        (hash % 100) < threshold
    }
}

// Usage in server:
if state.feature_flags().is_enabled("new_caching_strategy") {
    // Use new implementation
} else {
    // Use old implementation
}
```

---

## 2. Logging and Tracing

### 2.1 tracing Crate Patterns

Current implementation:

```rust
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_writer(std::io::stderr)
        .try_init()
        .ok();
}
```

**Enhanced Production Setup:**

```rust
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

pub fn init_production_tracing() -> Result<()> {
    // File appender with daily rotation
    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        "/var/log/spreadsheet-mcp",
        "app.log",
    );

    // JSON formatting for structured logs
    let json_layer = fmt::layer()
        .json()
        .with_writer(file_appender)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    // Console layer for human-readable logs
    let console_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_target(true)
        .with_ansi(true);

    // OpenTelemetry layer for distributed tracing
    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name("spreadsheet-mcp")
        .with_endpoint("jaeger:6831")
        .install_simple()?;

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Combine all layers
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(json_layer)
        .with(console_layer)
        .with(otel_layer)
        .init();

    Ok(())
}
```

### 2.2 Structured Logging

**Best Practices:**

```rust
use tracing::{info, warn, error, debug, instrument};

#[instrument(
    skip(state),
    fields(
        workbook_id = %params.workbook_id,
        tool = "list_sheets"
    )
)]
pub async fn list_sheets(
    state: Arc<AppState>,
    params: ListSheetsParams,
) -> Result<SheetListResponse> {
    debug!("loading workbook from cache");
    let workbook = state.open_workbook(&params.workbook_id).await?;

    info!(
        sheet_count = workbook.sheets.len(),
        "loaded workbook sheets"
    );

    Ok(SheetListResponse {
        sheets: workbook.sheets.clone(),
    })
}

// Span for long-running operations
async fn process_workbook(path: &Path) -> Result<()> {
    let span = tracing::info_span!(
        "process_workbook",
        path = %path.display(),
        otel.kind = "internal"
    );

    let _enter = span.enter();

    debug!("starting workbook processing");
    // ... processing logic
    info!("workbook processing complete");

    Ok(())
}
```

### 2.3 Log Levels and Filtering

**Environment-based Configuration:**

```bash
# Production: structured info-level logs
RUST_LOG=info,spreadsheet_mcp=info,tower_http=debug

# Development: verbose debugging
RUST_LOG=debug,spreadsheet_mcp=trace,rmcp=debug

# Specific module debugging
RUST_LOG=info,spreadsheet_mcp::tools::fork=trace

# Exclude noisy modules
RUST_LOG=info,hyper=warn,tokio=warn
```

**Dynamic Log Level Adjustment:**

```rust
use tracing_subscriber::reload;

pub struct LogLevelController {
    handle: reload::Handle<EnvFilter, tracing_subscriber::Registry>,
}

impl LogLevelController {
    pub fn set_level(&self, new_level: &str) -> Result<()> {
        let filter = EnvFilter::try_new(new_level)?;
        self.handle.reload(filter)?;
        tracing::info!(new_level, "log level updated");
        Ok(())
    }
}

// Expose via admin endpoint:
// POST /admin/log-level
// Body: {"level": "debug"}
```

### 2.4 Distributed Tracing

**OpenTelemetry Integration:**

```rust
use opentelemetry::{
    global,
    trace::{Tracer, TracerProvider, SpanKind},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;

pub fn init_distributed_tracing() -> Result<()> {
    let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(otlp_endpoint)
        )
        .with_trace_config(
            opentelemetry::sdk::trace::config()
                .with_resource(opentelemetry::sdk::Resource::new(vec![
                    KeyValue::new("service.name", "spreadsheet-mcp"),
                    KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                ]))
        )
        .install_batch(opentelemetry::runtime::Tokio)?;

    global::set_tracer_provider(tracer);

    Ok(())
}

// Usage in tools:
#[instrument(
    name = "tool.read_table",
    skip(state),
    fields(
        otel.kind = ?SpanKind::Internal,
        tool.name = "read_table",
        workbook.id = %params.workbook_id,
    )
)]
pub async fn read_table(
    state: Arc<AppState>,
    params: ReadTableParams,
) -> Result<ReadTableResponse> {
    // Span is automatically created and traced
    // ...
}
```

### 2.5 Log Aggregation

**Recommended Stack:**

1. **Vector.dev** for log collection
2. **Loki** or **Elasticsearch** for storage
3. **Grafana** for visualization

**Vector Configuration:**

```toml
# /etc/vector/vector.toml
[sources.app_logs]
type = "file"
include = ["/var/log/spreadsheet-mcp/*.log"]
read_from = "beginning"

[transforms.parse_json]
type = "remap"
inputs = ["app_logs"]
source = '''
. = parse_json!(.message)
.timestamp = to_timestamp!(.timestamp)
'''

[sinks.loki]
type = "loki"
inputs = ["parse_json"]
endpoint = "http://loki:3100"
encoding.codec = "json"
labels.service = "spreadsheet-mcp"
labels.env = "production"
```

---

## 3. Metrics and Observability

### 3.1 Prometheus Metrics

**Implementation using `prometheus` crate:**

```rust
use prometheus::{
    Registry, Counter, Histogram, HistogramOpts, IntGauge, IntCounter,
    Opts, register_counter, register_histogram, register_int_gauge,
};
use once_cell::sync::Lazy;

// Global metrics registry
pub static REGISTRY: Lazy<Registry> = Lazy::new(Registry::new);

// Request metrics
pub static REQUESTS_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    let counter = IntCounter::new(
        "spreadsheet_mcp_requests_total",
        "Total number of MCP tool requests"
    ).unwrap();
    REGISTRY.register(Box::new(counter.clone())).unwrap();
    counter
});

pub static REQUEST_DURATION: Lazy<Histogram> = Lazy::new(|| {
    let opts = HistogramOpts::new(
        "spreadsheet_mcp_request_duration_seconds",
        "MCP tool request duration in seconds"
    )
    .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]);

    let histogram = Histogram::with_opts(opts).unwrap();
    REGISTRY.register(Box::new(histogram.clone())).unwrap();
    histogram
});

// Cache metrics
pub static CACHE_SIZE: Lazy<IntGauge> = Lazy::new(|| {
    let gauge = IntGauge::new(
        "spreadsheet_mcp_cache_size",
        "Current number of workbooks in cache"
    ).unwrap();
    REGISTRY.register(Box::new(gauge.clone())).unwrap();
    gauge
});

pub static CACHE_HITS: Lazy<IntCounter> = Lazy::new(|| {
    let counter = IntCounter::new(
        "spreadsheet_mcp_cache_hits_total",
        "Total number of cache hits"
    ).unwrap();
    REGISTRY.register(Box::new(counter.clone())).unwrap();
    counter
});

pub static CACHE_MISSES: Lazy<IntCounter> = Lazy::new(|| {
    let counter = IntCounter::new(
        "spreadsheet_mcp_cache_misses_total",
        "Total number of cache misses"
    ).unwrap();
    REGISTRY.register(Box::new(counter.clone())).unwrap();
    counter
});

// Fork/recalc metrics
pub static ACTIVE_FORKS: Lazy<IntGauge> = Lazy::new(|| {
    let gauge = IntGauge::new(
        "spreadsheet_mcp_active_forks",
        "Current number of active forks"
    ).unwrap();
    REGISTRY.register(Box::new(gauge.clone())).unwrap();
    gauge
});

pub static RECALC_DURATION: Lazy<Histogram> = Lazy::new(|| {
    let opts = HistogramOpts::new(
        "spreadsheet_mcp_recalc_duration_seconds",
        "LibreOffice recalculation duration"
    )
    .buckets(vec![0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0]);

    let histogram = Histogram::with_opts(opts).unwrap();
    REGISTRY.register(Box::new(histogram.clone())).unwrap();
    histogram
});

// Error metrics
pub static ERRORS_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    let counter = IntCounter::new(
        "spreadsheet_mcp_errors_total",
        "Total number of errors"
    ).unwrap();
    REGISTRY.register(Box::new(counter.clone())).unwrap();
    counter
});
```

**Metrics Endpoint:**

```rust
use axum::{
    routing::get,
    Router,
    response::IntoResponse,
};
use prometheus::{Encoder, TextEncoder};

async fn metrics_handler() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();

    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    (
        [(axum::http::header::CONTENT_TYPE, encoder.format_type())],
        buffer,
    )
}

pub fn metrics_router() -> Router {
    Router::new()
        .route("/metrics", get(metrics_handler))
}
```

### 3.2 Custom Metrics Collection

**Instrumenting Application Code:**

```rust
use crate::metrics::*;

impl AppState {
    pub async fn open_workbook(&self, workbook_id: &WorkbookId) -> Result<Arc<WorkbookContext>> {
        // Track request
        REQUESTS_TOTAL.inc();
        let timer = REQUEST_DURATION.start_timer();

        // Update cache size gauge
        CACHE_SIZE.set(self.cache.read().len() as i64);

        let result = {
            let mut cache = self.cache.write();
            if let Some(entry) = cache.get(&canonical) {
                CACHE_HITS.inc();
                return Ok(entry.clone());
            }

            CACHE_MISSES.inc();

            // Load workbook...
            let workbook = task::spawn_blocking(move || {
                WorkbookContext::load(&config, &path_buf)
            }).await??;

            let workbook = Arc::new(workbook);
            cache.put(canonical.clone(), workbook.clone());

            CACHE_SIZE.set(cache.len() as i64);

            Ok(workbook)
        };

        // Stop timer and record duration
        timer.observe_duration();

        if result.is_err() {
            ERRORS_TOTAL.inc();
        }

        result
    }
}
```

### 3.3 Performance Dashboards

**Grafana Dashboard JSON:**

```json
{
  "dashboard": {
    "title": "Spreadsheet MCP Production",
    "panels": [
      {
        "title": "Request Rate",
        "targets": [
          {
            "expr": "rate(spreadsheet_mcp_requests_total[5m])",
            "legendFormat": "requests/sec"
          }
        ]
      },
      {
        "title": "Request Duration (p50, p95, p99)",
        "targets": [
          {
            "expr": "histogram_quantile(0.50, rate(spreadsheet_mcp_request_duration_seconds_bucket[5m]))",
            "legendFormat": "p50"
          },
          {
            "expr": "histogram_quantile(0.95, rate(spreadsheet_mcp_request_duration_seconds_bucket[5m]))",
            "legendFormat": "p95"
          },
          {
            "expr": "histogram_quantile(0.99, rate(spreadsheet_mcp_request_duration_seconds_bucket[5m]))",
            "legendFormat": "p99"
          }
        ]
      },
      {
        "title": "Cache Hit Rate",
        "targets": [
          {
            "expr": "rate(spreadsheet_mcp_cache_hits_total[5m]) / (rate(spreadsheet_mcp_cache_hits_total[5m]) + rate(spreadsheet_mcp_cache_misses_total[5m]))",
            "legendFormat": "hit_rate"
          }
        ]
      },
      {
        "title": "Active Forks",
        "targets": [
          {
            "expr": "spreadsheet_mcp_active_forks",
            "legendFormat": "forks"
          }
        ]
      },
      {
        "title": "Error Rate",
        "targets": [
          {
            "expr": "rate(spreadsheet_mcp_errors_total[5m])",
            "legendFormat": "errors/sec"
          }
        ]
      }
    ]
  }
}
```

### 3.4 SLO Monitoring

**Service Level Objectives:**

```yaml
# slo.yaml
slos:
  - name: availability
    target: 99.9
    window: 30d
    indicator:
      type: availability
      good: sum(rate(spreadsheet_mcp_requests_total{status!~"5.."}[5m]))
      total: sum(rate(spreadsheet_mcp_requests_total[5m]))

  - name: latency_p95
    target: 95.0
    window: 30d
    indicator:
      type: latency
      percentile: 0.95
      threshold: 1.0  # 1 second
      metric: spreadsheet_mcp_request_duration_seconds

  - name: error_rate
    target: 99.0
    window: 7d
    indicator:
      type: error_rate
      good: sum(rate(spreadsheet_mcp_requests_total{error="false"}[5m]))
      total: sum(rate(spreadsheet_mcp_requests_total[5m]))
```

### 3.5 Alert Rules

**Prometheus Alert Rules:**

```yaml
# alerts.yaml
groups:
  - name: spreadsheet_mcp
    interval: 30s
    rules:
      - alert: HighErrorRate
        expr: |
          rate(spreadsheet_mcp_errors_total[5m]) > 0.05
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High error rate detected"
          description: "Error rate is {{ $value | humanizePercentage }} for the last 5 minutes"

      - alert: HighLatency
        expr: |
          histogram_quantile(0.95, rate(spreadsheet_mcp_request_duration_seconds_bucket[5m])) > 2.0
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High latency detected"
          description: "P95 latency is {{ $value }}s"

      - alert: CacheHitRateLow
        expr: |
          rate(spreadsheet_mcp_cache_hits_total[10m]) /
          (rate(spreadsheet_mcp_cache_hits_total[10m]) + rate(spreadsheet_mcp_cache_misses_total[10m])) < 0.6
        for: 10m
        labels:
          severity: info
        annotations:
          summary: "Cache hit rate is low"
          description: "Hit rate is {{ $value | humanizePercentage }}, consider increasing cache size"

      - alert: ServiceDown
        expr: up{job="spreadsheet-mcp"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Service is down"
          description: "Spreadsheet MCP service has been down for more than 1 minute"
```

---

## 4. Health Checks

### 4.1 Liveness Endpoints

**Implementation:**

```rust
use axum::{
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct LivenessResponse {
    status: String,
    timestamp: String,
}

/// Liveness probe - checks if the service is running
/// Returns 200 if process is alive, 503 otherwise
pub async fn liveness_handler() -> impl IntoResponse {
    // Simple check: if we can respond, we're alive
    (
        StatusCode::OK,
        Json(LivenessResponse {
            status: "alive".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    )
}
```

### 4.2 Readiness Endpoints

**Implementation:**

```rust
#[derive(Serialize)]
pub struct ReadinessResponse {
    status: String,
    checks: HashMap<String, CheckStatus>,
    timestamp: String,
}

#[derive(Serialize)]
pub struct CheckStatus {
    status: String,
    message: Option<String>,
    latency_ms: Option<u64>,
}

/// Readiness probe - checks if service can handle traffic
/// Returns 200 if ready, 503 if not ready
pub async fn readiness_handler(
    state: Arc<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut checks = HashMap::new();
    let mut all_healthy = true;

    // Check workspace access
    let workspace_check = check_workspace(&state.config()).await;
    all_healthy &= workspace_check.status == "healthy";
    checks.insert("workspace".to_string(), workspace_check);

    // Check cache health
    let cache_check = check_cache(&state).await;
    all_healthy &= cache_check.status == "healthy";
    checks.insert("cache".to_string(), cache_check);

    // Check recalc backend (if enabled)
    #[cfg(feature = "recalc")]
    if state.config().recalc_enabled {
        let recalc_check = check_recalc_backend(&state).await;
        all_healthy &= recalc_check.status == "healthy";
        checks.insert("recalc".to_string(), recalc_check);
    }

    let status_code = if all_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    Ok((
        status_code,
        Json(ReadinessResponse {
            status: if all_healthy { "ready" } else { "not_ready" }.to_string(),
            checks,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    ))
}

async fn check_workspace(config: &ServerConfig) -> CheckStatus {
    let start = std::time::Instant::now();

    match tokio::fs::read_dir(&config.workspace_root).await {
        Ok(_) => CheckStatus {
            status: "healthy".to_string(),
            message: Some("workspace accessible".to_string()),
            latency_ms: Some(start.elapsed().as_millis() as u64),
        },
        Err(e) => CheckStatus {
            status: "unhealthy".to_string(),
            message: Some(format!("workspace not accessible: {}", e)),
            latency_ms: None,
        },
    }
}

async fn check_cache(state: &AppState) -> CheckStatus {
    let stats = state.cache_stats();

    CheckStatus {
        status: "healthy".to_string(),
        message: Some(format!(
            "cache: {}/{} entries, hit_rate: {:.2}%",
            stats.size,
            stats.capacity,
            stats.hit_rate() * 100.0
        )),
        latency_ms: Some(0),
    }
}

#[cfg(feature = "recalc")]
async fn check_recalc_backend(state: &AppState) -> CheckStatus {
    let start = std::time::Instant::now();

    if let Some(backend) = state.recalc_backend() {
        if backend.is_available() {
            CheckStatus {
                status: "healthy".to_string(),
                message: Some("LibreOffice available".to_string()),
                latency_ms: Some(start.elapsed().as_millis() as u64),
            }
        } else {
            CheckStatus {
                status: "degraded".to_string(),
                message: Some("LibreOffice not available".to_string()),
                latency_ms: None,
            }
        }
    } else {
        CheckStatus {
            status: "disabled".to_string(),
            message: Some("recalc backend not configured".to_string()),
            latency_ms: None,
        }
    }
}
```

### 4.3 Dependency Health Checks

**Checking External Dependencies:**

```rust
#[derive(Serialize)]
pub struct DependencyHealth {
    name: String,
    status: String,
    response_time_ms: Option<u64>,
    error: Option<String>,
}

async fn check_dependencies() -> Vec<DependencyHealth> {
    let mut checks = Vec::new();

    // Check if we can spawn LibreOffice
    #[cfg(feature = "recalc")]
    checks.push(check_libreoffice().await);

    // Check disk space
    checks.push(check_disk_space().await);

    // Check memory
    checks.push(check_memory().await);

    checks
}

#[cfg(feature = "recalc")]
async fn check_libreoffice() -> DependencyHealth {
    let start = std::time::Instant::now();

    match tokio::process::Command::new("soffice")
        .arg("--version")
        .output()
        .await
    {
        Ok(output) if output.status.success() => DependencyHealth {
            name: "libreoffice".to_string(),
            status: "healthy".to_string(),
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            error: None,
        },
        Ok(output) => DependencyHealth {
            name: "libreoffice".to_string(),
            status: "unhealthy".to_string(),
            response_time_ms: None,
            error: Some(format!("exit code: {}", output.status)),
        },
        Err(e) => DependencyHealth {
            name: "libreoffice".to_string(),
            status: "unavailable".to_string(),
            response_time_ms: None,
            error: Some(e.to_string()),
        },
    }
}

async fn check_disk_space() -> DependencyHealth {
    use sysinfo::{DiskExt, System, SystemExt};

    let mut sys = System::new_all();
    sys.refresh_disks_list();

    let workspace_disk = sys.disks().iter()
        .find(|disk| disk.mount_point() == Path::new("/data"));

    match workspace_disk {
        Some(disk) => {
            let available = disk.available_space();
            let total = disk.total_space();
            let percent_used = 100.0 - (available as f64 / total as f64 * 100.0);

            let status = if percent_used > 90.0 {
                "critical"
            } else if percent_used > 80.0 {
                "warning"
            } else {
                "healthy"
            };

            DependencyHealth {
                name: "disk_space".to_string(),
                status: status.to_string(),
                response_time_ms: Some(0),
                error: if status != "healthy" {
                    Some(format!("{}% used", percent_used as u64))
                } else {
                    None
                },
            }
        }
        None => DependencyHealth {
            name: "disk_space".to_string(),
            status: "unknown".to_string(),
            response_time_ms: None,
            error: Some("workspace disk not found".to_string()),
        },
    }
}
```

### 4.4 Circuit Breaker State

**Pattern for Circuit Breaker:**

```rust
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,  // Normal operation
    Open,    // Failing, rejecting requests
    HalfOpen, // Testing if service recovered
}

pub struct CircuitBreaker {
    state: Arc<parking_lot::RwLock<CircuitState>>,
    failure_count: AtomicUsize,
    success_count: AtomicUsize,
    last_failure: AtomicU64,
    config: CircuitBreakerConfig,
}

#[derive(Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: usize,
    pub success_threshold: usize,
    pub timeout_ms: u64,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: Arc::new(parking_lot::RwLock::new(CircuitState::Closed)),
            failure_count: AtomicUsize::new(0),
            success_count: AtomicUsize::new(0),
            last_failure: AtomicU64::new(0),
            config,
        }
    }

    pub fn is_open(&self) -> bool {
        *self.state.read() == CircuitState::Open
    }

    pub fn record_success(&self) {
        let state = *self.state.read();

        match state {
            CircuitState::Closed => {
                self.failure_count.store(0, Ordering::Relaxed);
            }
            CircuitState::HalfOpen => {
                let successes = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                if successes >= self.config.success_threshold {
                    *self.state.write() = CircuitState::Closed;
                    self.failure_count.store(0, Ordering::Relaxed);
                    self.success_count.store(0, Ordering::Relaxed);
                    tracing::info!("circuit breaker closed");
                }
            }
            CircuitState::Open => {
                // Check if we should transition to HalfOpen
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                let last_failure = self.last_failure.load(Ordering::Relaxed);

                if now - last_failure > self.config.timeout_ms {
                    *self.state.write() = CircuitState::HalfOpen;
                    self.success_count.store(0, Ordering::Relaxed);
                    tracing::info!("circuit breaker half-open");
                }
            }
        }
    }

    pub fn record_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        self.last_failure.store(now, Ordering::Relaxed);

        if failures >= self.config.failure_threshold {
            *self.state.write() = CircuitState::Open;
            tracing::warn!(failures, "circuit breaker opened");
        }
    }

    pub fn get_state(&self) -> CircuitState {
        *self.state.read()
    }
}

// Health check returns circuit breaker state
async fn check_circuit_breakers(state: &AppState) -> CheckStatus {
    let recalc_circuit = state.recalc_circuit_breaker();

    CheckStatus {
        status: match recalc_circuit.get_state() {
            CircuitState::Closed => "healthy",
            CircuitState::HalfOpen => "degraded",
            CircuitState::Open => "unhealthy",
        }.to_string(),
        message: Some(format!("circuit: {:?}", recalc_circuit.get_state())),
        latency_ms: Some(0),
    }
}
```

### 4.5 Cache Statistics

**Exposing Cache Metrics via Health Endpoint:**

```rust
#[derive(Serialize)]
pub struct CacheHealthResponse {
    status: String,
    statistics: CacheStatistics,
    timestamp: String,
}

#[derive(Serialize)]
pub struct CacheStatistics {
    size: usize,
    capacity: usize,
    utilization_percent: f64,
    hit_rate_percent: f64,
    total_operations: u64,
    total_hits: u64,
    total_misses: u64,
}

pub async fn cache_health_handler(
    state: Arc<AppState>,
) -> Json<CacheHealthResponse> {
    let stats = state.cache_stats();

    Json(CacheHealthResponse {
        status: "healthy".to_string(),
        statistics: CacheStatistics {
            size: stats.size,
            capacity: stats.capacity,
            utilization_percent: (stats.size as f64 / stats.capacity as f64) * 100.0,
            hit_rate_percent: stats.hit_rate() * 100.0,
            total_operations: stats.operations,
            total_hits: stats.hits,
            total_misses: stats.misses,
        },
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}
```

---

## 5. Graceful Shutdown

### 5.1 Signal Handling (SIGTERM, SIGINT)

**Current Implementation:**

```rust
// In lib.rs
tokio::select! {
    result = server.run() => {
        result.map_err(anyhow::Error::from)?;
        return Ok(());
    }
    ctrl = tokio::signal::ctrl_c() => {
        match ctrl {
            Ok(_) => tracing::info!("shutdown signal received"),
            Err(error) => tracing::warn!(?error, "ctrl_c listener exited unexpectedly"),
        }
    }
}
```

**Enhanced Signal Handling:**

```rust
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::broadcast;

pub async fn wait_for_shutdown_signal() -> ShutdownReason {
    let mut sigterm = signal(SignalKind::terminate())
        .expect("failed to register SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt())
        .expect("failed to register SIGINT handler");
    let mut sighup = signal(SignalKind::hangup())
        .expect("failed to register SIGHUP handler");

    tokio::select! {
        _ = sigterm.recv() => {
            tracing::info!("received SIGTERM, initiating graceful shutdown");
            ShutdownReason::Sigterm
        }
        _ = sigint.recv() => {
            tracing::info!("received SIGINT (Ctrl+C), initiating graceful shutdown");
            ShutdownReason::Sigint
        }
        _ = sighup.recv() => {
            tracing::info!("received SIGHUP, reloading configuration");
            ShutdownReason::Sighup
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ShutdownReason {
    Sigterm,
    Sigint,
    Sighup,
}
```

### 5.2 Connection Draining

**HTTP Server Graceful Shutdown:**

```rust
use axum::extract::ConnectInfo;
use tokio::sync::Notify;
use std::net::SocketAddr;

pub async fn run_http_server(
    app: Router,
    bind_addr: SocketAddr,
    shutdown_signal: Arc<Notify>,
) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;

    tracing::info!("HTTP server listening on {}", bind_addr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>()
    )
    .with_graceful_shutdown(async move {
        shutdown_signal.notified().await;
        tracing::info!("HTTP server received shutdown signal, draining connections...");

        // Give connections time to finish (30 seconds)
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    })
    .await?;

    tracing::info!("HTTP server shut down gracefully");
    Ok(())
}
```

### 5.3 Cleanup Ordering

**Coordinated Shutdown Sequence:**

```rust
pub struct ShutdownCoordinator {
    shutdown_tx: broadcast::Sender<()>,
    components: Vec<Box<dyn ShutdownComponent>>,
}

#[async_trait::async_trait]
pub trait ShutdownComponent: Send + Sync {
    async fn shutdown(&self) -> Result<()>;
    fn name(&self) -> &str;
    fn priority(&self) -> u8; // Lower = earlier shutdown
}

impl ShutdownCoordinator {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1);
        Self {
            shutdown_tx: tx,
            components: Vec::new(),
        }
    }

    pub fn register<C: ShutdownComponent + 'static>(&mut self, component: C) {
        self.components.push(Box::new(component));
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        tracing::info!("initiating coordinated shutdown");

        // Broadcast shutdown signal to all listeners
        let _ = self.shutdown_tx.send(());

        // Sort by priority (lower priority shuts down first)
        self.components.sort_by_key(|c| c.priority());

        for component in &self.components {
            let name = component.name();
            tracing::info!(component = name, "shutting down component");

            match tokio::time::timeout(
                Duration::from_secs(30),
                component.shutdown()
            ).await {
                Ok(Ok(())) => {
                    tracing::info!(component = name, "component shut down successfully");
                }
                Ok(Err(e)) => {
                    tracing::error!(component = name, error = ?e, "component shutdown failed");
                }
                Err(_) => {
                    tracing::error!(component = name, "component shutdown timed out");
                }
            }
        }

        tracing::info!("coordinated shutdown complete");
        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }
}

// Example components:

struct HttpServerComponent {
    server_handle: tokio::task::JoinHandle<()>,
}

#[async_trait::async_trait]
impl ShutdownComponent for HttpServerComponent {
    async fn shutdown(&self) -> Result<()> {
        // HTTP server shutdown is handled by its own graceful shutdown
        Ok(())
    }

    fn name(&self) -> &str {
        "http_server"
    }

    fn priority(&self) -> u8 {
        10 // Shutdown early to stop accepting new requests
    }
}

struct ForkRegistryComponent {
    registry: Arc<ForkRegistry>,
}

#[async_trait::async_trait]
impl ShutdownComponent for ForkRegistryComponent {
    async fn shutdown(&self) -> Result<()> {
        // Stop cleanup task
        self.registry.stop_cleanup_task().await;

        // Optionally save active forks metadata
        tracing::info!("saving fork registry state");

        Ok(())
    }

    fn name(&self) -> &str {
        "fork_registry"
    }

    fn priority(&self) -> u8 {
        20
    }
}

struct CacheComponent {
    state: Arc<AppState>,
}

#[async_trait::async_trait]
impl ShutdownComponent for CacheComponent {
    async fn shutdown(&self) -> Result<()> {
        let stats = self.state.cache_stats();
        tracing::info!(
            cache_size = stats.size,
            hit_rate = stats.hit_rate(),
            "clearing cache"
        );

        // Cache will be dropped automatically
        Ok(())
    }

    fn name(&self) -> &str {
        "workbook_cache"
    }

    fn priority(&self) -> u8 {
        30
    }
}

struct MetricsComponent;

#[async_trait::async_trait]
impl ShutdownComponent for MetricsComponent {
    async fn shutdown(&self) -> Result<()> {
        // Push final metrics to pushgateway
        tracing::info!("pushing final metrics");

        // Flush OpenTelemetry traces
        opentelemetry::global::shutdown_tracer_provider();

        Ok(())
    }

    fn name(&self) -> &str {
        "metrics"
    }

    fn priority(&self) -> u8 {
        90 // Shutdown last to capture all metrics
    }
}
```

### 5.4 Timeout Handling

**Enforcing Shutdown Timeouts:**

```rust
pub async fn run_with_shutdown(
    config: ServerConfig,
) -> Result<()> {
    let mut coordinator = ShutdownCoordinator::new();

    // Register components
    // ...

    // Wait for shutdown signal
    let shutdown_reason = wait_for_shutdown_signal().await;

    // Enforce total shutdown timeout
    const MAX_SHUTDOWN_TIME: Duration = Duration::from_secs(60);

    match tokio::time::timeout(MAX_SHUTDOWN_TIME, coordinator.shutdown()).await {
        Ok(Ok(())) => {
            tracing::info!("graceful shutdown completed");
            Ok(())
        }
        Ok(Err(e)) => {
            tracing::error!(error = ?e, "shutdown failed");
            Err(e)
        }
        Err(_) => {
            tracing::error!(
                timeout_secs = MAX_SHUTDOWN_TIME.as_secs(),
                "shutdown timed out, forcing exit"
            );
            std::process::exit(1);
        }
    }
}
```

### 5.5 State Persistence

**Persisting Critical State on Shutdown:**

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ServerSnapshot {
    timestamp: String,
    cache_stats: CacheStats,
    active_forks: Vec<ForkMetadata>,
    config: ServerConfigSnapshot,
}

pub async fn save_snapshot(state: &AppState) -> Result<()> {
    let snapshot = ServerSnapshot {
        timestamp: chrono::Utc::now().to_rfc3339(),
        cache_stats: state.cache_stats(),
        active_forks: state.fork_registry()
            .map(|r| r.list_forks())
            .unwrap_or_default(),
        config: ServerConfigSnapshot::from(state.config().as_ref()),
    };

    let snapshot_path = "/var/lib/spreadsheet-mcp/snapshot.json";
    let json = serde_json::to_string_pretty(&snapshot)?;

    tokio::fs::write(snapshot_path, json).await?;
    tracing::info!("saved server snapshot to {}", snapshot_path);

    Ok(())
}

pub async fn restore_snapshot(state: &AppState) -> Result<()> {
    let snapshot_path = "/var/lib/spreadsheet-mcp/snapshot.json";

    if !tokio::fs::try_exists(snapshot_path).await? {
        return Ok(());
    }

    let json = tokio::fs::read_to_string(snapshot_path).await?;
    let snapshot: ServerSnapshot = serde_json::from_str(&json)?;

    tracing::info!(
        timestamp = snapshot.timestamp,
        "restoring from snapshot"
    );

    // Restore fork registry state
    if let Some(registry) = state.fork_registry() {
        for fork_meta in snapshot.active_forks {
            // Re-register forks if they still exist
            if tokio::fs::try_exists(&fork_meta.path).await? {
                registry.restore_fork(fork_meta)?;
            }
        }
    }

    Ok(())
}
```

---

## 6. Container Deployment

### 6.1 Dockerfile Best Practices

**Current Dockerfiles Analysis:**

ggen-mcp has two Dockerfiles:
1. **Dockerfile** - Minimal (distroless, read-only)
2. **Dockerfile.full** - Full (LibreOffice, recalc features)

**Enhancements:**

```dockerfile
# syntax=docker/dockerfile:1.4

# ============================================================================
# Stage 1: Builder with dependency caching
# ============================================================================
FROM rust:1.91.1-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    musl-tools \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Create layer for dependencies (cached until Cargo.toml changes)
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release --locked --features recalc && \
    rm -rf src

# Copy source and build
COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    touch src/main.rs && \
    cargo build --release --locked --features recalc && \
    strip target/release/spreadsheet-mcp && \
    cp target/release/spreadsheet-mcp /usr/local/bin/

# ============================================================================
# Stage 2: Runtime with minimal footprint
# ============================================================================
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libreoffice-calc-nogui \
    default-jre-headless \
    fonts-liberation \
    fonts-noto-core \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 10000 -s /bin/bash mcpuser

# Security: Run as non-root
USER mcpuser
WORKDIR /home/mcpuser

# Copy binary
COPY --from=builder --chown=mcpuser:mcpuser /usr/local/bin/spreadsheet-mcp /usr/local/bin/

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/spreadsheet-mcp", "--help"] || exit 1

# Expose ports
EXPOSE 8079 9090

# Environment defaults
ENV SPREADSHEET_MCP_WORKSPACE=/data \
    SPREADSHEET_MCP_TRANSPORT=http \
    SPREADSHEET_MCP_HTTP_BIND=0.0.0.0:8079 \
    RUST_LOG=info \
    RUST_BACKTRACE=1

# Volume for data
VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/spreadsheet-mcp"]
CMD ["--workspace-root", "/data", "--transport", "http", "--http-bind", "0.0.0.0:8079"]
```

### 6.2 Multi-stage Builds

**Optimization Techniques:**

```dockerfile
# Builder with cross-compilation support
FROM --platform=$BUILDPLATFORM rust:1.91.1-bookworm AS builder
ARG TARGETPLATFORM
ARG BUILDPLATFORM

# Install cross-compilation tools
RUN case "$TARGETPLATFORM" in \
      "linux/amd64") ARCH=x86_64 ;; \
      "linux/arm64") ARCH=aarch64 ;; \
      *) echo "Unsupported platform: $TARGETPLATFORM" && exit 1 ;; \
    esac && \
    rustup target add ${ARCH}-unknown-linux-musl

# ... build steps with target arch
```

### 6.3 Image Optimization

**Size Reduction Techniques:**

```dockerfile
# Use multi-stage builds
# Use distroless or Alpine for minimal runtime
# Strip debug symbols
RUN strip target/release/spreadsheet-mcp

# Use layer caching effectively
# Place rarely-changing layers first

# Minimize runtime dependencies
RUN apt-get install --no-install-recommends \
    && rm -rf /var/lib/apt/lists/*

# Use .dockerignore
# .dockerignore file:
target/
.git/
.github/
tests/
docs/
*.md
.dockerignore
Dockerfile*
```

### 6.4 Resource Limits

**Docker Compose with Resource Constraints:**

```yaml
# docker-compose.yml
version: '3.8'

services:
  spreadsheet-mcp:
    image: spreadsheet-mcp:latest
    container_name: spreadsheet-mcp
    restart: unless-stopped

    # Resource limits
    deploy:
      resources:
        limits:
          cpus: '2.0'
          memory: 2G
        reservations:
          cpus: '0.5'
          memory: 512M

    # Ulimits
    ulimits:
      nofile:
        soft: 65536
        hard: 65536
      nproc:
        soft: 2048
        hard: 2048

    # Environment
    environment:
      SPREADSHEET_MCP_CACHE_CAPACITY: 50
      SPREADSHEET_MCP_MAX_CONCURRENT_RECALCS: 2
      RUST_LOG: info

    # Health check
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8079/health/live"]
      interval: 30s
      timeout: 3s
      retries: 3
      start_period: 10s

    # Ports
    ports:
      - "8079:8079"  # MCP HTTP
      - "9090:9090"  # Metrics

    # Volumes
    volumes:
      - ./data:/data
      - ./logs:/var/log/spreadsheet-mcp

    # Networks
    networks:
      - mcp-network

networks:
  mcp-network:
    driver: bridge
```

**Kubernetes Resource Limits:**

```yaml
# kubernetes/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: spreadsheet-mcp
  labels:
    app: spreadsheet-mcp
spec:
  replicas: 3
  selector:
    matchLabels:
      app: spreadsheet-mcp
  template:
    metadata:
      labels:
        app: spreadsheet-mcp
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
        prometheus.io/path: "/metrics"
    spec:
      containers:
      - name: spreadsheet-mcp
        image: spreadsheet-mcp:latest
        imagePullPolicy: Always

        # Resource requests and limits
        resources:
          requests:
            cpu: 500m
            memory: 512Mi
          limits:
            cpu: 2000m
            memory: 2Gi

        # Health checks
        livenessProbe:
          httpGet:
            path: /health/live
            port: 8079
          initialDelaySeconds: 10
          periodSeconds: 30
          timeoutSeconds: 3
          failureThreshold: 3

        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8079
          initialDelaySeconds: 5
          periodSeconds: 10
          timeoutSeconds: 3
          failureThreshold: 3

        # Ports
        ports:
        - name: http
          containerPort: 8079
          protocol: TCP
        - name: metrics
          containerPort: 9090
          protocol: TCP

        # Environment
        env:
        - name: SPREADSHEET_MCP_WORKSPACE
          value: "/data"
        - name: SPREADSHEET_MCP_CACHE_CAPACITY
          value: "50"
        - name: RUST_LOG
          value: "info"
        - name: POD_NAME
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        - name: POD_NAMESPACE
          valueFrom:
            fieldRef:
              fieldPath: metadata.namespace

        # Volumes
        volumeMounts:
        - name: data
          mountPath: /data
        - name: logs
          mountPath: /var/log/spreadsheet-mcp

      # Security context
      securityContext:
        runAsNonRoot: true
        runAsUser: 10000
        fsGroup: 10000

      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: spreadsheet-mcp-data
      - name: logs
        emptyDir: {}
```

### 6.5 Security Scanning

**Trivy Scan in CI/CD:**

```yaml
# .github/workflows/security-scan.yml
name: Security Scan

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
  schedule:
    - cron: '0 0 * * *'  # Daily at midnight

jobs:
  trivy:
    name: Trivy Security Scan
    runs-on: ubuntu-latest

    steps:
      - name: Checkout code
        uses: actions/checkout@v3

      - name: Build Docker image
        run: docker build -t spreadsheet-mcp:test -f Dockerfile.full .

      - name: Run Trivy vulnerability scanner
        uses: aquasecurity/trivy-action@master
        with:
          image-ref: 'spreadsheet-mcp:test'
          format: 'sarif'
          output: 'trivy-results.sarif'
          severity: 'CRITICAL,HIGH'

      - name: Upload Trivy results to GitHub Security
        uses: github/codeql-action/upload-sarif@v2
        with:
          sarif_file: 'trivy-results.sarif'

      - name: Run Trivy for misconfigurations
        run: |
          trivy config --severity CRITICAL,HIGH .

      - name: Scan Cargo dependencies
        run: |
          cargo install cargo-audit
          cargo audit
```

---

## 7. Monitoring and Alerting

### 7.1 Error Rate Monitoring

**Prometheus Query:**

```promql
# Error rate (errors per second)
rate(spreadsheet_mcp_errors_total[5m])

# Error percentage
(rate(spreadsheet_mcp_errors_total[5m]) /
 rate(spreadsheet_mcp_requests_total[5m])) * 100

# Errors by type/tool
rate(spreadsheet_mcp_errors_total[5m]) by (error_type, tool_name)
```

**Alert Rule:**

```yaml
- alert: HighErrorRate
  expr: |
    (rate(spreadsheet_mcp_errors_total[5m]) /
     rate(spreadsheet_mcp_requests_total[5m])) > 0.05
  for: 5m
  labels:
    severity: warning
    component: spreadsheet-mcp
  annotations:
    summary: "High error rate detected"
    description: "Error rate is {{ $value | humanizePercentage }} (threshold: 5%)"
    runbook_url: "https://runbooks.example.com/high-error-rate"
```

### 7.2 Latency Percentiles (p50, p95, p99)

**Prometheus Queries:**

```promql
# p50 latency
histogram_quantile(0.50,
  rate(spreadsheet_mcp_request_duration_seconds_bucket[5m]))

# p95 latency
histogram_quantile(0.95,
  rate(spreadsheet_mcp_request_duration_seconds_bucket[5m]))

# p99 latency
histogram_quantile(0.99,
  rate(spreadsheet_mcp_request_duration_seconds_bucket[5m]))

# Latency by tool
histogram_quantile(0.95,
  rate(spreadsheet_mcp_request_duration_seconds_bucket[5m])) by (tool_name)
```

### 7.3 Resource Utilization

**System Metrics:**

```promql
# CPU usage
rate(process_cpu_seconds_total{job="spreadsheet-mcp"}[5m]) * 100

# Memory usage (MB)
process_resident_memory_bytes{job="spreadsheet-mcp"} / 1024 / 1024

# Memory usage percentage
(process_resident_memory_bytes{job="spreadsheet-mcp"} /
 node_memory_MemTotal_bytes) * 100

# Open file descriptors
process_open_fds{job="spreadsheet-mcp"}

# Disk I/O
rate(process_disk_read_bytes_total[5m])
rate(process_disk_written_bytes_total[5m])
```

### 7.4 Custom Alerts

**Alert Rules Configuration:**

```yaml
groups:
  - name: spreadsheet_mcp_slo
    interval: 30s
    rules:
      - alert: SLOAvailabilityBreach
        expr: |
          (1 - (rate(spreadsheet_mcp_requests_total{status!~"5.."}[30d]) /
                rate(spreadsheet_mcp_requests_total[30d]))) > 0.001
        labels:
          severity: critical
          slo: availability
        annotations:
          summary: "SLO availability breach"
          description: "Availability is below 99.9% over 30 days"

      - alert: SLOLatencyBreach
        expr: |
          histogram_quantile(0.95,
            rate(spreadsheet_mcp_request_duration_seconds_bucket[30d])) > 1.0
        labels:
          severity: warning
          slo: latency
        annotations:
          summary: "SLO latency breach"
          description: "P95 latency exceeds 1s over 30 days"

  - name: spreadsheet_mcp_capacity
    interval: 1m
    rules:
      - alert: CacheCapacityHigh
        expr: spreadsheet_mcp_cache_size / spreadsheet_mcp_cache_capacity > 0.9
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Cache capacity nearly full"
          description: "Cache is {{ $value | humanizePercentage }} full"

      - alert: ConcurrentRecalcsHigh
        expr: spreadsheet_mcp_concurrent_recalcs > 8
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High number of concurrent recalculations"
          description: "{{ $value }} recalcs in progress (consider increasing max_concurrent_recalcs)"

      - alert: DiskSpacelow
        expr: |
          (node_filesystem_avail_bytes{mountpoint="/data"} /
           node_filesystem_size_bytes{mountpoint="/data"}) < 0.1
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Disk space critically low"
          description: "Only {{ $value | humanizePercentage }} disk space remaining"
```

### 7.5 Runbook Automation

**PagerDuty Integration:**

```rust
use reqwest::Client;
use serde_json::json;

pub struct PagerDutyClient {
    api_key: String,
    integration_key: String,
    client: Client,
}

impl PagerDutyClient {
    pub async fn trigger_incident(
        &self,
        summary: &str,
        severity: &str,
        details: serde_json::Value,
    ) -> Result<()> {
        let payload = json!({
            "routing_key": self.integration_key,
            "event_action": "trigger",
            "payload": {
                "summary": summary,
                "severity": severity,
                "source": "spreadsheet-mcp",
                "custom_details": details,
            }
        });

        self.client
            .post("https://events.pagerduty.com/v2/enqueue")
            .header("Authorization", format!("Token token={}", self.api_key))
            .json(&payload)
            .send()
            .await?;

        Ok(())
    }
}

// Auto-remediation example:
pub async fn auto_remediate_cache_pressure(state: &AppState) -> Result<()> {
    let stats = state.cache_stats();

    if (stats.size as f64 / stats.capacity as f64) > 0.9 {
        tracing::warn!("cache pressure detected, triggering cleanup");

        // Clear 20% of cache (LRU)
        let to_evict = (stats.size as f64 * 0.2) as usize;
        // ... eviction logic

        tracing::info!(evicted = to_evict, "cache cleanup complete");
    }

    Ok(())
}
```

---

## 8. Security Hardening

### 8.1 Dependency Scanning

**Cargo Audit in CI:**

```yaml
# .github/workflows/security.yml
- name: Security audit
  run: |
    cargo install cargo-audit
    cargo audit --deny warnings
```

**Automated Dependency Updates:**

```yaml
# .github/dependabot.yml
version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "weekly"
    open-pull-requests-limit: 10
    labels:
      - "dependencies"
      - "security"
```

### 8.2 Vulnerability Management

**RUSTSEC Advisory Monitoring:**

```bash
# Check for security advisories
cargo deny check advisories

# .cargo/deny.toml
[advisories]
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/rustsec/advisory-db"]
vulnerability = "deny"
unmaintained = "warn"
yanked = "deny"
notice = "warn"
```

### 8.3 Least Privilege

**Container Security:**

```dockerfile
# Run as non-root user
RUN useradd -m -u 10000 -s /bin/bash mcpuser
USER mcpuser

# Drop capabilities
RUN setcap cap_net_bind_service=+ep /usr/local/bin/spreadsheet-mcp
```

**Kubernetes Security Context:**

```yaml
securityContext:
  runAsNonRoot: true
  runAsUser: 10000
  runAsGroup: 10000
  fsGroup: 10000
  capabilities:
    drop:
      - ALL
    add:
      - NET_BIND_SERVICE
  readOnlyRootFilesystem: true
  allowPrivilegeEscalation: false
```

### 8.4 Secret Management

**Using External Secret Stores:**

```rust
use aws_sdk_secretsmanager::Client as SecretsClient;

pub struct SecretManager {
    client: SecretsClient,
}

impl SecretManager {
    pub async fn get_secret(&self, secret_id: &str) -> Result<String> {
        let response = self.client
            .get_secret_value()
            .secret_id(secret_id)
            .send()
            .await?;

        let secret = response
            .secret_string()
            .ok_or_else(|| anyhow!("secret not found"))?;

        Ok(secret.to_string())
    }
}

// Usage:
let secrets = SecretManager::new().await?;
let api_key = secrets.get_secret("spreadsheet-mcp/api-key").await?;
```

**Environment Variable Security:**

```rust
// Never log secrets
#[instrument(skip(api_key))]
async fn authenticate(api_key: &str) -> Result<()> {
    // ...
}

// Redact sensitive fields
#[derive(Debug)]
struct Config {
    #[debug(skip)]
    api_key: String,
    workspace: PathBuf,
}
```

### 8.5 Audit Logging

**Security Event Logging:**

```rust
use tracing::event;

pub struct SecurityAudit;

impl SecurityAudit {
    pub fn log_authentication_attempt(
        user_id: &str,
        success: bool,
        reason: Option<&str>,
    ) {
        event!(
            tracing::Level::WARN,
            event_type = "authentication",
            user_id = user_id,
            success = success,
            reason = reason,
            "authentication attempt"
        );
    }

    pub fn log_authorization_failure(
        user_id: &str,
        resource: &str,
        action: &str,
    ) {
        event!(
            tracing::Level::WARN,
            event_type = "authorization_failure",
            user_id = user_id,
            resource = resource,
            action = action,
            "unauthorized access attempt"
        );
    }

    pub fn log_data_access(
        user_id: &str,
        workbook_id: &str,
        operation: &str,
    ) {
        event!(
            tracing::Level::INFO,
            event_type = "data_access",
            user_id = user_id,
            workbook_id = workbook_id,
            operation = operation,
            "data access"
        );
    }
}
```

---

## Production Deployment Checklist

See [DEPLOYMENT_CHECKLIST.md](./DEPLOYMENT_CHECKLIST.md) for a comprehensive pre-deployment checklist.

---

## TPS Gemba Principles in Production

**"Go and See" in Production:**

1. **Real-time Observability**: Use distributed tracing to see the entire request flow
2. **Visual Management**: Dashboards make problems visible immediately (Andon)
3. **Metrics-Driven**: Measure everything, optimize what matters
4. **Fail-Fast**: Health checks and circuit breakers prevent cascading failures
5. **Continuous Improvement**: Use production metrics to drive kaizen

**Production Metrics are your Gemba** - they show you where the real work happens and where problems occur.

---

## References

- [Rust Tracing Documentation](https://tracing.rs/)
- [Prometheus Best Practices](https://prometheus.io/docs/practices/)
- [OpenTelemetry Rust](https://opentelemetry.io/docs/instrumentation/rust/)
- [Kubernetes Production Best Practices](https://kubernetes.io/docs/concepts/configuration/overview/)
- [12-Factor App Methodology](https://12factor.net/)
- [TPS Gemba Documentation](./TPS_GEMBA.md)
