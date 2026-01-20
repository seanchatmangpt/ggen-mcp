# GGEN MCP Production Monitoring Guide

## Overview

This document provides comprehensive guidance for monitoring the GGEN MCP server in production. The monitoring stack includes Prometheus for metrics, Grafana for visualization, Alertmanager for notifications, Loki for logs, and Jaeger for distributed tracing.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Architecture](#architecture)
3. [Dashboards](#dashboards)
4. [Alerts](#alerts)
5. [Metrics Catalog](#metrics-catalog)
6. [Troubleshooting](#troubleshooting)
7. [Maintenance](#maintenance)

## Quick Start

### Starting the Monitoring Stack

```bash
./scripts/start-monitoring.sh
```

This will start:
- Prometheus (metrics collection)
- Grafana (dashboards)
- Alertmanager (alert routing)
- Loki (log aggregation)
- Promtail (log collection)
- Jaeger (distributed tracing)
- Node Exporter (host metrics)
- cAdvisor (container metrics)
- Blackbox Exporter (endpoint monitoring)

### Access URLs

- **Grafana**: http://localhost:3000 (admin/admin)
- **Prometheus**: http://localhost:9090
- **Alertmanager**: http://localhost:9093
- **Jaeger**: http://localhost:16686
- **Loki**: http://localhost:3100

### Default Credentials

- **Grafana**: admin / admin (change on first login)

## Architecture

### Components

```
┌─────────────────┐
│   GGEN MCP      │
│   Server        │──┐
└─────────────────┘  │
                     │ /metrics
                     ▼
┌─────────────────┐  ┌──────────────┐
│  Prometheus     │──│ Recording    │
│  (Metrics)      │  │ Rules        │
└─────────────────┘  └──────────────┘
         │
         │ alerts    ┌──────────────┐
         └──────────▶│ Alertmanager │
                     └──────────────┘
                            │
              ┌─────────────┼─────────────┐
              ▼             ▼             ▼
         ┌────────┐   ┌─────────┐   ┌──────────┐
         │ Slack  │   │ Email   │   │PagerDuty │
         └────────┘   └─────────┘   └──────────┘

┌─────────────────┐
│  Application    │──logs──▶┌──────────┐
│  Logs           │         │ Promtail │
└─────────────────┘         └──────────┘
                                  │
                                  ▼
                            ┌──────────┐
                            │  Loki    │
                            └──────────┘
                                  │
                                  ▼
                            ┌──────────┐
                            │ Grafana  │◀──queries──┐
                            └──────────┘            │
                                  ▲                 │
                                  └─────────────────┘
```

### Data Flow

1. **Metrics Collection**: Prometheus scrapes `/metrics` endpoint every 15 seconds
2. **Alert Evaluation**: Prometheus evaluates alert rules every 15 seconds
3. **Alert Routing**: Alertmanager routes alerts based on severity
4. **Log Collection**: Promtail tails log files and sends to Loki
5. **Visualization**: Grafana queries Prometheus and Loki for dashboards

## Dashboards

### Main Dashboard (ggen-mcp-prod)

**URL**: http://localhost:3000/d/ggen-mcp-prod

**Panels**:

#### Overview Row
- **Request Rate**: Total requests per second by tool
- **Error Rate**: Errors per second by tool and type
- **Latency**: P50, P95, P99 percentiles by tool
- **Active Requests**: Current number of in-flight requests

#### Cache Performance Row
- **Cache Hit Rate**: Percentage of cache hits
- **Cache Size**: Number of workbooks in cache
- **Cache Memory**: Memory usage in bytes
- **Cache Eviction Rate**: Evictions per second

#### Resource Utilization Row
- **Active Forks**: Number of active fork processes
- **LibreOffice Processes**: Number of running LibreOffice instances
- **Memory Usage**: RSS, heap used, heap total
- **CPU Usage**: CPU utilization percentage

#### Operation Performance Row
- **Recalc Duration**: P50, P95, P99 for spreadsheet recalculation
- **Query Execution**: P50, P95, P99 for SPARQL queries
- **Template Render**: P50, P95, P99 for template rendering
- **Screenshot Generation**: P50, P95, P99 for screenshot creation

#### Health Row
- **Service Uptime**: Service availability status
- **Component Health**: Health status by component
- **Error Breakdown**: Pie chart of errors by type
- **Slow Queries**: Count of queries taking >5 seconds

### Cache Performance Dashboard (ggen-mcp-cache)

**URL**: http://localhost:3000/d/ggen-mcp-cache

Detailed view of cache behavior:
- Hit/miss trends over time
- Cache size and memory growth
- Operation rates (sets, deletes, evictions)
- Entry age distribution
- Per-tool cache effectiveness

## Alerts

### Alert Severity Levels

#### Critical (Page Immediately)
- Service is down
- Error rate > 5% for 5 minutes
- P95 latency > 10 seconds for 5 minutes
- Memory usage > 90% for 5 minutes
- No successful requests for 10 minutes

**Response Time**: Immediate (< 15 minutes)
**Notification**: PagerDuty + Slack

#### Warning (Notify Team)
- Error rate > 1% for 5 minutes
- P95 latency > 5 seconds for 5 minutes
- Cache hit rate < 50% for 10 minutes
- LibreOffice process count > 10 for 5 minutes
- Disk usage > 80%
- Memory usage > 70% for 10 minutes

**Response Time**: Within 1 hour
**Notification**: Slack

#### Info (Log and Track)
- Cache eviction rate increased
- Slow query detected (>10 seconds)
- Deployment completed
- Performance degradation detected

**Response Time**: Next business day
**Notification**: Slack (info channel)

### Alert Routing

Configured in `/alertmanager/config.yml`:

```yaml
Critical → PagerDuty + Slack (#critical)
Warning  → Slack (#warnings)
Info     → Slack (#info)
SLO      → Slack (#slo)
Deploy   → Slack (#deployments)
```

### Silencing Alerts

During maintenance or deployments:

```bash
# Using Alertmanager UI
http://localhost:9093/#/silences

# Using amtool CLI
amtool silence add alertname=GgenMcpDeploymentCompleted \
  --duration=1h \
  --comment="Planned deployment"
```

## Metrics Catalog

### Request Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `ggen_mcp_requests_total` | Counter | Total requests by tool and status |
| `ggen_mcp_request_duration_seconds` | Histogram | Request duration in seconds |
| `ggen_mcp_active_requests` | Gauge | Current active requests |

### Error Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `ggen_mcp_errors_total` | Counter | Total errors by tool and error_type |
| `ggen_mcp_error_rate_percentage` | Gauge | Error rate as percentage |

### Cache Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `ggen_mcp_cache_hits_total` | Counter | Cache hits by tool |
| `ggen_mcp_cache_misses_total` | Counter | Cache misses by tool |
| `ggen_mcp_cache_evictions_total` | Counter | Cache evictions |
| `ggen_mcp_cache_size_workbooks` | Gauge | Number of workbooks in cache |
| `ggen_mcp_cache_memory_bytes` | Gauge | Cache memory usage |

### Resource Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `ggen_mcp_active_forks` | Gauge | Active fork processes |
| `ggen_mcp_libreoffice_processes` | Gauge | LibreOffice processes |
| `process_resident_memory_bytes` | Gauge | RSS memory usage |
| `nodejs_heap_size_used_bytes` | Gauge | Node.js heap used |
| `process_cpu_seconds_total` | Counter | CPU time used |

### Operation Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `ggen_mcp_recalc_duration_seconds` | Histogram | Spreadsheet recalculation time |
| `ggen_mcp_query_duration_seconds` | Histogram | SPARQL query execution time |
| `ggen_mcp_template_render_duration_seconds` | Histogram | Template rendering time |
| `ggen_mcp_screenshot_duration_seconds` | Histogram | Screenshot generation time |

### Health Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `up` | Gauge | Service availability (0 or 1) |
| `ggen_mcp_component_health` | Gauge | Health status by component |
| `ggen_mcp_slow_queries_total` | Counter | Queries exceeding thresholds |

## Troubleshooting

### Dashboard Not Loading

**Symptoms**: Grafana shows "No data" or panels are empty

**Checks**:
1. Verify Prometheus is scraping successfully:
   ```bash
   curl http://localhost:9090/api/v1/targets
   ```
2. Check GGEN MCP metrics endpoint:
   ```bash
   curl http://localhost:9464/metrics
   ```
3. Verify datasource configuration in Grafana

**Resolution**:
- Restart monitoring stack: `./scripts/stop-monitoring.sh && ./scripts/start-monitoring.sh`
- Check Prometheus logs: `docker logs ggen-mcp-prometheus`

### Alerts Not Firing

**Symptoms**: Expected alerts don't trigger

**Checks**:
1. Verify alert rules are loaded:
   ```bash
   curl http://localhost:9090/api/v1/rules
   ```
2. Check alert evaluation:
   ```bash
   curl http://localhost:9090/api/v1/alerts
   ```
3. Verify Alertmanager connectivity:
   ```bash
   curl http://localhost:9093/api/v2/status
   ```

**Resolution**:
- Check alert rule syntax in `/prometheus/alerts/ggen-mcp.yml`
- Verify Alertmanager configuration in `/alertmanager/config.yml`
- Reload Prometheus: `curl -X POST http://localhost:9090/-/reload`

### High Memory Usage

**Symptoms**: Prometheus or Grafana consuming excessive memory

**Checks**:
1. Check Prometheus storage size:
   ```bash
   du -sh prometheus_data/
   ```
2. Review retention settings in `prometheus.yml`

**Resolution**:
- Reduce retention period (default: 30 days)
- Increase scrape interval for less frequent metrics
- Clean up old data: `docker-compose -f docker-compose.monitoring.yml down -v`

### Missing Logs in Loki

**Symptoms**: Log panels in Grafana show no data

**Checks**:
1. Verify Promtail is running:
   ```bash
   docker logs ggen-mcp-promtail
   ```
2. Check log file paths in `/promtail/config.yml`
3. Verify Loki is accepting logs:
   ```bash
   curl http://localhost:3100/ready
   ```

**Resolution**:
- Ensure log directories are mounted correctly
- Check Promtail configuration and restart if needed

## Maintenance

### Regular Tasks

#### Daily
- Review critical and warning alerts
- Check dashboard for anomalies
- Verify all services are healthy

#### Weekly
- Review SLO compliance
- Analyze slow queries
- Check disk space usage

#### Monthly
- Review and update alert thresholds
- Optimize retention policies
- Update dashboards based on feedback
- Review and archive old alerts

### Backup

#### Prometheus Data
```bash
docker cp ggen-mcp-prometheus:/prometheus ./backup/prometheus-$(date +%Y%m%d)
```

#### Grafana Dashboards
```bash
docker cp ggen-mcp-grafana:/var/lib/grafana ./backup/grafana-$(date +%Y%m%d)
```

#### Alertmanager Configuration
```bash
cp alertmanager/config.yml ./backup/alertmanager-config-$(date +%Y%m%d).yml
```

### Upgrading

1. **Backup current configuration**
2. **Update Docker images** in `docker-compose.monitoring.yml`
3. **Test in staging environment**
4. **Apply to production**:
   ```bash
   ./scripts/stop-monitoring.sh
   docker-compose -f docker-compose.monitoring.yml pull
   ./scripts/start-monitoring.sh
   ```
5. **Verify all services are working**

### Cleanup

#### Remove Old Data
```bash
# Remove data older than 7 days
docker exec ggen-mcp-prometheus \
  promtool tsdb prune --start=7d /prometheus
```

#### Compact Prometheus Database
```bash
docker exec ggen-mcp-prometheus \
  promtool tsdb compact /prometheus
```

## Best Practices

1. **Set meaningful alert thresholds** based on actual production behavior
2. **Use runbooks** for all critical alerts
3. **Tag alerts** with team ownership and severity
4. **Test alerts regularly** using load testing
5. **Monitor the monitors** - ensure monitoring stack is healthy
6. **Document all changes** to alerts and dashboards
7. **Review metrics regularly** to identify new monitoring needs
8. **Keep retention policies** aligned with compliance requirements

## Additional Resources

- [Service Level Objectives](./SERVICE_LEVEL_OBJECTIVES.md)
- [Incident Response Runbook](./INCIDENT_RESPONSE_RUNBOOK.md)
- [Prometheus Queries Examples](./PROMETHEUS_QUERIES.md)
- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Documentation](https://grafana.com/docs/)
- [Alertmanager Documentation](https://prometheus.io/docs/alerting/latest/alertmanager/)

## Support

For issues or questions:
- Create an issue in the project repository
- Contact the platform team
- Consult the incident response runbook
