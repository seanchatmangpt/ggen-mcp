# Graceful Shutdown System

Comprehensive documentation for the spreadsheet-mcp server's graceful shutdown implementation, designed for safe production deployments.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Shutdown Phases](#shutdown-phases)
- [Configuration](#configuration)
- [Integration](#integration)
- [Kubernetes Deployment](#kubernetes-deployment)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Overview

The graceful shutdown system ensures that the spreadsheet-mcp server can terminate safely without:
- Losing in-flight requests
- Corrupting data or state
- Leaving zombie processes
- Dropping audit logs

### Key Features

- **Signal Handling**: Responds to SIGTERM and SIGINT signals
- **Multi-Phase Shutdown**: Orchestrated shutdown with configurable timeouts
- **Component Coordination**: Manages shutdown of all server components
- **Request Tracking**: Monitors and waits for active requests to complete
- **Force Shutdown**: Fallback mechanism if graceful shutdown exceeds timeout
- **Production Ready**: Designed for container orchestrators like Kubernetes

## Architecture

### Components

```
┌─────────────────────────────────────────────────────────────┐
│                    ShutdownCoordinator                       │
│  - Signal handling (SIGTERM/SIGINT)                         │
│  - Phase orchestration                                       │
│  - Timeout management                                        │
│  - Request tracking                                          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ├── CancellationToken (broadcast)
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
┌─────────────┐      ┌─────────────┐      ┌─────────────┐
│  AppState   │      │ Audit Logger│      │ LibreOffice │
│  Handler    │      │   Handler   │      │   Handler   │
│             │      │             │      │  (optional) │
│ - Flush     │      │ - Flush     │      │ - Terminate │
│   cache     │      │   logs      │      │   processes │
│ - Shutdown  │      │ - Close     │      │             │
│   fork      │      │   files     │      │             │
│   registry  │      │             │      │             │
└─────────────┘      └─────────────┘      └─────────────┘
```

### Shutdown Token Pattern

The system uses `tokio_util::CancellationToken` for coordinating async task shutdown:

```rust
let shutdown_token = coordinator.token();

// In your async task:
tokio::select! {
    _ = shutdown_token.cancelled() => {
        // Cleanup and exit
        info!("shutdown signal received, exiting");
        break;
    }
    result = do_work() => {
        // Normal operation
        handle_result(result);
    }
}
```

## Shutdown Phases

The shutdown process proceeds through five distinct phases, each with its own timeout:

### Phase 1: Stop Accepting New Requests (Default: 2s)

**Purpose**: Signal all components to stop accepting new work.

**Actions**:
- Broadcast cancellation token to all async tasks
- HTTP server stops accepting new connections
- Tool handlers reject new requests

**Verification**:
```rust
assert_eq!(coordinator.phase().await, ShutdownPhase::StopAccepting);
assert!(coordinator.is_shutdown_initiated());
```

### Phase 2: Wait for In-Flight Requests (Default: 30s)

**Purpose**: Allow active requests to complete normally.

**Actions**:
- Monitor active request counter
- Poll every 100ms for completion
- Log progress of remaining requests

**Request Tracking**:
```rust
// At request start:
coordinator.request_started();

// At request completion:
coordinator.request_finished();
```

**Timeout Behavior**: If timeout is reached with active requests, proceed anyway (logged as warning).

### Phase 3: Flush Caches and Close Connections (Default: 5s)

**Purpose**: Persist state and close resources.

**Actions**:
- Flush workbook cache (log statistics)
- Flush fork checkpoints to disk
- Flush audit log buffers
- Close database connections (if any)

**Component Integration**:
```rust
#[async_trait]
impl ShutdownHandler for MyComponent {
    async fn flush(&self) -> Result<()> {
        // Flush pending data
        self.write_buffer_to_disk().await?;
        Ok(())
    }
}
```

### Phase 4: Final Cleanup (Default: 3s)

**Purpose**: Cleanup temporary resources and final logging.

**Actions**:
- Cleanup temporary fork files
- Final audit log entry
- Release file locks
- Shutdown telemetry

### Phase 5: Force Shutdown (If Timeout Exceeded)

**Purpose**: Emergency shutdown if graceful phases timeout.

**Actions**:
- Cancel all remaining async tasks
- Log warning about forced shutdown
- Exit immediately

**Configuration**:
```rust
let config = ShutdownConfig {
    force_shutdown_on_timeout: true, // Enable force shutdown
    total_timeout: Duration::from_secs(45),
    ..Default::default()
};
```

## Configuration

### Server Configuration

Add to your configuration file (YAML or JSON):

```yaml
graceful_shutdown_timeout_secs: 45  # Total shutdown timeout (default: 45)
```

Or via environment variable:

```bash
export SPREADSHEET_MCP_GRACEFUL_SHUTDOWN_TIMEOUT_SECS=60
```

Or via command-line flag:

```bash
spreadsheet-mcp --graceful-shutdown-timeout-secs 60
```

### Custom Shutdown Config

For programmatic configuration:

```rust
use spreadsheet_mcp::shutdown::ShutdownConfig;

let config = ShutdownConfig::default()
    .with_total_timeout(60)           // 60 second total timeout
    .with_in_flight_timeout(40);      // 40 second in-flight timeout

let coordinator = ShutdownCoordinator::new(config);
```

### Advanced Configuration

```rust
let config = ShutdownConfig {
    stop_accepting_timeout: Duration::from_secs(2),
    in_flight_timeout: Duration::from_secs(30),
    flush_timeout: Duration::from_secs(5),
    cleanup_timeout: Duration::from_secs(3),
    total_timeout: Duration::from_secs(45),
    force_shutdown_on_timeout: true,
};
```

## Integration

### HTTP Server Integration

The HTTP transport automatically uses graceful shutdown:

```rust
async fn run_stream_http_transport(
    config: Arc<ServerConfig>,
    state: Arc<AppState>
) -> Result<()> {
    let coordinator = Arc::new(ShutdownCoordinator::new(
        ShutdownConfig::default()
            .with_total_timeout(config.graceful_shutdown_timeout_secs)
    ));

    // Setup shutdown handlers
    let mut composite = CompositeShutdownHandler::new();
    composite.add_handler(Box::new(AppStateShutdownHandler::new(state)));
    // ... add more handlers

    // Server with graceful shutdown
    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            coordinator.wait_for_signal().await;
        })
        .await?;

    // Component shutdown
    composite.shutdown().await?;
    Ok(())
}
```

### Custom Component Integration

Implement `ShutdownHandler` trait for your components:

```rust
use spreadsheet_mcp::shutdown::ShutdownHandler;

struct MyComponent {
    // ... component state
}

#[async_trait::async_trait]
impl ShutdownHandler for MyComponent {
    async fn shutdown(&self) -> Result<()> {
        info!("shutting down MyComponent");

        // Cleanup logic
        self.close_connections().await?;
        self.cleanup_resources().await?;

        info!("MyComponent shutdown complete");
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        debug!("flushing MyComponent");

        // Flush pending data
        self.flush_buffers().await?;

        Ok(())
    }
}
```

### Request Tracking Integration

Track active requests in your request handlers:

```rust
async fn handle_request(
    coordinator: Arc<ShutdownCoordinator>,
    request: Request
) -> Result<Response> {
    // Check if shutdown is in progress
    if coordinator.is_shutdown_initiated() {
        return Err(anyhow!("Server shutting down"));
    }

    // Track active request
    coordinator.request_started();

    // Process request
    let result = async {
        process_request(request).await
    }.await;

    // Always decrement counter
    coordinator.request_finished();

    result
}
```

## Kubernetes Deployment

### Pod Configuration

Configure termination grace period to match shutdown timeout:

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: spreadsheet-mcp
spec:
  containers:
  - name: spreadsheet-mcp
    image: spreadsheet-mcp:latest
    env:
    - name: SPREADSHEET_MCP_GRACEFUL_SHUTDOWN_TIMEOUT_SECS
      value: "45"
    ports:
    - containerPort: 8079
      name: http
    livenessProbe:
      httpGet:
        path: /health
        port: http
      initialDelaySeconds: 10
      periodSeconds: 10
    readinessProbe:
      httpGet:
        path: /ready
        port: http
      initialDelaySeconds: 5
      periodSeconds: 5
  terminationGracePeriodSeconds: 60  # Must be > graceful_shutdown_timeout_secs
```

### Deployment Best Practices

1. **Grace Period**: Set `terminationGracePeriodSeconds` to at least 15 seconds more than `graceful_shutdown_timeout_secs`
   ```yaml
   terminationGracePeriodSeconds: 60  # For 45s shutdown timeout
   ```

2. **Readiness Probe**: Disable readiness during shutdown to stop new traffic
   ```yaml
   readinessProbe:
     httpGet:
       path: /ready
       port: http
     failureThreshold: 1  # Fail fast during shutdown
   ```

3. **Pre-Stop Hook** (Optional): Add additional delay before SIGTERM
   ```yaml
   lifecycle:
     preStop:
       exec:
         command: ["/bin/sh", "-c", "sleep 5"]
   ```

### Load Balancer Configuration

Configure your load balancer to respect readiness probes:

- **AWS ALB**: Target group deregistration delay should be > shutdown timeout
- **NGINX Ingress**: Set `nginx.ingress.kubernetes.io/proxy-connect-timeout` appropriately
- **Istio**: Configure `trafficPolicy.connectionPool.tcp.maxConnections`

### Shutdown Flow in Kubernetes

```
1. kubectl delete pod spreadsheet-mcp
2. Kubernetes marks pod as Terminating
3. Readiness probe fails → removed from service endpoints
4. Load balancer stops sending new traffic (takes 5-15s)
5. Kubernetes sends SIGTERM to pod
6. ShutdownCoordinator receives signal
7. Phase 1: Stop accepting (2s)
8. Phase 2: Wait for in-flight (30s)
9. Phase 3: Flush caches (5s)
10. Phase 4: Cleanup (3s)
11. Process exits (< 45s total)
12. If > 60s, Kubernetes sends SIGKILL
```

## Best Practices

### 1. Configure Appropriate Timeouts

Match timeouts to your workload characteristics:

```rust
// For long-running operations (data processing)
let config = ShutdownConfig::default()
    .with_in_flight_timeout(60)
    .with_total_timeout(90);

// For quick API responses
let config = ShutdownConfig::default()
    .with_in_flight_timeout(15)
    .with_total_timeout(30);
```

### 2. Monitor Shutdown Metrics

Log shutdown metrics for monitoring:

```rust
let start = Instant::now();
coordinator.shutdown().await?;
let duration = start.elapsed();

metrics::histogram!("shutdown_duration_seconds")
    .record(duration.as_secs_f64());
```

### 3. Test Shutdown Behavior

Test graceful shutdown in your CI/CD pipeline:

```rust
#[tokio::test]
async fn test_graceful_shutdown_integration() {
    let server = start_test_server().await;

    // Send in-flight requests
    let requests = send_concurrent_requests(&server, 10);

    // Trigger shutdown
    server.shutdown().await;

    // Verify all requests completed
    assert_all_requests_succeeded(requests).await;
}
```

### 4. Handle Shutdown in Long-Running Tasks

Check cancellation token in long-running operations:

```rust
async fn process_large_dataset(
    data: Dataset,
    shutdown_token: CancellationToken
) -> Result<()> {
    for chunk in data.chunks(1000) {
        // Check for shutdown
        if shutdown_token.is_cancelled() {
            info!("shutdown requested, saving progress");
            save_checkpoint(&chunk).await?;
            return Ok(());
        }

        process_chunk(chunk).await?;
    }
    Ok(())
}
```

### 5. Implement Idempotent Shutdown

Ensure shutdown handlers can be called multiple times:

```rust
#[async_trait]
impl ShutdownHandler for MyComponent {
    async fn shutdown(&self) -> Result<()> {
        // Use atomic flag to ensure idempotency
        if self.shutdown_flag.swap(true, Ordering::SeqCst) {
            debug!("shutdown already called, skipping");
            return Ok(());
        }

        // Actual shutdown logic
        self.cleanup().await?;
        Ok(())
    }
}
```

### 6. Log Shutdown Progress

Add detailed logging for debugging:

```rust
info!("shutdown initiated");
info!("phase 1: stopping new requests");
info!("phase 2: waiting for {} active requests", active_count);
info!("phase 3: flushing caches");
info!("phase 4: final cleanup");
info!("shutdown complete in {:?}", duration);
```

## Troubleshooting

### Issue: Shutdown Takes Too Long

**Symptoms**: Server exceeds timeout and is force-killed

**Solutions**:
1. Check for stuck requests:
   ```rust
   info!("active requests: {}", coordinator.active_request_count());
   ```

2. Review in-flight timeout:
   ```rust
   let config = ShutdownConfig::default()
       .with_in_flight_timeout(60); // Increase timeout
   ```

3. Add timeout to blocking operations:
   ```rust
   timeout(Duration::from_secs(5), blocking_operation()).await?;
   ```

### Issue: Data Loss During Shutdown

**Symptoms**: Lost audit logs, cache data, or fork checkpoints

**Solutions**:
1. Implement proper flush logic:
   ```rust
   async fn flush(&self) -> Result<()> {
       self.audit_logger.flush().await?;
       self.cache.persist().await?;
       Ok(())
   }
   ```

2. Increase flush timeout:
   ```rust
   let config = ShutdownConfig {
       flush_timeout: Duration::from_secs(10),
       ..Default::default()
   };
   ```

### Issue: LibreOffice Processes Not Terminating

**Symptoms**: Zombie soffice processes after shutdown

**Solutions**:
1. Ensure proper process cleanup:
   ```rust
   #[cfg(feature = "recalc")]
   impl ShutdownHandler for LibreOfficeShutdownHandler {
       async fn shutdown(&self) -> Result<()> {
           // Terminate all processes
           self.backend.terminate_all().await?;

           // Wait for processes to exit
           sleep(Duration::from_secs(2)).await;
           Ok(())
       }
   }
   ```

2. Monitor process termination:
   ```bash
   # Check for zombie processes
   ps aux | grep soffice
   ```

### Issue: Kubernetes SIGKILL Before Graceful Shutdown

**Symptoms**: Pods killed before shutdown completes

**Solutions**:
1. Increase `terminationGracePeriodSeconds`:
   ```yaml
   terminationGracePeriodSeconds: 90  # Increase from 60
   ```

2. Add pre-stop hook delay:
   ```yaml
   lifecycle:
     preStop:
       exec:
         command: ["/bin/sh", "-c", "sleep 10"]
   ```

### Issue: New Requests During Shutdown

**Symptoms**: 500 errors during rolling deployment

**Solutions**:
1. Check shutdown detection:
   ```rust
   if coordinator.is_shutdown_initiated() {
       return Err(StatusCode::SERVICE_UNAVAILABLE);
   }
   ```

2. Configure load balancer properly:
   ```yaml
   # For NGINX Ingress
   nginx.ingress.kubernetes.io/proxy-connect-timeout: "30"
   ```

## Monitoring and Observability

### Metrics to Track

```rust
// Shutdown duration
metrics::histogram!("shutdown_duration_seconds").record(duration);

// Active requests at shutdown
metrics::gauge!("shutdown_active_requests").set(count);

// Shutdown phase transitions
metrics::counter!("shutdown_phase_transitions")
    .increment(1)
    .with_label("phase", phase.to_string());

// Force shutdown count
metrics::counter!("shutdown_forced_total").increment(1);
```

### Health Check During Shutdown

```rust
async fn readiness_handler(
    State(coordinator): State<Arc<ShutdownCoordinator>>
) -> StatusCode {
    if coordinator.is_shutdown_initiated() {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    }
}
```

## References

- [Kubernetes Container Lifecycle Hooks](https://kubernetes.io/docs/concepts/containers/container-lifecycle-hooks/)
- [Tokio Graceful Shutdown](https://tokio.rs/tokio/topics/shutdown)
- [AWS ECS Task Lifecycle](https://docs.aws.amazon.com/AmazonECS/latest/developerguide/task-lifecycle.html)

## Support

For issues or questions about graceful shutdown:
- Open an issue on GitHub
- Check server logs for shutdown phase transitions
- Review audit logs for data persistence verification
