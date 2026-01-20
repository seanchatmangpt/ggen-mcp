//! Comprehensive tests for code generation validation
//!
//! Tests cover all components of the validation system:
//! - GeneratedCodeValidator
//! - CodeGenPipeline
//! - ArtifactTracker
//! - GenerationReceipt
//! - SafeCodeWriter
//!
//! Test Methodology: Chicago TDD with real collaborators

use anyhow::Result;
use spreadsheet_mcp::codegen::{
    ArtifactTracker, CodeGenPipeline, GeneratedCodeValidator, GenerationReceipt, SafeCodeWriter,
    ValidationSeverity, compute_string_hash,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// =============================================================================
// Helper Functions
// =============================================================================

fn setup_temp_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp directory")
}

fn temp_file_path(dir: &TempDir, name: &str) -> PathBuf {
    dir.path().join(name)
}

// =============================================================================
// GeneratedCodeValidator Tests
// =============================================================================

#[test]
fn test_validator_accepts_valid_rust_code() {
    // Arrange
    let mut validator = GeneratedCodeValidator::new();
    let valid_code = r#"
        pub struct MyStruct {
            pub field: String,
        }

        impl MyStruct {
            pub fn new(field: String) -> Self {
                Self { field }
            }
        }
    "#;

    // Act
    let result = validator.validate_code(valid_code, "test.rs");

    // Assert
    assert!(result.is_ok());
    let report = result.unwrap();
    assert!(!report.has_errors());
}

#[test]
fn test_validator_rejects_invalid_syntax() {
    // Arrange
    let mut validator = GeneratedCodeValidator::new();
    let invalid_code = r#"
        pub struct MyStruct {
            pub field: String,
        // Missing closing brace
    "#;

    // Act
    let result = validator.validate_code(invalid_code, "test.rs");

    // Assert
    assert!(result.is_err());
}

#[test]
fn test_validator_detects_unsafe_code() {
    // Arrange
    let mut validator = GeneratedCodeValidator::new();
    validator.allow_unsafe = false;

    let code_with_unsafe = r#"
        pub struct MyStruct {
            pub field: String,
        }

        impl MyStruct {
            pub fn dangerous(&self) {
                unsafe {
                    // Unsafe operation
                }
            }
        }
    "#;

    // Act
    let result = validator.validate_code(code_with_unsafe, "test.rs");

    // Assert
    assert!(result.is_ok());
    let report = result.unwrap();
    assert!(report.has_errors());
    assert!(report.issues.iter().any(|i| i.message.contains("Unsafe")));
}

#[test]
fn test_validator_detects_naming_convention_violations() {
    // Arrange
    let mut validator = GeneratedCodeValidator::new();
    let bad_naming = r#"
        pub struct my_struct {
            pub field: String,
        }

        impl my_struct {
            pub fn BadFunction() {}
        }
    "#;

    // Act
    let result = validator.validate_code(bad_naming, "test.rs");

    // Assert
    assert!(result.is_ok());
    let report = result.unwrap();
    assert!(report.warning_count > 0);
    assert!(report.issues.iter().any(|i| i.message.contains("PascalCase") || i.message.contains("snake_case")));
}

#[test]
fn test_validator_detects_duplicate_definitions() {
    // Arrange
    let mut validator = GeneratedCodeValidator::new();
    let duplicate_structs = r#"
        pub struct MyStruct {
            pub field1: String,
        }

        pub struct MyStruct {
            pub field2: String,
        }
    "#;

    // Act
    let result = validator.validate_code(duplicate_structs, "test.rs");

    // Assert
    assert!(result.is_ok());
    let report = result.unwrap();
    assert!(report.has_errors());
    assert!(report.issues.iter().any(|i| i.message.contains("Duplicate")));
}

#[test]
fn test_validator_detects_long_lines() {
    // Arrange
    let mut validator = GeneratedCodeValidator::new();
    validator.max_line_length = 80;

    let long_line = format!(
        "pub fn very_long_function_name_that_exceeds_the_maximum_line_length_limit() -> Result<String, Error> {{ Ok(String::from(\"test\")) }}"
    );

    // Act
    let result = validator.validate_code(&long_line, "test.rs");

    // Assert
    assert!(result.is_ok());
    let report = result.unwrap();
    assert!(report.warning_count > 0);
}

#[test]
fn test_validator_detects_missing_documentation() {
    // Arrange
    let mut validator = GeneratedCodeValidator::new();
    validator.require_doc_comments = true;

    let undocumented_code = r#"
        pub struct MyStruct {
            pub field: String,
        }

        pub fn my_function() {}
    "#;

    // Act
    let result = validator.validate_code(undocumented_code, "test.rs");

    // Assert
    assert!(result.is_ok());
    let report = result.unwrap();
    assert!(report.warning_count > 0);
    assert!(report.issues.iter().any(|i| i.message.contains("documentation")));
}

#[test]
fn test_validator_accepts_documented_code() {
    // Arrange
    let mut validator = GeneratedCodeValidator::new();
    validator.require_doc_comments = true;

    let documented_code = r#"
        /// My struct documentation
        pub struct MyStruct {
            pub field: String,
        }

        /// My function documentation
        pub fn my_function() {}
    "#;

    // Act
    let result = validator.validate_code(documented_code, "test.rs");

    // Assert
    assert!(result.is_ok());
    let report = result.unwrap();
    // Should have fewer warnings than undocumented code
    assert!(report.warning_count == 0 || !report.issues.iter().any(|i| i.message.contains("documentation")));
}

#[test]
fn test_validator_reset_clears_tracking() {
    // Arrange
    let mut validator = GeneratedCodeValidator::new();
    let code = r#"
        pub struct MyStruct {
            pub field: String,
        }
    "#;

    // Act - First validation
    let _ = validator.validate_code(code, "test1.rs");

    // Reset
    validator.reset();

    // Second validation with same code
    let result = validator.validate_code(code, "test2.rs");

    // Assert - Should not detect duplicate since we reset
    assert!(result.is_ok());
    let report = result.unwrap();
    assert!(!report.issues.iter().any(|i| i.message.contains("Duplicate")));
}

// =============================================================================
// CodeGenPipeline Tests
// =============================================================================

#[test]
fn test_pipeline_executes_successfully() {
    // Arrange
    let temp_dir = setup_temp_dir();
    let output_path = temp_file_path(&temp_dir, "output.rs");

    let mut pipeline = CodeGenPipeline::new();
    pipeline.run_rustfmt = false; // Disable for testing
    pipeline.run_clippy = false;
    pipeline.run_compile_check = false;

    let template = r#"pub struct {{ name }} { pub field: String }"#;
    let rendered = r#"pub struct MyStruct { pub field: String }"#;

    // Act
    let result = pipeline.execute(template, rendered, &output_path);

    // Assert
    assert!(result.is_ok());
    let gen_result = result.unwrap();
    assert!(gen_result.success);
    assert_eq!(gen_result.errors.len(), 0);
}

#[test]
fn test_pipeline_detects_template_errors() {
    // Arrange
    let temp_dir = setup_temp_dir();
    let output_path = temp_file_path(&temp_dir, "output.rs");

    let mut pipeline = CodeGenPipeline::new();
    pipeline.run_rustfmt = false;

    let bad_template = r#"pub struct {{ name } { pub field: String }"#; // Unbalanced braces
    let rendered = r#"pub struct MyStruct { pub field: String }"#;

    // Act
    let result = pipeline.execute(bad_template, rendered, &output_path);

    // Assert
    assert!(result.is_err());
}

#[test]
fn test_pipeline_detects_code_errors() {
    // Arrange
    let temp_dir = setup_temp_dir();
    let output_path = temp_file_path(&temp_dir, "output.rs");

    let mut pipeline = CodeGenPipeline::new();
    pipeline.run_rustfmt = false;

    let template = r#"pub struct {{ name }} { pub field: String }"#;
    let bad_rendered = r#"pub struct MyStruct { pub field: String"#; // Missing brace

    // Act
    let result = pipeline.execute(template, bad_rendered, &output_path);

    // Assert
    assert!(result.is_err());
}

// =============================================================================
// ArtifactTracker Tests
// =============================================================================

#[test]
fn test_tracker_saves_and_loads_state() -> Result<()> {
    // Arrange
    let temp_dir = setup_temp_dir();
    let state_file = temp_file_path(&temp_dir, "tracker.json");

    let mut tracker = ArtifactTracker::new(state_file.clone());
    let artifact_path = PathBuf::from("/test/artifact.rs");

    // Act - Save
    tracker.record_artifact(
        artifact_path.clone(),
        "ontology_hash".to_string(),
        "template_hash".to_string(),
        vec![],
    )?;
    tracker.save()?;

    // Load
    let loaded_tracker = ArtifactTracker::load(state_file)?;

    // Assert
    assert!(loaded_tracker.artifacts.contains_key(&artifact_path));
    let metadata = &loaded_tracker.artifacts[&artifact_path];
    assert_eq!(metadata.ontology_hash, "ontology_hash");
    assert_eq!(metadata.template_hash, "template_hash");

    Ok(())
}

#[test]
fn test_tracker_detects_stale_artifacts() -> Result<()> {
    // Arrange
    let temp_dir = setup_temp_dir();
    let state_file = temp_file_path(&temp_dir, "tracker.json");
    let artifact_path = temp_file_path(&temp_dir, "artifact.rs");

    // Create artifact file
    fs::write(&artifact_path, "pub struct Test {}")?;

    let mut tracker = ArtifactTracker::new(state_file);
    tracker.record_artifact(
        artifact_path.clone(),
        "old_hash".to_string(),
        "template_hash".to_string(),
        vec![],
    )?;

    // Act & Assert - Different ontology hash should be stale
    assert!(tracker.is_stale(&artifact_path, "new_hash", "template_hash"));

    // Same hashes should not be stale
    assert!(!tracker.is_stale(&artifact_path, "old_hash", "template_hash"));

    Ok(())
}

#[test]
fn test_tracker_detects_missing_files_as_stale() -> Result<()> {
    // Arrange
    let temp_dir = setup_temp_dir();
    let state_file = temp_file_path(&temp_dir, "tracker.json");
    let artifact_path = temp_file_path(&temp_dir, "nonexistent.rs");

    let mut tracker = ArtifactTracker::new(state_file);
    tracker.record_artifact(
        artifact_path.clone(),
        "hash".to_string(),
        "template_hash".to_string(),
        vec![],
    )?;

    // Act & Assert - Missing file should be stale
    assert!(tracker.is_stale(&artifact_path, "hash", "template_hash"));

    Ok(())
}

#[test]
fn test_tracker_finds_stale_artifacts() -> Result<()> {
    // Arrange
    let temp_dir = setup_temp_dir();
    let state_file = temp_file_path(&temp_dir, "tracker.json");

    let mut tracker = ArtifactTracker::new(state_file);

    let path1 = PathBuf::from("/test/artifact1.rs");
    let path2 = PathBuf::from("/test/artifact2.rs");

    tracker.record_artifact(path1.clone(), "old_hash".to_string(), "t1".to_string(), vec![])?;
    tracker.record_artifact(path2.clone(), "old_hash".to_string(), "t2".to_string(), vec![])?;

    // Act
    let stale = tracker.get_stale_artifacts("new_hash");

    // Assert
    assert_eq!(stale.len(), 2);
    assert!(stale.contains(&path1));
    assert!(stale.contains(&path2));

    Ok(())
}

#[test]
fn test_tracker_removes_artifacts() -> Result<()> {
    // Arrange
    let temp_dir = setup_temp_dir();
    let state_file = temp_file_path(&temp_dir, "tracker.json");

    let mut tracker = ArtifactTracker::new(state_file);
    let artifact_path = PathBuf::from("/test/artifact.rs");

    tracker.record_artifact(artifact_path.clone(), "hash".to_string(), "t".to_string(), vec![])?;

    // Act
    tracker.remove_artifact(&artifact_path);

    // Assert
    assert!(!tracker.artifacts.contains_key(&artifact_path));

    Ok(())
}

// =============================================================================
// GenerationReceipt Tests
// =============================================================================

#[test]
fn test_receipt_creation_is_deterministic() {
    // Arrange
    let ontology_hash = "ontology123".to_string();
    let template_hash = "template456".to_string();
    let artifact_hash = "artifact789".to_string();

    // Act
    let receipt1 = GenerationReceipt::new(
        ontology_hash.clone(),
        template_hash.clone(),
        artifact_hash.clone(),
    );
    let receipt2 = GenerationReceipt::new(ontology_hash, template_hash, artifact_hash);

    // Assert
    assert_eq!(receipt1.receipt_id, receipt2.receipt_id);
}

#[test]
fn test_receipt_verify_integrity() {
    // Arrange
    let receipt = GenerationReceipt::new(
        "ontology123".to_string(),
        "template456".to_string(),
        "artifact789".to_string(),
    );

    // Act & Assert
    assert!(receipt.verify());
}

#[test]
fn test_receipt_detects_tampering() {
    // Arrange
    let mut receipt = GenerationReceipt::new(
        "ontology123".to_string(),
        "template456".to_string(),
        "artifact789".to_string(),
    );

    // Act - Tamper with hash
    receipt.ontology_hash = "tampered".to_string();

    // Assert
    assert!(!receipt.verify());
}

#[test]
fn test_receipt_checks_reproducibility() {
    // Arrange
    let artifact_hash = "artifact789".to_string();
    let receipt = GenerationReceipt::new(
        "ontology123".to_string(),
        "template456".to_string(),
        artifact_hash.clone(),
    );

    // Act & Assert
    assert!(receipt.is_reproducible(&artifact_hash));
    assert!(!receipt.is_reproducible("different_hash"));
}

#[test]
fn test_receipt_saves_and_loads() -> Result<()> {
    // Arrange
    let temp_dir = setup_temp_dir();
    let receipt_path = temp_file_path(&temp_dir, "receipt.json");

    let mut receipt = GenerationReceipt::new(
        "ontology123".to_string(),
        "template456".to_string(),
        "artifact789".to_string(),
    );
    receipt.add_metadata("key".to_string(), "value".to_string());

    // Act - Save
    receipt.save(&receipt_path)?;

    // Load
    let loaded_receipt = GenerationReceipt::load(&receipt_path)?;

    // Assert
    assert_eq!(receipt.receipt_id, loaded_receipt.receipt_id);
    assert_eq!(receipt.ontology_hash, loaded_receipt.ontology_hash);
    assert_eq!(receipt.template_hash, loaded_receipt.template_hash);
    assert_eq!(receipt.artifact_hash, loaded_receipt.artifact_hash);
    assert!(loaded_receipt.generation_metadata.contains_key("key"));

    Ok(())
}

#[test]
fn test_receipt_metadata() {
    // Arrange
    let mut receipt = GenerationReceipt::new(
        "ontology123".to_string(),
        "template456".to_string(),
        "artifact789".to_string(),
    );

    // Act
    receipt.add_metadata("generator".to_string(), "ggen-mcp".to_string());
    receipt.add_metadata("version".to_string(), "0.1.0".to_string());

    // Assert
    assert_eq!(receipt.generation_metadata.get("generator"), Some(&"ggen-mcp".to_string()));
    assert_eq!(receipt.generation_metadata.get("version"), Some(&"0.1.0".to_string()));
}

// =============================================================================
// SafeCodeWriter Tests
// =============================================================================

#[test]
fn test_writer_creates_new_file() -> Result<()> {
    // Arrange
    let temp_dir = setup_temp_dir();
    let file_path = temp_file_path(&temp_dir, "new_file.rs");

    let writer = SafeCodeWriter::new();
    let content = "pub struct Test {}";

    // Act
    writer.write(&file_path, content)?;

    // Assert
    assert!(file_path.exists());
    let written_content = fs::read_to_string(&file_path)?;
    assert_eq!(written_content, content);

    Ok(())
}

#[test]
fn test_writer_overwrites_existing_file() -> Result<()> {
    // Arrange
    let temp_dir = setup_temp_dir();
    let file_path = temp_file_path(&temp_dir, "existing_file.rs");

    fs::write(&file_path, "old content")?;

    let writer = SafeCodeWriter::new();
    let new_content = "pub struct NewTest {}";

    // Act
    writer.write(&file_path, new_content)?;

    // Assert
    let written_content = fs::read_to_string(&file_path)?;
    assert_eq!(written_content, new_content);

    Ok(())
}

#[test]
fn test_writer_creates_backup() -> Result<()> {
    // Arrange
    let temp_dir = setup_temp_dir();
    let file_path = temp_file_path(&temp_dir, "file_with_backup.rs");

    let original_content = "original content";
    fs::write(&file_path, original_content)?;

    let writer = SafeCodeWriter::new();
    let new_content = "new content";

    // Act
    writer.write(&file_path, new_content)?;

    // Assert
    let backup_path = file_path.with_extension("bak");
    assert!(backup_path.exists());
    let backup_content = fs::read_to_string(&backup_path)?;
    assert_eq!(backup_content, original_content);

    Ok(())
}

#[test]
fn test_writer_rollback_restores_backup() -> Result<()> {
    // Arrange
    let temp_dir = setup_temp_dir();
    let file_path = temp_file_path(&temp_dir, "file_for_rollback.rs");

    let original_content = "original content";
    fs::write(&file_path, original_content)?;

    let writer = SafeCodeWriter::new();
    let new_content = "new content";

    writer.write(&file_path, new_content)?;

    // Act
    writer.rollback(&file_path)?;

    // Assert
    let restored_content = fs::read_to_string(&file_path)?;
    assert_eq!(restored_content, original_content);

    Ok(())
}

#[test]
fn test_writer_prevents_path_traversal() {
    // Arrange
    let writer = SafeCodeWriter::new();
    let malicious_path = PathBuf::from("../../../etc/passwd");
    let content = "malicious content";

    // Act
    let result = writer.write(&malicious_path, content);

    // Assert
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Path traversal"));
}

#[test]
fn test_writer_creates_parent_directories() -> Result<()> {
    // Arrange
    let temp_dir = setup_temp_dir();
    let nested_path = temp_dir.path().join("nested").join("dirs").join("file.rs");

    let writer = SafeCodeWriter::new();
    let content = "pub struct Test {}";

    // Act
    writer.write(&nested_path, content)?;

    // Assert
    assert!(nested_path.exists());
    let written_content = fs::read_to_string(&nested_path)?;
    assert_eq!(written_content, content);

    Ok(())
}

#[test]
fn test_writer_with_custom_backup_dir() -> Result<()> {
    // Arrange
    let temp_dir = setup_temp_dir();
    let backup_dir = temp_dir.path().join("backups");
    fs::create_dir_all(&backup_dir)?;

    let file_path = temp_file_path(&temp_dir, "file.rs");
    fs::write(&file_path, "original")?;

    let mut writer = SafeCodeWriter::new();
    writer.backup_dir = Some(backup_dir.clone());

    // Act
    writer.write(&file_path, "new content")?;

    // Assert
    let backup_path = backup_dir.join("file.rs.bak");
    assert!(backup_path.exists());

    Ok(())
}

// =============================================================================
// Integration Tests
// =============================================================================

#[test]
fn test_full_generation_workflow() -> Result<()> {
    // Arrange
    let temp_dir = setup_temp_dir();
    let output_path = temp_file_path(&temp_dir, "generated.rs");
    let state_file = temp_file_path(&temp_dir, "tracker.json");
    let receipt_path = temp_file_path(&temp_dir, "receipt.json");

    let mut pipeline = CodeGenPipeline::new();
    pipeline.run_rustfmt = false;

    let template = r#"pub struct {{ name }} { pub field: String }"#;
    let rendered = r#"pub struct MyStruct { pub field: String }"#;

    let ontology_hash = compute_string_hash("ontology content");
    let template_hash = compute_string_hash(template);

    // Act - Generate code
    let gen_result = pipeline.execute(template, rendered, &output_path)?;
    assert!(gen_result.success);

    // Write code
    let writer = SafeCodeWriter::new();
    writer.write(&output_path, rendered)?;

    // Track artifact
    let mut tracker = ArtifactTracker::new(state_file);
    let artifact_hash = spreadsheet_mcp::codegen::compute_file_hash(&output_path)?;
    tracker.record_artifact(
        output_path.clone(),
        ontology_hash.clone(),
        template_hash.clone(),
        vec![],
    )?;
    tracker.save()?;

    // Create receipt
    let receipt = GenerationReceipt::new(ontology_hash, template_hash, artifact_hash);
    receipt.save(&receipt_path)?;

    // Assert
    assert!(output_path.exists());
    assert!(receipt_path.exists());
    assert!(receipt.verify());

    Ok(())
}

#[test]
fn test_incremental_regeneration_workflow() -> Result<()> {
    // Arrange
    let temp_dir = setup_temp_dir();
    let output_path = temp_file_path(&temp_dir, "generated.rs");
    let state_file = temp_file_path(&temp_dir, "tracker.json");

    let ontology_hash = "hash123";
    let template_hash = "template456";

    // Initial generation
    fs::write(&output_path, "pub struct Test {}")?;

    let mut tracker = ArtifactTracker::new(state_file.clone());
    tracker.record_artifact(
        output_path.clone(),
        ontology_hash.to_string(),
        template_hash.to_string(),
        vec![],
    )?;
    tracker.save()?;

    // Act - Check if regeneration needed (same hashes)
    let loaded_tracker = ArtifactTracker::load(state_file)?;
    let needs_regen = loaded_tracker.is_stale(&output_path, ontology_hash, template_hash);

    // Assert
    assert!(!needs_regen, "Should not need regeneration with same hashes");

    // Act - Change ontology
    let new_ontology_hash = "new_hash123";
    let needs_regen_after_change = loaded_tracker.is_stale(&output_path, new_ontology_hash, template_hash);

    // Assert
    assert!(needs_regen_after_change, "Should need regeneration with new ontology hash");

    Ok(())
}

#[test]
fn test_error_recovery_with_rollback() -> Result<()> {
    // Arrange
    let temp_dir = setup_temp_dir();
    let file_path = temp_file_path(&temp_dir, "file.rs");

    let original_content = "pub struct Original {}";
    fs::write(&file_path, original_content)?;

    let writer = SafeCodeWriter::new();

    // Act - Write new content
    let new_content = "pub struct Updated {}";
    writer.write(&file_path, new_content)?;

    // Simulate error and rollback
    writer.rollback(&file_path)?;

    // Assert
    let restored_content = fs::read_to_string(&file_path)?;
    assert_eq!(restored_content, original_content);

    Ok(())
}
