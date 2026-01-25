# Structured Logging and Observability

This document describes the structured logging implementation in ggen-mcp for production observability.

## Table of Contents

- [Overview](#overview)
- [Configuration](#configuration)
- [Log Formats](#log-formats)
- [Structured Fields](#structured-fields)
- [Log Levels Strategy](#log-levels-strategy)
- [OpenTelemetry Integration](#opentelemetry-integration)
- [Helper Macros](#helper-macros)
- [Log Aggregation](#log-aggregation)
- [Best Practices](#best-practices)
- [Performance Impact](#performance-impact)

## Overview

ggen-mcp uses structured logging with:

- **JSON logging** for production environments (machine-readable)
- **Pretty logging** for development (human-readable)
- **File output** with daily rotation
- **OpenTelemetry integration** for distributed tracing
- **Contextual fields** for rich log data
- **Span-based context** propagation

## Configuration

### Environment Variables

| Variable | Values | Default | Description |
|----------|--------|---------|-------------|
| `LOG_FORMAT` | `json`, `pretty` | Auto-detect based on `ENVIRONMENT` | Log output format |
| `LOG_OUTPUT` | `stdout`, `stderr`, `file` | `stderr` | Log output destination |
| `LOG_DIR` | Path | `logs` | Directory for log files (when `LOG_OUTPUT=file`) |
| `ENVIRONMENT` | `production`, `development`, `staging` | `development` | Environment name |
| `RUST_LOG` | Log filter string | `info` (prod), `debug` (dev) | Log level filter |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | URL | None | OpenTelemetry OTLP endpoint |
| `OTEL_SAMPLING_RATE` | 0.0-1.0 | 0.1 (prod), 1.0 (dev) | Trace sampling rate |
| `OTEL_SERVICE_NAME` | String | `ggen-mcp` | Service name for traces |

### Example .env Configuration

#### Development
```bash
# Development environment with pretty logs to stderr
ENVIRONMENT=development
LOG_FORMAT=pretty
LOG_OUTPUT=stderr
RUST_LOG=debug,hyper=info,tower=info
```

#### Production
```bash
# Production environment with JSON logs to file and OpenTelemetry
ENVIRONMENT=production
LOG_FORMAT=json
LOG_OUTPUT=file
LOG_DIR=/var/log/ggen-mcp
RUST_LOG=info,spreadsheet_mcp=debug

# OpenTelemetry configuration
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
OTEL_SAMPLING_RATE=0.1
OTEL_SERVICE_NAME=ggen-mcp
```

## Log Formats

### JSON Format (Production)

Structured JSON logs suitable for log aggregation systems like Loki, Elasticsearch, or Datadog.

Example:
```json
{
  "timestamp": "2026-01-20T12:34:56.789Z",
  "level": "INFO",
  "message": "Tool invocation completed",
  "target": "spreadsheet_mcp::tools::fork",
  "span": {
    "name": "mcp_tool",
    "mcp.tool": "fork_create"
  },
  "fields": {
    "service": "spreadsheet-mcp",
    "version": "1.0.0",
    "mcp.workbook_id": "wb_123",
    "mcp.fork_id": "fork_456",
    "duration_ms": 145,
    "result": "success"
  }
}
```

### Pretty Format (Development)

Human-readable output with ANSI colors for local development.

Example:
```
2026-01-20T12:34:56.789Z  INFO spreadsheet_mcp::tools::fork: Tool invocation completed
    at src/tools/fork.rs:45
    in mcp_tool with mcp.tool=fork_create, service=spreadsheet-mcp, version=1.0.0
    mcp.workbook_id="wb_123" mcp.fork_id="fork_456" duration_ms=145 result="success"
```

## Structured Fields

### Standard Fields

All log entries include:

- `timestamp` - ISO 8601 timestamp
- `level` - Log level (ERROR, WARN, INFO, DEBUG, TRACE)
- `message` - Log message
- `target` - Module path where log originated
- `file` - Source file (in JSON mode)
- `line` - Line number (in JSON mode)

### MCP-Specific Fields

| Field | Description | Example |
|-------|-------------|---------|
| `service` | Service name | `spreadsheet-mcp` |
| `version` | Service version | `1.0.0` |
| `mcp.tool` | MCP tool name | `fork_create` |
| `mcp.workbook_id` | Workbook identifier | `wb_123` |
| `mcp.fork_id` | Fork identifier | `fork_456` |
| `mcp.operation` | Operation type | `read`, `write`, `merge` |
| `mcp.sheet_name` | Sheet name | `Sheet1` |
| `mcp.range` | Cell range | `A1:Z100` |
| `mcp.result` | Operation result | `success`, `error` |
| `mcp.cache_hit` | Cache hit/miss | `true`, `false` |

### Performance Fields

| Field | Description | Example |
|-------|-------------|---------|
| `duration_ms` | Operation duration in milliseconds | `145` |
| `performance.slow` | Indicates slow operation | `true` |
| `threshold_ms` | Slowness threshold | `1000` |

### Error Fields

| Field | Description | Example |
|-------|-------------|---------|
| `error.type` | Error type/code | `FileNotFound` |
| `error.message` | Error message | `Workbook not found: wb_123` |
| `error` | Error flag | `true` |

### Security Fields

| Field | Description | Example |
|-------|-------------|---------|
| `security.event_type` | Security event type | `path_traversal_attempt` |
| `security.alert` | Security alert flag | `true` |

### Cache Fields

| Field | Description | Example |
|-------|-------------|---------|
| `cache.key` | Cache key | `workbook_wb_123` |
| `cache.result` | Cache result | `hit`, `miss` |

## Log Levels Strategy

### ERROR

Actual errors requiring immediate attention. These indicate failures that prevent operations from completing successfully.

```rust
tracing::error!(
    error.type = "WorkbookLoadError",
    error.message = %error,
    mcp.workbook_id = %workbook_id,
    "Failed to load workbook"
);
```

### WARN

Degraded performance, fallbacks, or retry attempts. Indicates issues that don't prevent operation but should be investigated.

```rust
tracing::warn!(
    duration_ms = duration.as_millis(),
    threshold_ms = 1000,
    "Slow workbook operation detected"
);
```

### INFO

Significant state changes, request/response events, and operational milestones.

```rust
tracing::info!(
    mcp.tool = "fork_create",
    mcp.fork_id = %fork_id,
    duration_ms = duration.as_millis(),
    "Fork created successfully"
);
```

### DEBUG

Detailed operation flow for debugging purposes.

```rust
tracing::debug!(
    cache.key = %cache_key,
    cache.result = "hit",
    "Cache lookup successful"
);
```

### TRACE

Very detailed logging including function entry/exit (use sparingly).

```rust
tracing::trace!("Entering parse_formula");
```

## OpenTelemetry Integration

### Distributed Tracing

When configured with an OTLP endpoint, ggen-mcp sends distributed traces to OpenTelemetry collectors.

### Trace Hierarchy

```
mcp_tool (span)
├─ service=spreadsheet-mcp
├─ version=1.0.0
├─ mcp.tool=fork_create
└─ workbook_operation (child span)
   ├─ mcp.workbook_id=wb_123
   └─ mcp.operation=read
```

### Sampling

- **Production**: 10% sampling (configurable via `OTEL_SAMPLING_RATE`)
- **Development**: 100% sampling
- **Parent-based**: Child spans follow parent sampling decision

### OpenTelemetry Attributes

Use the `attributes` module for standard attribute creation:

```rust
use spreadsheet_mcp::tracing_setup::attributes;

let span = tracing::info_span!(
    "workbook_operation",
    { attributes::mcp_tool("fork_create") },
    { attributes::workbook_id(&workbook_id) },
);
```

## Helper Macros

### log_slow_operation!

Log operations that exceed a threshold:

```rust
use spreadsheet_mcp::log_slow_operation;

let start = std::time::Instant::now();
// ... operation ...
let duration = start.elapsed();

log_slow_operation!(
    duration,
    1000, // 1 second threshold
    mcp.operation = "workbook_load",
    mcp.workbook_id = %workbook_id,
    "Workbook operation completed"
);
```

### log_cache_operation!

Log cache hits and misses:

```rust
use spreadsheet_mcp::log_cache_operation;

if let Some(value) = cache.get(&key) {
    log_cache_operation!(hit, &key, "Retrieved from cache");
} else {
    log_cache_operation!(miss, &key, "Cache miss, loading from disk");
}
```

### log_mcp_tool!

Log MCP tool invocations:

```rust
use spreadsheet_mcp::log_mcp_tool;

let start = std::time::Instant::now();
let result = execute_tool();
let duration = start.elapsed();

log_mcp_tool!(
    "fork_create",
    "success",
    duration,
    mcp.fork_id = %fork_id
);
```

### log_security_event!

Log security-related events:

```rust
use spreadsheet_mcp::log_security_event;

log_security_event!(
    "path_traversal_attempt",
    path = %suspicious_path,
    "Blocked path traversal attempt"
);
```

## Log Aggregation

### Grafana Loki

#### Setup with Docker Compose

See [docker-compose.observability.yml](../docker-compose.observability.yml)

#### LogQL Query Examples

**All errors in the last hour:**
```logql
{service="ggen-mcp"} | json | level="ERROR" | line_format "{{.timestamp}} {{.message}}"
```

**Slow operations (>1s):**
```logql
{service="ggen-mcp"} | json | duration_ms > 1000 | line_format "{{.mcp_tool}} took {{.duration_ms}}ms"
```

**Tool invocations by result:**
```logql
sum by (mcp_tool, mcp_result) (count_over_time({service="ggen-mcp"} | json | mcp_tool != "" [5m]))
```

**Cache hit rate:**
```logql
sum by (cache_result) (count_over_time({service="ggen-mcp"} | json | cache_result != "" [1h]))
```

**Security events:**
```logql
{service="ggen-mcp"} | json | security_alert="true" | line_format "{{.security_event_type}}: {{.message}}"
```

### Elasticsearch

#### Index Template

```json
{
  "index_patterns": ["ggen-mcp-*"],
  "template": {
    "mappings": {
      "properties": {
        "timestamp": { "type": "date" },
        "level": { "type": "keyword" },
        "message": { "type": "text" },
        "target": { "type": "keyword" },
        "service": { "type": "keyword" },
        "version": { "type": "keyword" },
        "mcp.tool": { "type": "keyword" },
        "mcp.workbook_id": { "type": "keyword" },
        "mcp.fork_id": { "type": "keyword" },
        "duration_ms": { "type": "long" },
        "error.type": { "type": "keyword" },
        "error.message": { "type": "text" }
      }
    }
  }
}
```

#### Query Examples

**Errors in the last hour:**
```json
{
  "query": {
    "bool": {
      "must": [
        { "term": { "level": "ERROR" } },
        { "range": { "timestamp": { "gte": "now-1h" } } }
      ]
    }
  }
}
```

**Average operation duration by tool:**
```json
{
  "size": 0,
  "aggs": {
    "by_tool": {
      "terms": { "field": "mcp.tool" },
      "aggs": {
        "avg_duration": { "avg": { "field": "duration_ms" } }
      }
    }
  }
}
```

## Best Practices

### 1. Use Structured Fields

**Bad:**
```rust
tracing::info!("Fork fork_456 created for workbook wb_123 in 145ms");
```

**Good:**
```rust
tracing::info!(
    mcp.fork_id = "fork_456",
    mcp.workbook_id = "wb_123",
    duration_ms = 145,
    "Fork created successfully"
);
```

### 2. Use Spans for Context

**Bad:**
```rust
tracing::info!(mcp.workbook_id = %id, "Loading workbook");
// ... operations ...
tracing::info!(mcp.workbook_id = %id, "Workbook loaded");
```

**Good:**
```rust
let span = workbook_span(&workbook_id);
let _enter = span.enter();

tracing::info!("Loading workbook");
// ... operations ...
tracing::info!("Workbook loaded");
```

### 3. Log at Appropriate Levels

- Use INFO for user-facing operations
- Use DEBUG for internal flow
- Use WARN for recoverable issues
- Use ERROR only for actual errors

### 4. Include Error Context

```rust
if let Err(error) = operation() {
    tracing::error!(
        error.type = std::any::type_name_of_val(&error),
        error.message = %error,
        mcp.workbook_id = %workbook_id,
        "Operation failed"
    );
}
```

### 5. Don't Log Sensitive Data

**Never log:**
- User credentials
- API keys
- Personal information
- Full file contents

### 6. Use Instrument Attribute

```rust
#[tracing::instrument(
    skip(self),
    fields(
        mcp.workbook_id = %workbook_id,
        mcp.operation = "fork_create"
    )
)]
async fn create_fork(&self, workbook_id: &str) -> Result<Fork> {
    // Function body automatically gets span
}
```

## Performance Impact

### Overhead

| Operation | Overhead | Notes |
|-----------|----------|-------|
| JSON logging | ~5-10µs per log | Negligible for INFO and above |
| Pretty logging | ~10-20µs per log | Only use in development |
| Span creation | ~1µs | Very low cost |
| OpenTelemetry export | ~100µs | Batched, minimal impact |

### Optimization Tips

1. **Use appropriate log levels**: DEBUG and TRACE in production add overhead
2. **Leverage sampling**: 10% sampling reduces OTLP traffic by 90%
3. **Avoid logging in hot paths**: Cache lookups, tight loops
4. **Use lazy evaluation**: `%` and `?` for expensive formatting
5. **File output**: Faster than stdout/stderr in production

### Monitoring Logging Performance

```rust
// Log logger statistics
tracing::info!(
    logs.emitted = counter,
    logs.dropped = dropped,
    "Logging statistics"
);
```

## Troubleshooting

### Logs Not Appearing

1. Check `RUST_LOG` environment variable
2. Verify logging initialization in `main.rs`
3. Check file permissions if using file output
4. Ensure `_guard` is held for application lifetime

### OpenTelemetry Not Working

1. Verify `OTEL_EXPORTER_OTLP_ENDPOINT` is set
2. Check collector is running and reachable
3. Review firewall rules for port 4317
4. Check logs for OTLP export errors

### Performance Issues

1. Reduce log level in production (INFO minimum)
2. Disable DEBUG/TRACE in hot paths
3. Increase sampling rate if too many traces
4. Use file output instead of stdout

## JSON Schema

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["timestamp", "level", "message", "target"],
  "properties": {
    "timestamp": {
      "type": "string",
      "format": "date-time",
      "description": "ISO 8601 timestamp"
    },
    "level": {
      "type": "string",
      "enum": ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"]
    },
    "message": {
      "type": "string"
    },
    "target": {
      "type": "string",
      "description": "Module path"
    },
    "span": {
      "type": "object",
      "properties": {
        "name": { "type": "string" },
        "mcp.tool": { "type": "string" },
        "mcp.workbook_id": { "type": "string" },
        "mcp.fork_id": { "type": "string" }
      }
    },
    "fields": {
      "type": "object",
      "properties": {
        "service": { "type": "string" },
        "version": { "type": "string" },
        "duration_ms": { "type": "number" },
        "error.type": { "type": "string" },
        "error.message": { "type": "string" },
        "mcp.result": { "type": "string" },
        "cache.result": { "type": "string" }
      }
    }
  }
}
```

## References

- [tracing](https://docs.rs/tracing/latest/tracing/) - Rust tracing library
- [tracing-subscriber](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/) - Log formatting
- [OpenTelemetry](https://opentelemetry.io/) - Distributed tracing
- [Grafana Loki](https://grafana.com/oss/loki/) - Log aggregation
- [LogQL](https://grafana.com/docs/loki/latest/logql/) - Loki query language
