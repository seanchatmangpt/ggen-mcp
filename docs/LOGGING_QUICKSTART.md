# Structured Logging Quick Start Guide

This guide will get you up and running with structured logging in ggen-mcp in 5 minutes.

## Quick Start

### 1. Development Setup (Pretty Logs)

For local development with human-readable logs:

```bash
# Set environment variables
export ENVIRONMENT=development
export LOG_FORMAT=pretty
export LOG_OUTPUT=stderr
export RUST_LOG=debug

# Run the server
cargo run
```

You'll see colorized, pretty-printed logs:

```
2026-01-20T12:34:56.789Z  INFO spreadsheet_mcp: structured logging and tracing initialized
    at src/tracing_setup.rs:336
    service="spreadsheet-mcp" version="0.9.0" environment="development"
```

### 2. Production Setup (JSON Logs)

For production with JSON structured logs:

```bash
# Set environment variables
export ENVIRONMENT=production
export LOG_FORMAT=json
export LOG_OUTPUT=file
export LOG_DIR=./logs
export RUST_LOG=info

# Run the server
cargo run
```

Logs will be written to `./logs/ggen-mcp.YYYY-MM-DD` with daily rotation.

Example log entry:
```json
{
  "timestamp": "2026-01-20T12:34:56.789Z",
  "level": "INFO",
  "message": "Fork created successfully",
  "target": "spreadsheet_mcp::fork",
  "fields": {
    "service": "spreadsheet-mcp",
    "version": "0.9.0",
    "mcp.fork_id": "fork_456",
    "mcp.workbook_id": "wb_123",
    "duration_ms": 145
  }
}
```

### 3. With OpenTelemetry (Distributed Tracing)

To enable distributed tracing:

```bash
# Start observability stack
docker-compose -f docker-compose.observability.yml up -d

# Configure tracing
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export OTEL_SAMPLING_RATE=0.1

# Run the server
cargo run
```

Access dashboards:
- **Grafana**: http://localhost:3000 (admin/admin)
- **Jaeger UI**: http://localhost:16686
- **Prometheus**: http://localhost:9090

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `LOG_FORMAT` | Auto | `json` or `pretty` |
| `LOG_OUTPUT` | `stderr` | `stdout`, `stderr`, or `file` |
| `LOG_DIR` | `logs` | Directory for log files |
| `RUST_LOG` | Auto | Log level filter |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | None | OpenTelemetry endpoint |

See [.env.example](../.env.example) for all options.

## Common Use Cases

### Filtering Logs

```bash
# Show only errors
RUST_LOG=error cargo run

# Debug for ggen-mcp, info for dependencies
RUST_LOG=spreadsheet_mcp=debug,info cargo run

# Trace everything (very verbose!)
RUST_LOG=trace cargo run
```

### Query Logs in Loki

```bash
# View all logs
{service="ggen-mcp"}

# Filter by level
{service="ggen-mcp"} | json | level="ERROR"

# Find slow operations
{service="ggen-mcp"} | json | duration_ms > 1000

# Tool-specific logs
{service="ggen-mcp"} | json | mcp_tool="fork_create"
```

### Adding Structured Logging to Your Code

```rust
use tracing::{info, debug, error};

// Simple log
info!("Processing workbook");

// With structured fields
info!(
    mcp.workbook_id = %workbook_id,
    duration_ms = duration.as_millis(),
    "Workbook processed successfully"
);

// Using spans for context
let span = tracing::info_span!(
    "workbook_operation",
    mcp.workbook_id = %workbook_id
);
let _enter = span.enter();

debug!("Loading workbook");
// All logs here automatically include workbook_id
```

### Helper Macros

```rust
use spreadsheet_mcp::{log_slow_operation, log_cache_operation, log_mcp_tool};

// Log slow operations
let start = std::time::Instant::now();
// ... operation ...
log_slow_operation!(
    start.elapsed(),
    1000, // 1s threshold
    mcp.operation = "load",
    "Operation completed"
);

// Log cache operations
if let Some(value) = cache.get(&key) {
    log_cache_operation!(hit, &key, "Cache hit");
} else {
    log_cache_operation!(miss, &key, "Cache miss");
}

// Log MCP tool invocations
log_mcp_tool!(
    "fork_create",
    "success",
    duration,
    mcp.fork_id = %fork_id
);
```

## Troubleshooting

### Logs not appearing?

1. Check `RUST_LOG` is set (default is `info`)
2. Ensure logging is initialized in `main.rs`
3. Verify log file permissions if using `LOG_OUTPUT=file`

### Too many logs?

```bash
# Reduce log level
RUST_LOG=warn cargo run

# Or module-specific
RUST_LOG=spreadsheet_mcp=info,hyper=warn cargo run
```

### OpenTelemetry not working?

1. Verify observability stack is running:
   ```bash
   docker-compose -f docker-compose.observability.yml ps
   ```

2. Check OTLP endpoint is reachable:
   ```bash
   curl http://localhost:4317
   ```

3. Review logs for OTLP export errors

## Next Steps

- Read the [full documentation](STRUCTURED_LOGGING.md)
- Explore [Grafana dashboards](http://localhost:3000)
- Set up [log alerts](STRUCTURED_LOGGING.md#alerting)
- Configure [log retention](STRUCTURED_LOGGING.md#retention)

## Example: Complete Setup

```bash
# 1. Copy environment template
cp .env.example .env

# 2. Edit .env for your environment
nano .env

# 3. Start observability stack
docker-compose -f docker-compose.observability.yml up -d

# 4. Run the server
cargo run

# 5. Access Grafana
open http://localhost:3000

# 6. View logs
# Navigate to Explore > Loki > {service="ggen-mcp"}
```

That's it! You now have structured logging with distributed tracing and log aggregation. 🎉
