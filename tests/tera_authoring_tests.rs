//! Integration tests for Tera template authoring tools
//!
//! Chicago-style TDD: State-based verification, real Tera engine, minimal mocking.
//! Tests Tera template lifecycle: read, validate, test with context, create, list variables.

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use tempfile::TempDir;
use tera::{Context as TeraContext, Tera};

mod harness;
use harness::tera_template_harness::{TemplateContextBuilder, TemplateTestHarness};

// =============================================================================
// Test Harness for Tera Template Operations
// =============================================================================

struct TeraAuthoringHarness {
    workspace: TempDir,
}

impl TeraAuthoringHarness {
    fn new() -> Result<Self> {
        let workspace = tempfile::tempdir()?;
        let templates_dir = workspace.path().join("templates");
        let fixtures_dir = workspace.path().join("fixtures");
        fs::create_dir_all(&templates_dir)?;
        fs::create_dir_all(&fixtures_dir)?;

        Ok(Self { workspace })
    }

    fn template_path(&self, name: &str) -> std::path::PathBuf {
        self.workspace
            .path()
            .join("templates")
            .join(format!("{}.rs.tera", name))
    }

    fn write_template(&self, name: &str, content: &str) -> Result<()> {
        fs::write(self.template_path(name), content)?;
        Ok(())
    }

    fn read_template(&self, name: &str) -> Result<String> {
        Ok(fs::read_to_string(self.template_path(name))?)
    }

    fn workspace_path(&self) -> &Path {
        self.workspace.path()
    }

    fn templates_dir(&self) -> std::path::PathBuf {
        self.workspace.path().join("templates")
    }

    fn fixtures_dir(&self) -> std::path::PathBuf {
        self.workspace.path().join("fixtures")
    }
}

// =============================================================================
// Fixtures
// =============================================================================

fn simple_struct_template() -> &'static str {
    r#"
// Generated struct from template
#[derive(Debug, Clone)]
pub struct {{ entity_name }} {
    {% for field in fields %}
    pub {{ field.name }}: {{ field.rust_type }},
    {% endfor %}
}
"#
}

fn conditional_template() -> &'static str {
    r#"
#[derive(Debug, Clone{% if with_serde %}, serde::Serialize, serde::Deserialize{% endif %})]
pub struct {{ entity_name }} {
    {% if has_id %}
    pub id: {{ entity_name }}Id,
    {% endif %}
    {% for field in fields %}
    pub {{ field.name }}: {{ field.rust_type }},
    {% endfor %}
}

{% if generate_impl %}
impl {{ entity_name }} {
    pub fn new({% for field in fields %}{{ field.name }}: {{ field.rust_type }}{% if not loop.last %}, {% endif %}{% endfor %}) -> Self {
        Self {
            {% if has_id %}
            id: {{ entity_name }}Id::new(),
            {% endif %}
            {% for field in fields %}
            {{ field.name }},
            {% endfor %}
        }
    }
}
{% endif %}
"#
}

fn loop_template() -> &'static str {
    r#"
{% for entity in entities %}
/// {{ entity.description }}
pub struct {{ entity.name }} {
    pub id: {{ entity.name }}Id,
}
{% endfor %}
"#
}

fn filter_template() -> &'static str {
    r#"
pub struct {{ entity_name | upper }} {
    pub name: String,
}

pub const ENTITY_TYPE: &str = "{{ entity_name | lower }}";
"#
}

// =============================================================================
// Tests: read_tera_template
// =============================================================================

#[tokio::test]
async fn test_read_existing_template() -> Result<()> {
    // GIVEN: Template file exists
    let harness = TeraAuthoringHarness::new()?;
    harness.write_template("user_struct", simple_struct_template())?;

    // WHEN: We read the template
    let result = simulate_read_template(harness.template_path("user_struct").as_path()).await?;

    // THEN: Template content returned
    assert!(result.content.contains("entity_name"));
    assert!(result.content.contains("fields"));
    assert_eq!(result.name, "user_struct.rs.tera");

    Ok(())
}

#[tokio::test]
async fn test_read_nonexistent_template() -> Result<()> {
    // GIVEN: No template file
    let harness = TeraAuthoringHarness::new()?;

    // WHEN: We try to read non-existent template
    let result =
        simulate_read_template(harness.template_path("nonexistent").as_path()).await;

    // THEN: Error returned
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("not found") || err_msg.contains("No such file"));

    Ok(())
}

// =============================================================================
// Tests: validate_tera_template
// =============================================================================

#[tokio::test]
async fn test_validate_valid_template() -> Result<()> {
    // GIVEN: Valid Tera template
    let harness = TeraAuthoringHarness::new()?;
    harness.write_template("valid", simple_struct_template())?;

    // WHEN: We validate syntax
    let result = simulate_validate_template(harness.template_path("valid").as_path()).await?;

    // THEN: Validation passes
    assert!(result.valid);
    assert_eq!(result.errors.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_validate_invalid_syntax() -> Result<()> {
    // GIVEN: Template with syntax error
    let harness = TeraAuthoringHarness::new()?;
    harness.write_template("invalid", "{% if true %}Missing endif")?;

    // WHEN: We validate syntax
    let result = simulate_validate_template(harness.template_path("invalid").as_path()).await?;

    // THEN: Validation fails
    assert!(!result.valid);
    assert!(result.errors.len() > 0);
    assert!(result.errors[0].contains("endif") || result.errors[0].contains("syntax"));

    Ok(())
}

#[tokio::test]
async fn test_validate_unclosed_delimiter() -> Result<()> {
    // GIVEN: Template with unclosed delimiter
    let harness = TeraAuthoringHarness::new()?;
    harness.write_template("unclosed", "{{ entity_name")?;

    // WHEN: We validate syntax
    let result =
        simulate_validate_template(harness.template_path("unclosed").as_path()).await?;

    // THEN: Validation fails
    assert!(!result.valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("delimiter") || e.contains("unclosed")));

    Ok(())
}

// =============================================================================
// Tests: test_tera_template
// =============================================================================

#[tokio::test]
async fn test_template_renders_with_context() -> Result<()> {
    // GIVEN: Template and matching context
    let harness = TeraAuthoringHarness::new()?;
    harness.write_template("user", simple_struct_template())?;

    let context = TemplateContextBuilder::new()
        .entity("User")
        .field("name", "String")
        .field("email", "String")
        .build()?;

    // WHEN: We test template with context
    let result = simulate_test_template(
        harness.template_path("user").as_path(),
        harness.templates_dir().as_path(),
        &context,
    )
    .await?;

    // THEN: Render succeeds
    assert!(result.success);
    assert!(result.output.contains("pub struct User"));
    assert!(result.output.contains("pub name: String"));
    assert!(result.output.contains("pub email: String"));

    Ok(())
}

#[tokio::test]
async fn test_template_with_conditionals() -> Result<()> {
    // GIVEN: Template with conditionals
    let harness = TeraAuthoringHarness::new()?;
    harness.write_template("conditional", conditional_template())?;

    // WHEN: We render with flags enabled
    let context = TemplateContextBuilder::new()
        .entity("Product")
        .field("name", "String")
        .flag("with_serde", true)
        .flag("has_id", true)
        .flag("generate_impl", true)
        .build()?;

    let result = simulate_test_template(
        harness.template_path("conditional").as_path(),
        harness.templates_dir().as_path(),
        &context,
    )
    .await?;

    // THEN: Conditional blocks rendered
    assert!(result.output.contains("serde::Serialize"));
    assert!(result.output.contains("pub id: ProductId"));
    assert!(result.output.contains("impl Product"));

    Ok(())
}

#[tokio::test]
async fn test_template_with_loops() -> Result<()> {
    // GIVEN: Template with loops
    let harness = TeraAuthoringHarness::new()?;
    harness.write_template("loop", loop_template())?;

    let mut context = TeraContext::new();
    context.insert(
        "entities",
        &vec![
            serde_json::json!({"name": "User", "description": "User entity"}),
            serde_json::json!({"name": "Order", "description": "Order entity"}),
            serde_json::json!({"name": "Product", "description": "Product entity"}),
        ],
    );

    // WHEN: We render template
    let result = simulate_test_template(
        harness.template_path("loop").as_path(),
        harness.templates_dir().as_path(),
        &context,
    )
    .await?;

    // THEN: Loop iterations rendered
    assert!(result.output.contains("pub struct User"));
    assert!(result.output.contains("pub struct Order"));
    assert!(result.output.contains("pub struct Product"));
    assert!(result.output.contains("User entity"));

    Ok(())
}

#[tokio::test]
async fn test_template_with_filters() -> Result<()> {
    // GIVEN: Template with filters
    let harness = TeraAuthoringHarness::new()?;
    harness.write_template("filter", filter_template())?;

    let context = TemplateContextBuilder::new().entity("Product").build()?;

    // WHEN: We render template
    let result = simulate_test_template(
        harness.template_path("filter").as_path(),
        harness.templates_dir().as_path(),
        &context,
    )
    .await?;

    // THEN: Filters applied
    assert!(result.output.contains("pub struct PRODUCT"));  // upper filter
    assert!(result.output.contains("\"product\""));  // lower filter

    Ok(())
}

#[tokio::test]
async fn test_template_with_missing_variable_fails() -> Result<()> {
    // GIVEN: Template requiring variable
    let harness = TeraAuthoringHarness::new()?;
    harness.write_template("required", simple_struct_template())?;

    let context = TeraContext::new();  // Empty context

    // WHEN: We try to render without required variable
    let result = simulate_test_template(
        harness.template_path("required").as_path(),
        harness.templates_dir().as_path(),
        &context,
    )
    .await;

    // THEN: Render fails
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("entity_name") || err_msg.contains("variable"));

    Ok(())
}

// =============================================================================
// Tests: create_tera_template
// =============================================================================

#[tokio::test]
async fn test_create_new_template() -> Result<()> {
    // GIVEN: No existing template
    let harness = TeraAuthoringHarness::new()?;

    // WHEN: We create a new template
    simulate_create_template(
        harness.template_path("new_template").as_path(),
        simple_struct_template(),
        Some("Template for generating Rust structs"),
    )
    .await?;

    // THEN: Template file created
    assert!(harness.template_path("new_template").exists());

    // AND: Content correct
    let content = harness.read_template("new_template")?;
    assert_eq!(content, simple_struct_template());

    Ok(())
}

#[tokio::test]
async fn test_create_template_overwrites_with_flag() -> Result<()> {
    // GIVEN: Existing template
    let harness = TeraAuthoringHarness::new()?;
    harness.write_template("existing", "old content")?;

    // WHEN: We create with overwrite flag
    simulate_create_template_overwrite(
        harness.template_path("existing").as_path(),
        "new content",
        None,
    )
    .await?;

    // THEN: Template overwritten
    let content = harness.read_template("existing")?;
    assert_eq!(content, "new content");

    Ok(())
}

#[tokio::test]
async fn test_create_template_fails_without_overwrite() -> Result<()> {
    // GIVEN: Existing template
    let harness = TeraAuthoringHarness::new()?;
    harness.write_template("existing", "old content")?;

    // WHEN: We try to create without overwrite
    let result = simulate_create_template(
        harness.template_path("existing").as_path(),
        "new content",
        None,
    )
    .await;

    // THEN: Error returned
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("exists") || err_msg.contains("already"));

    Ok(())
}

// =============================================================================
// Tests: list_template_variables
// =============================================================================

#[tokio::test]
async fn test_list_variables_in_template() -> Result<()> {
    // GIVEN: Template with variables
    let harness = TeraAuthoringHarness::new()?;
    harness.write_template("vars", simple_struct_template())?;

    // WHEN: We list variables
    let result =
        simulate_list_variables(harness.template_path("vars").as_path(), harness.templates_dir().as_path()).await?;

    // THEN: Variables extracted
    assert!(result.variables.contains(&"entity_name".to_string()));
    assert!(result.variables.contains(&"fields".to_string()));
    assert_eq!(result.variable_count, 2);

    Ok(())
}

#[tokio::test]
async fn test_list_variables_with_conditionals() -> Result<()> {
    // GIVEN: Template with conditional variables
    let harness = TeraAuthoringHarness::new()?;
    harness.write_template("conditional", conditional_template())?;

    // WHEN: We list variables
    let result = simulate_list_variables(
        harness.template_path("conditional").as_path(),
        harness.templates_dir().as_path(),
    )
    .await?;

    // THEN: All variables extracted (including conditional ones)
    assert!(result.variables.contains(&"entity_name".to_string()));
    assert!(result.variables.contains(&"with_serde".to_string()));
    assert!(result.variables.contains(&"has_id".to_string()));
    assert!(result.variables.contains(&"generate_impl".to_string()));
    assert!(result.variables.contains(&"fields".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_list_variables_empty_template() -> Result<()> {
    // GIVEN: Template without variables
    let harness = TeraAuthoringHarness::new()?;
    harness.write_template("empty", "// Static template\npub struct User {}")?;

    // WHEN: We list variables
    let result =
        simulate_list_variables(harness.template_path("empty").as_path(), harness.templates_dir().as_path()).await?;

    // THEN: No variables
    assert_eq!(result.variable_count, 0);

    Ok(())
}

// =============================================================================
// Mock Implementation Helpers (Replace with real MCP tool calls)
// =============================================================================

#[derive(Debug)]
struct TemplateReadResult {
    name: String,
    content: String,
}

#[derive(Debug)]
struct ValidationResult {
    valid: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Debug)]
struct TemplateTestResult {
    success: bool,
    output: String,
    errors: Vec<String>,
}

#[derive(Debug)]
struct ListVariablesResult {
    variables: HashSet<String>,
    variable_count: usize,
}

async fn simulate_read_template(path: &Path) -> Result<TemplateReadResult> {
    let content = fs::read_to_string(path)?;
    let name = path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    Ok(TemplateReadResult { name, content })
}

async fn simulate_validate_template(path: &Path) -> Result<ValidationResult> {
    let content = fs::read_to_string(path)?;

    // Try parsing with Tera
    let mut tera = Tera::default();
    let template_name = path.file_name().unwrap().to_str().unwrap();

    match tera.add_raw_template(template_name, &content) {
        Ok(_) => Ok(ValidationResult {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }),
        Err(e) => Ok(ValidationResult {
            valid: false,
            errors: vec![format!("Template syntax error: {}", e)],
            warnings: Vec::new(),
        }),
    }
}

async fn simulate_test_template(
    template_path: &Path,
    templates_dir: &Path,
    context: &TeraContext,
) -> Result<TemplateTestResult> {
    let content = fs::read_to_string(template_path)?;

    let mut tera = Tera::default();
    let template_name = template_path.file_name().unwrap().to_str().unwrap();

    tera.add_raw_template(template_name, &content)
        .context("Failed to add template")?;

    match tera.render(template_name, context) {
        Ok(output) => Ok(TemplateTestResult {
            success: true,
            output,
            errors: Vec::new(),
        }),
        Err(e) => Err(anyhow::anyhow!("Template render failed: {}", e)),
    }
}

async fn simulate_create_template(
    path: &Path,
    content: &str,
    _description: Option<&str>,
) -> Result<()> {
    if path.exists() {
        anyhow::bail!("Template already exists: {:?}", path);
    }

    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(path, content)?;

    // Validate syntax
    simulate_validate_template(path).await?;

    Ok(())
}

async fn simulate_create_template_overwrite(
    path: &Path,
    content: &str,
    _description: Option<&str>,
) -> Result<()> {
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(path, content)?;

    // Validate syntax
    simulate_validate_template(path).await?;

    Ok(())
}

async fn simulate_list_variables(template_path: &Path, _templates_dir: &Path) -> Result<ListVariablesResult> {
    let content = fs::read_to_string(template_path)?;

    let mut variables = HashSet::new();

    // Simple regex-based extraction (in production would use AST)
    let var_pattern = regex::Regex::new(r"\{\{\s*([a-zA-Z_][a-zA-Z0-9_]*)")
        .expect("Invalid regex");

    for cap in var_pattern.captures_iter(&content) {
        if let Some(var_name) = cap.get(1) {
            variables.insert(var_name.as_str().to_string());
        }
    }

    // Extract conditional variables
    let if_pattern = regex::Regex::new(r"\{%\s*if\s+([a-zA-Z_][a-zA-Z0-9_]*)")
        .expect("Invalid regex");

    for cap in if_pattern.captures_iter(&content) {
        if let Some(var_name) = cap.get(1) {
            variables.insert(var_name.as_str().to_string());
        }
    }

    // Extract loop variables
    let for_pattern = regex::Regex::new(r"\{%\s*for\s+\w+\s+in\s+([a-zA-Z_][a-zA-Z0-9_]*)")
        .expect("Invalid regex");

    for cap in for_pattern.captures_iter(&content) {
        if let Some(var_name) = cap.get(1) {
            variables.insert(var_name.as_str().to_string());
        }
    }

    let variable_count = variables.len();

    Ok(ListVariablesResult {
        variables,
        variable_count,
    })
}
