//! Ontology Generation Test Harness
//!
//! Chicago-style TDD harness for testing ontology-driven code generation:
//! - Load RDF ontology → Execute SPARQL queries → Render Tera templates → Validate output
//!
//! # Test Philosophy
//!
//! - **State-based testing**: Verify generated artifacts, not internal calls
//! - **Real implementations**: Use actual OntologyEngine, SPARQL, Tera (no mocks)
//! - **Golden file validation**: Compare outputs against expected files
//! - **End-to-end workflows**: Test complete generation pipeline
//!
//! # Usage
//!
//! ```rust,no_run
//! use ontology_generation_harness::*;
//!
//! let mut harness = OntologyGenerationHarness::new()
//!     .with_fixture("test-api")
//!     .with_preview_mode(false);
//!
//! let result = harness.execute_workflow()?;
//! harness.verify_output(&result)?;
//! harness.compare_golden_files(&result)?;
//! ```

use anyhow::{Context, Result, anyhow};
use oxigraph::store::Store;
use oxigraph::sparql::{QueryResults, QuerySolution};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tera::{Context as TeraContext, Tera};

// Import pipeline components
use spreadsheet_mcp::ontology::{ConsistencyChecker, GraphIntegrityChecker, IntegrityConfig};
use spreadsheet_mcp::sparql::{QueryBuilder, QueryResultCache, ResultMapper};

// ============================================================================
// Core Test Harness
// ============================================================================

/// Test harness for ontology-driven code generation workflows
#[derive(Debug)]
pub struct OntologyGenerationHarness {
    /// Root directory for test fixtures
    fixture_root: PathBuf,
    /// Currently loaded fixture name
    current_fixture: Option<String>,
    /// Preview mode (render without writing)
    preview_mode: bool,
    /// Enable golden file comparison
    enable_golden_comparison: bool,
    /// Output directory for generated files
    output_dir: PathBuf,
    /// Golden files directory
    golden_dir: PathBuf,
    /// RDF store (loaded ontology)
    store: Option<Store>,
    /// Registered SPARQL queries
    queries: HashMap<String, String>,
    /// Tera template engine
    tera: Tera,
    /// Registered template contents
    template_contents: HashMap<String, String>,
    /// Query result cache
    query_cache: QueryResultCache,
    /// Workflow execution metrics
    metrics: WorkflowMetrics,
}

impl OntologyGenerationHarness {
    /// Create new harness instance
    pub fn new() -> Self {
        let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let fixture_root = project_root.join("tests/fixtures");
        let output_dir = project_root.join("target/test_ontology_output");
        let golden_dir = project_root.join("tests/golden");

        // Ensure directories exist
        fs::create_dir_all(&output_dir).ok();
        fs::create_dir_all(&golden_dir).ok();

        Self {
            fixture_root,
            current_fixture: None,
            preview_mode: false,
            enable_golden_comparison: true,
            output_dir,
            golden_dir,
            store: None,
            queries: HashMap::new(),
            tera: Tera::default(),
            template_contents: HashMap::new(),
            query_cache: QueryResultCache::default(),
            metrics: WorkflowMetrics::default(),
        }
    }

    /// Set current fixture
    pub fn with_fixture(mut self, name: &str) -> Self {
        self.current_fixture = Some(name.to_string());
        self
    }

    /// Enable/disable preview mode (render without writing)
    pub fn with_preview_mode(mut self, enabled: bool) -> Self {
        self.preview_mode = enabled;
        self
    }

    /// Enable/disable golden file comparison
    pub fn with_golden_comparison(mut self, enabled: bool) -> Self {
        self.enable_golden_comparison = enabled;
        self
    }

    // ========================================================================
    // Setup & Teardown
    // ========================================================================

    /// Load test ontology from fixture
    pub fn load_ontology(&mut self, fixture: &str) -> Result<()> {
        let start = Instant::now();
        let ontology_path = self.fixture_root.join("ontology").join(format!("{}.ttl", fixture));

        if !ontology_path.exists() {
            return Err(anyhow!(
                "Ontology fixture not found: {}",
                ontology_path.display()
            ));
        }

        let store = Store::new().context("Failed to create RDF store")?;
        let content = fs::read_to_string(&ontology_path)
            .context(format!("Failed to read ontology: {}", ontology_path.display()))?;

        store
            .load_from_reader(
                oxigraph::io::RdfFormat::Turtle,
                content.as_bytes(),
            )
            .context("Failed to parse Turtle ontology")?;

        self.store = Some(store);
        self.metrics.ontology_load_time = start.elapsed();

        println!("✓ Loaded ontology: {} ({:?})", fixture, self.metrics.ontology_load_time);
        Ok(())
    }

    /// Register SPARQL query from file
    pub fn register_query(&mut self, name: &str, query_file: &str) -> Result<()> {
        let query_path = self.fixture_root.join("queries").join(query_file);

        if !query_path.exists() {
            return Err(anyhow!(
                "Query file not found: {}",
                query_path.display()
            ));
        }

        let query = fs::read_to_string(&query_path)
            .context(format!("Failed to read query: {}", query_path.display()))?;

        self.queries.insert(name.to_string(), query);
        println!("✓ Registered query: {} from {}", name, query_file);
        Ok(())
    }

    /// Register Tera template from file
    pub fn register_template(&mut self, name: &str, template_file: &str) -> Result<()> {
        let template_path = self.fixture_root.join("templates").join(template_file);

        if !template_path.exists() {
            return Err(anyhow!(
                "Template file not found: {}",
                template_path.display()
            ));
        }

        let template_content = fs::read_to_string(&template_path)
            .context(format!("Failed to read template: {}", template_path.display()))?;

        // Add raw template to Tera
        self.tera.add_raw_template(name, &template_content)
            .context("Failed to register template")?;

        // Store content for later reference
        self.template_contents.insert(name.to_string(), template_content);

        println!("✓ Registered template: {} from {}", name, template_file);
        Ok(())
    }

    /// Clean up generated files
    pub fn teardown(&mut self) -> Result<()> {
        if self.output_dir.exists() {
            fs::remove_dir_all(&self.output_dir)
                .context("Failed to clean up output directory")?;
            fs::create_dir_all(&self.output_dir)?;
        }
        Ok(())
    }

    // ========================================================================
    // Workflow Execution
    // ========================================================================

    /// Execute complete workflow: Load → Query → Render → Write → Validate
    pub fn execute_workflow(&mut self) -> Result<WorkflowResult> {
        let fixture = self.get_current_fixture()?;
        let start = Instant::now();

        println!("🚀 Executing workflow for fixture: {}", fixture);

        // Step 1: Load ontology (if not already loaded)
        if self.store.is_none() {
            self.load_ontology(&fixture)?;
        }

        // Step 2: Execute all registered queries
        let query_results = self.execute_queries()?;

        // Step 3: Render all templates with query results
        let rendered_outputs = self.render_templates(&query_results)?;

        // Step 4: Write outputs (unless preview mode)
        let written_files = if !self.preview_mode {
            self.write_outputs(&rendered_outputs)?
        } else {
            println!("⏭️  Preview mode: skipping file writes");
            vec![]
        };

        // Step 5: Validate generated outputs
        let validation_report = self.validate_outputs(&rendered_outputs)?;

        let total_time = start.elapsed();
        self.metrics.total_workflow_time = total_time;

        println!("✅ Workflow completed in {:?}", total_time);

        Ok(WorkflowResult {
            fixture: fixture.clone(),
            query_results,
            rendered_outputs,
            written_files,
            validation_report,
            metrics: self.metrics.clone(),
        })
    }

    /// Execute all registered SPARQL queries
    fn execute_queries(&mut self) -> Result<HashMap<String, QueryResult>> {
        let start = Instant::now();
        let store = self.store.as_ref()
            .ok_or_else(|| anyhow!("No ontology loaded"))?;

        let mut results = HashMap::new();

        for (name, query_str) in &self.queries {
            println!("  🔍 Executing query: {}", name);
            let query_start = Instant::now();

            // Check cache first
            let cached = self.query_cache.get(query_str);
            let bindings = if let Some(cached_result) = cached {
                println!("    ⚡ Cache hit for query: {}", name);
                cached_result
            } else {
                let query_results = store
                    .query(query_str)
                    .context(format!("Failed to execute query: {}", name))?;

                let bindings = self.extract_bindings(query_results)?;
                self.query_cache.insert(query_str.clone(), bindings.clone());
                bindings
            };

            let elapsed = query_start.elapsed();
            println!("    ✓ Query completed in {:?} ({} results)", elapsed, bindings.len());

            results.insert(
                name.clone(),
                QueryResult {
                    query_name: name.clone(),
                    bindings,
                    execution_time: elapsed,
                    from_cache: cached.is_some(),
                },
            );
        }

        self.metrics.query_execution_time = start.elapsed();
        Ok(results)
    }

    /// Extract bindings from SPARQL query results
    fn extract_bindings(&self, results: QueryResults) -> Result<Vec<HashMap<String, String>>> {
        let mut bindings = Vec::new();

        if let QueryResults::Solutions(solutions) = results {
            for solution in solutions {
                let solution = solution.context("Failed to read query solution")?;
                let mut binding_map = HashMap::new();

                for (var, term) in solution.iter() {
                    binding_map.insert(var.as_str().to_string(), term.to_string());
                }

                bindings.push(binding_map);
            }
        }

        Ok(bindings)
    }

    /// Render all templates with query results
    fn render_templates(
        &mut self,
        query_results: &HashMap<String, QueryResult>,
    ) -> Result<HashMap<String, RenderedOutput>> {
        let start = Instant::now();
        let mut outputs = HashMap::new();

        // Get template names from registered templates
        let template_names: Vec<String> = self.template_contents.keys().cloned().collect();

        for template_name in template_names {
            println!("  📝 Rendering template: {}", template_name);
            let render_start = Instant::now();

            // Build context from query results
            let mut context = TeraContext::new();
            for (query_name, result) in query_results {
                context.insert(query_name, &result.bindings);
            }

            let rendered = self.tera.render(&template_name, &context)
                .context(format!("Failed to render template: {}", template_name))?;

            let elapsed = render_start.elapsed();
            println!("    ✓ Rendered {} chars in {:?}", rendered.len(), elapsed);

            outputs.insert(
                template_name.clone(),
                RenderedOutput {
                    template_name: template_name.clone(),
                    content: rendered,
                    render_time: elapsed,
                },
            );
        }

        self.metrics.template_render_time = start.elapsed();
        Ok(outputs)
    }

    /// Write rendered outputs to files
    fn write_outputs(
        &self,
        rendered: &HashMap<String, RenderedOutput>,
    ) -> Result<Vec<PathBuf>> {
        let start = Instant::now();
        let mut written_files = Vec::new();

        for (name, output) in rendered {
            // Determine output filename from template name
            let filename = self.template_name_to_filename(name);
            let output_path = self.output_dir.join(&filename);

            println!("  💾 Writing: {}", output_path.display());
            fs::write(&output_path, &output.content)
                .context(format!("Failed to write: {}", output_path.display()))?;

            written_files.push(output_path);
        }

        println!("✓ Wrote {} files in {:?}", written_files.len(), start.elapsed());
        Ok(written_files)
    }

    /// Validate generated outputs
    fn validate_outputs(
        &self,
        rendered: &HashMap<String, RenderedOutput>,
    ) -> Result<ValidationReport> {
        let start = Instant::now();
        let mut report = ValidationReport {
            valid: true,
            checks: Vec::new(),
        };

        for (name, output) in rendered {
            println!("  ✅ Validating: {}", name);

            // Check 1: Non-empty output
            if output.content.is_empty() {
                report.valid = false;
                report.checks.push(ValidationCheck {
                    check_name: format!("{}_non_empty", name),
                    passed: false,
                    message: "Output is empty".to_string(),
                });
            } else {
                report.checks.push(ValidationCheck {
                    check_name: format!("{}_non_empty", name),
                    passed: true,
                    message: format!("{} chars generated", output.content.len()),
                });
            }

            // Check 2: No TODO markers
            if output.content.contains("TODO") || output.content.contains("FIXME") {
                report.valid = false;
                report.checks.push(ValidationCheck {
                    check_name: format!("{}_no_todos", name),
                    passed: false,
                    message: "Output contains TODO/FIXME markers".to_string(),
                });
            } else {
                report.checks.push(ValidationCheck {
                    check_name: format!("{}_no_todos", name),
                    passed: true,
                    message: "No TODO markers found".to_string(),
                });
            }

            // Check 3: Minimum size (>100 bytes)
            if output.content.len() < 100 {
                report.valid = false;
                report.checks.push(ValidationCheck {
                    check_name: format!("{}_min_size", name),
                    passed: false,
                    message: format!("Output too small: {} bytes", output.content.len()),
                });
            } else {
                report.checks.push(ValidationCheck {
                    check_name: format!("{}_min_size", name),
                    passed: true,
                    message: format!("{} bytes (>100)", output.content.len()),
                });
            }
        }

        println!("✓ Validation completed in {:?}", start.elapsed());
        Ok(report)
    }

    // ========================================================================
    // Verification & Assertions
    // ========================================================================

    /// Verify workflow output meets all requirements
    pub fn verify_output(&self, result: &WorkflowResult) -> Result<()> {
        println!("🔍 Verifying workflow output...");

        // Verify all queries returned results
        for (name, query_result) in &result.query_results {
            if query_result.bindings.is_empty() {
                return Err(anyhow!("Query '{}' returned no results", name));
            }
        }

        // Verify all templates rendered successfully
        if result.rendered_outputs.is_empty() {
            return Err(anyhow!("No templates were rendered"));
        }

        // Verify validation passed
        if !result.validation_report.valid {
            let failed_checks: Vec<_> = result
                .validation_report
                .checks
                .iter()
                .filter(|c| !c.passed)
                .collect();

            return Err(anyhow!(
                "Validation failed: {} checks failed\n{}",
                failed_checks.len(),
                failed_checks
                    .iter()
                    .map(|c| format!("  ✗ {}: {}", c.check_name, c.message))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        println!("✅ All verifications passed");
        Ok(())
    }

    /// Compare generated outputs against golden files
    pub fn compare_golden_files(&self, result: &WorkflowResult) -> Result<GoldenFileComparison> {
        if !self.enable_golden_comparison {
            println!("⏭️  Golden file comparison disabled");
            return Ok(GoldenFileComparison {
                enabled: false,
                comparisons: Vec::new(),
                all_match: true,
            });
        }

        println!("📂 Comparing against golden files...");
        let mut comparisons = Vec::new();
        let mut all_match = true;

        for (name, output) in &result.rendered_outputs {
            let golden_filename = self.template_name_to_filename(name);
            let golden_path = self.golden_dir.join(&golden_filename);

            if !golden_path.exists() {
                println!("  ⚠️  No golden file for: {} (expected: {})", name, golden_path.display());
                comparisons.push(FileComparison {
                    template_name: name.clone(),
                    golden_path: golden_path.clone(),
                    matches: false,
                    difference: Some("Golden file does not exist".to_string()),
                });
                all_match = false;
                continue;
            }

            let golden_content = fs::read_to_string(&golden_path)
                .context(format!("Failed to read golden file: {}", golden_path.display()))?;

            let matches = golden_content == output.content;
            if matches {
                println!("  ✅ Matches golden: {}", name);
                comparisons.push(FileComparison {
                    template_name: name.clone(),
                    golden_path: golden_path.clone(),
                    matches: true,
                    difference: None,
                });
            } else {
                println!("  ❌ Differs from golden: {}", name);
                let diff = self.compute_diff(&golden_content, &output.content);
                comparisons.push(FileComparison {
                    template_name: name.clone(),
                    golden_path: golden_path.clone(),
                    matches: false,
                    difference: Some(diff),
                });
                all_match = false;
            }
        }

        Ok(GoldenFileComparison {
            enabled: true,
            comparisons,
            all_match,
        })
    }

    /// Test cache hit on second query execution
    pub fn test_cache_hit(&mut self) -> Result<CacheTestResult> {
        println!("🔄 Testing query cache...");

        // First execution (cache miss)
        let first_results = self.execute_queries()?;
        let first_times: HashMap<String, Duration> = first_results
            .iter()
            .map(|(k, v)| (k.clone(), v.execution_time))
            .collect();

        // Clear in-memory results but keep cache
        // Second execution (should be cache hit)
        let second_results = self.execute_queries()?;
        let cache_hits = second_results
            .values()
            .filter(|r| r.from_cache)
            .count();

        let all_cached = cache_hits == second_results.len();

        println!(
            "✓ Cache test: {}/{} queries from cache",
            cache_hits,
            second_results.len()
        );

        Ok(CacheTestResult {
            first_execution_times: first_times,
            cache_hits,
            total_queries: second_results.len(),
            all_cached,
        })
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    fn get_current_fixture(&self) -> Result<String> {
        self.current_fixture
            .clone()
            .ok_or_else(|| anyhow!("No fixture set. Call with_fixture() first"))
    }

    fn template_name_to_filename(&self, template_name: &str) -> String {
        // Remove .tera extension if present, add appropriate extension
        let base = template_name.trim_end_matches(".tera");
        if base.ends_with(".rs") {
            format!("{}", base)
        } else if base.ends_with(".yaml") || base.ends_with(".json") {
            format!("{}", base)
        } else {
            format!("{}.txt", base)
        }
    }

    fn compute_diff(&self, expected: &str, actual: &str) -> String {
        // Simple line-by-line diff
        let expected_lines: Vec<&str> = expected.lines().collect();
        let actual_lines: Vec<&str> = actual.lines().collect();

        let mut diff = String::new();
        let max_lines = expected_lines.len().max(actual_lines.len());

        for i in 0..max_lines.min(10) {
            // Show first 10 diffs
            let exp = expected_lines.get(i).unwrap_or(&"");
            let act = actual_lines.get(i).unwrap_or(&"");

            if exp != act {
                diff.push_str(&format!("Line {}:\n", i + 1));
                diff.push_str(&format!("  Expected: {}\n", exp));
                diff.push_str(&format!("  Actual:   {}\n", act));
            }
        }

        if diff.is_empty() {
            diff.push_str("Content length differs but lines match in sample");
        }

        diff
    }
}

impl Default for OntologyGenerationHarness {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Result Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    pub fixture: String,
    pub query_results: HashMap<String, QueryResult>,
    pub rendered_outputs: HashMap<String, RenderedOutput>,
    pub written_files: Vec<PathBuf>,
    pub validation_report: ValidationReport,
    pub metrics: WorkflowMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub query_name: String,
    pub bindings: Vec<HashMap<String, String>>,
    pub execution_time: Duration,
    pub from_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedOutput {
    pub template_name: String,
    pub content: String,
    pub render_time: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub valid: bool,
    pub checks: Vec<ValidationCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCheck {
    pub check_name: String,
    pub passed: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldenFileComparison {
    pub enabled: bool,
    pub comparisons: Vec<FileComparison>,
    pub all_match: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileComparison {
    pub template_name: String,
    pub golden_path: PathBuf,
    pub matches: bool,
    pub difference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheTestResult {
    pub first_execution_times: HashMap<String, Duration>,
    pub cache_hits: usize,
    pub total_queries: usize,
    pub all_cached: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowMetrics {
    pub ontology_load_time: Duration,
    pub query_execution_time: Duration,
    pub template_render_time: Duration,
    pub total_workflow_time: Duration,
}
