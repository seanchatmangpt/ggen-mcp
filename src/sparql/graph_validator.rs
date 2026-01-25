// =============================================================================
// SPARQL Graph Validation
// =============================================================================
// Validates CONSTRUCT query results and RDF graphs
// Implements poka-yoke error-proofing for graph structures

use oxigraph::model::{BlankNode, Graph, NamedNode, Subject, Term, Triple};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Errors that can occur during graph validation
#[derive(Debug, Error, Clone, PartialEq)]
pub enum GraphValidationError {
    #[error("Expected triple pattern not found: {0}")]
    MissingPattern(String),

    #[error("Invalid subject type: expected {expected}, got {actual}")]
    InvalidSubjectType { expected: String, actual: String },

    #[error("Invalid predicate: expected {expected}, got {actual}")]
    InvalidPredicate { expected: String, actual: String },

    #[error("Invalid object type: expected {expected}, got {actual}")]
    InvalidObjectType { expected: String, actual: String },

    #[error("Graph is not well-formed: {0}")]
    NotWellFormed(String),

    #[error("Cycle detected in graph: {0}")]
    CycleDetected(String),

    #[error("Orphaned blank node: {0}")]
    OrphanedBlankNode(String),

    #[error("Blank node reference count violation: {0}")]
    BlankNodeViolation(String),

    #[error("Required property missing for subject {subject}: {property}")]
    MissingProperty { subject: String, property: String },

    #[error("Cardinality violation for {subject}.{property}: expected {expected}, got {actual}")]
    CardinalityViolation {
        subject: String,
        property: String,
        expected: String,
        actual: usize,
    },

    #[error("Custom validation error: {0}")]
    Custom(String),
}

/// Expected triple pattern for validation
#[derive(Debug, Clone)]
pub struct TriplePattern {
    pub subject_type: SubjectType,
    pub predicate: Option<String>,
    pub object_type: ObjectType,
    pub required: bool,
}

/// Expected type for triple subject
#[derive(Debug, Clone, PartialEq)]
pub enum SubjectType {
    IRI,
    BlankNode,
    SpecificIRI(String),
    IRIWithPrefix(String),
    Any,
}

/// Expected type for triple object
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectType {
    IRI,
    BlankNode,
    Literal,
    LiteralWithDatatype(String),
    SpecificIRI(String),
    Any,
}

impl SubjectType {
    /// Check if a subject matches this type
    pub fn matches(&self, subject: oxigraph::model::NamedOrBlankNodeRef) -> bool {
        match self {
            SubjectType::IRI => subject.is_named_node(),
            SubjectType::BlankNode => subject.is_blank_node(),
            SubjectType::SpecificIRI(iri) => {
                match subject {
                    oxigraph::model::NamedOrBlankNodeRef::NamedNode(n) => n.as_str() == iri,
                    _ => false,
                }
            }
            SubjectType::IRIWithPrefix(prefix) => {
                match subject {
                    oxigraph::model::NamedOrBlankNodeRef::NamedNode(n) => n.as_str().starts_with(prefix),
                    _ => false,
                }
            }
            SubjectType::Any => true,
        }
    }
}

impl ObjectType {
    /// Check if an object matches this type
    pub fn matches(&self, object: oxigraph::model::TermRef) -> bool {
        match self {
            ObjectType::IRI => object.is_named_node(),
            ObjectType::BlankNode => object.is_blank_node(),
            ObjectType::Literal => object.is_literal(),
            ObjectType::LiteralWithDatatype(dt) => {
                match object {
                    oxigraph::model::TermRef::Literal(l) => l.datatype().as_str() == dt,
                    _ => false,
                }
            }
            ObjectType::SpecificIRI(iri) => {
                match object {
                    oxigraph::model::TermRef::NamedNode(n) => n.as_str() == iri,
                    _ => false,
                }
            }
            ObjectType::Any => true,
        }
    }
}

/// Property cardinality constraint
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyCardinality {
    ExactlyOne,
    ZeroOrOne,
    OneOrMore,
    ZeroOrMore,
    Exact(usize),
}

impl PropertyCardinality {
    /// Check if count satisfies cardinality
    pub fn satisfies(&self, count: usize) -> bool {
        match self {
            PropertyCardinality::ExactlyOne => count == 1,
            PropertyCardinality::ZeroOrOne => count <= 1,
            PropertyCardinality::OneOrMore => count >= 1,
            PropertyCardinality::ZeroOrMore => true,
            PropertyCardinality::Exact(n) => count == *n,
        }
    }
}

/// Property specification for validation
#[derive(Debug, Clone)]
pub struct PropertySpec {
    pub predicate: String,
    pub object_type: ObjectType,
    pub cardinality: PropertyCardinality,
}

/// SPARQL Graph Validator
///
/// Validates CONSTRUCT query results and RDF graphs:
/// - Expected triple patterns
/// - Subject/predicate/object type checking
/// - Well-formed graph validation
/// - Cycle detection
/// - Blank node management
/// - Property cardinality
#[derive(Debug, Clone)]
pub struct GraphValidator {
    patterns: Vec<TriplePattern>,
    property_specs: HashMap<String, Vec<PropertySpec>>,
    check_well_formed: bool,
    check_cycles: bool,
    check_orphaned_blanks: bool,
}

impl GraphValidator {
    /// Create a new graph validator
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
            property_specs: HashMap::new(),
            check_well_formed: false,
            check_cycles: false,
            check_orphaned_blanks: false,
        }
    }

    /// Add a triple pattern to validate
    pub fn with_pattern(mut self, pattern: TriplePattern) -> Self {
        self.patterns.push(pattern);
        self
    }

    /// Add property specification for a subject type
    pub fn with_property(mut self, subject_iri: String, spec: PropertySpec) -> Self {
        self.property_specs
            .entry(subject_iri)
            .or_default()
            .push(spec);
        self
    }

    /// Enable well-formedness checking
    pub fn check_well_formed(mut self, enable: bool) -> Self {
        self.check_well_formed = enable;
        self
    }

    /// Enable cycle detection
    pub fn check_cycles(mut self, enable: bool) -> Self {
        self.check_cycles = enable;
        self
    }

    /// Enable orphaned blank node detection
    pub fn check_orphaned_blanks(mut self, enable: bool) -> Self {
        self.check_orphaned_blanks = enable;
        self
    }

    /// Validate a graph
    pub fn validate(&self, graph: &Graph) -> Result<(), GraphValidationError> {
        // Check required patterns
        for pattern in &self.patterns {
            if pattern.required {
                self.validate_pattern_exists(graph, pattern)?;
            }
        }

        // Validate all triples match expected patterns
        for triple_ref in graph.iter() {
            // Convert TripleRef to owned Triple for validation
            let triple = Triple::new(
                triple_ref.subject.to_owned(),
                triple_ref.predicate.to_owned(),
                triple_ref.object.to_owned(),
            );
            self.validate_triple(&triple)?;
        }

        // Check property specifications
        self.validate_properties(graph)?;

        // Optional well-formedness checks
        if self.check_well_formed {
            self.validate_well_formed(graph)?;
        }

        if self.check_cycles {
            self.detect_cycles(graph)?;
        }

        if self.check_orphaned_blanks {
            self.detect_orphaned_blanks(graph)?;
        }

        Ok(())
    }

    /// Check if a required pattern exists in the graph
    fn validate_pattern_exists(
        &self,
        graph: &Graph,
        pattern: &TriplePattern,
    ) -> Result<(), GraphValidationError> {
        let found = graph.iter().any(|triple| {
            pattern.subject_type.matches(triple.subject.as_ref())
                && pattern.object_type.matches(triple.object.as_ref())
                && if let Some(ref pred) = pattern.predicate {
                    triple.predicate.as_str() == pred
                } else {
                    true
                }
        });

        if !found {
            return Err(GraphValidationError::MissingPattern(format!(
                "{:?} -> {:?} -> {:?}",
                pattern.subject_type, pattern.predicate, pattern.object_type
            )));
        }

        Ok(())
    }

    /// Validate a single triple
    fn validate_triple(&self, triple: &Triple) -> Result<(), GraphValidationError> {
        // If we have patterns, at least one must match
        if !self.patterns.is_empty() {
            let matches = self.patterns.iter().any(|pattern| {
                pattern.subject_type.matches(triple.subject.as_ref())
                    && pattern.object_type.matches(triple.object.as_ref())
                    && if let Some(ref pred) = pattern.predicate {
                        triple.predicate.as_str() == pred
                    } else {
                        true
                    }
            });

            if !matches {
                return Err(GraphValidationError::NotWellFormed(format!(
                    "Triple does not match any expected pattern: {} {} {}",
                    triple.subject, triple.predicate, triple.object
                )));
            }
        }

        Ok(())
    }

    /// Validate property specifications
    fn validate_properties(&self, graph: &Graph) -> Result<(), GraphValidationError> {
        // Group triples by subject
        let mut subject_properties: HashMap<String, HashMap<String, Vec<Term>>> = HashMap::new();

        for triple in graph.iter() {
            let subject_str = triple.subject.to_string();
            let predicate_str = triple.predicate.as_str().to_string();

            subject_properties
                .entry(subject_str)
                .or_default()
                .entry(predicate_str)
                .or_default()
                .push(triple.object.to_owned());
        }

        // Check each subject against its property specs
        for (subject_pattern, specs) in &self.property_specs {
            for (subject, properties) in &subject_properties {
                // Simple pattern matching - could be enhanced
                if subject.contains(subject_pattern) || subject_pattern == "*" {
                    for spec in specs {
                        let count = properties
                            .get(&spec.predicate)
                            .map(|v| v.len())
                            .unwrap_or(0);

                        if !spec.cardinality.satisfies(count) {
                            return Err(GraphValidationError::CardinalityViolation {
                                subject: subject.clone(),
                                property: spec.predicate.clone(),
                                expected: format!("{:?}", spec.cardinality),
                                actual: count,
                            });
                        }

                        // Validate object types
                        if let Some(objects) = properties.get(&spec.predicate) {
                            for object in objects {
                                if !spec.object_type.matches(object) {
                                    return Err(GraphValidationError::InvalidObjectType {
                                        expected: format!("{:?}", spec.object_type),
                                        actual: object.to_string(),
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

    /// Check if graph is well-formed (basic structural checks)
    fn validate_well_formed(&self, graph: &Graph) -> Result<(), GraphValidationError> {
        if graph.is_empty() {
            return Err(GraphValidationError::NotWellFormed(
                "Graph is empty".to_string(),
            ));
        }

        // Check for valid IRIs
        for triple in graph.iter() {
            // Predicate must be an IRI
            if triple.predicate.as_str().is_empty() {
                return Err(GraphValidationError::NotWellFormed(
                    "Predicate IRI is empty".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Detect cycles in the graph
    fn detect_cycles(&self, graph: &Graph) -> Result<(), GraphValidationError> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        // Build adjacency list
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();

        for triple in graph.iter() {
            match &triple.object {
                Term::NamedNode(obj_node) => {
                    let subject_str = triple.subject.to_string();
                    let object_str = obj_node.as_str().to_string();

                    adjacency.entry(subject_str).or_default().push(object_str);
                }
                _ => {}
            }
        }

        // DFS for cycle detection
        for node in adjacency.keys() {
            if !visited.contains(node) {
                if self.has_cycle_dfs(node, &adjacency, &mut visited, &mut rec_stack)? {
                    return Err(GraphValidationError::CycleDetected(node.clone()));
                }
            }
        }

        Ok(())
    }

    /// DFS helper for cycle detection
    fn has_cycle_dfs(
        &self,
        node: &str,
        adjacency: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> Result<bool, GraphValidationError> {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        if let Some(neighbors) = adjacency.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if self.has_cycle_dfs(neighbor, adjacency, visited, rec_stack)? {
                        return Ok(true);
                    }
                } else if rec_stack.contains(neighbor) {
                    return Ok(true);
                }
            }
        }

        rec_stack.remove(node);
        Ok(false)
    }

    /// Detect orphaned blank nodes
    fn detect_orphaned_blanks(&self, graph: &Graph) -> Result<(), GraphValidationError> {
        let mut blank_nodes = HashSet::new();
        let mut referenced_blanks = HashSet::new();

        // Collect all blank nodes and references
        for triple in graph.iter() {
            match &triple.subject {
                Subject::BlankNode(bn) => {
                    blank_nodes.insert(bn.as_str().to_string());
                }
                _ => {}
            }

            match &triple.object {
                Term::BlankNode(bn) => {
                    referenced_blanks.insert(bn.as_str().to_string());
                }
                _ => {}
            }
        }

        // Find orphaned (unreferenced) blank nodes
        for bn in blank_nodes.difference(&referenced_blanks) {
            // Allow if blank node appears as subject (it's a root)
            let has_incoming = graph.iter().any(|t| {
                match &t.object {
                    Term::BlankNode(obj_bn) => obj_bn.as_str() == bn,
                    _ => false,
                }
            });

            if !has_incoming {
                // Check if it has outgoing edges (not completely orphaned)
                let has_outgoing = graph.iter().any(|t| {
                    match &t.subject {
                        Subject::BlankNode(subj_bn) => subj_bn.as_str() == bn,
                        _ => false,
                    }
                });

                if !has_outgoing {
                    return Err(GraphValidationError::OrphanedBlankNode(bn.clone()));
                }
            }
        }

        Ok(())
    }

    /// Count triples matching a pattern
    pub fn count_matching(&self, graph: &Graph, pattern: &TriplePattern) -> usize {
        graph
            .iter()
            .filter(|triple| {
                pattern.subject_type.matches(triple.subject.as_ref())
                    && pattern.object_type.matches(triple.object.as_ref())
                    && if let Some(ref pred) = pattern.predicate {
                        triple.predicate.as_str() == pred
                    } else {
                        true
                    }
            })
            .count()
    }
}

impl Default for GraphValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subject_type_matches() {
        let iri_type = SubjectType::IRI;
        let named_node = NamedNode::new("http://example.org/test").unwrap();
        let subject = Subject::NamedNode(named_node);

        assert!(iri_type.matches(&subject));
    }

    #[test]
    fn test_property_cardinality() {
        let exactly_one = PropertyCardinality::ExactlyOne;
        assert!(exactly_one.satisfies(1));
        assert!(!exactly_one.satisfies(0));
        assert!(!exactly_one.satisfies(2));

        let one_or_more = PropertyCardinality::OneOrMore;
        assert!(!one_or_more.satisfies(0));
        assert!(one_or_more.satisfies(1));
        assert!(one_or_more.satisfies(5));
    }
}
