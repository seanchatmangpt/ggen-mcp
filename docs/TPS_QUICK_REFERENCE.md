# TPS Standardized Work - Quick Reference

> **One-page reference for MCP server development standards**
>
> For full details, see [TPS_STANDARDIZED_WORK.md](TPS_STANDARDIZED_WORK.md)

---

## Tool Implementation Checklist

```rust
// 1. Define params in src/model.rs
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ToolParams {
    pub workbook_id: WorkbookId,        // Required first
    #[serde(default)]
    pub optional: Option<u32>,          // Optional last
}

// 2. Define response in src/model.rs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolResponse {
    pub workbook_id: WorkbookId,        // Always include context
    pub workbook_short_id: String,      // Always include
    pub data: Vec<Item>,                // Your data
}

// 3. Implement in src/tools/mod.rs
pub async fn tool_name(
    state: Arc<AppState>,
    params: ToolParams,
) -> Result<ToolResponse> {
    // ✓ Validate inputs (fail-fast)
    validate_workbook_id(&params.workbook_id)?;

    // ✓ Acquire resources
    let workbook = state.open_workbook(&params.workbook_id).await?;

    // ✓ Use spawn_blocking for CPU work
    let result = tokio::task::spawn_blocking(move || {
        compute_result(workbook)
    }).await??;

    // ✓ Return response
    Ok(ToolResponse {
        workbook_id: params.workbook_id,
        workbook_short_id: result.short_id,
        data: result.data,
    })
}

// 4. Add MCP handler (src/generated/mcp_tools.rs)
#[tool(name = "tool_name", description = "Clear description")]
pub async fn tool_name_handler(
    server: &Server,
    Parameters(params): Parameters<ToolParams>,
) -> Result<Json<ToolResponse>, McpError> {
    server.ensure_tool_enabled("tool_name").map_err(to_mcp_error)?;
    server.run_tool_with_timeout("tool_name",
        tools::tool_name(server.state.clone(), params.into())
    ).await.map(Json).map_err(to_mcp_error)
}
```

---

## Standard Patterns

### Async vs Blocking Decision

```rust
// ✓ I/O bound → use async/await
let workbook = state.open_workbook(&id).await?;

// ✓ CPU bound → use spawn_blocking
let result = tokio::task::spawn_blocking(move || {
    expensive_computation(data)
}).await??;

// ✗ Never block async runtime
let data = std::fs::read(path)?;  // WRONG!
```

### Validation Pattern

```rust
// ✓ Validate ALL inputs at entry
validate_workbook_id(&params.workbook_id)?;
validate_sheet_name(&params.sheet_name)?;
validate_numeric_range("limit", params.limit, 1, 10_000)?;

// Continue with validated inputs...
```

### Error Handling Pattern

```rust
// ✓ Add context to all errors
load_workbook(id)
    .with_context(|| format!("Failed to load workbook {}", id))?;

// ✓ Use expect() instead of unwrap()
map.get(&key).expect("key validated above")

// ✗ Never use bare unwrap()
map.get(&key).unwrap()  // WRONG!
```

### Pagination Pattern

```rust
// Standard params
#[serde(default)]
pub limit: Option<u32>,
#[serde(default)]
pub offset: Option<u32>,

// Standard response
pub struct Response {
    pub data: Vec<T>,
    pub has_more: bool,
}
```

---

## Derives and Attributes

### Parameter Structs

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct Params {
    pub required: String,

    #[serde(default)]
    pub optional: Option<u32>,

    #[serde(alias = "alt_name")]
    pub field: String,
}
```

### Response Structs

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Response {
    pub always: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub maybe: Option<String>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub items: Vec<Item>,
}
```

### NewType Wrappers

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct WorkbookId(String);
```

---

## Field Ordering Standards

### Parameters (in order)

1. **Identifiers** - `workbook_id`, `fork_id`, `sheet_name`
2. **Core params** - `region_id`, `range`
3. **Filters** - `filters`, `query`
4. **Pagination** - `limit`, `offset`
5. **Output options** - `format`, `include_*`
6. **Flags** - `compact`, `verbose`

### Responses (in order)

1. **Context** - `workbook_id`, `workbook_short_id`
2. **Echo** - `sheet_name`, `region_id`
3. **Metadata** - `row_count`, `has_more`
4. **Primary data** - `data`, `rows`, `items`
5. **Secondary data** - `metadata`, `stats`
6. **Notes** - `notes`, `warnings`

---

## Naming Conventions

| Item | Convention | Example |
|------|-----------|---------|
| Modules | `snake_case` | `input_guards` |
| Types | `PascalCase` | `WorkbookId` |
| Functions | `snake_case` | `validate_input` |
| Constants | `SCREAMING_SNAKE_CASE` | `MAX_ROWS` |
| Validation | `validate_*` | `validate_sheet_name` |
| Booleans | `is_*`, `has_*`, `can_*` | `is_valid` |

---

## Error Messages

### Template

```
{Operation failed}: {specific reason}. {helpful context}
```

### Examples

```rust
// ✓ GOOD - Clear and actionable
"Invalid sheet_name 'Data[1]': contains illegal character '['. \
 Sheet names cannot contain: : \\ / ? * [ ]"

"Range A1:Z100 exceeds limit of 100 rows × 30 columns. \
 Try splitting: [A1:Z50, A51:Z100]"

// ✗ BAD - Vague and unhelpful
"Invalid input"
"Error"
```

---

## Testing Standards

### Test Naming

```rust
#[test]
fn test_{module}_{scenario}_{expected}() { }
```

### Test Structure (Given-When-Then)

```rust
#[test]
fn test_validation_rejects_empty_string() {
    // Given
    let input = "";

    // When
    let result = validate_non_empty_string("field", input);

    // Then
    assert!(result.is_err());
}
```

---

## Code Review Checklist

**Before submitting PR:**

- [ ] `cargo build` - No warnings
- [ ] `cargo test` - All tests pass
- [ ] `cargo fmt --check` - Formatted
- [ ] `cargo clippy -- -D warnings` - No clippy warnings
- [ ] All public items documented
- [ ] No bare `unwrap()` calls
- [ ] Input validation present
- [ ] Error messages descriptive
- [ ] Tests cover edge cases

---

## Common Anti-Patterns

### ❌ Blocking in Async

```rust
// WRONG - blocks async runtime
pub async fn bad() -> Result<Data> {
    let data = std::fs::read(path)?;  // Blocking!
    Ok(data)
}

// CORRECT - use spawn_blocking
pub async fn good(path: PathBuf) -> Result<Data> {
    tokio::task::spawn_blocking(move || {
        std::fs::read(&path)
    }).await??
}
```

### ❌ Bare unwrap()

```rust
// WRONG
let value = map.get(&key).unwrap();

// CORRECT
let value = map.get(&key)
    .expect("key exists after validation");
```

### ❌ Generic Errors

```rust
// WRONG
bail!("Invalid input");

// CORRECT
bail!("Invalid sheet_name '{}': {}", name, reason);
```

---

## Validation Constants

```rust
use crate::validation::bounds::*;

EXCEL_MAX_ROWS           // 1,048,576
EXCEL_MAX_COLUMNS        // 16,384
MAX_PAGINATION_LIMIT     // 10,000
MAX_PAGINATION_OFFSET    // 1,000,000
MAX_SAMPLE_SIZE          // 100,000
```

---

## Performance Guidelines

- Tool response time: < 100ms (p50), < 500ms (p99)
- Use `spawn_blocking` for operations > 10ms CPU time
- Cache expensive computations
- Use streaming for large responses
- Limit response size to < 1MB default

---

## Documentation Requirements

### Function Documentation

```rust
/// One-line summary (imperative mood).
///
/// Longer description explaining what this does and why.
///
/// # Arguments
/// * `param1` - Description
///
/// # Returns
/// Description of return value
///
/// # Errors
/// When this returns an error
///
/// # Examples
/// ```
/// // Usage example
/// ```
pub fn function_name() { }
```

### Module Documentation

```rust
//! Module purpose in one sentence.
//!
//! Longer description of what this module provides
//! and how it fits into the system.
```

---

## Configuration Standards

### Environment Variables

```bash
SPREADSHEET_MCP_WORKSPACE=/path
SPREADSHEET_MCP_CACHE_CAPACITY=10
SPREADSHEET_MCP_VBA_ENABLED=true
```

**Pattern**: `{PROJECT}_{SECTION}_{SETTING}`

---

## Import Order

```rust
// 1. Standard library
use std::sync::Arc;

// 2. External crates (alphabetically)
use anyhow::Result;
use serde::Deserialize;

// 3. Internal crates
use rmcp::tool;

// 4. Current crate (absolute)
use crate::model::*;
use crate::validation::*;

// 5. Relative
use super::utils;
```

---

## File Organization

```
src/
├── main.rs          # Entry point (minimal)
├── lib.rs           # Public API
├── server.rs        # MCP server
├── config.rs        # Configuration
├── state.rs         # App state
├── model.rs         # Data models
├── tools/           # Tool implementations
├── validation/      # Input validation
├── domain/          # Domain logic
└── recovery/        # Error recovery
```

---

## Quick Tips

💡 **Fail-fast**: Validate inputs before any I/O
💡 **Add context**: Every error needs `.with_context()`
💡 **Use NewTypes**: Prevent type confusion bugs
💡 **Document why**: Comments explain rationale, not what
💡 **Test edges**: Happy path, sad path, edge cases
💡 **Review standards**: Check this doc before PR

---

**Full Documentation**: [TPS_STANDARDIZED_WORK.md](TPS_STANDARDIZED_WORK.md)
**Research Findings**: [TPS_RESEARCH_FINDINGS.md](TPS_RESEARCH_FINDINGS.md)
**Last Updated**: 2026-01-20
