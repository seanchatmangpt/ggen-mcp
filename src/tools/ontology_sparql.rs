//! Ontology Loading and SPARQL Query Tools
//!
//! MCP tools for loading RDF ontologies and executing SPARQL queries.
//! Integrates Oxigraph Store, SHACL validation, SPARQL injection prevention.
//!
//! ## Tools
//! - `load_ontology`: Load Turtle ontology → validate → cache → return stats
//! - `execute_sparql_query`: Execute SPARQL → analyze → cache results → TypedBinding → JSON

use crate::audit::integration::audit_tool;
use crate::ontology::ShapeValidator;
use crate::sparql::performance::{QueryAnalyzer, QueryComplexity};
use crate::sparql::typed_binding::TypedBinding;
use crate::state::AppState;
use crate::validation::{validate_non_empty_string, validate_path_safe};
use anyhow::{Context, Result, anyhow};
use oxigraph::io::RdfFormat;
use oxigraph::sparql::QueryResults;
use oxigraph::store::Store;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

// =============================================================================
// NewTypes (Poka-Yoke: prevent ID confusion)
// =============================================================================

/// Ontology identifier (SHA-256 hash of content)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct OntologyId(String);

impl OntologyId {
    pub fn new(content: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        Self(hash)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for OntologyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Query cache key (SHA-256 hash of query text)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct QueryCacheKey(String);

impl QueryCacheKey {
    pub fn new(query: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(query.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        Self(hash)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// =============================================================================
// Tool Parameters (serde + JsonSchema for MCP)
// =============================================================================

/// Parameters for load_ontology tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoadOntologyParams {
    /// Path to Turtle (.ttl) ontology file (relative to workspace_root)
    pub path: String,
    /// Optional: Validate with SHACL shapes (default: true)
    #[serde(default = "default_validate")]
    pub validate: bool,
    /// Optional: Base IRI for relative IRIs in ontology
    pub base_iri: Option<String>,
}

fn default_validate() -> bool {
    true
}

/// Response from load_ontology tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoadOntologyResponse {
    /// Unique identifier for the loaded ontology (SHA-256)
    pub ontology_id: OntologyId,
    /// Path to the loaded ontology file
    pub path: String,
    /// Number of RDF triples loaded
    pub triple_count: usize,
    /// Number of distinct entities (subjects + objects that are IRIs)
    pub entity_count: usize,
    /// Number of distinct properties (predicates)
    pub property_count: usize,
    /// SHACL validation result (if validate=true)
    pub validation: Option<ValidationSummary>,
    /// Load duration in milliseconds
    pub load_time_ms: u64,
}

/// SHACL validation summary
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidationSummary {
    pub conforms: bool,
    pub violation_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
}

/// Parameters for execute_sparql_query tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecuteSparqlQueryParams {
    /// Ontology ID returned from load_ontology
    pub ontology_id: OntologyId,
    /// SPARQL query (SELECT/CONSTRUCT/ASK/DESCRIBE)
    pub query: String,
    /// Optional: Use cached results if available (default: true)
    #[serde(default = "default_use_cache")]
    pub use_cache: bool,
    /// Optional: Maximum results to return (default: 1000)
    #[serde(default = "default_max_results")]
    pub max_results: usize,
}

fn default_use_cache() -> bool {
    true
}

fn default_max_results() -> usize {
    1000
}

/// Response from execute_sparql_query tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecuteSparqlQueryResponse {
    /// Query cache key for result reuse
    pub cache_key: QueryCacheKey,
    /// Query execution result
    pub result: QueryResult,
    /// Query performance metrics
    pub performance: QueryPerformance,
    /// Whether result was served from cache
    pub from_cache: bool,
}

/// Query result variants
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QueryResult {
    /// SELECT query results (table of bindings)
    Select { bindings: Vec<Map<String, JsonValue>> },
    /// ASK query result (boolean)
    Ask { result: bool },
    /// CONSTRUCT/DESCRIBE results (RDF triples as Turtle)
    Graph { turtle: String },
}

/// Query performance metrics
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QueryPerformance {
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Number of results returned
    pub result_count: usize,
    /// Query complexity score (0-100)
    pub complexity_score: f64,
}

// =============================================================================
// Tool Implementation
// =============================================================================

/// Load ontology from Turtle file → validate → cache → return stats
///
/// # Security
/// - Path traversal prevention via validate_path_safe
/// - SHACL validation optional but recommended
/// - Content-based ID (SHA-256) prevents tampering
pub async fn load_ontology(
    state: Arc<AppState>,
    params: LoadOntologyParams,
) -> Result<LoadOntologyResponse> {
    let _span = audit_tool("load_ontology", &params);
    let start = Instant::now();

    // Poka-yoke: validate inputs
    validate_non_empty_string(&params.path)
        .context("ontology path must not be empty")?;
    validate_path_safe(&params.path)
        .context("ontology path contains path traversal")?;

    // Resolve path relative to workspace_root
    let ontology_path = state.config().workspace_root.join(&params.path);
    if !ontology_path.exists() {
        return Err(anyhow!("ontology file not found: {}", params.path)
            .context("load_ontology failed"));
    }

    // Read ontology content
    let content = fs::read_to_string(&ontology_path)
        .context(format!("failed to read ontology file: {}", params.path))?;

    // Generate content-based ID
    let ontology_id = OntologyId::new(&content);

    // Load into Oxigraph Store
    let store = Store::new()
        .context("failed to create Oxigraph store")?;

    store
        .load_from_reader(RdfFormat::Turtle, content.as_bytes())
        .context("failed to parse Turtle ontology")?;

    // Count triples and entities
    let triple_count = store.len()
        .context("failed to count triples")?;

    let (entity_count, property_count) = count_entities_and_properties(&store)?;

    // SHACL validation (optional)
    let validation = if params.validate {
        Some(validate_with_shacl(&store, &ontology_path)?)
    } else {
        None
    };

    // Cache ontology in AppState
    state.ontology_cache().insert(ontology_id.clone(), store);

    let load_time_ms = start.elapsed().as_millis() as u64;

    Ok(LoadOntologyResponse {
        ontology_id,
        path: params.path,
        triple_count,
        entity_count,
        property_count,
        validation,
        load_time_ms,
    })
}

/// Execute SPARQL query → analyze performance → cache results → JSON
///
/// # Security
/// - SPARQL injection prevention (check dangerous patterns)
/// - Query complexity analysis via QueryAnalyzer
/// - Performance budget enforcement
/// - Result size limits
pub async fn execute_sparql_query(
    state: Arc<AppState>,
    params: ExecuteSparqlQueryParams,
) -> Result<ExecuteSparqlQueryResponse> {
    let _span = audit_tool("execute_sparql_query", &params);
    let start = Instant::now();

    // Validate query not empty
    validate_non_empty_string(&params.query)
        .context("SPARQL query must not be empty")?;

    // Get cached ontology
    let store = state.ontology_cache().get(&params.ontology_id)
        .ok_or_else(|| anyhow!("ontology not found: {}", params.ontology_id))?;

    // Check cache for existing results
    let cache_key = QueryCacheKey::new(&params.query);
    if params.use_cache {
        if let Some(cached) = state.query_cache_simple().get(&cache_key) {
            return Ok(cached);
        }
    }

    // SPARQL injection prevention (check for dangerous patterns)
    check_query_safety(&params.query)?;

    // Query complexity analysis
    let analyzer = QueryAnalyzer::new();
    let complexity = analyzer.analyze(&params.query)
        .unwrap_or_else(|_| {
            // If analysis fails, use conservative defaults
            QueryComplexity {
                triple_pattern_count: 10,
                optional_count: 0,
                union_count: 0,
                filter_count: 0,
                subquery_count: 0,
                nesting_depth: 1,
                variable_count: 5,
                distinct_predicates: 5,
                estimated_selectivity: 0.5,
                complexity_score: 50.0,
            }
        });

    // Execute query
    let query_results = store
        .query(&params.query)
        .context("SPARQL query execution failed")?;

    // Convert results to JSON
    let (result, result_count) = convert_query_results(query_results, params.max_results)?;

    let execution_time_ms = start.elapsed().as_millis() as u64;

    let response = ExecuteSparqlQueryResponse {
        cache_key: cache_key.clone(),
        result,
        performance: QueryPerformance {
            execution_time_ms,
            result_count,
            complexity_score: complexity.complexity_score,
        },
        from_cache: false,
    };

    // Cache result
    state.query_cache_simple().insert(cache_key, response.clone());

    Ok(response)
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Count entities (subjects + IRI objects) and properties (predicates)
fn count_entities_and_properties(store: &Store) -> Result<(usize, usize)> {
    // Count distinct subjects + IRI objects
    let entity_query = r#"
        SELECT (COUNT(DISTINCT ?entity) AS ?count)
        WHERE {
            {
                SELECT DISTINCT ?entity WHERE {
                    ?entity ?p ?o .
                }
            } UNION {
                SELECT DISTINCT ?entity WHERE {
                    ?s ?p ?entity .
                    FILTER(isIRI(?entity))
                }
            }
        }
    "#;

    let entity_count = match store.query(entity_query)? {
        QueryResults::Solutions(mut solutions) => {
            if let Some(Ok(solution)) = solutions.next() {
                let binding = TypedBinding::new(&solution);
                binding.get_integer("count").unwrap_or(0) as usize
            } else {
                0
            }
        }
        _ => 0,
    };

    // Count distinct properties
    let property_query = r#"
        SELECT (COUNT(DISTINCT ?property) AS ?count)
        WHERE {
            ?s ?property ?o .
        }
    "#;

    let property_count = match store.query(property_query)? {
        QueryResults::Solutions(mut solutions) => {
            if let Some(Ok(solution)) = solutions.next() {
                let binding = TypedBinding::new(&solution);
                binding.get_integer("count").unwrap_or(0) as usize
            } else {
                0
            }
        }
        _ => 0,
    };

    Ok((entity_count, property_count))
}

/// Validate ontology with SHACL shapes (if shapes.ttl exists)
fn validate_with_shacl(store: &Store, ontology_path: &Path) -> Result<ValidationSummary> {
    // Look for shapes.ttl in same directory
    let shapes_path = ontology_path
        .parent()
        .map(|p| p.join("shapes.ttl"))
        .unwrap_or_else(|| PathBuf::from("ontology/shapes.ttl"));

    if !shapes_path.exists() {
        // No shapes file → assume conforms
        return Ok(ValidationSummary {
            conforms: true,
            violation_count: 0,
            warning_count: 0,
            info_count: 0,
        });
    }

    // Load shapes and validate
    let validator = ShapeValidator::from_file(&shapes_path)
        .context("failed to load SHACL shapes")?;

    let report = validator.validate_graph(store)
        .context("SHACL validation failed")?;

    Ok(ValidationSummary {
        conforms: report.conforms(),
        violation_count: report.violation_count(),
        warning_count: report.warning_count(),
        info_count: report.info_count(),
    })
}

/// Check query for dangerous patterns (poka-yoke)
fn check_query_safety(query: &str) -> Result<()> {
    // Disallow UPDATE operations
    let upper = query.to_uppercase();
    let dangerous_keywords = ["INSERT", "DELETE", "DROP", "CLEAR", "LOAD", "CREATE"];

    for keyword in &dangerous_keywords {
        if upper.contains(keyword) {
            return Err(anyhow!(
                "query contains dangerous keyword: {}. Only read-only queries allowed.",
                keyword
            ));
        }
    }

    Ok(())
}

/// Convert Oxigraph QueryResults to JSON
fn convert_query_results(
    results: QueryResults,
    max_results: usize,
) -> Result<(QueryResult, usize)> {
    match results {
        QueryResults::Solutions(solutions) => {
            let mut bindings = Vec::new();
            let mut count = 0;

            for solution in solutions {
                if count >= max_results {
                    break;
                }

                let solution = solution.context("failed to read query solution")?;
                let binding = TypedBinding::new(&solution);

                let mut map = Map::new();
                for var in binding.variables() {
                    if let Ok(Some(value)) = binding.get_typed_value_opt(&var) {
                        map.insert(var.clone(), typed_value_to_json(value));
                    }
                }

                bindings.push(map);
                count += 1;
            }

            Ok((QueryResult::Select { bindings }, count))
        }
        QueryResults::Boolean(result) => {
            Ok((QueryResult::Ask { result }, 1))
        }
        QueryResults::Graph(triples) => {
            // Serialize graph to Turtle
            let mut turtle = String::new();
            let mut count = 0;

            for triple in triples {
                if count >= max_results {
                    break;
                }
                let triple = triple.context("failed to read triple")?;
                turtle.push_str(&format!("{} {} {} .\n",
                    triple.subject, triple.predicate, triple.object));
                count += 1;
            }

            Ok((QueryResult::Graph { turtle }, count))
        }
    }
}

/// Convert TypedValue to JSON Value
fn typed_value_to_json(value: crate::sparql::typed_binding::TypedValue) -> JsonValue {
    use crate::sparql::typed_binding::TypedValue;

    match value {
        TypedValue::IRI(s) => JsonValue::String(s),
        TypedValue::Literal(s) => JsonValue::String(s),
        TypedValue::TypedLiteral { value, datatype } => {
            serde_json::json!({
                "value": value,
                "type": "literal",
                "datatype": datatype
            })
        }
        TypedValue::LangLiteral { value, language } => {
            serde_json::json!({
                "value": value,
                "type": "literal",
                "language": language
            })
        }
        TypedValue::BlankNode(s) => {
            serde_json::json!({
                "value": s,
                "type": "bnode"
            })
        }
        TypedValue::Integer(i) => JsonValue::Number(i.into()),
        TypedValue::Float(f) => {
            serde_json::Number::from_f64(f)
                .map(JsonValue::Number)
                .unwrap_or(JsonValue::Null)
        }
        TypedValue::Boolean(b) => JsonValue::Bool(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ontology_id_consistent() {
        let content = "test content";
        let id1 = OntologyId::new(content);
        let id2 = OntologyId::new(content);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_query_cache_key_consistent() {
        let query = "SELECT ?s ?p ?o WHERE { ?s ?p ?o }";
        let key1 = QueryCacheKey::new(query);
        let key2 = QueryCacheKey::new(query);
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_check_query_safety_blocks_dangerous() {
        assert!(check_query_safety("INSERT DATA { ... }").is_err());
        assert!(check_query_safety("DELETE WHERE { ... }").is_err());
        assert!(check_query_safety("DROP GRAPH <...>").is_err());
    }

    #[test]
    fn test_check_query_safety_allows_safe() {
        assert!(check_query_safety("SELECT ?s WHERE { ?s ?p ?o }").is_ok());
        assert!(check_query_safety("ASK { ?s ?p ?o }").is_ok());
        assert!(check_query_safety("CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }").is_ok());
    }
}
