// =============================================================================
// Type-Safe SPARQL Bindings
// =============================================================================
// Extract and convert SPARQL bindings to Rust types with comprehensive validation

use oxigraph::model::{BlankNode, Literal, NamedNode, Term};
use oxigraph::sparql::QuerySolution;
use std::str::FromStr;
use thiserror::Error;

/// Errors that can occur when extracting typed bindings
#[derive(Debug, Error, Clone, PartialEq)]
pub enum BindingError {
    #[error("Variable '{0}' not found in bindings")]
    NotFound(String),

    #[error("Variable '{0}' is unbound")]
    Unbound(String),

    #[error("Expected {expected} for '{var}', got {actual}")]
    TypeMismatch {
        var: String,
        expected: String,
        actual: String,
    },

    #[error("Failed to convert '{var}' to {target_type}: {reason}")]
    ConversionFailed {
        var: String,
        target_type: String,
        reason: String,
    },

    #[error("Invalid datatype for '{var}': expected {expected}, got {actual}")]
    InvalidDatatype {
        var: String,
        expected: String,
        actual: String,
    },

    #[error("Literal value error for '{var}': {reason}")]
    LiteralValueError { var: String, reason: String },
}

/// Type-safe value extracted from SPARQL bindings
#[derive(Debug, Clone, PartialEq)]
pub enum TypedValue {
    IRI(String),
    Literal(String),
    TypedLiteral { value: String, datatype: String },
    LangLiteral { value: String, language: String },
    BlankNode(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

impl TypedValue {
    /// Get the underlying string value
    pub fn as_str(&self) -> &str {
        match self {
            TypedValue::IRI(s)
            | TypedValue::Literal(s)
            | TypedValue::TypedLiteral { value: s, .. }
            | TypedValue::LangLiteral { value: s, .. }
            | TypedValue::BlankNode(s) => s,
            TypedValue::Integer(_) | TypedValue::Float(_) | TypedValue::Boolean(_) => {
                panic!("Cannot get string from numeric/boolean value")
            }
        }
    }

    /// Try to convert to integer
    pub fn as_i64(&self) -> Result<i64, BindingError> {
        match self {
            TypedValue::Integer(i) => Ok(*i),
            TypedValue::Literal(s) | TypedValue::TypedLiteral { value: s, .. } => s
                .parse::<i64>()
                .map_err(|e| BindingError::LiteralValueError {
                    var: "value".to_string(),
                    reason: e.to_string(),
                }),
            _ => Err(BindingError::TypeMismatch {
                var: "value".to_string(),
                expected: "integer".to_string(),
                actual: format!("{:?}", self),
            }),
        }
    }

    /// Try to convert to float
    pub fn as_f64(&self) -> Result<f64, BindingError> {
        match self {
            TypedValue::Float(f) => Ok(*f),
            TypedValue::Integer(i) => Ok(*i as f64),
            TypedValue::Literal(s) | TypedValue::TypedLiteral { value: s, .. } => s
                .parse::<f64>()
                .map_err(|e| BindingError::LiteralValueError {
                    var: "value".to_string(),
                    reason: e.to_string(),
                }),
            _ => Err(BindingError::TypeMismatch {
                var: "value".to_string(),
                expected: "float".to_string(),
                actual: format!("{:?}", self),
            }),
        }
    }

    /// Try to convert to boolean
    pub fn as_bool(&self) -> Result<bool, BindingError> {
        match self {
            TypedValue::Boolean(b) => Ok(*b),
            TypedValue::Literal(s) | TypedValue::TypedLiteral { value: s, .. } => {
                match s.to_lowercase().as_str() {
                    "true" | "1" => Ok(true),
                    "false" | "0" => Ok(false),
                    _ => Err(BindingError::LiteralValueError {
                        var: "value".to_string(),
                        reason: format!("Invalid boolean value: {}", s),
                    }),
                }
            }
            _ => Err(BindingError::TypeMismatch {
                var: "value".to_string(),
                expected: "boolean".to_string(),
                actual: format!("{:?}", self),
            }),
        }
    }
}

/// Type-safe binding extractor for SPARQL query solutions
///
/// Provides methods to extract values with automatic type conversion and validation
pub struct TypedBinding<'a> {
    solution: &'a QuerySolution,
}

impl<'a> TypedBinding<'a> {
    /// Create a new typed binding extractor
    pub fn new(solution: &'a QuerySolution) -> Self {
        Self { solution }
    }

    /// Get raw term for a variable
    pub fn get_term(&self, var: &str) -> Result<&Term, BindingError> {
        self.solution
            .get(var)
            .ok_or_else(|| BindingError::NotFound(var.to_string()))
    }

    /// Get optional term for a variable
    pub fn get_term_opt(&self, var: &str) -> Option<&Term> {
        self.solution.get(var)
    }

    /// Extract IRI as string
    pub fn get_iri(&self, var: &str) -> Result<String, BindingError> {
        match self.get_term(var)? {
            Term::NamedNode(node) => Ok(node.as_str().to_string()),
            term => Err(BindingError::TypeMismatch {
                var: var.to_string(),
                expected: "IRI".to_string(),
                actual: term_type_name(term),
            }),
        }
    }

    /// Extract optional IRI
    pub fn get_iri_opt(&self, var: &str) -> Result<Option<String>, BindingError> {
        match self.get_term_opt(var) {
            Some(Term::NamedNode(node)) => Ok(Some(node.as_str().to_string())),
            Some(term) => Err(BindingError::TypeMismatch {
                var: var.to_string(),
                expected: "IRI".to_string(),
                actual: term_type_name(term),
            }),
            None => Ok(None),
        }
    }

    /// Extract literal as string
    pub fn get_literal(&self, var: &str) -> Result<String, BindingError> {
        match self.get_term(var)? {
            Term::Literal(lit) => Ok(lit.value().to_string()),
            term => Err(BindingError::TypeMismatch {
                var: var.to_string(),
                expected: "Literal".to_string(),
                actual: term_type_name(term),
            }),
        }
    }

    /// Extract optional literal
    pub fn get_literal_opt(&self, var: &str) -> Result<Option<String>, BindingError> {
        match self.get_term_opt(var) {
            Some(Term::Literal(lit)) => Ok(Some(lit.value().to_string())),
            Some(term) => Err(BindingError::TypeMismatch {
                var: var.to_string(),
                expected: "Literal".to_string(),
                actual: term_type_name(term),
            }),
            None => Ok(None),
        }
    }

    /// Extract literal with specific datatype
    pub fn get_literal_with_datatype(
        &self,
        var: &str,
        datatype: &str,
    ) -> Result<String, BindingError> {
        match self.get_term(var)? {
            Term::Literal(lit) => {
                let actual_dt = lit.datatype().as_str();
                if actual_dt == datatype {
                    Ok(lit.value().to_string())
                } else {
                    Err(BindingError::InvalidDatatype {
                        var: var.to_string(),
                        expected: datatype.to_string(),
                        actual: actual_dt.to_string(),
                    })
                }
            }
            term => Err(BindingError::TypeMismatch {
                var: var.to_string(),
                expected: "Literal".to_string(),
                actual: term_type_name(term),
            }),
        }
    }

    /// Extract blank node
    pub fn get_blank_node(&self, var: &str) -> Result<String, BindingError> {
        match self.get_term(var)? {
            Term::BlankNode(node) => Ok(node.as_str().to_string()),
            term => Err(BindingError::TypeMismatch {
                var: var.to_string(),
                expected: "BlankNode".to_string(),
                actual: term_type_name(term),
            }),
        }
    }

    /// Extract integer (from xsd:integer or similar)
    pub fn get_integer(&self, var: &str) -> Result<i64, BindingError> {
        match self.get_term(var)? {
            Term::Literal(lit) => {
                lit.value()
                    .parse::<i64>()
                    .map_err(|e| BindingError::ConversionFailed {
                        var: var.to_string(),
                        target_type: "i64".to_string(),
                        reason: e.to_string(),
                    })
            }
            term => Err(BindingError::TypeMismatch {
                var: var.to_string(),
                expected: "Literal<integer>".to_string(),
                actual: term_type_name(term),
            }),
        }
    }

    /// Extract optional integer
    pub fn get_integer_opt(&self, var: &str) -> Result<Option<i64>, BindingError> {
        match self.get_term_opt(var) {
            Some(Term::Literal(lit)) => {
                let val =
                    lit.value()
                        .parse::<i64>()
                        .map_err(|e| BindingError::ConversionFailed {
                            var: var.to_string(),
                            target_type: "i64".to_string(),
                            reason: e.to_string(),
                        })?;
                Ok(Some(val))
            }
            Some(term) => Err(BindingError::TypeMismatch {
                var: var.to_string(),
                expected: "Literal<integer>".to_string(),
                actual: term_type_name(term),
            }),
            None => Ok(None),
        }
    }

    /// Extract float/double
    pub fn get_float(&self, var: &str) -> Result<f64, BindingError> {
        match self.get_term(var)? {
            Term::Literal(lit) => {
                lit.value()
                    .parse::<f64>()
                    .map_err(|e| BindingError::ConversionFailed {
                        var: var.to_string(),
                        target_type: "f64".to_string(),
                        reason: e.to_string(),
                    })
            }
            term => Err(BindingError::TypeMismatch {
                var: var.to_string(),
                expected: "Literal<float>".to_string(),
                actual: term_type_name(term),
            }),
        }
    }

    /// Extract boolean
    pub fn get_boolean(&self, var: &str) -> Result<bool, BindingError> {
        match self.get_term(var)? {
            Term::Literal(lit) => match lit.value() {
                "true" | "1" => Ok(true),
                "false" | "0" => Ok(false),
                other => Err(BindingError::ConversionFailed {
                    var: var.to_string(),
                    target_type: "bool".to_string(),
                    reason: format!("Invalid boolean value: {}", other),
                }),
            },
            term => Err(BindingError::TypeMismatch {
                var: var.to_string(),
                expected: "Literal<boolean>".to_string(),
                actual: term_type_name(term),
            }),
        }
    }

    /// Extract string with default value if unbound
    pub fn get_string_or(&self, var: &str, default: &str) -> String {
        self.get_literal_opt(var)
            .unwrap_or(None)
            .unwrap_or_else(|| default.to_string())
    }

    /// Extract integer with default value
    pub fn get_integer_or(&self, var: &str, default: i64) -> i64 {
        self.get_integer_opt(var).unwrap_or(None).unwrap_or(default)
    }

    /// Extract any value as TypedValue
    pub fn get_typed_value(&self, var: &str) -> Result<TypedValue, BindingError> {
        let term = self.get_term(var)?;
        term_to_typed_value(term)
    }

    /// Extract optional typed value
    pub fn get_typed_value_opt(&self, var: &str) -> Result<Option<TypedValue>, BindingError> {
        match self.get_term_opt(var) {
            Some(term) => term_to_typed_value(term).map(Some),
            None => Ok(None),
        }
    }

    /// Parse into a custom type implementing FromStr
    pub fn parse<T: FromStr>(&self, var: &str) -> Result<T, BindingError>
    where
        T::Err: std::fmt::Display,
    {
        let value = self.get_literal(var)?;
        value
            .parse::<T>()
            .map_err(|e| BindingError::ConversionFailed {
                var: var.to_string(),
                target_type: std::any::type_name::<T>().to_string(),
                reason: e.to_string(),
            })
    }

    /// Get all variable names in this solution
    pub fn variables(&self) -> Vec<String> {
        self.solution
            .variables()
            .iter()
            .map(|v| v.as_str().to_string())
            .collect()
    }
}

/// Convert a term to a typed value
fn term_to_typed_value(term: &Term) -> Result<TypedValue, BindingError> {
    match term {
        Term::NamedNode(node) => Ok(TypedValue::IRI(node.as_str().to_string())),
        Term::BlankNode(node) => Ok(TypedValue::BlankNode(node.as_str().to_string())),
        Term::Literal(lit) => {
            let value = lit.value().to_string();
            let datatype = lit.datatype().as_str();

            // Try to parse common XSD types
            match datatype {
                "http://www.w3.org/2001/XMLSchema#integer"
                | "http://www.w3.org/2001/XMLSchema#int"
                | "http://www.w3.org/2001/XMLSchema#long" => {
                    if let Ok(i) = value.parse::<i64>() {
                        return Ok(TypedValue::Integer(i));
                    }
                }
                "http://www.w3.org/2001/XMLSchema#decimal"
                | "http://www.w3.org/2001/XMLSchema#float"
                | "http://www.w3.org/2001/XMLSchema#double" => {
                    if let Ok(f) = value.parse::<f64>() {
                        return Ok(TypedValue::Float(f));
                    }
                }
                "http://www.w3.org/2001/XMLSchema#boolean" => {
                    if let Ok(b) = value.parse::<bool>() {
                        return Ok(TypedValue::Boolean(b));
                    }
                }
                "http://www.w3.org/2001/XMLSchema#string" => {
                    return Ok(TypedValue::Literal(value));
                }
                _ => {}
            }

            // Check for language tag
            if let Some(lang) = lit.language() {
                return Ok(TypedValue::LangLiteral {
                    value,
                    language: lang.to_string(),
                });
            }

            // Return as typed literal
            Ok(TypedValue::TypedLiteral {
                value,
                datatype: datatype.to_string(),
            })
        }
        Term::Triple(_) => Err(BindingError::UnsupportedType {
            variable: "unknown".to_string(),
            expected: "Term".to_string(),
            found: "Triple".to_string(),
        }),
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

#[cfg(test)]
mod tests {
    use super::*;
    use oxigraph::model::{Literal, NamedNode};
    use oxigraph::sparql::QuerySolution;

    fn create_test_solution() -> QuerySolution {
        let mut bindings = Vec::new();
        bindings.push((
            "iri".to_string(),
            Term::NamedNode(NamedNode::new("http://example.org/test").unwrap()),
        ));
        bindings.push((
            "literal".to_string(),
            Term::Literal(Literal::new_simple_literal("test value")),
        ));

        QuerySolution::from(bindings)
    }

    #[test]
    fn test_get_iri() {
        let solution = create_test_solution();
        let typed = TypedBinding::new(&solution);

        let iri = typed.get_iri("iri").unwrap();
        assert_eq!(iri, "http://example.org/test");
    }

    #[test]
    fn test_get_literal() {
        let solution = create_test_solution();
        let typed = TypedBinding::new(&solution);

        let literal = typed.get_literal("literal").unwrap();
        assert_eq!(literal, "test value");
    }

    #[test]
    fn test_missing_variable() {
        let solution = create_test_solution();
        let typed = TypedBinding::new(&solution);

        let result = typed.get_iri("missing");
        assert!(matches!(result, Err(BindingError::NotFound(_))));
    }
}
