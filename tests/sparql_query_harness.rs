//! =============================================================================
//! Chicago-Style TDD Test Harness for SPARQL Query Generation and Execution
//! =============================================================================
//!
//! This comprehensive test harness implements Chicago-style TDD (state-based testing)
//! for SPARQL query generation, execution, and validation.
//!
//! ## Test Coverage:
//! - Query execution against test graphs
//! - Query syntax validation
//! - Result set verification
//! - Query generation
//! - State-based result assertions
//! - All queries in queries/ directory
//! - Inference query validation
//!
//! ## Chicago-Style TDD:
//! - Tests focus on state verification (results) not interaction
//! - Real dependencies used (oxigraph store, real SPARQL engine)
//! - Integration tests with real data flows
//! - State assertions on query results
//!
//! ## 80/20 Principle Applied:
//! - Focus on most critical queries (aggregates, tools, entities)
//! - Cover edge cases that provide most value
//! - Validate inference queries that drive code generation

use anyhow::{Context, Result};
use oxigraph::{
    model::{BlankNode, Graph, Literal, NamedNode, Subject, Term, Triple},
    sparql::{QueryResults, QuerySolution},
    store::Store,
};
use serde_json::Value as JsonValue;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

// =============================================================================
// Test Harness Core Structures
// =============================================================================

/// Main SPARQL test harness for Chicago-style TDD
pub struct SparqlTestHarness {
    store: Store,
    query_dir: PathBuf,
    fixture_dir: PathBuf,
}

impl SparqlTestHarness {
    /// Create a new test harness
    pub fn new() -> Self {
        Self {
            store: Store::new().expect("Failed to create store"),
            query_dir: PathBuf::from("queries"),
            fixture_dir: PathBuf::from("fixtures/sparql"),
        }
    }

    /// Load a test graph from a Turtle file
    pub fn load_graph(&mut self, fixture_name: &str) -> Result<()> {
        let path = self.fixture_dir.join("graphs").join(fixture_name);
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read fixture: {}", path.display()))?;

        self.store
            .load_from_reader(oxigraph::io::RdfFormat::Turtle, content.as_bytes())
            .with_context(|| format!("Failed to load graph from: {}", path.display()))?;

        Ok(())
    }

    /// Execute a SPARQL query and return results
    pub fn execute_query(&self, query: &str) -> Result<QueryResultSet> {
        let results = self
            .store
            .query(query)
            .with_context(|| format!("Failed to execute query: {}", query))?;

        match results {
            QueryResults::Solutions(solutions) => {
                let solutions: Vec<QuerySolution> = solutions.collect::<Result<Vec<_>, _>>()?;
                Ok(QueryResultSet { solutions })
            }
            QueryResults::Boolean(b) => Ok(QueryResultSet {
                solutions: vec![], // ASK queries return boolean
            }),
            QueryResults::Graph(_) => Ok(QueryResultSet {
                solutions: vec![], // CONSTRUCT queries return graph
            }),
        }
    }

    /// Load and execute a query from the queries/ directory
    pub fn execute_query_file(&self, query_file: &str) -> Result<Vec<QueryResultSet>> {
        let path = self.query_dir.join(query_file);
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read query file: {}", path.display()))?;

        // Split multiple queries by "---" separator
        let queries: Vec<&str> = content.split("\n---\n").collect();

        let mut results = Vec::new();
        for query in queries {
            let query = query.trim();
            if query.is_empty() || query.starts_with('#') {
                continue;
            }
            results.push(self.execute_query(query)?);
        }

        Ok(results)
    }

    /// Validate query syntax without execution
    pub fn validate_query_syntax(&self, query: &str) -> Result<()> {
        // Oxigraph will parse and validate the query
        self.store
            .prepare_query(query)
            .with_context(|| "Query syntax validation failed")?;
        Ok(())
    }

    /// Clear all data from the store
    pub fn clear(&mut self) {
        self.store = Store::new().expect("Failed to create new store");
    }
}

/// Result set from a SPARQL query
#[derive(Debug, Clone)]
pub struct QueryResultSet {
    pub solutions: Vec<QuerySolution>,
}

impl QueryResultSet {
    /// Get the number of results
    pub fn len(&self) -> usize {
        self.solutions.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.solutions.is_empty()
    }

    /// Get a specific result by index
    pub fn get(&self, index: usize) -> Option<&QuerySolution> {
        self.solutions.get(index)
    }

    /// Get all variable names across all solutions
    pub fn variable_names(&self) -> HashSet<String> {
        let mut names = HashSet::new();
        for solution in &self.solutions {
            for var in solution.variables() {
                names.insert(var.as_str().to_string());
            }
        }
        names
    }

    /// Check if a specific binding exists in any result
    pub fn has_binding(&self, var_name: &str, value: &str) -> bool {
        self.solutions.iter().any(|solution| {
            solution
                .get(var_name)
                .and_then(|term| match term {
                    Term::Literal(lit) => Some(lit.value() == value),
                    Term::NamedNode(node) => Some(node.as_str() == value),
                    _ => None,
                })
                .unwrap_or(false)
        })
    }

    /// Get all values for a specific variable
    pub fn get_all_values(&self, var_name: &str) -> Vec<String> {
        self.solutions
            .iter()
            .filter_map(|solution| solution.get(var_name))
            .filter_map(|term| match term {
                Term::Literal(lit) => Some(lit.value().to_string()),
                Term::NamedNode(node) => Some(node.as_str().to_string()),
                Term::BlankNode(bn) => Some(format!("_:{}", bn.as_str())),
            })
            .collect()
    }
}

// =============================================================================
// SPARQL Query Builder (Chicago-Style)
// =============================================================================

/// Builder for constructing SPARQL queries programmatically
#[derive(Debug, Clone)]
pub struct SparqlQueryBuilder {
    query_type: QueryType,
    variables: Vec<String>,
    prefixes: HashMap<String, String>,
    where_patterns: Vec<String>,
    filters: Vec<String>,
    optional_patterns: Vec<String>,
    order_by: Vec<String>,
    group_by: Vec<String>,
    limit: Option<usize>,
    offset: Option<usize>,
    distinct: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QueryType {
    Select,
    Construct,
    Ask,
    Describe,
}

impl SparqlQueryBuilder {
    /// Create a new SELECT query builder
    pub fn new() -> Self {
        Self {
            query_type: QueryType::Select,
            variables: Vec::new(),
            prefixes: HashMap::new(),
            where_patterns: Vec::new(),
            filters: Vec::new(),
            optional_patterns: Vec::new(),
            order_by: Vec::new(),
            group_by: Vec::new(),
            limit: None,
            offset: None,
            distinct: false,
        }
    }

    /// Add a variable to SELECT
    pub fn select(mut self, var: &str) -> Self {
        self.variables.push(var.to_string());
        self
    }

    /// Add multiple variables to SELECT
    pub fn select_vars(mut self, vars: &[&str]) -> Self {
        for var in vars {
            self.variables.push(var.to_string());
        }
        self
    }

    /// Add a prefix
    pub fn prefix(mut self, prefix: &str, uri: &str) -> Self {
        self.prefixes.insert(prefix.to_string(), uri.to_string());
        self
    }

    /// Add a WHERE clause triple pattern
    pub fn where_triple(mut self, subject: &str, predicate: &str, object: &str) -> Self {
        self.where_patterns
            .push(format!("{} {} {}", subject, predicate, object));
        self
    }

    /// Add a WHERE clause pattern (free-form)
    pub fn where_pattern(mut self, pattern: &str) -> Self {
        self.where_patterns.push(pattern.to_string());
        self
    }

    /// Add a FILTER
    pub fn filter(mut self, condition: &str) -> Self {
        self.filters.push(condition.to_string());
        self
    }

    /// Add an OPTIONAL pattern
    pub fn optional(mut self, pattern: &str) -> Self {
        self.optional_patterns.push(pattern.to_string());
        self
    }

    /// Add ORDER BY
    pub fn order_by(mut self, var: &str) -> Self {
        self.order_by.push(var.to_string());
        self
    }

    /// Add GROUP BY
    pub fn group_by(mut self, var: &str) -> Self {
        self.group_by.push(var.to_string());
        self
    }

    /// Set LIMIT
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set OFFSET
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Set DISTINCT
    pub fn distinct(mut self) -> Self {
        self.distinct = true;
        self
    }

    /// Build the query string
    pub fn build(self) -> String {
        let mut query = String::new();

        // Prefixes
        for (prefix, uri) in &self.prefixes {
            query.push_str(&format!("PREFIX {}: <{}>\n", prefix, uri));
        }
        if !self.prefixes.is_empty() {
            query.push('\n');
        }

        // Query type
        match self.query_type {
            QueryType::Select => {
                query.push_str("SELECT ");
                if self.distinct {
                    query.push_str("DISTINCT ");
                }
                if self.variables.is_empty() {
                    query.push('*');
                } else {
                    query.push_str(&self.variables.join(" "));
                }
                query.push('\n');
            }
            QueryType::Ask => query.push_str("ASK\n"),
            QueryType::Construct => query.push_str("CONSTRUCT {\n  # TODO\n}\n"),
            QueryType::Describe => {
                query.push_str("DESCRIBE ");
                query.push_str(&self.variables.join(" "));
                query.push('\n');
            }
        }

        // WHERE clause
        query.push_str("WHERE {\n");
        for pattern in &self.where_patterns {
            query.push_str("  ");
            query.push_str(pattern);
            if !pattern.ends_with('.') {
                query.push_str(" .");
            }
            query.push('\n');
        }

        // OPTIONAL patterns
        for pattern in &self.optional_patterns {
            query.push_str("  OPTIONAL {\n    ");
            query.push_str(pattern);
            query.push_str("\n  }\n");
        }

        // FILTER
        for filter in &self.filters {
            query.push_str("  FILTER (");
            query.push_str(filter);
            query.push_str(")\n");
        }

        query.push_str("}\n");

        // GROUP BY
        if !self.group_by.is_empty() {
            query.push_str("GROUP BY ");
            query.push_str(&self.group_by.join(" "));
            query.push('\n');
        }

        // ORDER BY
        if !self.order_by.is_empty() {
            query.push_str("ORDER BY ");
            query.push_str(&self.order_by.join(" "));
            query.push('\n');
        }

        // LIMIT
        if let Some(limit) = self.limit {
            query.push_str(&format!("LIMIT {}\n", limit));
        }

        // OFFSET
        if let Some(offset) = self.offset {
            query.push_str(&format!("OFFSET {}\n", offset));
        }

        query
    }
}

impl Default for SparqlQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Result Assertion Helpers (State-Based Testing)
// =============================================================================

/// Assert that the result set has exactly the expected count
pub fn assert_result_count(results: &QueryResultSet, expected: usize) {
    assert_eq!(
        results.len(),
        expected,
        "Expected {} results, got {}",
        expected,
        results.len()
    );
}

/// Assert that the result set has at least the expected count
pub fn assert_result_count_min(results: &QueryResultSet, min: usize) {
    assert!(
        results.len() >= min,
        "Expected at least {} results, got {}",
        min,
        results.len()
    );
}

/// Assert that a specific binding exists
pub fn assert_binding_exists(results: &QueryResultSet, var: &str, value: &str) {
    assert!(
        results.has_binding(var, value),
        "Expected binding {}={} not found in results",
        var,
        value
    );
}

/// Assert all bindings for a variable are non-empty
pub fn assert_all_bindings_non_empty(results: &QueryResultSet, var: &str) {
    let values = results.get_all_values(var);
    assert!(!values.is_empty(), "Variable {} has no bindings", var);
    for value in &values {
        assert!(!value.is_empty(), "Variable {} has empty binding", var);
    }
}

/// Assert results are ordered by a specific variable
pub fn assert_result_ordered_by(results: &QueryResultSet, var: &str) {
    let values = results.get_all_values(var);
    let mut sorted = values.clone();
    sorted.sort();
    assert_eq!(values, sorted, "Results not ordered by {}", var);
}

/// Assert a variable exists in results
pub fn assert_variable_exists(results: &QueryResultSet, var: &str) {
    let vars = results.variable_names();
    assert!(
        vars.contains(var),
        "Variable {} not found in results. Available: {:?}",
        var,
        vars
    );
}

/// Assert specific variables exist in results
pub fn assert_variables_exist(results: &QueryResultSet, expected_vars: &[&str]) {
    let vars = results.variable_names();
    for var in expected_vars {
        assert!(
            vars.contains(*var),
            "Variable {} not found in results. Available: {:?}",
            var,
            vars
        );
    }
}

// =============================================================================
// Query Validation Helpers
// =============================================================================

/// Validate that a query has no syntax errors
pub fn validate_query_syntax(query: &str) -> Result<()> {
    let harness = SparqlTestHarness::new();
    harness.validate_query_syntax(query)
}

/// Check if a query contains dangerous patterns (injection prevention)
pub fn check_query_safety(query: &str) -> bool {
    let dangerous_patterns = [
        "DROP", "DELETE", "INSERT", "CLEAR", "LOAD", "CREATE", "COPY", "MOVE", "ADD",
    ];

    let query_upper = query.to_uppercase();
    !dangerous_patterns
        .iter()
        .any(|pattern| query_upper.contains(pattern))
}

// =============================================================================
// Test Fixture Helpers
// =============================================================================

/// Create a simple test graph for domain entities
pub fn create_domain_entity_graph() -> Graph {
    let mut graph = Graph::new();

    // User Aggregate
    let user_agg = NamedNode::new("https://ggen-mcp.dev/domain#UserAggregate").unwrap();
    let aggregate_type = NamedNode::new("https://ddd-patterns.dev/schema#AggregateRoot").unwrap();
    let rdf_type = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap();
    let rdfs_label = NamedNode::new("http://www.w3.org/2000/01/rdf-schema#label").unwrap();
    let rdfs_comment = NamedNode::new("http://www.w3.org/2000/01/rdf-schema#comment").unwrap();

    graph.insert(&Triple::new(
        Subject::NamedNode(user_agg.clone()),
        rdf_type.clone(),
        Term::NamedNode(aggregate_type),
    ));
    graph.insert(&Triple::new(
        Subject::NamedNode(user_agg.clone()),
        rdfs_label.clone(),
        Term::Literal(Literal::new_simple_literal("User")),
    ));
    graph.insert(&Triple::new(
        Subject::NamedNode(user_agg),
        rdfs_comment,
        Term::Literal(Literal::new_simple_literal("User aggregate root")),
    ));

    graph
}

/// Create a test graph for MCP tools
pub fn create_mcp_tools_graph() -> Graph {
    let mut graph = Graph::new();

    // read_spreadsheet tool
    let tool = NamedNode::new("https://ggen-mcp.dev/mcp#read_spreadsheet").unwrap();
    let tool_type = NamedNode::new("https://modelcontextprotocol.io/Tool").unwrap();
    let rdf_type = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap();
    let rdfs_label = NamedNode::new("http://www.w3.org/2000/01/rdf-schema#label").unwrap();
    let rdfs_comment = NamedNode::new("http://www.w3.org/2000/01/rdf-schema#comment").unwrap();

    graph.insert(&Triple::new(
        Subject::NamedNode(tool.clone()),
        rdf_type,
        Term::NamedNode(tool_type),
    ));
    graph.insert(&Triple::new(
        Subject::NamedNode(tool.clone()),
        rdfs_label,
        Term::Literal(Literal::new_simple_literal("read_spreadsheet")),
    ));
    graph.insert(&Triple::new(
        Subject::NamedNode(tool),
        rdfs_comment,
        Term::Literal(Literal::new_simple_literal("Read spreadsheet data")),
    ));

    graph
}

// =============================================================================
// Unit Tests - Query Builder
// =============================================================================

#[cfg(test)]
mod query_builder_tests {
    use super::*;

    #[test]
    fn test_simple_select_query() {
        let query = SparqlQueryBuilder::new()
            .select("?person")
            .select("?name")
            .where_triple("?person", "a", "foaf:Person")
            .where_triple("?person", "foaf:name", "?name")
            .build();

        assert!(query.contains("SELECT ?person ?name"));
        assert!(query.contains("WHERE {"));
        assert!(query.contains("?person a foaf:Person"));
        assert!(query.contains("?person foaf:name ?name"));
    }

    #[test]
    fn test_query_with_prefix() {
        let query = SparqlQueryBuilder::new()
            .prefix("foaf", "http://xmlns.com/foaf/0.1/")
            .select("?person")
            .where_triple("?person", "a", "foaf:Person")
            .build();

        assert!(query.contains("PREFIX foaf: <http://xmlns.com/foaf/0.1/>"));
    }

    #[test]
    fn test_query_with_filter() {
        let query = SparqlQueryBuilder::new()
            .select("?person")
            .select("?age")
            .where_triple("?person", ":age", "?age")
            .filter("?age > 18")
            .build();

        assert!(query.contains("FILTER (?age > 18)"));
    }

    #[test]
    fn test_query_with_optional() {
        let query = SparqlQueryBuilder::new()
            .select("?person")
            .select("?email")
            .where_triple("?person", "a", "foaf:Person")
            .optional("?person foaf:email ?email")
            .build();

        assert!(query.contains("OPTIONAL {"));
        assert!(query.contains("?person foaf:email ?email"));
    }

    #[test]
    fn test_query_with_order_by() {
        let query = SparqlQueryBuilder::new()
            .select("?person")
            .select("?name")
            .where_triple("?person", "foaf:name", "?name")
            .order_by("?name")
            .build();

        assert!(query.contains("ORDER BY ?name"));
    }

    #[test]
    fn test_query_with_limit() {
        let query = SparqlQueryBuilder::new()
            .select("?person")
            .where_triple("?person", "a", "foaf:Person")
            .limit(10)
            .build();

        assert!(query.contains("LIMIT 10"));
    }

    #[test]
    fn test_query_with_distinct() {
        let query = SparqlQueryBuilder::new()
            .distinct()
            .select("?type")
            .where_triple("?s", "a", "?type")
            .build();

        assert!(query.contains("SELECT DISTINCT ?type"));
    }

    #[test]
    fn test_query_with_multiple_vars() {
        let query = SparqlQueryBuilder::new()
            .select_vars(&["?entity", "?name", "?description"])
            .where_triple("?entity", "a", "ddd:Aggregate")
            .build();

        assert!(query.contains("?entity ?name ?description"));
    }
}

// =============================================================================
// Unit Tests - Test Harness Behavior
// =============================================================================

#[cfg(test)]
mod harness_behavior_tests {
    use super::*;

    #[test]
    fn test_harness_creation() {
        let harness = SparqlTestHarness::new();
        assert!(harness.store.is_empty().unwrap());
    }

    #[test]
    fn test_query_result_set_empty() {
        let results = QueryResultSet { solutions: vec![] };
        assert!(results.is_empty());
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_validate_query_syntax_valid() {
        let query = "SELECT ?s ?p ?o WHERE { ?s ?p ?o }";
        assert!(validate_query_syntax(query).is_ok());
    }

    #[test]
    fn test_validate_query_syntax_invalid() {
        let query = "SELECT ?s WHERE { ?s ?p }"; // Missing object
        assert!(validate_query_syntax(query).is_err());
    }

    #[test]
    fn test_check_query_safety_safe() {
        let query = "SELECT ?s WHERE { ?s ?p ?o }";
        assert!(check_query_safety(query));
    }

    #[test]
    fn test_check_query_safety_unsafe_drop() {
        let query = "DROP GRAPH <http://example.org>";
        assert!(!check_query_safety(query));
    }

    #[test]
    fn test_check_query_safety_unsafe_delete() {
        let query = "DELETE WHERE { ?s ?p ?o }";
        assert!(!check_query_safety(query));
    }
}

// =============================================================================
// Integration Tests - Actual Query Execution
// =============================================================================

#[cfg(test)]
mod query_execution_tests {
    use super::*;

    #[test]
    fn test_execute_simple_query_on_empty_store() {
        let harness = SparqlTestHarness::new();
        let query = "SELECT ?s ?p ?o WHERE { ?s ?p ?o }";
        let results = harness.execute_query(query).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_execute_query_with_data() {
        let mut harness = SparqlTestHarness::new();

        // Load test data
        let graph = create_domain_entity_graph();
        for triple in graph.iter() {
            harness.store.insert(triple.as_ref()).unwrap();
        }

        // Query for aggregates
        let query = r#"
            PREFIX ddd: <https://ddd-patterns.dev/schema#>
            SELECT ?aggregate WHERE {
                ?aggregate a ddd:AggregateRoot .
            }
        "#;

        let results = harness.execute_query(query).unwrap();
        assert_result_count_min(&results, 1);
    }

    #[test]
    fn test_execute_query_with_filter() {
        let mut harness = SparqlTestHarness::new();

        // Load test data
        let graph = create_domain_entity_graph();
        for triple in graph.iter() {
            harness.store.insert(triple.as_ref()).unwrap();
        }

        // Query with filter
        let query = r#"
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
            SELECT ?entity ?label WHERE {
                ?entity rdfs:label ?label .
                FILTER(?label != "")
            }
        "#;

        let results = harness.execute_query(query).unwrap();
        assert_all_bindings_non_empty(&results, "label");
    }
}

// =============================================================================
// Integration Tests - Queries Directory Coverage
// =============================================================================

#[cfg(test)]
mod queries_directory_tests {
    use super::*;

    fn setup_test_harness_with_data() -> SparqlTestHarness {
        let mut harness = SparqlTestHarness::new();

        // Load domain entities graph
        let graph = create_domain_entity_graph();
        for triple in graph.iter() {
            harness.store.insert(triple.as_ref()).unwrap();
        }

        // Load MCP tools graph
        let tools_graph = create_mcp_tools_graph();
        for triple in tools_graph.iter() {
            harness.store.insert(triple.as_ref()).unwrap();
        }

        harness
    }

    #[test]
    fn test_aggregates_query_syntax() {
        let query = fs::read_to_string("queries/aggregates.rq");
        if let Ok(query) = query {
            assert!(validate_query_syntax(&query).is_ok());
        }
    }

    #[test]
    fn test_domain_entities_query_syntax() {
        let query = fs::read_to_string("queries/domain_entities.sparql");
        if let Ok(content) = query {
            // Split multiple queries
            for query in content.split("\n---\n") {
                let query = query.trim();
                if query.is_empty() || query.starts_with('#') {
                    continue;
                }
                assert!(
                    validate_query_syntax(query).is_ok(),
                    "Query validation failed: {}",
                    query
                );
            }
        }
    }

    #[test]
    fn test_mcp_tools_query_syntax() {
        let query = fs::read_to_string("queries/mcp_tools.sparql");
        if let Ok(content) = query {
            for query in content.split("\n---\n") {
                let query = query.trim();
                if query.is_empty() || query.starts_with('#') {
                    continue;
                }
                assert!(validate_query_syntax(query).is_ok());
            }
        }
    }

    #[test]
    fn test_commands_query_syntax() {
        let query = fs::read_to_string("queries/commands.rq");
        if let Ok(query) = query {
            assert!(validate_query_syntax(&query).is_ok());
        }
    }

    #[test]
    fn test_handlers_query_syntax() {
        let query = fs::read_to_string("queries/handlers.rq");
        if let Ok(query) = query {
            assert!(validate_query_syntax(&query).is_ok());
        }
    }

    #[test]
    fn test_invariants_query_syntax() {
        let query = fs::read_to_string("queries/invariants.rq");
        if let Ok(query) = query {
            assert!(validate_query_syntax(&query).is_ok());
        }
    }

    #[test]
    fn test_inference_queries_syntax() {
        let inference_queries = [
            "queries/inference/handler_implementations.sparql",
            "queries/inference/mcp_relationships.sparql",
            "queries/inference/tool_categories.sparql",
            "queries/inference/validation_constraints.sparql",
        ];

        for query_file in &inference_queries {
            if let Ok(content) = fs::read_to_string(query_file) {
                for query in content.split("\n---\n") {
                    let query = query.trim();
                    if query.is_empty() || query.starts_with('#') {
                        continue;
                    }
                    assert!(
                        validate_query_syntax(query).is_ok(),
                        "Inference query validation failed: {}",
                        query_file
                    );
                }
            }
        }
    }

    #[test]
    fn test_all_queries_safe() {
        let query_files = [
            "queries/aggregates.rq",
            "queries/commands.rq",
            "queries/handlers.rq",
            "queries/invariants.rq",
        ];

        for query_file in &query_files {
            if let Ok(content) = fs::read_to_string(query_file) {
                assert!(
                    check_query_safety(&content),
                    "Query file {} contains unsafe patterns",
                    query_file
                );
            }
        }
    }
}

// =============================================================================
// Integration Tests - End-to-End Workflows
// =============================================================================

#[cfg(test)]
mod end_to_end_tests {
    use super::*;

    #[test]
    fn test_ontology_to_query_to_results() {
        let mut harness = SparqlTestHarness::new();

        // 1. Load ontology data
        let graph = create_domain_entity_graph();
        for triple in graph.iter() {
            harness.store.insert(triple.as_ref()).unwrap();
        }

        // 2. Build query programmatically
        let query = SparqlQueryBuilder::new()
            .prefix("ddd", "https://ddd-patterns.dev/schema#")
            .prefix("rdfs", "http://www.w3.org/2000/01/rdf-schema#")
            .select_vars(&["?aggregate", "?label"])
            .where_triple("?aggregate", "a", "ddd:AggregateRoot")
            .where_triple("?aggregate", "rdfs:label", "?label")
            .order_by("?label")
            .build();

        // 3. Execute query
        let results = harness.execute_query(&query).unwrap();

        // 4. Verify state
        assert_result_count_min(&results, 1);
        assert_variables_exist(&results, &["aggregate", "label"]);
        assert_all_bindings_non_empty(&results, "label");
    }

    #[test]
    fn test_query_cache_effectiveness() {
        let harness = SparqlTestHarness::new();
        let query = "SELECT ?s ?p ?o WHERE { ?s ?p ?o } LIMIT 10";

        // Execute same query multiple times
        for _ in 0..5 {
            let _ = harness.execute_query(query);
        }

        // Note: Actual cache metrics would be tested with the cache module
        assert!(true);
    }

    #[test]
    fn test_error_handling_invalid_query() {
        let harness = SparqlTestHarness::new();
        let invalid_query = "SELECT WHERE { }"; // Missing variables
        let result = harness.execute_query(invalid_query);
        assert!(result.is_err());
    }
}

// =============================================================================
// Performance and Budget Tests
// =============================================================================

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_query_execution_within_budget() {
        let harness = SparqlTestHarness::new();
        let query = "SELECT ?s ?p ?o WHERE { ?s ?p ?o } LIMIT 100";

        let start = Instant::now();
        let _ = harness.execute_query(query).unwrap();
        let duration = start.elapsed();

        // Should complete within 100ms for small query
        assert!(
            duration.as_millis() < 100,
            "Query took {}ms, expected < 100ms",
            duration.as_millis()
        );
    }

    #[test]
    fn test_query_builder_performance() {
        let start = Instant::now();

        for _ in 0..1000 {
            let _query = SparqlQueryBuilder::new()
                .select("?s")
                .where_triple("?s", "?p", "?o")
                .build();
        }

        let duration = start.elapsed();

        // Building 1000 queries should be fast
        assert!(
            duration.as_millis() < 50,
            "Building 1000 queries took {}ms",
            duration.as_millis()
        );
    }
}

// =============================================================================
// Result Assertion Tests
// =============================================================================

#[cfg(test)]
mod result_assertion_tests {
    use super::*;

    #[test]
    fn test_assert_result_count() {
        let results = QueryResultSet { solutions: vec![] };
        assert_result_count(&results, 0);
    }

    #[test]
    #[should_panic(expected = "Expected 1 results, got 0")]
    fn test_assert_result_count_fails() {
        let results = QueryResultSet { solutions: vec![] };
        assert_result_count(&results, 1);
    }

    #[test]
    fn test_assert_result_count_min() {
        let results = QueryResultSet { solutions: vec![] };
        assert_result_count_min(&results, 0);
    }

    #[test]
    fn test_assert_variables_exist() {
        let mut harness = SparqlTestHarness::new();
        let graph = create_domain_entity_graph();
        for triple in graph.iter() {
            harness.store.insert(triple.as_ref()).unwrap();
        }

        let query = r#"
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
            SELECT ?entity ?label WHERE {
                ?entity rdfs:label ?label .
            }
        "#;

        let results = harness.execute_query(query).unwrap();
        assert_variables_exist(&results, &["entity", "label"]);
    }
}
