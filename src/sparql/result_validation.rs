// =============================================================================
// SPARQL ResultSet Validation
// =============================================================================
// Validates SPARQL SELECT query results against expected schemas
// Implements poka-yoke error-proofing at the query result boundary

use oxigraph::model::Term;
use oxigraph::sparql::{QueryResults, QuerySolution};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Validation errors for SPARQL query results
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ValidationError {
    #[error("Expected variable '{0}' not found in results")]
    MissingVariable(String),

    #[error("Variable '{0}' has unexpected type. Expected {1}, got {2}")]
    TypeMismatch(String, String, String),

    #[error("Cardinality constraint violated for '{0}': {1}")]
    CardinalityViolation(String, String),

    #[error("Unbound value for required variable '{0}'")]
    UnboundRequired(String),

    #[error("Duplicate binding detected for variable '{0}'")]
    DuplicateBinding(String),

    #[error("Invalid result set: {0}")]
    InvalidResultSet(String),

    #[error("No results returned when at least one was expected")]
    NoResults,

    #[error("Multiple results returned when exactly one was expected")]
    MultipleResults,

    #[error("Validation rule '{0}' failed: {1}")]
    RuleFailed(String, String),
}

/// Cardinality constraints for query results
#[derive(Debug, Clone, PartialEq)]
pub enum CardinalityConstraint {
    /// Exactly one result
    ExactlyOne,
    /// Zero or one result
    ZeroOrOne,
    /// One or more results
    OneOrMore,
    /// Zero or more results (no constraint)
    ZeroOrMore,
    /// Exact count
    Exact(usize),
    /// Minimum count
    Min(usize),
    /// Maximum count
    Max(usize),
    /// Range (min, max)
    Range(usize, usize),
}

impl CardinalityConstraint {
    /// Validate count against constraint
    pub fn validate(&self, count: usize) -> Result<(), ValidationError> {
        let valid = match self {
            CardinalityConstraint::ExactlyOne => count == 1,
            CardinalityConstraint::ZeroOrOne => count <= 1,
            CardinalityConstraint::OneOrMore => count >= 1,
            CardinalityConstraint::ZeroOrMore => true,
            CardinalityConstraint::Exact(n) => count == *n,
            CardinalityConstraint::Min(n) => count >= *n,
            CardinalityConstraint::Max(n) => count <= *n,
            CardinalityConstraint::Range(min, max) => count >= *min && count <= *max,
        };

        if valid {
            Ok(())
        } else {
            Err(ValidationError::CardinalityViolation(
                "result_count".to_string(),
                format!("Expected {:?}, got {}", self, count),
            ))
        }
    }
}

/// Expected type for a SPARQL binding
#[derive(Debug, Clone, PartialEq)]
pub enum ExpectedType {
    IRI,
    Literal,
    BlankNode,
    LiteralWithDatatype(String),
    LiteralWithLanguage(String),
    Any,
}

impl ExpectedType {
    /// Check if a term matches the expected type
    pub fn matches(&self, term: &Term) -> bool {
        match (self, term) {
            (ExpectedType::IRI, Term::NamedNode(_)) => true,
            (ExpectedType::BlankNode, Term::BlankNode(_)) => true,
            (ExpectedType::Literal, Term::Literal(_)) => true,
            (ExpectedType::LiteralWithDatatype(dt), Term::Literal(lit)) => {
                lit.datatype().as_str() == dt
            }
            (ExpectedType::LiteralWithLanguage(lang), Term::Literal(lit)) => {
                lit.language() == Some(lang.as_str())
            }
            (ExpectedType::Any, _) => true,
            _ => false,
        }
    }

    /// Get type name for error messages
    pub fn type_name(&self) -> String {
        match self {
            ExpectedType::IRI => "IRI".to_string(),
            ExpectedType::Literal => "Literal".to_string(),
            ExpectedType::BlankNode => "BlankNode".to_string(),
            ExpectedType::LiteralWithDatatype(dt) => format!("Literal<{}>", dt),
            ExpectedType::LiteralWithLanguage(lang) => format!("Literal@{}", lang),
            ExpectedType::Any => "Any".to_string(),
        }
    }
}

/// Variable specification for validation
#[derive(Debug, Clone)]
pub struct VariableSpec {
    pub name: String,
    pub required: bool,
    pub expected_type: ExpectedType,
    pub allow_duplicates: bool,
}

impl VariableSpec {
    /// Create a required variable spec
    pub fn required(name: impl Into<String>, expected_type: ExpectedType) -> Self {
        Self {
            name: name.into(),
            required: true,
            expected_type,
            allow_duplicates: true,
        }
    }

    /// Create an optional variable spec
    pub fn optional(name: impl Into<String>, expected_type: ExpectedType) -> Self {
        Self {
            name: name.into(),
            required: false,
            expected_type,
            allow_duplicates: true,
        }
    }

    /// Set whether duplicates are allowed
    pub fn with_duplicates(mut self, allow: bool) -> Self {
        self.allow_duplicates = allow;
        self
    }
}

/// SPARQL ResultSet Validator
///
/// Validates SELECT query results against expected schema:
/// - Variable presence
/// - Type checking
/// - Cardinality constraints
/// - Null/unbound handling
/// - Duplicate detection
#[derive(Debug, Clone)]
pub struct ResultSetValidator {
    variables: Vec<VariableSpec>,
    cardinality: CardinalityConstraint,
    strict_mode: bool,
}

impl ResultSetValidator {
    /// Create a new validator with cardinality constraint
    pub fn new(cardinality: CardinalityConstraint) -> Self {
        Self {
            variables: Vec::new(),
            cardinality,
            strict_mode: false,
        }
    }

    /// Add a variable specification
    pub fn with_variable(mut self, spec: VariableSpec) -> Self {
        self.variables.push(spec);
        self
    }

    /// Enable strict mode (all variables must be declared)
    pub fn strict(mut self) -> Self {
        self.strict_mode = true;
        self
    }

    /// Validate a query solution
    pub fn validate_solution(&self, solution: &QuerySolution) -> Result<(), ValidationError> {
        // Check for required variables
        for spec in &self.variables {
            if spec.required {
                match solution.get(&spec.name) {
                    None => {
                        return Err(ValidationError::UnboundRequired(spec.name.clone()));
                    }
                    Some(term) => {
                        // Type check
                        if !spec.expected_type.matches(term) {
                            return Err(ValidationError::TypeMismatch(
                                spec.name.clone(),
                                spec.expected_type.type_name(),
                                term_type_name(term),
                            ));
                        }
                    }
                }
            } else if let Some(term) = solution.get(&spec.name) {
                // Type check optional variables if present
                if !spec.expected_type.matches(term) {
                    return Err(ValidationError::TypeMismatch(
                        spec.name.clone(),
                        spec.expected_type.type_name(),
                        term_type_name(term),
                    ));
                }
            }
        }

        // In strict mode, check for undeclared variables
        if self.strict_mode {
            let declared: HashSet<&str> = self.variables.iter().map(|v| v.name.as_str()).collect();
            for var_name in solution.variables() {
                if !declared.contains(var_name.as_str()) {
                    return Err(ValidationError::InvalidResultSet(format!(
                        "Undeclared variable '{}' in strict mode",
                        var_name
                    )));
                }
            }
        }

        Ok(())
    }

    /// Validate an entire result set
    pub fn validate_results(&self, results: Vec<QuerySolution>) -> Result<(), ValidationError> {
        // Check cardinality
        self.cardinality.validate(results.len())?;

        // Check for duplicates
        let mut seen_values: HashMap<String, HashSet<String>> = HashMap::new();

        for solution in &results {
            // Validate individual solution
            self.validate_solution(solution)?;

            // Check for duplicates
            for spec in &self.variables {
                if !spec.allow_duplicates {
                    if let Some(term) = solution.get(&spec.name) {
                        let value = term.to_string();
                        let values = seen_values.entry(spec.name.clone()).or_default();

                        if values.contains(&value) {
                            return Err(ValidationError::DuplicateBinding(spec.name.clone()));
                        }
                        values.insert(value);
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate and collect results from a QueryResults
    pub fn validate_and_collect(
        &self,
        query_results: QueryResults,
    ) -> Result<Vec<QuerySolution>, ValidationError> {
        match query_results {
            QueryResults::Solutions(solutions) => {
                let collected: Result<Vec<_>, _> = solutions.collect();
                let results = collected.map_err(|e| {
                    ValidationError::InvalidResultSet(format!("Error collecting results: {}", e))
                })?;

                self.validate_results(results.clone())?;
                Ok(results)
            }
            _ => Err(ValidationError::InvalidResultSet(
                "Expected SELECT results, got CONSTRUCT or ASK".to_string(),
            )),
        }
    }
}

/// Get human-readable type name for a term
fn term_type_name(term: &Term) -> String {
    match term {
        Term::NamedNode(_) => "IRI".to_string(),
        Term::BlankNode(_) => "BlankNode".to_string(),
        Term::Literal(lit) => {
            if let Some(lang) = lit.language() {
                format!("Literal@{}", lang)
            } else {
                format!("Literal<{}>", lit.datatype())
            }
        }
        Term::Triple(_) => "Triple".to_string(),
    }
}

/// Builder for common validation patterns
pub struct ValidatorBuilder;

impl ValidatorBuilder {
    /// Validator expecting exactly one result
    pub fn exactly_one() -> ResultSetValidator {
        ResultSetValidator::new(CardinalityConstraint::ExactlyOne)
    }

    /// Validator expecting zero or one result
    pub fn optional_single() -> ResultSetValidator {
        ResultSetValidator::new(CardinalityConstraint::ZeroOrOne)
    }

    /// Validator expecting one or more results
    pub fn at_least_one() -> ResultSetValidator {
        ResultSetValidator::new(CardinalityConstraint::OneOrMore)
    }

    /// Validator expecting any number of results
    pub fn any() -> ResultSetValidator {
        ResultSetValidator::new(CardinalityConstraint::ZeroOrMore)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cardinality_exactly_one() {
        let constraint = CardinalityConstraint::ExactlyOne;
        assert!(constraint.validate(1).is_ok());
        assert!(constraint.validate(0).is_err());
        assert!(constraint.validate(2).is_err());
    }

    #[test]
    fn test_cardinality_range() {
        let constraint = CardinalityConstraint::Range(2, 5);
        assert!(constraint.validate(1).is_err());
        assert!(constraint.validate(2).is_ok());
        assert!(constraint.validate(4).is_ok());
        assert!(constraint.validate(5).is_ok());
        assert!(constraint.validate(6).is_err());
    }

    #[test]
    fn test_expected_type_iri() {
        use oxigraph::model::NamedNode;

        let expected = ExpectedType::IRI;
        let iri = Term::NamedNode(NamedNode::new("http://example.org/test").unwrap());

        assert!(expected.matches(&iri));
    }
}
