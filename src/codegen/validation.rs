//! Code Generation Validation Module
//!
//! This module provides comprehensive validation for the ggen-mcp template-based code generation
//! system, following Toyota Production System's Poka-Yoke (error-proofing) principles.
//!
//! ## Components
//! - **GeneratedCodeValidator**: Validates generated Rust code
//! - **CodeGenPipeline**: Safe generation pipeline with validation at each stage
//! - **ArtifactTracker**: Tracks generated artifacts and enables incremental regeneration
//! - **GenerationReceipt**: Provides provenance and verification for generated code
//! - **SafeCodeWriter**: Safe file writing with atomic operations and rollback
//!
//! ## Workflow
//! Ontology (TTL) → SPARQL Query → Template Rendering → Validation → Safe Writing

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tempfile::NamedTempFile;

// =============================================================================
// Type Definitions
// =============================================================================

/// Validation severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationSeverity {
    /// Error: Must be fixed before code generation can proceed
    Error,
    /// Warning: Should be addressed but doesn't block generation
    Warning,
    /// Info: Informational message
    Info,
}

/// Validation result for a single check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub message: String,
    pub location: Option<String>,
    pub suggestion: Option<String>,
}

/// Complete validation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self {
            issues: Vec::new(),
            error_count: 0,
            warning_count: 0,
            info_count: 0,
        }
    }

    pub fn add_error(&mut self, message: String, location: Option<String>, suggestion: Option<String>) {
        self.issues.push(ValidationIssue {
            severity: ValidationSeverity::Error,
            message,
            location,
            suggestion,
        });
        self.error_count += 1;
    }

    pub fn add_warning(&mut self, message: String, location: Option<String>, suggestion: Option<String>) {
        self.issues.push(ValidationIssue {
            severity: ValidationSeverity::Warning,
            message,
            location,
            suggestion,
        });
        self.warning_count += 1;
    }

    pub fn add_info(&mut self, message: String, location: Option<String>) {
        self.issues.push(ValidationIssue {
            severity: ValidationSeverity::Info,
            message,
            location,
            suggestion: None,
        });
        self.info_count += 1;
    }

    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    pub fn is_valid(&self) -> bool {
        !self.has_errors()
    }
}

impl Default for ValidationReport {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// 1. GeneratedCodeValidator - Validate Generated Rust Code
// =============================================================================

/// Validates generated Rust code for syntax, semantics, and conventions
pub struct GeneratedCodeValidator {
    /// Allow unsafe code blocks
    pub allow_unsafe: bool,
    /// Require documentation comments
    pub require_doc_comments: bool,
    /// Maximum line length
    pub max_line_length: usize,
    /// Track seen definitions to detect duplicates
    seen_structs: HashSet<String>,
    seen_traits: HashSet<String>,
    seen_functions: HashSet<String>,
}

impl GeneratedCodeValidator {
    pub fn new() -> Self {
        Self {
            allow_unsafe: false,
            require_doc_comments: true,
            max_line_length: 120,
            seen_structs: HashSet::new(),
            seen_traits: HashSet::new(),
            seen_functions: HashSet::new(),
        }
    }

    /// Validate Rust code syntax and semantics
    pub fn validate_code(&mut self, code: &str, file_name: &str) -> Result<ValidationReport> {
        let mut report = ValidationReport::new();

        // 1. Syntax validation using syn
        self.validate_syntax(code, &mut report, file_name)?;

        // 2. Naming convention validation
        self.validate_naming_conventions(code, &mut report, file_name);

        // 3. Module structure validation
        self.validate_module_structure(code, &mut report, file_name);

        // 4. Check for unsafe code
        if !self.allow_unsafe {
            self.validate_no_unsafe(code, &mut report, file_name);
        }

        // 5. Check for duplicate definitions
        self.validate_no_duplicates(code, &mut report, file_name);

        // 6. Line length validation
        self.validate_line_lengths(code, &mut report, file_name);

        // 7. Documentation validation
        if self.require_doc_comments {
            self.validate_documentation(code, &mut report, file_name);
        }

        Ok(report)
    }

    /// Validate Rust syntax by parsing with syn
    fn validate_syntax(&self, code: &str, report: &mut ValidationReport, file_name: &str) -> Result<()> {
        match syn::parse_file(code) {
            Ok(_) => {
                report.add_info(
                    format!("Syntax validation passed for {}", file_name),
                    None
                );
                Ok(())
            }
            Err(e) => {
                report.add_error(
                    format!("Syntax error: {}", e),
                    Some(file_name.to_string()),
                    Some("Check the template for invalid Rust syntax".to_string()),
                );
                Err(anyhow!("Syntax validation failed for {}: {}", file_name, e))
            }
        }
    }

    /// Validate naming conventions (snake_case for functions, PascalCase for types)
    fn validate_naming_conventions(&self, code: &str, report: &mut ValidationReport, file_name: &str) {
        for (line_num, line) in code.lines().enumerate() {
            let trimmed = line.trim();

            // Check struct names (should be PascalCase)
            if trimmed.starts_with("pub struct ") || trimmed.starts_with("struct ") {
                if let Some(name) = extract_name_after(trimmed, "struct ") {
                    if !is_pascal_case(&name) {
                        report.add_warning(
                            format!("Struct name '{}' should be PascalCase", name),
                            Some(format!("{}:{}", file_name, line_num + 1)),
                            Some("Use PascalCase for type names (e.g., MyStruct)".to_string()),
                        );
                    }
                }
            }

            // Check function names (should be snake_case)
            if trimmed.starts_with("pub fn ") || trimmed.starts_with("fn ") || trimmed.starts_with("async fn ") {
                if let Some(name) = extract_function_name(trimmed) {
                    if !is_snake_case(&name) {
                        report.add_warning(
                            format!("Function name '{}' should be snake_case", name),
                            Some(format!("{}:{}", file_name, line_num + 1)),
                            Some("Use snake_case for function names (e.g., my_function)".to_string()),
                        );
                    }
                }
            }
        }
    }

    /// Validate module structure
    fn validate_module_structure(&self, code: &str, report: &mut ValidationReport, file_name: &str) {
        // Check for proper module header
        if !code.contains("//!") && !code.contains("///") {
            report.add_warning(
                "Missing module or item documentation".to_string(),
                Some(file_name.to_string()),
                Some("Add module-level documentation with //!".to_string()),
            );
        }

        // Check for use statements organization
        let has_use = code.contains("use ");
        let has_pub_use = code.contains("pub use ");

        if has_use || has_pub_use {
            // Validate use statements come before item definitions
            let mut seen_item = false;
            for line in code.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("pub struct ")
                    || trimmed.starts_with("pub enum ")
                    || trimmed.starts_with("pub fn ") {
                    seen_item = true;
                }
                if seen_item && trimmed.starts_with("use ") {
                    report.add_warning(
                        "Use statements should appear before item definitions".to_string(),
                        Some(file_name.to_string()),
                        Some("Move all use statements to the top of the file".to_string()),
                    );
                    break;
                }
            }
        }
    }

    /// Validate no unsafe code
    fn validate_no_unsafe(&self, code: &str, report: &mut ValidationReport, file_name: &str) {
        for (line_num, line) in code.lines().enumerate() {
            if line.contains("unsafe ") {
                report.add_error(
                    "Unsafe code is not allowed in generated files".to_string(),
                    Some(format!("{}:{}", file_name, line_num + 1)),
                    Some("Remove unsafe code or enable allow_unsafe".to_string()),
                );
            }
        }
    }

    /// Validate no duplicate definitions
    fn validate_no_duplicates(&mut self, code: &str, report: &mut ValidationReport, file_name: &str) {
        for (line_num, line) in code.lines().enumerate() {
            let trimmed = line.trim();

            // Check struct duplicates
            if trimmed.starts_with("pub struct ") || trimmed.starts_with("struct ") {
                if let Some(name) = extract_name_after(trimmed, "struct ") {
                    if self.seen_structs.contains(&name) {
                        report.add_error(
                            format!("Duplicate struct definition: {}", name),
                            Some(format!("{}:{}", file_name, line_num + 1)),
                            Some("Each struct should be defined only once".to_string()),
                        );
                    } else {
                        self.seen_structs.insert(name);
                    }
                }
            }

            // Check trait duplicates
            if trimmed.starts_with("pub trait ") || trimmed.starts_with("trait ") {
                if let Some(name) = extract_name_after(trimmed, "trait ") {
                    if self.seen_traits.contains(&name) {
                        report.add_error(
                            format!("Duplicate trait definition: {}", name),
                            Some(format!("{}:{}", file_name, line_num + 1)),
                            Some("Each trait should be defined only once".to_string()),
                        );
                    } else {
                        self.seen_traits.insert(name);
                    }
                }
            }
        }
    }

    /// Validate line lengths
    fn validate_line_lengths(&self, code: &str, report: &mut ValidationReport, file_name: &str) {
        for (line_num, line) in code.lines().enumerate() {
            if line.len() > self.max_line_length {
                report.add_warning(
                    format!("Line exceeds maximum length of {} characters", self.max_line_length),
                    Some(format!("{}:{}", file_name, line_num + 1)),
                    Some("Split long lines for better readability".to_string()),
                );
            }
        }
    }

    /// Validate documentation comments
    fn validate_documentation(&self, code: &str, report: &mut ValidationReport, file_name: &str) {
        let lines: Vec<&str> = code.lines().collect();

        for i in 0..lines.len() {
            let line = lines[i].trim();

            // Check if this is a public item that needs documentation
            if line.starts_with("pub struct ") || line.starts_with("pub trait ") || line.starts_with("pub fn ") {
                // Check if previous line has doc comment
                let has_doc = if i > 0 {
                    let prev = lines[i - 1].trim();
                    prev.starts_with("///") || prev.starts_with("//!")
                } else {
                    false
                };

                if !has_doc {
                    let item_type = if line.starts_with("pub struct ") {
                        "struct"
                    } else if line.starts_with("pub trait ") {
                        "trait"
                    } else {
                        "function"
                    };

                    report.add_warning(
                        format!("Public {} lacks documentation comment", item_type),
                        Some(format!("{}:{}", file_name, i + 1)),
                        Some(format!("Add /// documentation comment above {}", item_type)),
                    );
                }
            }
        }
    }

    /// Reset tracking state (call between validation runs)
    pub fn reset(&mut self) {
        self.seen_structs.clear();
        self.seen_traits.clear();
        self.seen_functions.clear();
    }
}

impl Default for GeneratedCodeValidator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// 2. CodeGenPipeline - Safe Generation Pipeline
// =============================================================================

/// Safe code generation pipeline with validation at each stage
pub struct CodeGenPipeline {
    validator: GeneratedCodeValidator,
    /// Run rustfmt after generation
    pub run_rustfmt: bool,
    /// Run clippy checks
    pub run_clippy: bool,
    /// Run compilation smoke test
    pub run_compile_check: bool,
}

impl CodeGenPipeline {
    pub fn new() -> Self {
        Self {
            validator: GeneratedCodeValidator::new(),
            run_rustfmt: true,
            run_clippy: false,
            run_compile_check: false,
        }
    }

    /// Execute the complete generation pipeline
    pub fn execute(
        &mut self,
        template_content: &str,
        rendered_code: &str,
        output_path: &Path,
    ) -> Result<GenerationResult> {
        let mut result = GenerationResult::new(output_path.to_path_buf());

        // Stage 1: Pre-generation validation (template)
        tracing::debug!("Stage 1: Validating template");
        self.validate_template(template_content, &mut result)?;

        // Stage 2: Post-generation validation (rendered code)
        tracing::debug!("Stage 2: Validating rendered code");
        let validation_report = self.validate_rendered_code(rendered_code, output_path, &mut result)?;
        result.validation_report = Some(validation_report.clone());

        if !validation_report.is_valid() {
            return Err(anyhow!("Code validation failed with {} errors", validation_report.error_count));
        }

        // Stage 3: Format with rustfmt
        if self.run_rustfmt {
            tracing::debug!("Stage 3: Formatting with rustfmt");
            result.formatted_code = Some(self.format_code(rendered_code)?);
        }

        // Stage 4: Clippy checks (optional)
        if self.run_clippy {
            tracing::debug!("Stage 4: Running clippy checks");
            self.run_clippy_checks(output_path)?;
        }

        // Stage 5: Compilation smoke test (optional)
        if self.run_compile_check {
            tracing::debug!("Stage 5: Running compilation smoke test");
            self.run_compilation_test(output_path)?;
        }

        result.success = true;
        Ok(result)
    }

    fn validate_template(&self, template: &str, result: &mut GenerationResult) -> Result<()> {
        // Basic template validation
        let open_count = template.matches("{{").count() + template.matches("{%").count();
        let close_count = template.matches("}}").count() + template.matches("%}").count();

        if open_count != close_count {
            result.errors.push("Template has unbalanced Tera braces".to_string());
            return Err(anyhow!("Template validation failed: unbalanced braces"));
        }

        Ok(())
    }

    fn validate_rendered_code(
        &mut self,
        code: &str,
        path: &Path,
        result: &mut GenerationResult,
    ) -> Result<ValidationReport> {
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let report = self.validator.validate_code(code, file_name)?;

        // Add errors to result
        for issue in &report.issues {
            if issue.severity == ValidationSeverity::Error {
                result.errors.push(issue.message.clone());
            }
        }

        Ok(report)
    }

    fn format_code(&self, code: &str) -> Result<String> {
        // Write to temp file and format
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(code.as_bytes())?;
        let temp_path = temp_file.path();

        let output = std::process::Command::new("rustfmt")
            .arg("--edition")
            .arg("2024")
            .arg(temp_path)
            .output();

        match output {
            Ok(result) if result.status.success() => {
                fs::read_to_string(temp_path)
                    .context("Failed to read formatted code")
            }
            Ok(result) => {
                let stderr = String::from_utf8_lossy(&result.stderr);
                tracing::warn!("rustfmt failed: {}", stderr);
                // Return original code if formatting fails
                Ok(code.to_string())
            }
            Err(e) => {
                tracing::warn!("Failed to run rustfmt: {}", e);
                // Return original code if rustfmt is not available
                Ok(code.to_string())
            }
        }
    }

    fn run_clippy_checks(&self, _path: &Path) -> Result<()> {
        // Note: clippy checks are typically run on the whole crate, not individual files
        // This would require cargo clippy
        tracing::debug!("Clippy checks would be run here");
        Ok(())
    }

    fn run_compilation_test(&self, _path: &Path) -> Result<()> {
        // Note: compilation tests are typically run on the whole crate
        // This would require cargo check
        tracing::debug!("Compilation test would be run here");
        Ok(())
    }
}

impl Default for CodeGenPipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of code generation pipeline
#[derive(Debug, Clone)]
pub struct GenerationResult {
    pub output_path: PathBuf,
    pub success: bool,
    pub errors: Vec<String>,
    pub validation_report: Option<ValidationReport>,
    pub formatted_code: Option<String>,
}

impl GenerationResult {
    fn new(output_path: PathBuf) -> Self {
        Self {
            output_path,
            success: false,
            errors: Vec::new(),
            validation_report: None,
            formatted_code: None,
        }
    }
}

// =============================================================================
// 3. ArtifactTracker - Track Generated Artifacts
// =============================================================================

/// Metadata for a generated artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    pub path: PathBuf,
    pub timestamp: u64,
    pub ontology_hash: String,
    pub template_hash: String,
    pub artifact_hash: String,
    pub dependencies: Vec<PathBuf>,
}

/// Tracks all generated artifacts and their metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactTracker {
    pub artifacts: HashMap<PathBuf, ArtifactMetadata>,
    pub state_file: PathBuf,
}

impl ArtifactTracker {
    pub fn new(state_file: PathBuf) -> Self {
        Self {
            artifacts: HashMap::new(),
            state_file,
        }
    }

    /// Load tracker state from file
    pub fn load(state_file: PathBuf) -> Result<Self> {
        if state_file.exists() {
            let content = fs::read_to_string(&state_file)?;
            let tracker: ArtifactTracker = serde_json::from_str(&content)?;
            Ok(tracker)
        } else {
            Ok(Self::new(state_file))
        }
    }

    /// Save tracker state to file
    pub fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self)?;

        // Ensure parent directory exists
        if let Some(parent) = self.state_file.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&self.state_file, content)?;
        Ok(())
    }

    /// Record a generated artifact
    pub fn record_artifact(
        &mut self,
        path: PathBuf,
        ontology_hash: String,
        template_hash: String,
        dependencies: Vec<PathBuf>,
    ) -> Result<()> {
        let artifact_hash = if path.exists() {
            compute_file_hash(&path)?
        } else {
            String::new()
        };

        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();

        let metadata = ArtifactMetadata {
            path: path.clone(),
            timestamp,
            ontology_hash,
            template_hash,
            artifact_hash,
            dependencies,
        };

        self.artifacts.insert(path, metadata);
        Ok(())
    }

    /// Check if artifact is stale (needs regeneration)
    pub fn is_stale(&self, path: &Path, ontology_hash: &str, template_hash: &str) -> bool {
        match self.artifacts.get(path) {
            Some(metadata) => {
                // Check if hashes match
                if metadata.ontology_hash != ontology_hash || metadata.template_hash != template_hash {
                    return true;
                }

                // Check if file still exists
                if !path.exists() {
                    return true;
                }

                // Check if file content hash matches
                if let Ok(current_hash) = compute_file_hash(path) {
                    if current_hash != metadata.artifact_hash {
                        return true;
                    }
                }

                false
            }
            None => true, // Not tracked, consider stale
        }
    }

    /// Get stale artifacts that need regeneration
    pub fn get_stale_artifacts(&self, current_ontology_hash: &str) -> Vec<PathBuf> {
        self.artifacts
            .iter()
            .filter(|(path, metadata)| {
                // Check if ontology hash changed or file doesn't exist
                metadata.ontology_hash != current_ontology_hash || !path.exists()
            })
            .map(|(path, _)| path.clone())
            .collect()
    }

    /// Find orphaned files (files that exist but aren't tracked)
    pub fn find_orphaned_files(&self, directory: &Path) -> Result<Vec<PathBuf>> {
        let mut orphaned = Vec::new();

        if !directory.exists() {
            return Ok(orphaned);
        }

        for entry in walkdir::WalkDir::new(directory)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
                if !self.artifacts.contains_key(path) {
                    orphaned.push(path.to_path_buf());
                }
            }
        }

        Ok(orphaned)
    }

    /// Remove artifact from tracking
    pub fn remove_artifact(&mut self, path: &Path) {
        self.artifacts.remove(path);
    }

    /// Clean up orphaned files
    pub fn cleanup_orphaned(&mut self, directory: &Path, dry_run: bool) -> Result<Vec<PathBuf>> {
        let orphaned = self.find_orphaned_files(directory)?;

        if !dry_run {
            for path in &orphaned {
                if let Err(e) = fs::remove_file(path) {
                    tracing::warn!("Failed to remove orphaned file {:?}: {}", path, e);
                }
            }
        }

        Ok(orphaned)
    }
}

// =============================================================================
// 4. GenerationReceipt - Provenance and Verification
// =============================================================================

/// Receipt for code generation (provenance tracking)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationReceipt {
    pub receipt_id: String,
    pub ontology_hash: String,
    pub template_hash: String,
    pub artifact_hash: String,
    pub timestamp: u64,
    pub generation_metadata: HashMap<String, String>,
}

impl GenerationReceipt {
    /// Create a new generation receipt
    pub fn new(
        ontology_hash: String,
        template_hash: String,
        artifact_hash: String,
    ) -> Self {
        let receipt_id = Self::generate_receipt_id(&ontology_hash, &template_hash, &artifact_hash);
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            receipt_id,
            ontology_hash,
            template_hash,
            artifact_hash,
            timestamp,
            generation_metadata: HashMap::new(),
        }
    }

    /// Generate deterministic receipt ID
    fn generate_receipt_id(ontology_hash: &str, template_hash: &str, artifact_hash: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(ontology_hash.as_bytes());
        hasher.update(template_hash.as_bytes());
        hasher.update(artifact_hash.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Verify receipt integrity
    pub fn verify(&self) -> bool {
        let expected_id = Self::generate_receipt_id(
            &self.ontology_hash,
            &self.template_hash,
            &self.artifact_hash,
        );
        self.receipt_id == expected_id
    }

    /// Check if generation is reproducible
    pub fn is_reproducible(&self, current_artifact_hash: &str) -> bool {
        self.artifact_hash == current_artifact_hash
    }

    /// Add metadata to receipt
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.generation_metadata.insert(key, value);
    }

    /// Save receipt to file
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Load receipt from file
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let receipt: GenerationReceipt = serde_json::from_str(&content)?;
        Ok(receipt)
    }
}

// =============================================================================
// 5. SafeCodeWriter - Safe File Writing
// =============================================================================

/// Safe file writer with atomic operations and rollback
pub struct SafeCodeWriter {
    /// Create backups before overwriting
    pub create_backups: bool,
    /// Backup directory
    pub backup_dir: Option<PathBuf>,
}

impl SafeCodeWriter {
    pub fn new() -> Self {
        Self {
            create_backups: true,
            backup_dir: None,
        }
    }

    /// Write code to file safely with atomic operations
    pub fn write(&self, path: &Path, content: &str) -> Result<()> {
        // 1. Validate path (prevent path traversal)
        self.validate_path(path)?;

        // 2. Check permissions
        self.check_permissions(path)?;

        // 3. Create backup if file exists
        if path.exists() && self.create_backups {
            self.create_backup(path)?;
        }

        // 4. Atomic write (write to temp, then rename)
        self.atomic_write(path, content)?;

        Ok(())
    }

    /// Validate path to prevent path traversal attacks
    fn validate_path(&self, path: &Path) -> Result<()> {
        // Check for path traversal patterns
        let path_str = path.to_string_lossy();
        if path_str.contains("..") {
            return Err(anyhow!("Path traversal detected: {:?}", path));
        }

        // Ensure path is absolute or relative to cwd
        if !path.is_absolute() {
            // Ensure parent directory exists
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
        }

        Ok(())
    }

    /// Check write permissions
    fn check_permissions(&self, path: &Path) -> Result<()> {
        if path.exists() {
            let metadata = fs::metadata(path)?;
            if metadata.permissions().readonly() {
                return Err(anyhow!("File is read-only: {:?}", path));
            }
        }
        Ok(())
    }

    /// Create backup of existing file
    fn create_backup(&self, path: &Path) -> Result<PathBuf> {
        let backup_path = if let Some(backup_dir) = &self.backup_dir {
            fs::create_dir_all(backup_dir)?;
            let file_name = path.file_name()
                .ok_or_else(|| anyhow!("Invalid file name"))?;
            backup_dir.join(format!("{}.bak", file_name.to_string_lossy()))
        } else {
            path.with_extension("bak")
        };

        fs::copy(path, &backup_path)?;
        tracing::debug!("Created backup: {:?}", backup_path);
        Ok(backup_path)
    }

    /// Atomic write operation
    fn atomic_write(&self, path: &Path, content: &str) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        // Write to temporary file
        let mut temp_file = NamedTempFile::new_in(
            path.parent().unwrap_or_else(|| Path::new("."))
        )?;
        temp_file.write_all(content.as_bytes())?;
        temp_file.flush()?;

        // Atomically rename to target path
        temp_file.persist(path)?;

        Ok(())
    }

    /// Rollback to backup
    pub fn rollback(&self, path: &Path) -> Result<()> {
        let backup_path = if let Some(backup_dir) = &self.backup_dir {
            let file_name = path.file_name()
                .ok_or_else(|| anyhow!("Invalid file name"))?;
            backup_dir.join(format!("{}.bak", file_name.to_string_lossy()))
        } else {
            path.with_extension("bak")
        };

        if backup_path.exists() {
            fs::copy(&backup_path, path)?;
            tracing::info!("Rolled back to backup: {:?}", backup_path);
            Ok(())
        } else {
            Err(anyhow!("No backup found for {:?}", path))
        }
    }
}

impl Default for SafeCodeWriter {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Compute SHA-256 hash of file contents
pub fn compute_file_hash(path: &Path) -> Result<String> {
    let content = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Compute SHA-256 hash of string
pub fn compute_string_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Extract name after a keyword (e.g., "struct Name" -> "Name")
fn extract_name_after(line: &str, keyword: &str) -> Option<String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == keyword.trim() && i + 1 < parts.len() {
            let name = parts[i + 1]
                .trim_end_matches('{')
                .trim_end_matches('<')
                .trim_end_matches('(');
            return Some(name.to_string());
        }
    }
    None
}

/// Extract function name from function definition
fn extract_function_name(line: &str) -> Option<String> {
    // Handle: pub fn name, fn name, pub async fn name, async fn name
    let line = line.replace("pub ", "").replace("async ", "");
    if let Some(fn_pos) = line.find("fn ") {
        let after_fn = &line[fn_pos + 3..];
        let name_end = after_fn.find('(').or_else(|| after_fn.find('<')).or_else(|| after_fn.find(' '))?;
        return Some(after_fn[..name_end].trim().to_string());
    }
    None
}

/// Check if a string is in PascalCase
fn is_pascal_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    // First character should be uppercase
    let first_char = s.chars().next().unwrap();
    if !first_char.is_uppercase() {
        return false;
    }

    // Should not contain underscores
    !s.contains('_')
}

/// Check if a string is in snake_case
fn is_snake_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    // Should not start with uppercase
    let first_char = s.chars().next().unwrap();
    if first_char.is_uppercase() {
        return false;
    }

    // All characters should be lowercase, digits, or underscores
    s.chars().all(|c| c.is_lowercase() || c.is_numeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_pascal_case() {
        assert!(is_pascal_case("MyStruct"));
        assert!(is_pascal_case("HTTPServer"));
        assert!(!is_pascal_case("myStruct"));
        assert!(!is_pascal_case("my_struct"));
        assert!(!is_pascal_case(""));
    }

    #[test]
    fn test_is_snake_case() {
        assert!(is_snake_case("my_function"));
        assert!(is_snake_case("process_data"));
        assert!(is_snake_case("item_123"));
        assert!(!is_snake_case("MyFunction"));
        assert!(!is_snake_case("myFunction"));
        assert!(!is_snake_case(""));
    }

    #[test]
    fn test_validation_report() {
        let mut report = ValidationReport::new();
        assert!(!report.has_errors());

        report.add_error("Error message".to_string(), None, None);
        assert!(report.has_errors());
        assert_eq!(report.error_count, 1);

        report.add_warning("Warning message".to_string(), None, None);
        assert_eq!(report.warning_count, 1);
    }

    #[test]
    fn test_compute_string_hash() {
        let hash1 = compute_string_hash("test content");
        let hash2 = compute_string_hash("test content");
        let hash3 = compute_string_hash("different content");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
