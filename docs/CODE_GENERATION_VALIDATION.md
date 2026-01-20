# Code Generation Validation

Comprehensive validation system for the ggen-mcp template-based code generation system, implementing Toyota Production System's **Poka-Yoke** (error-proofing) principles.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Components](#components)
- [Generation Pipeline](#generation-pipeline)
- [Validation Rules](#validation-rules)
- [Error Handling](#error-handling)
- [Receipt Verification](#receipt-verification)
- [Incremental Regeneration](#incremental-regeneration)
- [Testing Generated Code](#testing-generated-code)
- [Troubleshooting](#troubleshooting)

## Overview

The code generation validation system ensures that all generated Rust code is:

1. **Syntactically valid** - Parses correctly with the Rust compiler
2. **Semantically correct** - Follows Rust conventions and best practices
3. **Traceable** - Full provenance from ontology to generated code
4. **Deterministic** - Same inputs always produce same outputs
5. **Safe** - Written atomically with rollback support

### Poka-Yoke Principles

The system implements error prevention at multiple levels:

- **Specification Closure**: SHACL validation ensures ontology is complete before generation
- **Input Validation**: Templates and SPARQL queries are validated before execution
- **Output Validation**: Generated code is validated before writing
- **Atomic Operations**: All file writes are atomic with automatic backups
- **Provenance Tracking**: Every generated file has a verifiable receipt

## Architecture

### Code Generation Workflow

```
┌─────────────────────────────────────────────────────────────────────┐
│                    ONTOLOGY (Turtle/RDF)                           │
│  - Domain model in RDF/OWL                                          │
│  - DDD patterns (Aggregates, Commands, Events)                     │
│  - MCP tools and resources                                          │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│               SHACL VALIDATION (Pre-Generation)                     │
│  - Validate ontology shapes                                         │
│  - Check required properties                                        │
│  - Verify relationships                                             │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                   SPARQL QUERIES                                    │
│  - Extract data from ontology                                       │
│  - queries/*.rq files                                               │
│  - SELECT queries for template context                              │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                TEMPLATE RENDERING (Tera)                            │
│  - templates/*.rs.tera files                                        │
│  - Context from SPARQL results                                      │
│  - Generate Rust source code                                        │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│              CODE VALIDATION (Post-Generation)                      │
│  - Syntax validation (syn crate)                                    │
│  - Naming conventions                                               │
│  - No duplicates                                                    │
│  - Documentation requirements                                       │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      FORMATTING                                     │
│  - rustfmt (Rust 2024 edition)                                      │
│  - Consistent style                                                 │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    SAFE FILE WRITING                                │
│  - Atomic writes (temp + rename)                                    │
│  - Automatic backups                                                │
│  - Rollback on failure                                              │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                   ARTIFACT TRACKING                                 │
│  - Record metadata                                                  │
│  - Track dependencies                                               │
│  - Enable incremental regeneration                                  │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                   RECEIPT GENERATION                                │
│  - Provenance information                                           │
│  - Input/output hashes                                              │
│  - Verification data                                                │
└─────────────────────────────────────────────────────────────────────┘
```

## Components

### 1. GeneratedCodeValidator

Validates generated Rust code for correctness and conventions.

#### Features

- **Syntax Validation**: Uses `syn` crate to parse Rust code
- **Naming Conventions**: Enforces PascalCase for types, snake_case for functions
- **Module Structure**: Validates use statements, item organization
- **Duplicate Detection**: Prevents duplicate struct/trait/function definitions
- **Unsafe Code**: Optionally prevents unsafe blocks
- **Line Length**: Configurable maximum line length
- **Documentation**: Optionally requires doc comments

#### Configuration

```rust
use spreadsheet_mcp::codegen::GeneratedCodeValidator;

let mut validator = GeneratedCodeValidator::new();
validator.allow_unsafe = false;          // Reject unsafe code
validator.require_doc_comments = true;   // Require /// comments
validator.max_line_length = 120;         // Max line length
```

#### Usage

```rust
let code = r#"
    pub struct MyStruct {
        pub field: String,
    }
"#;

let report = validator.validate_code(code, "my_struct.rs")?;

if report.has_errors() {
    for issue in &report.issues {
        eprintln!("{:?}: {}", issue.severity, issue.message);
    }
}
```

### 2. CodeGenPipeline

Orchestrates the complete generation pipeline with validation at each stage.

#### Pipeline Stages

1. **Pre-generation validation** - Validate template syntax
2. **Template rendering** - Render with Tera engine
3. **Post-generation validation** - Validate generated code
4. **Formatting** - Run rustfmt (optional)
5. **Linting** - Run clippy checks (optional)
6. **Compilation test** - Smoke test compilation (optional)

#### Configuration

```rust
use spreadsheet_mcp::codegen::CodeGenPipeline;

let mut pipeline = CodeGenPipeline::new();
pipeline.run_rustfmt = true;        // Format with rustfmt
pipeline.run_clippy = false;        // Skip clippy (use cargo clippy separately)
pipeline.run_compile_check = false; // Skip compile test
```

#### Usage

```rust
let template = r#"pub struct {{ name }} { pub field: String }"#;
let rendered = r#"pub struct MyStruct { pub field: String }"#;
let output_path = Path::new("generated/my_struct.rs");

let result = pipeline.execute(template, rendered, output_path)?;

if result.success {
    println!("Generation succeeded!");
    if let Some(formatted) = result.formatted_code {
        // Use formatted code
    }
} else {
    eprintln!("Errors: {:?}", result.errors);
}
```

### 3. ArtifactTracker

Tracks all generated artifacts and their metadata for incremental regeneration.

#### Tracked Metadata

- **Path**: File system path
- **Timestamp**: Generation time
- **Ontology Hash**: SHA-256 of source ontology
- **Template Hash**: SHA-256 of template file
- **Artifact Hash**: SHA-256 of generated code
- **Dependencies**: Related artifacts

#### Usage

```rust
use spreadsheet_mcp::codegen::ArtifactTracker;
use std::path::PathBuf;

// Create tracker
let state_file = PathBuf::from(".ggen/artifacts.json");
let mut tracker = ArtifactTracker::load(state_file)?;

// Record artifact
tracker.record_artifact(
    PathBuf::from("generated/my_struct.rs"),
    ontology_hash,
    template_hash,
    vec![], // dependencies
)?;

// Check if stale
if tracker.is_stale(&path, &current_ontology_hash, &current_template_hash) {
    println!("Artifact needs regeneration");
}

// Save state
tracker.save()?;
```

#### Stale Detection

An artifact is considered stale if:

1. Ontology hash changed
2. Template hash changed
3. File doesn't exist
4. File content hash doesn't match recorded hash

#### Incremental Regeneration

```rust
// Get all stale artifacts
let stale = tracker.get_stale_artifacts(&current_ontology_hash);

for path in stale {
    println!("Need to regenerate: {:?}", path);
    // Regenerate only these files
}
```

### 4. GenerationReceipt

Provides provenance and verification for generated code.

#### Receipt Contents

- **Receipt ID**: Deterministic hash of inputs/outputs
- **Ontology Hash**: SHA-256 of source ontology
- **Template Hash**: SHA-256 of template
- **Artifact Hash**: SHA-256 of generated code
- **Timestamp**: Generation time
- **Metadata**: Custom key-value pairs

#### Usage

```rust
use spreadsheet_mcp::codegen::GenerationReceipt;

// Create receipt
let receipt = GenerationReceipt::new(
    ontology_hash,
    template_hash,
    artifact_hash,
);

// Add metadata
receipt.add_metadata("generator".to_string(), "ggen-mcp".to_string());
receipt.add_metadata("version".to_string(), "0.1.0".to_string());

// Save receipt
receipt.save(Path::new("generated/.receipts/my_struct.json"))?;

// Later: Load and verify
let loaded = GenerationReceipt::load(receipt_path)?;
assert!(loaded.verify()); // Check integrity

// Check reproducibility
let current_hash = compute_file_hash(&artifact_path)?;
if !loaded.is_reproducible(&current_hash) {
    println!("Warning: Artifact has been modified");
}
```

### 5. SafeCodeWriter

Safe file operations with atomic writes and rollback support.

#### Features

- **Atomic Writes**: Write to temp file, then rename
- **Automatic Backups**: Create .bak files before overwrite
- **Rollback Support**: Restore from backup on error
- **Path Traversal Prevention**: Validate paths for security
- **Permission Checking**: Verify write permissions
- **Directory Creation**: Auto-create parent directories

#### Configuration

```rust
use spreadsheet_mcp::codegen::SafeCodeWriter;

let mut writer = SafeCodeWriter::new();
writer.create_backups = true;                      // Create .bak files
writer.backup_dir = Some(PathBuf::from("backups")); // Custom backup location
```

#### Usage

```rust
use std::path::Path;

let path = Path::new("generated/my_struct.rs");
let code = "pub struct MyStruct {}";

// Write safely
writer.write(path, code)?;

// If error occurs, rollback
if something_went_wrong {
    writer.rollback(path)?; // Restore from backup
}
```

## Generation Pipeline

### End-to-End Example

```rust
use spreadsheet_mcp::codegen::{
    CodeGenPipeline, SafeCodeWriter, ArtifactTracker,
    GenerationReceipt, compute_string_hash, compute_file_hash,
};
use std::path::PathBuf;

fn generate_code() -> Result<()> {
    // 1. Setup
    let output_path = PathBuf::from("generated/my_struct.rs");
    let state_file = PathBuf::from(".ggen/artifacts.json");
    let receipt_dir = PathBuf::from("generated/.receipts");

    // 2. Load artifact tracker
    let mut tracker = ArtifactTracker::load(state_file.clone())?;

    // 3. Compute hashes
    let ontology_content = std::fs::read_to_string("ontology/mcp-domain.ttl")?;
    let template_content = std::fs::read_to_string("templates/my_template.rs.tera")?;
    let ontology_hash = compute_string_hash(&ontology_content);
    let template_hash = compute_string_hash(&template_content);

    // 4. Check if regeneration needed
    if !tracker.is_stale(&output_path, &ontology_hash, &template_hash) {
        println!("Artifact is up-to-date, skipping generation");
        return Ok(());
    }

    // 5. Generate code through pipeline
    let mut pipeline = CodeGenPipeline::new();
    pipeline.run_rustfmt = true;

    let rendered = "pub struct MyStruct { pub field: String }";
    let result = pipeline.execute(&template_content, rendered, &output_path)?;

    if !result.success {
        anyhow::bail!("Code generation failed: {:?}", result.errors);
    }

    // 6. Write code safely
    let writer = SafeCodeWriter::new();
    let code = result.formatted_code.as_ref().unwrap_or(&rendered.to_string());
    writer.write(&output_path, code)?;

    // 7. Track artifact
    let artifact_hash = compute_file_hash(&output_path)?;
    tracker.record_artifact(
        output_path.clone(),
        ontology_hash.clone(),
        template_hash.clone(),
        vec![],
    )?;
    tracker.save()?;

    // 8. Create receipt
    let receipt = GenerationReceipt::new(
        ontology_hash,
        template_hash,
        artifact_hash,
    );
    std::fs::create_dir_all(&receipt_dir)?;
    receipt.save(&receipt_dir.join("my_struct.json"))?;

    println!("Code generation complete!");
    Ok(())
}
```

## Validation Rules

### Syntax Validation

Uses the `syn` crate to parse generated code as valid Rust:

```rust
match syn::parse_file(code) {
    Ok(_) => println!("Valid Rust syntax"),
    Err(e) => eprintln!("Syntax error: {}", e),
}
```

### Naming Conventions

#### PascalCase (Types)

- Structs: `MyStruct`, `HTTPServer`
- Enums: `MyEnum`, `ErrorKind`
- Traits: `MyTrait`, `Clone`
- Type aliases: `MyType`

#### snake_case (Functions/Variables)

- Functions: `my_function`, `process_data`
- Variables: `my_var`, `item_count`
- Module names: `my_module`

### Module Structure

Valid:
```rust
// Use statements first
use std::collections::HashMap;

// Then items
pub struct MyStruct {}
```

Invalid:
```rust
pub struct MyStruct {}

use std::collections::HashMap; // Use after item
```

### Documentation

Public items should have doc comments:

```rust
/// Documentation for public struct
pub struct MyStruct {
    pub field: String,
}

/// Documentation for public function
pub fn my_function() {}
```

### Duplicates

Each type/trait/function should be defined only once per file:

```rust
// Error: Duplicate definition
pub struct MyStruct {}
pub struct MyStruct {} // Duplicate!
```

## Error Handling

### Validation Errors vs Warnings

#### Errors (Block Generation)

- Syntax errors
- Duplicate definitions
- Unsafe code (when disabled)
- Path traversal attempts

#### Warnings (Log but Continue)

- Naming convention violations
- Missing documentation
- Long lines
- Suboptimal module structure

### Error Recovery

```rust
// Pipeline automatically validates and reports errors
let result = pipeline.execute(template, rendered, output_path)?;

if !result.success {
    // Detailed error information
    for error in &result.errors {
        eprintln!("Error: {}", error);
    }

    // Validation report
    if let Some(report) = result.validation_report {
        for issue in report.issues {
            match issue.severity {
                ValidationSeverity::Error => {
                    eprintln!("ERROR: {}", issue.message);
                    if let Some(suggestion) = issue.suggestion {
                        eprintln!("  Suggestion: {}", suggestion);
                    }
                }
                ValidationSeverity::Warning => {
                    eprintln!("WARNING: {}", issue.message);
                }
                ValidationSeverity::Info => {
                    println!("INFO: {}", issue.message);
                }
            }
        }
    }
}
```

### Rollback on Failure

```rust
let writer = SafeCodeWriter::new();

// Original file is backed up automatically
writer.write(path, new_code)?;

// If something goes wrong...
if validation_failed {
    // Restore original from backup
    writer.rollback(path)?;
}
```

## Receipt Verification

### Why Receipts?

Receipts provide:

1. **Provenance**: Know exactly what inputs produced what outputs
2. **Integrity**: Detect if generated files were modified
3. **Reproducibility**: Verify deterministic generation
4. **Audit Trail**: Track all generation events

### Receipt Format

```json
{
  "receipt_id": "a1b2c3...",
  "ontology_hash": "e4f5g6...",
  "template_hash": "h7i8j9...",
  "artifact_hash": "k0l1m2...",
  "timestamp": 1706000000,
  "generation_metadata": {
    "generator": "ggen-mcp",
    "version": "0.1.0",
    "template_file": "templates/my_template.rs.tera",
    "query_file": "queries/my_query.rq"
  }
}
```

### Verifying Receipts

```rust
// Load receipt
let receipt = GenerationReceipt::load(receipt_path)?;

// 1. Verify receipt integrity
if !receipt.verify() {
    println!("WARNING: Receipt has been tampered with!");
}

// 2. Check if artifact was modified
let current_hash = compute_file_hash(artifact_path)?;
if !receipt.is_reproducible(&current_hash) {
    println!("WARNING: Artifact has been modified since generation");
    println!("  Expected: {}", receipt.artifact_hash);
    println!("  Current:  {}", current_hash);
}

// 3. Check if regeneration needed
let current_ontology_hash = compute_string_hash(&ontology_content);
if receipt.ontology_hash != current_ontology_hash {
    println!("Ontology changed, regeneration recommended");
}
```

## Incremental Regeneration

### Strategy

Only regenerate files when their inputs change:

```rust
fn should_regenerate(
    tracker: &ArtifactTracker,
    path: &Path,
    ontology_hash: &str,
    template_hash: &str,
) -> bool {
    tracker.is_stale(path, ontology_hash, template_hash)
}
```

### Batch Regeneration

```rust
// Get all stale artifacts
let stale_files = tracker.get_stale_artifacts(&current_ontology_hash);

println!("Need to regenerate {} files", stale_files.len());

for path in stale_files {
    // Regenerate this file
    generate_file(&path)?;
}
```

### Dependency Tracking

```rust
// Record dependencies
tracker.record_artifact(
    PathBuf::from("generated/my_struct.rs"),
    ontology_hash,
    template_hash,
    vec![
        PathBuf::from("generated/base_trait.rs"),   // Depends on this
        PathBuf::from("generated/helper_types.rs"), // And this
    ],
)?;

// When a dependency changes, regenerate dependents
```

### Orphan Cleanup

Remove generated files that are no longer needed:

```rust
let orphaned = tracker.find_orphaned_files(Path::new("generated"))?;

println!("Found {} orphaned files", orphaned.len());

// Dry run: just list
for path in &orphaned {
    println!("Would remove: {:?}", path);
}

// Actually remove
tracker.cleanup_orphaned(Path::new("generated"), false)?;
```

## Testing Generated Code

### Unit Tests

The validation module includes comprehensive tests:

```bash
cargo test --test codegen_validation_tests
```

### Integration Tests

Test the full generation pipeline:

```rust
#[test]
fn test_full_generation_workflow() -> Result<()> {
    // 1. Generate code
    let result = pipeline.execute(template, rendered, output_path)?;
    assert!(result.success);

    // 2. Write safely
    writer.write(&output_path, rendered)?;

    // 3. Track artifact
    tracker.record_artifact(path, ontology_hash, template_hash, vec![])?;

    // 4. Create receipt
    let receipt = GenerationReceipt::new(ontology_hash, template_hash, artifact_hash);
    receipt.save(&receipt_path)?;

    // 5. Verify everything
    assert!(output_path.exists());
    assert!(receipt.verify());

    Ok(())
}
```

### Smoke Tests

Verify generated code compiles:

```bash
# After generation
cargo check
cargo test
```

## Troubleshooting

### Common Issues

#### 1. Syntax Errors in Generated Code

**Symptom**: `syn::parse_file` fails

**Diagnosis**:
```rust
let report = validator.validate_code(code, "file.rs")?;
for issue in &report.issues {
    if issue.severity == ValidationSeverity::Error {
        println!("{}", issue.message);
    }
}
```

**Solutions**:
- Check template for invalid Rust syntax
- Ensure SPARQL query returns valid data
- Verify template context has all required variables

#### 2. Naming Convention Warnings

**Symptom**: Warnings about PascalCase/snake_case

**Solutions**:
- Update template to use proper naming
- Example: `{{ name | pascal_case }}` for struct names
- Example: `{{ name | snake_case }}` for function names

#### 3. Duplicate Definitions

**Symptom**: Errors about duplicate structs/traits

**Diagnosis**:
- Check SPARQL query for duplicates
- Verify template doesn't generate same item twice

**Solutions**:
- Add DISTINCT to SPARQL query
- Use `{% for item in items | unique %}`
- Call `validator.reset()` between validation runs

#### 4. Stale Artifacts Not Detected

**Symptom**: Files not regenerating when they should

**Diagnosis**:
```rust
println!("Recorded hash: {}", metadata.ontology_hash);
println!("Current hash:  {}", current_ontology_hash);
```

**Solutions**:
- Ensure hashes are computed consistently
- Call `tracker.save()` after recording
- Check file permissions on state file

#### 5. Rollback Fails

**Symptom**: Cannot restore backup

**Solutions**:
- Ensure `create_backups = true`
- Check backup directory exists
- Verify backup file wasn't deleted

### Debugging

Enable debug logging:

```bash
RUST_LOG=debug cargo run
```

Check validation reports:

```rust
let report = validator.validate_code(code, "file.rs")?;
println!("Errors: {}", report.error_count);
println!("Warnings: {}", report.warning_count);

for issue in &report.issues {
    println!("{:?} at {:?}: {}",
        issue.severity,
        issue.location,
        issue.message
    );
}
```

### Best Practices

1. **Always validate before writing**: Run pipeline before SafeCodeWriter
2. **Track all artifacts**: Don't skip artifact tracking
3. **Save receipts**: Enable audit trail
4. **Test templates**: Validate templates separately
5. **Use dry runs**: Test with `dry_run = true` first
6. **Monitor warnings**: Address warnings to improve quality
7. **Regular cleanup**: Remove orphaned files periodically

## References

- [ggen.toml Configuration](../ggen.toml)
- [Toyota Production System](https://en.wikipedia.org/wiki/Toyota_Production_System)
- [Poka-Yoke (Error Proofing)](https://en.wikipedia.org/wiki/Poka-yoke)
- [SHACL Validation](https://www.w3.org/TR/shacl/)
- [DDD Patterns](https://martinfowler.com/bliki/DomainDrivenDesign.html)
