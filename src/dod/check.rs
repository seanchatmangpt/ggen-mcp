//! DoD Check trait and execution infrastructure

use crate::dod::types::*;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;

/// Context provided to checks during execution
#[derive(Debug, Clone)]
pub struct CheckContext {
    pub workspace_root: PathBuf,
    pub timeout_ms: u64,
    pub mode: ValidationMode,
    pub metadata: HashMap<String, String>,
}

impl CheckContext {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            timeout_ms: 120_000, // 2 minutes default
            mode: ValidationMode::Fast,
            metadata: HashMap::new(),
        }
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Core trait for DoD checks
#[async_trait]
pub trait DodCheck: Send + Sync {
    /// Unique check identifier (e.g., "BUILD_CARGO_CHECK")
    fn id(&self) -> &str;

    /// Category this check belongs to
    fn category(&self) -> CheckCategory;

    /// Severity level (Fatal, Warning, Info)
    fn severity(&self) -> CheckSeverity;

    /// Human-readable description
    fn description(&self) -> &str;

    /// Execute the check
    async fn execute(&self, context: &CheckContext) -> Result<DodCheckResult>;

    /// Optional: declare dependencies (check IDs that must run first)
    fn dependencies(&self) -> Vec<String> {
        vec![]
    }

    /// Optional: whether this check should be skipped in certain profiles
    fn skip_in_profile(&self, _profile: &str) -> bool {
        false
    }
}

/// Registry of all available checks
pub struct CheckRegistry {
    checks: Vec<Box<dyn DodCheck>>,
}

impl CheckRegistry {
    pub fn new() -> Self {
        Self { checks: vec![] }
    }

    pub fn register(&mut self, check: Box<dyn DodCheck>) {
        self.checks.push(check);
    }

    pub fn get_all(&self) -> &[Box<dyn DodCheck>] {
        &self.checks
    }

    pub fn get_by_category(&self, category: CheckCategory) -> Vec<&Box<dyn DodCheck>> {
        self.checks
            .iter()
            .filter(|c| c.category() == category)
            .collect()
    }

    pub fn get_by_id(&self, id: &str) -> Option<&Box<dyn DodCheck>> {
        self.checks.iter().find(|c| c.id() == id)
    }

    /// Create a registry with all available checks
    pub fn with_all_checks() -> Self {
        crate::dod::checks::create_registry()
    }
}

impl Default for CheckRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockCheck;

    #[async_trait]
    impl DodCheck for MockCheck {
        fn id(&self) -> &str {
            "MOCK_CHECK"
        }

        fn category(&self) -> CheckCategory {
            CheckCategory::BuildCorrectness
        }

        fn severity(&self) -> CheckSeverity {
            CheckSeverity::Fatal
        }

        fn description(&self) -> &str {
            "Mock check for testing"
        }

        async fn execute(&self, _context: &CheckContext) -> Result<DodCheckResult> {
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Pass,
                severity: self.severity(),
                message: "Mock check passed".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms: 0,
                check_hash: "mock".to_string(),
            })
        }
    }

    #[test]
    fn check_context_default() {
        let ctx = CheckContext::new(PathBuf::from("/test"));
        assert_eq!(ctx.timeout_ms, 120_000);
    }

    #[test]
    fn check_context_with_timeout() {
        let ctx = CheckContext::new(PathBuf::from("/test")).with_timeout(60_000);
        assert_eq!(ctx.timeout_ms, 60_000);
    }

    #[test]
    fn registry_add_and_get() {
        let mut registry = CheckRegistry::new();
        registry.register(Box::new(MockCheck));
        assert_eq!(registry.get_all().len(), 1);
        assert!(registry.get_by_id("MOCK_CHECK").is_some());
    }

    #[tokio::test]
    async fn mock_check_executes() {
        let check = MockCheck;
        let ctx = CheckContext::new(PathBuf::from("/test"));
        let result = check.execute(&ctx).await.unwrap();
        assert_eq!(result.status, CheckStatus::Pass);
    }
}
