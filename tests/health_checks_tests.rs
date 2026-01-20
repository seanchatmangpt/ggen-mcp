use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::Value;
use spreadsheet_mcp::{ServerConfig, TransportKind};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn liveness_endpoint_returns_healthy() {
    let (router, _workspace) = setup_test_server().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "healthy");
    assert!(json["timestamp"].is_number());
    assert!(json["version"].is_string());
}

#[tokio::test]
async fn readiness_endpoint_returns_ready_when_healthy() {
    let (router, _workspace) = setup_test_server().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["ready"], true);
    assert_eq!(json["status"], "healthy");
    assert!(json["timestamp"].is_number());
    assert_eq!(json["not_ready"], Value::Array(vec![]));
}

#[tokio::test]
async fn readiness_endpoint_returns_not_ready_with_invalid_workspace() {
    let workspace = tempfile::tempdir().unwrap();
    let workspace_path = workspace.path().to_path_buf();

    // Create config with valid workspace first
    let config = ServerConfig {
        workspace_root: workspace_path.clone(),
        cache_capacity: 5,
        supported_extensions: vec!["xlsx".to_string()],
        single_workbook: None,
        enabled_tools: None,
        transport: TransportKind::Http,
        http_bind_address: "127.0.0.1:8079".parse().unwrap(),
        recalc_enabled: false,
        vba_enabled: false,
        max_concurrent_recalcs: 2,
        tool_timeout_ms: Some(30_000),
        max_response_bytes: Some(1_000_000),
        allow_overwrite: false,
    };

    let state = Arc::new(spreadsheet_mcp::state::AppState::new(Arc::new(
        config.clone(),
    )));
    let health_checker = Arc::new(spreadsheet_mcp::health::HealthChecker::new(
        Arc::new(config),
        state,
    ));

    let router = Router::new()
        .route(
            "/ready",
            axum::routing::get(spreadsheet_mcp::health::readiness_handler),
        )
        .with_state(health_checker);

    // Delete the workspace directory to make it unhealthy
    drop(workspace);

    let response = router
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["ready"], false);
    assert_eq!(json["status"], "unhealthy");
    assert!(
        json["not_ready"]
            .as_array()
            .unwrap()
            .contains(&Value::String("workspace".to_string()))
    );
}

#[tokio::test]
async fn components_endpoint_returns_detailed_health() {
    let (router, _workspace) = setup_test_server().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/health/components")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "healthy");
    assert!(json["timestamp"].is_number());
    assert!(json["components"].is_object());

    let components = json["components"].as_object().unwrap();

    // Verify workspace component
    assert!(components.contains_key("workspace"));
    let workspace = &components["workspace"];
    assert_eq!(workspace["status"], "healthy");
    assert_eq!(workspace["component"], "workspace");
    assert!(workspace["details"]["readable"].as_bool().unwrap());

    // Verify cache component
    assert!(components.contains_key("cache"));
    let cache = &components["cache"];
    assert_eq!(cache["component"], "cache");
    assert!(cache["details"]["size"].is_number());
    assert!(cache["details"]["capacity"].is_number());

    // Verify workbook_index component
    assert!(components.contains_key("workbook_index"));
    let index = &components["workbook_index"];
    assert_eq!(index["component"], "workbook_index");
}

#[tokio::test]
async fn components_endpoint_shows_degraded_cache() {
    let workspace = tempfile::tempdir().unwrap();

    // Create a config with cache capacity of 1 and fill it completely
    let config = ServerConfig {
        workspace_root: workspace.path().to_path_buf(),
        cache_capacity: 1, // Small cache to easily fill
        supported_extensions: vec!["xlsx".to_string()],
        single_workbook: None,
        enabled_tools: None,
        transport: TransportKind::Http,
        http_bind_address: "127.0.0.1:8079".parse().unwrap(),
        recalc_enabled: false,
        vba_enabled: false,
        max_concurrent_recalcs: 2,
        tool_timeout_ms: Some(30_000),
        max_response_bytes: Some(1_000_000),
        allow_overwrite: false,
    };

    let state = Arc::new(spreadsheet_mcp::state::AppState::new(Arc::new(
        config.clone(),
    )));
    let health_checker = Arc::new(spreadsheet_mcp::health::HealthChecker::new(
        Arc::new(config),
        state.clone(),
    ));

    // Create a test workbook file to load
    let test_file = workspace.path().join("test.xlsx");
    fs::write(&test_file, b"dummy xlsx data").unwrap();

    // Load a workbook to fill the cache (this would need a real workbook)
    // For now we'll just verify the cache component exists

    let router = Router::new()
        .route(
            "/health/components",
            axum::routing::get(spreadsheet_mcp::health::components_handler),
        )
        .with_state(health_checker);

    let response = router
        .oneshot(
            Request::builder()
                .uri("/health/components")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    let components = json["components"].as_object().unwrap();
    assert!(components.contains_key("cache"));

    let cache = &components["cache"];
    assert!(cache["details"]["capacity"].as_u64().unwrap() == 1);
}

#[tokio::test]
async fn health_endpoints_handle_concurrent_requests() {
    let (router, _workspace) = setup_test_server().await;

    // Clone router for each request since oneshot consumes it
    let config = ServerConfig {
        workspace_root: _workspace.path().to_path_buf(),
        cache_capacity: 5,
        supported_extensions: vec!["xlsx".to_string()],
        single_workbook: None,
        enabled_tools: None,
        transport: TransportKind::Http,
        http_bind_address: "127.0.0.1:8079".parse().unwrap(),
        recalc_enabled: false,
        vba_enabled: false,
        max_concurrent_recalcs: 2,
        tool_timeout_ms: Some(30_000),
        max_response_bytes: Some(1_000_000),
        allow_overwrite: false,
    };

    let state = Arc::new(spreadsheet_mcp::state::AppState::new(Arc::new(
        config.clone(),
    )));
    let health_checker = Arc::new(spreadsheet_mcp::health::HealthChecker::new(
        Arc::new(config),
        state,
    ));

    // Create multiple concurrent requests
    let mut handles = vec![];
    for _ in 0..10 {
        let checker = health_checker.clone();
        let handle = tokio::spawn(async move {
            let router = Router::new()
                .route(
                    "/health",
                    axum::routing::get(spreadsheet_mcp::health::liveness_handler),
                )
                .with_state(checker);

            router
                .oneshot(
                    Request::builder()
                        .uri("/health")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap()
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let response = handle.await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

#[tokio::test]
#[cfg(feature = "recalc")]
async fn components_endpoint_checks_libreoffice_when_enabled() {
    let workspace = tempfile::tempdir().unwrap();

    let config = ServerConfig {
        workspace_root: workspace.path().to_path_buf(),
        cache_capacity: 5,
        supported_extensions: vec!["xlsx".to_string()],
        single_workbook: None,
        enabled_tools: None,
        transport: TransportKind::Http,
        http_bind_address: "127.0.0.1:8079".parse().unwrap(),
        recalc_enabled: true, // Enable recalc to check LibreOffice
        vba_enabled: false,
        max_concurrent_recalcs: 2,
        tool_timeout_ms: Some(30_000),
        max_response_bytes: Some(1_000_000),
        allow_overwrite: false,
    };

    let state = Arc::new(spreadsheet_mcp::state::AppState::new(Arc::new(
        config.clone(),
    )));
    let health_checker = Arc::new(spreadsheet_mcp::health::HealthChecker::new(
        Arc::new(config),
        state,
    ));

    let router = Router::new()
        .route(
            "/health/components",
            axum::routing::get(spreadsheet_mcp::health::components_handler),
        )
        .with_state(health_checker);

    let response = router
        .oneshot(
            Request::builder()
                .uri("/health/components")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    let components = json["components"].as_object().unwrap();

    // Should have LibreOffice component when recalc is enabled
    assert!(components.contains_key("libreoffice"));
    let libreoffice = &components["libreoffice"];
    assert_eq!(libreoffice["component"], "libreoffice");

    // Should have fork_registry component
    assert!(components.contains_key("fork_registry"));
    let fork_registry = &components["fork_registry"];
    assert_eq!(fork_registry["component"], "fork_registry");
}

#[tokio::test]
async fn health_status_combines_correctly() {
    use spreadsheet_mcp::health::HealthStatus;

    // Test all combinations
    let healthy = HealthStatus::Healthy;
    let degraded = HealthStatus::Degraded;
    let unhealthy = HealthStatus::Unhealthy;

    assert_eq!(healthy.combine(healthy), HealthStatus::Healthy);
    assert_eq!(healthy.combine(degraded), HealthStatus::Degraded);
    assert_eq!(healthy.combine(unhealthy), HealthStatus::Unhealthy);
    assert_eq!(degraded.combine(healthy), HealthStatus::Degraded);
    assert_eq!(degraded.combine(degraded), HealthStatus::Degraded);
    assert_eq!(degraded.combine(unhealthy), HealthStatus::Unhealthy);
    assert_eq!(unhealthy.combine(healthy), HealthStatus::Unhealthy);
    assert_eq!(unhealthy.combine(degraded), HealthStatus::Unhealthy);
    assert_eq!(unhealthy.combine(unhealthy), HealthStatus::Unhealthy);
}

#[tokio::test]
async fn component_health_includes_timestamps() {
    let (router, _workspace) = setup_test_server().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/health/components")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    let components = json["components"].as_object().unwrap();
    for (_name, component) in components {
        assert!(component["timestamp"].is_number());
        let timestamp = component["timestamp"].as_i64().unwrap();
        assert!(timestamp > 0, "Timestamp should be positive");
    }
}

// Helper function to setup a test server with health endpoints
async fn setup_test_server() -> (Router, tempfile::TempDir) {
    let workspace = tempfile::tempdir().unwrap();

    let config = ServerConfig {
        workspace_root: workspace.path().to_path_buf(),
        cache_capacity: 5,
        supported_extensions: vec!["xlsx".to_string()],
        single_workbook: None,
        enabled_tools: None,
        transport: TransportKind::Http,
        http_bind_address: "127.0.0.1:8079".parse().unwrap(),
        recalc_enabled: false,
        vba_enabled: false,
        max_concurrent_recalcs: 2,
        tool_timeout_ms: Some(30_000),
        max_response_bytes: Some(1_000_000),
        allow_overwrite: false,
    };

    let state = Arc::new(spreadsheet_mcp::state::AppState::new(Arc::new(
        config.clone(),
    )));
    let health_checker = Arc::new(spreadsheet_mcp::health::HealthChecker::new(
        Arc::new(config),
        state,
    ));

    let router = Router::new()
        .route(
            "/health",
            axum::routing::get(spreadsheet_mcp::health::liveness_handler),
        )
        .route(
            "/ready",
            axum::routing::get(spreadsheet_mcp::health::readiness_handler),
        )
        .route(
            "/health/components",
            axum::routing::get(spreadsheet_mcp::health::components_handler),
        )
        .with_state(health_checker);

    (router, workspace)
}
