# SPARQL Test Fixtures

This directory contains test fixtures for SPARQL query testing using the Chicago-style TDD test harness.

## Directory Structure

```
fixtures/sparql/
├── graphs/              # RDF test data in Turtle format
│   ├── user_domain.ttl
│   ├── mcp_tools.ttl
│   └── complete_system.ttl
└── expected/            # Expected query results in JSON
    ├── aggregates_result.json
    ├── tools_result.json
    └── domain_entities_result.json
```

## Graph Fixtures

### user_domain.ttl

**Purpose:** Test fixture for domain-driven design entities

**Contains:**
- User aggregate root with properties (id, email, name)
- Email and UserId value objects with invariants
- Order aggregate with total validation
- CreateUser command and UserCreated event
- CreateUserHandler linking command to event
- UserRepository with methods
- UserManagement bounded context

**Use Cases:**
- Testing aggregate extraction queries
- Validating value object queries
- Testing command/event/handler relationships
- Repository interface extraction

**Example Usage:**
```rust
harness.load_graph("user_domain.ttl")?;
let results = harness.execute_query_file("aggregates.rq")?;
```

**Expected Results:**
- 2 aggregates (User, Order)
- 2 value objects (Email, UserId)
- 1 command (CreateUser)
- 1 event (UserCreated)
- 1 handler (CreateUserHandler)
- 1 repository (UserRepository)

### mcp_tools.ttl

**Purpose:** Test fixture for MCP server definitions

**Contains:**
- MCP tool definitions:
  - read_spreadsheet (read operation)
  - write_spreadsheet (write operation with proposal)
  - list_workbooks (discovery operation)
  - analyze_formulas (analysis operation)
- Tool parameters with types and requirements
- Guards for validation (PathValidationGuard, DataSizeGuard)
- Prompts for AI interaction
- Resources (WorkbookResource)
- Server configuration (SpreadsheetMCPServer)

**Use Cases:**
- Testing MCP tool extraction
- Validating parameter definitions
- Testing guard extraction
- Prompt and resource queries
- Server configuration validation

**Example Usage:**
```rust
harness.load_graph("mcp_tools.ttl")?;
let results = harness.execute_query_file("mcp_tools.sparql")?;
```

**Expected Results:**
- 4 tools (read_spreadsheet, write_spreadsheet, list_workbooks, analyze_formulas)
- Multiple parameters per tool
- 2 guards (PathValidationGuard, DataSizeGuard)
- 1 prompt (spreadsheet_analysis)
- 1 resource (workbook)
- 1 server definition

### complete_system.ttl

**Purpose:** Integrated test fixture showing domain + MCP integration

**Contains:**
- Product aggregate with properties
- ProductService domain service
- PricingPolicy domain policy
- create_product MCP tool
- CreateProduct command
- CreateProductHandler linking tool and command
- ProductCreated event
- ProductRepository

**Use Cases:**
- Testing integrated queries across domain and MCP
- Validating tool-to-command mappings
- Testing handler inference rules
- End-to-end workflow validation

**Example Usage:**
```rust
harness.load_graph("complete_system.ttl")?;
let domain_results = harness.execute_query_file("domain_entities.sparql")?;
let tool_results = harness.execute_query_file("mcp_tools.sparql")?;
let handler_results = harness.execute_query_file("inference/handler_implementations.sparql")?;
```

**Expected Results:**
- 1 aggregate (Product)
- 1 service (ProductService)
- 1 policy (PricingPolicy)
- 1 tool (create_product)
- 1 command (CreateProduct)
- 1 handler (CreateProductHandler) linking tool + command
- 1 repository (ProductRepository)

## Expected Results

### aggregates_result.json

Expected results for `queries/aggregates.rq`

**Structure:**
```json
{
  "description": "Expected results from aggregates.rq query",
  "expected_results": [...],
  "assertions": {
    "min_count": 1,
    "variables": ["aggregate", "label"],
    "ordered_by": "label",
    "non_empty": ["label"]
  }
}
```

### tools_result.json

Expected results for `queries/mcp_tools.sparql`

**Structure:**
```json
{
  "description": "Expected results from mcp_tools.sparql query",
  "expected_results": [...],
  "assertions": {
    "min_count": 1,
    "variables": ["toolName", "toolDescription"],
    "categories": {
      "read": [...],
      "write": [...],
      "analysis": [...]
    }
  }
}
```

### domain_entities_result.json

Expected results for `queries/domain_entities.sparql` (multi-query file)

**Structure:**
```json
{
  "description": "Expected results from domain_entities.sparql query",
  "queries": [
    {
      "query_index": 1,
      "description": "Extract Aggregate Roots",
      "expected_aggregates": [...]
    },
    {
      "query_index": 2,
      "description": "Extract Value Objects",
      "expected_value_objects": [...]
    }
  ]
}
```

## Using Fixtures

### Load Single Graph

```rust
let mut harness = SparqlTestHarness::new();
harness.load_graph("user_domain.ttl")?;
```

### Load Multiple Graphs

```rust
let mut harness = SparqlTestHarness::new();
harness.load_graph("user_domain.ttl")?;
harness.load_graph("mcp_tools.ttl")?;
harness.load_graph("complete_system.ttl")?;
```

### Verify Graph Loaded

```rust
assert!(!harness.store.is_empty().unwrap());
```

## Creating New Fixtures

### 1. Create Turtle File

```turtle
@prefix ddd: <https://ddd-patterns.dev/schema#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

# Your test data here
myns:MyAggregate a ddd:AggregateRoot ;
    rdfs:label "MyAggregate" .
```

### 2. Save to graphs/

```bash
fixtures/sparql/graphs/my_test_data.ttl
```

### 3. Load in Tests

```rust
harness.load_graph("my_test_data.ttl")?;
```

### 4. Document Expected Results

Create corresponding JSON in `expected/`:

```json
{
  "description": "Expected results from my_query.rq",
  "expected_results": [...],
  "assertions": {...}
}
```

## Fixture Guidelines

### DO:
- ✅ Use realistic domain models
- ✅ Include all required properties
- ✅ Add comments for clarity
- ✅ Follow RDF/Turtle best practices
- ✅ Use consistent namespaces
- ✅ Include edge cases

### DON'T:
- ❌ Create minimal/fake data
- ❌ Omit required properties
- ❌ Mix unrelated concerns
- ❌ Use inconsistent naming
- ❌ Skip validation rules

## Testing with Fixtures

### Pattern 1: Load and Query

```rust
#[test]
fn test_with_fixture() {
    let mut harness = SparqlTestHarness::new();
    harness.load_graph("user_domain.ttl").unwrap();

    let results = harness.execute_query_file("aggregates.rq").unwrap();
    assert_result_count_min(&results[0], 1);
}
```

### Pattern 2: Multi-Fixture Integration

```rust
#[test]
fn test_integration() {
    let mut harness = SparqlTestHarness::new();
    harness.load_graph("user_domain.ttl").unwrap();
    harness.load_graph("mcp_tools.ttl").unwrap();

    // Test domain queries
    let domain = harness.execute_query_file("domain_entities.sparql").unwrap();
    assert!(!domain[0].is_empty());

    // Test MCP queries
    let tools = harness.execute_query_file("mcp_tools.sparql").unwrap();
    assert!(!tools[0].is_empty());
}
```

### Pattern 3: Verify Expected Results

```rust
#[test]
fn test_expected_results() {
    let mut harness = SparqlTestHarness::new();
    harness.load_graph("user_domain.ttl").unwrap();

    let results = harness.execute_query_file("aggregates.rq").unwrap();

    // Load expected results
    let expected: JsonValue =
        serde_json::from_str(
            &fs::read_to_string("fixtures/sparql/expected/aggregates_result.json")?
        )?;

    // Verify count matches
    let expected_count = expected["expected_results"].as_array().unwrap().len();
    assert_result_count(&results[0], expected_count);
}
```

## Fixture Maintenance

### Adding New Test Data

1. Identify the domain concept to test
2. Create minimal but realistic RDF representation
3. Include all required properties and relationships
4. Add to appropriate fixture file or create new one
5. Document in this README
6. Create corresponding expected results JSON
7. Add tests that use the new fixture

### Updating Existing Fixtures

1. Identify what needs to change
2. Update the Turtle file
3. Update expected results JSON
4. Verify all tests still pass
5. Update documentation if semantics changed

### Removing Obsolete Fixtures

1. Check for tests using the fixture
2. Update or remove dependent tests
3. Remove fixture files
4. Update this README
5. Update expected results if needed

## Common Issues

### Issue: "Failed to read fixture"

**Cause:** File path incorrect or file doesn't exist

**Solution:**
```bash
ls -la fixtures/sparql/graphs/
```

Ensure file exists and path is relative to project root.

### Issue: "Failed to load graph"

**Cause:** Invalid Turtle syntax

**Solution:**
Validate Turtle syntax:
```bash
rapper -i turtle fixtures/sparql/graphs/your_file.ttl
```

Or use online validator: https://www.w3.org/RDF/Validator/

### Issue: "No results returned"

**Cause:** Query doesn't match fixture data

**Solution:**
- Verify prefixes match
- Check predicate/property names
- Inspect actual triples in fixture

### Issue: "Results don't match expected"

**Cause:** Fixture or expected results out of sync

**Solution:**
- Re-run query manually
- Update expected results JSON
- Verify fixture has correct data

## Validation Tools

### Validate Turtle Syntax

```bash
rapper -i turtle -o ntriples fixtures/sparql/graphs/user_domain.ttl
```

### Count Triples

```bash
rapper -i turtle -c fixtures/sparql/graphs/user_domain.ttl
```

### View as N-Triples

```bash
rapper -i turtle -o ntriples fixtures/sparql/graphs/user_domain.ttl > output.nt
```

## Future Enhancements

1. **Auto-generation:** Generate fixtures from ontology definitions
2. **Validation:** Automated fixture validation against schemas
3. **Versioning:** Track fixture versions with ontology versions
4. **Coverage:** Report which fixtures exercise which queries
5. **Minimization:** Reduce fixture size while maintaining coverage

## References

- [Turtle Specification](https://www.w3.org/TR/turtle/)
- [RDF Primer](https://www.w3.org/TR/rdf11-primer/)
- [SPARQL Test Harness](../tests/sparql_query_harness.rs)
- [Test Harness Documentation](../docs/TDD_SPARQL_HARNESS.md)
