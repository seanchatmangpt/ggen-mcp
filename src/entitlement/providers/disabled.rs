//! Disabled provider - allows all capabilities

use crate::entitlement::{Capability, EntitlementProvider, UsageUnit};
use anyhow::Result;
use async_trait::async_trait;

/// Provider that allows all capabilities (no entitlement checks)
#[derive(Debug, Default)]
pub struct DisabledProvider;

#[async_trait]
impl EntitlementProvider for DisabledProvider {
    async fn check_capability(&self, _cap: Capability) -> Result<bool> {
        Ok(true)
    }

    async fn report_usage(&self, _usage: UsageUnit) -> Result<()> {
        // No-op: usage not tracked when disabled
        Ok(())
    }

    fn name(&self) -> &str {
        "disabled"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_disabled_allows_all() {
        let provider = DisabledProvider;

        assert!(provider
            .check_capability(Capability::ApplyMode)
            .await
            .unwrap());
        assert!(provider
            .check_capability(Capability::JiraCreate)
            .await
            .unwrap());
        assert_eq!(provider.name(), "disabled");
    }
}
