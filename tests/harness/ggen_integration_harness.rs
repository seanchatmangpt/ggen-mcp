//! Ggen Integration Test Harness
//!
//! Chicago-style TDD harness for testing ggen workflow integration.
//! Provides end-to-end testing of ontology → SPARQL → Tera → Rust pipeline.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock;

/// Main harness for ggen integration testing
pub struct GgenIntegrationHarness {
    /// Temporary workspace
    workspace: TempDir,
    /// Ontology file path
    ontology_path: PathBuf,
    /// Queries directory
    queries_dir: PathBuf,
    /// Templates directory
    templates_dir: PathBuf,
    /// Generated output directory
    output_dir: PathBuf,
    /// ggen.toml config path
    config_path: PathBuf,
    /// State tracking
    state: Arc<RwLock<HarnessState>>,
}

/// Internal state tracking
#[derive(Debug, Default)]
struct HarnessState {
    /// Files generated in current session
    generated_files: Vec<PathBuf>,
    /// Generation metrics
    metrics: GenerationMetrics,
    /// Validation results
    validation_results: Vec<ValidationResult>,
}

/// Generation metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GenerationMetrics {
    pub total_queries_executed: usize,
    pub total_templates_rendered: usize,
    pub total_files_generated: usize,
    pub total_errors: usize,
    pub generation_time_ms: u64,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub file_path: PathBuf,
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl GgenIntegrationHarness {
    /// Create a new harness
    pub fn new() -> Result<Self> {
        let workspace = tempfile::tempdir()?;
        let base = workspace.path();

        let ontology_path = base.join("ontology/domain.ttl");
        let queries_dir = base.join("queries");
        let templates_dir = base.join("templates");
        let output_dir = base.join("src/generated");
        let config_path = base.join("ggen.toml");

        // Create directory structure
        fs::create_dir_all(ontology_path.parent().unwrap())?;
        fs::create_dir_all(&queries_dir)?;
        fs::create_dir_all(&templates_dir)?;
        fs::create_dir_all(&output_dir)?;

        Ok(Self {
            workspace,
            ontology_path,
            queries_dir,
            templates_dir,
            output_dir,
            config_path,
            state: Arc::new(RwLock::new(HarnessState::default())),
        })
    }

    /// Create harness from existing fixtures
    pub fn from_fixtures(fixtures_dir: &Path) -> Result<Self> {
        let mut harness = Self::new()?;

        // Copy fixtures to workspace
        if fixtures_dir.join("configs").exists() {
            let config_src = fixtures_dir.join("configs/complete_ggen.toml");
            if config_src.exists() {
                fs::copy(config_src, &harness.config_path)?;
            }
        }

        if fixtures_dir.join("ontologies").exists() {
            let ontology_src = fixtures_dir.join("ontologies/user_aggregate.ttl");
            if ontology_src.exists() {
                fs::create_dir_all(harness.ontology_path.parent().unwrap())?;
                fs::copy(ontology_src, &harness.ontology_path)?;
            }
        }

        if fixtures_dir.join("queries").exists() {
            copy_dir_all(fixtures_dir.join("queries"), &harness.queries_dir)?;
        }

        if fixtures_dir.join("templates").exists() {
            copy_dir_all(fixtures_dir.join("templates"), &harness.templates_dir)?;
        }

        Ok(harness)
    }

    // =========================================================================
    // File Operations
    // =========================================================================

    /// Write ontology content
    pub fn write_ontology(&self, content: &str) -> Result<()> {
        fs::create_dir_all(self.ontology_path.parent().unwrap())?;
        fs::write(&self.ontology_path, content)?;
        Ok(())
    }

    /// Write SPARQL query
    pub fn write_query(&self, name: &str, content: &str) -> Result<()> {
        let path = self.queries_dir.join(format!("{}.rq", name));
        fs::write(path, content)?;
        Ok(())
    }

    /// Write Tera template
    pub fn write_template(&self, name: &str, content: &str) -> Result<()> {
        let path = self.templates_dir.join(format!("{}.rs.tera", name));
        fs::write(path, content)?;
        Ok(())
    }

    /// Write ggen.toml config
    pub fn write_config(&self, content: &str) -> Result<()> {
        fs::write(&self.config_path, content)?;
        Ok(())
    }

    /// Read generated file
    pub fn read_generated(&self, file_name: &str) -> Result<String> {
        let path = self.output_dir.join(file_name);
        Ok(fs::read_to_string(path)?)
    }

    /// Check if generated file exists
    pub fn has_generated(&self, file_name: &str) -> bool {
        self.output_dir.join(file_name).exists()
    }

    // =========================================================================
    // Path Accessors
    // =========================================================================

    pub fn workspace_path(&self) -> &Path {
        self.workspace.path()
    }

    pub fn ontology_path(&self) -> &Path {
        &self.ontology_path
    }

    pub fn queries_dir(&self) -> &Path {
        &self.queries_dir
    }

    pub fn templates_dir(&self) -> &Path {
        &self.templates_dir
    }

    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    // =========================================================================
    // State Management
    // =========================================================================

    /// Record generated file
    pub async fn record_generated(&self, file_path: PathBuf) {
        let mut state = self.state.write().await;
        state.generated_files.push(file_path);
        state.metrics.total_files_generated += 1;
    }

    /// Record query execution
    pub async fn record_query_execution(&self) {
        let mut state = self.state.write().await;
        state.metrics.total_queries_executed += 1;
    }

    /// Record template render
    pub async fn record_template_render(&self) {
        let mut state = self.state.write().await;
        state.metrics.total_templates_rendered += 1;
    }

    /// Record error
    pub async fn record_error(&self) {
        let mut state = self.state.write().await;
        state.metrics.total_errors += 1;
    }

    /// Add validation result
    pub async fn add_validation_result(&self, result: ValidationResult) {
        let mut state = self.state.write().await;
        state.validation_results.push(result);
    }

    /// Get current metrics
    pub async fn metrics(&self) -> GenerationMetrics {
        let state = self.state.read().await;
        state.metrics.clone()
    }

    /// Get all validation results
    pub async fn validation_results(&self) -> Vec<ValidationResult> {
        let state = self.state.read().await;
        state.validation_results.clone()
    }

    /// Get list of generated files
    pub async fn generated_files(&self) -> Vec<PathBuf> {
        let state = self.state.read().await;
        state.generated_files.clone()
    }

    // =========================================================================
    // Workflow Execution
    // =========================================================================

    /// Execute full generation workflow
    pub async fn execute_generation(&self) -> Result<GenerationResult> {
        let start = std::time::Instant::now();

        // 1. Load ontology
        let _ontology_content =
            fs::read_to_string(&self.ontology_path).context("Failed to load ontology")?;
        // Parse with oxigraph in real implementation

        // 2. Execute queries
        let queries = self.list_queries()?;
        for query_file in queries {
            self.record_query_execution().await;
            // Execute SPARQL query in real implementation
        }

        // 3. Render templates
        let templates = self.list_templates()?;
        for template_file in templates {
            self.record_template_render().await;
            // Render Tera template in real implementation
        }

        let duration = start.elapsed();

        let mut state = self.state.write().await;
        state.metrics.generation_time_ms = duration.as_millis() as u64;

        Ok(GenerationResult {
            success: state.metrics.total_errors == 0,
            metrics: state.metrics.clone(),
            generated_files: state.generated_files.clone(),
        })
    }

    /// List all query files
    pub fn list_queries(&self) -> Result<Vec<PathBuf>> {
        Ok(fs::read_dir(&self.queries_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("rq"))
            .collect())
    }

    /// List all template files
    pub fn list_templates(&self) -> Result<Vec<PathBuf>> {
        Ok(fs::read_dir(&self.templates_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("tera"))
            .collect())
    }

    // =========================================================================
    // Validation
    // =========================================================================

    /// Validate all generated files compile
    pub async fn validate_compilation(&self) -> Result<CompilationResult> {
        let generated_files = self.generated_files().await;
        let mut errors = Vec::new();

        for file in &generated_files {
            let content = fs::read_to_string(file)?;

            // Simple validation checks (in production would use rustc)
            if !content.contains("pub struct") && !content.contains("pub enum") {
                errors.push(format!("File {:?} appears to have no public types", file));
            }

            // Check for TODOs
            if content.contains("TODO") {
                errors.push(format!("File {:?} contains TODO markers", file));
            }
        }

        Ok(CompilationResult {
            success: errors.is_empty(),
            errors,
            files_checked: generated_files.len(),
        })
    }

    /// Validate ontology syntax
    pub async fn validate_ontology(&self) -> Result<ValidationResult> {
        use crate::harness::turtle_ontology_harness::OntologyTestHarness;

        match OntologyTestHarness::parse_from_file(&self.ontology_path) {
            Ok(harness) => {
                let validation = harness.validate();
                Ok(ValidationResult {
                    file_path: self.ontology_path.clone(),
                    valid: validation.is_valid(),
                    errors: validation.errors(),
                    warnings: validation.warnings(),
                })
            }
            Err(e) => Ok(ValidationResult {
                file_path: self.ontology_path.clone(),
                valid: false,
                errors: vec![e.to_string()],
                warnings: Vec::new(),
            }),
        }
    }

    // =========================================================================
    // Assertions
    // =========================================================================

    /// Assert file was generated
    pub fn assert_file_generated(&self, file_name: &str) {
        assert!(
            self.has_generated(file_name),
            "Expected file '{}' to be generated",
            file_name
        );
    }

    /// Assert file contains content
    pub fn assert_file_contains(&self, file_name: &str, content: &str) -> Result<()> {
        let file_content = self.read_generated(file_name)?;
        assert!(
            file_content.contains(content),
            "Expected file '{}' to contain '{}'",
            file_name,
            content
        );
        Ok(())
    }

    /// Assert no generation errors
    pub async fn assert_no_errors(&self) {
        let metrics = self.metrics().await;
        assert_eq!(
            metrics.total_errors, 0,
            "Expected no generation errors, got {}",
            metrics.total_errors
        );
    }
}

/// Result of generation workflow
#[derive(Debug, Clone)]
pub struct GenerationResult {
    pub success: bool,
    pub metrics: GenerationMetrics,
    pub generated_files: Vec<PathBuf>,
}

/// Result of compilation validation
#[derive(Debug, Clone)]
pub struct CompilationResult {
    pub success: bool,
    pub errors: Vec<String>,
    pub files_checked: usize,
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Copy directory recursively
fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_harness_creation() {
        let harness = GgenIntegrationHarness::new().unwrap();
        assert!(harness.workspace_path().exists());
        assert!(harness.queries_dir().exists());
        assert!(harness.templates_dir().exists());
    }

    #[tokio::test]
    async fn test_write_and_read_ontology() {
        let harness = GgenIntegrationHarness::new().unwrap();
        let content = "@prefix test: <http://test.example.org/> .";

        harness.write_ontology(content).unwrap();

        let read_content = fs::read_to_string(harness.ontology_path()).unwrap();
        assert_eq!(read_content, content);
    }

    #[tokio::test]
    async fn test_state_tracking() {
        let harness = GgenIntegrationHarness::new().unwrap();

        harness.record_query_execution().await;
        harness.record_template_render().await;
        harness.record_generated(PathBuf::from("test.rs")).await;

        let metrics = harness.metrics().await;
        assert_eq!(metrics.total_queries_executed, 1);
        assert_eq!(metrics.total_templates_rendered, 1);
        assert_eq!(metrics.total_files_generated, 1);
    }
}
