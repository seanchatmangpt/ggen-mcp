//! Ontology Consistency Checking and Validation
//!
//! This module provides comprehensive validation and consistency checking for RDF ontologies,
//! specifically designed for DDD-based MCP code generation.
//!
//! # Components
//!
//! - [`ConsistencyChecker`]: Validates RDF graph consistency (cycles, domains, ranges, cardinality)
//! - [`SchemaValidator`]: Validates against expected schema patterns (namespaces, DDD structure)
//! - [`NamespaceManager`]: Safe namespace handling with collision detection
//! - [`OntologyMerger`]: Safe ontology merging with conflict detection
//! - [`HashVerifier`]: Verifies ontology integrity using cryptographic hashes

use anyhow::{Context, Result, anyhow};
use ggen_ontology_core::TripleStore;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fmt;

// =============================================================================
// Helper Functions for JSON Parsing
// =============================================================================

/// Extract IRI from JSON value (ggen format: string or {"value": "...", "type": "uri"})
fn extract_iri_from_json_value(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::String(s) => {
            // Check if it's an IRI (starts with < or http)
            if s.starts_with('<') && s.ends_with('>') {
                Some(s[1..s.len()-1].to_string())
            } else if s.starts_with("http://") || s.starts_with("https://") {
                Some(s.clone())
            } else {
                None
            }
        }
        JsonValue::Object(obj) => {
            if let Some(type_val) = obj.get("type") {
                if type_val.as_str() == Some("uri") {
                    obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Extract literal from JSON value (ggen format: string or {"value": "...", "type": "literal"})
fn extract_literal_from_json_value(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::String(s) => {
            // If it's not an IRI, treat as literal
            if !s.starts_with('<') && !s.starts_with("http://") && !s.starts_with("https://") {
                Some(s.clone())
            } else {
                None
            }
        }
        JsonValue::Object(obj) => {
            if let Some(type_val) = obj.get("type") {
                if type_val.as_str() == Some("literal") {
                    obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string())
                } else {
                    None
                }
            } else {
                // Default to value if no type specified
                obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string())
            }
        }
        _ => None,
    }
}

/// Extract term as string from JSON value (for to_string() compatibility)
fn extract_term_string_from_json_value(value: &JsonValue) -> String {
    match value {
        JsonValue::String(s) => s.clone(),
        JsonValue::Object(obj) => {
            if let Some(value_val) = obj.get("value") {
                if let Some(str_val) = value_val.as_str() {
                    str_val.to_string()
                } else {
                    value.to_string()
                }
            } else {
                value.to_string()
            }
        }
        _ => value.to_string(),
    }
}

// =============================================================================
// Error Types
// =============================================================================

/// Validation error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Cyclic class hierarchy detected
    CyclicHierarchy { cycle: Vec<String> },
    /// Property domain/range violation
    InvalidDomainRange {
        property: String,
        subject: String,
        object: String,
        message: String,
    },
    /// Cardinality constraint violation
    CardinalityViolation {
        node: String,
        property: String,
        expected: String,
        actual: usize,
    },
    /// Contradiction detected
    Contradiction {
        statement1: String,
        statement2: String,
        reason: String,
    },
    /// Required property missing
    MissingProperty { node: String, property: String },
    /// Required namespace missing
    MissingNamespace {
        prefix: String,
        expected_uri: String,
    },
    /// Invalid DDD structure
    InvalidDddStructure { aggregate: String, reason: String },
    /// Property without type
    UntypedProperty { property: String },
    /// Invalid invariant definition
    InvalidInvariant { node: String, reason: String },
    /// Orphaned node (no connections)
    OrphanedNode { node: String },
    /// Namespace collision
    NamespaceCollision {
        prefix: String,
        uri1: String,
        uri2: String,
    },
    /// Merge conflict
    MergeConflict { resource: String, reason: String },
    /// Hash mismatch (tampering detected)
    HashMismatch { expected: String, actual: String },
    /// Custom validation error
    Custom { message: String },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::CyclicHierarchy { cycle } => {
                write!(f, "Cyclic class hierarchy detected: {}", cycle.join(" -> "))
            }
            ValidationError::InvalidDomainRange {
                property,
                subject,
                object,
                message,
            } => {
                write!(
                    f,
                    "Invalid domain/range for property {}: {} -> {} ({})",
                    property, subject, object, message
                )
            }
            ValidationError::CardinalityViolation {
                node,
                property,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Cardinality violation at {}: property {} expected {}, found {}",
                    node, property, expected, actual
                )
            }
            ValidationError::Contradiction {
                statement1,
                statement2,
                reason,
            } => {
                write!(
                    f,
                    "Contradiction detected: '{}' conflicts with '{}' ({})",
                    statement1, statement2, reason
                )
            }
            ValidationError::MissingProperty { node, property } => {
                write!(f, "Required property {} missing at node {}", property, node)
            }
            ValidationError::MissingNamespace {
                prefix,
                expected_uri,
            } => {
                write!(
                    f,
                    "Required namespace missing: prefix '{}' (expected: {})",
                    prefix, expected_uri
                )
            }
            ValidationError::InvalidDddStructure { aggregate, reason } => {
                write!(f, "Invalid DDD structure for {}: {}", aggregate, reason)
            }
            ValidationError::UntypedProperty { property } => {
                write!(f, "Property {} has no type declaration", property)
            }
            ValidationError::InvalidInvariant { node, reason } => {
                write!(f, "Invalid invariant at {}: {}", node, reason)
            }
            ValidationError::OrphanedNode { node } => {
                write!(f, "Orphaned node (no connections): {}", node)
            }
            ValidationError::NamespaceCollision { prefix, uri1, uri2 } => {
                write!(
                    f,
                    "Namespace collision for prefix '{}': {} vs {}",
                    prefix, uri1, uri2
                )
            }
            ValidationError::MergeConflict { resource, reason } => {
                write!(f, "Merge conflict for {}: {}", resource, reason)
            }
            ValidationError::HashMismatch { expected, actual } => {
                write!(
                    f,
                    "Hash mismatch (possible tampering): expected {}, got {}",
                    expected, actual
                )
            }
            ValidationError::Custom { message } => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Result type for validation operations
pub type ValidationResult<T> = Result<T, ValidationError>;

// =============================================================================
// Consistency Report
// =============================================================================

/// Report from consistency checking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyReport {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub stats: ConsistencyStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyStats {
    pub total_triples: usize,
    pub total_classes: usize,
    pub total_properties: usize,
    pub total_individuals: usize,
    pub max_hierarchy_depth: usize,
}

impl ConsistencyReport {
    pub fn new() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            stats: ConsistencyStats {
                total_triples: 0,
                total_classes: 0,
                total_properties: 0,
                total_individuals: 0,
                max_hierarchy_depth: 0,
            },
        }
    }

    pub fn add_error(&mut self, error: ValidationError) {
        self.valid = false;
        self.errors.push(error.to_string());
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

impl Default for ConsistencyReport {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// ConsistencyChecker
// =============================================================================

/// Validates RDF graph consistency
///
/// Performs comprehensive validation including:
/// - Class hierarchy validation (no cycles)
/// - Property domain/range checking
/// - Cardinality constraints
/// - Contradiction detection
/// - Required property presence
pub struct ConsistencyChecker {
    store: TripleStore,
}

impl ConsistencyChecker {
    /// Create a new consistency checker with the given TripleStore
    pub fn new(store: TripleStore) -> Self {
        Self { store }
    }

    /// Check all consistency rules
    pub fn check_all(&self) -> ConsistencyReport {
        let mut report = ConsistencyReport::new();

        // Gather statistics
        report.stats.total_triples = self.count_triples();
        report.stats.total_classes = self.count_classes();
        report.stats.total_properties = self.count_properties();
        report.stats.total_individuals = self.count_individuals();

        // Run all checks
        if let Err(e) = self.check_class_hierarchy(&mut report) {
            report.add_error(ValidationError::Custom {
                message: format!("Class hierarchy check failed: {}", e),
            });
        }

        if let Err(e) = self.check_property_domains(&mut report) {
            report.add_error(ValidationError::Custom {
                message: format!("Property domain check failed: {}", e),
            });
        }

        if let Err(e) = self.check_property_ranges(&mut report) {
            report.add_error(ValidationError::Custom {
                message: format!("Property range check failed: {}", e),
            });
        }

        if let Err(e) = self.check_cardinality(&mut report) {
            report.add_error(ValidationError::Custom {
                message: format!("Cardinality check failed: {}", e),
            });
        }

        if let Err(e) = self.check_required_properties(&mut report) {
            report.add_error(ValidationError::Custom {
                message: format!("Required property check failed: {}", e),
            });
        }

        report
    }

    /// Check for cycles in class hierarchy
    pub fn check_class_hierarchy(&self, report: &mut ConsistencyReport) -> Result<()> {
        let query = r#"
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
            SELECT ?class ?superclass
            WHERE {
                ?class rdfs:subClassOf ?superclass .
                FILTER(isIRI(?superclass))
            }
        "#;

        let json_result = self.store.query_sparql(query)
            .map_err(|e| anyhow!("Failed to execute query: {}", e))?;
        
        let parsed: JsonValue = serde_json::from_str(&json_result)
            .context("Failed to parse query results")?;
        
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        // Parse JSON results from ggen
        if let Some(results) = parsed.get("results") {
            if let Some(bindings_array) = results.get("bindings").and_then(|b| b.as_array()) {
                for binding_obj in bindings_array {
                    if let Some(binding_map) = binding_obj.as_object() {
                        let class = binding_map
                            .get("class")
                            .and_then(|v| extract_iri_from_json_value(v));
                        let superclass = binding_map
                            .get("superclass")
                            .and_then(|v| extract_iri_from_json_value(v));
                        
                        if let (Some(class_str), Some(superclass_str)) = (class, superclass) {
                            graph
                                .entry(class_str)
                                .or_default()
                                .push(superclass_str);
                        }
                    }
                }
            }
        }

        // Detect cycles using DFS
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for class in graph.keys() {
            if !visited.contains(class) {
                if let Some(cycle) =
                    self.detect_cycle(class, &graph, &mut visited, &mut rec_stack, &mut path)
                {
                    report.add_error(ValidationError::CyclicHierarchy { cycle });
                }
            }
        }

        // Calculate max hierarchy depth
        report.stats.max_hierarchy_depth = self.calculate_max_depth(&graph);

        Ok(())
    }

    fn detect_cycle(
        &self,
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if let Some(cycle) =
                        self.detect_cycle(neighbor, graph, visited, rec_stack, path)
                    {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(neighbor) {
                    // Found a cycle
                    let cycle_start = path.iter().position(|n| n == neighbor).unwrap();
                    let mut cycle = path[cycle_start..].to_vec();
                    cycle.push(neighbor.to_string());
                    return Some(cycle);
                }
            }
        }

        rec_stack.remove(node);
        path.pop();
        None
    }

    fn calculate_max_depth(&self, graph: &HashMap<String, Vec<String>>) -> usize {
        let mut max_depth = 0;
        let mut visited = HashSet::new();

        for node in graph.keys() {
            let depth = self.get_depth(node, graph, &mut visited, 0);
            max_depth = max_depth.max(depth);
        }

        max_depth
    }

    fn get_depth(
        &self,
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        current_depth: usize,
    ) -> usize {
        if visited.contains(node) {
            return current_depth;
        }

        visited.insert(node.to_string());

        if let Some(neighbors) = graph.get(node) {
            let mut max_child_depth = current_depth;
            for neighbor in neighbors {
                let depth = self.get_depth(neighbor, graph, visited, current_depth + 1);
                max_child_depth = max_child_depth.max(depth);
            }
            max_child_depth
        } else {
            current_depth
        }
    }

    /// Check property domain constraints
    pub fn check_property_domains(&self, report: &mut ConsistencyReport) -> Result<()> {
        let query = r#"
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
            PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
            SELECT ?property ?domain ?subject
            WHERE {
                ?property rdfs:domain ?domain .
                ?subject ?property ?object .
                FILTER(?property != rdf:type)
            }
        "#;

        let json_result = self.store.query_sparql(query)
            .map_err(|e| anyhow!("Failed to execute query: {}", e))?;
        
        let parsed: JsonValue = serde_json::from_str(&json_result)
            .context("Failed to parse query results")?;

        // Parse JSON results from ggen
        if let Some(results) = parsed.get("results") {
            if let Some(bindings_array) = results.get("bindings").and_then(|b| b.as_array()) {
                for binding_obj in bindings_array {
                    if let Some(binding_map) = binding_obj.as_object() {
                        let property = binding_map.get("property")
                            .map(|v| extract_term_string_from_json_value(v));
                        let domain = binding_map.get("domain")
                            .and_then(|v| extract_iri_from_json_value(v));
                        let subject = binding_map.get("subject")
                            .and_then(|v| extract_iri_from_json_value(v));
                        
                        if let (Some(property_str), Some(domain_str), Some(subject_str)) = (property, domain, subject) {
                            // Check if subject is of the correct type
                            if !self.is_instance_of(&subject_str, &domain_str)? {
                                report.add_error(ValidationError::InvalidDomainRange {
                                    property: property_str,
                                    subject: subject_str,
                                    object: domain_str,
                                    message: "Subject not in property domain".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Check property range constraints
    pub fn check_property_ranges(&self, report: &mut ConsistencyReport) -> Result<()> {
        let query = r#"
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
            PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
            SELECT ?property ?range ?subject ?object
            WHERE {
                ?property rdfs:range ?range .
                ?subject ?property ?object .
                FILTER(?property != rdf:type && isIRI(?object))
            }
        "#;

        let json_result = self.store.query_sparql(query)
            .map_err(|e| anyhow!("Failed to execute query: {}", e))?;
        
        let parsed: JsonValue = serde_json::from_str(&json_result)
            .context("Failed to parse query results")?;

        // Parse JSON results from ggen
        if let Some(results) = parsed.get("results") {
            if let Some(bindings_array) = results.get("bindings").and_then(|b| b.as_array()) {
                for binding_obj in bindings_array {
                    if let Some(binding_map) = binding_obj.as_object() {
                        let property = binding_map.get("property")
                            .map(|v| extract_term_string_from_json_value(v));
                        let range = binding_map.get("range")
                            .and_then(|v| extract_iri_from_json_value(v));
                        let object = binding_map.get("object")
                            .and_then(|v| extract_iri_from_json_value(v));
                        
                        if let (Some(property_str), Some(range_str), Some(object_str)) = (property, range, object) {
                            // Check if object is of the correct type
                            if !self.is_instance_of(&object_str, &range_str)? {
                                report.add_error(ValidationError::InvalidDomainRange {
                                    property: property_str,
                                    subject: "N/A".to_string(),
                                    object: object_str,
                                    message: "Object not in property range".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Check SHACL cardinality constraints
    pub fn check_cardinality(&self, report: &mut ConsistencyReport) -> Result<()> {
        let query = r#"
            PREFIX sh: <http://www.w3.org/ns/shacl#>
            PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
            SELECT ?shape ?targetClass ?path ?minCount ?maxCount
            WHERE {
                ?shape a sh:NodeShape ;
                       sh:targetClass ?targetClass ;
                       sh:property ?propertyShape .
                ?propertyShape sh:path ?path .
                OPTIONAL { ?propertyShape sh:minCount ?minCount }
                OPTIONAL { ?propertyShape sh:maxCount ?maxCount }
            }
        "#;

        let json_result = self.store.query_sparql(query)
            .map_err(|e| anyhow!("Failed to execute query: {}", e))?;
        
        let parsed: JsonValue = serde_json::from_str(&json_result)
            .context("Failed to parse query results")?;

        // Parse JSON results from ggen
        if let Some(results) = parsed.get("results") {
            if let Some(bindings_array) = results.get("bindings").and_then(|b| b.as_array()) {
                for binding_obj in bindings_array {
                    if let Some(binding_map) = binding_obj.as_object() {
                        let target_class = binding_map.get("targetClass")
                            .and_then(|v| extract_iri_from_json_value(v));
                        let path = binding_map.get("path")
                            .and_then(|v| extract_iri_from_json_value(v));
                        
                        if let (Some(target_class_str), Some(path_str)) = (target_class, path) {
                            let min_count = binding_map.get("minCount")
                                .and_then(|v| extract_literal_from_json_value(v))
                                .and_then(|s| s.parse::<usize>().ok());
                            
                            let max_count = binding_map.get("maxCount")
                                .and_then(|v| extract_literal_from_json_value(v))
                                .and_then(|s| s.parse::<usize>().ok());
                            
                            // Check instances of target class
                            self.check_instance_cardinality(
                                &target_class_str,
                                &path_str,
                                min_count,
                                max_count,
                                report,
                            )?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn check_instance_cardinality(
        &self,
        target_class: &str,
        path: &str,
        min_count: Option<usize>,
        max_count: Option<usize>,
        report: &mut ConsistencyReport,
    ) -> Result<()> {
        let query = format!(
            r#"
            PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
            SELECT ?instance (COUNT(?value) as ?count)
            WHERE {{
                ?instance rdf:type <{}>  .
                OPTIONAL {{ ?instance <{}>  ?value }}
            }}
            GROUP BY ?instance
        "#,
            target_class, path
        );

        let json_result = self.store.query_sparql(&query)
            .map_err(|e| anyhow!("Failed to execute query: {}", e))?;
        
        let parsed: JsonValue = serde_json::from_str(&json_result)
            .context("Failed to parse query results")?;

        // Parse JSON results from ggen
        if let Some(results) = parsed.get("results") {
            if let Some(bindings_array) = results.get("bindings").and_then(|b| b.as_array()) {
                for binding_obj in bindings_array {
                    if let Some(binding_map) = binding_obj.as_object() {
                        let instance = binding_map.get("instance")
                            .and_then(|v| extract_iri_from_json_value(v));
                        let count_val = binding_map.get("count");
                        
                        if let Some(instance_str) = instance {
                            let count = count_val
                                .and_then(|v| extract_literal_from_json_value(v))
                                .and_then(|s| s.parse::<usize>().ok())
                                .unwrap_or(0);

                            if let Some(min) = min_count {
                                if count < min {
                                    report.add_error(ValidationError::CardinalityViolation {
                                        node: instance_str.clone(),
                                        property: path.to_string(),
                                        expected: format!("at least {}", min),
                                        actual: count,
                                    });
                                }
                            }

                            if let Some(max) = max_count {
                                if count > max {
                                    report.add_error(ValidationError::CardinalityViolation {
                                        node: instance_str,
                                        property: path.to_string(),
                                        expected: format!("at most {}", max),
                                        actual: count,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Check required properties from DDD patterns
    pub fn check_required_properties(&self, report: &mut ConsistencyReport) -> Result<()> {
        let query = r#"
            PREFIX ddd: <http://ggen-mcp.dev/ontology/ddd#>
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
            PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
            SELECT ?class ?instance ?requiredProp
            WHERE {
                ?class rdfs:subClassOf* ddd:AggregateRoot .
                ?class ddd:hasProperty ?requiredProp .
                ?instance rdf:type ?class .
                FILTER NOT EXISTS { ?instance ?requiredProp ?value }
            }
        "#;

        let json_result = self.store.query_sparql(query)
            .map_err(|e| anyhow!("Failed to execute query: {}", e))?;
        
        let parsed: JsonValue = serde_json::from_str(&json_result)
            .context("Failed to parse query results")?;

        // Parse JSON results from ggen
        if let Some(results) = parsed.get("results") {
            if let Some(bindings_array) = results.get("bindings").and_then(|b| b.as_array()) {
                for binding_obj in bindings_array {
                    if let Some(binding_map) = binding_obj.as_object() {
                        let instance = binding_map.get("instance")
                            .and_then(|v| extract_iri_from_json_value(v));
                        let required_prop = binding_map.get("requiredProp")
                            .map(|v| extract_term_string_from_json_value(v));
                        
                        if let (Some(instance_str), Some(required_prop_str)) = (instance, required_prop) {
                            report.add_error(ValidationError::MissingProperty {
                                node: instance_str,
                                property: required_prop_str,
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    // Helper methods

    fn is_instance_of(&self, instance: &str, class: &str) -> Result<bool> {
        let query = format!(
            r#"
            PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
            ASK {{
                <{}> rdf:type ?type .
                ?type rdfs:subClassOf* <{}> .
            }}
        "#,
            instance, class
        );

        let json_result = self.store.query_sparql(&query)
            .map_err(|e| anyhow!("Failed to execute ASK query: {}", e))?;
        
        let parsed: JsonValue = serde_json::from_str(&json_result)
            .context("Failed to parse ASK result")?;
        
        if let Some(boolean) = parsed.get("boolean") {
            if let Some(result) = boolean.as_bool() {
                return Ok(result);
            }
        }
        
        Ok(false)
    }

    fn count_triples(&self) -> usize {
        self.query_count("SELECT (COUNT(*) AS ?count) WHERE { ?s ?p ?o }")
    }

    fn count_classes(&self) -> usize {
        let query = r#"
            PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
            PREFIX owl: <http://www.w3.org/2002/07/owl#>
            SELECT (COUNT(DISTINCT ?class) as ?count)
            WHERE {
                { ?class rdf:type rdfs:Class }
                UNION
                { ?class rdf:type owl:Class }
            }
        "#;

        self.query_count(query)
    }

    fn count_properties(&self) -> usize {
        let query = r#"
            PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
            PREFIX owl: <http://www.w3.org/2002/07/owl#>
            SELECT (COUNT(DISTINCT ?prop) as ?count)
            WHERE {
                { ?prop rdf:type rdf:Property }
                UNION
                { ?prop rdf:type owl:ObjectProperty }
                UNION
                { ?prop rdf:type owl:DatatypeProperty }
            }
        "#;

        self.query_count(query)
    }

    fn count_individuals(&self) -> usize {
        let query = r#"
            PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
            PREFIX owl: <http://www.w3.org/2002/07/owl#>
            SELECT (COUNT(DISTINCT ?individual) as ?count)
            WHERE {
                ?individual rdf:type ?class .
                FILTER(?class != owl:Class && ?class != rdf:Property)
            }
        "#;

        self.query_count(query)
    }

    fn query_count(&self, query: &str) -> usize {
        let json_result = match self.store.query_sparql(query) {
            Ok(r) => r,
            Err(_) => return 0,
        };
        
        let parsed: JsonValue = match serde_json::from_str(&json_result) {
            Ok(p) => p,
            Err(_) => return 0,
        };
        
        // Extract count from SPARQL JSON results format
        if let Some(results) = parsed.get("results") {
            if let Some(bindings) = results.get("bindings").and_then(|b| b.as_array()) {
                if let Some(first) = bindings.first() {
                    if let Some(count_obj) = first.get("count") {
                        // Handle both string and structured format
                        if let Some(count_str) = count_obj.as_str() {
                            return count_str.parse().unwrap_or(0);
                        } else if let Some(count_val) = count_obj.get("value") {
                            if let Some(count_str) = count_val.as_str() {
                                return count_str.parse().unwrap_or(0);
                            }
                        }
                    }
                }
            }
        }
        
        0
    }
}

// =============================================================================
// SchemaValidator
// =============================================================================

/// Validates ontology against expected schema patterns
///
/// Checks for:
/// - Required namespaces (ddd, ggen, sh, rdfs, xsd)
/// - DDD aggregate structure
/// - Property type declarations
/// - Invariant definitions
/// - Orphaned nodes
pub struct SchemaValidator {
    store: TripleStore,
    required_namespaces: HashMap<String, String>,
}

impl SchemaValidator {
    pub fn new(store: TripleStore) -> Self {
        let mut required_namespaces = HashMap::new();
        required_namespaces.insert(
            "ddd".to_string(),
            "http://ggen-mcp.dev/ontology/ddd#".to_string(),
        );
        required_namespaces.insert(
            "ggen".to_string(),
            "http://ggen-mcp.dev/ontology/".to_string(),
        );
        required_namespaces.insert("sh".to_string(), "http://www.w3.org/ns/shacl#".to_string());
        required_namespaces.insert(
            "rdfs".to_string(),
            "http://www.w3.org/2000/01/rdf-schema#".to_string(),
        );
        required_namespaces.insert(
            "xsd".to_string(),
            "http://www.w3.org/2001/XMLSchema#".to_string(),
        );

        Self {
            store,
            required_namespaces,
        }
    }

    /// Validate all schema patterns
    pub fn validate_all(&self) -> ConsistencyReport {
        let mut report = ConsistencyReport::new();

        if let Err(e) = self.check_required_namespaces(&mut report) {
            report.add_error(ValidationError::Custom {
                message: format!("Namespace check failed: {}", e),
            });
        }

        if let Err(e) = self.check_ddd_aggregate_structure(&mut report) {
            report.add_error(ValidationError::Custom {
                message: format!("DDD aggregate structure check failed: {}", e),
            });
        }

        if let Err(e) = self.check_property_types(&mut report) {
            report.add_error(ValidationError::Custom {
                message: format!("Property type check failed: {}", e),
            });
        }

        if let Err(e) = self.check_invariants(&mut report) {
            report.add_error(ValidationError::Custom {
                message: format!("Invariant check failed: {}", e),
            });
        }

        if let Err(e) = self.check_orphaned_nodes(&mut report) {
            report.add_error(ValidationError::Custom {
                message: format!("Orphaned node check failed: {}", e),
            });
        }

        report
    }

    /// Check for required namespaces
    pub fn check_required_namespaces(&self, report: &mut ConsistencyReport) -> Result<()> {
        for (prefix, expected_uri) in &self.required_namespaces {
            let query = format!(
                r#"
                ASK {{
                    ?s ?p ?o .
                    FILTER(STRSTARTS(STR(?s), "{}") || STRSTARTS(STR(?p), "{}") || STRSTARTS(STR(?o), "{}"))
                }}
            "#,
                expected_uri, expected_uri, expected_uri
            );

            let json_result = self.store.query_sparql(&query)
                .map_err(|e| anyhow!("Failed to execute ASK query: {}", e))?;
            
            let parsed: JsonValue = serde_json::from_str(&json_result)
                .context("Failed to parse ASK result")?;
            
            if let Some(boolean) = parsed.get("boolean") {
                if let Some(found) = boolean.as_bool() {
                    if !found {
                        report.add_warning(format!(
                            "Recommended namespace '{}' ({}) not found in ontology",
                            prefix, expected_uri
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Check DDD aggregate structure
    pub fn check_ddd_aggregate_structure(&self, report: &mut ConsistencyReport) -> Result<()> {
        let query = r#"
            PREFIX ddd: <http://ggen-mcp.dev/ontology/ddd#>
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
            PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
            SELECT ?aggregate
            WHERE {
                ?aggregate rdfs:subClassOf ddd:AggregateRoot .
                FILTER NOT EXISTS { ?aggregate ddd:hasProperty ?prop }
            }
        "#;

        let json_result = self.store.query_sparql(query)
            .map_err(|e| anyhow!("Failed to execute query: {}", e))?;
        
        let parsed: JsonValue = serde_json::from_str(&json_result)
            .context("Failed to parse query results")?;

        // Parse JSON results from ggen
        if let Some(results) = parsed.get("results") {
            if let Some(bindings_array) = results.get("bindings").and_then(|b| b.as_array()) {
                for binding_obj in bindings_array {
                    if let Some(binding_map) = binding_obj.as_object() {
                        if let Some(aggregate_val) = binding_map.get("aggregate") {
                            let aggregate_str = extract_term_string_from_json_value(aggregate_val);
                            report.add_error(ValidationError::InvalidDddStructure {
                                aggregate: aggregate_str,
                                reason: "Aggregate has no properties defined".to_string(),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Check that all properties have types
    pub fn check_property_types(&self, report: &mut ConsistencyReport) -> Result<()> {
        let query = r#"
            PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
            PREFIX owl: <http://www.w3.org/2002/07/owl#>
            SELECT ?prop
            WHERE {
                ?s ?prop ?o .
                FILTER(isIRI(?prop))
                FILTER(?prop != rdf:type)
                FILTER NOT EXISTS {
                    { ?prop rdf:type rdf:Property }
                    UNION
                    { ?prop rdf:type owl:ObjectProperty }
                    UNION
                    { ?prop rdf:type owl:DatatypeProperty }
                }
            }
        "#;

        let json_result = self.store.query_sparql(query)
            .map_err(|e| anyhow!("Failed to execute query: {}", e))?;
        
        let parsed: JsonValue = serde_json::from_str(&json_result)
            .context("Failed to parse query results")?;
        
        let mut untyped = HashSet::new();

        // Parse JSON results from ggen
        if let Some(results) = parsed.get("results") {
            if let Some(bindings_array) = results.get("bindings").and_then(|b| b.as_array()) {
                for binding_obj in bindings_array {
                    if let Some(binding_map) = binding_obj.as_object() {
                        if let Some(prop_val) = binding_map.get("prop") {
                            let prop_str = extract_term_string_from_json_value(prop_val);
                            // Filter out common vocabularies that don't need explicit typing
                            if !prop_str.contains("rdfs#")
                                && !prop_str.contains("owl#")
                                && !prop_str.contains("rdf#")
                            {
                                untyped.insert(prop_str);
                            }
                        }
                    }
                }
            }
        }

        for prop in untyped {
            report.add_warning(format!(
                "Property {} has no explicit type declaration",
                prop
            ));
        }

        Ok(())
    }

    /// Check invariant definitions
    pub fn check_invariants(&self, report: &mut ConsistencyReport) -> Result<()> {
        let query = r#"
            PREFIX ddd: <http://ggen-mcp.dev/ontology/ddd#>
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
            SELECT ?class ?invariant
            WHERE {
                ?class ddd:hasInvariant ?invariant .
                FILTER NOT EXISTS { ?invariant ddd:check ?check }
            }
        "#;

        let json_result = self.store.query_sparql(query)
            .map_err(|e| anyhow!("Failed to execute query: {}", e))?;
        
        let parsed: JsonValue = serde_json::from_str(&json_result)
            .context("Failed to parse query results")?;

        // Parse JSON results from ggen
        if let Some(results) = parsed.get("results") {
            if let Some(bindings_array) = results.get("bindings").and_then(|b| b.as_array()) {
                for binding_obj in bindings_array {
                    if let Some(binding_map) = binding_obj.as_object() {
                        if let Some(class_val) = binding_map.get("class") {
                            let class_str = extract_term_string_from_json_value(class_val);
                            report.add_error(ValidationError::InvalidInvariant {
                                node: class_str,
                                reason: "Invariant has no check expression".to_string(),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Check for orphaned nodes
    pub fn check_orphaned_nodes(&self, report: &mut ConsistencyReport) -> Result<()> {
        let query = r#"
            PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
            SELECT DISTINCT ?node
            WHERE {
                ?node ?p1 ?o .
                FILTER(isIRI(?node))
                FILTER NOT EXISTS {
                    { ?s ?p2 ?node }
                    UNION
                    { ?node rdf:type ?type }
                }
            }
        "#;

        let json_result = self.store.query_sparql(query)
            .map_err(|e| anyhow!("Failed to execute query: {}", e))?;
        
        let parsed: JsonValue = serde_json::from_str(&json_result)
            .context("Failed to parse query results")?;

        // Parse JSON results from ggen
        if let Some(results) = parsed.get("results") {
            if let Some(bindings_array) = results.get("bindings").and_then(|b| b.as_array()) {
                for binding_obj in bindings_array {
                    if let Some(binding_map) = binding_obj.as_object() {
                        if let Some(node_val) = binding_map.get("node") {
                            let node_str = extract_term_string_from_json_value(node_val);
                            // Filter out blank nodes and common vocabularies
                            if !node_str.starts_with("_:") && !node_str.contains("/ns/") {
                                report.add_warning(format!("Potentially orphaned node: {}", node_str));
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

// =============================================================================
// NamespaceManager
// =============================================================================

/// Safe namespace handling
///
/// Features:
/// - Register and validate prefixes
/// - Prevent prefix collisions
/// - Resolve QNames safely
/// - URI expansion with validation
/// - Default namespace handling
#[derive(Debug, Clone)]
pub struct NamespaceManager {
    namespaces: HashMap<String, String>,
    default_namespace: Option<String>,
}

impl NamespaceManager {
    pub fn new() -> Self {
        let mut manager = Self {
            namespaces: HashMap::new(),
            default_namespace: None,
        };

        // Register common namespaces
        manager.register_common_namespaces();
        manager
    }

    fn register_common_namespaces(&mut self) {
        let _ = self.register("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#");
        let _ = self.register("rdfs", "http://www.w3.org/2000/01/rdf-schema#");
        let _ = self.register("owl", "http://www.w3.org/2002/07/owl#");
        let _ = self.register("xsd", "http://www.w3.org/2001/XMLSchema#");
        let _ = self.register("sh", "http://www.w3.org/ns/shacl#");
        let _ = self.register("ddd", "http://ggen-mcp.dev/ontology/ddd#");
    }

    /// Register a namespace prefix
    pub fn register(&mut self, prefix: &str, uri: &str) -> ValidationResult<()> {
        if let Some(existing_uri) = self.namespaces.get(prefix) {
            if existing_uri != uri {
                return Err(ValidationError::NamespaceCollision {
                    prefix: prefix.to_string(),
                    uri1: existing_uri.clone(),
                    uri2: uri.to_string(),
                });
            }
        }

        self.namespaces.insert(prefix.to_string(), uri.to_string());
        Ok(())
    }

    /// Set default namespace
    pub fn set_default(&mut self, uri: &str) {
        self.default_namespace = Some(uri.to_string());
    }

    /// Get namespace URI for prefix
    pub fn get(&self, prefix: &str) -> Option<&String> {
        self.namespaces.get(prefix)
    }

    /// Expand a QName to full URI
    pub fn expand(&self, qname: &str) -> ValidationResult<String> {
        if qname.contains("://") {
            // Already a full URI
            return Ok(qname.to_string());
        }

        if let Some(colon_pos) = qname.find(':') {
            let prefix = &qname[..colon_pos];
            let local = &qname[colon_pos + 1..];

            if let Some(uri) = self.namespaces.get(prefix) {
                Ok(format!("{}{}", uri, local))
            } else {
                Err(ValidationError::Custom {
                    message: format!("Unknown namespace prefix: {}", prefix),
                })
            }
        } else if let Some(default_ns) = &self.default_namespace {
            Ok(format!("{}{}", default_ns, qname))
        } else {
            Err(ValidationError::Custom {
                message: format!("Cannot expand '{}': no default namespace set", qname),
            })
        }
    }

    /// Compact a full URI to QName
    pub fn compact(&self, uri: &str) -> String {
        for (prefix, ns_uri) in &self.namespaces {
            if uri.starts_with(ns_uri) {
                let local = &uri[ns_uri.len()..];
                return format!("{}:{}", prefix, local);
            }
        }

        // Check default namespace
        if let Some(default_ns) = &self.default_namespace {
            if uri.starts_with(default_ns) {
                return uri[default_ns.len()..].to_string();
            }
        }

        uri.to_string()
    }

    /// Get all registered namespaces
    pub fn all(&self) -> &HashMap<String, String> {
        &self.namespaces
    }
}

impl Default for NamespaceManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// OntologyMerger
// =============================================================================

/// Result of a merge operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeResult {
    pub success: bool,
    pub merged_triples: usize,
    pub conflicts: Vec<String>,
    pub provenance: HashMap<String, String>,
}

/// Safe ontology merging
///
/// Features:
/// - Conflict detection before merge
/// - Preserve provenance
/// - Handle duplicate definitions
/// - Validate after merge
/// - Rollback on failure
pub struct OntologyMerger {
    namespaces: NamespaceManager,
}

impl OntologyMerger {
    pub fn new() -> Self {
        Self {
            namespaces: NamespaceManager::new(),
        }
    }

    /// Merge two TripleStores
    /// Note: TripleStore doesn't support direct iteration/insertion like Store.
    /// This implementation uses SPARQL to detect conflicts. Actual merging would require
    /// exporting source to Turtle and loading into target.
    pub fn merge(&self, target: &mut TripleStore, source: &TripleStore) -> Result<MergeResult> {
        let mut result = MergeResult {
            success: false,
            merged_triples: 0,
            conflicts: Vec::new(),
            provenance: HashMap::new(),
        };

        // Detect conflicts first (using SPARQL queries)
        self.detect_conflicts(target, source, &mut result)?;

        if !result.conflicts.is_empty() {
            return Ok(result);
        }

        // TripleStore doesn't support direct merge via insertion
        // Would need to:
        // 1. Export source triples to Turtle format
        // 2. Load Turtle into target
        // For now, mark as requiring manual merge
        result.success = false;
        result.conflicts.push("TripleStore merge requires Turtle export/import - not implemented".to_string());
        Ok(result)
    }

    fn detect_conflicts(
        &self,
        target: &TripleStore,
        source: &TripleStore,
        result: &mut MergeResult,
    ) -> Result<()> {
        // Check for conflicting class definitions
        self.check_class_conflicts(target, source, result)?;

        // Check for conflicting property definitions
        self.check_property_conflicts(target, source, result)?;

        Ok(())
    }

    fn check_class_conflicts(
        &self,
        target: &TripleStore,
        source: &TripleStore,
        result: &mut MergeResult,
    ) -> Result<()> {
        let query = r#"
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
            PREFIX owl: <http://www.w3.org/2002/07/owl#>
            SELECT ?class ?superclass
            WHERE {
                { ?class a rdfs:Class }
                UNION
                { ?class a owl:Class }
                OPTIONAL { ?class rdfs:subClassOf ?superclass }
            }
        "#;

        let source_json = source.query_sparql(query)
            .map_err(|e| anyhow!("Failed to query source classes: {}", e))?;
        
        let source_parsed: JsonValue = serde_json::from_str(&source_json)
            .context("Failed to parse source query results")?;
        
        let mut source_classes: HashMap<String, Option<String>> = HashMap::new();

        if let Some(results) = source_parsed.get("results") {
            if let Some(bindings_array) = results.get("bindings").and_then(|b| b.as_array()) {
                for binding_obj in bindings_array {
                    if let Some(binding_map) = binding_obj.as_object() {
                        let class = binding_map.get("class")
                            .and_then(|v| extract_iri_from_json_value(v));
                        let superclass = binding_map.get("superclass")
                            .and_then(|v| extract_iri_from_json_value(v));
                        
                        if let Some(class_str) = class {
                            source_classes.insert(class_str, superclass);
                        }
                    }
                }
            }
        }

        // Check against target
        for (class, source_super) in source_classes {
            let target_query = format!(
                r#"
                PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
                SELECT ?superclass
                WHERE {{
                    <{}> rdfs:subClassOf ?superclass .
                }}
            "#,
                class
            );

            let target_json = target.query_sparql(&target_query)
                .map_err(|e| anyhow!("Failed to query target: {}", e))?;
            
            let target_parsed: JsonValue = serde_json::from_str(&target_json)
                .context("Failed to parse target query results")?;

            if let Some(results) = target_parsed.get("results") {
                if let Some(bindings_array) = results.get("bindings").and_then(|b| b.as_array()) {
                    for binding_obj in bindings_array {
                        if let Some(binding_map) = binding_obj.as_object() {
                            if let Some(target_super_val) = binding_map.get("superclass") {
                                let target_super_str = extract_term_string_from_json_value(target_super_val);
                                if let Some(ref source_super_str) = source_super {
                                    if source_super_str != &target_super_str {
                                        result.conflicts.push(format!(
                                            "Class {} has conflicting superclass: {} vs {}",
                                            class, source_super_str, target_super_str
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn check_property_conflicts(
        &self,
        target: &TripleStore,
        source: &TripleStore,
        result: &mut MergeResult,
    ) -> Result<()> {
        let query = r#"
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
            PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
            SELECT ?property ?domain ?range
            WHERE {
                ?property a rdf:Property .
                OPTIONAL { ?property rdfs:domain ?domain }
                OPTIONAL { ?property rdfs:range ?range }
            }
        "#;

        let source_json = source.query_sparql(query)
            .map_err(|e| anyhow!("Failed to query source properties: {}", e))?;
        
        let source_parsed: JsonValue = serde_json::from_str(&source_json)
            .context("Failed to parse source query results")?;
        
        let mut source_props: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();

        if let Some(results) = source_parsed.get("results") {
            if let Some(bindings_array) = results.get("bindings").and_then(|b| b.as_array()) {
                for binding_obj in bindings_array {
                    if let Some(binding_map) = binding_obj.as_object() {
                        let property = binding_map.get("property")
                            .and_then(|v| extract_iri_from_json_value(v));
                        let domain = binding_map.get("domain")
                            .and_then(|v| extract_iri_from_json_value(v));
                        let range = binding_map.get("range")
                            .and_then(|v| extract_iri_from_json_value(v));
                        
                        if let Some(property_str) = property {
                            source_props.insert(property_str, (domain, range));
                        }
                    }
                }
            }
        }

        // Check for conflicts
        for (prop, (source_domain, source_range)) in source_props {
            let target_query = format!(
                r#"
                PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
                SELECT ?domain ?range
                WHERE {{
                    OPTIONAL {{ <{}> rdfs:domain ?domain }}
                    OPTIONAL {{ <{}> rdfs:range ?range }}
                }}
            "#,
                prop, prop
            );

            let target_json = target.query_sparql(&target_query)
                .map_err(|e| anyhow!("Failed to query target: {}", e))?;
            
            let target_parsed: JsonValue = serde_json::from_str(&target_json)
                .context("Failed to parse target query results")?;

            if let Some(results) = target_parsed.get("results") {
                if let Some(bindings_array) = results.get("bindings").and_then(|b| b.as_array()) {
                    for binding_obj in bindings_array {
                        if let Some(binding_map) = binding_obj.as_object() {
                            let target_domain = binding_map.get("domain")
                                .and_then(|v| extract_iri_from_json_value(v));
                            let target_range = binding_map.get("range")
                                .and_then(|v| extract_iri_from_json_value(v));

                            if source_domain.is_some()
                                && target_domain.is_some()
                                && source_domain != target_domain
                            {
                                result.conflicts.push(format!(
                                    "Property {} has conflicting domain: {:?} vs {:?}",
                                    prop, source_domain, target_domain
                                ));
                            }

                            if source_range.is_some()
                                && target_range.is_some()
                                && source_range != target_range
                            {
                                result.conflicts.push(format!(
                                    "Property {} has conflicting range: {:?} vs {:?}",
                                    prop, source_range, target_range
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for OntologyMerger {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// HashVerifier
// =============================================================================

/// Verifies ontology integrity using cryptographic hashes
///
/// Features:
/// - Compute consistent hashes (SHA-256)
/// - Detect tampering or corruption
/// - Verify hash matches expected value
/// - Track version changes
pub struct HashVerifier {
    store: TripleStore,
}

impl HashVerifier {
    pub fn new(store: TripleStore) -> Self {
        Self { store }
    }

    /// Compute SHA-256 hash of the ontology
    pub fn compute_hash(&self) -> Result<String> {
        let mut hasher = Sha256::new();
        let mut triples = Vec::new();

        // Collect all triples using SPARQL query
        let query = "SELECT ?s ?p ?o WHERE { ?s ?p ?o } ORDER BY ?s ?p ?o";
        let json_result = self.store.query_sparql(query)
            .map_err(|e| anyhow!("Failed to query triples: {}", e))?;
        
        let parsed: JsonValue = serde_json::from_str(&json_result)
            .context("Failed to parse query results")?;

        // Parse JSON results from ggen
        if let Some(results) = parsed.get("results") {
            if let Some(bindings_array) = results.get("bindings").and_then(|b| b.as_array()) {
                for binding_obj in bindings_array {
                    if let Some(binding_map) = binding_obj.as_object() {
                        let s = binding_map.get("s")
                            .map(|v| extract_term_string_from_json_value(v))
                            .unwrap_or_default();
                        let p = binding_map.get("p")
                            .map(|v| extract_term_string_from_json_value(v))
                            .unwrap_or_default();
                        let o = binding_map.get("o")
                            .map(|v| extract_term_string_from_json_value(v))
                            .unwrap_or_default();
                        
                        let triple_str = format!("{} {} {} .", s, p, o);
                        triples.push(triple_str);
                    }
                }
            }
        }

        // Sort for consistency
        triples.sort();

        // Hash all triples
        for triple in triples {
            hasher.update(triple.as_bytes());
        }

        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    /// Verify hash matches expected value
    pub fn verify_hash(&self, expected_hash: &str) -> ValidationResult<()> {
        let actual_hash = self.compute_hash().map_err(|e| ValidationError::Custom {
            message: format!("Failed to compute hash: {}", e),
        })?;

        if actual_hash != expected_hash {
            return Err(ValidationError::HashMismatch {
                expected: expected_hash.to_string(),
                actual: actual_hash,
            });
        }

        Ok(())
    }

    /// Get or set ontology hash
    pub fn get_ontology_hash(&self) -> Result<Option<String>> {
        let query = r#"
            PREFIX ggen: <http://ggen-mcp.dev/ontology/>
            SELECT ?hash
            WHERE {
                ?ontology a <http://www.w3.org/2002/07/owl#Ontology> ;
                          ggen:ontologyHash ?hash .
            }
        "#;

        let json_result = self.store.query_sparql(query)
            .map_err(|e| anyhow!("Failed to execute query: {}", e))?;
        
        let parsed: JsonValue = serde_json::from_str(&json_result)
            .context("Failed to parse query results")?;

        // Parse JSON results from ggen
        if let Some(results) = parsed.get("results") {
            if let Some(bindings_array) = results.get("bindings").and_then(|b| b.as_array()) {
                if let Some(first) = bindings_array.first() {
                    if let Some(binding_map) = first.as_object() {
                        if let Some(hash_val) = binding_map.get("hash") {
                            if let Some(hash_str) = extract_literal_from_json_value(hash_val) {
                                return Ok(Some(hash_str));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Store hash in ontology
    /// Note: TripleStore doesn't support direct insertion. This would need to be done via
    /// Turtle file modification or a different approach.
    pub fn store_hash(&self, _hash: &str) -> Result<()> {
        // TripleStore doesn't support direct triple insertion
        // Would need to export to Turtle, modify, and reload
        // For now, return an error indicating this needs implementation
        Err(anyhow!("TripleStore doesn't support direct hash storage. Use Turtle file modification instead."))
    }

    /// Verify and update hash
    pub fn verify_and_update(&self) -> Result<bool> {
        let current_hash = self.compute_hash()?;

        if let Some(stored_hash) = self.get_ontology_hash()? {
            if stored_hash != current_hash {
                return Ok(false);
            }
        }

        self.store_hash(&current_hash)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace_manager() {
        let mut ns = NamespaceManager::new();

        // Test registration
        assert!(ns.register("mcp", "http://ggen-mcp.dev/mcp#").is_ok());

        // Test collision detection
        assert!(ns.register("mcp", "http://different-uri.dev/mcp#").is_err());

        // Test expansion
        assert_eq!(
            ns.expand("mcp:Tool").unwrap(),
            "http://ggen-mcp.dev/mcp#Tool"
        );

        // Test compaction
        assert_eq!(ns.compact("http://ggen-mcp.dev/mcp#Tool"), "mcp:Tool");
    }

    #[test]
    fn test_consistency_report() {
        let mut report = ConsistencyReport::new();
        assert!(report.valid);

        report.add_error(ValidationError::Custom {
            message: "Test error".to_string(),
        });
        assert!(!report.valid);
        assert_eq!(report.errors.len(), 1);

        report.add_warning("Test warning".to_string());
        assert_eq!(report.warnings.len(), 1);
    }
}
