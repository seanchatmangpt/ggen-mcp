# SHACL Validation in ggen-mcp

## Table of Contents

- [Introduction](#introduction)
- [SHACL Basics](#shacl-basics)
- [Architecture](#architecture)
- [Shape Definition Patterns](#shape-definition-patterns)
- [Validation Workflow](#validation-workflow)
- [Constraint Types](#constraint-types)
- [Custom Constraints](#custom-constraints)
- [Error Interpretation](#error-interpretation)
- [Performance Tuning](#performance-tuning)
- [Best Practices](#best-practices)
- [Examples](#examples)

## Introduction

SHACL (Shapes Constraint Language) is a W3C standard for validating RDF graphs against a set of conditions. In ggen-mcp, SHACL validation ensures that:

- **MCP Tools** conform to naming conventions and structural requirements
- **MCP Resources** have valid URIs and MIME types
- **DDD Aggregates** follow domain-driven design patterns
- **Repositories** are properly associated with aggregate roots
- **Business Rules** (invariants) are enforced

### Why SHACL?

1. **Declarative Validation** - Define constraints in RDF, not code
2. **Standard Compliance** - W3C standard, interoperable with other tools
3. **Semantic Awareness** - Understands ontology structure and relationships
4. **Detailed Reporting** - Precise error messages with focus nodes and paths
5. **Extensibility** - Support for custom constraint types

## SHACL Basics

### Key Concepts

#### 1. Shapes (`sh:NodeShape`)

A shape defines validation rules for a class of nodes. Example:

```turtle
mcp:ToolShape a sh:NodeShape ;
    sh:targetClass mcp:Tool ;
    sh:name "MCP Tool Shape" ;
    sh:description "Validates MCP Tool definitions" .
```

#### 2. Target Selectors

Shapes specify which nodes they apply to:

- `sh:targetClass` - All instances of a class
- `sh:targetNode` - Specific named node
- `sh:targetSubjectsOf` - Subjects of a predicate
- `sh:targetObjectsOf` - Objects of a predicate

#### 3. Property Constraints (`sh:property`)

Define constraints on property values:

```turtle
sh:property [
    sh:path mcp:name ;
    sh:datatype xsd:string ;
    sh:minCount 1 ;
    sh:maxCount 1 ;
    sh:pattern "^[a-z][a-z0-9_]*$" ;
    sh:message "Tool name must be lowercase snake_case"
] .
```

#### 4. Severity Levels

- `sh:Violation` - Critical errors (default)
- `sh:Warning` - Issues that should be addressed
- `sh:Info` - Informational messages

## Architecture

### Component Overview

```
ShapeValidator
├── ShapeDiscovery      - Find applicable shapes for nodes
├── ConstraintChecker   - Validate individual constraints
├── CustomConstraints   - Domain-specific business rules
└── ValidationReport    - Collect and serialize results
```

### Data Flow

```
┌─────────────────┐
│  Shapes File    │
│ (shapes.ttl)    │
└────────┬────────┘
         │
         v
┌─────────────────┐     ┌─────────────────┐
│ ShapeValidator  │────>│ ShapeDiscovery  │
└────────┬────────┘     └─────────────────┘
         │                       │
         │                       v
         │              ┌─────────────────┐
         │              │ Find Applicable │
         │              │     Shapes      │
         │              └────────┬────────┘
         │                       │
         v                       v
┌─────────────────┐     ┌─────────────────┐
│   Data Graph    │────>│ConstraintChecker│
└─────────────────┘     └────────┬────────┘
                                 │
                                 v
                        ┌─────────────────┐
                        │CustomConstraints│
                        └────────┬────────┘
                                 │
                                 v
                        ┌─────────────────┐
                        │ValidationReport │
                        └─────────────────┘
```

## Shape Definition Patterns

### 1. Required Properties

Ensure properties are present:

```turtle
sh:property [
    sh:path ddd:hasProperty ;
    sh:minCount 1 ;
    sh:message "Aggregate must have at least one property"
] .
```

### 2. Cardinality Constraints

Control how many values a property can have:

```turtle
sh:property [
    sh:path mcp:name ;
    sh:minCount 1 ;    # Required
    sh:maxCount 1      # Single-valued
] .
```

### 3. Datatype Validation

Ensure values have the correct type:

```turtle
sh:property [
    sh:path mcp:name ;
    sh:datatype xsd:string
] .
```

### 4. Pattern Matching

Validate string format with regex:

```turtle
sh:property [
    sh:path mcp:name ;
    sh:pattern "^[a-z][a-z0-9_]*$" ;
    sh:message "Tool name must be lowercase snake_case, 1-64 characters"
] .
```

### 5. String Length

Control text length:

```turtle
sh:property [
    sh:path mcp:description ;
    sh:minLength 10 ;
    sh:maxLength 500
] .
```

### 6. Numeric Ranges

Validate numeric values:

```turtle
sh:property [
    sh:path ex:priority ;
    sh:datatype xsd:integer ;
    sh:minInclusive 1 ;
    sh:maxInclusive 10
] .
```

### 7. Enumeration

Restrict to specific values:

```turtle
sh:property [
    sh:path ex:status ;
    sh:in ( "draft" "published" "archived" )
] .
```

### 8. Class Constraints

Ensure object values are instances of a class:

```turtle
sh:property [
    sh:path ddd:forAggregate ;
    sh:class ddd:AggregateRoot ;
    sh:minCount 1 ;
    sh:maxCount 1
] .
```

### 9. Unique Language Tags

Ensure language-tagged strings are unique:

```turtle
sh:property [
    sh:path rdfs:label ;
    sh:uniqueLang true
] .
```

## Validation Workflow

### Basic Usage

```rust
use spreadsheet_mcp::ontology::shacl::ShapeValidator;

// 1. Load shapes
let validator = ShapeValidator::from_file("ontology/shapes.ttl")?;

// 2. Load data
let data_store = validator.load_data_from_file("data/model.ttl")?;

// 3. Validate
let report = validator.validate_graph(&data_store)?;

// 4. Check results
if !report.conforms() {
    for violation in report.violations() {
        eprintln!("Error at {}: {}",
            violation.focus_node(),
            violation.message()
        );
    }
}
```

### Validate Specific Node

```rust
use oxigraph::model::NamedNode;

let node = NamedNode::new("http://example.org/tools/my_tool")?;
let results = validator.validate_node(&node, &data_store)?;

for result in results {
    println!("{:?}", result);
}
```

### JSON Output

```rust
let json = report.to_json()?;
std::fs::write("validation_report.json", json)?;
```

## Constraint Types

### Core SHACL Constraints (Implemented)

| Constraint | Purpose | Example |
|------------|---------|---------|
| `sh:class` | Type checking | Object must be instance of class |
| `sh:datatype` | Literal type | Value must be `xsd:string` |
| `sh:minCount` | Min cardinality | At least 1 value |
| `sh:maxCount` | Max cardinality | At most 1 value |
| `sh:pattern` | Regex match | `^[a-z]+$` |
| `sh:minLength` | String min | At least 10 chars |
| `sh:maxLength` | String max | At most 500 chars |
| `sh:minInclusive` | Numeric min | >= 0 |
| `sh:maxInclusive` | Numeric max | <= 100 |
| `sh:in` | Enumeration | One of [A, B, C] |
| `sh:uniqueLang` | Language tags | No duplicate langs |

### Target Selectors (Implemented)

| Selector | Purpose |
|----------|---------|
| `sh:targetClass` | All instances of a class |
| `sh:targetNode` | Specific node |
| `sh:targetSubjectsOf` | Subjects with predicate |
| `sh:targetObjectsOf` | Objects with predicate |

## Custom Constraints

### DDD Invariant Checking

The validator supports custom DDD invariants via `ddd:hasInvariant`:

```turtle
ex:OrderAggregate a ddd:AggregateRoot ;
    rdfs:label "OrderAggregate" ;
    ddd:hasInvariant "total >= 0" ;
    ddd:hasInvariant "items.length > 0" .
```

These are checked by `CustomConstraints::check_ddd_invariants()`.

### Cross-Property Constraints

Business rules spanning multiple properties:

```rust
impl CustomConstraints {
    pub fn check_cross_property_constraints(
        &self,
        focus_node: &NamedNode,
        shape_id: &str,
    ) -> Vec<ValidationResult> {
        // Example: Repository must have forAggregate
        // Implementation in src/ontology/shacl.rs
    }
}
```

### Extending with Custom Validators

To add new custom constraints:

1. **Define the constraint in your ontology:**

```turtle
@prefix custom: <http://example.org/constraints#> .

ex:MyShape a sh:NodeShape ;
    custom:businessRule "validateOrderTotal" .
```

2. **Implement the validator:**

```rust
impl CustomConstraints {
    pub fn validate_order_total(
        &self,
        focus_node: &NamedNode,
        shape_id: &str,
    ) -> Vec<ValidationResult> {
        let mut results = Vec::new();
        // Your validation logic here
        results
    }
}
```

3. **Register in the validation flow:**

Update `ShapeValidator::validate_against_shape()` to call your custom validator.

## Error Interpretation

### Violation Structure

```json
{
  "focus_node": "http://example.org/tools/bad_tool",
  "result_path": "http://ggen-mcp.dev/mcp#name",
  "value": "InvalidName",
  "message": "Tool name must be lowercase snake_case, 1-64 characters",
  "severity": "Violation",
  "source_shape": "http://ggen-mcp.dev/mcp#ToolShape",
  "source_constraint": "sh:pattern"
}
```

### Common Error Patterns

#### Pattern Mismatch

```
Tool name must be lowercase snake_case, 1-64 characters
```

**Cause:** Value doesn't match regex pattern
**Fix:** Rename to match `^[a-z][a-z0-9_]*$`

#### Cardinality Violation

```
Property mcp:description must have at least 1 value(s)
```

**Cause:** Required property missing
**Fix:** Add the missing property

#### Length Constraint

```
Tool description must be 10-500 characters
```

**Cause:** String too short or long
**Fix:** Adjust description length

#### Type Mismatch

```
Value must have datatype http://www.w3.org/2001/XMLSchema#string
```

**Cause:** Wrong literal datatype
**Fix:** Use correct datatype in data

#### Class Constraint

```
Value must be an instance of ddd:AggregateRoot
```

**Cause:** Referenced object not of expected type
**Fix:** Ensure object has correct `rdf:type`

## Performance Tuning

### Optimization Strategies

#### 1. Batch Validation

Validate entire graph once, not node-by-node:

```rust
// Good - Efficient
let report = validator.validate_graph(&data_store)?;

// Avoid - Inefficient for many nodes
for node in nodes {
    validator.validate_node(&node, &data_store)?;
}
```

#### 2. Shape Caching

The `ShapeDiscovery` component caches loaded shapes. Reuse validators:

```rust
// Good - Reuse validator
let validator = ShapeValidator::from_file("shapes.ttl")?;
for data_file in data_files {
    let store = validator.load_data_from_file(data_file)?;
    validator.validate_graph(&store)?;
}

// Avoid - Reload shapes each time
for data_file in data_files {
    let validator = ShapeValidator::from_file("shapes.ttl")?;
    // ...
}
```

#### 3. Selective Validation

Use specific node validation when you know what changed:

```rust
// Only validate the modified node
let modified_node = NamedNode::new("http://example.org/tools/updated")?;
let results = validator.validate_node(&modified_node, &data_store)?;
```

#### 4. Store Optimization

Use oxigraph `Store` efficiently:

```rust
// Load data once
let mut store = Store::new()?;
store.load_from_reader(RdfFormat::Turtle, file)?;

// Query multiple times
validator.validate_graph(&store)?;
// ... other operations on same store
```

### Benchmarking

Test validation performance:

```rust
use std::time::Instant;

let start = Instant::now();
let report = validator.validate_graph(&data_store)?;
let duration = start.elapsed();

println!("Validated {} nodes in {:?}",
    report.results().len(),
    duration
);
```

## Best Practices

### 1. Shape Organization

Organize shapes by domain:

```
ontology/
├── shapes.ttl              # Main shapes file
├── mcp/
│   ├── tool_shapes.ttl     # MCP tool constraints
│   └── resource_shapes.ttl # MCP resource constraints
└── ddd/
    ├── aggregate_shapes.ttl
    ├── repository_shapes.ttl
    └── service_shapes.ttl
```

### 2. Clear Error Messages

Always provide helpful `sh:message`:

```turtle
# Good
sh:property [
    sh:path mcp:name ;
    sh:pattern "^[a-z][a-z0-9_]*$" ;
    sh:message "Tool name must be lowercase snake_case (e.g., 'list_workbooks')"
] .

# Avoid - Generic message
sh:property [
    sh:path mcp:name ;
    sh:pattern "^[a-z][a-z0-9_]*$"
] .
```

### 3. Progressive Validation

Start with basic shapes, add complexity:

```turtle
# Version 1: Basic validation
mcp:ToolShape a sh:NodeShape ;
    sh:targetClass mcp:Tool ;
    sh:property [
        sh:path mcp:name ;
        sh:minCount 1
    ] .

# Version 2: Add constraints
mcp:ToolShape a sh:NodeShape ;
    sh:targetClass mcp:Tool ;
    sh:property [
        sh:path mcp:name ;
        sh:minCount 1 ;
        sh:maxCount 1 ;
        sh:datatype xsd:string
    ] .

# Version 3: Add patterns
mcp:ToolShape a sh:NodeShape ;
    sh:targetClass mcp:Tool ;
    sh:property [
        sh:path mcp:name ;
        sh:minCount 1 ;
        sh:maxCount 1 ;
        sh:datatype xsd:string ;
        sh:pattern "^[a-z][a-z0-9_]*$" ;
        sh:minLength 1 ;
        sh:maxLength 64
    ] .
```

### 4. Test Shapes

Create test data for each shape:

```
tests/
├── fixtures/
│   ├── valid/
│   │   ├── tool_valid.ttl
│   │   └── aggregate_valid.ttl
│   └── invalid/
│       ├── tool_invalid_name.ttl
│       ├── tool_missing_description.ttl
│       └── aggregate_no_properties.ttl
```

### 5. Version Shapes

Include versioning metadata:

```turtle
@prefix dcterms: <http://purl.org/dc/terms/> .

mcp:ToolShape a sh:NodeShape ;
    dcterms:created "2025-01-20"^^xsd:date ;
    dcterms:modified "2025-01-20"^^xsd:date ;
    dcterms:version "1.0" ;
    sh:targetClass mcp:Tool .
```

## Examples

### Example 1: Validate Generated Code

```rust
use spreadsheet_mcp::ontology::shacl::ShapeValidator;

fn validate_generated_model(model_path: &str) -> Result<()> {
    let validator = ShapeValidator::from_file("ontology/shapes.ttl")?;
    let data_store = validator.load_data_from_file(model_path)?;
    let report = validator.validate_graph(&data_store)?;

    if !report.conforms() {
        eprintln!("Generated model has validation errors:");
        for violation in report.violations() {
            eprintln!("  [ERROR] {}: {}",
                violation.focus_node(),
                violation.message()
            );
        }
        return Err(anyhow::anyhow!("Validation failed"));
    }

    println!("Model is valid!");
    Ok(())
}
```

### Example 2: CI/CD Integration

```rust
#[test]
fn test_ontology_validity() {
    let validator = ShapeValidator::from_file("ontology/shapes.ttl")
        .expect("Failed to load shapes");

    let data_store = validator.load_data_from_file("ontology/domain.ttl")
        .expect("Failed to load domain ontology");

    let report = validator.validate_graph(&data_store)
        .expect("Validation failed");

    assert!(
        report.conforms(),
        "Ontology has {} violations",
        report.violations().count()
    );
}
```

### Example 3: Pre-commit Validation

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Validate ontology before commit
cargo test --test shacl_validation_tests

if [ $? -ne 0 ]; then
    echo "SHACL validation failed. Fix errors before committing."
    exit 1
fi
```

### Example 4: Runtime Validation

```rust
use spreadsheet_mcp::ontology::shacl::ShapeValidator;

pub struct ValidatedModelLoader {
    validator: ShapeValidator,
}

impl ValidatedModelLoader {
    pub fn new() -> Result<Self> {
        Ok(Self {
            validator: ShapeValidator::from_file("ontology/shapes.ttl")?,
        })
    }

    pub fn load_and_validate(&self, path: &str) -> Result<Store> {
        let store = self.validator.load_data_from_file(path)?;
        let report = self.validator.validate_graph(&store)?;

        if !report.conforms() {
            return Err(anyhow::anyhow!(
                "Model validation failed with {} violations",
                report.violations().count()
            ));
        }

        Ok(store)
    }
}
```

## Troubleshooting

### Shapes Not Applied

**Problem:** Validation passes but should fail

**Solutions:**
1. Check target selectors match your data
2. Verify RDF types are correctly set
3. Ensure shapes file is loaded correctly

### Too Many Violations

**Problem:** Validation reports many similar errors

**Solutions:**
1. Fix root cause (e.g., naming convention)
2. Use shape inheritance for common patterns
3. Adjust severity to Warning for non-critical issues

### Performance Issues

**Problem:** Validation is slow

**Solutions:**
1. Profile with larger datasets
2. Optimize shape complexity
3. Use selective validation
4. Cache validator instances

## Resources

- [W3C SHACL Specification](https://www.w3.org/TR/shacl/)
- [SHACL Playground](https://shacl.org/playground/)
- [Oxigraph Documentation](https://github.com/oxigraph/oxigraph)
- [RDF 1.1 Turtle](https://www.w3.org/TR/turtle/)

## Conclusion

SHACL validation provides a robust, standards-based approach to ensuring data quality in ggen-mcp. By defining constraints declaratively in the ontology, we can:

- Catch errors early in development
- Maintain consistency across generated code
- Document domain rules alongside the model
- Integrate validation into CI/CD pipelines
- Provide clear, actionable error messages

The implementation in `src/ontology/shacl.rs` supports the core SHACL constraint types and can be extended with custom validators for domain-specific business rules.
