//! MCP Testing Patterns - Reusable Test Utilities
//!
//! This example demonstrates reusable testing patterns and utilities
//! for testing Rust MCP servers. These patterns are based on the
//! ggen-mcp codebase and follow TPS Kaizen principles.
//!
//! # Usage
//!
//! Copy these patterns into your `tests/support/` directory and adapt
//! them for your specific MCP server.
//!
//! # Patterns Demonstrated
//!
//! 1. TestWorkspace - Isolated test environments
//! 2. OntologyBuilder - Fluent ontology construction
//! 3. TestMetrics - Test performance tracking
//! 4. AssertionHelpers - Rich assertions with context
//! 5. SparqlTestHelpers - SPARQL query testing
//! 6. PropertyTestGenerators - Custom proptest strategies

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

// ============================================================================
// TestWorkspace - Isolated Test Environments
// ============================================================================

/// Provides isolated temporary workspace for testing.
///
/// # Features
///
/// - Automatic cleanup on drop
/// - Unique directory per test
/// - Builder pattern for configuration
/// - Helper methods for common operations
///
/// # Example
///
/// ```rust
/// let workspace = TestWorkspace::new();
/// let file = workspace.create_file("test.ttl", "# RDF content");
/// let state = workspace.app_state();
/// // Automatic cleanup when workspace drops
/// ```
pub struct TestWorkspace {
    _tempdir: tempfile::TempDir,
    root: PathBuf,
}

impl TestWorkspace {
    /// Create new isolated test workspace
    pub fn new() -> Self {
        let tempdir = tempfile::tempdir().expect("create tempdir");
        let root = tempdir.path().to_path_buf();
        Self {
            _tempdir: tempdir,
            root,
        }
    }

    /// Get workspace root path
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get path relative to workspace root
    pub fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    /// Create file with content
    pub fn create_file(&self, name: &str, content: &str) -> PathBuf {
        let path = self.path(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent dirs");
        }
        std::fs::write(&path, content).expect("write file");
        path
    }

    /// Create TTL ontology file
    pub fn create_ontology(&self, name: &str, builder: OntologyBuilder) -> PathBuf {
        self.create_file(name, &builder.build_ttl())
    }

    /// Create SPARQL query file
    pub fn create_query(&self, name: &str, query: &str) -> PathBuf {
        self.create_file(name, query)
    }

    /// Create subdirectory
    pub fn create_dir(&self, name: &str) -> PathBuf {
        let path = self.path(name);
        std::fs::create_dir_all(&path).expect("create directory");
        path
    }

    /// List all files in workspace
    pub fn list_files(&self) -> Vec<PathBuf> {
        walkdir::WalkDir::new(&self.root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .collect()
    }

    /// Read file content
    pub fn read_file(&self, name: &str) -> String {
        std::fs::read_to_string(self.path(name)).expect("read file")
    }

    /// Check if file exists
    pub fn file_exists(&self, name: &str) -> bool {
        self.path(name).exists()
    }
}

impl Default for TestWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// OntologyBuilder - Fluent Ontology Construction
// ============================================================================

/// Fluent builder for creating test ontologies.
///
/// # Example
///
/// ```rust
/// let ttl = OntologyBuilder::new()
///     .add_prefix("ex", "http://example.org/")
///     .add_aggregate("User")
///     .add_property("User", "name", "string")
///     .add_command("CreateUser")
///     .add_event("UserCreated")
///     .build_ttl();
/// ```
#[derive(Debug, Clone)]
pub struct OntologyBuilder {
    prefixes: Vec<(String, String)>,
    triples: Vec<Triple>,
}

#[derive(Debug, Clone)]
struct Triple {
    subject: String,
    predicate: String,
    object: String,
}

impl OntologyBuilder {
    /// Create new ontology builder with standard prefixes
    pub fn new() -> Self {
        Self {
            prefixes: vec![
                ("ddd".into(), "https://ddd-patterns.dev#".into()),
                ("ggen".into(), "http://ggen.dev#".into()),
                ("rdf".into(), "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into()),
                ("rdfs".into(), "http://www.w3.org/2000/01/rdf-schema#".into()),
                ("xsd".into(), "http://www.w3.org/2001/XMLSchema#".into()),
            ],
            triples: Vec::new(),
        }
    }

    /// Add custom prefix
    pub fn add_prefix(mut self, prefix: &str, uri: &str) -> Self {
        self.prefixes.push((prefix.into(), uri.into()));
        self
    }

    /// Add DDD aggregate root
    pub fn add_aggregate(mut self, name: &str) -> Self {
        self.triples.push(Triple {
            subject: format!("ggen:{}", name),
            predicate: "a".into(),
            object: "ddd:AggregateRoot".into(),
        });
        self.triples.push(Triple {
            subject: format!("ggen:{}", name),
            predicate: "rdfs:label".into(),
            object: format!("\"{}\"", name),
        });
        self
    }

    /// Add property to entity
    pub fn add_property(mut self, entity: &str, property: &str, property_type: &str) -> Self {
        let property_uri = format!("ggen:{}", property);
        self.triples.push(Triple {
            subject: format!("ggen:{}", entity),
            predicate: "ddd:hasProperty".into(),
            object: property_uri.clone(),
        });
        self.triples.push(Triple {
            subject: property_uri,
            predicate: "rdfs:range".into(),
            object: format!("xsd:{}", property_type),
        });
        self
    }

    /// Add DDD command
    pub fn add_command(mut self, name: &str) -> Self {
        self.triples.push(Triple {
            subject: format!("ggen:{}", name),
            predicate: "a".into(),
            object: "ddd:Command".into(),
        });
        self
    }

    /// Add DDD event
    pub fn add_event(mut self, name: &str) -> Self {
        self.triples.push(Triple {
            subject: format!("ggen:{}", name),
            predicate: "a".into(),
            object: "ddd:DomainEvent".into(),
        });
        self
    }

    /// Add DDD value object
    pub fn add_value_object(mut self, name: &str) -> Self {
        self.triples.push(Triple {
            subject: format!("ggen:{}", name),
            predicate: "a".into(),
            object: "ddd:ValueObject".into(),
        });
        self
    }

    /// Add DDD repository
    pub fn add_repository(mut self, name: &str, for_aggregate: &str) -> Self {
        self.triples.push(Triple {
            subject: format!("ggen:{}", name),
            predicate: "a".into(),
            object: "ddd:Repository".into(),
        });
        self.triples.push(Triple {
            subject: format!("ggen:{}", name),
            predicate: "ddd:forAggregate".into(),
            object: format!("ggen:{}", for_aggregate),
        });
        self
    }

    /// Add invariant constraint
    pub fn add_invariant(mut self, entity: &str, invariant: &str) -> Self {
        self.triples.push(Triple {
            subject: format!("ggen:{}", entity),
            predicate: "ddd:hasInvariant".into(),
            object: format!("\"{}\"", invariant),
        });
        self
    }

    /// Build Turtle (TTL) format ontology
    pub fn build_ttl(self) -> String {
        let mut ttl = String::new();

        // Add prefixes
        for (prefix, uri) in &self.prefixes {
            ttl.push_str(&format!("@prefix {}: <{}> .\n", prefix, uri));
        }
        ttl.push('\n');

        // Add triples
        for triple in &self.triples {
            ttl.push_str(&format!(
                "{} {} {} .\n",
                triple.subject, triple.predicate, triple.object
            ));
        }

        ttl
    }

    /// Build RDF/XML format ontology
    pub fn build_rdf_xml(self) -> String {
        let mut xml = String::from("<?xml version=\"1.0\"?>\n");
        xml.push_str("<rdf:RDF\n");

        // Add namespace declarations
        for (prefix, uri) in &self.prefixes {
            xml.push_str(&format!("    xmlns:{}=\"{}\"\n", prefix, uri));
        }
        xml.push_str(">\n");

        // Add triples as RDF/XML
        // (Simplified - real implementation would group by subject)
        for triple in &self.triples {
            xml.push_str(&format!(
                "  <rdf:Description rdf:about=\"{}\">\n",
                triple.subject
            ));
            xml.push_str(&format!(
                "    <{} rdf:resource=\"{}\"/>\n",
                triple.predicate, triple.object
            ));
            xml.push_str("  </rdf:Description>\n");
        }

        xml.push_str("</rdf:RDF>\n");
        xml
    }
}

impl Default for OntologyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TestMetrics - Test Performance Tracking
// ============================================================================

/// Tracks test execution metrics for performance monitoring.
///
/// Implements TPS Kaizen principle: "Measure everything that matters"
///
/// # Example
///
/// ```rust
/// #[test]
/// fn test_with_metrics() {
///     let _metrics = TestMetrics::start("sparql_query_performance");
///     // Test code
///     // Metrics automatically printed on drop
/// }
/// ```
pub struct TestMetrics {
    name: String,
    start: Instant,
    checkpoints: Vec<(String, Instant)>,
}

impl TestMetrics {
    /// Start tracking test execution
    pub fn start(name: &str) -> Self {
        Self {
            name: name.to_string(),
            start: Instant::now(),
            checkpoints: Vec::new(),
        }
    }

    /// Add checkpoint for tracking intermediate progress
    pub fn checkpoint(&mut self, label: &str) {
        self.checkpoints
            .push((label.to_string(), Instant::now()));
    }

    /// Get elapsed time since start
    pub fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }

    /// Print metrics summary
    pub fn print_summary(&self) {
        eprintln!("\n=== Test Metrics: {} ===", self.name);
        eprintln!("Total time: {:?}", self.elapsed());

        if !self.checkpoints.is_empty() {
            eprintln!("\nCheckpoints:");
            let mut prev = self.start;
            for (label, instant) in &self.checkpoints {
                let duration = instant.duration_since(prev);
                eprintln!("  {}: {:?}", label, duration);
                prev = *instant;
            }
        }
        eprintln!("========================\n");
    }
}

impl Drop for TestMetrics {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("Test '{}' FAILED after {:?}", self.name, self.elapsed());
        } else {
            eprintln!("Test '{}' completed in {:?}", self.name, self.elapsed());
        }
    }
}

// ============================================================================
// AssertionHelpers - Rich Assertions with Context
// ============================================================================

/// Rich assertion helpers with detailed error messages.
///
/// Implements TPS Jidoka principle: "Automation with intelligence"
pub struct AssertionHelpers;

impl AssertionHelpers {
    /// Assert file exists with helpful error message
    pub fn assert_file_exists(path: &Path) {
        assert!(
            path.exists(),
            "Expected file to exist:\n\
            \tPath: {:?}\n\
            \tParent exists: {}\n\
            \tHint: Check file creation logic",
            path,
            path.parent().map_or(false, |p| p.exists())
        );
    }

    /// Assert file contains expected content
    pub fn assert_file_contains(path: &Path, expected: &str) {
        let content = std::fs::read_to_string(path).unwrap_or_else(|e| {
            panic!(
                "Failed to read file {:?}: {}\n\
                \tHint: Ensure file exists and is readable",
                path, e
            )
        });

        assert!(
            content.contains(expected),
            "File does not contain expected content:\n\
            \tFile: {:?}\n\
            \tExpected: {}\n\
            \tActual content (first 200 chars):\n\t{}",
            path,
            expected,
            &content.chars().take(200).collect::<String>()
        );
    }

    /// Assert TTL ontology is valid
    pub fn assert_valid_ttl(ttl: &str) {
        // Check balanced braces
        let open_braces = ttl.matches('{').count();
        let close_braces = ttl.matches('}').count();
        assert_eq!(
            open_braces, close_braces,
            "Unbalanced braces in TTL:\n\
            \tOpen: {}, Close: {}\n\
            \tTTL:\n{}",
            open_braces, close_braces, ttl
        );

        // Check for required prefixes
        assert!(
            ttl.contains("@prefix"),
            "TTL missing @prefix declarations:\n{}",
            ttl
        );
    }

    /// Assert SPARQL query is valid
    pub fn assert_valid_sparql(query: &str) {
        assert!(
            query.contains("SELECT")
                || query.contains("CONSTRUCT")
                || query.contains("ASK")
                || query.contains("DESCRIBE"),
            "Invalid SPARQL query - missing query type:\n{}",
            query
        );

        if query.contains("SELECT") {
            assert!(
                query.contains("WHERE"),
                "SELECT query missing WHERE clause:\n{}",
                query
            );
        }
    }

    /// Assert collection is not empty with context
    pub fn assert_not_empty<T>(collection: &[T], context: &str) {
        assert!(
            !collection.is_empty(),
            "Expected non-empty collection: {}\n\
            \tActual length: 0",
            context
        );
    }

    /// Assert error contains expected message
    pub fn assert_error_contains<E: std::fmt::Display>(
        result: Result<(), E>,
        expected_message: &str,
    ) {
        match result {
            Ok(_) => panic!(
                "Expected error containing '{}', but operation succeeded",
                expected_message
            ),
            Err(e) => {
                let error_message = e.to_string();
                assert!(
                    error_message.contains(expected_message),
                    "Error message does not contain expected text:\n\
                    \tExpected: {}\n\
                    \tActual: {}",
                    expected_message, error_message
                );
            }
        }
    }
}

// ============================================================================
// SparqlTestHelpers - SPARQL Query Testing Utilities
// ============================================================================

/// Helpers for testing SPARQL queries and results.
pub struct SparqlTestHelpers;

impl SparqlTestHelpers {
    /// Extract variable bindings from SPARQL result
    pub fn extract_bindings(
        results: &oxigraph::sparql::QueryResults,
        var_name: &str,
    ) -> Vec<String> {
        let mut bindings = Vec::new();

        if let oxigraph::sparql::QueryResults::Solutions(solutions) = results {
            for solution in solutions.clone() {
                if let Ok(sol) = solution {
                    if let Some(term) = sol.get(var_name) {
                        bindings.push(term.to_string());
                    }
                }
            }
        }

        bindings
    }

    /// Assert SPARQL query returns expected number of results
    pub fn assert_result_count(results: &oxigraph::sparql::QueryResults, expected: usize) {
        if let oxigraph::sparql::QueryResults::Solutions(solutions) = results {
            let actual = solutions.clone().count();
            assert_eq!(
                actual, expected,
                "SPARQL query returned unexpected number of results:\n\
                \tExpected: {}\n\
                \tActual: {}",
                expected, actual
            );
        } else {
            panic!("Expected Solutions result type");
        }
    }

    /// Create minimal test graph
    pub fn create_test_graph() -> oxigraph::store::Store {
        let store = oxigraph::store::Store::new().expect("create store");

        // Add some test triples
        use oxigraph::model::*;
        let ex = NamedNodeRef::new("http://example.org/").unwrap();

        store
            .insert(&Quad::new(
                NamedNode::new_unchecked(format!("{}Person1", ex)),
                NamedNode::new_unchecked(format!("{}name", ex)),
                Literal::new_simple_literal("Alice"),
                GraphName::DefaultGraph,
            ))
            .expect("insert triple");

        store
    }
}

// ============================================================================
// Property Test Generators - Custom Proptest Strategies
// ============================================================================

#[cfg(feature = "proptest")]
pub mod property_test_generators {
    use proptest::prelude::*;

    /// Generate valid SPARQL variable names
    pub fn valid_variable_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex("\\?[a-zA-Z_][a-zA-Z0-9_]{0,20}").expect("valid regex")
    }

    /// Generate valid IRIs
    pub fn valid_iri_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex("https?://[a-z]+\\.com/[a-z]+").expect("valid regex")
    }

    /// Generate malicious SPARQL injection attempts
    pub fn malicious_injection_strategy() -> impl Strategy<Value = String> {
        prop::sample::select(vec![
            "'; DROP TABLE users--".to_string(),
            "admin' UNION SELECT * FROM passwords".to_string(),
            "test # comment injection".to_string(),
            "FILTER (?x = 'attack')".to_string(),
            "' } UNION { ?s ?p ?o }".to_string(),
        ])
    }

    /// Generate test ontology structures
    pub fn ontology_structure_strategy() -> impl Strategy<Value = TestOntologyStructure> {
        (
            prop::collection::vec("[A-Z][a-zA-Z]+", 1..5),        // Aggregates
            prop::collection::vec("[A-Z][a-zA-Z]+Command", 1..10), // Commands
            prop::collection::vec("[A-Z][a-zA-Z]+Event", 1..10),   // Events
        )
            .prop_map(|(aggregates, commands, events)| TestOntologyStructure {
                aggregates,
                commands,
                events,
            })
    }

    #[derive(Debug, Clone)]
    pub struct TestOntologyStructure {
        pub aggregates: Vec<String>,
        pub commands: Vec<String>,
        pub events: Vec<String>,
    }
}

// ============================================================================
// Example Usage
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_test_workspace_usage() {
        // Create isolated test workspace
        let workspace = TestWorkspace::new();

        // Create test ontology
        let ontology = OntologyBuilder::new()
            .add_aggregate("User")
            .add_property("User", "name", "string")
            .add_command("CreateUser")
            .build_ttl();

        let ontology_path = workspace.create_file("test.ttl", &ontology);

        // Assertions
        AssertionHelpers::assert_file_exists(&ontology_path);
        AssertionHelpers::assert_file_contains(&ontology_path, "ddd:AggregateRoot");

        // Workspace automatically cleaned up on drop
    }

    #[test]
    fn example_ontology_builder_usage() {
        let ttl = OntologyBuilder::new()
            .add_aggregate("Order")
            .add_property("Order", "orderId", "string")
            .add_property("Order", "totalAmount", "decimal")
            .add_command("PlaceOrder")
            .add_event("OrderPlaced")
            .add_invariant("Order", "totalAmount must be positive")
            .build_ttl();

        AssertionHelpers::assert_valid_ttl(&ttl);
        assert!(ttl.contains("ggen:Order"));
        assert!(ttl.contains("ddd:AggregateRoot"));
        assert!(ttl.contains("PlaceOrder"));
    }

    #[test]
    fn example_test_metrics_usage() {
        let mut metrics = TestMetrics::start("ontology_loading");

        // Simulate test phases
        std::thread::sleep(std::time::Duration::from_millis(10));
        metrics.checkpoint("Parse TTL");

        std::thread::sleep(std::time::Duration::from_millis(5));
        metrics.checkpoint("Validate schema");

        std::thread::sleep(std::time::Duration::from_millis(15));
        metrics.checkpoint("Load into graph");

        // Metrics automatically printed on drop
    }

    #[test]
    fn example_assertion_helpers_usage() {
        let workspace = TestWorkspace::new();
        let file = workspace.create_file("test.rq", "SELECT ?s WHERE { ?s ?p ?o }");

        AssertionHelpers::assert_file_exists(&file);
        AssertionHelpers::assert_file_contains(&file, "SELECT");
        AssertionHelpers::assert_valid_sparql(&workspace.read_file("test.rq"));
    }

    #[test]
    #[cfg(feature = "proptest")]
    fn example_property_test_usage() {
        use proptest::prelude::*;
        use property_test_generators::*;

        proptest!(|(var in valid_variable_strategy())| {
            // Property: All generated variables should start with ?
            assert!(var.starts_with('?'));

            // Property: Variable names should be valid length
            assert!(var.len() >= 2 && var.len() <= 22);
        });
    }
}

// ============================================================================
// Documentation Examples
// ============================================================================

/// # Complete Testing Workflow Example
///
/// This example demonstrates a complete testing workflow using all utilities:
///
/// ```rust,no_run
/// use mcp_testing_patterns::*;
///
/// #[test]
/// fn complete_workflow_test() {
///     // 1. Start metrics tracking
///     let mut metrics = TestMetrics::start("ddd_pipeline_test");
///
///     // 2. Set up isolated workspace
///     let workspace = TestWorkspace::new();
///
///     // 3. Build test ontology
///     let ontology = OntologyBuilder::new()
///         .add_aggregate("Order")
///         .add_command("PlaceOrder")
///         .add_event("OrderPlaced")
///         .build_ttl();
///
///     let ontology_path = workspace.create_ontology("order.ttl", ontology);
///     metrics.checkpoint("Ontology created");
///
///     // 4. Validate ontology file
///     AssertionHelpers::assert_file_exists(&ontology_path);
///     AssertionHelpers::assert_valid_ttl(&workspace.read_file("order.ttl"));
///     metrics.checkpoint("Validation complete");
///
///     // 5. Test SPARQL queries
///     let query = "SELECT ?aggregate WHERE { ?aggregate a ddd:AggregateRoot }";
///     AssertionHelpers::assert_valid_sparql(query);
///     metrics.checkpoint("Query validation complete");
///
///     // Metrics printed automatically on drop
/// }
/// ```

fn main() {
    println!("MCP Testing Patterns - Example Utilities");
    println!("==========================================");
    println!();
    println!("This file contains reusable testing patterns for MCP servers.");
    println!("Copy the patterns you need into your tests/support/ directory.");
    println!();
    println!("Available utilities:");
    println!("  - TestWorkspace: Isolated test environments");
    println!("  - OntologyBuilder: Fluent ontology construction");
    println!("  - TestMetrics: Performance tracking");
    println!("  - AssertionHelpers: Rich assertions");
    println!("  - SparqlTestHelpers: SPARQL testing");
    println!("  - PropertyTestGenerators: Proptest strategies");
    println!();
    println!("Run tests with: cargo test --example mcp_testing_patterns");
}
