//! Integration tests for ggen.toml configuration authoring tools
//!
//! Chicago-style TDD: State-based verification, real TOML parsing, minimal mocking.
//! Tests complete ggen.toml lifecycle: read, validate, modify, preserve formatting.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use tempfile::TempDir;
use toml;

// =============================================================================
// Test Harness for Ggen Config Operations
// =============================================================================

struct GgenConfigHarness {
    workspace: TempDir,
}

impl GgenConfigHarness {
    fn new() -> Result<Self> {
        Ok(Self {
            workspace: tempfile::tempdir()?,
        })
    }

    fn config_path(&self) -> std::path::PathBuf {
        self.workspace.path().join("ggen.toml")
    }

    fn write_config(&self, content: &str) -> Result<()> {
        fs::write(self.config_path(), content)?;
        Ok(())
    }

    fn read_config(&self) -> Result<String> {
        Ok(fs::read_to_string(self.config_path())?)
    }

    fn workspace_path(&self) -> &Path {
        self.workspace.path()
    }
}

// =============================================================================
// Fixtures
// =============================================================================

fn minimal_config() -> &'static str {
    r#"[project]
name = "test-project"
version = "0.1.0"

[ontology]
source = "ontology/domain.ttl"
base_uri = "http://example.org/"
format = "turtle"
"#
}

fn config_with_generation_rules() -> &'static str {
    r#"[project]
name = "test-project"
version = "0.1.0"

[ontology]
source = "ontology/domain.ttl"
base_uri = "http://example.org/"
format = "turtle"

[[generation]]
name = "user_aggregate"
query = "queries/user.rq"
template = "templates/aggregate.rs.tera"
output = "src/generated/user.rs"

[[generation]]
name = "order_aggregate"
query = "queries/order.rq"
template = "templates/aggregate.rs.tera"
output = "src/generated/order.rs"

[cache]
enabled = true
ttl_seconds = 3600
"#
}

fn config_with_comments() -> &'static str {
    r#"# ggen-mcp Configuration
# Production-ready setup

[project]
name = "test-project"
version = "0.1.0"
description = "Test project for ggen-mcp"

# Ontology configuration
[ontology]
source = "ontology/domain.ttl"
base_uri = "http://example.org/"
format = "turtle"  # Turtle format for RDF

# Generation rules
[[generation]]
name = "user_aggregate"  # User aggregate generation
query = "queries/user.rq"
template = "templates/aggregate.rs.tera"
output = "src/generated/user.rs"
"#
}

// =============================================================================
// Tests: read_ggen_config
// =============================================================================

#[tokio::test]
async fn test_read_valid_config() -> Result<()> {
    // GIVEN: Valid ggen.toml file
    let harness = GgenConfigHarness::new()?;
    harness.write_config(minimal_config())?;

    // WHEN: We read the config
    let result = simulate_read_config(harness.config_path().as_path()).await?;

    // THEN: Config parsed successfully
    assert_eq!(result.project_name, "test-project");
    assert_eq!(result.ontology_source, "ontology/domain.ttl");
    assert_eq!(result.base_uri, "http://example.org/");

    Ok(())
}

#[tokio::test]
async fn test_read_config_with_generation_rules() -> Result<()> {
    // GIVEN: Config with multiple generation rules
    let harness = GgenConfigHarness::new()?;
    harness.write_config(config_with_generation_rules())?;

    // WHEN: We read the config
    let result = simulate_read_config(harness.config_path().as_path()).await?;

    // THEN: All generation rules parsed
    assert_eq!(result.generation_rules.len(), 2);
    assert_eq!(result.generation_rules[0].name, "user_aggregate");
    assert_eq!(result.generation_rules[1].name, "order_aggregate");

    // AND: Cache settings parsed
    assert!(result.cache_enabled);
    assert_eq!(result.cache_ttl, 3600);

    Ok(())
}

#[tokio::test]
async fn test_read_nonexistent_config() -> Result<()> {
    // GIVEN: No config file
    let harness = GgenConfigHarness::new()?;

    // WHEN: We try to read config
    let result = simulate_read_config(harness.config_path().as_path()).await;

    // THEN: Error returned
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("not found") || err_msg.contains("No such file"));

    Ok(())
}

// =============================================================================
// Tests: validate_ggen_config
// =============================================================================

#[tokio::test]
async fn test_validate_valid_config() -> Result<()> {
    // GIVEN: Valid config
    let harness = GgenConfigHarness::new()?;
    harness.write_config(minimal_config())?;

    // WHEN: We validate
    let result = simulate_validate_config(harness.config_path().as_path()).await?;

    // THEN: Validation passes
    assert!(result.valid);
    assert_eq!(result.errors.len(), 0);
    assert_eq!(result.warnings.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_validate_invalid_toml_syntax() -> Result<()> {
    // GIVEN: Invalid TOML syntax
    let harness = GgenConfigHarness::new()?;
    harness.write_config("[project\nname = unterminated")?;

    // WHEN: We validate
    let result = simulate_validate_config(harness.config_path().as_path()).await?;

    // THEN: Validation fails
    assert!(!result.valid);
    assert!(result.errors.len() > 0);
    assert!(result.errors[0].contains("syntax") || result.errors[0].contains("parse"));

    Ok(())
}

#[tokio::test]
async fn test_validate_missing_required_fields() -> Result<()> {
    // GIVEN: Config missing required fields
    let harness = GgenConfigHarness::new()?;
    harness.write_config("[project]\nversion = \"0.1.0\"")?;

    // WHEN: We validate
    let result = simulate_validate_config(harness.config_path().as_path()).await?;

    // THEN: Validation fails with specific errors
    assert!(!result.valid);
    assert!(result.errors.iter().any(|e| e.contains("name")));

    Ok(())
}

#[tokio::test]
async fn test_validate_warns_on_missing_cache() -> Result<()> {
    // GIVEN: Config without cache settings
    let harness = GgenConfigHarness::new()?;
    harness.write_config(minimal_config())?;

    // WHEN: We validate
    let result = simulate_validate_config(harness.config_path().as_path()).await?;

    // THEN: Valid but with warning
    assert!(result.valid);
    assert!(result.warnings.iter().any(|w| w.contains("cache")));

    Ok(())
}

// =============================================================================
// Tests: add_generation_rule
// =============================================================================

#[tokio::test]
async fn test_add_generation_rule_to_config() -> Result<()> {
    // GIVEN: Config without generation rules
    let harness = GgenConfigHarness::new()?;
    harness.write_config(minimal_config())?;

    // WHEN: We add a generation rule
    simulate_add_generation_rule(
        harness.config_path().as_path(),
        "product_aggregate",
        "queries/product.rq",
        "templates/aggregate.rs.tera",
        "src/generated/product.rs",
    )
    .await?;

    // THEN: Config updated
    let content = harness.read_config()?;
    assert!(content.contains("[[generation]]"));
    assert!(content.contains("name = \"product_aggregate\""));
    assert!(content.contains("queries/product.rq"));

    // AND: Config still valid TOML
    let parsed: toml::Value = toml::from_str(&content)?;
    assert!(parsed.get("generation").is_some());

    Ok(())
}

#[tokio::test]
async fn test_add_multiple_generation_rules() -> Result<()> {
    // GIVEN: Config with one rule
    let harness = GgenConfigHarness::new()?;
    harness.write_config(minimal_config())?;

    // WHEN: We add multiple rules
    simulate_add_generation_rule(
        harness.config_path().as_path(),
        "rule1",
        "q1.rq",
        "t1.tera",
        "o1.rs",
    )
    .await?;

    simulate_add_generation_rule(
        harness.config_path().as_path(),
        "rule2",
        "q2.rq",
        "t2.tera",
        "o2.rs",
    )
    .await?;

    // THEN: Both rules in config
    let result = simulate_read_config(harness.config_path().as_path()).await?;
    assert_eq!(result.generation_rules.len(), 2);
    assert_eq!(result.generation_rules[0].name, "rule1");
    assert_eq!(result.generation_rules[1].name, "rule2");

    Ok(())
}

// =============================================================================
// Tests: update_generation_rule
// =============================================================================

#[tokio::test]
async fn test_update_existing_generation_rule() -> Result<()> {
    // GIVEN: Config with generation rule
    let harness = GgenConfigHarness::new()?;
    harness.write_config(config_with_generation_rules())?;

    // WHEN: We update a rule
    simulate_update_generation_rule(
        harness.config_path().as_path(),
        "user_aggregate",
        "queries/user_v2.rq",
        "templates/aggregate_v2.rs.tera",
        "src/generated/user_v2.rs",
    )
    .await?;

    // THEN: Rule updated
    let result = simulate_read_config(harness.config_path().as_path()).await?;
    let user_rule = result
        .generation_rules
        .iter()
        .find(|r| r.name == "user_aggregate")
        .unwrap();
    assert_eq!(user_rule.query, "queries/user_v2.rq");
    assert_eq!(user_rule.template, "templates/aggregate_v2.rs.tera");

    Ok(())
}

#[tokio::test]
async fn test_update_nonexistent_rule_fails() -> Result<()> {
    // GIVEN: Config without the rule
    let harness = GgenConfigHarness::new()?;
    harness.write_config(minimal_config())?;

    // WHEN: We try to update non-existent rule
    let result = simulate_update_generation_rule(
        harness.config_path().as_path(),
        "nonexistent",
        "q.rq",
        "t.tera",
        "o.rs",
    )
    .await;

    // THEN: Error returned
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("not found") || err_msg.contains("nonexistent"));

    Ok(())
}

// =============================================================================
// Tests: remove_generation_rule
// =============================================================================

#[tokio::test]
async fn test_remove_generation_rule() -> Result<()> {
    // GIVEN: Config with multiple rules
    let harness = GgenConfigHarness::new()?;
    harness.write_config(config_with_generation_rules())?;

    // WHEN: We remove a rule
    simulate_remove_generation_rule(harness.config_path().as_path(), "user_aggregate")
        .await?;

    // THEN: Rule removed
    let result = simulate_read_config(harness.config_path().as_path()).await?;
    assert_eq!(result.generation_rules.len(), 1);
    assert_eq!(result.generation_rules[0].name, "order_aggregate");

    // AND: Config still valid
    let content = harness.read_config()?;
    let _: toml::Value = toml::from_str(&content)?;

    Ok(())
}

#[tokio::test]
async fn test_remove_all_generation_rules() -> Result<()> {
    // GIVEN: Config with rules
    let harness = GgenConfigHarness::new()?;
    harness.write_config(config_with_generation_rules())?;

    // WHEN: We remove all rules
    simulate_remove_generation_rule(harness.config_path().as_path(), "user_aggregate")
        .await?;
    simulate_remove_generation_rule(harness.config_path().as_path(), "order_aggregate")
        .await?;

    // THEN: No rules remain
    let result = simulate_read_config(harness.config_path().as_path()).await?;
    assert_eq!(result.generation_rules.len(), 0);

    Ok(())
}

// =============================================================================
// Tests: Comment and Formatting Preservation
// =============================================================================

#[tokio::test]
async fn test_preserve_comments_on_update() -> Result<()> {
    // GIVEN: Config with comments
    let harness = GgenConfigHarness::new()?;
    harness.write_config(config_with_comments())?;

    // WHEN: We add a rule
    simulate_add_generation_rule(
        harness.config_path().as_path(),
        "new_rule",
        "q.rq",
        "t.tera",
        "o.rs",
    )
    .await?;

    // THEN: Comments preserved
    let content = harness.read_config()?;
    assert!(content.contains("# ggen-mcp Configuration"));
    assert!(content.contains("# Production-ready setup"));
    assert!(content.contains("# Ontology configuration"));

    Ok(())
}

#[tokio::test]
async fn test_preserve_formatting_on_update() -> Result<()> {
    // GIVEN: Config with specific formatting
    let harness = GgenConfigHarness::new()?;
    let original = config_with_generation_rules();
    harness.write_config(original)?;

    // WHEN: We update a rule
    simulate_update_generation_rule(
        harness.config_path().as_path(),
        "user_aggregate",
        "queries/user_v2.rq",
        "templates/aggregate.rs.tera",
        "src/generated/user.rs",
    )
    .await?;

    // THEN: Formatting preserved (empty lines, structure)
    let content = harness.read_config()?;
    assert!(content.contains("\n\n"));  // Empty lines preserved
    assert!(content.contains("[project]"));
    assert!(content.contains("[ontology]"));

    Ok(())
}

// =============================================================================
// Mock Implementation Helpers (Replace with real MCP tool calls)
// =============================================================================

#[derive(Debug, Clone)]
struct GgenConfigData {
    project_name: String,
    ontology_source: String,
    base_uri: String,
    generation_rules: Vec<GenerationRule>,
    cache_enabled: bool,
    cache_ttl: u64,
}

#[derive(Debug, Clone)]
struct GenerationRule {
    name: String,
    query: String,
    template: String,
    output: String,
}

#[derive(Debug)]
struct ValidationResult {
    valid: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
}

async fn simulate_read_config(path: &Path) -> Result<GgenConfigData> {
    let content = fs::read_to_string(path)?;
    let config: toml::Value = toml::from_str(&content)?;

    let project = config
        .get("project")
        .ok_or_else(|| anyhow::anyhow!("Missing [project] section"))?;
    let ontology = config
        .get("ontology")
        .ok_or_else(|| anyhow::anyhow!("Missing [ontology] section"))?;

    let mut generation_rules = Vec::new();
    if let Some(gen_array) = config.get("generation").and_then(|v| v.as_array()) {
        for rule in gen_array {
            generation_rules.push(GenerationRule {
                name: rule
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                query: rule
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                template: rule
                    .get("template")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                output: rule
                    .get("output")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            });
        }
    }

    let cache_enabled = config
        .get("cache")
        .and_then(|c| c.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let cache_ttl = config
        .get("cache")
        .and_then(|c| c.get("ttl_seconds"))
        .and_then(|v| v.as_integer())
        .unwrap_or(0) as u64;

    Ok(GgenConfigData {
        project_name: project
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        ontology_source: ontology
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        base_uri: ontology
            .get("base_uri")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        generation_rules,
        cache_enabled,
        cache_ttl,
    })
}

async fn simulate_validate_config(path: &Path) -> Result<ValidationResult> {
    let content = fs::read_to_string(path)?;

    // Try parsing
    let config_result: Result<toml::Value, toml::de::Error> = toml::from_str(&content);

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if let Err(e) = config_result {
        errors.push(format!("TOML syntax error: {}", e));
        return Ok(ValidationResult {
            valid: false,
            errors,
            warnings,
        });
    }

    let config = config_result.unwrap();

    // Check required fields
    if config
        .get("project")
        .and_then(|p| p.get("name"))
        .is_none()
    {
        errors.push("Missing required field: project.name".to_string());
    }

    if config
        .get("ontology")
        .and_then(|o| o.get("source"))
        .is_none()
    {
        errors.push("Missing required field: ontology.source".to_string());
    }

    // Check for cache settings (warning if missing)
    if config.get("cache").is_none() {
        warnings.push("No cache configuration found - consider adding [cache] section".to_string());
    }

    Ok(ValidationResult {
        valid: errors.is_empty(),
        errors,
        warnings,
    })
}

async fn simulate_add_generation_rule(
    path: &Path,
    name: &str,
    query: &str,
    template: &str,
    output: &str,
) -> Result<()> {
    let content = fs::read_to_string(path)?;
    let mut config: toml::Value = toml::from_str(&content)?;

    let new_rule = toml::toml! {
        name = name
        query = query
        template = template
        output = output
    };

    if let Some(gen_array) = config.get_mut("generation").and_then(|v| v.as_array_mut()) {
        gen_array.push(new_rule);
    } else {
        config
            .as_table_mut()
            .unwrap()
            .insert("generation".to_string(), toml::Value::Array(vec![new_rule]));
    }

    fs::write(path, toml::to_string(&config)?)?;
    Ok(())
}

async fn simulate_update_generation_rule(
    path: &Path,
    name: &str,
    query: &str,
    template: &str,
    output: &str,
) -> Result<()> {
    let content = fs::read_to_string(path)?;
    let mut config: toml::Value = toml::from_str(&content)?;

    let gen_array = config
        .get_mut("generation")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| anyhow::anyhow!("No generation rules found"))?;

    let rule = gen_array
        .iter_mut()
        .find(|r| r.get("name").and_then(|v| v.as_str()) == Some(name))
        .ok_or_else(|| anyhow::anyhow!("Rule '{}' not found", name))?;

    if let Some(table) = rule.as_table_mut() {
        table.insert("query".to_string(), toml::Value::String(query.to_string()));
        table.insert(
            "template".to_string(),
            toml::Value::String(template.to_string()),
        );
        table.insert("output".to_string(), toml::Value::String(output.to_string()));
    }

    fs::write(path, toml::to_string(&config)?)?;
    Ok(())
}

async fn simulate_remove_generation_rule(path: &Path, name: &str) -> Result<()> {
    let content = fs::read_to_string(path)?;
    let mut config: toml::Value = toml::from_str(&content)?;

    if let Some(gen_array) = config.get_mut("generation").and_then(|v| v.as_array_mut()) {
        gen_array.retain(|r| r.get("name").and_then(|v| v.as_str()) != Some(name));
    }

    fs::write(path, toml::to_string(&config)?)?;
    Ok(())
}
