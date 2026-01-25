//! DoD Test Harness Utilities
//!
//! Helper functions and utilities for DoD integration tests.
//! Provides setup/teardown, mock workspace creation, and assertion helpers.

use spreadsheet_mcp::dod::*;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Test harness for DoD validation
pub struct DodTestHarness {
    pub temp_dir: TempDir,
    pub workspace_root: PathBuf,
}

impl DodTestHarness {
    /// Create a new test harness with temporary workspace
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let workspace_root = temp_dir.path().to_path_buf();

        Self {
            temp_dir,
            workspace_root,
        }
    }

    /// Get workspace root path
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// Create check context for this workspace
    pub fn create_context(&self) -> CheckContext {
        CheckContext::new(self.workspace_root.clone()).with_timeout(60_000)
    }

    /// Create a minimal valid Cargo.toml
    pub fn create_cargo_toml(&self, name: &str) -> std::io::Result<()> {
        let content = format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = {{ version = "1.0", features = ["derive"] }}
"#,
            name
        );

        fs::create_dir_all(self.workspace_root.join("src"))?;
        fs::write(self.workspace_root.join("Cargo.toml"), content)?;
        Ok(())
    }

    /// Create a simple lib.rs
    pub fn create_lib_rs(&self, content: &str) -> std::io::Result<()> {
        fs::create_dir_all(self.workspace_root.join("src"))?;
        fs::write(self.workspace_root.join("src/lib.rs"), content)?;
        Ok(())
    }

    /// Create a file with content
    pub fn create_file(&self, path: &str, content: &str) -> std::io::Result<()> {
        let file_path = self.workspace_root.join(path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(file_path, content)?;
        Ok(())
    }

    /// Initialize git repository
    pub fn init_git(&self) -> std::io::Result<()> {
        std::process::Command::new("git")
            .arg("init")
            .current_dir(&self.workspace_root)
            .output()?;

        std::process::Command::new("git")
            .args(&["config", "user.name", "Test User"])
            .current_dir(&self.workspace_root)
            .output()?;

        std::process::Command::new("git")
            .args(&["config", "user.email", "test@example.com"])
            .current_dir(&self.workspace_root)
            .output()?;

        Ok(())
    }

    /// Commit all changes
    pub fn git_commit(&self, message: &str) -> std::io::Result<()> {
        std::process::Command::new("git")
            .args(&["add", "."])
            .current_dir(&self.workspace_root)
            .output()?;

        std::process::Command::new("git")
            .args(&["commit", "-m", message])
            .current_dir(&self.workspace_root)
            .output()?;

        Ok(())
    }

    /// Create a valid minimal workspace
    pub fn create_valid_workspace(&self) -> std::io::Result<()> {
        self.create_cargo_toml("test-workspace")?;

        let lib_rs = r#"
//! Test workspace

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }
}
"#;
        self.create_lib_rs(lib_rs)?;

        self.create_file("README.md", "# Test Workspace\n")?;

        self.init_git()?;
        self.git_commit("Initial commit")?;

        Ok(())
    }

    /// Create a workspace with failing tests
    pub fn create_workspace_with_failing_tests(&self) -> std::io::Result<()> {
        self.create_cargo_toml("failing-tests")?;

        let lib_rs = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_wrong() {
        assert_eq!(add(2, 2), 5); // Intentionally wrong!
    }
}
"#;
        self.create_lib_rs(lib_rs)?;
        Ok(())
    }

    /// Create a workspace with formatting issues
    pub fn create_workspace_with_fmt_issues(&self) -> std::io::Result<()> {
        self.create_cargo_toml("fmt-issues")?;

        // Intentionally poorly formatted
        let lib_rs = r#"pub fn add(a:i32,b:i32)->i32{a+b}"#;
        self.create_lib_rs(lib_rs)?;
        Ok(())
    }

    /// Create a workspace with security issues
    pub fn create_workspace_with_secrets(&self) -> std::io::Result<()> {
        self.create_cargo_toml("with-secrets")?;

        let lib_rs = r#"
// WARNING: Hardcoded secret
const API_KEY: &str = "sk-1234567890abcdef";

pub fn get_api_key() -> &'static str {
    API_KEY
}
"#;
        self.create_lib_rs(lib_rs)?;
        Ok(())
    }

    /// Create a workspace with TODOs
    pub fn create_workspace_with_todos(&self) -> std::io::Result<()> {
        self.create_cargo_toml("with-todos")?;

        let lib_rs = r#"
// TODO: Implement this properly
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
        self.create_lib_rs(lib_rs)?;
        Ok(())
    }

    /// Create ontology files for ggen checks
    pub fn create_ontology_files(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.workspace_root.join("ontology"))?;

        let ttl_content = r#"
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

<http://example.org/TestEntity> a rdfs:Class ;
    rdfs:label "Test Entity" .
"#;
        self.create_file("ontology/test.ttl", ttl_content)?;

        fs::create_dir_all(self.workspace_root.join("queries"))?;
        let sparql_content = r#"
SELECT ?s ?p ?o
WHERE {
    ?s ?p ?o .
}
LIMIT 10
"#;
        self.create_file("queries/test.rq", sparql_content)?;

        fs::create_dir_all(self.workspace_root.join("templates"))?;
        let template_content = r#"
// Generated code
pub fn test() {}
"#;
        self.create_file("templates/test.rs.tera", template_content)?;

        Ok(())
    }
}

/// Assertion helpers for DoD results
pub struct DodAssertions;

impl DodAssertions {
    /// Assert check passed
    pub fn assert_check_passed(results: &[DodCheckResult], check_id: &str) {
        let result = results
            .iter()
            .find(|r| r.id == check_id)
            .unwrap_or_else(|| panic!("Check {} not found in results", check_id));

        assert_eq!(
            result.status,
            CheckStatus::Pass,
            "Check {} should pass. Message: {}",
            check_id,
            result.message
        );
    }

    /// Assert check failed
    pub fn assert_check_failed(results: &[DodCheckResult], check_id: &str) {
        let result = results
            .iter()
            .find(|r| r.id == check_id)
            .unwrap_or_else(|| panic!("Check {} not found in results", check_id));

        assert_eq!(
            result.status,
            CheckStatus::Fail,
            "Check {} should fail",
            check_id
        );
    }

    /// Assert check warned
    pub fn assert_check_warned(results: &[DodCheckResult], check_id: &str) {
        let result = results
            .iter()
            .find(|r| r.id == check_id)
            .unwrap_or_else(|| panic!("Check {} not found in results", check_id));

        assert_eq!(
            result.status,
            CheckStatus::Warn,
            "Check {} should warn",
            check_id
        );
    }

    /// Assert verdict is ready
    pub fn assert_ready(results: &[DodCheckResult]) {
        let verdict = compute_verdict(results);
        assert_eq!(
            verdict,
            OverallVerdict::Ready,
            "Verdict should be Ready. Fatal failures: {:?}",
            get_fatal_failures(results)
        );
    }

    /// Assert verdict is not ready
    pub fn assert_not_ready(results: &[DodCheckResult]) {
        let verdict = compute_verdict(results);
        assert_eq!(
            verdict,
            OverallVerdict::NotReady,
            "Verdict should be NotReady"
        );
    }

    /// Assert minimum readiness score
    pub fn assert_min_score(results: &[DodCheckResult], min_score: f64) {
        let mut category_scores = std::collections::HashMap::new();
        for category in [
            CheckCategory::BuildCorrectness,
            CheckCategory::TestTruth,
            CheckCategory::GgenPipeline,
            CheckCategory::ToolRegistry,
            CheckCategory::SafetyInvariants,
            CheckCategory::IntentAlignment,
        ] {
            let score = compute_category_score(category, results);
            category_scores.insert(category, score);
        }

        let readiness = compute_readiness_score(&category_scores);
        assert!(
            readiness >= min_score,
            "Readiness score {} should be >= {}",
            readiness,
            min_score
        );
    }

    /// Assert check count
    pub fn assert_check_count(results: &[DodCheckResult], expected: usize) {
        assert_eq!(
            results.len(),
            expected,
            "Should have {} checks, got {}",
            expected,
            results.len()
        );
    }

    /// Assert category has checks
    pub fn assert_category_has_checks(results: &[DodCheckResult], category: CheckCategory) {
        let count = results.iter().filter(|r| r.category == category).count();
        assert!(count > 0, "Category {:?} should have checks", category);
    }

    /// Assert all checks have evidence
    pub fn assert_all_have_evidence(results: &[DodCheckResult]) {
        for result in results {
            if result.status == CheckStatus::Fail || result.status == CheckStatus::Warn {
                assert!(
                    !result.evidence.is_empty(),
                    "Check {} should have evidence",
                    result.id
                );
            }
        }
    }

    /// Assert remediation exists for failures
    pub fn assert_remediation_for_failures(
        results: &[DodCheckResult],
        suggestions: &[crate::dod::RemediationSuggestion],
    ) {
        let failed_check_ids: Vec<_> = results
            .iter()
            .filter(|r| r.status == CheckStatus::Fail)
            .map(|r| r.id.as_str())
            .collect();

        for failed_id in failed_check_ids {
            assert!(
                suggestions.iter().any(|s| s.check_id == failed_id),
                "Failed check {} should have remediation",
                failed_id
            );
        }
    }

    /// Assert duration is reasonable
    pub fn assert_reasonable_duration(results: &[DodCheckResult], max_total_ms: u64) {
        let total_duration: u64 = results.iter().map(|r| r.duration_ms).sum();
        assert!(
            total_duration <= max_total_ms,
            "Total duration {}ms exceeds max {}ms",
            total_duration,
            max_total_ms
        );
    }
}

/// Create a mock check result
pub fn mock_check_result(
    id: &str,
    category: CheckCategory,
    status: CheckStatus,
    severity: CheckSeverity,
) -> DodCheckResult {
    DodCheckResult {
        id: id.to_string(),
        category,
        status,
        severity,
        message: format!("Mock result for {}", id),
        evidence: vec![],
        remediation: vec![],
        duration_ms: 100,
        check_hash: format!("mock_{}", id),
    }
}

/// Create mock evidence
pub fn mock_evidence(kind: EvidenceKind, content: &str) -> Evidence {
    Evidence {
        kind,
        content: content.to_string(),
        file_path: None,
        line_number: None,
        hash: format!("hash_{}", content.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_creates_temp_dir() {
        let harness = DodTestHarness::new();
        assert!(harness.workspace_root().exists());
    }

    #[test]
    fn test_harness_creates_cargo_toml() {
        let harness = DodTestHarness::new();
        harness.create_cargo_toml("test").unwrap();

        let cargo_toml = harness.workspace_root().join("Cargo.toml");
        assert!(cargo_toml.exists());

        let content = fs::read_to_string(cargo_toml).unwrap();
        assert!(content.contains("test"));
    }

    #[test]
    fn test_harness_creates_valid_workspace() {
        let harness = DodTestHarness::new();
        harness.create_valid_workspace().unwrap();

        assert!(harness.workspace_root().join("Cargo.toml").exists());
        assert!(harness.workspace_root().join("src/lib.rs").exists());
        assert!(harness.workspace_root().join(".git").exists());
    }

    #[test]
    fn mock_check_result_helper() {
        let result = mock_check_result(
            "TEST_CHECK",
            CheckCategory::BuildCorrectness,
            CheckStatus::Pass,
            CheckSeverity::Fatal,
        );

        assert_eq!(result.id, "TEST_CHECK");
        assert_eq!(result.category, CheckCategory::BuildCorrectness);
        assert_eq!(result.status, CheckStatus::Pass);
        assert_eq!(result.severity, CheckSeverity::Fatal);
    }

    #[test]
    fn assertions_check_passed() {
        let results = vec![mock_check_result(
            "TEST",
            CheckCategory::BuildCorrectness,
            CheckStatus::Pass,
            CheckSeverity::Fatal,
        )];

        DodAssertions::assert_check_passed(&results, "TEST");
    }

    #[test]
    #[should_panic(expected = "should fail")]
    fn assertions_check_failed_panics_on_pass() {
        let results = vec![mock_check_result(
            "TEST",
            CheckCategory::BuildCorrectness,
            CheckStatus::Pass,
            CheckSeverity::Fatal,
        )];

        DodAssertions::assert_check_failed(&results, "TEST");
    }
}
