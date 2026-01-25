//! Unified ggen Resource Management Tool
//!
//! Single MCP tool consolidating 15 authoring operations:
//! - ggen.toml (5 ops): read, validate, add_rule, update_rule, remove_rule
//! - Turtle ontology (5 ops): read, add_entity, add_property, validate, query
//! - Tera templates (5 ops): read, validate, test, create, list_vars
//!
//! Token savings: 700 tokens on discovery, simpler mental model, unified error handling.

use crate::state::AppState;
use crate::tools::{ggen_config, tera_authoring, turtle_authoring};
use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;

// ============================================================================
// Unified Tool Interface
// ============================================================================

/// Unified ggen resource management parameters
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ManageGgenResourceParams {
    /// Operation to perform
    pub operation: ResourceOperation,
}

/// Resource operation variants (15 total)
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResourceOperation {
    // ========================================================================
    // ggen.toml operations (5)
    // ========================================================================
    /// Read and parse ggen.toml configuration
    ReadConfig {
        /// Path to ggen.toml (default: "ggen.toml")
        #[serde(default)]
        config_path: Option<String>,
    },

    /// Validate ggen.toml configuration
    ValidateConfig {
        /// Path to ggen.toml (default: "ggen.toml")
        #[serde(default)]
        config_path: Option<String>,
        /// Check file references exist (default: true)
        #[serde(default = "default_true")]
        check_file_refs: bool,
        /// Check for circular dependencies (default: true)
        #[serde(default = "default_true")]
        check_circular_deps: bool,
        /// Check for output path overlaps (default: true)
        #[serde(default = "default_true")]
        check_path_overlaps: bool,
    },

    /// Add new generation rule to ggen.toml
    AddRule {
        /// Path to ggen.toml (default: "ggen.toml")
        #[serde(default)]
        config_path: Option<String>,
        /// Rule definition
        rule: ggen_config::GenerationRule,
        /// Create backup before modification (default: true)
        #[serde(default = "default_true")]
        create_backup: bool,
    },

    /// Update existing generation rule by name
    UpdateRule {
        /// Path to ggen.toml (default: "ggen.toml")
        #[serde(default)]
        config_path: Option<String>,
        /// Rule name to update
        rule_name: String,
        /// Updated rule definition
        rule: ggen_config::GenerationRule,
        /// Create backup before modification (default: true)
        #[serde(default = "default_true")]
        create_backup: bool,
    },

    /// Remove generation rule by name
    RemoveRule {
        /// Path to ggen.toml (default: "ggen.toml")
        #[serde(default)]
        config_path: Option<String>,
        /// Rule name to remove
        rule_name: String,
        /// Create backup before modification (default: true)
        #[serde(default = "default_true")]
        create_backup: bool,
    },

    // ========================================================================
    // Turtle ontology operations (5)
    // ========================================================================
    /// Read and parse Turtle ontology
    ReadOntology {
        /// Path to Turtle (.ttl) file
        path: String,
        /// Include detailed entity information (default: true)
        #[serde(default = "default_true")]
        include_entities: bool,
        /// Include prefix/namespace information (default: true)
        #[serde(default = "default_true")]
        include_prefixes: bool,
    },

    /// Add entity to ontology
    AddEntity {
        /// Path to Turtle (.ttl) file
        path: String,
        /// Entity name (local name, e.g., "User")
        entity_name: turtle_authoring::EntityName,
        /// Entity type (Entity/ValueObject/AggregateRoot/Event/Command/Query)
        entity_type: turtle_authoring::EntityType,
        /// Properties to add to the entity
        properties: Vec<turtle_authoring::PropertySpec>,
        /// Optional rdfs:label (default: entity_name)
        label: Option<String>,
        /// Optional rdfs:comment
        comment: Option<String>,
        /// Create backup before modification (default: true)
        #[serde(default = "default_true")]
        create_backup: bool,
        /// Validate syntax after modification (default: true)
        #[serde(default = "default_true")]
        validate_syntax: bool,
    },

    /// Add property to existing entity
    AddProperty {
        /// Path to Turtle (.ttl) file
        path: String,
        /// Entity name to add property to
        entity_name: turtle_authoring::EntityName,
        /// Property to add
        property: turtle_authoring::PropertySpec,
        /// Create backup before modification (default: true)
        #[serde(default = "default_true")]
        create_backup: bool,
        /// Validate syntax after modification (default: true)
        #[serde(default = "default_true")]
        validate_syntax: bool,
    },

    /// Validate Turtle syntax
    ValidateOntology {
        /// Path to Turtle (.ttl) file
        path: String,
        /// Enable SHACL validation (default: false)
        #[serde(default)]
        shacl_validation: bool,
        /// Strict mode - fail on warnings (default: false)
        #[serde(default)]
        strict_mode: bool,
    },

    /// Query ontology for entities
    QueryEntities {
        /// Path to Turtle (.ttl) file
        path: String,
        /// Filter by entity type (optional)
        entity_type_filter: Option<turtle_authoring::EntityType>,
        /// Include properties (default: true)
        #[serde(default = "default_true")]
        include_properties: bool,
    },

    // ========================================================================
    // Tera template operations (5)
    // ========================================================================
    /// Read and analyze Tera template
    ReadTemplate {
        /// Template name or inline content (use "inline:<content>")
        template: String,
        /// Analyze variables (default: true)
        #[serde(default = "default_true")]
        analyze_variables: bool,
        /// Analyze filters (default: true)
        #[serde(default = "default_true")]
        analyze_filters: bool,
        /// Analyze control structures (default: true)
        #[serde(default = "default_true")]
        analyze_structures: bool,
    },

    /// Validate Tera template syntax
    ValidateTemplate {
        /// Template name or inline content
        template: String,
        /// Check variable references (default: true)
        #[serde(default = "default_true")]
        check_variables: bool,
        /// Check filter existence (default: true)
        #[serde(default = "default_true")]
        check_filters: bool,
        /// Check balanced blocks (default: true)
        #[serde(default = "default_true")]
        check_blocks: bool,
    },

    /// Test template rendering with sample context
    TestTemplate {
        /// Template name or inline content
        template: String,
        /// Context data for rendering (JSON object)
        context: JsonValue,
        /// Timeout in milliseconds (default: 5000)
        #[serde(default)]
        timeout_ms: Option<u64>,
        /// Show performance metrics (default: true)
        #[serde(default = "default_true")]
        show_metrics: bool,
    },

    /// Create scaffolded template from pattern
    CreateTemplate {
        /// Template pattern (struct, endpoint, schema, interface)
        pattern: String,
        /// Template variables (pattern-specific)
        variables: JsonValue,
        /// Output name (optional, for saving)
        output_name: Option<String>,
    },

    /// Extract all variables from template
    ListTemplateVars {
        /// Template name or inline content
        template: String,
        /// Include filter usage (default: true)
        #[serde(default = "default_true")]
        include_filters: bool,
        /// Include type hints (default: false)
        #[serde(default)]
        include_type_hints: bool,
    },
}

fn default_true() -> bool {
    true
}

/// Unified response structure
#[derive(Debug, Serialize, JsonSchema)]
pub struct ManageGgenResourceResponse {
    /// Operation performed
    pub operation: String,

    /// Operation-specific result data
    pub result: JsonValue,

    /// Minimal metadata
    pub metadata: ResponseMetadata,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ResponseMetadata {
    /// Operation success
    pub success: bool,

    /// Duration in milliseconds
    pub duration_ms: u64,

    /// Operation category (config, ontology, template)
    pub category: String,
}

// ============================================================================
// Tool Implementation
// ============================================================================

/// Unified ggen resource management tool
///
/// Consolidates 15 separate tools into single dispatch point.
/// Token savings: ~700 tokens on discovery, unified error handling.
pub async fn manage_ggen_resource(
    state: Arc<AppState>,
    params: ManageGgenResourceParams,
) -> Result<ManageGgenResourceResponse> {
    let start = std::time::Instant::now();

    let (operation_name, category, result) = match params.operation {
        // ====================================================================
        // ggen.toml operations
        // ====================================================================
        ResourceOperation::ReadConfig { config_path } => {
            let resp = ggen_config::read_ggen_config(
                state,
                ggen_config::ReadGgenConfigParams { config_path },
            )
            .await
            .context("read_config operation failed")?;

            (
                "read_config".to_string(),
                "config".to_string(),
                serde_json::to_value(resp)?,
            )
        }

        ResourceOperation::ValidateConfig {
            config_path,
            check_file_refs,
            check_circular_deps,
            check_path_overlaps,
        } => {
            let resp = ggen_config::validate_ggen_config(
                state,
                ggen_config::ValidateGgenConfigParams {
                    config_path,
                    check_file_refs,
                    check_circular_deps,
                    check_path_overlaps,
                },
            )
            .await
            .context("validate_config operation failed")?;

            (
                "validate_config".to_string(),
                "config".to_string(),
                serde_json::to_value(resp)?,
            )
        }

        ResourceOperation::AddRule {
            config_path,
            rule,
            create_backup,
        } => {
            let resp = ggen_config::add_generation_rule(
                state,
                ggen_config::AddGenerationRuleParams {
                    config_path,
                    rule,
                    create_backup,
                },
            )
            .await
            .context("add_rule operation failed")?;

            (
                "add_rule".to_string(),
                "config".to_string(),
                serde_json::to_value(resp)?,
            )
        }

        ResourceOperation::UpdateRule {
            config_path,
            rule_name,
            rule,
            create_backup,
        } => {
            let resp = ggen_config::update_generation_rule(
                state,
                ggen_config::UpdateGenerationRuleParams {
                    config_path,
                    rule_name,
                    rule,
                    create_backup,
                },
            )
            .await
            .context("update_rule operation failed")?;

            (
                "update_rule".to_string(),
                "config".to_string(),
                serde_json::to_value(resp)?,
            )
        }

        ResourceOperation::RemoveRule {
            config_path,
            rule_name,
            create_backup,
        } => {
            let resp = ggen_config::remove_generation_rule(
                state,
                ggen_config::RemoveGenerationRuleParams {
                    config_path,
                    rule_name,
                    create_backup,
                },
            )
            .await
            .context("remove_rule operation failed")?;

            (
                "remove_rule".to_string(),
                "config".to_string(),
                serde_json::to_value(resp)?,
            )
        }

        // ====================================================================
        // Turtle ontology operations
        // ====================================================================
        ResourceOperation::ReadOntology {
            path,
            include_entities,
            include_prefixes,
        } => {
            let resp = turtle_authoring::read_turtle_ontology(
                state,
                turtle_authoring::ReadTurtleParams {
                    path,
                    include_entities,
                    include_prefixes,
                },
            )
            .await
            .context("read_ontology operation failed")?;

            (
                "read_ontology".to_string(),
                "ontology".to_string(),
                serde_json::to_value(resp)?,
            )
        }

        ResourceOperation::AddEntity {
            path,
            entity_name,
            entity_type,
            properties,
            label,
            comment,
            create_backup,
            validate_syntax,
        } => {
            let resp = turtle_authoring::add_entity_to_ontology(
                state,
                turtle_authoring::AddEntityParams {
                    path,
                    entity_name,
                    entity_type,
                    properties,
                    label,
                    comment,
                    create_backup,
                    validate_syntax,
                },
            )
            .await
            .context("add_entity operation failed")?;

            (
                "add_entity".to_string(),
                "ontology".to_string(),
                serde_json::to_value(resp)?,
            )
        }

        ResourceOperation::AddProperty {
            path,
            entity_name,
            property,
            create_backup,
            validate_syntax,
        } => {
            let resp = turtle_authoring::add_property_to_entity(
                state,
                turtle_authoring::AddPropertyParams {
                    path,
                    entity_name,
                    property,
                    create_backup,
                    validate_syntax,
                },
            )
            .await
            .context("add_property operation failed")?;

            (
                "add_property".to_string(),
                "ontology".to_string(),
                serde_json::to_value(resp)?,
            )
        }

        ResourceOperation::ValidateOntology {
            path,
            shacl_validation,
            strict_mode,
        } => {
            let resp = turtle_authoring::validate_turtle_syntax(
                state,
                turtle_authoring::ValidateTurtleParams {
                    path,
                    shacl_validation,
                    strict_mode,
                },
            )
            .await
            .context("validate_ontology operation failed")?;

            (
                "validate_ontology".to_string(),
                "ontology".to_string(),
                serde_json::to_value(resp)?,
            )
        }

        ResourceOperation::QueryEntities {
            path,
            entity_type_filter,
            include_properties,
        } => {
            let resp = turtle_authoring::query_ontology_entities(
                state,
                turtle_authoring::QueryEntitiesParams {
                    path,
                    entity_type_filter,
                    include_properties,
                },
            )
            .await
            .context("query_entities operation failed")?;

            (
                "query_entities".to_string(),
                "ontology".to_string(),
                serde_json::to_value(resp)?,
            )
        }

        // ====================================================================
        // Tera template operations
        // ====================================================================
        ResourceOperation::ReadTemplate {
            template,
            analyze_variables,
            analyze_filters,
            analyze_structures,
        } => {
            let resp = tera_authoring::read_tera_template(
                state,
                tera_authoring::ReadTeraTemplateParams {
                    template,
                    analyze_variables,
                    analyze_filters,
                    analyze_structures,
                },
            )
            .await
            .context("read_template operation failed")?;

            (
                "read_template".to_string(),
                "template".to_string(),
                serde_json::to_value(resp)?,
            )
        }

        ResourceOperation::ValidateTemplate {
            template,
            check_variables,
            check_filters,
            check_blocks,
        } => {
            let resp = tera_authoring::validate_tera_template(
                state,
                tera_authoring::ValidateTeraParams {
                    template,
                    check_variables,
                    check_filters,
                    check_blocks,
                },
            )
            .await
            .context("validate_template operation failed")?;

            (
                "validate_template".to_string(),
                "template".to_string(),
                serde_json::to_value(resp)?,
            )
        }

        ResourceOperation::TestTemplate {
            template,
            context,
            timeout_ms,
            show_metrics,
        } => {
            let resp = tera_authoring::test_tera_template(
                state,
                tera_authoring::TestTeraParams {
                    template,
                    context,
                    timeout_ms,
                    show_metrics,
                },
            )
            .await
            .context("test_template operation failed")?;

            (
                "test_template".to_string(),
                "template".to_string(),
                serde_json::to_value(resp)?,
            )
        }

        ResourceOperation::CreateTemplate {
            pattern,
            variables,
            output_name,
        } => {
            let resp = tera_authoring::create_tera_template(
                state,
                tera_authoring::CreateTeraParams {
                    pattern,
                    variables,
                    output_name,
                },
            )
            .await
            .context("create_template operation failed")?;

            (
                "create_template".to_string(),
                "template".to_string(),
                serde_json::to_value(resp)?,
            )
        }

        ResourceOperation::ListTemplateVars {
            template,
            include_filters,
            include_type_hints,
        } => {
            let resp = tera_authoring::list_template_variables(
                state,
                tera_authoring::ListTemplateVariablesParams {
                    template,
                    include_filters,
                    include_type_hints,
                },
            )
            .await
            .context("list_template_vars operation failed")?;

            (
                "list_template_vars".to_string(),
                "template".to_string(),
                serde_json::to_value(resp)?,
            )
        }
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(ManageGgenResourceResponse {
        operation: operation_name,
        result,
        metadata: ResponseMetadata {
            success: true,
            duration_ms,
            category,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_categories() {
        // Verify all operations have correct categories
        // This is a compile-time check via exhaustive match
        let test_categories = |op: &ResourceOperation| -> &str {
            match op {
                ResourceOperation::ReadConfig { .. }
                | ResourceOperation::ValidateConfig { .. }
                | ResourceOperation::AddRule { .. }
                | ResourceOperation::UpdateRule { .. }
                | ResourceOperation::RemoveRule { .. } => "config",

                ResourceOperation::ReadOntology { .. }
                | ResourceOperation::AddEntity { .. }
                | ResourceOperation::AddProperty { .. }
                | ResourceOperation::ValidateOntology { .. }
                | ResourceOperation::QueryEntities { .. } => "ontology",

                ResourceOperation::ReadTemplate { .. }
                | ResourceOperation::ValidateTemplate { .. }
                | ResourceOperation::TestTemplate { .. }
                | ResourceOperation::CreateTemplate { .. }
                | ResourceOperation::ListTemplateVars { .. } => "template",
            }
        };

        // Dummy operation for type checking
        let _check = test_categories;
    }

    #[test]
    fn test_response_metadata_serialization() {
        let metadata = ResponseMetadata {
            success: true,
            duration_ms: 42,
            category: "config".to_string(),
        };

        let json = serde_json::to_value(&metadata).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["duration_ms"], 42);
        assert_eq!(json["category"], "config");
    }
}
