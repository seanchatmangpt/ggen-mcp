//! Integration tests for Turtle ontology authoring tools
//!
//! Chicago-style TDD: State-based verification, real RDF parsing, minimal mocking.
//! Tests Turtle ontology lifecycle: read, add entities, add properties, validate syntax, query.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

mod harness;
use harness::turtle_ontology_harness::{OntologyBuilder, OntologyTestHarness};

// =============================================================================
// Test Harness for Turtle Authoring Operations
// =============================================================================

struct TurtleAuthoringHarness {
    workspace: TempDir,
}

impl TurtleAuthoringHarness {
    fn new() -> Result<Self> {
        Ok(Self {
            workspace: tempfile::tempdir()?,
        })
    }

    fn ontology_path(&self) -> std::path::PathBuf {
        self.workspace.path().join("ontology/domain.ttl")
    }

    fn write_ontology(&self, content: &str) -> Result<()> {
        fs::create_dir_all(self.ontology_path().parent().unwrap())?;
        fs::write(self.ontology_path(), content)?;
        Ok(())
    }

    fn read_ontology(&self) -> Result<String> {
        Ok(fs::read_to_string(self.ontology_path())?)
    }

    fn workspace_path(&self) -> &Path {
        self.workspace.path()
    }
}

// =============================================================================
// Fixtures
// =============================================================================

fn minimal_ontology() -> &'static str {
    r#"
@prefix test: <http://test.example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix ddd: <http://ggen-mcp.dev/ontology/ddd#> .

test:User a owl:Class ;
    rdfs:subClassOf ddd:AggregateRoot ;
    rdfs:label "User"@en .
"#
}

fn ontology_with_properties() -> &'static str {
    r#"
@prefix test: <http://test.example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix ddd: <http://ggen-mcp.dev/ontology/ddd#> .

test:User a owl:Class ;
    rdfs:subClassOf ddd:AggregateRoot ;
    rdfs:label "User"@en ;
    ddd:hasProperty test:userId, test:userName, test:userEmail .

test:userId a owl:ObjectProperty ;
    rdfs:domain test:User ;
    rdfs:range test:UserId .

test:UserId a owl:Class ;
    rdfs:subClassOf ddd:ValueObject .

test:userName a owl:DatatypeProperty ;
    rdfs:domain test:User ;
    rdfs:range xsd:string .

test:userEmail a owl:ObjectProperty ;
    rdfs:domain test:User ;
    rdfs:range test:Email .

test:Email a owl:Class ;
    rdfs:subClassOf ddd:ValueObject .
"#
}

// =============================================================================
// Tests: read_turtle_ontology
// =============================================================================

#[tokio::test]
async fn test_read_valid_ontology() -> Result<()> {
    // GIVEN: Valid Turtle ontology file
    let harness = TurtleAuthoringHarness::new()?;
    harness.write_ontology(minimal_ontology())?;

    // WHEN: We read the ontology
    let result = simulate_read_ontology(harness.ontology_path().as_path()).await?;

    // THEN: Ontology parsed successfully
    assert!(result.valid);
    assert_eq!(result.class_count, 1);  // User class
    assert!(result.classes.contains(&"http://test.example.org/User".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_read_ontology_with_properties() -> Result<()> {
    // GIVEN: Ontology with properties
    let harness = TurtleAuthoringHarness::new()?;
    harness.write_ontology(ontology_with_properties())?;

    // WHEN: We read the ontology
    let result = simulate_read_ontology(harness.ontology_path().as_path()).await?;

    // THEN: Properties parsed
    assert_eq!(result.property_count, 3);  // userId, userName, userEmail
    assert!(result
        .properties
        .contains(&"http://test.example.org/userId".to_string()));
    assert!(result
        .properties
        .contains(&"http://test.example.org/userName".to_string()));
    assert!(result
        .properties
        .contains(&"http://test.example.org/userEmail".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_read_nonexistent_ontology() -> Result<()> {
    // GIVEN: No ontology file
    let harness = TurtleAuthoringHarness::new()?;

    // WHEN: We try to read ontology
    let result = simulate_read_ontology(harness.ontology_path().as_path()).await;

    // THEN: Error returned
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("not found") || err_msg.contains("No such file"));

    Ok(())
}

// =============================================================================
// Tests: add_entity_to_ontology
// =============================================================================

#[tokio::test]
async fn test_add_aggregate_to_ontology() -> Result<()> {
    // GIVEN: Minimal ontology
    let harness = TurtleAuthoringHarness::new()?;
    harness.write_ontology(minimal_ontology())?;

    // WHEN: We add a new aggregate
    simulate_add_entity(
        harness.ontology_path().as_path(),
        "Order",
        "AggregateRoot",
        Some("Order aggregate for managing purchases"),
    )
    .await?;

    // THEN: Entity added to ontology
    let content = harness.read_ontology()?;
    assert!(content.contains("test:Order"));
    assert!(content.contains("ddd:AggregateRoot"));
    assert!(content.contains("Order aggregate for managing purchases"));

    // AND: Ontology still valid
    let result = simulate_read_ontology(harness.ontology_path().as_path()).await?;
    assert!(result.valid);
    assert_eq!(result.class_count, 2);  // User + Order

    Ok(())
}

#[tokio::test]
async fn test_add_value_object_to_ontology() -> Result<()> {
    // GIVEN: Ontology with aggregate
    let harness = TurtleAuthoringHarness::new()?;
    harness.write_ontology(minimal_ontology())?;

    // WHEN: We add a value object
    simulate_add_entity(
        harness.ontology_path().as_path(),
        "PhoneNumber",
        "ValueObject",
        Some("Phone number value object"),
    )
    .await?;

    // THEN: Value object added
    let content = harness.read_ontology()?;
    assert!(content.contains("test:PhoneNumber"));
    assert!(content.contains("ddd:ValueObject"));

    Ok(())
}

#[tokio::test]
async fn test_add_duplicate_entity_fails() -> Result<()> {
    // GIVEN: Ontology with User entity
    let harness = TurtleAuthoringHarness::new()?;
    harness.write_ontology(minimal_ontology())?;

    // WHEN: We try to add duplicate User
    let result = simulate_add_entity(
        harness.ontology_path().as_path(),
        "User",
        "AggregateRoot",
        None,
    )
    .await;

    // THEN: Error returned
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("already exists") || err_msg.contains("duplicate"));

    Ok(())
}

// =============================================================================
// Tests: add_property_to_entity
// =============================================================================

#[tokio::test]
async fn test_add_datatype_property() -> Result<()> {
    // GIVEN: Ontology with User entity
    let harness = TurtleAuthoringHarness::new()?;
    harness.write_ontology(minimal_ontology())?;

    // WHEN: We add a string property
    simulate_add_property(
        harness.ontology_path().as_path(),
        "User",
        "age",
        "DatatypeProperty",
        "xsd:integer",
        None,
    )
    .await?;

    // THEN: Property added
    let content = harness.read_ontology()?;
    assert!(content.contains("test:age"));
    assert!(content.contains("owl:DatatypeProperty"));
    assert!(content.contains("rdfs:domain test:User"));
    assert!(content.contains("rdfs:range xsd:integer"));

    Ok(())
}

#[tokio::test]
async fn test_add_object_property() -> Result<()> {
    // GIVEN: Ontology with User entity
    let harness = TurtleAuthoringHarness::new()?;
    harness.write_ontology(minimal_ontology())?;

    // WHEN: We add an object property
    simulate_add_property(
        harness.ontology_path().as_path(),
        "User",
        "address",
        "ObjectProperty",
        "test:Address",
        Some("User's address"),
    )
    .await?;

    // THEN: Property added
    let content = harness.read_ontology()?;
    assert!(content.contains("test:address"));
    assert!(content.contains("owl:ObjectProperty"));
    assert!(content.contains("rdfs:range test:Address"));

    Ok(())
}

#[tokio::test]
async fn test_add_property_to_nonexistent_entity_fails() -> Result<()> {
    // GIVEN: Minimal ontology (no Order entity)
    let harness = TurtleAuthoringHarness::new()?;
    harness.write_ontology(minimal_ontology())?;

    // WHEN: We try to add property to non-existent entity
    let result = simulate_add_property(
        harness.ontology_path().as_path(),
        "Order",
        "total",
        "DatatypeProperty",
        "xsd:decimal",
        None,
    )
    .await;

    // THEN: Error returned
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("not found") || err_msg.contains("Order"));

    Ok(())
}

// =============================================================================
// Tests: validate_turtle_syntax
// =============================================================================

#[tokio::test]
async fn test_validate_valid_syntax() -> Result<()> {
    // GIVEN: Valid Turtle ontology
    let harness = TurtleAuthoringHarness::new()?;
    harness.write_ontology(minimal_ontology())?;

    // WHEN: We validate syntax
    let result = simulate_validate_syntax(harness.ontology_path().as_path()).await?;

    // THEN: Validation passes
    assert!(result.valid);
    assert_eq!(result.errors.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_validate_invalid_syntax() -> Result<()> {
    // GIVEN: Invalid Turtle syntax
    let harness = TurtleAuthoringHarness::new()?;
    harness.write_ontology("@prefix test: INVALID SYNTAX @@@")?;

    // WHEN: We validate syntax
    let result = simulate_validate_syntax(harness.ontology_path().as_path()).await?;

    // THEN: Validation fails
    assert!(!result.valid);
    assert!(result.errors.len() > 0);
    assert!(result.errors[0].contains("syntax") || result.errors[0].contains("parse"));

    Ok(())
}

#[tokio::test]
async fn test_validate_missing_prefix() -> Result<()> {
    // GIVEN: Ontology with undefined prefix
    let harness = TurtleAuthoringHarness::new()?;
    harness.write_ontology("undefined:Entity a owl:Class .")?;

    // WHEN: We validate syntax
    let result = simulate_validate_syntax(harness.ontology_path().as_path()).await?;

    // THEN: Validation fails
    assert!(!result.valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("prefix") || e.contains("undefined")));

    Ok(())
}

// =============================================================================
// Tests: query_ontology_entities
// =============================================================================

#[tokio::test]
async fn test_query_aggregate_roots() -> Result<()> {
    // GIVEN: Ontology with aggregates
    let harness = TurtleAuthoringHarness::new()?;
    harness.write_ontology(ontology_with_properties())?;

    // Add another aggregate
    simulate_add_entity(
        harness.ontology_path().as_path(),
        "Order",
        "AggregateRoot",
        None,
    )
    .await?;

    // WHEN: We query for aggregate roots
    let result = simulate_query_entities(
        harness.ontology_path().as_path(),
        "SELECT ?agg WHERE { ?agg rdfs:subClassOf ddd:AggregateRoot }",
    )
    .await?;

    // THEN: Both aggregates returned
    assert_eq!(result.entities.len(), 2);
    assert!(result
        .entities
        .contains(&"http://test.example.org/User".to_string()));
    assert!(result
        .entities
        .contains(&"http://test.example.org/Order".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_query_value_objects() -> Result<()> {
    // GIVEN: Ontology with value objects
    let harness = TurtleAuthoringHarness::new()?;
    harness.write_ontology(ontology_with_properties())?;

    // WHEN: We query for value objects
    let result = simulate_query_entities(
        harness.ontology_path().as_path(),
        "SELECT ?vo WHERE { ?vo rdfs:subClassOf ddd:ValueObject }",
    )
    .await?;

    // THEN: Value objects returned
    assert_eq!(result.entities.len(), 2);  // UserId, Email
    assert!(result
        .entities
        .contains(&"http://test.example.org/UserId".to_string()));
    assert!(result
        .entities
        .contains(&"http://test.example.org/Email".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_query_properties_by_entity() -> Result<()> {
    // GIVEN: Ontology with properties
    let harness = TurtleAuthoringHarness::new()?;
    harness.write_ontology(ontology_with_properties())?;

    // WHEN: We query properties of User
    let result = simulate_query_entities(
        harness.ontology_path().as_path(),
        "SELECT ?prop WHERE { ?prop rdfs:domain test:User }",
    )
    .await?;

    // THEN: All User properties returned
    assert_eq!(result.entities.len(), 3);  // userId, userName, userEmail

    Ok(())
}

// =============================================================================
// Tests: Builder Pattern Integration
// =============================================================================

#[tokio::test]
async fn test_builder_pattern_creates_valid_ontology() -> Result<()> {
    // WHEN: We build ontology using fluent API
    let harness_result = OntologyBuilder::new()
        .add_aggregate("Product")
        .add_value_object("Price")
        .add_command("CreateProduct")
        .add_event("ProductCreated")
        .build();

    // THEN: Valid ontology created
    assert!(harness_result.is_ok());
    let harness = harness_result.unwrap();

    // AND: Contains expected entities
    harness.assert_class_is_aggregate_root("http://test.example.org/Product");
    harness.assert_class_is_value_object("http://test.example.org/Price");
    harness.assert_class_is_command("http://test.example.org/CreateProductCommand");
    harness.assert_class_is_event("http://test.example.org/ProductCreatedEvent");

    Ok(())
}

// =============================================================================
// Mock Implementation Helpers (Replace with real MCP tool calls)
// =============================================================================

#[derive(Debug)]
struct OntologyReadResult {
    valid: bool,
    class_count: usize,
    property_count: usize,
    classes: Vec<String>,
    properties: Vec<String>,
}

#[derive(Debug)]
struct ValidationResult {
    valid: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Debug)]
struct QueryResult {
    entities: Vec<String>,
}

async fn simulate_read_ontology(path: &Path) -> Result<OntologyReadResult> {
    // Mock implementation - replace with real MCP tool call
    let harness = OntologyTestHarness::parse_from_file(path)?;

    let classes = harness.get_classes()?;
    let properties = harness.get_properties()?;

    Ok(OntologyReadResult {
        valid: true,
        class_count: classes.len(),
        property_count: properties.len(),
        classes,
        properties,
    })
}

async fn simulate_add_entity(
    path: &Path,
    name: &str,
    entity_type: &str,
    description: Option<&str>,
) -> Result<()> {
    // Parse existing ontology
    let content = fs::read_to_string(path)?;

    // Check for duplicates
    if content.contains(&format!("test:{}", name)) {
        anyhow::bail!("Entity '{}' already exists", name);
    }

    // Add new entity
    let ddd_type = match entity_type {
        "AggregateRoot" => "ddd:AggregateRoot",
        "ValueObject" => "ddd:ValueObject",
        "Entity" => "ddd:Entity",
        _ => anyhow::bail!("Unknown entity type: {}", entity_type),
    };

    let desc_line = description
        .map(|d| format!("    rdfs:comment \"{}\"@en ;\n", d))
        .unwrap_or_default();

    let new_entity = format!(
        "\ntest:{} a owl:Class ;\n    rdfs:subClassOf {} ;\n{}    rdfs:label \"{}\"@en .\n",
        name, ddd_type, desc_line, name
    );

    fs::write(path, format!("{}{}", content, new_entity))?;

    // Validate syntax
    OntologyTestHarness::parse_from_file(path)?;

    Ok(())
}

async fn simulate_add_property(
    path: &Path,
    entity_name: &str,
    property_name: &str,
    property_type: &str,
    range: &str,
    description: Option<&str>,
) -> Result<()> {
    // Parse existing ontology
    let content = fs::read_to_string(path)?;

    // Check entity exists
    if !content.contains(&format!("test:{}", entity_name)) {
        anyhow::bail!("Entity '{}' not found", entity_name);
    }

    let owl_type = match property_type {
        "DatatypeProperty" => "owl:DatatypeProperty",
        "ObjectProperty" => "owl:ObjectProperty",
        _ => anyhow::bail!("Unknown property type: {}", property_type),
    };

    let desc_line = description
        .map(|d| format!("    rdfs:comment \"{}\"@en ;\n", d))
        .unwrap_or_default();

    let new_property = format!(
        "\ntest:{} a {} ;\n    rdfs:domain test:{} ;\n{}    rdfs:range {} .\n",
        property_name, owl_type, entity_name, desc_line, range
    );

    fs::write(path, format!("{}{}", content, new_property))?;

    // Validate syntax
    OntologyTestHarness::parse_from_file(path)?;

    Ok(())
}

async fn simulate_validate_syntax(path: &Path) -> Result<ValidationResult> {
    match OntologyTestHarness::parse_from_file(path) {
        Ok(_) => Ok(ValidationResult {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }),
        Err(e) => {
            let error_msg = e.to_string();
            Ok(ValidationResult {
                valid: false,
                errors: vec![error_msg],
                warnings: Vec::new(),
            })
        }
    }
}

async fn simulate_query_entities(path: &Path, sparql: &str) -> Result<QueryResult> {
    let harness = OntologyTestHarness::parse_from_file(path)?;

    // Execute SPARQL query
    let results = harness.query(sparql)?;

    let mut entities = Vec::new();
    if let oxigraph::sparql::QueryResults::Solutions(solutions) = results {
        for solution in solutions {
            let solution = solution?;
            if let Some(var_name) = solution.variables().first() {
                if let Some(oxigraph::model::Term::NamedNode(node)) = solution.get(var_name) {
                    entities.push(node.as_str().to_string());
                }
            }
        }
    }

    Ok(QueryResult { entities })
}
