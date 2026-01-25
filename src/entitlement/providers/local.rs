//! Local file provider - reads license from .ggen_license file

use crate::entitlement::{Capability, EntitlementProvider, License, UsageUnit};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};

/// Provider that reads license from local file
#[derive(Debug)]
pub struct LocalFileProvider {
    license_path: PathBuf,
    capabilities: HashSet<Capability>,
}

impl LocalFileProvider {
    pub fn new(path: &str) -> Result<Self> {
        let license_path = PathBuf::from(path);
        let capabilities = Self::load_license(&license_path)?;

        tracing::info!(
            path = %license_path.display(),
            capabilities = ?capabilities,
            "Local license loaded"
        );

        Ok(Self {
            license_path,
            capabilities,
        })
    }

    fn load_license(path: &Path) -> Result<HashSet<Capability>> {
        let content = fs::read_to_string(path).context("Failed to read license file")?;

        let license: License =
            serde_json::from_str(&content).context("Failed to parse license JSON")?;

        // FUTURE: Verify signature (requires crypto library integration)
        // See: https://docs.rs/ring/latest/ring/ for signature verification
        // For now, just log a warning
        tracing::warn!("License signature verification not implemented");

        // Check expiration
        if license.expires_at < chrono::Utc::now() {
            anyhow::bail!("License expired on {}", license.expires_at);
        }

        Ok(license.capabilities.into_iter().collect())
    }
}

#[async_trait]
impl EntitlementProvider for LocalFileProvider {
    async fn check_capability(&self, cap: Capability) -> Result<bool> {
        Ok(self.capabilities.contains(&cap))
    }

    async fn report_usage(&self, usage: UsageUnit) -> Result<()> {
        // Write to .usage.jsonl log file next to license
        let usage_path = self.license_path.with_extension("usage.jsonl");

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&usage_path)
            .context("Failed to open usage log file")?;

        let json = serde_json::to_string(&usage)?;
        writeln!(file, "{}", json)?;

        Ok(())
    }

    fn name(&self) -> &str {
        "local_file"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use std::collections::HashMap;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_valid_license() {
        let license = License {
            version: "1.0".to_string(),
            capabilities: vec![Capability::ApplyMode, Capability::JiraCreate],
            expires_at: Utc::now() + Duration::days(30),
            signature: "test_signature".to_string(),
            holder: Some("Test User".to_string()),
            license_id: Some("test-123".to_string()),
        };

        let temp_file = NamedTempFile::new().unwrap();
        let json = serde_json::to_string(&license).unwrap();
        std::fs::write(temp_file.path(), json).unwrap();

        let provider = LocalFileProvider::new(temp_file.path().to_str().unwrap()).unwrap();

        assert_eq!(provider.capabilities.len(), 2);
        assert!(provider.capabilities.contains(&Capability::ApplyMode));
        assert!(provider.capabilities.contains(&Capability::JiraCreate));
    }

    #[test]
    fn test_expired_license() {
        let license = License {
            version: "1.0".to_string(),
            capabilities: vec![Capability::ApplyMode],
            expires_at: Utc::now() - Duration::days(1), // Expired
            signature: "test_signature".to_string(),
            holder: None,
            license_id: None,
        };

        let temp_file = NamedTempFile::new().unwrap();
        let json = serde_json::to_string(&license).unwrap();
        std::fs::write(temp_file.path(), json).unwrap();

        let result = LocalFileProvider::new(temp_file.path().to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expired"));
    }

    #[tokio::test]
    async fn test_check_capability() {
        let license = License {
            version: "1.0".to_string(),
            capabilities: vec![Capability::ApplyMode],
            expires_at: Utc::now() + Duration::days(30),
            signature: "test_signature".to_string(),
            holder: None,
            license_id: None,
        };

        let temp_file = NamedTempFile::new().unwrap();
        let json = serde_json::to_string(&license).unwrap();
        std::fs::write(temp_file.path(), json).unwrap();

        let provider = LocalFileProvider::new(temp_file.path().to_str().unwrap()).unwrap();

        assert!(
            provider
                .check_capability(Capability::ApplyMode)
                .await
                .unwrap()
        );
        assert!(
            !provider
                .check_capability(Capability::JiraCreate)
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_usage_reporting() {
        let license = License {
            version: "1.0".to_string(),
            capabilities: vec![Capability::ApplyMode],
            expires_at: Utc::now() + Duration::days(30),
            signature: "test_signature".to_string(),
            holder: None,
            license_id: None,
        };

        let temp_file = NamedTempFile::new().unwrap();
        let json = serde_json::to_string(&license).unwrap();
        std::fs::write(temp_file.path(), json).unwrap();

        let provider = LocalFileProvider::new(temp_file.path().to_str().unwrap()).unwrap();

        let usage = UsageUnit {
            operation: "test_op".to_string(),
            timestamp: Utc::now(),
            workspace_hash: "test_hash".to_string(),
            user_id: Some("user-123".to_string()),
            metadata: HashMap::new(),
        };

        let result = provider.report_usage(usage).await;
        assert!(result.is_ok());

        // Verify usage log file was created
        let usage_path = temp_file.path().with_extension("usage.jsonl");
        assert!(usage_path.exists());

        let contents = std::fs::read_to_string(usage_path).unwrap();
        assert!(contents.contains("test_op"));
    }
}
