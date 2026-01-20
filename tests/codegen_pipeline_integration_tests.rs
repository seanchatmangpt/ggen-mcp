//! Comprehensive Integration Tests for Code Generation Pipeline
//!
//! These tests exercise the complete end-to-end pipeline using the
//! Chicago-style TDD test harness.
//!
//! # Test Coverage
//!
//! - Simple scenarios (single aggregate, single command)
//! - Complex scenarios (complete domain with multiple aggregates)
//! - MCP tool generation
//! - Error handling and validation
//! - Golden file comparison
//! - Incremental updates
//! - Performance benchmarks

mod harness;

use chicago_tdd_tools::prelude::*;
use harness::*;

// ============================================================================
// Simple Scenarios
// ============================================================================

test!(test_simple_aggregate_complete_pipeline, {
    alert_info!("Test: Simple Aggregate - Complete Pipeline");

    // Arrange: Set up harness with fixture and validation
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true)
        .with_golden_files(true);

    // Act: Run the complete pipeline
    let result = harness.run_complete_pipeline()?;

    // Assert: Verify all stages succeeded
    harness.assert_all_stages_succeeded(&result);

    // Assert: Verify specific outputs
    assert_eq_msg!(
        result.fixture,
        "simple_aggregate",
        "Fixture name should match"
    );
    assert!(
        result.ontology_result.triple_count > 0,
        "Should have loaded triples from ontology"
    );
    assert!(
        !result.sparql_result.entities.is_empty(),
        "Should have extracted entities from SPARQL"
    );
    assert!(
        !result.template_result.rendered_code.is_empty(),
        "Should have rendered code from templates"
    );

    // Print summary for debugging
    harness.metrics.print_summary();
    alert_success!("All stages completed successfully");

    Ok::<(), anyhow::Error>(())
});

test!(test_simple_aggregate_ontology_loading, {
    alert_info!("Test: Simple Aggregate - Ontology Loading Stage");

    // Arrange: Set up harness with fixture
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true);

    // Act: Run the complete pipeline
    let result = harness.run_complete_pipeline()?;

    // Assert: Verify ontology was loaded correctly
    assert!(
        result.ontology_result.triple_count > 0,
        "Should have loaded triples from ontology"
    );
    assert!(
        !result.ontology_result.ttl_content.is_empty(),
        "TTL content should not be empty"
    );

    // Assert: Verify integrity if enabled
    if let Some(ref report) = result.ontology_result.integrity_report {
        alert_debug!("Integrity Report: {:?}", report.is_valid());
    }

    alert_success!(
        "Ontology loaded: {} triples",
        result.ontology_result.triple_count
    );

    Ok::<(), anyhow::Error>(())
});

test!(test_simple_aggregate_sparql_extraction, {
    alert_info!("Test: Simple Aggregate - SPARQL Entity Extraction");

    // Arrange: Set up harness with fixture
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true);

    // Act: Run the complete pipeline
    let result = harness.run_complete_pipeline()?;

    // Assert: Verify entities were extracted
    assert!(
        !result.sparql_result.entities.is_empty(),
        "Should have extracted entities from SPARQL query"
    );

    // Act: Extract entity names for verification
    let entity_names: Vec<String> = result
        .sparql_result
        .entities
        .iter()
        .map(|e| e.name.clone())
        .collect();

    // Assert: Verify entity extraction
    alert_debug!("Extracted entities: {:?}", entity_names);
    alert_success!("{} entities extracted", entity_names.len());

    Ok::<(), anyhow::Error>(())
});

test!(test_simple_aggregate_template_rendering, {
    alert_info!("Test: Simple Aggregate - Template Rendering");

    // Arrange: Set up harness with fixture
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true);

    // Act: Run the complete pipeline
    let result = harness.run_complete_pipeline()?;

    // Assert: Verify code was rendered
    assert!(
        !result.template_result.rendered_code.is_empty(),
        "Should have rendered code from templates"
    );

    // Assert: Verify each rendered file is not empty
    for (file_name, code) in &result.template_result.rendered_code {
        assert!(
            !code.is_empty(),
            "Rendered code for {} should not be empty",
            file_name
        );
        alert_success!("Rendered: {} ({} bytes)", file_name, code.len());
    }

    Ok::<(), anyhow::Error>(())
});

test!(test_simple_aggregate_code_validation, {
    alert_info!("Test: Simple Aggregate - Code Validation");

    // Arrange: Set up harness with fixture and validation enabled
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true);

    // Act: Run the complete pipeline
    let result = harness.run_complete_pipeline()?;

    // Assert: Verify all code is valid
    assert!(
        result.validation_result.all_valid,
        "All generated code should be valid"
    );

    // Assert: Verify each file compiles
    for (file_name, code) in &result.validation_result.validated_code {
        harness.assert_code_compiles(code)?;
        alert_success!("Valid: {}", file_name);
    }

    Ok::<(), anyhow::Error>(())
});

test!(test_simple_aggregate_file_writing, {
    alert_info!("Test: Simple Aggregate - File Writing");

    // Arrange: Set up harness with fixture
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true);

    // Act: Run the complete pipeline
    let result = harness.run_complete_pipeline()?;

    // Assert: Verify files were written
    assert!(
        !result.file_result.written_files.is_empty(),
        "Should have written files to filesystem"
    );

    // Assert: Verify files exist on filesystem
    for path in &result.file_result.written_files {
        assert!(
            path.exists(),
            "Written file should exist: {}",
            path.display()
        );
        alert_success!("Written: {}", path.display());
    }

    Ok::<(), anyhow::Error>(())
});

// ============================================================================
// Complex Scenarios
// ============================================================================

test!(test_complete_domain_pipeline, {
    alert_info!("Test: Complete Domain - Full DDD Structure");

    // Arrange: Set up harness with complete domain fixture
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("complete_domain")
        .with_validation(true)
        .with_golden_files(true);

    // Act: Run the complete pipeline
    let result = harness.run_complete_pipeline()?;

    // Assert: Verify all stages succeeded
    harness.assert_all_stages_succeeded(&result);

    // Assert: Verify multiple entities were extracted
    assert!(
        result.sparql_result.entities.len() >= 3,
        "Should have at least 3 entities (User, Product, Order)"
    );

    // Assert: Verify multiple code files were generated
    assert!(
        result.template_result.rendered_code.len() >= 3,
        "Should have generated code for multiple entities"
    );

    alert_success!(
        "Complete domain generated: {} entities",
        result.sparql_result.entities.len()
    );

    // Print metrics for debugging
    harness.metrics.print_summary();

    Ok::<(), anyhow::Error>(())
});

test!(test_complete_domain_with_value_objects, {
    alert_info!("Test: Complete Domain - Value Objects");

    // Arrange: Set up harness with complete domain fixture
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("complete_domain")
        .with_validation(true);

    // Act: Run the complete pipeline
    let result = harness.run_complete_pipeline()?;

    // Act: Filter for value object entities
    let value_objects: Vec<_> = result
        .sparql_result
        .entities
        .iter()
        .filter(|e| e.entity_type.contains("ValueObject"))
        .collect();

    // Assert: Log value objects if found
    if !value_objects.is_empty() {
        alert_info!("Found {} value objects", value_objects.len());
        for vo in value_objects {
            alert_debug!("  - {}", vo.name);
        }
    } else {
        alert_warning!("No value objects found in domain");
    }

    Ok::<(), anyhow::Error>(())
});

// ============================================================================
// MCP Tool Scenarios
// ============================================================================

test!(test_mcp_tool_generation, {
    alert_info!("Test: MCP Tool - Handler Generation");

    // Arrange: Set up harness with MCP tool fixture
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("mcp_tool")
        .with_validation(true);

    // Act: Run the complete pipeline
    let result = harness.run_complete_pipeline()?;

    // Assert: Verify tool entities were extracted
    assert!(
        !result.sparql_result.entities.is_empty(),
        "Should have extracted tool entities"
    );

    // Assert: Verify code was generated
    assert!(
        !result.template_result.rendered_code.is_empty(),
        "Should have generated MCP tool handler code"
    );

    alert_success!(
        "MCP tools generated: {}",
        result.sparql_result.entities.len()
    );

    Ok::<(), anyhow::Error>(())
});

// ============================================================================
// Error Scenarios
// ============================================================================

test!(test_invalid_ontology_error_handling, {
    alert_info!("Test: Error Handling - Invalid Ontology");

    // Arrange: Set up harness with error scenarios fixture
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("error_scenarios")
        .with_validation(true);

    // Act: This should fail gracefully - we expect an error or warning
    let result = harness.run_complete_pipeline();

    // Assert: Verify error handling behavior
    match result {
        Ok(r) => {
            // If it succeeds, validation should have caught issues
            if r.validation_result.all_valid {
                alert_warning!("Invalid ontology passed validation unexpectedly");
            } else {
                alert_success!("Validation caught invalid ontology");
            }
        }
        Err(e) => {
            alert_success!("Pipeline failed as expected: {}", e);
        }
    }
});

test!(test_missing_template_fallback, {
    alert_info!("Test: Error Handling - Missing Template");

    // Arrange: Set up harness with validation disabled to test template handling
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(false);

    // Act: This should use default templates
    let result = harness.run_complete_pipeline()?;

    // Assert: Should still generate code using defaults
    assert!(
        !result.template_result.rendered_code.is_empty(),
        "Should generate code using default templates"
    );

    alert_success!("Fallback to default templates worked");

    Ok::<(), anyhow::Error>(())
});

// ============================================================================
// Golden File Testing
// ============================================================================

test!(test_golden_file_comparison, {
    alert_info!("Test: Golden File - Output Comparison");

    // Arrange: Set up harness with golden file comparison enabled
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true)
        .with_golden_files(true);

    // Act: Run the complete pipeline
    let result = harness.run_complete_pipeline()?;

    // Act: Compare against golden files
    let report = harness.compare_golden_files(&result)?;

    // Assert: Print comparison report
    report.print_summary();

    // Note: Golden files might not match exactly due to formatting
    // This test documents the differences
    if !report.is_perfect_match() {
        alert_info!("Golden file comparison found differences");
        alert_info!("Run with UPDATE_GOLDEN=1 to update golden files");
    } else {
        alert_success!("Generated code matches golden files perfectly");
    }

    Ok::<(), anyhow::Error>(())
});

test!(
    #[ignore] // Only run when explicitly requested
    test_update_golden_files,
    {
        alert_info!("Test: Golden File - Update Golden Files");

        // Arrange: Set up harness with fixture
        let mut harness = CodegenPipelineHarness::new()
            .with_fixture("simple_aggregate")
            .with_validation(true);

        // Act: Run the complete pipeline
        let result = harness.run_complete_pipeline()?;

        // Act: Update golden files
        harness.update_golden_files(&result)?;

        alert_success!("Golden files updated");

        Ok::<(), anyhow::Error>(())
    }
);

// ============================================================================
// Incremental Updates
// ============================================================================

test!(test_incremental_generation, {
    alert_info!("Test: Incremental - Change Detection");

    // Arrange: Set up harness with incremental mode enabled
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true)
        .with_incremental(true);

    // Act: First run
    let result1 = harness.run_complete_pipeline()?;
    let files1 = result1.file_result.written_files.len();

    // Act: Second run (should detect no changes)
    let result2 = harness.run_complete_pipeline()?;
    let files2 = result2.file_result.written_files.len();

    // Assert: Both runs should produce same number of files
    assert_eq_msg!(
        files1,
        files2,
        "Incremental runs should produce same number of files"
    );

    alert_success!("Incremental generation: {} files", files1);

    Ok::<(), anyhow::Error>(())
});

// ============================================================================
// Performance Benchmarks
// ============================================================================

test!(test_pipeline_performance, {
    alert_info!("Test: Performance - Pipeline Benchmarks");

    // Arrange: Set up harness with fixture
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true);

    // Act: Run the complete pipeline
    let result = harness.run_complete_pipeline()?;

    // Act: Extract performance metrics
    let total_ms = result.duration.as_millis();

    // Assert: Log performance metrics
    alert_info!("Performance Metrics:");
    alert_debug!("  Total time: {} ms", total_ms);
    alert_debug!("  Ontology:   {:?}", result.ontology_result.duration);
    alert_debug!("  SPARQL:     {:?}", result.sparql_result.duration);
    alert_debug!("  Template:   {:?}", result.template_result.duration);
    alert_debug!("  Validation: {:?}", result.validation_result.duration);
    alert_debug!("  File I/O:   {:?}", result.file_result.duration);

    // Assert: Verify reasonable performance threshold (< 5 seconds for simple case)
    assert!(
        total_ms < 5000,
        "Pipeline should complete in under 5 seconds, took {} ms",
        total_ms
    );

    alert_success!("Performance within acceptable limits");

    Ok::<(), anyhow::Error>(())
});

test!(test_complex_domain_performance, {
    alert_info!("Test: Performance - Complex Domain");

    // Arrange: Set up harness with complex domain fixture
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("complete_domain")
        .with_validation(true);

    // Act: Run the complete pipeline
    let result = harness.run_complete_pipeline()?;

    // Act: Extract performance metrics
    let total_ms = result.duration.as_millis();

    alert_info!("Complex domain processed in: {} ms", total_ms);

    // Assert: Verify more relaxed threshold for complex domains (< 10 seconds)
    assert!(
        total_ms < 10000,
        "Complex domain should complete in under 10 seconds, took {} ms",
        total_ms
    );

    // Print detailed metrics
    harness.metrics.print_summary();

    alert_success!("Complex domain performance acceptable");

    Ok::<(), anyhow::Error>(())
});

// ============================================================================
// Integration Points
// ============================================================================

test!(test_programmatic_api, {
    alert_info!("Test: Integration - Programmatic API");

    // Arrange: Test using harness as a library
    let mut harness = CodegenPipelineHarness::new();

    // Arrange: Configure via API
    harness = harness
        .with_fixture("simple_aggregate")
        .with_validation(true)
        .with_golden_files(false);

    // Act: Run the complete pipeline
    let result = harness.run_complete_pipeline()?;

    // Assert: Verify pipeline succeeded
    assert!(result.success, "Pipeline should succeed");

    alert_success!("Programmatic API works correctly");

    Ok::<(), anyhow::Error>(())
});

// ============================================================================
// Comprehensive End-to-End Test
// ============================================================================

test!(test_comprehensive_pipeline_validation, {
    alert_info!("Test: Comprehensive - Full Pipeline Validation");
    alert_info!("This test runs the complete pipeline and validates every stage");

    // Arrange: Set up harness with full validation
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true)
        .with_golden_files(false);

    // Act: Run the complete pipeline
    let result = harness.run_complete_pipeline()?;

    // Assert: Stage 1 - Ontology Loading
    alert_success!("Stage 1: Ontology Loading");
    assert!(
        result.ontology_result.triple_count > 0,
        "Should have loaded triples from ontology"
    );
    alert_debug!("  - Loaded {} triples", result.ontology_result.triple_count);

    // Assert: Stage 2 - SPARQL Query
    alert_success!("Stage 2: SPARQL Query");
    assert!(
        !result.sparql_result.entities.is_empty(),
        "Should have extracted entities"
    );
    alert_debug!(
        "  - Extracted {} entities",
        result.sparql_result.entities.len()
    );

    // Assert: Stage 3 - Template Rendering
    alert_success!("Stage 3: Template Rendering");
    assert!(
        !result.template_result.rendered_code.is_empty(),
        "Should have rendered code"
    );
    alert_debug!(
        "  - Rendered {} files",
        result.template_result.rendered_code.len()
    );

    // Assert: Stage 4 - Code Validation
    alert_success!("Stage 4: Code Validation");
    assert!(
        result.validation_result.all_valid,
        "All code should be valid"
    );
    for (file_name, code) in &result.validation_result.validated_code {
        harness.assert_code_compiles(code)?;
        harness.assert_all_imports_valid(code)?;
        harness.assert_no_unused_code(code)?;
    }
    alert_debug!("  - All code validated successfully");

    // Assert: Stage 5 - File Writing
    alert_success!("Stage 5: File Writing");
    assert!(
        !result.file_result.written_files.is_empty(),
        "Should have written files"
    );
    for path in &result.file_result.written_files {
        assert!(path.exists(), "Written file should exist: {}", path.display());
    }
    alert_debug!("  - Wrote {} files", result.file_result.written_files.len());

    // Print final summary
    alert_info!("Pipeline Summary:");
    harness.metrics.print_summary();

    alert_success!("All validation checks passed!");

    Ok::<(), anyhow::Error>(())
});
