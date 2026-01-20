# GGEN MCP Service Level Objectives (SLOs)

## Overview

This document defines the Service Level Objectives (SLOs) for the GGEN MCP server. These objectives represent our commitments to service reliability, performance, and availability.

## Table of Contents

1. [SLO Definitions](#slo-definitions)
2. [SLI Measurements](#sli-measurements)
3. [Error Budgets](#error-budgets)
4. [Monitoring and Alerting](#monitoring-and-alerting)
5. [Review Process](#review-process)

## SLO Definitions

### 1. Availability SLO

**Objective**: 99.9% uptime (three nines)
**Measurement Period**: 30 days
**Allowed Downtime**: 43 minutes per month

#### Definition

Service is considered "available" when:
- Health endpoint (`/health`) returns HTTP 200
- Service can process requests successfully
- At least one instance is serving traffic

#### Exclusions

The following are excluded from availability calculations:
- Planned maintenance windows (with 7 days notice)
- Client-side errors (4xx responses)
- DDoS attacks or abuse
- Force majeure events

#### SLI Query

```promql
# Availability over 30 days
(
  sum(up{job="ggen-mcp"}) * 100
  /
  count(up{job="ggen-mcp"})
)
```

#### Alert

```yaml
- alert: GgenMcpAvailabilitySLOViolation
  expr: |
    (
      sum(rate(ggen_mcp_requests_total[1h]))
      -
      sum(rate(ggen_mcp_errors_total{error_type=~"5xx|unavailable"}[1h]))
    )
    /
    sum(rate(ggen_mcp_requests_total[1h]))
    < 0.999
  for: 5m
```

### 2. Latency SLO

**Objective**: 95% of requests complete within target latency thresholds
**Measurement Period**: 7 days

#### Latency Targets by Tool Category

| Category | Operation Type | P95 Latency | P99 Latency |
|----------|---------------|-------------|-------------|
| Fast | Health checks, metrics | < 100ms | < 200ms |
| Medium | SPARQL queries, template rendering | < 500ms | < 1s |
| Slow | Recalculation, screenshot generation | < 5s | < 10s |

#### Fast Operations
- `/health` endpoint
- `/metrics` endpoint
- Simple tool calls without heavy computation

#### Medium Operations
- `execute_sparql_query` tool
- `render_template` tool
- Cache lookups

#### Slow Operations
- `recalculate_workbook` tool
- `generate_screenshot` tool
- Complex spreadsheet operations

#### SLI Query

```promql
# Percentage of requests under 5s (overall)
(
  sum(rate(ggen_mcp_request_duration_seconds_bucket{le="5"}[7d]))
  /
  sum(rate(ggen_mcp_request_duration_seconds_count[7d]))
) * 100
```

#### Tool-Specific Queries

```promql
# Fast operations (<100ms)
histogram_quantile(0.95,
  rate(ggen_mcp_request_duration_seconds_bucket{tool=~"health|metrics"}[5m])
) < 0.1

# Medium operations (<500ms)
histogram_quantile(0.95,
  rate(ggen_mcp_request_duration_seconds_bucket{tool=~"query|template"}[5m])
) < 0.5

# Slow operations (<5s)
histogram_quantile(0.95,
  rate(ggen_mcp_request_duration_seconds_bucket{tool=~"recalc|screenshot"}[5m])
) < 5
```

#### Alert

```yaml
- alert: GgenMcpLatencySLOViolation
  expr: |
    (
      sum(rate(ggen_mcp_request_duration_seconds_bucket{le="5"}[1h]))
      /
      sum(rate(ggen_mcp_request_duration_seconds_count[1h]))
    ) < 0.95
  for: 5m
```

### 3. Error Rate SLO

**Objective**: < 0.1% error rate (99.9% success rate)
**Measurement Period**: 7 days

#### Definition

Errors include:
- 5xx HTTP responses
- Unhandled exceptions
- Service unavailability
- Timeout errors

#### Exclusions

The following are NOT counted as errors:
- 4xx client errors (except 429 rate limiting)
- Validation errors
- Expected business logic errors
- Cancelled requests

#### SLI Query

```promql
# Error rate percentage
(
  sum(rate(ggen_mcp_errors_total[7d]))
  /
  sum(rate(ggen_mcp_requests_total[7d]))
) * 100
```

#### Alert

```yaml
- alert: GgenMcpErrorRateSLOViolation
  expr: |
    (
      sum(rate(ggen_mcp_errors_total[1h]))
      /
      sum(rate(ggen_mcp_requests_total[1h]))
    ) > 0.001
  for: 5m
```

### 4. Data Durability SLO

**Objective**: 99.999% durability (five nines)
**Measurement Period**: 365 days

#### Definition

Data is considered durable when:
- No data loss occurs during normal operations
- Backups are successfully created and verified
- Recovery procedures can restore data

#### Metrics

- Backup success rate: 100%
- Backup verification success rate: 100%
- Recovery time objective (RTO): < 4 hours
- Recovery point objective (RPO): < 1 hour

#### SLI Query

```promql
# Backup success rate
(
  sum(rate(ggen_mcp_backup_success_total[30d]))
  /
  sum(rate(ggen_mcp_backup_attempts_total[30d]))
) * 100
```

### 5. Cache Performance SLO

**Objective**: > 60% cache hit rate (target: 80%)
**Measurement Period**: 7 days

#### Definition

Cache hit rate measures the effectiveness of the workbook cache:
- **Hit**: Requested workbook found in cache
- **Miss**: Workbook needs to be loaded from disk/created

#### SLI Query

```promql
# Cache hit rate percentage
(
  rate(ggen_mcp_cache_hits_total[7d])
  /
  (rate(ggen_mcp_cache_hits_total[7d]) + rate(ggen_mcp_cache_misses_total[7d]))
) * 100
```

#### Alert

```yaml
- alert: GgenMcpCacheHitRateSLOViolation
  expr: |
    (
      rate(ggen_mcp_cache_hits_total[10m])
      /
      (rate(ggen_mcp_cache_hits_total[10m]) + rate(ggen_mcp_cache_misses_total[10m]))
    ) * 100 < 50
  for: 10m
```

## SLI Measurements

### Service Level Indicators (SLIs)

SLIs are the actual measurements used to calculate SLO compliance:

| SLI | Measurement | Good Event | Bad Event | Target |
|-----|-------------|------------|-----------|--------|
| Availability | Uptime ratio | Service up | Service down | 99.9% |
| Latency | Request duration | < threshold | ≥ threshold | 95% |
| Error Rate | Success ratio | HTTP 2xx/3xx | HTTP 5xx | 99.9% |
| Durability | Data integrity | No loss | Data loss | 99.999% |
| Cache Hit Rate | Cache ratio | Cache hit | Cache miss | 60% |

### Data Collection

All SLIs are collected via:
- Prometheus metrics from `/metrics` endpoint
- Scrape interval: 15 seconds
- Retention period: 30 days
- Recording rules for aggregated metrics

### Calculation Windows

| SLO | Window | Reason |
|-----|--------|--------|
| Availability | 30 days | Industry standard, accounts for monthly patterns |
| Latency | 7 days | Quick feedback on performance changes |
| Error Rate | 7 days | Balance between responsiveness and stability |
| Durability | 365 days | Long-term commitment, rare events |
| Cache Hit Rate | 7 days | Reflects cache effectiveness over typical workload |

## Error Budgets

### Error Budget Definition

An error budget is the maximum amount of unavailability or errors allowed while still meeting the SLO.

### Calculation

```
Error Budget = 100% - SLO Target
```

### Error Budgets by SLO

| SLO | Target | Error Budget | Monthly Budget |
|-----|--------|--------------|----------------|
| Availability | 99.9% | 0.1% | 43 minutes |
| Latency | 95% under threshold | 5% slow requests | 108,000 slow requests/month (at 50 req/s) |
| Error Rate | 99.9% success | 0.1% errors | 2,160 errors/month (at 50 req/s) |
| Cache Hit Rate | 60% hits | 40% misses | Unlimited (target, not guarantee) |

### Error Budget Policy

#### When Error Budget is Depleted (< 10% remaining)

1. **Freeze non-critical features** - Focus on reliability
2. **Increase monitoring** - More frequent reviews
3. **Post-incident reviews** - Root cause analysis required
4. **Improve testing** - Add more reliability tests
5. **Defer risky changes** - Wait for budget to recover

#### When Error Budget is Healthy (> 50% remaining)

1. **Normal development** - Ship features as planned
2. **Take calculated risks** - Experiment with new approaches
3. **Maintenance work** - Infrastructure upgrades
4. **Performance improvements** - Optimize for future

### Error Budget Tracking

Query current error budget:

```promql
# Availability error budget remaining (%)
(
  (ggen_mcp:availability_percentage:1h - 99.9) / 0.1 * 100
)

# Error rate budget remaining (%)
(
  (0.1 - ggen_mcp:error_rate_percentage:5m) / 0.1 * 100
)
```

Dashboard panel:
- Red: < 10% remaining
- Yellow: 10-50% remaining
- Green: > 50% remaining

## Monitoring and Alerting

### SLO Dashboard

**URL**: http://localhost:3000/d/ggen-mcp-slo

**Panels**:
- Current SLO compliance (%)
- Error budget remaining (%)
- Trend over time (7 days, 30 days)
- Breakdown by component
- Recent SLO violations

### Alerting Strategy

#### SLO Violation Alerts

Alerts fire when:
- **Burn rate is high**: Consuming error budget rapidly
- **Budget exhaustion**: Error budget < 10%
- **Sustained degradation**: Poor performance for extended period

#### Multi-window, Multi-burn-rate Alerts

```yaml
# Fast burn (1h window) - Page immediately
- alert: GgenMcpAvailabilityFastBurn
  expr: |
    (1 - ggen_mcp:availability_percentage:1h / 100) > (14.4 * 0.001)
  for: 2m

# Slow burn (6h window) - Warning
- alert: GgenMcpAvailabilitySlowBurn
  expr: |
    (1 - ggen_mcp:availability_percentage:6h / 100) > (6 * 0.001)
  for: 15m
```

### Review Cadence

- **Real-time**: Automated alerts for violations
- **Daily**: Review error budget consumption
- **Weekly**: SLO compliance review with team
- **Monthly**: Comprehensive SLO report to stakeholders
- **Quarterly**: SLO target review and adjustment

## Review Process

### Weekly SLO Review

**Participants**: Engineering team, SRE, product owners

**Agenda**:
1. Current SLO compliance status
2. Error budget remaining
3. Recent incidents and impact
4. Trends and patterns
5. Action items

### Monthly SLO Report

**Contents**:
- SLO compliance summary (met/not met)
- Error budget utilization
- Top contributors to SLO violations
- Improvement initiatives
- Forecast for next month

### Quarterly SLO Adjustment

**Criteria for Changing SLOs**:
1. **Consistently exceeding**: SLO too loose, consider tightening
2. **Consistently missing**: SLO too tight, consider loosening
3. **Business requirements change**: Adjust to match user expectations
4. **Cost considerations**: Balance reliability with infrastructure costs

**Process**:
1. Analyze historical data (3+ months)
2. Gather stakeholder feedback
3. Propose new targets
4. Simulate impact on error budgets
5. Update SLO definitions
6. Update monitoring and alerts
7. Communicate changes

## Appendix

### Prometheus Recording Rules for SLOs

Located in `/prometheus/rules/ggen-mcp.yml`:

```yaml
# Availability (1h)
- record: ggen_mcp:availability_percentage:1h
  expr: |
    (
      sum(rate(ggen_mcp_requests_total[1h]))
      -
      sum(rate(ggen_mcp_errors_total{error_type=~"5xx|unavailable"}[1h]))
    )
    /
    sum(rate(ggen_mcp_requests_total[1h]))
    * 100

# Latency SLO compliance (1h)
- record: ggen_mcp:latency_slo_percentage:1h
  expr: |
    (
      sum(rate(ggen_mcp_request_duration_seconds_bucket{le="5"}[1h]))
      /
      sum(rate(ggen_mcp_request_duration_seconds_count[1h]))
    ) * 100
```

### SLO Reporting Queries

```promql
# Monthly availability
avg_over_time(ggen_mcp:availability_percentage:1h[30d])

# Weekly latency compliance
avg_over_time(ggen_mcp:latency_slo_percentage:1h[7d])

# Error budget consumption rate (per day)
(1 - ggen_mcp:availability_percentage:1h / 100) * 24 * 3600
```

### References

- [Google SRE Book - Service Level Objectives](https://sre.google/sre-book/service-level-objectives/)
- [Implementing SLOs](https://sre.google/workbook/implementing-slos/)
- [The Art of SLOs](https://www.usenix.org/conference/srecon19americas/presentation/slides)

## Revision History

| Date | Version | Changes | Author |
|------|---------|---------|--------|
| 2026-01-20 | 1.0 | Initial SLO definitions | Platform Team |
