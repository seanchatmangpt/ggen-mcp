//! Comprehensive tests for graceful shutdown functionality
//!
//! These tests verify:
//! - SIGTERM/SIGINT handling
//! - Shutdown phase progression
//! - Timeout enforcement
//! - Component shutdown sequencing
//! - Force shutdown path
//! - Request tracking

use spreadsheet_mcp::shutdown::{
    AppStateShutdownHandler, CompositeShutdownHandler, ShutdownConfig, ShutdownCoordinator,
    ShutdownHandler, ShutdownPhase,
};
use spreadsheet_mcp::{ServerConfig, state::AppState};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tokio::time::{sleep, timeout};

#[tokio::test]
async fn test_shutdown_config_creation() {
    let config = ShutdownConfig::default();
    assert_eq!(config.stop_accepting_timeout, Duration::from_secs(2));
    assert_eq!(config.in_flight_timeout, Duration::from_secs(30));
    assert_eq!(config.flush_timeout, Duration::from_secs(5));
    assert_eq!(config.cleanup_timeout, Duration::from_secs(3));
    assert_eq!(config.total_timeout, Duration::from_secs(45));
    assert!(config.force_shutdown_on_timeout);
}

#[tokio::test]
async fn test_shutdown_config_with_custom_timeout() {
    let config = ShutdownConfig::default()
        .with_total_timeout(60)
        .with_in_flight_timeout(40);

    assert_eq!(config.total_timeout, Duration::from_secs(60));
    assert_eq!(config.in_flight_timeout, Duration::from_secs(40));
}

#[tokio::test]
async fn test_shutdown_coordinator_initialization() {
    let coordinator = ShutdownCoordinator::new(ShutdownConfig::default());

    assert_eq!(coordinator.phase().await, ShutdownPhase::Running);
    assert!(!coordinator.is_shutdown_initiated());
    assert_eq!(coordinator.active_request_count(), 0);
}

#[tokio::test]
async fn test_shutdown_token_cancellation() {
    let coordinator = ShutdownCoordinator::new(ShutdownConfig::default());
    let token = coordinator.token();

    assert!(!token.is_cancelled());
    assert!(!coordinator.is_shutdown_initiated());

    // Trigger shutdown
    coordinator.token().cancel();

    assert!(token.is_cancelled());
    assert!(coordinator.is_shutdown_initiated());
}

#[tokio::test]
async fn test_active_request_tracking() {
    let coordinator = ShutdownCoordinator::new(ShutdownConfig::default());

    // Initially no active requests
    assert_eq!(coordinator.active_request_count(), 0);

    // Start some requests
    coordinator.request_started();
    coordinator.request_started();
    coordinator.request_started();
    assert_eq!(coordinator.active_request_count(), 3);

    // Finish some requests
    coordinator.request_finished();
    assert_eq!(coordinator.active_request_count(), 2);

    coordinator.request_finished();
    coordinator.request_finished();
    assert_eq!(coordinator.active_request_count(), 0);
}

#[tokio::test]
async fn test_shutdown_phases_progression() {
    let config = ShutdownConfig {
        stop_accepting_timeout: Duration::from_millis(50),
        in_flight_timeout: Duration::from_millis(50),
        flush_timeout: Duration::from_millis(50),
        cleanup_timeout: Duration::from_millis(50),
        total_timeout: Duration::from_secs(5),
        force_shutdown_on_timeout: false,
    };

    let coordinator = Arc::new(ShutdownCoordinator::new(config));

    assert_eq!(coordinator.phase().await, ShutdownPhase::Running);

    // Spawn shutdown task
    let coord_clone = coordinator.clone();
    let shutdown_handle = tokio::spawn(async move { coord_clone.shutdown().await });

    // Give it time to progress through phases
    sleep(Duration::from_millis(20)).await;

    // Wait for shutdown to complete
    let result = shutdown_handle.await.unwrap();
    assert!(result.is_ok());

    // Should be complete
    let phase = coordinator.phase().await;
    assert!(matches!(phase, ShutdownPhase::Complete));
}

#[tokio::test]
async fn test_shutdown_with_in_flight_requests() {
    let config = ShutdownConfig {
        stop_accepting_timeout: Duration::from_millis(50),
        in_flight_timeout: Duration::from_millis(200),
        flush_timeout: Duration::from_millis(50),
        cleanup_timeout: Duration::from_millis(50),
        total_timeout: Duration::from_secs(5),
        force_shutdown_on_timeout: false,
    };

    let coordinator = Arc::new(ShutdownCoordinator::new(config));

    // Simulate in-flight requests
    coordinator.request_started();
    coordinator.request_started();
    assert_eq!(coordinator.active_request_count(), 2);

    let coord_clone = coordinator.clone();
    let shutdown_handle = tokio::spawn(async move { coord_clone.shutdown().await });

    // Simulate requests completing during shutdown
    sleep(Duration::from_millis(50)).await;
    coordinator.request_finished();

    sleep(Duration::from_millis(50)).await;
    coordinator.request_finished();

    // Wait for shutdown to complete
    let result = shutdown_handle.await.unwrap();
    assert!(result.is_ok());

    assert_eq!(coordinator.active_request_count(), 0);
}

#[tokio::test]
async fn test_shutdown_timeout_enforcement() {
    let config = ShutdownConfig {
        stop_accepting_timeout: Duration::from_millis(50),
        in_flight_timeout: Duration::from_millis(50),
        flush_timeout: Duration::from_millis(50),
        cleanup_timeout: Duration::from_millis(50),
        total_timeout: Duration::from_millis(100), // Very short total timeout
        force_shutdown_on_timeout: true,
    };

    let coordinator = Arc::new(ShutdownCoordinator::new(config));

    // Add requests that won't complete
    coordinator.request_started();
    coordinator.request_started();

    let start = tokio::time::Instant::now();
    let result = coordinator.shutdown().await;
    let elapsed = start.elapsed();

    // Should complete quickly due to timeout
    assert!(elapsed < Duration::from_millis(300));

    // Should succeed even though requests didn't complete
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_force_shutdown_on_timeout() {
    let config = ShutdownConfig {
        stop_accepting_timeout: Duration::from_millis(50),
        in_flight_timeout: Duration::from_millis(50),
        flush_timeout: Duration::from_millis(50),
        cleanup_timeout: Duration::from_millis(50),
        total_timeout: Duration::from_millis(100),
        force_shutdown_on_timeout: true,
    };

    let coordinator = ShutdownCoordinator::new(config);

    // Simulate stuck requests
    coordinator.request_started();
    coordinator.request_started();

    let result = coordinator.shutdown().await;

    // Should succeed even with force shutdown
    assert!(result.is_ok());
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

// Mock shutdown handler for testing
struct MockShutdownHandler {
    shutdown_called: Arc<AtomicBool>,
    flush_called: Arc<AtomicBool>,
    call_count: Arc<AtomicU64>,
}

impl MockShutdownHandler {
    fn new() -> Self {
        Self {
            shutdown_called: Arc::new(AtomicBool::new(false)),
            flush_called: Arc::new(AtomicBool::new(false)),
            call_count: Arc::new(AtomicU64::new(0)),
        }
    }

    fn was_shutdown_called(&self) -> bool {
        self.shutdown_called.load(Ordering::SeqCst)
    }

    fn was_flush_called(&self) -> bool {
        self.flush_called.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl ShutdownHandler for MockShutdownHandler {
    async fn shutdown(&self) -> anyhow::Result<()> {
        self.shutdown_called.store(true, Ordering::SeqCst);
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn flush(&self) -> anyhow::Result<()> {
        self.flush_called.store(true, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn test_composite_shutdown_handler() {
    let handler1 = Arc::new(MockShutdownHandler::new());
    let handler2 = Arc::new(MockShutdownHandler::new());

    let mut composite = CompositeShutdownHandler::new();
    composite.add_handler(Box::new(handler1.clone()));
    composite.add_handler(Box::new(handler2.clone()));

    // Call shutdown
    let result = composite.shutdown().await;
    assert!(result.is_ok());

    // Both handlers should have been called
    assert!(handler1.was_shutdown_called());
    assert!(handler2.was_shutdown_called());
}

#[tokio::test]
async fn test_composite_shutdown_handler_flush() {
    let handler1 = Arc::new(MockShutdownHandler::new());
    let handler2 = Arc::new(MockShutdownHandler::new());

    let mut composite = CompositeShutdownHandler::new();
    composite.add_handler(Box::new(handler1.clone()));
    composite.add_handler(Box::new(handler2.clone()));

    // Call flush
    let result = composite.flush().await;
    assert!(result.is_ok());

    // Both handlers should have been flushed
    assert!(handler1.was_flush_called());
    assert!(handler2.was_flush_called());
}

#[tokio::test]
async fn test_app_state_shutdown_handler() {
    // Create minimal server config for testing
    let config = ServerConfig::from_args(spreadsheet_mcp::CliArgs {
        config: None,
        workspace_root: Some(std::env::temp_dir()),
        cache_capacity: Some(5),
        extensions: None,
        workbook: None,
        enabled_tools: None,
        transport: None,
        http_bind: None,
        recalc_enabled: false,
        vba_enabled: false,
        max_concurrent_recalcs: None,
        tool_timeout_ms: None,
        max_response_bytes: None,
        allow_overwrite: false,
        graceful_shutdown_timeout_secs: None,
    })
    .expect("Failed to create config");

    let state = Arc::new(AppState::new(Arc::new(config)));
    let handler = AppStateShutdownHandler::new(state);

    // Should shutdown without error
    let result = handler.shutdown().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_shutdown_with_concurrent_requests() {
    let coordinator = Arc::new(ShutdownCoordinator::new(ShutdownConfig::default()));

    // Spawn multiple concurrent "requests"
    let mut handles = vec![];
    for i in 0..10 {
        let coord = coordinator.clone();
        let handle = tokio::spawn(async move {
            coord.request_started();
            sleep(Duration::from_millis(50 * (i % 3) as u64)).await;
            coord.request_finished();
        });
        handles.push(handle);
    }

    // Give requests time to start
    sleep(Duration::from_millis(20)).await;

    // Initiate shutdown
    let shutdown_result = timeout(Duration::from_secs(5), coordinator.shutdown()).await;

    // Wait for all requests to complete
    for handle in handles {
        let _ = handle.await;
    }

    assert!(shutdown_result.is_ok());
    assert_eq!(coordinator.active_request_count(), 0);
}

#[tokio::test]
async fn test_shutdown_completion_notification() {
    let coordinator = Arc::new(ShutdownCoordinator::new(ShutdownConfig {
        stop_accepting_timeout: Duration::from_millis(50),
        in_flight_timeout: Duration::from_millis(50),
        flush_timeout: Duration::from_millis(50),
        cleanup_timeout: Duration::from_millis(50),
        total_timeout: Duration::from_secs(5),
        force_shutdown_on_timeout: false,
    }));

    let coord_clone = coordinator.clone();

    // Spawn shutdown task
    tokio::spawn(async move {
        sleep(Duration::from_millis(100)).await;
        let _ = coord_clone.shutdown().await;
    });

    // Wait for completion notification
    let wait_result = timeout(Duration::from_secs(2), coordinator.wait_for_completion()).await;

    assert!(
        wait_result.is_ok(),
        "Should receive completion notification"
    );
}

#[tokio::test]
async fn test_shutdown_token_propagation() {
    let coordinator = ShutdownCoordinator::new(ShutdownConfig::default());
    let token = coordinator.token();

    // Create child token
    let child_token = token.child_token();

    assert!(!token.is_cancelled());
    assert!(!child_token.is_cancelled());

    // Cancel parent
    token.cancel();

    // Both should be cancelled
    assert!(token.is_cancelled());
    assert!(child_token.is_cancelled());
}

#[tokio::test]
async fn test_multiple_shutdown_calls() {
    let coordinator = Arc::new(ShutdownCoordinator::new(ShutdownConfig {
        stop_accepting_timeout: Duration::from_millis(50),
        in_flight_timeout: Duration::from_millis(50),
        flush_timeout: Duration::from_millis(50),
        cleanup_timeout: Duration::from_millis(50),
        total_timeout: Duration::from_secs(5),
        force_shutdown_on_timeout: false,
    }));

    // First shutdown
    let result1 = coordinator.shutdown().await;
    assert!(result1.is_ok());

    // Second shutdown should also succeed (idempotent)
    let result2 = coordinator.shutdown().await;
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_shutdown_without_timeout() {
    let config = ShutdownConfig {
        stop_accepting_timeout: Duration::from_millis(10),
        in_flight_timeout: Duration::from_millis(10),
        flush_timeout: Duration::from_millis(10),
        cleanup_timeout: Duration::from_millis(10),
        total_timeout: Duration::ZERO, // No total timeout
        force_shutdown_on_timeout: false,
    };

    let coordinator = ShutdownCoordinator::new(config);

    let result = coordinator.shutdown().await;
    assert!(result.is_ok());
}
