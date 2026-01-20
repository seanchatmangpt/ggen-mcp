//! Code Generation Module
//!
//! This module provides comprehensive code generation capabilities for the ggen-mcp system,
//! following Toyota Production System's Poka-Yoke (error-proofing) principles.
//!
//! ## Architecture
//!
//! The code generation pipeline follows this workflow:
//!
//! ```text
//! Ontology (TTL) → SPARQL Query → Template Rendering → Validation → Safe Writing
//! ```
//!
//! ## Modules
//!
//! - **validation**: Comprehensive validation for generated code
//!   - GeneratedCodeValidator: Validates syntax, semantics, and conventions
//!   - CodeGenPipeline: Orchestrates the generation pipeline
//!   - ArtifactTracker: Tracks generated files and dependencies
//!   - GenerationReceipt: Provides provenance and verification
//!   - SafeCodeWriter: Safe file operations with atomic writes
//!
//! ## Error Prevention (Poka-Yoke)
//!
//! The module implements several error-prevention mechanisms:
//!
//! 1. **Pre-generation validation**: Validate templates and inputs
//! 2. **Post-generation validation**: Validate generated code
//! 3. **Atomic operations**: All file writes are atomic
//! 4. **Backup and rollback**: Automatic backups before overwrites
//! 5. **Dependency tracking**: Track relationships between artifacts
//! 6. **Provenance tracking**: Generate verifiable receipts
//! 7. **Incremental regeneration**: Only regenerate what changed
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use spreadsheet_mcp::codegen::validation::{CodeGenPipeline, SafeCodeWriter};
//! use std::path::Path;
//!
//! # fn example() -> anyhow::Result<()> {
//! let mut pipeline = CodeGenPipeline::new();
//! pipeline.run_rustfmt = true;
//!
//! let template = r#"
//!     pub struct {{ name }} {
//!         pub field: String,
//!     }
//! "#;
//!
//! let rendered = "pub struct MyStruct { pub field: String, }";
//! let output_path = Path::new("generated/my_struct.rs");
//!
//! let result = pipeline.execute(template, rendered, output_path)?;
//!
//! if result.success {
//!     let writer = SafeCodeWriter::new();
//!     let code = result.formatted_code.as_ref().unwrap_or(&rendered.to_string());
//!     writer.write(output_path, code)?;
//! }
//! # Ok(())
//! # }
//! ```

pub mod validation;

pub use validation::{
    ArtifactMetadata, ArtifactTracker, CodeGenPipeline, GeneratedCodeValidator, GenerationReceipt,
    GenerationResult, SafeCodeWriter, ValidationIssue, ValidationReport, ValidationSeverity,
    compute_file_hash, compute_string_hash,
};
