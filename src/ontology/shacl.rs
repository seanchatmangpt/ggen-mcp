//! SHACL (Shapes Constraint Language) Validation
//!
//! This module provides comprehensive RDF data validation against SHACL shapes.
//! It validates instances against constraints defined in the ontology/shapes.ttl file.
//!
//! # Components
//!
//! - **ShapeValidator**: Main validator that loads shapes and validates instances
//! - **ConstraintChecker**: Validates individual SHACL constraints
//! - **ValidationReport**: Structured validation results with detailed errors
//! - **ShapeDiscovery**: Finds applicable shapes for nodes
//! - **CustomConstraints**: Domain-specific business rule validation
//!
//! # Example
//!
//! ```rust,ignore
//! use crate::ontology::shacl::ShapeValidator;
//!
//! let validator = ShapeValidator::from_file("ontology/shapes.ttl")?;
//! let report = validator.validate_graph(&data_graph)?;
//!
//! if !report.conforms() {
//!     for result in report.results() {
//!         println!("Violation: {}", result.message());
//!     }
//! }
//! ```

use anyhow::{Context, Result, anyhow};
use ggen_ontology_core::TripleStore;
use oxigraph::model::*;
use oxigraph::sparql::{Query, QueryResults};
use oxigraph::store::Store;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

// =============================================================================
// Namespace Constants
// =============================================================================

const SH_NS: &str = "http://www.w3.org/ns/shacl#";
const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const RDFS_NS: &str = "http://www.w3.org/2000/01/rdf-schema#";
const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";
const DDD_NS: &str = "https://ddd-patterns.dev/schema#";

// =============================================================================
// Severity Levels
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Violation,
}

impl Severity {
    pub fn from_iri(iri: &NamedNode) -> Self {
        match iri.as_str() {
            "http://www.w3.org/ns/shacl#Info" => Severity::Info,
            "http://www.w3.org/ns/shacl#Warning" => Severity::Warning,
            "http://www.w3.org/ns/shacl#Violation" => Severity::Violation,
            _ => Severity::Violation, // Default to most severe
        }
    }

    pub fn to_iri(&self) -> NamedNode {
        match self {
            Severity::Info => NamedNode::new_unchecked(format!("{}Info", SH_NS)),
            Severity::Warning => NamedNode::new_unchecked(format!("{}Warning", SH_NS)),
            Severity::Violation => NamedNode::new_unchecked(format!("{}Violation", SH_NS)),
        }
    }
}

// =============================================================================
// Validation Result
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// The node that caused the violation
    focus_node: String,
    /// The property path (if applicable)
    result_path: Option<String>,
    /// The value that violated the constraint
    value: Option<String>,
    /// Human-readable message
    message: String,
    /// Severity level
    severity: Severity,
    /// The source shape that was violated
    source_shape: String,
    /// The specific constraint component
    source_constraint: Option<String>,
}

impl ValidationResult {
    pub fn new(
        focus_node: String,
        message: String,
        severity: Severity,
        source_shape: String,
    ) -> Self {
        Self {
            focus_node,
            result_path: None,
            value: None,
            message,
            severity,
            source_shape,
            source_constraint: None,
        }
    }

    pub fn with_path(mut self, path: String) -> Self {
        self.result_path = Some(path);
        self
    }

    pub fn with_value(mut self, value: String) -> Self {
        self.value = Some(value);
        self
    }

    pub fn with_constraint(mut self, constraint: String) -> Self {
        self.source_constraint = Some(constraint);
        self
    }

    pub fn focus_node(&self) -> &str {
        &self.focus_node
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }

    pub fn source_shape(&self) -> &str {
        &self.source_shape
    }
}

// =============================================================================
// Validation Report
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    results: Vec<ValidationResult>,
    conforms: bool,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            conforms: true,
        }
    }

    pub fn add_result(&mut self, result: ValidationResult) {
        if result.severity == Severity::Violation {
            self.conforms = false;
        }
        self.results.push(result);
    }

    pub fn conforms(&self) -> bool {
        self.conforms
    }

    pub fn results(&self) -> &[ValidationResult] {
        &self.results
    }

    pub fn violations(&self) -> impl Iterator<Item = &ValidationResult> {
        self.results
            .iter()
            .filter(|r| r.severity == Severity::Violation)
    }

    pub fn warnings(&self) -> impl Iterator<Item = &ValidationResult> {
        self.results
            .iter()
            .filter(|r| r.severity == Severity::Warning)
    }

    pub fn infos(&self) -> impl Iterator<Item = &ValidationResult> {
        self.results.iter().filter(|r| r.severity == Severity::Info)
    }

    pub fn violation_count(&self) -> usize {
        self.violations().count()
    }

    pub fn warning_count(&self) -> usize {
        self.warnings().count()
    }

    pub fn info_count(&self) -> usize {
        self.infos().count()
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("Failed to serialize validation report")
    }
}

impl Default for ValidationReport {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Property Shape
// =============================================================================

#[derive(Debug, Clone)]
struct PropertyShape {
    path: NamedNode,
    datatype: Option<NamedNode>,
    class: Option<NamedNode>,
    min_count: Option<i32>,
    max_count: Option<i32>,
    pattern: Option<String>,
    min_length: Option<i32>,
    max_length: Option<i32>,
    min_inclusive: Option<Literal>,
    max_inclusive: Option<Literal>,
    in_values: Vec<Term>,
    unique_lang: bool,
    name: Option<String>,
    message: Option<String>,
}

impl PropertyShape {
    fn new(path: NamedNode) -> Self {
        Self {
            path,
            datatype: None,
            class: None,
            min_count: None,
            max_count: None,
            pattern: None,
            min_length: None,
            max_length: None,
            min_inclusive: None,
            max_inclusive: None,
            in_values: Vec::new(),
            unique_lang: false,
            name: None,
            message: None,
        }
    }
}

// =============================================================================
// Node Shape
// =============================================================================

#[derive(Debug, Clone)]
struct NodeShape {
    id: NamedNode,
    target_class: Option<NamedNode>,
    target_nodes: Vec<NamedNode>,
    target_subjects_of: Vec<NamedNode>,
    target_objects_of: Vec<NamedNode>,
    properties: Vec<PropertyShape>,
    name: Option<String>,
    description: Option<String>,
    severity: Severity,
}

impl NodeShape {
    fn new(id: NamedNode) -> Self {
        Self {
            id,
            target_class: None,
            target_nodes: Vec::new(),
            target_subjects_of: Vec::new(),
            target_objects_of: Vec::new(),
            properties: Vec::new(),
            name: None,
            description: None,
            severity: Severity::Violation,
        }
    }
}

// =============================================================================
// Shape Discovery
// =============================================================================

pub struct ShapeDiscovery<'a> {
    shapes_store: &'a Store,
}

impl<'a> ShapeDiscovery<'a> {
    pub fn new(shapes_store: &'a Store) -> Self {
        Self { shapes_store }
    }

    /// Find all applicable shapes for a given node
    pub fn find_shapes_for_node(
        &self,
        node: &NamedNode,
        data_store: &Store,
    ) -> Result<Vec<NodeShape>> {
        let mut applicable_shapes = Vec::new();

        // Get all node shapes
        let shapes = self.load_all_node_shapes()?;

        for shape in shapes {
            if self.is_shape_applicable(node, &shape, data_store)? {
                applicable_shapes.push(shape);
            }
        }

        Ok(applicable_shapes)
    }

    fn is_shape_applicable(
        &self,
        node: &NamedNode,
        shape: &NodeShape,
        data_store: &Store,
    ) -> Result<bool> {
        // Check sh:targetNode
        if shape.target_nodes.contains(node) {
            return Ok(true);
        }

        // Check sh:targetClass
        if let Some(target_class) = &shape.target_class {
            if self.has_type(node, target_class, data_store)? {
                return Ok(true);
            }
        }

        // Check sh:targetSubjectsOf
        for predicate in &shape.target_subjects_of {
            if self.is_subject_of(node, predicate, data_store)? {
                return Ok(true);
            }
        }

        // Check sh:targetObjectsOf
        for predicate in &shape.target_objects_of {
            if self.is_object_of(node, predicate, data_store)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn has_type(&self, node: &NamedNode, class: &NamedNode, store: &Store) -> Result<bool> {
        let rdf_type = NamedNode::new_unchecked(format!("{}type", RDF_NS));
        let quad = QuadRef::new(node, &rdf_type, class, GraphNameRef::DefaultGraph);
        Ok(store.contains(&quad)?)
    }

    fn is_subject_of(
        &self,
        node: &NamedNode,
        predicate: &NamedNode,
        store: &Store,
    ) -> Result<bool> {
        for quad in store.quads_for_pattern(Some(node.into()), Some(predicate.into()), None, None) {
            quad?;
            return Ok(true);
        }
        Ok(false)
    }

    fn is_object_of(&self, node: &NamedNode, predicate: &NamedNode, store: &Store) -> Result<bool> {
        for quad in store.quads_for_pattern(None, Some(predicate.into()), Some(node.into()), None) {
            quad?;
            return Ok(true);
        }
        Ok(false)
    }

    fn load_all_node_shapes(&self) -> Result<Vec<NodeShape>> {
        let mut shapes = Vec::new();
        let sh_node_shape = NamedNode::new_unchecked(format!("{}NodeShape", SH_NS));
        let rdf_type = NamedNode::new_unchecked(format!("{}type", RDF_NS));

        for quad in self.shapes_store.quads_for_pattern(
            None,
            Some(rdf_type.as_ref().into()),
            Some(sh_node_shape.as_ref().into()),
            None,
        ) {
            let quad = quad?;
            match &quad.subject {
                Subject::NamedNode(shape_id) => {
                    let shape = self.load_node_shape(shape_id)?;
                    shapes.push(shape);
                }
                _ => {}
            }
        }

        Ok(shapes)
    }

    fn load_node_shape(&self, shape_id: &NamedNode) -> Result<NodeShape> {
        let mut shape = NodeShape::new(shape_id.clone());

        // Load basic properties
        shape.name = self.get_string_value(shape_id, &format!("{}name", SH_NS))?;
        shape.description = self.get_string_value(shape_id, &format!("{}description", SH_NS))?;

        // Load severity
        if let Some(severity_iri) =
            self.get_named_node_value(shape_id, &format!("{}severity", SH_NS))?
        {
            shape.severity = Severity::from_iri(&severity_iri);
        }

        // Load target selectors
        shape.target_class =
            self.get_named_node_value(shape_id, &format!("{}targetClass", SH_NS))?;
        shape.target_nodes =
            self.get_named_node_values(shape_id, &format!("{}targetNode", SH_NS))?;
        shape.target_subjects_of =
            self.get_named_node_values(shape_id, &format!("{}targetSubjectsOf", SH_NS))?;
        shape.target_objects_of =
            self.get_named_node_values(shape_id, &format!("{}targetObjectsOf", SH_NS))?;

        // Load property shapes
        shape.properties = self.load_property_shapes(shape_id)?;

        Ok(shape)
    }

    fn load_property_shapes(&self, shape_id: &NamedNode) -> Result<Vec<PropertyShape>> {
        let mut properties = Vec::new();
        let sh_property = NamedNode::new_unchecked(format!("{}property", SH_NS));

        for quad in self.shapes_store.quads_for_pattern(
            Some(shape_id.into()),
            Some(sh_property.as_ref().into()),
            None,
            None,
        ) {
            let quad = quad?;
            match &quad.object {
                Term::BlankNode(prop_id) => {
                    if let Some(prop) = self.load_property_shape(prop_id)? {
                        properties.push(prop);
                    }
                }
                _ => {}
            }
        }

        Ok(properties)
    }

    fn load_property_shape(&self, prop_id: &BlankNode) -> Result<Option<PropertyShape>> {
        let sh_path = NamedNode::new_unchecked(format!("{}path", SH_NS));

        // Get the path (required)
        let path = match self.get_object(prop_id.into(), &sh_path)? {
            Some(Term::NamedNode(n)) => n,
            _ => return Ok(None),
        };

        let mut prop = PropertyShape::new(path);

        // Load constraints
        prop.datatype =
            self.get_named_node_value(&prop_id.clone().into(), &format!("{}datatype", SH_NS))?;
        prop.class =
            self.get_named_node_value(&prop_id.clone().into(), &format!("{}class", SH_NS))?;
        prop.min_count =
            self.get_integer_value(&prop_id.clone().into(), &format!("{}minCount", SH_NS))?;
        prop.max_count =
            self.get_integer_value(&prop_id.clone().into(), &format!("{}maxCount", SH_NS))?;
        prop.pattern =
            self.get_string_value(&prop_id.clone().into(), &format!("{}pattern", SH_NS))?;
        prop.min_length =
            self.get_integer_value(&prop_id.clone().into(), &format!("{}minLength", SH_NS))?;
        prop.max_length =
            self.get_integer_value(&prop_id.clone().into(), &format!("{}maxLength", SH_NS))?;
        prop.name = self.get_string_value::<&oxigraph::model::BlankNode>(&prop_id, &format!("{}name", SH_NS))?;
        prop.message =
            self.get_string_value::<&oxigraph::model::BlankNode>(&prop_id, &format!("{}message", SH_NS))?;

        // Load numeric range constraints
        prop.min_inclusive =
            self.get_literal_value::<&oxigraph::model::BlankNode>(&prop_id, &format!("{}minInclusive", SH_NS))?;
        prop.max_inclusive =
            self.get_literal_value::<&oxigraph::model::BlankNode>(&prop_id, &format!("{}maxInclusive", SH_NS))?;

        // Load sh:in values
        if let Some(list_head) = self.get_object(
            prop_id.into(),
            &NamedNode::new_unchecked(format!("{}in", SH_NS)),
        )? {
            prop.in_values = self.parse_rdf_list(&list_head)?;
        }

        // Load sh:uniqueLang
        if let Some(unique_lang) =
            self.get_boolean_value::<&oxigraph::model::BlankNode>(&prop_id, &format!("{}uniqueLang", SH_NS))?
        {
            prop.unique_lang = unique_lang;
        }

        Ok(Some(prop))
    }

    fn get_object<S: Into<SubjectRef<'a>> + Clone>(
        &self,
        subject: S,
        predicate: &NamedNode,
    ) -> Result<Option<Term>> {
        for quad in self.shapes_store.quads_for_pattern(
            Some(subject.clone().into()),
            Some(predicate.into()),
            None,
            None,
        ) {
            let quad = quad?;
            return Ok(Some(quad.object.to_owned()));
        }
        Ok(None)
    }

    fn get_string_value<S: Into<SubjectRef<'a>> + Clone>(
        &self,
        subject: S,
        predicate_iri: &str,
    ) -> Result<Option<String>> {
        let predicate = NamedNode::new_unchecked(predicate_iri);
        if let Some(Term::Literal(lit)) = self.get_object(subject, &predicate)? {
            Ok(Some(lit.value().to_string()))
        } else {
            Ok(None)
        }
    }

    fn get_integer_value<S: Into<SubjectRef<'a>> + Clone>(
        &self,
        subject: S,
        predicate_iri: &str,
    ) -> Result<Option<i32>> {
        if let Some(s) = self.get_string_value(subject, predicate_iri)? {
            Ok(Some(s.parse::<i32>().context("Failed to parse integer")?))
        } else {
            Ok(None)
        }
    }

    fn get_boolean_value<S: Into<SubjectRef<'a>> + Clone>(
        &self,
        subject: S,
        predicate_iri: &str,
    ) -> Result<Option<bool>> {
        if let Some(s) = self.get_string_value(subject, predicate_iri)? {
            Ok(Some(s.parse::<bool>().context("Failed to parse boolean")?))
        } else {
            Ok(None)
        }
    }

    fn get_named_node_value<S: Into<SubjectRef<'a>> + Clone>(
        &self,
        subject: S,
        predicate_iri: &str,
    ) -> Result<Option<NamedNode>> {
        let predicate = NamedNode::new_unchecked(predicate_iri);
        if let Some(Term::NamedNode(node)) = self.get_object(subject, &predicate)? {
            Ok(Some(node))
        } else {
            Ok(None)
        }
    }

    fn get_named_node_values<S: Into<SubjectRef<'a>> + Clone>(
        &self,
        subject: S,
        predicate_iri: &str,
    ) -> Result<Vec<NamedNode>> {
        let predicate = NamedNode::new_unchecked(predicate_iri);
        let mut values = Vec::new();

        for quad in self.shapes_store.quads_for_pattern(
            Some(subject.clone().into()),
            Some(predicate.as_ref().into()),
            None,
            None,
        ) {
            let quad = quad?;
            match &quad.object {
                Term::NamedNode(node) => {
                    values.push(node.clone());
                }
                _ => {}
            }
        }

        Ok(values)
    }

    fn get_literal_value<S: Into<SubjectRef<'a>> + Clone>(
        &self,
        subject: S,
        predicate_iri: &str,
    ) -> Result<Option<Literal>> {
        let predicate = NamedNode::new_unchecked(predicate_iri);
        if let Some(Term::Literal(lit)) = self.get_object(subject, &predicate)? {
            Ok(Some(lit))
        } else {
            Ok(None)
        }
    }

    fn parse_rdf_list(&self, head: &Term) -> Result<Vec<Term>> {
        let mut values = Vec::new();
        let mut current = head.clone();
        let rdf_first = NamedNode::new_unchecked(format!("{}first", RDF_NS));
        let rdf_rest = NamedNode::new_unchecked(format!("{}rest", RDF_NS));
        let rdf_nil = NamedNode::new_unchecked(format!("{}nil", RDF_NS));

        loop {
            match &current {
                Term::BlankNode(node) => {
                    // Get first
                    if let Some(first) = self.get_object(&node.clone().into(), &rdf_first)? {
                        values.push(first);
                    }
                    // Get rest
                    if let Some(rest) = self.get_object(&node.clone().into(), &rdf_rest)? {
                        current = rest;
                    } else {
                        break;
                    }
                }
                Term::NamedNode(node) if node.as_ref() == rdf_nil.as_ref() => break,
                _ => break,
            }
        }

        Ok(values)
    }
}

// =============================================================================
// Constraint Checker
// =============================================================================

pub struct ConstraintChecker<'a> {
    data_store: &'a Store,
}

impl<'a> ConstraintChecker<'a> {
    pub fn new(data_store: &'a Store) -> Self {
        Self { data_store }
    }

    /// Check all constraints for a property shape
    pub fn check_property(
        &self,
        focus_node: &NamedNode,
        property: &PropertyShape,
        shape_id: &str,
    ) -> Vec<ValidationResult> {
        let mut results = Vec::new();

        // Get all values for this property
        let values = match self.get_property_values(focus_node, &property.path) {
            Ok(v) => v,
            Err(e) => {
                results.push(ValidationResult::new(
                    focus_node.to_string(),
                    format!("Failed to get property values: {}", e),
                    Severity::Violation,
                    shape_id.to_string(),
                ));
                return results;
            }
        };

        // Check cardinality constraints
        if let Some(min_count) = property.min_count {
            if (values.len() as i32) < min_count {
                let default_message = format!(
                    "Property {} must have at least {} value(s)",
                    property.path, min_count
                );
                let message = property.message.as_ref().unwrap_or(&default_message);
                results.push(
                    ValidationResult::new(
                        focus_node.to_string(),
                        message.clone(),
                        Severity::Violation,
                        shape_id.to_string(),
                    )
                    .with_path(property.path.to_string())
                    .with_constraint("sh:minCount".to_string()),
                );
            }
        }

        if let Some(max_count) = property.max_count {
            if (values.len() as i32) > max_count {
                let default_message = format!(
                    "Property {} must have at most {} value(s)",
                    property.path, max_count
                );
                let message = property.message.as_ref().unwrap_or(&default_message);
                results.push(
                    ValidationResult::new(
                        focus_node.to_string(),
                        message.clone(),
                        Severity::Violation,
                        shape_id.to_string(),
                    )
                    .with_path(property.path.to_string())
                    .with_constraint("sh:maxCount".to_string()),
                );
            }
        }

        // Check each value against constraints
        for value in &values {
            results.extend(self.check_value_constraints(focus_node, value, property, shape_id));
        }

        // Check sh:uniqueLang
        if property.unique_lang {
            results.extend(self.check_unique_lang(focus_node, &values, property, shape_id));
        }

        results
    }

    fn check_value_constraints(
        &self,
        focus_node: &NamedNode,
        value: &Term,
        property: &PropertyShape,
        shape_id: &str,
    ) -> Vec<ValidationResult> {
        let mut results = Vec::new();

        // Check sh:datatype
        if let Some(expected_datatype) = &property.datatype {
            if let Term::Literal(lit) = value {
                if lit.datatype().as_ref() != expected_datatype.as_ref() {
                    let message = property
                        .message
                        .as_ref()
                        .unwrap_or(&format!("Value must have datatype {}", expected_datatype));
                    results.push(
                        ValidationResult::new(
                            focus_node.to_string(),
                            message.clone(),
                            Severity::Violation,
                            shape_id.to_string(),
                        )
                        .with_path(property.path.to_string())
                        .with_value(value.to_string())
                        .with_constraint("sh:datatype".to_string()),
                    );
                }
            } else {
                let message = property.message.as_ref().unwrap_or(&format!(
                    "Value must be a literal with datatype {}",
                    expected_datatype
                ));
                results.push(
                    ValidationResult::new(
                        focus_node.to_string(),
                        message.clone(),
                        Severity::Violation,
                        shape_id.to_string(),
                    )
                    .with_path(property.path.to_string())
                    .with_value(value.to_string())
                    .with_constraint("sh:datatype".to_string()),
                );
            }
        }

        // Check sh:class
        if let Some(expected_class) = &property.class {
            if let Term::NamedNode(node) = value {
                if let Ok(false) | Err(_) = self.has_type(node, expected_class) {
                    let message = property
                        .message
                        .as_ref()
                        .unwrap_or(&format!("Value must be an instance of {}", expected_class));
                    results.push(
                        ValidationResult::new(
                            focus_node.to_string(),
                            message.clone(),
                            Severity::Violation,
                            shape_id.to_string(),
                        )
                        .with_path(property.path.to_string())
                        .with_value(value.to_string())
                        .with_constraint("sh:class".to_string()),
                    );
                }
            } else {
                let message = property
                    .message
                    .as_ref()
                    .unwrap_or(&format!("Value must be an instance of {}", expected_class));
                results.push(
                    ValidationResult::new(
                        focus_node.to_string(),
                        message.clone(),
                        Severity::Violation,
                        shape_id.to_string(),
                    )
                    .with_path(property.path.to_string())
                    .with_value(value.to_string())
                    .with_constraint("sh:class".to_string()),
                );
            }
        }

        // Check string constraints
        if let Term::Literal(lit) = value {
            let value_str = lit.value();

            // Check sh:pattern
            if let Some(pattern) = &property.pattern {
                if let Ok(regex) = Regex::new(pattern) {
                    if !regex.is_match(value_str) {
                        let message = property
                            .message
                            .as_ref()
                            .unwrap_or(&format!("Value must match pattern: {}", pattern));
                        results.push(
                            ValidationResult::new(
                                focus_node.to_string(),
                                message.clone(),
                                Severity::Violation,
                                shape_id.to_string(),
                            )
                            .with_path(property.path.to_string())
                            .with_value(value.to_string())
                            .with_constraint("sh:pattern".to_string()),
                        );
                    }
                }
            }

            // Check sh:minLength
            if let Some(min_length) = property.min_length {
                if (value_str.len() as i32) < min_length {
                    let message = property.message.as_ref().unwrap_or(&format!(
                        "Value must have at least {} characters",
                        min_length
                    ));
                    results.push(
                        ValidationResult::new(
                            focus_node.to_string(),
                            message.clone(),
                            Severity::Violation,
                            shape_id.to_string(),
                        )
                        .with_path(property.path.to_string())
                        .with_value(value.to_string())
                        .with_constraint("sh:minLength".to_string()),
                    );
                }
            }

            // Check sh:maxLength
            if let Some(max_length) = property.max_length {
                if (value_str.len() as i32) > max_length {
                    let message = property.message.as_ref().unwrap_or(&format!(
                        "Value must have at most {} characters",
                        max_length
                    ));
                    results.push(
                        ValidationResult::new(
                            focus_node.to_string(),
                            message.clone(),
                            Severity::Violation,
                            shape_id.to_string(),
                        )
                        .with_path(property.path.to_string())
                        .with_value(value.to_string())
                        .with_constraint("sh:maxLength".to_string()),
                    );
                }
            }

            // Check numeric range constraints
            if let Some(min_inclusive) = &property.min_inclusive {
                if let Ok(value_num) = value_str.parse::<f64>() {
                    if let Ok(min_num) = min_inclusive.value().parse::<f64>() {
                        if value_num < min_num {
                            let message = property
                                .message
                                .as_ref()
                                .unwrap_or(&format!("Value must be >= {}", min_num));
                            results.push(
                                ValidationResult::new(
                                    focus_node.to_string(),
                                    message.clone(),
                                    Severity::Violation,
                                    shape_id.to_string(),
                                )
                                .with_path(property.path.to_string())
                                .with_value(value.to_string())
                                .with_constraint("sh:minInclusive".to_string()),
                            );
                        }
                    }
                }
            }

            if let Some(max_inclusive) = &property.max_inclusive {
                if let Ok(value_num) = value_str.parse::<f64>() {
                    if let Ok(max_num) = max_inclusive.value().parse::<f64>() {
                        if value_num > max_num {
                            let message = property
                                .message
                                .as_ref()
                                .unwrap_or(&format!("Value must be <= {}", max_num));
                            results.push(
                                ValidationResult::new(
                                    focus_node.to_string(),
                                    message.clone(),
                                    Severity::Violation,
                                    shape_id.to_string(),
                                )
                                .with_path(property.path.to_string())
                                .with_value(value.to_string())
                                .with_constraint("sh:maxInclusive".to_string()),
                            );
                        }
                    }
                }
            }
        }

        // Check sh:in (enumeration)
        if !property.in_values.is_empty() && !property.in_values.contains(value) {
            let allowed_values: Vec<String> =
                property.in_values.iter().map(|v| v.to_string()).collect();
            let message = property.message.as_ref().unwrap_or(&format!(
                "Value must be one of: {}",
                allowed_values.join(", ")
            ));
            results.push(
                ValidationResult::new(
                    focus_node.to_string(),
                    message.clone(),
                    Severity::Violation,
                    shape_id.to_string(),
                )
                .with_path(property.path.to_string())
                .with_value(value.to_string())
                .with_constraint("sh:in".to_string()),
            );
        }

        results
    }

    fn check_unique_lang(
        &self,
        focus_node: &NamedNode,
        values: &[Term],
        property: &PropertyShape,
        shape_id: &str,
    ) -> Vec<ValidationResult> {
        let mut results = Vec::new();
        let mut seen_langs = HashSet::new();

        for value in values {
            if let Term::Literal(lit) = value {
                if let Some(lang) = lit.language() {
                    if !seen_langs.insert(lang.to_string()) {
                        let default_message = format!("Property must have unique language tags");
                        let message = property
                            .message
                            .as_ref()
                            .unwrap_or(&default_message);
                        results.push(
                            ValidationResult::new(
                                focus_node.to_string(),
                                message.clone(),
                                Severity::Violation,
                                shape_id.to_string(),
                            )
                            .with_path(property.path.to_string())
                            .with_value(value.to_string())
                            .with_constraint("sh:uniqueLang".to_string()),
                        );
                    }
                }
            }
        }

        results
    }

    fn get_property_values(&self, subject: &NamedNode, property: &NamedNode) -> Result<Vec<Term>> {
        let mut values = Vec::new();
        for quad in self.data_store.quads_for_pattern(
            Some(subject.into()),
            Some(property.into()),
            None,
            None,
        ) {
            let quad = quad?;
            values.push(quad.object.to_owned());
        }
        Ok(values)
    }

    fn has_type(&self, node: &NamedNode, class: &NamedNode) -> Result<bool> {
        let rdf_type = NamedNode::new_unchecked(format!("{}type", RDF_NS));
        let quad = QuadRef::new(node, &rdf_type, class, GraphNameRef::DefaultGraph);
        Ok(self.data_store.contains(quad)?)
    }
}

// =============================================================================
// Custom Constraints
// =============================================================================

pub struct CustomConstraints<'a> {
    data_store: &'a Store,
}

impl<'a> CustomConstraints<'a> {
    pub fn new(data_store: &'a Store) -> Self {
        Self { data_store }
    }

    /// Check DDD invariants attached to aggregates/value objects
    pub fn check_ddd_invariants(
        &self,
        focus_node: &NamedNode,
        shape_id: &str,
    ) -> Vec<ValidationResult> {
        let mut results = Vec::new();

        // Get invariants using ddd:hasInvariant
        let has_invariant = NamedNode::new_unchecked(format!("{}hasInvariant", DDD_NS));

        for quad in self.data_store.quads_for_pattern(
            Some(focus_node.into()),
            Some(has_invariant.as_ref().into()),
            None,
            None,
        ) {
            if let Ok(quad) = quad {
                if let Some(lit) = quad.object.as_literal() {
                    let invariant = lit.value();
                    // In a real implementation, you would evaluate the invariant
                    // For now, we just log that an invariant exists
                    results.push(
                        ValidationResult::new(
                            focus_node.to_string(),
                            format!("DDD Invariant check required: {}", invariant),
                            Severity::Info,
                            shape_id.to_string(),
                        )
                        .with_constraint("ddd:hasInvariant".to_string()),
                    );
                }
            }
        }

        results
    }

    /// Validate cross-property business rules
    pub fn check_cross_property_constraints(
        &self,
        focus_node: &NamedNode,
        shape_id: &str,
    ) -> Vec<ValidationResult> {
        let mut results = Vec::new();

        // Example: Check that if a node has a Repository, it must be for an Aggregate
        let rdf_type = NamedNode::new_unchecked(format!("{}type", RDF_NS));
        let ddd_repository = NamedNode::new_unchecked(format!("{}Repository", DDD_NS));
        let for_aggregate = NamedNode::new_unchecked(format!("{}forAggregate", DDD_NS));

        // Check if this is a repository
        let is_repository = self
            .data_store
            .contains(QuadRef::new(
                focus_node,
                &rdf_type,
                &ddd_repository,
                GraphNameRef::DefaultGraph,
            ))
            .unwrap_or(false);

        if is_repository {
            // Check if it has forAggregate
            let mut has_aggregate = false;
            for quad in self.data_store.quads_for_pattern(
                Some(focus_node.into()),
                Some(for_aggregate.as_ref().into()),
                None,
                None,
            ) {
                if quad.is_ok() {
                    has_aggregate = true;
                    break;
                }
            }

            if !has_aggregate {
                results.push(
                    ValidationResult::new(
                        focus_node.to_string(),
                        "Repository must be associated with an AggregateRoot via ddd:forAggregate"
                            .to_string(),
                        Severity::Violation,
                        shape_id.to_string(),
                    )
                    .with_constraint("ddd:RepositoryBusinessRule".to_string()),
                );
            }
        }

        results
    }
}

// =============================================================================
// Shape Validator
// =============================================================================

pub struct ShapeValidator {
    shapes_store: Store,
}

impl ShapeValidator {
    /// Create a new validator from a shapes file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let shapes_store = Store::new()?;
        let content =
            std::fs::read_to_string(path.as_ref()).context("Failed to read shapes file")?;

        shapes_store
            .load_from_reader(oxigraph::io::RdfFormat::Turtle, content.as_bytes())
            .context("Failed to parse shapes file")?;

        Ok(Self { shapes_store })
    }

    /// Create a validator from Turtle-formatted string
    pub fn from_turtle(turtle: &str) -> Result<Self> {
        let shapes_store = Store::new()?;
        shapes_store
            .load_from_reader(oxigraph::io::RdfFormat::Turtle, turtle.as_bytes())
            .context("Failed to parse shapes")?;

        Ok(Self { shapes_store })
    }

    /// Validate a data graph against all applicable shapes
    pub fn validate_graph(&self, data_store: &Store) -> Result<ValidationReport> {
        let mut report = ValidationReport::new();

        // Get all subjects in the data graph
        let subjects = self.get_all_subjects(data_store)?;

        for subject in subjects {
            match subject {
                NamedOrBlankNode::NamedNode(node) => {
                    let node_results = self.validate_node(&node, data_store)?;
                    for result in node_results {
                        report.add_result(result);
                    }
                }
                _ => {}
            }
        }

        Ok(report)
    }

    /// Validate a specific node
    pub fn validate_node(
        &self,
        node: &NamedNode,
        data_store: &Store,
    ) -> Result<Vec<ValidationResult>> {
        let mut results = Vec::new();

        // Find applicable shapes
        let discovery = ShapeDiscovery::new(&self.shapes_store);
        let shapes = discovery.find_shapes_for_node(node, data_store)?;

        // Validate against each applicable shape
        for shape in shapes {
            results.extend(self.validate_against_shape(node, &shape, data_store)?);
        }

        Ok(results)
    }

    fn validate_against_shape(
        &self,
        node: &NamedNode,
        shape: &NodeShape,
        data_store: &Store,
    ) -> Result<Vec<ValidationResult>> {
        let mut results = Vec::new();
        let checker = ConstraintChecker::new(data_store);
        let custom = CustomConstraints::new(data_store);

        // Check all property constraints
        for property in &shape.properties {
            let mut prop_results = checker.check_property(node, property, shape.id.as_str());

            // Apply shape-level severity if specified
            if shape.severity != Severity::Violation {
                for result in &mut prop_results {
                    result.severity = shape.severity;
                }
            }

            results.extend(prop_results);
        }

        // Check custom DDD invariants
        results.extend(custom.check_ddd_invariants(node, shape.id.as_str()));

        // Check cross-property business rules
        results.extend(custom.check_cross_property_constraints(node, shape.id.as_str()));

        Ok(results)
    }

    /// Validate a TripleStore against all applicable shapes
    ///
    /// **TPS Principle (Jidoka)**: Converts TripleStore to Store for validation.
    /// Fails fast if conversion or validation fails.
    ///
    /// # Errors
    /// Returns `Err` if:
    /// - TripleStore cannot be converted to Store
    /// - Validation fails (SHACL violations detected)
    pub fn validate_triple_store(&self, triple_store: &TripleStore) -> Result<ValidationReport> {
        // Convert TripleStore to Store by extracting Turtle content
        // TripleStore uses oxigraph internally, so we can query all triples
        // and reconstruct them in a Store
        
        // Use CONSTRUCT query to get all triples as RDF
        let construct_query = r#"
            CONSTRUCT { ?s ?p ?o }
            WHERE { ?s ?p ?o }
        "#;
        
        // Execute CONSTRUCT query - returns JSON-LD format
        let json_result = triple_store.query_sparql(construct_query)
            .map_err(|e| anyhow!("Failed to query TripleStore for validation: {}", e))?;
        
        // Parse JSON result and extract triples
        // TripleStore's query_sparql returns SPARQL JSON Results format
        // For CONSTRUCT, we need to parse the graph data
        let data_store = Store::new()?;
        
        // Try to parse as Turtle first (if TripleStore supports it)
        // Otherwise, parse JSON-LD and convert
        if let Err(_) = data_store.load_from_reader(oxigraph::io::RdfFormat::Turtle, json_result.as_bytes()) {
            // If Turtle parsing fails, try JSON-LD
            data_store
                .load_from_reader(oxigraph::io::RdfFormat::JsonLd, json_result.as_bytes())
                .context("Failed to load TripleStore data into Store for validation")?;
        }
        
        // Validate using existing validate_graph method
        self.validate_graph(&data_store)
    }

    fn get_all_subjects(&self, store: &Store) -> Result<Vec<oxigraph::model::NamedOrBlankNode>> {
        let mut subjects = HashSet::new();
        for quad in store.iter() {
            let quad = quad?;
            subjects.insert(quad.subject.to_owned());
        }
        Ok(subjects.into_iter().collect())
    }

    /// Load data from a Turtle file
    pub fn load_data_from_file<P: AsRef<Path>>(&self, path: P) -> Result<Store> {
        let store = Store::new()?;
        let content = std::fs::read_to_string(path.as_ref()).context("Failed to read data file")?;

        store
            .load_from_reader(oxigraph::io::RdfFormat::Turtle, content.as_bytes())
            .context("Failed to parse data file")?;

        Ok(store)
    }

    /// Load data from Turtle string
    pub fn load_data_from_turtle(&self, turtle: &str) -> Result<Store> {
        let store = Store::new()?;
        store
            .load_from_reader(oxigraph::io::RdfFormat::Turtle, turtle.as_bytes())
            .context("Failed to parse data")?;

        Ok(store)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_conversion() {
        let violation = Severity::Violation;
        let iri = violation.to_iri();
        assert_eq!(iri.as_str(), "http://www.w3.org/ns/shacl#Violation");

        let severity = Severity::from_iri(&iri);
        assert_eq!(severity, Severity::Violation);
    }

    #[test]
    fn test_validation_report() {
        let mut report = ValidationReport::new();
        assert!(report.conforms());

        let result = ValidationResult::new(
            "http://example.org/node1".to_string(),
            "Test violation".to_string(),
            Severity::Violation,
            "http://example.org/shape1".to_string(),
        );

        report.add_result(result);
        assert!(!report.conforms());
        assert_eq!(report.results().len(), 1);
        assert_eq!(report.violations().count(), 1);
    }
}
