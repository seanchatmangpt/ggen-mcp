pub mod analysis;
pub mod audit;
pub mod caps;
pub mod codegen;
pub mod config;
#[cfg(feature = "recalc")]
pub mod diff;
pub mod dod;
pub mod domain;
pub mod entitlement;
pub mod error;
#[cfg(feature = "recalc")]
pub mod fork;
pub mod formula;
pub mod generated;
pub mod guards;
pub mod health;
pub mod logging;
pub mod metrics;
pub mod model;
pub mod ontology;
#[cfg(feature = "recalc")]
pub mod recalc;
pub mod recovery;
pub mod server;
pub mod shutdown;
pub mod sparql;
pub mod state;
pub mod styles;
pub mod template;
pub mod tools;
pub mod utils;
pub mod validation;
pub mod workbook;

pub use config::{CliArgs, ServerConfig, TransportKind};
pub use error::{ERROR_METRICS, ErrorCode, ErrorMetrics, McpError, to_mcp_error, to_rmcp_error};
pub use logging::{LoggingConfig, init_logging, shutdown_telemetry};
pub use server::SpreadsheetServer;
pub use shutdown::{ShutdownConfig, ShutdownCoordinator};

use anyhow::Result;
use audit::{AuditConfig, init_audit_logger};
use axum::Router;
use model::WorkbookListResponse;
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};
use state::AppState;
use std::{future::IntoFuture, sync::Arc};
use tokio::{
    net::TcpListener,
    time::{Duration, timeout},
};
use tools::filters::WorkbookFilter;

const HTTP_SERVICE_PATH: &str = "/mcp";

pub async fn run_server(config: ServerConfig) -> Result<()> {
    let config = Arc::new(config);
    config.ensure_workspace_root()?;

    // Initialize audit logger
    let audit_config = AuditConfig::default();
    if let Err(e) = init_audit_logger(audit_config) {
        tracing::warn!("failed to initialize audit logger: {}", e);
    } else {
        tracing::info!("audit trail logging enabled");
    }

    let state = Arc::new(AppState::new(config.clone()));

    tracing::info!(
        transport = %config.transport,
        workspace = %config.workspace_root.display(),
        "starting spreadsheet MCP server",
    );

    match startup_scan(&state) {
        Ok(response) => {
            let count = response.workbooks.len();
            if count == 0 {
                tracing::info!("startup scan complete: no workbooks discovered");
            } else {
                let sample = response
                    .workbooks
                    .iter()
                    .take(3)
                    .map(|descriptor| descriptor.path.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                tracing::info!(
                    workbook_count = count,
                    sample = %sample,
                    "startup scan discovered workbooks"
                );
            }
        }
        Err(error) => {
            tracing::warn!(?error, "startup scan failed");
        }
    }

    match config.transport {
        TransportKind::Stdio => {
            let server = SpreadsheetServer::from_state(state);
            server.run_stdio().await
        }
        TransportKind::Http => run_stream_http_transport(config, state).await,
    }
}

/// Prometheus metrics endpoint handler
async fn metrics_handler() -> (axum::http::StatusCode, String) {
    let metrics_text = metrics::METRICS.encode();
    (axum::http::StatusCode::OK, metrics_text)
}

async fn run_stream_http_transport(config: Arc<ServerConfig>, state: Arc<AppState>) -> Result<()> {
    use shutdown::{
        AppStateShutdownHandler, AuditShutdownHandler, CompositeShutdownHandler, ShutdownHandler,
    };

    // Create shutdown coordinator
    let shutdown_config =
        ShutdownConfig::default().with_total_timeout(config.graceful_shutdown_timeout_secs);
    let coordinator = Arc::new(ShutdownCoordinator::new(shutdown_config));

    // Setup composite shutdown handler
    let mut composite_handler = CompositeShutdownHandler::new();
    composite_handler.add_handler(Box::new(AppStateShutdownHandler::new(state.clone())));
    composite_handler.add_handler(Box::new(AuditShutdownHandler::new()));

    #[cfg(feature = "recalc")]
    if let Some(backend) = state.recalc_backend() {
        use shutdown::LibreOfficeShutdownHandler;
        composite_handler.add_handler(Box::new(LibreOfficeShutdownHandler::new(backend.clone())));
    }

    let composite_handler = Arc::new(composite_handler);

    let bind_addr = config.http_bind_address;
    let service_state = state.clone();
    let service = StreamableHttpService::new(
        move || Ok(SpreadsheetServer::from_state(service_state.clone())),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    // Create health checker
    let health_checker = Arc::new(health::HealthChecker::new(config.clone(), state.clone()));

    let router = Router::new()
        .nest_service(HTTP_SERVICE_PATH, service)
        .route("/health", axum::routing::get(health::liveness_handler))
        .route("/ready", axum::routing::get(health::readiness_handler))
        .route(
            "/health/components",
            axum::routing::get(health::components_handler),
        )
        .route("/metrics", axum::routing::get(metrics_handler))
        .with_state(health_checker);
    let listener = TcpListener::bind(bind_addr).await?;
    let actual_addr = listener.local_addr()?;
    tracing::info!(transport = "http", bind = %actual_addr, path = HTTP_SERVICE_PATH, "listening" );

    // Clone coordinator for graceful shutdown
    let shutdown_coordinator = coordinator.clone();

    // Spawn server with graceful shutdown
    let server_future = axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            shutdown_coordinator.wait_for_signal().await;
        })
        .into_future();

    tokio::pin!(server_future);

    // Wait for server to complete
    let server_result = server_future.await;

    // Perform component shutdown
    tracing::info!("server stopped, running shutdown handlers");
    if let Err(e) = composite_handler.shutdown().await {
        tracing::error!("error during shutdown: {}", e);
    }

    server_result.map_err(anyhow::Error::from)
}

pub fn startup_scan(state: &Arc<AppState>) -> Result<WorkbookListResponse> {
    state.list_workbooks(WorkbookFilter::default())
}
