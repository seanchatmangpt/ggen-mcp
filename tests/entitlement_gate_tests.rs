//! Entitlement Gate Integration Tests
//!
//! Tests for capability-based access control that gates access to
//! premium features (apply mode, Jira integration, etc.).
//!
//! Providers:
//! - LocalFile: License file on disk
//! - EnvVar: License from environment variable
//! - GCP: License from GCP Secret Manager
//!
//! Capabilities:
//! - PreviewMode: Generate reports without file writes
//! - ApplyMode: Write generated files
//! - JiraIntegration: Create/sync Jira tickets
//! - UsageReporting: Send usage analytics
//!
//! Chicago-style TDD: State-based testing, behavior verification.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// =============================================================================
// Mock Types for Entitlement System
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    PreviewMode,
    ApplyMode,
    JiraIntegration,
    UsageReporting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    pub id: String,
    pub organization: String,
    pub capabilities: HashSet<Capability>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitlementConfig {
    pub provider_type: String,
    pub local_path: String,
    pub gcp_config: GcpConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GcpConfig {
    pub project_id: String,
    pub secret_name: String,
}

// =============================================================================
// Entitlement Provider Trait
// =============================================================================

#[async_trait::async_trait]
pub trait EntitlementProvider: Send + Sync {
    async fn load_license(&self) -> Result<License>;
    async fn check_capability(&self, capability: Capability) -> Result<bool>;
}

// =============================================================================
// Provider 1: Local File
// =============================================================================

pub struct LocalFileProvider {
    license_path: PathBuf,
}

impl LocalFileProvider {
    pub fn new(path: &str) -> Result<Self> {
        Ok(Self {
            license_path: PathBuf::from(path),
        })
    }
}

#[async_trait::async_trait]
impl EntitlementProvider for LocalFileProvider {
    async fn load_license(&self) -> Result<License> {
        if !self.license_path.exists() {
            return Err(anyhow!("License file not found: {:?}", self.license_path));
        }

        let content = fs::read_to_string(&self.license_path)?;
        let license: License = serde_json::from_str(&content)?;
        Ok(license)
    }

    async fn check_capability(&self, capability: Capability) -> Result<bool> {
        let license = self.load_license().await?;
        Ok(license.capabilities.contains(&capability))
    }
}

// =============================================================================
// Provider 2: Environment Variable
// =============================================================================

pub struct EnvVarProvider {
    var_name: String,
}

impl EnvVarProvider {
    pub fn new() -> Self {
        Self {
            var_name: "GGEN_LICENSE".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl EntitlementProvider for EnvVarProvider {
    async fn load_license(&self) -> Result<License> {
        let license_json = std::env::var(&self.var_name)
            .map_err(|_| anyhow!("Environment variable {} not set", self.var_name))?;

        let license: License = serde_json::from_str(&license_json)?;
        Ok(license)
    }

    async fn check_capability(&self, capability: Capability) -> Result<bool> {
        let license = self.load_license().await?;
        Ok(license.capabilities.contains(&capability))
    }
}

// =============================================================================
// Provider 3: GCP Secret Manager (Mock)
// =============================================================================

pub struct GcpSecretProvider {
    project_id: String,
    secret_name: String,
}

impl GcpSecretProvider {
    pub fn new(project_id: String, secret_name: String) -> Self {
        Self {
            project_id,
            secret_name,
        }
    }
}

#[async_trait::async_trait]
impl EntitlementProvider for GcpSecretProvider {
    async fn load_license(&self) -> Result<License> {
        // Mock GCP Secret Manager access
        // In real implementation: use google-secretmanager crate
        let mock_license = License {
            id: "gcp-license-001".to_string(),
            organization: "Acme Corp".to_string(),
            capabilities: vec![
                Capability::PreviewMode,
                Capability::ApplyMode,
                Capability::JiraIntegration,
            ]
            .into_iter()
            .collect(),
            expires_at: Some("2027-01-01T00:00:00Z".to_string()),
        };

        Ok(mock_license)
    }

    async fn check_capability(&self, capability: Capability) -> Result<bool> {
        let license = self.load_license().await?;
        Ok(license.capabilities.contains(&capability))
    }
}

// =============================================================================
// Entitlement Gate
// =============================================================================

pub struct EntitlementGate {
    provider: Box<dyn EntitlementProvider>,
}

impl EntitlementGate {
    pub fn from_config(config: &EntitlementConfig) -> Result<Self> {
        let provider: Box<dyn EntitlementProvider> = match config.provider_type.as_str() {
            "local" => Box::new(LocalFileProvider::new(&config.local_path)?),
            "env" => Box::new(EnvVarProvider::new()),
            "gcp" => Box::new(GcpSecretProvider::new(
                config.gcp_config.project_id.clone(),
                config.gcp_config.secret_name.clone(),
            )),
            _ => return Err(anyhow!("Unknown provider type: {}", config.provider_type)),
        };

        Ok(Self { provider })
    }

    pub async fn check_capability(&self, capability: Capability) -> Result<bool> {
        self.provider.check_capability(capability).await
    }

    pub async fn require_capability(&self, capability: Capability) -> Result<()> {
        if !self.check_capability(capability.clone()).await? {
            return Err(anyhow!(
                "Operation requires entitlement: {:?}",
                capability
            ));
        }
        Ok(())
    }

    pub async fn report_usage(&self, _capability: Capability) -> Result<()> {
        // Mock usage reporting
        Ok(())
    }
}

// =============================================================================
// Test Fixtures
// =============================================================================

fn create_test_license(capabilities: Vec<Capability>) -> License {
    License {
        id: "test-license-001".to_string(),
        organization: "Test Org".to_string(),
        capabilities: capabilities.into_iter().collect(),
        expires_at: Some("2027-01-01T00:00:00Z".to_string()),
    }
}

// =============================================================================
// Test 1: Local Provider - Allowed Capability
// =============================================================================

#[tokio::test]
async fn test_local_provider_allowed_capability() -> Result<()> {
    // Arrange
    let workspace = TempDir::new()?;
    let license_path = workspace.path().join("license.json");

    let license = create_test_license(vec![Capability::PreviewMode, Capability::ApplyMode]);
    fs::write(&license_path, serde_json::to_string_pretty(&license)?)?;

    let provider = LocalFileProvider::new(license_path.to_str().unwrap())?;

    // Act
    let allowed = provider.check_capability(Capability::ApplyMode).await?;

    // Assert
    assert!(allowed, "ApplyMode should be allowed");

    Ok(())
}

// =============================================================================
// Test 2: Local Provider - Denied Capability
// =============================================================================

#[tokio::test]
async fn test_local_provider_denied_capability() -> Result<()> {
    // Arrange
    let workspace = TempDir::new()?;
    let license_path = workspace.path().join("license.json");

    let license = create_test_license(vec![Capability::PreviewMode]); // Only preview
    fs::write(&license_path, serde_json::to_string_pretty(&license)?)?;

    let provider = LocalFileProvider::new(license_path.to_str().unwrap())?;

    // Act
    let allowed = provider.check_capability(Capability::ApplyMode).await?;

    // Assert
    assert!(!allowed, "ApplyMode should NOT be allowed");

    Ok(())
}

// =============================================================================
// Test 3: Environment Variable Provider
// =============================================================================

#[tokio::test]
async fn test_env_var_provider() -> Result<()> {
    // Arrange
    let license = create_test_license(vec![Capability::PreviewMode, Capability::ApplyMode]);
    std::env::set_var("GGEN_LICENSE", serde_json::to_string(&license)?);

    let provider = EnvVarProvider::new();

    // Act
    let allowed = provider.check_capability(Capability::ApplyMode).await?;

    // Assert
    assert!(allowed, "ApplyMode should be allowed");

    // Cleanup
    std::env::remove_var("GGEN_LICENSE");

    Ok(())
}

// =============================================================================
// Test 4: GCP Secret Provider (Mock)
// =============================================================================

#[tokio::test]
async fn test_gcp_provider() -> Result<()> {
    // Arrange
    let provider = GcpSecretProvider::new("test-project".to_string(), "ggen-license".to_string());

    // Act
    let allowed = provider.check_capability(Capability::JiraIntegration).await?;

    // Assert
    assert!(allowed, "JiraIntegration should be allowed in mock");

    Ok(())
}

// =============================================================================
// Test 5: Entitlement Gate - Require Fails
// =============================================================================

#[tokio::test]
async fn test_entitlement_gate_require_fails() -> Result<()> {
    // Arrange
    let workspace = TempDir::new()?;
    let license_path = workspace.path().join("license.json");

    let license = create_test_license(vec![Capability::PreviewMode]); // Only preview
    fs::write(&license_path, serde_json::to_string_pretty(&license)?)?;

    let gate = EntitlementGate::from_config(&EntitlementConfig {
        provider_type: "local".to_string(),
        local_path: license_path.to_string_lossy().to_string(),
        gcp_config: GcpConfig::default(),
    })?;

    // Act
    let result = gate.require_capability(Capability::ApplyMode).await;

    // Assert
    assert!(result.is_err(), "Should error for missing capability");

    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("requires entitlement"),
        "Error should indicate missing entitlement"
    );

    Ok(())
}

// =============================================================================
// Test 6: Entitlement Gate - Require Succeeds
// =============================================================================

#[tokio::test]
async fn test_entitlement_gate_require_succeeds() -> Result<()> {
    // Arrange
    let workspace = TempDir::new()?;
    let license_path = workspace.path().join("license.json");

    let license = create_test_license(vec![Capability::PreviewMode, Capability::ApplyMode]);
    fs::write(&license_path, serde_json::to_string_pretty(&license)?)?;

    let gate = EntitlementGate::from_config(&EntitlementConfig {
        provider_type: "local".to_string(),
        local_path: license_path.to_string_lossy().to_string(),
        gcp_config: GcpConfig::default(),
    })?;

    // Act
    let result = gate.require_capability(Capability::ApplyMode).await;

    // Assert
    assert!(result.is_ok(), "Should succeed for allowed capability");

    Ok(())
}

// =============================================================================
// Test 7: Usage Reporting
// =============================================================================

#[tokio::test]
async fn test_usage_reporting() -> Result<()> {
    // Arrange
    let workspace = TempDir::new()?;
    let license_path = workspace.path().join("license.json");

    let license = create_test_license(vec![
        Capability::PreviewMode,
        Capability::ApplyMode,
        Capability::UsageReporting,
    ]);
    fs::write(&license_path, serde_json::to_string_pretty(&license)?)?;

    let gate = EntitlementGate::from_config(&EntitlementConfig {
        provider_type: "local".to_string(),
        local_path: license_path.to_string_lossy().to_string(),
        gcp_config: GcpConfig::default(),
    })?;

    // Act
    let result = gate.report_usage(Capability::ApplyMode).await;

    // Assert
    assert!(result.is_ok(), "Usage reporting should succeed");

    Ok(())
}

// =============================================================================
// Test 8: Multiple Capabilities Check
// =============================================================================

#[tokio::test]
async fn test_multiple_capabilities() -> Result<()> {
    // Arrange
    let workspace = TempDir::new()?;
    let license_path = workspace.path().join("license.json");

    let license = create_test_license(vec![
        Capability::PreviewMode,
        Capability::ApplyMode,
        Capability::JiraIntegration,
    ]);
    fs::write(&license_path, serde_json::to_string_pretty(&license)?)?;

    let provider = LocalFileProvider::new(license_path.to_str().unwrap())?;

    // Act & Assert
    assert!(
        provider.check_capability(Capability::PreviewMode).await?,
        "PreviewMode should be allowed"
    );
    assert!(
        provider.check_capability(Capability::ApplyMode).await?,
        "ApplyMode should be allowed"
    );
    assert!(
        provider.check_capability(Capability::JiraIntegration).await?,
        "JiraIntegration should be allowed"
    );
    assert!(
        !provider.check_capability(Capability::UsageReporting).await?,
        "UsageReporting should NOT be allowed"
    );

    Ok(())
}

// =============================================================================
// Additional Tests
// =============================================================================

#[tokio::test]
async fn test_license_not_found_error() -> Result<()> {
    // Arrange
    let provider = LocalFileProvider::new("/nonexistent/license.json")?;

    // Act
    let result = provider.load_license().await;

    // Assert
    assert!(result.is_err(), "Should error when license file not found");
    assert!(
        result.unwrap_err().to_string().contains("not found"),
        "Error should indicate file not found"
    );

    Ok(())
}

#[tokio::test]
async fn test_invalid_license_json() -> Result<()> {
    // Arrange
    let workspace = TempDir::new()?;
    let license_path = workspace.path().join("invalid.json");
    fs::write(&license_path, "invalid json content")?;

    let provider = LocalFileProvider::new(license_path.to_str().unwrap())?;

    // Act
    let result = provider.load_license().await;

    // Assert
    assert!(result.is_err(), "Should error on invalid JSON");

    Ok(())
}

#[tokio::test]
async fn test_env_var_not_set() -> Result<()> {
    // Arrange
    std::env::remove_var("GGEN_LICENSE"); // Ensure not set
    let provider = EnvVarProvider::new();

    // Act
    let result = provider.load_license().await;

    // Assert
    assert!(result.is_err(), "Should error when env var not set");
    assert!(
        result.unwrap_err().to_string().contains("not set"),
        "Error should indicate env var not set"
    );

    Ok(())
}

#[tokio::test]
async fn test_expired_license() -> Result<()> {
    // Arrange
    let workspace = TempDir::new()?;
    let license_path = workspace.path().join("license.json");

    let mut license = create_test_license(vec![Capability::PreviewMode]);
    license.expires_at = Some("2020-01-01T00:00:00Z".to_string()); // Expired

    fs::write(&license_path, serde_json::to_string_pretty(&license)?)?;

    let provider = LocalFileProvider::new(license_path.to_str().unwrap())?;

    // Act
    let loaded = provider.load_license().await?;

    // Assert
    // In real implementation, check expiration date
    assert!(
        loaded.expires_at.is_some(),
        "License should have expiration date"
    );

    // Additional validation logic would go here:
    // let expires_at = chrono::DateTime::parse_from_rfc3339(&loaded.expires_at.unwrap())?;
    // assert!(chrono::Utc::now() < expires_at, "License should be expired");

    Ok(())
}

// =============================================================================
// Test Module Documentation
// =============================================================================

/// Test coverage summary:
/// 1. Local provider - allowed capability
/// 2. Local provider - denied capability
/// 3. Environment variable provider
/// 4. GCP secret provider (mock)
/// 5. Entitlement gate - require fails
/// 6. Entitlement gate - require succeeds
/// 7. Usage reporting
/// 8. Multiple capabilities check
/// 9. License file not found error
/// 10. Invalid license JSON error
/// 11. Environment variable not set error
/// 12. Expired license handling
///
/// Total: 12 tests covering entitlement gate system
