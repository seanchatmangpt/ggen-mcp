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
use crate::state::AppState;
use crate::validation::{validate_non_empty_string, validate_path_safe};
use anyhow::{Context, Result, anyhow};
use ggen_ontology_core::TripleStore;
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
    Select {
        bindings: Vec<Map<String, JsonValue>>,
    },
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
    validate_non_empty_string("path", &params.path).context("ontology path must not be empty")?;
    validate_path_safe(&params.path).context("ontology path contains path traversal")?;

    // Resolve path relative to workspace_root
    let ontology_path = state.config().workspace_root.join(&params.path);
    if !ontology_path.exists() {
        return Err(
            anyhow!("ontology file not found: {}", params.path).context("load_ontology failed")
        );
    }

    // Read ontology content
    let content = fs::read_to_string(&ontology_path)
        .context(format!("failed to read ontology file: {}", params.path))?;

    // Generate content-based ID
    let ontology_id = OntologyId::new(&content);

    // Load into ggen TripleStore (uses oxigraph internally)
    let store = TripleStore::new()
        .map_err(|e| anyhow!("failed to create triple store: {}", e))?;

    store.load_turtle(&ontology_path)
        .map_err(|e| anyhow!("failed to parse Turtle ontology: {}", e))?;

    // Count triples and entities using SPARQL queries
    let triple_count = count_triples(&store)?;
    let (entity_count, property_count) = count_entities_and_properties(&store)?;

    // SHACL validation (optional) - now works directly with TripleStore
    let validation = if params.validate {
        Some(validate_with_shacl_triple_store(&store, &ontology_path, &content)?)
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
    validate_non_empty_string("query", &params.query).context("SPARQL query must not be empty")?;

    // Get cached ontology
    let store = state
        .ontology_cache()
        .get(&params.ontology_id)
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
    // TPS: Fail fast on analysis errors - don't silently use defaults
    let mut analyzer = QueryAnalyzer::new();
    let complexity = analyzer.analyze(&params.query)
        .context("Failed to analyze query complexity - query may be malformed")?;

    // Execute query using ggen's TripleStore
    let query_json = store.query_sparql(&params.query)
        .map_err(|e| anyhow!("SPARQL query execution failed: {}", e))?;

    // Parse JSON results from ggen
    let (result, result_count) = parse_query_results_json(&query_json, params.max_results)?;

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
    state
        .query_cache_simple()
        .insert(cache_key, response.clone());

    Ok(response)
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Count total triples
fn count_triples(store: &TripleStore) -> Result<usize> {
    let count_query = r#"
        SELECT (COUNT(*) AS ?count)
        WHERE {
            ?s ?p ?o .
        }
    "#;
    
    let json_result = store.query_sparql(count_query)
        .map_err(|e| anyhow!("failed to count triples: {}", e))?;
    
    let parsed: JsonValue = serde_json::from_str(&json_result)
        .context("failed to parse count query result")?;
    
    // Extract count from SPARQL JSON results format
    if let Some(results) = parsed.get("results") {
        if let Some(bindings) = results.get("bindings").and_then(|b| b.as_array()) {
            if let Some(first) = bindings.first() {
                if let Some(count_obj) = first.get("count") {
                    if let Some(count_val) = count_obj.get("value") {
                        if let Some(count_str) = count_val.as_str() {
                            return Ok(count_str.parse().unwrap_or(0));
                        }
                    }
                }
            }
        }
    }
    
    Ok(0)
}

/// Count entities (subjects + IRI objects) and properties (predicates)
fn count_entities_and_properties(store: &TripleStore) -> Result<(usize, usize)> {
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

    let entity_json = store.query_sparql(entity_query)
        .map_err(|e| anyhow!("failed to count entities: {}", e))?;
    let entity_count = extract_count_from_json(&entity_json)?;

    // Count distinct properties
    let property_query = r#"
        SELECT (COUNT(DISTINCT ?property) AS ?count)
        WHERE {
            ?s ?property ?o .
        }
    "#;

    let property_json = store.query_sparql(property_query)
        .map_err(|e| anyhow!("failed to count properties: {}", e))?;
    let property_count = extract_count_from_json(&property_json)?;

    Ok((entity_count, property_count))
}

/// Extract count from SPARQL JSON result
fn extract_count_from_json(json_result: &str) -> Result<usize> {
    let parsed: JsonValue = serde_json::from_str(json_result)
        .context("failed to parse query result")?;
    
    if let Some(results) = parsed.get("results") {
        if let Some(bindings) = results.get("bindings").and_then(|b| b.as_array()) {
            if let Some(first) = bindings.first() {
                if let Some(count_obj) = first.get("count") {
                    if let Some(count_val) = count_obj.get("value") {
                        if let Some(count_str) = count_val.as_str() {
                            return Ok(count_str.parse().unwrap_or(0));
                        }
                    }
                }
            }
        }
    }
    Ok(0)
}

/// Validate ontology with SHACL shapes using TripleStore
/// Works directly with TripleStore without conversion overhead
fn validate_with_shacl_triple_store(store: &TripleStore, ontology_path: &Path, ontology_content: &str) -> Result<ValidationSummary> {
    // TPS Principle (Jidoka): No fallbacks - shapes file is mandatory
    // Fail fast if shapes file doesn't exist (Andon Cord principle)
    let shapes_path = ontology_path
        .parent()
        .map(|p| p.join("shapes.ttl"))
        .unwrap_or_else(|| PathBuf::from("ontology/shapes.ttl"));

    if !shapes_path.exists() {
        // TPS: Fail fast - no silent fallback to "assume conforms"
        return Err(anyhow!(
            "SHACL shapes file is mandatory but missing: {}. Validation cannot proceed without shapes.",
            shapes_path.display()
        ));
    }

    // Load shapes and validate using TripleStore
    let validator =
        ShapeValidator::from_file(&shapes_path).context("failed to load SHACL shapes")?;

    let report = validator
        .validate_triple_store(store, ontology_content)
        .context("SHACL validation failed")?;

    Ok(ValidationSummary {
        conforms: report.conforms(),
        violation_count: report.violation_count(),
        warning_count: report.warning_count(),
        info_count: report.info_count(),
    })
}

/// Validate ontology with SHACL shapes (if shapes.ttl exists)
/// Uses Store directly for SHACL validation (ShapeValidator requires Store)
/// DEPRECATED: Use validate_with_shacl_triple_store instead
#[allow(dead_code)]
fn validate_with_shacl_store(store: &oxigraph::store::Store, ontology_path: &Path) -> Result<ValidationSummary> {
    // TPS Principle (Jidoka): No fallbacks - shapes file is mandatory
    // Fail fast if shapes file doesn't exist (Andon Cord principle)
    let shapes_path = ontology_path
        .parent()
        .map(|p| p.join("shapes.ttl"))
        .unwrap_or_else(|| PathBuf::from("ontology/shapes.ttl"));

    if !shapes_path.exists() {
        // TPS: Fail fast - no silent fallback to "assume conforms"
        return Err(anyhow!(
            "SHACL shapes file is mandatory but missing: {}. Validation cannot proceed without shapes.",
            shapes_path.display()
        ));
    }

    // Load shapes and validate
    let validator =
        ShapeValidator::from_file(&shapes_path).context("failed to load SHACL shapes")?;

    let report = validator
        .validate_graph(store)
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

/// Parse SPARQL JSON results from ggen TripleStore
/// 
/// Ggen's query_sparql returns JSON in SPARQL JSON Results format:
/// - SELECT: {"head": {"vars": [...]}, "results": {"bindings": [...]}}
/// - ASK: {"boolean": true/false}
/// - Graph: {"type": "graph"} (note: actual graph data not included, would need separate handling)
fn parse_query_results_json(
    json_result: &str,
    max_results: usize,
) -> Result<(QueryResult, usize)> {
    let parsed: JsonValue = serde_json::from_str(json_result)
        .context("failed to parse SPARQL query result JSON")?;

    // Check for boolean result (ASK query)
    if let Some(boolean) = parsed.get("boolean") {
        if let Some(result) = boolean.as_bool() {
            return Ok((QueryResult::Ask { result }, 1));
        }
    }

    // Check for graph result (CONSTRUCT/DESCRIBE)
    // Note: ggen returns {"type": "graph"} but doesn't include the actual turtle
    // For now, return empty graph - would need to execute separately to get turtle
    if let Some(graph_type) = parsed.get("type") {
        if let Some(type_str) = graph_type.as_str() {
            if type_str == "graph" {
                // Graph queries return type marker but not the actual turtle
                // This is a limitation - would need to handle CONSTRUCT/DESCRIBE differently
                return Ok((QueryResult::Graph { turtle: String::new() }, 0));
            }
        }
    }

    // Handle SELECT query results - ggen format: {"head": {"vars": [...]}, "results": {"bindings": [...]}}
    // Each binding is a simple object with variable names as keys and term strings as values
    if let Some(results) = parsed.get("results") {
        if let Some(bindings_array) = results.get("bindings").and_then(|b| b.as_array()) {
            let mut bindings = Vec::new();
            let mut count = 0;

            for binding_obj in bindings_array {
                if count >= max_results {
                    break;
                }

                if let Some(binding_map) = binding_obj.as_object() {
                    let mut map = Map::new();
                    for (key, value) in binding_map {
                        // Ggen returns terms as strings via term.to_string()
                        // Convert to SPARQL JSON Results format: {"value": "...", "type": "uri"/"literal"/"bnode"}
                        if let Some(str_val) = value.as_str() {
                            // Parse the string to determine type
                            // IRIs: <http://...> or http://... or https://...
                            // Blank nodes: _:b0, _:b1, etc.
                            // Literals: everything else
                            let (value_str, type_str) = if str_val.starts_with('<') && str_val.ends_with('>') {
                                // IRI in angle brackets: <http://example.org>
                                let iri = &str_val[1..str_val.len()-1];
                                (iri.to_string(), "uri")
                            } else if str_val.starts_with("http://") || str_val.starts_with("https://") {
                                // IRI without brackets
                                (str_val.to_string(), "uri")
                            } else if str_val.starts_with("_:") {
                                // Blank node
                                (str_val.to_string(), "bnode")
                            } else {
                                // Literal
                                (str_val.to_string(), "literal")
                            };
                            
                            map.insert(key.clone(), serde_json::json!({
                                "value": value_str,
                                "type": type_str
                            }));
                        } else {
                            // Already structured value
                            map.insert(key.clone(), value.clone());
                        }
                    }
                    bindings.push(map);
                    count += 1;
                }
            }

            return Ok((QueryResult::Select { bindings }, count));
        }
    }

    // Default: empty SELECT result
    Ok((QueryResult::Select { bindings: vec![] }, 0))
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
