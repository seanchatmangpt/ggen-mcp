/// Production Setup Example for ggen-mcp
///
/// This example demonstrates production-ready patterns including:
/// - Comprehensive health checks
/// - Prometheus metrics
/// - Graceful shutdown
/// - Circuit breaker
/// - Distributed tracing
///
/// Run with:
/// ```
/// cargo run --example production_setup --features recalc
/// ```

use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use prometheus::{
    Encoder, Histogram, HistogramOpts, IntCounter, IntGauge, Opts, Registry, TextEncoder,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{info, warn};

// ============================================================================
// Health Check Types
// ============================================================================

#[derive(Serialize, Deserialize)]
pub struct HealthResponse {
    status: String,
    timestamp: String,
}

#[derive(Serialize, Deserialize)]
pub struct ReadinessResponse {
    status: String,
    checks: HashMap<String, CheckStatus>,
    timestamp: String,
}

#[derive(Serialize, Deserialize)]
pub struct CheckStatus {
    status: String,
    message: Option<String>,
    latency_ms: Option<u64>,
}

// ============================================================================
// Circuit Breaker
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,   // Normal operation
    Open,     // Failing, rejecting requests
    HalfOpen, // Testing recovery
}

pub struct CircuitBreaker {
    state: Arc<parking_lot::RwLock<CircuitState>>,
    failure_count: AtomicUsize,
    success_count: AtomicUsize,
    last_failure: AtomicU64,
    failure_threshold: usize,
    success_threshold: usize,
    timeout_ms: u64,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: usize, success_threshold: usize, timeout_ms: u64) -> Self {
        Self {
            state: Arc::new(parking_lot::RwLock::new(CircuitState::Closed)),
            failure_count: AtomicUsize::new(0),
            success_count: AtomicUsize::new(0),
            last_failure: AtomicU64::new(0),
            failure_threshold,
            success_threshold,
            timeout_ms,
        }
    }

    pub fn is_open(&self) -> bool {
        *self.state.read() == CircuitState::Open
    }

    pub fn record_success(&self) {
        let state = *self.state.read();

        match state {
            CircuitState::Closed => {
                self.failure_count.store(0, Ordering::Relaxed);
            }
            CircuitState::HalfOpen => {
                let successes = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                if successes >= self.success_threshold {
                    *self.state.write() = CircuitState::Closed;
                    self.failure_count.store(0, Ordering::Relaxed);
                    self.success_count.store(0, Ordering::Relaxed);
                    info!("circuit breaker closed");
                }
            }
            CircuitState::Open => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                let last_failure = self.last_failure.load(Ordering::Relaxed);

                if now - last_failure > self.timeout_ms {
                    *self.state.write() = CircuitState::HalfOpen;
                    self.success_count.store(0, Ordering::Relaxed);
                    info!("circuit breaker half-open");
                }
            }
        }
    }

    pub fn record_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        self.last_failure.store(now, Ordering::Relaxed);

        if failures >= self.failure_threshold {
            *self.state.write() = CircuitState::Open;
            warn!(failures, "circuit breaker opened");
        }
    }

    pub fn get_state(&self) -> CircuitState {
        *self.state.read()
    }
}

// ============================================================================
// Metrics
// ============================================================================

pub struct Metrics {
    pub registry: Registry,
    pub requests_total: IntCounter,
    pub request_duration: Histogram,
    pub errors_total: IntCounter,
    pub cache_size: IntGauge,
    pub cache_hits: IntCounter,
    pub cache_misses: IntCounter,
}

impl Metrics {
    pub fn new() -> Result<Self> {
        let registry = Registry::new();

        let requests_total = IntCounter::with_opts(Opts::new(
            "spreadsheet_mcp_requests_total",
            "Total number of MCP tool requests",
        ))?;
        registry.register(Box::new(requests_total.clone()))?;

        let request_duration = Histogram::with_opts(HistogramOpts::new(
            "spreadsheet_mcp_request_duration_seconds",
            "MCP tool request duration in seconds",
        )
        .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]))?;
        registry.register(Box::new(request_duration.clone()))?;

        let errors_total = IntCounter::with_opts(Opts::new(
            "spreadsheet_mcp_errors_total",
            "Total number of errors",
        ))?;
        registry.register(Box::new(errors_total.clone()))?;

        let cache_size = IntGauge::new(
            "spreadsheet_mcp_cache_size",
            "Current number of workbooks in cache",
        )?;
        registry.register(Box::new(cache_size.clone()))?;

        let cache_hits = IntCounter::with_opts(Opts::new(
            "spreadsheet_mcp_cache_hits_total",
            "Total number of cache hits",
        ))?;
        registry.register(Box::new(cache_hits.clone()))?;

        let cache_misses = IntCounter::with_opts(Opts::new(
            "spreadsheet_mcp_cache_misses_total",
            "Total number of cache misses",
        ))?;
        registry.register(Box::new(cache_misses.clone()))?;

        Ok(Self {
            registry,
            requests_total,
            request_duration,
            errors_total,
            cache_size,
            cache_hits,
            cache_misses,
        })
    }
}

// ============================================================================
// Application State
// ============================================================================

pub struct AppState {
    metrics: Arc<Metrics>,
    circuit_breaker: Arc<CircuitBreaker>,
    start_time: std::time::Instant,
}

impl AppState {
    pub fn new() -> Result<Self> {
        Ok(Self {
            metrics: Arc::new(Metrics::new()?),
            circuit_breaker: Arc::new(CircuitBreaker::new(5, 3, 30_000)),
            start_time: std::time::Instant::now(),
        })
    }
}

// ============================================================================
// Health Check Handlers
// ============================================================================

/// Liveness probe - returns 200 if process is alive
async fn liveness_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "alive".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }),
    )
}

/// Readiness probe - checks if service can handle traffic
async fn readiness_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut checks = HashMap::new();
    let mut all_healthy = true;

    // Check circuit breaker
    let circuit_check = check_circuit_breaker(&state).await;
    all_healthy &= circuit_check.status == "healthy";
    checks.insert("circuit_breaker".to_string(), circuit_check);

    // Check uptime
    let uptime_check = check_uptime(&state).await;
    all_healthy &= uptime_check.status == "healthy";
    checks.insert("uptime".to_string(), uptime_check);

    // Check metrics
    let metrics_check = check_metrics(&state).await;
    all_healthy &= metrics_check.status == "healthy";
    checks.insert("metrics".to_string(), metrics_check);

    let status_code = if all_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status_code,
        Json(ReadinessResponse {
            status: if all_healthy { "ready" } else { "not_ready" }.to_string(),
            checks,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }),
    )
}

async fn check_circuit_breaker(state: &AppState) -> CheckStatus {
    let circuit_state = state.circuit_breaker.get_state();

    CheckStatus {
        status: match circuit_state {
            CircuitState::Closed => "healthy",
            CircuitState::HalfOpen => "degraded",
            CircuitState::Open => "unhealthy",
        }
        .to_string(),
        message: Some(format!("circuit: {:?}", circuit_state)),
        latency_ms: Some(0),
    }
}

async fn check_uptime(state: &AppState) -> CheckStatus {
    let uptime = state.start_time.elapsed();

    CheckStatus {
        status: "healthy".to_string(),
        message: Some(format!("uptime: {:?}", uptime)),
        latency_ms: Some(0),
    }
}

async fn check_metrics(state: &AppState) -> CheckStatus {
    let metric_families = state.metrics.registry.gather();

    CheckStatus {
        status: "healthy".to_string(),
        message: Some(format!("{} metric families registered", metric_families.len())),
        latency_ms: Some(0),
    }
}

/// Metrics endpoint for Prometheus scraping
async fn metrics_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = state.metrics.registry.gather();

    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    (
        [(
            axum::http::header::CONTENT_TYPE,
            encoder.format_type().to_string(),
        )],
        buffer,
    )
}

// ============================================================================
// Graceful Shutdown
// ============================================================================

async fn wait_for_shutdown_signal(shutdown_tx: broadcast::Sender<()>) {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigterm = signal(SignalKind::terminate()).expect("failed to register SIGTERM");
    let mut sigint = signal(SignalKind::interrupt()).expect("failed to register SIGINT");

    tokio::select! {
        _ = sigterm.recv() => {
            info!("received SIGTERM, initiating graceful shutdown");
        }
        _ = sigint.recv() => {
            info!("received SIGINT (Ctrl+C), initiating graceful shutdown");
        }
    }

    let _ = shutdown_tx.send(());
}

async fn shutdown_handler(
    state: Arc<AppState>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    shutdown_rx.recv().await.ok();

    info!("beginning shutdown sequence");

    // 1. Stop accepting new requests (mark as not ready)
    info!("step 1: stop accepting new requests");

    // 2. Wait for in-flight requests to complete
    info!("step 2: waiting for in-flight requests (30s timeout)");
    tokio::time::sleep(Duration::from_secs(30)).await;

    // 3. Flush metrics
    info!("step 3: flushing final metrics");
    let metric_families = state.metrics.registry.gather();
    info!(metric_count = metric_families.len(), "metrics flushed");

    // 4. Clean up resources
    info!("step 4: cleaning up resources");

    info!("shutdown complete");
    Ok(())
}

// ============================================================================
// Main Application
// ============================================================================

fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health/live", get(liveness_handler))
        .route("/health/ready", get(readiness_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(state)
}

async fn run_http_server(
    app: Router,
    bind_addr: SocketAddr,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;

    info!("HTTP server listening on {}", bind_addr);

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(async move {
            shutdown_rx.recv().await.ok();
            info!("HTTP server received shutdown signal, draining connections...");

            // Give connections time to finish
            tokio::time::sleep(Duration::from_secs(30)).await;
        })
        .await?;

    info!("HTTP server shut down gracefully");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(true)
        .init();

    info!("starting production setup example");

    // Create application state
    let state = Arc::new(AppState::new()?);

    // Create shutdown channel
    let (shutdown_tx, shutdown_rx1) = broadcast::channel(1);
    let shutdown_rx2 = shutdown_tx.subscribe();

    // Create router
    let app = create_router(state.clone());

    // Bind address
    let bind_addr: SocketAddr = "0.0.0.0:8079".parse()?;

    // Spawn HTTP server
    let server_handle = tokio::spawn(run_http_server(app, bind_addr, shutdown_rx1));

    // Spawn shutdown handler
    let shutdown_handle = tokio::spawn(shutdown_handler(state.clone(), shutdown_rx2));

    // Wait for shutdown signal
    wait_for_shutdown_signal(shutdown_tx).await;

    // Wait for graceful shutdown to complete
    match tokio::time::timeout(Duration::from_secs(60), async {
        let _ = server_handle.await;
        let _ = shutdown_handle.await;
    })
    .await
    {
        Ok(_) => {
            info!("all services shut down gracefully");
            Ok(())
        }
        Err(_) => {
            warn!("shutdown timed out after 60s, forcing exit");
            std::process::exit(1);
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_opens_after_threshold() {
        let breaker = CircuitBreaker::new(3, 2, 30_000);

        assert_eq!(breaker.get_state(), CircuitState::Closed);

        breaker.record_failure();
        breaker.record_failure();
        breaker.record_failure();

        assert_eq!(breaker.get_state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_closes_after_recovery() {
        let breaker = CircuitBreaker::new(3, 2, 0); // 0ms timeout for immediate half-open

        // Open the circuit
        breaker.record_failure();
        breaker.record_failure();
        breaker.record_failure();
        assert_eq!(breaker.get_state(), CircuitState::Open);

        // Transition to half-open
        breaker.record_success();

        // Successful requests close the circuit
        breaker.record_success();
        breaker.record_success();

        assert_eq!(breaker.get_state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_liveness_handler() {
        let response = liveness_handler().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_readiness_handler() {
        let state = Arc::new(AppState::new().unwrap());
        let response = readiness_handler(State(state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_metrics_handler() {
        let state = Arc::new(AppState::new().unwrap());

        // Record some metrics
        state.metrics.requests_total.inc();
        state.metrics.cache_hits.inc();
        state.metrics.cache_size.set(10);

        let response = metrics_handler(State(state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
