# Validation Module

Runtime JSON schema validation for MCP tool inputs using the `schemars` crate.

## Quick Start

```rust
use spreadsheet_mcp::validation::integration::create_validation_middleware;

// Create pre-configured middleware
let middleware = create_validation_middleware();

// Validate tool parameters
let params = serde_json::json!({
    "workbook_or_fork_id": "my-workbook",
    "sheet_name": "Sheet1"
});

middleware.validate_tool_call("read_table", &params)?;
```

## Modules

- **`schema.rs`** - Core JSON schema validation logic
- **`middleware.rs`** - Middleware integration with rmcp framework
- **`integration.rs`** - Pre-configured validators with tool registration
- **`bounds.rs`** - Numeric boundary validation (Excel limits, cache limits, etc.)
- **`input_guards.rs`** - Input validation guards (poka-yoke principles)

## Key Features

- ✓ Automatic schema generation from `JsonSchema` derives
- ✓ Runtime parameter validation before tool execution
- ✓ Detailed field-level error messages
- ✓ Thread-safe validator with `Arc<T>` sharing
- ✓ Comprehensive constraint validation (types, ranges, patterns, etc.)
- ✓ Seamless rmcp framework integration

## Usage Patterns

### 1. Using Pre-Configured Validator

```rust
use spreadsheet_mcp::validation::integration::create_validation_middleware;

let middleware = create_validation_middleware();
```

### 2. Custom Validator Configuration

```rust
use spreadsheet_mcp::validation::SchemaValidatorBuilder;

let validator = SchemaValidatorBuilder::new()
    .register::<MyParams>("my_tool")
    .build();
```

### 3. Bulk Registration

```rust
use spreadsheet_mcp::register_tool_schemas;

let mut validator = SchemaValidator::new();
register_tool_schemas!(
    validator,
    "list_workbooks" => ListWorkbooksParams,
    "describe_workbook" => DescribeWorkbookParams,
);
```

### 4. Integration with Tool Handlers

```rust
use rmcp::handler::server::wrapper::Parameters;

pub async fn my_tool(
    &self,
    Parameters(params): Parameters<MyToolParams>,
) -> Result<Json<Response>, McpError> {
    // Parameters are already validated
    // Proceed with tool logic
}
```

## Validation Constraints

The validator supports all JSON Schema constraints:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
struct ToolParams {
    // Required field
    required_field: String,

    // Optional field
    #[serde(default)]
    optional_field: Option<String>,

    // Numeric constraints
    #[schemars(range(min = 1, max = 100))]
    count: u32,

    // String constraints
    #[schemars(length(min = 1, max = 255))]
    name: String,

    // Pattern validation
    #[schemars(regex(pattern = r"^[a-zA-Z0-9_]+$"))]
    identifier: String,

    // Array constraints
    #[schemars(length(min = 1, max = 10))]
    items: Vec<String>,
}
```

## Error Handling

```rust
use spreadsheet_mcp::validation::{SchemaValidationError, format_validation_errors};

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
    Err(e) => { /* other error */ }
}
```

## Testing

Run the validation tests:

```bash
cargo test --lib validation
cargo test --test validation_integration_test
```

Run the example:

```bash
cargo run --example validation_example
```

## Documentation

See [docs/validation.md](../../docs/validation.md) for comprehensive documentation.

## Performance

- Schemas generated once at startup and cached
- Thread-safe sharing via `Arc<SchemaValidator>`
- Fast validation with early exit on errors
- Minimal runtime overhead

## Integration with Poka-Yoke

The validation system implements mistake-proofing principles:

1. **Prevention** - Invalid inputs caught before execution
2. **Detection** - Clear error messages help users correct mistakes
3. **Fail-fast** - Validation happens early in request pipeline
4. **Type safety** - Leverages Rust's type system + runtime validation
5. **Documentation** - Schemas serve as machine-readable docs

## Related

- [JSON Schema Specification](https://json-schema.org/)
- [schemars Documentation](https://docs.rs/schemars/)
- [rmcp Framework](https://docs.rs/rmcp/)
- [MCP Protocol](https://modelcontextprotocol.io/)
