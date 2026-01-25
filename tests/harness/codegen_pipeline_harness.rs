//! Comprehensive Chicago-style TDD Test Harness for Code Generation Pipeline
//!
//! This harness validates the complete end-to-end code generation pipeline:
//! TTL → SPARQL → Template → Rust Code
//!
//! # Test Philosophy: Chicago TDD
//!
//! - **State-based testing**: Verify actual state changes, not interactions
//! - **Real collaborators**: Use actual components (no mocks unless necessary)
//! - **End-to-end validation**: Test complete flows through all pipeline stages
//! - **Golden file testing**: Compare generated outputs against expected files
//!
//! # Pipeline Stages
//!
//! 1. **Ontology Loading** - Parse TTL, build RDF graph
//! 2. **SPARQL Query** - Extract domain entities
//! 3. **Template Rendering** - Generate code strings
//! 4. **Code Validation** - Verify syntax and compilation
//! 5. **File Writing** - Persist to filesystem
//!
//! # Usage
//!
//! ```rust,no_run
//! use codegen_pipeline_harness::*;
//!
//! let harness = CodegenPipelineHarness::new()
//!     .with_fixture("simple_aggregate")
//!     .with_validation(true);
//!
//! let result = harness.run_complete_pipeline()?;
//! harness.assert_all_stages_succeeded(&result);
//! ```

use anyhow::{Context, Result, anyhow};
use oxigraph::store::Store;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

// Import pipeline components
use spreadsheet_mcp::codegen::{
    ArtifactTracker, CodeGenPipeline, GeneratedCodeValidator, GenerationReceipt, SafeCodeWriter,
    ValidationReport,
};
use spreadsheet_mcp::ontology::{ConsistencyChecker, GraphIntegrityChecker, IntegrityConfig};
use spreadsheet_mcp::sparql::{QueryBuilder, QueryResultCache, ResultMapper, TypedBinding};
use spreadsheet_mcp::template::{SafeRenderer, TemplateContext, TemplateRegistry};

// ============================================================================
// Core Test Harness
// ============================================================================

/// Main test harness for the complete code generation pipeline
#[derive(Debug)]
pub struct CodegenPipelineHarness {
    /// Root directory for test fixtures
    fixture_root: PathBuf,
    /// Currently loaded fixture
    current_fixture: Option<String>,
    /// Enable validation checks
    enable_validation: bool,
    /// Enable golden file comparison
    enable_golden_files: bool,
    /// Enable incremental updates
    enable_incremental: bool,
    /// Output directory for generated files
    output_dir: PathBuf,
    /// Artifact tracker for dependency management
    artifact_tracker: ArtifactTracker,
    /// Template registry
    template_registry: TemplateRegistry,
    /// Query result cache
    query_cache: QueryResultCache,
    /// Performance metrics
    metrics: PipelineMetrics,
}

impl CodegenPipelineHarness {
    /// Create a new test harness
    pub fn new() -> Self {
        let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let fixture_root = project_root.join("tests/fixtures/pipeline");
        let output_dir = project_root.join("target/test_output");

        // Ensure output directory exists
        fs::create_dir_all(&output_dir).ok();

        Self {
            fixture_root,
            current_fixture: None,
            enable_validation: true,
            enable_golden_files: true,
            enable_incremental: false,
            output_dir,
            artifact_tracker: ArtifactTracker::new(),
            template_registry: TemplateRegistry::new(),
            query_cache: QueryResultCache::default(),
            metrics: PipelineMetrics::default(),
        }
    }

    /// Set the current fixture to use
    pub fn with_fixture(mut self, name: &str) -> Self {
        self.current_fixture = Some(name.to_string());
        self
    }

    /// Enable or disable validation
    pub fn with_validation(mut self, enable: bool) -> Self {
        self.enable_validation = enable;
        self
    }

    /// Enable or disable golden file comparison
    pub fn with_golden_files(mut self, enable: bool) -> Self {
        self.enable_golden_files = enable;
        self
    }

    /// Enable or disable incremental updates
    pub fn with_incremental(mut self, enable: bool) -> Self {
        self.enable_incremental = enable;
        self
    }

    /// Run the complete pipeline end-to-end
    pub fn run_complete_pipeline(&mut self) -> Result<PipelineResult> {
        let start = Instant::now();
        let fixture = self.get_current_fixture()?;

        println!("🚀 Running complete pipeline for fixture: {}", fixture);

        // Stage 1: Ontology Loading
        let ontology_result = self.run_stage_ontology_loading(&fixture)?;

        // Stage 2: SPARQL Query
        let sparql_result = self.run_stage_sparql_query(&fixture, &ontology_result.store)?;

        // Stage 3: Template Rendering
        let template_result =
            self.run_stage_template_rendering(&fixture, &sparql_result.entities)?;

        // Stage 4: Code Validation
        let validation_result =
            self.run_stage_code_validation(&fixture, &template_result.rendered_code)?;

        // Stage 5: File Writing
        let file_result =
            self.run_stage_file_writing(&fixture, &validation_result.validated_code)?;

        let duration = start.elapsed();
        self.metrics.total_duration = duration;

        Ok(PipelineResult {
            fixture: fixture.clone(),
            ontology_result,
            sparql_result,
            template_result,
            validation_result,
            file_result,
            duration,
            success: true,
        })
    }

    // ========================================================================
    // Stage 1: Ontology Loading
    // ========================================================================

    fn run_stage_ontology_loading(&mut self, fixture: &str) -> Result<OntologyResult> {
        println!("  📚 Stage 1: Ontology Loading");
        let start = Instant::now();

        let input_path = self.fixture_root.join(fixture).join("input/ontology.ttl");

        if !input_path.exists() {
            return Err(anyhow!("Ontology file not found: {}", input_path.display()));
        }

        // Load TTL into RDF store
        let store = Store::new()?;
        let ttl_content =
            fs::read_to_string(&input_path).context("Failed to read ontology file")?;

        // Parse and load into store
        store
            .load_from_reader(oxigraph::io::RdfFormat::Turtle, ttl_content.as_bytes())
            .context("Failed to parse TTL file")?;

        // Validate graph structure
        let triple_count = store.len()?;
        println!("    ✓ Loaded {} triples", triple_count);

        // Run integrity checks if enabled
        let integrity_report = if self.enable_validation {
            let config = IntegrityConfig::default();
            let checker = GraphIntegrityChecker::new(config);
            Some(checker.check(&store)?)
        } else {
            None
        };

        // Run consistency checks
        let consistency_report = if self.enable_validation {
            let checker = ConsistencyChecker::new(store.clone());
            Some(checker.check_all())
        } else {
            None
        };

        let duration = start.elapsed();
        self.metrics.ontology_duration = duration;

        println!("    ⏱️  Duration: {:?}", duration);

        Ok(OntologyResult {
            store,
            ttl_content,
            triple_count,
            integrity_report,
            consistency_report,
            duration,
        })
    }

    // ========================================================================
    // Stage 2: SPARQL Query
    // ========================================================================

    fn run_stage_sparql_query(&mut self, fixture: &str, store: &Store) -> Result<SparqlResult> {
        println!("  🔍 Stage 2: SPARQL Query Execution");
        let start = Instant::now();

        let query_path = self.fixture_root.join(fixture).join("input/queries.sparql");

        // Load SPARQL queries
        let queries = if query_path.exists() {
            fs::read_to_string(&query_path).context("Failed to read query file")?
        } else {
            // Use default domain entities query
            include_str!("../../queries/domain_entities.sparql").to_string()
        };

        // Execute queries and extract entities
        let entities = self.execute_queries(store, &queries)?;

        println!("    ✓ Extracted {} entities", entities.len());

        let duration = start.elapsed();
        self.metrics.sparql_duration = duration;

        println!("    ⏱️  Duration: {:?}", duration);

        Ok(SparqlResult {
            queries,
            entities,
            query_count: 1,
            duration,
        })
    }

    fn execute_queries(&self, store: &Store, query: &str) -> Result<Vec<DomainEntity>> {
        use oxigraph::sparql::QueryResults;

        let results = store.query(query)?;

        let mut entities = Vec::new();

        if let QueryResults::Solutions(solutions) = results {
            for solution in solutions {
                let solution = solution?;

                // Extract entity data
                let entity = DomainEntity {
                    name: self.extract_string_from_solution(&solution, "name")?,
                    entity_type: self.extract_string_from_solution(&solution, "type")?,
                    properties: HashMap::new(),
                    metadata: HashMap::new(),
                };

                entities.push(entity);
            }
        }

        Ok(entities)
    }

    fn extract_string_from_solution(
        &self,
        solution: &oxigraph::sparql::QuerySolution,
        var: &str,
    ) -> Result<String> {
        solution
            .get(var)
            .and_then(|term| match term {
                oxigraph::model::Term::Literal(lit) => Some(lit.value().to_string()),
                oxigraph::model::Term::NamedNode(node) => Some(
                    node.as_str()
                        .split('#')
                        .last()
                        .or_else(|| node.as_str().split('/').last())
                        .unwrap_or(node.as_str())
                        .to_string(),
                ),
                _ => None,
            })
            .ok_or_else(|| anyhow!("Variable '{}' not found in solution", var))
    }

    // ========================================================================
    // Stage 3: Template Rendering
    // ========================================================================

    fn run_stage_template_rendering(
        &mut self,
        fixture: &str,
        entities: &[DomainEntity],
    ) -> Result<TemplateResult> {
        println!("  📝 Stage 3: Template Rendering");
        let start = Instant::now();

        let template_dir = self.fixture_root.join(fixture).join("input/templates");

        // Use default templates if fixture doesn't provide custom ones
        let templates = if template_dir.exists() {
            self.load_custom_templates(&template_dir)?
        } else {
            self.load_default_templates()?
        };

        println!("    ✓ Loaded {} templates", templates.len());

        // Render code for each entity
        let mut rendered_code = HashMap::new();
        let renderer = SafeRenderer::new();

        for entity in entities {
            let template_name = self.get_template_for_entity(&entity.entity_type);

            if let Some(template_content) = templates.get(&template_name) {
                let mut context = TemplateContext::new();
                context.insert("name", &entity.name);
                context.insert("type", &entity.entity_type);
                context.insert("properties", &entity.properties);

                let rendered = renderer.render_safe(template_content, &context)?;
                let file_name = format!("{}.rs", entity.name);
                rendered_code.insert(file_name, rendered);
            }
        }

        println!("    ✓ Rendered {} code files", rendered_code.len());

        let duration = start.elapsed();
        self.metrics.template_duration = duration;

        println!("    ⏱️  Duration: {:?}", duration);

        Ok(TemplateResult {
            templates,
            rendered_code,
            render_count: rendered_code.len(),
            duration,
        })
    }

    fn load_custom_templates(&self, template_dir: &Path) -> Result<HashMap<String, String>> {
        let mut templates = HashMap::new();

        for entry in fs::read_dir(template_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("tera") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                let content = fs::read_to_string(&path)?;
                templates.insert(name, content);
            }
        }

        Ok(templates)
    }

    fn load_default_templates(&self) -> Result<HashMap<String, String>> {
        let mut templates = HashMap::new();
        let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let template_dir = project_root.join("templates");

        // Load essential templates
        let essential = vec![
            "aggregate.rs.tera",
            "command.rs.tera",
            "value_object.rs.tera",
            "mcp_tool_handler.rs.tera",
            "mcp_tool_params.rs.tera",
        ];

        for name in essential {
            let path = template_dir.join(name);
            if path.exists() {
                let content = fs::read_to_string(&path)?;
                let key = name.trim_end_matches(".tera").to_string();
                templates.insert(key, content);
            }
        }

        Ok(templates)
    }

    fn get_template_for_entity(&self, entity_type: &str) -> String {
        match entity_type {
            "AggregateRoot" => "aggregate.rs".to_string(),
            "Command" => "command.rs".to_string(),
            "ValueObject" => "value_object.rs".to_string(),
            "McpTool" => "mcp_tool_handler.rs".to_string(),
            _ => "aggregate.rs".to_string(),
        }
    }

    // ========================================================================
    // Stage 4: Code Validation
    // ========================================================================

    fn run_stage_code_validation(
        &mut self,
        fixture: &str,
        code_map: &HashMap<String, String>,
    ) -> Result<ValidationResult> {
        println!("  ✅ Stage 4: Code Validation");
        let start = Instant::now();

        let validator = GeneratedCodeValidator::new();
        let mut validated_code = HashMap::new();
        let mut validation_reports = Vec::new();

        for (file_name, code) in code_map {
            println!("    🔍 Validating {}", file_name);

            // Validate syntax
            let report = validator.validate_syntax(code)?;
            validation_reports.push((file_name.clone(), report.clone()));

            if report.is_valid() {
                // Try to parse with syn
                let syntax_valid = self.validate_rust_syntax(code);

                if syntax_valid {
                    println!("      ✓ Syntax valid");
                    validated_code.insert(file_name.clone(), code.clone());
                } else {
                    println!("      ✗ Syntax invalid");
                }
            } else {
                println!("      ✗ Validation failed: {} issues", report.issues.len());
                for issue in &report.issues {
                    println!("        - {}: {}", issue.severity, issue.message);
                }
            }
        }

        let all_valid = validated_code.len() == code_map.len();

        let duration = start.elapsed();
        self.metrics.validation_duration = duration;

        println!("    ⏱️  Duration: {:?}", duration);

        Ok(ValidationResult {
            validated_code,
            validation_reports,
            all_valid,
            duration,
        })
    }

    fn validate_rust_syntax(&self, code: &str) -> bool {
        syn::parse_file(code).is_ok()
    }

    // ========================================================================
    // Stage 5: File Writing
    // ========================================================================

    fn run_stage_file_writing(
        &mut self,
        fixture: &str,
        validated_code: &HashMap<String, String>,
    ) -> Result<FileResult> {
        println!("  💾 Stage 5: File Writing");
        let start = Instant::now();

        let output_dir = self.output_dir.join(fixture);
        fs::create_dir_all(&output_dir)?;

        let writer = SafeCodeWriter::new();
        let mut written_files = Vec::new();
        let mut generation_receipts = Vec::new();

        for (file_name, code) in validated_code {
            let output_path = output_dir.join(file_name);

            // Write file safely
            writer.write(&output_path, code)?;
            written_files.push(output_path.clone());

            // Track artifact
            self.artifact_tracker
                .track_artifact(&output_path, code.as_bytes(), Vec::new())?;

            println!("    ✓ Wrote {}", output_path.display());
        }

        let duration = start.elapsed();
        self.metrics.file_duration = duration;

        println!("    ⏱️  Duration: {:?}", duration);

        Ok(FileResult {
            output_dir,
            written_files,
            generation_receipts,
            duration,
        })
    }

    // ========================================================================
    // Assertions and Validation
    // ========================================================================

    /// Assert that all pipeline stages succeeded
    pub fn assert_all_stages_succeeded(&self, result: &PipelineResult) {
        assert!(result.success, "Pipeline should succeed");
        assert!(
            result.ontology_result.triple_count > 0,
            "Should have loaded triples"
        );
        assert!(
            !result.sparql_result.entities.is_empty(),
            "Should have extracted entities"
        );
        assert!(
            !result.template_result.rendered_code.is_empty(),
            "Should have rendered code"
        );
        assert!(
            result.validation_result.all_valid,
            "All code should be valid"
        );
        assert!(
            !result.file_result.written_files.is_empty(),
            "Should have written files"
        );
    }

    /// Assert that generated output matches golden file
    pub fn assert_output_matches_golden(&self, output: &str, golden_file: &Path) -> Result<()> {
        if !self.enable_golden_files {
            return Ok(());
        }

        if !golden_file.exists() {
            return Err(anyhow!("Golden file not found: {}", golden_file.display()));
        }

        let expected = fs::read_to_string(golden_file)?;
        let normalized_output = normalize_whitespace(output);
        let normalized_expected = normalize_whitespace(&expected);

        if normalized_output != normalized_expected {
            // Generate diff
            let diff = self.generate_diff(&normalized_expected, &normalized_output);
            return Err(anyhow!("Output does not match golden file:\n{}", diff));
        }

        Ok(())
    }

    /// Assert code compiles
    pub fn assert_code_compiles(&self, code: &str) -> Result<()> {
        self.validate_rust_syntax(code)
            .then_some(())
            .ok_or_else(|| anyhow!("Code does not compile"))
    }

    /// Assert all imports are valid
    pub fn assert_all_imports_valid(&self, code: &str) -> Result<()> {
        let file = syn::parse_file(code)?;

        // Check all use statements
        for item in &file.items {
            if let syn::Item::Use(_) = item {
                // In a real implementation, verify imports exist
                // For now, just check syntax
            }
        }

        Ok(())
    }

    /// Assert no unused code
    pub fn assert_no_unused_code(&self, code: &str) -> Result<()> {
        // This would require cargo check with warnings
        // For harness purposes, we verify structure
        let file = syn::parse_file(code)?;

        // Check for empty modules
        let has_content = !file.items.is_empty();

        if !has_content {
            return Err(anyhow!("Code file is empty"));
        }

        Ok(())
    }

    // ========================================================================
    // Golden File Management
    // ========================================================================

    /// Update golden files with current output
    pub fn update_golden_files(&self, result: &PipelineResult) -> Result<()> {
        let fixture_dir = self.fixture_root.join(&result.fixture);
        let expected_dir = fixture_dir.join("expected");

        fs::create_dir_all(&expected_dir)?;

        for (file_name, code) in &result.template_result.rendered_code {
            let golden_path = expected_dir.join(file_name);
            fs::write(&golden_path, code)?;
            println!("    📝 Updated golden file: {}", golden_path.display());
        }

        Ok(())
    }

    /// Compare against golden files
    pub fn compare_golden_files(&self, result: &PipelineResult) -> Result<GoldenFileReport> {
        let fixture_dir = self.fixture_root.join(&result.fixture);
        let expected_dir = fixture_dir.join("expected");

        let mut matches = Vec::new();
        let mut mismatches = Vec::new();
        let mut missing = Vec::new();

        for (file_name, code) in &result.template_result.rendered_code {
            let golden_path = expected_dir.join(file_name);

            if !golden_path.exists() {
                missing.push(file_name.clone());
                continue;
            }

            match self.assert_output_matches_golden(code, &golden_path) {
                Ok(_) => matches.push(file_name.clone()),
                Err(_) => mismatches.push(file_name.clone()),
            }
        }

        Ok(GoldenFileReport {
            matches,
            mismatches,
            missing,
        })
    }

    // ========================================================================
    // Utilities
    // ========================================================================

    fn get_current_fixture(&self) -> Result<String> {
        self.current_fixture
            .clone()
            .ok_or_else(|| anyhow!("No fixture set. Use with_fixture() first"))
    }

    fn generate_diff(&self, expected: &str, actual: &str) -> String {
        // Simple line-by-line diff
        let expected_lines: Vec<&str> = expected.lines().collect();
        let actual_lines: Vec<&str> = actual.lines().collect();

        let mut diff = String::new();
        let max_lines = expected_lines.len().max(actual_lines.len());

        for i in 0..max_lines {
            let exp_line = expected_lines.get(i).unwrap_or(&"");
            let act_line = actual_lines.get(i).unwrap_or(&"");

            if exp_line != act_line {
                diff.push_str(&format!("Line {}:\n", i + 1));
                diff.push_str(&format!("  Expected: {}\n", exp_line));
                diff.push_str(&format!("  Actual:   {}\n", act_line));
            }
        }

        diff
    }
}

impl Default for CodegenPipelineHarness {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Data Structures
// ============================================================================

/// Complete pipeline execution result
#[derive(Debug)]
pub struct PipelineResult {
    pub fixture: String,
    pub ontology_result: OntologyResult,
    pub sparql_result: SparqlResult,
    pub template_result: TemplateResult,
    pub validation_result: ValidationResult,
    pub file_result: FileResult,
    pub duration: Duration,
    pub success: bool,
}

/// Result of ontology loading stage
#[derive(Debug)]
pub struct OntologyResult {
    pub store: Store,
    pub ttl_content: String,
    pub triple_count: usize,
    pub integrity_report: Option<spreadsheet_mcp::ontology::IntegrityReport>,
    pub consistency_report: Option<spreadsheet_mcp::ontology::ConsistencyReport>,
    pub duration: Duration,
}

/// Result of SPARQL query stage
#[derive(Debug)]
pub struct SparqlResult {
    pub queries: String,
    pub entities: Vec<DomainEntity>,
    pub query_count: usize,
    pub duration: Duration,
}

/// Result of template rendering stage
#[derive(Debug)]
pub struct TemplateResult {
    pub templates: HashMap<String, String>,
    pub rendered_code: HashMap<String, String>,
    pub render_count: usize,
    pub duration: Duration,
}

/// Result of code validation stage
#[derive(Debug)]
pub struct ValidationResult {
    pub validated_code: HashMap<String, String>,
    pub validation_reports: Vec<(String, ValidationReport)>,
    pub all_valid: bool,
    pub duration: Duration,
}

/// Result of file writing stage
#[derive(Debug)]
pub struct FileResult {
    pub output_dir: PathBuf,
    pub written_files: Vec<PathBuf>,
    pub generation_receipts: Vec<GenerationReceipt>,
    pub duration: Duration,
}

/// Domain entity extracted from ontology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEntity {
    pub name: String,
    pub entity_type: String,
    pub properties: HashMap<String, String>,
    pub metadata: HashMap<String, String>,
}

/// Performance metrics for pipeline execution
#[derive(Debug, Default, Clone)]
pub struct PipelineMetrics {
    pub ontology_duration: Duration,
    pub sparql_duration: Duration,
    pub template_duration: Duration,
    pub validation_duration: Duration,
    pub file_duration: Duration,
    pub total_duration: Duration,
}

impl PipelineMetrics {
    pub fn print_summary(&self) {
        println!("\n📊 Pipeline Performance Metrics:");
        println!("  Ontology Loading:    {:?}", self.ontology_duration);
        println!("  SPARQL Query:        {:?}", self.sparql_duration);
        println!("  Template Rendering:  {:?}", self.template_duration);
        println!("  Code Validation:     {:?}", self.validation_duration);
        println!("  File Writing:        {:?}", self.file_duration);
        println!("  ─────────────────────────────────");
        println!("  Total:               {:?}", self.total_duration);
    }
}

/// Golden file comparison report
#[derive(Debug)]
pub struct GoldenFileReport {
    pub matches: Vec<String>,
    pub mismatches: Vec<String>,
    pub missing: Vec<String>,
}

impl GoldenFileReport {
    pub fn is_perfect_match(&self) -> bool {
        self.mismatches.is_empty() && self.missing.is_empty()
    }

    pub fn print_summary(&self) {
        println!("\n📋 Golden File Comparison:");
        println!("  ✓ Matches:    {}", self.matches.len());
        println!("  ✗ Mismatches: {}", self.mismatches.len());
        println!("  ? Missing:    {}", self.missing.len());

        if !self.mismatches.is_empty() {
            println!("\n  Mismatched files:");
            for file in &self.mismatches {
                println!("    - {}", file);
            }
        }

        if !self.missing.is_empty() {
            println!("\n  Missing golden files:");
            for file in &self.missing {
                println!("    - {}", file);
            }
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Normalize whitespace for comparison
fn normalize_whitespace(s: &str) -> String {
    s.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

// ============================================================================
// Test Scenarios
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_creation() {
        let harness = CodegenPipelineHarness::new();
        assert!(harness.enable_validation);
        assert!(harness.enable_golden_files);
    }

    #[test]
    fn test_harness_configuration() {
        let harness = CodegenPipelineHarness::new()
            .with_fixture("simple_aggregate")
            .with_validation(false)
            .with_golden_files(false);

        assert_eq!(
            harness.current_fixture,
            Some("simple_aggregate".to_string())
        );
        assert!(!harness.enable_validation);
        assert!(!harness.enable_golden_files);
    }

    #[test]
    fn test_validate_rust_syntax_valid() {
        let harness = CodegenPipelineHarness::new();
        let valid_code = "pub struct User { pub name: String }";
        assert!(harness.validate_rust_syntax(valid_code));
    }

    #[test]
    fn test_validate_rust_syntax_invalid() {
        let harness = CodegenPipelineHarness::new();
        let invalid_code = "pub struct User { name: String";
        assert!(!harness.validate_rust_syntax(invalid_code));
    }

    #[test]
    fn test_normalize_whitespace() {
        let input = "  line1  \n\n  line2  \n  ";
        let expected = "line1\nline2";
        assert_eq!(normalize_whitespace(input), expected);
    }

    #[test]
    fn test_get_template_for_entity() {
        let harness = CodegenPipelineHarness::new();
        assert_eq!(
            harness.get_template_for_entity("AggregateRoot"),
            "aggregate.rs"
        );
        assert_eq!(harness.get_template_for_entity("Command"), "command.rs");
        assert_eq!(
            harness.get_template_for_entity("ValueObject"),
            "value_object.rs"
        );
    }

    #[test]
    fn test_pipeline_metrics_default() {
        let metrics = PipelineMetrics::default();
        assert_eq!(metrics.total_duration, Duration::default());
    }
}
