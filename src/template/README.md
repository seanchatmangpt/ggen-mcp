# Template Parameter Validation Module

This module provides comprehensive parameter validation and type-safety for Tera templates in ggen-mcp, implementing poka-yoke (error-proofing) principles from the Toyota Production System.

## Module Structure

```
src/template/
├── mod.rs                     # Module exports
├── parameter_validation.rs    # Core validation logic
├── schemas.rs                 # Template schema definitions
└── README.md                  # This file
```

## Components

### 1. TemplateContext
Type-safe context builder for template parameters.

```rust
let mut ctx = TemplateContext::new("domain_entity.rs.tera");
ctx.insert_string("entity_name", "User")?;
ctx.insert_bool("has_id", true)?;
ctx.validate()?;
```

### 2. ParameterSchema
Define expected parameters for each template.

```rust
let schema = ParameterSchema::new("my_template.rs.tera")
    .parameter(
        ParameterDefinition::new("name", ParameterType::String)
            .required()
            .rule(ValidationRule::NotEmpty)
    );
```

### 3. TemplateValidator
Pre-render validation to catch errors early.

```rust
let validator = TemplateValidator::new("templates")?;
validator.validate_context(&ctx)?;
```

### 4. SafeFilterRegistry
Safe custom filters with input validation and rate limiting.

```rust
let filter = SafeFilter::new("uppercase", Box::new(UpperCaseFilter))
    .with_rate_limit(1000);
registry.register_filter(filter);
```

### 5. TemplateRegistry
Centralized template management with validation.

```rust
let mut registry = TemplateRegistry::new()?;
registry.register_schemas(TEMPLATE_SCHEMAS.clone());
let output = registry.render("template.tera", &ctx)?;
```

## Features

- **Type-safe parameter insertion** - Compile-time type checking
- **Required vs optional tracking** - Catch missing parameters
- **Type validation** - Prevent type mismatches
- **Validation rules** - Min/max length, regex, custom validators
- **Unused parameter detection** - Warn about unused parameters
- **Parameter typo detection** - Catch spelling mistakes
- **Safe filter execution** - Rate limiting and input validation
- **Template dependency tracking** - Detect circular dependencies
- **Hot reload support** - Reload templates in development

## Available Schemas

All templates in the `templates/` directory have predefined schemas:

- `domain_entity.rs.tera` - Domain entity generation
- `mcp_tool_handler.rs.tera` - MCP tool handler generation
- `mcp_resource_handler.rs.tera` - MCP resource handler generation
- `mcp_tool_params.rs.tera` - Tool parameters from SPARQL
- `mcp_tools.rs.tera` - Tools module generation
- `domain_service.rs.tera` - Domain service generation
- `value_object.rs.tera` - Value object generation
- And more...

## Usage Example

```rust
use spreadsheet_mcp::template::{TemplateContext, TemplateRegistry, TEMPLATE_SCHEMAS};

// Create registry with schemas
let mut registry = TemplateRegistry::new()?;
registry.register_schemas(TEMPLATE_SCHEMAS.clone());

// Create and validate context
let mut ctx = TemplateContext::new("domain_entity.rs.tera");
ctx.insert_string("entity_name", "User")?;
ctx.insert_string("description", "User entity")?;
ctx.insert_bool("has_id", true)?;
ctx.insert("fields", serde_json::json!([]))?;
ctx.insert("invariants", serde_json::json!([]))?;

// Render with automatic validation
let output = registry.render("domain_entity.rs.tera", &ctx)?;
```

## Error Prevention

The validation system prevents common errors:

1. **Missing required parameters**
   ```
   Error: missing required parameter: entity_name
   ```

2. **Type mismatches**
   ```
   Error: type mismatch for parameter 'count': expected Number, got String
   ```

3. **Unknown parameters (typos)**
   ```
   Error: unknown parameter: entitiy_name
   ```

4. **Validation rule failures**
   ```
   Error: regex validation failed for entity_name: ^[A-Z][A-Za-z0-9]*$
   ```

5. **Unused parameters**
   ```
   Warning: unused parameters detected: ["debug_mode"]
   ```

## Documentation

See [docs/TEMPLATE_PARAMETER_VALIDATION.md](../../docs/TEMPLATE_PARAMETER_VALIDATION.md) for comprehensive documentation including:

- Detailed API reference
- Schema format specification
- Validation rules guide
- Common error solutions
- Best practices
- Integration guide

## Testing

Run the test suite:

```bash
# All tests
cargo test template

# Specific test file
cargo test --test template_validation_tests

# With output
cargo test template -- --nocapture
```

Example tests are available in:
- `tests/template_validation_tests.rs` - Comprehensive test suite
- `examples/template_validation_example.rs` - Usage examples

## Adding New Templates

1. Create your template in `templates/`
2. Define a schema in `src/template/schemas.rs`
3. Register the schema in `TEMPLATE_SCHEMAS`
4. Add tests for the template

Example:

```rust
fn my_template_schema() -> ParameterSchema {
    ParameterSchema::new("my_template.rs.tera")
        .description("My custom template")
        .parameter(
            ParameterDefinition::new("name", ParameterType::String)
                .required()
                .rule(ValidationRule::NotEmpty)
        )
}

// Add to TEMPLATE_SCHEMAS
pub static TEMPLATE_SCHEMAS: Lazy<Vec<ParameterSchema>> = Lazy::new(|| {
    vec![
        // ... existing schemas
        my_template_schema(),
    ]
});
```

## Performance Considerations

- Parameter validation is fast (microseconds)
- Schemas are lazily initialized
- Template compilation is cached by Tera
- Rate limiting prevents expensive filter abuse
- No runtime overhead if validation is disabled

## Safety Guarantees

This module provides several safety guarantees:

1. **Type Safety** - Parameters are strongly typed
2. **Validation Safety** - All parameters are validated before rendering
3. **Error Safety** - Invalid parameters are caught before template execution
4. **Memory Safety** - Rate limiting prevents unbounded resource usage
5. **Security Safety** - Input sanitization in filters prevents injection

## Integration with Toyota Production System

This module implements several TPS principles:

- **Jidoka (Automation with Human Touch)** - Automatic validation with clear error messages
- **Poka-Yoke (Error Proofing)** - Design prevents errors from occurring
- **Kaizen (Continuous Improvement)** - Easy to add new validation rules
- **Andon (Visual Management)** - Clear visibility of validation status

## License

Apache-2.0
