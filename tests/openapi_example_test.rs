//! End-to-end test for OpenAPI example generation
//!
//! This test validates the complete workflow:
//! 1. Load blog API ontology and schema
//! 2. Execute 13 generation steps from workflow JSON
//! 3. Validate all outputs against expected patterns
//! 4. Compare with golden files (when available)
//!
//! Test Methodology: Chicago TDD - State-based testing with real collaborators
//! Pattern: AAA (Arrange-Act-Assert)

use anyhow::{Context, Result};
use chicago_tdd_tools::prelude::*;
use serde_json::Value;
use spreadsheet_mcp::ontology::{OntologyEngine, OntologyEngineConfig};
use spreadsheet_mcp::template::rendering_safety::{RenderConfig, RenderContext, SafeRenderer};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ============================================================================
// Test Configuration
// ============================================================================

const PROJECT_ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn workspace_path(relative: &str) -> PathBuf {
    PathBuf::from(PROJECT_ROOT).join("workspace").join(relative)
}

fn test_output_path(relative: &str) -> PathBuf {
    PathBuf::from(PROJECT_ROOT)
        .join("target")
        .join("test_output")
        .join("openapi")
        .join(relative)
}

fn golden_path(relative: &str) -> PathBuf {
    PathBuf::from(PROJECT_ROOT)
        .join("tests")
        .join("golden")
        .join("openapi")
        .join(relative)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Read workflow JSON
fn load_workflow() -> Result<Value> {
    let workflow_path = workspace_path("workflows/openapi_generation.json");
    let content = fs::read_to_string(&workflow_path)
        .with_context(|| format!("Failed to read workflow from {:?}", workflow_path))?;
    serde_json::from_str(&content).context("Failed to parse workflow JSON")
}

/// Initialize test output directory
fn init_test_output() -> Result<()> {
    let output_dir = test_output_path("");
    if output_dir.exists() {
        fs::remove_dir_all(&output_dir)?;
    }
    fs::create_dir_all(&output_dir)?;
    fs::create_dir_all(output_dir.join("openapi"))?;
    fs::create_dir_all(output_dir.join("types"))?;
    fs::create_dir_all(output_dir.join("schemas"))?;
    fs::create_dir_all(output_dir.join("guards"))?;
    Ok(())
}

/// Load ontology engine with blog API and schema
fn load_ontology_engine() -> Result<OntologyEngine> {
    let blog_api_path = workspace_path("ontology/blog-api.ttl");
    let api_schema_path = workspace_path("ontology/api-schema.ttl");

    let config = OntologyEngineConfig {
        base_iri: Some("https://ggen.io/examples/blog/".to_string()),
        enable_inference: true,
        enable_validation: true,
    };

    let mut engine = OntologyEngine::new(config)?;

    // Load schema first
    engine
        .load_ontology_file(&api_schema_path)
        .with_context(|| format!("Failed to load schema from {:?}", api_schema_path))?;

    // Load blog API ontology
    engine
        .load_ontology_file(&blog_api_path)
        .with_context(|| format!("Failed to load ontology from {:?}", blog_api_path))?;

    Ok(engine)
}

/// Execute SPARQL query and return results as HashMap
fn execute_query(engine: &OntologyEngine, query: &str) -> Result<Vec<HashMap<String, String>>> {
    let results = engine.execute_sparql_query(query)?;
    Ok(results)
}

/// Render template with context
fn render_template_safe(
    renderer: &SafeRenderer,
    template_name: &str,
    sparql_results: Vec<HashMap<String, String>>,
) -> Result<String> {
    let mut context = RenderContext::new();
    context.insert("sparql_results", &sparql_results)?;
    Ok(renderer.render_safe(template_name, &context)?)
}

// ============================================================================
// Test Cases
// ============================================================================

test!(test_load_ontology_and_schema, {
    // Arrange & Act
    let engine = load_ontology_engine()?;

    // Assert - Verify ontology is loaded
    let query = "PREFIX api: <https://ggen.io/ontology/api#> SELECT (COUNT(?entity) as ?count) WHERE { ?entity a api:Entity }";
    let results = engine.execute_sparql_query(query)?;

    assert!(!results.is_empty(), "Query returned no results");
    let count = results[0].get("count").and_then(|s| s.parse::<i32>().ok());
    assert!(count.is_some(), "Count is not a valid number");
    assert!(
        count.unwrap() >= 4,
        "Expected at least 4 entities (User, Post, Comment, Tag)"
    );

    Ok(())
});

test!(test_workflow_json_structure, {
    // Arrange & Act
    let workflow = load_workflow()?;

    // Assert - Verify workflow structure
    assert!(workflow.get("name").is_some(), "Workflow missing 'name'");
    assert!(workflow.get("steps").is_some(), "Workflow missing 'steps'");

    let steps = workflow["steps"].as_array().expect("steps is not an array");
    assert_eq!(
        steps.len(),
        24,
        "Expected 24 steps (23 generation + 1 validation)"
    );

    // Verify first step is load-ontology
    assert_eq!(
        steps[0]["id"], "load-ontology",
        "First step should be load-ontology"
    );

    Ok(())
});

test!(test_query_api_info, {
    // Arrange
    let engine = load_ontology_engine()?;

    // Act - Query API specification metadata
    let query = r#"
PREFIX api: <https://ggen.io/ontology/api#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

SELECT ?title ?description ?version ?serverUrl ?serverDescription
WHERE {
  ?spec a api:Specification ;
        api:title ?title ;
        api:version ?version .
  OPTIONAL { ?spec api:description ?description }
  OPTIONAL {
    ?spec api:server ?server .
    ?server api:url ?serverUrl .
    OPTIONAL { ?server api:description ?serverDescription }
  }
}
LIMIT 1
"#;

    let results = execute_query(&engine, query)?;

    // Assert - Verify API info extracted
    assert!(!results.is_empty(), "API info query returned no results");
    let info = &results[0];

    assert_eq!(info.get("title").map(|s| s.as_str()), Some("Blog API"));
    assert_eq!(info.get("version").map(|s| s.as_str()), Some("1.0.0"));
    assert!(info.contains_key("description"));
    assert!(info.contains_key("serverUrl"));

    Ok(())
});

test!(test_query_entity_schemas, {
    // Arrange
    let engine = load_ontology_engine()?;

    // Act - Query entity schemas
    let query = r#"
PREFIX api: <https://ggen.io/ontology/api#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

SELECT ?entityName ?propertyName ?propertyType ?required
WHERE {
  ?entity a api:Entity ;
          api:name ?entityName .
  ?entity api:hasProperty ?property .
  ?property api:name ?propertyName ;
            api:type ?propertyType .
  OPTIONAL { ?property api:required ?required }
}
ORDER BY ?entityName ?propertyName
"#;

    let results = execute_query(&engine, query)?;

    // Assert - Verify entities and properties extracted
    assert!(
        !results.is_empty(),
        "Entity schemas query returned no results"
    );

    // Verify User entity exists
    let user_props: Vec<&HashMap<String, String>> = results
        .iter()
        .filter(|r| r.get("entityName").map(|s| s.as_str()) == Some("User"))
        .collect();
    assert!(
        !user_props.is_empty(),
        "User entity not found in query results"
    );

    // Verify User has required properties
    let prop_names: Vec<&str> = user_props
        .iter()
        .filter_map(|r| r.get("propertyName").map(|s| s.as_str()))
        .collect();
    assert!(prop_names.contains(&"id"), "User missing 'id' property");
    assert!(
        prop_names.contains(&"email"),
        "User missing 'email' property"
    );
    assert!(
        prop_names.contains(&"username"),
        "User missing 'username' property"
    );

    Ok(())
});

test!(test_render_openapi_info, {
    // Arrange
    init_test_output()?;
    let engine = load_ontology_engine()?;
    let config = RenderConfig::default();
    let renderer = SafeRenderer::from_directory(workspace_path("templates/openapi"), config)?;

    // Act - Execute query and render template
    let query = r#"
PREFIX api: <https://ggen.io/ontology/api#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

SELECT ?title ?description ?version ?serverUrl ?serverDescription
WHERE {
  ?spec a api:Specification ;
        api:title ?title ;
        api:version ?version .
  OPTIONAL { ?spec api:description ?description }
  OPTIONAL {
    ?spec api:server ?server .
    ?server api:url ?serverUrl .
    OPTIONAL { ?server api:description ?serverDescription }
  }
}
LIMIT 1
"#;

    let sparql_results = execute_query(&engine, query)?;
    let output = render_template_safe(&renderer, "openapi-info.tera", sparql_results)?;

    // Assert - Verify rendered output
    assert!(
        output.contains("openapi: \"3.0.0\""),
        "Missing OpenAPI version"
    );
    assert!(output.contains("title: Blog API"), "Missing API title");
    assert!(output.contains("version: 1.0.0"), "Missing API version");
    assert!(output.contains("servers:"), "Missing servers section");
    assert!(
        output.contains("http://localhost:3000"),
        "Missing server URL"
    );

    // Write output for inspection
    fs::write(test_output_path("openapi/api-info.yaml"), &output)?;

    Ok(())
});

test!(test_render_zod_schemas, {
    // Arrange
    init_test_output()?;
    let engine = load_ontology_engine()?;
    let config = RenderConfig::default();
    let renderer = SafeRenderer::from_directory(workspace_path("templates/openapi"), config)?;

    // Act - Execute query and render template
    let query = r#"
PREFIX api: <https://ggen.io/ontology/api#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

SELECT ?entityName ?entityDescription ?propertyName ?propertyType
       ?required ?minLength ?maxLength ?format
WHERE {
  ?entity a api:Entity ;
          api:name ?entityName .
  OPTIONAL { ?entity rdfs:comment ?entityDescription }

  ?entity api:hasProperty ?property .
  ?property api:name ?propertyName ;
            api:type ?propertyType .
  OPTIONAL { ?property api:required ?required }
  OPTIONAL { ?property api:minLength ?minLength }
  OPTIONAL { ?property api:maxLength ?maxLength }
  OPTIONAL { ?property api:format ?format }
}
ORDER BY ?entityName ?propertyName
"#;

    let sparql_results = execute_query(&engine, query)?;
    let output = render_template_safe(&renderer, "zod-schemas.tera", sparql_results)?;

    // Assert - Verify rendered output
    assert!(
        output.contains("import { z } from \"zod\""),
        "Missing Zod import"
    );
    assert!(
        output.contains("userSchema"),
        "Missing userSchema definition"
    );
    assert!(
        output.contains("postSchema"),
        "Missing postSchema definition"
    );
    assert!(output.contains("z.string()"), "Missing string type");
    assert!(output.contains(".email("), "Missing email validation");

    // Write output for inspection
    fs::write(test_output_path("schemas/entities.mjs"), &output)?;

    Ok(())
});

test!(test_full_workflow_execution, {
    // Arrange
    init_test_output()?;
    let engine = load_ontology_engine()?;
    let config = RenderConfig::default();
    let renderer = SafeRenderer::from_directory(workspace_path("templates/openapi"), config)?;
    let workflow = load_workflow()?;
    let steps = workflow["steps"].as_array().expect("steps is not an array");

    let mut generated_files = Vec::new();

    // Act - Execute workflow steps (simplified version - just render templates)
    for step in steps.iter().take(13) {
        // Skip validation step
        if step["tool"] == "validate_generated_files" {
            continue;
        }

        if step["tool"] == "render_template" {
            let step_id = step["id"].as_str().unwrap_or("unknown");
            let template_name = step["params"]["template_path"]
                .as_str()
                .unwrap()
                .split('/')
                .last()
                .unwrap();
            let output_path = step["params"]["output_path"].as_str().unwrap();

            // Find corresponding query step
            if let Some(depends_on) = step.get("depends_on") {
                let query_id = depends_on[0].as_str().unwrap();
                let query_step = steps
                    .iter()
                    .find(|s| s["id"] == query_id)
                    .expect("Query step not found");

                let query = query_step["params"]["query"].as_str().unwrap();
                let sparql_results = execute_query(&engine, query)?;

                // Render template
                let output = render_template_safe(&renderer, template_name, sparql_results)?;

                // Write output
                let full_output_path = test_output_path(output_path);
                if let Some(parent) = full_output_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&full_output_path, &output)?;

                generated_files.push((step_id.to_string(), full_output_path));
            }
        }
    }

    // Assert - Verify all expected files were generated
    assert!(
        generated_files.len() >= 10,
        "Expected at least 10 files generated, got {}",
        generated_files.len()
    );

    // Verify key files exist and have content
    for (step_id, path) in &generated_files {
        assert!(
            path.exists(),
            "Step '{}' did not generate file: {:?}",
            step_id,
            path
        );
        let content = fs::read_to_string(path)?;
        assert!(
            !content.is_empty(),
            "Step '{}' generated empty file: {:?}",
            step_id,
            path
        );
    }

    Ok(())
});

// ============================================================================
// Golden File Comparison Tests (Optional - requires golden files)
// ============================================================================

test!(test_compare_with_golden_files, {
    // Arrange
    let test_output_dir = test_output_path("");
    let golden_dir = golden_path("lib");

    // Skip if golden files don't exist yet
    if !golden_dir.exists() {
        eprintln!("Skipping golden file comparison - golden files not generated yet");
        eprintln!("Run: cd ggen/examples/openapi && ggen sync");
        eprintln!("Then: cp -r lib/* ../../tests/golden/openapi/lib/");
        return Ok(());
    }

    // Act & Assert - Compare test outputs with golden files
    let test_files = [
        "openapi/api-info.yaml",
        "schemas/entities.mjs",
        "types/entities.mjs",
    ];

    for file_path in &test_files {
        let test_file = test_output_dir.join(file_path);
        let golden_file = golden_dir.join(file_path);

        if test_file.exists() && golden_file.exists() {
            let test_content = fs::read_to_string(&test_file)?;
            let golden_content = fs::read_to_string(&golden_file)?;

            // Allow minor whitespace differences
            let test_normalized = test_content.lines().map(|l| l.trim()).collect::<Vec<_>>();
            let golden_normalized = golden_content.lines().map(|l| l.trim()).collect::<Vec<_>>();

            assert_eq!(
                test_normalized, golden_normalized,
                "Output differs from golden file: {}",
                file_path
            );
        }
    }

    Ok(())
});
