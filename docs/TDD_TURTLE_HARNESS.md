# TDD Turtle/TTL Ontology Test Harness

## Overview

The Turtle Ontology Test Harness provides a comprehensive Chicago-style TDD framework for testing Turtle/TTL ontology parsing, validation, and DDD pattern compliance.

## Philosophy: Chicago-Style TDD

This harness follows the Chicago school of TDD:

- **State-Based Verification**: Tests verify the final state of the system rather than interactions
- **Real Dependencies**: Uses actual RDF stores (oxigraph) instead of mocks
- **Observable Behavior**: Focuses on what the system does, not how it does it
- **Clear Given-When-Then**: Every test follows the AAA (Arrange-Act-Assert) pattern

### Why Chicago-Style for Ontology Testing?

Ontology testing is inherently about **state and structure**:
- Does the ontology contain the expected triples?
- Are class hierarchies correct?
- Do properties have the right domains and ranges?
- Are DDD patterns properly implemented?

These are all state-based questions, making Chicago-style TDD the natural fit.

## Architecture

```
tests/
├── harness/
│   ├── mod.rs                          # Module exports
│   └── turtle_ontology_harness.rs      # Main harness implementation
├── fixtures/
│   └── ttl/
│       ├── valid/
│       │   ├── user_aggregate.ttl      # Complete user DDD aggregate
│       │   ├── order_aggregate.ttl     # E-commerce order aggregate
│       │   └── mcp_tools.ttl          # MCP tool definitions
│       └── invalid/
│           ├── syntax_error.ttl        # TTL syntax errors
│           ├── missing_properties.ttl  # DDD violations
│           ├── circular_dependencies.ttl # Cyclic hierarchies
│           ├── broken_references.ttl   # Undefined references
│           └── type_mismatches.ttl     # SHACL violations
└── turtle_harness_integration_tests.rs # Integration test suite
```

## Core Components

### 1. OntologyTestHarness

The main test harness that wraps an RDF store and provides assertion methods.

```rust
use harness::OntologyTestHarness;

// Create from file
let harness = OntologyTestHarness::parse_from_file(
    "tests/fixtures/ttl/valid/user_aggregate.ttl"
)?;

// Create from string
let harness = OntologyTestHarness::parse_from_string(r#"
    @prefix ex: <http://example.org/> .
    @prefix owl: <http://www.w3.org/2002/07/owl#> .

    ex:MyClass a owl:Class .
"#)?;

// Create empty
let harness = OntologyTestHarness::new();
```

### 2. Validation Methods

#### Consistency Validation

Checks for structural consistency:
- Cyclic class hierarchies
- Cardinality violations
- Missing required properties
- Broken references

```rust
let report = harness.validate_consistency();
assert!(report.valid, "Ontology has consistency errors: {:?}", report.errors);
```

#### Schema Validation

Validates against DDD patterns and ontology structure:
- Aggregates must have properties
- Value objects must have properties
- Invariants must have check expressions
- Repositories must reference aggregates

```rust
let report = harness.validate_schema();
assert!(report.valid, "Schema validation failed: {:?}", report.errors);
```

#### Combined Validation

```rust
let result = harness.validate();
assert!(result.is_valid(), "Ontology validation failed");
```

### 3. Query Methods

#### Get All Classes

```rust
let classes = harness.get_classes()?;
assert!(classes.contains(&"http://example.org/User".to_string()));
```

#### Get All Properties

```rust
let properties = harness.get_properties()?;
```

#### Get DDD Components

```rust
// Get aggregate roots
let aggregates = harness.get_aggregate_roots()?;

// Get value objects
let value_objects = harness.get_value_objects()?;

// Get commands
let commands = harness.get_commands()?;

// Get events
let events = harness.get_events()?;
```

#### Execute SPARQL

```rust
let query = r#"
    PREFIX ddd: <http://ggen-mcp.dev/ontology/ddd#>
    SELECT ?aggregate WHERE {
        ?aggregate rdfs:subClassOf ddd:AggregateRoot .
    }
"#;

let results = harness.query(query)?;
```

#### Count Triples

```rust
// Count all triples with rdfs:label
let count = harness.count_triples(
    None,
    Some("http://www.w3.org/2000/01/rdf-schema#label"),
    None
)?;

// Count triples from specific subject
let count = harness.count_triples(
    Some("http://example.org/User"),
    None,
    None
)?;
```

### 4. Assertion Methods

#### Class Assertions

```rust
// Assert class is defined
harness.assert_class_defined("user:User");

// Assert class is aggregate root
harness.assert_class_is_aggregate_root("user:User");

// Assert class is value object
harness.assert_class_is_value_object("user:Email");

// Assert class is command
harness.assert_class_is_command("user:CreateUserCommand");

// Assert class is event
harness.assert_class_is_event("user:UserCreatedEvent");
```

#### Property Assertions

```rust
// Assert property exists
harness.assert_property_exists("user:email");

// Assert property domain
harness.assert_property_domain("user:email", "user:User");

// Assert property range
harness.assert_property_range("user:email", "user:Email");
```

#### Triple Assertions

```rust
// Assert specific triple exists
harness.assert_triple_exists(
    "http://example.org/User",
    "http://www.w3.org/2000/01/rdf-schema#subClassOf",
    "http://ggen-mcp.dev/ontology/ddd#AggregateRoot"
);
```

#### Count Assertions

```rust
// Assert specific number of classes
harness.assert_class_count(10);

// Assert specific number of properties
harness.assert_property_count(5);
```

#### DDD Structure Assertions

```rust
// Assert aggregate has complete DDD structure
// - Must be AggregateRoot
// - Must have at least one property
harness.assert_aggregate_structure("user:User");
```

#### Validation Assertions

```rust
// Assert entire ontology is valid
harness.assert_valid();
```

### 5. OntologyBuilder

Fluent builder for constructing test ontologies programmatically.

```rust
use harness::OntologyBuilder;

let harness = OntologyBuilder::new()
    // Add aggregate root
    .add_aggregate("User")

    // Add value objects
    .add_value_object("Email")
    .add_value_object("Username")

    // Add commands
    .add_command("CreateUser")
    .add_command("UpdateUser")

    // Add events
    .add_event("UserCreated")
    .add_event("UserUpdated")

    // Add repository
    .add_repository("User", "User")

    // Add custom TTL
    .add_raw_ttl(r#"
        test:CustomProperty a owl:ObjectProperty .
    "#)

    // Build harness
    .build()?;

// Now use the harness
harness.assert_valid();
```

#### Custom Prefixes

```rust
let harness = OntologyBuilder::new()
    .with_prefix("custom", "http://custom.example.org/")
    .add_raw_ttl(r#"
        custom:MyClass a owl:Class .
    "#)
    .build()?;
```

#### Get Generated TTL

```rust
let builder = OntologyBuilder::new()
    .add_aggregate("User")
    .add_value_object("Email");

let ttl = builder.build_ttl();
println!("{}", ttl);
```

## Test Patterns

### Pattern 1: Valid Ontology Test

```rust
#[test]
fn test_user_aggregate_is_valid() {
    // GIVEN: A valid user aggregate ontology
    let harness = OntologyTestHarness::parse_from_file(
        "tests/fixtures/ttl/valid/user_aggregate.ttl"
    ).expect("Failed to parse");

    // WHEN: We validate the ontology
    let result = harness.validate();

    // THEN: It should be valid
    assert!(result.is_valid(), "Errors: {:?}", result.errors());

    // AND: It should have expected structure
    harness.assert_class_is_aggregate_root("user:User");
    harness.assert_class_is_value_object("user:Email");
    harness.assert_class_is_command("user:CreateUserCommand");
    harness.assert_class_is_event("user:UserCreatedEvent");
}
```

### Pattern 2: Invalid Ontology Test

```rust
#[test]
fn test_circular_dependency_detection() {
    // GIVEN: An ontology with circular class hierarchy
    let harness = OntologyTestHarness::parse_from_file(
        "tests/fixtures/ttl/invalid/circular_dependencies.ttl"
    ).expect("Should parse");

    // WHEN: We validate consistency
    let result = harness.validate_consistency();

    // THEN: It should detect the cycle
    assert!(!result.valid, "Should detect circular dependencies");

    let has_cycle_error = result.errors.iter()
        .any(|e| e.contains("cycle") || e.contains("Cyclic"));

    assert!(has_cycle_error, "Should report cyclic hierarchy");
}
```

### Pattern 3: Syntax Error Test

```rust
#[test]
fn test_syntax_error_detection() {
    // GIVEN: An ontology with syntax errors
    // WHEN: We try to parse it
    let result = OntologyTestHarness::parse_from_file(
        "tests/fixtures/ttl/invalid/syntax_error.ttl"
    );

    // THEN: It should fail to parse
    assert!(result.is_err(), "Should fail to parse syntax errors");
}
```

### Pattern 4: Builder-Based Test

```rust
#[test]
fn test_builder_creates_valid_aggregate() {
    // GIVEN: An ontology built with the builder
    let harness = OntologyBuilder::new()
        .add_aggregate("Product")
        .add_value_object("SKU")
        .add_command("CreateProduct")
        .build()
        .expect("Failed to build");

    // WHEN: We validate it
    let result = harness.validate();

    // THEN: It should be valid
    assert!(result.is_valid());

    // AND: All components should be present
    harness.assert_class_is_aggregate_root("test:Product");
    harness.assert_class_is_value_object("test:SKU");
    harness.assert_class_is_command("test:CreateProductCommand");
}
```

### Pattern 5: SPARQL Query Test

```rust
#[test]
fn test_query_value_objects() {
    // GIVEN: A user aggregate ontology
    let harness = OntologyTestHarness::parse_from_file(
        "tests/fixtures/ttl/valid/user_aggregate.ttl"
    ).expect("Failed to parse");

    // WHEN: We query for value objects
    let query = r#"
        PREFIX ddd: <http://ggen-mcp.dev/ontology/ddd#>
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

        SELECT ?vo ?label WHERE {
            ?vo rdfs:subClassOf ddd:ValueObject ;
                rdfs:label ?label .
        }
    "#;

    let results = harness.query(query).expect("Query failed");

    // THEN: Should find value objects
    if let QueryResults::Solutions(solutions) = results {
        let count = solutions.count();
        assert!(count >= 3, "Should find at least 3 value objects");
    }
}
```

### Pattern 6: Property Domain/Range Test

```rust
#[test]
fn test_property_structure() {
    // GIVEN: A user aggregate
    let harness = OntologyTestHarness::parse_from_file(
        "tests/fixtures/ttl/valid/user_aggregate.ttl"
    ).expect("Failed to parse");

    // THEN: Properties should have correct domains and ranges
    harness.assert_property_domain("user:email", "user:User");
    harness.assert_property_range("user:email", "user:Email");

    harness.assert_property_domain("user:userId", "user:User");
    harness.assert_property_range("user:userId", "user:UserId");
}
```

### Pattern 7: DDD Compliance Test

```rust
#[test]
fn test_aggregate_ddd_compliance() {
    // GIVEN: A user aggregate
    let harness = OntologyTestHarness::parse_from_file(
        "tests/fixtures/ttl/valid/user_aggregate.ttl"
    ).expect("Failed to parse");

    // THEN: Should have complete DDD structure
    harness.assert_aggregate_structure("user:User");

    // AND: Should have invariants
    let query = r#"
        PREFIX ddd: <http://ggen-mcp.dev/ontology/ddd#>
        PREFIX user: <http://ggen-mcp.dev/domain/user#>

        SELECT ?invariant WHERE {
            user:User ddd:hasInvariant ?invariant .
        }
    "#;

    let results = harness.query(query).expect("Query failed");
    if let QueryResults::Solutions(solutions) = results {
        assert!(solutions.count() >= 2, "Should have invariants");
    }
}
```

## Test Fixtures

### Valid Ontologies

#### `user_aggregate.ttl`

Complete user aggregate demonstrating:
- Aggregate root (User)
- Value objects (Email, Username, UserId, UserProfile)
- Commands (CreateUser, UpdateUserProfile, ChangeEmail, DeleteUser)
- Events (UserCreated, UserProfileUpdated, EmailChanged, UserDeleted)
- Repository (UserRepository)
- Invariants with validation
- SHACL shapes for constraint validation

**Use for**: Testing complete DDD patterns, validation, SHACL constraints

#### `order_aggregate.ttl`

E-commerce order aggregate demonstrating:
- Aggregate root (Order)
- Entities (LineItem)
- Value objects (Money, Quantity, OrderId, OrderStatus)
- Commands (PlaceOrder, AddLineItem, ConfirmOrder, CancelOrder)
- Events (OrderPlaced, OrderConfirmed, OrderCancelled)
- Business invariants (order must have items, total must match)

**Use for**: Testing entities within aggregates, business rules, e-commerce patterns

#### `mcp_tools.ttl`

MCP tool definitions demonstrating:
- Tool definitions with schemas
- Resource definitions
- Prompt definitions with arguments
- SHACL validation for tools

**Use for**: Testing MCP-specific structures, tool validation

### Invalid Ontologies

#### `syntax_error.ttl`

Contains intentional syntax errors:
- Unclosed strings
- Missing dots
- Invalid URIs
- Malformed triples

**Use for**: Testing parse error handling

#### `missing_properties.ttl`

DDD pattern violations:
- Aggregate without properties
- Value object without properties
- Command without properties
- Repository without forAggregate
- Invariant without check expression

**Use for**: Testing DDD compliance validation

#### `circular_dependencies.ttl`

Cyclic hierarchies:
- A → B → C → A class cycle
- Self-referential class
- Circular property references

**Use for**: Testing cycle detection

#### `broken_references.ttl`

Undefined references:
- Reference to undefined parent class
- Property with undefined domain
- Repository for non-existent aggregate
- Instance of undefined class

**Use for**: Testing reference integrity

#### `type_mismatches.ttl`

SHACL constraint violations:
- Missing required property (minCount)
- Multiple values (maxCount)
- Pattern mismatch (email format)
- Datatype mismatch (age as string)
- Range violation (age > 150)

**Use for**: Testing SHACL validation

## Coverage: 80/20 Principle

The harness covers the 80% of use cases that matter most:

### Covered (80%)

✅ **DDD Patterns**
- Aggregate roots with properties
- Value objects with validation
- Commands and events
- Repositories linked to aggregates
- Domain services

✅ **MCP Structures**
- Tools with input schemas
- Resources with URIs
- Prompts with arguments

✅ **Validation**
- Syntax errors
- Missing properties
- Circular dependencies
- Broken references
- SHACL constraints

✅ **Queries**
- Get all classes
- Get all properties
- Get DDD components
- Execute SPARQL
- Count triples

✅ **Assertions**
- Class existence and type
- Property domain/range
- Triple existence
- DDD structure compliance
- Validation success

### Not Covered (20%)

❌ **Advanced OWL Reasoning**
- Property chains
- Complex class expressions
- Inverse properties
- Symmetric/Transitive properties

❌ **Performance Testing**
- Large ontology handling
- Query optimization
- Memory usage

❌ **Ontology Merging**
- Conflict resolution strategies
- Import handling
- Version compatibility

These can be added if needed, but the core 80% provides comprehensive testing capability.

## Best Practices

### 1. Use Fixtures for Complex Ontologies

```rust
// ✅ GOOD: Use pre-built fixtures
let harness = OntologyTestHarness::parse_from_file(
    "tests/fixtures/ttl/valid/user_aggregate.ttl"
)?;

// ❌ AVOID: Inline complex TTL in tests
let harness = OntologyTestHarness::parse_from_string(r#"
    @prefix user: <http://...> .
    # 100 lines of TTL...
"#)?;
```

### 2. Use Builder for Simple Test Cases

```rust
// ✅ GOOD: Use builder for simple, focused tests
let harness = OntologyBuilder::new()
    .add_aggregate("User")
    .add_value_object("Email")
    .build()?;

// Test specific behavior
harness.assert_aggregate_structure("test:User");
```

### 3. Test One Concept Per Test

```rust
// ✅ GOOD: Focused test
#[test]
fn test_user_has_email_property() {
    let harness = load_user_aggregate();
    harness.assert_property_exists("user:email");
    harness.assert_property_domain("user:email", "user:User");
}

// ❌ AVOID: Testing everything
#[test]
fn test_entire_user_aggregate() {
    // Tests 20 different things...
}
```

### 4. Use Descriptive Test Names

```rust
// ✅ GOOD: Clear what is tested
#[test]
fn test_circular_class_hierarchy_is_detected() { }

#[test]
fn test_aggregate_without_properties_fails_validation() { }

// ❌ AVOID: Vague names
#[test]
fn test_validation() { }

#[test]
fn test_ontology() { }
```

### 5. Follow Given-When-Then

```rust
#[test]
fn test_example() {
    // GIVEN: Setup - what we start with
    let harness = OntologyTestHarness::parse_from_file("...")?;

    // WHEN: Action - what we do
    let result = harness.validate();

    // THEN: Assertion - what we expect
    assert!(result.is_valid());
}
```

### 6. Test Both Happy and Sad Paths

```rust
// ✅ GOOD: Test valid case
#[test]
fn test_valid_aggregate_passes_validation() { }

// ✅ GOOD: Test invalid case
#[test]
fn test_aggregate_without_properties_fails_validation() { }
```

### 7. Use Helpful Assertion Messages

```rust
// ✅ GOOD: Clear error messages
assert!(
    result.is_valid(),
    "User aggregate should be valid. Errors: {:?}",
    result.errors()
);

// ❌ AVOID: No context
assert!(result.is_valid());
```

## Running Tests

```bash
# Run all harness tests
cargo test turtle

# Run specific test
cargo test test_user_aggregate_is_valid

# Run with output
cargo test turtle -- --nocapture

# Run integration tests only
cargo test --test turtle_harness_integration_tests
```

## Extending the Harness

### Adding New Assertion Methods

```rust
impl OntologyTestHarness {
    /// Assert that a class has a specific annotation
    pub fn assert_class_has_annotation(&self, class_uri: &str, annotation: &str) {
        // Implementation...
    }
}
```

### Adding New Builder Methods

```rust
impl OntologyBuilder {
    /// Add a domain service
    pub fn add_service(mut self, name: &str) -> Self {
        let ttl = format!(
            r#"
            test:{name}Service a owl:Class ;
                rdfs:subClassOf ddd:Service ;
                rdfs:label "{name} Service"@en .
            "#,
            name = name
        );
        self.ttl_parts.push(ttl);
        self
    }
}
```

### Adding New Fixtures

1. Create TTL file in `tests/fixtures/ttl/valid/` or `invalid/`
2. Document what it tests in this README
3. Add integration test using the fixture

## Troubleshooting

### Parse Errors

**Problem**: "Failed to parse TTL file"

**Solution**: Check TTL syntax:
- All statements end with `.`
- Strings are properly quoted
- Prefixes are defined before use
- URIs are valid

### Assertion Failures

**Problem**: "Expected class to be defined, but it was not found"

**Solution**: Check URI format:
- Use full URIs or registered prefixes
- URIs are case-sensitive
- Check namespace is correct

### Query Errors

**Problem**: SPARQL query fails

**Solution**:
- Verify PREFIX declarations
- Check URI syntax
- Ensure variables start with `?`
- Test query in isolation

## References

- [Turtle Specification](https://www.w3.org/TR/turtle/)
- [SHACL Specification](https://www.w3.org/TR/shacl/)
- [Oxigraph Documentation](https://docs.rs/oxigraph/)
- [Domain-Driven Design](https://www.domainlanguage.com/ddd/)
- [Chicago-Style TDD](http://www.mockobjects.com/2007/04/test-driven-development-is-not-about.html)

## License

Apache 2.0 - Same as parent project
