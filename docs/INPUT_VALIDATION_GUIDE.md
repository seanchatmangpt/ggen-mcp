# Input Validation Guards - Integration Guide

This document explains how to integrate the comprehensive input validation guards into MCP tool handlers.

## Overview

The `src/validation/input_guards.rs` module provides poka-yoke (mistake-proofing) validation functions that prevent invalid inputs from reaching tool handlers. These guards protect against:

- Empty or whitespace-only strings
- Numeric parameters outside valid ranges
- Path traversal attacks
- Invalid identifiers (sheet names, workbook IDs, cell addresses)

## Validation Functions

### String Validation

#### `validate_non_empty_string(parameter_name, value) -> ValidationResult<&str>`

Ensures a string parameter is not empty or whitespace-only.

```rust
use crate::validation::validate_non_empty_string;

// In your tool handler
let sheet_name = validate_non_empty_string("sheet_name", &params.sheet_name)
    .map_err(|e| anyhow::anyhow!(e))?;
```

### Numeric Range Validation

#### `validate_numeric_range(parameter_name, value, min, max) -> ValidationResult<T>`

Validates that a numeric parameter is within acceptable bounds.

```rust
use crate::validation::validate_numeric_range;

// Validate limit parameter
let limit = validate_numeric_range("limit", params.limit, 1u32, 10000u32)
    .map_err(|e| anyhow::anyhow!(e))?;
```

#### `validate_optional_numeric_range(parameter_name, value, min, max) -> ValidationResult<Option<T>>`

Validates optional numeric parameters.

```rust
use crate::validation::validate_optional_numeric_range;

// Validate optional max_regions parameter
let max_regions = validate_optional_numeric_range("max_regions", params.max_regions, 1u32, 100u32)
    .map_err(|e| anyhow::anyhow!(e))?;
```

### Path Safety Validation

#### `validate_path_safe(path) -> ValidationResult<&str>`

Prevents path traversal attacks by checking for:
- Parent directory references (`..`)
- Absolute paths
- Null bytes
- Suspicious path patterns

```rust
use crate::validation::validate_path_safe;

// Validate file path
let safe_path = validate_path_safe(&params.folder)
    .map_err(|e| anyhow::anyhow!(e))?;
```

### Identifier Validation

#### `validate_sheet_name(name) -> ValidationResult<&str>`

Validates sheet names according to Excel rules:
- Not empty or whitespace-only
- Maximum 31 characters
- No invalid characters: `:`, `\`, `/`, `?`, `*`, `[`, `]`
- Not reserved name "History"

```rust
use crate::validation::validate_sheet_name;

// Validate sheet name
let sheet_name = validate_sheet_name(&params.sheet_name)
    .map_err(|e| anyhow::anyhow!(e))?;
```

#### `validate_workbook_id(id) -> ValidationResult<&str>`

Validates workbook IDs:
- Not empty or whitespace-only
- Maximum 255 characters
- Only safe characters: alphanumeric, `-`, `_`, `.`, `:`

```rust
use crate::validation::validate_workbook_id;

// Validate workbook ID
let workbook_id = validate_workbook_id(params.workbook_or_fork_id.as_str())
    .map_err(|e| anyhow::anyhow!(e))?;
```

#### `validate_cell_address(address) -> ValidationResult<&str>`

Validates cell addresses in A1 notation:
- Column letters (A-XFD)
- Row numbers (1-1048576)

```rust
use crate::validation::validate_cell_address;

// Validate cell address
let address = validate_cell_address(&params.cell_address)
    .map_err(|e| anyhow::anyhow!(e))?;
```

#### `validate_range_string(range) -> ValidationResult<&str>`

Validates range strings:
- Single cells: "A1"
- Cell ranges: "A1:B10"
- Column ranges: "A:A"
- Row ranges: "1:10"

```rust
use crate::validation::validate_range_string;

// Validate range
if let Some(ref range) = params.range {
    validate_range_string(range)
        .map_err(|e| anyhow::anyhow!(e))?;
}
```

## Integration Examples

### Example 1: Validating Tool Parameters

Here's how to add validation to a tool handler in `src/tools/mod.rs`:

```rust
pub async fn sheet_overview(
    state: Arc<AppState>,
    params: SheetOverviewParams,
) -> Result<SheetOverviewResponse> {
    // Validate workbook ID
    validate_workbook_id(params.workbook_or_fork_id.as_str())
        .map_err(|e| anyhow::anyhow!("Invalid workbook_id: {}", e))?;

    // Validate sheet name
    validate_sheet_name(&params.sheet_name)
        .map_err(|e| anyhow::anyhow!("Invalid sheet_name: {}", e))?;

    // Validate optional numeric ranges
    let max_regions = validate_optional_numeric_range(
        "max_regions",
        params.max_regions,
        1u32,
        1000u32
    ).map_err(|e| anyhow::anyhow!(e))?;

    let max_headers = validate_optional_numeric_range(
        "max_headers",
        params.max_headers,
        1u32,
        500u32
    ).map_err(|e| anyhow::anyhow!(e))?;

    // Continue with validated parameters
    let workbook = state.open_workbook(&params.workbook_or_fork_id).await?;
    // ... rest of implementation
}
```

### Example 2: Validating in Generated Tool Handlers

For generated tool handlers in `src/generated/mcp_tools.rs`, you can add validation before calling the tool function:

```rust
#[tool(
    name = "sheet_overview",
    description = "Get narrative overview for a sheet"
)]
pub async fn sheet_overview(
    server: &crate::server::SpreadsheetServer,
    Parameters(params): Parameters<SheetOverviewParams>,
) -> Result<Json<SheetOverviewResponse>, McpError> {
    server
        .ensure_tool_enabled("sheet_overview")
        .map_err(to_mcp_error)?;

    // Add input validation here
    use crate::validation::{validate_workbook_id, validate_sheet_name};

    validate_workbook_id(params.workbook_or_fork_id.as_str())
        .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

    validate_sheet_name(&params.sheet_name)
        .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

    server
        .run_tool_with_timeout(
            "sheet_overview",
            tools::sheet_overview(server.state.clone(), params.into()),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
}
```

### Example 3: Path Validation for File Operations

When dealing with file paths:

```rust
pub async fn save_fork(
    state: Arc<AppState>,
    params: SaveForkParams,
) -> Result<SaveForkResponse> {
    // Validate fork ID
    validate_workbook_id(&params.fork_id)
        .map_err(|e| anyhow::anyhow!("Invalid fork_id: {}", e))?;

    // Validate target path for safety
    validate_path_safe(&params.target_path)
        .map_err(|e| anyhow::anyhow!("Invalid target_path: {}", e))?;

    // Continue with safe path
    // ... implementation
}
```

### Example 4: Batch Validation

For batch operations with multiple parameters:

```rust
pub async fn read_table(
    state: Arc<AppState>,
    params: ReadTableParams,
) -> Result<ReadTableResponse> {
    // Validate all required string parameters
    validate_workbook_id(params.workbook_or_fork_id.as_str())
        .map_err(|e| anyhow::anyhow!("Invalid workbook_id: {}", e))?;

    if let Some(ref sheet_name) = params.sheet_name {
        validate_sheet_name(sheet_name)
            .map_err(|e| anyhow::anyhow!("Invalid sheet_name: {}", e))?;
    }

    if let Some(ref range) = params.range {
        validate_range_string(range)
            .map_err(|e| anyhow::anyhow!("Invalid range: {}", e))?;
    }

    // Validate numeric parameters
    let limit = validate_optional_numeric_range("limit", params.limit, 1u32, 100000u32)
        .map_err(|e| anyhow::anyhow!(e))?;

    let offset = validate_optional_numeric_range("offset", params.offset, 0u32, u32::MAX)
        .map_err(|e| anyhow::anyhow!(e))?;

    // Continue with validated parameters
    // ... implementation
}
```

## Error Handling

All validation functions return `ValidationResult<T>`, which is `Result<T, ValidationError>`.

The `ValidationError` enum provides detailed error messages:

```rust
pub enum ValidationError {
    EmptyString { parameter: String },
    NumericOutOfRange { parameter: String, value: i64, min: i64, max: i64 },
    PathTraversal { path: String },
    InvalidSheetName { name: String, reason: String },
    InvalidWorkbookId { id: String, reason: String },
    InvalidCellAddress { address: String, reason: String },
    InvalidRange { range: String, reason: String },
    Generic { message: String },
}
```

Convert to appropriate error types:

```rust
// Convert to anyhow::Error
validate_sheet_name(&params.sheet_name)
    .map_err(|e| anyhow::anyhow!(e))?;

// Convert to MCP error with custom message
validate_workbook_id(id)
    .map_err(|e| McpError::invalid_params(format!("Validation failed: {}", e), None))?;
```

## Best Practices

1. **Validate Early**: Perform validation as early as possible in the handler, before any I/O operations.

2. **Be Specific**: Use the most specific validation function available (e.g., `validate_sheet_name` instead of `validate_non_empty_string` for sheet names).

3. **Provide Context**: Include parameter names in error messages to help users identify the issue.

4. **Chain Validations**: Validate all parameters before proceeding with business logic.

5. **Use Type Safety**: The validation functions preserve types where possible, reducing the need for unwrapping.

6. **Document Constraints**: Document the validation constraints in tool parameter schemas and descriptions.

## Validation Constants

Common validation ranges used throughout the codebase:

```rust
// Row/column limits (from validation::bounds)
use crate::validation::{EXCEL_MAX_ROWS, EXCEL_MAX_COLUMNS};

// Pagination limits
const MAX_LIMIT: u32 = 10000;
const MAX_OFFSET: u32 = 1000000;

// String lengths
const MAX_SHEET_NAME_LENGTH: usize = 31;
const MAX_WORKBOOK_ID_LENGTH: usize = 255;
```

## Testing

Test your validation integration:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::validate_sheet_name;

    #[test]
    fn test_sheet_overview_validates_sheet_name() {
        let result = validate_sheet_name("Sheet[1]");
        assert!(result.is_err());

        let result = validate_sheet_name("ValidSheet");
        assert!(result.is_ok());
    }
}
```

## Migration Strategy

To integrate validation into existing tools:

1. Identify all input parameters that need validation
2. Add validation calls at the beginning of each tool handler
3. Update tests to cover validation edge cases
4. Update API documentation to reflect validation rules
5. Consider adding validation to the parameter structs using custom deserialization

## Future Enhancements

Potential improvements to the validation system:

- Custom validators using traits
- Validation at deserialization time using serde
- Automatic validation in the MCP parameter parsing layer
- Performance benchmarks for validation overhead
- Integration with the middleware layer for centralized validation
