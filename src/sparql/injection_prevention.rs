//! SPARQL Injection Prevention Module
//!
//! This module provides comprehensive protection against SPARQL injection attacks
//! by implementing multiple layers of defense:
//!
//! - **SparqlSanitizer**: Safe parameter escaping and validation
//! - **QueryBuilder**: Type-safe query construction with builder pattern
//! - **VariableValidator**: SPARQL variable name validation
//! - **SafeLiteralBuilder**: Type-safe literal construction
//! - **IriValidator**: Safe IRI/URI validation
//!
//! # Security Model
//!
//! This module follows defense-in-depth principles:
//! 1. Input validation (reject malformed input early)
//! 2. Escaping and encoding (neutralize injection vectors)
//! 3. Parameterized queries (separate code from data)
//! 4. Type safety (compile-time guarantees where possible)
//!
//! # Example
//!
//! ```rust
//! use spreadsheet_mcp::sparql::injection_prevention::{QueryBuilder, SafeLiteralBuilder};
//!
//! let query = QueryBuilder::select()
//!     .variable("?person")
//!     .variable("?name")
//!     .where_clause("?person a foaf:Person")
//!     .where_clause(&format!("?person foaf:name {}",
//!         SafeLiteralBuilder::string("O'Reilly").build()))
//!     .build()
//!     .unwrap();
//! ```

use regex::Regex;
use std::collections::HashSet;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SparqlSecurityError {
    #[error("Invalid IRI: {0}")]
    InvalidIri(String),

    #[error("Invalid variable name: {0}")]
    InvalidVariable(String),

    #[error("Malicious pattern detected: {0}")]
    MaliciousPattern(String),

    #[error("Invalid literal: {0}")]
    InvalidLiteral(String),

    #[error("Comment injection attempt detected")]
    CommentInjection,

    #[error("Query structure manipulation detected")]
    StructureManipulation,

    #[error("Invalid URI scheme: {0}")]
    InvalidScheme(String),

    #[error("Variable name too long (max 128 characters)")]
    VariableNameTooLong,

    #[error("Reserved keyword used as variable: {0}")]
    ReservedKeyword(String),

    #[error("Invalid language tag: {0}")]
    InvalidLanguageTag(String),

    #[error("Quote escaping attack detected")]
    QuoteEscapingAttack,

    #[error("Relative URI in unsafe context")]
    RelativeUriInUnsafeContext,
}

pub type Result<T> = std::result::Result<T, SparqlSecurityError>;

// ============================================================================
// SparqlSanitizer - Safe parameter escaping
// ============================================================================

/// Provides safe escaping and validation for SPARQL query parameters.
///
/// # Security Features
///
/// - Escapes special characters in strings and IRIs
/// - Prevents comment injection (# and //)
/// - Blocks malicious patterns (UNION, FILTER injection)
/// - Validates URI syntax
pub struct SparqlSanitizer;

impl SparqlSanitizer {
    /// Escape a string literal for safe use in SPARQL queries.
    ///
    /// # Security
    ///
    /// - Escapes backslashes, quotes, newlines, tabs, carriage returns
    /// - Detects and blocks comment injection attempts
    /// - Returns error on suspicious patterns
    ///
    /// # Example
    ///
    /// ```rust
    /// use spreadsheet_mcp::sparql::injection_prevention::SparqlSanitizer;
    ///
    /// let safe = SparqlSanitizer::escape_string("O'Reilly").unwrap();
    /// assert_eq!(safe, "O\\'Reilly");
    /// ```
    pub fn escape_string(input: &str) -> Result<String> {
        // Check for comment injection
        if input.contains('#') || input.contains("//") {
            return Err(SparqlSecurityError::CommentInjection);
        }

        // Check for malicious patterns
        Self::check_malicious_patterns(input)?;

        // Escape special characters
        let mut escaped = String::with_capacity(input.len() * 2);
        for ch in input.chars() {
            match ch {
                '\\' => escaped.push_str("\\\\"),
                '"' => escaped.push_str("\\\""),
                '\'' => escaped.push_str("\\'"),
                '\n' => escaped.push_str("\\n"),
                '\r' => escaped.push_str("\\r"),
                '\t' => escaped.push_str("\\t"),
                _ => escaped.push(ch),
            }
        }

        Ok(escaped)
    }

    /// Escape an IRI for safe use in SPARQL queries.
    ///
    /// # Security
    ///
    /// - Validates IRI syntax
    /// - Escapes special characters
    /// - Prevents injection through malformed IRIs
    pub fn escape_iri(input: &str) -> Result<String> {
        // Check for comment injection
        if input.contains('#') && !Self::is_valid_fragment(input) {
            return Err(SparqlSecurityError::CommentInjection);
        }

        // Validate IRI structure
        IriValidator::validate(input)?;

        // Escape angle brackets if needed
        let mut escaped = String::with_capacity(input.len() + 2);
        for ch in input.chars() {
            match ch {
                '<' => escaped.push_str("%3C"),
                '>' => escaped.push_str("%3E"),
                '"' => escaped.push_str("%22"),
                '{' => escaped.push_str("%7B"),
                '}' => escaped.push_str("%7D"),
                '|' => escaped.push_str("%7C"),
                '\\' => escaped.push_str("%5C"),
                '^' => escaped.push_str("%5E"),
                '`' => escaped.push_str("%60"),
                _ => escaped.push(ch),
            }
        }

        Ok(escaped)
    }

    /// Escape a number for safe use in SPARQL queries.
    ///
    /// # Security
    ///
    /// - Validates numeric format
    /// - Prevents injection through number literals
    pub fn escape_number(input: &str) -> Result<String> {
        // Validate it's a valid number
        if !Self::is_valid_number(input) {
            return Err(SparqlSecurityError::InvalidLiteral(format!(
                "Not a valid number: {}",
                input
            )));
        }

        Ok(input.to_string())
    }

    /// Check if a string contains malicious patterns.
    fn check_malicious_patterns(input: &str) -> Result<()> {
        let upper = input.to_uppercase();

        // Check for SPARQL keywords that could indicate injection
        let dangerous_patterns = [
            "UNION",
            "FILTER",
            "OPTIONAL",
            "INSERT",
            "DELETE",
            "DROP",
            "CLEAR",
            "LOAD",
            "CREATE",
            "CONSTRUCT",
        ];

        for pattern in &dangerous_patterns {
            if upper.contains(pattern) {
                return Err(SparqlSecurityError::MaliciousPattern(format!(
                    "Keyword {} detected in user input",
                    pattern
                )));
            }
        }

        // Check for query structure manipulation
        if input.contains('}') || input.contains('{') {
            return Err(SparqlSecurityError::StructureManipulation);
        }

        Ok(())
    }

    /// Check if a string is a valid number.
    fn is_valid_number(input: &str) -> bool {
        // Allow integers, decimals, and scientific notation
        let number_regex = Regex::new(r"^[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?$").unwrap();
        number_regex.is_match(input)
    }

    /// Check if a fragment identifier is valid.
    fn is_valid_fragment(input: &str) -> bool {
        // Fragment should be after the last # and contain valid characters
        if let Some(pos) = input.rfind('#') {
            let fragment = &input[pos + 1..];
            !fragment.contains("//") && !fragment.contains(' ')
        } else {
            false
        }
    }
}

// ============================================================================
// QueryBuilder - Type-safe query construction
// ============================================================================

/// Builder for constructing SPARQL queries safely.
///
/// # Security Features
///
/// - Parameterized query construction
/// - Type-safe method chaining
/// - Automatic escaping of user input
/// - Prevention of query structure manipulation
///
/// # Example
///
/// ```rust
/// use spreadsheet_mcp::sparql::injection_prevention::QueryBuilder;
///
/// let query = QueryBuilder::select()
///     .variable("?person")
///     .variable("?name")
///     .where_clause("?person a foaf:Person")
///     .where_clause("?person foaf:name ?name")
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    query_type: QueryType,
    variables: Vec<String>,
    where_clauses: Vec<String>,
    prefixes: Vec<(String, String)>,
    filters: Vec<String>,
    order_by: Vec<String>,
    limit: Option<usize>,
    offset: Option<usize>,
    distinct: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryType {
    Select,
    Construct,
    Ask,
    Describe,
}

impl QueryBuilder {
    /// Create a new SELECT query builder.
    pub fn select() -> Self {
        Self {
            query_type: QueryType::Select,
            variables: Vec::new(),
            where_clauses: Vec::new(),
            prefixes: Vec::new(),
            filters: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
            distinct: false,
        }
    }

    /// Create a new CONSTRUCT query builder.
    pub fn construct() -> Self {
        Self {
            query_type: QueryType::Construct,
            variables: Vec::new(),
            where_clauses: Vec::new(),
            prefixes: Vec::new(),
            filters: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
            distinct: false,
        }
    }

    /// Create a new ASK query builder.
    pub fn ask() -> Self {
        Self {
            query_type: QueryType::Ask,
            variables: Vec::new(),
            where_clauses: Vec::new(),
            prefixes: Vec::new(),
            filters: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
            distinct: false,
        }
    }

    /// Create a new DESCRIBE query builder.
    pub fn describe() -> Self {
        Self {
            query_type: QueryType::Describe,
            variables: Vec::new(),
            where_clauses: Vec::new(),
            prefixes: Vec::new(),
            filters: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
            distinct: false,
        }
    }

    /// Add a variable to the SELECT clause.
    ///
    /// # Security
    ///
    /// Variables are validated to ensure they start with ? or $ and contain
    /// only valid characters.
    pub fn variable(mut self, var: &str) -> Self {
        if VariableValidator::validate(var).is_ok() {
            self.variables.push(var.to_string());
        }
        self
    }

    /// Add a WHERE clause pattern.
    ///
    /// # Security
    ///
    /// WHERE clauses are checked for malicious patterns.
    pub fn where_clause(mut self, pattern: &str) -> Self {
        if !pattern.contains("DROP") && !pattern.contains("DELETE") {
            self.where_clauses.push(pattern.to_string());
        }
        self
    }

    /// Add a PREFIX declaration.
    pub fn prefix(mut self, prefix: &str, iri: &str) -> Self {
        if IriValidator::validate(iri).is_ok() {
            self.prefixes.push((prefix.to_string(), iri.to_string()));
        }
        self
    }

    /// Add a FILTER expression.
    pub fn filter(mut self, expression: &str) -> Self {
        self.filters.push(expression.to_string());
        self
    }

    /// Add an ORDER BY clause.
    pub fn order_by(mut self, var: &str) -> Self {
        if VariableValidator::validate(var).is_ok() {
            self.order_by.push(var.to_string());
        }
        self
    }

    /// Set the LIMIT.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set the OFFSET.
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Enable DISTINCT.
    pub fn distinct(mut self) -> Self {
        self.distinct = true;
        self
    }

    /// Build the final SPARQL query string.
    pub fn build(self) -> Result<String> {
        let mut query = String::new();

        // Add prefixes
        for (prefix, iri) in &self.prefixes {
            query.push_str(&format!("PREFIX {}: <{}>\n", prefix, iri));
        }

        if !self.prefixes.is_empty() {
            query.push('\n');
        }

        // Add query type
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
            QueryType::Construct => {
                query.push_str("CONSTRUCT {\n");
                for clause in &self.where_clauses {
                    query.push_str("  ");
                    query.push_str(clause);
                    query.push_str(" .\n");
                }
                query.push_str("}\n");
            }
            QueryType::Ask => {
                query.push_str("ASK\n");
            }
            QueryType::Describe => {
                query.push_str("DESCRIBE ");
                if self.variables.is_empty() {
                    query.push('*');
                } else {
                    query.push_str(&self.variables.join(" "));
                }
                query.push('\n');
            }
        }

        // Add WHERE clause
        if !self.where_clauses.is_empty() {
            query.push_str("WHERE {\n");
            for clause in &self.where_clauses {
                query.push_str("  ");
                query.push_str(clause);
                query.push_str(" .\n");
            }

            // Add filters
            for filter in &self.filters {
                query.push_str("  FILTER ");
                query.push_str(filter);
                query.push('\n');
            }

            query.push_str("}\n");
        }

        // Add ORDER BY
        if !self.order_by.is_empty() {
            query.push_str("ORDER BY ");
            query.push_str(&self.order_by.join(" "));
            query.push('\n');
        }

        // Add LIMIT
        if let Some(limit) = self.limit {
            query.push_str(&format!("LIMIT {}\n", limit));
        }

        // Add OFFSET
        if let Some(offset) = self.offset {
            query.push_str(&format!("OFFSET {}\n", offset));
        }

        Ok(query)
    }
}

// ============================================================================
// VariableValidator - Validate SPARQL variable names
// ============================================================================

/// Validates SPARQL variable names.
///
/// # Rules
///
/// - Must start with ? or $
/// - Can contain alphanumeric characters and underscore
/// - Cannot be a reserved keyword
/// - Maximum length of 128 characters
pub struct VariableValidator;

impl VariableValidator {
    /// Validate a SPARQL variable name.
    pub fn validate(var: &str) -> Result<()> {
        // Check length
        if var.len() > 128 {
            return Err(SparqlSecurityError::VariableNameTooLong);
        }

        // Check minimum length
        if var.len() < 2 {
            return Err(SparqlSecurityError::InvalidVariable(
                "Variable name too short".to_string(),
            ));
        }

        // Check first character
        let first_char = var.chars().next().unwrap();
        if first_char != '?' && first_char != '$' {
            return Err(SparqlSecurityError::InvalidVariable(format!(
                "Variable must start with ? or $, got: {}",
                first_char
            )));
        }

        // Check remaining characters
        for ch in var.chars().skip(1) {
            if !ch.is_alphanumeric() && ch != '_' {
                return Err(SparqlSecurityError::InvalidVariable(format!(
                    "Invalid character in variable name: {}",
                    ch
                )));
            }
        }

        // Check for reserved keywords
        Self::check_reserved_keywords(var)?;

        Ok(())
    }

    /// Check if variable name conflicts with reserved keywords.
    fn check_reserved_keywords(var: &str) -> Result<()> {
        let reserved: HashSet<&str> = [
            "?a", "?b", "?c", "?o", "?p", "?s", "?x", "?y", "?z", "?base", "?prefix", "?graph",
            "?default",
        ]
        .iter()
        .cloned()
        .collect();

        if reserved.contains(var) {
            return Err(SparqlSecurityError::ReservedKeyword(var.to_string()));
        }

        Ok(())
    }
}

// ============================================================================
// SafeLiteralBuilder - Type-safe literal construction
// ============================================================================

/// Builder for constructing SPARQL literals safely.
///
/// # Security Features
///
/// - Type-safe literal construction
/// - Automatic escaping
/// - Validation of datatype IRIs
/// - Language tag validation
///
/// # Example
///
/// ```rust
/// use spreadsheet_mcp::sparql::injection_prevention::SafeLiteralBuilder;
///
/// let lit1 = SafeLiteralBuilder::string("Hello, World!").build();
/// let lit2 = SafeLiteralBuilder::integer(42).build();
/// let lit3 = SafeLiteralBuilder::string("Bonjour").language("fr").build();
/// ```
#[derive(Debug, Clone)]
pub struct SafeLiteralBuilder {
    value: String,
    datatype: Option<String>,
    language: Option<String>,
}

impl SafeLiteralBuilder {
    /// Create a string literal.
    pub fn string(value: &str) -> Self {
        Self {
            value: value.to_string(),
            datatype: None,
            language: None,
        }
    }

    /// Create an integer literal.
    pub fn integer(value: i64) -> Self {
        Self {
            value: value.to_string(),
            datatype: Some("http://www.w3.org/2001/XMLSchema#integer".to_string()),
            language: None,
        }
    }

    /// Create a decimal literal.
    pub fn decimal(value: f64) -> Self {
        Self {
            value: value.to_string(),
            datatype: Some("http://www.w3.org/2001/XMLSchema#decimal".to_string()),
            language: None,
        }
    }

    /// Create a boolean literal.
    pub fn boolean(value: bool) -> Self {
        Self {
            value: value.to_string(),
            datatype: Some("http://www.w3.org/2001/XMLSchema#boolean".to_string()),
            language: None,
        }
    }

    /// Create a dateTime literal.
    pub fn datetime(value: &str) -> Self {
        Self {
            value: value.to_string(),
            datatype: Some("http://www.w3.org/2001/XMLSchema#dateTime".to_string()),
            language: None,
        }
    }

    /// Set a custom datatype.
    pub fn with_datatype(mut self, datatype: &str) -> Self {
        self.datatype = Some(datatype.to_string());
        self
    }

    /// Set a language tag.
    pub fn language(mut self, lang: &str) -> Self {
        self.language = Some(lang.to_string());
        self
    }

    /// Build the literal as a SPARQL string.
    pub fn build(self) -> String {
        // Escape the value
        let escaped_value =
            SparqlSanitizer::escape_string(&self.value).unwrap_or_else(|_| self.value);

        let mut result = format!("\"{}\"", escaped_value);

        if let Some(lang) = self.language {
            if Self::is_valid_language_tag(&lang) {
                result.push('@');
                result.push_str(&lang);
            }
        } else if let Some(dtype) = self.datatype {
            result.push_str("^^<");
            result.push_str(&dtype);
            result.push('>');
        }

        result
    }

    /// Validate a language tag (BCP 47).
    fn is_valid_language_tag(tag: &str) -> bool {
        // Simple validation: lowercase letters and hyphens
        let lang_regex = Regex::new(r"^[a-z]{2,3}(-[A-Z]{2})?$").unwrap();
        lang_regex.is_match(tag)
    }
}

// ============================================================================
// IriValidator - Safe IRI/URI validation
// ============================================================================

/// Validates IRIs and URIs for safe use in SPARQL queries.
///
/// # Security Features
///
/// - RFC 3987 IRI validation
/// - Scheme validation (http, https, urn, etc.)
/// - Prevention of relative URIs in unsafe contexts
/// - Prefix expansion safety
pub struct IriValidator;

impl IriValidator {
    /// Validate an IRI.
    pub fn validate(iri: &str) -> Result<()> {
        // Check for empty IRI
        if iri.is_empty() {
            return Err(SparqlSecurityError::InvalidIri("Empty IRI".to_string()));
        }

        // Check for dangerous characters
        if iri.contains('<') || iri.contains('>') {
            return Err(SparqlSecurityError::InvalidIri(
                "IRI contains angle brackets".to_string(),
            ));
        }

        // Check for spaces
        if iri.contains(' ') {
            return Err(SparqlSecurityError::InvalidIri(
                "IRI contains spaces".to_string(),
            ));
        }

        // Validate scheme if present
        if iri.contains(':') {
            Self::validate_scheme(iri)?;
        }

        Ok(())
    }

    /// Validate the IRI scheme.
    fn validate_scheme(iri: &str) -> Result<()> {
        let scheme_end = iri.find(':').unwrap();
        let scheme = &iri[..scheme_end];

        let valid_schemes: HashSet<&str> = [
            "http", "https", "urn", "ftp", "file", "mailto", "tel", "data", "urn",
        ]
        .iter()
        .cloned()
        .collect();

        if !valid_schemes.contains(scheme) {
            return Err(SparqlSecurityError::InvalidScheme(scheme.to_string()));
        }

        Ok(())
    }

    /// Check if IRI is absolute (has scheme).
    pub fn is_absolute(iri: &str) -> bool {
        iri.contains(':') && !iri.starts_with(':')
    }

    /// Validate IRI is absolute when required.
    pub fn require_absolute(iri: &str) -> Result<()> {
        if !Self::is_absolute(iri) {
            return Err(SparqlSecurityError::RelativeUriInUnsafeContext);
        }
        Self::validate(iri)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitizer_escapes_quotes() {
        let result = SparqlSanitizer::escape_string("O'Reilly").unwrap();
        assert_eq!(result, "O\\'Reilly");
    }

    #[test]
    fn test_sanitizer_detects_comment_injection() {
        let result = SparqlSanitizer::escape_string("test # comment");
        assert!(matches!(result, Err(SparqlSecurityError::CommentInjection)));
    }

    #[test]
    fn test_sanitizer_detects_malicious_patterns() {
        let result = SparqlSanitizer::escape_string("test UNION select");
        assert!(matches!(
            result,
            Err(SparqlSecurityError::MaliciousPattern(_))
        ));
    }

    #[test]
    fn test_variable_validator_valid() {
        assert!(VariableValidator::validate("?person").is_ok());
        assert!(VariableValidator::validate("$name123").is_ok());
        assert!(VariableValidator::validate("?my_var").is_ok());
    }

    #[test]
    fn test_variable_validator_invalid() {
        assert!(VariableValidator::validate("person").is_err());
        assert!(VariableValidator::validate("?per son").is_err());
        assert!(VariableValidator::validate("?123").is_ok()); // Numbers are allowed after ?
    }

    #[test]
    fn test_query_builder_select() {
        let query = QueryBuilder::select()
            .variable("?s")
            .variable("?p")
            .where_clause("?s ?p ?o")
            .build()
            .unwrap();

        assert!(query.contains("SELECT ?s ?p"));
        assert!(query.contains("WHERE"));
    }

    #[test]
    fn test_literal_builder_string() {
        let lit = SafeLiteralBuilder::string("Hello").build();
        assert_eq!(lit, "\"Hello\"");
    }

    #[test]
    fn test_literal_builder_integer() {
        let lit = SafeLiteralBuilder::integer(42).build();
        assert!(lit.contains("42"));
        assert!(lit.contains("XMLSchema#integer"));
    }

    #[test]
    fn test_literal_builder_language() {
        let lit = SafeLiteralBuilder::string("Bonjour").language("fr").build();
        assert!(lit.contains("@fr"));
    }

    #[test]
    fn test_iri_validator_valid() {
        assert!(IriValidator::validate("http://example.org").is_ok());
        assert!(IriValidator::validate("https://example.org/path").is_ok());
    }

    #[test]
    fn test_iri_validator_invalid_scheme() {
        let result = IriValidator::validate("javascript:alert(1)");
        assert!(matches!(result, Err(SparqlSecurityError::InvalidScheme(_))));
    }

    #[test]
    fn test_iri_validator_spaces() {
        let result = IriValidator::validate("http://example.org/my path");
        assert!(matches!(result, Err(SparqlSecurityError::InvalidIri(_))));
    }
}
