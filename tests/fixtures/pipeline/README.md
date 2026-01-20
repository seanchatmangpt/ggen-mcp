# Code Generation Pipeline Test Fixtures

This directory contains test fixtures for the comprehensive Chicago-style TDD code generation pipeline harness.

## Overview

Each fixture represents a complete test scenario with:
- **Input**: Ontology (TTL), queries (SPARQL), and optional templates
- **Expected Output**: Golden files showing expected generated code

## Fixture Structure

```
fixture_name/
├── input/
│   ├── ontology.ttl       # REQUIRED: RDF ontology in Turtle format
│   ├── queries.sparql     # OPTIONAL: Custom SPARQL queries
│   └── templates/         # OPTIONAL: Custom Tera templates
│       └── *.tera
└── expected/              # REQUIRED: Expected output (golden files)
    └── *.rs
```

## Available Fixtures

### 1. simple_aggregate

**Purpose**: Test basic aggregate root and command generation

**Input**:
- Single `User` aggregate with id, email, name
- Single `CreateUser` command

**Expected Output**:
- `User.rs` - Aggregate root implementation
- `CreateUser.rs` - Command implementation

**Use Case**: Validate basic DDD pattern generation

**Test**:
```rust
cargo test test_simple_aggregate_complete_pipeline
```

### 2. complete_domain

**Purpose**: Test complex domain with multiple aggregates

**Input**:
- `User` aggregate
- `Product` aggregate
- `Order` aggregate
- `Money` value object
- `OrderStatus` value object

**Expected Output**:
- `aggregates/User.rs`
- `aggregates/Product.rs`
- `aggregates/Order.rs`
- `value_objects/Money.rs`
- `value_objects/OrderStatus.rs`

**Use Case**: Validate complete e-commerce domain generation

**Test**:
```rust
cargo test test_complete_domain_pipeline
```

### 3. mcp_tool

**Purpose**: Test MCP tool handler generation

**Input**:
- `ReadFile` MCP tool with parameters
- `WriteFile` MCP tool with parameters

**Expected Output**:
- `tools/read_file.rs` - Tool handler and params
- `tools/write_file.rs` - Tool handler and params

**Use Case**: Validate MCP server tool generation

**Test**:
```rust
cargo test test_mcp_tool_generation
```

### 4. error_scenarios

**Purpose**: Test error handling for invalid ontologies

**Input**:
- Invalid ontology (missing properties, broken references)

**Expected**: Pipeline should fail gracefully or report validation errors

**Use Case**: Validate error detection and reporting

**Test**:
```rust
cargo test test_invalid_ontology_error_handling
```

## Creating New Fixtures

### Step 1: Create Directory Structure

```bash
mkdir -p tests/fixtures/pipeline/my_fixture/input
mkdir -p tests/fixtures/pipeline/my_fixture/expected
```

### Step 2: Create Ontology

Create `input/ontology.ttl`:

```turtle
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ddd: <http://example.org/ddd#> .
@prefix ggen: <http://example.org/ggen#> .

# Define your aggregate
ggen:MyAggregate a ddd:AggregateRoot ;
    rdfs:label "MyAggregate" ;
    ddd:hasProperty ggen:MyAggregate_id ;
    ddd:hasProperty ggen:MyAggregate_name .

# Define properties
ggen:MyAggregate_id a ddd:Property ;
    rdfs:label "id" ;
    ddd:propertyType "Uuid" .

ggen:MyAggregate_name a ddd:Property ;
    rdfs:label "name" ;
    ddd:propertyType "String" .
```

### Step 3: Create Expected Output (Optional at First)

Create `expected/MyAggregate.rs`:

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyAggregate {
    pub id: Uuid,
    pub name: String,
}

impl MyAggregate {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
        }
    }
}
```

### Step 4: Write Test

Add to `tests/codegen_pipeline_integration_tests.rs`:

```rust
#[test]
fn test_my_fixture() -> Result<()> {
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("my_fixture")
        .with_validation(true);

    let result = harness.run_complete_pipeline()?;

    harness.assert_all_stages_succeeded(&result);

    Ok(())
}
```

### Step 5: Run Test and Generate Golden Files

```bash
# Run test to generate output
cargo test test_my_fixture

# Review generated output
cat target/test_output/my_fixture/MyAggregate.rs

# If correct, update golden files
cargo test test_update_golden_files -- --ignored

# Or manually copy
cp target/test_output/my_fixture/MyAggregate.rs \
   tests/fixtures/pipeline/my_fixture/expected/
```

## Ontology Patterns

### Aggregate Root Pattern

```turtle
ggen:EntityName a ddd:AggregateRoot ;
    rdfs:label "EntityName" ;
    ddd:hasProperty ggen:EntityName_id ;
    ddd:hasProperty ggen:EntityName_field .
```

### Command Pattern

```turtle
ggen:CommandName a ddd:Command ;
    rdfs:label "CommandName" ;
    ddd:targetsAggregate ggen:TargetAggregate ;
    ddd:hasParameter ggen:CommandName_param .
```

### Value Object Pattern

```turtle
ggen:ValueObjectName a ddd:ValueObject ;
    rdfs:label "ValueObjectName" ;
    ddd:hasProperty ggen:ValueObjectName_field .
```

### MCP Tool Pattern

```turtle
ggen:ToolName a mcp:Tool ;
    rdfs:label "tool_name" ;
    mcp:description "Tool description" ;
    mcp:hasParameter ggen:ToolName_param ;
    mcp:returns "ReturnType" .
```

## SPARQL Queries

### Default Query (used if not specified)

Located at: `queries/domain_entities.sparql`

Extracts all DDD entities (Aggregates, Commands, Value Objects)

### Custom Query Example

Create `input/queries.sparql`:

```sparql
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX ddd: <http://example.org/ddd#>

SELECT ?name ?type ?description
WHERE {
    ?entity a ?type ;
           rdfs:label ?name .
    OPTIONAL { ?entity rdfs:comment ?description }

    FILTER(?type IN (ddd:AggregateRoot, ddd:Command, ddd:ValueObject))
}
ORDER BY ?type ?name
```

## Custom Templates

### Using Default Templates

If no custom templates are provided, the harness uses templates from:
- `templates/aggregate.rs.tera`
- `templates/command.rs.tera`
- `templates/value_object.rs.tera`
- `templates/mcp_tool_handler.rs.tera`

### Custom Template Example

Create `input/templates/my_aggregate.rs.tera`:

```rust
// Generated aggregate: {{ name }}

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// {{ description | default(value="Aggregate root") }}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct {{ name }} {
    {% for prop in properties %}
    pub {{ prop.name }}: {{ prop.type }},
    {% endfor %}
}

impl {{ name }} {
    pub fn new(/* params */) -> Self {
        Self {
            id: Uuid::new_v4(),
            // Initialize fields
        }
    }
}
```

## Validation Rules

The harness validates:

1. **Ontology Structure**
   - Valid Turtle syntax
   - Required properties present
   - Type consistency

2. **Generated Code**
   - Valid Rust syntax (syn parser)
   - All imports resolve
   - No unused code
   - Compiles successfully

3. **Golden Files** (if enabled)
   - Exact match (after normalization)
   - All expected files present

4. **Performance**
   - Simple fixtures: < 5 seconds
   - Complex fixtures: < 10 seconds

## Fixture Best Practices

### ✅ Do

- Use descriptive fixture names (`simple_aggregate`, `complete_domain`)
- Keep fixtures focused on one scenario
- Include comprehensive expected output
- Document fixture purpose in this README
- Use realistic domain examples

### ❌ Don't

- Create test-specific fixtures (use generic ones)
- Include generated files in git (only expected output)
- Use overly complex ontologies for simple tests
- Skip golden file validation

## Troubleshooting

### Fixture Not Found

```
Error: Fixture not found: my_fixture
```

**Solution**: Check fixture directory exists and spelling is correct

```bash
ls tests/fixtures/pipeline/my_fixture
```

### Ontology Parse Error

```
Error: Failed to parse TTL file
```

**Solution**: Validate Turtle syntax

```bash
# Use online validator or rapper tool
rapper -i turtle input/ontology.ttl
```

### Golden File Mismatch

```
Output does not match golden file
```

**Solution**: Review differences and update if correct

```bash
# See actual output
cat target/test_output/my_fixture/Entity.rs

# Update golden file if correct
cp target/test_output/my_fixture/Entity.rs \
   tests/fixtures/pipeline/my_fixture/expected/
```

### Performance Timeout

```
Pipeline should complete in under 5 seconds, took 8234 ms
```

**Solution**: Simplify fixture or increase threshold

```rust
// Increase threshold for complex fixture
assert!(total_ms < 10000);
```

## Testing Commands

```bash
# Run all pipeline tests
cargo test --test codegen_pipeline_integration_tests

# Run specific fixture
cargo test test_simple_aggregate

# Run with output
cargo test test_simple_aggregate -- --nocapture

# Update golden files
cargo test test_update_golden_files -- --ignored

# Run performance tests
cargo test test_.*_performance
```

## Contributing

When adding new fixtures:

1. Create fixture directory structure
2. Add ontology and expected output
3. Write integration test
4. Update this README with fixture description
5. Run tests to verify

## Reference

- **Harness Documentation**: `docs/TDD_CODEGEN_PIPELINE_HARNESS.md`
- **Example Usage**: `examples/codegen_pipeline_harness_example.rs`
- **Integration Tests**: `tests/codegen_pipeline_integration_tests.rs`
