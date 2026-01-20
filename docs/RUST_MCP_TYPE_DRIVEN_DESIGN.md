# Type-Driven Development Patterns for Rust MCP Servers

**For ggen-mcp: Leveraging Rust's Type System for Correctness**

> "Make illegal states unrepresentable." — Yaron Minsky

This document provides comprehensive guidance on type-driven development patterns for building robust, type-safe MCP (Model Context Protocol) servers in Rust. These patterns are demonstrated in the ggen-mcp spreadsheet server implementation.

---

## Table of Contents

1. [NewType Patterns for MCP](#1-newtype-patterns-for-mcp)
2. [Type State Patterns](#2-type-state-patterns)
3. [Trait-Based Design](#3-trait-based-design)
4. [Generic Programming](#4-generic-programming)
5. [Zero-Cost Abstractions](#5-zero-cost-abstractions)
6. [Type-Level Validation](#6-type-level-validation)
7. [Serde Integration](#7-serde-integration)
8. [TPS Poka-Yoke Principles](#8-tps-poka-yoke-principles)

---

## 1. NewType Patterns for MCP

### Overview

The NewType pattern wraps primitive types in zero-cost abstractions to prevent type confusion at compile time. This is the foundation of type safety in ggen-mcp.

### Why NewTypes Matter in MCP

MCP servers handle many string-based identifiers (workbook IDs, fork IDs, sheet names, cell addresses). Without NewTypes, it's easy to mix them up:

```rust
// Without NewTypes - DANGEROUS!
fn delete_fork(fork_id: String) -> Result<()> { /* ... */ }
fn get_workbook(workbook_id: String) -> Result<Workbook> { /* ... */ }

// Easy to mix up:
let workbook_id = "wb-123".to_string();
delete_fork(workbook_id)?;  // Oops! Deleted a workbook!
```

### Implementation in ggen-mcp

**Location**: `src/domain/value_objects.rs` (754 lines)

#### 1.1 WorkbookId

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkbookId(String);

impl WorkbookId {
    const MAX_LENGTH: usize = 1024;

    pub fn new(id: String) -> Result<Self, ValidationError> {
        if id.is_empty() {
            return Err(ValidationError::Empty("WorkbookId"));
        }
        if id.len() > Self::MAX_LENGTH {
            return Err(ValidationError::TooLong {
                field: "WorkbookId",
                max: Self::MAX_LENGTH,
                actual: id.len(),
            });
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}
```

**Key Features**:
- **Validation on construction**: Empty or too-long IDs rejected
- **Transparent serialization**: JSON sees just the string
- **Type safety**: Cannot mix with ForkId
- **Zero cost**: No runtime overhead

#### 1.2 ForkId

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ForkId(String);

impl ForkId {
    const MAX_LENGTH: usize = 256;

    pub fn new(id: String) -> Result<Self, ValidationError> {
        // Similar validation...
        Ok(Self(id))
    }
}
```

**Type Safety Demonstration**:
```rust
fn create_fork(workbook_id: WorkbookId) -> Result<ForkId> { /* ... */ }
fn delete_fork(fork_id: ForkId) -> Result<()> { /* ... */ }

let workbook = WorkbookId::new("wb-123".to_string())?;
let fork = create_fork(workbook.clone())?;

delete_fork(fork)?;           // ✓ OK
delete_fork(workbook)?;       // ✗ Compile error! Type mismatch
```

#### 1.3 CellAddress - Complex NewType

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CellAddress(String);

impl CellAddress {
    const MAX_COLUMN: u32 = 16384;  // Excel XFD
    const MAX_ROW: u32 = 1048576;   // Excel limit

    /// Parse from A1 notation (e.g., "B5", "AA42")
    pub fn parse(s: &str) -> Result<Self, ValidationError> {
        let split_idx = s
            .find(|c: char| c.is_ascii_digit())
            .ok_or_else(|| ValidationError::Invalid {
                field: "CellAddress",
                reason: "must contain row number",
            })?;

        let (col_str, row_str) = s.split_at(split_idx);

        let row = row_str.parse::<u32>()
            .map_err(|_| ValidationError::Invalid {
                field: "CellAddress",
                reason: "invalid row number",
            })?;

        if row == 0 || row > Self::MAX_ROW {
            return Err(ValidationError::OutOfRange {
                field: "CellAddress row",
                min: 1,
                max: Self::MAX_ROW,
                actual: row,
            });
        }

        let col = Self::column_from_letters(col_str)?;

        if col == 0 || col > Self::MAX_COLUMN {
            return Err(ValidationError::OutOfRange {
                field: "CellAddress column",
                min: 1,
                max: Self::MAX_COLUMN,
                actual: col,
            });
        }

        Ok(Self(s.to_uppercase()))
    }

    pub fn column(&self) -> u32 { /* ... */ }
    pub fn row(&self) -> u32 { /* ... */ }
}
```

**Benefits**:
- **Impossible to create invalid addresses**: "A0", "1A", "" all rejected at construction
- **Parse once, use everywhere**: No re-parsing needed
- **Type-safe HashMap keys**: `HashMap<CellAddress, Value>`

#### 1.4 RegionId - Preventing Index Confusion

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RegionId(u32);

impl RegionId {
    pub fn new(id: u32) -> Result<Self, ValidationError> {
        if id == 0 {
            return Err(ValidationError::Invalid {
                field: "RegionId",
                reason: "RegionId must be positive (> 0)",
            });
        }
        Ok(Self(id))
    }
}
```

**Prevents Index Confusion**:
```rust
fn process_region(region_id: RegionId, row: u32, col: u32) { /* ... */ }

let region = RegionId::new(5)?;
let row = 10u32;
let col = 3u32;

process_region(region, row, col);  // ✓ OK
process_region(row, region, col);  // ✗ Compile error!
```

### Best Practices for NewTypes

1. **Always validate on construction**
   ```rust
   pub fn new(value: T) -> Result<Self, ValidationError> {
       // Validate here
       Ok(Self(value))
   }
   ```

2. **Provide unsafe constructor for trusted sources**
   ```rust
   pub fn new_unchecked(value: T) -> Self {
       Self(value)
   }
   ```

3. **Use `#[serde(transparent)]` for JSON compatibility**
   ```rust
   #[derive(Serialize, Deserialize)]
   #[serde(transparent)]
   pub struct MyId(String);
   // Serializes as "abc", not {"0": "abc"}
   ```

4. **Implement common traits**
   ```rust
   impl Display for WorkbookId { /* ... */ }
   impl AsRef<str> for WorkbookId { /* ... */ }
   impl From<WorkbookId> for String { /* ... */ }
   ```

---

## 2. Type State Patterns

### Overview

Type state patterns encode state machines in the type system, making invalid state transitions impossible at compile time.

### Builder Pattern with Type States

```rust
// State types (zero-sized!)
struct New;
struct WithWorkbook;
struct WithSheet;
struct Ready;

struct QueryBuilder<State> {
    workbook_id: Option<WorkbookId>,
    sheet_name: Option<String>,
    query: Option<String>,
    _state: PhantomData<State>,
}

impl QueryBuilder<New> {
    fn new() -> Self {
        Self {
            workbook_id: None,
            sheet_name: None,
            query: None,
            _state: PhantomData,
        }
    }

    fn workbook(self, id: WorkbookId) -> QueryBuilder<WithWorkbook> {
        QueryBuilder {
            workbook_id: Some(id),
            sheet_name: self.sheet_name,
            query: self.query,
            _state: PhantomData,
        }
    }
}

impl QueryBuilder<WithWorkbook> {
    fn sheet(self, name: String) -> QueryBuilder<WithSheet> {
        QueryBuilder {
            workbook_id: self.workbook_id,
            sheet_name: Some(name),
            query: self.query,
            _state: PhantomData,
        }
    }
}

impl QueryBuilder<WithSheet> {
    fn query(self, q: String) -> QueryBuilder<Ready> {
        QueryBuilder {
            workbook_id: self.workbook_id,
            sheet_name: self.sheet_name,
            query: Some(q),
            _state: PhantomData,
        }
    }
}

impl QueryBuilder<Ready> {
    fn build(self) -> Query {
        Query {
            workbook_id: self.workbook_id.unwrap(),
            sheet_name: self.sheet_name.unwrap(),
            query: self.query.unwrap(),
        }
    }
}
```

**Usage**:
```rust
let query = QueryBuilder::new()
    .workbook(workbook_id)
    .sheet("Sheet1".to_string())
    .query("SELECT * FROM A1:D10".to_string())
    .build();  // ✓ OK

// This won't compile:
// let bad = QueryBuilder::new().build();  // ✗ No build() method on QueryBuilder<New>
```

### Connection State Machine

```rust
struct Disconnected;
struct Connected;
struct InTransaction;

struct Database<State> {
    connection: Option<Connection>,
    _state: PhantomData<State>,
}

impl Database<Disconnected> {
    fn connect(self) -> Result<Database<Connected>> {
        let conn = establish_connection()?;
        Ok(Database {
            connection: Some(conn),
            _state: PhantomData,
        })
    }
}

impl Database<Connected> {
    fn begin_transaction(self) -> Result<Database<InTransaction>> {
        self.connection.as_ref().unwrap().begin()?;
        Ok(Database {
            connection: self.connection,
            _state: PhantomData,
        })
    }

    fn disconnect(self) -> Database<Disconnected> {
        drop(self.connection);
        Database {
            connection: None,
            _state: PhantomData,
        }
    }
}

impl Database<InTransaction> {
    fn commit(self) -> Result<Database<Connected>> {
        self.connection.as_ref().unwrap().commit()?;
        Ok(Database {
            connection: self.connection,
            _state: PhantomData,
        })
    }

    fn rollback(self) -> Result<Database<Connected>> {
        self.connection.as_ref().unwrap().rollback()?;
        Ok(Database {
            connection: self.connection,
            _state: PhantomData,
        })
    }
}
```

**Benefits**:
- Cannot call `commit()` without `begin_transaction()`
- Cannot `begin_transaction()` while already in transaction
- Zero runtime cost (states are zero-sized)

---

## 3. Trait-Based Design

### 3.1 Tool Handler Trait

```rust
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// The parameter type for this tool
    type Params: DeserializeOwned + JsonSchema + Send;

    /// The response type for this tool
    type Response: Serialize + Send;

    /// Execute the tool with validated parameters
    async fn execute(
        &self,
        state: Arc<AppState>,
        params: Self::Params,
    ) -> Result<Self::Response>;

    /// Tool name for registration
    fn name(&self) -> &'static str;

    /// Tool description
    fn description(&self) -> &'static str;
}
```

**Implementation Example**:
```rust
pub struct ListSheetsHandler;

#[async_trait]
impl ToolHandler for ListSheetsHandler {
    type Params = ListSheetsParams;
    type Response = SheetListResponse;

    async fn execute(
        &self,
        state: Arc<AppState>,
        params: Self::Params,
    ) -> Result<Self::Response> {
        let workbook = state.open_workbook(&params.workbook_or_fork_id).await?;
        let summaries = workbook.list_summaries()?;

        Ok(SheetListResponse {
            workbook_id: workbook.id.clone(),
            workbook_short_id: workbook.short_id.clone(),
            sheets: summaries,
        })
    }

    fn name(&self) -> &'static str {
        "list_sheets"
    }

    fn description(&self) -> &'static str {
        "List all sheets in a workbook"
    }
}
```

### 3.2 Recalc Backend Trait

**Location**: `src/recalc/backend.rs`

```rust
#[async_trait]
pub trait RecalcBackend: Send + Sync {
    async fn recalculate(&self, fork_work_path: &Path) -> Result<RecalcResult>;
    fn is_available(&self) -> bool;
    fn name(&self) -> &'static str;
}
```

**Multiple Implementations**:
```rust
pub struct LibreOfficeBackend {
    executor: Arc<dyn RecalcExecutor>,
}

#[async_trait]
impl RecalcBackend for LibreOfficeBackend {
    async fn recalculate(&self, fork_work_path: &Path) -> Result<RecalcResult> {
        self.executor.recalculate(fork_work_path).await
    }

    fn is_available(&self) -> bool {
        self.executor.is_available()
    }

    fn name(&self) -> &'static str {
        "libreoffice"
    }
}

pub struct ExcelBackend { /* ... */ }

#[async_trait]
impl RecalcBackend for ExcelBackend {
    // Different implementation
}
```

### 3.3 Extension Traits

Extension traits add methods to existing types:

```rust
pub trait ValidateParams {
    fn validate_params(
        &self,
        tool_name: &str,
        params: &Value,
    ) -> Result<ValidationResult>;
}

impl ValidateParams for SchemaValidator {
    fn validate_params(
        &self,
        tool_name: &str,
        params: &Value,
    ) -> Result<ValidationResult> {
        match self.validate(tool_name, params) {
            Ok(()) => Ok(ValidationResult::success(tool_name.to_string())),
            Err(SchemaValidationError::ValidationFailed { errors, .. }) => {
                Ok(ValidationResult::failure(tool_name.to_string(), errors))
            }
            Err(e) => Err(anyhow!(e)),
        }
    }
}
```

### 3.4 Recoverable Trait

**Location**: `src/recovery/mod.rs`

```rust
pub trait Recoverable<T> {
    fn execute(&self) -> Result<T>;
    fn operation_name(&self) -> &str;
    fn max_retries(&self) -> u32 {
        3  // Default implementation
    }
}
```

---

## 4. Generic Programming

### 4.1 Generic Functions with Constraints

```rust
pub fn validate_and_deserialize<T>(
    validator: &SchemaValidator,
    tool_name: &str,
    params: Value,
) -> Result<T>
where
    T: DeserializeOwned + JsonSchema,
{
    validator.validate(tool_name, &params)?;
    let result: T = serde_json::from_value(params)?;
    Ok(result)
}
```

**Benefits**:
- Works with any type implementing the constraints
- Type-safe: Compiler ensures constraints are met
- No runtime overhead: Monomorphized at compile time

### 4.2 Associated Types vs Type Parameters

**Type Parameters** - When you want flexibility:
```rust
trait Container<T> {
    fn insert(&mut self, item: T);
    fn get(&self, index: usize) -> Option<&T>;
}

// Can implement Container<String> AND Container<i32> for the same type
impl Container<String> for MyVec { /* ... */ }
impl Container<i32> for MyVec { /* ... */ }
```

**Associated Types** - When there's a single logical type:
```rust
trait ToolHandler {
    type Params;   // Only one Params type per implementation
    type Response; // Only one Response type per implementation

    async fn execute(&self, params: Self::Params) -> Result<Self::Response>;
}
```

### 4.3 Where Clauses

```rust
// Complex where clause example
pub async fn execute_with_recovery<T, F, Fut>(
    operation_name: &str,
    operation: F,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    // Implementation
}
```

**When to use where clauses**:
- Multiple trait bounds: `where T: Clone + Debug + Serialize`
- Bounds on associated types: `where T::Item: Display`
- Higher-rank trait bounds: `where F: for<'a> Fn(&'a str) -> &'a str`

### 4.4 Generic Error Handling

```rust
pub struct GracefulDegradation<T> {
    primary: Box<dyn Fn() -> Result<T> + Send + Sync>,
    fallback: Option<Box<dyn Fn() -> Result<T> + Send + Sync>>,
    operation_name: String,
}

impl<T> GracefulDegradation<T> {
    pub fn new(operation_name: impl Into<String>) -> Self {
        Self {
            primary: Box::new(|| Err(anyhow!("no primary operation set"))),
            fallback: None,
            operation_name: operation_name.into(),
        }
    }

    pub fn primary<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Result<T> + Send + Sync + 'static,
    {
        self.primary = Box::new(f);
        self
    }

    pub fn fallback<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Result<T> + Send + Sync + 'static,
    {
        self.fallback = Some(Box::new(f));
        self
    }

    pub fn execute(self) -> Result<T> {
        match (self.primary)() {
            Ok(result) => Ok(result),
            Err(primary_err) => {
                match self.fallback {
                    Some(fallback_fn) => fallback_fn(),
                    None => Err(primary_err),
                }
            }
        }
    }
}
```

---

## 5. Zero-Cost Abstractions

### Principle

Rust's zero-cost abstractions mean you pay no runtime cost for type safety:

```rust
// No runtime overhead despite type safety
let workbook_id = WorkbookId(String::from("wb-123"));
let raw_string = String::from("wb-123");

// Both compile to the same machine code!
```

### 5.1 Monomorphization

Generic functions are monomorphized at compile time:

```rust
fn process<T: Display>(value: T) {
    println!("{}", value);
}

// Usage:
process(42);
process("hello");

// Compiler generates:
// fn process_i32(value: i32) { println!("{}", value); }
// fn process_str(value: &str) { println!("{}", value); }
```

**Trade-off**: Increased binary size for zero runtime cost.

### 5.2 Inline Optimization

```rust
#[inline]
pub fn as_str(&self) -> &str {
    &self.0
}

#[inline(always)]
pub fn value(self) -> u32 {
    self.0
}
```

**Usage**:
- `#[inline]`: Suggests inlining to compiler
- `#[inline(always)]`: Forces inlining
- Use for small, frequently-called methods

### 5.3 Static Dispatch

```rust
// Static dispatch (zero cost)
fn handle<T: ToolHandler>(handler: &T, params: T::Params) {
    handler.execute(params)
}

// Dynamic dispatch (small runtime cost)
fn handle_dyn(handler: &dyn ToolHandler, params: ???) {
    handler.execute(params)
}
```

**Prefer static dispatch when**:
- Type is known at compile time
- Performance is critical
- Binary size is acceptable

**Use dynamic dispatch when**:
- Need heterogeneous collections: `Vec<Box<dyn Trait>>`
- Plugin systems
- Binary size is a concern

### 5.4 Const Generics

```rust
struct Buffer<const N: usize> {
    data: [u8; N],
}

impl<const N: usize> Buffer<N> {
    fn new() -> Self {
        Self {
            data: [0; N],
        }
    }
}

// Different types at compile time!
let small: Buffer<64> = Buffer::new();
let large: Buffer<4096> = Buffer::new();
```

**Benefits**:
- Array sizes in types
- Compile-time bounds checking
- Zero runtime overhead

---

## 6. Type-Level Validation

### 6.1 Phantom Types

Phantom types exist only at compile time:

```rust
use std::marker::PhantomData;

struct Validated;
struct Unvalidated;

struct Data<State> {
    value: String,
    _state: PhantomData<State>,
}

impl Data<Unvalidated> {
    fn new(value: String) -> Self {
        Self {
            value,
            _state: PhantomData,
        }
    }

    fn validate(self) -> Result<Data<Validated>, ValidationError> {
        if self.value.is_empty() {
            return Err(ValidationError::Empty("value"));
        }
        Ok(Data {
            value: self.value,
            _state: PhantomData,
        })
    }
}

impl Data<Validated> {
    fn process(&self) {
        // Can only call on validated data!
    }
}

// Usage:
let data = Data::new("test".to_string());
// data.process();  // ✗ Compile error!

let validated = data.validate()?;
validated.process();  // ✓ OK
```

### 6.2 Encoding Constraints in Types

```rust
struct Positive<T>(T);
struct NonZero<T>(T);
struct InRange<T, const MIN: i32, const MAX: i32>(T);

impl Positive<i32> {
    fn new(value: i32) -> Option<Self> {
        if value > 0 {
            Some(Self(value))
        } else {
            None
        }
    }
}

impl NonZero<u32> {
    fn new(value: u32) -> Option<Self> {
        if value != 0 {
            Some(Self(value))
        } else {
            None
        }
    }
}

// Usage prevents invalid operations:
fn divide(numerator: i32, denominator: NonZero<i32>) -> i32 {
    numerator / denominator.0  // Safe! Cannot be zero
}
```

### 6.3 Type-Level State Machines

```rust
struct Request<State> {
    url: String,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
    _state: PhantomData<State>,
}

struct Initial;
struct WithAuth;
struct WithBody;
struct Ready;

impl Request<Initial> {
    fn new(url: String) -> Self {
        Self {
            url,
            headers: HashMap::new(),
            body: None,
            _state: PhantomData,
        }
    }

    fn with_auth(mut self, token: String) -> Request<WithAuth> {
        self.headers.insert("Authorization".to_string(), token);
        Request {
            url: self.url,
            headers: self.headers,
            body: self.body,
            _state: PhantomData,
        }
    }
}

impl Request<WithAuth> {
    fn with_body(mut self, body: Vec<u8>) -> Request<Ready> {
        Request {
            url: self.url,
            headers: self.headers,
            body: Some(body),
            _state: PhantomData,
        }
    }

    fn send(self) -> Result<Response> {
        // Can send without body
    }
}

impl Request<Ready> {
    fn send(self) -> Result<Response> {
        // Can send with body
    }
}
```

---

## 7. Serde Integration

### 7.1 Type-Safe Serialization

```rust
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ToolParams {
    pub workbook_id: WorkbookId,  // NewType!
    pub sheet_name: String,
    #[serde(default)]
    pub limit: Option<u32>,
}
```

**JSON**:
```json
{
  "workbook_id": "wb-123",
  "sheet_name": "Sheet1",
  "limit": 100
}
```

### 7.2 Transparent NewTypes

```rust
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkbookId(String);

// Serializes as: "wb-123"
// Not as: {"0": "wb-123"}
```

### 7.3 Custom Validation on Deserialization

```rust
#[derive(Deserialize)]
struct Params {
    #[serde(deserialize_with = "deserialize_workbook_id")]
    workbook_id: WorkbookId,
}

fn deserialize_workbook_id<'de, D>(deserializer: D) -> Result<WorkbookId, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    WorkbookId::new(s).map_err(serde::de::Error::custom)
}
```

### 7.4 Schema Generation

```rust
use schemars::JsonSchema;

#[derive(JsonSchema)]
pub struct ListSheetsParams {
    /// The workbook or fork identifier
    #[schemars(description = "Workbook or fork ID to list sheets from")]
    pub workbook_or_fork_id: WorkbookId,
}
```

**Generated JSON Schema**:
```json
{
  "type": "object",
  "properties": {
    "workbook_or_fork_id": {
      "type": "string",
      "description": "Workbook or fork ID to list sheets from"
    }
  },
  "required": ["workbook_or_fork_id"]
}
```

### 7.5 Validation Errors

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ValidationError {
    Empty(&'static str),
    TooLong {
        field: &'static str,
        max: usize,
        actual: usize,
    },
    InvalidCharacter {
        field: &'static str,
        character: char,
    },
    Invalid {
        field: &'static str,
        reason: &'static str,
    },
    OutOfRange {
        field: &'static str,
        min: u32,
        max: u32,
        actual: u32,
    },
}
```

---

## 8. TPS Poka-Yoke Principles

### Type Safety as Error-Proofing

The Toyota Production System's poka-yoke (error-proofing) principle translates directly to type-driven development:

> **Poka-Yoke**: Design systems so that errors are impossible or immediately detected.

### 8.1 Type-Level Poka-Yoke

```rust
// ❌ Without NewTypes - Runtime error possible
fn delete_resource(id: String, resource_type: String) -> Result<()> {
    if resource_type == "fork" {
        delete_fork(id)
    } else {
        delete_workbook(id)
    }
}

// ✅ With NewTypes - Compile-time prevention
fn delete_fork(fork_id: ForkId) -> Result<()> { /* ... */ }
fn delete_workbook(workbook_id: WorkbookId) -> Result<()> { /* ... */ }

// Cannot mix up!
```

### 8.2 Validation as Type Boundary

**Validation Layers** (from `src/validation/`):

1. **Schema Validation**: JSON structure
2. **NewType Validation**: Domain constraints
3. **Bounds Validation**: Numeric ranges
4. **Business Logic Validation**: Complex rules

```rust
// Type boundaries enforce validation
pub async fn list_sheets(
    state: Arc<AppState>,
    params: ListSheetsParams,  // Already validated!
) -> Result<SheetListResponse> {
    // params.workbook_or_fork_id is guaranteed valid
    let workbook = state.open_workbook(&params.workbook_or_fork_id).await?;
    // ...
}
```

### 8.3 Impossible States

```rust
// ❌ Bad: Invalid states possible
struct Connection {
    connected: bool,
    in_transaction: bool,
}

// Can have: connected=false, in_transaction=true (?!)

// ✅ Good: Invalid states impossible
enum ConnectionState {
    Disconnected,
    Connected,
    InTransaction,
}

struct Connection {
    state: ConnectionState,
}
```

### 8.4 Fail Fast with Types

```rust
// Validation happens once, at construction
let workbook_id = WorkbookId::new(input)?;  // Fails here if invalid

// Everywhere else, just use it - no need to re-validate
process_workbook(workbook_id.clone())?;
log_access(workbook_id.clone())?;
cache_result(workbook_id)?;
```

---

## Summary: Type-Driven MCP Development Checklist

### ✅ NewTypes
- [ ] Wrap all domain primitives (IDs, names, addresses)
- [ ] Validate on construction
- [ ] Use `#[serde(transparent)]` for JSON
- [ ] Implement Display, AsRef, From traits

### ✅ Type States
- [ ] Encode state machines in types
- [ ] Use PhantomData for zero-cost states
- [ ] Builder pattern with type progression
- [ ] Prevent invalid state transitions

### ✅ Traits
- [ ] Define handler traits with associated types
- [ ] Use extension traits for existing types
- [ ] Prefer static dispatch when possible
- [ ] Document trait requirements clearly

### ✅ Generics
- [ ] Use where clauses for complex bounds
- [ ] Prefer associated types for single logical types
- [ ] Use type parameters for flexibility
- [ ] Leverage const generics for compile-time sizes

### ✅ Zero-Cost
- [ ] Inline frequently-called methods
- [ ] Rely on monomorphization
- [ ] Use static dispatch by default
- [ ] PhantomData for compile-time state

### ✅ Validation
- [ ] Validate at type boundaries
- [ ] Encode constraints in types
- [ ] Use phantom types for validation states
- [ ] Generate schemas from types

### ✅ Serde
- [ ] Derive JsonSchema for all params
- [ ] Custom deserializers for complex validation
- [ ] Tagged enums for sum types
- [ ] Transparent wrappers for NewTypes

### ✅ Poka-Yoke
- [ ] Make illegal states unrepresentable
- [ ] Fail fast at boundaries
- [ ] Type safety prevents errors
- [ ] Compiler enforces invariants

---

## References

- **ggen-mcp Source**: `/home/user/ggen-mcp/src/`
  - `domain/value_objects.rs` - NewType implementations
  - `recovery/mod.rs` - Generic error handling
  - `recalc/backend.rs` - Trait design
  - `validation/middleware.rs` - Type-safe validation
  - `tools/mod.rs` - MCP tool handlers

- **Related Documentation**:
  - `TPS_FOR_MCP_SERVERS.md` - Poka-yoke patterns
  - `TPS_JIDOKA.md` - Error detection principles
  - `TPS_STANDARDIZED_WORK.md` - Consistent patterns

- **Examples**:
  - `examples/newtype_integration.rs` - NewType usage
  - `examples/type_driven_mcp.rs` - Advanced patterns
  - `examples/validation_example.rs` - Type-safe validation

---

**Last Updated**: 2026-01-20
**Author**: Claude (Anthropic)
**Project**: ggen-mcp
