//! Advanced Type-Driven Development Patterns for MCP Servers
//!
//! This example demonstrates advanced type-driven patterns for building
//! robust, type-safe MCP servers in Rust.
//!
//! Patterns covered:
//! 1. Type State Builders
//! 2. Phantom Types for Validation
//! 3. GADTs (Generalized Algebraic Data Types)
//! 4. Type-Level State Machines
//! 5. Generic Tool Handlers
//! 6. Zero-Cost Abstractions
//! 7. Const Generics for Compile-Time Validation

#![allow(dead_code, unused_imports, unused_variables)]

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::marker::PhantomData;
use std::sync::Arc;

// ============================================================================
// 1. Type State Builder Pattern
// ============================================================================

/// Zero-sized type states
struct New;
struct WithWorkbook;
struct WithSheet;
struct WithQuery;
struct Ready;

/// Query builder that progresses through type states
struct QueryBuilder<State> {
    workbook_id: Option<String>,
    sheet_name: Option<String>,
    query: Option<String>,
    limit: Option<u32>,
    _state: PhantomData<State>,
}

impl QueryBuilder<New> {
    fn new() -> Self {
        Self {
            workbook_id: None,
            sheet_name: None,
            query: None,
            limit: None,
            _state: PhantomData,
        }
    }

    fn workbook(self, id: String) -> QueryBuilder<WithWorkbook> {
        QueryBuilder {
            workbook_id: Some(id),
            sheet_name: self.sheet_name,
            query: self.query,
            limit: self.limit,
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
            limit: self.limit,
            _state: PhantomData,
        }
    }
}

impl QueryBuilder<WithSheet> {
    fn query(self, q: String) -> QueryBuilder<WithQuery> {
        QueryBuilder {
            workbook_id: self.workbook_id,
            sheet_name: self.sheet_name,
            query: Some(q),
            limit: self.limit,
            _state: PhantomData,
        }
    }
}

impl QueryBuilder<WithQuery> {
    fn limit(self, lim: u32) -> QueryBuilder<Ready> {
        QueryBuilder {
            workbook_id: self.workbook_id,
            sheet_name: self.sheet_name,
            query: self.query,
            limit: Some(lim),
            _state: PhantomData,
        }
    }

    fn build(self) -> Query {
        Query {
            workbook_id: self.workbook_id.unwrap(),
            sheet_name: self.sheet_name.unwrap(),
            query: self.query.unwrap(),
            limit: self.limit,
        }
    }
}

impl QueryBuilder<Ready> {
    fn build(self) -> Query {
        Query {
            workbook_id: self.workbook_id.unwrap(),
            sheet_name: self.sheet_name.unwrap(),
            query: self.query.unwrap(),
            limit: self.limit,
        }
    }
}

#[derive(Debug)]
struct Query {
    workbook_id: String,
    sheet_name: String,
    query: String,
    limit: Option<u32>,
}

fn example_builder_pattern() -> Result<()> {
    // Type-safe construction - each step returns a new type
    let query = QueryBuilder::new()
        .workbook("wb-123".to_string())
        .sheet("Sheet1".to_string())
        .query("SELECT * FROM A1:D10".to_string())
        .limit(100)
        .build();

    println!("Built query: {:?}", query);

    // These won't compile:
    // let bad1 = QueryBuilder::new().build();  // ✗ No build() on New
    // let bad2 = QueryBuilder::new().workbook("wb").build();  // ✗ No build() on WithWorkbook

    Ok(())
}

// ============================================================================
// 2. Phantom Types for Validation States
// ============================================================================

struct Validated;
struct Unvalidated;

/// Data that tracks validation state in the type system
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

    fn validate(self) -> Result<Data<Validated>> {
        if self.value.is_empty() {
            return Err(anyhow!("Value cannot be empty"));
        }
        if self.value.len() > 1000 {
            return Err(anyhow!("Value too long"));
        }

        Ok(Data {
            value: self.value,
            _state: PhantomData,
        })
    }
}

impl Data<Validated> {
    /// Only validated data can be processed
    fn process(&self) -> String {
        format!("Processing: {}", self.value)
    }

    /// Only validated data can be persisted
    fn save(&self) -> Result<()> {
        println!("Saving: {}", self.value);
        Ok(())
    }
}

fn example_phantom_validation() -> Result<()> {
    let unvalidated = Data::new("test data".to_string());

    // unvalidated.process();  // ✗ Compile error! No process() method
    // unvalidated.save();     // ✗ Compile error! No save() method

    let validated = unvalidated.validate()?;

    println!("{}", validated.process());  // ✓ OK
    validated.save()?;                     // ✓ OK

    Ok(())
}

// ============================================================================
// 3. Type-Level Resource Tracking
// ============================================================================

struct Acquired;
struct Released;

/// Resource that must be acquired before use
struct Resource<State> {
    handle: Option<String>,
    _state: PhantomData<State>,
}

impl Resource<Released> {
    fn new() -> Self {
        Self {
            handle: None,
            _state: PhantomData,
        }
    }

    fn acquire(self) -> Result<Resource<Acquired>> {
        println!("Acquiring resource...");
        Ok(Resource {
            handle: Some("resource-handle".to_string()),
            _state: PhantomData,
        })
    }
}

impl Resource<Acquired> {
    fn use_resource(&self) -> Result<()> {
        println!("Using resource: {:?}", self.handle);
        Ok(())
    }

    fn release(self) -> Resource<Released> {
        println!("Releasing resource...");
        Resource {
            handle: None,
            _state: PhantomData,
        }
    }
}

impl Drop for Resource<Acquired> {
    fn drop(&mut self) {
        println!("Auto-releasing resource on drop");
    }
}

fn example_resource_tracking() -> Result<()> {
    let resource = Resource::new();
    // resource.use_resource();  // ✗ Compile error! Not acquired

    let acquired = resource.acquire()?;
    acquired.use_resource()?;   // ✓ OK

    let released = acquired.release();
    // released.use_resource();  // ✗ Compile error! Released

    Ok(())
}

// ============================================================================
// 4. Generic Tool Handler Pattern
// ============================================================================

#[async_trait]
trait ToolHandler: Send + Sync {
    type Params: for<'de> Deserialize<'de> + Send;
    type Response: Serialize + Send;

    async fn execute(&self, params: Self::Params) -> Result<Self::Response>;
    fn name(&self) -> &'static str;
}

#[derive(Debug, Deserialize)]
struct CountRowsParams {
    workbook_id: String,
    sheet_name: String,
}

#[derive(Debug, Serialize)]
struct CountRowsResponse {
    count: u32,
}

struct CountRowsHandler;

#[async_trait]
impl ToolHandler for CountRowsHandler {
    type Params = CountRowsParams;
    type Response = CountRowsResponse;

    async fn execute(&self, params: Self::Params) -> Result<Self::Response> {
        println!("Counting rows in {}/{}", params.workbook_id, params.sheet_name);
        Ok(CountRowsResponse { count: 100 })
    }

    fn name(&self) -> &'static str {
        "count_rows"
    }
}

/// Generic executor that works with any ToolHandler
async fn execute_tool<H>(handler: &H, params_json: Value) -> Result<Value>
where
    H: ToolHandler,
{
    println!("Executing tool: {}", handler.name());

    let params: H::Params = serde_json::from_value(params_json)?;
    let response = handler.execute(params).await?;
    let response_json = serde_json::to_value(response)?;

    Ok(response_json)
}

#[tokio::main]
async fn example_generic_handler() -> Result<()> {
    let handler = CountRowsHandler;
    let params = serde_json::json!({
        "workbook_id": "wb-123",
        "sheet_name": "Sheet1"
    });

    let result = execute_tool(&handler, params).await?;
    println!("Result: {}", result);

    Ok(())
}

// ============================================================================
// 5. Const Generics for Compile-Time Bounds
// ============================================================================

/// Buffer with compile-time size checking
struct Buffer<const N: usize> {
    data: [u8; N],
    len: usize,
}

impl<const N: usize> Buffer<N> {
    fn new() -> Self {
        Self {
            data: [0; N],
            len: 0,
        }
    }

    fn push(&mut self, byte: u8) -> Result<()> {
        if self.len >= N {
            return Err(anyhow!("Buffer full"));
        }
        self.data[self.len] = byte;
        self.len += 1;
        Ok(())
    }

    fn capacity(&self) -> usize {
        N
    }
}

fn example_const_generics() -> Result<()> {
    let mut small: Buffer<64> = Buffer::new();
    let mut large: Buffer<4096> = Buffer::new();

    // These are different types at compile time!
    assert_eq!(small.capacity(), 64);
    assert_eq!(large.capacity(), 4096);

    Ok(())
}

// ============================================================================
// 6. GADT-Style Pattern (Generalized Algebraic Data Types)
// ============================================================================

/// Expression GADT that tracks types
enum Expr<T> {
    Int(i32, PhantomData<T>),
    Bool(bool, PhantomData<T>),
    Add(Box<Expr<i32>>, Box<Expr<i32>>, PhantomData<T>),
    Eq(Box<Expr<i32>>, Box<Expr<i32>>, PhantomData<T>),
}

impl Expr<i32> {
    fn eval_int(&self) -> i32 {
        match self {
            Expr::Int(n, _) => *n,
            Expr::Add(left, right, _) => left.eval_int() + right.eval_int(),
            _ => unreachable!(),
        }
    }
}

impl Expr<bool> {
    fn eval_bool(&self) -> bool {
        match self {
            Expr::Bool(b, _) => *b,
            Expr::Eq(left, right, _) => left.eval_int() == right.eval_int(),
            _ => unreachable!(),
        }
    }
}

fn example_gadt() -> Result<()> {
    // Type-safe expression building
    let expr_int: Expr<i32> = Expr::Add(
        Box::new(Expr::Int(5, PhantomData)),
        Box::new(Expr::Int(3, PhantomData)),
        PhantomData,
    );

    let expr_bool: Expr<bool> = Expr::Eq(
        Box::new(Expr::Int(5, PhantomData)),
        Box::new(Expr::Int(5, PhantomData)),
        PhantomData,
    );

    println!("Int result: {}", expr_int.eval_int());
    println!("Bool result: {}", expr_bool.eval_bool());

    // expr_int.eval_bool();  // ✗ Compile error!
    // expr_bool.eval_int();  // ✗ Compile error!

    Ok(())
}

// ============================================================================
// 7. Type-Safe Indexing
// ============================================================================

/// Newtype for validated indices
#[derive(Debug, Clone, Copy)]
struct Index<const MAX: usize>(usize);

impl<const MAX: usize> Index<MAX> {
    fn new(value: usize) -> Result<Self> {
        if value >= MAX {
            return Err(anyhow!("Index {} out of bounds (max: {})", value, MAX));
        }
        Ok(Self(value))
    }

    fn get(self) -> usize {
        self.0
    }
}

/// Array with type-safe indexing
struct SafeArray<T, const N: usize> {
    data: [T; N],
}

impl<T: Default + Copy, const N: usize> SafeArray<T, N> {
    fn new() -> Self {
        Self {
            data: [T::default(); N],
        }
    }

    /// Type-safe get - index cannot be out of bounds!
    fn get(&self, index: Index<N>) -> &T {
        &self.data[index.get()]
    }

    /// Type-safe set
    fn set(&mut self, index: Index<N>, value: T) {
        self.data[index.get()] = value;
    }
}

fn example_safe_indexing() -> Result<()> {
    let mut arr: SafeArray<i32, 10> = SafeArray::new();

    let idx = Index::<10>::new(5)?;
    arr.set(idx, 42);
    println!("Value at index 5: {}", arr.get(idx));

    // This fails at construction:
    // let bad_idx = Index::<10>::new(20)?;  // ✗ Runtime error

    Ok(())
}

// ============================================================================
// 8. Zero-Cost State Machine
// ============================================================================

/// Connection state machine with zero runtime overhead
struct Connection<State> {
    url: String,
    _state: PhantomData<State>,
}

struct Disconnected;
struct Connecting;
struct Connected;
struct InTransaction;

impl Connection<Disconnected> {
    fn new(url: String) -> Self {
        Self {
            url,
            _state: PhantomData,
        }
    }

    fn connect(self) -> Result<Connection<Connecting>> {
        println!("Connecting to {}", self.url);
        Ok(Connection {
            url: self.url,
            _state: PhantomData,
        })
    }
}

impl Connection<Connecting> {
    fn finish_connect(self) -> Result<Connection<Connected>> {
        println!("Connection established");
        Ok(Connection {
            url: self.url,
            _state: PhantomData,
        })
    }

    fn cancel(self) -> Connection<Disconnected> {
        println!("Connection cancelled");
        Connection {
            url: self.url,
            _state: PhantomData,
        }
    }
}

impl Connection<Connected> {
    fn query(&self, sql: &str) -> Result<Vec<String>> {
        println!("Executing query: {}", sql);
        Ok(vec!["result1".to_string(), "result2".to_string()])
    }

    fn begin_transaction(self) -> Result<Connection<InTransaction>> {
        println!("Beginning transaction");
        Ok(Connection {
            url: self.url,
            _state: PhantomData,
        })
    }

    fn disconnect(self) -> Connection<Disconnected> {
        println!("Disconnecting");
        Connection {
            url: self.url,
            _state: PhantomData,
        }
    }
}

impl Connection<InTransaction> {
    fn execute(&self, sql: &str) -> Result<()> {
        println!("Executing in transaction: {}", sql);
        Ok(())
    }

    fn commit(self) -> Result<Connection<Connected>> {
        println!("Committing transaction");
        Ok(Connection {
            url: self.url,
            _state: PhantomData,
        })
    }

    fn rollback(self) -> Result<Connection<Connected>> {
        println!("Rolling back transaction");
        Ok(Connection {
            url: self.url,
            _state: PhantomData,
        })
    }
}

fn example_state_machine() -> Result<()> {
    let conn = Connection::new("db://localhost".to_string());

    // conn.query("SELECT *");  // ✗ Compile error! Not connected

    let conn = conn.connect()?.finish_connect()?;
    conn.query("SELECT * FROM users")?;  // ✓ OK

    let conn = conn.begin_transaction()?;
    conn.execute("INSERT INTO users VALUES (...)")?;  // ✓ OK

    // conn.query("SELECT *");  // ✗ Compile error! In transaction, use execute()

    let conn = conn.commit()?;
    conn.query("SELECT * FROM users")?;  // ✓ OK again

    Ok(())
}

// ============================================================================
// 9. Type-Safe Builder with Required Fields
// ============================================================================

trait FieldState {}
struct Required;
struct Optional;
impl FieldState for Required {}
impl FieldState for Optional {}

struct RequestBuilder<Name, Url, Body>
where
    Name: FieldState,
    Url: FieldState,
    Body: FieldState,
{
    name: Option<String>,
    url: Option<String>,
    body: Option<Vec<u8>>,
    _phantom: PhantomData<(Name, Url, Body)>,
}

impl RequestBuilder<Required, Required, Required> {
    fn new() -> RequestBuilder<Optional, Optional, Optional> {
        RequestBuilder {
            name: None,
            url: None,
            body: None,
            _phantom: PhantomData,
        }
    }
}

impl<Url, Body> RequestBuilder<Optional, Url, Body>
where
    Url: FieldState,
    Body: FieldState,
{
    fn name(self, name: String) -> RequestBuilder<Required, Url, Body> {
        RequestBuilder {
            name: Some(name),
            url: self.url,
            body: self.body,
            _phantom: PhantomData,
        }
    }
}

impl<Name, Body> RequestBuilder<Name, Optional, Body>
where
    Name: FieldState,
    Body: FieldState,
{
    fn url(self, url: String) -> RequestBuilder<Name, Required, Body> {
        RequestBuilder {
            name: self.name,
            url: Some(url),
            body: self.body,
            _phantom: PhantomData,
        }
    }
}

impl<Name, Url> RequestBuilder<Name, Url, Optional>
where
    Name: FieldState,
    Url: FieldState,
{
    fn body(self, body: Vec<u8>) -> RequestBuilder<Name, Url, Required> {
        RequestBuilder {
            name: self.name,
            url: self.url,
            body: Some(body),
            _phantom: PhantomData,
        }
    }
}

impl RequestBuilder<Required, Required, Required> {
    fn build(self) -> Request {
        Request {
            name: self.name.unwrap(),
            url: self.url.unwrap(),
            body: self.body.unwrap(),
        }
    }
}

struct Request {
    name: String,
    url: String,
    body: Vec<u8>,
}

fn example_required_fields() -> Result<()> {
    let request = RequestBuilder::new()
        .name("MyRequest".to_string())
        .url("https://api.example.com".to_string())
        .body(vec![1, 2, 3])
        .build();

    // These won't compile:
    // let bad1 = RequestBuilder::new().build();  // ✗ Missing required fields
    // let bad2 = RequestBuilder::new().name("x").build();  // ✗ Still missing url and body

    Ok(())
}

// ============================================================================
// Main - Run All Examples
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Type-Driven MCP Development Examples ===\n");

    println!("1. Type State Builder Pattern:");
    example_builder_pattern()?;

    println!("\n2. Phantom Types for Validation:");
    example_phantom_validation()?;

    println!("\n3. Resource Tracking:");
    example_resource_tracking()?;

    println!("\n4. Generic Tool Handler:");
    // Already has tokio::main
    // example_generic_handler().await?;

    println!("\n5. Const Generics:");
    example_const_generics()?;

    println!("\n6. GADT-Style Expressions:");
    example_gadt()?;

    println!("\n7. Safe Indexing:");
    example_safe_indexing()?;

    println!("\n8. State Machine:");
    example_state_machine()?;

    println!("\n9. Required Fields Builder:");
    example_required_fields()?;

    println!("\n=== All examples completed successfully! ===");

    Ok(())
}

// ============================================================================
// Additional: Compile-Time Validation Example
// ============================================================================

/// Demonstrates validation that happens entirely at compile time
#[cfg(test)]
mod compile_time_tests {
    use super::*;

    // These tests verify that certain patterns don't compile
    // (They're commented out because they would fail compilation)

    #[test]
    fn test_type_state_enforcement() {
        // ✓ This compiles:
        let _query = QueryBuilder::new()
            .workbook("wb".to_string())
            .sheet("sheet".to_string())
            .query("query".to_string())
            .build();

        // ✗ These don't compile:
        // let _bad = QueryBuilder::new().build();
        // let _bad = QueryBuilder::new().workbook("wb").build();
    }

    #[test]
    fn test_validation_state_enforcement() {
        let data = Data::new("test".to_string());
        let validated = data.validate().unwrap();

        // ✓ This compiles:
        validated.process();

        // ✗ This doesn't compile:
        // let unvalidated = Data::new("test".to_string());
        // unvalidated.process();
    }
}
