# SPARQL Test Harness Quick Start Guide

## Installation

The SPARQL test harness is located at:
```
tests/sparql_query_harness.rs
```

## Basic Usage

### 1. Create a Test Harness

```rust
use crate::{SparqlTestHarness, SparqlQueryBuilder};

#[test]
fn my_sparql_test() {
    let mut harness = SparqlTestHarness::new();
}
```

### 2. Load Test Data

```rust
harness.load_graph("user_domain.ttl").unwrap();
```

### 3. Execute a Query

**From String:**
```rust
let results = harness.execute_query(r#"
    PREFIX ddd: <https://ddd-patterns.dev/schema#>
    SELECT ?aggregate WHERE {
        ?aggregate a ddd:AggregateRoot .
    }
"#).unwrap();
```

**From File:**
```rust
let results = harness.execute_query_file("aggregates.rq").unwrap();
```

**Using Builder:**
```rust
let query = SparqlQueryBuilder::new()
    .select("?aggregate")
    .where_triple("?aggregate", "a", "ddd:AggregateRoot")
    .build();

let results = harness.execute_query(&query).unwrap();
```

### 4. Verify Results

```rust
assert_result_count_min(&results, 1);
assert_variables_exist(&results, &["aggregate"]);
assert_all_bindings_non_empty(&results, "aggregate");
```

## Common Test Patterns

### Pattern 1: Query Syntax Validation

```rust
#[test]
fn test_query_syntax() {
    let query = fs::read_to_string("queries/aggregates.rq").unwrap();
    assert!(validate_query_syntax(&query).is_ok());
}
```

### Pattern 2: Query Execution with Real Data

```rust
#[test]
fn test_query_execution() {
    let mut harness = SparqlTestHarness::new();
    harness.load_graph("user_domain.ttl").unwrap();

    let results = harness.execute_query_file("aggregates.rq").unwrap();

    assert!(!results[0].is_empty());
}
```

### Pattern 3: End-to-End Integration

```rust
#[test]
fn test_full_pipeline() {
    let mut harness = SparqlTestHarness::new();

    // Load ontology
    harness.load_graph("complete_system.ttl").unwrap();

    // Extract entities
    let entities = harness.execute_query_file("domain_entities.sparql").unwrap();
    assert_result_count_min(&entities[0], 1);

    // Extract tools
    let tools = harness.execute_query_file("mcp_tools.sparql").unwrap();
    assert_result_count_min(&tools[0], 1);

    // Verify they can be linked
    // (Code generation logic would go here)
}
```

### Pattern 4: Query Builder Construction

```rust
#[test]
fn test_query_builder() {
    let query = SparqlQueryBuilder::new()
        .prefix("ddd", "https://ddd-patterns.dev/schema#")
        .prefix("rdfs", "http://www.w3.org/2000/01/rdf-schema#")
        .select_vars(&["?aggregate", "?label", "?comment"])
        .where_triple("?aggregate", "a", "ddd:AggregateRoot")
        .where_triple("?aggregate", "rdfs:label", "?label")
        .optional("?aggregate rdfs:comment ?comment")
        .order_by("?label")
        .limit(10)
        .build();

    assert!(query.contains("SELECT"));
    assert!(query.contains("OPTIONAL"));
    assert!(query.contains("ORDER BY"));
}
```

### Pattern 5: Performance Testing

```rust
#[test]
fn test_query_performance() {
    let harness = SparqlTestHarness::new();
    let query = "SELECT ?s ?p ?o WHERE { ?s ?p ?o } LIMIT 10";

    let start = Instant::now();
    let _ = harness.execute_query(query).unwrap();
    let duration = start.elapsed();

    assert!(duration.as_millis() < 50);
}
```

## Available Assertions

### Count Assertions
- `assert_result_count(results, expected)` - Exactly N results
- `assert_result_count_min(results, min)` - At least N results

### Binding Assertions
- `assert_binding_exists(results, var, value)` - Specific binding exists
- `assert_all_bindings_non_empty(results, var)` - No empty values

### Variable Assertions
- `assert_variable_exists(results, var)` - Variable present
- `assert_variables_exist(results, vars)` - Multiple variables present

### Ordering Assertions
- `assert_result_ordered_by(results, var)` - Results sorted by variable

## Test Fixtures

### Available Graphs

1. **user_domain.ttl** - User aggregate, value objects, commands, events
2. **mcp_tools.ttl** - MCP tool definitions, parameters, guards
3. **complete_system.ttl** - Integrated domain + MCP system

### Location
```
fixtures/sparql/graphs/
```

### Loading Multiple Graphs
```rust
harness.load_graph("user_domain.ttl")?;
harness.load_graph("mcp_tools.ttl")?;
harness.load_graph("complete_system.ttl")?;
```

## Query Builder API

### Basic Selection
```rust
.select("?var")                    // Single variable
.select_vars(&["?a", "?b", "?c"])  // Multiple variables
```

### Prefixes
```rust
.prefix("ddd", "https://ddd-patterns.dev/schema#")
.prefix("rdfs", "http://www.w3.org/2000/01/rdf-schema#")
```

### WHERE Patterns
```rust
.where_triple("?s", "a", "ddd:Aggregate")     // Triple pattern
.where_pattern("?s ddd:hasProperty ?p")       // Free-form pattern
```

### Optional Patterns
```rust
.optional("?s rdfs:comment ?comment")
```

### Filters
```rust
.filter("?age > 18")
.filter("?name != ''")
```

### Ordering
```rust
.order_by("?name")
.order_by("DESC(?count)")
```

### Limiting Results
```rust
.limit(10)
.offset(20)
```

### Modifiers
```rust
.distinct()   // SELECT DISTINCT
```

### Grouping
```rust
.group_by("?type")
```

## Running Tests

### Run All Tests
```bash
cargo test --test sparql_query_harness
```

### Run Specific Module
```bash
cargo test --test sparql_query_harness query_builder_tests
```

### Run Single Test
```bash
cargo test --test sparql_query_harness test_simple_select_query
```

### Show Output
```bash
cargo test --test sparql_query_harness -- --nocapture
```

## Common Issues

### Issue: "Failed to read fixture"

**Solution:** Ensure fixtures exist:
```bash
ls -la fixtures/sparql/graphs/
```

### Issue: "Query validation failed"

**Solution:** Check query syntax:
```rust
validate_query_syntax(query)?;
```

### Issue: "No results returned"

**Solution:** Verify test data loaded:
```rust
assert!(!harness.store.is_empty().unwrap());
```

### Issue: "Variable not found"

**Solution:** Check variable names match query:
```rust
let vars = results.variable_names();
println!("Available vars: {:?}", vars);
```

## Best Practices

1. **Use State-Based Assertions** - Verify results, not interactions
2. **Load Real Data** - Use realistic test fixtures
3. **Test End-to-End** - Complete workflows from ontology to code
4. **Name Tests Clearly** - Describe what is tested
5. **Reuse Fixtures** - Create setup functions
6. **Check Performance** - Add timing assertions
7. **Validate Safety** - Ensure no dangerous patterns

## Example Test Suite

```rust
#[cfg(test)]
mod my_query_tests {
    use super::*;

    fn setup() -> SparqlTestHarness {
        let mut harness = SparqlTestHarness::new();
        harness.load_graph("user_domain.ttl").unwrap();
        harness
    }

    #[test]
    fn test_syntax() {
        let query = fs::read_to_string("queries/aggregates.rq").unwrap();
        assert!(validate_query_syntax(&query).is_ok());
    }

    #[test]
    fn test_execution() {
        let harness = setup();
        let results = harness.execute_query_file("aggregates.rq").unwrap();
        assert!(!results[0].is_empty());
    }

    #[test]
    fn test_results() {
        let harness = setup();
        let results = harness.execute_query_file("aggregates.rq").unwrap();
        assert_result_count_min(&results[0], 1);
        assert_variables_exist(&results[0], &["aggregate", "label"]);
    }
}
```

## Next Steps

1. Review full documentation: [docs/TDD_SPARQL_HARNESS.md](../docs/TDD_SPARQL_HARNESS.md)
2. Explore test fixtures: `fixtures/sparql/`
3. Add tests for your queries: `queries/`
4. Run the full test suite
5. Check coverage and add missing tests

## Support

For issues or questions:
- Check the full documentation
- Review existing tests in `tests/sparql_query_harness.rs`
- Examine test fixtures in `fixtures/sparql/`
