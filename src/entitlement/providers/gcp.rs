//! GCP Marketplace provider (stub for future implementation)

use crate::entitlement::{Capability, EntitlementProvider, GcpConfig, UsageUnit};
use anyhow::Result;
use async_trait::async_trait;

/// Provider for GCP Marketplace entitlements (stub)
#[derive(Debug, Clone)]
pub struct GcpMarketplaceProvider {
    config: GcpConfig,
}

impl GcpMarketplaceProvider {
    pub fn new(config: &GcpConfig) -> Result<Self> {
        tracing::info!(
            project_id = %config.project_id,
            "GCP Marketplace provider initialized (stub)"
        );

        Ok(Self {
            config: config.clone(),
        })
    }
}

#[async_trait]
impl EntitlementProvider for GcpMarketplaceProvider {
    async fn check_capability(&self, cap: Capability) -> Result<bool> {
        // TODO: Implement GCP Procurement API integration
        // For now: stub that allows all capabilities
        tracing::warn!(
            capability = %cap,
            project_id = %self.config.project_id,
            "GCP entitlement check not implemented (stub allows all)"
        );
        Ok(true)
    }

    async fn report_usage(&self, usage: UsageUnit) -> Result<()> {
        // TODO: Implement Pub/Sub publishing for usage metering
        tracing::info!(
            operation = %usage.operation,
            workspace_hash = %usage.workspace_hash,
            project_id = %self.config.project_id,
            "GCP usage reporting (stub, not published)"
        );
        Ok(())
    }

    fn name(&self) -> &str {
        "gcp_marketplace"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gcp_stub_allows_all() {
        let config = GcpConfig {
            project_id: "test-project".to_string(),
            procurement_api_endpoint: "https://test.googleapis.com".to_string(),
        };

        let provider = GcpMarketplaceProvider::new(&config).unwrap();

        // Stub should allow all capabilities
        assert!(provider
            .check_capability(Capability::ApplyMode)
            .await
            .unwrap());
        assert!(provider
            .check_capability(Capability::JiraCreate)
            .await
            .unwrap());
        assert_eq!(provider.name(), "gcp_marketplace");
    }
}
