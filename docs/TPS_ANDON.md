# TPS Andon: Visual Management and Alerts for MCP Servers

## Overview

This document describes how **Andon** principles from the Toyota Production System (TPS) apply to MCP (Model Context Protocol) servers. Andon (アンドン, Japanese for "lantern") is a visual management system that makes problems immediately visible and enables rapid intervention when issues occur.

In manufacturing, Andon cords allow any worker to stop the production line when they detect a problem. In MCP servers, Andon principles ensure that operational issues are immediately visible, actionable, and traceable.

## What is Andon?

Andon is a principle of jidoka (autonomation) that encompasses:

1. **Visual Management**: Make status and problems immediately visible
2. **Stop-the-Line Authority**: Empower systems to signal when intervention is needed
3. **Immediate Response**: Enable rapid problem detection and resolution
4. **Root Cause Analysis**: Provide data to understand and prevent future issues
5. **Continuous Improvement**: Use visibility to drive ongoing optimization

### Core Andon Principles for MCP Servers

| Principle | Manufacturing | MCP Server Application |
|-----------|--------------|------------------------|
| **Visual Signals** | Lights on production line | Health check endpoints, status dashboards, log levels |
| **Stop the Line** | Worker pulls cord | Circuit breakers, rate limiting, graceful degradation |
| **Problem Escalation** | Alert supervisor | Logging levels (info → warn → error), alert routing |
| **Work-in-Progress** | Kanban boards | Active requests, queue depths, fork registry |
| **Capacity Indicators** | Machine utilization | Cache stats, semaphore counts, resource metrics |

## Current Andon Implementation in ggen-mcp

### 1. Audit Trail System

**Location**: `/home/user/ggen-mcp/src/audit/mod.rs`

The audit system provides comprehensive event tracking with three outcomes:

```rust
pub enum AuditOutcome {
    Success,   // ✓ Green light - operation completed successfully
    Failure,   // ✗ Red light - operation failed, intervention may be needed
    Partial,   // ⚠ Yellow light - partial success, review recommended
}
```

**Andon Characteristics**:
- **Visual Categorization**: Events are color-coded by outcome (success/failure/partial)
- **Persistent Trail**: Events are logged to rotating files (audit-{timestamp}.jsonl)
- **In-Memory Buffer**: Recent 10,000 events available for quick queries
- **Structured Logging**: Events include event_id, timestamp, resource, duration_ms, error details

**Current Event Types**:
- Tool invocations
- Fork lifecycle (create, edit, recalc, save, discard)
- Checkpoint operations
- Staged change operations
- File operations
- Workbook operations
- Error events

**Example Usage**:

```rust
use crate::audit::{AuditEvent, AuditEventType, AuditOutcome};

// Log a successful operation
let event = AuditEvent::new(AuditEventType::ForkCreate)
    .with_resource("fork-abc123")
    .with_outcome(AuditOutcome::Success)
    .with_duration_ms(150);

audit_event(event);

// Log a failure that requires attention
let event = AuditEvent::new(AuditEventType::ForkRecalc)
    .with_resource("fork-abc123")
    .with_error("LibreOffice timeout after 30s")
    .with_duration_ms(30000);

audit_event(event); // Automatically sets outcome to Failure
```

### 2. Cache Statistics (Capacity Indicators)

**Location**: `/home/user/ggen-mcp/src/state.rs`

The workbook cache tracks operational metrics for capacity planning:

```rust
pub struct CacheStats {
    pub operations: u64,      // Total cache operations
    pub hits: u64,            // Successful cache lookups
    pub misses: u64,          // Cache misses requiring load
    pub size: usize,          // Current cache entries
    pub capacity: usize,      // Max cache capacity
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.operations == 0 {
            0.0
        } else {
            self.hits as f64 / self.operations as f64
        }
    }
}
```

**Andon Characteristics**:
- **Capacity Visualization**: Current size vs. capacity shows resource utilization
- **Performance Indicators**: Hit rate indicates cache effectiveness
- **Real-time Metrics**: Atomic counters provide instant statistics

**Interpreting Metrics**:

| Metric | Healthy Range | Warning Signs | Action Required |
|--------|---------------|---------------|-----------------|
| **Hit Rate** | > 70% | 50-70% | < 50% - consider increasing cache capacity |
| **Size/Capacity** | < 80% | 80-95% | > 95% - increase capacity or evict stale entries |
| **Operations** | Steady growth | Sudden spikes | Investigate abnormal access patterns |

### 3. Circuit Breaker Pattern (Stop-the-Line)

**Location**: `/home/user/ggen-mcp/src/recovery/circuit_breaker.rs`

The circuit breaker implements Andon's "stop-the-line" principle:

```rust
pub enum CircuitBreakerState {
    Closed,    // ✓ Green - normal operation
    HalfOpen,  // ⚠ Yellow - testing recovery
    Open,      // ✗ Red - failing fast, preventing cascade
}
```

**Configuration Presets**:

```rust
// For LibreOffice recalc operations
CircuitBreakerConfig::recalc() {
    failure_threshold: 3,      // Open after 3 failures
    success_threshold: 2,      // Close after 2 successes in half-open
    timeout: Duration::from_secs(30),
    failure_window: Duration::from_secs(60),
}

// For file operations
CircuitBreakerConfig::file_io() {
    failure_threshold: 5,
    success_threshold: 3,
    timeout: Duration::from_secs(15),
    failure_window: Duration::from_secs(60),
}
```

**Andon Application**:
- **Visual State**: Three clear states (Closed/HalfOpen/Open)
- **Automatic Protection**: Prevents cascade failures by failing fast
- **Self-Recovery**: Transitions to HalfOpen after timeout, tests recovery
- **Manual Override**: `reset()` method for operator intervention

### 4. Recovery System (Error Visibility)

**Location**: `/home/user/ggen-mcp/src/recovery/mod.rs`

The recovery system categorizes errors and determines appropriate responses:

```rust
pub enum RecoveryStrategy {
    Retry,          // Transient issue - try again with backoff
    Fallback,       // Use alternative approach
    PartialSuccess, // Continue with what succeeded
    Fail,           // Stop and report error
}
```

**Error Classification**:

| Error Pattern | Recovery Strategy | Andon Signal | Escalation |
|---------------|-------------------|--------------|------------|
| Timeout, "timed out" | Retry | ⚠ Yellow | Warn after 3 retries |
| "not found", "corrupted", "invalid" | Fallback | ⚠ Yellow | Error if fallback fails |
| "too many", "resource", "unavailable" | Retry + Backoff | ⚠ Yellow | Error if exhausted |
| "batch", "some operations failed" | PartialSuccess | ⚠ Yellow | Warn with details |
| Unknown errors | Fail | ✗ Red | Immediate error |

**Exponential Backoff**:

```rust
// Delays: 100ms → 200ms → 400ms → 800ms
let delay = exponential_backoff(attempt, Duration::from_millis(100));
```

### 5. Structured Logging (Visual Management)

**Location**: Throughout codebase, initialized in `/home/user/ggen-mcp/src/main.rs`

The tracing system provides hierarchical, structured logging:

```rust
tracing::info!(
    event_id = %event.event_id,
    event_type = ?event.event_type,
    resource = ?event.resource,
    duration_ms = ?event.duration_ms,
    "audit event"
);

tracing::error!(
    circuit_breaker = %self.name,
    failure_count = inner.failure_count,
    threshold = self.config.failure_threshold,
    "threshold exceeded, opening circuit"
);
```

**Log Level Hierarchy** (Andon Escalation):

| Level | Color | When to Use | Example |
|-------|-------|-------------|---------|
| **debug** | Gray | Development/diagnostics | "cache hit", "resolved from index" |
| **info** | Blue | Normal operations | "startup scan complete", "audit event" |
| **warn** | Yellow | Degraded but operational | "cache smaller than max_concurrent_recalcs", "retrying operation" |
| **error** | Red | Failure requiring attention | "operation failed", "circuit breaker opened" |

### 6. Configuration Validation (Error-Proofing)

**Location**: `/home/user/ggen-mcp/src/config.rs`

Pre-flight validation implements "poka-yoke" (error-proofing):

```rust
impl ServerConfig {
    pub fn validate(&self) -> Result<()> {
        // 1. Workspace validation
        anyhow::ensure!(self.workspace_root.exists(), "workspace root does not exist");

        // 2. Cache capacity bounds
        anyhow::ensure!(
            self.cache_capacity >= MIN_CACHE_CAPACITY,
            "cache_capacity must be at least {}", MIN_CACHE_CAPACITY
        );

        // 3. Tool timeout bounds
        if let Some(timeout_ms) = self.tool_timeout_ms {
            anyhow::ensure!(
                timeout_ms >= MIN_TOOL_TIMEOUT_MS,
                "tool_timeout_ms must be at least {}ms", MIN_TOOL_TIMEOUT_MS
            );
        }

        // 4. Port privilege warning
        if self.transport == TransportKind::Http {
            let port = self.http_bind_address.port();
            if port < 1024 {
                tracing::warn!(
                    port = port,
                    "HTTP bind port in privileged range (< 1024)"
                );
            }
        }

        Ok(())
    }
}
```

**Validation Categories**:
- **Existence Checks**: Files and directories exist before starting
- **Bounds Validation**: Numeric parameters within safe ranges
- **Permission Warnings**: Potential permission issues flagged early
- **Consistency Checks**: Related parameters validated together

### 7. Resource Semaphores (Work-in-Progress Limits)

**Location**: `/home/user/ggen-mcp/src/state.rs`

Semaphores implement WIP (Work-in-Progress) limits:

```rust
// Recalc operations limited by semaphore
let recalc_semaphore = GlobalRecalcLock::new(config.max_concurrent_recalcs);
let screenshot_semaphore = GlobalScreenshotLock::new();
```

**Andon Visualization**:
- **Current WIP**: Number of permits currently held
- **Capacity**: Maximum concurrent operations
- **Queue Depth**: Operations waiting for permits

### 8. Timeout Enforcement (Response Time Monitoring)

**Location**: `/home/user/ggen-mcp/src/server.rs`

Tool timeout enforcement provides SLA monitoring:

```rust
async fn run_tool_with_timeout<T, F>(&self, tool: &str, fut: F) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    let result = if let Some(timeout_duration) = self.state.config().tool_timeout() {
        match tokio::time::timeout(timeout_duration, fut).await {
            Ok(result) => result,
            Err(_) => Err(anyhow!(
                "tool '{}' timed out after {}ms",
                tool,
                timeout_duration.as_millis()
            )),
        }
    } else {
        fut.await
    }?;

    self.ensure_response_size(tool, &result)?;
    Ok(result)
}
```

**Timeout Signals**:
- **Green**: Tool completes within timeout
- **Red**: Timeout exceeded → log error, return failure

## Recommended Andon Enhancements

### 1. Health Check Endpoint

**Purpose**: Provide a simple, fast endpoint for monitoring systems

**Recommended Implementation**:

```rust
#[derive(Serialize)]
pub struct HealthCheckResponse {
    pub status: HealthStatus,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub uptime_seconds: u64,
    pub checks: HashMap<String, ComponentHealth>,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,   // ✓ All systems operational
    Degraded,  // ⚠ Some issues, but functional
    Unhealthy, // ✗ Critical failures
}

#[derive(Serialize)]
pub struct ComponentHealth {
    pub status: HealthStatus,
    pub message: Option<String>,
    pub last_check: DateTime<Utc>,
    pub details: Option<serde_json::Value>,
}

pub async fn health_check(state: Arc<AppState>) -> Result<HealthCheckResponse> {
    let mut checks = HashMap::new();
    let mut overall_status = HealthStatus::Healthy;

    // 1. Check workspace accessibility
    checks.insert("workspace".to_string(), check_workspace(&state));

    // 2. Check cache health
    checks.insert("cache".to_string(), check_cache(&state));

    // 3. Check fork registry (if enabled)
    #[cfg(feature = "recalc")]
    if let Some(registry) = state.fork_registry() {
        checks.insert("fork_registry".to_string(), check_fork_registry(registry));
    }

    // 4. Check recalc backend (if enabled)
    #[cfg(feature = "recalc")]
    if let Some(backend) = state.recalc_backend() {
        checks.insert("recalc_backend".to_string(), check_recalc_backend(backend));
    }

    // Determine overall status
    for check in checks.values() {
        match check.status {
            HealthStatus::Unhealthy => {
                overall_status = HealthStatus::Unhealthy;
                break;
            }
            HealthStatus::Degraded if overall_status == HealthStatus::Healthy => {
                overall_status = HealthStatus::Degraded;
            }
            _ => {}
        }
    }

    Ok(HealthCheckResponse {
        status: overall_status,
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: STARTUP_TIME.elapsed().as_secs(),
        checks,
    })
}

fn check_workspace(state: &AppState) -> ComponentHealth {
    match state.config().workspace_root.try_exists() {
        Ok(true) => ComponentHealth {
            status: HealthStatus::Healthy,
            message: Some("Workspace accessible".to_string()),
            last_check: Utc::now(),
            details: None,
        },
        Ok(false) => ComponentHealth {
            status: HealthStatus::Unhealthy,
            message: Some("Workspace does not exist".to_string()),
            last_check: Utc::now(),
            details: None,
        },
        Err(e) => ComponentHealth {
            status: HealthStatus::Unhealthy,
            message: Some(format!("Workspace check failed: {}", e)),
            last_check: Utc::now(),
            details: None,
        },
    }
}

fn check_cache(state: &AppState) -> ComponentHealth {
    let stats = state.cache_stats();
    let utilization = (stats.size as f64) / (stats.capacity as f64);
    let hit_rate = stats.hit_rate();

    let status = if utilization > 0.95 || hit_rate < 0.5 {
        HealthStatus::Degraded
    } else {
        HealthStatus::Healthy
    };

    ComponentHealth {
        status,
        message: Some(format!(
            "Cache: {}/{} entries, {:.1}% hit rate",
            stats.size, stats.capacity, hit_rate * 100.0
        )),
        last_check: Utc::now(),
        details: Some(serde_json::json!({
            "size": stats.size,
            "capacity": stats.capacity,
            "utilization": utilization,
            "hit_rate": hit_rate,
            "operations": stats.operations,
        })),
    }
}
```

**Endpoint Location**:
- HTTP mode: `GET /health`
- Stdio mode: Not applicable (use startup logs)

### 2. Metrics Endpoint (Prometheus Format)

**Purpose**: Expose operational metrics for time-series monitoring

**Recommended Metrics**:

```
# HELP spreadsheet_mcp_cache_size Current number of cached workbooks
# TYPE spreadsheet_mcp_cache_size gauge
spreadsheet_mcp_cache_size 3

# HELP spreadsheet_mcp_cache_capacity Maximum cache capacity
# TYPE spreadsheet_mcp_cache_capacity gauge
spreadsheet_mcp_cache_capacity 5

# HELP spreadsheet_mcp_cache_operations_total Total cache operations
# TYPE spreadsheet_mcp_cache_operations_total counter
spreadsheet_mcp_cache_operations_total 1247

# HELP spreadsheet_mcp_cache_hits_total Total cache hits
# TYPE spreadsheet_mcp_cache_hits_total counter
spreadsheet_mcp_cache_hits_total 892

# HELP spreadsheet_mcp_cache_misses_total Total cache misses
# TYPE spreadsheet_mcp_cache_misses_total counter
spreadsheet_mcp_cache_misses_total 355

# HELP spreadsheet_mcp_tool_duration_seconds Tool execution duration
# TYPE spreadsheet_mcp_tool_duration_seconds histogram
spreadsheet_mcp_tool_duration_seconds_bucket{tool="list_workbooks",le="0.1"} 42
spreadsheet_mcp_tool_duration_seconds_bucket{tool="list_workbooks",le="0.5"} 98
spreadsheet_mcp_tool_duration_seconds_bucket{tool="list_workbooks",le="1.0"} 100
spreadsheet_mcp_tool_duration_seconds_sum{tool="list_workbooks"} 23.4
spreadsheet_mcp_tool_duration_seconds_count{tool="list_workbooks"} 100

# HELP spreadsheet_mcp_active_forks Current number of active forks
# TYPE spreadsheet_mcp_active_forks gauge
spreadsheet_mcp_active_forks 2

# HELP spreadsheet_mcp_recalc_operations_total Total recalc operations
# TYPE spreadsheet_mcp_recalc_operations_total counter
spreadsheet_mcp_recalc_operations_total{status="success"} 45
spreadsheet_mcp_recalc_operations_total{status="failure"} 2
spreadsheet_mcp_recalc_operations_total{status="timeout"} 1

# HELP spreadsheet_mcp_circuit_breaker_state Circuit breaker state (0=closed, 1=half_open, 2=open)
# TYPE spreadsheet_mcp_circuit_breaker_state gauge
spreadsheet_mcp_circuit_breaker_state{name="recalc"} 0
```

### 3. Status Dashboard (Visual Management)

**Purpose**: Real-time operational visibility

**Recommended Dashboard Panels**:

#### Panel 1: System Health Overview
```
┌─────────────────────────────────────────────┐
│ System Health: ✓ HEALTHY                   │
│ Uptime: 2d 14h 23m                          │
│ Version: 0.9.0                              │
├─────────────────────────────────────────────┤
│ Component Status:                           │
│ ✓ Workspace       Accessible               │
│ ✓ Cache           85% hit rate              │
│ ✓ Fork Registry   2 active forks            │
│ ✓ Recalc Backend  Available                 │
│ ⚠ Circuit Breaker recalc: HALF_OPEN         │
└─────────────────────────────────────────────┘
```

#### Panel 2: Cache Utilization
```
┌─────────────────────────────────────────────┐
│ Cache Utilization                           │
│ [████████░░] 3/5 entries (60%)              │
│                                             │
│ Hit Rate:  [████████░░] 85%                 │
│ Operations: 1,247                           │
│   Hits: 892                                 │
│   Misses: 355                               │
└─────────────────────────────────────────────┘
```

#### Panel 3: Active Operations
```
┌─────────────────────────────────────────────┐
│ Active Operations                           │
│ Recalc Queue: 0/2 slots available           │
│ Screenshot Queue: 1/1 slots available       │
│ Active Forks: 2                             │
│   - fork-abc123 (age: 5m)                   │
│   - fork-def456 (age: 2m)                   │
└─────────────────────────────────────────────┘
```

#### Panel 4: Recent Errors
```
┌─────────────────────────────────────────────┐
│ Recent Errors (Last 1h)                     │
│ 14:23 ERROR recalc timeout fork-abc123      │
│ 14:15 WARN  cache eviction due to capacity  │
│ 13:45 ERROR workbook not found wb-xyz789    │
└─────────────────────────────────────────────┘
```

### 4. Alert Categorization

**Purpose**: Route alerts based on severity and urgency

#### Alert Severity Levels

| Level | Color | Response Time | Example Conditions |
|-------|-------|---------------|-------------------|
| **INFO** | Blue | Best effort | Workbook cache eviction, fork created |
| **WARNING** | Yellow | < 30 minutes | Cache hit rate < 50%, recalc queue saturated |
| **ERROR** | Red | < 5 minutes | Circuit breaker opened, workspace inaccessible |
| **CRITICAL** | Red/Blinking | Immediate | System unresponsive, all tools timing out |

#### Alert Rules

```rust
pub struct AlertRule {
    pub name: String,
    pub condition: Box<dyn Fn(&SystemState) -> bool>,
    pub severity: AlertSeverity,
    pub message: String,
    pub cooldown: Duration,
}

// Example alert rules
let rules = vec![
    AlertRule {
        name: "cache_hit_rate_low".to_string(),
        condition: Box::new(|state| {
            state.cache_stats().hit_rate() < 0.5
        }),
        severity: AlertSeverity::Warning,
        message: "Cache hit rate below 50% - consider increasing capacity".to_string(),
        cooldown: Duration::from_secs(300), // 5 minutes
    },
    AlertRule {
        name: "recalc_circuit_open".to_string(),
        condition: Box::new(|state| {
            state.recalc_circuit_state() == CircuitBreakerState::Open
        }),
        severity: AlertSeverity::Error,
        message: "Recalc circuit breaker is OPEN - LibreOffice issues detected".to_string(),
        cooldown: Duration::from_secs(60),
    },
    AlertRule {
        name: "workspace_inaccessible".to_string(),
        condition: Box::new(|state| {
            !state.config().workspace_root.exists()
        }),
        severity: AlertSeverity::Critical,
        message: "Workspace root is inaccessible - server cannot function".to_string(),
        cooldown: Duration::from_secs(30),
    },
];
```

### 5. SLA/SLO Monitoring

**Purpose**: Track service level objectives and identify degradation

#### Recommended SLOs

| Metric | SLO Target | Measurement Window | Alert Threshold |
|--------|------------|-------------------|-----------------|
| **Tool Response Time (p95)** | < 1s | 5 minutes | > 2s |
| **Tool Response Time (p99)** | < 3s | 5 minutes | > 5s |
| **Tool Success Rate** | > 99% | 1 hour | < 95% |
| **Cache Hit Rate** | > 70% | 15 minutes | < 50% |
| **Recalc Success Rate** | > 95% | 1 hour | < 90% |
| **System Uptime** | > 99.9% | 30 days | < 99% |

#### SLO Tracking

```rust
pub struct SloTracker {
    metrics: Arc<RwLock<SloMetrics>>,
    window_size: Duration,
}

pub struct SloMetrics {
    pub tool_durations: Vec<(String, Duration)>, // (tool_name, duration)
    pub tool_outcomes: Vec<(String, bool)>,      // (tool_name, success)
    pub cache_operations: Vec<bool>,              // hit (true) or miss (false)
    pub recalc_outcomes: Vec<bool>,               // success (true) or failure (false)
    pub window_start: Instant,
}

impl SloTracker {
    pub fn record_tool_invocation(&self, tool: &str, duration: Duration, success: bool) {
        let mut metrics = self.metrics.write();
        metrics.tool_durations.push((tool.to_string(), duration));
        metrics.tool_outcomes.push((tool.to_string(), success));
        self.prune_old_entries(&mut metrics);
    }

    pub fn calculate_p95_latency(&self, tool: Option<&str>) -> Duration {
        let metrics = self.metrics.read();
        let mut durations: Vec<Duration> = metrics.tool_durations
            .iter()
            .filter(|(t, _)| tool.map_or(true, |name| t == name))
            .map(|(_, d)| *d)
            .collect();

        if durations.is_empty() {
            return Duration::from_secs(0);
        }

        durations.sort();
        let idx = (durations.len() as f64 * 0.95) as usize;
        durations[idx.min(durations.len() - 1)]
    }

    pub fn calculate_success_rate(&self, tool: Option<&str>) -> f64 {
        let metrics = self.metrics.read();
        let outcomes: Vec<bool> = metrics.tool_outcomes
            .iter()
            .filter(|(t, _)| tool.map_or(true, |name| t == name))
            .map(|(_, success)| *success)
            .collect();

        if outcomes.is_empty() {
            return 1.0;
        }

        let successes = outcomes.iter().filter(|&&s| s).count();
        successes as f64 / outcomes.len() as f64
    }

    pub fn check_slo_violations(&self) -> Vec<SloViolation> {
        let mut violations = Vec::new();

        // Check p95 latency SLO
        let p95 = self.calculate_p95_latency(None);
        if p95 > Duration::from_secs(2) {
            violations.push(SloViolation {
                metric: "p95_latency".to_string(),
                current_value: p95.as_secs_f64(),
                threshold: 2.0,
                severity: AlertSeverity::Warning,
            });
        }

        // Check success rate SLO
        let success_rate = self.calculate_success_rate(None);
        if success_rate < 0.95 {
            violations.push(SloViolation {
                metric: "success_rate".to_string(),
                current_value: success_rate,
                threshold: 0.95,
                severity: AlertSeverity::Error,
            });
        }

        violations
    }
}
```

### 6. Escalation Procedures

**Purpose**: Define clear escalation paths for different failure scenarios

#### Escalation Matrix

| Scenario | Detection | L1 Response | L2 Escalation | L3 Escalation |
|----------|-----------|-------------|---------------|---------------|
| **Cache thrashing** | Hit rate < 50% for 15m | Monitor, log warning | Increase cache capacity | Review workbook access patterns |
| **Circuit breaker open** | Circuit state = Open | Log error, alert on-call | Check LibreOffice health | Restart LibreOffice, investigate root cause |
| **Workspace inaccessible** | Health check failure | Log critical, page on-call | Check filesystem, permissions | Restore from backup |
| **High error rate** | Success rate < 95% for 1h | Review error logs | Identify common failure patterns | Code fix or config adjustment |
| **Recalc timeout spike** | Recalc timeouts > 10% for 1h | Log warnings | Check LibreOffice CPU/memory | Increase timeout, add parallelism |
| **Tool timeout** | Single tool > 30s | Log error, return timeout | Check tool-specific resources | Optimize tool logic |

#### Runbook Template

```markdown
# Runbook: Circuit Breaker Opened

## Symptoms
- Circuit breaker state transitions to OPEN
- Subsequent operations fail fast with "circuit breaker is open" error
- Error logs show repeated failures before circuit opened

## Impact
- Operations protected by the circuit breaker will fail immediately
- Other operations continue to function normally
- System is preventing cascade failures

## Investigation Steps
1. Check circuit breaker statistics:
   ```rust
   let stats = circuit_breaker.stats();
   tracing::info!("Circuit breaker stats: {:?}", stats);
   ```

2. Review recent error logs:
   ```bash
   grep -A 5 "circuit breaker" /tmp/mcp-audit-logs/audit-*.jsonl | tail -50
   ```

3. For recalc circuit breaker:
   - Check LibreOffice process health: `ps aux | grep soffice`
   - Check available disk space: `df -h /tmp`
   - Review recent recalc timeouts

4. For file I/O circuit breaker:
   - Check workspace accessibility: `ls -la /data`
   - Check filesystem errors: `dmesg | grep -i error`
   - Review disk I/O metrics

## Resolution Steps

### Immediate (< 5 minutes)
1. Verify the underlying issue is resolved
2. Manually reset the circuit breaker if safe:
   ```rust
   circuit_breaker.reset();
   ```

### Short-term (< 1 hour)
1. If LibreOffice issues: Restart LibreOffice processes
2. If filesystem issues: Remount filesystem or restore from backup
3. Monitor circuit breaker state after reset

### Long-term (< 1 day)
1. Review failure patterns to prevent future occurrences
2. Adjust circuit breaker thresholds if needed
3. Implement additional monitoring or health checks
4. Update runbook with lessons learned

## Prevention
- Regular health checks on LibreOffice
- Filesystem monitoring and alerting
- Capacity planning for concurrent operations
- Automated recovery procedures
```

## Best Practices for Andon in MCP Servers

### 1. Make Problems Immediately Visible

**Good**:
```rust
// Clear, actionable error with context
tracing::error!(
    tool = "recalculate",
    fork_id = %fork_id,
    duration_ms = elapsed_ms,
    error = %err,
    "recalculation failed - LibreOffice timeout"
);
```

**Bad**:
```rust
// Vague, unhelpful error
tracing::error!("Error in recalc");
```

### 2. Use Consistent Signal Levels

**Log Level Discipline**:
- **DEBUG**: Only for development/troubleshooting (disabled in production)
- **INFO**: Normal operational events (startup, successful operations)
- **WARN**: Degraded performance or recoverable errors (retries, fallbacks)
- **ERROR**: Failures requiring investigation (timeouts, circuit breakers)

### 3. Include Context in All Signals

**Always Include**:
- **Operation**: What was being attempted
- **Resource**: What was being operated on (workbook_id, fork_id, path)
- **Duration**: How long it took (for performance analysis)
- **Error Details**: Specific error message and type

### 4. Provide Actionable Information

**Good**:
```rust
tracing::warn!(
    cache_capacity = config.cache_capacity,
    max_concurrent_recalcs = config.max_concurrent_recalcs,
    "cache_capacity is smaller than max_concurrent_recalcs; \
     this may cause workbooks to be evicted during recalculation. \
     Consider increasing cache_capacity to at least {}",
    config.max_concurrent_recalcs
);
```

**Bad**:
```rust
tracing::warn!("Cache might be too small");
```

### 5. Implement Progressive Escalation

**Escalation Ladder**:
1. **First occurrence**: Log at DEBUG or INFO
2. **Retry attempt**: Log at WARN with retry count
3. **After N retries**: Log at ERROR with full context
4. **Pattern detected**: Trigger circuit breaker, log CRITICAL

### 6. Enable Rapid Problem Diagnosis

**Correlation IDs**:
```rust
// Use span IDs to correlate related logs
let span = info_span!(
    "tool_invocation",
    tool = tool_name,
    request_id = %request_id
);
let _guard = span.enter();

// All logs within this scope are correlated
tracing::info!("starting operation");
// ... perform work ...
tracing::info!("operation complete");
```

### 7. Track Recovery Operations

**Before**:
```rust
tracing::warn!("Operation failed, retrying...");
```

**After**:
```rust
tracing::warn!(
    operation = operation_name,
    attempt = context.attempt,
    max_attempts = context.max_attempts,
    delay_ms = delay.as_millis(),
    error = %err,
    "retrying operation"
);
```

### 8. Monitor Resource Boundaries

**Key Resources to Monitor**:
- Cache size vs capacity
- Semaphore permits vs total
- Active forks vs limits
- Queue depths
- Disk space
- Memory usage

### 9. Provide Self-Service Diagnostics

**Diagnostic Tools**:
```rust
// Cache statistics
pub fn cache_stats(&self) -> CacheStats;

// Circuit breaker status
pub fn circuit_breaker_stats(&self) -> CircuitBreakerStats;

// Audit trail query
pub fn query_events(&self, filter: AuditFilter) -> Vec<AuditEvent>;

// Recent errors
pub fn recent_errors(&self, limit: usize) -> Vec<AuditEvent>;
```

### 10. Document Normal Operating Ranges

**Establish Baselines**:
```rust
// Document expected ranges in configuration
pub const EXPECTED_CACHE_HIT_RATE_MIN: f64 = 0.70;
pub const EXPECTED_CACHE_UTILIZATION_MAX: f64 = 0.85;
pub const EXPECTED_P95_LATENCY_MS: u64 = 1000;
pub const EXPECTED_SUCCESS_RATE_MIN: f64 = 0.99;
```

## Integration Checklist

When adding new operations to the MCP server, ensure Andon principles are applied:

- [ ] **Logging**: Add structured logs at appropriate levels
- [ ] **Metrics**: Track operation count, duration, success/failure
- [ ] **Audit Trail**: Log significant events to audit system
- [ ] **Error Handling**: Categorize errors and apply recovery strategies
- [ ] **Timeouts**: Set reasonable timeouts and handle gracefully
- [ ] **Circuit Breaker**: Protect against cascade failures if appropriate
- [ ] **Health Check**: Include component in health check endpoint
- [ ] **Documentation**: Add operation to runbook with escalation procedures
- [ ] **Alerts**: Define alert rules for abnormal conditions
- [ ] **Dashboard**: Add relevant metrics to status dashboard

## Conclusion

Andon principles from the Toyota Production System provide a robust framework for operational excellence in MCP servers. By making problems immediately visible, enabling rapid intervention, and providing clear escalation paths, we ensure that the spreadsheet-mcp server is reliable, observable, and maintainable.

The current implementation already includes strong foundations:
- Comprehensive audit trail system
- Cache statistics and monitoring
- Circuit breaker pattern for fault tolerance
- Structured logging with tracing
- Recovery mechanisms with retry and fallback
- Configuration validation (poka-yoke)
- Resource management with semaphores

Future enhancements should focus on:
- Dedicated health check endpoint
- Metrics exposure (Prometheus format)
- Real-time status dashboard
- SLA/SLO monitoring
- Automated alert routing
- Enhanced escalation procedures

By continuously improving these Andon capabilities, we create a production-ready MCP server that operators can trust and maintain with confidence.

## References

- Current implementations:
  - `/home/user/ggen-mcp/src/audit/mod.rs` - Audit trail system
  - `/home/user/ggen-mcp/src/state.rs` - Cache statistics
  - `/home/user/ggen-mcp/src/recovery/circuit_breaker.rs` - Circuit breaker pattern
  - `/home/user/ggen-mcp/src/recovery/mod.rs` - Recovery strategies
  - `/home/user/ggen-mcp/src/config.rs` - Configuration validation
  - `/home/user/ggen-mcp/src/server.rs` - Timeout enforcement

- Related documentation:
  - `POKA_YOKE_PATTERN.md` - Error-proofing with NewType wrappers
  - `VALIDATION_INTEGRATION_EXAMPLE.rs` - Input validation patterns
  - `RECOVERY_IMPLEMENTATION.md` - Recovery and fallback mechanisms
  - `AUDIT_INTEGRATION_GUIDE.md` - Audit trail usage guide

- External resources:
  - Toyota Production System and Jidoka principles
  - Prometheus metrics best practices
  - OpenTelemetry observability standards
  - Site Reliability Engineering (SRE) best practices
