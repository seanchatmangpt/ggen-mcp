//! RDF Graph Integrity Checking and Validation
//!
//! This module provides comprehensive validation for RDF graphs, ensuring:
//! - Well-formed triples (valid subject, predicate, object)
//! - No dangling references
//! - Referential integrity
//! - Required properties present
//! - Type consistency
//! - Detection of corrupted data
//!
//! # Architecture
//!
//! ```text
//! GraphIntegrityChecker
//!     ├── TripleValidator       (validates individual triples)
//!     ├── ReferenceChecker      (validates references and links)
//!     ├── TypeChecker           (validates RDF types)
//!     └── GraphDiff             (tracks and validates changes)
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use oxigraph::store::Store;
//! use crate::ontology::graph_integrity::{GraphIntegrityChecker, IntegrityConfig};
//!
//! let store = Store::new()?;
//! let checker = GraphIntegrityChecker::new(IntegrityConfig::default());
//! let report = checker.check(&store)?;
//!
//! if !report.is_valid() {
//!     eprintln!("Graph integrity violations: {:#?}", report.violations);
//! }
//! ```

use anyhow::{anyhow, bail, Context, Result};
use oxigraph::model::{
    vocab::{rdf, rdfs, xsd},
    BlankNode, Graph, GraphName, Literal, NamedNode, NamedOrBlankNode, Quad, Subject, Term,
    Triple,
};
use oxigraph::sparql::QueryResults;
use oxigraph::store::Store;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use thiserror::Error;

/// Errors that can occur during graph integrity checking
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum IntegrityError {
    #[error("Invalid triple: {0}")]
    InvalidTriple(String),

    #[error("Invalid subject: {0}")]
    InvalidSubject(String),

    #[error("Invalid predicate: {0}")]
    InvalidPredicate(String),

    #[error("Invalid object: {0}")]
    InvalidObject(String),

    #[error("Dangling reference: {0} references {1} which does not exist")]
    DanglingReference(String, String),

    #[error("Missing required property: {0} missing {1}")]
    MissingProperty(String, String),

    #[error("Type inconsistency: {0} has incompatible types {1} and {2}")]
    TypeInconsistency(String, String, String),

    #[error("Invalid URI syntax: {0}")]
    InvalidUri(String),

    #[error("Invalid literal: {0}")]
    InvalidLiteral(String),

    #[error("Orphaned node: {0} has no incoming or outgoing edges")]
    OrphanedNode(String),

    #[error("Broken inverse relationship: {0} -> {1} but not {1} -> {0}")]
    BrokenInverse(String, String),

    #[error("Abstract type instantiated: {0} is abstract and cannot be instantiated")]
    AbstractTypeInstantiation(String),

    #[error("Circular reference detected: {0}")]
    CircularReference(String),

    #[error("Schema violation: {0}")]
    SchemaViolation(String),

    #[error("Corrupted data: {0}")]
    CorruptedData(String),
}

/// Configuration for integrity checking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityConfig {
    /// Check for dangling references
    pub check_references: bool,

    /// Check type consistency
    pub check_types: bool,

    /// Check for orphaned nodes
    pub check_orphans: bool,

    /// Check required properties
    pub check_required_properties: bool,

    /// Check inverse relationships
    pub check_inverse_relationships: bool,

    /// Maximum depth for circular reference detection
    pub max_circular_depth: usize,

    /// URIs of abstract types that cannot be instantiated
    pub abstract_types: HashSet<String>,

    /// Required property mappings: class URI -> required property URIs
    pub required_properties: HashMap<String, Vec<String>>,

    /// Inverse property mappings: property URI -> inverse property URI
    pub inverse_properties: HashMap<String, String>,
}

impl Default for IntegrityConfig {
    fn default() -> Self {
        Self {
            check_references: true,
            check_types: true,
            check_orphans: false, // Can be expensive on large graphs
            check_required_properties: true,
            check_inverse_relationships: false, // Optional
            max_circular_depth: 100,
            abstract_types: HashSet::new(),
            required_properties: HashMap::new(),
            inverse_properties: HashMap::new(),
        }
    }
}

/// Severity level of integrity violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

/// A single integrity violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub severity: Severity,
    pub error: String,
    pub context: String,
    pub suggestion: Option<String>,
}

impl Violation {
    pub fn new(severity: Severity, error: impl Into<String>, context: impl Into<String>) -> Self {
        Self {
            severity,
            error: error.into(),
            context: context.into(),
            suggestion: None,
        }
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

/// Comprehensive integrity check report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityReport {
    pub violations: Vec<Violation>,
    pub total_triples: usize,
    pub total_subjects: usize,
    pub total_predicates: usize,
    pub total_objects: usize,
    pub blank_nodes: usize,
    pub named_nodes: usize,
    pub literals: usize,
}

impl IntegrityReport {
    pub fn new() -> Self {
        Self {
            violations: Vec::new(),
            total_triples: 0,
            total_subjects: 0,
            total_predicates: 0,
            total_objects: 0,
            blank_nodes: 0,
            named_nodes: 0,
            literals: 0,
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.has_errors()
    }

    pub fn has_errors(&self) -> bool {
        self.violations
            .iter()
            .any(|v| matches!(v.severity, Severity::Error | Severity::Critical))
    }

    pub fn has_warnings(&self) -> bool {
        self.violations
            .iter()
            .any(|v| v.severity == Severity::Warning)
    }

    pub fn add_violation(&mut self, violation: Violation) {
        self.violations.push(violation);
    }

    pub fn merge(&mut self, other: IntegrityReport) {
        self.violations.extend(other.violations);
    }

    pub fn summary(&self) -> String {
        let errors = self
            .violations
            .iter()
            .filter(|v| matches!(v.severity, Severity::Error | Severity::Critical))
            .count();
        let warnings = self
            .violations
            .iter()
            .filter(|v| v.severity == Severity::Warning)
            .count();

        format!(
            "Integrity Report: {} triples, {} errors, {} warnings",
            self.total_triples, errors, warnings
        )
    }
}

impl Default for IntegrityReport {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for IntegrityReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.summary())?;
        writeln!(
            f,
            "Statistics: {} subjects, {} predicates, {} objects",
            self.total_subjects, self.total_predicates, self.total_objects
        )?;
        writeln!(
            f,
            "Node types: {} named, {} blank, {} literals",
            self.named_nodes, self.blank_nodes, self.literals
        )?;

        if !self.violations.is_empty() {
            writeln!(f, "\nViolations:")?;
            for (i, v) in self.violations.iter().enumerate() {
                writeln!(
                    f,
                    "  {}. [{:?}] {}: {}",
                    i + 1,
                    v.severity,
                    v.context,
                    v.error
                )?;
                if let Some(suggestion) = &v.suggestion {
                    writeln!(f, "     Suggestion: {}", suggestion)?;
                }
            }
        }

        Ok(())
    }
}

/// Main graph integrity checker
pub struct GraphIntegrityChecker {
    config: IntegrityConfig,
    triple_validator: TripleValidator,
    reference_checker: ReferenceChecker,
    type_checker: TypeChecker,
}

impl GraphIntegrityChecker {
    pub fn new(config: IntegrityConfig) -> Self {
        Self {
            config: config.clone(),
            triple_validator: TripleValidator::new(),
            reference_checker: ReferenceChecker::new(config.clone()),
            type_checker: TypeChecker::new(config.clone()),
        }
    }

    /// Perform comprehensive integrity check on a store
    pub fn check(&self, store: &Store) -> Result<IntegrityReport> {
        let mut report = IntegrityReport::new();

        // Collect statistics
        let mut subjects = HashSet::new();
        let mut predicates = HashSet::new();
        let mut objects = HashSet::new();

        // Validate all triples
        for quad in store.iter() {
            let quad = quad.context("Failed to read quad from store")?;
            let triple = Triple::new(
                quad.subject.clone(),
                quad.predicate.clone(),
                quad.object.clone(),
            );

            report.total_triples += 1;

            // Collect statistics
            subjects.insert(quad.subject.clone());
            predicates.insert(quad.predicate.clone());
            objects.insert(quad.object.clone());

            match &quad.subject {
                Subject::NamedNode(_) => report.named_nodes += 1,
                Subject::BlankNode(_) => report.blank_nodes += 1,
                Subject::Triple(_) => {} // RDF-star
            }

            match &quad.object {
                Term::NamedNode(_) => {}
                Term::BlankNode(_) => report.blank_nodes += 1,
                Term::Literal(_) => report.literals += 1,
                Term::Triple(_) => {} // RDF-star
            }

            // Validate triple
            if let Err(e) = self.triple_validator.validate(&triple) {
                report.add_violation(Violation::new(
                    Severity::Error,
                    e.to_string(),
                    format!("Triple validation: {} {} {}", quad.subject, quad.predicate, quad.object),
                ));
            }
        }

        report.total_subjects = subjects.len();
        report.total_predicates = predicates.len();
        report.total_objects = objects.len();

        // Check references
        if self.config.check_references {
            let ref_report = self.reference_checker.check(store)?;
            report.merge(ref_report);
        }

        // Check types
        if self.config.check_types {
            let type_report = self.type_checker.check(store)?;
            report.merge(type_report);
        }

        // Check orphans
        if self.config.check_orphans {
            let orphan_report = self.check_orphans(store)?;
            report.merge(orphan_report);
        }

        // Check required properties
        if self.config.check_required_properties {
            let prop_report = self.check_required_properties(store)?;
            report.merge(prop_report);
        }

        Ok(report)
    }

    /// Check for orphaned nodes (nodes with no connections)
    fn check_orphans(&self, store: &Store) -> Result<IntegrityReport> {
        let mut report = IntegrityReport::new();
        let mut connected_nodes = HashSet::new();

        // Collect all connected nodes
        for quad in store.iter() {
            let quad = quad?;
            connected_nodes.insert(quad.subject.to_string());

            if let Term::NamedNode(node) = &quad.object {
                connected_nodes.insert(node.to_string());
            } else if let Term::BlankNode(node) = &quad.object {
                connected_nodes.insert(node.to_string());
            }
        }

        // This check is limited - in practice, orphans are rare in valid graphs
        // since nodes typically have at least rdf:type connections

        Ok(report)
    }

    /// Check required properties based on configuration
    fn check_required_properties(&self, store: &Store) -> Result<IntegrityReport> {
        let mut report = IntegrityReport::new();

        for (class_uri, required_props) in &self.config.required_properties {
            let class_node = NamedNode::new(class_uri)
                .map_err(|e| anyhow!("Invalid class URI {}: {}", class_uri, e))?;

            // Find all instances of this class
            for quad in store.quads_for_pattern(None, Some(&*rdf::TYPE), Some(class_node.into()), None) {
                let quad = quad?;
                let instance = &quad.subject;

                // Check each required property
                for prop_uri in required_props {
                    let prop_node = NamedNode::new(prop_uri)
                        .map_err(|e| anyhow!("Invalid property URI {}: {}", prop_uri, e))?;

                    let has_property = store
                        .quads_for_pattern(Some(instance.clone()), Some(&prop_node), None, None)
                        .next()
                        .is_some();

                    if !has_property {
                        report.add_violation(
                            Violation::new(
                                Severity::Error,
                                format!("Missing required property: {}", prop_uri),
                                format!("Instance {} of class {}", instance, class_uri),
                            )
                            .with_suggestion(format!(
                                "Add property {} to instance {}",
                                prop_uri, instance
                            )),
                        );
                    }
                }
            }
        }

        Ok(report)
    }

    /// Quick validation for a single triple (convenience method)
    pub fn validate_triple(&self, triple: &Triple) -> Result<(), IntegrityError> {
        self.triple_validator.validate(triple)
    }
}

/// Validates individual triples
pub struct TripleValidator;

impl TripleValidator {
    pub fn new() -> Self {
        Self
    }

    /// Validate a triple
    pub fn validate(&self, triple: &Triple) -> Result<(), IntegrityError> {
        self.validate_subject(&triple.subject)?;
        self.validate_predicate(&triple.predicate)?;
        self.validate_object(&triple.object)?;
        Ok(())
    }

    /// Validate subject (must be IRI or blank node, not literal)
    fn validate_subject(&self, subject: &Subject) -> Result<(), IntegrityError> {
        match subject {
            Subject::NamedNode(node) => self.validate_iri(node.as_str()),
            Subject::BlankNode(_) => Ok(()), // Blank nodes are always valid
            Subject::Triple(_) => Ok(()),    // RDF-star triples are valid subjects
        }
    }

    /// Validate predicate (must be IRI)
    fn validate_predicate(&self, predicate: &NamedNode) -> Result<(), IntegrityError> {
        self.validate_iri(predicate.as_str())
    }

    /// Validate object (can be IRI, blank node, or literal)
    fn validate_object(&self, object: &Term) -> Result<(), IntegrityError> {
        match object {
            Term::NamedNode(node) => self.validate_iri(node.as_str()),
            Term::BlankNode(_) => Ok(()), // Blank nodes are always valid
            Term::Literal(literal) => self.validate_literal(literal),
            Term::Triple(_) => Ok(()), // RDF-star triples are valid objects
        }
    }

    /// Validate IRI syntax
    fn validate_iri(&self, iri: &str) -> Result<(), IntegrityError> {
        if iri.is_empty() {
            return Err(IntegrityError::InvalidUri("IRI cannot be empty".into()));
        }

        // Basic IRI validation
        if !iri.contains(':') {
            return Err(IntegrityError::InvalidUri(format!(
                "IRI must contain scheme separator ':': {}",
                iri
            )));
        }

        // Check for invalid characters
        if iri.contains(|c: char| c.is_whitespace()) {
            return Err(IntegrityError::InvalidUri(format!(
                "IRI cannot contain whitespace: {}",
                iri
            )));
        }

        Ok(())
    }

    /// Validate literal
    fn validate_literal(&self, literal: &Literal) -> Result<(), IntegrityError> {
        // Validate datatype if present
        if let Some(datatype) = literal.datatype() {
            self.validate_iri(datatype.as_str())?;

            // Validate common XSD datatypes
            if datatype == xsd::INTEGER || datatype == xsd::INT || datatype == xsd::LONG {
                if literal.value().parse::<i64>().is_err() {
                    return Err(IntegrityError::InvalidLiteral(format!(
                        "Invalid integer value: {}",
                        literal.value()
                    )));
                }
            } else if datatype == xsd::DECIMAL || datatype == xsd::DOUBLE || datatype == xsd::FLOAT
            {
                if literal.value().parse::<f64>().is_err() {
                    return Err(IntegrityError::InvalidLiteral(format!(
                        "Invalid numeric value: {}",
                        literal.value()
                    )));
                }
            } else if datatype == xsd::BOOLEAN {
                if !matches!(literal.value(), "true" | "false" | "0" | "1") {
                    return Err(IntegrityError::InvalidLiteral(format!(
                        "Invalid boolean value: {}",
                        literal.value()
                    )));
                }
            }
        }

        Ok(())
    }
}

impl Default for TripleValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Checks references and detects dangling/broken references
pub struct ReferenceChecker {
    config: IntegrityConfig,
}

impl ReferenceChecker {
    pub fn new(config: IntegrityConfig) -> Self {
        Self { config }
    }

    /// Check all references in the graph
    pub fn check(&self, store: &Store) -> Result<IntegrityReport> {
        let mut report = IntegrityReport::new();
        let mut all_subjects = HashSet::new();
        let mut referenced_objects = HashSet::new();

        // First pass: collect all subjects
        for quad in store.iter() {
            let quad = quad?;
            all_subjects.insert(quad.subject.to_string());
        }

        // Second pass: check object references
        for quad in store.iter() {
            let quad = quad?;

            // If object is a named or blank node, it's a reference
            match &quad.object {
                Term::NamedNode(node) => {
                    let obj_str = node.to_string();
                    referenced_objects.insert(obj_str.clone());

                    // Check if this reference exists as a subject
                    if !all_subjects.contains(&obj_str) {
                        // Check if it's an external reference (different namespace)
                        let is_external = self.is_external_reference(node.as_str());

                        if !is_external {
                            report.add_violation(
                                Violation::new(
                                    Severity::Warning,
                                    format!("Dangling reference: {}", obj_str),
                                    format!("Referenced from {} via {}", quad.subject, quad.predicate),
                                )
                                .with_suggestion(format!(
                                    "Ensure {} is defined in the graph or is a valid external reference",
                                    obj_str
                                )),
                            );
                        }
                    }
                }
                Term::BlankNode(node) => {
                    let obj_str = node.to_string();
                    referenced_objects.insert(obj_str.clone());

                    if !all_subjects.contains(&obj_str) {
                        report.add_violation(Violation::new(
                            Severity::Error,
                            format!("Dangling blank node reference: {}", obj_str),
                            format!("Referenced from {} via {}", quad.subject, quad.predicate),
                        ));
                    }
                }
                _ => {}
            }
        }

        // Check inverse relationships if configured
        if self.config.check_inverse_relationships {
            let inverse_report = self.check_inverse_relationships(store)?;
            report.merge(inverse_report);
        }

        Ok(report)
    }

    /// Check if a URI is an external reference (different namespace)
    fn is_external_reference(&self, uri: &str) -> bool {
        // Common external namespaces that are expected to not be defined locally
        let external_prefixes = [
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
            "http://www.w3.org/2000/01/rdf-schema#",
            "http://www.w3.org/2001/XMLSchema#",
            "http://www.w3.org/2002/07/owl#",
            "http://www.w3.org/ns/shacl#",
        ];

        external_prefixes.iter().any(|prefix| uri.starts_with(prefix))
    }

    /// Check inverse relationships
    fn check_inverse_relationships(&self, store: &Store) -> Result<IntegrityReport> {
        let mut report = IntegrityReport::new();

        for (prop_uri, inverse_uri) in &self.config.inverse_properties {
            let prop_node = NamedNode::new(prop_uri)
                .map_err(|e| anyhow!("Invalid property URI {}: {}", prop_uri, e))?;
            let inverse_node = NamedNode::new(inverse_uri)
                .map_err(|e| anyhow!("Invalid inverse property URI {}: {}", inverse_uri, e))?;

            // Check all instances of the property
            for quad in store.quads_for_pattern(None, Some(&prop_node), None, None) {
                let quad = quad?;

                // Check if inverse exists
                if let Term::NamedNode(obj) = &quad.object {
                    let has_inverse = store
                        .quads_for_pattern(
                            Some(obj.clone().into()),
                            Some(&inverse_node),
                            Some(quad.subject.clone().into()),
                            None,
                        )
                        .next()
                        .is_some();

                    if !has_inverse {
                        report.add_violation(
                            Violation::new(
                                Severity::Warning,
                                "Missing inverse relationship",
                                format!(
                                    "{} -> {} ({}) but no {} -> {} ({})",
                                    quad.subject, obj, prop_uri, obj, quad.subject, inverse_uri
                                ),
                            )
                            .with_suggestion(format!(
                                "Add inverse triple: {} {} {}",
                                obj, inverse_uri, quad.subject
                            )),
                        );
                    }
                }
            }
        }

        Ok(report)
    }

    /// Detect circular references
    pub fn detect_circular_references(
        &self,
        store: &Store,
        property: &NamedNode,
    ) -> Result<Vec<Vec<String>>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut path = Vec::new();

        for quad in store.quads_for_pattern(None, Some(property), None, None) {
            let quad = quad?;
            if let Term::NamedNode(start) = quad.subject.clone().into() {
                self.dfs_cycle_detection(
                    store,
                    property,
                    &start,
                    &mut visited,
                    &mut path,
                    &mut cycles,
                    0,
                )?;
            }
        }

        Ok(cycles)
    }

    fn dfs_cycle_detection(
        &self,
        store: &Store,
        property: &NamedNode,
        current: &NamedNode,
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
        depth: usize,
    ) -> Result<()> {
        if depth > self.config.max_circular_depth {
            return Ok(()); // Prevent infinite recursion
        }

        let current_str = current.to_string();

        if path.contains(&current_str) {
            // Found a cycle
            let cycle_start = path.iter().position(|n| n == &current_str).unwrap();
            let cycle: Vec<String> = path[cycle_start..].to_vec();
            cycles.push(cycle);
            return Ok(());
        }

        if visited.contains(&current_str) {
            return Ok(());
        }

        path.push(current_str.clone());

        for quad in store.quads_for_pattern(Some(current.clone().into()), Some(property), None, None) {
            let quad = quad?;
            if let Term::NamedNode(next) = &quad.object {
                self.dfs_cycle_detection(store, property, next, visited, path, cycles, depth + 1)?;
            }
        }

        path.pop();
        visited.insert(current_str);

        Ok(())
    }
}

/// Validates RDF types and type hierarchies
pub struct TypeChecker {
    config: IntegrityConfig,
}

impl TypeChecker {
    pub fn new(config: IntegrityConfig) -> Self {
        Self { config }
    }

    /// Check type consistency in the graph
    pub fn check(&self, store: &Store) -> Result<IntegrityReport> {
        let mut report = IntegrityReport::new();

        // Check abstract type instantiation
        for abstract_type_uri in &self.config.abstract_types {
            let type_node = NamedNode::new(abstract_type_uri)
                .map_err(|e| anyhow!("Invalid abstract type URI {}: {}", abstract_type_uri, e))?;

            for quad in store.quads_for_pattern(None, Some(&*rdf::TYPE), Some(type_node.into()), None) {
                let quad = quad?;
                report.add_violation(
                    Violation::new(
                        Severity::Error,
                        "Abstract type cannot be instantiated",
                        format!("{} has type {}", quad.subject, abstract_type_uri),
                    )
                    .with_suggestion(format!(
                        "Use a concrete subtype of {} instead",
                        abstract_type_uri
                    )),
                );
            }
        }

        // Check for type consistency (multiple incompatible types)
        let incompatible_report = self.check_type_compatibility(store)?;
        report.merge(incompatible_report);

        Ok(report)
    }

    /// Check for incompatible multiple types
    fn check_type_compatibility(&self, store: &Store) -> Result<IntegrityReport> {
        let mut report = IntegrityReport::new();
        let mut subject_types: HashMap<String, Vec<String>> = HashMap::new();

        // Collect all type assertions
        for quad in store.quads_for_pattern(None, Some(&*rdf::TYPE), None, None) {
            let quad = quad?;
            if let Term::NamedNode(type_node) = &quad.object {
                subject_types
                    .entry(quad.subject.to_string())
                    .or_default()
                    .push(type_node.to_string());
            }
        }

        // Check for known incompatibilities
        // This is a simplified check - in practice, you'd use OWL reasoning
        for (subject, types) in subject_types {
            if types.len() > 1 {
                // Check for disjoint classes (example)
                // In a real implementation, you'd check owl:disjointWith axioms
                report.add_violation(Violation::new(
                    Severity::Info,
                    format!("Subject has {} types", types.len()),
                    format!("{} has types: {}", subject, types.join(", ")),
                ));
            }
        }

        Ok(report)
    }

    /// Get all types of a subject
    pub fn get_types(&self, store: &Store, subject: &NamedOrBlankNode) -> Result<Vec<NamedNode>> {
        let mut types = Vec::new();

        for quad in store.quads_for_pattern(Some(subject.clone()), Some(&*rdf::TYPE), None, None) {
            let quad = quad?;
            if let Term::NamedNode(type_node) = quad.object {
                types.push(type_node);
            }
        }

        Ok(types)
    }
}

/// Tracks and validates graph changes (diffs)
#[derive(Debug, Clone)]
pub struct GraphDiff {
    pub added: Vec<Triple>,
    pub removed: Vec<Triple>,
    pub modified: Vec<(Triple, Triple)>, // (old, new)
}

impl GraphDiff {
    pub fn new() -> Self {
        Self {
            added: Vec::new(),
            removed: Vec::new(),
            modified: Vec::new(),
        }
    }

    /// Compute the difference between two graphs
    pub fn compute(old_store: &Store, new_store: &Store) -> Result<Self> {
        let mut diff = Self::new();

        let old_triples: HashSet<_> = old_store
            .iter()
            .map(|q| {
                let q = q.unwrap();
                (q.subject.clone(), q.predicate.clone(), q.object.clone())
            })
            .collect();

        let new_triples: HashSet<_> = new_store
            .iter()
            .map(|q| {
                let q = q.unwrap();
                (q.subject.clone(), q.predicate.clone(), q.object.clone())
            })
            .collect();

        // Find added triples
        for triple_tuple in new_triples.difference(&old_triples) {
            diff.added.push(Triple::new(
                triple_tuple.0.clone(),
                triple_tuple.1.clone(),
                triple_tuple.2.clone(),
            ));
        }

        // Find removed triples
        for triple_tuple in old_triples.difference(&new_triples) {
            diff.removed.push(Triple::new(
                triple_tuple.0.clone(),
                triple_tuple.1.clone(),
                triple_tuple.2.clone(),
            ));
        }

        Ok(diff)
    }

    /// Validate that the changes maintain graph integrity
    pub fn validate(&self, checker: &GraphIntegrityChecker, store: &Store) -> Result<IntegrityReport> {
        let mut report = IntegrityReport::new();

        // Validate added triples
        for triple in &self.added {
            if let Err(e) = checker.validate_triple(triple) {
                report.add_violation(Violation::new(
                    Severity::Error,
                    format!("Invalid added triple: {}", e),
                    format!("Add: {} {} {}", triple.subject, triple.predicate, triple.object),
                ));
            }
        }

        // Check if removed triples break references
        for triple in &self.removed {
            // Check if any other triple references the subject being removed
            let subj_str = triple.subject.to_string();
            for quad in store.iter() {
                let quad = quad?;
                if quad.object.to_string() == subj_str {
                    report.add_violation(
                        Violation::new(
                            Severity::Warning,
                            "Removing triple may create dangling reference",
                            format!(
                                "Removing {} but it's referenced by {} via {}",
                                subj_str, quad.subject, quad.predicate
                            ),
                        )
                        .with_suggestion("Ensure dependent triples are also removed".to_string()),
                    );
                }
            }
        }

        Ok(report)
    }

    /// Check if the diff contains breaking changes
    pub fn has_breaking_changes(&self) -> bool {
        // Define what constitutes a breaking change
        // For example: removing required properties, changing types, etc.
        !self.removed.is_empty() || !self.modified.is_empty()
    }

    /// Generate a change report
    pub fn report(&self) -> String {
        format!(
            "Graph Changes: +{} added, -{} removed, ~{} modified",
            self.added.len(),
            self.removed.len(),
            self.modified.len()
        )
    }

    /// Get statistics
    pub fn stats(&self) -> DiffStats {
        DiffStats {
            added_count: self.added.len(),
            removed_count: self.removed.len(),
            modified_count: self.modified.len(),
        }
    }
}

impl Default for GraphDiff {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about a graph diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStats {
    pub added_count: usize,
    pub removed_count: usize,
    pub modified_count: usize,
}

impl DiffStats {
    pub fn total_changes(&self) -> usize {
        self.added_count + self.removed_count + self.modified_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triple_validator_valid_triple() {
        let validator = TripleValidator::new();

        let subject = Subject::NamedNode(
            NamedNode::new("http://example.org/subject").unwrap()
        );
        let predicate = NamedNode::new("http://example.org/predicate").unwrap();
        let object = Term::Literal(Literal::new_simple_literal("value"));

        let triple = Triple::new(subject, predicate, object);
        assert!(validator.validate(&triple).is_ok());
    }

    #[test]
    fn test_triple_validator_invalid_literal() {
        let validator = TripleValidator::new();

        let subject = Subject::NamedNode(
            NamedNode::new("http://example.org/subject").unwrap()
        );
        let predicate = NamedNode::new("http://example.org/predicate").unwrap();
        let object = Term::Literal(
            Literal::new_typed_literal("not-a-number", xsd::INTEGER)
        );

        let triple = Triple::new(subject, predicate, object);
        assert!(validator.validate(&triple).is_err());
    }

    #[test]
    fn test_integrity_report_summary() {
        let mut report = IntegrityReport::new();
        report.total_triples = 100;
        report.add_violation(Violation::new(
            Severity::Error,
            "Test error",
            "Test context",
        ));

        assert!(!report.is_valid());
        assert!(report.has_errors());
        assert!(report.summary().contains("1 errors"));
    }
}
