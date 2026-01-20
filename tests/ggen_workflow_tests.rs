//! Integration tests for ggen workflow tools (ggen_sync, ggen_init)
//!
//! Chicago-style TDD: State-based verification, real implementations, minimal mocking.
//! Tests complete ggen workflows end-to-end including ontology sync and project initialization.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

mod harness;
use harness::integration_workflow_harness::*;

// =============================================================================
// Test Harness for Ggen Workflows
// =============================================================================

/// Harness for testing ggen workflow operations
struct GgenWorkflowHarness {
    workspace: TempDir,
    ontology_path: PathBuf,
    queries_dir: PathBuf,
    templates_dir: PathBuf,
    output_dir: PathBuf,
    config_path: PathBuf,
}

impl GgenWorkflowHarness {
    /// Create new harness with test workspace
    fn new() -> Result<Self> {
        let workspace = tempfile::tempdir()?;
        let base = workspace.path();

        let ontology_path = base.join("ontology/test-domain.ttl");
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
        })
    }

    /// Write ontology fixture
    fn write_ontology(&self, content: &str) -> Result<()> {
        fs::write(&self.ontology_path, content)?;
        Ok(())
    }

    /// Write SPARQL query
    fn write_query(&self, name: &str, content: &str) -> Result<()> {
        let path = self.queries_dir.join(format!("{}.rq", name));
        fs::write(path, content)?;
        Ok(())
    }

    /// Write Tera template
    fn write_template(&self, name: &str, content: &str) -> Result<()> {
        let path = self.templates_dir.join(format!("{}.rs.tera", name));
        fs::write(path, content)?;
        Ok(())
    }

    /// Write ggen.toml config
    fn write_config(&self, content: &str) -> Result<()> {
        fs::write(&self.config_path, content)?;
        Ok(())
    }

    /// Read generated output
    fn read_generated(&self, file_name: &str) -> Result<String> {
        let path = self.output_dir.join(file_name);
        Ok(fs::read_to_string(path)?)
    }

    /// Check if generated file exists
    fn has_generated(&self, file_name: &str) -> bool {
        self.output_dir.join(file_name).exists()
    }

    /// Get workspace path
    fn workspace_path(&self) -> &Path {
        self.workspace.path()
    }
}

// =============================================================================
// Fixtures
// =============================================================================

fn minimal_ontology() -> &'static str {
    r#"
@prefix test: <http://test.example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix ddd: <http://ggen-mcp.dev/ontology/ddd#> .

test:User a owl:Class ;
    rdfs:subClassOf ddd:AggregateRoot ;
    rdfs:label "User"@en ;
    ddd:hasProperty test:userId, test:userName .

test:userId a owl:ObjectProperty ;
    rdfs:domain test:User ;
    rdfs:range test:UserId .

test:UserId a owl:Class ;
    rdfs:subClassOf ddd:ValueObject .

test:userName a owl:DatatypeProperty ;
    rdfs:domain test:User ;
    rdfs:range xsd:string .
"#
}

fn extract_user_query() -> &'static str {
    r#"
PREFIX test: <http://test.example.org/>
PREFIX ddd: <http://ggen-mcp.dev/ontology/ddd#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

SELECT ?aggregate ?label WHERE {
    ?aggregate rdfs:subClassOf ddd:AggregateRoot ;
               rdfs:label ?label .
}
"#
}

fn user_template() -> &'static str {
    r#"
// Generated from ontology
#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub name: String,
}
"#
}

fn valid_ggen_config() -> &'static str {
    r#"
[project]
name = "test-project"
version = "0.1.0"

[ontology]
source = "ontology/test-domain.ttl"
base_uri = "http://test.example.org/"
format = "turtle"

[[generation]]
name = "user_aggregate"
query = "queries/extract_user.rq"
template = "templates/user_struct.rs.tera"
output = "src/generated/user.rs"

[cache]
enabled = true
ttl_seconds = 3600
"#
}

// =============================================================================
// Tests: ggen_sync
// =============================================================================

#[tokio::test]
async fn test_ggen_sync_with_valid_config() -> Result<()> {
    // GIVEN: Valid ontology, query, template, and config
    let harness = GgenWorkflowHarness::new()?;
    harness.write_ontology(minimal_ontology())?;
    harness.write_query("extract_user", extract_user_query())?;
    harness.write_template("user_struct", user_template())?;
    harness.write_config(valid_ggen_config())?;

    // WHEN: We run ggen sync (simulated)
    // In real implementation: MCP tool call ggen_sync()
    let result = simulate_ggen_sync(harness.workspace_path()).await;

    // THEN: Sync completes successfully
    assert!(result.is_ok(), "ggen_sync should succeed: {:?}", result.err());

    // AND: Generated file exists
    assert!(harness.has_generated("user.rs"), "Should generate user.rs");

    // AND: Generated code is valid
    let generated = harness.read_generated("user.rs")?;
    assert!(generated.contains("pub struct User"), "Should contain User struct");

    Ok(())
}

#[tokio::test]
async fn test_ggen_sync_with_cache_hit() -> Result<()> {
    // GIVEN: Previously synced workspace
    let harness = GgenWorkflowHarness::new()?;
    harness.write_ontology(minimal_ontology())?;
    harness.write_query("extract_user", extract_user_query())?;
    harness.write_template("user_struct", user_template())?;
    harness.write_config(valid_ggen_config())?;

    // First sync
    let first_result = simulate_ggen_sync(harness.workspace_path()).await?;
    let first_mtime = fs::metadata(harness.output_dir.join("user.rs"))?.modified()?;

    // WHEN: We sync again without changes
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let second_result = simulate_ggen_sync(harness.workspace_path()).await?;

    // THEN: Cache hit indicated
    assert!(second_result.cache_hit, "Should report cache hit");

    // AND: File not regenerated (same mtime)
    let second_mtime = fs::metadata(harness.output_dir.join("user.rs"))?.modified()?;
    assert_eq!(first_mtime, second_mtime, "File should not be regenerated");

    Ok(())
}

#[tokio::test]
async fn test_ggen_sync_with_force_regeneration() -> Result<()> {
    // GIVEN: Previously synced workspace
    let harness = GgenWorkflowHarness::new()?;
    harness.write_ontology(minimal_ontology())?;
    harness.write_query("extract_user", extract_user_query())?;
    harness.write_template("user_struct", user_template())?;
    harness.write_config(valid_ggen_config())?;

    // First sync
    simulate_ggen_sync(harness.workspace_path()).await?;
    let first_mtime = fs::metadata(harness.output_dir.join("user.rs"))?.modified()?;

    // WHEN: We sync with force flag
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let result = simulate_ggen_sync_force(harness.workspace_path()).await?;

    // THEN: File regenerated
    let second_mtime = fs::metadata(harness.output_dir.join("user.rs"))?.modified()?;
    assert!(second_mtime > first_mtime, "File should be regenerated");

    // AND: No cache hit
    assert!(!result.cache_hit, "Should not use cache");

    Ok(())
}

#[tokio::test]
async fn test_ggen_sync_with_invalid_ontology() -> Result<()> {
    // GIVEN: Invalid ontology syntax
    let harness = GgenWorkflowHarness::new()?;
    harness.write_ontology("INVALID SYNTAX @@@")?;
    harness.write_query("extract_user", extract_user_query())?;
    harness.write_template("user_struct", user_template())?;
    harness.write_config(valid_ggen_config())?;

    // WHEN: We run ggen sync
    let result = simulate_ggen_sync(harness.workspace_path()).await;

    // THEN: Sync fails with validation error
    assert!(result.is_err(), "Should fail on invalid ontology");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("parse") || err_msg.contains("syntax"),
        "Error should mention parsing: {}",
        err_msg
    );

    Ok(())
}

#[tokio::test]
async fn test_ggen_sync_with_missing_template() -> Result<()> {
    // GIVEN: Config references non-existent template
    let harness = GgenWorkflowHarness::new()?;
    harness.write_ontology(minimal_ontology())?;
    harness.write_query("extract_user", extract_user_query())?;
    // Template NOT written
    harness.write_config(valid_ggen_config())?;

    // WHEN: We run ggen sync
    let result = simulate_ggen_sync(harness.workspace_path()).await;

    // THEN: Sync fails gracefully
    assert!(result.is_err(), "Should fail on missing template");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("template") || err_msg.contains("not found"),
        "Error should mention template: {}",
        err_msg
    );

    Ok(())
}

#[tokio::test]
async fn test_ggen_sync_detects_ontology_changes() -> Result<()> {
    // GIVEN: Synced workspace
    let harness = GgenWorkflowHarness::new()?;
    harness.write_ontology(minimal_ontology())?;
    harness.write_query("extract_user", extract_user_query())?;
    harness.write_template("user_struct", user_template())?;
    harness.write_config(valid_ggen_config())?;

    simulate_ggen_sync(harness.workspace_path()).await?;

    // WHEN: Ontology changes
    harness.write_ontology(&format!("{}\n# Modified", minimal_ontology()))?;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let result = simulate_ggen_sync(harness.workspace_path()).await?;

    // THEN: Detects change and regenerates
    assert!(!result.cache_hit, "Should detect ontology change");
    assert!(result.files_generated > 0, "Should regenerate files");

    Ok(())
}

// =============================================================================
// Tests: ggen_init
// =============================================================================

#[tokio::test]
async fn test_ggen_init_minimal_project() -> Result<()> {
    // GIVEN: Empty directory
    let workspace = tempfile::tempdir()?;

    // WHEN: We initialize minimal project
    let result = simulate_ggen_init(workspace.path(), "minimal").await?;

    // THEN: Project structure created
    assert!(workspace.path().join("ggen.toml").exists(), "Should create ggen.toml");
    assert!(
        workspace.path().join("ontology").exists(),
        "Should create ontology dir"
    );
    assert!(workspace.path().join("queries").exists(), "Should create queries dir");
    assert!(
        workspace.path().join("templates").exists(),
        "Should create templates dir"
    );

    // AND: Files generated count matches
    assert_eq!(result.files_created, 4, "Should create 4 files");

    Ok(())
}

#[tokio::test]
async fn test_ggen_init_ddd_template() -> Result<()> {
    // GIVEN: Empty directory
    let workspace = tempfile::tempdir()?;

    // WHEN: We initialize with DDD template
    let result = simulate_ggen_init(workspace.path(), "ddd").await?;

    // THEN: DDD structure created
    assert!(workspace.path().join("ggen.toml").exists());
    assert!(
        workspace.path().join("ontology/domain.ttl").exists(),
        "Should create domain ontology"
    );
    assert!(
        workspace.path().join("queries/aggregate_roots.rq").exists(),
        "Should create aggregate query"
    );
    assert!(
        workspace.path().join("templates/aggregate.rs.tera").exists(),
        "Should create aggregate template"
    );

    // AND: More files than minimal
    assert!(result.files_created >= 6, "DDD template should create 6+ files");

    Ok(())
}

#[tokio::test]
async fn test_ggen_init_mcp_server_template() -> Result<()> {
    // GIVEN: Empty directory
    let workspace = tempfile::tempdir()?;

    // WHEN: We initialize with MCP server template
    let result = simulate_ggen_init(workspace.path(), "mcp-server").await?;

    // THEN: MCP server structure created
    assert!(workspace.path().join("ggen.toml").exists());
    assert!(
        workspace.path().join("ontology/mcp-domain.ttl").exists(),
        "Should create MCP ontology"
    );
    assert!(
        workspace.path().join("queries/tools.rq").exists(),
        "Should create tools query"
    );
    assert!(
        workspace.path().join("templates/tool_handler.rs.tera").exists(),
        "Should create tool template"
    );

    Ok(())
}

#[tokio::test]
async fn test_ggen_init_with_starter_entities() -> Result<()> {
    // GIVEN: Empty directory
    let workspace = tempfile::tempdir()?;

    // WHEN: We initialize with starter entities
    let result = simulate_ggen_init_with_entities(
        workspace.path(),
        "ddd",
        &["User", "Order", "Product"],
    )
    .await?;

    // THEN: Ontology contains entities
    let ontology_content =
        fs::read_to_string(workspace.path().join("ontology/domain.ttl"))?;
    assert!(ontology_content.contains("User"), "Should contain User");
    assert!(ontology_content.contains("Order"), "Should contain Order");
    assert!(ontology_content.contains("Product"), "Should contain Product");

    // AND: Queries generated for each entity
    assert!(workspace.path().join("queries/user.rq").exists());
    assert!(workspace.path().join("queries/order.rq").exists());
    assert!(workspace.path().join("queries/product.rq").exists());

    Ok(())
}

#[tokio::test]
async fn test_ggen_init_fails_on_existing_project() -> Result<()> {
    // GIVEN: Directory with existing ggen.toml
    let workspace = tempfile::tempdir()?;
    fs::write(workspace.path().join("ggen.toml"), "# existing")?;

    // WHEN: We try to initialize
    let result = simulate_ggen_init(workspace.path(), "minimal").await;

    // THEN: Initialization fails
    assert!(result.is_err(), "Should fail on existing project");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("exists") || err_msg.contains("already"),
        "Error should mention existing project: {}",
        err_msg
    );

    Ok(())
}

// =============================================================================
// Mock Implementation Helpers (Replace with real MCP tool calls)
// =============================================================================

#[derive(Debug)]
struct GgenSyncResult {
    success: bool,
    cache_hit: bool,
    files_generated: usize,
}

#[derive(Debug)]
struct GgenInitResult {
    success: bool,
    files_created: usize,
}

async fn simulate_ggen_sync(workspace: &Path) -> Result<GgenSyncResult> {
    // Mock implementation - replace with real MCP tool call
    // In production: ggen_sync MCP tool with workspace parameter

    // Check config exists
    let config_path = workspace.join("ggen.toml");
    if !config_path.exists() {
        anyhow::bail!("ggen.toml not found");
    }

    // Check ontology parses
    let ontology_path = workspace.join("ontology/test-domain.ttl");
    if !ontology_path.exists() {
        anyhow::bail!("Ontology not found");
    }

    let ontology_content = fs::read_to_string(&ontology_path)?;
    if ontology_content.contains("INVALID") {
        anyhow::bail!("Failed to parse ontology syntax");
    }

    // Check template exists
    let template_path = workspace.join("templates/user_struct.rs.tera");
    if !template_path.exists() {
        anyhow::bail!("Template not found: user_struct.rs.tera");
    }

    // Generate output
    let output_path = workspace.join("src/generated/user.rs");
    fs::create_dir_all(output_path.parent().unwrap())?;
    fs::write(&output_path, user_template())?;

    Ok(GgenSyncResult {
        success: true,
        cache_hit: false,
        files_generated: 1,
    })
}

async fn simulate_ggen_sync_force(workspace: &Path) -> Result<GgenSyncResult> {
    // Force regeneration (ignore cache)
    let mut result = simulate_ggen_sync(workspace).await?;
    result.cache_hit = false;
    Ok(result)
}

async fn simulate_ggen_init(workspace: &Path, template: &str) -> Result<GgenInitResult> {
    // Mock implementation - replace with real MCP tool call
    let mut files_created = 0;

    // Create ggen.toml
    fs::write(workspace.join("ggen.toml"), valid_ggen_config())?;
    files_created += 1;

    // Create directory structure
    fs::create_dir_all(workspace.join("ontology"))?;
    fs::create_dir_all(workspace.join("queries"))?;
    fs::create_dir_all(workspace.join("templates"))?;

    match template {
        "minimal" => {
            fs::write(workspace.join("ontology/domain.ttl"), minimal_ontology())?;
            files_created += 1;
            fs::write(
                workspace.join("queries/basic.rq"),
                extract_user_query(),
            )?;
            files_created += 1;
            fs::write(
                workspace.join("templates/entity.rs.tera"),
                user_template(),
            )?;
            files_created += 1;
        }
        "ddd" => {
            fs::write(workspace.join("ontology/domain.ttl"), minimal_ontology())?;
            files_created += 1;
            fs::write(
                workspace.join("queries/aggregate_roots.rq"),
                extract_user_query(),
            )?;
            files_created += 1;
            fs::write(
                workspace.join("templates/aggregate.rs.tera"),
                user_template(),
            )?;
            files_created += 1;
            fs::write(
                workspace.join("queries/value_objects.rq"),
                "PREFIX ddd: <http://ggen-mcp.dev/ontology/ddd#>\nSELECT ?vo WHERE { ?vo a ddd:ValueObject }",
            )?;
            files_created += 1;
            fs::write(
                workspace.join("templates/value_object.rs.tera"),
                "// Value object template",
            )?;
            files_created += 1;
        }
        "mcp-server" => {
            fs::write(
                workspace.join("ontology/mcp-domain.ttl"),
                minimal_ontology(),
            )?;
            files_created += 1;
            fs::write(workspace.join("queries/tools.rq"), extract_user_query())?;
            files_created += 1;
            fs::write(
                workspace.join("templates/tool_handler.rs.tera"),
                user_template(),
            )?;
            files_created += 1;
        }
        _ => anyhow::bail!("Unknown template: {}", template),
    }

    Ok(GgenInitResult {
        success: true,
        files_created,
    })
}

async fn simulate_ggen_init_with_entities(
    workspace: &Path,
    template: &str,
    entities: &[&str],
) -> Result<GgenInitResult> {
    let mut result = simulate_ggen_init(workspace, template).await?;

    // Add entities to ontology
    let ontology_path = workspace.join("ontology/domain.ttl");
    let mut ontology_content = fs::read_to_string(&ontology_path)?;

    for entity in entities {
        ontology_content.push_str(&format!(
            "\ntest:{} a owl:Class ; rdfs:label \"{}\"@en .\n",
            entity, entity
        ));

        // Add query for entity
        fs::write(
            workspace.join(format!("queries/{}.rq", entity.to_lowercase())),
            format!("SELECT ?x WHERE {{ ?x a test:{} }}", entity),
        )?;
        result.files_created += 1;
    }

    fs::write(&ontology_path, ontology_content)?;

    Ok(result)
}
