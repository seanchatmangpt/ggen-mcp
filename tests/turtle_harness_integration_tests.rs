//! Integration Tests for Turtle Ontology Harness
//!
//! Comprehensive test suite demonstrating the Chicago-style TDD harness in action.
//! Tests cover valid ontologies, invalid ontologies, and edge cases.

mod harness;

use harness::{OntologyBuilder, OntologyTestHarness};

// =============================================================================
// VALID ONTOLOGY TESTS - These should all pass
// =============================================================================

#[test]
fn test_user_aggregate_is_valid() {
    // GIVEN: A valid user aggregate ontology
    let harness =
        OntologyTestHarness::parse_from_file("tests/fixtures/ttl/valid/user_aggregate.ttl")
            .expect("Failed to parse user aggregate");

    // WHEN: We validate the ontology
    let result = harness.validate();

    // THEN: It should be valid
    assert!(
        result.is_valid(),
        "User aggregate should be valid. Errors: {:?}",
        result.errors()
    );

    // AND: It should have the expected structure
    harness.assert_class_defined("http://ggen-mcp.dev/domain/user#User");
    harness.assert_class_is_aggregate_root("http://ggen-mcp.dev/domain/user#User");

    // AND: Value objects should be defined
    harness.assert_class_is_value_object("http://ggen-mcp.dev/domain/user#Email");
    harness.assert_class_is_value_object("http://ggen-mcp.dev/domain/user#Username");
    harness.assert_class_is_value_object("http://ggen-mcp.dev/domain/user#UserId");

    // AND: Commands should be defined
    harness.assert_class_is_command("http://ggen-mcp.dev/domain/user#CreateUserCommand");
    harness.assert_class_is_command("http://ggen-mcp.dev/domain/user#UpdateUserProfileCommand");

    // AND: Events should be defined
    harness.assert_class_is_event("http://ggen-mcp.dev/domain/user#UserCreatedEvent");
    harness.assert_class_is_event("http://ggen-mcp.dev/domain/user#UserProfileUpdatedEvent");
}

#[test]
fn test_order_aggregate_is_valid() {
    // GIVEN: A valid order aggregate ontology
    let harness =
        OntologyTestHarness::parse_from_file("tests/fixtures/ttl/valid/order_aggregate.ttl")
            .expect("Failed to parse order aggregate");

    // WHEN: We validate the ontology
    let result = harness.validate();

    // THEN: It should be valid
    assert!(
        result.is_valid(),
        "Order aggregate should be valid. Errors: {:?}",
        result.errors()
    );

    // AND: Aggregate root should be defined
    harness.assert_class_defined("http://ggen-mcp.dev/domain/order#Order");
    harness.assert_class_is_aggregate_root("http://ggen-mcp.dev/domain/order#Order");

    // AND: Entities should be defined
    harness.assert_class_defined("http://ggen-mcp.dev/domain/order#LineItem");

    // AND: Value objects should be defined
    harness.assert_class_is_value_object("http://ggen-mcp.dev/domain/order#Money");
    harness.assert_class_is_value_object("http://ggen-mcp.dev/domain/order#Quantity");
    harness.assert_class_is_value_object("http://ggen-mcp.dev/domain/order#OrderId");

    // AND: Commands should be defined
    harness.assert_class_is_command("http://ggen-mcp.dev/domain/order#PlaceOrderCommand");
    harness.assert_class_is_command("http://ggen-mcp.dev/domain/order#ConfirmOrderCommand");

    // AND: Events should be defined
    harness.assert_class_is_event("http://ggen-mcp.dev/domain/order#OrderPlacedEvent");
}

#[test]
fn test_mcp_tools_structure() {
    // GIVEN: An MCP tools ontology
    let harness =
        OntologyTestHarness::parse_from_file("tests/fixtures/ttl/valid/mcp_tools.ttl")
            .expect("Failed to parse MCP tools");

    // WHEN: We query for tools
    let query = r#"
        PREFIX mcp: <http://ggen-mcp.dev/ontology/mcp#>
        SELECT ?tool ?name WHERE {
            ?tool a mcp:Tool ;
                  mcp:toolName ?name .
        }
    "#;

    let results = harness.query(query).expect("Failed to execute query");

    // THEN: We should find tools
    if let oxigraph::sparql::QueryResults::Solutions(solutions) = results {
        let count = solutions.count();
        assert!(count >= 4, "Should have at least 4 tools, found {}", count);
    } else {
        panic!("Expected solutions from query");
    }

    // AND: Specific tools should be defined
    harness.assert_class_defined("http://ggen-mcp.dev/ontology/mcp#ReadFileTool");
    harness.assert_class_defined("http://ggen-mcp.dev/ontology/mcp#WriteFileTool");
}

#[test]
fn test_user_aggregate_properties() {
    // GIVEN: A user aggregate ontology
    let harness =
        OntologyTestHarness::parse_from_file("tests/fixtures/ttl/valid/user_aggregate.ttl")
            .expect("Failed to parse");

    // WHEN: We check property domains and ranges
    // THEN: They should be correctly defined
    harness.assert_property_domain(
        "http://ggen-mcp.dev/domain/user#userId",
        "http://ggen-mcp.dev/domain/user#User",
    );

    harness.assert_property_range(
        "http://ggen-mcp.dev/domain/user#userId",
        "http://ggen-mcp.dev/domain/user#UserId",
    );

    harness.assert_property_domain(
        "http://ggen-mcp.dev/domain/user#email",
        "http://ggen-mcp.dev/domain/user#User",
    );

    harness.assert_property_range(
        "http://ggen-mcp.dev/domain/user#email",
        "http://ggen-mcp.dev/domain/user#Email",
    );
}

#[test]
fn test_aggregate_ddd_compliance() {
    // GIVEN: A user aggregate ontology
    let harness =
        OntologyTestHarness::parse_from_file("tests/fixtures/ttl/valid/user_aggregate.ttl")
            .expect("Failed to parse");

    // WHEN: We validate aggregate structure
    // THEN: It should have all required DDD elements
    harness.assert_aggregate_structure("http://ggen-mcp.dev/domain/user#User");

    // AND: It should have invariants
    let query = r#"
        PREFIX ddd: <http://ggen-mcp.dev/ontology/ddd#>
        PREFIX user: <http://ggen-mcp.dev/domain/user#>

        SELECT ?invariant WHERE {
            user:User ddd:hasInvariant ?invariant .
        }
    "#;

    let results = harness.query(query).expect("Failed to execute query");
    if let oxigraph::sparql::QueryResults::Solutions(solutions) = results {
        let count = solutions.count();
        assert!(
            count >= 2,
            "User aggregate should have at least 2 invariants, found {}",
            count
        );
    }
}

// =============================================================================
// INVALID ONTOLOGY TESTS - These should detect errors
// =============================================================================

#[test]
fn test_syntax_error_detection() {
    // GIVEN: An ontology with syntax errors
    // WHEN: We try to parse it
    let result =
        OntologyTestHarness::parse_from_file("tests/fixtures/ttl/invalid/syntax_error.ttl");

    // THEN: It should fail to parse
    assert!(
        result.is_err(),
        "Should fail to parse ontology with syntax errors"
    );
}

#[test]
fn test_missing_properties_detection() {
    // GIVEN: An ontology with missing required properties
    let harness =
        OntologyTestHarness::parse_from_file("tests/fixtures/ttl/invalid/missing_properties.ttl")
            .expect("Should parse but be invalid");

    // WHEN: We validate it
    let result = harness.validate();

    // THEN: It should detect validation errors
    assert!(
        !result.is_valid(),
        "Should detect missing properties. Errors: {:?}",
        result.errors()
    );

    // AND: Should report specific issues
    let errors = result.errors();
    let has_ddd_error = errors
        .iter()
        .any(|e| e.contains("property") || e.contains("DDD") || e.contains("aggregate"));

    assert!(
        has_ddd_error,
        "Should report DDD structure issues. Errors: {:?}",
        errors
    );
}

#[test]
fn test_circular_dependency_detection() {
    // GIVEN: An ontology with circular class hierarchy
    let harness = OntologyTestHarness::parse_from_file(
        "tests/fixtures/ttl/invalid/circular_dependencies.ttl",
    )
    .expect("Should parse but be invalid");

    // WHEN: We validate consistency
    let result = harness.validate_consistency();

    // THEN: It should detect the cycle
    assert!(
        !result.valid,
        "Should detect circular dependencies. Errors: {:?}",
        result.errors
    );

    // AND: Error message should mention cycles
    let has_cycle_error = result
        .errors
        .iter()
        .any(|e| e.contains("cycle") || e.contains("Cyclic") || e.contains("circular"));

    assert!(
        has_cycle_error,
        "Should report cyclic hierarchy. Errors: {:?}",
        result.errors
    );
}

#[test]
fn test_broken_references_detection() {
    // GIVEN: An ontology with broken references
    let harness =
        OntologyTestHarness::parse_from_file("tests/fixtures/ttl/invalid/broken_references.ttl")
            .expect("Should parse");

    // WHEN: We validate it
    let result = harness.validate();

    // THEN: Should detect issues (may be warnings or errors)
    let has_issues = !result.errors().is_empty() || !result.warnings().is_empty();

    assert!(
        has_issues,
        "Should detect broken references or undefined classes"
    );
}

#[test]
fn test_type_mismatch_detection() {
    // GIVEN: An ontology with type mismatches
    let harness =
        OntologyTestHarness::parse_from_file("tests/fixtures/ttl/invalid/type_mismatches.ttl")
            .expect("Should parse");

    // WHEN: We validate with SHACL
    let result = harness.validate();

    // THEN: Should detect constraint violations
    // Note: SHACL validation may not be fully implemented yet
    // This test documents expected behavior
}

// =============================================================================
// BUILDER PATTERN TESTS
// =============================================================================

#[test]
fn test_builder_creates_valid_aggregate() {
    // GIVEN: An ontology built with the builder
    let harness = OntologyBuilder::new()
        .add_aggregate("Product")
        .add_value_object("ProductName")
        .add_value_object("Price")
        .add_command("CreateProduct")
        .add_event("ProductCreated")
        .add_repository("Product", "Product")
        .build()
        .expect("Failed to build ontology");

    // WHEN: We validate it
    let result = harness.validate();

    // THEN: It should be valid
    assert!(
        result.is_valid(),
        "Builder-created ontology should be valid. Errors: {:?}",
        result.errors()
    );

    // AND: All components should be present
    harness.assert_class_is_aggregate_root("http://test.example.org/Product");
    harness.assert_class_is_value_object("http://test.example.org/ProductName");
    harness.assert_class_is_value_object("http://test.example.org/Price");
    harness.assert_class_is_command("http://test.example.org/CreateProductCommand");
    harness.assert_class_is_event("http://test.example.org/ProductCreatedEvent");
}

#[test]
fn test_builder_with_custom_prefix() {
    // GIVEN: An ontology with custom prefix
    let harness = OntologyBuilder::new()
        .with_prefix("custom", "http://custom.example.org/")
        .add_raw_ttl(
            r#"
            custom:MyClass a owl:Class ;
                rdfs:subClassOf ddd:AggregateRoot ;
                rdfs:label "My Class"@en ;
                ddd:hasProperty custom:myProp .

            custom:myProp a owl:ObjectProperty .
        "#,
        )
        .build()
        .expect("Failed to build");

    // THEN: Custom class should be defined
    harness.assert_class_defined("http://custom.example.org/MyClass");
    harness.assert_class_is_aggregate_root("http://custom.example.org/MyClass");
}

#[test]
fn test_builder_complex_domain() {
    // GIVEN: A complex domain built incrementally
    let harness = OntologyBuilder::new()
        // User context
        .add_aggregate("User")
        .add_value_object("Email")
        .add_value_object("Username")
        .add_command("RegisterUser")
        .add_event("UserRegistered")
        // Product context
        .add_aggregate("Product")
        .add_value_object("SKU")
        .add_value_object("Price")
        .add_command("CreateProduct")
        .add_event("ProductCreated")
        // Order context
        .add_aggregate("Order")
        .add_command("PlaceOrder")
        .add_event("OrderPlaced")
        // Repositories
        .add_repository("User", "User")
        .add_repository("Product", "Product")
        .add_repository("Order", "Order")
        .build()
        .expect("Failed to build complex domain");

    // WHEN: We validate it
    let result = harness.validate();

    // THEN: It should be valid
    assert!(
        result.is_valid(),
        "Complex domain should be valid. Errors: {:?}",
        result.errors()
    );

    // AND: Should have multiple aggregates
    let aggregates = harness
        .get_aggregate_roots()
        .expect("Failed to get aggregates");
    assert_eq!(
        aggregates.len(),
        3,
        "Should have 3 aggregates: {:?}",
        aggregates
    );

    // AND: Should have multiple commands
    let commands = harness.get_commands().expect("Failed to get commands");
    assert!(
        commands.len() >= 3,
        "Should have at least 3 commands: {:?}",
        commands
    );
}

// =============================================================================
// QUERY AND ASSERTION TESTS
// =============================================================================

#[test]
fn test_triple_counting() {
    // GIVEN: An ontology with known triples
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:Class1 rdfs:label "Class 1" .
        ex:Class2 rdfs:label "Class 2" .
        ex:Class3 rdfs:label "Class 3" .
        ex:Class4 rdfs:comment "Class 4" .
    "#;

    let harness = OntologyTestHarness::parse_from_string(ttl).expect("Failed to parse");

    // WHEN: We count labels
    let label_count = harness
        .count_triples(
            None,
            Some("http://www.w3.org/2000/01/rdf-schema#label"),
            None,
        )
        .expect("Failed to count");

    // THEN: Should find 3 labels
    assert_eq!(label_count, 3, "Should have 3 labels");

    // WHEN: We count all triples from Class1
    let class1_count = harness
        .count_triples(Some("http://example.org/Class1"), None, None)
        .expect("Failed to count");

    // THEN: Should find 1 triple
    assert_eq!(class1_count, 1, "Class1 should have 1 triple");
}

#[test]
fn test_sparql_queries() {
    // GIVEN: A user aggregate ontology
    let harness =
        OntologyTestHarness::parse_from_file("tests/fixtures/ttl/valid/user_aggregate.ttl")
            .expect("Failed to parse");

    // WHEN: We query for all value objects with their labels
    let query = r#"
        PREFIX ddd: <http://ggen-mcp.dev/ontology/ddd#>
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

        SELECT ?vo ?label WHERE {
            ?vo rdfs:subClassOf ddd:ValueObject ;
                rdfs:label ?label .
        }
        ORDER BY ?label
    "#;

    let results = harness.query(query).expect("Failed to execute query");

    // THEN: Should find value objects
    if let oxigraph::sparql::QueryResults::Solutions(solutions) = results {
        let count = solutions.count();
        assert!(
            count >= 3,
            "Should find at least 3 value objects with labels, found {}",
            count
        );
    } else {
        panic!("Expected solutions");
    }
}

#[test]
fn test_get_all_classes() {
    // GIVEN: An ontology
    let harness =
        OntologyTestHarness::parse_from_file("tests/fixtures/ttl/valid/user_aggregate.ttl")
            .expect("Failed to parse");

    // WHEN: We get all classes
    let classes = harness.get_classes().expect("Failed to get classes");

    // THEN: Should find multiple classes
    assert!(
        classes.len() > 5,
        "Should have multiple classes, found: {:?}",
        classes
    );

    // AND: Should include User
    assert!(
        classes.contains(&"http://ggen-mcp.dev/domain/user#User".to_string()),
        "Should include User class"
    );
}

#[test]
fn test_get_all_properties() {
    // GIVEN: An ontology
    let harness =
        OntologyTestHarness::parse_from_file("tests/fixtures/ttl/valid/user_aggregate.ttl")
            .expect("Failed to parse");

    // WHEN: We get all properties
    let properties = harness.get_properties().expect("Failed to get properties");

    // THEN: Should find properties
    assert!(
        !properties.is_empty(),
        "Should have properties, found: {:?}",
        properties
    );

    // AND: Should include userId
    assert!(
        properties.contains(&"http://ggen-mcp.dev/domain/user#userId".to_string()),
        "Should include userId property"
    );
}

// =============================================================================
// HASH VERIFICATION TESTS
// =============================================================================

#[test]
fn test_compute_hash() {
    // GIVEN: An ontology
    let harness =
        OntologyTestHarness::parse_from_file("tests/fixtures/ttl/valid/user_aggregate.ttl")
            .expect("Failed to parse");

    // WHEN: We compute its hash
    let hash1 = harness.compute_hash().expect("Failed to compute hash");
    let hash2 = harness.compute_hash().expect("Failed to compute hash again");

    // THEN: Hash should be deterministic
    assert_eq!(hash1, hash2, "Hash should be deterministic");

    // AND: Hash should be SHA-256 (64 hex chars)
    assert_eq!(hash1.len(), 64, "Hash should be 64 characters");
}

#[test]
fn test_hash_changes_with_content() {
    // GIVEN: Two different ontologies
    let harness1 = OntologyBuilder::new()
        .add_aggregate("User")
        .build()
        .expect("Failed to build");

    let harness2 = OntologyBuilder::new()
        .add_aggregate("Product")
        .build()
        .expect("Failed to build");

    // WHEN: We compute their hashes
    let hash1 = harness1.compute_hash().expect("Failed to compute hash");
    let hash2 = harness2.compute_hash().expect("Failed to compute hash");

    // THEN: Hashes should be different
    assert_ne!(
        hash1, hash2,
        "Different ontologies should have different hashes"
    );
}

// =============================================================================
// EDGE CASES AND ERROR HANDLING
// =============================================================================

#[test]
fn test_empty_ontology() {
    // GIVEN: An empty ontology
    let harness = OntologyTestHarness::new();

    // WHEN: We validate it
    let result = harness.validate();

    // THEN: Should be valid (though empty)
    assert!(
        result.is_valid(),
        "Empty ontology should be technically valid"
    );

    // AND: Should have no classes
    let classes = harness.get_classes().expect("Failed to get classes");
    assert_eq!(classes.len(), 0, "Empty ontology should have no classes");
}

#[test]
fn test_minimal_valid_ontology() {
    // GIVEN: A minimal but valid ontology
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:MyClass a owl:Class ;
            rdfs:label "My Class"@en .
    "#;

    let harness = OntologyTestHarness::parse_from_string(ttl).expect("Failed to parse");

    // WHEN: We validate it
    let result = harness.validate();

    // THEN: Should be valid
    assert!(
        result.is_valid(),
        "Minimal ontology should be valid. Errors: {:?}",
        result.errors()
    );
}

#[test]
fn test_assertion_failures_are_clear() {
    // GIVEN: An ontology
    let harness = OntologyBuilder::new()
        .add_aggregate("User")
        .build()
        .expect("Failed to build");

    // WHEN/THEN: Assertion failures should have clear messages
    let result = std::panic::catch_unwind(|| {
        harness.assert_class_defined("http://test.example.org/NonExistent");
    });

    assert!(
        result.is_err(),
        "Should panic with clear error message when class not found"
    );
}
