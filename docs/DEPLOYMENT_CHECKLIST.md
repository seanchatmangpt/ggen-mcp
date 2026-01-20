# Production Deployment Checklist

> **TPS Principle**: Standardized work ensures consistent quality. This checklist implements poka-yoke (mistake-proofing) for production deployments.

## Pre-Deployment Checklist

### 1. Configuration Validation

- [ ] **Configuration Files Reviewed**
  - [ ] All environment variables documented
  - [ ] Config files validated against schema
  - [ ] Secrets not hardcoded in config
  - [ ] Resource limits set appropriately
  - [ ] Timeout values tested under load

- [ ] **Feature Flags Configured**
  - [ ] Production feature flags set correctly
  - [ ] Dangerous features disabled (if applicable)
  - [ ] Gradual rollout percentages configured

- [ ] **Environment-Specific Settings**
  - [ ] `RUST_LOG` set to appropriate level (info/warn)
  - [ ] `RUST_BACKTRACE` configured (0 for prod, 1 for debugging)
  - [ ] Workspace paths verified
  - [ ] File permissions validated

### 2. Security Hardening

- [ ] **Dependency Security**
  - [ ] `cargo audit` passes with no warnings
  - [ ] All dependencies up to date
  - [ ] Known vulnerabilities addressed
  - [ ] RUSTSEC advisories reviewed

- [ ] **Container Security**
  - [ ] Running as non-root user (UID > 1000)
  - [ ] Minimal base image (distroless/alpine)
  - [ ] No unnecessary capabilities
  - [ ] Read-only root filesystem where possible
  - [ ] Security scanning passed (Trivy/Snyk)

- [ ] **Secret Management**
  - [ ] Secrets loaded from secure vault (not env vars)
  - [ ] API keys rotated
  - [ ] No secrets in logs
  - [ ] Secrets properly scoped (least privilege)

- [ ] **Network Security**
  - [ ] TLS enabled for all external connections
  - [ ] Certificate validity checked
  - [ ] Firewall rules configured
  - [ ] Only necessary ports exposed

### 3. Observability Setup

- [ ] **Logging**
  - [ ] Structured logging enabled (JSON format)
  - [ ] Log aggregation configured (Loki/ELK)
  - [ ] Log retention policy set
  - [ ] PII/sensitive data redacted
  - [ ] Log levels appropriate for production

- [ ] **Tracing**
  - [ ] Distributed tracing enabled (Jaeger/Zipkin)
  - [ ] Trace sampling configured (1-10% for high traffic)
  - [ ] Trace exporter endpoint verified
  - [ ] Parent/child span relationships tested

- [ ] **Metrics**
  - [ ] Prometheus metrics endpoint exposed
  - [ ] Custom business metrics implemented
  - [ ] Metric cardinality reviewed (prevent explosion)
  - [ ] Metric labels validated
  - [ ] Scrape interval configured (15-60s)

### 4. Health Checks

- [ ] **Liveness Probe**
  - [ ] Endpoint responds with 200 when alive
  - [ ] Timeout configured (3-5s)
  - [ ] Failure threshold set (3 failures)
  - [ ] Initial delay configured (10-30s)

- [ ] **Readiness Probe**
  - [ ] Checks all critical dependencies
  - [ ] Returns 503 when not ready
  - [ ] Periodic interval configured (10-30s)
  - [ ] Does not restart container on failure

- [ ] **Startup Probe** (for slow-starting apps)
  - [ ] Allows sufficient startup time
  - [ ] Transitions to liveness after success
  - [ ] Failure threshold accommodates startup time

### 5. Resource Limits

- [ ] **CPU Limits**
  - [ ] Request set based on load testing (e.g., 500m)
  - [ ] Limit set with headroom (e.g., 2000m)
  - [ ] Throttling behavior tested
  - [ ] Burst capacity validated

- [ ] **Memory Limits**
  - [ ] Request set based on baseline (e.g., 512Mi)
  - [ ] Limit set to prevent OOM (e.g., 2Gi)
  - [ ] Memory leak tests passed
  - [ ] OOM behavior tested

- [ ] **Disk Limits**
  - [ ] Persistent volume sized appropriately
  - [ ] Disk I/O limits configured
  - [ ] Ephemeral storage limits set
  - [ ] Cleanup policies configured

- [ ] **File Descriptors**
  - [ ] ulimit configured (soft: 65536, hard: 65536)
  - [ ] Connection pool sizes validated
  - [ ] File handle leaks tested

### 6. Scaling and Capacity

- [ ] **Horizontal Scaling**
  - [ ] Minimum replicas set (e.g., 3 for HA)
  - [ ] Maximum replicas configured
  - [ ] Autoscaling metrics defined (CPU/memory/custom)
  - [ ] Scale-up threshold tested
  - [ ] Scale-down threshold tested

- [ ] **Load Testing**
  - [ ] Baseline performance established
  - [ ] Peak load tested (2-5x expected)
  - [ ] Sustained load tested (24h+)
  - [ ] Error rate under load < 0.1%
  - [ ] Latency SLO met under load

- [ ] **Capacity Planning**
  - [ ] Expected QPS calculated
  - [ ] Resource requirements estimated
  - [ ] Growth projections considered
  - [ ] Burst capacity planned

### 7. Data Persistence

- [ ] **Persistent Volumes**
  - [ ] Storage class configured
  - [ ] Volume size appropriate
  - [ ] Backup strategy implemented
  - [ ] Disaster recovery tested

- [ ] **Caching**
  - [ ] Cache size tuned (ggen-mcp: 50-100 workbooks)
  - [ ] Eviction policy tested (LRU)
  - [ ] Cache hit rate monitored (target: >70%)
  - [ ] Cache warming strategy (if applicable)

- [ ] **State Management**
  - [ ] Stateless where possible
  - [ ] State persistence tested (fork registry)
  - [ ] State recovery tested
  - [ ] Cleanup policies configured

### 8. Graceful Shutdown

- [ ] **Signal Handling**
  - [ ] SIGTERM handler implemented
  - [ ] SIGINT handler implemented
  - [ ] Shutdown timeout configured (30-60s)
  - [ ] Force kill after timeout

- [ ] **Connection Draining**
  - [ ] In-flight requests completed
  - [ ] New connections rejected during shutdown
  - [ ] Connection drain timeout set (30s)

- [ ] **State Cleanup**
  - [ ] Temporary files cleaned up
  - [ ] Active forks saved/discarded
  - [ ] Metrics pushed to pushgateway
  - [ ] Final logs flushed

### 9. Monitoring and Alerting

- [ ] **Dashboards**
  - [ ] Service overview dashboard created
  - [ ] Key metrics visualized (requests, latency, errors)
  - [ ] Resource utilization dashboard
  - [ ] Business metrics dashboard

- [ ] **Alerts Configured**
  - [ ] High error rate alert (>5% over 5min)
  - [ ] High latency alert (p95 > 1s over 5min)
  - [ ] Service down alert (>1min)
  - [ ] Disk space low alert (<10% free)
  - [ ] Memory pressure alert (>80% usage)
  - [ ] Certificate expiry alert (30 days)

- [ ] **Alert Routing**
  - [ ] PagerDuty/Opsgenie integration tested
  - [ ] Alert severity levels configured
  - [ ] Escalation policies defined
  - [ ] On-call schedule configured

- [ ] **Runbooks**
  - [ ] Runbook for each alert type
  - [ ] Runbooks linked in alert annotations
  - [ ] Runbooks tested and up-to-date
  - [ ] Auto-remediation where possible

### 10. Deployment Strategy

- [ ] **Rolling Update**
  - [ ] Max unavailable configured (e.g., 1)
  - [ ] Max surge configured (e.g., 1)
  - [ ] Update strategy tested
  - [ ] Rollback plan documented

- [ ] **Blue-Green Deployment**
  - [ ] Blue environment stable
  - [ ] Green environment deployed
  - [ ] Traffic switching tested
  - [ ] Rollback procedure verified

- [ ] **Canary Deployment**
  - [ ] Canary percentage configured (5-10%)
  - [ ] Canary metrics monitored
  - [ ] Automatic rollback on errors
  - [ ] Full rollout criteria defined

### 11. Backup and Recovery

- [ ] **Backup Strategy**
  - [ ] Automated backups scheduled (daily)
  - [ ] Backup retention policy (30 days)
  - [ ] Off-site backups configured
  - [ ] Backup encryption enabled

- [ ] **Recovery Testing**
  - [ ] Restore from backup tested
  - [ ] RTO (Recovery Time Objective) measured
  - [ ] RPO (Recovery Point Objective) validated
  - [ ] Disaster recovery runbook tested

### 12. Documentation

- [ ] **Operational Documentation**
  - [ ] Architecture diagram updated
  - [ ] Deployment guide written
  - [ ] Troubleshooting guide created
  - [ ] API documentation current

- [ ] **Change Log**
  - [ ] Release notes prepared
  - [ ] Breaking changes documented
  - [ ] Migration guide (if needed)
  - [ ] Known issues listed

### 13. Compliance and Governance

- [ ] **Regulatory Compliance**
  - [ ] Data residency requirements met
  - [ ] Privacy regulations followed (GDPR/CCPA)
  - [ ] Audit trail enabled
  - [ ] Data retention policies enforced

- [ ] **Access Control**
  - [ ] RBAC policies configured
  - [ ] Service accounts created
  - [ ] API keys rotated
  - [ ] Least privilege enforced

### 14. Testing

- [ ] **Functional Testing**
  - [ ] All critical paths tested
  - [ ] Integration tests passed
  - [ ] End-to-end tests passed
  - [ ] Smoke tests after deployment

- [ ] **Performance Testing**
  - [ ] Load tests passed
  - [ ] Stress tests passed
  - [ ] Soak tests passed (24h+)
  - [ ] Baseline metrics established

- [ ] **Chaos Engineering**
  - [ ] Pod failure tested
  - [ ] Network partition tested
  - [ ] Dependency failure tested
  - [ ] Cascading failure prevented

### 15. Post-Deployment

- [ ] **Smoke Tests**
  - [ ] Health checks passing
  - [ ] Key endpoints responding
  - [ ] Sample requests successful
  - [ ] No errors in logs

- [ ] **Monitoring**
  - [ ] Metrics flowing to Prometheus
  - [ ] Logs flowing to aggregator
  - [ ] Traces flowing to backend
  - [ ] Alerts firing correctly

- [ ] **Communication**
  - [ ] Deployment announced to team
  - [ ] Stakeholders notified
  - [ ] On-call engineer briefed
  - [ ] Post-deployment review scheduled

---

## ggen-mcp Specific Checklist

### Configuration

- [ ] `SPREADSHEET_MCP_WORKSPACE` points to valid directory
- [ ] `SPREADSHEET_MCP_CACHE_CAPACITY` tuned (50-100)
- [ ] `SPREADSHEET_MCP_MAX_CONCURRENT_RECALCS` set (2-5)
- [ ] `SPREADSHEET_MCP_TOOL_TIMEOUT_MS` configured (30000-60000)
- [ ] `SPREADSHEET_MCP_MAX_RESPONSE_BYTES` set (1000000)

### LibreOffice (if recalc enabled)

- [ ] LibreOffice installed in container
- [ ] `soffice` binary accessible
- [ ] LibreOffice version tested
- [ ] Headless mode working
- [ ] Font dependencies installed
- [ ] Macro security configured

### Cache Tuning

- [ ] Cache hit rate monitored (target: >70%)
- [ ] Cache size vs workbook count analyzed
- [ ] Eviction behavior tested
- [ ] Memory usage under cache load validated

### Fork Management

- [ ] Fork cleanup interval configured (1 hour)
- [ ] Max fork age tested
- [ ] Fork disk space monitored
- [ ] Concurrent fork limit tested

### File System

- [ ] Workspace directory permissions (read)
- [ ] Screenshots directory writable
- [ ] Fork directory writable
- [ ] Temp directory cleanup configured

---

## Approval Sign-offs

### Development Team

- [ ] Code review completed
- [ ] Tests passed
- [ ] Documentation updated
- [ ] Signed off by: _________________ Date: _______

### Operations Team

- [ ] Infrastructure ready
- [ ] Monitoring configured
- [ ] Runbooks prepared
- [ ] Signed off by: _________________ Date: _______

### Security Team

- [ ] Security scan passed
- [ ] Compliance verified
- [ ] Access controls reviewed
- [ ] Signed off by: _________________ Date: _______

### Product Owner

- [ ] Feature validation complete
- [ ] Acceptance criteria met
- [ ] Release notes approved
- [ ] Signed off by: _________________ Date: _______

---

## Emergency Rollback Plan

If issues are detected post-deployment:

1. **Immediate Actions** (0-5 minutes)
   - [ ] Stop new deployments
   - [ ] Alert on-call engineer
   - [ ] Assess severity

2. **Rollback Decision** (5-10 minutes)
   - [ ] Determine if rollback needed
   - [ ] Get approval from incident commander
   - [ ] Communicate rollback to team

3. **Execute Rollback** (10-15 minutes)
   - [ ] Revert to previous version
   - [ ] Verify rollback successful
   - [ ] Monitor for stability

4. **Post-Rollback** (15-30 minutes)
   - [ ] Confirm service restored
   - [ ] Document incident
   - [ ] Schedule post-mortem

---

## TPS Poka-Yoke (Mistake-Proofing)

This checklist implements several error-proofing mechanisms:

1. **Prevention**: Steps that prevent errors before they occur
2. **Detection**: Steps that catch errors early
3. **Mitigation**: Steps that reduce impact of errors

**Remember**: If you can't check off an item, don't deploy. Investigate why and resolve the issue first.

**Gemba**: Use production metrics as your "go and see" to verify deployment success.

---

## Deployment Command Reference

### Docker Deployment

```bash
# Build
docker build -t spreadsheet-mcp:latest -f Dockerfile.full .

# Run with health check
docker run -d \
  --name spreadsheet-mcp \
  --health-cmd="curl -f http://localhost:8079/health/live || exit 1" \
  --health-interval=30s \
  --health-timeout=3s \
  --health-retries=3 \
  -p 8079:8079 \
  -p 9090:9090 \
  -v $(pwd)/data:/data \
  -e SPREADSHEET_MCP_CACHE_CAPACITY=50 \
  -e RUST_LOG=info \
  spreadsheet-mcp:latest
```

### Kubernetes Deployment

```bash
# Apply configuration
kubectl apply -f kubernetes/

# Check rollout status
kubectl rollout status deployment/spreadsheet-mcp

# Verify health
kubectl get pods -l app=spreadsheet-mcp
kubectl logs -l app=spreadsheet-mcp --tail=100

# Check metrics
kubectl port-forward svc/spreadsheet-mcp 9090:9090
curl http://localhost:9090/metrics
```

### Health Check Verification

```bash
# Liveness
curl -f http://localhost:8079/health/live

# Readiness
curl -f http://localhost:8079/health/ready

# Metrics
curl http://localhost:9090/metrics | grep spreadsheet_mcp
```

---

## Version History

| Version | Date | Changes | Author |
|---------|------|---------|--------|
| 1.0 | 2026-01-20 | Initial checklist | System |

---

## References

- [Production Deployment Guide](./RUST_MCP_PRODUCTION_DEPLOYMENT.md)
- [TPS Poka-Yoke](./POKA_YOKE_PATTERN.md)
- [Kubernetes Best Practices](https://kubernetes.io/docs/concepts/configuration/overview/)
