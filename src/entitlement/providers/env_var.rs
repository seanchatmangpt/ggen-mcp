//! Environment variable provider - reads license from GGEN_LICENSE env var

use crate::entitlement::{Capability, EntitlementProvider, UsageUnit};
use anyhow::{Context, Result};
use async_trait::async_trait;

/// Provider that reads license from GGEN_LICENSE environment variable
#[derive(Debug, Default)]
pub struct EnvVarProvider;

impl EnvVarProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EntitlementProvider for EnvVarProvider {
    async fn check_capability(&self, cap: Capability) -> Result<bool> {
        // Read GGEN_LICENSE env var
        let license_json = std::env::var("GGEN_LICENSE").unwrap_or_else(|_| "{}".to_string());

        let license: serde_json::Value =
            serde_json::from_str(&license_json).context("Failed to parse GGEN_LICENSE JSON")?;

        // Extract capabilities array
        let capabilities = license["capabilities"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let cap_str = cap.to_string();
        Ok(capabilities.iter().any(|c| c == &cap_str))
    }

    async fn report_usage(&self, usage: UsageUnit) -> Result<()> {
        // Log usage to stderr
        let json = serde_json::to_string(&usage)?;
        eprintln!("[USAGE] {}", json);
        Ok(())
    }

    fn name(&self) -> &str {
        "env_var"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_env_var_no_license() {
        std::env::remove_var("GGEN_LICENSE");
        let provider = EnvVarProvider::new();

        let result = provider.check_capability(Capability::ApplyMode).await;
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Empty license = no capabilities
    }

    #[tokio::test]
    async fn test_env_var_with_license() {
        let license_json = r#"{
            "capabilities": ["apply_mode", "jira_create"]
        }"#;
        std::env::set_var("GGEN_LICENSE", license_json);

        let provider = EnvVarProvider::new();

        assert!(
            provider
                .check_capability(Capability::ApplyMode)
                .await
                .unwrap()
        );
        assert!(
            provider
                .check_capability(Capability::JiraCreate)
                .await
                .unwrap()
        );
        assert!(
            !provider
                .check_capability(Capability::JiraSync)
                .await
                .unwrap()
        );

        std::env::remove_var("GGEN_LICENSE");
    }

    #[tokio::test]
    async fn test_usage_reporting() {
        let provider = EnvVarProvider::new();
        let usage = UsageUnit {
            operation: "test_op".to_string(),
            timestamp: Utc::now(),
            workspace_hash: "test_hash".to_string(),
            user_id: None,
            metadata: HashMap::new(),
        };

        let result = provider.report_usage(usage).await;
        assert!(result.is_ok());
    }
}
