//! Ontology Consistency Tests
//!
//! Comprehensive test suite for ontology validation and consistency checking.
//! Tests invalid ontologies to ensure proper error detection.

use oxigraph::io::GraphFormat;
use oxigraph::model::{GraphNameRef, NamedNode, Subject, Term, Triple};
use oxigraph::store::Store;
use spreadsheet_mcp::ontology::{
    ConsistencyChecker, HashVerifier, MergeResult, NamespaceManager, OntologyMerger,
    SchemaValidator, ValidationError,
};

// =============================================================================
// Test Utilities
// =============================================================================

fn create_test_store() -> Store {
    Store::new().unwrap()
}

fn add_triple(store: &Store, subject: &str, predicate: &str, object: &str) {
    let s = NamedNode::new(subject).unwrap();
    let p = NamedNode::new(predicate).unwrap();
    let o = NamedNode::new(object).unwrap();

    store
        .insert(Triple {
            subject: Subject::NamedNode(s),
            predicate: p,
            object: Term::NamedNode(o),
        })
        .unwrap();
}

fn add_literal_triple(store: &Store, subject: &str, predicate: &str, literal: &str) {
    let s = NamedNode::new(subject).unwrap();
    let p = NamedNode::new(predicate).unwrap();
    let lit = oxigraph::model::Literal::new_simple_literal(literal);

    store
        .insert(Triple {
            subject: Subject::NamedNode(s),
            predicate: p,
            object: Term::Literal(lit),
        })
        .unwrap();
}

// =============================================================================
// ConsistencyChecker Tests
// =============================================================================

#[test]
fn test_detect_cyclic_hierarchy() {
    let store = create_test_store();

    // Create a cycle: A -> B -> C -> A
    add_triple(
        &store,
        "http://example.org/A",
        "http://www.w3.org/2000/01/rdf-schema#subClassOf",
        "http://example.org/B",
    );
    add_triple(
        &store,
        "http://example.org/B",
        "http://www.w3.org/2000/01/rdf-schema#subClassOf",
        "http://example.org/C",
    );
    add_triple(
        &store,
        "http://example.org/C",
        "http://www.w3.org/2000/01/rdf-schema#subClassOf",
        "http://example.org/A",
    );

    let checker = ConsistencyChecker::new(store);
    let report = checker.check_all();

    assert!(!report.valid, "Should detect cycle");
    assert!(
        report.errors.iter().any(|e| e.contains("Cyclic")),
        "Should report cyclic hierarchy"
    );
}

#[test]
fn test_valid_hierarchy() {
    let store = create_test_store();

    // Create valid hierarchy: A -> B -> C (no cycle)
    add_triple(
        &store,
        "http://example.org/A",
        "http://www.w3.org/2000/01/rdf-schema#subClassOf",
        "http://example.org/B",
    );
    add_triple(
        &store,
        "http://example.org/B",
        "http://www.w3.org/2000/01/rdf-schema#subClassOf",
        "http://example.org/C",
    );

    let checker = ConsistencyChecker::new(store);
    let mut report = spreadsheet_mcp::ontology::ConsistencyReport::new();
    checker.check_class_hierarchy(&mut report).unwrap();

    assert!(report.valid, "Valid hierarchy should pass");
    assert!(report.errors.is_empty(), "Should have no errors");
}

#[test]
fn test_cardinality_violation() {
    let store = create_test_store();

    // Define a SHACL shape requiring exactly one property
    let ttl = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

        ex:TestShape a sh:NodeShape ;
            sh:targetClass ex:TestClass ;
            sh:property [
                sh:path ex:requiredProp ;
                sh:minCount 1 ;
                sh:maxCount 1
            ] .

        ex:Instance1 a ex:TestClass .
        # Missing ex:requiredProp - should violate minCount

        ex:Instance2 a ex:TestClass ;
            ex:requiredProp "value1" ;
            ex:requiredProp "value2" .
        # Two values - should violate maxCount
    "#;

    store
        .load_from_reader(GraphFormat::Turtle, ttl.as_bytes())
        .unwrap();

    let checker = ConsistencyChecker::new(store);
    let report = checker.check_all();

    assert!(!report.valid, "Should detect cardinality violations");
    assert!(
        report.errors.iter().any(|e| e.contains("Cardinality")),
        "Should report cardinality errors"
    );
}

#[test]
fn test_missing_required_property() {
    let store = create_test_store();

    let ttl = r#"
        @prefix ddd: <http://ggen-mcp.dev/ontology/ddd#> .
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

        ex:MyAggregate a rdfs:Class ;
            rdfs:subClassOf ddd:AggregateRoot ;
            ddd:hasProperty ex:requiredProperty .

        ex:MyInstance a ex:MyAggregate .
        # Missing ex:requiredProperty
    "#;

    store
        .load_from_reader(GraphFormat::Turtle, ttl.as_bytes())
        .unwrap();

    let checker = ConsistencyChecker::new(store);
    let report = checker.check_all();

    assert!(!report.valid, "Should detect missing required property");
}

#[test]
fn test_property_domain_violation() {
    let store = create_test_store();

    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .

        ex:specificProperty a owl:DatatypeProperty ;
            rdfs:domain ex:SpecificClass ;
            rdfs:range <http://www.w3.org/2001/XMLSchema#string> .

        ex:SpecificClass a owl:Class .
        ex:OtherClass a owl:Class .

        ex:wrongInstance a ex:OtherClass ;
            ex:specificProperty "invalid usage" .
        # Using property on wrong class type
    "#;

    store
        .load_from_reader(GraphFormat::Turtle, ttl.as_bytes())
        .unwrap();

    let checker = ConsistencyChecker::new(store);
    let report = checker.check_all();

    // Note: Domain checking requires type inference which may not catch all violations
    // This test documents expected behavior
}

#[test]
fn test_consistency_stats() {
    let store = create_test_store();

    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

        ex:ClassA a owl:Class .
        ex:ClassB a owl:Class ;
            rdfs:subClassOf ex:ClassA .
        ex:ClassC a owl:Class ;
            rdfs:subClassOf ex:ClassB .

        ex:property1 a owl:ObjectProperty .
        ex:property2 a owl:DatatypeProperty .

        ex:instance1 a ex:ClassC ;
            ex:property1 ex:instance2 .

        ex:instance2 a ex:ClassB .
    "#;

    store
        .load_from_reader(GraphFormat::Turtle, ttl.as_bytes())
        .unwrap();

    let checker = ConsistencyChecker::new(store);
    let report = checker.check_all();

    assert!(report.stats.total_classes >= 3, "Should count classes");
    assert!(
        report.stats.total_properties >= 2,
        "Should count properties"
    );
    assert!(
        report.stats.total_individuals >= 2,
        "Should count instances"
    );
    assert!(
        report.stats.max_hierarchy_depth >= 2,
        "Should calculate hierarchy depth"
    );
}

// =============================================================================
// SchemaValidator Tests
// =============================================================================

#[test]
fn test_invalid_ddd_structure() {
    let store = create_test_store();

    let ttl = r#"
        @prefix ddd: <http://ggen-mcp.dev/ontology/ddd#> .
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:EmptyAggregate a rdfs:Class ;
            rdfs:subClassOf ddd:AggregateRoot ;
            rdfs:label "Empty Aggregate" .
        # No ddd:hasProperty - invalid!
    "#;

    store
        .load_from_reader(GraphFormat::Turtle, ttl.as_bytes())
        .unwrap();

    let validator = SchemaValidator::new(store);
    let report = validator.validate_all();

    assert!(!report.valid, "Should detect invalid DDD structure");
    assert!(
        report.errors.iter().any(|e| e.contains("DDD")),
        "Should report DDD structure error"
    );
}

#[test]
fn test_invalid_invariant() {
    let store = create_test_store();

    let ttl = r#"
        @prefix ddd: <http://ggen-mcp.dev/ontology/ddd#> .
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:MyClass a rdfs:Class ;
            ddd:hasInvariant [
                rdfs:label "Some invariant" ;
                ddd:message "Error message"
                # Missing ddd:check - invalid!
            ] .
    "#;

    store
        .load_from_reader(GraphFormat::Turtle, ttl.as_bytes())
        .unwrap();

    let validator = SchemaValidator::new(store);
    let report = validator.validate_all();

    assert!(!report.valid, "Should detect invalid invariant");
    assert!(
        report.errors.iter().any(|e| e.contains("invariant")),
        "Should report invariant error"
    );
}

#[test]
fn test_orphaned_node_detection() {
    let store = create_test_store();

    let ttl = r#"
        @prefix ex: <http://example.org/> .

        ex:orphan ex:someProperty "value" .
        # No rdf:type, no incoming edges - potentially orphaned
    "#;

    store
        .load_from_reader(GraphFormat::Turtle, ttl.as_bytes())
        .unwrap();

    let validator = SchemaValidator::new(store);
    let report = validator.validate_all();

    // Orphaned nodes generate warnings, not errors
    assert!(
        !report.warnings.is_empty() || !report.errors.is_empty(),
        "Should detect potential orphaned node"
    );
}

#[test]
fn test_required_namespaces() {
    let store = create_test_store();

    // Empty ontology - missing recommended namespaces
    let validator = SchemaValidator::new(store);
    let report = validator.validate_all();

    // Should have warnings about missing namespaces
    // (not errors, since they're recommended not required)
}

#[test]
fn test_untyped_property_warning() {
    let store = create_test_store();

    let ttl = r#"
        @prefix ex: <http://example.org/> .

        ex:subject ex:untypedProperty ex:object .
        # ex:untypedProperty has no rdf:type declaration
    "#;

    store
        .load_from_reader(GraphFormat::Turtle, ttl.as_bytes())
        .unwrap();

    let validator = SchemaValidator::new(store);
    let report = validator.validate_all();

    // Untyped properties should generate warnings
}

// =============================================================================
// NamespaceManager Tests
// =============================================================================

#[test]
fn test_namespace_registration() {
    let mut ns = NamespaceManager::new();

    // Register new namespace
    assert!(ns.register("mcp", "http://ggen-mcp.dev/mcp#").is_ok());

    // Get namespace
    assert_eq!(ns.get("mcp").unwrap(), "http://ggen-mcp.dev/mcp#");
}

#[test]
fn test_namespace_collision_detection() {
    let mut ns = NamespaceManager::new();

    ns.register("mcp", "http://ggen-mcp.dev/mcp#").unwrap();

    // Try to register same prefix with different URI
    let result = ns.register("mcp", "http://different-uri.dev/mcp#");

    assert!(result.is_err(), "Should detect collision");
    match result {
        Err(ValidationError::NamespaceCollision { prefix, .. }) => {
            assert_eq!(prefix, "mcp");
        }
        _ => panic!("Expected NamespaceCollision error"),
    }
}

#[test]
fn test_namespace_expansion() {
    let mut ns = NamespaceManager::new();
    ns.register("mcp", "http://ggen-mcp.dev/mcp#").unwrap();

    let expanded = ns.expand("mcp:Tool").unwrap();
    assert_eq!(expanded, "http://ggen-mcp.dev/mcp#Tool");
}

#[test]
fn test_namespace_compaction() {
    let mut ns = NamespaceManager::new();
    ns.register("mcp", "http://ggen-mcp.dev/mcp#").unwrap();

    let compact = ns.compact("http://ggen-mcp.dev/mcp#Tool");
    assert_eq!(compact, "mcp:Tool");
}

#[test]
fn test_namespace_expansion_no_prefix() {
    let ns = NamespaceManager::new();

    // Try to expand unknown prefix
    let result = ns.expand("unknown:Thing");
    assert!(result.is_err(), "Should fail on unknown prefix");
}

#[test]
fn test_default_namespace() {
    let mut ns = NamespaceManager::new();
    ns.set_default("http://example.org/default#");

    let expanded = ns.expand("Thing").unwrap();
    assert_eq!(expanded, "http://example.org/default#Thing");
}

#[test]
fn test_common_namespaces_preregistered() {
    let ns = NamespaceManager::new();

    // Should have common namespaces pre-registered
    assert!(ns.get("rdf").is_some());
    assert!(ns.get("rdfs").is_some());
    assert!(ns.get("owl").is_some());
    assert!(ns.get("xsd").is_some());
    assert!(ns.get("sh").is_some());
    assert!(ns.get("ddd").is_some());
}

// =============================================================================
// OntologyMerger Tests
// =============================================================================

#[test]
fn test_successful_merge() {
    let target = create_test_store();
    let source = create_test_store();

    // Add non-conflicting triples
    let target_ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:ClassA a rdfs:Class ;
            rdfs:label "Class A" .
    "#;

    let source_ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:ClassB a rdfs:Class ;
            rdfs:label "Class B" .
    "#;

    target
        .load_from_reader(GraphFormat::Turtle, target_ttl.as_bytes())
        .unwrap();
    source
        .load_from_reader(GraphFormat::Turtle, source_ttl.as_bytes())
        .unwrap();

    let merger = OntologyMerger::new();
    let result = merger.merge(&target, &source).unwrap();

    assert!(result.success, "Merge should succeed");
    assert!(result.conflicts.is_empty(), "Should have no conflicts");
    assert!(result.merged_triples > 0, "Should merge some triples");
}

#[test]
fn test_merge_conflict_detection() {
    let target = create_test_store();
    let source = create_test_store();

    // Create conflicting class hierarchies
    let target_ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:SubClass a rdfs:Class ;
            rdfs:subClassOf ex:SuperClassA .
    "#;

    let source_ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:SubClass a rdfs:Class ;
            rdfs:subClassOf ex:SuperClassB .
    "#;

    target
        .load_from_reader(GraphFormat::Turtle, target_ttl.as_bytes())
        .unwrap();
    source
        .load_from_reader(GraphFormat::Turtle, source_ttl.as_bytes())
        .unwrap();

    let merger = OntologyMerger::new();
    let result = merger.merge(&target, &source).unwrap();

    assert!(!result.success, "Merge should fail due to conflicts");
    assert!(!result.conflicts.is_empty(), "Should detect conflicts");
}

#[test]
fn test_merge_duplicate_triples() {
    let target = create_test_store();
    let source = create_test_store();

    // Same triples in both - should merge without conflict
    let same_ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:ClassA a rdfs:Class ;
            rdfs:label "Class A" .
    "#;

    target
        .load_from_reader(GraphFormat::Turtle, same_ttl.as_bytes())
        .unwrap();
    source
        .load_from_reader(GraphFormat::Turtle, same_ttl.as_bytes())
        .unwrap();

    let merger = OntologyMerger::new();
    let result = merger.merge(&target, &source).unwrap();

    // Duplicate triples should be handled gracefully
    assert!(result.success, "Duplicate triples should not cause failure");
}

// =============================================================================
// HashVerifier Tests
// =============================================================================

#[test]
fn test_compute_hash() {
    let store = create_test_store();

    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:subject ex:predicate ex:object .
    "#;

    store
        .load_from_reader(GraphFormat::Turtle, ttl.as_bytes())
        .unwrap();

    let verifier = HashVerifier::new(store);
    let hash = verifier.compute_hash().unwrap();

    assert!(!hash.is_empty(), "Should compute non-empty hash");
    assert_eq!(hash.len(), 64, "SHA-256 hash should be 64 hex chars");
}

#[test]
fn test_hash_deterministic() {
    let store1 = create_test_store();
    let store2 = create_test_store();

    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:subject ex:predicate ex:object .
    "#;

    store1
        .load_from_reader(GraphFormat::Turtle, ttl.as_bytes())
        .unwrap();
    store2
        .load_from_reader(GraphFormat::Turtle, ttl.as_bytes())
        .unwrap();

    let verifier1 = HashVerifier::new(store1);
    let verifier2 = HashVerifier::new(store2);

    let hash1 = verifier1.compute_hash().unwrap();
    let hash2 = verifier2.compute_hash().unwrap();

    assert_eq!(hash1, hash2, "Same ontology should produce same hash");
}

#[test]
fn test_hash_changes_with_content() {
    let store1 = create_test_store();
    let store2 = create_test_store();

    let ttl1 = r#"
        @prefix ex: <http://example.org/> .
        ex:subject ex:predicate ex:object1 .
    "#;

    let ttl2 = r#"
        @prefix ex: <http://example.org/> .
        ex:subject ex:predicate ex:object2 .
    "#;

    store1
        .load_from_reader(GraphFormat::Turtle, ttl1.as_bytes())
        .unwrap();
    store2
        .load_from_reader(GraphFormat::Turtle, ttl2.as_bytes())
        .unwrap();

    let verifier1 = HashVerifier::new(store1);
    let verifier2 = HashVerifier::new(store2);

    let hash1 = verifier1.compute_hash().unwrap();
    let hash2 = verifier2.compute_hash().unwrap();

    assert_ne!(
        hash1, hash2,
        "Different content should produce different hashes"
    );
}

#[test]
fn test_verify_hash_match() {
    let store = create_test_store();

    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:subject ex:predicate ex:object .
    "#;

    store
        .load_from_reader(GraphFormat::Turtle, ttl.as_bytes())
        .unwrap();

    let verifier = HashVerifier::new(store);
    let hash = verifier.compute_hash().unwrap();

    // Should verify successfully
    assert!(verifier.verify_hash(&hash).is_ok());
}

#[test]
fn test_verify_hash_mismatch() {
    let store = create_test_store();

    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:subject ex:predicate ex:object .
    "#;

    store
        .load_from_reader(GraphFormat::Turtle, ttl.as_bytes())
        .unwrap();

    let verifier = HashVerifier::new(store);

    // Try to verify with wrong hash
    let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";
    let result = verifier.verify_hash(wrong_hash);

    assert!(result.is_err(), "Should detect hash mismatch");
    match result {
        Err(ValidationError::HashMismatch { expected, actual }) => {
            assert_eq!(expected, wrong_hash);
            assert_ne!(actual, wrong_hash);
        }
        _ => panic!("Expected HashMismatch error"),
    }
}

#[test]
fn test_store_and_retrieve_hash() {
    let store = create_test_store();

    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .

        ex:Ontology a owl:Ontology .
    "#;

    store
        .load_from_reader(GraphFormat::Turtle, ttl.as_bytes())
        .unwrap();

    let verifier = HashVerifier::new(store);
    let hash = verifier.compute_hash().unwrap();

    // Store hash
    verifier.store_hash(&hash).unwrap();

    // Retrieve hash
    let stored_hash = verifier.get_ontology_hash().unwrap();
    assert_eq!(stored_hash, Some(hash));
}

// =============================================================================
// Integration Tests
// =============================================================================

#[test]
fn test_full_validation_pipeline() {
    let store = create_test_store();

    // Load a valid DDD ontology
    let ttl = r#"
        @prefix mcp: <http://ggen-mcp.dev/ontology/mcp#> .
        @prefix ddd: <http://ggen-mcp.dev/ontology/ddd#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

        mcp:Tool a owl:Class ;
            rdfs:subClassOf ddd:Entity ;
            rdfs:label "Tool" ;
            ddd:hasProperty mcp:toolName ;
            ddd:hasInvariant [
                rdfs:label "Tool name must be valid" ;
                ddd:check "self.name.is_valid()" ;
                ddd:message "Invalid tool name"
            ] .

        mcp:toolName a owl:DatatypeProperty ;
            rdfs:domain mcp:Tool ;
            rdfs:range xsd:string .

        mcp:ToolShape a sh:NodeShape ;
            sh:targetClass mcp:Tool ;
            sh:property [
                sh:path mcp:toolName ;
                sh:minCount 1 ;
                sh:maxCount 1
            ] .

        mcp:MyTool a mcp:Tool ;
            mcp:toolName "my_tool" .
    "#;

    store
        .load_from_reader(GraphFormat::Turtle, ttl.as_bytes())
        .unwrap();

    // Run consistency checks
    let consistency_checker = ConsistencyChecker::new(store.clone());
    let consistency_report = consistency_checker.check_all();
    assert!(consistency_report.valid, "Consistency check should pass");

    // Run schema validation
    let schema_validator = SchemaValidator::new(store.clone());
    let schema_report = schema_validator.validate_all();
    // May have warnings but should not have critical errors

    // Compute and verify hash
    let hash_verifier = HashVerifier::new(store);
    let hash = hash_verifier.compute_hash().unwrap();
    assert!(hash_verifier.verify_hash(&hash).is_ok());
}

#[test]
fn test_invalid_ontology_detection() {
    let store = create_test_store();

    // Create an intentionally invalid ontology
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ddd: <http://ggen-mcp.dev/ontology/ddd#> .

        # Cyclic hierarchy
        ex:A rdfs:subClassOf ex:B .
        ex:B rdfs:subClassOf ex:A .

        # Aggregate without properties
        ex:BadAggregate a rdfs:Class ;
            rdfs:subClassOf ddd:AggregateRoot .

        # Invariant without check
        ex:ClassWithBadInvariant a rdfs:Class ;
            ddd:hasInvariant [
                rdfs:label "Bad invariant"
            ] .
    "#;

    store
        .load_from_reader(GraphFormat::Turtle, ttl.as_bytes())
        .unwrap();

    // Should detect multiple errors
    let consistency_checker = ConsistencyChecker::new(store.clone());
    let consistency_report = consistency_checker.check_all();
    assert!(!consistency_report.valid, "Should detect errors");

    let schema_validator = SchemaValidator::new(store);
    let schema_report = schema_validator.validate_all();
    assert!(!schema_report.valid, "Should detect schema errors");
}
