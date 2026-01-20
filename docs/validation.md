# JSON Schema Validation for MCP Tool Inputs

This document describes the JSON schema validation system implemented for the MCP (Model Context Protocol) server.

## Overview

The validation system provides runtime JSON schema validation for all MCP tool inputs using the `schemars` crate. It ensures that tool parameters are validated against their schemas before execution, providing detailed error messages for invalid inputs.

## Features

- **Automatic Schema Generation**: JSON schemas are automatically generated from Rust structs with `JsonSchema` derive macros
- **Runtime Validation**: Parameters are validated at runtime before tool execution
- **Detailed Error Messages**: Field-level validation errors with clear, actionable messages
- **Thread-Safe**: Schema validator can be shared across threads using `Arc`
- **Middleware Integration**: Seamless integration with rmcp framework through middleware
- **Comprehensive Constraints**: Validates types, ranges, patterns, string lengths, array sizes, and more

## Architecture

The validation system consists of several modules:

### `schema.rs`
Core schema validation logic:
- `SchemaValidator`: Main validator that stores and validates against JSON schemas
- `SchemaValidationError`: Detailed error types for validation failures
- Schema generation from schemars annotations

### `middleware.rs`
Middleware integration with rmcp:
- `ValidationMiddleware`: Wraps the validator for use in tool handlers
- `ValidateParams` trait: Extension trait for validation operations
- Helper functions for formatting validation errors

### `integration.rs`
Integration utilities and tool registration:
- Pre-configured validators with all tool schemas registered
- Feature-gated validators for VBA and recalc tools
- Convenience functions for creating middleware

## Usage

### Basic Usage

```rust
use crate::validation::{SchemaValidator, SchemaValidationMiddleware};
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize, JsonSchema)]
struct MyToolParams {
    required_field: String,
    #[serde(default)]
    optional_field: Option<i32>,
}

// Create and configure validator
let mut validator = SchemaValidator::new();
validator.register_schema::<MyToolParams>("my_tool");

// Validate parameters
let params = serde_json::json!({
    "required_field": "value",
    "optional_field": 42
});

validator.validate("my_tool", &params)?;
```

### Using Pre-Configured Validator

The easiest way to use validation is with the pre-configured validator:

```rust
use crate::validation::integration::create_validation_middleware;

// Create middleware with all tools registered
let middleware = create_validation_middleware();

// In a tool handler:
let params = serde_json::json!({
    "workbook_or_fork_id": "my-workbook"
});

middleware.validate_tool_call("describe_workbook", &params)?;
```

### Integration with Tool Handlers

The validation middleware integrates with rmcp's `Parameters` wrapper:

```rust
use rmcp::handler::server::wrapper::Parameters;
use crate::validation::SchemaValidationMiddleware;

pub async fn my_tool(
    &self,
    Parameters(params): Parameters<MyToolParams>,
) -> Result<Json<Response>, McpError> {
    // Parameters are already validated at this point
    // Proceed with tool logic
    // ...
}
```

### Bulk Schema Registration

Use the `register_tool_schemas!` macro to register multiple schemas at once:

```rust
use crate::register_tool_schemas;
use crate::validation::SchemaValidator;

let mut validator = SchemaValidator::new();
register_tool_schemas!(
    validator,
    "list_workbooks" => ListWorkbooksParams,
    "describe_workbook" => DescribeWorkbookParams,
    "read_table" => ReadTableParams,
);
```

### Builder Pattern

Use `SchemaValidatorBuilder` for fluent schema registration:

```rust
use crate::validation::SchemaValidatorBuilder;

let validator = SchemaValidatorBuilder::new()
    .register::<ListWorkbooksParams>("list_workbooks")
    .register::<DescribeWorkbookParams>("describe_workbook")
    .register::<ReadTableParams>("read_table")
    .build();
```

## Validation Constraints

The validator supports comprehensive JSON Schema constraints:

### Type Validation
```rust
#[derive(Debug, Deserialize, JsonSchema)]
struct Params {
    string_field: String,
    number_field: i32,
    boolean_field: bool,
    array_field: Vec<String>,
}
```

### Required vs Optional Fields
```rust
#[derive(Debug, Deserialize, JsonSchema)]
struct Params {
    required_field: String,
    #[serde(default)]
    optional_field: Option<String>,
}
```

### Numeric Constraints
```rust
#[derive(Debug, Deserialize, JsonSchema)]
struct Params {
    #[schemars(range(min = 1, max = 100))]
    count: u32,

    #[schemars(range(min = 0.0, max = 1.0))]
    percentage: f64,
}
```

### String Constraints
```rust
#[derive(Debug, Deserialize, JsonSchema)]
struct Params {
    #[schemars(length(min = 1, max = 255))]
    name: String,

    #[schemars(regex(pattern = r"^[a-zA-Z0-9_]+$"))]
    identifier: String,
}
```

### Array Constraints
```rust
#[derive(Debug, Deserialize, JsonSchema)]
struct Params {
    #[schemars(length(min = 1, max = 10))]
    items: Vec<String>,
}
```

### Enum Constraints
```rust
#[derive(Debug, Deserialize, JsonSchema)]
enum Mode {
    Preview,
    Apply,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct Params {
    mode: Mode,
}
```

## Error Handling

The validator provides detailed error messages:

```rust
// Missing required field
SchemaValidationError::ValidationFailed {
    tool: "my_tool",
    errors: vec!["Missing required field: required_field"],
}

// Invalid type
SchemaValidationError::ValidationFailed {
    tool: "my_tool",
    errors: vec!["Field 'count': expected integer, got string"],
}

// Out of range
SchemaValidationError::ValidationFailed {
    tool: "my_tool",
    errors: vec!["Field 'count': value 150 exceeds maximum 100"],
}
```

### Error Message Formatting

Use `format_validation_errors` for user-friendly error display:

```rust
use crate::validation::format_validation_errors;

match validator.validate("my_tool", &params) {
    Ok(()) => { /* proceed */ }
    Err(SchemaValidationError::ValidationFailed { errors, .. }) => {
        let message = format_validation_errors("my_tool", &errors);
        eprintln!("{}", message);
        // Outputs:
        // Tool 'my_tool' parameter validation failed:
        //   1. Missing required field: required_field
        //   2. Field 'count': value 150 exceeds maximum 100
    }
}
```

## Performance Considerations

- **Schema Caching**: Schemas are generated once at startup and cached in the validator
- **Thread Safety**: Validator uses `Arc` for efficient sharing across threads
- **Lazy Validation**: Validation only occurs when explicitly called
- **Early Exit**: Validation stops at first critical error for required fields

## Feature Gates

The validation system respects feature gates:

- **Base**: Core spreadsheet tool validation
- **VBA**: VBA tool validation (when `vba` feature is enabled)
- **Recalc**: Fork and recalculation tool validation (when `recalc` feature is enabled)

Use the appropriate validator constructor:

```rust
// Core tools only
let validator = create_configured_validator();

// Core + VBA tools
#[cfg(feature = "vba")]
let validator = create_configured_validator_with_vba();

// Core + Recalc tools
#[cfg(feature = "recalc")]
let validator = create_configured_validator_with_recalc();

// All available tools
let validator = create_full_validator();
```

## Testing

The validation system includes comprehensive tests:

```rust
#[test]
fn test_valid_params() {
    let mut validator = SchemaValidator::new();
    validator.register_schema::<TestParams>("test_tool");

    let params = serde_json::json!({
        "required_field": "value",
        "optional_field": 42
    });

    assert!(validator.validate("test_tool", &params).is_ok());
}

#[test]
fn test_missing_required_field() {
    let mut validator = SchemaValidator::new();
    validator.register_schema::<TestParams>("test_tool");

    let params = serde_json::json!({
        "optional_field": 42
    });

    let result = validator.validate("test_tool", &params);
    assert!(result.is_err());
}
```

## Best Practices

1. **Register schemas at startup**: Create the validator once during server initialization
2. **Use Arc for sharing**: Wrap validator in `Arc` for thread-safe sharing
3. **Validate early**: Validate parameters before any business logic
4. **Provide clear errors**: Use the error formatting utilities for user-friendly messages
5. **Keep schemas in sync**: Update validator registration when adding new tools
6. **Use builder pattern**: Prefer `SchemaValidatorBuilder` for cleaner code
7. **Test validation**: Write tests for both valid and invalid parameter cases

## Integration with Poka-Yoke Principles

The validation system implements poka-yoke (mistake-proofing) principles:

1. **Prevention**: Invalid inputs are caught before execution
2. **Detection**: Clear error messages help users correct mistakes
3. **Fail-fast**: Validation happens early in the request pipeline
4. **Type safety**: Leverages Rust's type system with runtime validation
5. **Documentation**: Schemas serve as machine-readable documentation

## Future Enhancements

Potential improvements for the validation system:

- **OpenAPI Integration**: Generate OpenAPI specs from schemas
- **Custom Validators**: Support for custom validation functions
- **Performance Metrics**: Track validation performance and cache hits
- **Schema Versioning**: Support for multiple schema versions
- **Auto-registration**: Automatically register schemas from tool definitions
- **JSON Schema Draft Support**: Support for newer JSON Schema draft versions
- **Async Validation**: Support for asynchronous validation rules

## Related Modules

- `validation::bounds`: Numeric boundary validation for Excel limits
- `validation::input_guards`: Input validation guards (poka-yoke)
- `tools`: Tool implementation modules
- `server`: MCP server implementation with rmcp framework

## References

- [JSON Schema Specification](https://json-schema.org/)
- [schemars crate documentation](https://docs.rs/schemars/)
- [rmcp framework documentation](https://docs.rs/rmcp/)
- [MCP Protocol Specification](https://modelcontextprotocol.io/)
