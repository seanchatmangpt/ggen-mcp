# Prometheus Metrics for Spreadsheet MCP Server

## Overview

The Spreadsheet MCP server exposes comprehensive Prometheus metrics for production observability. All metrics are available at the `/metrics` endpoint when running in HTTP transport mode.

## Metrics Endpoint

**URL**: `http://localhost:8079/metrics` (default)

**Format**: Prometheus text format

**Method**: GET

**Response**: Plain text metrics in Prometheus exposition format

## Complete Metrics Catalog

### Request Metrics

#### `mcp_requests_total`

**Type**: Counter

**Description**: Total number of MCP tool requests

**Labels**:
- `tool` - Tool name (e.g., "list_workbooks", "read_table", "recalculate")
- `status` - Request status ("success", "error", "timeout")

**Example**:
```promql
# Rate of successful requests per second
rate(mcp_requests_total{status="success"}[5m])

# Error rate by tool
rate(mcp_requests_total{status="error"}[5m])

# P99 error rate
histogram_quantile(0.99, rate(mcp_requests_total{status="error"}[5m]))
```

#### `mcp_request_duration_seconds`

**Type**: Histogram

**Description**: Request latency distribution in seconds

**Labels**:
- `tool` - Tool name

**Buckets**: Exponential buckets from 10ms to 30s (0.01, 0.025, 0.0625, 0.156, 0.390, 0.977, 2.44, 6.10, 15.26, 38.14)

**Example**:
```promql
# P95 latency for read_table requests
histogram_quantile(0.95, rate(mcp_request_duration_seconds_bucket{tool="read_table"}[5m]))

# Average request duration
rate(mcp_request_duration_seconds_sum[5m]) / rate(mcp_request_duration_seconds_count[5m])

# Slowest tools (average duration)
topk(5, rate(mcp_request_duration_seconds_sum[5m]) / rate(mcp_request_duration_seconds_count[5m]))
```

#### `mcp_active_requests`

**Type**: Gauge

**Description**: Number of requests currently being processed

**Labels**:
- `tool` - Tool name

**Example**:
```promql
# Current active requests by tool
sum(mcp_active_requests) by (tool)

# Total active requests
sum(mcp_active_requests)

# Alert on high concurrent requests
mcp_active_requests > 50
```

### Cache Metrics

#### `mcp_cache_hits_total`

**Type**: Counter

**Description**: Total number of workbook cache hits

**Example**:
```promql
# Cache hit rate over 5 minutes
rate(mcp_cache_hits_total[5m]) / (rate(mcp_cache_hits_total[5m]) + rate(mcp_cache_misses_total[5m]))

# Increase in cache hits
increase(mcp_cache_hits_total[1h])
```

#### `mcp_cache_misses_total`

**Type**: Counter

**Description**: Total number of workbook cache misses

**Example**:
```promql
# Cache miss rate
rate(mcp_cache_misses_total[5m])

# Cache effectiveness (hit ratio)
sum(rate(mcp_cache_hits_total[5m])) / sum(rate(mcp_cache_hits_total[5m]) + rate(mcp_cache_misses_total[5m]))
```

#### `mcp_cache_size_bytes`

**Type**: Gauge

**Description**: Estimated cache size in bytes

**Example**:
```promql
# Cache size in MB
mcp_cache_size_bytes / 1024 / 1024

# Alert if cache is too large
mcp_cache_size_bytes > 1e9  # 1GB
```

#### `mcp_workbooks_total`

**Type**: Gauge

**Description**: Total number of workbooks currently in cache

**Example**:
```promql
# Current workbook count
mcp_workbooks_total

# Average workbook size
mcp_cache_size_bytes / mcp_workbooks_total

# Alert if cache is full
mcp_workbooks_total >= 100  # Assuming max capacity of 100
```

### Fork Metrics (Recalc Feature)

#### `mcp_forks_total`

**Type**: Gauge

**Description**: Total number of active forks

**Example**:
```promql
# Current fork count
mcp_forks_total

# Alert on too many forks
mcp_forks_total > 10

# Fork creation rate (approximation)
rate(mcp_forks_total[5m])
```

#### `mcp_libreoffice_processes_active`

**Type**: Gauge

**Description**: Number of active LibreOffice processes

**Example**:
```promql
# Current LibreOffice process count
mcp_libreoffice_processes_active

# Alert on process leak
mcp_libreoffice_processes_active > 5
```

#### `mcp_recalc_duration_seconds`

**Type**: Histogram

**Description**: Recalculation duration in seconds

**Buckets**: Exponential buckets from 100ms to 400s (0.1, 0.2, 0.4, 0.8, 1.6, 3.2, 6.4, 12.8, 25.6, 51.2, 102.4, 204.8, 409.6)

**Example**:
```promql
# P99 recalc duration
histogram_quantile(0.99, rate(mcp_recalc_duration_seconds_bucket[5m]))

# Average recalc time
rate(mcp_recalc_duration_seconds_sum[5m]) / rate(mcp_recalc_duration_seconds_count[5m])

# Slow recalculations (>10s)
histogram_quantile(0.95, rate(mcp_recalc_duration_seconds_bucket[5m])) > 10
```

### Error Metrics

#### `mcp_errors_total`

**Type**: Counter

**Description**: Total number of errors by tool and error type

**Labels**:
- `tool` - Tool name
- `error_type` - Error classification ("not_found", "timeout", "permission_denied", "invalid_input", "parse_error", "io_error", "capacity_exceeded", "fork_error", "recalc_error", "cache_error", "unknown")

**Example**:
```promql
# Error rate by type
rate(mcp_errors_total[5m])

# Top error types
topk(5, sum(rate(mcp_errors_total[5m])) by (error_type))

# Errors by tool
sum(rate(mcp_errors_total[5m])) by (tool)

# Timeout errors
rate(mcp_errors_total{error_type="timeout"}[5m])
```

## Grafana Dashboard

### Dashboard JSON

```json
{
  "dashboard": {
    "title": "Spreadsheet MCP Server Metrics",
    "timezone": "browser",
    "refresh": "30s",
    "panels": [
      {
        "title": "Request Rate",
        "targets": [
          {
            "expr": "sum(rate(mcp_requests_total[5m])) by (status)",
            "legendFormat": "{{status}}"
          }
        ],
        "type": "graph"
      },
      {
        "title": "Request Latency (P95, P99)",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, rate(mcp_request_duration_seconds_bucket[5m]))",
            "legendFormat": "P95"
          },
          {
            "expr": "histogram_quantile(0.99, rate(mcp_request_duration_seconds_bucket[5m]))",
            "legendFormat": "P99"
          }
        ],
        "type": "graph"
      },
      {
        "title": "Cache Hit Rate",
        "targets": [
          {
            "expr": "rate(mcp_cache_hits_total[5m]) / (rate(mcp_cache_hits_total[5m]) + rate(mcp_cache_misses_total[5m]))",
            "legendFormat": "Hit Rate"
          }
        ],
        "type": "graph"
      },
      {
        "title": "Active Requests",
        "targets": [
          {
            "expr": "sum(mcp_active_requests)",
            "legendFormat": "Active Requests"
          }
        ],
        "type": "graph"
      },
      {
        "title": "Workbooks in Cache",
        "targets": [
          {
            "expr": "mcp_workbooks_total",
            "legendFormat": "Workbooks"
          }
        ],
        "type": "graph"
      },
      {
        "title": "Active Forks",
        "targets": [
          {
            "expr": "mcp_forks_total",
            "legendFormat": "Forks"
          }
        ],
        "type": "graph"
      },
      {
        "title": "Error Rate by Type",
        "targets": [
          {
            "expr": "sum(rate(mcp_errors_total[5m])) by (error_type)",
            "legendFormat": "{{error_type}}"
          }
        ],
        "type": "graph"
      },
      {
        "title": "Recalc Duration (P95)",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, rate(mcp_recalc_duration_seconds_bucket[5m]))",
            "legendFormat": "P95"
          }
        ],
        "type": "graph"
      }
    ]
  }
}
```

### Dashboard Import

1. Open Grafana
2. Go to Dashboards → Import
3. Paste the JSON above
4. Select your Prometheus data source
5. Click Import

## Prometheus Alerting Rules

### Critical Alerts

```yaml
groups:
  - name: mcp_critical
    interval: 30s
    rules:
      # High error rate
      - alert: MCPHighErrorRate
        expr: |
          rate(mcp_requests_total{status="error"}[5m]) / rate(mcp_requests_total[5m]) > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "MCP server has high error rate"
          description: "Error rate is {{ $value | humanizePercentage }} (threshold: 5%)"

      # Service down
      - alert: MCPServiceDown
        expr: up{job="mcp-server"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "MCP server is down"
          description: "MCP server has been down for more than 1 minute"

      # High latency
      - alert: MCPHighLatency
        expr: |
          histogram_quantile(0.99, rate(mcp_request_duration_seconds_bucket[5m])) > 10
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "MCP server has high latency"
          description: "P99 latency is {{ $value }}s (threshold: 10s)"

      # Too many active forks
      - alert: MCPTooManyForks
        expr: mcp_forks_total > 10
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Too many active forks"
          description: "{{ $value }} forks are active (threshold: 10)"

      # LibreOffice process leak
      - alert: MCPLibreOfficeProcessLeak
        expr: mcp_libreoffice_processes_active > 5
        for: 10m
        labels:
          severity: critical
        annotations:
          summary: "Potential LibreOffice process leak"
          description: "{{ $value }} LibreOffice processes are active (threshold: 5)"
```

### Warning Alerts

```yaml
groups:
  - name: mcp_warning
    interval: 1m
    rules:
      # Low cache hit rate
      - alert: MCPLowCacheHitRate
        expr: |
          rate(mcp_cache_hits_total[10m]) / (rate(mcp_cache_hits_total[10m]) + rate(mcp_cache_misses_total[10m])) < 0.5
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Low cache hit rate"
          description: "Cache hit rate is {{ $value | humanizePercentage }} (threshold: 50%)"

      # High timeout rate
      - alert: MCPHighTimeoutRate
        expr: |
          rate(mcp_requests_total{status="timeout"}[5m]) / rate(mcp_requests_total[5m]) > 0.01
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High request timeout rate"
          description: "Timeout rate is {{ $value | humanizePercentage }} (threshold: 1%)"

      # Slow recalculations
      - alert: MCPSlowRecalculations
        expr: |
          histogram_quantile(0.95, rate(mcp_recalc_duration_seconds_bucket[5m])) > 30
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Slow recalculations detected"
          description: "P95 recalc duration is {{ $value }}s (threshold: 30s)"

      # Cache nearly full
      - alert: MCPCacheNearlyFull
        expr: mcp_workbooks_total >= 90  # Assuming capacity of 100
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Cache nearly full"
          description: "{{ $value }} workbooks in cache (threshold: 90)"
```

## Prometheus Configuration

Add this job to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'mcp-server'
    scrape_interval: 15s
    scrape_timeout: 10s
    metrics_path: '/metrics'
    static_configs:
      - targets: ['localhost:8079']
        labels:
          service: 'spreadsheet-mcp'
          environment: 'production'
```

## Query Examples

### Performance Analysis

```promql
# Request throughput (requests per second)
sum(rate(mcp_requests_total[5m]))

# Top 5 slowest tools
topk(5,
  rate(mcp_request_duration_seconds_sum[5m]) /
  rate(mcp_request_duration_seconds_count[5m])
)

# Success rate by tool
sum(rate(mcp_requests_total{status="success"}[5m])) by (tool) /
sum(rate(mcp_requests_total[5m])) by (tool)

# Request latency percentiles
histogram_quantile(0.50, rate(mcp_request_duration_seconds_bucket[5m])) # P50
histogram_quantile(0.95, rate(mcp_request_duration_seconds_bucket[5m])) # P95
histogram_quantile(0.99, rate(mcp_request_duration_seconds_bucket[5m])) # P99
```

### Cache Analysis

```promql
# Cache hit rate (percentage)
100 * rate(mcp_cache_hits_total[5m]) /
(rate(mcp_cache_hits_total[5m]) + rate(mcp_cache_misses_total[5m]))

# Cache efficiency over time
(rate(mcp_cache_hits_total[1h]) /
(rate(mcp_cache_hits_total[1h]) + rate(mcp_cache_misses_total[1h])))

# Average workbook size in cache
mcp_cache_size_bytes / mcp_workbooks_total

# Cache utilization (as percentage of capacity)
100 * mcp_workbooks_total / 100  # Assuming capacity of 100
```

### Error Analysis

```promql
# Error rate (errors per second)
sum(rate(mcp_errors_total[5m]))

# Errors by type (top 5)
topk(5, sum(rate(mcp_errors_total[5m])) by (error_type))

# Error percentage by tool
100 * sum(rate(mcp_requests_total{status="error"}[5m])) by (tool) /
sum(rate(mcp_requests_total[5m])) by (tool)

# Most problematic tools (by error count)
topk(5, sum(rate(mcp_errors_total[5m])) by (tool))
```

### Fork & Recalc Analysis

```promql
# Fork lifecycle duration (approximate)
changes(mcp_forks_total[5m])

# Recalc throughput (recalcs per second)
rate(mcp_recalc_duration_seconds_count[5m])

# Average recalc duration
rate(mcp_recalc_duration_seconds_sum[5m]) /
rate(mcp_recalc_duration_seconds_count[5m])

# Long-running recalculations (P99)
histogram_quantile(0.99, rate(mcp_recalc_duration_seconds_bucket[5m]))

# Concurrent LibreOffice processes over time
mcp_libreoffice_processes_active
```

## Integration Guide

### Docker Compose Example

```yaml
version: '3.8'

services:
  mcp-server:
    image: spreadsheet-mcp:latest
    ports:
      - "8079:8079"
    environment:
      - SPREADSHEET_MCP_TRANSPORT=http
      - SPREADSHEET_MCP_HTTP_BIND=0.0.0.0:8079
    volumes:
      - ./data:/data

  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - ./alerts.yml:/etc/prometheus/alerts.yml
      - prometheus-data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
      - GF_USERS_ALLOW_SIGN_UP=false
    volumes:
      - grafana-data:/var/lib/grafana
    depends_on:
      - prometheus

volumes:
  prometheus-data:
  grafana-data:
```

### Kubernetes Monitoring

```yaml
apiVersion: v1
kind: Service
metadata:
  name: mcp-server
  labels:
    app: mcp-server
spec:
  ports:
  - port: 8079
    name: http
  selector:
    app: mcp-server

---
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: mcp-server
  labels:
    app: mcp-server
spec:
  selector:
    matchLabels:
      app: mcp-server
  endpoints:
  - port: http
    path: /metrics
    interval: 15s
```

## Best Practices

### 1. Scrape Interval

- **Development**: 30s - 1m
- **Production**: 15s - 30s
- **High-traffic**: 10s - 15s

### 2. Retention

- **Short-term**: 15 days (detailed metrics)
- **Long-term**: 1 year (aggregated metrics)

### 3. Alerting Thresholds

Adjust these based on your workload:

- **Error rate**: 5% (critical), 2% (warning)
- **Latency P99**: 10s (critical), 5s (warning)
- **Cache hit rate**: <30% (critical), <50% (warning)
- **Active forks**: >10 (critical), >5 (warning)

### 4. Recording Rules

Create recording rules for frequently used queries:

```yaml
groups:
  - name: mcp_recording_rules
    interval: 30s
    rules:
      - record: job:mcp_request_rate:5m
        expr: sum(rate(mcp_requests_total[5m]))

      - record: job:mcp_error_rate:5m
        expr: sum(rate(mcp_requests_total{status="error"}[5m]))

      - record: job:mcp_cache_hit_rate:5m
        expr: |
          rate(mcp_cache_hits_total[5m]) /
          (rate(mcp_cache_hits_total[5m]) + rate(mcp_cache_misses_total[5m]))
```

## Troubleshooting

### Metrics not appearing

1. Verify HTTP transport is enabled: `SPREADSHEET_MCP_TRANSPORT=http`
2. Check `/metrics` endpoint: `curl http://localhost:8079/metrics`
3. Verify Prometheus configuration and targets
4. Check Prometheus logs for scrape errors

### High cardinality

If you see high cardinality warnings:

1. Limit tool names (avoid dynamic tool names)
2. Aggregate error types (don't create unique error types per error message)
3. Use recording rules for complex queries

### Missing metrics

Ensure metrics are being recorded:

1. Tool handlers use `run_tool_with_timeout` (automatically instrumented)
2. Cache operations call `update_cache_metrics()`
3. Fork operations call `update_fork_metrics()`
4. Recalc operations record duration

## Security Considerations

### 1. Metrics Endpoint Protection

The `/metrics` endpoint should be protected in production:

- Use network policies to restrict access
- Place behind a reverse proxy with authentication
- Use TLS for transport encryption

### 2. Sensitive Data

Metrics do not contain:
- Workbook content
- Cell values
- File paths (only counts)
- User information

### 3. Rate Limiting

Consider rate limiting the `/metrics` endpoint to prevent abuse.

## Further Reading

- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Documentation](https://grafana.com/docs/)
- [Prometheus Best Practices](https://prometheus.io/docs/practices/)
- [PromQL Cheat Sheet](https://promlabs.com/promql-cheat-sheet/)
