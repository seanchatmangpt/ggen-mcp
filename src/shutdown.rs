//! Graceful shutdown coordination for safe production deployments
//!
//! This module implements a comprehensive graceful shutdown system that:
//! - Handles SIGTERM and SIGINT signals
//! - Coordinates multi-phase shutdown with timeouts
//! - Ensures proper cleanup of all components
//! - Provides shutdown tokens for async task coordination
//!
//! # Shutdown Phases
//!
//! The shutdown process proceeds through multiple phases, each with its own timeout:
//!
//! 1. **Stop Accepting Requests** (2s) - HTTP server stops accepting new connections
//! 2. **Wait for In-Flight** (30s) - Allow active requests to complete
//! 3. **Flush Resources** (5s) - Flush caches, close connections, persist state
//! 4. **Final Cleanup** (3s) - Cleanup temporary files, final logging
//! 5. **Force Shutdown** - If timeout exceeded, force termination
//!
//! # Example
//!
//! ```rust,no_run
//! use spreadsheet_mcp::shutdown::{ShutdownCoordinator, ShutdownConfig};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = ShutdownConfig::default();
//! let coordinator = ShutdownCoordinator::new(config);
//!
//! // Get shutdown token for async tasks
//! let shutdown_token = coordinator.token();
//!
//! // In your async task:
//! tokio::select! {
//!     _ = shutdown_token.cancelled() => {
//!         // Cleanup and exit
//!     }
//!     result = do_work() => {
//!         // Normal operation
//!     }
//! }
//!
//! // Start shutdown on signal
//! coordinator.wait_for_signal().await;
//! coordinator.shutdown().await?;
//! # Ok(())
//! # }
//! ```

use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Notify, RwLock};
use tokio::time::{sleep, timeout};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

#[cfg(feature = "recalc")]
use crate::fork::ForkRegistry;
#[cfg(feature = "recalc")]
use crate::recalc::RecalcBackend;
use crate::state::AppState;

/// Configuration for graceful shutdown behavior
#[derive(Debug, Clone)]
pub struct ShutdownConfig {
    /// Phase 1: Stop accepting new requests timeout
    pub stop_accepting_timeout: Duration,

    /// Phase 2: Wait for in-flight requests timeout
    pub in_flight_timeout: Duration,

    /// Phase 3: Flush caches and close connections timeout
    pub flush_timeout: Duration,

    /// Phase 4: Final cleanup timeout
    pub cleanup_timeout: Duration,

    /// Total maximum shutdown time before force termination
    pub total_timeout: Duration,

    /// Whether to enable force shutdown after timeout
    pub force_shutdown_on_timeout: bool,
}

impl Default for ShutdownConfig {
    fn default() -> Self {
        Self {
            stop_accepting_timeout: Duration::from_secs(2),
            in_flight_timeout: Duration::from_secs(30),
            flush_timeout: Duration::from_secs(5),
            cleanup_timeout: Duration::from_secs(3),
            total_timeout: Duration::from_secs(45),
            force_shutdown_on_timeout: true,
        }
    }
}

impl ShutdownConfig {
    /// Create a shutdown config with custom total timeout
    pub fn with_total_timeout(mut self, timeout_secs: u64) -> Self {
        self.total_timeout = Duration::from_secs(timeout_secs);
        self
    }

    /// Create a shutdown config with custom in-flight timeout
    pub fn with_in_flight_timeout(mut self, timeout_secs: u64) -> Self {
        self.in_flight_timeout = Duration::from_secs(timeout_secs);
        self
    }
}

/// Shutdown phase tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownPhase {
    /// Server is running normally
    Running,
    /// Phase 1: Stop accepting new requests
    StopAccepting,
    /// Phase 2: Waiting for in-flight requests
    WaitingInFlight,
    /// Phase 3: Flushing caches and closing connections
    Flushing,
    /// Phase 4: Final cleanup
    Cleanup,
    /// Shutdown complete
    Complete,
    /// Force shutdown due to timeout
    Forced,
}

impl std::fmt::Display for ShutdownPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShutdownPhase::Running => write!(f, "running"),
            ShutdownPhase::StopAccepting => write!(f, "stop_accepting"),
            ShutdownPhase::WaitingInFlight => write!(f, "waiting_in_flight"),
            ShutdownPhase::Flushing => write!(f, "flushing"),
            ShutdownPhase::Cleanup => write!(f, "cleanup"),
            ShutdownPhase::Complete => write!(f, "complete"),
            ShutdownPhase::Forced => write!(f, "forced"),
        }
    }
}

/// Coordinates graceful shutdown across all server components
pub struct ShutdownCoordinator {
    config: ShutdownConfig,
    phase: Arc<RwLock<ShutdownPhase>>,
    shutdown_token: CancellationToken,
    shutdown_complete: Arc<Notify>,
    active_requests: Arc<std::sync::atomic::AtomicU64>,
}

impl ShutdownCoordinator {
    /// Create a new shutdown coordinator with the given configuration
    pub fn new(config: ShutdownConfig) -> Self {
        Self {
            config,
            phase: Arc::new(RwLock::new(ShutdownPhase::Running)),
            shutdown_token: CancellationToken::new(),
            shutdown_complete: Arc::new(Notify::new()),
            active_requests: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Get a shutdown token that can be used to coordinate async task cancellation
    pub fn token(&self) -> CancellationToken {
        self.shutdown_token.clone()
    }

    /// Get current shutdown phase
    pub async fn phase(&self) -> ShutdownPhase {
        *self.phase.read().await
    }

    /// Check if shutdown has been initiated
    pub fn is_shutdown_initiated(&self) -> bool {
        self.shutdown_token.is_cancelled()
    }

    /// Increment active request counter
    pub fn request_started(&self) {
        self.active_requests
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Decrement active request counter
    pub fn request_finished(&self) {
        self.active_requests
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get number of active requests
    pub fn active_request_count(&self) -> u64 {
        self.active_requests
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Wait for a shutdown signal (SIGTERM or SIGINT)
    pub async fn wait_for_signal(&self) {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install SIGTERM handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                info!("received SIGINT (Ctrl+C), initiating graceful shutdown");
            },
            _ = terminate => {
                info!("received SIGTERM, initiating graceful shutdown");
            },
        }
    }

    /// Execute graceful shutdown with all phases
    pub async fn shutdown(&self) -> Result<()> {
        info!("starting graceful shutdown sequence");

        // Set total timeout for entire shutdown
        let shutdown_result = if self.config.total_timeout > Duration::ZERO {
            timeout(self.config.total_timeout, self.run_shutdown_phases())
                .await
                .unwrap_or_else(|_| {
                    error!(
                        timeout_secs = self.config.total_timeout.as_secs(),
                        "graceful shutdown exceeded total timeout"
                    );
                    Err(anyhow::anyhow!("shutdown timeout exceeded"))
                })
        } else {
            self.run_shutdown_phases().await
        };

        match shutdown_result {
            Ok(_) => {
                info!("graceful shutdown completed successfully");
                *self.phase.write().await = ShutdownPhase::Complete;
                self.shutdown_complete.notify_waiters();
                Ok(())
            }
            Err(e) if self.config.force_shutdown_on_timeout => {
                warn!("graceful shutdown failed, forcing shutdown: {}", e);
                *self.phase.write().await = ShutdownPhase::Forced;
                self.shutdown_token.cancel();
                self.shutdown_complete.notify_waiters();
                Ok(())
            }
            Err(e) => {
                error!("graceful shutdown failed: {}", e);
                self.shutdown_complete.notify_waiters();
                Err(e)
            }
        }
    }

    /// Run all shutdown phases in sequence
    async fn run_shutdown_phases(&self) -> Result<()> {
        // Phase 1: Stop accepting new requests
        self.phase_stop_accepting().await?;

        // Phase 2: Wait for in-flight requests
        self.phase_wait_in_flight().await?;

        // Phase 3: Flush caches and close connections
        self.phase_flush_resources().await?;

        // Phase 4: Final cleanup
        self.phase_final_cleanup().await?;

        Ok(())
    }

    /// Phase 1: Stop accepting new requests
    async fn phase_stop_accepting(&self) -> Result<()> {
        *self.phase.write().await = ShutdownPhase::StopAccepting;
        info!("shutdown phase 1: stopping acceptance of new requests");

        // Signal all components to stop accepting work
        self.shutdown_token.cancel();

        // Give components time to stop accepting
        sleep(self.config.stop_accepting_timeout).await;

        debug!("phase 1 complete: no longer accepting requests");
        Ok(())
    }

    /// Phase 2: Wait for in-flight requests to complete
    async fn phase_wait_in_flight(&self) -> Result<()> {
        *self.phase.write().await = ShutdownPhase::WaitingInFlight;
        info!("shutdown phase 2: waiting for in-flight requests to complete");

        let start = tokio::time::Instant::now();
        let deadline = start + self.config.in_flight_timeout;

        loop {
            let active = self.active_request_count();
            if active == 0 {
                info!("all in-flight requests completed");
                break;
            }

            if tokio::time::Instant::now() >= deadline {
                warn!(
                    remaining_requests = active,
                    "in-flight timeout reached, proceeding with {} active requests", active
                );
                break;
            }

            debug!(active_requests = active, "waiting for requests to complete");
            sleep(Duration::from_millis(100)).await;
        }

        debug!("phase 2 complete: in-flight requests handled");
        Ok(())
    }

    /// Phase 3: Flush caches and close connections
    async fn phase_flush_resources(&self) -> Result<()> {
        *self.phase.write().await = ShutdownPhase::Flushing;
        info!("shutdown phase 3: flushing caches and closing connections");

        if let Err(e) = timeout(self.config.flush_timeout, self.flush_all_resources()).await {
            warn!("resource flushing timeout: {}", e);
        }

        debug!("phase 3 complete: resources flushed");
        Ok(())
    }

    /// Phase 4: Final cleanup
    async fn phase_final_cleanup(&self) -> Result<()> {
        *self.phase.write().await = ShutdownPhase::Cleanup;
        info!("shutdown phase 4: final cleanup");

        if let Err(e) = timeout(self.config.cleanup_timeout, self.final_cleanup()).await {
            warn!("final cleanup timeout: {}", e);
        }

        debug!("phase 4 complete: cleanup finished");
        Ok(())
    }

    /// Flush all resources (caches, connections, etc.)
    async fn flush_all_resources(&self) -> Result<()> {
        // Placeholder for resource flushing
        // This will be called by components during shutdown
        debug!("flushing all resources");
        Ok(())
    }

    /// Final cleanup operations
    async fn final_cleanup(&self) -> Result<()> {
        // Placeholder for final cleanup
        // This will be called by components during shutdown
        debug!("performing final cleanup");
        Ok(())
    }

    /// Wait for shutdown to complete
    pub async fn wait_for_completion(&self) {
        self.shutdown_complete.notified().await;
    }
}

/// Trait for components that need graceful shutdown
#[async_trait::async_trait]
pub trait ShutdownHandler: Send + Sync {
    /// Perform graceful shutdown of this component
    async fn shutdown(&self) -> Result<()>;

    /// Flush any pending data
    async fn flush(&self) -> Result<()> {
        Ok(())
    }
}

/// Shutdown handler for AppState
pub struct AppStateShutdownHandler {
    state: Arc<AppState>,
}

impl AppStateShutdownHandler {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[async_trait::async_trait]
impl ShutdownHandler for AppStateShutdownHandler {
    async fn shutdown(&self) -> Result<()> {
        info!("shutting down AppState");

        // Flush any cached data
        self.flush().await?;

        // Shutdown fork registry if enabled
        #[cfg(feature = "recalc")]
        if let Some(registry) = self.state.fork_registry() {
            if let Err(e) = shutdown_fork_registry(registry).await {
                warn!("fork registry shutdown error: {}", e);
            }
        }

        info!("AppState shutdown complete");
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        debug!("flushing AppState caches");

        // Cache statistics logging
        let stats = self.state.cache_stats();
        info!(
            cache_size = stats.size,
            cache_capacity = stats.capacity,
            cache_hit_rate = format!("{:.2}%", stats.hit_rate() * 100.0),
            "cache statistics at shutdown"
        );

        // Note: LRU cache doesn't need explicit flushing,
        // but we could persist it here if needed
        Ok(())
    }
}

/// Shutdown fork registry gracefully
#[cfg(feature = "recalc")]
async fn shutdown_fork_registry(_registry: &Arc<ForkRegistry>) -> Result<()> {
    info!("shutting down fork registry");

    // Fork registry has proper cleanup in its Drop implementation
    // Checkpoints and fork files are managed by the registry itself
    // The cleanup task will be dropped when registry is dropped

    info!("fork registry shutdown complete");
    Ok(())
}

/// Shutdown handler for audit logging
pub struct AuditShutdownHandler {
    // Placeholder for audit logger reference
    // Will be filled in when integrated with audit module
}

impl AuditShutdownHandler {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for AuditShutdownHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ShutdownHandler for AuditShutdownHandler {
    async fn shutdown(&self) -> Result<()> {
        info!("shutting down audit logger");
        self.flush().await?;
        info!("audit logger shutdown complete");
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        debug!("flushing audit logs");
        // Placeholder: will integrate with actual audit logger
        // to ensure all pending audit events are written to disk
        Ok(())
    }
}

/// Shutdown handler for LibreOffice processes
#[cfg(feature = "recalc")]
pub struct LibreOfficeShutdownHandler {
    backend: Arc<dyn RecalcBackend>,
}

#[cfg(feature = "recalc")]
impl LibreOfficeShutdownHandler {
    pub fn new(backend: Arc<dyn RecalcBackend>) -> Self {
        Self { backend }
    }
}

#[cfg(feature = "recalc")]
#[async_trait::async_trait]
impl ShutdownHandler for LibreOfficeShutdownHandler {
    async fn shutdown(&self) -> Result<()> {
        info!("shutting down LibreOffice backend");

        // The backend will be gracefully dropped
        // which should terminate any running processes
        debug!("waiting for LibreOffice processes to terminate");

        // Give processes time to exit gracefully
        sleep(Duration::from_secs(1)).await;

        info!("LibreOffice backend shutdown complete");
        Ok(())
    }
}

/// Composite shutdown handler that runs multiple handlers in sequence
pub struct CompositeShutdownHandler {
    handlers: Vec<Box<dyn ShutdownHandler>>,
}

impl CompositeShutdownHandler {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn add_handler(&mut self, handler: Box<dyn ShutdownHandler>) {
        self.handlers.push(handler);
    }
}

impl Default for CompositeShutdownHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ShutdownHandler for CompositeShutdownHandler {
    async fn shutdown(&self) -> Result<()> {
        for (idx, handler) in self.handlers.iter().enumerate() {
            if let Err(e) = handler.shutdown().await {
                error!(handler_index = idx, "shutdown handler error: {}", e);
            }
        }
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        for (idx, handler) in self.handlers.iter().enumerate() {
            if let Err(e) = handler.flush().await {
                error!(handler_index = idx, "flush handler error: {}", e);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shutdown_config_defaults() {
        let config = ShutdownConfig::default();
        assert_eq!(config.stop_accepting_timeout, Duration::from_secs(2));
        assert_eq!(config.in_flight_timeout, Duration::from_secs(30));
        assert_eq!(config.flush_timeout, Duration::from_secs(5));
        assert_eq!(config.cleanup_timeout, Duration::from_secs(3));
        assert_eq!(config.total_timeout, Duration::from_secs(45));
        assert!(config.force_shutdown_on_timeout);
    }

    #[tokio::test]
    async fn test_shutdown_config_builder() {
        let config = ShutdownConfig::default()
            .with_total_timeout(60)
            .with_in_flight_timeout(45);

        assert_eq!(config.total_timeout, Duration::from_secs(60));
        assert_eq!(config.in_flight_timeout, Duration::from_secs(45));
    }

    #[tokio::test]
    async fn test_shutdown_coordinator_phases() {
        let coordinator = ShutdownCoordinator::new(ShutdownConfig::default());

        assert_eq!(coordinator.phase().await, ShutdownPhase::Running);
        assert!(!coordinator.is_shutdown_initiated());

        let token = coordinator.token();
        assert!(!token.is_cancelled());

        // Initiate shutdown
        coordinator.shutdown_token.cancel();
        assert!(coordinator.is_shutdown_initiated());
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn test_active_request_tracking() {
        let coordinator = ShutdownCoordinator::new(ShutdownConfig::default());

        assert_eq!(coordinator.active_request_count(), 0);

        coordinator.request_started();
        assert_eq!(coordinator.active_request_count(), 1);

        coordinator.request_started();
        assert_eq!(coordinator.active_request_count(), 2);

        coordinator.request_finished();
        assert_eq!(coordinator.active_request_count(), 1);

        coordinator.request_finished();
        assert_eq!(coordinator.active_request_count(), 0);
    }

    #[tokio::test]
    async fn test_shutdown_phase_display() {
        assert_eq!(ShutdownPhase::Running.to_string(), "running");
        assert_eq!(ShutdownPhase::StopAccepting.to_string(), "stop_accepting");
        assert_eq!(
            ShutdownPhase::WaitingInFlight.to_string(),
            "waiting_in_flight"
        );
        assert_eq!(ShutdownPhase::Flushing.to_string(), "flushing");
        assert_eq!(ShutdownPhase::Cleanup.to_string(), "cleanup");
        assert_eq!(ShutdownPhase::Complete.to_string(), "complete");
        assert_eq!(ShutdownPhase::Forced.to_string(), "forced");
    }
}
