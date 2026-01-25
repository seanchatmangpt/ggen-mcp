//! Turtle Ontology Test Harness
//!
//! Comprehensive Chicago-style TDD test harness for Turtle/TTL ontology parsing and validation.
//!
//! This harness provides:
//! - Fluent test builders for ontology construction
//! - State-based assertions for verification
//! - Triple query and validation helpers
//! - Fixture-based testing support
//! - SHACL validation integration
//! - DDD pattern compliance checking
//!
//! # Chicago-Style TDD
//!
//! This harness follows Chicago school TDD principles:
//! - State-based verification over interaction testing
//! - Real dependencies (oxigraph Store) instead of mocks
//! - Focus on observable behavior and final state
//! - Clear Given-When-Then structure
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use turtle_ontology_harness::{OntologyTestHarness, OntologyBuilder};
//!
//! #[test]
//! fn test_user_aggregate_structure() {
//!     // GIVEN: A user aggregate ontology
//!     let harness = OntologyTestHarness::new()
//!         .parse_from_file("tests/fixtures/ttl/valid/user_aggregate.ttl")
//!         .expect("Failed to parse ontology");
//!
//!     // WHEN: We validate the structure
//!     let result = harness.validate();
//!
//!     // THEN: The ontology should be valid
//!     assert!(result.is_valid(), "Ontology validation failed: {:?}", result.errors());
//!
//!     // AND: It should have the expected classes
//!     harness.assert_class_defined("user:User");
//!     harness.assert_class_is_aggregate_root("user:User");
//!     harness.assert_property_exists("user:email");
//! }
//! ```

use anyhow::{Context, Result, anyhow};
use oxigraph::io::GraphFormat;
use oxigraph::model::{GraphNameRef, NamedNode, NamedNodeRef, Subject, Term, Triple};
use oxigraph::sparql::{Query, QueryResults};
use oxigraph::store::Store;
use spreadsheet_mcp::ontology::{
    ConsistencyChecker, ConsistencyReport, HashVerifier, NamespaceManager, SchemaValidator,
    ValidationReport,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

// =============================================================================
// ONTOLOGY TEST HARNESS
// =============================================================================

/// Main test harness for Turtle ontology testing
#[derive(Clone)]
pub struct OntologyTestHarness {
    store: Store,
    namespace_manager: NamespaceManager,
    source_info: SourceInfo,
}

#[derive(Clone, Debug)]
struct SourceInfo {
    source_type: SourceType,
    description: String,
}

#[derive(Clone, Debug, PartialEq)]
enum SourceType {
    File(PathBuf),
    String,
    Builder,
}

impl OntologyTestHarness {
    /// Create a new empty test harness
    pub fn new() -> Self {
        let mut namespace_manager = NamespaceManager::new();

        // Pre-register common test namespaces
        let _ = namespace_manager.register("test", "http://test.example.org/");
        let _ = namespace_manager.register("user", "http://ggen-mcp.dev/domain/user#");
        let _ = namespace_manager.register("order", "http://ggen-mcp.dev/domain/order#");

        Self {
            store: Store::new().expect("Failed to create RDF store"),
            namespace_manager,
            source_info: SourceInfo {
                source_type: SourceType::Builder,
                description: "Empty harness".to_string(),
            },
        }
    }

    /// Parse ontology from a TTL file
    pub fn parse_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let store = Store::new()?;

        store
            .load_from_file(path, GraphFormat::Turtle)
            .with_context(|| format!("Failed to parse TTL file: {:?}", path))?;

        let mut harness = Self::new();
        harness.store = store;
        harness.source_info = SourceInfo {
            source_type: SourceType::File(path.to_path_buf()),
            description: format!("Loaded from file: {}", path.display()),
        };

        Ok(harness)
    }

    /// Parse ontology from a TTL string
    pub fn parse_from_string(ttl: &str) -> Result<Self> {
        let store = Store::new()?;

        store
            .load_from_reader(GraphFormat::Turtle, ttl.as_bytes())
            .context("Failed to parse TTL string")?;

        let mut harness = Self::new();
        harness.store = store;
        harness.source_info = SourceInfo {
            source_type: SourceType::String,
            description: format!("Loaded from string ({} bytes)", ttl.len()),
        };

        Ok(harness)
    }

    /// Get reference to the underlying RDF store
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Get source information
    pub fn source_info(&self) -> &SourceInfo {
        &self.source_info
    }

    // =========================================================================
    // VALIDATION METHODS
    // =========================================================================

    /// Run full consistency validation
    pub fn validate_consistency(&self) -> ConsistencyReport {
        let checker = ConsistencyChecker::new(self.store.clone());
        checker.check_all()
    }

    /// Run schema validation
    pub fn validate_schema(&self) -> ValidationReport {
        let validator = SchemaValidator::new(self.store.clone());
        validator.validate_all()
    }

    /// Run all validations and return combined result
    pub fn validate(&self) -> ValidationResult {
        let consistency = self.validate_consistency();
        let schema = self.validate_schema();

        ValidationResult {
            consistency,
            schema,
        }
    }

    /// Compute ontology hash for change detection
    pub fn compute_hash(&self) -> Result<String> {
        let verifier = HashVerifier::new(self.store.clone());
        verifier.compute_hash()
    }

    // =========================================================================
    // QUERY METHODS
    // =========================================================================

    /// Execute a SPARQL query
    pub fn query(&self, sparql: &str) -> Result<QueryResults> {
        let query = Query::parse(sparql, None)?;
        Ok(self.store.query(query)?)
    }

    /// Count triples matching a pattern
    pub fn count_triples(
        &self,
        subject: Option<&str>,
        predicate: Option<&str>,
        object: Option<&str>,
    ) -> Result<usize> {
        let s_term = subject
            .map(|s| NamedNode::new(s))
            .transpose()?
            .map(|n| Subject::NamedNode(n));

        let p_term = predicate.map(|p| NamedNode::new(p)).transpose()?;

        let o_term = object
            .map(|o| NamedNode::new(o))
            .transpose()?
            .map(|n| Term::NamedNode(n));

        let count = self
            .store
            .quads_for_pattern(
                s_term.as_ref().map(|s| s.as_ref()),
                p_term.as_ref().map(|p| p.as_ref()),
                o_term.as_ref().map(|o| o.as_ref()),
                Some(GraphNameRef::DefaultGraph),
            )
            .count();

        Ok(count)
    }

    /// Get all classes defined in the ontology
    pub fn get_classes(&self) -> Result<Vec<String>> {
        let query = r#"
            PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
            PREFIX owl: <http://www.w3.org/2002/07/owl#>

            SELECT DISTINCT ?class WHERE {
                { ?class a owl:Class }
                UNION
                { ?class a rdfs:Class }
            }
        "#;

        self.extract_uris_from_query(query)
    }

    /// Get all properties defined in the ontology
    pub fn get_properties(&self) -> Result<Vec<String>> {
        let query = r#"
            PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
            PREFIX owl: <http://www.w3.org/2002/07/owl#>

            SELECT DISTINCT ?prop WHERE {
                { ?prop a owl:ObjectProperty }
                UNION
                { ?prop a owl:DatatypeProperty }
                UNION
                { ?prop a rdf:Property }
            }
        "#;

        self.extract_uris_from_query(query)
    }

    /// Get all aggregate roots
    pub fn get_aggregate_roots(&self) -> Result<Vec<String>> {
        let query = r#"
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
            PREFIX ddd: <http://ggen-mcp.dev/ontology/ddd#>

            SELECT DISTINCT ?aggregate WHERE {
                ?aggregate rdfs:subClassOf ddd:AggregateRoot .
            }
        "#;

        self.extract_uris_from_query(query)
    }

    /// Get all value objects
    pub fn get_value_objects(&self) -> Result<Vec<String>> {
        let query = r#"
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
            PREFIX ddd: <http://ggen-mcp.dev/ontology/ddd#>

            SELECT DISTINCT ?vo WHERE {
                ?vo rdfs:subClassOf ddd:ValueObject .
            }
        "#;

        self.extract_uris_from_query(query)
    }

    /// Get all commands
    pub fn get_commands(&self) -> Result<Vec<String>> {
        let query = r#"
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
            PREFIX ddd: <http://ggen-mcp.dev/ontology/ddd#>

            SELECT DISTINCT ?cmd WHERE {
                ?cmd rdfs:subClassOf ddd:Command .
            }
        "#;

        self.extract_uris_from_query(query)
    }

    /// Get all events
    pub fn get_events(&self) -> Result<Vec<String>> {
        let query = r#"
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
            PREFIX ddd: <http://ggen-mcp.dev/ontology/ddd#>

            SELECT DISTINCT ?event WHERE {
                ?event rdfs:subClassOf ddd:DomainEvent .
            }
        "#;

        self.extract_uris_from_query(query)
    }

    /// Helper to extract URIs from a SELECT query
    fn extract_uris_from_query(&self, sparql: &str) -> Result<Vec<String>> {
        let results = self.query(sparql)?;

        let mut uris = Vec::new();
        if let QueryResults::Solutions(solutions) = results {
            for solution in solutions {
                let solution = solution?;
                if let Some(var_name) = solution.variables().first() {
                    if let Some(Term::NamedNode(node)) = solution.get(var_name) {
                        uris.push(node.as_str().to_string());
                    }
                }
            }
        }

        Ok(uris)
    }

    // =========================================================================
    // ASSERTION HELPERS
    // =========================================================================

    /// Assert that a triple exists in the ontology
    pub fn assert_triple_exists(&self, subject: &str, predicate: &str, object: &str) {
        let count = self
            .count_triples(Some(subject), Some(predicate), Some(object))
            .expect("Failed to query triples");

        assert!(
            count > 0,
            "Expected triple <{} {} {}> to exist, but it was not found",
            subject,
            predicate,
            object
        );
    }

    /// Assert that a class is defined
    pub fn assert_class_defined(&self, class_uri: &str) {
        let classes = self.get_classes().expect("Failed to get classes");
        assert!(
            classes.contains(&class_uri.to_string()),
            "Expected class <{}> to be defined, but it was not found.\nDefined classes: {:?}",
            class_uri,
            classes
        );
    }

    /// Assert that a property is defined
    pub fn assert_property_exists(&self, property_uri: &str) {
        let properties = self.get_properties().expect("Failed to get properties");
        assert!(
            properties.contains(&property_uri.to_string()),
            "Expected property <{}> to be defined, but it was not found.\nDefined properties: {:?}",
            property_uri,
            properties
        );
    }

    /// Assert that a class is an aggregate root
    pub fn assert_class_is_aggregate_root(&self, class_uri: &str) {
        let aggregates = self
            .get_aggregate_roots()
            .expect("Failed to get aggregates");
        assert!(
            aggregates.contains(&class_uri.to_string()),
            "Expected <{}> to be an AggregateRoot, but it was not found.\nDefined aggregates: {:?}",
            class_uri,
            aggregates
        );
    }

    /// Assert that a class is a value object
    pub fn assert_class_is_value_object(&self, class_uri: &str) {
        let vos = self
            .get_value_objects()
            .expect("Failed to get value objects");
        assert!(
            vos.contains(&class_uri.to_string()),
            "Expected <{}> to be a ValueObject, but it was not found.\nDefined value objects: {:?}",
            class_uri,
            vos
        );
    }

    /// Assert that a class is a command
    pub fn assert_class_is_command(&self, class_uri: &str) {
        let commands = self.get_commands().expect("Failed to get commands");
        assert!(
            commands.contains(&class_uri.to_string()),
            "Expected <{}> to be a Command, but it was not found.\nDefined commands: {:?}",
            class_uri,
            commands
        );
    }

    /// Assert that a class is an event
    pub fn assert_class_is_event(&self, class_uri: &str) {
        let events = self.get_events().expect("Failed to get events");
        assert!(
            events.contains(&class_uri.to_string()),
            "Expected <{}> to be a DomainEvent, but it was not found.\nDefined events: {:?}",
            class_uri,
            events
        );
    }

    /// Assert that a property has a specific domain
    pub fn assert_property_domain(&self, property_uri: &str, domain_uri: &str) {
        self.assert_triple_exists(
            property_uri,
            "http://www.w3.org/2000/01/rdf-schema#domain",
            domain_uri,
        );
    }

    /// Assert that a property has a specific range
    pub fn assert_property_range(&self, property_uri: &str, range_uri: &str) {
        self.assert_triple_exists(
            property_uri,
            "http://www.w3.org/2000/01/rdf-schema#range",
            range_uri,
        );
    }

    /// Assert that ontology is valid
    pub fn assert_valid(&self) {
        let result = self.validate();
        assert!(
            result.is_valid(),
            "Ontology validation failed:\n{:#?}",
            result
        );
    }

    /// Assert that ontology has specific number of classes
    pub fn assert_class_count(&self, expected: usize) {
        let classes = self.get_classes().expect("Failed to get classes");
        assert_eq!(
            classes.len(),
            expected,
            "Expected {} classes, found {}.\nClasses: {:?}",
            expected,
            classes.len(),
            classes
        );
    }

    /// Assert that ontology has specific number of properties
    pub fn assert_property_count(&self, expected: usize) {
        let properties = self.get_properties().expect("Failed to get properties");
        assert_eq!(
            properties.len(),
            expected,
            "Expected {} properties, found {}.\nProperties: {:?}",
            expected,
            properties.len(),
            properties
        );
    }

    /// Assert that aggregate has required DDD structure
    pub fn assert_aggregate_structure(&self, aggregate_uri: &str) {
        // Must be defined as aggregate root
        self.assert_class_is_aggregate_root(aggregate_uri);

        // Must have at least one property
        let query = format!(
            r#"
            PREFIX ddd: <http://ggen-mcp.dev/ontology/ddd#>

            SELECT ?prop WHERE {{
                <{aggregate}> ddd:hasProperty ?prop .
            }}
            "#,
            aggregate = aggregate_uri
        );

        let properties = self
            .extract_uris_from_query(&query)
            .expect("Failed to query properties");

        assert!(
            !properties.is_empty(),
            "Aggregate <{}> must have at least one property",
            aggregate_uri
        );
    }
}

// =============================================================================
// VALIDATION RESULT
// =============================================================================

#[derive(Debug)]
pub struct ValidationResult {
    pub consistency: ConsistencyReport,
    pub schema: ValidationReport,
}

impl ValidationResult {
    /// Check if all validations passed
    pub fn is_valid(&self) -> bool {
        self.consistency.valid && self.schema.valid
    }

    /// Get all errors from both validations
    pub fn errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        errors.extend(self.consistency.errors.clone());
        errors.extend(self.schema.errors.clone());
        errors
    }

    /// Get all warnings from both validations
    pub fn warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        warnings.extend(self.consistency.warnings.clone());
        warnings.extend(self.schema.warnings.clone());
        warnings
    }
}

// =============================================================================
// ONTOLOGY BUILDER
// =============================================================================

/// Fluent builder for constructing ontologies in tests
pub struct OntologyBuilder {
    ttl_parts: Vec<String>,
    prefixes: HashMap<String, String>,
}

impl OntologyBuilder {
    pub fn new() -> Self {
        let mut prefixes = HashMap::new();

        // Add standard prefixes
        prefixes.insert(
            "rdf".to_string(),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string(),
        );
        prefixes.insert(
            "rdfs".to_string(),
            "http://www.w3.org/2000/01/rdf-schema#".to_string(),
        );
        prefixes.insert(
            "owl".to_string(),
            "http://www.w3.org/2002/07/owl#".to_string(),
        );
        prefixes.insert(
            "xsd".to_string(),
            "http://www.w3.org/2001/XMLSchema#".to_string(),
        );
        prefixes.insert("sh".to_string(), "http://www.w3.org/ns/shacl#".to_string());
        prefixes.insert(
            "ddd".to_string(),
            "http://ggen-mcp.dev/ontology/ddd#".to_string(),
        );
        prefixes.insert("test".to_string(), "http://test.example.org/".to_string());

        Self {
            ttl_parts: Vec::new(),
            prefixes,
        }
    }

    /// Add a custom prefix
    pub fn with_prefix(mut self, prefix: &str, uri: &str) -> Self {
        self.prefixes.insert(prefix.to_string(), uri.to_string());
        self
    }

    /// Add an aggregate root
    pub fn add_aggregate(mut self, name: &str) -> Self {
        let ttl = format!(
            r#"
test:{name} a owl:Class ;
    rdfs:subClassOf ddd:AggregateRoot ;
    rdfs:label "{name}"@en ;
    ddd:hasProperty test:{name}Id .

test:{name}Id a owl:ObjectProperty ;
    rdfs:domain test:{name} ;
    rdfs:range test:{name}IdType .

test:{name}IdType a owl:Class ;
    rdfs:subClassOf ddd:ValueObject ;
    rdfs:label "{name} ID"@en .
"#,
            name = name
        );
        self.ttl_parts.push(ttl);
        self
    }

    /// Add a value object
    pub fn add_value_object(mut self, name: &str) -> Self {
        let ttl = format!(
            r#"
test:{name} a owl:Class ;
    rdfs:subClassOf ddd:ValueObject ;
    rdfs:label "{name}"@en ;
    ddd:hasProperty test:{name}Value .

test:{name}Value a owl:DatatypeProperty ;
    rdfs:domain test:{name} ;
    rdfs:range xsd:string .
"#,
            name = name
        );
        self.ttl_parts.push(ttl);
        self
    }

    /// Add a command
    pub fn add_command(mut self, name: &str) -> Self {
        let ttl = format!(
            r#"
test:{name}Command a owl:Class ;
    rdfs:subClassOf ddd:Command ;
    rdfs:label "{name} Command"@en .
"#,
            name = name
        );
        self.ttl_parts.push(ttl);
        self
    }

    /// Add a domain event
    pub fn add_event(mut self, name: &str) -> Self {
        let ttl = format!(
            r#"
test:{name}Event a owl:Class ;
    rdfs:subClassOf ddd:DomainEvent ;
    rdfs:label "{name} Event"@en ;
    ddd:hasProperty test:timestamp .

test:timestamp a owl:DatatypeProperty ;
    rdfs:range xsd:dateTime .
"#,
            name = name
        );
        self.ttl_parts.push(ttl);
        self
    }

    /// Add a repository
    pub fn add_repository(mut self, name: &str, aggregate: &str) -> Self {
        let ttl = format!(
            r#"
test:{name}Repository a owl:Class ;
    rdfs:subClassOf ddd:Repository ;
    rdfs:label "{name} Repository"@en ;
    ddd:forAggregate test:{aggregate} .
"#,
            name = name,
            aggregate = aggregate
        );
        self.ttl_parts.push(ttl);
        self
    }

    /// Add raw TTL
    pub fn add_raw_ttl(mut self, ttl: &str) -> Self {
        self.ttl_parts.push(ttl.to_string());
        self
    }

    /// Build the complete TTL string
    pub fn build_ttl(self) -> String {
        let mut result = String::new();

        // Add prefixes
        for (prefix, uri) in &self.prefixes {
            result.push_str(&format!("@prefix {}: <{}> .\n", prefix, uri));
        }
        result.push('\n');

        // Add ontology parts
        for part in self.ttl_parts {
            result.push_str(&part);
            result.push('\n');
        }

        result
    }

    /// Build and parse into a test harness
    pub fn build(self) -> Result<OntologyTestHarness> {
        let ttl = self.build_ttl();
        OntologyTestHarness::parse_from_string(&ttl)
    }
}

impl Default for OntologyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_creation() {
        let harness = OntologyTestHarness::new();
        assert!(harness.store().is_empty().unwrap());
    }

    #[test]
    fn test_parse_valid_user_aggregate() {
        let harness =
            OntologyTestHarness::parse_from_file("tests/fixtures/ttl/valid/user_aggregate.ttl")
                .expect("Failed to parse user aggregate");

        // Verify basic structure
        harness.assert_class_defined("http://ggen-mcp.dev/domain/user#User");
        harness.assert_class_is_aggregate_root("http://ggen-mcp.dev/domain/user#User");
        harness.assert_class_is_value_object("http://ggen-mcp.dev/domain/user#Email");
    }

    #[test]
    fn test_parse_valid_order_aggregate() {
        let harness =
            OntologyTestHarness::parse_from_file("tests/fixtures/ttl/valid/order_aggregate.ttl")
                .expect("Failed to parse order aggregate");

        harness.assert_class_defined("http://ggen-mcp.dev/domain/order#Order");
        harness.assert_class_is_aggregate_root("http://ggen-mcp.dev/domain/order#Order");
    }

    #[test]
    fn test_parse_syntax_error() {
        let result =
            OntologyTestHarness::parse_from_file("tests/fixtures/ttl/invalid/syntax_error.ttl");

        assert!(result.is_err(), "Should fail to parse syntax errors");
    }

    #[test]
    fn test_detect_circular_dependencies() {
        let harness = OntologyTestHarness::parse_from_file(
            "tests/fixtures/ttl/invalid/circular_dependencies.ttl",
        )
        .expect("Failed to parse");

        let result = harness.validate_consistency();
        assert!(!result.valid, "Should detect circular dependencies");
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("Cyclic") || e.contains("cycle")),
            "Should report cyclic hierarchy: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_builder_pattern() {
        let harness = OntologyBuilder::new()
            .add_aggregate("User")
            .add_value_object("Email")
            .add_command("CreateUser")
            .add_event("UserCreated")
            .add_repository("User", "User")
            .build()
            .expect("Failed to build ontology");

        // Verify generated ontology
        harness.assert_class_is_aggregate_root("http://test.example.org/User");
        harness.assert_class_is_value_object("http://test.example.org/Email");
        harness.assert_class_is_command("http://test.example.org/CreateUserCommand");
        harness.assert_class_is_event("http://test.example.org/UserCreatedEvent");
    }

    #[test]
    fn test_triple_assertion() {
        let ttl = r#"
            @prefix ex: <http://example.org/> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

            ex:MyClass rdfs:subClassOf ex:ParentClass .
        "#;

        let harness = OntologyTestHarness::parse_from_string(ttl).expect("Failed to parse");

        harness.assert_triple_exists(
            "http://example.org/MyClass",
            "http://www.w3.org/2000/01/rdf-schema#subClassOf",
            "http://example.org/ParentClass",
        );
    }

    #[test]
    fn test_query_aggregate_roots() {
        let harness =
            OntologyTestHarness::parse_from_file("tests/fixtures/ttl/valid/user_aggregate.ttl")
                .expect("Failed to parse");

        let aggregates = harness
            .get_aggregate_roots()
            .expect("Failed to get aggregates");
        assert!(aggregates.len() > 0, "Should find at least one aggregate");
        assert!(
            aggregates.contains(&"http://ggen-mcp.dev/domain/user#User".to_string()),
            "Should find User aggregate"
        );
    }

    #[test]
    fn test_aggregate_structure_validation() {
        let harness =
            OntologyTestHarness::parse_from_file("tests/fixtures/ttl/valid/user_aggregate.ttl")
                .expect("Failed to parse");

        harness.assert_aggregate_structure("http://ggen-mcp.dev/domain/user#User");
    }

    #[test]
    #[should_panic(expected = "must have at least one property")]
    fn test_invalid_aggregate_structure() {
        let ttl = r#"
            @prefix test: <http://test.example.org/> .
            @prefix ddd: <http://ggen-mcp.dev/ontology/ddd#> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            @prefix owl: <http://www.w3.org/2002/07/owl#> .

            test:EmptyAggregate a owl:Class ;
                rdfs:subClassOf ddd:AggregateRoot .
        "#;

        let harness = OntologyTestHarness::parse_from_string(ttl).expect("Failed to parse");

        harness.assert_aggregate_structure("http://test.example.org/EmptyAggregate");
    }

    #[test]
    fn test_count_triples() {
        let ttl = r#"
            @prefix ex: <http://example.org/> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

            ex:Class1 rdfs:label "Class 1" .
            ex:Class2 rdfs:label "Class 2" .
            ex:Class3 rdfs:label "Class 3" .
        "#;

        let harness = OntologyTestHarness::parse_from_string(ttl).expect("Failed to parse");

        let count = harness
            .count_triples(
                None,
                Some("http://www.w3.org/2000/01/rdf-schema#label"),
                None,
            )
            .expect("Failed to count");

        assert_eq!(count, 3, "Should find 3 label triples");
    }

    #[test]
    fn test_mcp_tools_fixture() {
        let harness =
            OntologyTestHarness::parse_from_file("tests/fixtures/ttl/valid/mcp_tools.ttl")
                .expect("Failed to parse MCP tools");

        // Verify tools are defined
        let query = r#"
            PREFIX mcp: <http://ggen-mcp.dev/ontology/mcp#>
            SELECT ?tool WHERE {
                ?tool a mcp:Tool .
            }
        "#;

        let tools = harness
            .extract_uris_from_query(query)
            .expect("Failed to query tools");

        assert!(tools.len() >= 4, "Should have at least 4 tools defined");
    }
}
