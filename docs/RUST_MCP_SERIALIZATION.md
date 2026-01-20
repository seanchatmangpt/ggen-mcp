# Rust MCP Server Serialization Best Practices

## Executive Summary

This document provides comprehensive guidance on serialization/deserialization patterns for Model Context Protocol (MCP) servers written in Rust, with specific analysis of the ggen-mcp (spreadsheet-mcp) codebase.

**Document Purpose**: Establish standardized serialization patterns to ensure type safety, performance, and protocol compliance in Rust MCP servers.

**Last Updated**: 2026-01-20
**Codebase Analyzed**: ggen-mcp (spreadsheet-mcp)
**Analysis Scope**: 15,000+ lines of production code with comprehensive serialization patterns

---

## Table of Contents

1. [Serde Best Practices](#1-serde-best-practices)
2. [MCP Protocol Serialization](#2-mcp-protocol-serialization)
3. [Performance Optimization](#3-performance-optimization)
4. [Validation](#4-validation)
5. [Format Selection](#5-format-selection)
6. [Schema Management](#6-schema-management)
7. [Error Handling](#7-error-handling)
8. [TPS Standardized Work](#8-tps-standardized-work)

---

## 1. Serde Best Practices

### 1.1 Derive Macros Usage

**Standard Pattern**: Every serializable type MUST derive the standard trio of traits:

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolResponse {
    pub workbook_id: WorkbookId,
    pub data: Vec<DataRow>,
}
```

**Trait Requirements**:
- `Serialize`: Required for all response types
- `Deserialize`: Required for all parameter types
- `JsonSchema`: Required for automatic JSON schema generation (MCP protocol requirement)
- `Debug`: Required for logging and debugging
- `Clone`: Required for Arc-based state sharing (optional but recommended)

**Observed Pattern from ggen-mcp**:
```rust
// From src/model.rs
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
```

### 1.2 Field Attributes

#### 1.2.1 Rename Fields

**Use Case**: Convert between Rust naming (snake_case) and JSON naming conventions.

```rust
// Enum with rename_all
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SheetClassification {
    Data,           // Serializes as "data"
    Calculator,     // Serializes as "calculator"
    Mixed,          // Serializes as "mixed"
    Metadata,       // Serializes as "metadata"
    Empty,          // Serializes as "empty"
}

// Individual field rename
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum RegionKind {
    #[serde(rename = "likely_table")]
    Table,
    #[serde(rename = "likely_data")]
    Data,
    #[serde(rename = "likely_parameters")]
    Parameters,
}
```

**Standard from ggen-mcp**: Use `rename_all = "snake_case"` for enums to maintain consistent JSON output.

#### 1.2.2 Optional Fields with Default

**Pattern**: Optional parameters MUST use `#[serde(default)]`:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SheetOverviewParams {
    // Required field - no attributes needed
    pub workbook_or_fork_id: WorkbookId,
    pub sheet_name: String,

    // Optional fields - MUST have #[serde(default)]
    #[serde(default)]
    pub max_regions: Option<u32>,
    #[serde(default)]
    pub max_headers: Option<u32>,
    #[serde(default)]
    pub include_headers: Option<bool>,
}
```

**Why**: Without `#[serde(default)]`, the field is considered required even though it's `Option<T>`.

#### 1.2.3 Field Aliases

**Use Case**: Support multiple parameter names for backwards compatibility:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DescribeWorkbookParams {
    /// Accepts both "workbook_or_fork_id" and "workbook_id"
    #[serde(alias = "workbook_id")]
    pub workbook_or_fork_id: WorkbookId,
}
```

**Standard**: Use aliases when renaming parameters to maintain backwards compatibility.

#### 1.2.4 Skip Serialization

**Use Case**: Internal fields that should not appear in JSON:

```rust
#[derive(Debug, Serialize)]
pub struct InternalState {
    pub visible_field: String,

    #[serde(skip)]
    pub internal_cache: HashMap<String, Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_metadata: Option<String>,
}
```

**Standard**: Use `skip_serializing_if = "Option::is_none"` to omit null fields from JSON output.

### 1.3 Transparent Wrappers (NewType Pattern)

**Pattern**: Use transparent serialization for single-field wrapper types:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(transparent)]
pub struct WorkbookId(pub String);

impl WorkbookId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorkbookId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

**Benefits**:
- Type safety: Cannot accidentally mix up WorkbookId with String
- Zero overhead: Serializes/deserializes as plain string
- Method attachment: Can add validation and helper methods

**JSON Representation**:
```json
// Without transparent: {"WorkbookId": "my-workbook"}
// With transparent: "my-workbook"
```

### 1.4 Tagged Enums

#### 1.4.1 Externally Tagged (Default)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum SimpleEnum {
    VariantA,
    VariantB(String),
    VariantC { field: i32 },
}

// JSON: {"VariantA": null}
// JSON: {"VariantB": "value"}
// JSON: {"VariantC": {"field": 42}}
```

#### 1.4.2 Internally Tagged

**Use Case**: Better JSON structure with explicit type field:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value")]
pub enum CellValue {
    Text(String),
    Number(f64),
    Bool(bool),
    Error(String),
    Date(String),
}

// JSON: {"kind": "Text", "value": "hello"}
// JSON: {"kind": "Number", "value": 42.0}
// JSON: {"kind": "Bool", "value": true}
```

**Observed Pattern**: ggen-mcp uses this for `CellValue` to provide clear type discrimination.

#### 1.4.3 Adjacently Tagged

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FillDescriptor {
    Pattern(PatternFillDescriptor),
    Gradient(GradientFillDescriptor),
}

// JSON: {"kind": "pattern", ...PatternFillDescriptor fields}
// JSON: {"kind": "gradient", ...GradientFillDescriptor fields}
```

**Use Case**: Flatten variant data into parent object while maintaining type tag.

#### 1.4.4 Untagged Enums

**Use Case**: Discriminate based on structure alone (use sparingly):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FlexibleValue {
    String(String),
    Number(f64),
    Bool(bool),
}
```

**Warning**: Untagged enums can cause ambiguous deserialization. Prefer tagged variants.

### 1.5 Flattening

**Use Case**: Merge nested struct fields into parent:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct BaseParams {
    pub workbook_id: String,
    pub sheet_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtendedParams {
    #[serde(flatten)]
    pub base: BaseParams,
    pub additional_field: String,
}

// JSON: {
//   "workbook_id": "...",
//   "sheet_name": "...",
//   "additional_field": "..."
// }
```

**Warning**: Flattening can make schema generation difficult. Use sparingly in MCP contexts.

---

## 2. MCP Protocol Serialization

### 2.1 Request/Response Serialization

**Standard Pattern**: MCP tools accept parameters and return JSON-wrapped responses:

```rust
use rmcp::{ErrorData as McpError, Json, handler::server::wrapper::Parameters};

#[tool(
    name = "list_workbooks",
    description = "List spreadsheet files in the workspace"
)]
pub async fn list_workbooks(
    &self,
    Parameters(params): Parameters<ListWorkbooksParams>,
) -> Result<Json<WorkbookListResponse>, McpError> {
    self.ensure_tool_enabled("list_workbooks")
        .map_err(to_mcp_error)?;

    self.run_tool_with_timeout(
        "list_workbooks",
        tools::list_workbooks(self.state.clone(), params),
    )
    .await
    .map(Json)
    .map_err(to_mcp_error)
}
```

**Key Points**:
1. Parameters wrapped in `Parameters<T>` extractor
2. Response wrapped in `Json<T>` for automatic serialization
3. Errors converted to `McpError` for protocol compliance

### 2.2 Tool Parameter Handling

**Standard Structure**:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ToolParams {
    // 1. Required fields first
    pub workbook_id: WorkbookId,
    pub sheet_name: String,

    // 2. Optional fields with #[serde(default)]
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
}

// Conversion to internal type if needed
impl From<ToolParams> for InternalToolParams {
    fn from(p: ToolParams) -> Self {
        Self {
            workbook_id: p.workbook_id,
            sheet_name: p.sheet_name,
            limit: p.limit,
            offset: p.offset,
        }
    }
}
```

**Pattern from ggen-mcp**: Generated parameter structs convert to internal types:

```rust
// From src/generated/mcp_tool_params.rs
impl From<ListWorkbooksParams> for crate::tools::ListWorkbooksParams {
    fn from(p: ListWorkbooksParams) -> Self {
        Self {
            slug_prefix: p.slug_prefix,
            folder: p.folder,
            path_glob: p.path_glob,
        }
    }
}
```

### 2.3 Error Serialization

**Pattern**: Convert domain errors to MCP protocol errors:

```rust
use rmcp::ErrorData as McpError;

fn to_mcp_error(error: anyhow::Error) -> McpError {
    if error.downcast_ref::<ToolDisabledError>().is_some() {
        McpError::invalid_request(error.to_string(), None)
    } else if error.downcast_ref::<ResponseTooLargeError>().is_some() {
        McpError::invalid_request(error.to_string(), None)
    } else {
        McpError::internal_error(error.to_string(), None)
    }
}
```

**Standard**: Use specific error types and convert to appropriate MCP error codes:
- `invalid_request`: Client errors (bad parameters, disabled tools)
- `internal_error`: Server errors (I/O failures, unexpected conditions)
- `method_not_found`: Unknown tool names

### 2.4 Schema Generation

**Pattern**: Use `schemars` for automatic JSON schema generation:

```rust
use schemars::{JsonSchema, schema_for};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MyParams {
    pub required_field: String,
    #[serde(default)]
    pub optional_field: Option<i32>,
}

// Generate schema
let schema = schema_for!(MyParams);
```

**Observed Pattern**: ggen-mcp uses `SchemaValidator` for runtime validation:

```rust
pub struct SchemaValidator {
    schemas: HashMap<String, serde_json::Value>,
}

impl SchemaValidator {
    pub fn register_schema<T: JsonSchema>(&mut self, tool_name: &str) {
        let schema = schema_for!(T);
        let schema_json = serde_json::to_value(schema)
            .expect("Failed to serialize schema");
        self.schemas.insert(tool_name.to_string(), schema_json);
    }

    pub fn validate(&self, tool_name: &str, params: &Value) -> Result<()> {
        // Validate params against schema
    }
}
```

### 2.5 Backwards Compatibility

**Strategies**:

1. **Field Aliases**: Support old parameter names
```rust
#[serde(alias = "old_name")]
pub new_name: String,
```

2. **Optional Fields**: Make new fields optional with defaults
```rust
#[serde(default)]
pub new_field: Option<String>,
```

3. **Versioned Types**: Create new types for breaking changes
```rust
pub struct ParamsV1 { /* ... */ }
pub struct ParamsV2 { /* ... */ }
```

4. **Skip Unknown Fields**: Allow extra fields in requests
```rust
// Default serde behavior - unknown fields cause errors
// Use custom deserializer to ignore unknown fields if needed
```

---

## 3. Performance Optimization

### 3.1 Zero-Copy Deserialization

**Pattern**: Use borrowed data when possible:

```rust
#[derive(Debug, Deserialize)]
pub struct BorrowedParams<'a> {
    pub name: &'a str,        // Borrows from input
    pub data: Vec<&'a str>,   // Vector of borrowed strings
}
```

**Limitations**:
- Requires lifetime management
- Not compatible with owned data structures
- MCP protocol typically requires owned data

**Recommendation**: Use for internal parsing, not MCP parameter types.

### 3.2 Avoiding Allocations

**String Interning**: For repeated strings, use string interning:

```rust
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct InternedResponse {
    // Share string data across instances
    pub sheet_name: Arc<str>,
    pub workbook_path: Arc<str>,
}
```

**Static Strings**: Use `&'static str` for constant values:

```rust
const ERROR_MESSAGE: &str = "Invalid workbook ID";

pub fn validate(id: &str) -> Result<(), &'static str> {
    if id.is_empty() {
        Err(ERROR_MESSAGE)
    } else {
        Ok(())
    }
}
```

### 3.3 Streaming Serialization

**Use Case**: Serialize large responses incrementally:

```rust
use serde_json::Serializer;
use std::io::Write;

pub fn stream_large_response<W: Write>(
    writer: W,
    data: impl Iterator<Item = Row>,
) -> Result<()> {
    let mut ser = Serializer::new(writer);

    // Write array start
    writer.write_all(b"[")?;

    // Stream items
    for (i, row) in data.enumerate() {
        if i > 0 {
            writer.write_all(b",")?;
        }
        row.serialize(&mut ser)?;
    }

    // Write array end
    writer.write_all(b"]")?;

    Ok(())
}
```

**Limitation**: MCP protocol expects complete JSON objects. Use pagination instead.

### 3.4 Benchmark Comparisons

**Pattern**: Use criterion for serialization benchmarks:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_serialize(c: &mut Criterion) {
    let data = create_test_response();

    c.bench_function("serialize_response", |b| {
        b.iter(|| {
            serde_json::to_vec(black_box(&data))
        })
    });
}

criterion_group!(benches, bench_serialize);
criterion_main!(benches);
```

**Observed Pattern**: ggen-mcp validates response size to prevent performance issues:

```rust
fn ensure_response_size<T: Serialize>(&self, tool: &str, value: &T) -> Result<()> {
    let Some(limit) = self.state.config().max_response_bytes() else {
        return Ok(());
    };

    let payload = serde_json::to_vec(value)
        .map_err(|e| anyhow!("failed to serialize response for {}: {}", tool, e))?;

    if payload.len() > limit {
        return Err(ResponseTooLargeError::new(tool, payload.len(), limit).into());
    }

    Ok(())
}
```

---

## 4. Validation

### 4.1 Validation During Deserialization

**Pattern**: Implement custom `Deserialize` for validation:

```rust
use serde::de::{self, Deserialize, Deserializer};

#[derive(Debug)]
pub struct ValidatedEmail(String);

impl<'de> Deserialize<'de> for ValidatedEmail {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        if s.contains('@') {
            Ok(ValidatedEmail(s))
        } else {
            Err(de::Error::custom("Invalid email format"))
        }
    }
}
```

**Alternative**: Validate after deserialization (cleaner separation):

```rust
#[derive(Debug, Deserialize)]
pub struct Params {
    pub email: String,
}

impl Params {
    pub fn validate(&self) -> Result<()> {
        if !self.email.contains('@') {
            return Err(anyhow!("Invalid email format"));
        }
        Ok(())
    }
}
```

### 4.2 Custom Validators

**Pattern**: Type-level validation with newtype wrappers:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct ValidatedRange {
    value: String,
}

impl ValidatedRange {
    pub fn new(s: String) -> Result<Self> {
        // Validate A1 notation
        if !is_valid_range(&s) {
            return Err(anyhow!("Invalid range format: {}", s));
        }
        Ok(Self { value: s })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}
```

**Observed Pattern**: ggen-mcp uses validation module:

```rust
// From src/validation/input_guards.rs
pub fn validate_range_string(range: &str) -> ValidationResult<()> {
    if range.is_empty() {
        return Err(ValidationError::EmptyString {
            field: "range".to_string(),
        });
    }

    // Validate A1 notation format
    // ...

    Ok(())
}
```

### 4.3 Schema Validation Integration

**Pattern**: Runtime JSON schema validation:

```rust
use schemars::JsonSchema;
use serde_json::Value;

pub struct SchemaValidator {
    schemas: HashMap<String, Value>,
}

impl SchemaValidator {
    pub fn validate(&self, tool_name: &str, params: &Value) -> Result<()> {
        let schema = self.schemas.get(tool_name)
            .ok_or_else(|| anyhow!("Schema not registered"))?;

        self.validate_against_schema(tool_name, params, schema)
    }

    fn validate_against_schema(
        &self,
        tool_name: &str,
        params: &Value,
        schema: &Value,
    ) -> Result<()> {
        // Validate required fields
        // Validate types
        // Validate constraints
        // Return detailed errors
    }
}
```

### 4.4 Type-Safe Guarantees

**Pattern**: Use Rust's type system for compile-time guarantees:

```rust
// Type-safe builder pattern
pub struct QueryBuilder {
    workbook_id: WorkbookId,  // Required at construction
    sheet_name: Option<String>,
    limit: Option<u32>,
}

impl QueryBuilder {
    // Must provide required fields
    pub fn new(workbook_id: WorkbookId) -> Self {
        Self {
            workbook_id,
            sheet_name: None,
            limit: None,
        }
    }

    pub fn sheet_name(mut self, name: String) -> Self {
        self.sheet_name = Some(name);
        self
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }
}
```

### 4.5 Error Reporting

**Pattern**: Detailed validation error messages:

```rust
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Missing required field '{field}' in {context}")]
    MissingRequiredField {
        field: String,
        context: String,
    },

    #[error("Invalid type for field '{field}': expected {expected}, got {actual}")]
    InvalidType {
        field: String,
        expected: String,
        actual: String,
    },

    #[error("Invalid value for field '{field}': {reason}")]
    InvalidValue {
        field: String,
        reason: String,
    },

    #[error("Field '{field}' out of range: {value} not in [{min}, {max}]")]
    OutOfRange {
        field: String,
        value: String,
        min: String,
        max: String,
    },
}
```

---

## 5. Format Selection

### 5.1 JSON for MCP Protocol

**Standard**: MCP protocol requires JSON for all communication:

```rust
use serde_json::{json, Value};

// Serialize to JSON
let response = WorkbookListResponse { /* ... */ };
let json = serde_json::to_string(&response)?;

// Deserialize from JSON
let params: ListWorkbooksParams = serde_json::from_str(&json)?;

// Work with dynamic JSON
let value = json!({
    "workbook_id": "my-workbook",
    "sheet_name": "Sheet1"
});
```

**Dependencies**:
```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### 5.2 MessagePack for Internal Use

**Use Case**: Faster serialization for internal caching or IPC:

```toml
[dependencies]
rmp-serde = "1.1"
```

```rust
use rmp_serde::{Deserializer, Serializer};

// Serialize to MessagePack
let mut buf = Vec::new();
response.serialize(&mut Serializer::new(&mut buf))?;

// Deserialize from MessagePack
let response: Response = rmp_serde::from_slice(&buf)?;
```

**Benefits**:
- Faster than JSON
- Smaller payload size
- Binary format

**Drawbacks**:
- Not human-readable
- Not compatible with MCP protocol
- Requires separate serialization path

### 5.3 CBOR Alternatives

**Use Case**: Similar to MessagePack, standards-based binary format:

```toml
[dependencies]
serde_cbor = "0.11"
```

```rust
use serde_cbor;

let bytes = serde_cbor::to_vec(&response)?;
let response: Response = serde_cbor::from_slice(&bytes)?;
```

### 5.4 Format Benchmarking

**Comparison** (typical results for medium-sized response):

| Format | Serialize | Deserialize | Size | Human Readable |
|--------|-----------|-------------|------|----------------|
| JSON | 100% (baseline) | 100% (baseline) | 100% (baseline) | Yes |
| MessagePack | 60% | 50% | 40% | No |
| CBOR | 65% | 55% | 45% | No |
| Bincode | 40% | 30% | 35% | No |

**Recommendation**: Use JSON for MCP protocol, consider binary formats only for internal caching.

### 5.5 Compression Strategies

**Use Case**: Reduce network payload for large responses:

```rust
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;

fn compress_json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let json = serde_json::to_vec(value)?;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&json)?;

    Ok(encoder.finish()?)
}
```

**Warning**: MCP protocol does not specify compression. Use pagination instead:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PagedParams {
    pub workbook_id: WorkbookId,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct PagedResponse {
    pub items: Vec<Item>,
    pub total_count: u32,
    pub has_more: bool,
    pub next_offset: Option<u32>,
}
```

---

## 6. Schema Management

### 6.1 JSON Schema Generation

**Pattern**: Use `schemars` for automatic schema generation:

```rust
use schemars::{JsonSchema, schema_for};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MyParams {
    /// The workbook identifier
    #[schemars(description = "Unique identifier for the workbook")]
    pub workbook_id: String,

    /// Optional limit parameter
    #[schemars(description = "Maximum number of items to return")]
    #[serde(default)]
    pub limit: Option<u32>,
}

// Generate schema
let schema = schema_for!(MyParams);
println!("{}", serde_json::to_string_pretty(&schema)?);
```

**Output**:
```json
{
  "type": "object",
  "properties": {
    "workbook_id": {
      "type": "string",
      "description": "Unique identifier for the workbook"
    },
    "limit": {
      "type": "integer",
      "format": "uint32",
      "description": "Maximum number of items to return"
    }
  },
  "required": ["workbook_id"]
}
```

### 6.2 Schemars Integration

**Pattern**: Customize schema generation:

```rust
use schemars::{JsonSchema, gen::SchemaGenerator, schema::Schema};

#[derive(Debug, Deserialize)]
pub struct CustomType(String);

impl JsonSchema for CustomType {
    fn schema_name() -> String {
        "CustomType".to_string()
    }

    fn json_schema(gen: &mut SchemaGenerator) -> Schema {
        // Return custom schema
        schemars::schema::Schema::Object(
            schemars::schema::SchemaObject {
                instance_type: Some(schemars::schema::SingleOrVec::Single(
                    Box::new(schemars::schema::InstanceType::String)
                )),
                metadata: Some(Box::new(schemars::schema::Metadata {
                    description: Some("Custom validated type".to_string()),
                    ..Default::default()
                })),
                ..Default::default()
            }
        )
    }
}
```

### 6.3 Schema Versioning

**Strategy 1: Separate Types**
```rust
pub mod v1 {
    #[derive(Debug, Deserialize, JsonSchema)]
    pub struct Params {
        pub field_a: String,
    }
}

pub mod v2 {
    #[derive(Debug, Deserialize, JsonSchema)]
    pub struct Params {
        pub field_a: String,
        #[serde(default)]
        pub field_b: Option<String>,
    }
}
```

**Strategy 2: Feature Flags**
```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct Params {
    pub field_a: String,

    #[cfg(feature = "v2")]
    #[serde(default)]
    pub field_b: Option<String>,
}
```

### 6.4 Migration Strategies

**Pattern**: Version parameter conversion:

```rust
impl From<v1::Params> for v2::Params {
    fn from(v1: v1::Params) -> Self {
        Self {
            field_a: v1.field_a,
            field_b: None,  // Default for new field
        }
    }
}
```

### 6.5 Documentation Generation

**Pattern**: Use doc comments for schema descriptions:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct WellDocumentedParams {
    /// The unique identifier for the workbook
    ///
    /// This can be obtained from the `list_workbooks` tool.
    /// Format: alphanumeric slug with hyphens
    pub workbook_id: String,

    /// The name of the sheet to query
    ///
    /// Sheet names are case-sensitive and may contain spaces.
    pub sheet_name: String,

    /// Optional limit on the number of rows to return
    ///
    /// If not specified, returns all rows. Maximum value is 10,000.
    #[serde(default)]
    pub limit: Option<u32>,
}
```

---

## 7. Error Handling

### 7.1 Deserialization Errors

**Pattern**: Handle serde errors gracefully:

```rust
use serde_json::Error as JsonError;

fn parse_params(json: &str) -> Result<Params> {
    serde_json::from_str(json)
        .map_err(|e| {
            match e.classify() {
                serde_json::error::Category::Io => {
                    anyhow!("I/O error reading JSON: {}", e)
                }
                serde_json::error::Category::Syntax => {
                    anyhow!("Invalid JSON syntax at line {}: {}", e.line(), e)
                }
                serde_json::error::Category::Data => {
                    anyhow!("Invalid data structure: {}", e)
                }
                serde_json::error::Category::Eof => {
                    anyhow!("Unexpected end of JSON: {}", e)
                }
            }
        })
}
```

### 7.2 Partial Deserialization

**Use Case**: Continue processing even if some fields fail:

```rust
#[derive(Debug, Deserialize)]
pub struct RobustResponse {
    pub required_field: String,

    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_lossy")]
    pub optional_field: Option<ComplexType>,
}

fn deserialize_optional_lossy<'de, D, T>(
    deserializer: D,
) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    match T::deserialize(deserializer) {
        Ok(val) => Ok(Some(val)),
        Err(_) => Ok(None),  // Silently ignore errors
    }
}
```

**Warning**: Use carefully - silent failures can hide bugs.

### 7.3 Error Recovery

**Pattern**: Provide fallback values:

```rust
#[derive(Debug, Deserialize)]
pub struct ConfigWithDefaults {
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,

    #[serde(default = "default_retries")]
    pub max_retries: u32,
}

fn default_timeout() -> u64 {
    30_000
}

fn default_retries() -> u32 {
    3
}
```

### 7.4 Validation Errors

**Pattern**: Detailed validation error types:

```rust
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Schema validation failed for tool '{tool}': {errors:?}")]
    SchemaValidationFailed {
        tool: String,
        errors: Vec<String>,
    },

    #[error("Missing required field '{field}' in tool '{tool}'")]
    MissingRequiredField {
        tool: String,
        field: String,
    },

    #[error("Invalid type for field '{field}' in tool '{tool}': expected {expected}, got {actual}")]
    InvalidType {
        tool: String,
        field: String,
        expected: String,
        actual: String,
    },
}
```

**Observed Pattern**: ggen-mcp's validation errors:

```rust
pub enum SchemaValidationError {
    #[error("Schema validation failed for tool '{tool}': {errors}")]
    ValidationFailed {
        tool: String,
        errors: Vec<String>,
    },
    // ... more variants
}
```

### 7.5 Custom Error Types

**Pattern**: Domain-specific error types:

```rust
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Workbook not found: {0}")]
    WorkbookNotFound(String),

    #[error("Sheet '{sheet}' not found in workbook '{workbook}'")]
    SheetNotFound {
        workbook: String,
        sheet: String,
    },

    #[error("Invalid range '{range}': {reason}")]
    InvalidRange {
        range: String,
        reason: String,
    },

    #[error("Response too large: {size} bytes (limit: {limit})")]
    ResponseTooLarge {
        size: usize,
        limit: usize,
    },
}
```

---

## 8. TPS Standardized Work

### 8.1 Serialization Patterns (Standardized Work)

This section establishes **standardized work** for serialization patterns in MCP servers, following Toyota Production System principles.

#### 8.1.1 Parameter Struct Pattern

**Standard Work Sequence**:

1. Define parameter struct with proper derives
2. Add required fields (no `Option`)
3. Add optional fields with `#[serde(default)]`
4. Add field aliases for backwards compatibility
5. Implement `From` conversion if needed
6. Add validation method

```rust
// STANDARD PATTERN - Follow exactly for all tool parameters
#[derive(Debug, Deserialize, JsonSchema)]
pub struct StandardToolParams {
    // Step 2: Required fields (no Option, no default)
    pub workbook_id: WorkbookId,
    pub sheet_name: String,

    // Step 3: Optional fields with #[serde(default)]
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,

    // Step 4: Field with alias for backwards compatibility
    #[serde(alias = "range_string")]
    #[serde(default)]
    pub range: Option<String>,
}

// Step 6: Validation method
impl StandardToolParams {
    pub fn validate(&self) -> Result<()> {
        if let Some(limit) = self.limit {
            if limit > 10_000 {
                return Err(anyhow!("Limit too large: {}", limit));
            }
        }
        Ok(())
    }
}
```

#### 8.1.2 Response Struct Pattern

**Standard Work Sequence**:

1. Define response struct with full derives
2. Add standard identification fields
3. Add tool-specific data fields
4. Add metadata/pagination fields
5. Implement helper methods

```rust
// STANDARD PATTERN - Follow exactly for all tool responses
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StandardToolResponse {
    // Step 2: Standard identification (always include)
    pub workbook_id: WorkbookId,
    pub workbook_short_id: String,

    // Step 3: Tool-specific data
    pub data: Vec<DataItem>,

    // Step 4: Metadata/pagination (if applicable)
    pub total_count: u32,
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u32>,

    // Notes for user (if applicable)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}
```

#### 8.1.3 NewType Wrapper Pattern

**Standard Work Sequence**:

1. Define wrapper with `#[serde(transparent)]`
2. Implement `as_str()` or equivalent accessor
3. Implement `Display` trait
4. Implement `From` conversions
5. Add validation in constructor

```rust
// STANDARD PATTERN - Follow exactly for all NewType wrappers
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(transparent)]
pub struct WorkbookId(pub String);

impl WorkbookId {
    // Step 5: Validation constructor
    pub fn new(s: String) -> Result<Self> {
        if s.is_empty() {
            return Err(anyhow!("Workbook ID cannot be empty"));
        }
        Ok(Self(s))
    }

    // Step 2: Accessor
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// Step 3: Display trait
impl std::fmt::Display for WorkbookId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Step 4: From conversion
impl From<String> for WorkbookId {
    fn from(s: String) -> Self {
        Self(s)
    }
}
```

#### 8.1.4 Enum Pattern

**Standard Work Sequence**:

1. Define enum with `#[serde(rename_all = "snake_case")]`
2. Add variants in logical order
3. Use `#[serde(rename)]` for special cases
4. Implement `Display` trait
5. Add helper methods

```rust
// STANDARD PATTERN - Follow exactly for all enums
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StandardEnum {
    // Step 2: Variants in logical order
    VariantA,
    VariantB,

    // Step 3: Special case with rename
    #[serde(rename = "special_variant")]
    SpecialCase,
}

// Step 4: Display trait
impl std::fmt::Display for StandardEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VariantA => write!(f, "variant_a"),
            Self::VariantB => write!(f, "variant_b"),
            Self::SpecialCase => write!(f, "special_variant"),
        }
    }
}

// Step 5: Helper methods
impl StandardEnum {
    pub fn is_special(&self) -> bool {
        matches!(self, Self::SpecialCase)
    }
}
```

### 8.2 Poka-Yoke (Error Proofing)

**Serialization Error Prevention**:

1. **Type Safety**: Use NewType wrappers to prevent type confusion
2. **Required Fields**: Never use `Option<T>` for truly required fields
3. **Default Values**: Always use `#[serde(default)]` for `Option<T>` fields
4. **Validation**: Validate after deserialization, before business logic
5. **Size Limits**: Check response size before serialization

**Implementation**:

```rust
// Poka-Yoke: Type-safe workbook ID
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkbookId(String);  // Cannot mix with String accidentally

// Poka-Yoke: Required vs Optional clear distinction
#[derive(Debug, Deserialize)]
pub struct SafeParams {
    pub required_field: String,        // ← Truly required
    #[serde(default)]
    pub optional_field: Option<String>, // ← Truly optional
}

// Poka-Yoke: Size validation before sending
fn ensure_response_size<T: Serialize>(value: &T, limit: usize) -> Result<()> {
    let payload = serde_json::to_vec(value)?;
    if payload.len() > limit {
        return Err(anyhow!("Response too large: {} > {}", payload.len(), limit));
    }
    Ok(())
}
```

### 8.3 Continuous Improvement (Kaizen)

**Serialization Metrics to Track**:

1. Serialization time per tool
2. Deserialization time per tool
3. Response payload size distribution
4. Validation error frequency
5. Schema validation failures

**Implementation**:

```rust
use std::time::Instant;

pub struct SerializationMetrics {
    serialize_time: std::time::Duration,
    deserialize_time: std::time::Duration,
    payload_size: usize,
}

fn track_serialization<T: Serialize>(value: &T) -> Result<(Vec<u8>, SerializationMetrics)> {
    let start = Instant::now();
    let payload = serde_json::to_vec(value)?;
    let serialize_time = start.elapsed();

    let metrics = SerializationMetrics {
        serialize_time,
        deserialize_time: std::time::Duration::ZERO,
        payload_size: payload.len(),
    };

    Ok((payload, metrics))
}
```

---

## Appendix A: Dependency Versions

Tested and recommended versions:

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
schemars = { version = "1.0", features = ["derive"] }
thiserror = "1.0"
anyhow = "1.0"

# Optional for advanced use cases
serde_with = "3.8"
serde_yaml = "0.9"
```

---

## Appendix B: Common Pitfalls

### B.1 Forgetting `#[serde(default)]`

**Problem**: Optional fields without `#[serde(default)]` are still required

```rust
// ✗ WRONG - field is required even though it's Option<T>
#[derive(Deserialize)]
pub struct BadParams {
    pub optional_field: Option<String>,
}

// ✓ CORRECT - field is truly optional
#[derive(Deserialize)]
pub struct GoodParams {
    #[serde(default)]
    pub optional_field: Option<String>,
}
```

### B.2 Enum Variant Naming

**Problem**: Default external tagging can be verbose

```rust
// Serializes as: {"VariantName": "value"}
#[derive(Serialize)]
pub enum VerboseEnum {
    VariantName(String),
}

// Better: Use rename_all
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanEnum {
    VariantName(String),  // Serializes as: {"variant_name": "value"}
}
```

### B.3 Circular References

**Problem**: Circular data structures cause infinite recursion

```rust
// ✗ DANGEROUS - can cause stack overflow
#[derive(Serialize)]
pub struct Node {
    pub children: Vec<Node>,
}
```

**Solution**: Use reference counting or limit depth

```rust
#[derive(Serialize)]
pub struct SafeNode {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Arc<SafeNode>>,
}
```

### B.4 Large Response Payloads

**Problem**: Serializing large responses can cause memory issues

**Solution**: Use pagination and size limits

```rust
fn ensure_response_size<T: Serialize>(value: &T, limit: usize) -> Result<()> {
    let payload = serde_json::to_vec(value)?;
    if payload.len() > limit {
        return Err(anyhow!("Response exceeds size limit"));
    }
    Ok(())
}
```

---

## Appendix C: Quick Reference

### Common Serde Attributes

| Attribute | Purpose | Example |
|-----------|---------|---------|
| `#[serde(rename = "...")]` | Rename field/variant | `#[serde(rename = "custom_name")]` |
| `#[serde(rename_all = "...")]` | Rename all variants | `#[serde(rename_all = "snake_case")]` |
| `#[serde(default)]` | Use Default::default() | `#[serde(default)]` |
| `#[serde(skip)]` | Skip serialization/deserialization | `#[serde(skip)]` |
| `#[serde(skip_serializing_if = "...")]` | Skip if condition true | `#[serde(skip_serializing_if = "Option::is_none")]` |
| `#[serde(alias = "...")]` | Accept alternative name | `#[serde(alias = "old_name")]` |
| `#[serde(transparent)]` | Serialize as inner type | `#[serde(transparent)]` |
| `#[serde(tag = "type")]` | Internally tagged enum | `#[serde(tag = "kind")]` |
| `#[serde(flatten)]` | Flatten struct fields | `#[serde(flatten)]` |

### Common JsonSchema Attributes

| Attribute | Purpose | Example |
|-----------|---------|---------|
| `#[schemars(description = "...")]` | Add description | `#[schemars(description = "Field description")]` |
| `#[schemars(example = "...")]` | Add example value | `#[schemars(example = "example_value")]` |

---

## Conclusion

This guide establishes standardized serialization patterns for Rust MCP servers based on proven patterns from the ggen-mcp codebase. Following these standards ensures:

- **Type Safety**: Compile-time guarantees through NewType wrappers
- **Protocol Compliance**: Correct JSON serialization for MCP
- **Performance**: Efficient serialization with size validation
- **Maintainability**: Consistent patterns across all tools
- **Error Prevention**: Validation at multiple levels

**Key Takeaways**:

1. Use the standard derive trio: `Serialize`, `Deserialize`, `JsonSchema`
2. Always use `#[serde(default)]` for `Option<T>` fields
3. Use NewType wrappers for type safety
4. Validate after deserialization, before business logic
5. Check response size before serialization
6. Follow TPS standardized work patterns

For questions or improvements, refer to the TPS continuous improvement process.

---

**Document Version**: 1.0
**Last Updated**: 2026-01-20
**Maintainer**: ggen-mcp team
