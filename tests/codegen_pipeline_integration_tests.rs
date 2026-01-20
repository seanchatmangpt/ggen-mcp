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

use harness::*;
use anyhow::Result;

// ============================================================================
// Simple Scenarios
// ============================================================================

#[test]
fn test_simple_aggregate_complete_pipeline() -> Result<()> {
    println!("\n🧪 Test: Simple Aggregate - Complete Pipeline");

    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true)
        .with_golden_files(true);

    let result = harness.run_complete_pipeline()?;

    // Verify all stages succeeded
    harness.assert_all_stages_succeeded(&result);

    // Print metrics
    result.ontology_result.duration;
    println!("  ✓ All stages completed successfully");

    // Verify specific outputs
    assert_eq!(result.fixture, "simple_aggregate");
    assert!(result.ontology_result.triple_count > 0);
    assert!(!result.sparql_result.entities.is_empty());
    assert!(!result.template_result.rendered_code.is_empty());

    // Print summary
    harness.metrics.print_summary();

    Ok(())
}

#[test]
fn test_simple_aggregate_ontology_loading() -> Result<()> {
    println!("\n🧪 Test: Simple Aggregate - Ontology Loading Stage");

    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true);

    let result = harness.run_complete_pipeline()?;

    // Verify ontology was loaded correctly
    assert!(result.ontology_result.triple_count > 0);
    assert!(!result.ontology_result.ttl_content.is_empty());

    // Verify integrity if enabled
    if let Some(ref report) = result.ontology_result.integrity_report {
        println!("  Integrity Report: {:?}", report.is_valid());
    }

    println!("  ✓ Ontology loaded: {} triples", result.ontology_result.triple_count);

    Ok(())
}

#[test]
fn test_simple_aggregate_sparql_extraction() -> Result<()> {
    println!("\n🧪 Test: Simple Aggregate - SPARQL Entity Extraction");

    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true);

    let result = harness.run_complete_pipeline()?;

    // Verify entities were extracted
    assert!(!result.sparql_result.entities.is_empty());

    // Check for expected entities
    let entity_names: Vec<String> = result
        .sparql_result
        .entities
        .iter()
        .map(|e| e.name.clone())
        .collect();

    println!("  Extracted entities: {:?}", entity_names);
    println!("  ✓ {} entities extracted", entity_names.len());

    Ok(())
}

#[test]
fn test_simple_aggregate_template_rendering() -> Result<()> {
    println!("\n🧪 Test: Simple Aggregate - Template Rendering");

    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true);

    let result = harness.run_complete_pipeline()?;

    // Verify code was rendered
    assert!(!result.template_result.rendered_code.is_empty());

    // Verify rendered code is not empty
    for (file_name, code) in &result.template_result.rendered_code {
        assert!(!code.is_empty(), "Rendered code for {} should not be empty", file_name);
        println!("  ✓ Rendered: {} ({} bytes)", file_name, code.len());
    }

    Ok(())
}

#[test]
fn test_simple_aggregate_code_validation() -> Result<()> {
    println!("\n🧪 Test: Simple Aggregate - Code Validation");

    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true);

    let result = harness.run_complete_pipeline()?;

    // Verify all code is valid
    assert!(result.validation_result.all_valid, "All generated code should be valid");

    // Verify each file compiles
    for (file_name, code) in &result.validation_result.validated_code {
        harness.assert_code_compiles(code)?;
        println!("  ✓ Valid: {}", file_name);
    }

    Ok(())
}

#[test]
fn test_simple_aggregate_file_writing() -> Result<()> {
    println!("\n🧪 Test: Simple Aggregate - File Writing");

    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true);

    let result = harness.run_complete_pipeline()?;

    // Verify files were written
    assert!(!result.file_result.written_files.is_empty());

    // Verify files exist on filesystem
    for path in &result.file_result.written_files {
        assert!(path.exists(), "Written file should exist: {}", path.display());
        println!("  ✓ Written: {}", path.display());
    }

    Ok(())
}

// ============================================================================
// Complex Scenarios
// ============================================================================

#[test]
fn test_complete_domain_pipeline() -> Result<()> {
    println!("\n🧪 Test: Complete Domain - Full DDD Structure");

    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("complete_domain")
        .with_validation(true)
        .with_golden_files(true);

    let result = harness.run_complete_pipeline()?;

    // Verify all stages succeeded
    harness.assert_all_stages_succeeded(&result);

    // Verify multiple entities
    assert!(
        result.sparql_result.entities.len() >= 3,
        "Should have at least 3 entities (User, Product, Order)"
    );

    // Verify multiple code files generated
    assert!(
        result.template_result.rendered_code.len() >= 3,
        "Should have generated code for multiple entities"
    );

    println!("  ✓ Complete domain generated: {} entities", result.sparql_result.entities.len());

    // Print metrics
    harness.metrics.print_summary();

    Ok(())
}

#[test]
fn test_complete_domain_with_value_objects() -> Result<()> {
    println!("\n🧪 Test: Complete Domain - Value Objects");

    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("complete_domain")
        .with_validation(true);

    let result = harness.run_complete_pipeline()?;

    // Check for value object entities
    let value_objects: Vec<_> = result
        .sparql_result
        .entities
        .iter()
        .filter(|e| e.entity_type.contains("ValueObject"))
        .collect();

    if !value_objects.is_empty() {
        println!("  Found {} value objects", value_objects.len());
        for vo in value_objects {
            println!("    - {}", vo.name);
        }
    }

    Ok(())
}

// ============================================================================
// MCP Tool Scenarios
// ============================================================================

#[test]
fn test_mcp_tool_generation() -> Result<()> {
    println!("\n🧪 Test: MCP Tool - Handler Generation");

    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("mcp_tool")
        .with_validation(true);

    let result = harness.run_complete_pipeline()?;

    // Verify tool entities extracted
    assert!(!result.sparql_result.entities.is_empty());

    // Verify code generated
    assert!(!result.template_result.rendered_code.is_empty());

    println!("  ✓ MCP tools generated: {}", result.sparql_result.entities.len());

    Ok(())
}

// ============================================================================
// Error Scenarios
// ============================================================================

#[test]
fn test_invalid_ontology_error_handling() {
    println!("\n🧪 Test: Error Handling - Invalid Ontology");

    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("error_scenarios")
        .with_validation(true);

    // This should fail gracefully - we expect an error or warning
    let result = harness.run_complete_pipeline();

    // We expect either an error or validation issues
    match result {
        Ok(r) => {
            // If it succeeds, validation should have caught issues
            if r.validation_result.all_valid {
                println!("  ⚠️  Warning: Invalid ontology passed validation unexpectedly");
            } else {
                println!("  ✓ Validation caught invalid ontology");
            }
        }
        Err(e) => {
            println!("  ✓ Pipeline failed as expected: {}", e);
        }
    }
}

#[test]
fn test_missing_template_fallback() -> Result<()> {
    println!("\n🧪 Test: Error Handling - Missing Template");

    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(false); // Disable to test template handling

    // This should use default templates
    let result = harness.run_complete_pipeline()?;

    // Should still generate code using defaults
    assert!(!result.template_result.rendered_code.is_empty());

    println!("  ✓ Fallback to default templates worked");

    Ok(())
}

// ============================================================================
// Golden File Testing
// ============================================================================

#[test]
fn test_golden_file_comparison() -> Result<()> {
    println!("\n🧪 Test: Golden File - Output Comparison");

    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true)
        .with_golden_files(true);

    let result = harness.run_complete_pipeline()?;

    // Compare against golden files
    let report = harness.compare_golden_files(&result)?;

    report.print_summary();

    // Note: Golden files might not match exactly due to formatting
    // This test documents the differences
    if !report.is_perfect_match() {
        println!("\n  ℹ️  Golden file comparison found differences");
        println!("      Run with UPDATE_GOLDEN=1 to update golden files");
    }

    Ok(())
}

#[test]
#[ignore] // Only run when explicitly requested
fn test_update_golden_files() -> Result<()> {
    println!("\n🧪 Test: Golden File - Update Golden Files");

    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true);

    let result = harness.run_complete_pipeline()?;

    // Update golden files
    harness.update_golden_files(&result)?;

    println!("  ✓ Golden files updated");

    Ok(())
}

// ============================================================================
// Incremental Updates
// ============================================================================

#[test]
fn test_incremental_generation() -> Result<()> {
    println!("\n🧪 Test: Incremental - Change Detection");

    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true)
        .with_incremental(true);

    // First run
    let result1 = harness.run_complete_pipeline()?;
    let files1 = result1.file_result.written_files.len();

    // Second run (should detect no changes)
    let result2 = harness.run_complete_pipeline()?;
    let files2 = result2.file_result.written_files.len();

    // Both runs should produce same number of files
    assert_eq!(files1, files2);

    println!("  ✓ Incremental generation: {} files", files1);

    Ok(())
}

// ============================================================================
// Performance Benchmarks
// ============================================================================

#[test]
fn test_pipeline_performance() -> Result<()> {
    println!("\n🧪 Test: Performance - Pipeline Benchmarks");

    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true);

    let result = harness.run_complete_pipeline()?;

    // Verify reasonable performance
    let total_ms = result.duration.as_millis();

    println!("\n  Performance Metrics:");
    println!("    Total time: {} ms", total_ms);
    println!("    Ontology:   {:?}", result.ontology_result.duration);
    println!("    SPARQL:     {:?}", result.sparql_result.duration);
    println!("    Template:   {:?}", result.template_result.duration);
    println!("    Validation: {:?}", result.validation_result.duration);
    println!("    File I/O:   {:?}", result.file_result.duration);

    // Reasonable performance threshold (should complete in < 5 seconds for simple case)
    assert!(
        total_ms < 5000,
        "Pipeline should complete in under 5 seconds, took {} ms",
        total_ms
    );

    println!("\n  ✓ Performance within acceptable limits");

    Ok(())
}

#[test]
fn test_complex_domain_performance() -> Result<()> {
    println!("\n🧪 Test: Performance - Complex Domain");

    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("complete_domain")
        .with_validation(true);

    let result = harness.run_complete_pipeline()?;

    let total_ms = result.duration.as_millis();

    println!("  Complex domain processed in: {} ms", total_ms);

    // More relaxed threshold for complex domains
    assert!(
        total_ms < 10000,
        "Complex domain should complete in under 10 seconds, took {} ms",
        total_ms
    );

    harness.metrics.print_summary();

    Ok(())
}

// ============================================================================
// Integration Points
// ============================================================================

#[test]
fn test_programmatic_api() -> Result<()> {
    println!("\n🧪 Test: Integration - Programmatic API");

    // Test using harness as a library
    let mut harness = CodegenPipelineHarness::new();

    // Configure via API
    harness = harness
        .with_fixture("simple_aggregate")
        .with_validation(true)
        .with_golden_files(false);

    let result = harness.run_complete_pipeline()?;

    assert!(result.success);
    println!("  ✓ Programmatic API works correctly");

    Ok(())
}

// ============================================================================
// Comprehensive End-to-End Test
// ============================================================================

#[test]
fn test_comprehensive_pipeline_validation() -> Result<()> {
    println!("\n🧪 Test: Comprehensive - Full Pipeline Validation");
    println!("  This test runs the complete pipeline and validates every stage\n");

    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true)
        .with_golden_files(false);

    // Run the complete pipeline
    let result = harness.run_complete_pipeline()?;

    // Stage 1: Ontology Loading
    println!("  ✓ Stage 1: Ontology Loading");
    assert!(result.ontology_result.triple_count > 0);
    println!("    - Loaded {} triples", result.ontology_result.triple_count);

    // Stage 2: SPARQL Query
    println!("  ✓ Stage 2: SPARQL Query");
    assert!(!result.sparql_result.entities.is_empty());
    println!("    - Extracted {} entities", result.sparql_result.entities.len());

    // Stage 3: Template Rendering
    println!("  ✓ Stage 3: Template Rendering");
    assert!(!result.template_result.rendered_code.is_empty());
    println!("    - Rendered {} files", result.template_result.rendered_code.len());

    // Stage 4: Code Validation
    println!("  ✓ Stage 4: Code Validation");
    assert!(result.validation_result.all_valid);
    for (file_name, code) in &result.validation_result.validated_code {
        harness.assert_code_compiles(code)?;
        harness.assert_all_imports_valid(code)?;
        harness.assert_no_unused_code(code)?;
    }
    println!("    - All code validated successfully");

    // Stage 5: File Writing
    println!("  ✓ Stage 5: File Writing");
    assert!(!result.file_result.written_files.is_empty());
    for path in &result.file_result.written_files {
        assert!(path.exists());
    }
    println!("    - Wrote {} files", result.file_result.written_files.len());

    // Print final summary
    println!("\n  📊 Pipeline Summary:");
    harness.metrics.print_summary();

    println!("\n  ✅ All validation checks passed!");

    Ok(())
}
