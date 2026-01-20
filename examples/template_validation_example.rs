//! Template Validation Example
//!
//! This example demonstrates how to use the template parameter validation
//! system in ggen-mcp.

use serde_json::json;
use spreadsheet_mcp::template::{
    ParameterDefinition, ParameterSchema, ParameterType, TemplateContext, TemplateRegistry,
    ValidationRule,
};

fn main() -> anyhow::Result<()> {
    println!("=== Template Parameter Validation Example ===\n");

    // Example 1: Basic template context usage
    basic_context_example()?;

    // Example 2: Custom schema definition
    custom_schema_example()?;

    // Example 3: Validation error handling
    validation_error_example()?;

    // Example 4: Using predefined schemas
    predefined_schema_example()?;

    Ok(())
}

fn basic_context_example() -> anyhow::Result<()> {
    println!("1. Basic Template Context Example");
    println!("-----------------------------------");

    // Create a new context for a template
    let mut ctx = TemplateContext::new("domain_entity.rs.tera");

    // Add parameters with type safety
    ctx.insert_string("entity_name", "User")?;
    ctx.insert_string("description", "User entity with authentication")?;
    ctx.insert_bool("has_id", true)?;
    ctx.insert_bool("has_timestamps", true)?;
    ctx.insert_bool("has_validation", true)?;
    ctx.insert_bool("has_builder", true)?;

    // Add complex parameters
    let fields = json!([
        {
            "name": "email",
            "rust_type": "String",
            "description": "User email address",
            "required": true
        },
        {
            "name": "age",
            "rust_type": "u32",
            "description": "User age",
            "required": false
        }
    ]);
    ctx.insert("fields", fields)?;
    ctx.insert("invariants", json!([]))?;

    println!(
        "✓ Created context with {} parameters",
        ctx.parameter_names().len()
    );
    println!("  Parameters: {:?}\n", ctx.parameter_names());

    Ok(())
}

fn custom_schema_example() -> anyhow::Result<()> {
    println!("2. Custom Schema Definition Example");
    println!("------------------------------------");

    // Define a custom schema with strict validation
    let schema = ParameterSchema::new("custom_template.rs.tera")
        .description("Custom template with comprehensive validation")
        .parameter(
            ParameterDefinition::new("module_name", ParameterType::String)
                .required()
                .rule(ValidationRule::NotEmpty)
                .rule(ValidationRule::Regex(
                    regex::Regex::new(r"^[a-z][a-z0-9_]*$").unwrap(),
                ))
                .rule(ValidationRule::MinLength(3))
                .rule(ValidationRule::MaxLength(50))
                .description("Module name in snake_case"),
        )
        .parameter(
            ParameterDefinition::new("version", ParameterType::String)
                .required()
                .rule(ValidationRule::Regex(
                    regex::Regex::new(r"^\d+\.\d+\.\d+$").unwrap(),
                ))
                .description("Semantic version number"),
        )
        .parameter(
            ParameterDefinition::new("port", ParameterType::Number)
                .default(json!(8080))
                .rule(ValidationRule::Min(1024))
                .rule(ValidationRule::Max(65535))
                .description("Server port number"),
        )
        .parameter(
            ParameterDefinition::new(
                "features",
                ParameterType::Array(Box::new(ParameterType::String)),
            )
            .default(json!([]))
            .rule(ValidationRule::MaxLength(10))
            .description("List of feature flags"),
        );

    println!("✓ Created schema: {}", schema.template_name);
    println!("  Description: {}", schema.description.as_ref().unwrap());
    println!("  Required parameters: {:?}", schema.required_parameters());
    println!(
        "  Optional parameters: {:?}\n",
        schema.optional_parameters()
    );

    Ok(())
}

fn validation_error_example() -> anyhow::Result<()> {
    println!("3. Validation Error Handling Example");
    println!("-------------------------------------");

    let schema = ParameterSchema::new("test.tera")
        .parameter(
            ParameterDefinition::new("username", ParameterType::String)
                .required()
                .rule(ValidationRule::NotEmpty)
                .rule(ValidationRule::MinLength(3))
                .rule(ValidationRule::MaxLength(20))
                .rule(ValidationRule::Regex(
                    regex::Regex::new(r"^[a-zA-Z0-9_]+$").unwrap(),
                )),
        )
        .parameter(
            ParameterDefinition::new("age", ParameterType::Number)
                .required()
                .rule(ValidationRule::Min(0))
                .rule(ValidationRule::Max(150)),
        );

    // Test case 1: Valid input
    println!("Test case 1: Valid input");
    let mut valid_context = std::collections::HashMap::new();
    valid_context.insert("username".to_string(), json!("valid_user123"));
    valid_context.insert("age".to_string(), json!(25));

    match schema.validate_context(&valid_context) {
        Ok(_) => println!("  ✓ Validation passed"),
        Err(errors) => println!("  ✗ Validation failed: {:?}", errors),
    }

    // Test case 2: Missing required parameter
    println!("\nTest case 2: Missing required parameter");
    let mut missing_context = std::collections::HashMap::new();
    missing_context.insert("username".to_string(), json!("user"));

    match schema.validate_context(&missing_context) {
        Ok(_) => println!("  ✗ Should have failed"),
        Err(errors) => {
            println!("  ✓ Caught validation errors:");
            for error in &errors {
                println!("    - {}", error);
            }
        }
    }

    // Test case 3: Invalid type
    println!("\nTest case 3: Type mismatch");
    let mut type_error_context = std::collections::HashMap::new();
    type_error_context.insert("username".to_string(), json!("valid_user"));
    type_error_context.insert("age".to_string(), json!("not a number"));

    match schema.validate_context(&type_error_context) {
        Ok(_) => println!("  ✗ Should have failed"),
        Err(errors) => {
            println!("  ✓ Caught validation errors:");
            for error in &errors {
                println!("    - {}", error);
            }
        }
    }

    // Test case 4: Validation rule failure
    println!("\nTest case 4: Validation rule failure");
    let mut rule_error_context = std::collections::HashMap::new();
    rule_error_context.insert("username".to_string(), json!("ab")); // Too short
    rule_error_context.insert("age".to_string(), json!(200)); // Too large

    match schema.validate_context(&rule_error_context) {
        Ok(_) => println!("  ✗ Should have failed"),
        Err(errors) => {
            println!("  ✓ Caught validation errors:");
            for error in &errors {
                println!("    - {}", error);
            }
        }
    }

    println!();
    Ok(())
}

fn predefined_schema_example() -> anyhow::Result<()> {
    println!("4. Using Predefined Schemas Example");
    println!("------------------------------------");

    use spreadsheet_mcp::template::schemas::{get_schema, schema_names};

    // List all available schemas
    println!("Available template schemas:");
    for name in schema_names() {
        println!("  - {}", name);
    }

    // Get a specific schema
    if let Some(schema) = get_schema("domain_entity.rs.tera") {
        println!("\nSchema details for 'domain_entity.rs.tera':");
        println!("  Description: {:?}", schema.description);
        println!("  Required parameters:");
        for param_name in schema.required_parameters() {
            if let Some(param) = schema.get_parameter(param_name) {
                println!("    - {} ({})", param.name, param.param_type);
                if let Some(desc) = &param.description {
                    println!("      {}", desc);
                }
            }
        }
        println!("  Optional parameters:");
        for param_name in schema.optional_parameters() {
            if let Some(param) = schema.get_parameter(param_name) {
                println!("    - {} ({})", param.name, param.param_type);
                if let Some(default) = &param.default {
                    println!("      Default: {}", default);
                }
            }
        }
    }

    println!();
    Ok(())
}
