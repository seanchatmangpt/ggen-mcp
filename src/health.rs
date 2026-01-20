use crate::config::ServerConfig;
use crate::state::AppState;
use anyhow::{Context, Result};
use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;

/// Health status for a component or the overall system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// Component is functioning normally
    Healthy,
    /// Component is functioning but with degraded performance or partial failures
    Degraded,
    /// Component is not functioning
    Unhealthy,
}

impl HealthStatus {
    /// Returns the HTTP status code for this health status
    pub fn status_code(&self) -> StatusCode {
        match self {
            HealthStatus::Healthy => StatusCode::OK,
            HealthStatus::Degraded => StatusCode::OK, // Still serve traffic but indicate degradation
            HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    /// Combines two health statuses, returning the worse of the two
    pub fn combine(self, other: Self) -> Self {
        match (self, other) {
            (HealthStatus::Unhealthy, _) | (_, HealthStatus::Unhealthy) => HealthStatus::Unhealthy,
            (HealthStatus::Degraded, _) | (_, HealthStatus::Degraded) => HealthStatus::Degraded,
            _ => HealthStatus::Healthy,
        }
    }
}

/// Health check result for a component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component name
    pub component: String,
    /// Health status
    pub status: HealthStatus,
    /// Optional error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Timestamp of the check
    pub timestamp: i64,
    /// Optional additional details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ComponentHealth {
    /// Creates a healthy component health check
    pub fn healthy(component: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            status: HealthStatus::Healthy,
            error: None,
            timestamp: Self::now(),
            details: None,
        }
    }

    /// Creates a healthy component health check with details
    pub fn healthy_with_details(component: impl Into<String>, details: serde_json::Value) -> Self {
        Self {
            component: component.into(),
            status: HealthStatus::Healthy,
            error: None,
            timestamp: Self::now(),
            details: Some(details),
        }
    }

    /// Creates a degraded component health check
    pub fn degraded(component: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            status: HealthStatus::Degraded,
            error: Some(error.into()),
            timestamp: Self::now(),
            details: None,
        }
    }

    /// Creates a degraded component health check with details
    pub fn degraded_with_details(
        component: impl Into<String>,
        error: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            component: component.into(),
            status: HealthStatus::Degraded,
            error: Some(error.into()),
            timestamp: Self::now(),
            details: Some(details),
        }
    }

    /// Creates an unhealthy component health check
    pub fn unhealthy(component: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            status: HealthStatus::Unhealthy,
            error: Some(error.into()),
            timestamp: Self::now(),
            details: None,
        }
    }

    /// Creates an unhealthy component health check with details
    pub fn unhealthy_with_details(
        component: impl Into<String>,
        error: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            component: component.into(),
            status: HealthStatus::Unhealthy,
            error: Some(error.into()),
            timestamp: Self::now(),
            details: Some(details),
        }
    }

    fn now() -> i64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }
}

/// Overall health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Overall health status
    pub status: HealthStatus,
    /// Timestamp of the check
    pub timestamp: i64,
    /// Server version
    pub version: String,
}

impl IntoResponse for HealthResponse {
    fn into_response(self) -> Response {
        let status = self.status.status_code();
        (status, Json(self)).into_response()
    }
}

/// Readiness check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessResponse {
    /// Readiness status
    pub ready: bool,
    /// Overall health status
    pub status: HealthStatus,
    /// Timestamp of the check
    pub timestamp: i64,
    /// Components that are not ready
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub not_ready: Vec<String>,
}

impl IntoResponse for ReadinessResponse {
    fn into_response(self) -> Response {
        let status = if self.ready {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        };
        (status, Json(self)).into_response()
    }
}

/// Detailed component health response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealthResponse {
    /// Overall health status
    pub status: HealthStatus,
    /// Timestamp of the check
    pub timestamp: i64,
    /// Individual component health checks
    pub components: HashMap<String, ComponentHealth>,
}

impl IntoResponse for ComponentHealthResponse {
    fn into_response(self) -> Response {
        let status = self.status.status_code();
        (status, Json(self)).into_response()
    }
}

/// Main health checker coordinator
#[derive(Clone)]
pub struct HealthChecker {
    config: Arc<ServerConfig>,
    state: Arc<AppState>,
}

impl HealthChecker {
    /// Creates a new health checker
    pub fn new(config: Arc<ServerConfig>, state: Arc<AppState>) -> Self {
        Self { config, state }
    }

    /// Performs a liveness check - returns healthy if server is running
    pub fn liveness(&self) -> HealthResponse {
        HealthResponse {
            status: HealthStatus::Healthy,
            timestamp: Self::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Performs a readiness check - returns ready if server can accept requests
    pub async fn readiness(&self) -> ReadinessResponse {
        let components = self.check_all_components().await;
        let mut overall = HealthStatus::Healthy;
        let mut not_ready = Vec::new();

        for (name, health) in &components {
            overall = overall.combine(health.status);
            if health.status == HealthStatus::Unhealthy {
                not_ready.push(name.clone());
            }
        }

        ReadinessResponse {
            ready: overall != HealthStatus::Unhealthy,
            status: overall,
            timestamp: Self::now(),
            not_ready,
        }
    }

    /// Performs detailed component health checks
    pub async fn components(&self) -> ComponentHealthResponse {
        let components = self.check_all_components().await;
        let mut overall = HealthStatus::Healthy;

        for health in components.values() {
            overall = overall.combine(health.status);
        }

        ComponentHealthResponse {
            status: overall,
            timestamp: Self::now(),
            components,
        }
    }

    /// Checks all components and returns their health status
    async fn check_all_components(&self) -> HashMap<String, ComponentHealth> {
        let mut components = HashMap::new();

        // Check workspace directory
        components.insert(
            "workspace".to_string(),
            self.check_workspace_directory().await,
        );

        // Check cache status
        components.insert("cache".to_string(), self.check_cache_status().await);

        // Check LibreOffice availability (if recalc enabled)
        #[cfg(feature = "recalc")]
        if self.config.recalc_enabled {
            components.insert(
                "libreoffice".to_string(),
                self.check_libreoffice_availability().await,
            );
        }

        // Check fork registry (if recalc enabled)
        #[cfg(feature = "recalc")]
        if self.config.recalc_enabled {
            components.insert(
                "fork_registry".to_string(),
                self.check_fork_registry().await,
            );
        }

        // Check workbook index
        components.insert(
            "workbook_index".to_string(),
            self.check_workbook_index().await,
        );

        components
    }

    /// Checks workspace directory accessibility
    async fn check_workspace_directory(&self) -> ComponentHealth {
        let workspace = &self.config.workspace_root;

        // Check if directory exists
        if !workspace.exists() {
            return ComponentHealth::unhealthy(
                "workspace",
                format!(
                    "workspace directory does not exist: {}",
                    workspace.display()
                ),
            );
        }

        // Check if it's a directory
        if !workspace.is_dir() {
            return ComponentHealth::unhealthy(
                "workspace",
                format!("workspace path is not a directory: {}", workspace.display()),
            );
        }

        // Check if directory is readable
        match fs::read_dir(workspace) {
            Ok(_) => {
                let details = serde_json::json!({
                    "path": workspace.display().to_string(),
                    "readable": true,
                });
                ComponentHealth::healthy_with_details("workspace", details)
            }
            Err(e) => ComponentHealth::unhealthy(
                "workspace",
                format!(
                    "workspace directory is not readable: {} ({})",
                    workspace.display(),
                    e
                ),
            ),
        }
    }

    /// Checks cache status
    async fn check_cache_status(&self) -> ComponentHealth {
        let stats = self.state.cache_stats();
        let capacity_usage = stats.size as f64 / stats.capacity as f64;

        let details = serde_json::json!({
            "size": stats.size,
            "capacity": stats.capacity,
            "capacity_usage_pct": (capacity_usage * 100.0).round(),
            "operations": stats.operations,
            "hits": stats.hits,
            "misses": stats.misses,
            "hit_rate_pct": (stats.hit_rate() * 100.0).round(),
        });

        // Warn if cache is nearly full
        if capacity_usage >= 0.95 {
            ComponentHealth::degraded_with_details(
                "cache",
                format!(
                    "cache is {}% full ({}/{})",
                    (capacity_usage * 100.0).round(),
                    stats.size,
                    stats.capacity
                ),
                details,
            )
        } else {
            ComponentHealth::healthy_with_details("cache", details)
        }
    }

    /// Checks LibreOffice availability
    #[cfg(feature = "recalc")]
    async fn check_libreoffice_availability(&self) -> ComponentHealth {
        use tokio::process::Command;

        // Try to spawn soffice with --version
        match Command::new("soffice").arg("--version").output().await {
            Ok(output) => {
                if output.status.success() {
                    let version = String::from_utf8_lossy(&output.stdout);
                    let version_str = version.trim().to_string();
                    let details = serde_json::json!({
                        "available": true,
                        "version": version_str,
                    });
                    ComponentHealth::healthy_with_details("libreoffice", details)
                } else {
                    ComponentHealth::unhealthy(
                        "libreoffice",
                        "soffice command failed (non-zero exit code)",
                    )
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    ComponentHealth::unhealthy("libreoffice", "soffice command not found in PATH")
                } else {
                    ComponentHealth::unhealthy(
                        "libreoffice",
                        format!("failed to execute soffice: {}", e),
                    )
                }
            }
        }
    }

    /// Checks fork registry status
    #[cfg(feature = "recalc")]
    async fn check_fork_registry(&self) -> ComponentHealth {
        match self.state.fork_registry() {
            Some(registry) => {
                let fork_count = registry.list_forks().len();
                let details = serde_json::json!({
                    "active_forks": fork_count,
                    "available": true,
                });
                ComponentHealth::healthy_with_details("fork_registry", details)
            }
            None => ComponentHealth::unhealthy(
                "fork_registry",
                "fork registry not initialized (recalc enabled but initialization failed)",
            ),
        }
    }

    /// Checks workbook index status
    async fn check_workbook_index(&self) -> ComponentHealth {
        use crate::tools::filters::WorkbookFilter;

        // Try to list workbooks to verify index is working
        match self.state.list_workbooks(WorkbookFilter::default()) {
            Ok(response) => {
                let details = serde_json::json!({
                    "workbook_count": response.workbooks.len(),
                    "available": true,
                });
                ComponentHealth::healthy_with_details("workbook_index", details)
            }
            Err(e) => ComponentHealth::unhealthy(
                "workbook_index",
                format!("failed to list workbooks: {}", e),
            ),
        }
    }

    fn now() -> i64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }
}

/// Axum handler for liveness endpoint
pub async fn liveness_handler(State(checker): State<Arc<HealthChecker>>) -> impl IntoResponse {
    checker.liveness()
}

/// Axum handler for readiness endpoint
pub async fn readiness_handler(State(checker): State<Arc<HealthChecker>>) -> impl IntoResponse {
    checker.readiness().await
}

/// Axum handler for components endpoint
pub async fn components_handler(State(checker): State<Arc<HealthChecker>>) -> impl IntoResponse {
    checker.components().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_status_combine() {
        assert_eq!(
            HealthStatus::Healthy.combine(HealthStatus::Healthy),
            HealthStatus::Healthy
        );
        assert_eq!(
            HealthStatus::Healthy.combine(HealthStatus::Degraded),
            HealthStatus::Degraded
        );
        assert_eq!(
            HealthStatus::Healthy.combine(HealthStatus::Unhealthy),
            HealthStatus::Unhealthy
        );
        assert_eq!(
            HealthStatus::Degraded.combine(HealthStatus::Degraded),
            HealthStatus::Degraded
        );
        assert_eq!(
            HealthStatus::Degraded.combine(HealthStatus::Unhealthy),
            HealthStatus::Unhealthy
        );
        assert_eq!(
            HealthStatus::Unhealthy.combine(HealthStatus::Unhealthy),
            HealthStatus::Unhealthy
        );
    }

    #[test]
    fn component_health_constructors() {
        let healthy = ComponentHealth::healthy("test");
        assert_eq!(healthy.status, HealthStatus::Healthy);
        assert_eq!(healthy.component, "test");
        assert!(healthy.error.is_none());

        let degraded = ComponentHealth::degraded("test", "warning message");
        assert_eq!(degraded.status, HealthStatus::Degraded);
        assert_eq!(degraded.error, Some("warning message".to_string()));

        let unhealthy = ComponentHealth::unhealthy("test", "error message");
        assert_eq!(unhealthy.status, HealthStatus::Unhealthy);
        assert_eq!(unhealthy.error, Some("error message".to_string()));
    }

    #[test]
    fn health_status_codes() {
        assert_eq!(HealthStatus::Healthy.status_code(), StatusCode::OK);
        assert_eq!(HealthStatus::Degraded.status_code(), StatusCode::OK);
        assert_eq!(
            HealthStatus::Unhealthy.status_code(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }
}
