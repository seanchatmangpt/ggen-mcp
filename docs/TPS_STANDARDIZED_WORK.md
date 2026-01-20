# TPS Standardized Work for MCP Servers

## Executive Summary

This document applies Toyota Production System (TPS) Standardized Work principles to Model Context Protocol (MCP) server development. Based on analysis of the ggen-mcp (spreadsheet-mcp) codebase, this guide codifies proven patterns, standards, and best practices for building reliable, maintainable MCP servers.

**Document Purpose**: Establish standardized work to eliminate variation, reduce defects, and enable continuous improvement in MCP server development.

**Last Updated**: 2026-01-20
**Codebase Analyzed**: ggen-mcp (spreadsheet-mcp)
**Analysis Scope**: 15,000+ lines of production code, 60+ documentation files, 60+ tests

---

## Table of Contents

1. [Introduction to TPS Standardized Work](#1-introduction-to-tps-standardized-work)
2. [MCP Tool Design Standards](#2-mcp-tool-design-standards)
3. [Error Response Standards](#3-error-response-standards)
4. [Validation Standards](#4-validation-standards)
5. [Configuration Standards](#5-configuration-standards)
6. [Code Organization Standards](#6-code-organization-standards)
7. [Documentation Standards](#7-documentation-standards)
8. [Testing Standards](#8-testing-standards)
9. [Quality Assurance Standards](#9-quality-assurance-standards)
10. [Continuous Improvement Process](#10-continuous-improvement-process)

---

## 1. Introduction to TPS Standardized Work

### 1.1 What is Standardized Work?

Standardized Work is a TPS principle that establishes the current best method for performing a task. It consists of three elements:

1. **Takt Time**: The rate at which the customer needs the product/service
2. **Work Sequence**: The precise order of operations
3. **Standard Inventory**: The minimum materials/work-in-progress needed

### 1.2 Application to MCP Servers

In MCP server development, Standardized Work means:

1. **Response Time Standards**: Consistent, predictable tool execution times
2. **Development Sequence**: Standardized patterns for implementing tools
3. **Code Standards**: Minimal variation in structure, naming, and patterns

### 1.3 Benefits for MCP Development

- **Reduced Defects**: Fewer bugs through consistent patterns
- **Faster Development**: Developers follow proven patterns
- **Easier Maintenance**: Uniform code is easier to understand
- **Knowledge Transfer**: New developers quickly learn standards
- **Continuous Improvement**: Standards provide baseline for improvement

### 1.4 Core TPS Principles Applied

#### Jidoka (Automation with Human Touch)
- Type system catches errors at compile time (NewType wrappers)
- JSON schema validation catches invalid inputs before execution
- Circuit breakers prevent cascading failures

#### Poka-Yoke (Error Proofing)
- Input validation guards prevent invalid data
- Transaction guards prevent resource leaks
- Boundary checks prevent out-of-range errors

#### Kaizen (Continuous Improvement)
- Documentation of current standards enables improvement
- Metrics collection (audit trails, cache stats) enables analysis
- Regular review of standards based on production experience

---

## 2. MCP Tool Design Standards

### 2.1 Standard Tool Structure

Every MCP tool MUST follow this standardized structure:

```rust
// 1. Parameter struct with validation attributes
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ToolNameParams {
    /// Required parameters come first
    pub required_field: RequiredType,

    /// Optional parameters come last with #[serde(default)]
    #[serde(default)]
    pub optional_field: Option<OptionalType>,
}

// 2. Tool handler function with consistent signature
pub async fn tool_name(
    state: Arc<AppState>,
    params: ToolNameParams,
) -> Result<ToolNameResponse> {
    // 3. Input validation (fail-fast)
    validate_inputs(&params)?;

    // 4. Resource acquisition
    let workbook = state.open_workbook(&params.workbook_id).await?;

    // 5. Business logic (blocking work in spawn_blocking)
    let result = tokio::task::spawn_blocking(move || {
        perform_computation(workbook, params)
    }).await??;

    // 6. Response construction
    Ok(ToolNameResponse {
        // Standard fields first
        workbook_id: result.workbook_id,
        // Tool-specific fields
        data: result.data,
    })
}

// 7. MCP integration with rmcp macros
#[tool(
    name = "tool_name",
    description = "Clear, concise description"
)]
pub async fn tool_name_handler(
    server: &SpreadsheetServer,
    Parameters(params): Parameters<ToolNameParams>,
) -> Result<Json<ToolNameResponse>, McpError> {
    server.ensure_tool_enabled("tool_name").map_err(to_mcp_error)?;

    server.run_tool_with_timeout(
        "tool_name",
        tools::tool_name(server.state.clone(), params.into()),
    )
    .await
    .map(Json)
    .map_err(to_mcp_error)
}
```

### 2.2 Parameter Design Standards

#### 2.2.1 Required vs Optional Fields

**Standard**: Required fields MUST NOT use `Option<T>`

```rust
// ✓ CORRECT
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadTableParams {
    pub workbook_id: WorkbookId,      // Required, no Option
    pub sheet_name: String,            // Required, no Option

    #[serde(default)]
    pub region_id: Option<u32>,        // Optional, uses Option
    #[serde(default)]
    pub limit: Option<u32>,            // Optional, uses Option
}

// ✗ INCORRECT
pub struct BadParams {
    pub workbook_id: Option<WorkbookId>,  // Required field using Option
}
```

#### 2.2.2 Field Ordering

**Standard**: Fields MUST be ordered by:
1. Identifiers (workbook_id, fork_id, sheet_name)
2. Core parameters (region_id, range)
3. Filtering parameters (filters, queries)
4. Pagination (limit, offset)
5. Output options (format, include_*)
6. Boolean flags

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct StandardParams {
    // 1. Identifiers
    pub workbook_id: WorkbookId,
    pub sheet_name: String,

    // 2. Core parameters
    #[serde(default)]
    pub range: Option<String>,

    // 3. Filtering
    #[serde(default)]
    pub filters: Option<Vec<String>>,

    // 4. Pagination
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,

    // 5. Output options
    #[serde(default)]
    pub include_metadata: Option<bool>,

    // 6. Boolean flags
    #[serde(default)]
    pub compact: bool,
}
```

#### 2.2.3 Naming Conventions

**Standard**: Parameter names MUST follow these conventions:

| Concept | Standard Name | Type | Example |
|---------|--------------|------|---------|
| Workbook identifier | `workbook_id` | `WorkbookId` | "wb-abc123" |
| Fork identifier | `fork_id` | `ForkId` | "fork-xyz789" |
| Sheet name | `sheet_name` | `String` | "Q1 Revenue" |
| Cell range | `range` | `Option<String>` | "A1:B10" |
| Region identifier | `region_id` | `Option<u32>` | 1 |
| Pagination limit | `limit` | `Option<u32>` | 100 |
| Pagination offset | `offset` | `Option<u32>` | 0 |
| Boolean flags | `include_*` or `enable_*` | `bool` | `include_headers` |

### 2.3 Response Design Standards

#### 2.3.1 Standard Response Structure

**Standard**: All responses MUST include context fields:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolResponse {
    // 1. Context fields (REQUIRED)
    pub workbook_id: WorkbookId,
    pub workbook_short_id: String,

    // 2. Request echo (when applicable)
    pub sheet_name: String,

    // 3. Result metadata
    pub row_count: usize,
    pub has_more: bool,

    // 4. Primary data
    pub data: Vec<DataItem>,

    // 5. Secondary data (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,

    // 6. Warnings/notes (optional)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub notes: Vec<String>,
}
```

#### 2.3.2 Derive Macro Standard

**Standard**: All model structs MUST derive these traits in this order:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModelStruct {
    // fields
}

// Additional derives for specific use cases:
// - Add PartialEq, Eq for value objects
// - Add Hash for types used as map keys
// - Add Default only when a sensible default exists
```

#### 2.3.3 Field Serialization Standards

**Standard**: Use these serde attributes for optional/conditional fields:

```rust
#[derive(Serialize)]
pub struct Response {
    // Always included
    pub required_field: String,

    // Omit if None
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_field: Option<String>,

    // Omit if empty vector
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub items: Vec<Item>,

    // Omit if empty map
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub metadata: HashMap<String, String>,

    // Custom condition
    #[serde(skip_serializing_if = "is_default")]
    pub custom: CustomType,
}

fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    value == &T::default()
}
```

### 2.4 Tool Implementation Sequence

**Standard Work Sequence** for implementing a new tool:

1. **Define Parameter Struct** (5 min)
   - Add to `src/model.rs` or tool-specific file
   - Follow field ordering standard
   - Add JsonSchema derives
   - Document all fields

2. **Define Response Struct** (5 min)
   - Add to `src/model.rs`
   - Include standard context fields
   - Follow derive macro standard
   - Document all fields

3. **Implement Business Logic** (variable)
   - Add function to `src/tools/mod.rs` or appropriate module
   - Follow standard tool structure
   - Use spawn_blocking for CPU-intensive work
   - Add comprehensive error context

4. **Add Input Validation** (10 min)
   - Use validation guards from `src/validation/`
   - Validate at function entry
   - Provide clear error messages

5. **Register MCP Handler** (5 min)
   - Add `#[tool]` macro handler
   - Register in tool router
   - Add to enabled_tools check

6. **Write Tests** (20 min)
   - Unit tests for business logic
   - Integration tests for full flow
   - Edge case tests

7. **Document Tool** (10 min)
   - Add to README tool table
   - Update server instructions
   - Add examples if complex

**Total Standard Time**: ~55 minutes + business logic complexity

### 2.5 Async vs Blocking Work

**Standard**: Follow this decision tree:

```
Is the operation I/O bound? (network, file, database)
├─ YES → Use async/await
│  └─ Example: state.open_workbook().await
│
└─ NO → Is it CPU intensive? (parsing, calculation, compression)
   ├─ YES → Use tokio::task::spawn_blocking
   │  └─ Example: spawn_blocking(move || parse_workbook(data))
   │
   └─ NO → Use regular sync code
      └─ Example: simple validation, data structure manipulation
```

```rust
// ✓ CORRECT: I/O bound operation
pub async fn load_data(state: Arc<AppState>) -> Result<Data> {
    let workbook = state.open_workbook(&id).await?;  // Async I/O
    Ok(workbook.data)
}

// ✓ CORRECT: CPU intensive operation
pub async fn process_large_sheet(workbook: Arc<WorkbookContext>) -> Result<Stats> {
    tokio::task::spawn_blocking(move || {
        // Heavy CPU work here
        calculate_statistics(&workbook)
    }).await??
}

// ✗ INCORRECT: Blocking I/O in async function
pub async fn bad_example(path: &Path) -> Result<Data> {
    let data = std::fs::read(path)?;  // Blocks async runtime!
    Ok(parse(data))
}
```

---

## 3. Error Response Standards

### 3.1 Error Type Hierarchy

**Standard**: Use this error handling approach:

```rust
// Internal errors: anyhow::Result
pub async fn internal_function() -> Result<Data> {
    // Use ? operator freely
    let workbook = load_workbook()?;
    let sheet = workbook.get_sheet(name)?;
    Ok(process(sheet)?)
}

// MCP boundary: rmcp::ErrorData
pub async fn mcp_handler() -> Result<Json<Response>, McpError> {
    internal_function()
        .await
        .map(Json)
        .map_err(to_mcp_error)  // Convert at boundary
}
```

### 3.2 Error Context Standards

**Standard**: Every error MUST have contextual information:

```rust
// ✓ CORRECT: Rich error context
fn open_sheet(workbook_id: &str, sheet_name: &str) -> Result<Sheet> {
    let workbook = load_workbook(workbook_id)
        .with_context(|| format!("Failed to load workbook: {}", workbook_id))?;

    workbook.get_sheet(sheet_name)
        .with_context(|| format!(
            "Sheet '{}' not found in workbook {}. Available sheets: {:?}",
            sheet_name,
            workbook_id,
            workbook.sheet_names()
        ))
}

// ✗ INCORRECT: Generic error
fn bad_example(workbook_id: &str, sheet_name: &str) -> Result<Sheet> {
    let workbook = load_workbook(workbook_id)?;  // What failed?
    Ok(workbook.get_sheet(sheet_name)?)          // Why did it fail?
}
```

### 3.3 Error Message Format

**Standard**: Error messages MUST follow this template:

```
{Operation failed}: {specific reason}. {helpful context}
```

Examples:

```rust
// Parameter validation
"Invalid sheet_name: contains illegal character ':'. Sheet names cannot contain: : \\ / ? * [ ]"

// Resource not found
"Sheet 'Revenue' not found in workbook wb-abc123. Available sheets: ['Data', 'Summary', 'Config']"

// Operation failed
"Failed to calculate formulas: LibreOffice process exited with code 1. Check that LibreOffice is installed and accessible."

// Constraint violation
"Range A1:Z100 exceeds screenshot limit of 100 rows × 30 columns. Try splitting into smaller ranges: [A1:Z50, A51:Z100]"
```

### 3.4 MCP Error Code Mapping

**Standard**: Map internal errors to appropriate MCP error codes:

```rust
fn to_mcp_error(err: anyhow::Error) -> McpError {
    let error_str = err.to_string();

    // Check error message patterns
    if error_str.contains("not found") {
        McpError::invalid_params(error_str, None)
    } else if error_str.contains("timeout") {
        McpError::request_timeout(error_str, None)
    } else if error_str.contains("validation") {
        McpError::invalid_params(error_str, None)
    } else if error_str.contains("permission") {
        McpError::permission_denied(error_str, None)
    } else {
        // Default to internal error
        McpError::internal_error(error_str, None)
    }
}
```

### 3.5 Error Recovery Standards

**Standard**: Implement recovery following this hierarchy:

1. **Retry** - For transient failures (network, LibreOffice)
2. **Fallback** - For optional features (region detection → full sheet)
3. **Partial Success** - For batch operations (continue processing)
4. **Circuit Breaker** - For cascading failures (stop calling failing service)
5. **Graceful Degradation** - Return limited results instead of failure

```rust
// Example: Retry with fallback
async fn get_regions_with_fallback(sheet: &Sheet) -> Result<Vec<Region>> {
    // Try advanced region detection
    match detect_regions_with_retry(sheet, 3).await {
        Ok(regions) => Ok(regions),
        Err(e) => {
            tracing::warn!("Region detection failed: {}. Falling back to simple detection", e);
            // Fallback to simple detection
            Ok(detect_simple_regions(sheet)?)
        }
    }
}
```

---

## 4. Validation Standards

### 4.1 Validation Layer Architecture

**Standard**: Implement validation at three layers:

```
┌─────────────────────────────────────────┐
│  Layer 1: JSON Schema Validation       │  ← Type, structure, required fields
│  (Automated via schemars)               │
└─────────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────────┐
│  Layer 2: Input Guards                 │  ← Bounds, formats, safe characters
│  (Manual via validation::input_guards)  │
└─────────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────────┐
│  Layer 3: Business Logic Validation    │  ← Domain rules, consistency
│  (Tool-specific validation)             │
└─────────────────────────────────────────┘
```

### 4.2 Input Validation Standards

**Standard**: Validate all inputs at function entry:

```rust
pub async fn read_table(
    state: Arc<AppState>,
    params: ReadTableParams,
) -> Result<ReadTableResponse> {
    // Layer 2: Input guards (fail-fast at entry)
    use crate::validation::{
        validate_workbook_id,
        validate_sheet_name,
        validate_optional_numeric_range,
    };

    // Validate identifiers
    validate_workbook_id(params.workbook_id.as_str())
        .map_err(|e| anyhow!("Invalid workbook_id: {}", e))?;

    validate_sheet_name(&params.sheet_name)
        .map_err(|e| anyhow!("Invalid sheet_name: {}", e))?;

    // Validate numeric ranges
    let limit = validate_optional_numeric_range(
        "limit",
        params.limit,
        1u32,
        10_000u32
    ).map_err(|e| anyhow!(e))?;

    let offset = validate_optional_numeric_range(
        "offset",
        params.offset,
        0u32,
        1_000_000u32
    ).map_err(|e| anyhow!(e))?;

    // Layer 3: Business logic validation
    if let (Some(region_id), Some(range)) = (params.region_id, &params.range) {
        bail!("Cannot specify both region_id and range. Choose one.");
    }

    // Proceed with validated inputs
    let workbook = state.open_workbook(&params.workbook_id).await?;
    // ...
}
```

### 4.3 Validation Function Standards

**Standard**: All validation functions MUST:

1. Have `validate_` prefix
2. Return `Result<T, ValidationError>`
3. Take parameter name as first argument (for error messages)
4. Include clear error messages with constraints

```rust
/// Validates that a numeric value is within acceptable bounds.
///
/// # Arguments
/// * `parameter_name` - Name of the parameter being validated (for errors)
/// * `value` - The value to validate
/// * `min` - Minimum acceptable value (inclusive)
/// * `max` - Maximum acceptable value (inclusive)
///
/// # Returns
/// Ok(value) if valid, Err with descriptive message if invalid
pub fn validate_numeric_range<T>(
    parameter_name: &str,
    value: T,
    min: T,
    max: T,
) -> ValidationResult<T>
where
    T: PartialOrd + Display + Copy,
{
    if value < min || value > max {
        Err(ValidationError::NumericOutOfRange {
            parameter: parameter_name.to_string(),
            value: format!("{}", value),
            min: format!("{}", min),
            max: format!("{}", max),
        })
    } else {
        Ok(value)
    }
}
```

### 4.4 Validation Constants

**Standard**: Define validation limits as public constants:

```rust
// src/validation/bounds.rs

/// Maximum number of rows in an Excel worksheet (2^20)
pub const EXCEL_MAX_ROWS: u32 = 1_048_576;

/// Maximum number of columns in an Excel worksheet (2^14)
pub const EXCEL_MAX_COLUMNS: u32 = 16_384;

/// Maximum reasonable sample size for statistics
pub const MAX_SAMPLE_SIZE: usize = 100_000;

/// Maximum reasonable limit value for pagination
pub const MAX_PAGINATION_LIMIT: usize = 10_000;

/// Maximum reasonable offset value for pagination
pub const MAX_PAGINATION_OFFSET: usize = 1_000_000;
```

**Usage**:

```rust
use crate::validation::bounds::*;

validate_numeric_range("limit", params.limit, 1, MAX_PAGINATION_LIMIT as u32)?;
```

### 4.5 NewType Validation Pattern

**Standard**: Use NewType wrappers for domain primitives:

```rust
/// Workbook identifier with validation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct WorkbookId(String);

impl WorkbookId {
    /// Create a new WorkbookId with validation
    pub fn new(id: String) -> Result<Self, ValidationError> {
        if id.is_empty() {
            return Err(ValidationError::Empty("workbook_id"));
        }
        if id.len() > 1024 {
            return Err(ValidationError::TooLong {
                field: "workbook_id",
                max: 1024,
                actual: id.len(),
            });
        }
        Ok(Self(id))
    }

    /// Access the inner value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

**Benefits**:
- Compile-time prevention of type confusion
- Centralized validation
- Zero runtime cost
- Self-documenting code

---

## 5. Configuration Standards

### 5.1 Configuration Structure

**Standard**: Use a three-tier configuration system:

```rust
pub struct ServerConfig {
    // Tier 1: Essential (required for startup)
    pub workspace_root: PathBuf,
    pub transport: TransportKind,

    // Tier 2: Operational (with sensible defaults)
    pub cache_capacity: usize,              // default: 5
    pub supported_extensions: Vec<String>,   // default: ["xlsx", "xlsm"]

    // Tier 3: Feature flags (disabled by default)
    pub recalc_enabled: bool,               // default: false
    pub vba_enabled: bool,                  // default: false

    // Tier 4: Limits and constraints
    pub max_concurrent_recalcs: usize,      // default: 2
    pub tool_timeout_ms: Option<u64>,       // default: Some(30_000)
    pub max_response_bytes: Option<u64>,    // default: Some(1_000_000)
}
```

### 5.2 Configuration Sources Priority

**Standard**: Configuration sources in priority order (highest to lowest):

1. **Command-line arguments** - Explicit user intent
2. **Environment variables** - Container/deployment config
3. **Configuration file** - Persistent settings
4. **Defaults** - Sensible fallbacks

```rust
impl ServerConfig {
    pub fn from_args(args: CliArgs) -> Result<Self> {
        // Load file config if specified
        let file_config = if let Some(path) = args.config {
            load_config_file(&path)?
        } else {
            PartialConfig::default()
        };

        // Merge with priority: CLI > Env > File > Default
        let workspace_root = args.workspace_root
            .or(env::var("WORKSPACE_ROOT").ok().map(PathBuf::from))
            .or(file_config.workspace_root)
            .unwrap_or_else(|| PathBuf::from("."));

        // ... merge other fields

        Ok(Self {
            workspace_root,
            // ...
        })
    }
}
```

### 5.3 Configuration Validation

**Standard**: Validate all configuration at startup (fail-fast):

```rust
impl ServerConfig {
    /// Validates configuration before server start.
    /// Returns detailed errors for any invalid settings.
    pub fn validate(&self) -> Result<()> {
        // 1. Validate workspace
        self.ensure_workspace_root()
            .context("Workspace validation failed")?;

        // 2. Validate numeric bounds
        validate_cache_capacity(self.cache_capacity)
            .context("Invalid cache_capacity")?;

        validate_numeric_range(
            "max_concurrent_recalcs",
            self.max_concurrent_recalcs,
            MIN_CONCURRENT_RECALCS,
            MAX_CONCURRENT_RECALCS,
        ).context("Invalid max_concurrent_recalcs")?;

        // 3. Validate feature combinations
        if self.recalc_enabled && !cfg!(feature = "recalc") {
            bail!("recalc_enabled=true but 'recalc' feature not compiled");
        }

        // 4. Validate extensions list
        ensure!(
            !self.supported_extensions.is_empty(),
            "supported_extensions cannot be empty"
        );

        Ok(())
    }

    fn ensure_workspace_root(&self) -> Result<()> {
        let path = &self.workspace_root;

        ensure!(path.exists(), "workspace_root {:?} does not exist", path);
        ensure!(path.is_dir(), "workspace_root {:?} is not a directory", path);

        // Check read permission
        std::fs::read_dir(path)
            .with_context(|| format!("Cannot read workspace_root {:?}", path))?;

        Ok(())
    }
}
```

### 5.4 Environment Variable Naming

**Standard**: Environment variables MUST follow this pattern:

```
{PROJECT}__{SECTION}__{SETTING}
```

Examples:

```bash
# General settings
SPREADSHEET_MCP_WORKSPACE=/data/workbooks
SPREADSHEET_MCP_TRANSPORT=stdio

# Cache settings
SPREADSHEET_MCP_CACHE_CAPACITY=10

# Feature flags
SPREADSHEET_MCP_VBA_ENABLED=true
SPREADSHEET_MCP_RECALC_ENABLED=true

# Limits
SPREADSHEET_MCP_MAX_CONCURRENT_RECALCS=4
SPREADSHEET_MCP_TOOL_TIMEOUT_MS=60000
SPREADSHEET_MCP_MAX_RESPONSE_BYTES=5000000
```

### 5.5 Configuration Documentation

**Standard**: Every configuration option MUST be documented in a table:

| Flag | Env | Type | Default | Description |
|------|-----|------|---------|-------------|
| `--workspace-root` | `SPREADSHEET_MCP_WORKSPACE` | Path | `.` | Root directory for workbooks |
| `--cache-capacity` | `SPREADSHEET_MCP_CACHE_CAPACITY` | uint | `5` | LRU cache size (1-100) |
| `--vba-enabled` | `SPREADSHEET_MCP_VBA_ENABLED` | bool | `false` | Enable VBA inspection tools |
| `--tool-timeout-ms` | `SPREADSHEET_MCP_TOOL_TIMEOUT_MS` | uint | `30000` | Tool timeout in ms (0=none) |

---

## 6. Code Organization Standards

### 6.1 Module Structure

**Standard**: Organize code in this directory structure:

```
src/
├── main.rs                 # Entry point (minimal, delegates to lib)
├── lib.rs                  # Public API surface
├── server.rs               # MCP server implementation
├── config.rs               # Configuration management
├── state.rs                # Application state
├── model.rs                # Data models (params, responses)
├── workbook.rs             # Core domain logic
├── utils.rs                # Shared utilities
│
├── tools/                  # Tool implementations
│   ├── mod.rs             # Core tools
│   ├── fork.rs            # Fork management tools
│   ├── vba.rs             # VBA tools
│   └── filters.rs         # Filtering utilities
│
├── validation/             # Input validation
│   ├── mod.rs             # Public API
│   ├── bounds.rs          # Boundary checks
│   ├── input_guards.rs    # Input validation
│   ├── schema.rs          # JSON schema validation
│   └── middleware.rs      # Validation middleware
│
├── domain/                 # Domain-driven design
│   ├── mod.rs             # Domain exports
│   ├── value_objects.rs   # NewType wrappers
│   ├── aggregates.rs      # Domain aggregates
│   └── commands.rs        # Domain commands
│
├── recovery/               # Error recovery
│   ├── mod.rs             # Recovery framework
│   ├── retry.rs           # Retry logic
│   ├── circuit_breaker.rs # Circuit breaker
│   ├── fallback.rs        # Fallback strategies
│   └── partial_success.rs # Partial results
│
├── audit/                  # Audit trail
│   ├── mod.rs             # Audit system
│   ├── integration.rs     # Integration helpers
│   └── examples.rs        # Usage examples
│
└── analysis/               # Analysis features
    ├── mod.rs
    ├── stats.rs
    ├── formula.rs
    └── classification.rs
```

### 6.2 Import Organization

**Standard**: Order imports in this sequence:

```rust
// 1. Standard library
use std::collections::HashMap;
use std::sync::Arc;

// 2. External crates (alphabetically)
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use tokio::task;

// 3. Internal crates
use rmcp::{tool, ErrorData as McpError};

// 4. Current crate (absolute paths)
use crate::config::ServerConfig;
use crate::model::{ReadTableParams, ReadTableResponse};
use crate::state::AppState;
use crate::validation::{validate_sheet_name, validate_workbook_id};

// 5. Parent/sibling modules (relative paths)
use super::filters::WorkbookFilter;
```

### 6.3 File Size Limits

**Standard**: Enforce these file size guidelines:

- **Maximum file size**: 1,000 lines
- **Recommended file size**: 500 lines
- **When to split**: > 800 lines or > 3 major concerns

```rust
// ✓ GOOD: Focused module (300 lines)
// src/tools/read_operations.rs
pub mod read_table;
pub mod read_range;
pub mod sheet_page;

// ✗ BAD: Monolithic module (3,000 lines)
// src/tools.rs
// Contains all tools, utilities, filters, etc.
```

### 6.4 Function Length Standards

**Standard**: Follow these function length guidelines:

- **Maximum**: 100 lines
- **Recommended**: 30 lines
- **Extract when**: Logic is repeated or function has > 2 responsibilities

```rust
// ✓ GOOD: Focused function
pub fn validate_inputs(params: &Params) -> Result<()> {
    validate_workbook_id(&params.workbook_id)?;
    validate_sheet_name(&params.sheet_name)?;
    validate_range(&params)?;
    Ok(())
}

// ✗ BAD: Monolithic function (150 lines)
pub fn process_sheet(params: Params) -> Result<Response> {
    // validation (20 lines)
    // data loading (30 lines)
    // processing (50 lines)
    // response building (30 lines)
    // error handling (20 lines)
}
```

### 6.5 Naming Conventions

**Standard**: Follow these naming patterns:

| Item | Convention | Example |
|------|------------|---------|
| Modules | `snake_case` | `input_guards`, `circuit_breaker` |
| Types | `PascalCase` | `WorkbookId`, `ValidationError` |
| Functions | `snake_case` | `validate_sheet_name`, `open_workbook` |
| Constants | `SCREAMING_SNAKE_CASE` | `EXCEL_MAX_ROWS`, `DEFAULT_TIMEOUT` |
| Statics | `SCREAMING_SNAKE_CASE` | `GLOBAL_CONFIG`, `VALIDATOR` |
| Type parameters | Single uppercase letter or `PascalCase` | `T`, `E`, `TResponse` |

**Function naming patterns**:

- `validate_*` - Validation functions
- `ensure_*` - Assertion-like checks
- `is_*`, `has_*`, `can_*` - Boolean predicates
- `get_*` - Getters (may fail)
- `try_*` - Fallible operations
- `build_*` - Builder/constructor pattern
- `parse_*` - Parsing operations
- `format_*` - Formatting operations

---

## 7. Documentation Standards

### 7.1 Code Documentation

**Standard**: All public items MUST have doc comments:

```rust
/// Reads a table region from a spreadsheet with optional filtering and sampling.
///
/// This tool provides efficient access to tabular data by leveraging region detection
/// or explicit range specification. It supports distributed sampling to minimize
/// token usage while maintaining data representativeness.
///
/// # Arguments
///
/// * `state` - Application state containing workbook cache and configuration
/// * `params` - Table read parameters including workbook ID, sheet name, and filters
///
/// # Returns
///
/// Returns a `ReadTableResponse` containing:
/// - Table headers (auto-detected or from first row)
/// - Row data (paginated and optionally sampled)
/// - Metadata about the table (total rows, sampling info)
///
/// # Errors
///
/// Returns an error if:
/// - Workbook or sheet not found
/// - Invalid region_id or range specification
/// - Both region_id and range specified (mutually exclusive)
/// - Pagination parameters overflow (offset + limit > total rows)
///
/// # Examples
///
/// ```no_run
/// use spreadsheet_mcp::tools::read_table;
/// use spreadsheet_mcp::model::ReadTableParams;
///
/// let params = ReadTableParams {
///     workbook_id: "wb-abc123".into(),
///     sheet_name: "Sales".to_string(),
///     region_id: Some(1),
///     limit: Some(100),
///     sample_mode: Some(SampleMode::Distributed),
///     ..Default::default()
/// };
///
/// let response = read_table(state, params).await?;
/// println!("Read {} rows", response.rows.len());
/// ```
///
/// # Performance
///
/// - Region-scoped reads are O(rows in region)
/// - Distributed sampling is O(limit), not O(total rows)
/// - Uses cached sheet metrics when available
pub async fn read_table(
    state: Arc<AppState>,
    params: ReadTableParams,
) -> Result<ReadTableResponse> {
    // implementation
}
```

### 7.2 README Documentation Structure

**Standard**: Every README MUST include these sections:

1. **Quick Start** - Get running in < 5 minutes
2. **Architecture** - High-level overview
3. **Tool Surface** - Table of all tools
4. **Configuration** - All config options
5. **Examples** - Common use cases
6. **Testing** - How to run tests
7. **Development** - How to contribute

### 7.3 API Documentation Standards

**Standard**: Document all tools in a standardized table:

| Tool | Purpose | Key Parameters | Response |
|------|---------|----------------|----------|
| `list_workbooks` | Discover available workbooks | `folder`, `path_glob` | Array of workbook descriptors |
| `read_table` | Extract tabular data | `region_id`, `limit`, `sample_mode` | Rows with headers |
| `sheet_overview` | Get region detection results | `max_regions`, `include_headers` | Detected regions |

### 7.4 Error Documentation

**Standard**: Document common errors and solutions:

```markdown
## Common Errors

### "Sheet 'Revenue' not found"

**Cause**: The specified sheet name doesn't exist in the workbook.

**Solution**: Use `list_sheets` to see available sheets. Check for:
- Exact name match (case-sensitive)
- Leading/trailing spaces
- Hidden sheets

### "Region ID 5 not found"

**Cause**: The region ID doesn't exist on the specified sheet.

**Solution**: Use `sheet_overview` to see available regions and their IDs.
```

### 7.5 Inline Comments Standards

**Standard**: Use comments to explain "why", not "what":

```rust
// ✓ GOOD: Explains why
// Use spawn_blocking because umya's XML parsing is CPU-intensive
// and can block the async runtime for large spreadsheets
let workbook = tokio::task::spawn_blocking(move || {
    parse_workbook(&path)
}).await??;

// ✗ BAD: Explains what (obvious from code)
// Parse the workbook
let workbook = parse_workbook(&path)?;
```

**Comment guidelines**:

- Explain non-obvious decisions
- Document performance trade-offs
- Note TODO items with JIRA tickets
- Warn about gotchas or edge cases
- Reference related issues/PRs

---

## 8. Testing Standards

### 8.1 Test Organization

**Standard**: Organize tests in three tiers:

```
tests/
├── unit/                   # Unit tests (fast, isolated)
│   ├── validation.rs
│   ├── parsing.rs
│   └── formatting.rs
│
├── integration/            # Integration tests (slower, realistic)
│   ├── tools.rs
│   ├── fork_operations.rs
│   └── error_recovery.rs
│
└── e2e/                    # End-to-end tests (slow, full system)
    ├── docker/            # Docker-based tests
    └── scenarios/         # User scenario tests
```

### 8.2 Test Naming Standards

**Standard**: Name tests using this pattern:

```rust
#[test]
fn test_{module}_{scenario}_{expected_outcome}() {
    // test implementation
}
```

Examples:

```rust
#[test]
fn test_validation_empty_string_returns_error() {
    let result = validate_non_empty_string("param", "");
    assert!(result.is_err());
}

#[test]
fn test_read_table_distributed_sampling_returns_evenly_spaced_rows() {
    // setup
    let params = ReadTableParams {
        limit: Some(10),
        sample_mode: Some(SampleMode::Distributed),
        // ...
    };

    // execute
    let response = read_table(state, params).await.unwrap();

    // verify
    assert_eq!(response.rows.len(), 10);
    assert!(rows_are_evenly_distributed(&response.rows));
}

#[test]
fn test_circuit_breaker_opens_after_threshold_failures() {
    // Given a circuit breaker with threshold 3
    let breaker = CircuitBreaker::new(3, Duration::from_secs(60));

    // When we record 3 failures
    for _ in 0..3 {
        breaker.record_failure();
    }

    // Then the circuit should be open
    assert_eq!(breaker.state(), CircuitState::Open);
}
```

### 8.3 Test Structure Standards

**Standard**: Use the Given-When-Then pattern:

```rust
#[test]
fn test_scenario() {
    // Given: Setup test conditions
    let workbook = create_test_workbook();
    let params = ReadTableParams {
        region_id: Some(1),
        limit: Some(100),
        ..Default::default()
    };

    // When: Execute the operation
    let result = read_table(state, params).await;

    // Then: Verify the outcome
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.rows.len(), 100);
    assert_eq!(response.headers, vec!["A", "B", "C"]);
}
```

### 8.4 Test Coverage Standards

**Standard**: Achieve these coverage targets:

- **Overall**: > 80%
- **Core logic**: > 90% (tools, validation, business logic)
- **Error paths**: > 70% (error handling, recovery)
- **Integration**: > 60% (full flow tests)

**Exceptions**: Auto-generated code, trivial getters/setters

### 8.5 Test Fixture Standards

**Standard**: Use builder pattern for test fixtures:

```rust
// tests/support/builders.rs
pub struct WorkbookBuilder {
    sheets: Vec<SheetBuilder>,
}

impl WorkbookBuilder {
    pub fn new() -> Self {
        Self { sheets: vec![] }
    }

    pub fn add_sheet(mut self, name: &str) -> SheetBuilder {
        let sheet = SheetBuilder::new(name);
        self.sheets.push(sheet.clone());
        sheet
    }

    pub fn build(self) -> Workbook {
        // construct workbook from builders
    }
}

// Usage in tests
#[test]
fn test_with_builder() {
    let workbook = WorkbookBuilder::new()
        .add_sheet("Data")
            .with_cell("A1", "Name")
            .with_cell("A2", "John")
            .done()
        .add_sheet("Summary")
            .with_formula("A1", "=Data!A2")
            .done()
        .build();

    // test with workbook
}
```

### 8.6 Assertion Standards

**Standard**: Use specific assertion methods:

```rust
// ✓ GOOD: Specific assertions
assert_eq!(actual, expected, "rows should match");
assert!(result.is_ok(), "read_table should succeed");
assert_matches!(error, ValidationError::OutOfRange { .. });

// ✗ BAD: Generic assertions
assert!(actual == expected);  // Use assert_eq! instead
assert!(result.is_ok() == true);  // Redundant
```

**Custom assertion helpers**:

```rust
// Define domain-specific assertions
fn assert_valid_workbook_id(id: &str) {
    assert!(
        validate_workbook_id(id).is_ok(),
        "Expected valid workbook ID, got: {}",
        id
    );
}

fn assert_regions_sorted(regions: &[Region]) {
    let sorted = regions.iter()
        .zip(regions.iter().skip(1))
        .all(|(a, b)| a.id <= b.id);
    assert!(sorted, "Regions should be sorted by ID");
}
```

---

## 9. Quality Assurance Standards

### 9.1 Pre-Commit Checklist

**Standard**: Every commit MUST pass this checklist:

- [ ] Code compiles without warnings (`cargo build`)
- [ ] All tests pass (`cargo test`)
- [ ] Formatting is correct (`cargo fmt --check`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Documentation builds (`cargo doc`)
- [ ] No new unwrap() calls (use expect() with message)
- [ ] All public items documented
- [ ] Error messages are descriptive

### 9.2 Code Review Standards

**Standard**: Code reviews MUST check:

1. **Correctness**
   - Logic is sound
   - Edge cases handled
   - Error handling is comprehensive

2. **Standards Compliance**
   - Follows standardized tool structure
   - Uses correct naming conventions
   - Includes required documentation

3. **Performance**
   - No unnecessary allocations
   - Appropriate use of async/blocking
   - Caching opportunities identified

4. **Security**
   - Input validation present
   - Path traversal prevention
   - No unsafe code without justification

5. **Maintainability**
   - Clear, self-documenting code
   - Appropriate abstraction level
   - No premature optimization

### 9.3 Continuous Integration Standards

**Standard**: CI pipeline MUST include:

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      # 1. Check formatting
      - name: Check formatting
        run: cargo fmt --check

      # 2. Run clippy
      - name: Clippy
        run: cargo clippy -- -D warnings

      # 3. Run tests
      - name: Test
        run: cargo test --all-features

      # 4. Build docs
      - name: Docs
        run: cargo doc --no-deps

      # 5. Security audit
      - name: Security audit
        run: cargo audit
```

### 9.4 Performance Benchmarking

**Standard**: Benchmark critical paths:

```rust
// benches/read_operations.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_read_table(c: &mut Criterion) {
    let workbook = setup_test_workbook();

    c.bench_function("read_table_1000_rows", |b| {
        b.iter(|| {
            read_table(
                black_box(&workbook),
                black_box(1000),
            )
        });
    });
}

criterion_group!(benches, bench_read_table);
criterion_main!(benches);
```

**Performance targets**:

- Tool response time: < 100ms (p50), < 500ms (p99)
- Memory usage: < 500MB per workbook
- Startup time: < 2 seconds

### 9.5 Security Standards

**Standard**: All tools MUST implement:

1. **Input Validation**
   - Validate all user inputs
   - Check bounds and formats
   - Sanitize strings

2. **Path Safety**
   - No path traversal (validate_path_safe)
   - Restrict to workspace_root
   - No absolute paths from user input

3. **Resource Limits**
   - Timeout on long operations
   - Memory limits on responses
   - Rate limiting (if applicable)

4. **Audit Logging**
   - Log all tool invocations
   - Log security-relevant events
   - Log errors with context

---

## 10. Continuous Improvement Process

### 10.1 Kaizen Cycle for Standards

**Standard**: Review and update standards quarterly:

```
┌─────────────────────────────────────────┐
│  1. Observe (Gemba Walk)                │
│  - Review recent code                   │
│  - Identify patterns                    │
│  - Collect metrics                      │
└─────────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────────┐
│  2. Analyze                             │
│  - What worked well?                    │
│  - What caused problems?                │
│  - Where was variation?                 │
└─────────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────────┐
│  3. Propose Changes                     │
│  - Document new pattern                 │
│  - Update this guide                    │
│  - Create migration plan                │
└─────────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────────┐
│  4. Implement & Measure                 │
│  - Apply new standard                   │
│  - Track adoption                       │
│  - Measure impact                       │
└─────────────────────────────────────────┘
```

### 10.2 Metrics Collection

**Standard**: Track these quality metrics:

```rust
// src/metrics.rs
pub struct QualityMetrics {
    // Defect metrics
    pub bugs_per_kloc: f64,           // Bugs per 1000 lines
    pub critical_bugs: u32,            // P0/P1 bugs
    pub mean_time_to_fix: Duration,    // Average bug fix time

    // Standards compliance
    pub doc_coverage: f64,             // % of public items documented
    pub test_coverage: f64,            // % code coverage
    pub clippy_warnings: u32,          // Clippy warning count

    // Performance metrics
    pub p50_response_time: Duration,   // Median response time
    pub p99_response_time: Duration,   // 99th percentile
    pub error_rate: f64,               // % of requests that error

    // Development velocity
    pub cycle_time: Duration,          // Time from commit to deploy
    pub deployment_frequency: f64,     // Deploys per week
    pub lead_time: Duration,           // Time from idea to production
}
```

### 10.3 Standard Review Process

**Standard**: Quarterly review meeting agenda:

1. **Review Metrics** (15 min)
   - Present quality metrics
   - Identify trends
   - Highlight concerns

2. **Discuss Challenges** (20 min)
   - Where did standards help?
   - Where did standards hinder?
   - What gaps exist?

3. **Propose Updates** (20 min)
   - New patterns discovered
   - Obsolete patterns to remove
   - Clarifications needed

4. **Action Items** (5 min)
   - Assign document updates
   - Schedule migration tasks
   - Set next review date

### 10.4 Standard Evolution

**Standard**: Version this document and track changes:

```markdown
## Version History

### v1.1.0 (2026-04-20)
- Added async/blocking decision tree
- Clarified error context requirements
- New section on performance benchmarking

### v1.0.0 (2026-01-20)
- Initial version based on ggen-mcp analysis
- Established core standards
- Documented existing patterns
```

### 10.5 Feedback Mechanism

**Standard**: Collect feedback through:

1. **Code Review Comments** - Tag with `[STANDARD]`
2. **Retrospectives** - Discuss what worked/didn't
3. **Issues** - Label with `standards` tag
4. **Pull Requests** - Propose standard updates

Example:

```markdown
## Standard Update Proposal

**Current Standard**: All responses must include `workbook_id`

**Issue**: Some tools don't operate on workbooks (e.g., `get_manifest_stub`)

**Proposed Change**: Make `workbook_id` required only for workbook-scoped operations

**Impact**: ~5 tool signatures would change

**Migration**: Add deprecation warnings, update in v2.0.0
```

---

## Appendices

### Appendix A: Quick Reference Checklist

#### New Tool Checklist

- [ ] Define parameter struct in `src/model.rs`
- [ ] Define response struct in `src/model.rs`
- [ ] Implement business logic in `src/tools/`
- [ ] Add input validation
- [ ] Register MCP handler with `#[tool]` macro
- [ ] Add to tool router
- [ ] Write unit tests
- [ ] Write integration test
- [ ] Document in README tool table
- [ ] Add usage example

#### Code Review Checklist

- [ ] Follows standard tool structure
- [ ] All inputs validated
- [ ] Error messages are descriptive
- [ ] Uses spawn_blocking for CPU work
- [ ] All public items documented
- [ ] Tests cover edge cases
- [ ] No unwrap() without expect()
- [ ] Clippy warnings resolved

### Appendix B: Standard Patterns Library

#### Pattern: Pagination

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PaginatedParams {
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
}

fn apply_pagination<T>(
    items: Vec<T>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> (Vec<T>, bool) {
    let offset = offset.unwrap_or(0) as usize;
    let limit = limit.unwrap_or(u32::MAX) as usize;

    let total = items.len();
    let start = offset.min(total);
    let end = (start + limit).min(total);
    let has_more = end < total;

    (items[start..end].to_vec(), has_more)
}
```

#### Pattern: Optional Field with Default

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ConfigParams {
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
    30_000
}
```

#### Pattern: Enum with String Serialization

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SampleMode {
    First,
    Distributed,
    Random,
}
```

### Appendix C: Anti-Patterns

#### Anti-Pattern: Bare unwrap()

```rust
// ✗ WRONG
let value = map.get(&key).unwrap();

// ✓ CORRECT
let value = map.get(&key)
    .expect("key should exist after validation");
```

#### Anti-Pattern: Generic Error Messages

```rust
// ✗ WRONG
bail!("Invalid input");

// ✓ CORRECT
bail!("Invalid sheet_name '{}': contains illegal character ':'. \
       Sheet names cannot contain: : \\ / ? * [ ]", sheet_name);
```

#### Anti-Pattern: Blocking in Async

```rust
// ✗ WRONG
pub async fn parse_workbook(path: &Path) -> Result<Workbook> {
    let data = std::fs::read(path)?;  // Blocks async runtime!
    Ok(parse(data))
}

// ✓ CORRECT
pub async fn parse_workbook(path: PathBuf) -> Result<Workbook> {
    tokio::task::spawn_blocking(move || {
        let data = std::fs::read(&path)?;
        Ok(parse(data))
    }).await??
}
```

### Appendix D: Glossary

- **Gemba**: The actual place where work happens (in software: the code)
- **Kaizen**: Continuous improvement
- **Jidoka**: Automation with human intelligence/touch
- **Poka-Yoke**: Error-proofing/mistake-proofing
- **Takt Time**: Rate at which customers consume product
- **Standardized Work**: Current best method documented
- **Andon**: Signal of a problem (in software: alerts, errors)
- **Muda**: Waste (unnecessary code, duplication, etc.)

### Appendix E: References

#### Internal Documentation
- `docs/POKA_YOKE_PATTERN.md` - NewType pattern guide
- `docs/INPUT_VALIDATION_GUIDE.md` - Validation integration
- `docs/VALIDATION_QUICK_REFERENCE.md` - Validation API
- `docs/FORK_TRANSACTION_GUARDS.md` - Transaction safety
- `docs/AUDIT_TRAIL.md` - Audit system
- `RECOVERY_IMPLEMENTATION.md` - Error recovery

#### External Resources
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [MCP Specification](https://modelcontextprotocol.io/)
- [Toyota Production System Overview](https://global.toyota/en/company/vision-and-philosophy/production-system/)
- [The Toyota Way](https://en.wikipedia.org/wiki/The_Toyota_Way)

---

## Document Maintenance

**Owner**: Engineering Team
**Review Frequency**: Quarterly
**Next Review**: 2026-04-20
**Feedback**: Open GitHub issue with `standards` label

---

*This document represents the current standardized work for MCP server development. It is a living document that evolves based on production experience and continuous improvement efforts.*

*Version: 1.0.0*
*Last Updated: 2026-01-20*
*Based on: ggen-mcp (spreadsheet-mcp) codebase analysis*
