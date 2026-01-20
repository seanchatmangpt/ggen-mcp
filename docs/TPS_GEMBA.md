# Gemba (現場) Principles for MCP Servers

**Document Version**: 1.0
**Last Updated**: 2026-01-20
**Status**: Research and Best Practices Guide

---

## Table of Contents

1. [Introduction to Gemba](#introduction-to-gemba)
2. [Gemba Principles for MCP Servers](#gemba-principles-for-mcp-servers)
3. [Current Observability Analysis](#current-observability-analysis)
4. [Real-Time Monitoring](#real-time-monitoring)
5. [Structured Logging Strategies](#structured-logging-strategies)
6. [Distributed Tracing for MCP](#distributed-tracing-for-mcp)
7. [Performance Profiling](#performance-profiling)
8. [Production Debugging](#production-debugging)
9. [User Behavior Analytics](#user-behavior-analytics)
10. [Data-Driven Decision Making](#data-driven-decision-making)
11. [Implementation Roadmap](#implementation-roadmap)
12. [References](#references)

---

## Introduction to Gemba

### What is Gemba?

**Gemba** (現場, "the actual place") is a core principle of the Toyota Production System (TPS) that emphasizes going to the source to observe actual processes, understand problems, and make data-driven decisions. In manufacturing, this means going to the factory floor. In software, this means observing actual system behavior in production.

### Core Gemba Principles

1. **Go and See** (Genchi Genbutsu) - Observe the actual system, not reports about it
2. **Ask Why Five Times** - Root cause analysis through deep observation
3. **Respect for People** - Understand how users actually interact with the system
4. **Continuous Improvement** (Kaizen) - Use observations to drive incremental improvements
5. **Waste Elimination** (Muda) - Identify and eliminate inefficiencies through observation

### Gemba Applied to MCP Servers

For MCP servers, Gemba means:

- **Observing actual MCP protocol messages** being exchanged
- **Watching how LLM agents use tools** in real scenarios
- **Measuring actual performance** under production load
- **Understanding real failure modes** rather than theoretical ones
- **Analyzing user behavior patterns** to improve tool design
- **Debugging issues where they occur** (production environment)

---

## Gemba Principles for MCP Servers

### 1. Real-Time Observability

**Principle**: You cannot improve what you cannot see.

The MCP server must provide visibility into:
- **Tool invocations**: Which tools are called, with what parameters, and how often
- **Request flow**: Complete lifecycle of each MCP request
- **Resource usage**: Memory, CPU, file handles, cache hit rates
- **Error patterns**: What fails, when, and why
- **Performance characteristics**: Latency distribution, throughput, bottlenecks

### 2. Production-First Debugging

**Principle**: The production environment is the ultimate source of truth.

- Most bugs only manifest under real workloads
- Synthetic tests cannot replicate all edge cases
- Users interact with the system in unexpected ways
- Production data reveals actual usage patterns

### 3. User Journey Observation

**Principle**: Understand how the system is actually used, not how you think it's used.

- Track sequences of tool calls (workflow patterns)
- Identify common error paths
- Discover which features are heavily used vs. ignored
- Understand failure recovery patterns

### 4. Data-Driven Decisions

**Principle**: Decisions should be based on observation, not assumption.

- Measure before optimizing
- Track metrics before and after changes
- Use A/B testing for feature improvements
- Validate hypotheses with production data

### 5. Continuous Feedback Loops

**Principle**: Short feedback cycles enable rapid improvement.

- Real-time dashboards for immediate insights
- Automated alerts for anomalies
- Weekly review of key metrics
- Post-mortem analysis for incidents

---

## Current Observability Analysis

### Existing Strengths

#### 1. Comprehensive Audit Trail System ✅

**Location**: `src/audit/mod.rs`, `src/audit/integration.rs`

**Capabilities**:
- Structured event logging (JSON Lines format)
- Persistent storage with automatic rotation (100MB files, 30-day retention)
- In-memory buffer (10,000 events)
- Event filtering and querying
- Automatic timestamp and duration tracking

**Event Types Tracked**:
```rust
pub enum AuditEventType {
    ToolInvocation,
    ForkCreate, ForkEdit, ForkRecalc, ForkSave, ForkDiscard,
    CheckpointCreate, CheckpointRestore, CheckpointDelete,
    StagedChangeCreate, StagedChangeApply, StagedChangeDiscard,
    FileRead, FileWrite, FileCopy, FileDelete,
    DirectoryCreate, DirectoryDelete,
    WorkbookOpen, WorkbookClose, WorkbookList,
    Error,
}
```

**Example Audit Event**:
```json
{
  "event_id": "evt-abc123",
  "timestamp": "2026-01-20T10:30:45.123Z",
  "event_type": "tool_invocation",
  "outcome": "success",
  "resource": "list_workbooks",
  "details": {
    "params": {"filter": "*.xlsx"}
  },
  "duration_ms": 45
}
```

#### 2. Structured Tracing with Spans ✅

**Location**: `src/main.rs`, throughout codebase

**Implementation**:
- Using `tracing` crate for structured logging
- Environment-based log level configuration
- Hierarchical span tracking
- Target filtering support

**Current Usage**:
```rust
// Span creation for operations
let span = audit_tool_span("list_workbooks", &params);
let _enter = span.enter();

// Structured field logging
tracing::info!(
    transport = %config.transport,
    workspace = %config.workspace_root.display(),
    "starting spreadsheet MCP server",
);
```

#### 3. Cache Statistics ✅

**Location**: `src/state.rs`

**Metrics Collected**:
```rust
pub struct CacheStats {
    pub operations: u64,  // Total cache operations
    pub hits: u64,        // Cache hits
    pub misses: u64,      // Cache misses
    pub size: usize,      // Current cache size
    pub capacity: usize,  // Max cache capacity
}
```

**Hit Rate Calculation**:
```rust
pub fn hit_rate(&self) -> f64 {
    if self.operations == 0 {
        0.0
    } else {
        self.hits as f64 / self.operations as f64
    }
}
```

#### 4. Recovery and Resilience Monitoring ✅

**Location**: `src/recovery/`

**Components**:
- **Circuit Breaker**: Tracks failure counts, state transitions
- **Retry Logic**: Logs retry attempts with exponential backoff
- **Partial Success**: Tracks success/failure ratios in batch operations
- **Fallback Strategies**: Records when fallbacks are triggered

**Observability Features**:
```rust
// Circuit breaker state tracking
pub enum CircuitBreakerState {
    Closed,    // Normal operation
    Open,      // Failing, rejecting requests
    HalfOpen,  // Testing recovery
}

// Logged on state transitions
warn!(
    operation = operation_name,
    attempt = context.attempt,
    max_attempts = context.max_attempts,
    delay_ms = delay.as_millis(),
    error = %err,
    "retrying operation"
);
```

### Gaps and Opportunities

#### 1. No Distributed Tracing Integration ⚠️

**Current State**: Spans are logged but not propagated across system boundaries.

**Missing**:
- OpenTelemetry integration
- Trace context propagation (MCP request → tool execution → external services)
- Distributed trace visualization (Jaeger, Zipkin)
- Cross-service correlation

**Impact**: Cannot trace requests end-to-end in distributed deployments.

#### 2. No Metrics Aggregation System ⚠️

**Current State**: Cache stats are collected but not exported.

**Missing**:
- Prometheus metrics endpoint
- StatsD integration
- Histogram/Summary metrics for latencies
- Counter/Gauge metrics for resources
- Rate metrics for throughput

**Impact**: No real-time monitoring dashboards, no alerting on anomalies.

#### 3. No Health Check Endpoints ⚠️

**Current State**: No standardized health/readiness endpoints.

**Missing**:
- `/health` endpoint for liveness checks
- `/ready` endpoint for readiness checks
- `/metrics` endpoint for Prometheus scraping
- Dependency health checks (LibreOffice availability, disk space)

**Impact**: Difficult to integrate with orchestration systems (Kubernetes, Docker Swarm).

#### 4. Limited Performance Profiling ⚠️

**Current State**: Duration tracking in audit events, but no detailed profiling.

**Missing**:
- CPU profiling integration (pprof)
- Memory allocation tracking
- Flame graphs for hot paths
- Async task profiling
- Lock contention analysis

**Impact**: Difficult to identify performance bottlenecks in production.

#### 5. No User Behavior Analytics ⚠️

**Current State**: Tool invocations are logged but not analyzed for patterns.

**Missing**:
- Tool usage frequency analysis
- Workflow pattern detection (common tool sequences)
- Session tracking and correlation
- User cohort analysis
- Feature adoption metrics

**Impact**: Cannot make data-driven decisions about feature prioritization.

#### 6. No Real-Time Dashboards ⚠️

**Current State**: Logs are written to files but not visualized.

**Missing**:
- Grafana dashboards
- Real-time metric visualization
- Alerting rules (PagerDuty, Slack)
- SLO/SLI tracking
- Anomaly detection

**Impact**: Reactive rather than proactive operations.

#### 7. Limited Error Context ⚠️

**Current State**: Errors are logged with messages but minimal context.

**Missing**:
- Stack trace capture in production
- Error aggregation and grouping (Sentry, Rollbar)
- Error frequency trends
- Error impact analysis (% of requests affected)
- Contextual metadata (workbook size, operation complexity)

**Impact**: Difficult to prioritize and debug production errors.

#### 8. No Request/Response Instrumentation ⚠️

**Current State**: MCP protocol messages not captured for analysis.

**Missing**:
- Request/response size tracking
- MCP message type distribution
- Tool parameter analysis
- Response payload size optimization
- Request rate limiting metrics

**Impact**: Cannot optimize payload sizes or detect abuse.

---

## Real-Time Monitoring

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                          MCP Server                             │
│                                                                 │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐      │
│  │   Tracing    │   │   Metrics    │   │  Audit Logs  │      │
│  │  (OpenTelem) │   │ (Prometheus) │   │   (JSONL)    │      │
│  └──────┬───────┘   └──────┬───────┘   └──────┬───────┘      │
│         │                  │                  │               │
└─────────┼──────────────────┼──────────────────┼───────────────┘
          │                  │                  │
          │                  │                  │
          v                  v                  v
    ┌─────────┐        ┌──────────┐      ┌──────────┐
    │ Jaeger  │        │ Grafana  │      │   Loki   │
    │         │        │          │      │          │
    └─────────┘        └──────────┘      └──────────┘
         │                   │                 │
         └───────────────────┴─────────────────┘
                             │
                             v
                    ┌─────────────────┐
                    │  Operations     │
                    │  Dashboard      │
                    └─────────────────┘
```

### Key Metrics to Monitor

#### System Health Metrics

```rust
// Counter: Total requests processed
mcp_requests_total{method="tools/list", status="success"}

// Histogram: Request duration
mcp_request_duration_seconds{method="tools/call", tool="list_workbooks"}

// Gauge: Active connections
mcp_active_connections

// Gauge: Cache hit rate
cache_hit_rate{cache_type="workbook"}

// Counter: Error rate by type
mcp_errors_total{error_type="validation", severity="warning"}
```

#### Tool-Specific Metrics

```rust
// Counter: Tool invocation count
tool_invocations_total{tool="read_table", outcome="success"}

// Histogram: Tool execution duration
tool_duration_seconds{tool="sheet_overview"}

// Counter: Tool parameter validation failures
tool_validation_errors_total{tool="edit_batch", field="sheet_name"}

// Gauge: Active forks
fork_count{state="active"}

// Counter: Fork operations
fork_operations_total{operation="create", outcome="success"}
```

#### Resource Metrics

```rust
// Gauge: Memory usage
process_resident_memory_bytes

// Gauge: Open file descriptors
process_open_fds

// Counter: Workbooks loaded
workbooks_loaded_total

// Gauge: Cache size
cache_entries{cache_type="workbook"}

// Gauge: Temporary files count
temp_files_count{directory="forks"}
```

#### Performance Metrics

```rust
// Histogram: LibreOffice recalc duration
recalc_duration_seconds

// Counter: Region detection timeouts
region_detection_timeouts_total

// Histogram: Workbook load time
workbook_load_duration_seconds

// Histogram: Sheet parse time
sheet_parse_duration_seconds{classification="likely_data"}
```

### Monitoring Dashboard Layout

#### Overview Dashboard

```
┌─────────────────────────────────────────────────────────────────┐
│                     MCP Server Overview                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Request Rate       Error Rate         P95 Latency             │
│  ┌─────────┐        ┌─────────┐        ┌─────────┐            │
│  │ 450/min │        │  0.2%   │        │  125ms  │            │
│  └─────────┘        └─────────┘        └─────────┘            │
│                                                                 │
│  Active Connections  Cache Hit Rate    Memory Usage            │
│  ┌─────────┐        ┌─────────┐        ┌─────────┐            │
│  │    12   │        │  92.3%  │        │  2.1GB  │            │
│  └─────────┘        └─────────┘        └─────────┘            │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│  Request Rate (5min avg)                                        │
│  ▁▂▃▅▆▇██▇▆▅▃▂▁                                                │
│                                                                 │
│  Error Rate (5min avg)                                          │
│  ▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁                                              │
│                                                                 │
│  P95 Latency (5min avg)                                         │
│  ▃▄▄▃▃▄▅▅▄▃▃▄▄▃▃                                              │
└─────────────────────────────────────────────────────────────────┘
```

#### Tool Usage Dashboard

```
┌─────────────────────────────────────────────────────────────────┐
│                    Tool Usage Analytics                         │
├─────────────────────────────────────────────────────────────────┤
│  Top 10 Tools (24h)                                             │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━                       │
│  list_workbooks        ████████████████ 1,234                   │
│  read_table            ████████████ 892                          │
│  sheet_overview        ██████████ 756                            │
│  range_values          ████████ 623                              │
│  find_value            ██████ 445                                │
│  formula_trace         ████ 312                                  │
│  edit_batch            ███ 267                                   │
│  create_fork           ██ 198                                    │
│  recalculate           ██ 156                                    │
│  get_changeset         █ 89                                      │
│                                                                 │
│  Tool Error Rates                                               │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━                       │
│  recalculate          ▓▓▓▓▓ 2.3%                                │
│  region_detection     ▓▓▓ 1.2%                                  │
│  edit_batch           ▓▓ 0.8%                                   │
│  sheet_overview       ▓ 0.4%                                    │
│  list_workbooks       ▁ 0.1%                                    │
│                                                                 │
│  Tool Latency Distribution (P50/P95/P99)                        │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━                       │
│  recalculate          45ms / 1.2s / 3.5s                        │
│  sheet_overview       89ms / 450ms / 890ms                      │
│  read_table           23ms / 120ms / 250ms                      │
│  list_workbooks       12ms / 45ms / 78ms                        │
└─────────────────────────────────────────────────────────────────┘
```

#### Error Tracking Dashboard

```
┌─────────────────────────────────────────────────────────────────┐
│                      Error Analysis                             │
├─────────────────────────────────────────────────────────────────┤
│  Error Rate Trend (24h)                                         │
│  │                                                              │
│  │     ▁                                                        │
│  │    ▁█▁                                                       │
│  │  ▁▁███▁▁▁                                                    │
│  │▁▁███████▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁                             │
│  └──────────────────────────────────────────────────────       │
│     00:00  06:00  12:00  18:00  24:00                          │
│                                                                 │
│  Top Errors (24h)                                               │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━                       │
│  1. validation_error: Invalid sheet name "Sheet 1!" (67)        │
│  2. timeout: LibreOffice recalc timeout (23)                    │
│  3. not_found: Workbook not found (19)                          │
│  4. corrupted: Failed to parse workbook (12)                    │
│  5. limit_exceeded: Cache capacity exceeded (8)                 │
│                                                                 │
│  Error Impact                                                   │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━                       │
│  Requests affected:  0.3% (129 / 43,567)                        │
│  Users affected:     2.1% (4 / 189)                             │
│  Tools affected:     6 / 42                                     │
└─────────────────────────────────────────────────────────────────┘
```

### Alert Rules

#### Critical Alerts (Page Immediately)

```yaml
# Error rate spike
- alert: HighErrorRate
  expr: rate(mcp_errors_total[5m]) > 0.05
  annotations:
    summary: "Error rate above 5%"
    description: "{{ $value }}% of requests failing"

# Service down
- alert: ServiceDown
  expr: up{job="mcp-server"} == 0
  for: 1m
  annotations:
    summary: "MCP server is down"

# Memory leak
- alert: MemoryLeak
  expr: |
    rate(process_resident_memory_bytes[1h]) > 10485760
  for: 15m
  annotations:
    summary: "Memory growing at 10MB/hour"
```

#### Warning Alerts (Investigate)

```yaml
# High latency
- alert: HighLatency
  expr: |
    histogram_quantile(0.95,
      rate(mcp_request_duration_seconds_bucket[5m])
    ) > 2.0
  for: 5m
  annotations:
    summary: "P95 latency above 2s"

# Low cache hit rate
- alert: LowCacheHitRate
  expr: cache_hit_rate < 0.7
  for: 10m
  annotations:
    summary: "Cache hit rate below 70%"

# Circuit breaker open
- alert: CircuitBreakerOpen
  expr: circuit_breaker_state{state="open"} > 0
  annotations:
    summary: "Circuit breaker in open state"
```

---

## Structured Logging Strategies

### Log Levels and When to Use Them

#### ERROR - Actionable Failures

Use for errors that require immediate attention:
```rust
error!(
    workbook_id = %id,
    error = %err,
    "failed to load workbook - file may be corrupted"
);
```

**When to use ERROR**:
- Operations that should always succeed but failed
- Data corruption detected
- External service failures (LibreOffice crash)
- Security violations

#### WARN - Degraded Operation

Use for issues that are handled but indicate problems:
```rust
warn!(
    operation = "region_detection",
    sheet = %sheet_name,
    cell_count = metrics.non_empty_cells,
    "using fallback due to complexity"
);
```

**When to use WARN**:
- Fallback strategies activated
- Retry attempts triggered
- Validation failures (user input)
- Resource limits approached

#### INFO - Significant Events

Use for important state changes and operations:
```rust
info!(
    event_type = "fork_created",
    fork_id = %fork_id,
    base_workbook = %workbook_id,
    duration_ms = start.elapsed().as_millis(),
    "fork created successfully"
);
```

**When to use INFO**:
- Tool invocations
- State transitions (fork created, checkpoint saved)
- Configuration changes
- Startup/shutdown events

#### DEBUG - Detailed Flow

Use for understanding program flow during debugging:
```rust
debug!(
    cache_hit = cache_hit,
    workbook_id = %id,
    "checking workbook cache"
);
```

**When to use DEBUG**:
- Cache hit/miss details
- Decision points in algorithms
- Intermediate calculation results
- Lock acquisition/release

#### TRACE - Verbose Detail

Use for extremely detailed debugging:
```rust
trace!(
    row = row,
    col = col,
    value = ?cell_value,
    "processing cell"
);
```

**When to use TRACE**:
- Per-cell processing
- Every iteration of loops
- Fine-grained state changes
- Protocol message details

### Structured Fields Best Practices

#### Use Semantic Field Names

**Good**:
```rust
info!(
    workbook_id = %id,
    cache_hit_rate = cache_stats.hit_rate(),
    cache_size = cache_stats.size,
    "cache statistics"
);
```

**Bad**:
```rust
info!("cache: hit_rate={} size={}", cache_stats.hit_rate(), cache_stats.size);
```

#### Include Contextual Metadata

```rust
info!(
    // Identify the resource
    workbook_id = %id,
    fork_id = ?fork_id,

    // Operation context
    operation = "recalculate",
    user_id = ?user_id,

    // Performance data
    duration_ms = elapsed.as_millis(),

    // Result metadata
    cells_changed = changeset.len(),

    "recalculation completed"
);
```

#### Use Field Types Appropriately

```rust
// Display trait (%)
workbook_id = %id,

// Debug trait (?)
error = ?err,

// As-is
duration_ms = 123,
success = true,
```

### Log Correlation

#### Request ID Propagation

```rust
use tracing::Span;
use uuid::Uuid;

// Create request-scoped span
let request_id = Uuid::new_v4();
let span = info_span!("mcp_request", request_id = %request_id);
let _guard = span.enter();

// All logs within this scope will include request_id
info!("processing tool call");  // Includes request_id automatically
```

#### Hierarchical Spans

```rust
// Outer span for tool invocation
let tool_span = info_span!(
    "tool_invocation",
    tool = "read_table",
    request_id = %request_id
);
let _tool_guard = tool_span.enter();

// Inner span for sub-operations
let parse_span = info_span!("parse_range");
let _parse_guard = parse_span.enter();

// Nested span for even finer granularity
let validate_span = info_span!("validate_bounds");
// ...
```

### Log Sampling for High-Volume Operations

```rust
use std::sync::atomic::{AtomicU64, Ordering};

static LOG_COUNTER: AtomicU64 = AtomicU64::new(0);

// Log only every 1000th cell processed
fn process_cell(row: u32, col: u32, value: &CellValue) {
    let count = LOG_COUNTER.fetch_add(1, Ordering::Relaxed);

    if count % 1000 == 0 {
        trace!(
            cells_processed = count,
            current_cell = %format!("{}{}", column_number_to_name(col), row),
            "processing cells"
        );
    }

    // ... actual processing ...
}
```

### Error Context Enrichment

```rust
use anyhow::Context;

fn load_workbook(path: &Path) -> Result<Workbook> {
    let file = File::open(path)
        .with_context(|| format!("failed to open workbook: {}", path.display()))?;

    let workbook = parse_workbook(file)
        .with_context(|| format!("failed to parse workbook: {}", path.display()))?;

    Ok(workbook)
}

// When logged, provides full error chain:
// Error: failed to load workbook
// Caused by:
//     failed to parse workbook: /path/to/file.xlsx
// Caused by:
//     invalid ZIP format
```

### Log Aggregation Strategy

#### Production Log Flow

```
Application Logs (stderr)
    │
    │ JSON Lines format
    │
    v
Docker Container
    │
    │ docker logs
    │
    v
Log Aggregator (Fluentd/Logstash)
    │
    │ Parse, enrich, route
    │
    ├──> Loki (structured log storage)
    │
    ├──> Elasticsearch (full-text search)
    │
    └──> S3 (long-term archival)
```

#### JSON Log Format

```json
{
  "timestamp": "2026-01-20T10:30:45.123456Z",
  "level": "info",
  "target": "spreadsheet_mcp::server",
  "fields": {
    "request_id": "req-abc123",
    "tool": "read_table",
    "workbook_id": "wb-def456",
    "duration_ms": 145,
    "rows_returned": 500
  },
  "message": "tool invocation completed",
  "span": {
    "name": "tool_invocation",
    "fields": {
      "request_id": "req-abc123"
    }
  }
}
```

---

## Distributed Tracing for MCP

### Why Distributed Tracing Matters

MCP servers often interact with multiple components:

```
LLM Agent
    │
    │ MCP Protocol
    v
MCP Server
    │
    ├──> Workbook Cache
    │
    ├──> File System
    │
    ├──> LibreOffice (subprocess)
    │
    └──> Audit Logger
```

Distributed tracing allows you to:
- **Follow requests end-to-end** across all components
- **Identify bottlenecks** in the request path
- **Understand dependencies** between operations
- **Measure component latency** individually

### OpenTelemetry Integration

#### Architecture

```rust
use opentelemetry::global;
use opentelemetry::sdk::trace::{self, RandomIdGenerator, Sampler};
use opentelemetry::sdk::Resource;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

fn init_telemetry() -> Result<()> {
    // Configure OTLP exporter (sends to Jaeger/Tempo)
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://localhost:4317")
        )
        .with_trace_config(
            trace::config()
                .with_sampler(Sampler::AlwaysOn)
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(Resource::new(vec![
                    KeyValue::new("service.name", "spreadsheet-mcp"),
                    KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                ]))
        )
        .install_batch(opentelemetry::runtime::Tokio)?;

    // Create tracing subscriber that exports to OTLP
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    // Combine with existing logging
    let subscriber = Registry::default()
        .with(tracing_subscriber::fmt::layer())
        .with(telemetry);

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}
```

#### Instrumenting Tool Handlers

```rust
use tracing::instrument;

#[instrument(
    skip(state),
    fields(
        tool = "list_workbooks",
        workbook_count = tracing::field::Empty,
        cache_hit_rate = tracing::field::Empty,
    )
)]
async fn list_workbooks(
    state: Arc<AppState>,
    params: ListWorkbooksParams,
) -> Result<WorkbookListResponse> {
    let response = state.list_workbooks(params.filter)?;

    // Record dynamic fields
    Span::current().record("workbook_count", response.workbooks.len());

    let cache_stats = state.cache_stats();
    Span::current().record("cache_hit_rate", cache_stats.hit_rate());

    Ok(response)
}
```

#### Cross-Process Trace Propagation

When calling LibreOffice as a subprocess:

```rust
use opentelemetry::global;
use opentelemetry::propagation::TextMapPropagator;

async fn recalculate_with_tracing(path: &Path) -> Result<RecalcResult> {
    let cx = tracing::Span::current().context();

    // Extract trace context
    let mut carrier = HashMap::new();
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&cx, &mut carrier);
    });

    // Pass trace context via environment variables
    let mut cmd = Command::new("soffice");
    for (key, value) in carrier {
        cmd.env(format!("OTEL_{}", key), value);
    }

    // Execute with trace context
    let result = cmd.output().await?;

    Ok(parse_recalc_result(result)?)
}
```

#### Trace Visualization in Jaeger

```
Trace: req-abc123 (total: 1.2s)
│
├─ tool_invocation: read_table (1.2s)
│  │
│  ├─ cache_lookup (2ms) ✓
│  │
│  ├─ load_workbook (450ms)
│  │  │
│  │  ├─ file_open (5ms) ✓
│  │  │
│  │  ├─ parse_xml (380ms)
│  │  │  │
│  │  │  ├─ parse_workbook (120ms) ✓
│  │  │  │
│  │  │  ├─ parse_sheets (200ms)
│  │  │  │
│  │  │  └─ parse_styles (60ms) ✓
│  │  │
│  │  └─ build_cache (65ms) ✓
│  │
│  ├─ region_detection (280ms)
│  │  │
│  │  ├─ compute_metrics (45ms) ✓
│  │  │
│  │  ├─ detect_regions (220ms)
│  │  │  │
│  │  │  ├─ occupancy_grid (90ms) ✓
│  │  │  │
│  │  │  └─ classify_regions (130ms) ✓
│  │  │
│  │  └─ cache_regions (15ms) ✓
│  │
│  └─ extract_table_data (468ms)
│     │
│     ├─ parse_range (12ms) ✓
│     │
│     ├─ read_cells (420ms)
│     │
│     └─ format_response (36ms) ✓
│
└─ audit_log (8ms) ✓
```

### Sampling Strategies

For high-throughput servers, sample traces to reduce overhead:

```rust
use opentelemetry::sdk::trace::Sampler;

// Sample 10% of requests
Sampler::TraceIdRatioBased(0.1)

// Always sample errors, 5% of successes
Sampler::ParentBased(Box::new(
    CustomSampler {
        error_rate: 1.0,
        success_rate: 0.05,
    }
))
```

---

## Performance Profiling

### Profiling in Production

#### Continuous Profiling with pprof

```rust
use pprof::ProfilerGuard;

// Enable CPU profiling via HTTP endpoint
#[get("/debug/pprof/profile")]
async fn profile(duration_secs: Query<u64>) -> Result<Vec<u8>> {
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(1000) // 1000 samples/sec
        .blocklist(&["libc", "pthread"])
        .build()?;

    tokio::time::sleep(Duration::from_secs(duration_secs.0)).await;

    let report = guard.report().build()?;
    let mut buffer = Vec::new();
    report.flamegraph(&mut buffer)?;

    Ok(buffer)
}
```

#### Memory Profiling

```rust
use jemalloc_ctl::{stats, epoch};

#[get("/debug/memory")]
async fn memory_stats() -> Result<Json<MemoryStats>> {
    epoch::mib()?.advance()?;

    let allocated = stats::allocated::mib()?.read()?;
    let resident = stats::resident::mib()?.read()?;
    let metadata = stats::metadata::mib()?.read()?;

    Ok(Json(MemoryStats {
        allocated_bytes: allocated,
        resident_bytes: resident,
        metadata_bytes: metadata,
        fragmentation_ratio: resident as f64 / allocated as f64,
    }))
}
```

#### Async Task Profiling

```rust
use tokio_metrics::RuntimeMonitor;

let monitor = RuntimeMonitor::new(&runtime);

tokio::spawn(async move {
    for metrics in monitor.intervals() {
        warn_if_overloaded(&metrics);

        // Log every 30 seconds
        tokio::time::sleep(Duration::from_secs(30)).await;

        info!(
            workers_count = metrics.workers_count,
            total_park_count = metrics.total_park_count,
            max_park_count = metrics.max_park_count,
            total_busy_duration_ms = metrics.total_busy_duration.as_millis(),
            "tokio runtime metrics"
        );
    }
});

fn warn_if_overloaded(metrics: &RuntimeMetrics) {
    // Detect saturated runtime
    if metrics.total_busy_duration > Duration::from_secs(25) {
        warn!("tokio runtime saturated - consider increasing worker threads");
    }
}
```

### Identifying Bottlenecks

#### Cache Performance Analysis

```rust
// Add detailed cache metrics
pub struct DetailedCacheStats {
    pub operations: u64,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub size: usize,
    pub capacity: usize,

    // Performance metrics
    pub avg_lookup_time_ns: u64,
    pub avg_insertion_time_ns: u64,

    // Hit rate by workbook type
    pub xlsx_hit_rate: f64,
    pub xlsm_hit_rate: f64,
}

impl AppState {
    pub fn detailed_cache_stats(&self) -> DetailedCacheStats {
        // Collect detailed statistics
        // ...
    }
}
```

#### Hot Path Detection

```rust
use std::time::Instant;

struct PerformanceTracker {
    operation_times: DashMap<String, Vec<Duration>>,
}

impl PerformanceTracker {
    pub fn track<T>(&self, operation: &str, f: impl FnOnce() -> T) -> T {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();

        self.operation_times
            .entry(operation.to_string())
            .or_insert_with(Vec::new)
            .push(duration);

        result
    }

    pub fn report_percentiles(&self, operation: &str) {
        if let Some(times) = self.operation_times.get(operation) {
            let mut sorted = times.clone();
            sorted.sort();

            let p50 = sorted[sorted.len() / 2];
            let p95 = sorted[sorted.len() * 95 / 100];
            let p99 = sorted[sorted.len() * 99 / 100];

            info!(
                operation = operation,
                p50_ms = p50.as_millis(),
                p95_ms = p95.as_millis(),
                p99_ms = p99.as_millis(),
                "performance percentiles"
            );
        }
    }
}
```

---

## Production Debugging

### Debug Endpoints

```rust
use axum::{Router, routing::get};

// Debug router (only enable in development or with auth)
fn debug_routes() -> Router {
    Router::new()
        .route("/debug/cache", get(cache_debug))
        .route("/debug/forks", get(forks_debug))
        .route("/debug/metrics", get(metrics_debug))
        .route("/debug/pprof/heap", get(heap_profile))
        .route("/debug/pprof/profile", get(cpu_profile))
}

async fn cache_debug(state: Arc<AppState>) -> Json<CacheDebugInfo> {
    let stats = state.cache_stats();
    let cache = state.cache.read();

    Json(CacheDebugInfo {
        stats,
        entries: cache.iter()
            .map(|(id, wb)| (id.clone(), wb.summary()))
            .collect(),
    })
}

async fn forks_debug(state: Arc<AppState>) -> Json<ForksDebugInfo> {
    #[cfg(feature = "recalc")]
    if let Some(registry) = state.fork_registry() {
        return Json(ForksDebugInfo {
            active_forks: registry.list_forks(),
            checkpoint_count: registry.checkpoint_count(),
            disk_usage_bytes: registry.disk_usage(),
        });
    }

    Json(ForksDebugInfo::default())
}
```

### Live Log Level Adjustment

```rust
use tracing_subscriber::reload;

// Create reloadable subscriber
let (filter, reload_handle) = reload::Layer::new(EnvFilter::new("info"));
let subscriber = Registry::default()
    .with(filter)
    .with(tracing_subscriber::fmt::layer());

// Store reload handle in app state
struct AppState {
    // ...
    log_level_handle: reload::Handle<EnvFilter, Registry>,
}

// Endpoint to change log level at runtime
#[post("/debug/log_level")]
async fn set_log_level(
    state: Arc<AppState>,
    level: String,
) -> Result<()> {
    let new_filter = EnvFilter::new(level);
    state.log_level_handle.reload(new_filter)?;

    info!(new_level = level, "log level updated");
    Ok(())
}
```

### Request Replay for Debugging

```rust
// Capture requests for replay
#[derive(Serialize, Deserialize)]
struct CapturedRequest {
    timestamp: DateTime<Utc>,
    request_id: String,
    method: String,
    params: serde_json::Value,
    response: serde_json::Value,
    error: Option<String>,
}

// Save requests when errors occur
fn capture_on_error(request: &McpRequest, error: &Error) {
    let captured = CapturedRequest {
        timestamp: Utc::now(),
        request_id: request.id.clone(),
        method: request.method.clone(),
        params: request.params.clone(),
        response: serde_json::Value::Null,
        error: Some(error.to_string()),
    };

    let path = format!("/tmp/mcp-captures/{}.json", request.id);
    if let Err(e) = std::fs::write(&path, serde_json::to_string_pretty(&captured).unwrap()) {
        warn!(error = %e, "failed to capture request");
    } else {
        info!(path = %path, "request captured for replay");
    }
}
```

### Stack Trace Capture

```rust
use backtrace::Backtrace;

// Capture stack traces on errors
fn handle_error(err: &Error) {
    let bt = Backtrace::new();

    error!(
        error = %err,
        backtrace = %bt,
        "unhandled error with stack trace"
    );
}

// For production, use Sentry for aggregated stack traces
use sentry::capture_error;

fn handle_error_production(err: &Error) {
    // Send to Sentry for aggregation and alerting
    sentry::capture_error(err);

    error!(error = %err, "error reported to sentry");
}
```

---

## User Behavior Analytics

### Workflow Pattern Detection

```rust
use std::collections::VecDeque;

#[derive(Debug, Clone)]
struct UserSession {
    session_id: String,
    started_at: DateTime<Utc>,
    last_activity: DateTime<Utc>,
    tool_sequence: VecDeque<ToolCall>,
}

#[derive(Debug, Clone)]
struct ToolCall {
    timestamp: DateTime<Utc>,
    tool_name: String,
    workbook_id: Option<String>,
    success: bool,
    duration_ms: u64,
}

impl UserSession {
    pub fn detect_pattern(&self) -> Option<WorkflowPattern> {
        // Detect common patterns
        let tools: Vec<&str> = self.tool_sequence
            .iter()
            .map(|t| t.tool_name.as_str())
            .collect();

        match tools.as_slice() {
            ["list_workbooks", "sheet_overview", "read_table", ..] => {
                Some(WorkflowPattern::DataExtraction)
            }
            ["create_fork", "edit_batch", "recalculate", "get_changeset", "save_fork"] => {
                Some(WorkflowPattern::WhatIfAnalysis)
            }
            ["find_value", "range_values", ..] => {
                Some(WorkflowPattern::SpotCheck)
            }
            _ => None
        }
    }
}

#[derive(Debug)]
enum WorkflowPattern {
    DataExtraction,
    WhatIfAnalysis,
    SpotCheck,
    FormulaAnalysis,
    StyleInspection,
}
```

### Feature Adoption Tracking

```rust
struct FeatureAdoptionMetrics {
    total_users: u64,
    users_by_feature: HashMap<String, HashSet<String>>,
}

impl FeatureAdoptionMetrics {
    pub fn adoption_rate(&self, feature: &str) -> f64 {
        let users = self.users_by_feature
            .get(feature)
            .map(|set| set.len())
            .unwrap_or(0);

        users as f64 / self.total_users as f64
    }

    pub fn report(&self) -> Vec<FeatureReport> {
        self.users_by_feature
            .iter()
            .map(|(feature, users)| {
                FeatureReport {
                    feature: feature.clone(),
                    user_count: users.len(),
                    adoption_rate: self.adoption_rate(feature),
                }
            })
            .collect()
    }
}

// Track feature usage
fn track_feature_usage(user_id: &str, feature: &str) {
    FEATURE_METRICS
        .users_by_feature
        .entry(feature.to_string())
        .or_insert_with(HashSet::new)
        .insert(user_id.to_string());
}
```

### Error Path Analysis

```rust
// Track which operations commonly fail together
struct ErrorCorrelation {
    error_sequences: Vec<Vec<String>>,
}

impl ErrorCorrelation {
    pub fn find_correlations(&self) -> Vec<(String, String, f64)> {
        // Find pairs of errors that occur together frequently
        let mut correlations = Vec::new();

        for seq in &self.error_sequences {
            for window in seq.windows(2) {
                let (err1, err2) = (&window[0], &window[1]);
                let correlation = self.calculate_correlation(err1, err2);

                if correlation > 0.7 {
                    correlations.push((
                        err1.clone(),
                        err2.clone(),
                        correlation
                    ));
                }
            }
        }

        correlations
    }
}
```

---

## Data-Driven Decision Making

### A/B Testing Infrastructure

```rust
use rand::Rng;

#[derive(Clone, Copy)]
enum Variant {
    Control,
    Treatment,
}

struct ABTest {
    name: String,
    allocation_rate: f64,  // % users in treatment
    metric_tracker: Arc<MetricTracker>,
}

impl ABTest {
    pub fn assign_variant(&self, user_id: &str) -> Variant {
        // Consistent hashing for stable assignment
        let hash = hash_user_id(user_id);
        let ratio = (hash % 100) as f64 / 100.0;

        if ratio < self.allocation_rate {
            Variant::Treatment
        } else {
            Variant::Control
        }
    }

    pub fn report(&self) -> ABTestReport {
        let control_metrics = self.metric_tracker.get_metrics(Variant::Control);
        let treatment_metrics = self.metric_tracker.get_metrics(Variant::Treatment);

        ABTestReport {
            test_name: self.name.clone(),
            control: control_metrics,
            treatment: treatment_metrics,
            statistical_significance: self.calculate_significance(
                &control_metrics,
                &treatment_metrics
            ),
        }
    }
}

// Example: Test new region detection algorithm
async fn sheet_overview_with_test(
    user_id: &str,
    sheet: &Worksheet,
) -> Result<SheetOverviewResponse> {
    let test = AB_TESTS.get("region_detection_v2").unwrap();
    let variant = test.assign_variant(user_id);

    let start = Instant::now();

    let regions = match variant {
        Variant::Control => detect_regions_v1(sheet)?,
        Variant::Treatment => detect_regions_v2(sheet)?,
    };

    let duration = start.elapsed();

    // Track metrics
    test.metric_tracker.record(variant, ABMetric {
        duration,
        region_count: regions.len(),
        success: true,
    });

    Ok(SheetOverviewResponse { regions })
}
```

### Performance Optimization Tracking

```rust
struct OptimizationExperiment {
    baseline_p95_ms: u64,
    current_p95_ms: u64,
    target_p95_ms: u64,
}

impl OptimizationExperiment {
    pub fn improvement(&self) -> f64 {
        let delta = self.baseline_p95_ms as f64 - self.current_p95_ms as f64;
        delta / self.baseline_p95_ms as f64
    }

    pub fn on_track(&self) -> bool {
        self.current_p95_ms <= self.target_p95_ms
    }

    pub fn report(&self) -> String {
        format!(
            "Optimization: {:.1}% improvement (baseline: {}ms, current: {}ms, target: {}ms)",
            self.improvement() * 100.0,
            self.baseline_p95_ms,
            self.current_p95_ms,
            self.target_p95_ms
        )
    }
}
```

### SLO Tracking

```rust
#[derive(Debug)]
struct ServiceLevelObjective {
    name: String,
    target: f64,  // e.g., 0.999 for 99.9%
    window: Duration,  // e.g., 30 days
}

impl ServiceLevelObjective {
    // Error budget: allowable failure rate
    pub fn error_budget(&self) -> f64 {
        1.0 - self.target
    }

    // How much error budget is remaining
    pub fn budget_remaining(&self, current_success_rate: f64) -> f64 {
        let actual_error_rate = 1.0 - current_success_rate;
        let allowed_error_rate = self.error_budget();

        (allowed_error_rate - actual_error_rate) / allowed_error_rate
    }

    pub fn burn_rate_alert(&self, recent_success_rate: f64) -> bool {
        // If we're burning through error budget too fast
        let budget_used = 1.0 - self.budget_remaining(recent_success_rate);

        // Alert if we're using budget 10x faster than expected
        budget_used > 10.0 * (1.0 / 30.0)  // 30-day window
    }
}

// Example SLOs
const SLOS: &[ServiceLevelObjective] = &[
    ServiceLevelObjective {
        name: "Tool Success Rate",
        target: 0.999,  // 99.9%
        window: Duration::from_secs(30 * 24 * 60 * 60),  // 30 days
    },
    ServiceLevelObjective {
        name: "P95 Latency",
        target: 0.95,  // 95% under threshold
        window: Duration::from_secs(7 * 24 * 60 * 60),  // 7 days
    },
];
```

---

## Implementation Roadmap

### Phase 1: Foundation (Week 1-2)

#### Metrics Infrastructure

**Priority**: High
**Effort**: Medium

1. Add `prometheus` and `opentelemetry` dependencies
2. Create metrics registry
3. Implement basic counters/histograms
4. Add `/metrics` endpoint
5. Set up Prometheus scraping

**Deliverables**:
- [ ] Prometheus integration
- [ ] Basic metrics (request count, duration, errors)
- [ ] Grafana dashboard skeleton

#### Enhanced Structured Logging

**Priority**: High
**Effort**: Low

1. Add request ID generation
2. Implement hierarchical spans for all tools
3. Add JSON log formatter
4. Configure log rotation

**Deliverables**:
- [ ] Request ID in all logs
- [ ] JSON structured logs
- [ ] Log aggregation ready

### Phase 2: Observability (Week 3-4)

#### Distributed Tracing

**Priority**: Medium
**Effort**: Medium

1. Add OpenTelemetry tracing layer
2. Instrument all tool handlers
3. Add trace context propagation
4. Set up Jaeger/Tempo

**Deliverables**:
- [ ] End-to-end trace visualization
- [ ] Jaeger UI integrated
- [ ] Sampling configured

#### Real-Time Dashboards

**Priority**: High
**Effort**: High

1. Create Grafana dashboards
2. Set up alerting rules
3. Configure notification channels
4. Add SLO tracking

**Deliverables**:
- [ ] Operations dashboard
- [ ] Tool usage dashboard
- [ ] Error tracking dashboard
- [ ] Alert rules configured

### Phase 3: Advanced Analytics (Week 5-6)

#### User Behavior Tracking

**Priority**: Low
**Effort**: Medium

1. Implement session tracking
2. Add workflow pattern detection
3. Create feature adoption metrics
4. Build analytics dashboard

**Deliverables**:
- [ ] Session analytics
- [ ] Workflow pattern reports
- [ ] Feature adoption dashboard

#### Performance Profiling

**Priority**: Medium
**Effort**: Medium

1. Add pprof endpoints
2. Implement continuous profiling
3. Set up flame graph generation
4. Add async task monitoring

**Deliverables**:
- [ ] CPU profiling endpoint
- [ ] Memory profiling endpoint
- [ ] Tokio runtime metrics

### Phase 4: Production Readiness (Week 7-8)

#### Health Checks

**Priority**: High
**Effort**: Low

1. Implement `/health` endpoint
2. Add `/ready` endpoint
3. Include dependency checks
4. Add graceful shutdown

**Deliverables**:
- [ ] Health check endpoints
- [ ] Kubernetes-ready probes
- [ ] Dependency health monitoring

#### Debugging Tools

**Priority**: Low
**Effort**: Low

1. Add debug endpoints
2. Implement request capture
3. Add live log level adjustment
4. Create debugging runbook

**Deliverables**:
- [ ] Debug endpoints
- [ ] Request replay capability
- [ ] Production debugging guide

---

## References

### Books and Papers

1. **"Implementing Lean Software Development"** - Mary and Tom Poppendieck
2. **"The Toyota Way"** - Jeffrey Liker
3. **"Observability Engineering"** - Charity Majors, Liz Fong-Jones, George Miranda
4. **"Site Reliability Engineering"** - Google SRE Team
5. **"Distributed Tracing in Practice"** - Austin Parker et al.

### Tools and Technologies

#### Metrics and Monitoring
- [Prometheus](https://prometheus.io/) - Metrics collection and storage
- [Grafana](https://grafana.com/) - Visualization and dashboards
- [VictoriaMetrics](https://victoriametrics.com/) - Long-term metrics storage

#### Distributed Tracing
- [Jaeger](https://www.jaegertracing.io/) - Distributed tracing platform
- [Tempo](https://grafana.com/oss/tempo/) - Grafana's tracing backend
- [OpenTelemetry](https://opentelemetry.io/) - Observability framework

#### Logging
- [Loki](https://grafana.com/oss/loki/) - Log aggregation
- [Vector](https://vector.dev/) - Log routing and transformation
- [Elasticsearch](https://www.elastic.co/) - Log search and analytics

#### Error Tracking
- [Sentry](https://sentry.io/) - Error tracking and monitoring
- [Rollbar](https://rollbar.com/) - Error monitoring

#### Profiling
- [pprof](https://github.com/google/pprof) - Performance profiling
- [async-profiler](https://github.com/jvm-profiling-tools/async-profiler) - Low-overhead profiling

### Rust Crates

```toml
[dependencies]
# Metrics
prometheus = "0.13"
opentelemetry = "0.21"
opentelemetry-prometheus = "0.14"

# Tracing
tracing = "0.1"
tracing-subscriber = "0.3"
tracing-opentelemetry = "0.22"
opentelemetry-otlp = "0.14"

# Profiling
pprof = "0.13"
jemalloc-ctl = "0.5"
tokio-metrics = "0.3"

# Error tracking
sentry = "0.32"
sentry-tracing = "0.32"
```

### Key Metrics for MCP Servers

#### Golden Signals

1. **Latency**: Request duration distribution
2. **Traffic**: Requests per second
3. **Errors**: Error rate and types
4. **Saturation**: Resource utilization

#### MCP-Specific Metrics

1. **Tool Usage**: Invocation count by tool
2. **Cache Performance**: Hit rate, eviction rate
3. **Fork Lifecycle**: Creation, duration, save rate
4. **Workbook Operations**: Load time, parse errors
5. **Region Detection**: Success rate, fallback rate

---

## Conclusion

Applying Gemba principles to MCP servers means **going to where the work happens** - production - and observing actual system behavior. This requires comprehensive observability infrastructure:

1. **Structured logging** for understanding what happened
2. **Distributed tracing** for understanding how it happened
3. **Metrics and dashboards** for understanding patterns
4. **User analytics** for understanding why
5. **Performance profiling** for understanding where to optimize

The current ggen-mcp implementation has a solid foundation with:
- Comprehensive audit trail system
- Structured tracing with spans
- Recovery and resilience monitoring

The next steps are to add:
- Metrics aggregation (Prometheus)
- Distributed tracing (OpenTelemetry)
- Real-time dashboards (Grafana)
- Production debugging tools
- User behavior analytics

By implementing these observability features, we enable true Gemba practice: **data-driven decision making based on actual observations of the system in production**.

---

**Document Status**: ✅ Research Complete
**Next Steps**: Prioritize and implement based on roadmap
**Maintained By**: Platform Engineering Team
**Last Review**: 2026-01-20
