# Serialization Quick Reference Guide

Quick reference for common serialization patterns in Rust MCP servers.

## Quick Links

- **Full Documentation**: [RUST_MCP_SERIALIZATION.md](./RUST_MCP_SERIALIZATION.md)
- **Code Examples**: [../examples/serialization_patterns.rs](../examples/serialization_patterns.rs)
- **TPS Standards**: [TPS_STANDARDIZED_WORK.md](./TPS_STANDARDIZED_WORK.md)

---

## Common Patterns Cheat Sheet

### 1. Standard Parameter Struct

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MyToolParams {
    // Required fields (no Option, no default)
    pub workbook_id: WorkbookId,
    pub sheet_name: String,

    // Optional fields (with #[serde(default)])
    #[serde(default)]
    pub limit: Option<u32>,

    // Field with alias for backwards compatibility
    #[serde(alias = "old_name")]
    pub new_name: String,
}
```

### 2. Standard Response Struct

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MyToolResponse {
    // Standard identification
    pub workbook_id: WorkbookId,
    pub workbook_short_id: String,

    // Tool-specific data
    pub items: Vec<DataItem>,

    // Pagination metadata
    pub total_count: u32,
    pub has_more: bool,

    // Skip None values
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u32>,
}
```

### 3. NewType Wrapper (Transparent)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct WorkbookId(pub String);

impl WorkbookId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

### 4. Simple Enum

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Active,      // → "active"
    Inactive,    // → "inactive"
    Pending,     // → "pending"
}
```

### 5. Tagged Enum

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value")]
pub enum CellValue {
    Text(String),    // → {"kind": "Text", "value": "hello"}
    Number(f64),     // → {"kind": "Number", "value": 42.0}
}
```

### 6. Custom Validation

```rust
impl MyToolParams {
    pub fn validate(&self) -> Result<()> {
        if self.workbook_id.is_empty() {
            return Err(anyhow!("workbook_id cannot be empty"));
        }
        if let Some(limit) = self.limit {
            if limit > 10_000 {
                return Err(anyhow!("limit too large: {}", limit));
            }
        }
        Ok(())
    }
}
```

---

## Common Serde Attributes

| Attribute | Use Case | Example |
|-----------|----------|---------|
| `#[serde(default)]` | Optional field with default | `#[serde(default)]`<br>`pub field: Option<T>` |
| `#[serde(rename = "name")]` | Custom JSON name | `#[serde(rename = "customName")]`<br>`pub field: String` |
| `#[serde(rename_all = "...")]` | Rename all variants | `#[serde(rename_all = "snake_case")]`<br>`enum MyEnum { ... }` |
| `#[serde(alias = "name")]` | Accept alternative name | `#[serde(alias = "old_name")]`<br>`pub new_name: String` |
| `#[serde(skip)]` | Skip serialization | `#[serde(skip)]`<br>`pub internal: Cache` |
| `#[serde(skip_serializing_if = "...")]` | Conditional skip | `#[serde(skip_serializing_if = "Option::is_none")]`<br>`pub field: Option<T>` |
| `#[serde(transparent)]` | Serialize as inner type | `#[serde(transparent)]`<br>`struct Wrapper(String);` |
| `#[serde(tag = "type")]` | Tagged enum | `#[serde(tag = "kind", content = "value")]`<br>`enum Value { ... }` |
| `#[serde(flatten)]` | Flatten nested struct | `#[serde(flatten)]`<br>`pub base: BaseStruct` |

---

## Validation Checklist

Before implementing a new tool, ensure:

- [ ] Parameter struct has `Debug, Deserialize, JsonSchema`
- [ ] Response struct has `Debug, Clone, Serialize, Deserialize, JsonSchema`
- [ ] Required fields do NOT use `Option<T>`
- [ ] Optional fields have `#[serde(default)]`
- [ ] Backwards compatibility uses `#[serde(alias = "...")]`
- [ ] Validation method implemented on params
- [ ] Response size validation in place
- [ ] Pagination for large result sets
- [ ] Error types convert to `McpError` correctly

---

## Performance Tips

1. **Response Size**: Validate before serialization
   ```rust
   fn ensure_response_size<T: Serialize>(value: &T, limit: usize) -> Result<()> {
       let payload = serde_json::to_vec(value)?;
       if payload.len() > limit {
           return Err(anyhow!("Response too large"));
       }
       Ok(())
   }
   ```

2. **Pagination**: Use for large result sets
   ```rust
   #[derive(Debug, Deserialize)]
   pub struct PagedParams {
       #[serde(default)]
       pub limit: Option<u32>,
       #[serde(default)]
       pub offset: Option<u32>,
   }
   ```

3. **Skip Empty Collections**: Reduce JSON size
   ```rust
   #[serde(skip_serializing_if = "Vec::is_empty")]
   pub notes: Vec<String>,
   ```

---

## Common Pitfalls

### ❌ Missing `#[serde(default)]`

```rust
// WRONG - field is required even though it's Option<T>
pub struct BadParams {
    pub optional: Option<String>,
}
```

```rust
// CORRECT - field is truly optional
pub struct GoodParams {
    #[serde(default)]
    pub optional: Option<String>,
}
```

### ❌ Using Option for Required Fields

```rust
// WRONG - makes required field optional
pub struct BadParams {
    pub workbook_id: Option<String>,
}
```

```rust
// CORRECT - required field without Option
pub struct GoodParams {
    pub workbook_id: String,
}
```

### ❌ Large Responses Without Pagination

```rust
// WRONG - returns all data at once
pub struct BadResponse {
    pub all_items: Vec<Item>,  // Could be millions
}
```

```rust
// CORRECT - paginated response
pub struct GoodResponse {
    pub items: Vec<Item>,
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u32>,
}
```

---

## Schema Generation

Generate JSON schema from any type with `JsonSchema`:

```rust
use schemars::{JsonSchema, schema_for};

#[derive(JsonSchema)]
pub struct MyType {
    /// Field description (appears in schema)
    pub field: String,
}

// Generate schema
let schema = schema_for!(MyType);
let json = serde_json::to_string_pretty(&schema)?;
```

---

## Testing Patterns

### Test Serialization

```rust
#[test]
fn test_serialize_params() {
    let params = MyParams {
        field: "value".to_string(),
    };

    let json = serde_json::to_string(&params).unwrap();
    assert!(json.contains("\"field\""));
}
```

### Test Deserialization

```rust
#[test]
fn test_deserialize_params() {
    let json = r#"{"field": "value"}"#;
    let params: MyParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.field, "value");
}
```

### Test Validation

```rust
#[test]
fn test_validation() {
    let valid = MyParams { field: "value".to_string() };
    assert!(valid.validate().is_ok());

    let invalid = MyParams { field: "".to_string() };
    assert!(invalid.validate().is_err());
}
```

---

## Dependencies

Required in `Cargo.toml`:

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
schemars = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
thiserror = "1.0"
```

---

## Real-World Example from ggen-mcp

From `/home/user/ggen-mcp/src/model.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkbookListResponse {
    pub workbooks: Vec<WorkbookDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkbookDescriptor {
    pub workbook_id: WorkbookId,
    pub short_id: String,
    pub slug: String,
    pub folder: Option<String>,
    pub path: String,
    pub bytes: u64,
    pub last_modified: Option<String>,
    pub caps: BackendCaps,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(transparent)]
pub struct WorkbookId(pub String);
```

---

## See Also

- [RUST_MCP_SERIALIZATION.md](./RUST_MCP_SERIALIZATION.md) - Complete documentation
- [../examples/serialization_patterns.rs](../examples/serialization_patterns.rs) - Runnable examples
- [TPS_STANDARDIZED_WORK.md](./TPS_STANDARDIZED_WORK.md) - Standardized work patterns
- [INPUT_VALIDATION_GUIDE.md](./INPUT_VALIDATION_GUIDE.md) - Validation best practices

---

**Last Updated**: 2026-01-20
