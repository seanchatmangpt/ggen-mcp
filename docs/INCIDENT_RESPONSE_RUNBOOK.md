# GGEN MCP Incident Response Runbook

## Overview

This runbook provides step-by-step procedures for responding to incidents with the GGEN MCP server. Follow these procedures to quickly diagnose and resolve issues.

## Table of Contents

1. [General Response Procedures](#general-response-procedures)
2. [Service Down](#service-down)
3. [High Error Rate](#high-error-rate)
4. [High Latency](#high-latency)
5. [Memory Issues](#memory-issues)
6. [Cache Issues](#cache-issues)
7. [Resource Leaks](#resource-leaks)
8. [Disk Space Issues](#disk-space-issues)
9. [Common Commands](#common-commands)

---

## General Response Procedures

### Incident Response Steps

1. **Acknowledge**: Acknowledge the alert in PagerDuty/Alertmanager
2. **Assess**: Determine severity and impact
3. **Communicate**: Post in incident channel
4. **Investigate**: Use dashboards and logs to diagnose
5. **Mitigate**: Apply temporary fix if needed
6. **Resolve**: Implement permanent solution
7. **Document**: Record incident details
8. **Follow-up**: Schedule post-mortem if needed

### Communication Template

```
🚨 INCIDENT: [Brief Description]
Severity: [Critical/High/Medium/Low]
Start Time: [Timestamp]
Impact: [User-facing impact]
Status: [Investigating/Mitigating/Resolved]
Next Update: [Time]
```

### Essential Links

- Grafana Dashboard: http://localhost:3000/d/ggen-mcp-prod
- Prometheus: http://localhost:9090
- Alertmanager: http://localhost:9093
- Logs: http://localhost:3000/explore (Loki)
- Traces: http://localhost:16686 (Jaeger)

---

## Service Down

### Alert: `GgenMcpServiceDown`

**Severity**: Critical
**Description**: Service is not responding to health checks for 1 minute

### Investigation Steps

1. **Check if service is running**:
   ```bash
   docker ps | grep ggen-mcp
   # or
   ps aux | grep ggen-mcp
   ```

2. **Check service logs**:
   ```bash
   docker logs --tail 100 ggen-mcp-server
   # or
   journalctl -u ggen-mcp -n 100
   ```

3. **Check system resources**:
   ```bash
   top
   free -h
   df -h
   ```

4. **Check network connectivity**:
   ```bash
   curl http://localhost:9464/health
   netstat -tlnp | grep 9464
   ```

### Common Causes

#### 1. Process Crashed

**Symptoms**: Process not in process list, recent error in logs

**Resolution**:
```bash
# Restart the service
docker restart ggen-mcp-server
# or
systemctl restart ggen-mcp

# Verify it's running
curl http://localhost:9464/health
```

#### 2. Port Already in Use

**Symptoms**: "EADDRINUSE" error in logs

**Resolution**:
```bash
# Find process using port
lsof -i :9464
# or
netstat -tlnp | grep 9464

# Kill the conflicting process
kill -9 <PID>

# Restart service
docker restart ggen-mcp-server
```

#### 3. Out of Memory (OOM Killed)

**Symptoms**: "Killed" in logs, dmesg shows OOM

**Resolution**:
```bash
# Check OOM events
dmesg | grep -i "out of memory"
dmesg | grep -i "killed process"

# Increase memory limits (docker-compose.yml)
mem_limit: 4g
mem_reservation: 2g

# Restart with new limits
docker-compose up -d
```

#### 4. Dependency Unavailable

**Symptoms**: Connection errors in logs (database, Redis, etc.)

**Resolution**:
```bash
# Check dependent services
docker ps -a
systemctl status redis
systemctl status postgresql

# Restart dependent services
docker-compose restart redis
systemctl restart postgresql

# Then restart GGEN MCP
docker restart ggen-mcp-server
```

### Post-Resolution

- Verify service is healthy for 5+ minutes
- Check error rate and latency have normalized
- Review logs for any anomalies
- Document root cause and resolution

---

## High Error Rate

### Alert: `GgenMcpHighErrorRate`

**Severity**: Critical (>5%), Warning (>1%)
**Description**: Error rate exceeds threshold for sustained period

### Investigation Steps

1. **Check error types**:
   ```bash
   # View error distribution
   curl -s 'http://localhost:9090/api/v1/query?query=sum by (error_type) (rate(ggen_mcp_errors_total[5m]))' | jq
   ```

2. **View recent errors in logs**:
   ```bash
   docker logs --tail 100 ggen-mcp-server | grep -i error
   ```

3. **Check which tools are failing**:
   ```bash
   # View errors by tool
   curl -s 'http://localhost:9090/api/v1/query?query=sum by (tool) (rate(ggen_mcp_errors_total[5m]))' | jq
   ```

### Common Causes

#### 1. Invalid Input Data

**Symptoms**: Validation errors, parse errors

**Resolution**:
```bash
# Check recent requests
docker logs ggen-mcp-server | grep -A5 "validation error"

# Identify problematic clients
# Review client implementations
# Add more validation with better error messages
```

#### 2. Database Connection Issues

**Symptoms**: Connection timeout, connection refused

**Resolution**:
```bash
# Check database connectivity
psql -h localhost -U ggen_mcp -c "SELECT 1"

# Check connection pool
# View pool stats in metrics
curl http://localhost:9464/metrics | grep pool

# Increase connection pool size if needed
# Restart database if connections are stuck
```

#### 3. Third-Party API Failures

**Symptoms**: HTTP 500/503 errors, timeouts

**Resolution**:
```bash
# Check external API status
curl -I https://external-api.example.com/health

# Implement circuit breaker if not present
# Add retry logic with exponential backoff
# Cache responses where appropriate
```

#### 4. LibreOffice Process Issues

**Symptoms**: "Failed to start LibreOffice" errors

**Resolution**:
```bash
# Check LibreOffice processes
ps aux | grep soffice

# Kill hung processes
pkill -9 soffice

# Check available file descriptors
ulimit -n

# Increase limits if needed
ulimit -n 4096
```

### Post-Resolution

- Monitor error rate for 15+ minutes
- Verify error types have returned to normal
- Add more specific error handling if needed
- Consider adding circuit breakers

---

## High Latency

### Alert: `GgenMcpHighLatency`

**Severity**: Critical (P95 >10s), Warning (P95 >5s)
**Description**: Request latency exceeds acceptable thresholds

### Investigation Steps

1. **Check latency by tool**:
   ```bash
   curl -s 'http://localhost:9090/api/v1/query?query=histogram_quantile(0.95, rate(ggen_mcp_request_duration_seconds_bucket[5m])) by (tool)' | jq
   ```

2. **Check system load**:
   ```bash
   uptime
   top
   iostat -x 1 5
   ```

3. **Check slow queries**:
   ```bash
   docker logs ggen-mcp-server | grep "slow query"
   ```

### Common Causes

#### 1. CPU Saturation

**Symptoms**: High CPU usage (>80%)

**Resolution**:
```bash
# Check CPU usage
top
mpstat -P ALL 1 5

# Identify CPU-intensive processes
ps aux --sort=-%cpu | head -10

# Scale horizontally if possible
# Add more instances behind load balancer

# Optimize hot paths in code
# Profile with node --prof
```

#### 2. Memory Pressure

**Symptoms**: High memory usage, swap usage

**Resolution**:
```bash
# Check memory usage
free -h
vmstat 1 5

# Check for memory leaks
docker stats ggen-mcp-server

# Restart service to clear memory
docker restart ggen-mcp-server

# Increase memory limits
# Investigate and fix memory leaks
```

#### 3. Slow Database Queries

**Symptoms**: Database query metrics show high duration

**Resolution**:
```bash
# Check slow query log
tail -f /var/log/postgresql/postgresql.log | grep "duration"

# Identify slow queries
psql -c "SELECT query, mean_exec_time FROM pg_stat_statements ORDER BY mean_exec_time DESC LIMIT 10"

# Add indexes
# Optimize queries
# Consider caching results
```

#### 4. Cache Thrashing

**Symptoms**: Low cache hit rate, high eviction rate

**Resolution**:
```bash
# Check cache metrics
curl http://localhost:9464/metrics | grep cache

# Increase cache size in config
CACHE_MAX_SIZE=1000  # Increase from default

# Improve cache key strategy
# Pre-warm cache for common requests
# Adjust TTL values
```

#### 5. LibreOffice Bottleneck

**Symptoms**: High screenshot/recalc duration

**Resolution**:
```bash
# Check LibreOffice process count
ps aux | grep soffice | wc -l

# Increase max processes in config
MAX_LIBREOFFICE_PROCESSES=5  # Increase from default

# Use process pooling
# Optimize workbook complexity
# Add timeout limits
```

### Post-Resolution

- Monitor latency percentiles for 15+ minutes
- Verify all tools have acceptable latency
- Load test to confirm fix under load
- Document optimization applied

---

## Memory Issues

### Alert: `GgenMcpMemoryCritical` / `GgenMcpElevatedMemoryUsage`

**Severity**: Critical (>90%), Warning (>70%)
**Description**: Memory usage exceeds safe thresholds

### Investigation Steps

1. **Check memory usage breakdown**:
   ```bash
   # Process memory
   ps aux --sort=-%mem | head -10

   # Docker stats
   docker stats --no-stream

   # Node.js heap
   curl http://localhost:9464/metrics | grep heap
   ```

2. **Check for memory leaks**:
   ```bash
   # Watch memory over time
   watch -n 5 'docker stats --no-stream ggen-mcp-server'

   # Check heap size trend
   # Look for consistently increasing memory
   ```

3. **Review recent changes**:
   - New features deployed?
   - Traffic increase?
   - Data size increase?

### Common Causes

#### 1. Memory Leak

**Symptoms**: Memory continuously increases, never plateaus

**Resolution**:
```bash
# Take heap snapshot
kill -USR2 <PID>  # If using heapdump

# Analyze with Chrome DevTools
# Identify leaked objects
# Fix memory leak in code

# Temporary mitigation: Restart service
docker restart ggen-mcp-server

# Add memory monitoring alerts
# Implement automatic restart on threshold
```

#### 2. Cache Grown Too Large

**Symptoms**: Cache size metric is very high

**Resolution**:
```bash
# Check cache size
curl http://localhost:9464/metrics | grep cache_size

# Clear cache
curl -X POST http://localhost:9464/admin/cache/clear

# Reduce cache size in config
CACHE_MAX_SIZE=500  # Reduce from current

# Implement LRU eviction
# Add cache TTL
```

#### 3. Large Request Payloads

**Symptoms**: Memory spikes with specific requests

**Resolution**:
```bash
# Add request size limits
MAX_REQUEST_SIZE=10mb

# Stream large files instead of loading into memory
# Implement chunked processing
# Add payload validation
```

#### 4. Too Many LibreOffice Processes

**Symptoms**: Multiple soffice processes consuming memory

**Resolution**:
```bash
# Check process count and memory
ps aux | grep soffice

# Kill old processes
pkill -9 soffice

# Reduce max processes
MAX_LIBREOFFICE_PROCESSES=3

# Implement process pooling with timeouts
# Kill idle processes after timeout
```

### Post-Resolution

- Monitor memory usage for 1+ hour
- Verify memory is stable
- Set up heap profiling for future analysis
- Document root cause and fix

---

## Cache Issues

### Alert: `GgenMcpLowCacheHitRate`

**Severity**: Warning
**Description**: Cache hit rate below 50% for extended period

### Investigation Steps

1. **Check cache metrics**:
   ```bash
   # Hit rate
   curl -s 'http://localhost:9090/api/v1/query?query=ggen_mcp:cache_hit_rate_percentage:5m' | jq

   # Cache operations
   curl http://localhost:9464/metrics | grep cache
   ```

2. **Check cache size and evictions**:
   ```bash
   # Size and evictions
   curl -s 'http://localhost:9090/api/v1/query?query=ggen_mcp_cache_size_workbooks' | jq
   curl -s 'http://localhost:9090/api/v1/query?query=rate(ggen_mcp_cache_evictions_total[5m])' | jq
   ```

3. **Analyze access patterns**:
   ```bash
   # View logs for cache operations
   docker logs ggen-mcp-server | grep cache
   ```

### Common Causes

#### 1. Cache Too Small

**Symptoms**: High eviction rate, size at max

**Resolution**:
```bash
# Increase cache size
CACHE_MAX_SIZE=2000  # Increase

# Monitor impact
curl http://localhost:9464/metrics | grep cache

# Restart service
docker restart ggen-mcp-server
```

#### 2. Cache Keys Not Optimal

**Symptoms**: Different keys for same content

**Resolution**:
```bash
# Review cache key generation logic
# Normalize keys (e.g., sort query parameters)
# Use content-based keys instead of timestamp-based
# Implement cache key versioning
```

#### 3. High Request Variance

**Symptoms**: Each request is unique

**Resolution**:
```bash
# Analyze request patterns
# Identify common patterns to cache
# Implement partial caching
# Use cache warming for common requests
```

#### 4. TTL Too Short

**Symptoms**: Frequent evictions despite space

**Resolution**:
```bash
# Increase TTL
CACHE_TTL=3600  # 1 hour instead of shorter

# Implement tiered TTLs
# Hot data: longer TTL
# Cold data: shorter TTL
```

### Post-Resolution

- Monitor cache hit rate for 1+ hour
- Target: >60% (ideal: >80%)
- Adjust cache parameters as needed
- Document cache strategy changes

---

## Resource Leaks

### LibreOffice Process Leak

**Alert**: `GgenMcpHighLibreOfficeProcessCount`

**Symptoms**: Many soffice processes accumulating

**Resolution**:
```bash
# Count processes
ps aux | grep soffice | wc -l

# Kill all LibreOffice processes
pkill -9 soffice

# Check for zombie processes
ps aux | grep defunct

# Improve process cleanup in code
# Add timeout for operations
# Implement process reaping

# Add monitoring for process lifecycle
```

### Fork Process Leak

**Alert**: `GgenMcpHighActiveForks`

**Symptoms**: Many fork processes not being cleaned up

**Resolution**:
```bash
# Check fork count
ps aux | grep fork

# Ensure proper cleanup
# Check for missing wait() calls
# Add process cleanup on error paths
# Implement fork timeout

# Restart service
docker restart ggen-mcp-server
```

### File Descriptor Leak

**Symptoms**: "Too many open files" errors

**Resolution**:
```bash
# Check open file descriptors
lsof -p <PID> | wc -l

# Check limit
ulimit -n

# Increase limit
ulimit -n 8192

# Fix code to close file descriptors
# Ensure cleanup in finally blocks
# Add file descriptor monitoring
```

---

## Disk Space Issues

### Alert: `GgenMcpHighDiskUsage`

**Severity**: Warning (>80%)
**Description**: Disk space running low

### Investigation Steps

```bash
# Check disk usage
df -h

# Find large directories
du -h -d 1 / | sort -h | tail -20

# Find large files
find / -type f -size +100M -exec ls -lh {} \;
```

### Common Causes

#### 1. Log Files Growing

**Resolution**:
```bash
# Rotate logs
logrotate /etc/logrotate.d/ggen-mcp

# Configure log retention
# Delete old logs
find /var/log/ggen-mcp -name "*.log" -mtime +7 -delete

# Reduce log verbosity
LOG_LEVEL=info  # Change from debug
```

#### 2. Prometheus Data Growing

**Resolution**:
```bash
# Check Prometheus storage
du -sh prometheus_data/

# Reduce retention
# Edit prometheus.yml
retention.time: 15d  # Reduce from 30d

# Restart Prometheus
docker restart ggen-mcp-prometheus
```

#### 3. Temporary Files Not Cleaned

**Resolution**:
```bash
# Find temp files
find /tmp -name "ggen-mcp-*" -mtime +1

# Clean up
find /tmp -name "ggen-mcp-*" -mtime +1 -delete

# Add cleanup to code
# Implement scheduled cleanup job
```

---

## Common Commands

### Service Management

```bash
# Start monitoring stack
./scripts/start-monitoring.sh

# Stop monitoring stack
./scripts/stop-monitoring.sh

# Restart GGEN MCP
docker restart ggen-mcp-server

# View logs
docker logs -f ggen-mcp-server

# View logs (last 100 lines)
docker logs --tail 100 ggen-mcp-server
```

### Health Checks

```bash
# Check service health
curl http://localhost:9464/health

# Check metrics endpoint
curl http://localhost:9464/metrics

# Query Prometheus
curl 'http://localhost:9090/api/v1/query?query=up{job="ggen-mcp"}'
```

### Debugging

```bash
# Check running processes
ps aux | grep ggen-mcp

# Check ports
netstat -tlnp | grep 9464

# Check resource usage
docker stats ggen-mcp-server

# Interactive shell in container
docker exec -it ggen-mcp-server /bin/bash

# View detailed metrics
curl http://localhost:9464/metrics | less
```

### Cache Management

```bash
# View cache metrics
curl http://localhost:9464/metrics | grep cache

# Clear cache (if endpoint exists)
curl -X POST http://localhost:9464/admin/cache/clear

# View cache hit rate
curl 'http://localhost:9090/api/v1/query?query=ggen_mcp:cache_hit_rate_percentage:5m'
```

### Load Testing

```bash
# Run load test
./scripts/load-test.sh

# Generate normal traffic
./scripts/load-test.sh normal

# Generate high load
./scripts/load-test.sh high

# Simulate errors
./scripts/load-test.sh errors
```

---

## Escalation

### When to Escalate

- Incident persists > 30 minutes
- Multiple services affected
- Data loss or corruption suspected
- Security incident suspected
- Unknown root cause after investigation

### Escalation Contacts

- Platform Team Lead: [Contact]
- Engineering Manager: [Contact]
- CTO: [Contact]
- Security Team: [Contact]

### Incident Documentation

Create incident report with:
- Incident timeline
- Root cause analysis
- Impact assessment
- Resolution steps
- Preventive measures
- Follow-up actions

---

## Post-Incident

### Post-Mortem Template

1. **Incident Summary**
   - What happened
   - When it happened
   - Duration and impact

2. **Root Cause**
   - Technical root cause
   - Contributing factors

3. **Resolution**
   - How it was fixed
   - Why the fix worked

4. **Lessons Learned**
   - What went well
   - What could be improved

5. **Action Items**
   - Preventive measures
   - Monitoring improvements
   - Process changes
   - Assigned owners and deadlines

### Continuous Improvement

- Update runbooks based on incidents
- Add new alerts for gaps discovered
- Improve monitoring coverage
- Enhance documentation
- Share learnings with team
