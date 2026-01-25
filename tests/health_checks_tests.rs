use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use chicago_tdd_tools::prelude::*;
use http_body_util::BodyExt;
use serde_json::Value;
use spreadsheet_mcp::{ServerConfig, TransportKind};
use std::fs;
use std::sync::Arc;
use tower::ServiceExt;

async_test_with_timeout!(liveness_endpoint_returns_healthy, 30, {
    // Arrange: Setup test server with health endpoints
    let (router, _workspace) = setup_test_server().await?;

    // Act: Send request to liveness endpoint
    let response = router
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .map_err(|e| format!("Failed to build request: {}", e))?,
        )
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .map_err(|e| format!("Failed to collect body: {}", e))?
        .to_bytes();
    let json: Value =
        serde_json::from_slice(&body).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // Assert: Verify response status and structure
    assert_eq!(status, StatusCode::OK, "Expected OK status");
    assert_eq!(json["status"], "healthy", "Expected healthy status");
    assert!(
        json["timestamp"].is_number(),
        "Expected timestamp to be a number"
    );
    assert!(
        json["version"].is_string(),
        "Expected version to be a string"
    );

    Ok::<(), Box<dyn std::error::Error>>(())
});

async_test_with_timeout!(readiness_endpoint_returns_ready_when_healthy, 30, {
    // Arrange: Setup test server with health endpoints
    let (router, _workspace) = setup_test_server().await?;

    // Act: Send request to readiness endpoint
    let response = router
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .map_err(|e| format!("Failed to build request: {}", e))?,
        )
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .map_err(|e| format!("Failed to collect body: {}", e))?
        .to_bytes();
    let json: Value =
        serde_json::from_slice(&body).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // Assert: Verify readiness response
    assert_eq!(status, StatusCode::OK, "Expected OK status");
    assert_eq!(json["ready"], true, "Expected ready to be true");
    assert_eq!(json["status"], "healthy", "Expected healthy status");
    assert!(
        json["timestamp"].is_number(),
        "Expected timestamp to be a number"
    );
    assert_eq!(
        json["not_ready"],
        Value::Array(vec![]),
        "Expected empty not_ready array"
    );

    Ok::<(), Box<dyn std::error::Error>>(())
});

async_test_with_timeout!(
    readiness_endpoint_returns_not_ready_with_invalid_workspace,
    30,
    {
        // Arrange: Create test server with workspace that will be deleted
        let workspace =
            tempfile::tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;
        let workspace_path = workspace.path().to_path_buf();

        // Create config with valid workspace first
        let config = ServerConfig {
            workspace_root: workspace_path.clone(),
            cache_capacity: 5,
            supported_extensions: vec!["xlsx".to_string()],
            single_workbook: None,
            enabled_tools: None,
            transport: TransportKind::Http,
            http_bind_address: "127.0.0.1:8079"
                .parse()
                .map_err(|e| format!("Failed to parse address: {}", e))?,
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

        // Act: Send request to readiness endpoint with deleted workspace
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .map_err(|e| format!("Failed to build request: {}", e))?,
            )
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let status = response.status();
        let body = response
            .into_body()
            .collect()
            .await
            .map_err(|e| format!("Failed to collect body: {}", e))?
            .to_bytes();
        let json: Value =
            serde_json::from_slice(&body).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        // Assert: Verify unhealthy response
        assert_eq!(
            status,
            StatusCode::SERVICE_UNAVAILABLE,
            "Expected SERVICE_UNAVAILABLE status"
        );
        assert_eq!(json["ready"], false, "Expected ready to be false");
        assert_eq!(json["status"], "unhealthy", "Expected unhealthy status");
        let not_ready = json["not_ready"]
            .as_array()
            .ok_or("Expected not_ready to be an array")?;
        assert!(
            not_ready.contains(&Value::String("workspace".to_string())),
            "Expected not_ready to contain 'workspace'"
        );

        Ok::<(), Box<dyn std::error::Error>>(())
    }
);

async_test_with_timeout!(components_endpoint_returns_detailed_health, 30, {
    // Arrange: Setup test server with health endpoints
    let (router, _workspace) = setup_test_server().await?;

    // Act: Send request to components endpoint
    let response = router
        .oneshot(
            Request::builder()
                .uri("/health/components")
                .body(Body::empty())
                .map_err(|e| format!("Failed to build request: {}", e))?,
        )
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .map_err(|e| format!("Failed to collect body: {}", e))?
        .to_bytes();
    let json: Value =
        serde_json::from_slice(&body).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // Assert: Verify component health details
    assert_eq!(status, StatusCode::OK, "Expected OK status");
    assert_eq!(json["status"], "healthy", "Expected healthy status");
    assert!(
        json["timestamp"].is_number(),
        "Expected timestamp to be a number"
    );
    assert!(
        json["components"].is_object(),
        "Expected components to be an object"
    );

    let components = json["components"]
        .as_object()
        .ok_or("Expected components to be an object")?;

    // Verify workspace component
    assert!(
        components.contains_key("workspace"),
        "Expected workspace component"
    );
    let workspace = &components["workspace"];
    assert_eq!(
        workspace["status"], "healthy",
        "Expected workspace to be healthy"
    );
    assert_eq!(
        workspace["component"], "workspace",
        "Expected component name to be workspace"
    );
    let readable = workspace["details"]["readable"]
        .as_bool()
        .ok_or("Expected readable to be a boolean")?;
    assert!(readable, "Expected workspace to be readable");

    // Verify cache component
    assert!(components.contains_key("cache"), "Expected cache component");
    let cache = &components["cache"];
    assert_eq!(
        cache["component"], "cache",
        "Expected component name to be cache"
    );
    assert!(
        cache["details"]["size"].is_number(),
        "Expected cache size to be a number"
    );
    assert!(
        cache["details"]["capacity"].is_number(),
        "Expected cache capacity to be a number"
    );

    // Verify workbook_index component
    assert!(
        components.contains_key("workbook_index"),
        "Expected workbook_index component"
    );
    let index = &components["workbook_index"];
    assert_eq!(
        index["component"], "workbook_index",
        "Expected component name to be workbook_index"
    );

    Ok::<(), Box<dyn std::error::Error>>(())
});

async_test_with_timeout!(components_endpoint_shows_degraded_cache, 30, {
    // Arrange: Create test server with small cache capacity
    let workspace = tempfile::tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;

    // Create a config with cache capacity of 1 and fill it completely
    let config = ServerConfig {
        workspace_root: workspace.path().to_path_buf(),
        cache_capacity: 1, // Small cache to easily fill
        supported_extensions: vec!["xlsx".to_string()],
        single_workbook: None,
        enabled_tools: None,
        transport: TransportKind::Http,
        http_bind_address: "127.0.0.1:8079"
            .parse()
            .map_err(|e| format!("Failed to parse address: {}", e))?,
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
    fs::write(&test_file, b"dummy xlsx data")
        .map_err(|e| format!("Failed to write test file: {}", e))?;

    let router = Router::new()
        .route(
            "/health/components",
            axum::routing::get(spreadsheet_mcp::health::components_handler),
        )
        .with_state(health_checker);

    // Act: Send request to components endpoint
    let response = router
        .oneshot(
            Request::builder()
                .uri("/health/components")
                .body(Body::empty())
                .map_err(|e| format!("Failed to build request: {}", e))?,
        )
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let body = response
        .into_body()
        .collect()
        .await
        .map_err(|e| format!("Failed to collect body: {}", e))?
        .to_bytes();
    let json: Value =
        serde_json::from_slice(&body).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // Assert: Verify cache capacity is set correctly
    let components = json["components"]
        .as_object()
        .ok_or("Expected components to be an object")?;
    assert!(components.contains_key("cache"), "Expected cache component");

    let cache = &components["cache"];
    let capacity = cache["details"]["capacity"]
        .as_u64()
        .ok_or("Expected capacity to be a number")?;
    assert_eq!(capacity, 1, "Expected cache capacity to be 1");

    Ok::<(), Box<dyn std::error::Error>>(())
});

async_test_with_timeout!(health_endpoints_handle_concurrent_requests, 30, {
    // Arrange: Setup test server and create multiple concurrent request handlers
    let (router, _workspace) = setup_test_server().await?;

    // Clone router for each request since oneshot consumes it
    let config = ServerConfig {
        workspace_root: _workspace.path().to_path_buf(),
        cache_capacity: 5,
        supported_extensions: vec!["xlsx".to_string()],
        single_workbook: None,
        enabled_tools: None,
        transport: TransportKind::Http,
        http_bind_address: "127.0.0.1:8079"
            .parse()
            .map_err(|e| format!("Failed to parse address: {}", e))?,
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

    // Act: Create multiple concurrent requests
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
                        .map_err(|e| format!("Failed to build request: {}", e))?,
                )
                .await
                .map_err(|e| format!("Request failed: {}", e))
        });
        handles.push(handle);
    }

    // Assert: Wait for all requests to complete successfully
    for handle in handles {
        let response = handle.await.map_err(|e| format!("Task failed: {}", e))??;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Expected OK status from concurrent request"
        );
    }

    Ok::<(), Box<dyn std::error::Error>>(())
});

#[cfg(feature = "recalc")]
async_test_with_timeout!(components_endpoint_checks_libreoffice_when_enabled, 30, {
    // Arrange: Create test server with recalc enabled
    let workspace = tempfile::tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let config = ServerConfig {
        workspace_root: workspace.path().to_path_buf(),
        cache_capacity: 5,
        supported_extensions: vec!["xlsx".to_string()],
        single_workbook: None,
        enabled_tools: None,
        transport: TransportKind::Http,
        http_bind_address: "127.0.0.1:8079"
            .parse()
            .map_err(|e| format!("Failed to parse address: {}", e))?,
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

    // Act: Send request to components endpoint
    let response = router
        .oneshot(
            Request::builder()
                .uri("/health/components")
                .body(Body::empty())
                .map_err(|e| format!("Failed to build request: {}", e))?,
        )
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let body = response
        .into_body()
        .collect()
        .await
        .map_err(|e| format!("Failed to collect body: {}", e))?
        .to_bytes();
    let json: Value =
        serde_json::from_slice(&body).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // Assert: Verify LibreOffice and fork_registry components are present
    let components = json["components"]
        .as_object()
        .ok_or("Expected components to be an object")?;

    // Should have LibreOffice component when recalc is enabled
    assert!(
        components.contains_key("libreoffice"),
        "Expected libreoffice component"
    );
    let libreoffice = &components["libreoffice"];
    assert_eq!(
        libreoffice["component"], "libreoffice",
        "Expected component name to be libreoffice"
    );

    // Should have fork_registry component
    assert!(
        components.contains_key("fork_registry"),
        "Expected fork_registry component"
    );
    let fork_registry = &components["fork_registry"];
    assert_eq!(
        fork_registry["component"], "fork_registry",
        "Expected component name to be fork_registry"
    );

    Ok::<(), Box<dyn std::error::Error>>(())
});

test!(health_status_combines_correctly, {
    use spreadsheet_mcp::health::HealthStatus;

    // Arrange: Create all possible health status values
    let healthy = HealthStatus::Healthy;
    let degraded = HealthStatus::Degraded;
    let unhealthy = HealthStatus::Unhealthy;

    // Act & Assert: Test all combinations of health status merging
    // Healthy combinations
    assert_eq!(
        healthy.combine(healthy),
        HealthStatus::Healthy,
        "Healthy + Healthy = Healthy"
    );
    assert_eq!(
        healthy.combine(degraded),
        HealthStatus::Degraded,
        "Healthy + Degraded = Degraded"
    );
    assert_eq!(
        healthy.combine(unhealthy),
        HealthStatus::Unhealthy,
        "Healthy + Unhealthy = Unhealthy"
    );

    // Degraded combinations
    assert_eq!(
        degraded.combine(healthy),
        HealthStatus::Degraded,
        "Degraded + Healthy = Degraded"
    );
    assert_eq!(
        degraded.combine(degraded),
        HealthStatus::Degraded,
        "Degraded + Degraded = Degraded"
    );
    assert_eq!(
        degraded.combine(unhealthy),
        HealthStatus::Unhealthy,
        "Degraded + Unhealthy = Unhealthy"
    );

    // Unhealthy combinations
    assert_eq!(
        unhealthy.combine(healthy),
        HealthStatus::Unhealthy,
        "Unhealthy + Healthy = Unhealthy"
    );
    assert_eq!(
        unhealthy.combine(degraded),
        HealthStatus::Unhealthy,
        "Unhealthy + Degraded = Unhealthy"
    );
    assert_eq!(
        unhealthy.combine(unhealthy),
        HealthStatus::Unhealthy,
        "Unhealthy + Unhealthy = Unhealthy"
    );
});

async_test_with_timeout!(component_health_includes_timestamps, 30, {
    // Arrange: Setup test server with health endpoints
    let (router, _workspace) = setup_test_server().await?;

    // Act: Send request to components endpoint
    let response = router
        .oneshot(
            Request::builder()
                .uri("/health/components")
                .body(Body::empty())
                .map_err(|e| format!("Failed to build request: {}", e))?,
        )
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let body = response
        .into_body()
        .collect()
        .await
        .map_err(|e| format!("Failed to collect body: {}", e))?
        .to_bytes();
    let json: Value =
        serde_json::from_slice(&body).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // Assert: Verify all components have valid timestamps
    let components = json["components"]
        .as_object()
        .ok_or("Expected components to be an object")?;

    for (_name, component) in components {
        assert!(
            component["timestamp"].is_number(),
            "Expected timestamp to be a number"
        );
        let timestamp = component["timestamp"]
            .as_i64()
            .ok_or("Expected timestamp to be an i64")?;
        assert!(
            timestamp > 0,
            "Timestamp should be positive, got {}",
            timestamp
        );
    }

    Ok::<(), Box<dyn std::error::Error>>(())
});

// Helper function to setup a test server with health endpoints
async fn setup_test_server() -> Result<(Router, tempfile::TempDir), Box<dyn std::error::Error>> {
    let workspace = tempfile::tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let config = ServerConfig {
        workspace_root: workspace.path().to_path_buf(),
        cache_capacity: 5,
        supported_extensions: vec!["xlsx".to_string()],
        single_workbook: None,
        enabled_tools: None,
        transport: TransportKind::Http,
        http_bind_address: "127.0.0.1:8079"
            .parse()
            .map_err(|e| format!("Failed to parse address: {}", e))?,
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

    Ok((router, workspace))
}
