//! Entitlement gate for capability-based monetization
//!
//! Provides pluggable entitlement checking for MCP tools:
//! - Local file provider (license file)
//! - Environment variable provider (license JSON)
//! - GCP Marketplace provider (stub for future)
//!
//! Usage:
//! ```rust,ignore
//! // Check entitlement before protected operation
//! state.entitlement_gate
//!     .require_capability(Capability::ApplyMode)
//!     .await?;
//!
//! // Report usage for metering
//! state.entitlement_gate.report_usage(usage).await;
//! ```

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

pub mod providers;

use providers::{DisabledProvider, EnvVarProvider, GcpMarketplaceProvider, LocalFileProvider};

// =============================================================================
// CAPABILITY DEFINITIONS
// =============================================================================

/// Capabilities that can be gated by entitlement
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    // Free tier capabilities
    /// Preview mode (read-only, dry-run operations)
    PreviewMode,
    /// Read-only tools access
    ReadOnlyTools,

    // Paid tier capabilities
    /// Apply mode (write operations)
    ApplyMode,
    /// Create Jira tickets
    JiraCreate,
    /// Sync with Jira bidirectionally
    JiraSync,
    /// Full guard suite (validation, poka-yoke)
    FullGuardSuite,
    /// Receipt verification for transactions
    ReceiptVerification,

    // Enterprise tier capabilities
    /// Multi-workspace support
    MultiWorkspace,
    /// Team collaboration features
    TeamCollaboration,
    /// Audit reporting and compliance
    AuditReporting,
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Capability::PreviewMode => write!(f, "preview_mode"),
            Capability::ReadOnlyTools => write!(f, "read_only_tools"),
            Capability::ApplyMode => write!(f, "apply_mode"),
            Capability::JiraCreate => write!(f, "jira_create"),
            Capability::JiraSync => write!(f, "jira_sync"),
            Capability::FullGuardSuite => write!(f, "full_guard_suite"),
            Capability::ReceiptVerification => write!(f, "receipt_verification"),
            Capability::MultiWorkspace => write!(f, "multi_workspace"),
            Capability::TeamCollaboration => write!(f, "team_collaboration"),
            Capability::AuditReporting => write!(f, "audit_reporting"),
        }
    }
}

// =============================================================================
// USAGE TRACKING
// =============================================================================

/// Unit of usage for metering and reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageUnit {
    /// Operation identifier (e.g., "sync_ggen_apply", "jira_create_tickets")
    pub operation: String,
    /// Timestamp when operation occurred
    pub timestamp: DateTime<Utc>,
    /// Workspace fingerprint (hashed)
    pub workspace_hash: String,
    /// User identifier (optional)
    pub user_id: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

// =============================================================================
// ENTITLEMENT PROVIDER TRAIT
// =============================================================================

/// Trait for entitlement providers
#[async_trait]
pub trait EntitlementProvider: Send + Sync {
    /// Check if capability is entitled
    async fn check_capability(&self, cap: Capability) -> Result<bool>;

    /// Report usage for metering
    async fn report_usage(&self, usage: UsageUnit) -> Result<()>;

    /// Get provider name for logging
    fn name(&self) -> &str;
}

// =============================================================================
// ENTITLEMENT GATE
// =============================================================================

/// Main entitlement gate
pub struct EntitlementGate {
    provider: Box<dyn EntitlementProvider>,
}

impl EntitlementGate {
    /// Create from configuration
    pub fn from_config(config: &EntitlementConfig) -> Result<Self> {
        let provider: Box<dyn EntitlementProvider> = match config.provider_type.as_str() {
            "local" => Box::new(LocalFileProvider::new(&config.local_path)?),
            "env" => Box::new(EnvVarProvider::new()),
            "gcp" => Box::new(GcpMarketplaceProvider::new(&config.gcp_config)?),
            "disabled" => Box::new(DisabledProvider),
            _ => {
                return Err(anyhow!(
                    "Unknown entitlement provider: {}",
                    config.provider_type
                ));
            }
        };

        Ok(Self { provider })
    }

    /// Create disabled gate (all capabilities allowed)
    pub fn disabled() -> Self {
        Self {
            provider: Box::new(DisabledProvider),
        }
    }

    /// Check capability, return error if not entitled
    pub async fn require_capability(&self, cap: Capability) -> Result<()> {
        if !self.provider.check_capability(cap.clone()).await? {
            return Err(anyhow!(
                "Capability '{}' requires entitlement. Contact sales or upgrade plan.",
                cap
            ));
        }
        Ok(())
    }

    /// Check capability without requiring (returns bool)
    pub async fn has_capability(&self, cap: Capability) -> bool {
        self.provider.check_capability(cap).await.unwrap_or(false)
    }

    /// Report usage (fire-and-forget, errors logged only)
    pub async fn report_usage(&self, usage: UsageUnit) {
        if let Err(e) = self.provider.report_usage(usage).await {
            tracing::warn!(error = %e, "Failed to report usage");
        }
    }

    /// Get provider name
    pub fn provider_name(&self) -> &str {
        self.provider.name()
    }
}

// =============================================================================
// CONFIGURATION
// =============================================================================

/// Entitlement configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitlementConfig {
    /// Provider type: "local", "env", "gcp", "disabled"
    pub provider_type: String,
    /// Path to local license file (for "local" provider)
    #[serde(default = "default_local_path")]
    pub local_path: String,
    /// GCP configuration (for "gcp" provider)
    #[serde(default)]
    pub gcp_config: GcpConfig,
}

fn default_local_path() -> String {
    ".ggen_license".to_string()
}

impl Default for EntitlementConfig {
    fn default() -> Self {
        Self {
            provider_type: "disabled".to_string(),
            local_path: default_local_path(),
            gcp_config: GcpConfig::default(),
        }
    }
}

/// GCP Marketplace configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GcpConfig {
    /// GCP project ID
    #[serde(default)]
    pub project_id: String,
    /// Procurement API endpoint
    #[serde(default = "default_gcp_endpoint")]
    pub procurement_api_endpoint: String,
}

fn default_gcp_endpoint() -> String {
    "https://cloudcommerceprocurement.googleapis.com".to_string()
}

// =============================================================================
// LICENSE STRUCTURE (for local provider)
// =============================================================================

/// License file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    /// License version
    pub version: String,
    /// List of entitled capabilities
    pub capabilities: Vec<Capability>,
    /// License expiration timestamp
    pub expires_at: DateTime<Utc>,
    /// Signature (future: crypto verification)
    pub signature: String,
    /// License holder information (optional)
    #[serde(default)]
    pub holder: Option<String>,
    /// License ID
    #[serde(default)]
    pub license_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_display() {
        assert_eq!(Capability::ApplyMode.to_string(), "apply_mode");
        assert_eq!(Capability::JiraCreate.to_string(), "jira_create");
    }

    #[test]
    fn test_default_config() {
        let config = EntitlementConfig::default();
        assert_eq!(config.provider_type, "disabled");
        assert_eq!(config.local_path, ".ggen_license");
    }

    #[tokio::test]
    async fn test_disabled_gate() {
        let gate = EntitlementGate::disabled();
        assert!(gate.has_capability(Capability::ApplyMode).await);
        assert!(gate.has_capability(Capability::JiraCreate).await);
        assert_eq!(gate.provider_name(), "disabled");
    }

    #[tokio::test]
    async fn test_require_capability_disabled() {
        let gate = EntitlementGate::disabled();
        let result = gate.require_capability(Capability::ApplyMode).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_usage_reporting() {
        let gate = EntitlementGate::disabled();
        let usage = UsageUnit {
            operation: "test_op".to_string(),
            timestamp: Utc::now(),
            workspace_hash: "test_hash".to_string(),
            user_id: None,
            metadata: HashMap::new(),
        };

        // Should not panic or error (fire-and-forget)
        gate.report_usage(usage).await;
    }
}
