# SPARQL Result Validation and Type-Safe Bindings

## Overview

This document describes the comprehensive SPARQL query result validation and type-safe binding system implemented for ggen-mcp. The system follows **Toyota Production System poka-yoke** (error-proofing) principles to prevent errors at the query result boundary.

## Table of Contents

1. [Architecture](#architecture)
2. [ResultSetValidator](#resultsetvalidator)
3. [TypedBinding](#typedbinding)
4. [ResultMapper](#resultmapper)
5. [GraphValidator](#graphvalidator)
6. [QueryResultCache](#queryresultcache)
7. [Type-Safe Query Wrappers](#type-safe-query-wrappers)
8. [Error Handling](#error-handling)
9. [Performance Optimization](#performance-optimization)
10. [Best Practices](#best-practices)

## Architecture

The SPARQL result validation system consists of five main components:

```
┌─────────────────────────────────────────────────────────────┐
│                    SPARQL Query Execution                     │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   ResultSetValidator                          │
│  - Variable presence validation                              │
│  - Type checking                                              │
│  - Cardinality constraints                                    │
│  - Duplicate detection                                        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      TypedBinding                             │
│  - Type-safe value extraction                                │
│  - Automatic type conversion                                 │
│  - Optional vs required values                               │
│  - Default values                                             │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      ResultMapper                             │
│  - Map to Rust structs                                       │
│  - Collection handling                                        │
│  - Error accumulation                                         │
│  - Validation                                                 │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   Type-Safe Result Structs                    │
└─────────────────────────────────────────────────────────────┘
```

## ResultSetValidator

### Purpose

Validates SPARQL SELECT query results against expected schemas to catch errors early.

### Usage

```rust
use ggen_mcp::sparql::{
    ResultSetValidator, CardinalityConstraint, VariableSpec, ExpectedType
};

// Create a validator expecting exactly one result
let validator = ResultSetValidator::new(CardinalityConstraint::ExactlyOne)
    .with_variable(VariableSpec::required("name", ExpectedType::Literal))
    .with_variable(VariableSpec::optional("description", ExpectedType::Literal))
    .with_variable(VariableSpec::required("iri", ExpectedType::IRI));

// Validate query results
let validated = validator.validate_and_collect(query_results)?;
```

### Cardinality Constraints

```rust
// Exactly one result
CardinalityConstraint::ExactlyOne

// Zero or one result
CardinalityConstraint::ZeroOrOne

// One or more results
CardinalityConstraint::OneOrMore

// Any number of results
CardinalityConstraint::ZeroOrMore

// Exact count
CardinalityConstraint::Exact(5)

// Minimum count
CardinalityConstraint::Min(3)

// Maximum count
CardinalityConstraint::Max(10)

// Range
CardinalityConstraint::Range(2, 8)
```

### Expected Types

```rust
// IRI/Named Node
ExpectedType::IRI

// Any literal
ExpectedType::Literal

// Blank node
ExpectedType::BlankNode

// Literal with specific datatype
ExpectedType::LiteralWithDatatype("http://www.w3.org/2001/XMLSchema#integer")

// Literal with language tag
ExpectedType::LiteralWithLanguage("en")

// Any type
ExpectedType::Any
```

### Strict Mode

Enable strict mode to require all variables to be declared:

```rust
let validator = ResultSetValidator::new(CardinalityConstraint::ZeroOrMore)
    .with_variable(VariableSpec::required("name", ExpectedType::Literal))
    .strict(); // Enable strict mode
```

## TypedBinding

### Purpose

Provides type-safe extraction of values from SPARQL query solutions with automatic type conversion.

### Basic Usage

```rust
use ggen_mcp::sparql::TypedBinding;

let binding = TypedBinding::new(&solution);

// Extract IRI
let iri = binding.get_iri("subject")?;

// Extract literal
let name = binding.get_literal("name")?;

// Extract integer
let count = binding.get_integer("count")?;

// Extract boolean
let enabled = binding.get_boolean("enabled")?;
```

### Optional Values

```rust
// Returns Option<String>
let description = binding.get_literal_opt("description")?;

// Returns Option<i64>
let optional_count = binding.get_integer_opt("count")?;
```

### Default Values

```rust
// Provide default if unbound
let name = binding.get_string_or("name", "Unknown");
let count = binding.get_integer_or("count", 0);
```

### Type Conversion

```rust
// Extract with specific datatype
let value = binding.get_literal_with_datatype(
    "value",
    "http://www.w3.org/2001/XMLSchema#integer"
)?;

// Parse to custom type
let custom: MyType = binding.parse("customValue")?;
```

### Typed Values

```rust
// Get as TypedValue enum for dynamic handling
let typed = binding.get_typed_value("field")?;

match typed {
    TypedValue::IRI(iri) => println!("IRI: {}", iri),
    TypedValue::Integer(i) => println!("Integer: {}", i),
    TypedValue::Literal(s) => println!("Literal: {}", s),
    _ => {}
}
```

## ResultMapper

### Purpose

Maps SPARQL query results to Rust types with validation and error accumulation.

### Implementing FromSparql

```rust
use ggen_mcp::sparql::{FromSparql, TypedBinding, MappingError};
use oxigraph::sparql::QuerySolution;

#[derive(Debug, Clone)]
pub struct AggregatRoot {
    pub name: String,
    pub description: Option<String>,
    pub properties: Vec<String>,
}

impl FromSparql for AggregateRoot {
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError> {
        let binding = TypedBinding::new(solution);

        Ok(Self {
            name: binding.get_literal("name")
                .map_err(|_| MappingError::MissingField("name".to_string()))?,
            description: binding.get_literal_opt("description").ok().flatten(),
            properties: vec![], // Aggregate from multiple rows
        })
    }
}
```

### Mapping Collections

```rust
use ggen_mcp::sparql::ResultMapper;

// Map to Vec
let aggregates: Vec<AggregateRoot> = ResultMapper::map_many(solutions)?;

// Map with partial results (collect errors)
let (results, errors) = ResultMapper::map_partial::<AggregateRoot>(solutions);

// Map to HashMap
let map = ResultMapper::map_to_hashmap::<AggregateRoot>(solutions, "name")?;

// Group by variable
let groups = ResultMapper::group_by::<AggregateRoot>(solutions, "category")?;
```

### Custom Mapping

```rust
let results = ResultMapper::map_with(solutions, |solution| {
    let binding = TypedBinding::new(solution);
    // Custom transformation
    Ok(MyCustomType {
        field: binding.get_literal("field")?,
    })
})?;
```

### Validation Builder

```rust
use ggen_mcp::sparql::MappingBuilder;

let results = MappingBuilder::<AggregateRoot>::new(solutions)
    .validate(|solution| {
        // Custom validation
        let binding = TypedBinding::new(solution);
        if binding.get_literal("name")?.is_empty() {
            return Err("Name cannot be empty".to_string());
        }
        Ok(())
    })
    .build()?;
```

## GraphValidator

### Purpose

Validates CONSTRUCT query results and RDF graphs for structural correctness.

### Basic Usage

```rust
use ggen_mcp::sparql::{
    GraphValidator, TriplePattern, SubjectType, ObjectType
};

let validator = GraphValidator::new()
    .with_pattern(TriplePattern {
        subject_type: SubjectType::IRI,
        predicate: Some("http://example.org/hasProperty".to_string()),
        object_type: ObjectType::Literal,
        required: true,
    })
    .check_well_formed(true)
    .check_cycles(true)
    .check_orphaned_blanks(true);

// Validate graph
validator.validate(&graph)?;
```

### Property Specifications

```rust
use ggen_mcp::sparql::{PropertySpec, PropertyCardinality};

let validator = GraphValidator::new()
    .with_property(
        "http://example.org/Person".to_string(),
        PropertySpec {
            predicate: "http://example.org/name".to_string(),
            object_type: ObjectType::Literal,
            cardinality: PropertyCardinality::ExactlyOne,
        }
    )
    .with_property(
        "http://example.org/Person".to_string(),
        PropertySpec {
            predicate: "http://example.org/email".to_string(),
            object_type: ObjectType::Literal,
            cardinality: PropertyCardinality::ZeroOrMore,
        }
    );
```

### Subject and Object Types

```rust
// Specific IRI
SubjectType::SpecificIRI("http://example.org/MyClass".to_string())

// IRI with prefix
SubjectType::IRIWithPrefix("http://example.org/".to_string())

// Any IRI
SubjectType::IRI

// Blank node
SubjectType::BlankNode

// Any subject
SubjectType::Any
```

### Validation Features

- **Well-formedness**: Checks basic RDF graph structure
- **Cycle detection**: Detects circular references in the graph
- **Orphaned blank nodes**: Finds unreferenced blank nodes
- **Property cardinality**: Validates property occurrence counts
- **Type checking**: Validates subject/predicate/object types

## QueryResultCache

### Purpose

Caches validated query results with TTL, memory bounds, and flexible invalidation strategies.

### Basic Usage

```rust
use ggen_mcp::sparql::{QueryResultCache, CacheConfig};

let config = CacheConfig {
    max_entries: 1000,
    default_ttl: 300, // 5 minutes
    auto_evict: true,
    max_memory_bytes: 100 * 1024 * 1024, // 100 MB
};

let cache = QueryResultCache::new(config);

// Cache query results
cache.put(query, solutions, Some(600), vec!["domain".to_string()]);

// Get cached results
if let Some(cached) = cache.get(query) {
    println!("Cache hit!");
}
```

### Invalidation Strategies

```rust
use ggen_mcp::sparql::CacheInvalidationStrategy;

// Invalidate all
cache.invalidate(CacheInvalidationStrategy::All);

// Invalidate specific query
cache.invalidate(CacheInvalidationStrategy::ByQuery(query.to_string()));

// Invalidate by prefix
cache.invalidate(CacheInvalidationStrategy::ByPrefix("SELECT".to_string()));

// Invalidate by tag
cache.invalidate(CacheInvalidationStrategy::ByTag("domain".to_string()));

// Invalidate expired only
cache.invalidate(CacheInvalidationStrategy::Expired);

// Custom predicate
cache.invalidate_if(|fingerprint, entry| {
    entry.remaining_ttl() < 60 // Less than 1 minute left
});
```

### Cache Statistics

```rust
let stats = cache.stats();
println!("Entries: {}", stats.entries);
println!("Hit rate: {:.2}%", stats.hit_rate * 100.0);
println!("Total size: {} bytes", stats.total_size_bytes);
println!("Evictions: {}", stats.evictions);
```

### Cache Maintenance

```rust
// Manual maintenance (remove expired)
cache.maintain();

// Refresh TTL
cache.refresh(query, Some(600));

// Get cache info
if let Some(info) = cache.get_info(query) {
    println!("Remaining TTL: {} seconds", info.remaining_ttl);
    println!("Result count: {}", info.result_count);
}
```

## Type-Safe Query Wrappers

### Pre-defined Wrappers

The system includes type-safe wrappers for all project SPARQL queries:

```rust
use ggen_mcp::sparql::{AggregateRootResult, load_aggregate_roots};

// Execute query and get typed results
let results = load_aggregate_roots(query_solutions)?;

for aggregate in results {
    println!("Aggregate: {}", aggregate.aggregate_name);
    if let Some(desc) = aggregate.aggregate_description {
        println!("Description: {}", desc);
    }
}
```

### Available Wrappers

- `AggregateRootResult` - Domain aggregate roots
- `ValueObjectResult` - Domain value objects
- `EntityClassResult` - Entity class definitions
- `RepositoryResult` - Repository interfaces
- `McpToolResult` - MCP tool definitions
- `McpToolCategoryResult` - Tool categories
- `GuardResult` - Guard definitions
- `ToolGuardBindingResult` - Tool-guard bindings
- `HandlerImplementationResult` - Handler implementations
- `CommandEventResult` - Commands and events
- `HandlerBindingResult` - Handler bindings
- `PolicyResult` - Policy definitions

### Creating Custom Wrappers

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct MyQueryResult {
    pub field1: String,
    pub field2: Option<i64>,
}

impl FromSparql for MyQueryResult {
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError> {
        let binding = TypedBinding::new(solution);

        Ok(Self {
            field1: binding.get_literal("field1")
                .map_err(|_| MappingError::MissingField("field1".to_string()))?,
            field2: binding.get_integer_opt("field2").ok().flatten(),
        })
    }
}

impl MyQueryResult {
    pub fn validator() -> ResultSetValidator {
        ResultSetValidator::new(CardinalityConstraint::ZeroOrMore)
            .with_variable(VariableSpec::required("field1", ExpectedType::Literal))
            .with_variable(VariableSpec::optional("field2", ExpectedType::Literal))
    }
}
```

## Error Handling

### Error Types

```rust
// Validation errors
ValidationError::MissingVariable(String)
ValidationError::TypeMismatch { var, expected, actual }
ValidationError::CardinalityViolation(String, String)
ValidationError::UnboundRequired(String)

// Binding errors
BindingError::NotFound(String)
BindingError::TypeMismatch { var, expected, actual }
BindingError::ConversionFailed { var, target_type, reason }

// Mapping errors
MappingError::Binding(BindingError)
MappingError::Validation(String)
MappingError::Multiple(Vec<String>)
MappingError::MissingField(String)

// Graph validation errors
GraphValidationError::MissingPattern(String)
GraphValidationError::InvalidSubjectType { expected, actual }
GraphValidationError::CycleDetected(String)
GraphValidationError::CardinalityViolation { subject, property, expected, actual }
```

### Error Handling Strategies

```rust
// Fail fast
let results = ResultMapper::map_many::<AggregateRoot>(solutions)?;

// Collect partial results
let (results, errors) = ResultMapper::map_partial::<AggregateRoot>(solutions);
for error in errors {
    eprintln!("Mapping error: {}", error);
}

// Custom error handling
match ResultMapper::map_many::<AggregateRoot>(solutions) {
    Ok(results) => process_results(results),
    Err(MappingError::Multiple(errors)) => {
        for error in errors {
            log::error!("Mapping error: {}", error);
        }
    }
    Err(e) => return Err(e),
}
```

## Performance Optimization

### Caching Strategy

```rust
// Cache frequently-used queries with long TTL
cache.put(domain_query, results, Some(3600), vec!["domain".to_string()]);

// Short TTL for dynamic data
cache.put(user_query, results, Some(60), vec!["user".to_string()]);

// Invalidate on updates
fn update_domain_model() {
    // ... perform update ...
    cache.invalidate(CacheInvalidationStrategy::ByTag("domain".to_string()));
}
```

### Batch Validation

```rust
// Validate once, map many
let validator = AggregateRootResult::validator();
validator.validate_results(solutions.clone())?;
let results = ResultMapper::map_many(solutions)?;
```

### Memory Management

```rust
// Configure memory bounds
let config = CacheConfig {
    max_entries: 500,
    max_memory_bytes: 50 * 1024 * 1024, // 50 MB
    ..Default::default()
};

// Monitor cache size
let stats = cache.stats();
if stats.total_size_bytes > 40 * 1024 * 1024 {
    cache.invalidate(CacheInvalidationStrategy::Expired);
}
```

## Best Practices

### 1. Always Validate Before Mapping

```rust
// Good
let validator = MyResult::validator();
validator.validate_results(solutions.clone())?;
let results = ResultMapper::map_many(solutions)?;

// Avoid: Direct mapping without validation
let results = ResultMapper::map_many(solutions)?; // May fail unexpectedly
```

### 2. Use Type-Safe Wrappers

```rust
// Good: Type-safe with validation
let tools = load_mcp_tools(solutions)?;

// Avoid: Manual extraction
for solution in solutions {
    let binding = TypedBinding::new(&solution);
    let tool_name = binding.get_literal("toolName")?;
    // ... more manual extraction ...
}
```

### 3. Handle Optional Values Properly

```rust
// Good: Explicit optional handling
let description = binding.get_literal_opt("description").ok().flatten();

// Good: Default values
let count = binding.get_integer_or("count", 0);

// Avoid: Unwrapping optionals
let description = binding.get_literal_opt("description").unwrap(); // May panic!
```

### 4. Cache Strategically

```rust
// Good: Cache with appropriate TTL and tags
cache.put(query, results, Some(ttl), tags);

// Good: Invalidate related queries
cache.invalidate(CacheInvalidationStrategy::ByTag("domain".to_string()));

// Avoid: Caching everything with long TTL
cache.put(query, results, Some(86400), vec![]); // 24 hours may be too long
```

### 5. Use Partial Results for Robustness

```rust
// Good: Continue processing valid results
let (results, errors) = ResultMapper::map_partial::<MyType>(solutions);
for error in errors {
    log::warn!("Skipped invalid result: {}", error);
}
process(results);

// Avoid: Failing entire operation on one bad result
let results = ResultMapper::map_many::<MyType>(solutions)?; // Fails if any invalid
```

### 6. Implement Validators for Custom Types

```rust
// Good: Comprehensive validation
impl MyResult {
    pub fn validator() -> ResultSetValidator {
        ResultSetValidator::new(CardinalityConstraint::OneOrMore)
            .with_variable(VariableSpec::required("id", ExpectedType::IRI))
            .with_variable(VariableSpec::required("name", ExpectedType::Literal))
            .strict() // Reject undeclared variables
    }
}

// Avoid: No validation
impl FromSparql for MyResult {
    fn from_solution(solution: &QuerySolution) -> Result<Self, MappingError> {
        // Direct extraction without validation
    }
}
```

### 7. Monitor Cache Performance

```rust
// Good: Regular monitoring
let stats = cache.stats();
if stats.hit_rate < 0.5 {
    log::warn!("Low cache hit rate: {:.2}%", stats.hit_rate * 100.0);
}

// Good: Periodic maintenance
tokio::spawn(async move {
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        cache.maintain();
    }
});
```

## Examples

### Complete Workflow Example

```rust
use ggen_mcp::sparql::*;
use oxigraph::store::Store;

async fn query_and_process() -> anyhow::Result<()> {
    // 1. Create cache
    let cache = QueryResultCache::default();

    // 2. Define query
    let query = r#"
        SELECT ?toolName ?toolDescription ?category
        WHERE {
            ?tool a mcp:Tool ;
                  rdfs:label ?toolName .
            OPTIONAL { ?tool rdfs:comment ?toolDescription }
            OPTIONAL { ?tool ggen:category ?category }
        }
    "#;

    // 3. Check cache
    if let Some(cached) = cache.get(query) {
        let tools = ResultMapper::map_many::<McpToolResult>(cached)?;
        return Ok(());
    }

    // 4. Execute query
    let store = Store::new()?;
    let results = store.query(query)?;

    // 5. Validate
    let validator = McpToolResult::validator();
    let solutions = validator.validate_and_collect(results)?;

    // 6. Cache results
    cache.put(query, solutions.clone(), Some(600), vec!["mcp".to_string()]);

    // 7. Map to types
    let tools = ResultMapper::map_many::<McpToolResult>(solutions)?;

    // 8. Process
    for tool in tools {
        println!("Tool: {}", tool.tool_name);
    }

    Ok(())
}
```

## Conclusion

The SPARQL result validation system provides comprehensive type-safety and error-proofing for query results in ggen-mcp. By following the poka-yoke principles, it prevents errors at the query boundary and ensures data integrity throughout the application.

For more information, see the API documentation and source code in `src/sparql/`.
