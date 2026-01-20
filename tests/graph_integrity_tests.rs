//! Comprehensive tests for RDF graph integrity checking
//!
//! These tests verify the integrity checker can detect various types of
//! corrupted and invalid RDF graphs.

use anyhow::Result;
use oxigraph::model::{
    vocab::{rdf, rdfs, xsd},
    BlankNode, Literal, NamedNode, Subject, Term, Triple,
};
use oxigraph::store::Store;
use spreadsheet_mcp::ontology::{
    GraphDiff, GraphIntegrityChecker, IntegrityConfig, ReferenceChecker, Severity,
    TripleValidator, TypeChecker,
};
use std::collections::{HashMap, HashSet};

/// Helper to create a store with triples
fn create_store_with_triples(triples: Vec<Triple>) -> Result<Store> {
    let store = Store::new()?;
    for triple in triples {
        store.insert(&triple.into())?;
    }
    Ok(store)
}

/// Helper to create a named node
fn nn(iri: &str) -> NamedNode {
    NamedNode::new(iri).unwrap()
}

/// Helper to create a subject from named node
fn subj(iri: &str) -> Subject {
    Subject::NamedNode(nn(iri))
}

/// Helper to create a term from named node
fn term(iri: &str) -> Term {
    Term::NamedNode(nn(iri))
}

#[test]
fn test_valid_graph() -> Result<()> {
    let triples = vec![
        Triple::new(
            subj("http://example.org/person1"),
            nn("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
            term("http://example.org/Person"),
        ),
        Triple::new(
            subj("http://example.org/person1"),
            nn("http://example.org/name"),
            Term::Literal(Literal::new_simple_literal("Alice")),
        ),
        Triple::new(
            subj("http://example.org/person1"),
            nn("http://example.org/age"),
            Term::Literal(Literal::new_typed_literal("30", xsd::INTEGER)),
        ),
    ];

    let store = create_store_with_triples(triples)?;
    let checker = GraphIntegrityChecker::new(IntegrityConfig::default());
    let report = checker.check(&store)?;

    assert!(report.is_valid(), "Valid graph should pass integrity check");
    assert_eq!(report.violations.len(), 0);
    assert_eq!(report.total_triples, 3);

    Ok(())
}

#[test]
fn test_invalid_integer_literal() -> Result<()> {
    let validator = TripleValidator::new();

    let triple = Triple::new(
        subj("http://example.org/subject"),
        nn("http://example.org/property"),
        Term::Literal(Literal::new_typed_literal("not-a-number", xsd::INTEGER)),
    );

    let result = validator.validate(&triple);
    assert!(result.is_err(), "Invalid integer should fail validation");

    Ok(())
}

#[test]
fn test_invalid_boolean_literal() -> Result<()> {
    let validator = TripleValidator::new();

    let triple = Triple::new(
        subj("http://example.org/subject"),
        nn("http://example.org/property"),
        Term::Literal(Literal::new_typed_literal("maybe", xsd::BOOLEAN)),
    );

    let result = validator.validate(&triple);
    assert!(result.is_err(), "Invalid boolean should fail validation");

    Ok(())
}

#[test]
fn test_dangling_reference() -> Result<()> {
    let triples = vec![
        Triple::new(
            subj("http://example.org/person1"),
            nn("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
            term("http://example.org/Person"),
        ),
        Triple::new(
            subj("http://example.org/person1"),
            nn("http://example.org/knows"),
            // This references person2 which doesn't exist
            term("http://example.org/person2"),
        ),
    ];

    let store = create_store_with_triples(triples)?;
    let checker = GraphIntegrityChecker::new(IntegrityConfig::default());
    let report = checker.check(&store)?;

    // Should have a warning about dangling reference
    let has_dangling_warning = report
        .violations
        .iter()
        .any(|v| v.error.contains("Dangling reference"));

    assert!(
        has_dangling_warning,
        "Should detect dangling reference to person2"
    );

    Ok(())
}

#[test]
fn test_missing_required_property() -> Result<()> {
    let triples = vec![Triple::new(
        subj("http://example.org/person1"),
        nn("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
        term("http://example.org/Person"),
    )];

    let store = create_store_with_triples(triples)?;

    // Configure required properties
    let mut config = IntegrityConfig::default();
    let mut required = HashMap::new();
    required.insert(
        "http://example.org/Person".to_string(),
        vec!["http://example.org/name".to_string()],
    );
    config.required_properties = required;

    let checker = GraphIntegrityChecker::new(config);
    let report = checker.check(&store)?;

    // Should have error about missing required property
    let has_missing_property = report
        .violations
        .iter()
        .any(|v| v.error.contains("Missing required property"));

    assert!(
        has_missing_property,
        "Should detect missing required property 'name'"
    );

    Ok(())
}

#[test]
fn test_abstract_type_instantiation() -> Result<()> {
    let triples = vec![Triple::new(
        subj("http://example.org/thing1"),
        nn("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
        term("http://example.org/AbstractClass"),
    )];

    let store = create_store_with_triples(triples)?;

    // Configure abstract types
    let mut config = IntegrityConfig::default();
    let mut abstract_types = HashSet::new();
    abstract_types.insert("http://example.org/AbstractClass".to_string());
    config.abstract_types = abstract_types;

    let checker = GraphIntegrityChecker::new(config);
    let report = checker.check(&store)?;

    // Should have error about abstract type instantiation
    let has_abstract_error = report
        .violations
        .iter()
        .any(|v| v.error.contains("Abstract type cannot be instantiated"));

    assert!(
        has_abstract_error,
        "Should detect abstract type instantiation"
    );

    Ok(())
}

#[test]
fn test_blank_node_dangling_reference() -> Result<()> {
    let blank1 = BlankNode::new("b1").unwrap();
    let blank2 = BlankNode::new("b2").unwrap();

    let triples = vec![Triple::new(
        Subject::BlankNode(blank1),
        nn("http://example.org/references"),
        Term::BlankNode(blank2.clone()),
    )];

    let store = create_store_with_triples(triples)?;
    let checker = GraphIntegrityChecker::new(IntegrityConfig::default());
    let report = checker.check(&store)?;

    // Should detect dangling blank node reference
    let has_blank_error = report
        .violations
        .iter()
        .any(|v| v.error.contains("Dangling blank node reference"));

    assert!(
        has_blank_error,
        "Should detect dangling blank node reference"
    );

    Ok(())
}

#[test]
fn test_circular_reference_detection() -> Result<()> {
    let triples = vec![
        Triple::new(
            subj("http://example.org/A"),
            nn("http://example.org/references"),
            term("http://example.org/B"),
        ),
        Triple::new(
            subj("http://example.org/B"),
            nn("http://example.org/references"),
            term("http://example.org/C"),
        ),
        Triple::new(
            subj("http://example.org/C"),
            nn("http://example.org/references"),
            term("http://example.org/A"),
        ),
    ];

    let store = create_store_with_triples(triples)?;
    let config = IntegrityConfig::default();
    let reference_checker = ReferenceChecker::new(config);

    let property = nn("http://example.org/references");
    let cycles = reference_checker.detect_circular_references(&store, &property)?;

    assert!(!cycles.is_empty(), "Should detect circular reference A->B->C->A");

    Ok(())
}

#[test]
fn test_inverse_relationship_missing() -> Result<()> {
    let triples = vec![
        Triple::new(
            subj("http://example.org/person1"),
            nn("http://example.org/knows"),
            term("http://example.org/person2"),
        ),
        Triple::new(
            subj("http://example.org/person2"),
            nn("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
            term("http://example.org/Person"),
        ),
    ];

    let store = create_store_with_triples(triples)?;

    // Configure inverse properties
    let mut config = IntegrityConfig::default();
    config.check_inverse_relationships = true;
    let mut inverse = HashMap::new();
    inverse.insert(
        "http://example.org/knows".to_string(),
        "http://example.org/knownBy".to_string(),
    );
    config.inverse_properties = inverse;

    let checker = GraphIntegrityChecker::new(config);
    let report = checker.check(&store)?;

    // Should have warning about missing inverse relationship
    let has_inverse_warning = report
        .violations
        .iter()
        .any(|v| v.error.contains("Missing inverse relationship"));

    assert!(
        has_inverse_warning,
        "Should detect missing inverse relationship"
    );

    Ok(())
}

#[test]
fn test_graph_diff_computation() -> Result<()> {
    // Old graph
    let old_triples = vec![
        Triple::new(
            subj("http://example.org/person1"),
            nn("http://example.org/name"),
            Term::Literal(Literal::new_simple_literal("Alice")),
        ),
        Triple::new(
            subj("http://example.org/person1"),
            nn("http://example.org/age"),
            Term::Literal(Literal::new_typed_literal("30", xsd::INTEGER)),
        ),
    ];

    // New graph
    let new_triples = vec![
        Triple::new(
            subj("http://example.org/person1"),
            nn("http://example.org/name"),
            Term::Literal(Literal::new_simple_literal("Alice")),
        ),
        Triple::new(
            subj("http://example.org/person1"),
            nn("http://example.org/age"),
            Term::Literal(Literal::new_typed_literal("31", xsd::INTEGER)),
        ),
        Triple::new(
            subj("http://example.org/person1"),
            nn("http://example.org/email"),
            Term::Literal(Literal::new_simple_literal("alice@example.org")),
        ),
    ];

    let old_store = create_store_with_triples(old_triples)?;
    let new_store = create_store_with_triples(new_triples)?;

    let diff = GraphDiff::compute(&old_store, &new_store)?;

    assert_eq!(diff.added.len(), 2, "Should have 2 added triples");
    assert_eq!(diff.removed.len(), 1, "Should have 1 removed triple");

    let stats = diff.stats();
    assert_eq!(stats.total_changes(), 3);

    Ok(())
}

#[test]
fn test_graph_diff_validation() -> Result<()> {
    let diff = GraphDiff {
        added: vec![Triple::new(
            subj("http://example.org/person1"),
            nn("http://example.org/age"),
            Term::Literal(Literal::new_typed_literal("not-a-number", xsd::INTEGER)),
        )],
        removed: vec![],
        modified: vec![],
    };

    let store = Store::new()?;
    let checker = GraphIntegrityChecker::new(IntegrityConfig::default());
    let report = diff.validate(&checker, &store)?;

    assert!(
        report.has_errors(),
        "Should detect invalid triple in diff"
    );

    Ok(())
}

#[test]
fn test_multiple_types() -> Result<()> {
    let triples = vec![
        Triple::new(
            subj("http://example.org/entity1"),
            nn("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
            term("http://example.org/Person"),
        ),
        Triple::new(
            subj("http://example.org/entity1"),
            nn("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
            term("http://example.org/Organization"),
        ),
    ];

    let store = create_store_with_triples(triples)?;
    let checker = GraphIntegrityChecker::new(IntegrityConfig::default());
    let report = checker.check(&store)?;

    // Should have info about multiple types
    let has_multiple_types_info = report
        .violations
        .iter()
        .any(|v| v.severity == Severity::Info && v.error.contains("types"));

    assert!(
        has_multiple_types_info,
        "Should report entity with multiple types"
    );

    Ok(())
}

#[test]
fn test_corrupted_graph_comprehensive() -> Result<()> {
    let triples = vec![
        // Valid triple
        Triple::new(
            subj("http://example.org/person1"),
            nn("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
            term("http://example.org/Person"),
        ),
        // Dangling reference
        Triple::new(
            subj("http://example.org/person1"),
            nn("http://example.org/manager"),
            term("http://example.org/person999"),
        ),
        // Invalid integer
        Triple::new(
            subj("http://example.org/person1"),
            nn("http://example.org/age"),
            Term::Literal(Literal::new_typed_literal("xyz", xsd::INTEGER)),
        ),
        // Invalid boolean
        Triple::new(
            subj("http://example.org/person1"),
            nn("http://example.org/active"),
            Term::Literal(Literal::new_typed_literal("yes", xsd::BOOLEAN)),
        ),
    ];

    let store = create_store_with_triples(triples)?;
    let checker = GraphIntegrityChecker::new(IntegrityConfig::default());
    let report = checker.check(&store)?;

    assert!(
        !report.is_valid(),
        "Corrupted graph should fail validation"
    );
    assert!(report.has_errors(), "Should have errors");
    assert!(
        report.violations.len() >= 3,
        "Should detect multiple violations"
    );

    Ok(())
}

#[test]
fn test_integrity_report_display() -> Result<()> {
    let triples = vec![Triple::new(
        subj("http://example.org/test"),
        nn("http://example.org/value"),
        Term::Literal(Literal::new_typed_literal("invalid", xsd::INTEGER)),
    )];

    let store = create_store_with_triples(triples)?;
    let checker = GraphIntegrityChecker::new(IntegrityConfig::default());
    let report = checker.check(&store)?;

    let report_str = report.to_string();
    assert!(report_str.contains("Integrity Report"));
    assert!(report_str.contains("error"));

    let summary = report.summary();
    assert!(summary.contains("1 triples"));

    Ok(())
}

#[test]
fn test_type_checker_get_types() -> Result<()> {
    let triples = vec![
        Triple::new(
            subj("http://example.org/person1"),
            nn("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
            term("http://example.org/Person"),
        ),
        Triple::new(
            subj("http://example.org/person1"),
            nn("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
            term("http://example.org/Employee"),
        ),
    ];

    let store = create_store_with_triples(triples)?;
    let config = IntegrityConfig::default();
    let type_checker = TypeChecker::new(config);

    let types = type_checker.get_types(&store, &nn("http://example.org/person1").into())?;

    assert_eq!(types.len(), 2, "Should have 2 types");

    Ok(())
}

#[test]
fn test_large_graph_performance() -> Result<()> {
    // Create a larger graph to test performance
    let mut triples = Vec::new();

    for i in 0..1000 {
        let subject = format!("http://example.org/entity{}", i);
        triples.push(Triple::new(
            subj(&subject),
            nn("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
            term("http://example.org/Thing"),
        ));
        triples.push(Triple::new(
            subj(&subject),
            nn("http://example.org/name"),
            Term::Literal(Literal::new_simple_literal(&format!("Entity {}", i))),
        ));
    }

    let store = create_store_with_triples(triples)?;
    let checker = GraphIntegrityChecker::new(IntegrityConfig::default());

    let start = std::time::Instant::now();
    let report = checker.check(&store)?;
    let duration = start.elapsed();

    assert!(report.is_valid(), "Large graph should be valid");
    assert_eq!(report.total_triples, 2000);
    println!("Checked 2000 triples in {:?}", duration);

    Ok(())
}

#[test]
fn test_config_customization() {
    let mut config = IntegrityConfig::default();

    // Customize configuration
    config.check_references = false;
    config.check_types = true;
    config.check_orphans = true;
    config.max_circular_depth = 50;

    assert!(!config.check_references);
    assert!(config.check_types);
    assert!(config.check_orphans);
    assert_eq!(config.max_circular_depth, 50);
}
