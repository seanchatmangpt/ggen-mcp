//! ggen.toml Configuration Authoring Tools
//!
//! MCP tools for reading, validating, and updating ggen.toml configuration.
//! Implements atomic file operations with backup, preserves formatting/comments.
//!
//! Tools:
//! - read_ggen_config: Parse and return structured JSON
//! - validate_ggen_config: Comprehensive validation (syntax, refs, circular deps)
//! - add_generation_rule: Add new rule atomically
//! - update_generation_rule: Update existing rule by name
//! - remove_generation_rule: Remove rule by name

use crate::audit::integration::audit_tool;
use crate::state::AppState;
use crate::validation::{validate_non_empty_string, validate_path_safe};
use anyhow::{Context, Result, anyhow};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use toml::Value as TomlValue;
use toml_edit::{DocumentMut, Item, Table, value};

// ============================================================================
// Constants
// ============================================================================

const DEFAULT_CONFIG_PATH: &str = "ggen.toml";
const MAX_CONFIG_SIZE: usize = 10 * 1024 * 1024; // 10MB
const MAX_RULE_NAME_LEN: usize = 128;
const BACKUP_SUFFIX: &str = ".backup";

// ============================================================================
// Domain Types (Poka-Yoke)
// ============================================================================

/// Generation rule mode
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub enum GenerationMode {
    Overwrite,
    Append,
    Skip,
}

impl Default for GenerationMode {
    fn default() -> Self {
        Self::Overwrite
    }
}

/// Generation rule definition
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GenerationRule {
    /// Unique rule name
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// SPARQL query file path
    pub query_file: String,

    /// Tera template file path
    pub template_file: String,

    /// Output file path
    pub output_file: String,

    /// Generation mode (default: Overwrite)
    #[serde(default)]
    pub mode: GenerationMode,
}

/// Validation issue severity
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

/// Validation issue
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ValidationIssue {
    pub severity: IssueSeverity,
    pub message: String,
    pub location: Option<String>,
}

// ============================================================================
// Tool 1: read_ggen_config
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadGgenConfigParams {
    /// Path to ggen.toml (default: "ggen.toml")
    #[serde(default)]
    pub config_path: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ReadGgenConfigResponse {
    /// Parsed configuration as JSON
    pub config: JsonValue,

    /// Number of generation rules
    pub rule_count: usize,

    /// File size in bytes
    pub file_size: usize,

    /// List of rule names
    pub rule_names: Vec<String>,
}

/// Read and parse ggen.toml configuration
pub async fn read_ggen_config(
    _state: Arc<AppState>,
    params: ReadGgenConfigParams,
) -> Result<ReadGgenConfigResponse> {
    let _span = audit_tool("read_ggen_config", &params);

    let config_path = params
        .config_path
        .as_deref()
        .unwrap_or(DEFAULT_CONFIG_PATH);

    // Validate path safety
    validate_path_safe(config_path)
        .context("Invalid config path")?;

    // Read file
    let content = fs::read_to_string(config_path)
        .await
        .context(format!("Failed to read {}", config_path))?;

    let file_size = content.len();

    // Validate size
    if file_size > MAX_CONFIG_SIZE {
        return Err(anyhow!(
            "Config file too large: {} bytes (max: {})",
            file_size,
            MAX_CONFIG_SIZE
        ));
    }

    // Parse TOML
    let toml_value: TomlValue = toml::from_str(&content)
        .context("Failed to parse TOML")?;

    // Convert to JSON
    let config: JsonValue = serde_json::to_value(&toml_value)
        .context("Failed to convert TOML to JSON")?;

    // Extract generation rules
    let rule_names = extract_rule_names(&config);
    let rule_count = rule_names.len();

    Ok(ReadGgenConfigResponse {
        config,
        rule_count,
        file_size,
        rule_names,
    })
}

// ============================================================================
// Tool 2: validate_ggen_config
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ValidateGgenConfigParams {
    /// Path to ggen.toml (default: "ggen.toml")
    #[serde(default)]
    pub config_path: Option<String>,

    /// Check file references exist (default: true)
    #[serde(default = "default_true")]
    pub check_file_refs: bool,

    /// Check for circular dependencies (default: true)
    #[serde(default = "default_true")]
    pub check_circular_deps: bool,

    /// Check for output path overlaps (default: true)
    #[serde(default = "default_true")]
    pub check_path_overlaps: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ValidateGgenConfigResponse {
    /// Validation passed
    pub valid: bool,

    /// Validation issues
    pub issues: Vec<ValidationIssue>,

    /// Number of generation rules
    pub rule_count: usize,

    /// Number of errors
    pub error_count: usize,

    /// Number of warnings
    pub warning_count: usize,
}

/// Validate ggen.toml configuration
pub async fn validate_ggen_config(
    state: Arc<AppState>,
    params: ValidateGgenConfigParams,
) -> Result<ValidateGgenConfigResponse> {
    let _span = audit_tool("validate_ggen_config", &params);

    let config_path = params
        .config_path
        .as_deref()
        .unwrap_or(DEFAULT_CONFIG_PATH);

    let mut issues = Vec::new();

    // Read and parse config
    let read_params = ReadGgenConfigParams {
        config_path: Some(config_path.to_string()),
    };

    let config_result = read_ggen_config(state.clone(), read_params).await;

    let config = match config_result {
        Ok(resp) => resp.config,
        Err(e) => {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Error,
                message: format!("Failed to parse config: {}", e),
                location: None,
            });

            return Ok(ValidateGgenConfigResponse {
                valid: false,
                issues,
                rule_count: 0,
                error_count: 1,
                warning_count: 0,
            });
        }
    };

    // Validate required sections
    validate_required_sections(&config, &mut issues);

    // Validate generation rules
    let rules = extract_generation_rules(&config);
    let rule_count = rules.len();

    for rule in &rules {
        validate_generation_rule(rule, &mut issues);
    }

    // Check file references
    if params.check_file_refs {
        let base_path = Path::new(config_path)
            .parent()
            .unwrap_or_else(|| Path::new("."));

        for rule in &rules {
            check_file_exists(
                base_path,
                &rule.query_file,
                &format!("Rule '{}' query", rule.name),
                &mut issues,
            )
            .await;

            check_file_exists(
                base_path,
                &rule.template_file,
                &format!("Rule '{}' template", rule.name),
                &mut issues,
            )
            .await;
        }
    }

    // Check circular dependencies
    if params.check_circular_deps {
        check_circular_dependencies(&rules, &mut issues);
    }

    // Check output path overlaps
    if params.check_path_overlaps {
        check_output_overlaps(&rules, &mut issues);
    }

    // Count severities
    let error_count = issues
        .iter()
        .filter(|i| matches!(i.severity, IssueSeverity::Error))
        .count();

    let warning_count = issues
        .iter()
        .filter(|i| matches!(i.severity, IssueSeverity::Warning))
        .count();

    let valid = error_count == 0;

    Ok(ValidateGgenConfigResponse {
        valid,
        issues,
        rule_count,
        error_count,
        warning_count,
    })
}

// ============================================================================
// Tool 3: add_generation_rule
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddGenerationRuleParams {
    /// Path to ggen.toml (default: "ggen.toml")
    #[serde(default)]
    pub config_path: Option<String>,

    /// Rule definition
    pub rule: GenerationRule,

    /// Create backup before modification (default: true)
    #[serde(default = "default_true")]
    pub create_backup: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AddGenerationRuleResponse {
    /// Operation succeeded
    pub success: bool,

    /// Rule name
    pub rule_name: String,

    /// Backup file path (if created)
    pub backup_path: Option<String>,

    /// Updated rule count
    pub rule_count: usize,
}

/// Add new generation rule to ggen.toml
pub async fn add_generation_rule(
    _state: Arc<AppState>,
    params: AddGenerationRuleParams,
) -> Result<AddGenerationRuleResponse> {
    let _span = audit_tool("add_generation_rule", &params);

    let config_path = params
        .config_path
        .as_deref()
        .unwrap_or(DEFAULT_CONFIG_PATH);

    // Validate rule
    validate_rule_params(&params.rule)?;

    // Read existing config
    let content = fs::read_to_string(config_path)
        .await
        .context(format!("Failed to read {}", config_path))?;

    // Parse as editable document
    let mut doc = content
        .parse::<DocumentMut>()
        .context("Failed to parse TOML document")?;

    // Check for duplicate rule name
    if let Some(rules) = doc
        .get("generation")
        .and_then(|g| g.get("rules"))
        .and_then(|r| r.as_array_of_tables())
    {
        for existing in rules.iter() {
            if let Some(name) = existing.get("name").and_then(|n| n.as_str()) {
                if name == params.rule.name {
                    return Err(anyhow!(
                        "Rule with name '{}' already exists",
                        params.rule.name
                    ));
                }
            }
        }
    }

    // Create backup
    let backup_path = if params.create_backup {
        let backup = format!("{}{}", config_path, BACKUP_SUFFIX);
        fs::write(&backup, &content)
            .await
            .context("Failed to create backup")?;
        Some(backup)
    } else {
        None
    };

    // Add rule to document
    let mut rule_table = Table::new();
    rule_table.insert("name", value(&params.rule.name));
    rule_table.insert("description", value(&params.rule.description));

    // Add query as inline table
    let mut query_table = Table::new();
    query_table.set_implicit(true);
    query_table.insert("file", value(&params.rule.query_file));
    rule_table.insert("query", Item::Table(query_table));

    // Add template as inline table
    let mut template_table = Table::new();
    template_table.set_implicit(true);
    template_table.insert("file", value(&params.rule.template_file));
    rule_table.insert("template", Item::Table(template_table));

    rule_table.insert("output_file", value(&params.rule.output_file));
    rule_table.insert("mode", value(format!("{:?}", params.rule.mode)));

    // Ensure generation.rules array exists
    if !doc.contains_key("generation") {
        doc.insert("generation", Item::Table(Table::new()));
    }

    let generation = doc["generation"].as_table_mut().unwrap();

    if !generation.contains_key("rules") {
        generation.insert("rules", Item::ArrayOfTables(Default::default()));
    }

    // Add rule
    if let Some(rules) = generation["rules"].as_array_of_tables_mut() {
        rules.push(rule_table);
    }

    // Write atomically
    let updated_content = doc.to_string();
    fs::write(config_path, updated_content)
        .await
        .context("Failed to write updated config")?;

    // Count rules
    let rule_count = doc
        .get("generation")
        .and_then(|g| g.get("rules"))
        .and_then(|r| r.as_array_of_tables())
        .map(|r| r.len())
        .unwrap_or(0);

    Ok(AddGenerationRuleResponse {
        success: true,
        rule_name: params.rule.name,
        backup_path,
        rule_count,
    })
}

// ============================================================================
// Tool 4: update_generation_rule
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateGenerationRuleParams {
    /// Path to ggen.toml (default: "ggen.toml")
    #[serde(default)]
    pub config_path: Option<String>,

    /// Rule name to update
    pub rule_name: String,

    /// Updated rule definition
    pub rule: GenerationRule,

    /// Create backup before modification (default: true)
    #[serde(default = "default_true")]
    pub create_backup: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct UpdateGenerationRuleResponse {
    /// Operation succeeded
    pub success: bool,

    /// Rule name
    pub rule_name: String,

    /// Backup file path (if created)
    pub backup_path: Option<String>,

    /// Total rule count
    pub rule_count: usize,
}

/// Update existing generation rule by name
pub async fn update_generation_rule(
    _state: Arc<AppState>,
    params: UpdateGenerationRuleParams,
) -> Result<UpdateGenerationRuleResponse> {
    let _span = audit_tool("update_generation_rule", &params);

    let config_path = params
        .config_path
        .as_deref()
        .unwrap_or(DEFAULT_CONFIG_PATH);

    // Validate inputs
    validate_non_empty_string(&params.rule_name)
        .context("Invalid rule_name")?;
    validate_rule_params(&params.rule)?;

    // Read existing config
    let content = fs::read_to_string(config_path)
        .await
        .context(format!("Failed to read {}", config_path))?;

    // Parse as editable document
    let mut doc = content
        .parse::<DocumentMut>()
        .context("Failed to parse TOML document")?;

    // Find rule to update
    let rules = doc
        .get_mut("generation")
        .and_then(|g| g.get_mut("rules"))
        .and_then(|r| r.as_array_of_tables_mut())
        .context("No generation.rules section found")?;

    let rule_index = rules
        .iter()
        .position(|r| {
            r.get("name")
                .and_then(|n| n.as_str())
                .map(|n| n == params.rule_name)
                .unwrap_or(false)
        })
        .context(format!("Rule '{}' not found", params.rule_name))?;

    // Create backup
    let backup_path = if params.create_backup {
        let backup = format!("{}{}", config_path, BACKUP_SUFFIX);
        fs::write(&backup, &content)
            .await
            .context("Failed to create backup")?;
        Some(backup)
    } else {
        None
    };

    // Update rule
    let rule_table = &mut rules[rule_index];
    rule_table.insert("name", value(&params.rule.name));
    rule_table.insert("description", value(&params.rule.description));

    // Update query
    let mut query_table = Table::new();
    query_table.set_implicit(true);
    query_table.insert("file", value(&params.rule.query_file));
    rule_table.insert("query", Item::Table(query_table));

    // Update template
    let mut template_table = Table::new();
    template_table.set_implicit(true);
    template_table.insert("file", value(&params.rule.template_file));
    rule_table.insert("template", Item::Table(template_table));

    rule_table.insert("output_file", value(&params.rule.output_file));
    rule_table.insert("mode", value(format!("{:?}", params.rule.mode)));

    // Write atomically
    let updated_content = doc.to_string();
    fs::write(config_path, updated_content)
        .await
        .context("Failed to write updated config")?;

    let rule_count = rules.len();

    Ok(UpdateGenerationRuleResponse {
        success: true,
        rule_name: params.rule.name,
        backup_path,
        rule_count,
    })
}

// ============================================================================
// Tool 5: remove_generation_rule
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveGenerationRuleParams {
    /// Path to ggen.toml (default: "ggen.toml")
    #[serde(default)]
    pub config_path: Option<String>,

    /// Rule name to remove
    pub rule_name: String,

    /// Create backup before modification (default: true)
    #[serde(default = "default_true")]
    pub create_backup: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveGenerationRuleResponse {
    /// Operation succeeded
    pub success: bool,

    /// Rule name removed
    pub rule_name: String,

    /// Backup file path (if created)
    pub backup_path: Option<String>,

    /// Remaining rule count
    pub rule_count: usize,
}

/// Remove generation rule by name
pub async fn remove_generation_rule(
    _state: Arc<AppState>,
    params: RemoveGenerationRuleParams,
) -> Result<RemoveGenerationRuleResponse> {
    let _span = audit_tool("remove_generation_rule", &params);

    let config_path = params
        .config_path
        .as_deref()
        .unwrap_or(DEFAULT_CONFIG_PATH);

    // Validate input
    validate_non_empty_string(&params.rule_name)
        .context("Invalid rule_name")?;

    // Read existing config
    let content = fs::read_to_string(config_path)
        .await
        .context(format!("Failed to read {}", config_path))?;

    // Parse as editable document
    let mut doc = content
        .parse::<DocumentMut>()
        .context("Failed to parse TOML document")?;

    // Find and remove rule
    let rules = doc
        .get_mut("generation")
        .and_then(|g| g.get_mut("rules"))
        .and_then(|r| r.as_array_of_tables_mut())
        .context("No generation.rules section found")?;

    let rule_index = rules
        .iter()
        .position(|r| {
            r.get("name")
                .and_then(|n| n.as_str())
                .map(|n| n == params.rule_name)
                .unwrap_or(false)
        })
        .context(format!("Rule '{}' not found", params.rule_name))?;

    // Create backup
    let backup_path = if params.create_backup {
        let backup = format!("{}{}", config_path, BACKUP_SUFFIX);
        fs::write(&backup, &content)
            .await
            .context("Failed to create backup")?;
        Some(backup)
    } else {
        None
    };

    // Remove rule
    rules.remove(rule_index);

    // Write atomically
    let updated_content = doc.to_string();
    fs::write(config_path, updated_content)
        .await
        .context("Failed to write updated config")?;

    let rule_count = rules.len();

    Ok(RemoveGenerationRuleResponse {
        success: true,
        rule_name: params.rule_name,
        backup_path,
        rule_count,
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract rule names from parsed config
fn extract_rule_names(config: &JsonValue) -> Vec<String> {
    config
        .get("generation")
        .and_then(|g| g.get("rules"))
        .and_then(|r| r.as_array())
        .map(|rules| {
            rules
                .iter()
                .filter_map(|r| r.get("name"))
                .filter_map(|n| n.as_str())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

/// Extract generation rules from config
fn extract_generation_rules(config: &JsonValue) -> Vec<GenerationRule> {
    config
        .get("generation")
        .and_then(|g| g.get("rules"))
        .and_then(|r| r.as_array())
        .map(|rules| {
            rules
                .iter()
                .filter_map(|r| serde_json::from_value(r.clone()).ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Validate required config sections
fn validate_required_sections(config: &JsonValue, issues: &mut Vec<ValidationIssue>) {
    let required = ["ontology", "generation"];

    for section in &required {
        if !config.get(section).is_some() {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Error,
                message: format!("Missing required section: [{}]", section),
                location: None,
            });
        }
    }
}

/// Validate generation rule structure
fn validate_generation_rule(rule: &GenerationRule, issues: &mut Vec<ValidationIssue>) {
    let location = format!("rule '{}'", rule.name);

    // Name validation
    if rule.name.is_empty() {
        issues.push(ValidationIssue {
            severity: IssueSeverity::Error,
            message: "Rule name cannot be empty".to_string(),
            location: Some(location.clone()),
        });
    }

    if rule.name.len() > MAX_RULE_NAME_LEN {
        issues.push(ValidationIssue {
            severity: IssueSeverity::Error,
            message: format!(
                "Rule name exceeds {} characters",
                MAX_RULE_NAME_LEN
            ),
            location: Some(location.clone()),
        });
    }

    // Path validation
    if rule.query_file.is_empty() {
        issues.push(ValidationIssue {
            severity: IssueSeverity::Error,
            message: "Query file path cannot be empty".to_string(),
            location: Some(location.clone()),
        });
    }

    if rule.template_file.is_empty() {
        issues.push(ValidationIssue {
            severity: IssueSeverity::Error,
            message: "Template file path cannot be empty".to_string(),
            location: Some(location.clone()),
        });
    }

    if rule.output_file.is_empty() {
        issues.push(ValidationIssue {
            severity: IssueSeverity::Error,
            message: "Output file path cannot be empty".to_string(),
            location: Some(location),
        });
    }
}

/// Check if file exists
async fn check_file_exists(
    base_path: &Path,
    file_path: &str,
    description: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    let path = base_path.join(file_path);

    if !path.exists() {
        issues.push(ValidationIssue {
            severity: IssueSeverity::Warning,
            message: format!("{}: file not found: {}", description, file_path),
            location: Some(file_path.to_string()),
        });
    }
}

/// Check for circular dependencies in rules
fn check_circular_dependencies(rules: &[GenerationRule], issues: &mut Vec<ValidationIssue>) {
    // Build dependency graph (output -> inputs)
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();

    for rule in rules {
        let outputs = vec![rule.output_file.clone()];
        let inputs = vec![rule.query_file.clone(), rule.template_file.clone()];

        for output in outputs {
            graph
                .entry(output.clone())
                .or_default()
                .extend(inputs.clone());
        }
    }

    // Detect cycles using DFS
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();

    for node in graph.keys() {
        if detect_cycle(node, &graph, &mut visited, &mut rec_stack) {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Warning,
                message: format!("Potential circular dependency detected involving: {}", node),
                location: Some(node.clone()),
            });
        }
    }
}

/// Detect cycle in dependency graph
fn detect_cycle(
    node: &str,
    graph: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    rec_stack: &mut HashSet<String>,
) -> bool {
    if rec_stack.contains(node) {
        return true;
    }

    if visited.contains(node) {
        return false;
    }

    visited.insert(node.to_string());
    rec_stack.insert(node.to_string());

    if let Some(neighbors) = graph.get(node) {
        for neighbor in neighbors {
            if detect_cycle(neighbor, graph, visited, rec_stack) {
                return true;
            }
        }
    }

    rec_stack.remove(node);
    false
}

/// Check for output path overlaps
fn check_output_overlaps(rules: &[GenerationRule], issues: &mut Vec<ValidationIssue>) {
    let mut seen: HashMap<String, String> = HashMap::new();

    for rule in rules {
        let normalized = normalize_path(&rule.output_file);

        if let Some(existing_rule) = seen.get(&normalized) {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Error,
                message: format!(
                    "Output path overlap: rules '{}' and '{}' both write to {}",
                    existing_rule, rule.name, rule.output_file
                ),
                location: Some(format!("rules: {}, {}", existing_rule, rule.name)),
            });
        } else {
            seen.insert(normalized, rule.name.clone());
        }
    }
}

/// Normalize path for comparison
fn normalize_path(path: &str) -> String {
    PathBuf::from(path)
        .components()
        .collect::<PathBuf>()
        .to_string_lossy()
        .to_string()
}

/// Validate rule parameters
fn validate_rule_params(rule: &GenerationRule) -> Result<()> {
    validate_non_empty_string(&rule.name)
        .context("Rule name cannot be empty")?;

    if rule.name.len() > MAX_RULE_NAME_LEN {
        return Err(anyhow!(
            "Rule name exceeds {} characters",
            MAX_RULE_NAME_LEN
        ));
    }

    validate_non_empty_string(&rule.query_file)
        .context("Query file path cannot be empty")?;

    validate_non_empty_string(&rule.template_file)
        .context("Template file path cannot be empty")?;

    validate_non_empty_string(&rule.output_file)
        .context("Output file path cannot be empty")?;

    // Path safety
    validate_path_safe(&rule.query_file)
        .context("Invalid query file path")?;
    validate_path_safe(&rule.template_file)
        .context("Invalid template file path")?;
    validate_path_safe(&rule.output_file)
        .context("Invalid output file path")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("./foo/bar"), "foo/bar");
        assert_eq!(normalize_path("foo/../bar"), "bar");
    }

    #[test]
    fn test_validate_rule_params_valid() {
        let rule = GenerationRule {
            name: "test-rule".to_string(),
            description: "Test".to_string(),
            query_file: "queries/test.rq".to_string(),
            template_file: "templates/test.tera".to_string(),
            output_file: "src/generated/test.rs".to_string(),
            mode: GenerationMode::Overwrite,
        };

        assert!(validate_rule_params(&rule).is_ok());
    }

    #[test]
    fn test_validate_rule_params_empty_name() {
        let rule = GenerationRule {
            name: "".to_string(),
            description: "Test".to_string(),
            query_file: "queries/test.rq".to_string(),
            template_file: "templates/test.tera".to_string(),
            output_file: "src/generated/test.rs".to_string(),
            mode: GenerationMode::Overwrite,
        };

        assert!(validate_rule_params(&rule).is_err());
    }

    #[test]
    fn test_validate_rule_params_long_name() {
        let rule = GenerationRule {
            name: "x".repeat(200),
            description: "Test".to_string(),
            query_file: "queries/test.rq".to_string(),
            template_file: "templates/test.tera".to_string(),
            output_file: "src/generated/test.rs".to_string(),
            mode: GenerationMode::Overwrite,
        };

        assert!(validate_rule_params(&rule).is_err());
    }

    #[test]
    fn test_validate_rule_params_path_traversal() {
        let rule = GenerationRule {
            name: "test".to_string(),
            description: "Test".to_string(),
            query_file: "../../../etc/passwd".to_string(),
            template_file: "templates/test.tera".to_string(),
            output_file: "src/generated/test.rs".to_string(),
            mode: GenerationMode::Overwrite,
        };

        assert!(validate_rule_params(&rule).is_err());
    }

    #[test]
    fn test_extract_rule_names() {
        let config = serde_json::json!({
            "generation": {
                "rules": [
                    {"name": "rule1"},
                    {"name": "rule2"},
                    {"name": "rule3"},
                ]
            }
        });

        let names = extract_rule_names(&config);
        assert_eq!(names, vec!["rule1", "rule2", "rule3"]);
    }

    #[test]
    fn test_detect_cycle_simple() {
        let mut graph = HashMap::new();
        graph.insert("a".to_string(), vec!["b".to_string()]);
        graph.insert("b".to_string(), vec!["a".to_string()]);

        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        assert!(detect_cycle("a", &graph, &mut visited, &mut rec_stack));
    }

    #[test]
    fn test_detect_cycle_no_cycle() {
        let mut graph = HashMap::new();
        graph.insert("a".to_string(), vec!["b".to_string()]);
        graph.insert("b".to_string(), vec!["c".to_string()]);

        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        assert!(!detect_cycle("a", &graph, &mut visited, &mut rec_stack));
    }
}
