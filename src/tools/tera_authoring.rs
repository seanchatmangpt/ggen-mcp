//! Tera Template Authoring Tools
//!
//! MCP tools for:
//! - Reading and analyzing Tera templates
//! - Validating template syntax and structure
//! - Testing templates with sample context
//! - Creating scaffolded templates from patterns
//! - Extracting template variables and metadata

use crate::error::{ErrorCode, McpError};
use crate::state::AppState;
use crate::template::{RenderConfig, RenderContext, SafeRenderer};
use anyhow::{Context, Result, anyhow};
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tera::Tera;

// ============================================================================
// Constants
// ============================================================================

const MAX_TEMPLATE_SIZE: usize = 1024 * 1024; // 1MB
const MAX_TEMPLATE_NAME_LEN: usize = 256;
const DEFAULT_RENDER_TIMEOUT_MS: u64 = 5000;

// ============================================================================
// Template Helpers (Embedded Library)
// ============================================================================

/// Built-in template library for common patterns
pub struct TemplateLibrary;

impl TemplateLibrary {
    /// Get template content by name
    pub fn get(name: &str) -> Option<&'static str> {
        match name {
            "struct" | "struct.rs" => Some(TEMPLATE_RUST_STRUCT),
            "endpoint" | "endpoint.rs" => Some(TEMPLATE_RUST_ENDPOINT),
            "schema" | "schema.yaml" => Some(TEMPLATE_OPENAPI_SCHEMA),
            "interface" | "interface.ts" => Some(TEMPLATE_TYPESCRIPT_INTERFACE),
            _ => None,
        }
    }

    /// List all available template names
    pub fn list() -> Vec<&'static str> {
        vec!["struct.rs", "endpoint.rs", "schema.yaml", "interface.ts"]
    }
}

// Rust struct template
const TEMPLATE_RUST_STRUCT: &str = r#"/// {{ description }}
#[derive(Debug, Clone{% if serde %}, Serialize, Deserialize{% endif %}{% if schema %}, JsonSchema{% endif %})]
pub struct {{ struct_name }} {
    {% for field in fields %}
    /// {{ field.description | default(value="Field") }}
    pub {{ field.name }}: {{ field.type_name }},
    {% endfor %}
}

impl {{ struct_name }} {
    /// Creates a new {{ struct_name }}
    pub fn new({% for field in fields %}{{ field.name }}: {{ field.type_name }}{% if not loop.last %}, {% endif %}{% endfor %}) -> Self {
        Self {
            {% for field in fields %}
            {{ field.name }},
            {% endfor %}
        }
    }
    {% if validation %}

    /// Validates the struct invariants
    pub fn validate(&self) -> Result<(), String> {
        {% for field in fields %}
        {% if field.required %}
        if self.{{ field.name }}.is_empty() {
            return Err("{{ field.name }} cannot be empty".to_string());
        }
        {% endif %}
        {% endfor %}
        Ok(())
    }
    {% endif %}
}
"#;

// API endpoint template
const TEMPLATE_RUST_ENDPOINT: &str = r#"/// {{ description }}
///
/// Method: {{ method | upper }}
/// Path: {{ path }}
pub async fn {{ handler_name }}(
    state: Arc<AppState>,
    params: {{ params_type }},
) -> Result<{{ response_type }}, McpError> {
    tracing::info!("Handling {{ handler_name }} request");

    // Validate input
    {% for param in required_params %}
    if params.{{ param }}.is_empty() {
        return Err(McpError::validation()
            .message("{{ param }} is required")
            .param("{{ param }}", &params.{{ param }})
            .build());
    }
    {% endfor %}

    // Implementation
    {% if implementation %}
    {{ implementation }}
    {% else %}
    // TODO: Implement {{ handler_name }} logic
    {% endif %}

    Ok({{ response_type }} {
        // TODO: Build response
    })
}
"#;

// OpenAPI schema template
const TEMPLATE_OPENAPI_SCHEMA: &str = r#"{{ schema_name }}:
  type: object
  description: {{ description }}
  {% if required %}
  required:
    {% for field in required %}
    - {{ field }}
    {% endfor %}
  {% endif %}
  properties:
    {% for field in fields %}
    {{ field.name }}:
      type: {{ field.type }}
      description: {{ field.description | default(value="Field") }}
      {% if field.format %}
      format: {{ field.format }}
      {% endif %}
      {% if field.example %}
      example: {{ field.example }}
      {% endif %}
    {% endfor %}
"#;

// TypeScript interface template
const TEMPLATE_TYPESCRIPT_INTERFACE: &str = r#"/**
 * {{ description }}
 */
export interface {{ interface_name }} {
  {% for field in fields %}
  /** {{ field.description | default(value="Field") }} */
  {{ field.name }}{% if not field.required %}?{% endif %}: {{ field.type_name }};
  {% endfor %}
}

{% if validators %}
/**
 * Validates {{ interface_name }} instance
 */
export function validate{{ interface_name }}(obj: {{ interface_name }}): string[] {
  const errors: string[] = [];
  {% for field in fields %}
  {% if field.required %}
  if (!obj.{{ field.name }}) {
    errors.push('{{ field.name }} is required');
  }
  {% endif %}
  {% endfor %}
  return errors;
}
{% endif %}
"#;

// ============================================================================
// MCP Tool: read_tera_template
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReadTeraTemplateParams {
    /// Template name or inline content (use "inline:<content>")
    pub template: String,

    /// Analyze variables (default: true)
    #[serde(default = "default_true")]
    pub analyze_variables: bool,

    /// Analyze filters (default: true)
    #[serde(default = "default_true")]
    pub analyze_filters: bool,

    /// Analyze control structures (default: true)
    #[serde(default = "default_true")]
    pub analyze_structures: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ReadTeraTemplateResponse {
    /// Template content
    pub content: String,

    /// Template size in bytes
    pub size: usize,

    /// Variables used ({{ var }})
    pub variables: Vec<String>,

    /// Filters used ({{ var | filter }})
    pub filters: Vec<String>,

    /// Control structures found
    pub structures: Vec<ControlStructure>,

    /// Template blocks ({% block name %})
    pub blocks: Vec<String>,

    /// Include directives
    pub includes: Vec<String>,

    /// Macro definitions
    pub macros: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ControlStructure {
    /// Structure type (if, for, block, macro, etc.)
    pub kind: String,

    /// Line number where structure starts
    pub line: usize,

    /// Structure details (condition, loop variable, etc.)
    pub details: String,
}

fn default_true() -> bool {
    true
}

/// Read and analyze a Tera template
pub async fn read_tera_template(
    _state: Arc<AppState>,
    params: ReadTeraTemplateParams,
) -> Result<ReadTeraTemplateResponse, McpError> {
    validate_template_param(&params.template)?;

    // Load template content
    let content = if params.template.starts_with("inline:") {
        params.template[7..].to_string()
    } else if let Some(builtin) = TemplateLibrary::get(&params.template) {
        builtin.to_string()
    } else {
        // Try to load from templates directory
        load_template_file(&params.template).map_err(|e| {
            McpError::builder(ErrorCode::TemplateError)
                .message(format!("Failed to load template: {}", e))
                .param("template", &params.template)
                .suggestion("Use a built-in template name or inline content")
                .suggestion(format!("Available templates: {}", TemplateLibrary::list().join(", ")))
                .build()
        })?
    };

    validate_template_size(&content)?;

    // Analyze template
    let mut variables = Vec::new();
    let mut filters = Vec::new();
    let mut structures = Vec::new();
    let mut blocks = Vec::new();
    let mut includes = Vec::new();
    let mut macros = Vec::new();

    if params.analyze_variables {
        variables = extract_variables(&content);
    }

    if params.analyze_filters {
        filters = extract_filters(&content);
    }

    if params.analyze_structures {
        structures = extract_structures(&content);
        blocks = extract_blocks(&content);
        includes = extract_includes(&content);
        macros = extract_macros(&content);
    }

    Ok(ReadTeraTemplateResponse {
        size: content.len(),
        content,
        variables,
        filters,
        structures,
        blocks,
        includes,
        macros,
    })
}

// ============================================================================
// MCP Tool: validate_tera_template
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ValidateTeraParams {
    /// Template name or inline content
    pub template: String,

    /// Check variable references (default: true)
    #[serde(default = "default_true")]
    pub check_variables: bool,

    /// Check filter existence (default: true)
    #[serde(default = "default_true")]
    pub check_filters: bool,

    /// Check balanced blocks (default: true)
    #[serde(default = "default_true")]
    pub check_blocks: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ValidateTeraResponse {
    /// Whether template is valid
    pub valid: bool,

    /// Validation errors
    pub errors: Vec<ValidationError>,

    /// Validation warnings
    pub warnings: Vec<String>,

    /// Syntax check passed
    pub syntax_valid: bool,

    /// Blocks are balanced
    pub blocks_balanced: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ValidationError {
    /// Error type
    pub error_type: String,

    /// Error message
    pub message: String,

    /// Line number (if applicable)
    pub line: Option<usize>,

    /// Suggestion for fixing
    pub suggestion: Option<String>,
}

/// Validate Tera template syntax and structure
pub async fn validate_tera_template(
    _state: Arc<AppState>,
    params: ValidateTeraParams,
) -> Result<ValidateTeraResponse, McpError> {
    validate_template_param(&params.template)?;

    let content = load_template_content(&params.template)?;
    validate_template_size(&content)?;

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Check Tera syntax
    let syntax_valid = match validate_tera_syntax(&content, &params.template) {
        Ok(_) => true,
        Err(e) => {
            errors.push(ValidationError {
                error_type: "syntax_error".to_string(),
                message: e.to_string(),
                line: None,
                suggestion: Some("Check for unclosed tags or invalid syntax".to_string()),
            });
            false
        }
    };

    // Check balanced blocks
    let blocks_balanced = if params.check_blocks {
        match check_balanced_blocks(&content) {
            Ok(_) => true,
            Err(e) => {
                errors.push(ValidationError {
                    error_type: "unbalanced_blocks".to_string(),
                    message: e,
                    line: None,
                    suggestion: Some("Ensure all {% if %}, {% for %}, {% block %} tags are closed".to_string()),
                });
                false
            }
        }
    } else {
        true
    };

    // Check filters
    if params.check_filters {
        let filters = extract_filters(&content);
        let unknown_filters = check_filter_existence(&filters);
        for filter in unknown_filters {
            warnings.push(format!("Unknown filter '{}' - may not exist in Tera", filter));
        }
    }

    // Check for common issues
    if content.contains("{{{{") {
        warnings.push("Found '{{{{' - possible escaping issue".to_string());
    }

    let valid = errors.is_empty();

    Ok(ValidateTeraResponse {
        valid,
        errors,
        warnings,
        syntax_valid,
        blocks_balanced,
    })
}

// ============================================================================
// MCP Tool: test_tera_template
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TestTeraParams {
    /// Template name or inline content
    pub template: String,

    /// Context data for rendering (JSON object)
    pub context: JsonValue,

    /// Timeout in milliseconds (default: 5000)
    #[serde(default)]
    pub timeout_ms: Option<u64>,

    /// Show performance metrics (default: true)
    #[serde(default = "default_true")]
    pub show_metrics: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TestTeraResponse {
    /// Rendered output
    pub output: String,

    /// Render succeeded
    pub success: bool,

    /// Compile errors (if any)
    pub errors: Vec<String>,

    /// Render duration in milliseconds
    pub duration_ms: u64,

    /// Output size in bytes
    pub output_size: usize,

    /// Variables used from context
    pub variables_used: Vec<String>,
}

/// Test template rendering with sample context
pub async fn test_tera_template(
    _state: Arc<AppState>,
    params: TestTeraParams,
) -> Result<TestTeraResponse, McpError> {
    validate_template_param(&params.template)?;

    if !params.context.is_object() {
        return Err(McpError::validation()
            .message("context must be a JSON object")
            .param("context", params.context)
            .build());
    }

    let content = load_template_content(&params.template)?;
    validate_template_size(&content)?;

    let timeout_ms = params.timeout_ms.unwrap_or(DEFAULT_RENDER_TIMEOUT_MS);

    // Create renderer
    let config = RenderConfig::default()
        .with_timeout_ms(timeout_ms)
        .with_syntax_validation(true);

    let renderer = SafeRenderer::new(config).map_err(|e| {
        McpError::builder(ErrorCode::TemplateError)
            .message(format!("Failed to create renderer: {}", e))
            .build()
    })?;

    // Add template
    let template_name = if params.template.starts_with("inline:") {
        "test_template"
    } else {
        &params.template
    };

    renderer
        .add_template(template_name, &content)
        .map_err(|e| {
            McpError::builder(ErrorCode::TemplateError)
                .message(format!("Failed to add template: {}", e))
                .build()
        })?;

    // Build context
    let mut render_context = RenderContext::new();
    if let JsonValue::Object(obj) = &params.context {
        for (key, value) in obj {
            render_context
                .insert(key, value)
                .map_err(|e| {
                    McpError::validation()
                        .message(format!("Failed to insert context variable '{}': {}", key, e))
                        .build()
                })?;
        }
    }

    // Extract variables from template
    let variables_used = extract_variables(&content);

    // Render
    let start = std::time::Instant::now();
    let result = renderer.render_safe(template_name, &render_context);
    let duration_ms = start.elapsed().as_millis() as u64;

    let (output, success, errors) = match result {
        Ok(rendered) => (rendered, true, Vec::new()),
        Err(e) => (
            String::new(),
            false,
            vec![format!("Render error: {}", e)],
        ),
    };

    Ok(TestTeraResponse {
        output_size: output.len(),
        output,
        success,
        errors,
        duration_ms,
        variables_used,
    })
}

// ============================================================================
// MCP Tool: create_tera_template
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateTeraParams {
    /// Template pattern (struct, endpoint, schema, interface)
    pub pattern: String,

    /// Template variables (pattern-specific)
    pub variables: JsonValue,

    /// Output name (optional, for saving)
    pub output_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CreateTeraResponse {
    /// Generated template content
    pub template: String,

    /// Pattern used
    pub pattern: String,

    /// Template size in bytes
    pub size: usize,

    /// Suggested file name
    pub suggested_name: String,
}

/// Create scaffolded template from common pattern
pub async fn create_tera_template(
    _state: Arc<AppState>,
    params: CreateTeraParams,
) -> Result<CreateTeraResponse, McpError> {
    // Get pattern template
    let base_template = TemplateLibrary::get(&params.pattern).ok_or_else(|| {
        McpError::validation()
            .message(format!("Unknown pattern: {}", params.pattern))
            .param("pattern", &params.pattern)
            .suggestion(format!("Available patterns: {}", TemplateLibrary::list().join(", ")))
            .build()
    })?;

    // For create_tera_template, we return the pattern template itself
    // The user can then customize it with their own variables
    let template = base_template.to_string();

    let suggested_name = params
        .output_name
        .clone()
        .unwrap_or_else(|| match params.pattern.as_str() {
            "struct" => "struct.rs.tera".to_string(),
            "endpoint" => "endpoint.rs.tera".to_string(),
            "schema" => "schema.yaml.tera".to_string(),
            "interface" => "interface.ts.tera".to_string(),
            _ => format!("{}.tera", params.pattern),
        });

    Ok(CreateTeraResponse {
        size: template.len(),
        template,
        pattern: params.pattern,
        suggested_name,
    })
}

// ============================================================================
// MCP Tool: list_template_variables
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListTemplateVariablesParams {
    /// Template name or inline content
    pub template: String,

    /// Include filter usage (default: true)
    #[serde(default = "default_true")]
    pub include_filters: bool,

    /// Include type hints (default: false)
    #[serde(default)]
    pub include_type_hints: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ListTemplateVariablesResponse {
    /// All variables found
    pub variables: Vec<TemplateVariable>,

    /// Total variable count
    pub count: usize,

    /// Required variables (used without defaults)
    pub required: Vec<String>,

    /// Optional variables (used with defaults)
    pub optional: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TemplateVariable {
    /// Variable name
    pub name: String,

    /// How many times it's used
    pub usage_count: usize,

    /// Filters applied to this variable
    pub filters: Vec<String>,

    /// Has default value
    pub has_default: bool,

    /// Inferred type hint (if available)
    pub type_hint: Option<String>,
}

/// Extract all variables from template
pub async fn list_template_variables(
    _state: Arc<AppState>,
    params: ListTemplateVariablesParams,
) -> Result<ListTemplateVariablesResponse, McpError> {
    validate_template_param(&params.template)?;

    let content = load_template_content(&params.template)?;
    validate_template_size(&content)?;

    let variable_names = extract_variables(&content);
    let mut variable_map: HashMap<String, TemplateVariable> = HashMap::new();

    // Analyze each variable
    for var_name in &variable_names {
        let entry = variable_map
            .entry(var_name.clone())
            .or_insert_with(|| TemplateVariable {
                name: var_name.clone(),
                usage_count: 0,
                filters: Vec::new(),
                has_default: false,
                type_hint: None,
            });

        entry.usage_count += 1;
    }

    // Extract filter information
    if params.include_filters {
        let filter_usage = extract_variable_filters(&content);
        for (var, filters) in filter_usage {
            if let Some(entry) = variable_map.get_mut(&var) {
                entry.filters = filters;
            }
        }
    }

    // Check for defaults
    let defaults = extract_defaults(&content);
    for var in defaults {
        if let Some(entry) = variable_map.get_mut(&var) {
            entry.has_default = true;
        }
    }

    // Infer type hints
    if params.include_type_hints {
        for entry in variable_map.values_mut() {
            entry.type_hint = infer_type_hint(&entry.name, &entry.filters);
        }
    }

    let mut variables: Vec<TemplateVariable> = variable_map.into_values().collect();
    variables.sort_by(|a, b| a.name.cmp(&b.name));

    let required: Vec<String> = variables
        .iter()
        .filter(|v| !v.has_default)
        .map(|v| v.name.clone())
        .collect();

    let optional: Vec<String> = variables
        .iter()
        .filter(|v| v.has_default)
        .map(|v| v.name.clone())
        .collect();

    Ok(ListTemplateVariablesResponse {
        count: variables.len(),
        variables,
        required,
        optional,
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

fn validate_template_param(template: &str) -> Result<(), McpError> {
    if template.is_empty() {
        return Err(McpError::validation()
            .message("template parameter cannot be empty")
            .build());
    }

    if template.len() > MAX_TEMPLATE_NAME_LEN && !template.starts_with("inline:") {
        return Err(McpError::validation()
            .message(format!(
                "template name exceeds maximum length of {}",
                MAX_TEMPLATE_NAME_LEN
            ))
            .build());
    }

    Ok(())
}

fn validate_template_size(content: &str) -> Result<(), McpError> {
    if content.len() > MAX_TEMPLATE_SIZE {
        return Err(McpError::validation()
            .message(format!(
                "template size ({} bytes) exceeds maximum of {} bytes",
                content.len(),
                MAX_TEMPLATE_SIZE
            ))
            .build());
    }
    Ok(())
}

fn load_template_content(template: &str) -> Result<String, McpError> {
    if template.starts_with("inline:") {
        Ok(template[7..].to_string())
    } else if let Some(builtin) = TemplateLibrary::get(template) {
        Ok(builtin.to_string())
    } else {
        load_template_file(template).map_err(|e| {
            McpError::builder(ErrorCode::TemplateError)
                .message(format!("Failed to load template: {}", e))
                .param("template", template)
                .build()
        })
    }
}

fn load_template_file(name: &str) -> Result<String> {
    let path = std::path::Path::new("templates").join(name);
    std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read template file: {}", path.display()))
}

/// Extract variables from template ({{ var }})
fn extract_variables(content: &str) -> Vec<String> {
    let re = Regex::new(r"\{\{\s*([a-zA-Z_][a-zA-Z0-9_\.]*)\s*(?:\||}})")
        .expect("Invalid regex");

    let mut vars = Vec::new();
    for cap in re.captures_iter(content) {
        if let Some(var_name) = cap.get(1) {
            vars.push(var_name.as_str().to_string());
        }
    }
    vars
}

/// Extract filters from template ({{ var | filter }})
fn extract_filters(content: &str) -> Vec<String> {
    let re = Regex::new(r"\{\{[^}]*\|\s*([a-zA-Z_][a-zA-Z0-9_]*)")
        .expect("Invalid regex");

    let mut filters = HashSet::new();
    for cap in re.captures_iter(content) {
        if let Some(filter_name) = cap.get(1) {
            filters.insert(filter_name.as_str().to_string());
        }
    }
    filters.into_iter().collect()
}

/// Extract control structures
fn extract_structures(content: &str) -> Vec<ControlStructure> {
    let re = Regex::new(r"\{%\s*(if|for|block|macro|set)\s+([^%]+)%\}")
        .expect("Invalid regex");

    let mut structures = Vec::new();
    let mut line_num = 1;

    for (idx, c) in content.chars().enumerate() {
        if c == '\n' {
            line_num += 1;
        }

        if let Some(cap) = re.captures(&content[idx..]) {
            if let (Some(kind), Some(details)) = (cap.get(1), cap.get(2)) {
                structures.push(ControlStructure {
                    kind: kind.as_str().to_string(),
                    line: line_num,
                    details: details.as_str().trim().to_string(),
                });
            }
        }
    }

    structures
}

/// Extract block names
fn extract_blocks(content: &str) -> Vec<String> {
    let re = Regex::new(r"\{%\s*block\s+([a-zA-Z_][a-zA-Z0-9_]*)")
        .expect("Invalid regex");

    re.captures_iter(content)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

/// Extract include directives
fn extract_includes(content: &str) -> Vec<String> {
    let re = Regex::new(r#"\{%\s*include\s+"([^"]+)""#)
        .expect("Invalid regex");

    re.captures_iter(content)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

/// Extract macro definitions
fn extract_macros(content: &str) -> Vec<String> {
    let re = Regex::new(r"\{%\s*macro\s+([a-zA-Z_][a-zA-Z0-9_]*)")
        .expect("Invalid regex");

    re.captures_iter(content)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

/// Validate Tera syntax by attempting to compile
fn validate_tera_syntax(content: &str, name: &str) -> Result<()> {
    let mut tera = Tera::default();
    tera.add_raw_template(name, content)?;
    Ok(())
}

/// Check balanced blocks (if/endif, for/endfor, block/endblock)
fn check_balanced_blocks(content: &str) -> Result<(), String> {
    let mut stack = Vec::new();
    let open_re = Regex::new(r"\{%\s*(if|for|block|macro)\s")
        .expect("Invalid regex");
    let close_re = Regex::new(r"\{%\s*end(if|for|block|macro)\s*%\}")
        .expect("Invalid regex");

    let mut pos = 0;
    while pos < content.len() {
        let remaining = &content[pos..];

        if let Some(cap) = open_re.captures(remaining) {
            if let Some(kind) = cap.get(1) {
                stack.push(kind.as_str().to_string());
                pos += cap.get(0).unwrap().end();
                continue;
            }
        }

        if let Some(cap) = close_re.captures(remaining) {
            if let Some(kind) = cap.get(1) {
                let close_kind = kind.as_str();
                if let Some(open_kind) = stack.pop() {
                    if open_kind != close_kind {
                        return Err(format!(
                            "Mismatched block: expected 'end{}' but found 'end{}'",
                            open_kind, close_kind
                        ));
                    }
                } else {
                    return Err(format!("Unexpected closing tag 'end{}'", close_kind));
                }
                pos += cap.get(0).unwrap().end();
                continue;
            }
        }

        pos += 1;
    }

    if !stack.is_empty() {
        return Err(format!("Unclosed blocks: {}", stack.join(", ")));
    }

    Ok(())
}

/// Check if filters are known Tera filters
fn check_filter_existence(filters: &[String]) -> Vec<String> {
    const KNOWN_FILTERS: &[&str] = &[
        "upper", "lower", "capitalize", "title", "trim", "truncate",
        "wordcount", "replace", "addslashes", "slugify", "indent",
        "safe", "escape", "linebreaks", "striptags", "urlencode",
        "date", "filesizeformat", "default", "length", "reverse",
        "sort", "unique", "first", "last", "join", "split",
        "round", "abs", "plus", "minus", "times", "divided_by",
        "json_encode", "as_str", "concat", "get", "snake_case",
        "pascal_case", "camel_case", "kebab_case",
    ];

    filters
        .iter()
        .filter(|f| !KNOWN_FILTERS.contains(&f.as_str()))
        .cloned()
        .collect()
}

/// Extract variable-filter pairs
fn extract_variable_filters(content: &str) -> HashMap<String, Vec<String>> {
    let re = Regex::new(r"\{\{\s*([a-zA-Z_][a-zA-Z0-9_\.]*)\s*\|([^}]+)\}\}")
        .expect("Invalid regex");

    let mut result = HashMap::new();

    for cap in re.captures_iter(content) {
        if let (Some(var), Some(filter_chain)) = (cap.get(1), cap.get(2)) {
            let var_name = var.as_str().to_string();
            let filters: Vec<String> = filter_chain
                .as_str()
                .split('|')
                .map(|f| f.trim().split('(').next().unwrap_or("").trim().to_string())
                .filter(|f| !f.is_empty())
                .collect();

            result.insert(var_name, filters);
        }
    }

    result
}

/// Extract variables with default values
fn extract_defaults(content: &str) -> Vec<String> {
    // This is a simplified check - real implementation would need full parsing
    // For now, we look for "| default(value=...)" patterns
    let var_re = Regex::new(r"\{\{\s*([a-zA-Z_][a-zA-Z0-9_\.]*)[^}]*default")
        .expect("Invalid regex");

    var_re
        .captures_iter(content)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

/// Infer type hint from variable name and filters
fn infer_type_hint(name: &str, filters: &[String]) -> Option<String> {
    // Check filters first
    for filter in filters {
        match filter.as_str() {
            "date" => return Some("DateTime".to_string()),
            "filesizeformat" | "plus" | "minus" | "times" | "divided_by" => {
                return Some("number".to_string())
            }
            "length" | "wordcount" => return Some("collection".to_string()),
            _ => {}
        }
    }

    // Check name patterns
    if name.ends_with("_id") || name.ends_with("_count") || name.contains("num") {
        return Some("number".to_string());
    }
    if name.ends_with("_at") || name.contains("date") || name.contains("time") {
        return Some("DateTime".to_string());
    }
    if name.ends_with("s") && !name.ends_with("ss") {
        return Some("array".to_string());
    }

    Some("string".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_variables() {
        let template = r#"{{ name }} {{ user.email }} {{ count }}"#;
        let vars = extract_variables(template);
        assert_eq!(vars.len(), 3);
        assert!(vars.contains(&"name".to_string()));
        assert!(vars.contains(&"user".to_string()));
        assert!(vars.contains(&"count".to_string()));
    }

    #[test]
    fn test_extract_filters() {
        let template = r#"{{ name | upper }} {{ date | date }} {{ text | trim | lower }}"#;
        let filters = extract_filters(template);
        assert!(filters.contains(&"upper".to_string()));
        assert!(filters.contains(&"date".to_string()));
        assert!(filters.contains(&"trim".to_string()));
        assert!(filters.contains(&"lower".to_string()));
    }

    #[test]
    fn test_balanced_blocks_valid() {
        let template = r#"{% if condition %}...{% endif %}{% for item in items %}...{% endfor %}"#;
        assert!(check_balanced_blocks(template).is_ok());
    }

    #[test]
    fn test_balanced_blocks_invalid() {
        let template = r#"{% if condition %}...{% endfor %}"#;
        assert!(check_balanced_blocks(template).is_err());
    }

    #[test]
    fn test_template_library_get() {
        assert!(TemplateLibrary::get("struct").is_some());
        assert!(TemplateLibrary::get("endpoint").is_some());
        assert!(TemplateLibrary::get("schema").is_some());
        assert!(TemplateLibrary::get("interface").is_some());
        assert!(TemplateLibrary::get("nonexistent").is_none());
    }

    #[test]
    fn test_validate_tera_syntax_valid() {
        let template = r#"{{ name | upper }}"#;
        assert!(validate_tera_syntax(template, "test").is_ok());
    }

    #[test]
    fn test_validate_tera_syntax_invalid() {
        let template = r#"{{ name | }}"#;
        assert!(validate_tera_syntax(template, "test").is_err());
    }

    #[test]
    fn test_extract_blocks() {
        let template = r#"{% block header %}...{% endblock %} {% block content %}...{% endblock %}"#;
        let blocks = extract_blocks(template);
        assert_eq!(blocks.len(), 2);
        assert!(blocks.contains(&"header".to_string()));
        assert!(blocks.contains(&"content".to_string()));
    }
}
