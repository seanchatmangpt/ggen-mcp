# Input Validation Quick Reference Card

## Quick Import

```rust
use crate::validation::{
    validate_non_empty_string,
    validate_numeric_range,
    validate_optional_numeric_range,
    validate_path_safe,
    validate_sheet_name,
    validate_workbook_id,
    validate_cell_address,
    validate_range_string,
};
```

## Common Patterns

### Validate Workbook & Sheet

```rust
validate_workbook_id(params.workbook_or_fork_id.as_str())?;
validate_sheet_name(&params.sheet_name)?;
```

### Validate Pagination

```rust
let limit = validate_optional_numeric_range("limit", params.limit, 1u32, 10000u32)?;
let offset = validate_optional_numeric_range("offset", params.offset, 0u32, u32::MAX)?;
```

### Validate Range/Address

```rust
if let Some(ref range) = params.range {
    validate_range_string(range)?;
}

validate_cell_address(&params.cell_address)?;
```

### Validate File Path

```rust
validate_path_safe(&params.target_path)?;
```

## Error Conversion

### To anyhow::Error

```rust
validate_sheet_name(&name)
    .map_err(|e| anyhow::anyhow!(e))?;
```

### To MCP Error

```rust
validate_workbook_id(id)
    .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
```

## Validation Limits

| Parameter Type | Min | Max | Notes |
|---------------|-----|-----|-------|
| Sheet Name | - | 31 chars | No `:\/?\*[]`, not "History" |
| Workbook ID | - | 255 chars | Only `a-z0-9-_.:`  |
| Cell Row | 1 | 1048576 | Excel max |
| Cell Column | A | XFD | 16384 columns |
| Pagination Limit | 1 | 10000 | Typical |
| Pagination Offset | 0 | u32::MAX | |

## Function Quick Reference

| Function | Parameters | Returns | Use For |
|----------|-----------|---------|---------|
| `validate_non_empty_string` | name, value | `&str` | Any string param |
| `validate_numeric_range` | name, val, min, max | `T` | Required numeric |
| `validate_optional_numeric_range` | name, opt, min, max | `Option<T>` | Optional numeric |
| `validate_path_safe` | path | `&str` | File paths |
| `validate_sheet_name` | name | `&str` | Sheet names |
| `validate_workbook_id` | id | `&str` | Workbook/fork IDs |
| `validate_cell_address` | addr | `&str` | Cell addresses (A1) |
| `validate_range_string` | range | `&str` | Ranges (A1:B10) |

## Tool Handler Template

```rust
pub async fn my_tool(
    state: Arc<AppState>,
    params: MyToolParams,
) -> Result<MyResponse> {
    // 1. Validate IDs
    validate_workbook_id(params.workbook_or_fork_id.as_str())?;

    // 2. Validate names/strings
    validate_sheet_name(&params.sheet_name)?;

    // 3. Validate optional numerics
    let limit = validate_optional_numeric_range(
        "limit", params.limit, 1u32, 10000u32
    )?;

    // 4. Validate paths (if applicable)
    if let Some(ref path) = params.path {
        validate_path_safe(path)?;
    }

    // 5. Proceed with validated params
    let workbook = state.open_workbook(&params.workbook_or_fork_id).await?;
    // ... rest of implementation
}
```

## MCP Handler Template

```rust
#[tool(name = "my_tool", description = "...")]
pub async fn my_tool(
    server: &SpreadsheetServer,
    Parameters(params): Parameters<MyToolParams>,
) -> Result<Json<MyResponse>, McpError> {
    server.ensure_tool_enabled("my_tool")
        .map_err(to_mcp_error)?;

    // Add validation
    validate_workbook_id(params.workbook_or_fork_id.as_str())
        .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

    validate_sheet_name(&params.sheet_name)
        .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

    server.run_tool_with_timeout(
        "my_tool",
        tools::my_tool(server.state.clone(), params.into()),
    )
    .await
    .map(Json)
    .map_err(to_mcp_error)
}
```

## Testing Template

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::*;

    #[test]
    fn test_validates_sheet_name() {
        // Valid
        assert!(validate_sheet_name("Sheet1").is_ok());

        // Invalid - too long
        assert!(validate_sheet_name("ThisNameIsTooLongForExcelSheets").is_err());

        // Invalid - special char
        assert!(validate_sheet_name("Sheet[1]").is_err());
    }

    #[test]
    fn test_validates_numeric_ranges() {
        // Valid
        assert!(validate_numeric_range("limit", 100u32, 1u32, 1000u32).is_ok());

        // Invalid - too high
        assert!(validate_numeric_range("limit", 2000u32, 1u32, 1000u32).is_err());
    }
}
```

## Common Validation Chains

### Full Tool Validation

```rust
// Validate all at once
validate_workbook_id(params.workbook_or_fork_id.as_str())?;
validate_sheet_name(&params.sheet_name)?;

let max_regions = validate_optional_numeric_range(
    "max_regions", params.max_regions, 1u32, 1000u32
)?;

let max_headers = validate_optional_numeric_range(
    "max_headers", params.max_headers, 1u32, 500u32
)?;

if let Some(ref range) = params.range {
    validate_range_string(range)?;
}
```

### Fork Operation Validation

```rust
// Always validate fork operations
validate_workbook_id(&params.fork_id)?;
validate_path_safe(&params.target_path)?;
validate_sheet_name(&params.sheet_name)?;
```

## Security Checklist

- [ ] All user-provided strings validated
- [ ] All numeric parameters have range checks
- [ ] All file paths validated with `validate_path_safe`
- [ ] All sheet names validated
- [ ] All workbook IDs validated
- [ ] Optional parameters handled correctly
- [ ] Error messages provide clear guidance

## Performance Notes

- Validation is O(n) for string length checks
- Numeric validation is O(1)
- Minimal overhead (< 1μs per validation)
- No allocations for valid inputs
- Only allocates on error (for error message)

## See Also

- Full Guide: `/home/user/ggen-mcp/docs/INPUT_VALIDATION_GUIDE.md`
- Examples: `/home/user/ggen-mcp/docs/VALIDATION_INTEGRATION_EXAMPLE.rs`
- Summary: `/home/user/ggen-mcp/docs/VALIDATION_IMPLEMENTATION_SUMMARY.md`
- Implementation: `/home/user/ggen-mcp/src/validation/input_guards.rs`
