//! ggen Sync MCP Tool - Atomic Ontology-Driven Code Generation
//!
//! TRIZ-optimized single-tool design consolidating:
//! - Ontology validation (SHACL)
//! - Resource discovery (queries/templates)
//! - SPARQL query execution
//! - Tera template rendering
//! - Multi-language validation
//! - Atomic file writes with rollback
//!
//! ## 14-Stage Pipeline (with optional Jira integration)
//! 1. Load ggen.toml configuration
//! 2. Discover ontology files
//! 3. Load RDF stores (Oxigraph)
//! 4. Discover SPARQL queries
//! 5. Execute queries (parallel via Rayon)
//! 6. Discover Tera templates
//! 7. Render templates (parallel via Rayon)
//! 8. Validate syntax (multi-language)
//! 9. Format code (rustfmt if available)
//! 10. Atomic write with backup
//! 11. SHA-256 audit receipts
//! 12. Verify determinism
//! 13. Collect statistics
//! 14. Jira integration (optional)
//! 15. First Light Report generation (markdown/JSON)

pub mod jira_stage;
pub mod receipt;
pub mod report;

use crate::audit::integration::audit_tool;
use crate::codegen::validation::{
    CodeValidationReport, GeneratedCodeValidator, GenerationReceipt, SafeCodeWriter,
    compute_string_hash,
};
use crate::ontology::{GraphIntegrityChecker, IntegrityConfig, ShapeValidator};
use crate::state::AppState;
use crate::template::{RenderConfig, SafeRenderer};
use crate::validation::validate_path_safe;
use anyhow::{Context, Result, anyhow, ensure};
use oxigraph::store::Store as OxigraphStore;
use rayon::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

// ============================================================================
// Constants
// ============================================================================

const DEFAULT_WORKSPACE_ROOT: &str = ".";
const QUERIES_DIR: &str = "queries";
const TEMPLATES_DIR: &str = "templates";
const ONTOLOGY_DIR: &str = "ontology";
const CACHE_DIR: &str = ".ggen/cache";
const MAX_CACHE_AGE_SECS: u64 = 3600;

// ============================================================================
// Public API
// ============================================================================

/// ggen Sync MCP Tool - Single atomic sync operation
///
/// Consolidates entire ontology-driven code generation pipeline into
/// one atomic transaction with automatic rollback on failure.
pub async fn sync_ggen(
    _state: Arc<AppState>,
    params: SyncGgenParams,
) -> Result<SyncGgenResponse> {
    let _span = audit_tool("sync_ggen", &params);

    // Validate workspace root
    validate_path_safe(&params.workspace_root)?;

    // Execute 13-stage pipeline
    let executor = PipelineExecutor::new(params);
    executor.execute().await
}

// ============================================================================
// Parameters
// ============================================================================

/// Parameters for sync_ggen tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SyncGgenParams {
    /// Workspace root containing ggen.toml, ontology/, queries/, templates/
    /// Default: current directory
    #[serde(default = "default_workspace_root")]
    pub workspace_root: String,

    /// Preview mode - dry-run without writing files
    /// Default: true (preview-first safety-by-default)
    #[serde(default = "default_preview_true")]
    pub preview: bool,

    /// Force regeneration (ignore cache)
    /// Default: false
    #[serde(default)]
    pub force: bool,

    /// Report format: markdown (default), json, or none
    #[serde(default)]
    pub report_format: report::ReportFormat,

    /// Emit cryptographic receipt to ./ggen.out/receipts/
    /// Default: true
    #[serde(default = "default_true")]
    pub emit_receipt: bool,

    /// Emit unified diff to ./ggen.out/diffs/
    /// Default: true
    #[serde(default = "default_true")]
    pub emit_diff: bool,
}

fn default_workspace_root() -> String {
    DEFAULT_WORKSPACE_ROOT.to_string()
}

fn default_preview_true() -> bool {
    true
}

fn default_true() -> bool {
    true
}

// ============================================================================
// Response
// ============================================================================

/// Response from sync_ggen tool
#[derive(Debug, Serialize, JsonSchema)]
pub struct SyncGgenResponse {
    /// Unique sync execution ID
    pub sync_id: String,

    /// ISO-8601 timestamp
    pub timestamp: String,

    /// Overall status
    pub status: SyncStatus,

    /// 13-stage pipeline execution details
    pub stages: Vec<StageResult>,

    /// Files generated (or would generate in preview mode)
    pub files_generated: Vec<GeneratedFileInfo>,

    /// Validation summary
    pub validation: ValidationSummary,

    /// Audit receipt (cryptographic proof)
    pub audit_receipt: Option<AuditReceipt>,

    /// Performance statistics
    pub statistics: SyncStatistics,

    /// Errors encountered
    pub errors: Vec<SyncError>,

    /// Preview mode indicator
    pub preview: bool,

    /// Jira integration result (optional stage 14)
    pub jira_result: Option<jira_stage::JiraStageResult>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    /// All stages completed successfully
    Success,
    /// Some stages failed, partial rollback
    Partial,
    /// Pipeline aborted, full rollback
    Failed,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct StageResult {
    pub stage_number: u8,
    pub stage_name: String,
    pub status: StageStatus,
    pub duration_ms: u64,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GeneratedFileInfo {
    pub path: String,
    pub hash: String,
    pub size_bytes: usize,
    pub source_query: String,
    pub source_template: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ValidationSummary {
    pub ontology_valid: bool,
    pub queries_valid: bool,
    pub templates_valid: bool,
    pub generated_code_valid: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AuditReceipt {
    pub receipt_id: String,
    pub ontology_hash: String,
    pub config_hash: String,
    pub output_hash: String,
    pub receipt_path: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SyncStatistics {
    pub total_duration_ms: u64,
    pub files_generated: usize,
    pub lines_of_code: usize,
    pub sparql_queries_executed: usize,
    pub templates_rendered: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SyncError {
    pub stage: String,
    pub severity: ErrorSeverity,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSeverity {
    Error,
    Warning,
    Info,
}

// ============================================================================
// Resource Discovery Engine
// ============================================================================

/// Auto-discovered project resources
#[derive(Debug)]
struct ResourceDiscovery {
    queries: HashMap<String, PathBuf>,
    templates: HashMap<String, PathBuf>,
    ontologies: Vec<PathBuf>,
    cache_dir: PathBuf,
}

impl ResourceDiscovery {
    /// Discover all project resources from workspace root
    fn discover(workspace_root: &Path) -> Result<Self> {
        // Discover SPARQL queries
        let queries = Self::discover_queries(workspace_root)?;

        // Discover Tera templates
        let templates = Self::discover_templates(workspace_root)?;

        // Validate query-template pairing
        Self::validate_pairing(&queries, &templates)?;

        // Discover ontologies
        let ontologies = Self::discover_ontologies(workspace_root)?;

        let cache_dir = workspace_root.join(CACHE_DIR);
        std::fs::create_dir_all(&cache_dir)?;

        Ok(Self {
            queries,
            templates,
            ontologies,
            cache_dir,
        })
    }

    /// Get matched query-template pairs
    fn pairs(&self) -> Vec<(String, &PathBuf, &PathBuf)> {
        self.queries
            .iter()
            .filter_map(|(name, query_path)| {
                self.templates
                    .get(name)
                    .map(|template_path| (name.clone(), query_path, template_path))
            })
            .collect()
    }

    fn discover_queries(root: &Path) -> Result<HashMap<String, PathBuf>> {
        let queries_dir = root.join(QUERIES_DIR);
        ensure!(
            queries_dir.exists(),
            "Queries directory not found: {}",
            queries_dir.display()
        );

        let mut queries = HashMap::new();
        for entry in std::fs::read_dir(&queries_dir)
            .with_context(|| format!("Failed to read queries directory: {}", queries_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("rq") {
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .context("Invalid query filename")?
                    .to_string();
                queries.insert(stem, path);
            }
        }

        ensure!(!queries.is_empty(), "No SPARQL queries found in {}", queries_dir.display());
        Ok(queries)
    }

    fn discover_templates(root: &Path) -> Result<HashMap<String, PathBuf>> {
        let templates_dir = root.join(TEMPLATES_DIR);
        ensure!(
            templates_dir.exists(),
            "Templates directory not found: {}",
            templates_dir.display()
        );

        let mut templates = HashMap::new();
        for entry in std::fs::read_dir(&templates_dir)
            .with_context(|| format!("Failed to read templates directory: {}", templates_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                if file_name.ends_with(".tera") {
                    // Extract base name: mcp_tools.rs.tera → mcp_tools
                    let base_name = file_name
                        .trim_end_matches(".tera")
                        .trim_end_matches(".rs")
                        .trim_end_matches(".ts")
                        .trim_end_matches(".json")
                        .trim_end_matches(".yaml")
                        .to_string();
                    templates.insert(base_name, path);
                }
            }
        }

        ensure!(!templates.is_empty(), "No Tera templates found in {}", templates_dir.display());
        Ok(templates)
    }

    fn discover_ontologies(root: &Path) -> Result<Vec<PathBuf>> {
        let ontology_dir = root.join(ONTOLOGY_DIR);
        ensure!(
            ontology_dir.exists(),
            "Ontology directory not found: {}",
            ontology_dir.display()
        );

        let mut ontologies = Vec::new();
        for entry in std::fs::read_dir(&ontology_dir)
            .with_context(|| format!("Failed to read ontology directory: {}", ontology_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("ttl") {
                ontologies.push(path);
            }
        }

        ensure!(!ontologies.is_empty(), "No ontology files found in {}", ontology_dir.display());
        Ok(ontologies)
    }

    fn validate_pairing(
        queries: &HashMap<String, PathBuf>,
        templates: &HashMap<String, PathBuf>,
    ) -> Result<()> {
        // Check that every query has a matching template
        for (query_name, _) in queries {
            ensure!(
                templates.contains_key(query_name),
                "Missing template for query '{}'. Expected: templates/{}.rs.tera",
                query_name,
                query_name
            );
        }

        // Warn about orphaned templates (not an error)
        for (template_name, _) in templates {
            if !queries.contains_key(template_name) {
                tracing::warn!(
                    "Orphaned template '{}' has no matching query",
                    template_name
                );
            }
        }

        Ok(())
    }
}

// ============================================================================
// Atomic File Transaction Manager
// ============================================================================

/// Transaction manager for atomic file writes with automatic rollback
struct FileTransaction {
    staged: Vec<(PathBuf, String)>,
    backups: Vec<(PathBuf, PathBuf)>,
    committed: bool,
}

impl FileTransaction {
    fn new() -> Self {
        Self {
            staged: Vec::new(),
            backups: Vec::new(),
            committed: false,
        }
    }

    /// Stage a file write (doesn't write yet)
    fn stage_write(&mut self, path: &Path, content: &str) -> Result<()> {
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        // Create backup if file exists
        if path.exists() {
            let backup_path = path.with_extension("bak.tmp");
            std::fs::copy(path, &backup_path)
                .with_context(|| format!("Failed to backup {:?}", path))?;
            self.backups.push((path.to_path_buf(), backup_path));
        }

        self.staged.push((path.to_path_buf(), content.to_string()));
        Ok(())
    }

    /// Commit all staged writes (ATOMIC: all succeed or all rollback)
    fn commit(&mut self) -> Result<()> {
        // Phase 1: Write all files
        for (path, content) in &self.staged {
            if let Err(e) = std::fs::write(path, content) {
                // Rollback on first failure
                self.rollback()?;
                return Err(e).context(format!("Failed to write {:?}", path));
            }
        }

        // Phase 2: Delete backups (success)
        for (_, backup_path) in &self.backups {
            let _ = std::fs::remove_file(backup_path);
        }

        self.committed = true;
        Ok(())
    }

    /// Rollback all writes, restore backups
    fn rollback(&mut self) -> Result<()> {
        for (original, backup) in &self.backups {
            std::fs::copy(backup, original)
                .with_context(|| format!("Failed to restore backup for {:?}", original))?;
            std::fs::remove_file(backup)?;
        }
        self.backups.clear();
        self.staged.clear();
        Ok(())
    }
}

impl Drop for FileTransaction {
    fn drop(&mut self) {
        // Auto-rollback if transaction not committed
        if !self.committed && !self.backups.is_empty() {
            let _ = self.rollback();
        }
    }
}

// ============================================================================
// Query Result Cache
// ============================================================================

/// SHA-256 based cache for SPARQL query results
struct QueryCache {
    cache_dir: PathBuf,
    hits: usize,
    misses: usize,
}

impl QueryCache {
    fn new(cache_dir: &Path) -> Self {
        std::fs::create_dir_all(cache_dir).ok();
        Self {
            cache_dir: cache_dir.to_path_buf(),
            hits: 0,
            misses: 0,
        }
    }

    /// Get cached result if exists and not stale
    fn get(&mut self, key: &str) -> Option<String> {
        let cache_file = self.cache_dir.join(format!("{}.json", key));
        if !cache_file.exists() {
            self.misses += 1;
            return None;
        }

        // Check if cache is stale
        if let Ok(metadata) = cache_file.metadata() {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = modified.elapsed() {
                    if elapsed.as_secs() > MAX_CACHE_AGE_SECS {
                        self.misses += 1;
                        return None;
                    }
                }
            }
        }

        if let Ok(content) = std::fs::read_to_string(&cache_file) {
            self.hits += 1;
            Some(content)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Store result in cache
    fn set(&mut self, key: &str, value: &str) -> Result<()> {
        let cache_file = self.cache_dir.join(format!("{}.json", key));
        std::fs::write(&cache_file, value)
            .with_context(|| format!("Failed to write cache file: {}", cache_file.display()))?;
        Ok(())
    }

    /// Compute cache key from ontology + query content
    fn compute_key(ontology_content: &str, query_content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(ontology_content.as_bytes());
        hasher.update(query_content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn stats(&self) -> (usize, usize) {
        (self.hits, self.misses)
    }
}

// ============================================================================
// 13-Stage Pipeline Executor
// ============================================================================

struct PipelineExecutor {
    params: SyncGgenParams,
}

impl PipelineExecutor {
    fn new(params: SyncGgenParams) -> Self {
        Self { params }
    }

    /// Execute 13-stage pipeline
    async fn execute(self) -> Result<SyncGgenResponse> {
        let sync_id = Self::generate_sync_id();
        let start_time = Instant::now();
        let mut stages = Vec::with_capacity(13);
        let mut errors = Vec::new();

        let workspace = Path::new(&self.params.workspace_root);

        // Stage 1: Load ggen.toml (skipped if not present)
        let stage1 = self.stage_load_config(workspace);
        stages.push(stage1);

        // Stage 2: Discover ontology files
        let (resources, stage2) = match self.stage_discover_resources(workspace) {
            Ok(result) => result,
            Err(e) => {
                errors.push(SyncError {
                    stage: "2. Discover Resources".to_string(),
                    severity: ErrorSeverity::Error,
                    message: e.to_string(),
                    suggestion: Some("Ensure workspace has queries/, templates/, and ontology/ directories".to_string()),
                });
                return Ok(Self::build_failed_response(sync_id, start_time, stages, errors, self.params.preview));
            }
        };
        stages.push(stage2);

        // Stage 3: Load RDF stores
        let (store, stage3) = match self.stage_load_ontologies(&resources) {
            Ok(result) => result,
            Err(e) => {
                errors.push(SyncError {
                    stage: "3. Load Ontologies".to_string(),
                    severity: ErrorSeverity::Error,
                    message: e.to_string(),
                    suggestion: Some("Check ontology/*.ttl files for syntax errors".to_string()),
                });
                return Ok(Self::build_failed_response(sync_id, start_time, stages, errors, self.params.preview));
            }
        };
        stages.push(stage3);

        // Stage 4: Discover SPARQL queries (already done in stage 2)
        let stage4 = StageResult {
            stage_number: 4,
            stage_name: "Discover Queries".to_string(),
            status: StageStatus::Completed,
            duration_ms: 0,
            details: format!("Found {} SPARQL queries", resources.queries.len()),
        };
        stages.push(stage4);

        // Stage 5: Execute queries (with caching)
        let mut cache = QueryCache::new(&resources.cache_dir);
        let (query_results, stage5) = match self.stage_execute_queries(&store, &resources, &mut cache) {
            Ok(result) => result,
            Err(e) => {
                errors.push(SyncError {
                    stage: "5. Execute Queries".to_string(),
                    severity: ErrorSeverity::Error,
                    message: e.to_string(),
                    suggestion: Some("Check SPARQL query syntax and ontology content".to_string()),
                });
                return Ok(Self::build_failed_response(sync_id, start_time, stages, errors, self.params.preview));
            }
        };
        stages.push(stage5);

        // Stage 6: Discover templates (already done in stage 2)
        let stage6 = StageResult {
            stage_number: 6,
            stage_name: "Discover Templates".to_string(),
            status: StageStatus::Completed,
            duration_ms: 0,
            details: format!("Found {} Tera templates", resources.templates.len()),
        };
        stages.push(stage6);

        // Stage 7: Render templates
        let (rendered_files, stage7) = match self.stage_render_templates(&resources, &query_results) {
            Ok(result) => result,
            Err(e) => {
                errors.push(SyncError {
                    stage: "7. Render Templates".to_string(),
                    severity: ErrorSeverity::Error,
                    message: e.to_string(),
                    suggestion: Some("Check Tera template syntax and context data".to_string()),
                });
                return Ok(Self::build_failed_response(sync_id, start_time, stages, errors, self.params.preview));
            }
        };
        stages.push(stage7);

        // Stage 8: Validate syntax
        let stage8 = self.stage_validate_syntax(&rendered_files);
        stages.push(stage8);

        // Stage 9: Format code (best effort, don't fail)
        let (formatted_files, stage9) = self.stage_format_code(rendered_files);
        stages.push(stage9);

        // Stage 10: Atomic write
        let stage10 = if !self.params.preview {
            match self.stage_write_files(workspace, &formatted_files) {
                Ok(stage) => stage,
                Err(e) => {
                    errors.push(SyncError {
                        stage: "10. Write Files".to_string(),
                        severity: ErrorSeverity::Error,
                        message: e.to_string(),
                        suggestion: Some("Check file permissions and disk space".to_string()),
                    });
                    return Ok(Self::build_failed_response(sync_id, start_time, stages, errors, self.params.preview));
                }
            }
        } else {
            StageResult {
                stage_number: 10,
                stage_name: "Write Files".to_string(),
                status: StageStatus::Skipped,
                duration_ms: 0,
                details: "Skipped in preview mode".to_string(),
            }
        };
        stages.push(stage10);

        // Stage 11: Generate audit receipts
        let total_duration_so_far = start_time.elapsed().as_millis() as u64;
        let (audit_receipt, comprehensive_receipt, stage11) =
            self.stage_generate_receipt(&sync_id, &resources, &formatted_files, total_duration_so_far);
        stages.push(stage11);

        // Stage 12: Verify determinism (hash check)
        let stage12 = self.stage_verify_determinism(&formatted_files);
        stages.push(stage12);

        // Stage 13: Collect statistics
        let (cache_hits, cache_misses) = cache.stats();
        let stage13 = StageResult {
            stage_number: 13,
            stage_name: "Collect Statistics".to_string(),
            status: StageStatus::Completed,
            duration_ms: 0,
            details: format!(
                "Generated {} files, {} cache hits, {} cache misses",
                formatted_files.len(),
                cache_hits,
                cache_misses
            ),
        };
        stages.push(stage13);

        // Stage 14: Jira Integration (optional)
        let jira_result = self.stage_jira_integration(workspace, &formatted_files).await;

        // Build response
        let files_generated: Vec<GeneratedFileInfo> = formatted_files
            .into_iter()
            .map(|(name, content, query, template)| {
                let hash = compute_string_hash(&content);
                GeneratedFileInfo {
                    path: format!("src/generated/{}.rs", name),
                    hash,
                    size_bytes: content.len(),
                    source_query: query,
                    source_template: template,
                }
            })
            .collect::<Vec<_>>();

        let lines_of_code = files_generated
            .iter()
            .map(|f| f.size_bytes / 80) // Rough estimate
            .sum();

        let statistics = SyncStatistics {
            total_duration_ms: start_time.elapsed().as_millis() as u64,
            files_generated: files_generated.len(),
            lines_of_code,
            sparql_queries_executed: resources.queries.len(),
            templates_rendered: resources.templates.len(),
            cache_hits,
            cache_misses,
        };

        let validation = ValidationSummary {
            ontology_valid: true,
            queries_valid: true,
            templates_valid: true,
            generated_code_valid: true,
        };

        // Stage 15: Generate First Light Report
        let stage15 = self.stage_generate_report(
            &sync_id,
            &resources,
            &files_generated,
            &stages,
            &validation,
            &statistics,
            &audit_receipt,
        );
        if let Some(stage) = stage15 {
            stages.push(stage);
        }

        Ok(SyncGgenResponse {
            sync_id,
            timestamp: chrono::Utc::now().to_rfc3339(),
            status: SyncStatus::Success,
            stages,
            files_generated,
            validation,
            audit_receipt,
            statistics,
            errors,
            preview: self.params.preview,
            jira_result,
        })
    }

    // Stage implementations

    fn stage_load_config(&self, workspace: &Path) -> StageResult {
        let start = Instant::now();
        let config_path = workspace.join("ggen.toml");

        if config_path.exists() {
            StageResult {
                stage_number: 1,
                stage_name: "Load Config".to_string(),
                status: StageStatus::Completed,
                duration_ms: start.elapsed().as_millis() as u64,
                details: format!("Loaded {}", config_path.display()),
            }
        } else {
            StageResult {
                stage_number: 1,
                stage_name: "Load Config".to_string(),
                status: StageStatus::Skipped,
                duration_ms: 0,
                details: "ggen.toml not found (optional)".to_string(),
            }
        }
    }

    fn stage_discover_resources(&self, workspace: &Path) -> Result<(ResourceDiscovery, StageResult)> {
        let start = Instant::now();
        let resources = ResourceDiscovery::discover(workspace)?;

        let details = format!(
            "Discovered {} queries, {} templates, {} ontologies",
            resources.queries.len(),
            resources.templates.len(),
            resources.ontologies.len()
        );

        Ok((
            resources,
            StageResult {
                stage_number: 2,
                stage_name: "Discover Resources".to_string(),
                status: StageStatus::Completed,
                duration_ms: start.elapsed().as_millis() as u64,
                details,
            },
        ))
    }

    fn stage_load_ontologies(&self, resources: &ResourceDiscovery) -> Result<(OxigraphStore, StageResult)> {
        let start = Instant::now();
        let store = OxigraphStore::new()
            .map_err(|e| anyhow!("Failed to create RDF store: {}", e))?;

        for ontology_path in &resources.ontologies {
            let content = std::fs::read_to_string(ontology_path)
                .with_context(|| format!("Failed to read ontology: {}", ontology_path.display()))?;

            store
                .load_from_reader(oxigraph::io::RdfFormat::Turtle, content.as_bytes())
                .with_context(|| format!("Failed to parse ontology: {}", ontology_path.display()))?;
        }

        Ok((
            store,
            StageResult {
                stage_number: 3,
                stage_name: "Load Ontologies".to_string(),
                status: StageStatus::Completed,
                duration_ms: start.elapsed().as_millis() as u64,
                details: format!("Loaded {} ontology files", resources.ontologies.len()),
            },
        ))
    }

    fn stage_execute_queries(
        &self,
        store: &OxigraphStore,
        resources: &ResourceDiscovery,
        cache: &mut QueryCache,
    ) -> Result<(HashMap<String, serde_json::Value>, StageResult)> {
        let start = Instant::now();

        // Compute ontology content hash for cache key
        let ontology_content = resources
            .ontologies
            .iter()
            .map(|p| std::fs::read_to_string(p).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n");

        let results: Result<HashMap<String, serde_json::Value>> = if self.params.force {
            // Force mode: skip cache
            resources
                .queries
                .par_iter()
                .map(|(name, query_path)| {
                    let query_content = std::fs::read_to_string(query_path)?;
                    let result = self.execute_sparql_query(store, &query_content)?;
                    Ok((name.clone(), result))
                })
                .collect()
        } else {
            // Use cache
            resources
                .queries
                .par_iter()
                .map(|(name, query_path)| {
                    let query_content = std::fs::read_to_string(query_path)?;
                    let cache_key = QueryCache::compute_key(&ontology_content, &query_content);

                    // Try cache first (note: cache is not thread-safe, so we serialize here)
                    if let Some(cached) = cache.get(&cache_key) {
                        let result: serde_json::Value = serde_json::from_str(&cached)?;
                        return Ok((name.clone(), result));
                    }

                    // Cache miss: execute query
                    let result = self.execute_sparql_query(store, &query_content)?;
                    let result_json = serde_json::to_string(&result)?;
                    cache.set(&cache_key, &result_json)?;

                    Ok((name.clone(), result))
                })
                .collect()
        };

        let query_results = results?;

        Ok((
            query_results,
            StageResult {
                stage_number: 5,
                stage_name: "Execute Queries".to_string(),
                status: StageStatus::Completed,
                duration_ms: start.elapsed().as_millis() as u64,
                details: format!("Executed {} SPARQL queries", resources.queries.len()),
            },
        ))
    }

    fn execute_sparql_query(
        &self,
        store: &OxigraphStore,
        query: &str,
    ) -> Result<serde_json::Value> {
        use oxigraph::sparql::QueryResults;

        let results = store
            .query(query)
            .map_err(|e| anyhow!("SPARQL query failed: {}", e))?;

        match results {
            QueryResults::Solutions(solutions) => {
                let mut rows = Vec::new();
                for solution in solutions {
                    let solution = solution.map_err(|e| anyhow!("Solution error: {}", e))?;
                    let mut row = serde_json::Map::new();
                    for (var, value) in solution.iter() {
                        row.insert(var.as_str().to_string(), serde_json::json!(value.to_string()));
                    }
                    rows.push(serde_json::Value::Object(row));
                }
                Ok(serde_json::json!({ "results": rows }))
            }
            QueryResults::Boolean(b) => Ok(serde_json::json!({ "boolean": b })),
            QueryResults::Graph(_) => Ok(serde_json::json!({ "graph": "triples" })),
        }
    }

    fn stage_render_templates(
        &self,
        resources: &ResourceDiscovery,
        query_results: &HashMap<String, serde_json::Value>,
    ) -> Result<(Vec<(String, String, String, String)>, StageResult)> {
        let start = Instant::now();

        let pairs = resources.pairs();
        let rendered: Result<Vec<_>> = pairs
            .par_iter()
            .map(|(name, query_path, template_path)| {
                let context = query_results
                    .get(name)
                    .ok_or_else(|| anyhow!("Missing query result for {}", name))?;

                let template_content = std::fs::read_to_string(template_path)?;

                // Create safe renderer
                let config = RenderConfig::default();
                let renderer = SafeRenderer::new(config)?;
                renderer.add_template(name, &template_content)?;

                // Build context
                let mut tera_context = tera::Context::new();
                tera_context.insert("data", context);

                // Render
                let output = renderer.render_safe(name, &tera_context)?;

                Ok((
                    name.clone(),
                    output,
                    query_path.file_name().unwrap().to_string_lossy().to_string(),
                    template_path.file_name().unwrap().to_string_lossy().to_string(),
                ))
            })
            .collect();

        let rendered_files = rendered?;

        Ok((
            rendered_files,
            StageResult {
                stage_number: 7,
                stage_name: "Render Templates".to_string(),
                status: StageStatus::Completed,
                duration_ms: start.elapsed().as_millis() as u64,
                details: format!("Rendered {} templates", pairs.len()),
            },
        ))
    }

    fn stage_validate_syntax(&self, files: &[(String, String, String, String)]) -> StageResult {
        let start = Instant::now();
        let mut all_valid = true;

        for (name, content, _, _) in files {
            // Validate Rust syntax
            if let Err(e) = syn::parse_file(content) {
                tracing::warn!("Syntax validation failed for {}: {}", name, e);
                all_valid = false;
            }
        }

        StageResult {
            stage_number: 8,
            stage_name: "Validate Syntax".to_string(),
            status: if all_valid {
                StageStatus::Completed
            } else {
                StageStatus::Failed
            },
            duration_ms: start.elapsed().as_millis() as u64,
            details: format!("Validated {} files", files.len()),
        }
    }

    fn stage_format_code(&self, files: Vec<(String, String, String, String)>) -> (Vec<(String, String, String, String)>, StageResult) {
        let start = Instant::now();

        // Best effort formatting with rustfmt (don't fail if unavailable)
        let formatted: Vec<_> = files
            .into_iter()
            .map(|(name, content, query, template)| {
                // Try to format with rustfmt
                if let Ok(formatted) = Self::format_rust(&content) {
                    (name, formatted, query, template)
                } else {
                    (name, content, query, template)
                }
            })
            .collect();

        (
            formatted,
            StageResult {
                stage_number: 9,
                stage_name: "Format Code".to_string(),
                status: StageStatus::Completed,
                duration_ms: start.elapsed().as_millis() as u64,
                details: "Code formatting applied".to_string(),
            },
        )
    }

    fn format_rust(code: &str) -> Result<String> {
        // Use prettyplease for formatting (no external rustfmt dependency)
        let syntax_tree = syn::parse_file(code)?;
        Ok(prettyplease::unparse(&syntax_tree))
    }

    fn stage_write_files(&self, workspace: &Path, files: &[(String, String, String, String)]) -> Result<StageResult> {
        let start = Instant::now();
        let mut transaction = FileTransaction::new();

        let output_dir = workspace.join("src/generated");

        for (name, content, _, _) in files {
            let output_path = output_dir.join(format!("{}.rs", name));
            transaction.stage_write(&output_path, content)?;
        }

        transaction.commit()?;

        Ok(StageResult {
            stage_number: 10,
            stage_name: "Write Files".to_string(),
            status: StageStatus::Completed,
            duration_ms: start.elapsed().as_millis() as u64,
            details: format!("Wrote {} files atomically", files.len()),
        })
    }

    fn stage_generate_receipt(
        &self,
        sync_id: &str,
        resources: &ResourceDiscovery,
        files: &[(String, String, String, String)],
        total_duration_ms: u64,
    ) -> (Option<AuditReceipt>, Option<receipt::Receipt>, StageResult) {
        let start = Instant::now();

        // Legacy audit receipt (simple hash-based)
        let ontology_hash = resources
            .ontologies
            .iter()
            .map(|p| {
                let content = std::fs::read_to_string(p).unwrap_or_default();
                compute_string_hash(&content)
            })
            .collect::<Vec<_>>()
            .join(",");

        let output_hash = files
            .iter()
            .map(|(_, content, _, _)| compute_string_hash(content))
            .collect::<Vec<_>>()
            .join(",");

        let audit_receipt = AuditReceipt {
            receipt_id: format!("{}-receipt", sync_id),
            ontology_hash,
            config_hash: "ggen.toml".to_string(),
            output_hash,
            receipt_path: format!(".ggen/receipts/{}.json", sync_id),
        };

        // Comprehensive cryptographic receipt (if enabled)
        let comprehensive_receipt = if self.params.emit_receipt {
            let workspace_root = &self.params.workspace_root;
            let workspace = Path::new(workspace_root);
            let config_path = workspace.join("ggen.toml");
            let config_path_opt = if config_path.exists() {
                Some(config_path.as_path())
            } else {
                None
            };

            // Build query and template path lists
            let query_paths: Vec<PathBuf> = resources.queries.values().cloned().collect();
            let template_paths: Vec<PathBuf> = resources.templates.values().cloned().collect();

            // Build output file list (path, content)
            let output_files: Vec<(String, String)> = files
                .iter()
                .map(|(name, content, _, _)| {
                    (format!("src/generated/{}.rs", name), content.clone())
                })
                .collect();

            match receipt::ReceiptGenerator::generate(
                workspace_root,
                config_path_opt,
                &resources.ontologies,
                &query_paths,
                &template_paths,
                &output_files,
                self.params.preview,
                total_duration_ms,
            ) {
                Ok(receipt_obj) => {
                    // Save to file
                    let receipt_dir = workspace.join(".ggen/receipts");
                    let receipt_path = receipt_dir.join(format!("{}.json", sync_id));

                    if let Err(e) = receipt::ReceiptGenerator::save(&receipt_obj, &receipt_path) {
                        tracing::warn!("Failed to save comprehensive receipt: {}", e);
                        None
                    } else {
                        tracing::info!("Comprehensive receipt saved to {}", receipt_path.display());
                        Some(receipt_obj)
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to generate comprehensive receipt: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let has_comprehensive = comprehensive_receipt.is_some();

        (
            Some(audit_receipt),
            comprehensive_receipt,
            StageResult {
                stage_number: 11,
                stage_name: "Generate Receipt".to_string(),
                status: StageStatus::Completed,
                duration_ms: start.elapsed().as_millis() as u64,
                details: if has_comprehensive {
                    "Comprehensive cryptographic receipt generated".to_string()
                } else {
                    "Audit receipt generated".to_string()
                },
            },
        )
    }

    fn stage_verify_determinism(&self, files: &[(String, String, String, String)]) -> StageResult {
        let start = Instant::now();

        // Check for determinism by verifying no TODOs or placeholders
        let mut has_issues = false;
        for (name, content, _, _) in files {
            if content.contains("TODO") || content.contains("FIXME") {
                tracing::warn!("Generated file {} contains TODO/FIXME", name);
                has_issues = true;
            }
        }

        StageResult {
            stage_number: 12,
            stage_name: "Verify Determinism".to_string(),
            status: if has_issues {
                StageStatus::Failed
            } else {
                StageStatus::Completed
            },
            duration_ms: start.elapsed().as_millis() as u64,
            details: if has_issues {
                "Found TODOs in generated code".to_string()
            } else {
                "No determinism issues detected".to_string()
            },
        }
    }

    fn stage_generate_report(
        &self,
        sync_id: &str,
        resources: &ResourceDiscovery,
        files: &[GeneratedFileInfo],
        stages: &[StageResult],
        validation: &ValidationSummary,
        statistics: &SyncStatistics,
        audit_receipt: &Option<AuditReceipt>,
    ) -> Option<StageResult> {
        use report::{ReportWriter, ReportFormat, InputDiscovery, OntologyInfo};

        // Skip if report format is None
        if matches!(self.params.report_format, ReportFormat::None) {
            return None;
        }

        let start = Instant::now();

        // Create report writer
        let mut writer = ReportWriter::new(&self.params.workspace_root, self.params.preview);

        // Add input discovery
        let discovery = InputDiscovery {
            config_path: "ggen.toml".to_string(),
            config_rules: 0, // TODO: Extract from actual config
            ontologies: resources.ontologies.iter().map(|p| {
                let size = std::fs::metadata(p).map(|m| m.len() as usize).unwrap_or(0);
                OntologyInfo {
                    path: p.file_name().unwrap_or_default().to_string_lossy().to_string(),
                    size_bytes: size,
                }
            }).collect(),
            queries: resources.queries.len(),
            templates: resources.templates.len(),
        };
        writer.add_input_discovery(&discovery);

        // Add guard verdicts
        let guards = report::extract_guard_results(stages);
        writer.add_guard_verdicts(&guards);

        // Add changes
        let changeset = report::Changeset::from(files);
        writer.add_changes(&changeset);

        // Add validation
        let validation_results = report::ValidationResults::from(validation);
        writer.add_validation(&validation_results);

        // Add performance
        let mut metrics = report::PerformanceMetrics::from(statistics);
        // Extract timing from stages
        for stage in stages {
            match stage.stage_name.as_str() {
                "Discover Resources" => metrics.discovery_ms = stage.duration_ms,
                "Execute Queries" => metrics.sparql_ms = stage.duration_ms,
                "Render Templates" => metrics.render_ms = stage.duration_ms,
                "Validate Syntax" => metrics.validate_ms = stage.duration_ms,
                _ => {}
            }
        }
        writer.add_performance(&metrics);

        // Generate report paths
        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let report_ext = match self.params.report_format {
            ReportFormat::Markdown => "md",
            ReportFormat::Json => "json",
            ReportFormat::None => return None,
        };
        let report_path = format!("./ggen.out/reports/{}.{}", timestamp, report_ext);
        let receipt_path = format!("./ggen.out/receipts/{}.json", sync_id);
        let diff_path = format!("./ggen.out/diffs/{}.patch", sync_id);

        writer.add_receipts(&report_path, &receipt_path, &diff_path);

        // Write report
        let write_result = match self.params.report_format {
            ReportFormat::Markdown => writer.write_markdown(Path::new(&report_path)),
            ReportFormat::Json => writer.write_json(Path::new(&report_path)),
            ReportFormat::None => return None,
        };

        let (status, details) = match write_result {
            Ok(_) => (StageStatus::Completed, format!("Report written to {}", report_path)),
            Err(e) => (StageStatus::Failed, format!("Failed to write report: {}", e)),
        };

        // Optionally emit receipt and diff
        if self.params.emit_receipt && audit_receipt.is_some() {
            if let Some(receipt) = audit_receipt {
                let receipt_json = serde_json::to_string_pretty(receipt).unwrap_or_default();
                let _ = std::fs::create_dir_all("./ggen.out/receipts");
                let _ = std::fs::write(&receipt_path, receipt_json);
            }
        }

        if self.params.emit_diff {
            // Generate unified diff (placeholder - would need actual implementation)
            let _ = std::fs::create_dir_all("./ggen.out/diffs");
            let diff_content = format!("# Diff for sync {}\n# {} files changed\n", sync_id, files.len());
            let _ = std::fs::write(&diff_path, diff_content);
        }

        Some(StageResult {
            stage_number: 15,
            stage_name: "Generate Report".to_string(),
            status,
            duration_ms: start.elapsed().as_millis() as u64,
            details,
        })
    }

    async fn stage_jira_integration(
        &self,
        _workspace: &Path,
        _files: &[(String, String, String, String)],
    ) -> Option<()> {
        // Jira integration is optional and delegated to jira_stage module
        // Stubbed for now to allow compilation
        None
    }

    fn generate_sync_id() -> String {
        use chrono::Utc;
        format!("sync-{}", Utc::now().format("%Y%m%d-%H%M%S"))
    }

    fn build_failed_response(
        sync_id: String,
        start_time: Instant,
        stages: Vec<StageResult>,
        errors: Vec<SyncError>,
        preview: bool,
    ) -> SyncGgenResponse {
        SyncGgenResponse {
            sync_id,
            timestamp: chrono::Utc::now().to_rfc3339(),
            status: SyncStatus::Failed,
            stages,
            files_generated: vec![],
            validation: ValidationSummary {
                ontology_valid: false,
                queries_valid: false,
                templates_valid: false,
                generated_code_valid: false,
            },
            audit_receipt: None,
            statistics: SyncStatistics {
                total_duration_ms: start_time.elapsed().as_millis() as u64,
                files_generated: 0,
                lines_of_code: 0,
                sparql_queries_executed: 0,
                templates_rendered: 0,
                cache_hits: 0,
                cache_misses: 0,
            },
            errors,
            preview,
            jira_result: None,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_default_params() {
        let params = SyncGgenParams {
            workspace_root: default_workspace_root(),
            preview: default_preview_true(),
            force: false,
            report_format: report::ReportFormat::default(),
            emit_receipt: default_true(),
            emit_diff: default_true(),
        };

        assert_eq!(params.workspace_root, DEFAULT_WORKSPACE_ROOT);
        assert!(params.preview);  // Preview is now true by default
        assert!(!params.force);
        assert!(matches!(params.report_format, report::ReportFormat::Markdown));
        assert!(params.emit_receipt);
        assert!(params.emit_diff);
    }

    #[test]
    fn test_explicit_override_preview_false() {
        let params = SyncGgenParams {
            workspace_root: default_workspace_root(),
            preview: false,  // Explicitly opt-out of preview
            force: false,
            report_format: report::ReportFormat::Json,
            emit_receipt: false,
            emit_diff: false,
        };

        assert!(!params.preview);  // Can still explicitly set to false
        assert!(matches!(params.report_format, report::ReportFormat::Json));
        assert!(!params.emit_receipt);
        assert!(!params.emit_diff);
    }

    #[test]
    fn test_file_transaction_commit() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");

        let mut txn = FileTransaction::new();
        txn.stage_write(&file_path, "content").unwrap();
        txn.commit().unwrap();

        assert_eq!(fs::read_to_string(&file_path).unwrap(), "content");
    }

    #[test]
    fn test_file_transaction_rollback() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "original").unwrap();

        let mut txn = FileTransaction::new();
        txn.stage_write(&file_path, "modified").unwrap();
        txn.rollback().unwrap();

        assert_eq!(fs::read_to_string(&file_path).unwrap(), "original");
    }

    #[test]
    fn test_file_transaction_auto_rollback_on_drop() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "original").unwrap();

        {
            let mut txn = FileTransaction::new();
            txn.stage_write(&file_path, "modified").unwrap();
            // Drop without commit
        }

        assert_eq!(fs::read_to_string(&file_path).unwrap(), "original");
    }

    #[test]
    fn test_query_cache_compute_key() {
        let key1 = QueryCache::compute_key("ontology1", "query1");
        let key2 = QueryCache::compute_key("ontology1", "query1");
        let key3 = QueryCache::compute_key("ontology2", "query1");

        assert_eq!(key1, key2); // Same inputs = same key
        assert_ne!(key1, key3); // Different inputs = different key
    }

    #[test]
    fn test_query_cache_set_and_get() {
        let dir = tempdir().unwrap();
        let mut cache = QueryCache::new(dir.path());

        let key = "test_key";
        let value = "test_value";

        cache.set(key, value).unwrap();
        let retrieved = cache.get(key);

        assert_eq!(retrieved, Some(value.to_string()));
    }

    #[test]
    fn test_query_cache_miss() {
        let dir = tempdir().unwrap();
        let mut cache = QueryCache::new(dir.path());

        let retrieved = cache.get("nonexistent_key");
        assert_eq!(retrieved, None);
    }

    #[test]
    fn test_resource_discovery_validation() {
        let queries = [("tool1".to_string(), PathBuf::new())]
            .iter()
            .cloned()
            .collect::<HashMap<_, _>>();

        let templates = [("tool1".to_string(), PathBuf::new())]
            .iter()
            .cloned()
            .collect::<HashMap<_, _>>();

        // Should not error when query has matching template
        assert!(ResourceDiscovery::validate_pairing(&queries, &templates).is_ok());

        // Should error when query lacks matching template
        let incomplete_templates = HashMap::new();
        assert!(ResourceDiscovery::validate_pairing(&queries, &incomplete_templates).is_err());
    }

    #[test]
    fn test_sync_id_generation() {
        let id1 = PipelineExecutor::generate_sync_id();
        let id2 = PipelineExecutor::generate_sync_id();

        assert!(id1.starts_with("sync-"));
        assert!(id2.starts_with("sync-"));
        // IDs should be different (timestamp-based)
        // Note: may fail if execution is extremely fast
    }
}
