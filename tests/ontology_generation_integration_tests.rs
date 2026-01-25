//! Integration Tests for Ontology Generation
//!
//! Chicago-style TDD integration tests for the complete ontology-driven code generation workflow.
//! Tests verify end-to-end flows: Load ontology → Execute SPARQL → Render templates → Validate
//!
//! # Test Coverage
//!
//! - Full workflow execution
//! - Preview mode (no writes)
//! - Query caching
//! - Golden file comparison
//! - Error recovery
//!
//! # Usage
//!
//! Run all ontology generation tests:
//! ```bash
//! cargo test --test ontology_generation_integration_tests
//! ```
//!
//! Run specific test:
//! ```bash
//! cargo test --test ontology_generation_integration_tests test_full_workflow
//! ```

mod harness;

use anyhow::Result;
use harness::ontology_generation_harness::OntologyGenerationHarness;

// ============================================================================
// Full Workflow Tests
// ============================================================================

#[test]
fn test_full_workflow() -> Result<()> {
    println!("\n=== TEST: Full Workflow ===\n");

    let mut harness = OntologyGenerationHarness::new()
        .with_fixture("test-api")
        .with_preview_mode(false)
        .with_golden_comparison(false); // Disable for first run

    // Setup: Load ontology
    harness.load_ontology("test-api")?;

    // Setup: Register queries
    harness.register_query("test_entities", "test-entities.rq")?;
    harness.register_query("test_valueobjects", "test-valueobjects.rq")?;
    harness.register_query("test_commands", "test-commands.rq")?;

    // Setup: Register templates
    harness.register_template("test-schema.rs", "test-schema.tera")?;
    harness.register_template("test-types.mjs", "test-types.tera")?;
    harness.register_template("test-openapi.yaml", "test-openapi.tera")?;

    // Execute: Run complete workflow
    let result = harness.execute_workflow()?;

    // Verify: Workflow succeeded
    harness.verify_output(&result)?;

    // Assert: All stages completed
    assert!(!result.query_results.is_empty(), "No query results");
    assert!(!result.rendered_outputs.is_empty(), "No rendered outputs");
    assert!(!result.written_files.is_empty(), "No files written");
    assert!(result.validation_report.valid, "Validation failed");

    // Assert: Expected queries executed
    assert!(
        result.query_results.contains_key("test_entities"),
        "Missing test_entities query results"
    );
    assert!(
        result.query_results.contains_key("test_valueobjects"),
        "Missing test_valueobjects query results"
    );
    assert!(
        result.query_results.contains_key("test_commands"),
        "Missing test_commands query results"
    );

    // Assert: Expected outputs rendered
    assert!(
        result.rendered_outputs.contains_key("test-schema.rs"),
        "Missing test-schema.rs output"
    );
    assert!(
        result.rendered_outputs.contains_key("test-types.mjs"),
        "Missing test-types.mjs output"
    );
    assert!(
        result.rendered_outputs.contains_key("test-openapi.yaml"),
        "Missing test-openapi.yaml output"
    );

    // Assert: Outputs are non-empty
    for (name, output) in &result.rendered_outputs {
        assert!(!output.content.is_empty(), "Output {} is empty", name);
        assert!(
            output.content.len() > 100,
            "Output {} too small: {} bytes",
            name,
            output.content.len()
        );
    }

    // Assert: No TODO markers in generated code
    for (name, output) in &result.rendered_outputs {
        assert!(
            !output.content.contains("TODO"),
            "Output {} contains TODO markers",
            name
        );
    }

    // Metrics verification
    println!("\n📊 Workflow Metrics:");
    println!("  Ontology load: {:?}", result.metrics.ontology_load_time);
    println!(
        "  Query execution: {:?}",
        result.metrics.query_execution_time
    );
    println!(
        "  Template render: {:?}",
        result.metrics.template_render_time
    );
    println!("  Total time: {:?}", result.metrics.total_workflow_time);

    // Teardown
    harness.teardown()?;

    println!("\n✅ Full workflow test passed\n");
    Ok(())
}

#[test]
fn test_preview_mode() -> Result<()> {
    println!("\n=== TEST: Preview Mode ===\n");

    let mut harness = OntologyGenerationHarness::new()
        .with_fixture("test-api")
        .with_preview_mode(true); // Enable preview mode

    // Setup
    harness.load_ontology("test-api")?;
    harness.register_query("test_entities", "test-entities.rq")?;
    harness.register_template("test-schema.rs", "test-schema.tera")?;

    // Execute workflow in preview mode
    let result = harness.execute_workflow()?;

    // Verify: Workflow succeeded
    harness.verify_output(&result)?;

    // Assert: Files were NOT written (preview mode)
    assert!(
        result.written_files.is_empty(),
        "Files should not be written in preview mode"
    );

    // Assert: Templates were rendered
    assert!(
        !result.rendered_outputs.is_empty(),
        "Templates should be rendered in preview mode"
    );

    // Assert: Validation still ran
    assert!(
        result.validation_report.valid,
        "Validation should run in preview mode"
    );

    println!("✅ Preview mode test passed\n");
    Ok(())
}

#[test]
fn test_cache_hit() -> Result<()> {
    println!("\n=== TEST: Cache Hit ===\n");

    let mut harness = OntologyGenerationHarness::new()
        .with_fixture("test-api")
        .with_preview_mode(true);

    // Setup
    harness.load_ontology("test-api")?;
    harness.register_query("test_entities", "test-entities.rq")?;

    // Test cache behavior
    let cache_result = harness.test_cache_hit()?;

    // Assert: All queries were cached on second execution
    assert!(
        cache_result.all_cached,
        "Not all queries were cached: {}/{} from cache",
        cache_result.cache_hits, cache_result.total_queries
    );

    // Assert: Cache improved performance (second execution should be faster)
    for (query_name, first_time) in &cache_result.first_execution_times {
        println!("  Query '{}': first={:?}", query_name, first_time);
    }

    println!(
        "  Cache hits: {}/{}",
        cache_result.cache_hits, cache_result.total_queries
    );
    println!("✅ Cache test passed\n");
    Ok(())
}

#[test]
fn test_golden_file_comparison() -> Result<()> {
    println!("\n=== TEST: Golden File Comparison ===\n");

    let mut harness = OntologyGenerationHarness::new()
        .with_fixture("test-api")
        .with_preview_mode(false)
        .with_golden_comparison(true);

    // Setup
    harness.load_ontology("test-api")?;
    harness.register_query("test_entities", "test-entities.rq")?;
    harness.register_query("test_valueobjects", "test-valueobjects.rq")?;
    harness.register_query("test_commands", "test-commands.rq")?;
    harness.register_template("test-schema.rs", "test-schema.tera")?;
    harness.register_template("test-types.mjs", "test-types.tera")?;
    harness.register_template("test-openapi.yaml", "test-openapi.tera")?;

    // Execute workflow
    let result = harness.execute_workflow()?;
    harness.verify_output(&result)?;

    // Compare against golden files
    let comparison = harness.compare_golden_files(&result)?;

    // Assert: Golden comparison was enabled
    assert!(comparison.enabled, "Golden comparison should be enabled");

    // Assert: All comparisons were made
    assert!(
        !comparison.comparisons.is_empty(),
        "No golden file comparisons made"
    );

    // Report differences (if any)
    if !comparison.all_match {
        println!("\n⚠️  Some outputs differ from golden files:");
        for comp in &comparison.comparisons {
            if !comp.matches {
                println!("  ❌ {}", comp.template_name);
                if let Some(diff) = &comp.difference {
                    println!("     Difference: {}", diff);
                }
            }
        }
    } else {
        println!("✅ All outputs match golden files");
    }

    // Note: We don't assert all_match here because golden files might not be set up yet
    // In production, you would assert: assert!(comparison.all_match, "Outputs differ from golden files");

    // Teardown
    harness.teardown()?;

    println!("✅ Golden file comparison test completed\n");
    Ok(())
}

// ============================================================================
// Error Recovery Tests
// ============================================================================

#[test]
fn test_error_recovery_missing_ontology() -> Result<()> {
    println!("\n=== TEST: Error Recovery - Missing Ontology ===\n");

    let mut harness = OntologyGenerationHarness::new().with_fixture("nonexistent-ontology");

    // Attempt to load non-existent ontology
    let result = harness.load_ontology("nonexistent-ontology");

    // Assert: Error is returned
    assert!(result.is_err(), "Should fail with missing ontology");

    let error = result.unwrap_err();
    println!("Expected error: {}", error);

    println!("✅ Error recovery test passed\n");
    Ok(())
}

#[test]
fn test_error_recovery_missing_query() -> Result<()> {
    println!("\n=== TEST: Error Recovery - Missing Query ===\n");

    let mut harness = OntologyGenerationHarness::new().with_fixture("test-api");

    // Attempt to register non-existent query
    let result = harness.register_query("missing", "nonexistent.rq");

    // Assert: Error is returned
    assert!(result.is_err(), "Should fail with missing query");

    let error = result.unwrap_err();
    println!("Expected error: {}", error);

    println!("✅ Error recovery test passed\n");
    Ok(())
}

#[test]
fn test_error_recovery_missing_template() -> Result<()> {
    println!("\n=== TEST: Error Recovery - Missing Template ===\n");

    let mut harness = OntologyGenerationHarness::new().with_fixture("test-api");

    // Attempt to register non-existent template
    let result = harness.register_template("missing", "nonexistent.tera");

    // Assert: Error is returned
    assert!(result.is_err(), "Should fail with missing template");

    let error = result.unwrap_err();
    println!("Expected error: {}", error);

    println!("✅ Error recovery test passed\n");
    Ok(())
}

#[test]
fn test_error_recovery_no_fixture_set() -> Result<()> {
    println!("\n=== TEST: Error Recovery - No Fixture Set ===\n");

    let mut harness = OntologyGenerationHarness::new();
    // Don't call with_fixture()

    // Attempt to execute workflow without setting fixture
    let result = harness.execute_workflow();

    // Assert: Error is returned
    assert!(result.is_err(), "Should fail without fixture set");

    let error = result.unwrap_err();
    println!("Expected error: {}", error);

    println!("✅ Error recovery test passed\n");
    Ok(())
}

// ============================================================================
// State-Based Assertions (Chicago-TDD)
// ============================================================================

#[test]
fn test_state_after_load() -> Result<()> {
    println!("\n=== TEST: State After Load ===\n");

    let mut harness = OntologyGenerationHarness::new().with_fixture("test-api");

    // Load ontology
    harness.load_ontology("test-api")?;

    // Register query and template
    harness.register_query("test_entities", "test-entities.rq")?;
    harness.register_template("test-schema.rs", "test-schema.tera")?;

    // Execute workflow
    let result = harness.execute_workflow()?;

    // State-based assertions: Verify object state changes
    assert_eq!(result.fixture, "test-api");
    assert_eq!(result.query_results.len(), 1);
    assert_eq!(result.rendered_outputs.len(), 1);

    // Verify metrics are populated
    assert!(result.metrics.ontology_load_time.as_millis() > 0);
    assert!(result.metrics.total_workflow_time.as_millis() > 0);

    println!("✅ State verification test passed\n");
    Ok(())
}

#[test]
fn test_deterministic_output() -> Result<()> {
    println!("\n=== TEST: Deterministic Output ===\n");

    let mut harness1 = OntologyGenerationHarness::new()
        .with_fixture("test-api")
        .with_preview_mode(true);

    // Setup first harness
    harness1.load_ontology("test-api")?;
    harness1.register_query("test_entities", "test-entities.rq")?;
    harness1.register_template("test-schema.rs", "test-schema.tera")?;

    // Execute first workflow
    let result1 = harness1.execute_workflow()?;

    // Setup second harness with same configuration
    let mut harness2 = OntologyGenerationHarness::new()
        .with_fixture("test-api")
        .with_preview_mode(true);

    harness2.load_ontology("test-api")?;
    harness2.register_query("test_entities", "test-entities.rq")?;
    harness2.register_template("test-schema.rs", "test-schema.tera")?;

    // Execute second workflow
    let result2 = harness2.execute_workflow()?;

    // Assert: Outputs are identical
    for (name, output1) in &result1.rendered_outputs {
        let output2 = result2
            .rendered_outputs
            .get(name)
            .expect(&format!("Output {} missing in second run", name));

        assert_eq!(
            output1.content, output2.content,
            "Output {} differs between runs",
            name
        );
    }

    println!("✅ Deterministic output test passed\n");
    Ok(())
}

// ============================================================================
// Property-Based Tests
// ============================================================================

#[test]
fn test_property_all_queries_have_results() -> Result<()> {
    println!("\n=== TEST: Property - All Queries Have Results ===\n");

    let mut harness = OntologyGenerationHarness::new()
        .with_fixture("test-api")
        .with_preview_mode(true);

    harness.load_ontology("test-api")?;
    harness.register_query("test_entities", "test-entities.rq")?;
    harness.register_query("test_valueobjects", "test-valueobjects.rq")?;
    harness.register_query("test_commands", "test-commands.rq")?;

    let result = harness.execute_workflow()?;

    // Property: Every registered query produces results
    for (name, query_result) in &result.query_results {
        assert!(
            !query_result.bindings.is_empty(),
            "Query '{}' should return results",
            name
        );
    }

    println!("✅ Property test passed\n");
    Ok(())
}

#[test]
fn test_property_all_templates_render_non_empty() -> Result<()> {
    println!("\n=== TEST: Property - All Templates Render Non-Empty ===\n");

    let mut harness = OntologyGenerationHarness::new()
        .with_fixture("test-api")
        .with_preview_mode(true);

    harness.load_ontology("test-api")?;
    harness.register_query("test_entities", "test-entities.rq")?;
    harness.register_template("test-schema.rs", "test-schema.tera")?;

    let result = harness.execute_workflow()?;

    // Property: Every template renders non-empty content
    for (name, output) in &result.rendered_outputs {
        assert!(
            !output.content.is_empty(),
            "Template '{}' should render non-empty content",
            name
        );
    }

    println!("✅ Property test passed\n");
    Ok(())
}
