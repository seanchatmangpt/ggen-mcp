//! Ontology Generation and Validation Tools
//!
//! MCP tools for:
//! - Rendering Tera templates with SPARQL query results
//! - Writing generated artifacts with validation and audit trail
//! - Validating generated code with multi-language support
//! - Golden file comparison and detailed error reporting

use crate::audit::integration::audit_tool;
use crate::codegen::validation::{
    CodeValidationReport, DiffReport, GeneratedCodeValidator,
    GenerationReceipt, SafeCodeWriter, compute_string_hash,
    compute_diff, load_golden_file, update_golden_file,
    validate_json, validate_typescript, validate_yaml,
};
use crate::state::AppState;
use crate::template::{
    RenderConfig, SafeRenderer,
};
use anyhow::{Context, Result, anyhow};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// ============================================================================
// Constants
// ============================================================================

const DEFAULT_TIMEOUT_MS: u64 = 5000;
const DEFAULT_MAX_OUTPUT_SIZE: usize = 1024 * 1024; // 1MB
const MAX_TEMPLATE_NAME_LEN: usize = 256;
const MAX_OUTPUT_PATH_LEN: usize = 1024;

// ============================================================================
// MCP Tool: render_template
// ============================================================================

/// Parameters for render_template tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RenderTemplateParams {
    /// Template name or content (inline templates use "inline:<content>")
    pub template: String,

    /// Context data for template rendering (JSON object)
    pub context: JsonValue,

    /// Timeout in milliseconds (default: 5000, max: 30000)
    #[serde(default)]
    pub timeout_ms: Option<u64>,

    /// Maximum output size in bytes (default: 1MB, max: 10MB)
    #[serde(default)]
    pub max_output_size: Option<usize>,

    /// Enable syntax validation (default: true)
    #[serde(default = "default_true")]
    pub validate_syntax: Option<bool>,

    /// Enable security checks (default: true)
    #[serde(default = "default_true")]
    pub security_checks: Option<bool>,

    /// Preview mode - don't write to file (default: false)
    #[serde(default)]
    pub preview: bool,

    /// Target output format for validation (rust, typescript, yaml, json, toml)
    #[serde(default)]
    pub output_format: Option<String>,
}

fn default_true() -> Option<bool> {
    Some(true)
}

/// Response from render_template tool
#[derive(Debug, Serialize, JsonSchema)]
pub struct RenderTemplateResponse {
    /// Rendered output
    pub output: String,

    /// Size of rendered output in bytes
    pub output_size: usize,

    /// Rendering duration in milliseconds
    pub duration_ms: u64,

    /// Validation warnings (if any)
    pub warnings: Vec<String>,

    /// Preview mode indicator
    pub preview: bool,

    /// Content hash (SHA-256)
    pub content_hash: String,
}

/// Render a Tera template with context data
pub async fn render_template(
    _state: Arc<AppState>,
    params: RenderTemplateParams,
) -> Result<RenderTemplateResponse> {
    let _span = audit_tool("render_template", &params);

    // Validate parameters
    validate_template_param(&params.template)?;
    validate_context_param(&params.context)?;

    // Build render configuration
    let mut config = RenderConfig::default()
        .with_timeout_ms(params.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS))
        .with_syntax_validation(params.validate_syntax.unwrap_or(true))
        .with_security_checks(params.security_checks.unwrap_or(true));

    // Set max_output_size directly (no fluent method available)
    config.max_output_size = params
        .max_output_size
        .unwrap_or(DEFAULT_MAX_OUTPUT_SIZE)
        .min(10 * 1024 * 1024);

    // Create safe renderer
    let renderer = SafeRenderer::new(config)
        .context("Failed to create safe renderer")?;

    // Build template context
    let template_context = build_template_context(&params)?;

    // Determine template name
    let template_name = if params.template.starts_with("inline:") {
        let inline_content = &params.template[7..];
        renderer
            .add_template("inline_template", inline_content)
            .context("Failed to add inline template")?;
        "inline_template"
    } else {
        &params.template
    };

    // Render template
    let start = std::time::Instant::now();
    let output = renderer
        .render_safe(template_name, &template_context)
        .context("Template rendering failed")?;
    let duration_ms = start.elapsed().as_millis() as u64;

    // Validate output format if specified
    let warnings = if let Some(format) = &params.output_format {
        validate_output_format(&output, format)?
    } else {
        Vec::new()
    };

    // Compute content hash
    let content_hash = compute_string_hash(&output);

    Ok(RenderTemplateResponse {
        output_size: output.len(),
        output,
        duration_ms,
        warnings,
        preview: params.preview,
        content_hash,
    })
}

/// Validate template parameter
fn validate_template_param(template: &str) -> Result<()> {
    if template.is_empty() {
        return Err(anyhow!("template parameter cannot be empty"));
    }
    if template.len() > MAX_TEMPLATE_NAME_LEN && !template.starts_with("inline:") {
        return Err(anyhow!(
            "template name exceeds maximum length of {}",
            MAX_TEMPLATE_NAME_LEN
        ));
    }
    Ok(())
}

/// Validate context parameter
fn validate_context_param(context: &JsonValue) -> Result<()> {
    if !context.is_object() {
        return Err(anyhow!("context must be a JSON object"));
    }
    Ok(())
}

/// Build template context from JSON value
fn build_template_context(params: &RenderTemplateParams) -> Result<crate::template::RenderContext> {
    let mut context = crate::template::RenderContext::new();

    // Extract object fields and insert into context
    if let JsonValue::Object(obj) = &params.context {
        for (key, value) in obj {
            context
                .insert(key, &value)
                .with_context(|| format!("Failed to insert context variable '{}'", key))?;
        }
    }

    Ok(context)
}

/// Validate output format
fn validate_output_format(output: &str, format: &str) -> Result<Vec<String>> {
    let mut warnings = Vec::new();

    match format.to_lowercase().as_str() {
        "rust" => {
            // Basic Rust syntax validation
            if let Err(e) = syn::parse_file(output) {
                warnings.push(format!("Rust syntax validation warning: {}", e));
            }
        }
        "json" => {
            // JSON validation
            if let Err(e) = serde_json::from_str::<JsonValue>(output) {
                warnings.push(format!("JSON validation warning: {}", e));
            }
        }
        "yaml" => {
            // YAML validation
            if let Err(e) = serde_yaml::from_str::<serde_yaml::Value>(output) {
                warnings.push(format!("YAML validation warning: {}", e));
            }
        }
        "toml" => {
            // TOML validation - basic check for common syntax issues
            if output.contains("[[") && !output.contains("]]") {
                warnings.push("TOML: unbalanced double brackets detected".to_string());
            }
            if output.contains("[") && output.matches('[').count() != output.matches(']').count() {
                warnings.push("TOML: unbalanced brackets detected".to_string());
            }
        }
        "typescript" | "ts" => {
            // Basic TypeScript validation (check for common syntax issues)
            if output.contains(";;") {
                warnings.push("TypeScript: consecutive semicolons detected".to_string());
            }
        }
        _ => {
            warnings.push(format!("Unknown output format '{}', skipping validation", format));
        }
    }

    Ok(warnings)
}

// ============================================================================
// MCP Tool: write_generated_artifact
// ============================================================================

/// Parameters for write_generated_artifact tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WriteGeneratedArtifactParams {
    /// Content to write
    pub content: String,

    /// Output file path
    pub output_path: String,

    /// Create backup before overwriting (default: true)
    #[serde(default = "default_true")]
    pub create_backup: Option<bool>,

    /// Ontology hash (for provenance tracking)
    #[serde(default)]
    pub ontology_hash: Option<String>,

    /// Template hash (for provenance tracking)
    #[serde(default)]
    pub template_hash: Option<String>,

    /// Additional metadata for generation receipt
    #[serde(default)]
    pub metadata: Option<serde_json::Map<String, JsonValue>>,

    /// Preview mode - validate but don't write (default: false)
    #[serde(default)]
    pub preview: bool,
}

/// Response from write_generated_artifact tool
#[derive(Debug, Serialize, JsonSchema)]
pub struct WriteGeneratedArtifactResponse {
    /// Output file path
    pub output_path: String,

    /// Whether the file was written (false in preview mode)
    pub written: bool,

    /// SHA-256 hash of the content
    pub content_hash: String,

    /// Generation receipt ID
    pub receipt_id: String,

    /// Backup path (if created)
    pub backup_path: Option<String>,

    /// Content size in bytes
    pub size: usize,

    /// Preview mode indicator
    pub preview: bool,
}

/// Write generated code to file with validation and audit trail
pub async fn write_generated_artifact(
    _state: Arc<AppState>,
    params: WriteGeneratedArtifactParams,
) -> Result<WriteGeneratedArtifactResponse> {
    let _span = audit_tool("write_generated_artifact", &params);

    // Validate parameters
    validate_output_path(&params.output_path)?;
    validate_content(&params.content)?;

    // Compute content hash
    let content_hash = compute_string_hash(&params.content);

    // Create hashes for receipt
    let ontology_hash = params
        .ontology_hash
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let template_hash = params
        .template_hash
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    // Create generation receipt
    let mut receipt = GenerationReceipt::new(
        ontology_hash.clone(),
        template_hash.clone(),
        content_hash.clone(),
    );

    // Add metadata to receipt
    if let Some(metadata) = &params.metadata {
        for (key, value) in metadata {
            if let Some(s) = value.as_str() {
                receipt.add_metadata(key.clone(), s.to_string());
            }
        }
    }

    let receipt_id = receipt.receipt_id.clone();

    // Convert output_path to PathBuf
    let output_path = PathBuf::from(&params.output_path);

    let backup_path = if !params.preview {
        // Create safe code writer
        let mut writer = SafeCodeWriter::new();
        writer.create_backups = params.create_backup.unwrap_or(true);

        // Determine backup path
        let backup_path_result = if output_path.exists() && writer.create_backups {
            Some(output_path.with_extension("bak"))
        } else {
            None
        };

        // Write file
        writer
            .write(&output_path, &params.content)
            .context("Failed to write generated artifact")?;

        // Save receipt
        let receipt_path = output_path.with_extension("receipt.json");
        receipt
            .save(&receipt_path)
            .context("Failed to save generation receipt")?;

        tracing::info!(
            "Wrote generated artifact to {:?} (size: {} bytes, hash: {})",
            output_path,
            params.content.len(),
            content_hash
        );

        backup_path_result.map(|p| p.to_string_lossy().to_string())
    } else {
        None
    };

    Ok(WriteGeneratedArtifactResponse {
        output_path: params.output_path,
        written: !params.preview,
        content_hash,
        receipt_id,
        backup_path,
        size: params.content.len(),
        preview: params.preview,
    })
}

/// Validate output path
fn validate_output_path(path: &str) -> Result<()> {
    if path.is_empty() {
        return Err(anyhow!("output_path cannot be empty"));
    }
    if path.len() > MAX_OUTPUT_PATH_LEN {
        return Err(anyhow!(
            "output_path exceeds maximum length of {}",
            MAX_OUTPUT_PATH_LEN
        ));
    }
    // Check for path traversal
    if path.contains("..") {
        return Err(anyhow!("path traversal not allowed in output_path"));
    }
    Ok(())
}

/// Validate content
fn validate_content(content: &str) -> Result<()> {
    if content.is_empty() {
        return Err(anyhow!("content cannot be empty"));
    }
    Ok(())
}

// =============================================================================
// MCP Tool: validate_generated_code
// =============================================================================

/// Parameters for validate_generated_code tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ValidateGeneratedCodeParams {
    /// Generated code content to validate
    pub code: String,

    /// Programming language (rust, typescript, yaml, json)
    pub language: String,

    /// File name for context in error messages
    pub file_name: String,

    /// Optional path to golden file for comparison
    pub golden_file_path: Option<String>,

    /// Strict mode: fail on warnings
    #[serde(default)]
    pub strict_mode: bool,

    /// Update golden file if UPDATE_GOLDEN env var is set
    #[serde(default)]
    pub allow_golden_update: bool,
}

/// Response from validate_generated_code tool
#[derive(Debug, Serialize, JsonSchema)]
pub struct ValidateGeneratedCodeResponse {
    /// Whether validation passed
    pub valid: bool,

    /// Validation errors (syntax, semantics)
    pub errors: Vec<ValidationError>,

    /// Warnings (style, conventions)
    pub warnings: Vec<String>,

    /// Suggestions for improvement
    pub suggestions: Vec<String>,

    /// Golden file diff if comparison was performed
    pub golden_file_diff: Option<GoldenFileDiff>,

    /// Language that was validated
    pub language: String,

    /// Summary message
    pub summary: String,
}

/// Validation error with location and suggestion
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ValidationError {
    /// Error message
    pub message: String,

    /// File location (file:line:col)
    pub location: Option<String>,

    /// Suggestion for fixing
    pub suggestion: Option<String>,
}

/// Golden file diff report
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct GoldenFileDiff {
    /// Path to golden file
    pub golden_file: String,

    /// Number of lines added
    pub additions: usize,

    /// Number of lines deleted
    pub deletions: usize,

    /// Number of lines changed
    pub changes: usize,

    /// Whether files are identical
    pub is_identical: bool,

    /// Sample of diff lines (up to 20 lines)
    pub diff_sample: Vec<String>,
}

impl From<DiffReport> for GoldenFileDiff {
    fn from(report: DiffReport) -> Self {
        let diff_sample = report.diff_lines
            .into_iter()
            .take(20)
            .map(|line| {
                format!("{:4} {}", line.line_num, line.content)
            })
            .collect();

        Self {
            golden_file: report.golden_file.to_string_lossy().to_string(),
            additions: report.additions,
            deletions: report.deletions,
            changes: report.changes,
            is_identical: report.is_identical,
            diff_sample,
        }
    }
}

/// Validate generated code with multi-language support
pub async fn validate_generated_code(
    params: ValidateGeneratedCodeParams,
) -> Result<ValidateGeneratedCodeResponse> {
    // 1. Validate syntax based on language
    let mut report = match params.language.to_lowercase().as_str() {
        "rust" | "rs" => validate_rust_code(&params.code, &params.file_name)?,
        "typescript" | "ts" | "javascript" | "js" | "mjs" => {
            validate_typescript(&params.code, &params.file_name)
        }
        "yaml" | "yml" => {
            validate_yaml(&params.code, &params.file_name)
        }
        "json" => {
            validate_json(&params.code, &params.file_name)
        }
        other => {
            return Err(anyhow!("Unsupported language: {}. Supported: rust, typescript, yaml, json", other));
        }
    };

    // 2. Golden file comparison if path provided
    let golden_diff = if let Some(golden_path_str) = &params.golden_file_path {
        let golden_path = PathBuf::from(golden_path_str);

        match load_golden_file(&golden_path)? {
            Some(golden_content) => {
                let diff = compute_diff(&params.code, &golden_content, &golden_path);

                // Check if we should update golden file
                let should_update = params.allow_golden_update &&
                    std::env::var("UPDATE_GOLDEN").is_ok();

                if should_update && !diff.is_identical {
                    update_golden_file(&golden_path, &params.code)?;
                    report.warnings.push(format!("Updated golden file: {}", golden_path_str));
                } else if !diff.is_identical {
                    report.warnings.push(format!(
                        "Generated code differs from golden file: {} additions, {} deletions, {} changes",
                        diff.additions, diff.deletions, diff.changes
                    ));
                }

                Some(diff)
            }
            None => {
                // Golden file doesn't exist
                if params.allow_golden_update && std::env::var("UPDATE_GOLDEN").is_ok() {
                    update_golden_file(&golden_path, &params.code)?;
                    report.warnings.push(format!("Created new golden file: {}", golden_path_str));
                } else {
                    report.warnings.push(format!("Golden file not found: {}", golden_path_str));
                }
                None
            }
        }
    } else {
        None
    };

    // 3. Build response
    let errors: Vec<ValidationError> = report.errors.iter().map(|err| {
        ValidationError {
            message: err.to_string(),
            location: None,
            suggestion: None,
        }
    }).collect();

    let valid = if params.strict_mode {
        errors.is_empty() && report.warnings.is_empty()
    } else {
        errors.is_empty()
    };

    let summary = if valid {
        if let Some(ref diff) = golden_diff {
            if diff.is_identical {
                format!("✓ Validation passed. Code matches golden file.")
            } else {
                format!("✓ Validation passed. Code differs from golden file ({} changes).",
                    diff.additions + diff.deletions + diff.changes)
            }
        } else {
            format!("✓ Validation passed for {} code.", params.language)
        }
    } else {
        format!("✗ Validation failed: {} errors, {} warnings.",
            errors.len(), report.warnings.len())
    };

    Ok(ValidateGeneratedCodeResponse {
        valid,
        errors,
        warnings: report.warnings,
        suggestions: report.suggestions,
        golden_file_diff: golden_diff.map(Into::into),
        language: params.language,
        summary,
    })
}

/// Validate Rust code using syn parser
fn validate_rust_code(code: &str, file_name: &str) -> Result<CodeValidationReport> {
    let mut validator = GeneratedCodeValidator::new();
    validator.allow_unsafe = false;
    validator.require_doc_comments = false; // Relaxed for generated code
    validator.max_line_length = 120;

    let validation_report = validator.validate_code(code, file_name)?;

    let report = CodeValidationReport::from_validation_report(validation_report, "rust".to_string());

    Ok(report)
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Determine golden file path from generated file path
pub fn golden_file_path(generated_path: &Path) -> PathBuf {
    // Convert src/generated/foo.rs -> tests/golden/foo.rs
    let file_name = generated_path.file_name().unwrap_or_default();
    PathBuf::from("tests/golden").join(file_name)
}

/// Check if golden file update is enabled
pub fn is_golden_update_enabled() -> bool {
    std::env::var("UPDATE_GOLDEN").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validate_rust_code() {
        let params = ValidateGeneratedCodeParams {
            code: "pub struct Test { pub field: String }".to_string(),
            language: "rust".to_string(),
            file_name: "test.rs".to_string(),
            golden_file_path: None,
            strict_mode: false,
            allow_golden_update: false,
        };

        let result = validate_generated_code(params).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.valid);
    }

    #[tokio::test]
    async fn test_validate_invalid_rust() {
        let params = ValidateGeneratedCodeParams {
            code: "pub struct Test {".to_string(), // Missing closing brace
            language: "rust".to_string(),
            file_name: "test.rs".to_string(),
            golden_file_path: None,
            strict_mode: false,
            allow_golden_update: false,
        };

        let result = validate_generated_code(params).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.valid);
        assert!(!response.errors.is_empty());
    }

    #[tokio::test]
    async fn test_validate_json() {
        let params = ValidateGeneratedCodeParams {
            code: r#"{"name": "test", "value": 42}"#.to_string(),
            language: "json".to_string(),
            file_name: "test.json".to_string(),
            golden_file_path: None,
            strict_mode: false,
            allow_golden_update: false,
        };

        let result = validate_generated_code(params).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.valid);
    }

    #[tokio::test]
    async fn test_validate_invalid_json() {
        let params = ValidateGeneratedCodeParams {
            code: r#"{"name": "test", "value": 42,}"#.to_string(), // Trailing comma
            language: "json".to_string(),
            file_name: "test.json".to_string(),
            golden_file_path: None,
            strict_mode: false,
            allow_golden_update: false,
        };

        let result = validate_generated_code(params).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.valid);
    }

    #[tokio::test]
    async fn test_validate_yaml() {
        let params = ValidateGeneratedCodeParams {
            code: "name: test\nvalue: 42\n".to_string(),
            language: "yaml".to_string(),
            file_name: "test.yaml".to_string(),
            golden_file_path: None,
            strict_mode: false,
            allow_golden_update: false,
        };

        let result = validate_generated_code(params).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.valid);
    }

    #[tokio::test]
    async fn test_validate_typescript() {
        let params = ValidateGeneratedCodeParams {
            code: "export interface Test { name: string; }".to_string(),
            language: "typescript".to_string(),
            file_name: "test.ts".to_string(),
            golden_file_path: None,
            strict_mode: false,
            allow_golden_update: false,
        };

        let result = validate_generated_code(params).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.valid);
    }

    #[test]
    fn test_golden_file_path() {
        let gen_path = PathBuf::from("src/generated/entities.rs");
        let golden = golden_file_path(&gen_path);
        assert_eq!(golden, PathBuf::from("tests/golden/entities.rs"));
    }

    #[test]
    fn test_validate_template_param() {
        assert!(validate_template_param("valid.rs.tera").is_ok());
        assert!(validate_template_param("inline:{{ value }}").is_ok());
        assert!(validate_template_param("").is_err());
        assert!(validate_template_param(&"x".repeat(300)).is_err());
    }

    #[test]
    fn test_validate_context_param() {
        let valid = serde_json::json!({"key": "value"});
        assert!(validate_context_param(&valid).is_ok());

        let invalid = serde_json::json!("not an object");
        assert!(validate_context_param(&invalid).is_err());
    }

    #[test]
    fn test_validate_output_path() {
        assert!(validate_output_path("src/generated/test.rs").is_ok());
        assert!(validate_output_path("").is_err());
        assert!(validate_output_path(&"x".repeat(2000)).is_err());
        assert!(validate_output_path("../etc/passwd").is_err());
    }

    #[test]
    fn test_validate_content() {
        assert!(validate_content("pub fn test() {}").is_ok());
        assert!(validate_content("").is_err());
    }

    #[test]
    fn test_validate_output_format_json() {
        let valid_json = r#"{"key": "value"}"#;
        let warnings = validate_output_format(valid_json, "json").unwrap();
        assert!(warnings.is_empty());

        let invalid_json = r#"{"key": invalid}"#;
        let warnings = validate_output_format(invalid_json, "json").unwrap();
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_validate_output_format_rust() {
        let valid_rust = "fn main() {}";
        let warnings = validate_output_format(valid_rust, "rust").unwrap();
        assert!(warnings.is_empty());

        let invalid_rust = "fn main() {";
        let warnings = validate_output_format(invalid_rust, "rust").unwrap();
        assert!(!warnings.is_empty());
    }
}
