# Template Parameter Validation

## Overview

The Template Parameter Validation system provides comprehensive type-safety and error-proofing for Tera templates in ggen-mcp. It implements poka-yoke (error-proofing) principles from the Toyota Production System to prevent template rendering errors before they occur.

## Table of Contents

- [Features](#features)
- [Quick Start](#quick-start)
- [Core Components](#core-components)
- [Parameter Schema Format](#parameter-schema-format)
- [Validation Rules](#validation-rules)
- [Common Template Errors](#common-template-errors)
- [Safe Template Patterns](#safe-template-patterns)
- [Integration Guide](#integration-guide)
- [Template Schemas](#template-schemas)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Features

### 1. Type-Safe Context Building

```rust
use ggen_mcp::template::TemplateContext;

let mut ctx = TemplateContext::new("domain_entity.rs.tera");
ctx.insert_string("entity_name", "User")?;
ctx.insert_bool("has_id", true)?;
ctx.insert_array("fields", vec![/* ... */])?;

// Validate before rendering
ctx.validate()?;
```

### 2. Parameter Schema Definition

```rust
use ggen_mcp::template::{ParameterSchema, ParameterDefinition, ParameterType, ValidationRule};

let schema = ParameterSchema::new("my_template.rs.tera")
    .description("Generates custom code")
    .parameter(
        ParameterDefinition::new("name", ParameterType::String)
            .required()
            .rule(ValidationRule::NotEmpty)
            .description("The name parameter")
    )
    .parameter(
        ParameterDefinition::new("count", ParameterType::Number)
            .default(serde_json::json!(10))
            .rule(ValidationRule::Min(1))
            .rule(ValidationRule::Max(100))
    );
```

### 3. Pre-Render Validation

- **Syntax checking** before execution
- **Undefined variable** detection
- **Type compatibility** checking
- **Parameter typo** detection
- **Unused parameter** warnings

### 4. Safe Custom Filters

```rust
use ggen_mcp::template::SafeFilter;
use tera::{Filter, Value};

struct MyFilter;
impl Filter for MyFilter {
    fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
        // Filter implementation with validation
        Ok(value.clone())
    }
}

let safe_filter = SafeFilter::new("my_filter", Box::new(MyFilter))
    .with_rate_limit(100); // Max 100 calls per second

registry.register_filter(safe_filter);
```

### 5. Centralized Template Management

```rust
use ggen_mcp::template::TemplateRegistry;

let mut registry = TemplateRegistry::new()?;

// Register schemas
registry.register_schemas(TEMPLATE_SCHEMAS.clone());

// Render with validation
let output = registry.render("domain_entity.rs.tera", &ctx)?;
```

## Quick Start

### Basic Usage

```rust
use ggen_mcp::template::{TemplateContext, TemplateRegistry};

// 1. Create a context
let mut ctx = TemplateContext::new("domain_entity.rs.tera");

// 2. Add parameters
ctx.insert_string("entity_name", "User")?;
ctx.insert_string("description", "User entity with authentication")?;
ctx.insert_bool("has_id", true)?;
ctx.insert_bool("has_timestamps", true)?;
ctx.insert_bool("has_validation", true)?;
ctx.insert_bool("has_builder", true)?;

// 3. Add complex parameters
let fields = serde_json::json!([
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

// 4. Validate and render
let registry = TemplateRegistry::new()?;
let output = registry.render("domain_entity.rs.tera", &ctx)?;

println!("{}", output);
```

### Advanced Usage with Custom Schemas

```rust
use ggen_mcp::template::*;
use regex::Regex;

// Define a custom schema
let schema = ParameterSchema::new("custom_template.rs.tera")
    .description("Custom template with strict validation")
    .parameter(
        ParameterDefinition::new("module_name", ParameterType::String)
            .required()
            .rule(ValidationRule::Regex(
                Regex::new(r"^[a-z][a-z0-9_]*$").unwrap()
            ))
            .rule(ValidationRule::MinLength(3))
            .rule(ValidationRule::MaxLength(50))
            .description("Module name in snake_case")
    )
    .parameter(
        ParameterDefinition::new("features", ParameterType::Array(
            Box::new(ParameterType::String)
        ))
            .default(serde_json::json!([]))
            .rule(ValidationRule::MaxLength(10))
            .description("List of feature flags")
    );

// Register the schema
let mut registry = TemplateRegistry::new()?;
registry.register_schema(schema);

// Use it
let mut ctx = TemplateContext::new("custom_template.rs.tera");
ctx.insert_string("module_name", "user_auth")?;
ctx.insert_array("features", vec![
    serde_json::json!("authentication"),
    serde_json::json!("authorization"),
])?;

let output = registry.render("custom_template.rs.tera", &ctx)?;
```

## Core Components

### 1. TemplateContext

A type-safe builder for template contexts that tracks parameters and validates before rendering.

**Methods:**
- `new(template_name)` - Create a new context
- `insert_string(name, value)` - Add a string parameter
- `insert_bool(name, value)` - Add a boolean parameter
- `insert_number(name, value)` - Add a number parameter
- `insert_float(name, value)` - Add a float parameter
- `insert_array(name, value)` - Add an array parameter
- `insert_object(name, value)` - Add an object parameter
- `insert(name, value)` - Add a raw JSON value
- `validate()` - Validate the context
- `unused_parameters()` - Get list of unused parameters

### 2. ParameterSchema

Defines expected parameters for a template with types and validation rules.

**Fields:**
- `template_name` - Name of the template
- `parameters` - Map of parameter definitions
- `allow_unknown` - Whether to allow unknown parameters
- `description` - Template description

**Methods:**
- `new(template_name)` - Create a new schema
- `parameter(definition)` - Add a parameter definition
- `allow_unknown()` - Enable unknown parameters
- `description(text)` - Set description
- `validate_context(context)` - Validate a context

### 3. ParameterDefinition

Defines a single parameter with type and validation rules.

**Fields:**
- `name` - Parameter name
- `param_type` - Parameter type
- `required` - Whether required
- `default` - Default value
- `rules` - Validation rules
- `description` - Parameter description

**Methods:**
- `new(name, type)` - Create a new definition
- `required()` - Mark as required
- `default(value)` - Set default value
- `rule(rule)` - Add validation rule
- `description(text)` - Set description

### 4. TemplateValidator

Pre-render validator that checks syntax and parameters.

**Methods:**
- `new(template_dir)` - Create a new validator
- `register_schema(schema)` - Register a schema
- `validate_syntax(template_name)` - Check template syntax
- `validate_context(context)` - Validate a context

### 5. SafeFilterRegistry

Registry for safe custom filters with input validation and rate limiting.

**Methods:**
- `new()` - Create a new registry
- `register(filter)` - Register a safe filter
- `get(name)` - Get a filter by name
- `register_with_tera(tera)` - Register all filters with Tera

### 6. TemplateRegistry

Centralized template management with validation and hot reload.

**Methods:**
- `new()` - Create a registry (uses `templates/` directory)
- `with_template_dir(path)` - Create with custom directory
- `register_schema(schema)` - Register a schema
- `register_schemas(schemas)` - Register multiple schemas
- `register_filter(filter)` - Register a safe filter
- `render(template_name, context)` - Render a template
- `reload()` - Reload all templates

## Parameter Schema Format

### Parameter Types

```rust
pub enum ParameterType {
    String,                          // String value
    Bool,                            // Boolean value
    Number,                          // Integer number (i64)
    Float,                           // Floating point number (f64)
    Array(Box<ParameterType>),      // Array of values
    Object(IndexMap<String, ParameterType>), // Object with typed fields
    Optional(Box<ParameterType>),   // Optional parameter
    Any,                             // Any type (no validation)
}
```

### Example Schema

```rust
let schema = ParameterSchema::new("domain_entity.rs.tera")
    .description("Generates domain entity")
    .parameter(
        ParameterDefinition::new("entity_name", ParameterType::String)
            .required()
            .rule(ValidationRule::NotEmpty)
            .rule(ValidationRule::Regex(Regex::new(r"^[A-Z][A-Za-z0-9]*$").unwrap()))
            .description("Entity name in PascalCase")
    )
    .parameter(
        ParameterDefinition::new("fields", ParameterType::Array(
            Box::new(ParameterType::Object(field_type))
        ))
            .default(serde_json::json!([]))
            .description("Entity fields")
    )
    .parameter(
        ParameterDefinition::new("has_id", ParameterType::Bool)
            .default(serde_json::json!(true))
            .description("Whether to include ID field")
    );
```

## Validation Rules

### Built-in Rules

1. **MinLength(n)** - Minimum length for strings/arrays
   ```rust
   .rule(ValidationRule::MinLength(3))
   ```

2. **MaxLength(n)** - Maximum length for strings/arrays
   ```rust
   .rule(ValidationRule::MaxLength(100))
   ```

3. **Min(n)** - Minimum value for numbers
   ```rust
   .rule(ValidationRule::Min(0))
   ```

4. **Max(n)** - Maximum value for numbers
   ```rust
   .rule(ValidationRule::Max(1000))
   ```

5. **Regex(pattern)** - Regex pattern for strings
   ```rust
   .rule(ValidationRule::Regex(Regex::new(r"^[a-z]+$").unwrap()))
   ```

6. **NotEmpty** - Value must not be empty
   ```rust
   .rule(ValidationRule::NotEmpty)
   ```

7. **OneOf(values)** - Value must be one of the specified options
   ```rust
   .rule(ValidationRule::OneOf(vec![
       serde_json::json!("debug"),
       serde_json::json!("release"),
   ]))
   ```

8. **Custom(fn)** - Custom validation function
   ```rust
   .rule(ValidationRule::Custom(Arc::new(|value| {
       // Custom validation logic
       if /* some condition */ {
           Ok(())
       } else {
           Err(anyhow!("validation failed"))
       }
   })))
   ```

### Combining Rules

```rust
ParameterDefinition::new("username", ParameterType::String)
    .required()
    .rules(vec![
        ValidationRule::NotEmpty,
        ValidationRule::MinLength(3),
        ValidationRule::MaxLength(20),
        ValidationRule::Regex(Regex::new(r"^[a-zA-Z0-9_]+$").unwrap()),
    ])
```

## Common Template Errors

### 1. Missing Required Parameters

**Error:**
```
missing required parameter: entity_name
```

**Solution:**
```rust
ctx.insert_string("entity_name", "User")?;
```

### 2. Type Mismatch

**Error:**
```
type mismatch for parameter 'count': expected Number, got String
```

**Solution:**
```rust
// Wrong:
ctx.insert_string("count", "42")?;

// Correct:
ctx.insert_number("count", 42)?;
```

### 3. Unknown Parameter (Typo)

**Error:**
```
unknown parameter: entitiy_name
```

**Solution:**
Check parameter name spelling against the schema.

### 4. Validation Rule Failure

**Error:**
```
regex validation failed for entity_name: ^[A-Z][A-Za-z0-9]*$
```

**Solution:**
Ensure the value matches the expected pattern (PascalCase in this example).

### 5. Unused Parameters Warning

**Warning:**
```
unused parameters detected: ["debug_mode"]
```

**Solution:**
Either use the parameter in the template or remove it from the context.

## Safe Template Patterns

### 1. Always Validate Before Rendering

```rust
// GOOD
let mut ctx = TemplateContext::new("template.tera");
ctx.insert_string("name", "value")?;
ctx.validate()?;
let output = registry.render("template.tera", &ctx)?;

// BAD - No validation
let output = tera.render("template.tera", &context)?;
```

### 2. Use Type-Safe Insertions

```rust
// GOOD
ctx.insert_bool("enabled", true)?;
ctx.insert_number("count", 42)?;

// BAD - Using generic insert with wrong types
ctx.insert("enabled", serde_json::json!("true"))?; // String instead of bool
```

### 3. Define Strict Schemas

```rust
// GOOD - Strict schema with validation
ParameterSchema::new("template.tera")
    .parameter(
        ParameterDefinition::new("name", ParameterType::String)
            .required()
            .rule(ValidationRule::NotEmpty)
    )

// BAD - Allowing unknown parameters without validation
ParameterSchema::new("template.tera")
    .allow_unknown()
```

### 4. Use Default Values for Optional Parameters

```rust
// GOOD
ParameterDefinition::new("has_logging", ParameterType::Bool)
    .default(serde_json::json!(false))

// BAD - Required parameter that should be optional
ParameterDefinition::new("has_logging", ParameterType::Bool)
    .required()
```

### 5. Prevent XSS in Generated Code

```rust
// Use safe filters
SafeFilter::new("sanitize", Box::new(SanitizeFilter))
    .with_rate_limit(1000)
```

### 6. Limit Array Sizes

```rust
ParameterDefinition::new("items", ParameterType::Array(Box::new(ParameterType::String)))
    .rule(ValidationRule::MaxLength(1000))
```

## Integration Guide

### Step 1: Define Your Schema

Create a schema for your template in `src/template/schemas.rs`:

```rust
fn my_template_schema() -> ParameterSchema {
    ParameterSchema::new("my_template.rs.tera")
        .description("My custom template")
        .parameter(
            ParameterDefinition::new("name", ParameterType::String)
                .required()
                .rule(ValidationRule::NotEmpty)
        )
        // Add more parameters...
}
```

### Step 2: Register the Schema

Add your schema to the `TEMPLATE_SCHEMAS` list:

```rust
pub static TEMPLATE_SCHEMAS: Lazy<Vec<ParameterSchema>> = Lazy::new(|| {
    vec![
        // ... existing schemas
        my_template_schema(),
    ]
});
```

### Step 3: Use the Template

```rust
use ggen_mcp::template::{TemplateContext, TemplateRegistry};

let mut registry = TemplateRegistry::new()?;
registry.register_schemas(TEMPLATE_SCHEMAS.clone());

let mut ctx = TemplateContext::new("my_template.rs.tera");
ctx.insert_string("name", "MyModule")?;

let output = registry.render("my_template.rs.tera", &ctx)?;
```

### Step 4: Add Tests

```rust
#[test]
fn test_my_template_validation() {
    let schema = my_template_schema();

    // Test valid context
    let mut valid_ctx = HashMap::new();
    valid_ctx.insert("name".to_string(), serde_json::json!("Test"));
    assert!(schema.validate_context(&valid_ctx).is_ok());

    // Test missing required parameter
    let invalid_ctx = HashMap::new();
    assert!(schema.validate_context(&invalid_ctx).is_err());
}
```

## Template Schemas

### Available Template Schemas

The following templates have predefined schemas:

1. **domain_entity.rs.tera** - Domain entity generation
2. **mcp_tool_handler.rs.tera** - MCP tool handler generation
3. **mcp_resource_handler.rs.tera** - MCP resource handler generation
4. **mcp_tool_params.rs.tera** - MCP tool parameters from SPARQL
5. **mcp_tools.rs.tera** - MCP tools module generation
6. **domain_service.rs.tera** - Domain service generation
7. **value_object.rs.tera** - Value object generation
8. **aggregate.rs.tera** - Aggregate root generation
9. **command.rs.tera** - Command pattern generation

### View All Schemas

```rust
use ggen_mcp::template::schemas::print_schema_summary;

print_schema_summary();
```

### Get a Specific Schema

```rust
use ggen_mcp::template::schemas::get_schema;

if let Some(schema) = get_schema("domain_entity.rs.tera") {
    println!("Required parameters: {:?}", schema.required_parameters());
    println!("Optional parameters: {:?}", schema.optional_parameters());
}
```

## Best Practices

### 1. Always Define Schemas

Every template should have a corresponding schema, even if it's simple:

```rust
fn simple_template_schema() -> ParameterSchema {
    ParameterSchema::new("simple.rs.tera")
        .allow_unknown()  // If parameters are dynamic
}
```

### 2. Use Descriptive Parameter Names

```rust
// GOOD
"entity_name", "has_timestamps", "field_definitions"

// BAD
"n", "flag", "data"
```

### 3. Provide Clear Descriptions

```rust
ParameterDefinition::new("timeout_secs", ParameterType::Number)
    .description("Timeout in seconds (default: 30, max: 300)")
    .default(serde_json::json!(30))
    .rule(ValidationRule::Max(300))
```

### 4. Use Appropriate Types

```rust
// GOOD
ParameterType::Bool for flags
ParameterType::Number for counts
ParameterType::String for names
ParameterType::Array for lists

// BAD
ParameterType::String for everything
```

### 5. Validate Early

```rust
// Validate as soon as the context is built
let mut ctx = TemplateContext::new("template.tera");
ctx.insert_string("name", user_input)?;
ctx.validate()?;  // Fail fast if invalid
```

### 6. Handle Errors Gracefully

```rust
match registry.render("template.tera", &ctx) {
    Ok(output) => {
        // Use output
    }
    Err(e) => {
        eprintln!("Template rendering failed: {}", e);
        // Provide helpful error message to user
    }
}
```

### 7. Test Your Templates

```rust
#[test]
fn test_template_rendering() {
    let registry = TemplateRegistry::new().unwrap();
    let mut ctx = TemplateContext::new("test.tera");
    ctx.insert_string("name", "Test").unwrap();

    let output = registry.render("test.tera", &ctx).unwrap();
    assert!(output.contains("Test"));
}
```

## Troubleshooting

### Problem: Template Not Found

**Symptoms:**
```
template not found: my_template.rs.tera
```

**Solutions:**
1. Check the template exists in the templates directory
2. Verify the file extension is `.tera`
3. Try reloading templates: `registry.reload()?`

### Problem: Syntax Error

**Symptoms:**
```
template syntax error in domain_entity.rs.tera: ...
```

**Solutions:**
1. Check for unclosed tags: `{% if %}` needs `{% endif %}`
2. Verify variable syntax: `{{ variable_name }}`
3. Test template syntax separately

### Problem: Validation Always Fails

**Symptoms:**
All templates fail validation with type mismatch errors.

**Solutions:**
1. Check you're using the correct insert methods
2. Verify the schema matches the template expectations
3. Use `ctx.inner()` to inspect the actual context
4. Enable debug logging

### Problem: Rate Limit Exceeded

**Symptoms:**
```
rate limit exceeded for filter: my_filter
```

**Solutions:**
1. Increase the rate limit on the filter
2. Reduce filter usage in templates
3. Cache filter results

### Problem: Unused Parameters Warning

**Symptoms:**
```
unused parameters detected: ["field1", "field2"]
```

**Solutions:**
1. Remove unused parameters from context
2. Use parameters in template
3. This is just a warning - template still renders

## Advanced Topics

### Custom Filters with Validation

```rust
use tera::{Filter, Value};
use std::collections::HashMap;

struct UpperCaseFilter;

impl Filter for UpperCaseFilter {
    fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
        if let Some(s) = value.as_str() {
            Ok(Value::String(s.to_uppercase()))
        } else {
            Err(tera::Error::msg("expected a string"))
        }
    }
}

let filter = SafeFilter::new("uppercase", Box::new(UpperCaseFilter))
    .with_rate_limit(1000);

registry.register_filter(filter);
```

### Template Dependency Tracking

```rust
let mut registry = TemplateRegistry::new()?;

// Define dependencies
registry.add_dependency("main.tera", "header.tera");
registry.add_dependency("main.tera", "footer.tera");

// Check for circular dependencies
registry.check_circular_dependencies()?;
```

### Hot Reload in Development

```rust
#[cfg(debug_assertions)]
{
    // Watch for file changes and reload
    registry.reload()?;
}
```

## References

- [Tera Template Engine Documentation](https://tera.netlify.app/)
- [Toyota Production System (TPS)](https://en.wikipedia.org/wiki/Toyota_Production_System)
- [Poka-Yoke Error Proofing](https://en.wikipedia.org/wiki/Poka-yoke)

## Contributing

When adding new templates:

1. Create a template file in `templates/`
2. Define a schema in `src/template/schemas.rs`
3. Register the schema in `TEMPLATE_SCHEMAS`
4. Add tests for the template
5. Update this documentation

## License

This module is part of ggen-mcp and is licensed under Apache-2.0.
