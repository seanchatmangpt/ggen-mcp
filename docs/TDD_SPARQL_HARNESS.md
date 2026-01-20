# Chicago-Style TDD SPARQL Query Test Harness

## Overview

This document describes the comprehensive Chicago-style TDD test harness for SPARQL query generation, execution, and validation in the ggen-mcp project.

## Table of Contents

1. [Architecture](#architecture)
2. [Chicago-Style TDD Approach](#chicago-style-tdd-approach)
3. [Test Harness Components](#test-harness-components)
4. [Query Coverage](#query-coverage)
5. [Usage Examples](#usage-examples)
6. [Test Fixtures](#test-fixtures)
7. [Best Practices](#best-practices)
8. [Performance Budgets](#performance-budgets)

## Architecture

### Core Components

```
tests/harness/
  └── sparql_query_harness.rs     # Main test harness

fixtures/sparql/
  ├── graphs/                      # Test RDF graphs
  │   ├── user_domain.ttl
  │   ├── mcp_tools.ttl
  │   └── complete_system.ttl
  └── expected/                    # Expected results
      ├── aggregates_result.json
      ├── tools_result.json
      └── domain_entities_result.json
```

### Test Harness Structure

```rust
SparqlTestHarness
├── store: Store                 // Oxigraph in-memory store
├── query_dir: PathBuf          // queries/ directory
└── fixture_dir: PathBuf        // fixtures/sparql/ directory

Methods:
├── load_graph()                // Load test data from Turtle
├── execute_query()             // Execute SPARQL and return results
├── execute_query_file()        // Execute query from file
├── validate_query_syntax()     // Validate without execution
└── clear()                     // Reset store
```

## Chicago-Style TDD Approach

### What is Chicago-Style TDD?

Chicago-style (classical) TDD focuses on **state verification** rather than interaction verification:

1. **State-Based Testing**: Verify the state of the system after operations
2. **Real Dependencies**: Use real objects and databases, not mocks
3. **Integration Focus**: Test real behavior with actual data flows
4. **End-to-End Verification**: Complete workflows from input to output

### Why Chicago-Style for SPARQL?

1. **Real Query Execution**: SPARQL queries must execute against real graphs
2. **State Verification**: Query results are state that must be validated
3. **No Interaction Testing**: We care about results, not method calls
4. **Integration Critical**: Ontology → Query → Results → Code Gen

### Example: Chicago vs London Style

**London-Style (Mockist):**
```rust
// Mock the store, verify interaction
let mock_store = MockStore::new();
mock_store.expect_query()
    .with(eq("SELECT..."))
    .times(1)
    .returning(|| mock_results);
```

**Chicago-Style (Classicist):**
```rust
// Real store, verify state
let mut harness = SparqlTestHarness::new();
harness.load_graph("user_domain.ttl")?;

let results = harness.execute_query("SELECT ?s WHERE { ?s a ddd:AggregateRoot }")?;

assert_result_count_min(&results, 1);
assert_all_bindings_non_empty(&results, "s");
```

## Test Harness Components

### 1. SparqlTestHarness

Main test harness for query execution and validation.

**Creation:**
```rust
let harness = SparqlTestHarness::new();
```

**Loading Test Data:**
```rust
harness.load_graph("user_domain.ttl")?;
harness.load_graph("mcp_tools.ttl")?;
```

**Executing Queries:**
```rust
let results = harness.execute_query(r#"
    PREFIX ddd: <https://ddd-patterns.dev/schema#>
    SELECT ?aggregate WHERE {
        ?aggregate a ddd:AggregateRoot .
    }
"#)?;
```

**Executing Query Files:**
```rust
let results = harness.execute_query_file("aggregates.rq")?;
```

### 2. SparqlQueryBuilder

Programmatic SPARQL query construction with fluent API.

**Basic SELECT:**
```rust
let query = SparqlQueryBuilder::new()
    .select("?entity")
    .select("?name")
    .where_triple("?entity", "rdf:type", "ddd:Aggregate")
    .filter("?name != ''")
    .build();
```

**With Prefixes:**
```rust
let query = SparqlQueryBuilder::new()
    .prefix("ddd", "https://ddd-patterns.dev/schema#")
    .prefix("rdfs", "http://www.w3.org/2000/01/rdf-schema#")
    .select_vars(&["?aggregate", "?label"])
    .where_triple("?aggregate", "a", "ddd:AggregateRoot")
    .where_triple("?aggregate", "rdfs:label", "?label")
    .order_by("?label")
    .build();
```

**With Optional Patterns:**
```rust
let query = SparqlQueryBuilder::new()
    .select_vars(&["?entity", "?description"])
    .where_triple("?entity", "a", "ddd:Entity")
    .optional("?entity rdfs:comment ?description")
    .build();
```

**With Aggregation:**
```rust
let query = SparqlQueryBuilder::new()
    .select_vars(&["?type", "(COUNT(?s) AS ?count)"])
    .where_triple("?s", "a", "?type")
    .group_by("?type")
    .order_by("DESC(?count)")
    .limit(10)
    .build();
```

### 3. Result Assertions (State-Based)

State verification functions for query results.

**Count Assertions:**
```rust
assert_result_count(&results, 5);           // Exactly 5
assert_result_count_min(&results, 1);        // At least 1
```

**Binding Assertions:**
```rust
assert_binding_exists(&results, "name", "User");
assert_all_bindings_non_empty(&results, "label");
```

**Variable Assertions:**
```rust
assert_variable_exists(&results, "aggregate");
assert_variables_exist(&results, &["entity", "name", "type"]);
```

**Ordering Assertions:**
```rust
assert_result_ordered_by(&results, "name");
```

### 4. Query Validation

**Syntax Validation:**
```rust
validate_query_syntax("SELECT ?s WHERE { ?s ?p ?o }")?;
```

**Safety Checks:**
```rust
assert!(check_query_safety("SELECT ?s WHERE { ?s ?p ?o }"));
assert!(!check_query_safety("DROP GRAPH <http://example.org>"));
```

## Query Coverage

### 80/20 Principle Applied

Focus on the 20% of queries that provide 80% of value:

#### Critical Queries (High Priority)

1. **aggregates.rq** - Domain aggregate roots
2. **domain_entities.sparql** - Complete entity extraction
3. **mcp_tools.sparql** - MCP tool definitions
4. **handlers.rq** - Command handlers
5. **inference/handler_implementations.sparql** - Generated handlers

#### Standard Queries (Medium Priority)

6. **commands.rq** - Domain commands
7. **value_objects.rq** - Value object definitions
8. **repositories.rq** - Repository interfaces
9. **services.rq** - Domain services
10. **mcp_guards.sparql** - Input guards

#### Supporting Queries (Lower Priority)

11. **invariants.rq** - Business rules
12. **policies.rq** - Domain policies
13. **properties.rq** - Property definitions
14. **tests.rq** - Test scaffolding

### Query Test Matrix

| Query File | Syntax | Execution | Results | Integration |
|-----------|--------|-----------|---------|-------------|
| aggregates.rq | ✓ | ✓ | ✓ | ✓ |
| domain_entities.sparql | ✓ | ✓ | ✓ | ✓ |
| mcp_tools.sparql | ✓ | ✓ | ✓ | ✓ |
| handlers.rq | ✓ | ✓ | ✓ | ✓ |
| commands.rq | ✓ | ✓ | ✓ | - |
| value_objects.rq | ✓ | ✓ | ✓ | - |
| repositories.rq | ✓ | ✓ | - | - |
| services.rq | ✓ | ✓ | - | - |
| invariants.rq | ✓ | - | - | - |
| policies.rq | ✓ | - | - | - |

**Legend:**
- Syntax: Query parses correctly
- Execution: Query executes without error
- Results: Result validation tests exist
- Integration: End-to-end workflow tested

## Usage Examples

### Example 1: Basic Query Execution Test

```rust
#[test]
fn test_aggregates_query() {
    let mut harness = SparqlTestHarness::new();
    harness.load_graph("user_domain.ttl").unwrap();

    let results = harness.execute_query_file("aggregates.rq").unwrap();
    let result_set = &results[0];

    assert_result_count_min(result_set, 1);
    assert_variables_exist(result_set, &["aggregate", "label"]);
    assert_all_bindings_non_empty(result_set, "label");
}
```

### Example 2: Query Builder Test

```rust
#[test]
fn test_query_builder_aggregates() {
    let mut harness = SparqlTestHarness::new();
    harness.load_graph("user_domain.ttl").unwrap();

    let query = SparqlQueryBuilder::new()
        .prefix("ddd", "https://ddd-patterns.dev/schema#")
        .prefix("rdfs", "http://www.w3.org/2000/01/rdf-schema#")
        .select_vars(&["?aggregate", "?label"])
        .where_triple("?aggregate", "a", "ddd:AggregateRoot")
        .where_triple("?aggregate", "rdfs:label", "?label")
        .order_by("?label")
        .build();

    let results = harness.execute_query(&query).unwrap();

    assert_result_ordered_by(&results, "label");
}
```

### Example 3: End-to-End Integration Test

```rust
#[test]
fn test_ontology_to_code_generation() {
    let mut harness = SparqlTestHarness::new();

    // 1. Load complete ontology
    harness.load_graph("complete_system.ttl").unwrap();

    // 2. Execute domain entity query
    let entity_results = harness.execute_query_file("domain_entities.sparql").unwrap();

    // 3. Verify aggregates extracted
    assert_result_count_min(&entity_results[0], 1);

    // 4. Execute tool query
    let tool_results = harness.execute_query_file("mcp_tools.sparql").unwrap();

    // 5. Verify tools extracted
    assert_result_count_min(&tool_results[0], 1);

    // 6. Execute handler inference query
    let handler_results = harness
        .execute_query_file("inference/handler_implementations.sparql")
        .unwrap();

    // 7. Verify handlers generated
    assert_result_count_min(&handler_results[0], 1);
}
```

### Example 4: Query Safety Validation

```rust
#[test]
fn test_all_queries_are_safe() {
    let query_files = vec![
        "aggregates.rq",
        "commands.rq",
        "handlers.rq",
        "domain_entities.sparql",
        "mcp_tools.sparql",
    ];

    for query_file in query_files {
        let content = fs::read_to_string(format!("queries/{}", query_file)).unwrap();
        assert!(
            check_query_safety(&content),
            "Query {} contains unsafe patterns",
            query_file
        );
    }
}
```

### Example 5: Performance Budget Test

```rust
#[test]
fn test_query_execution_performance() {
    let mut harness = SparqlTestHarness::new();
    harness.load_graph("complete_system.ttl").unwrap();

    let query = harness.execute_query_file("aggregates.rq").unwrap();

    let start = Instant::now();
    for _ in 0..100 {
        let _ = harness.execute_query_file("aggregates.rq").unwrap();
    }
    let duration = start.elapsed();

    // 100 queries should complete in < 500ms
    assert!(
        duration.as_millis() < 500,
        "100 queries took {}ms, expected < 500ms",
        duration.as_millis()
    );
}
```

## Test Fixtures

### Graph Fixtures

#### user_domain.ttl

Contains:
- User aggregate with properties and invariants
- Email and UserId value objects
- Order aggregate
- Commands, events, handlers
- Repositories
- Bounded context

**Usage:**
```rust
harness.load_graph("user_domain.ttl")?;
```

#### mcp_tools.ttl

Contains:
- MCP tool definitions (read_spreadsheet, write_spreadsheet, etc.)
- Tool parameters with types
- Guards and validation rules
- Prompts and resources
- Server configuration

**Usage:**
```rust
harness.load_graph("mcp_tools.ttl")?;
```

#### complete_system.ttl

Contains:
- Complete integration of domain and MCP
- Product aggregate with service and policy
- MCP tools linked to domain commands
- Handler processing both tools and commands

**Usage:**
```rust
harness.load_graph("complete_system.ttl")?;
```

### Expected Results Fixtures

JSON files in `fixtures/sparql/expected/` contain:
- Expected query results
- Assertion requirements
- Validation criteria

**Example:**
```json
{
  "description": "Expected results from aggregates.rq query",
  "expected_results": [...],
  "assertions": {
    "min_count": 1,
    "variables": ["aggregate", "label"],
    "ordered_by": "label"
  }
}
```

## Best Practices

### 1. Test Organization

```rust
#[cfg(test)]
mod aggregate_query_tests {
    use super::*;

    fn setup() -> SparqlTestHarness {
        let mut harness = SparqlTestHarness::new();
        harness.load_graph("user_domain.ttl").unwrap();
        harness
    }

    #[test]
    fn test_aggregate_extraction() {
        let harness = setup();
        // Test code...
    }
}
```

### 2. State-Based Assertions

Always verify **state** (results), not interactions:

✅ **Good (State):**
```rust
let results = harness.execute_query(query)?;
assert_result_count(&results, 5);
assert_binding_exists(&results, "name", "User");
```

❌ **Bad (Interaction):**
```rust
mock_store.expect_query()
    .with(eq("SELECT..."))
    .times(1);
```

### 3. Use Real Data

Load realistic test data that represents actual usage:

✅ **Good:**
```rust
harness.load_graph("user_domain.ttl")?;  // Real domain model
```

❌ **Bad:**
```rust
let mock_data = vec![];  // Empty or fake data
```

### 4. End-to-End Testing

Test complete workflows, not isolated units:

✅ **Good:**
```rust
// Load ontology → Execute query → Validate results → Map to Rust
harness.load_graph("complete_system.ttl")?;
let results = harness.execute_query_file("domain_entities.sparql")?;
assert_result_count_min(&results[0], 1);
```

❌ **Bad:**
```rust
// Test only query parsing
let parsed = parse_query(query)?;
```

### 5. Meaningful Test Names

```rust
#[test]
fn test_aggregates_query_returns_all_domain_roots_ordered_by_label() {
    // Test code...
}
```

### 6. Fixture Reuse

Create reusable fixtures for common scenarios:

```rust
fn create_domain_with_aggregates() -> SparqlTestHarness {
    let mut harness = SparqlTestHarness::new();
    harness.load_graph("user_domain.ttl").unwrap();
    harness.load_graph("complete_system.ttl").unwrap();
    harness
}
```

## Performance Budgets

### Query Execution Budgets

| Query Type | Budget | Rationale |
|-----------|--------|-----------|
| Simple SELECT | 10ms | Basic triple pattern |
| Complex SELECT | 50ms | Multiple patterns, filters |
| CONSTRUCT | 100ms | Graph construction overhead |
| Inference | 200ms | Rule evaluation |
| Full System | 500ms | Complete query suite |

### Test Budgets

| Test Type | Budget | Rationale |
|-----------|--------|-----------|
| Unit Test | 1ms | Query builder, assertions |
| Query Syntax | 10ms | Parse only, no execution |
| Single Query Execution | 50ms | Simple data load + execute |
| Integration Test | 500ms | Multiple queries + validation |
| Full Suite | 5000ms | All queries, all fixtures |

### Performance Test Example

```rust
#[test]
fn test_query_execution_within_budget() {
    let harness = SparqlTestHarness::new();
    let query = "SELECT ?s ?p ?o WHERE { ?s ?p ?o } LIMIT 100";

    let start = Instant::now();
    let _ = harness.execute_query(query).unwrap();
    let duration = start.elapsed();

    assert!(
        duration.as_millis() < 10,
        "Query exceeded budget: {}ms > 10ms",
        duration.as_millis()
    );
}
```

## Running Tests

### Run All Harness Tests

```bash
cargo test --test sparql_query_harness
```

### Run Specific Test Module

```bash
cargo test --test sparql_query_harness query_builder_tests
```

### Run With Output

```bash
cargo test --test sparql_query_harness -- --nocapture
```

### Run Performance Tests

```bash
cargo test --test sparql_query_harness performance_tests
```

## Integration with CI/CD

### GitHub Actions Example

```yaml
- name: Run SPARQL Test Harness
  run: |
    cargo test --test sparql_query_harness --verbose
    cargo test --test sparql_query_harness performance_tests -- --ignored
```

### Coverage Requirements

- **Query Syntax**: 100% of queries must parse
- **Query Execution**: 80% of queries must execute successfully
- **Result Validation**: 60% of queries have result validation tests
- **Integration**: 40% of queries have end-to-end tests

## Troubleshooting

### Query Fails to Execute

1. Check query syntax with `validate_query_syntax()`
2. Verify test data loaded with `harness.store.is_empty()`
3. Check namespace prefixes match fixture data

### Results Don't Match Expected

1. Examine actual results with `results.get_all_values()`
2. Check variable names match query
3. Verify test data contains expected triples

### Performance Issues

1. Use smaller test fixtures for unit tests
2. Check for query complexity (joins, filters)
3. Consider query optimization patterns

## Future Enhancements

1. **Property-Based Testing**: Use proptest for query generation
2. **Query Mutation Testing**: Validate error handling
3. **Performance Regression Detection**: Track query performance over time
4. **Coverage Reporting**: Track which queries lack tests
5. **Fixture Generation**: Auto-generate fixtures from ontology

## References

- [Chicago-Style TDD](http://www.growing-object-oriented-software.com/)
- [SPARQL 1.1 Specification](https://www.w3.org/TR/sparql11-query/)
- [Oxigraph Documentation](https://docs.rs/oxigraph/)
- [Toyota Production System Research](docs/RESEARCH_TOYOTA_PRODUCTION_SYSTEM.md)
- [Rust MCP Best Practices](docs/RESEARCH_RUST_MCP_BEST_PRACTICES.md)
