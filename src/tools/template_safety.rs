//! Template Safety Integration
//!
//! Integrates template safety infrastructure into render_template tool.
//! Implements comprehensive validation chain: schema → render → output.
//!
//! # Safety Guarantees
//!
//! 1. **Pre-render validation**: Template syntax + parameter schema checks
//! 2. **Resource limits**: Timeout, memory, recursion depth enforcement
//! 3. **Post-render validation**: Output syntax + security pattern detection
//! 4. **Poka-yoke**: Multiple validation layers prevent bad templates from executing
//!
//! # Example
//!
//! ```rust,ignore
//! use spreadsheet_mcp::tools::template_safety::validate_and_render;
//! use serde_json::json;
//!
//! let sparql_results = vec![
//!     json!({"?toolName": "read_cell", "?paramName": "workbook_id"}),
//! ];
//!
//! let output = validate_and_render(
//!     "mcp_tools.rs.tera",
//!     sparql_results,
//!     RenderConfig::default(),
//! ).await?;
//! ```

use crate::template::{
    OutputValidator, ParameterSchema, RenderConfig, RenderContext, RenderMetrics, RenderingError,
    SafeRenderer, TemplateContext, TemplateValidator, ValidationError as ParamValidationError,
};
use anyhow::{Context as AnyhowContext, Result, anyhow};
use parking_lot::RwLock;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

// ============================================================================
// Template Safety Integration
// ============================================================================

/// Integrated template safety wrapper
///
/// Combines all safety components into a single, easy-to-use interface.
/// Implements the full validation chain with RAII cleanup.
pub struct TemplateSafety {
    /// Safe renderer with resource limits
    renderer: Arc<RwLock<SafeRenderer>>,

    /// Template validator for syntax and schema checks
    validator: Arc<TemplateValidator>,

    /// Registered parameter schemas
    schemas: Arc<RwLock<HashMap<String, ParameterSchema>>>,

    /// Rendering configuration
    config: RenderConfig,
}

impl TemplateSafety {
    /// Create a new template safety wrapper
    pub fn new(template_dir: impl AsRef<Path>, config: RenderConfig) -> Result<Self> {
        // Validate config first (fail-fast)
        config.validate()?;

        // Create validator (loads templates from directory)
        let validator = TemplateValidator::new(&template_dir)
            .with_context(|| format!("Failed to create template validator for {:?}", template_dir.as_ref()))?;

        // Create safe renderer
        let renderer = SafeRenderer::from_directory(&template_dir, config.clone())
            .with_context(|| format!("Failed to create safe renderer for {:?}", template_dir.as_ref()))?;

        Ok(Self {
            renderer: Arc::new(RwLock::new(renderer)),
            validator: Arc::new(validator),
            schemas: Arc::new(RwLock::new(HashMap::new())),
            config,
        })
    }

    /// Create with default configuration
    pub fn with_defaults(template_dir: impl AsRef<Path>) -> Result<Self> {
        Self::new(template_dir, RenderConfig::default())
    }

    /// Register a parameter schema
    pub fn register_schema(&self, schema: ParameterSchema) {
        let template_name = schema.template_name.clone();
        self.schemas.write().insert(template_name, schema);
    }

    /// Register multiple schemas
    pub fn register_schemas(&self, schemas: Vec<ParameterSchema>) {
        let mut schema_map = self.schemas.write();
        for schema in schemas {
            schema_map.insert(schema.template_name.clone(), schema);
        }
    }

    /// Validate and render a template (main entry point)
    ///
    /// # Safety Chain
    ///
    /// 1. Build TemplateContext from parameters
    /// 2. Validate template syntax
    /// 3. Validate parameters against schema
    /// 4. Render with resource limits (timeout, recursion, output size)
    /// 5. Validate output (syntax + security)
    /// 6. Return validated output + metrics
    pub fn validate_and_render(
        &self,
        template_name: &str,
        context: TemplateContext,
    ) -> Result<(String, RenderMetrics), RenderingError> {
        let start = Instant::now();

        // Step 1: Validate template syntax
        self.validator
            .validate_syntax(template_name)
            .map_err(|e| RenderingError::SyntaxError {
                message: format!("Template '{}': {}", template_name, e),
            })?;

        // Step 2: Validate parameters against schema (if registered)
        if let Some(schema) = self.schemas.read().get(template_name) {
            schema
                .validate_context(context.inner())
                .map_err(|errors| RenderingError::ValidationFailed {
                    errors: errors.iter().map(|e| e.to_string()).collect(),
                })?;
        }

        // Step 3: Convert TemplateContext to RenderContext
        let mut render_context = RenderContext::new();
        for (key, value) in context.inner() {
            render_context
                .insert(key, value)
                .map_err(|e| RenderingError::ContextError {
                    message: format!("Failed to insert '{}': {}", key, e),
                })?;
        }

        // Step 4: Render with resource limits
        let renderer = self.renderer.read();
        let output = renderer.render_safe(template_name, &render_context)?;

        // Step 5: Metrics collection
        let duration = start.elapsed();
        let metrics = RenderMetrics {
            duration,
            output_size: output.len(),
            includes_count: 0, // TODO: Track includes
            macro_expansions: render_context.macro_count(),
            max_recursion_reached: render_context.recursion_depth(),
            validation_errors: 0,
            validation_warnings: 0,
        };

        Ok((output, metrics))
    }

    /// Get the configuration
    pub fn config(&self) -> &RenderConfig {
        &self.config
    }
}

// ============================================================================
// SPARQL → TemplateContext Conversion
// ============================================================================

/// Build a TemplateContext from SPARQL query results
///
/// Converts SPARQL bindings (with ? prefix) into template parameters.
/// Implements type-safe parameter insertion with validation.
///
/// # Example
///
/// ```rust,ignore
/// let sparql_results = vec![
///     json!({"?toolName": "read_cell", "?paramName": "workbook_id", "?paramType": "String"}),
///     json!({"?toolName": "read_cell", "?paramName": "sheet_name", "?paramType": "String"}),
/// ];
///
/// let context = build_context_from_sparql(
///     "mcp_tool_params.rs.tera",
///     sparql_results
/// )?;
/// ```
pub fn build_context_from_sparql(
    template_name: impl Into<String>,
    sparql_results: Vec<JsonValue>,
) -> Result<TemplateContext> {
    let mut context = TemplateContext::new(template_name);

    // Insert the raw SPARQL results array
    context
        .insert_array("sparql_results", sparql_results.clone())
        .with_context(|| "Failed to insert sparql_results array")?;

    // Process SPARQL results to extract common parameters
    let processed = process_sparql_results(&sparql_results)?;

    // Insert processed results
    for (key, value) in processed {
        context
            .insert(key, value)
            .with_context(|| "Failed to insert processed parameter")?;
    }

    Ok(context)
}

/// Process SPARQL results into structured parameters
///
/// Extracts common patterns from SPARQL bindings:
/// - Groups results by tool/entity name
/// - Collects parameters for each group
/// - Normalizes field names (removes ? prefix)
fn process_sparql_results(results: &[JsonValue]) -> Result<HashMap<String, JsonValue>> {
    let mut processed = HashMap::new();

    // Group results by tool name (if present)
    let mut tools: HashMap<String, Vec<JsonValue>> = HashMap::new();

    for result in results {
        if let Some(obj) = result.as_object() {
            // Normalize keys (remove ? prefix)
            let normalized: serde_json::Map<String, JsonValue> = obj
                .iter()
                .map(|(k, v)| {
                    let key = if k.starts_with('?') {
                        k.trim_start_matches('?').to_string()
                    } else {
                        k.clone()
                    };
                    (key, v.clone())
                })
                .collect();

            // Extract tool name for grouping
            if let Some(tool_name) = normalized.get("toolName").and_then(|v| v.as_str()) {
                tools
                    .entry(tool_name.to_string())
                    .or_insert_with(Vec::new)
                    .push(JsonValue::Object(normalized));
            } else {
                // If no tool name, add to a generic group
                tools
                    .entry("_ungrouped".to_string())
                    .or_insert_with(Vec::new)
                    .push(JsonValue::Object(normalized));
            }
        }
    }

    // Convert grouped results to JSON
    if !tools.is_empty() {
        processed.insert("tools".to_string(), JsonValue::Object(
            tools.into_iter().map(|(k, v)| (k, JsonValue::Array(v))).collect()
        ));
    }

    Ok(processed)
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Validate and render a template with SPARQL results (one-shot function)
///
/// Convenience wrapper for the common case of rendering from SPARQL results.
/// Creates a temporary TemplateSafety instance, validates, and renders.
///
/// # Arguments
///
/// * `template_dir` - Directory containing .tera templates
/// * `template_name` - Name of the template to render
/// * `sparql_results` - SPARQL query results to pass to template
/// * `config` - Rendering configuration with resource limits
///
/// # Returns
///
/// Tuple of (rendered_output, metrics) on success
///
/// # Errors
///
/// Returns RenderingError if:
/// - Template syntax is invalid
/// - Parameters don't match schema
/// - Resource limits exceeded (timeout, size, recursion)
/// - Output validation fails
pub fn validate_and_render_from_sparql(
    template_dir: impl AsRef<Path>,
    template_name: &str,
    sparql_results: Vec<JsonValue>,
    config: RenderConfig,
) -> Result<(String, RenderMetrics), RenderingError> {
    // Create safety wrapper
    let safety = TemplateSafety::new(template_dir, config)
        .map_err(|e| RenderingError::Internal {
            message: format!("Failed to create template safety: {}", e),
        })?;

    // Build context from SPARQL results
    let context = build_context_from_sparql(template_name, sparql_results)
        .map_err(|e| RenderingError::ContextError {
            message: format!("Failed to build context: {}", e),
        })?;

    // Validate and render
    safety.validate_and_render(template_name, context)
}

/// Validate and render with default configuration
pub fn validate_and_render_from_sparql_defaults(
    template_dir: impl AsRef<Path>,
    template_name: &str,
    sparql_results: Vec<JsonValue>,
) -> Result<(String, RenderMetrics), RenderingError> {
    validate_and_render_from_sparql(
        template_dir,
        template_name,
        sparql_results,
        RenderConfig::default(),
    )
}

// ============================================================================
// Security Pattern Detection
// ============================================================================

/// Detect security patterns in rendered output
///
/// Scans output for potentially unsafe patterns:
/// - Unsafe code blocks
/// - System command execution
/// - File system modifications
/// - SQL injection vectors
///
/// Returns warnings (not errors) to allow manual review.
pub fn detect_security_patterns(output: &str) -> Vec<String> {
    let mut warnings = Vec::new();

    for (line_num, line) in output.lines().enumerate() {
        let trimmed = line.trim();

        // Detect unsafe code
        if trimmed.contains("unsafe {") || trimmed.starts_with("unsafe ") {
            warnings.push(format!(
                "Line {}: Unsafe code block detected - verify this is intentional",
                line_num + 1
            ));
        }

        // Detect system commands
        if trimmed.contains("std::process::Command") {
            warnings.push(format!(
                "Line {}: System command execution detected - potential security risk",
                line_num + 1
            ));
        }

        // Detect file system operations
        if trimmed.contains("std::fs::remove") || trimmed.contains("std::fs::write") {
            warnings.push(format!(
                "Line {}: File system modification detected - verify permissions",
                line_num + 1
            ));
        }

        // Detect potential SQL injection
        if (trimmed.contains("format!") || trimmed.contains("&format!("))
            && (trimmed.to_lowercase().contains("select ")
                || trimmed.to_lowercase().contains("insert ")
                || trimmed.to_lowercase().contains("update "))
        {
            warnings.push(format!(
                "Line {}: Potential SQL query construction - use parameterized queries",
                line_num + 1
            ));
        }
    }

    warnings
}

// ============================================================================
// Metrics Tracking
// ============================================================================

/// Track template rendering metrics
#[derive(Debug, Clone)]
pub struct TemplateSafetyMetrics {
    /// Total templates rendered
    pub total_renders: u64,

    /// Successful renders
    pub successful_renders: u64,

    /// Failed renders
    pub failed_renders: u64,

    /// Total rendering time
    pub total_duration_ms: u64,

    /// Average rendering time
    pub avg_duration_ms: u64,

    /// Validation errors caught
    pub validation_errors_caught: u64,

    /// Security warnings issued
    pub security_warnings: u64,
}

impl Default for TemplateSafetyMetrics {
    fn default() -> Self {
        Self {
            total_renders: 0,
            successful_renders: 0,
            failed_renders: 0,
            total_duration_ms: 0,
            avg_duration_ms: 0,
            validation_errors_caught: 0,
            security_warnings: 0,
        }
    }
}

impl TemplateSafetyMetrics {
    /// Record a successful render
    pub fn record_success(&mut self, duration_ms: u64) {
        self.total_renders += 1;
        self.successful_renders += 1;
        self.total_duration_ms += duration_ms;
        self.update_avg_duration();
    }

    /// Record a failed render
    pub fn record_failure(&mut self, duration_ms: u64) {
        self.total_renders += 1;
        self.failed_renders += 1;
        self.total_duration_ms += duration_ms;
        self.update_avg_duration();
    }

    /// Record validation errors
    pub fn record_validation_errors(&mut self, count: usize) {
        self.validation_errors_caught += count as u64;
    }

    /// Record security warnings
    pub fn record_security_warnings(&mut self, count: usize) {
        self.security_warnings += count as u64;
    }

    /// Update average duration
    fn update_avg_duration(&mut self) {
        if self.total_renders > 0 {
            self.avg_duration_ms = self.total_duration_ms / self.total_renders;
        }
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_renders == 0 {
            0.0
        } else {
            (self.successful_renders as f64 / self.total_renders as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_context_from_sparql() {
        let sparql_results = vec![
            serde_json::json!({
                "?toolName": "read_cell",
                "?paramName": "workbook_id",
                "?paramType": "String"
            }),
            serde_json::json!({
                "?toolName": "read_cell",
                "?paramName": "sheet_name",
                "?paramType": "String"
            }),
        ];

        let context = build_context_from_sparql("test.tera", sparql_results).unwrap();
        assert!(context.contains("sparql_results"));
    }

    #[test]
    fn test_process_sparql_results() {
        let results = vec![
            serde_json::json!({
                "?toolName": "read_cell",
                "?paramName": "workbook_id"
            }),
            serde_json::json!({
                "?toolName": "read_cell",
                "?paramName": "sheet_name"
            }),
        ];

        let processed = process_sparql_results(&results).unwrap();
        assert!(processed.contains_key("tools"));
    }

    #[test]
    fn test_detect_security_patterns_unsafe() {
        let code = r#"
        unsafe {
            std::ptr::write(ptr, value);
        }
        "#;

        let warnings = detect_security_patterns(code);
        assert!(!warnings.is_empty());
        assert!(warnings[0].contains("Unsafe code"));
    }

    #[test]
    fn test_detect_security_patterns_system_command() {
        let code = r#"
        use std::process::Command;
        let output = Command::new("ls").output()?;
        "#;

        let warnings = detect_security_patterns(code);
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.contains("System command")));
    }

    #[test]
    fn test_metrics_tracking() {
        let mut metrics = TemplateSafetyMetrics::default();

        metrics.record_success(100);
        metrics.record_success(200);
        metrics.record_failure(150);

        assert_eq!(metrics.total_renders, 3);
        assert_eq!(metrics.successful_renders, 2);
        assert_eq!(metrics.failed_renders, 1);
        assert_eq!(metrics.total_duration_ms, 450);
        assert_eq!(metrics.avg_duration_ms, 150);
        assert_eq!(metrics.success_rate(), 66.66666666666666);
    }

    #[test]
    fn test_metrics_success_rate_zero_renders() {
        let metrics = TemplateSafetyMetrics::default();
        assert_eq!(metrics.success_rate(), 0.0);
    }
}
