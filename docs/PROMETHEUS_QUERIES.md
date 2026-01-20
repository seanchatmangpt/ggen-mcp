# Prometheus Queries for GGEN MCP Monitoring

## Overview

This document provides a comprehensive collection of Prometheus queries for monitoring and analyzing GGEN MCP server performance. All queries can be used in Grafana dashboards or directly in the Prometheus UI.

## Table of Contents

1. [Request Metrics](#request-metrics)
2. [Error Metrics](#error-metrics)
3. [Latency Metrics](#latency-metrics)
4. [Cache Metrics](#cache-metrics)
5. [Resource Metrics](#resource-metrics)
6. [Operation Performance](#operation-performance)
7. [Health & SLO Metrics](#health--slo-metrics)
8. [Troubleshooting Queries](#troubleshooting-queries)
9. [Advanced Analytics](#advanced-analytics)

---

## Request Metrics

### Total Request Rate

```promql
# Requests per second (last 5 minutes)
rate(ggen_mcp_requests_total[5m])

# Total requests per second across all tools
sum(rate(ggen_mcp_requests_total[5m]))

# Requests per minute
rate(ggen_mcp_requests_total[1m]) * 60
```

### Request Rate by Tool

```promql
# Requests per second by tool
sum by (tool) (rate(ggen_mcp_requests_total[5m]))

# Top 5 most used tools
topk(5, sum by (tool) (rate(ggen_mcp_requests_total[5m])))

# Request rate for specific tool
sum(rate(ggen_mcp_requests_total{tool="execute_sparql_query"}[5m]))
```

### Request Rate by Status

```promql
# Requests by HTTP status code
sum by (status) (rate(ggen_mcp_requests_total[5m]))

# Success rate (2xx and 3xx)
sum(rate(ggen_mcp_requests_total{status=~"2..|3.."}[5m]))

# Failed requests (4xx and 5xx)
sum(rate(ggen_mcp_requests_total{status=~"4..|5.."}[5m]))
```

### Request Volume

```promql
# Total requests in last hour
sum(increase(ggen_mcp_requests_total[1h]))

# Total requests today
sum(increase(ggen_mcp_requests_total[1d]))

# Request count by tool (last hour)
sum by (tool) (increase(ggen_mcp_requests_total[1h]))
```

### Active Requests

```promql
# Current number of active requests
ggen_mcp_active_requests

# Max active requests in last 5 minutes
max_over_time(ggen_mcp_active_requests[5m])

# Average active requests
avg_over_time(ggen_mcp_active_requests[5m])
```

---

## Error Metrics

### Error Rate

```promql
# Errors per second
rate(ggen_mcp_errors_total[5m])

# Total errors per second
sum(rate(ggen_mcp_errors_total[5m]))

# Error rate as percentage
(
  sum(rate(ggen_mcp_errors_total[5m]))
  /
  sum(rate(ggen_mcp_requests_total[5m]))
) * 100
```

### Errors by Type

```promql
# Errors per second by error type
sum by (error_type) (rate(ggen_mcp_errors_total[5m]))

# Most common error types
topk(5, sum by (error_type) (rate(ggen_mcp_errors_total[5m])))

# Specific error type rate
sum(rate(ggen_mcp_errors_total{error_type="validation_error"}[5m]))
```

### Errors by Tool

```promql
# Errors by tool
sum by (tool) (rate(ggen_mcp_errors_total[5m]))

# Tools with highest error rate
topk(5, sum by (tool) (rate(ggen_mcp_errors_total[5m])))

# Error percentage by tool
(
  sum by (tool) (rate(ggen_mcp_errors_total[5m]))
  /
  sum by (tool) (rate(ggen_mcp_requests_total[5m]))
) * 100
```

### Error Volume

```promql
# Total errors in last hour
sum(increase(ggen_mcp_errors_total[1h]))

# Errors by type in last hour
sum by (error_type) (increase(ggen_mcp_errors_total[1h]))

# Error count trend (compare to 1 hour ago)
sum(increase(ggen_mcp_errors_total[1h]))
-
sum(increase(ggen_mcp_errors_total[1h] offset 1h))
```

### Error Rate Change

```promql
# Error rate change from 1 hour ago (percentage)
(
  (
    sum(rate(ggen_mcp_errors_total[5m]))
    -
    sum(rate(ggen_mcp_errors_total[5m] offset 1h))
  )
  /
  sum(rate(ggen_mcp_errors_total[5m] offset 1h))
) * 100
```

---

## Latency Metrics

### Latency Percentiles

```promql
# P50 (median) latency
histogram_quantile(0.50,
  sum(rate(ggen_mcp_request_duration_seconds_bucket[5m])) by (le)
)

# P95 latency
histogram_quantile(0.95,
  sum(rate(ggen_mcp_request_duration_seconds_bucket[5m])) by (le)
)

# P99 latency
histogram_quantile(0.99,
  sum(rate(ggen_mcp_request_duration_seconds_bucket[5m])) by (le)
)

# P99.9 latency
histogram_quantile(0.999,
  sum(rate(ggen_mcp_request_duration_seconds_bucket[5m])) by (le)
)
```

### Latency by Tool

```promql
# P95 latency by tool
histogram_quantile(0.95,
  sum by (tool, le) (rate(ggen_mcp_request_duration_seconds_bucket[5m]))
)

# Average latency by tool
sum by (tool) (rate(ggen_mcp_request_duration_seconds_sum[5m]))
/
sum by (tool) (rate(ggen_mcp_request_duration_seconds_count[5m]))

# Slowest tools (P95)
topk(5, histogram_quantile(0.95,
  sum by (tool, le) (rate(ggen_mcp_request_duration_seconds_bucket[5m]))
))
```

### Latency Distribution

```promql
# Requests under 100ms
sum(rate(ggen_mcp_request_duration_seconds_bucket{le="0.1"}[5m]))

# Requests between 100ms and 1s
sum(rate(ggen_mcp_request_duration_seconds_bucket{le="1"}[5m]))
-
sum(rate(ggen_mcp_request_duration_seconds_bucket{le="0.1"}[5m]))

# Requests over 5s
sum(rate(ggen_mcp_request_duration_seconds_bucket{le="+Inf"}[5m]))
-
sum(rate(ggen_mcp_request_duration_seconds_bucket{le="5"}[5m]))
```

### Latency Trends

```promql
# P95 latency change from 1 hour ago
histogram_quantile(0.95,
  sum(rate(ggen_mcp_request_duration_seconds_bucket[5m])) by (le)
)
-
histogram_quantile(0.95,
  sum(rate(ggen_mcp_request_duration_seconds_bucket[5m] offset 1h)) by (le)
)
```

---

## Cache Metrics

### Cache Hit Rate

```promql
# Cache hit rate (percentage)
(
  rate(ggen_mcp_cache_hits_total[5m])
  /
  (rate(ggen_mcp_cache_hits_total[5m]) + rate(ggen_mcp_cache_misses_total[5m]))
) * 100

# Cache hit rate by tool
(
  rate(ggen_mcp_cache_hits_total[5m])
  /
  (rate(ggen_mcp_cache_hits_total[5m]) + rate(ggen_mcp_cache_misses_total[5m]))
) by (tool) * 100

# Cache miss rate
(
  rate(ggen_mcp_cache_misses_total[5m])
  /
  (rate(ggen_mcp_cache_hits_total[5m]) + rate(ggen_mcp_cache_misses_total[5m]))
) * 100
```

### Cache Operations

```promql
# Cache hits per second
rate(ggen_mcp_cache_hits_total[5m])

# Cache misses per second
rate(ggen_mcp_cache_misses_total[5m])

# Cache evictions per second
rate(ggen_mcp_cache_evictions_total[5m])

# Total cache operations per second
sum(
  rate(ggen_mcp_cache_hits_total[5m]) +
  rate(ggen_mcp_cache_misses_total[5m]) +
  rate(ggen_mcp_cache_evictions_total[5m])
)
```

### Cache Size and Memory

```promql
# Current cache size (workbooks)
ggen_mcp_cache_size_workbooks

# Cache memory usage (bytes)
ggen_mcp_cache_memory_bytes

# Cache memory usage (MB)
ggen_mcp_cache_memory_bytes / (1024 * 1024)

# Cache memory usage (GB)
ggen_mcp_cache_memory_bytes / (1024 * 1024 * 1024)

# Average cache size over time
avg_over_time(ggen_mcp_cache_size_workbooks[1h])
```

### Cache Efficiency

```promql
# Cache effectiveness score (0-100)
(
  (rate(ggen_mcp_cache_hits_total[5m]) * 2)
  /
  (rate(ggen_mcp_cache_hits_total[5m]) + rate(ggen_mcp_cache_misses_total[5m]))
) * 50

# Cache utilization (percentage of max size)
(ggen_mcp_cache_size_workbooks / 1000) * 100  # Assuming max 1000

# Eviction rate relative to cache operations
rate(ggen_mcp_cache_evictions_total[5m])
/
(rate(ggen_mcp_cache_hits_total[5m]) + rate(ggen_mcp_cache_misses_total[5m]))
```

---

## Resource Metrics

### CPU Usage

```promql
# CPU usage percentage
rate(process_cpu_seconds_total{job="ggen-mcp"}[5m]) * 100

# CPU usage over time
avg_over_time(
  rate(process_cpu_seconds_total{job="ggen-mcp"}[5m])[1h:]
) * 100

# CPU saturation (if multi-core)
rate(process_cpu_seconds_total{job="ggen-mcp"}[5m]) / 4 * 100  # 4 cores
```

### Memory Usage

```promql
# Resident memory (RSS) in bytes
process_resident_memory_bytes{job="ggen-mcp"}

# RSS in MB
process_resident_memory_bytes{job="ggen-mcp"} / (1024 * 1024)

# RSS in GB
process_resident_memory_bytes{job="ggen-mcp"} / (1024 * 1024 * 1024)

# Memory usage percentage (assuming 4GB limit)
(process_resident_memory_bytes{job="ggen-mcp"} / (4 * 1024 * 1024 * 1024)) * 100
```

### Node.js Heap

```promql
# Heap used (bytes)
nodejs_heap_size_used_bytes{job="ggen-mcp"}

# Heap total (bytes)
nodejs_heap_size_total_bytes{job="ggen-mcp"}

# Heap usage percentage
(
  nodejs_heap_size_used_bytes{job="ggen-mcp"}
  /
  nodejs_heap_size_total_bytes{job="ggen-mcp"}
) * 100

# Heap growth rate (bytes per second)
deriv(nodejs_heap_size_used_bytes{job="ggen-mcp"}[5m])
```

### Process Counts

```promql
# Active fork processes
ggen_mcp_active_forks

# LibreOffice processes
ggen_mcp_libreoffice_processes

# Total process count
ggen_mcp_active_forks + ggen_mcp_libreoffice_processes

# Max processes in last hour
max_over_time(ggen_mcp_libreoffice_processes[1h])
```

### File Descriptors

```promql
# Open file descriptors
process_open_fds{job="ggen-mcp"}

# Max file descriptors
process_max_fds{job="ggen-mcp"}

# File descriptor usage percentage
(
  process_open_fds{job="ggen-mcp"}
  /
  process_max_fds{job="ggen-mcp"}
) * 100
```

---

## Operation Performance

### Recalculation Performance

```promql
# P95 recalculation duration
histogram_quantile(0.95,
  rate(ggen_mcp_recalc_duration_seconds_bucket[5m])
)

# Average recalculation duration
sum(rate(ggen_mcp_recalc_duration_seconds_sum[5m]))
/
sum(rate(ggen_mcp_recalc_duration_seconds_count[5m]))

# Recalculations per second
rate(ggen_mcp_recalc_duration_seconds_count[5m])
```

### Query Performance

```promql
# P95 query execution duration
histogram_quantile(0.95,
  rate(ggen_mcp_query_duration_seconds_bucket[5m])
)

# Average query duration
sum(rate(ggen_mcp_query_duration_seconds_sum[5m]))
/
sum(rate(ggen_mcp_query_duration_seconds_count[5m]))

# Queries per second
rate(ggen_mcp_query_duration_seconds_count[5m])

# Slow queries (>5s) count
increase(ggen_mcp_slow_queries_total{duration=">5s"}[5m])
```

### Template Rendering Performance

```promql
# P95 template render duration
histogram_quantile(0.95,
  rate(ggen_mcp_template_render_duration_seconds_bucket[5m])
)

# Average render duration
sum(rate(ggen_mcp_template_render_duration_seconds_sum[5m]))
/
sum(rate(ggen_mcp_template_render_duration_seconds_count[5m]))
```

### Screenshot Generation Performance

```promql
# P95 screenshot duration
histogram_quantile(0.95,
  rate(ggen_mcp_screenshot_duration_seconds_bucket[5m])
)

# Average screenshot duration
sum(rate(ggen_mcp_screenshot_duration_seconds_sum[5m]))
/
sum(rate(ggen_mcp_screenshot_duration_seconds_count[5m]))

# Screenshots per minute
rate(ggen_mcp_screenshot_duration_seconds_count[5m]) * 60
```

---

## Health & SLO Metrics

### Service Availability

```promql
# Service up (1) or down (0)
up{job="ggen-mcp"}

# Availability percentage (last 1 hour)
avg_over_time(up{job="ggen-mcp"}[1h]) * 100

# Uptime in seconds
time() - process_start_time_seconds{job="ggen-mcp"}

# Uptime in hours
(time() - process_start_time_seconds{job="ggen-mcp"}) / 3600
```

### SLO Compliance

```promql
# Availability SLO (99.9%)
(
  sum(rate(ggen_mcp_requests_total[1h]))
  -
  sum(rate(ggen_mcp_errors_total{error_type=~"5xx|unavailable"}[1h]))
)
/
sum(rate(ggen_mcp_requests_total[1h]))
* 100

# Latency SLO (95% < 5s)
(
  sum(rate(ggen_mcp_request_duration_seconds_bucket{le="5"}[1h]))
  /
  sum(rate(ggen_mcp_request_duration_seconds_count[1h]))
) * 100

# Error rate SLO (< 0.1%)
(
  sum(rate(ggen_mcp_errors_total[1h]))
  /
  sum(rate(ggen_mcp_requests_total[1h]))
) * 100
```

### Component Health

```promql
# Component health status
ggen_mcp_component_health

# Unhealthy components
count(ggen_mcp_component_health == 0)

# Health score by component
avg by (component) (ggen_mcp_component_health)
```

---

## Troubleshooting Queries

### Identify High Error Rate Tools

```promql
# Tools with error rate > 5%
(
  sum by (tool) (rate(ggen_mcp_errors_total[5m]))
  /
  sum by (tool) (rate(ggen_mcp_requests_total[5m]))
) * 100 > 5
```

### Identify Slow Tools

```promql
# Tools with P95 latency > 5s
histogram_quantile(0.95,
  sum by (tool, le) (rate(ggen_mcp_request_duration_seconds_bucket[5m]))
) > 5
```

### Detect Memory Leaks

```promql
# Memory growth rate (MB per hour)
deriv(process_resident_memory_bytes{job="ggen-mcp"}[1h])
* 3600
/ (1024 * 1024)

# Heap growth rate (MB per hour)
deriv(nodejs_heap_size_used_bytes{job="ggen-mcp"}[1h])
* 3600
/ (1024 * 1024)
```

### Detect Process Leaks

```promql
# LibreOffice process growth rate
deriv(ggen_mcp_libreoffice_processes[5m]) * 3600  # per hour

# Fork process growth rate
deriv(ggen_mcp_active_forks[5m]) * 3600  # per hour
```

### Identify Cache Issues

```promql
# Cache hit rate degradation
(
  ggen_mcp:cache_hit_rate_percentage:5m
  -
  ggen_mcp:cache_hit_rate_percentage:5m offset 1h
)

# High eviction rate
rate(ggen_mcp_cache_evictions_total[5m]) > 0.5  # >0.5/sec
```

---

## Advanced Analytics

### Request Rate Prediction

```promql
# Predict request rate for next hour (simple linear extrapolation)
predict_linear(
  sum(rate(ggen_mcp_requests_total[5m]))[1h:],
  3600
)
```

### Anomaly Detection

```promql
# Request rate deviation from weekly pattern
abs(
  sum(rate(ggen_mcp_requests_total[5m]))
  -
  avg_over_time(sum(rate(ggen_mcp_requests_total[5m]))[7d:])
)
/
stddev_over_time(sum(rate(ggen_mcp_requests_total[5m]))[7d:])
> 2  # More than 2 standard deviations
```

### Capacity Planning

```promql
# Requests per second at current trend in 30 days
predict_linear(
  sum(rate(ggen_mcp_requests_total[5m]))[30d:],
  30 * 24 * 3600
)

# Memory usage trend in 7 days
predict_linear(
  process_resident_memory_bytes{job="ggen-mcp"}[7d:],
  7 * 24 * 3600
) / (1024 * 1024 * 1024)  # GB
```

### Performance Correlation

```promql
# Correlation between request rate and latency
# (High request rate often correlates with higher latency)
(
  sum(rate(ggen_mcp_requests_total[5m]))
  *
  histogram_quantile(0.95,
    sum(rate(ggen_mcp_request_duration_seconds_bucket[5m])) by (le)
  )
)
```

### Throughput Analysis

```promql
# Successful requests per second
sum(rate(ggen_mcp_requests_total{status=~"2..|3.."}[5m]))

# Effective throughput (accounting for retries)
sum(rate(ggen_mcp_requests_total{status=~"2..|3.."}[5m]))
/
sum(rate(ggen_mcp_requests_total[5m]))
* 100
```

### Apdex Score

```promql
# Apdex score (satisfied: <1s, tolerating: <5s)
(
  sum(rate(ggen_mcp_request_duration_seconds_bucket{le="1"}[5m]))
  +
  (sum(rate(ggen_mcp_request_duration_seconds_bucket{le="5"}[5m]))
   - sum(rate(ggen_mcp_request_duration_seconds_bucket{le="1"}[5m]))) / 2
)
/
sum(rate(ggen_mcp_request_duration_seconds_count[5m]))
```

---

## Query Best Practices

### Rate vs Increase

```promql
# Use rate() for per-second rates
rate(ggen_mcp_requests_total[5m])

# Use increase() for total count over period
increase(ggen_mcp_requests_total[1h])

# Never use rate() with [1s] or very short ranges
```

### Aggregation

```promql
# Always aggregate before calculating percentiles
histogram_quantile(0.95,
  sum by (le) (rate(ggen_mcp_request_duration_seconds_bucket[5m]))
)

# Not this:
histogram_quantile(0.95,
  rate(ggen_mcp_request_duration_seconds_bucket[5m])
)
```

### Range Selection

```promql
# Use 4x scrape interval minimum for rate()
rate(ggen_mcp_requests_total[1m])  # 15s scrape = 1m min

# Use longer ranges for stable metrics
rate(ggen_mcp_requests_total[5m])  # Better for dashboards
```

---

## Using Queries in Grafana

### Variables

```promql
# Create variable for tools
label_values(ggen_mcp_requests_total, tool)

# Use variable in query
sum(rate(ggen_mcp_requests_total{tool="$tool"}[5m]))
```

### Time Range

```promql
# Use $__range for dynamic range
sum(rate(ggen_mcp_requests_total[$__range]))

# Use $__interval for auto-adjust
sum(rate(ggen_mcp_requests_total[$__interval]))
```

---

## Additional Resources

- [Prometheus Query Documentation](https://prometheus.io/docs/prometheus/latest/querying/basics/)
- [PromQL Cheat Sheet](https://promlabs.com/promql-cheat-sheet/)
- [Grafana Dashboard Best Practices](https://grafana.com/docs/grafana/latest/best-practices/)
