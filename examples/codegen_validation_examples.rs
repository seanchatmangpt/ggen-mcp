//! Code Generation Validation Examples
//!
//! This file contains practical examples of using the code generation validation system.
//! These examples can be run as integration tests or adapted for production use.

#![allow(dead_code)]

use anyhow::Result;
use spreadsheet_mcp::codegen::{
    ArtifactTracker, CodeGenPipeline, GeneratedCodeValidator, GenerationReceipt, SafeCodeWriter,
    ValidationSeverity, compute_file_hash, compute_string_hash,
};
use std::path::{Path, PathBuf};

// =============================================================================
// Example 1: Basic Code Validation
// =============================================================================

/// Validate a simple Rust struct
fn example_basic_validation() -> Result<()> {
    let mut validator = GeneratedCodeValidator::new();

    let code = r#"
        /// A simple data structure
        pub struct User {
            pub id: u64,
            pub name: String,
            pub email: String,
        }

        impl User {
            /// Create a new user
            pub fn new(id: u64, name: String, email: String) -> Self {
                Self { id, name, email }
            }
        }
    "#;

    let report = validator.validate_code(code, "user.rs")?;

    println!("Validation Results:");
    println!("  Errors: {}", report.error_count);
    println!("  Warnings: {}", report.warning_count);
    println!("  Info: {}", report.info_count);

    for issue in &report.issues {
        match issue.severity {
            ValidationSeverity::Error => println!("❌ ERROR: {}", issue.message),
            ValidationSeverity::Warning => println!("⚠️  WARNING: {}", issue.message),
            ValidationSeverity::Info => println!("ℹ️  INFO: {}", issue.message),
        }
    }

    Ok(())
}

// =============================================================================
// Example 2: Detecting Common Issues
// =============================================================================

/// Demonstrate validation of code with common issues
fn example_detect_issues() -> Result<()> {
    let mut validator = GeneratedCodeValidator::new();
    validator.require_doc_comments = true;
    validator.allow_unsafe = false;

    // Code with various issues
    let problematic_code = r#"
        // Missing doc comment
        pub struct my_struct {  // Wrong naming: should be PascalCase
            pub field: String,
        }

        pub fn BadFunction() {  // Wrong naming: should be snake_case
            unsafe {  // Unsafe code detected
                // Dangerous operation
            }
        }

        pub struct MyStruct {}  // Duplicate definition
        pub struct MyStruct {}  // Duplicate!
    "#;

    let report = validator.validate_code(problematic_code, "bad.rs")?;

    println!("\n=== Issues Detected ===");
    for issue in &report.issues {
        println!("\n{:?}: {}", issue.severity, issue.message);
        if let Some(location) = &issue.location {
            println!("  Location: {}", location);
        }
        if let Some(suggestion) = &issue.suggestion {
            println!("  Suggestion: {}", suggestion);
        }
    }

    Ok(())
}

// =============================================================================
// Example 3: Full Generation Pipeline
// =============================================================================

/// Demonstrate the complete code generation pipeline
fn example_full_pipeline(output_dir: &Path) -> Result<()> {
    // Setup pipeline
    let mut pipeline = CodeGenPipeline::new();
    pipeline.run_rustfmt = true;
    pipeline.run_clippy = false;
    pipeline.run_compile_check = false;

    // Template and rendered code
    let template = r#"
        /// {{ description }}
        pub struct {{ name }} {
            {% for field in fields %}
            pub {{ field.name }}: {{ field.type }},
            {% endfor %}
        }
    "#;

    let rendered = r#"
        /// A user entity
        pub struct User {
            pub id: u64,
            pub name: String,
            pub email: String,
        }
    "#;

    let output_path = output_dir.join("user.rs");

    // Execute pipeline
    println!("\n=== Running Generation Pipeline ===");
    let result = pipeline.execute(template, rendered, &output_path)?;

    if result.success {
        println!("✅ Generation successful!");

        if let Some(formatted) = &result.formatted_code {
            println!("\nFormatted code length: {} bytes", formatted.len());
        }

        if let Some(report) = &result.validation_report {
            println!(
                "Validation: {} errors, {} warnings",
                report.error_count, report.warning_count
            );
        }
    } else {
        println!("❌ Generation failed!");
        for error in &result.errors {
            println!("  Error: {}", error);
        }
    }

    Ok(())
}

// =============================================================================
// Example 4: Safe File Writing with Rollback
// =============================================================================

/// Demonstrate safe file writing with backup and rollback
fn example_safe_writing(output_dir: &Path) -> Result<()> {
    let file_path = output_dir.join("config.rs");
    let writer = SafeCodeWriter::new();

    // Original content
    let original_content = r#"
        pub struct Config {
            pub version: String,
        }
    "#;

    println!("\n=== Safe File Writing ===");

    // Write original
    println!("Writing original file...");
    writer.write(&file_path, original_content)?;
    println!("✅ Original written to {:?}", file_path);

    // Backup is created automatically
    let backup_path = file_path.with_extension("bak");

    // Update content
    let updated_content = r#"
        pub struct Config {
            pub version: String,
            pub debug: bool,
        }
    "#;

    println!("Updating file...");
    writer.write(&file_path, updated_content)?;
    println!("✅ File updated (backup at {:?})", backup_path);

    // Simulate error and rollback
    println!("Simulating error, rolling back...");
    writer.rollback(&file_path)?;
    println!("✅ Rolled back to original");

    // Verify rollback
    let current = std::fs::read_to_string(&file_path)?;
    assert_eq!(current.trim(), original_content.trim());
    println!("✅ Rollback verified");

    Ok(())
}

// =============================================================================
// Example 5: Artifact Tracking and Incremental Regeneration
// =============================================================================

/// Demonstrate artifact tracking for incremental regeneration
fn example_artifact_tracking(output_dir: &Path) -> Result<()> {
    let state_file = output_dir.join("artifacts.json");
    let artifact_path = output_dir.join("entity.rs");

    println!("\n=== Artifact Tracking ===");

    // Create tracker
    let mut tracker = ArtifactTracker::new(state_file.clone());

    // Simulate first generation
    let ontology_content = r#"
        @prefix ggen: <https://ggen-mcp.dev/domain#> .
        ggen:User a ddd:AggregateRoot .
    "#;
    let template_content = "pub struct {{ name }} {}";

    let ontology_hash = compute_string_hash(ontology_content);
    let template_hash = compute_string_hash(template_content);

    // Write artifact
    let code = "pub struct User {}";
    std::fs::write(&artifact_path, code)?;

    // Record artifact
    let artifact_hash = compute_file_hash(&artifact_path)?;
    tracker.record_artifact(
        artifact_path.clone(),
        ontology_hash.clone(),
        template_hash.clone(),
        vec![],
    )?;
    tracker.save()?;
    println!("✅ Artifact tracked: {:?}", artifact_path);

    // Check if stale (should not be)
    let is_stale = tracker.is_stale(&artifact_path, &ontology_hash, &template_hash);
    println!("Is stale? {}", is_stale);
    assert!(!is_stale);

    // Modify ontology
    let new_ontology = r#"
        @prefix ggen: <https://ggen-mcp.dev/domain#> .
        ggen:User a ddd:AggregateRoot .
        ggen:hasName "string" .
    "#;
    let new_ontology_hash = compute_string_hash(new_ontology);

    // Check if stale (should be now)
    let is_stale = tracker.is_stale(&artifact_path, &new_ontology_hash, &template_hash);
    println!("After ontology change, is stale? {}", is_stale);
    assert!(is_stale);

    // Get all stale artifacts
    let stale = tracker.get_stale_artifacts(&new_ontology_hash);
    println!("Stale artifacts: {} files", stale.len());

    Ok(())
}

// =============================================================================
// Example 6: Generation Receipts for Provenance
// =============================================================================

/// Demonstrate generation receipts for provenance tracking
fn example_receipts(output_dir: &Path) -> Result<()> {
    let receipt_path = output_dir.join("user.receipt.json");

    println!("\n=== Generation Receipts ===");

    // Create receipt
    let ontology_hash = compute_string_hash("ontology content");
    let template_hash = compute_string_hash("template content");
    let artifact_hash = compute_string_hash("generated code");

    let mut receipt = GenerationReceipt::new(
        ontology_hash.clone(),
        template_hash.clone(),
        artifact_hash.clone(),
    );

    // Add metadata
    receipt.add_metadata("generator".to_string(), "ggen-mcp".to_string());
    receipt.add_metadata("version".to_string(), "0.1.0".to_string());
    receipt.add_metadata("template".to_string(), "templates/user.rs.tera".to_string());

    println!("Receipt ID: {}", receipt.receipt_id);
    println!("Timestamp: {}", receipt.timestamp);

    // Verify integrity
    assert!(receipt.verify());
    println!("✅ Receipt integrity verified");

    // Save receipt
    receipt.save(&receipt_path)?;
    println!("✅ Receipt saved to {:?}", receipt_path);

    // Load and verify
    let loaded = GenerationReceipt::load(&receipt_path)?;
    assert!(loaded.verify());
    println!("✅ Loaded receipt verified");

    // Check reproducibility
    let reproducible = loaded.is_reproducible(&artifact_hash);
    println!("Is reproducible? {}", reproducible);
    assert!(reproducible);

    // Simulate modification
    let modified_hash = compute_string_hash("modified code");
    let still_reproducible = loaded.is_reproducible(&modified_hash);
    println!(
        "After modification, is reproducible? {}",
        still_reproducible
    );
    assert!(!still_reproducible);

    Ok(())
}

// =============================================================================
// Example 7: Complete Workflow with Error Handling
// =============================================================================

/// Demonstrate complete workflow with proper error handling
fn example_complete_workflow(output_dir: &Path) -> Result<()> {
    println!("\n=== Complete Workflow ===");

    let artifact_path = output_dir.join("product.rs");
    let state_file = output_dir.join("tracker.json");
    let receipt_dir = output_dir.join("receipts");
    std::fs::create_dir_all(&receipt_dir)?;

    // 1. Load tracker
    let mut tracker = ArtifactTracker::load(state_file.clone())?;
    println!("✅ Tracker loaded");

    // 2. Compute hashes
    let ontology_content = std::fs::read_to_string("ontology/mcp-domain.ttl")
        .unwrap_or_else(|_| "mock ontology".to_string());
    let template_content = "pub struct {{ name }} { pub id: u64 }";

    let ontology_hash = compute_string_hash(&ontology_content);
    let template_hash = compute_string_hash(template_content);
    println!("✅ Hashes computed");

    // 3. Check if regeneration needed
    if !tracker.is_stale(&artifact_path, &ontology_hash, &template_hash) {
        println!("⏭️  Artifact up-to-date, skipping generation");
        return Ok(());
    }
    println!("🔄 Artifact stale, regenerating...");

    // 4. Generate through pipeline
    let mut pipeline = CodeGenPipeline::new();
    pipeline.run_rustfmt = true;

    let rendered = "pub struct Product { pub id: u64 }";
    let result = pipeline.execute(template_content, rendered, &artifact_path)?;

    if !result.success {
        anyhow::bail!("Generation failed: {:?}", result.errors);
    }
    println!("✅ Code generated and validated");

    // 5. Write safely
    let writer = SafeCodeWriter::new();
    let code = result
        .formatted_code
        .as_ref()
        .unwrap_or(&rendered.to_string());

    match writer.write(&artifact_path, code) {
        Ok(_) => println!("✅ Code written to {:?}", artifact_path),
        Err(e) => {
            eprintln!("❌ Write failed: {}", e);
            // Rollback would happen here if needed
            return Err(e);
        }
    }

    // 6. Track artifact
    let artifact_hash = compute_file_hash(&artifact_path)?;
    tracker.record_artifact(
        artifact_path.clone(),
        ontology_hash.clone(),
        template_hash.clone(),
        vec![],
    )?;
    tracker.save()?;
    println!("✅ Artifact tracked");

    // 7. Create receipt
    let mut receipt = GenerationReceipt::new(ontology_hash, template_hash, artifact_hash);
    receipt.add_metadata("generator".to_string(), "example".to_string());

    let receipt_path = receipt_dir.join("product.json");
    receipt.save(&receipt_path)?;
    println!("✅ Receipt saved to {:?}", receipt_path);

    println!("\n🎉 Complete workflow finished successfully!");

    Ok(())
}

// =============================================================================
// Example 8: Batch Processing Multiple Artifacts
// =============================================================================

/// Process multiple artifacts in batch
fn example_batch_processing(output_dir: &Path) -> Result<()> {
    println!("\n=== Batch Processing ===");

    let state_file = output_dir.join("tracker.json");
    let tracker = ArtifactTracker::load(state_file)?;

    // Define artifacts to generate
    let artifacts = vec![
        ("user.rs", "pub struct User {}"),
        ("product.rs", "pub struct Product {}"),
        ("order.rs", "pub struct Order {}"),
    ];

    let ontology_hash = compute_string_hash("ontology");
    let stale_artifacts = tracker.get_stale_artifacts(&ontology_hash);

    println!("Total artifacts: {}", artifacts.len());
    println!("Stale artifacts: {}", stale_artifacts.len());

    // Only regenerate stale artifacts
    for (name, code) in artifacts {
        let path = output_dir.join(name);

        if stale_artifacts.contains(&path) {
            println!("🔄 Regenerating {}", name);
            std::fs::write(&path, code)?;
        } else {
            println!("⏭️  Skipping {} (up-to-date)", name);
        }
    }

    Ok(())
}

// =============================================================================
// Example 9: Custom Validation Rules
// =============================================================================

/// Demonstrate custom validation configuration
fn example_custom_validation() -> Result<()> {
    println!("\n=== Custom Validation Rules ===");

    // Strict mode: no unsafe, require docs, short lines
    let mut strict_validator = GeneratedCodeValidator::new();
    strict_validator.allow_unsafe = false;
    strict_validator.require_doc_comments = true;
    strict_validator.max_line_length = 80;

    // Permissive mode: allow unsafe, skip docs, long lines ok
    let mut permissive_validator = GeneratedCodeValidator::new();
    permissive_validator.allow_unsafe = true;
    permissive_validator.require_doc_comments = false;
    permissive_validator.max_line_length = 200;

    let code = r#"
        pub struct Test {
            pub field: String,
        }

        pub fn test() {
            unsafe { /* operation */ }
        }
    "#;

    println!("Strict validation:");
    let strict_report = strict_validator.validate_code(code, "test.rs")?;
    println!("  Errors: {}", strict_report.error_count);
    println!("  Warnings: {}", strict_report.warning_count);

    println!("\nPermissive validation:");
    let permissive_report = permissive_validator.validate_code(code, "test.rs")?;
    println!("  Errors: {}", permissive_report.error_count);
    println!("  Warnings: {}", permissive_report.warning_count);

    Ok(())
}

// =============================================================================
// Main Function (for testing examples)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_all_examples() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let output_dir = temp_dir.path();

        println!("\n{'='*60}");
        println!("Running Code Generation Validation Examples");
        println!("{'='*60}");

        example_basic_validation()?;
        example_detect_issues()?;
        example_full_pipeline(output_dir)?;
        example_safe_writing(output_dir)?;
        example_artifact_tracking(output_dir)?;
        example_receipts(output_dir)?;
        example_complete_workflow(output_dir)?;
        example_batch_processing(output_dir)?;
        example_custom_validation()?;

        println!("\n{'='*60}");
        println!("All examples completed successfully!");
        println!("{'='*60}\n");

        Ok(())
    }
}
