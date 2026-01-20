# Health Check Endpoints

This document describes the health check endpoints available in the Spreadsheet MCP server for production deployment monitoring and orchestration.

## Overview

The Spreadsheet MCP server provides three health check endpoints for different purposes:

- `/health` - Liveness probe (basic server health)
- `/ready` - Readiness probe (ready to accept requests)
- `/health/components` - Detailed component-level health status

These endpoints are designed for integration with container orchestration platforms like Kubernetes, Docker, and cloud load balancers.

## Endpoints

### 1. Liveness Probe: `/health`

**Purpose:** Determines if the server process is alive and running.

**Method:** `GET`

**Response Codes:**
- `200 OK` - Server is alive
- `503 Service Unavailable` - Server is not functioning (rare, usually means process crash)

**Response Body:**
```json
{
  "status": "healthy",
  "timestamp": 1704067200,
  "version": "0.1.0"
}
```

**Fields:**
- `status` - Always "healthy" if server responds
- `timestamp` - Unix timestamp of the check
- `version` - Server version string

**Use Case:**
- Kubernetes liveness probes
- Docker HEALTHCHECK
- Process monitoring

**Example:**
```bash
curl http://localhost:8079/health
```

### 2. Readiness Probe: `/ready`

**Purpose:** Determines if the server is ready to accept and process requests.

**Method:** `GET`

**Response Codes:**
- `200 OK` - Server is ready to accept requests
- `503 Service Unavailable` - Server is not ready (dependencies unhealthy)

**Response Body (Ready):**
```json
{
  "ready": true,
  "status": "healthy",
  "timestamp": 1704067200,
  "not_ready": []
}
```

**Response Body (Not Ready):**
```json
{
  "ready": false,
  "status": "unhealthy",
  "timestamp": 1704067200,
  "not_ready": ["workspace", "libreoffice"]
}
```

**Fields:**
- `ready` - Boolean indicating overall readiness
- `status` - Overall health status: "healthy", "degraded", or "unhealthy"
- `timestamp` - Unix timestamp of the check
- `not_ready` - Array of component names that are unhealthy (only present if not empty)

**Use Case:**
- Kubernetes readiness probes
- Load balancer health checks
- Traffic routing decisions

**Example:**
```bash
curl http://localhost:8079/ready
```

### 3. Component Health: `/health/components`

**Purpose:** Provides detailed health information for all system components.

**Method:** `GET`

**Response Codes:**
- `200 OK` - All components healthy or degraded
- `503 Service Unavailable` - One or more components unhealthy

**Response Body:**
```json
{
  "status": "healthy",
  "timestamp": 1704067200,
  "components": {
    "workspace": {
      "component": "workspace",
      "status": "healthy",
      "timestamp": 1704067200,
      "details": {
        "path": "/data",
        "readable": true
      }
    },
    "cache": {
      "component": "cache",
      "status": "healthy",
      "timestamp": 1704067200,
      "details": {
        "size": 3,
        "capacity": 5,
        "capacity_usage_pct": 60,
        "operations": 150,
        "hits": 120,
        "misses": 30,
        "hit_rate_pct": 80
      }
    },
    "workbook_index": {
      "component": "workbook_index",
      "status": "healthy",
      "timestamp": 1704067200,
      "details": {
        "workbook_count": 42,
        "available": true
      }
    },
    "libreoffice": {
      "component": "libreoffice",
      "status": "healthy",
      "timestamp": 1704067200,
      "details": {
        "available": true,
        "version": "LibreOffice 7.6.4.1"
      }
    },
    "fork_registry": {
      "component": "fork_registry",
      "status": "healthy",
      "timestamp": 1704067200,
      "details": {
        "active_forks": 2,
        "available": true
      }
    }
  }
}
```

**Fields:**
- `status` - Overall health status (worst status among all components)
- `timestamp` - Unix timestamp of the check
- `components` - Map of component names to their health status

**Component Health Fields:**
- `component` - Component name
- `status` - Component status: "healthy", "degraded", or "unhealthy"
- `timestamp` - Unix timestamp of the component check
- `error` - Error message (only present if degraded or unhealthy)
- `details` - Additional component-specific information (optional)

**Use Case:**
- Detailed diagnostics
- Monitoring dashboards
- Alerting systems
- Troubleshooting

**Example:**
```bash
curl http://localhost:8079/health/components
```

## Health Status Types

### Healthy
Component is functioning normally with no issues.

### Degraded
Component is functioning but with reduced performance or partial failures. The server can still process requests but may have limitations.

**Examples:**
- Cache is 95%+ full
- Some workbooks failed to index
- Non-critical errors

### Unhealthy
Component is not functioning. The server may not be able to process requests properly.

**Examples:**
- Workspace directory not accessible
- LibreOffice not available (when recalc enabled)
- Fork registry initialization failed

## Component Checks

### Workspace
Checks if the workspace directory exists, is a directory, and is readable.

**Healthy:** Directory exists and is accessible
**Unhealthy:** Directory missing, not a directory, or not readable

### Cache
Monitors the workbook cache status and capacity.

**Healthy:** Cache operating normally (< 95% full)
**Degraded:** Cache is 95%+ full (may cause evictions)

**Details:**
- Current size and capacity
- Cache hit/miss statistics
- Hit rate percentage

### Workbook Index
Verifies that the workbook indexing system is working.

**Healthy:** Can list workbooks successfully
**Unhealthy:** Cannot list workbooks (filesystem or permission issues)

### LibreOffice (Recalc Feature Only)
Checks if LibreOffice is available for formula recalculation.

**Healthy:** soffice command available and working
**Unhealthy:** soffice not found or not executable

This check is only performed when `recalc_enabled=true`.

### Fork Registry (Recalc Feature Only)
Verifies the fork registry is initialized and available.

**Healthy:** Registry initialized and operational
**Unhealthy:** Registry failed to initialize

This check is only performed when `recalc_enabled=true`.

## Integration Guides

### Kubernetes

#### Liveness Probe
```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 8079
  initialDelaySeconds: 5
  periodSeconds: 30
  timeoutSeconds: 3
  failureThreshold: 3
```

#### Readiness Probe
```yaml
readinessProbe:
  httpGet:
    path: /ready
    port: 8079
  initialDelaySeconds: 10
  periodSeconds: 10
  timeoutSeconds: 5
  failureThreshold: 3
```

#### Complete Example
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: spreadsheet-mcp
spec:
  replicas: 3
  selector:
    matchLabels:
      app: spreadsheet-mcp
  template:
    metadata:
      labels:
        app: spreadsheet-mcp
    spec:
      containers:
      - name: spreadsheet-mcp
        image: ghcr.io/psu3d0/spreadsheet-mcp:latest
        ports:
        - containerPort: 8079
          name: http
        env:
        - name: SPREADSHEET_MCP_WORKSPACE
          value: /data
        - name: SPREADSHEET_MCP_RECALC_ENABLED
          value: "true"
        volumeMounts:
        - name: data
          mountPath: /data
        livenessProbe:
          httpGet:
            path: /health
            port: 8079
          initialDelaySeconds: 5
          periodSeconds: 30
          timeoutSeconds: 3
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /ready
            port: 8079
          initialDelaySeconds: 10
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "1Gi"
            cpu: "1000m"
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: spreadsheet-data
```

### Docker Compose

```yaml
version: '3.8'

services:
  spreadsheet-mcp:
    image: ghcr.io/psu3d0/spreadsheet-mcp:full
    ports:
      - "8079:8079"
    volumes:
      - ./data:/data
    environment:
      - SPREADSHEET_MCP_WORKSPACE=/data
      - SPREADSHEET_MCP_RECALC_ENABLED=true
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8079/health"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s
    restart: unless-stopped
```

### Docker HEALTHCHECK

The Dockerfiles include built-in health checks:

**Dockerfile (minimal image):**
```dockerfile
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/spreadsheet-mcp", "--version"] || exit 1
```

**Dockerfile.full (with LibreOffice):**
```dockerfile
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8079/health || exit 1
```

### AWS Application Load Balancer

```json
{
  "HealthCheckEnabled": true,
  "HealthCheckPath": "/ready",
  "HealthCheckProtocol": "HTTP",
  "HealthCheckIntervalSeconds": 30,
  "HealthCheckTimeoutSeconds": 5,
  "HealthyThresholdCount": 2,
  "UnhealthyThresholdCount": 3,
  "Matcher": {
    "HttpCode": "200"
  }
}
```

### Prometheus Monitoring

You can create custom Prometheus metrics from the health endpoints:

```yaml
- job_name: 'spreadsheet-mcp-health'
  metrics_path: '/health/components'
  static_configs:
    - targets: ['localhost:8079']
  metric_relabel_configs:
    - source_labels: [__name__]
      regex: 'spreadsheet_mcp_.*'
      action: keep
```

Example queries:
```promql
# Cache usage percentage
spreadsheet_mcp_cache_usage_pct

# Cache hit rate
spreadsheet_mcp_cache_hit_rate_pct

# Component health (1=healthy, 0.5=degraded, 0=unhealthy)
spreadsheet_mcp_component_health{component="workspace"}
```

## Best Practices

### 1. Use Appropriate Probes

- **Liveness:** Use `/health` for liveness probes (restarts unhealthy pods)
- **Readiness:** Use `/ready` for readiness probes (removes pods from load balancer)
- **Never** use `/health/components` for probes (too expensive)

### 2. Configure Timeouts Properly

- Liveness: Short timeout (3-5s), longer period (30s+)
- Readiness: Moderate timeout (5s), shorter period (10s)
- Start period: Allow time for initialization (10-30s for full image)

### 3. Set Appropriate Thresholds

- Failure threshold: 3-5 consecutive failures before marking unhealthy
- Success threshold: 1-2 consecutive successes before marking healthy

### 4. Monitor Component Health

- Use `/health/components` for monitoring dashboards
- Set up alerts for degraded components
- Track cache hit rates and capacity usage

### 5. Handle Degraded State

Degraded components indicate potential issues:
- Monitor cache usage and increase capacity if frequently degraded
- Check disk space if workspace checks show issues
- Verify LibreOffice availability in production

### 6. Logging and Debugging

Health check failures are logged with context:
```
[WARN] health check failed: component=workspace error="directory not readable"
```

Use component health endpoint for detailed diagnostics:
```bash
curl http://localhost:8079/health/components | jq '.components.workspace'
```

### 7. Testing Health Checks

Test health checks in your CI/CD pipeline:
```bash
#!/bin/bash
set -e

# Start server
docker-compose up -d

# Wait for server to be ready
timeout 30s bash -c 'until curl -f http://localhost:8079/ready; do sleep 1; done'

# Verify all components healthy
response=$(curl -s http://localhost:8079/health/components)
status=$(echo "$response" | jq -r '.status')

if [ "$status" != "healthy" ]; then
    echo "Health check failed: $response"
    exit 1
fi

echo "All health checks passed"
```

## Troubleshooting

### Liveness Probe Failures

**Symptom:** Pod/container repeatedly restarting

**Possible Causes:**
1. Server crash or panic
2. Timeout too short for server to respond
3. Network issues

**Solutions:**
- Check server logs for errors
- Increase timeout or period
- Verify network connectivity

### Readiness Probe Failures

**Symptom:** Pod/container not receiving traffic

**Possible Causes:**
1. Workspace directory issues
2. LibreOffice not available
3. Initialization taking too long

**Solutions:**
- Check `/health/components` for specific failures
- Verify workspace mount and permissions
- Check LibreOffice installation (full image)
- Increase `initialDelaySeconds` if needed

### Component Health Issues

#### Workspace Unhealthy
- Verify directory exists and is mounted
- Check file permissions
- Verify disk space

#### Cache Degraded
- Increase cache capacity in configuration
- Monitor workbook access patterns
- Consider implementing cache warming

#### LibreOffice Unhealthy
- Verify LibreOffice installation
- Check PATH environment variable
- Test `soffice --version` manually

#### Fork Registry Unhealthy
- Check disk space for fork directory
- Verify write permissions
- Review initialization logs

## Security Considerations

### 1. Authentication

Health check endpoints are **unauthenticated** by design for orchestration platforms. If you need to restrict access:

```yaml
# Kubernetes Network Policy
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: spreadsheet-mcp-health
spec:
  podSelector:
    matchLabels:
      app: spreadsheet-mcp
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: kube-system
    ports:
    - protocol: TCP
      port: 8079
```

### 2. Information Disclosure

The `/health/components` endpoint exposes system information. Consider:
- Restricting access to internal networks only
- Using separate ports for health vs. application endpoints
- Implementing authentication for detailed diagnostics

### 3. Rate Limiting

Health checks can be frequent. Ensure your rate limiting excludes health endpoints:
```rust
// Example middleware configuration
.layer(
    ServiceBuilder::new()
        .layer(rate_limit_layer)
        .layer(middleware::from_fn(exclude_health_from_rate_limit))
)
```

## Performance Considerations

### Health Check Overhead

- `/health`: Near-zero overhead (in-memory check)
- `/ready`: Low overhead (checks critical dependencies)
- `/health/components`: Moderate overhead (multiple system checks)

**Recommendations:**
- Use `/health` for frequent checks (every 10-30s)
- Use `/ready` for moderate frequency (every 30-60s)
- Use `/health/components` for monitoring only (every 5-10m)

### Concurrency

All health check endpoints are thread-safe and support concurrent access. The implementation uses read locks where possible to minimize contention.

## Migration Guide

If you have an existing deployment without health checks:

1. **Add health check endpoints** (already included in latest version)
2. **Update Docker Compose** with healthcheck configuration
3. **Update Kubernetes manifests** with liveness/readiness probes
4. **Monitor health metrics** in your observability platform
5. **Set up alerts** for degraded components

## Related Documentation

- [Production Deployment Guide](RUST_MCP_PRODUCTION_DEPLOYMENT.md)
- [Performance Optimization](PERFORMANCE_QUICK_REFERENCE.md)
- [Error Handling](ERROR_HANDLING_README.md)
- [Deployment Checklist](DEPLOYMENT_CHECKLIST.md)

## Support

For issues or questions about health checks:
- Check component details: `curl http://localhost:8079/health/components | jq`
- Review server logs for health check warnings
- Open an issue on GitHub with health check output
