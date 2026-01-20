//! Template Parameter Schemas
//!
//! This module contains schema definitions for all Tera templates in ggen-mcp.
//! Each schema defines the expected parameters, their types, validation rules,
//! and whether they are required or optional.
//!
//! These schemas are used by the TemplateRegistry to validate template contexts
//! before rendering, preventing common errors like missing parameters, type
//! mismatches, and typos in parameter names.

use crate::template::parameter_validation::{
    ParameterDefinition, ParameterSchema, ParameterType, ValidationRule,
};
use indexmap::IndexMap;
use once_cell::sync::Lazy;
use regex::Regex;

/// All registered template schemas
pub static TEMPLATE_SCHEMAS: Lazy<Vec<ParameterSchema>> = Lazy::new(|| {
    vec![
        domain_entity_schema(),
        mcp_tool_handler_schema(),
        mcp_resource_handler_schema(),
        mcp_tool_params_schema(),
        mcp_tools_schema(),
        domain_service_schema(),
        value_object_schema(),
        aggregate_schema(),
        command_schema(),
        repositories_schema(),
        services_schema(),
        handlers_schema(),
        policies_schema(),
        tests_schema(),
        domain_mod_schema(),
        application_mod_schema(),
        value_objects_schema(),
    ]
});

// ============================================================================
// DOMAIN ENTITY TEMPLATE SCHEMA
// ============================================================================

fn domain_entity_schema() -> ParameterSchema {
    ParameterSchema::new("domain_entity.rs.tera")
        .description("Generates domain entity structs with validation and builder pattern")
        .parameter(
            ParameterDefinition::new("entity_name", ParameterType::String)
                .required()
                .rule(ValidationRule::NotEmpty)
                .rule(ValidationRule::Regex(
                    Regex::new(r"^[A-Za-z][A-Za-z0-9_]*$").unwrap()
                ))
                .description("Name of the entity (e.g., 'User', 'Order')")
        )
        .parameter(
            ParameterDefinition::new("description", ParameterType::String)
                .required()
                .rule(ValidationRule::NotEmpty)
                .description("Human-readable description of the entity")
        )
        .parameter(
            ParameterDefinition::new("has_id", ParameterType::Bool)
                .default(serde_json::json!(true))
                .description("Whether the entity has a unique ID field")
        )
        .parameter(
            ParameterDefinition::new("has_timestamps", ParameterType::Bool)
                .default(serde_json::json!(false))
                .description("Whether to include created_at and updated_at timestamps")
        )
        .parameter(
            ParameterDefinition::new("has_validation", ParameterType::Bool)
                .default(serde_json::json!(true))
                .description("Whether to include validation methods")
        )
        .parameter(
            ParameterDefinition::new("has_builder", ParameterType::Bool)
                .default(serde_json::json!(true))
                .description("Whether to generate a builder pattern implementation")
        )
        .parameter(
            ParameterDefinition::new("fields", ParameterType::Array(Box::new(
                ParameterType::Object(field_object_type())
            )))
                .default(serde_json::json!([]))
                .description("Array of field definitions for the entity")
        )
        .parameter(
            ParameterDefinition::new("invariants", ParameterType::Array(Box::new(
                ParameterType::Object(invariant_object_type())
            )))
                .default(serde_json::json!([]))
                .description("Array of business invariants to enforce")
        )
}

// ============================================================================
// MCP TOOL HANDLER TEMPLATE SCHEMA
// ============================================================================

fn mcp_tool_handler_schema() -> ParameterSchema {
    ParameterSchema::new("mcp_tool_handler.rs.tera")
        .description("Generates MCP tool handler with parameter validation and response types")
        .parameter(
            ParameterDefinition::new("tool_name", ParameterType::String)
                .required()
                .rule(ValidationRule::NotEmpty)
                .rule(ValidationRule::Regex(
                    Regex::new(r"^[a-z][a-z0-9_]*$").unwrap()
                ))
                .description("Name of the tool in snake_case (e.g., 'list_workbooks')")
        )
        .parameter(
            ParameterDefinition::new("description", ParameterType::String)
                .required()
                .rule(ValidationRule::NotEmpty)
                .description("Description of what the tool does")
        )
        .parameter(
            ParameterDefinition::new("category", ParameterType::String)
                .default(serde_json::json!("general"))
                .description("Tool category for organization")
        )
        .parameter(
            ParameterDefinition::new("has_params", ParameterType::Bool)
                .default(serde_json::json!(true))
                .description("Whether the tool accepts parameters")
        )
        .parameter(
            ParameterDefinition::new("has_pagination", ParameterType::Bool)
                .default(serde_json::json!(false))
                .description("Whether to include pagination support")
        )
        .parameter(
            ParameterDefinition::new("has_filters", ParameterType::Bool)
                .default(serde_json::json!(false))
                .description("Whether to include filter support")
        )
        .parameter(
            ParameterDefinition::new("params", ParameterType::Array(Box::new(
                ParameterType::Object(param_object_type())
            )))
                .default(serde_json::json!([]))
                .description("Array of parameter definitions")
        )
        .parameter(
            ParameterDefinition::new("response_fields", ParameterType::Array(Box::new(
                ParameterType::Object(field_object_type())
            )))
                .default(serde_json::json!([]))
                .description("Array of response field definitions")
        )
}

// ============================================================================
// MCP RESOURCE HANDLER TEMPLATE SCHEMA
// ============================================================================

fn mcp_resource_handler_schema() -> ParameterSchema {
    ParameterSchema::new("mcp_resource_handler.rs.tera")
        .description("Generates MCP resource handler with caching and subscription support")
        .parameter(
            ParameterDefinition::new("resource_name", ParameterType::String)
                .required()
                .rule(ValidationRule::NotEmpty)
                .rule(ValidationRule::Regex(
                    Regex::new(r"^[A-Za-z][A-Za-z0-9_]*$").unwrap()
                ))
                .description("Name of the resource (e.g., 'Workbook', 'Cell')")
        )
        .parameter(
            ParameterDefinition::new("description", ParameterType::String)
                .required()
                .rule(ValidationRule::NotEmpty)
                .description("Description of the resource")
        )
        .parameter(
            ParameterDefinition::new("uri_template", ParameterType::String)
                .default(serde_json::json!(""))
                .description("URI template pattern for the resource")
        )
        .parameter(
            ParameterDefinition::new("mime_type", ParameterType::String)
                .default(serde_json::json!("application/json"))
                .description("MIME type of the resource content")
        )
        .parameter(
            ParameterDefinition::new("has_caching", ParameterType::Bool)
                .default(serde_json::json!(false))
                .description("Whether to include caching support")
        )
        .parameter(
            ParameterDefinition::new("has_subscriptions", ParameterType::Bool)
                .default(serde_json::json!(false))
                .description("Whether to include subscription/notification support")
        )
        .parameter(
            ParameterDefinition::new("cache_ttl_secs", ParameterType::Number)
                .default(serde_json::json!(300))
                .rule(ValidationRule::Min(0))
                .rule(ValidationRule::Max(86400)) // Max 1 day
                .description("Cache time-to-live in seconds")
        )
        .parameter(
            ParameterDefinition::new("fields", ParameterType::Array(Box::new(
                ParameterType::Object(field_object_type())
            )))
                .default(serde_json::json!([]))
                .description("Array of resource field definitions")
        )
}

// ============================================================================
// MCP TOOL PARAMS TEMPLATE SCHEMA
// ============================================================================

fn mcp_tool_params_schema() -> ParameterSchema {
    ParameterSchema::new("mcp_tool_params.rs.tera")
        .description("Generates parameter structs from SPARQL query results")
        .parameter(
            ParameterDefinition::new("sparql_results", ParameterType::Array(Box::new(
                ParameterType::Object(sparql_result_object_type())
            )))
                .required()
                .description("Array of SPARQL query results containing tool parameter definitions")
        )
}

// ============================================================================
// MCP TOOLS TEMPLATE SCHEMA
// ============================================================================

fn mcp_tools_schema() -> ParameterSchema {
    ParameterSchema::new("mcp_tools.rs.tera")
        .description("Generates MCP tools module from SPARQL query results")
        .parameter(
            ParameterDefinition::new("sparql_results", ParameterType::Array(Box::new(
                ParameterType::Object(sparql_result_object_type())
            )))
                .required()
                .description("Array of SPARQL query results containing tool definitions")
        )
}

// ============================================================================
// DOMAIN SERVICE TEMPLATE SCHEMA
// ============================================================================

fn domain_service_schema() -> ParameterSchema {
    ParameterSchema::new("domain_service.rs.tera")
        .description("Generates domain service with business logic")
        .parameter(
            ParameterDefinition::new("service_name", ParameterType::String)
                .required()
                .rule(ValidationRule::NotEmpty)
                .rule(ValidationRule::Regex(
                    Regex::new(r"^[A-Za-z][A-Za-z0-9_]*$").unwrap()
                ))
                .description("Name of the service")
        )
        .parameter(
            ParameterDefinition::new("description", ParameterType::String)
                .required()
                .description("Service description")
        )
        .parameter(
            ParameterDefinition::new("methods", ParameterType::Array(Box::new(
                ParameterType::Object(method_object_type())
            )))
                .default(serde_json::json!([]))
                .description("Array of service method definitions")
        )
}

// ============================================================================
// VALUE OBJECT TEMPLATE SCHEMA
// ============================================================================

fn value_object_schema() -> ParameterSchema {
    ParameterSchema::new("value_object.rs.tera")
        .description("Generates value object with immutability and validation")
        .parameter(
            ParameterDefinition::new("vo_name", ParameterType::String)
                .required()
                .rule(ValidationRule::NotEmpty)
                .rule(ValidationRule::Regex(
                    Regex::new(r"^[A-Za-z][A-Za-z0-9_]*$").unwrap()
                ))
                .description("Name of the value object")
        )
        .parameter(
            ParameterDefinition::new("properties", ParameterType::Array(Box::new(
                ParameterType::Object(property_object_type())
            )))
                .default(serde_json::json!([]))
                .description("Array of property definitions")
        )
        .parameter(
            ParameterDefinition::new("invariants", ParameterType::Array(Box::new(
                ParameterType::Object(invariant_object_type())
            )))
                .default(serde_json::json!([]))
                .description("Array of invariants to validate")
        )
}

// ============================================================================
// AGGREGATE TEMPLATE SCHEMA
// ============================================================================

fn aggregate_schema() -> ParameterSchema {
    ParameterSchema::new("aggregate.rs.tera")
        .description("Generates domain aggregate root")
        .allow_unknown() // This template has a simple structure
}

// ============================================================================
// COMMAND TEMPLATE SCHEMA
// ============================================================================

fn command_schema() -> ParameterSchema {
    ParameterSchema::new("command.rs.tera")
        .description("Generates command pattern implementation")
        .parameter(
            ParameterDefinition::new("command_name", ParameterType::String)
                .required()
                .rule(ValidationRule::NotEmpty)
                .description("Name of the command")
        )
        .parameter(
            ParameterDefinition::new("params", ParameterType::Array(Box::new(
                ParameterType::Object(param_object_type())
            )))
                .default(serde_json::json!([]))
                .description("Command parameters")
        )
}

// ============================================================================
// SIMPLE MODULE SCHEMAS
// ============================================================================

fn repositories_schema() -> ParameterSchema {
    ParameterSchema::new("repositories.rs.tera")
        .description("Generates repository traits and implementations")
        .allow_unknown()
}

fn services_schema() -> ParameterSchema {
    ParameterSchema::new("services.rs.tera")
        .description("Generates service layer module")
        .allow_unknown()
}

fn handlers_schema() -> ParameterSchema {
    ParameterSchema::new("handlers.rs.tera")
        .description("Generates handler module")
        .allow_unknown()
}

fn policies_schema() -> ParameterSchema {
    ParameterSchema::new("policies.rs.tera")
        .description("Generates policy module")
        .allow_unknown()
}

fn tests_schema() -> ParameterSchema {
    ParameterSchema::new("tests.rs.tera")
        .description("Generates test module")
        .allow_unknown()
}

fn domain_mod_schema() -> ParameterSchema {
    ParameterSchema::new("domain_mod.rs.tera")
        .description("Generates domain module exports")
        .allow_unknown()
}

fn application_mod_schema() -> ParameterSchema {
    ParameterSchema::new("application_mod.rs.tera")
        .description("Generates application module exports")
        .allow_unknown()
}

fn value_objects_schema() -> ParameterSchema {
    ParameterSchema::new("value_objects.rs.tera")
        .description("Generates value objects module")
        .allow_unknown()
}

// ============================================================================
// OBJECT TYPE HELPERS
// ============================================================================

/// Field object type definition
fn field_object_type() -> IndexMap<String, ParameterType> {
    let mut map = IndexMap::new();
    map.insert("name".to_string(), ParameterType::String);
    map.insert("rust_type".to_string(), ParameterType::Optional(Box::new(ParameterType::String)));
    map.insert("description".to_string(), ParameterType::Optional(Box::new(ParameterType::String)));
    map.insert("required".to_string(), ParameterType::Optional(Box::new(ParameterType::Bool)));
    map.insert("optional".to_string(), ParameterType::Optional(Box::new(ParameterType::Bool)));
    map
}

/// Invariant object type definition
fn invariant_object_type() -> IndexMap<String, ParameterType> {
    let mut map = IndexMap::new();
    map.insert("expression".to_string(), ParameterType::String);
    map.insert("message".to_string(), ParameterType::Optional(Box::new(ParameterType::String)));
    map.insert("description".to_string(), ParameterType::Optional(Box::new(ParameterType::String)));
    map
}

/// Parameter object type definition
fn param_object_type() -> IndexMap<String, ParameterType> {
    let mut map = IndexMap::new();
    map.insert("name".to_string(), ParameterType::String);
    map.insert("rust_type".to_string(), ParameterType::Optional(Box::new(ParameterType::String)));
    map.insert("description".to_string(), ParameterType::Optional(Box::new(ParameterType::String)));
    map.insert("required".to_string(), ParameterType::Optional(Box::new(ParameterType::Bool)));
    map.insert("default".to_string(), ParameterType::Optional(Box::new(ParameterType::String)));
    map.insert("filterable".to_string(), ParameterType::Optional(Box::new(ParameterType::Bool)));
    map
}

/// Property object type definition
fn property_object_type() -> IndexMap<String, ParameterType> {
    let mut map = IndexMap::new();
    map.insert("name".to_string(), ParameterType::String);
    map.insert("type".to_string(), ParameterType::String);
    map
}

/// Method object type definition
fn method_object_type() -> IndexMap<String, ParameterType> {
    let mut map = IndexMap::new();
    map.insert("name".to_string(), ParameterType::String);
    map.insert("description".to_string(), ParameterType::Optional(Box::new(ParameterType::String)));
    map.insert("params".to_string(), ParameterType::Optional(Box::new(
        ParameterType::Array(Box::new(ParameterType::Object(param_object_type())))
    )));
    map.insert("return_type".to_string(), ParameterType::Optional(Box::new(ParameterType::String)));
    map
}

/// SPARQL result object type definition
fn sparql_result_object_type() -> IndexMap<String, ParameterType> {
    let mut map = IndexMap::new();
    // SPARQL results can have any fields with ? prefix
    // We'll use Any type for flexibility
    map.insert("?toolName".to_string(), ParameterType::Optional(Box::new(ParameterType::String)));
    map.insert("?paramName".to_string(), ParameterType::Optional(Box::new(ParameterType::String)));
    map.insert("?paramType".to_string(), ParameterType::Optional(Box::new(ParameterType::String)));
    map.insert("?paramRequired".to_string(), ParameterType::Optional(Box::new(ParameterType::String)));
    map.insert("?paramDescription".to_string(), ParameterType::Optional(Box::new(ParameterType::String)));
    map.insert("?paramAlias".to_string(), ParameterType::Optional(Box::new(ParameterType::String)));
    map.insert("?paramDefault".to_string(), ParameterType::Optional(Box::new(ParameterType::String)));
    map
}

// ============================================================================
// SCHEMA LOOKUP
// ============================================================================

/// Get a schema by template name
pub fn get_schema(template_name: &str) -> Option<ParameterSchema> {
    TEMPLATE_SCHEMAS
        .iter()
        .find(|s| s.template_name == template_name)
        .cloned()
}

/// Get all schema names
pub fn schema_names() -> Vec<&'static str> {
    TEMPLATE_SCHEMAS
        .iter()
        .map(|s| s.template_name.as_str())
        .collect()
}

/// Print a summary of all schemas
pub fn print_schema_summary() {
    println!("Template Parameter Schemas:");
    println!("==========================\n");

    for schema in TEMPLATE_SCHEMAS.iter() {
        println!("Template: {}", schema.template_name);
        if let Some(desc) = &schema.description {
            println!("Description: {}", desc);
        }
        println!("Required parameters:");
        for param_name in schema.required_parameters() {
            if let Some(param) = schema.get_parameter(param_name) {
                println!("  - {} ({})", param.name, param.param_type);
                if let Some(desc) = &param.description {
                    println!("    {}", desc);
                }
            }
        }
        println!("Optional parameters:");
        for param_name in schema.optional_parameters() {
            if let Some(param) = schema.get_parameter(param_name) {
                println!("  - {} ({})", param.name, param.param_type);
                if let Some(desc) = &param.description {
                    println!("    {}", desc);
                }
                if let Some(default) = &param.default {
                    println!("    Default: {}", default);
                }
            }
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_schemas_registered() {
        assert!(TEMPLATE_SCHEMAS.len() > 0);
        println!("Registered {} template schemas", TEMPLATE_SCHEMAS.len());
    }

    #[test]
    fn test_domain_entity_schema() {
        let schema = domain_entity_schema();
        assert_eq!(schema.template_name, "domain_entity.rs.tera");
        assert!(schema.get_parameter("entity_name").is_some());
        assert!(schema.get_parameter("description").is_some());
    }

    #[test]
    fn test_schema_lookup() {
        let schema = get_schema("domain_entity.rs.tera");
        assert!(schema.is_some());
        assert_eq!(schema.unwrap().template_name, "domain_entity.rs.tera");
    }

    #[test]
    fn test_schema_names() {
        let names = schema_names();
        assert!(names.contains(&"domain_entity.rs.tera"));
        assert!(names.contains(&"mcp_tool_handler.rs.tera"));
    }
}
