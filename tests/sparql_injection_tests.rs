//! Comprehensive SPARQL Injection Prevention Tests
//!
//! This test suite validates the security measures implemented in the
//! sparql::injection_prevention module. It includes real-world attack
//! scenarios and edge cases.
//!
//! Test Categories:
//! 1. Comment Injection Attacks
//! 2. Union-Based Injection
//! 3. Filter Manipulation
//! 4. IRI Injection
//! 5. Literal Escaping
//! 6. Variable Validation
//! 7. Query Structure Manipulation

use spreadsheet_mcp::sparql::{
    IriValidator, QueryBuilder, SafeLiteralBuilder, SparqlSanitizer,
    SparqlSecurityError, VariableValidator,
};

// ============================================================================
// Comment Injection Tests
// ============================================================================

#[test]
fn test_hash_comment_injection_blocked() {
    let malicious = "admin' # comment out rest";
    let result = SparqlSanitizer::escape_string(malicious);
    assert!(result.is_err());
    assert!(matches!(result, Err(SparqlSecurityError::CommentInjection)));
}

#[test]
fn test_double_slash_comment_injection_blocked() {
    let malicious = "admin' // comment";
    let result = SparqlSanitizer::escape_string(malicious);
    assert!(result.is_err());
    assert!(matches!(result, Err(SparqlSecurityError::CommentInjection)));
}

#[test]
fn test_fragment_identifier_allowed_in_iri() {
    // Fragment identifiers (#) are valid in IRIs
    let valid_iri = "http://example.org/resource#fragment";
    let result = IriValidator::validate(valid_iri);
    assert!(result.is_ok());
}

// ============================================================================
// Union-Based Injection Tests
// ============================================================================

#[test]
fn test_union_injection_attempt_blocked() {
    let malicious = "' } UNION { ?s ?p ?o }";
    let result = SparqlSanitizer::escape_string(malicious);
    assert!(result.is_err());
    assert!(matches!(result, Err(SparqlSecurityError::MaliciousPattern(_))));
}

#[test]
fn test_union_keyword_in_literal_blocked() {
    let malicious = "European UNION policies";
    let result = SparqlSanitizer::escape_string(malicious);
    assert!(result.is_err());
}

#[test]
fn test_case_insensitive_union_detection() {
    let malicious = "test uNiOn select";
    let result = SparqlSanitizer::escape_string(malicious);
    assert!(result.is_err());
}

// ============================================================================
// Filter Manipulation Tests
// ============================================================================

#[test]
fn test_filter_injection_blocked() {
    let malicious = "' } FILTER (?age > 0) { ";
    let result = SparqlSanitizer::escape_string(malicious);
    assert!(result.is_err());
    assert!(matches!(result, Err(SparqlSecurityError::MaliciousPattern(_))));
}

#[test]
fn test_optional_injection_blocked() {
    let malicious = "' } OPTIONAL { ?s ?p ?o }";
    let result = SparqlSanitizer::escape_string(malicious);
    assert!(result.is_err());
}

// ============================================================================
// Destructive Query Tests
// ============================================================================

#[test]
fn test_insert_query_blocked() {
    let malicious = "'; INSERT DATA { <http://evil.com> }";
    let result = SparqlSanitizer::escape_string(malicious);
    assert!(result.is_err());
}

#[test]
fn test_delete_query_blocked() {
    let malicious = "'; DELETE WHERE { ?s ?p ?o }";
    let result = SparqlSanitizer::escape_string(malicious);
    assert!(result.is_err());
}

#[test]
fn test_drop_graph_blocked() {
    let malicious = "'; DROP GRAPH <http://example.org/graph>";
    let result = SparqlSanitizer::escape_string(malicious);
    assert!(result.is_err());
}

#[test]
fn test_clear_graph_blocked() {
    let malicious = "'; CLEAR GRAPH <http://example.org/graph>";
    let result = SparqlSanitizer::escape_string(malicious);
    assert!(result.is_err());
}

// ============================================================================
// IRI Injection Tests
// ============================================================================

#[test]
fn test_angle_brackets_in_iri_blocked() {
    let malicious = "http://example.org/<script>";
    let result = IriValidator::validate(malicious);
    assert!(result.is_err());
}

#[test]
fn test_spaces_in_iri_blocked() {
    let malicious = "http://example.org/my resource";
    let result = IriValidator::validate(malicious);
    assert!(result.is_err());
    assert!(matches!(result, Err(SparqlSecurityError::InvalidIri(_))));
}

#[test]
fn test_javascript_scheme_blocked() {
    let malicious = "javascript:alert(1)";
    let result = IriValidator::validate(malicious);
    assert!(result.is_err());
    assert!(matches!(result, Err(SparqlSecurityError::InvalidScheme(_))));
}

#[test]
fn test_data_scheme_allowed() {
    let valid = "data:text/plain,hello";
    let result = IriValidator::validate(valid);
    assert!(result.is_ok());
}

#[test]
fn test_valid_http_iri() {
    let valid = "http://example.org/resource";
    let result = IriValidator::validate(valid);
    assert!(result.is_ok());
}

#[test]
fn test_valid_https_iri() {
    let valid = "https://example.org/resource?query=value";
    let result = IriValidator::validate(valid);
    assert!(result.is_ok());
}

#[test]
fn test_valid_urn_iri() {
    let valid = "urn:isbn:0451450523";
    let result = IriValidator::validate(valid);
    assert!(result.is_ok());
}

#[test]
fn test_relative_uri_requires_absolute() {
    let relative = "path/to/resource";
    let result = IriValidator::require_absolute(relative);
    assert!(result.is_err());
    assert!(matches!(result, Err(SparqlSecurityError::RelativeUriInUnsafeContext)));
}

#[test]
fn test_absolute_uri_check() {
    assert!(IriValidator::is_absolute("http://example.org"));
    assert!(!IriValidator::is_absolute("relative/path"));
    assert!(!IriValidator::is_absolute(":fragment"));
}

// ============================================================================
// Literal Escaping Tests
// ============================================================================

#[test]
fn test_single_quote_escaped() {
    let result = SparqlSanitizer::escape_string("O'Reilly").unwrap();
    assert_eq!(result, "O\\'Reilly");
}

#[test]
fn test_double_quote_escaped() {
    let result = SparqlSanitizer::escape_string("He said \"hello\"").unwrap();
    assert_eq!(result, "He said \\\"hello\\\"");
}

#[test]
fn test_backslash_escaped() {
    let result = SparqlSanitizer::escape_string("C:\\path\\file").unwrap();
    assert_eq!(result, "C:\\\\path\\\\file");
}

#[test]
fn test_newline_escaped() {
    let result = SparqlSanitizer::escape_string("line1\nline2").unwrap();
    assert_eq!(result, "line1\\nline2");
}

#[test]
fn test_tab_escaped() {
    let result = SparqlSanitizer::escape_string("col1\tcol2").unwrap();
    assert_eq!(result, "col1\\tcol2");
}

#[test]
fn test_carriage_return_escaped() {
    let result = SparqlSanitizer::escape_string("line1\rline2").unwrap();
    assert_eq!(result, "line1\\rline2");
}

#[test]
fn test_multiple_escapes() {
    let result = SparqlSanitizer::escape_string("O'Reilly\n\"quote\"").unwrap();
    assert_eq!(result, "O\\'Reilly\\n\\\"quote\\\"");
}

// ============================================================================
// Variable Validation Tests
// ============================================================================

#[test]
fn test_valid_question_mark_variable() {
    assert!(VariableValidator::validate("?person").is_ok());
    assert!(VariableValidator::validate("?name123").is_ok());
    assert!(VariableValidator::validate("?my_var").is_ok());
}

#[test]
fn test_valid_dollar_variable() {
    assert!(VariableValidator::validate("$person").is_ok());
    assert!(VariableValidator::validate("$name").is_ok());
}

#[test]
fn test_variable_without_prefix_invalid() {
    let result = VariableValidator::validate("person");
    assert!(result.is_err());
    assert!(matches!(result, Err(SparqlSecurityError::InvalidVariable(_))));
}

#[test]
fn test_variable_with_space_invalid() {
    let result = VariableValidator::validate("?my var");
    assert!(result.is_err());
}

#[test]
fn test_variable_with_special_chars_invalid() {
    assert!(VariableValidator::validate("?name@domain").is_err());
    assert!(VariableValidator::validate("?name-var").is_err());
    assert!(VariableValidator::validate("?name.var").is_err());
}

#[test]
fn test_variable_too_long_invalid() {
    let long_var = format!("?{}", "a".repeat(128));
    let result = VariableValidator::validate(&long_var);
    assert!(result.is_err());
    assert!(matches!(result, Err(SparqlSecurityError::VariableNameTooLong)));
}

#[test]
fn test_reserved_variable_names() {
    // Common single-letter variables are reserved
    assert!(VariableValidator::validate("?s").is_err());
    assert!(VariableValidator::validate("?p").is_err());
    assert!(VariableValidator::validate("?o").is_err());
    assert!(VariableValidator::validate("?x").is_err());
}

#[test]
fn test_variable_with_numbers() {
    assert!(VariableValidator::validate("?var123").is_ok());
    assert!(VariableValidator::validate("?123var").is_ok());
    assert!(VariableValidator::validate("?123").is_ok());
}

// ============================================================================
// Safe Literal Builder Tests
// ============================================================================

#[test]
fn test_string_literal_builder() {
    let lit = SafeLiteralBuilder::string("Hello, World!").build();
    assert_eq!(lit, "\"Hello, World!\"");
}

#[test]
fn test_string_literal_with_quotes() {
    let lit = SafeLiteralBuilder::string("O'Reilly").build();
    assert!(lit.contains("O\\'Reilly"));
}

#[test]
fn test_integer_literal_builder() {
    let lit = SafeLiteralBuilder::integer(42).build();
    assert!(lit.contains("42"));
    assert!(lit.contains("XMLSchema#integer"));
}

#[test]
fn test_negative_integer_literal() {
    let lit = SafeLiteralBuilder::integer(-123).build();
    assert!(lit.contains("-123"));
}

#[test]
fn test_decimal_literal_builder() {
    let lit = SafeLiteralBuilder::decimal(3.14159).build();
    assert!(lit.contains("3.14159"));
    assert!(lit.contains("XMLSchema#decimal"));
}

#[test]
fn test_boolean_true_literal() {
    let lit = SafeLiteralBuilder::boolean(true).build();
    assert!(lit.contains("true"));
    assert!(lit.contains("XMLSchema#boolean"));
}

#[test]
fn test_boolean_false_literal() {
    let lit = SafeLiteralBuilder::boolean(false).build();
    assert!(lit.contains("false"));
}

#[test]
fn test_datetime_literal_builder() {
    let lit = SafeLiteralBuilder::datetime("2024-01-20T12:00:00Z").build();
    assert!(lit.contains("2024-01-20T12:00:00Z"));
    assert!(lit.contains("XMLSchema#dateTime"));
}

#[test]
fn test_language_tagged_literal() {
    let lit = SafeLiteralBuilder::string("Bonjour").language("fr").build();
    assert!(lit.contains("Bonjour"));
    assert!(lit.contains("@fr"));
}

#[test]
fn test_language_tagged_literal_with_region() {
    let lit = SafeLiteralBuilder::string("Hello").language("en-US").build();
    assert!(lit.contains("@en-US"));
}

#[test]
fn test_invalid_language_tag_ignored() {
    let lit = SafeLiteralBuilder::string("Test").language("invalid123").build();
    // Invalid language tags should be ignored
    assert!(!lit.contains("@invalid123"));
}

#[test]
fn test_custom_datatype() {
    let lit = SafeLiteralBuilder::string("custom")
        .with_datatype("http://example.org/customType")
        .build();
    assert!(lit.contains("http://example.org/customType"));
}

// ============================================================================
// Number Validation Tests
// ============================================================================

#[test]
fn test_valid_integer() {
    assert!(SparqlSanitizer::escape_number("123").is_ok());
    assert!(SparqlSanitizer::escape_number("-456").is_ok());
    assert!(SparqlSanitizer::escape_number("+789").is_ok());
}

#[test]
fn test_valid_decimal() {
    assert!(SparqlSanitizer::escape_number("3.14").is_ok());
    assert!(SparqlSanitizer::escape_number("-0.5").is_ok());
    assert!(SparqlSanitizer::escape_number(".5").is_ok());
    assert!(SparqlSanitizer::escape_number("5.").is_ok());
}

#[test]
fn test_valid_scientific_notation() {
    assert!(SparqlSanitizer::escape_number("1e10").is_ok());
    assert!(SparqlSanitizer::escape_number("1.5E-5").is_ok());
    assert!(SparqlSanitizer::escape_number("-3.2e+10").is_ok());
}

#[test]
fn test_invalid_number_format() {
    assert!(SparqlSanitizer::escape_number("abc").is_err());
    assert!(SparqlSanitizer::escape_number("12.34.56").is_err());
    assert!(SparqlSanitizer::escape_number("1e2e3").is_err());
}

// ============================================================================
// Query Builder Integration Tests
// ============================================================================

#[test]
fn test_query_builder_simple_select() {
    let query = QueryBuilder::select()
        .variable("?person")
        .variable("?name")
        .where_clause("?person a foaf:Person")
        .where_clause("?person foaf:name ?name")
        .build()
        .unwrap();

    assert!(query.contains("SELECT ?person ?name"));
    assert!(query.contains("WHERE"));
    assert!(query.contains("?person a foaf:Person"));
}

#[test]
fn test_query_builder_with_prefix() {
    let query = QueryBuilder::select()
        .prefix("foaf", "http://xmlns.com/foaf/0.1/")
        .variable("?person")
        .where_clause("?person a foaf:Person")
        .build()
        .unwrap();

    assert!(query.contains("PREFIX foaf: <http://xmlns.com/foaf/0.1/>"));
}

#[test]
fn test_query_builder_with_filter() {
    let query = QueryBuilder::select()
        .variable("?person")
        .variable("?age")
        .where_clause("?person :age ?age")
        .filter("(?age > 18)")
        .build()
        .unwrap();

    assert!(query.contains("FILTER (?age > 18)"));
}

#[test]
fn test_query_builder_with_order_by() {
    let query = QueryBuilder::select()
        .variable("?person")
        .variable("?name")
        .where_clause("?person :name ?name")
        .order_by("?name")
        .build()
        .unwrap();

    assert!(query.contains("ORDER BY ?name"));
}

#[test]
fn test_query_builder_with_limit() {
    let query = QueryBuilder::select()
        .variable("?person")
        .where_clause("?person a :Person")
        .limit(10)
        .build()
        .unwrap();

    assert!(query.contains("LIMIT 10"));
}

#[test]
fn test_query_builder_with_offset() {
    let query = QueryBuilder::select()
        .variable("?person")
        .where_clause("?person a :Person")
        .limit(10)
        .offset(20)
        .build()
        .unwrap();

    assert!(query.contains("LIMIT 10"));
    assert!(query.contains("OFFSET 20"));
}

#[test]
fn test_query_builder_distinct() {
    let query = QueryBuilder::select()
        .distinct()
        .variable("?type")
        .where_clause("?s a ?type")
        .build()
        .unwrap();

    assert!(query.contains("SELECT DISTINCT ?type"));
}

#[test]
fn test_query_builder_select_all() {
    let query = QueryBuilder::select()
        .where_clause("?s ?p ?o")
        .build()
        .unwrap();

    assert!(query.contains("SELECT *"));
}

#[test]
fn test_query_builder_ask() {
    let query = QueryBuilder::ask()
        .where_clause("?person a foaf:Person")
        .build()
        .unwrap();

    assert!(query.contains("ASK"));
    assert!(query.contains("WHERE"));
}

#[test]
fn test_query_builder_describe() {
    let query = QueryBuilder::describe()
        .variable("?person")
        .where_clause("?person a foaf:Person")
        .build()
        .unwrap();

    assert!(query.contains("DESCRIBE ?person"));
}

#[test]
fn test_query_builder_construct() {
    let query = QueryBuilder::construct()
        .where_clause("?s :name ?name")
        .where_clause("?s :age ?age")
        .build()
        .unwrap();

    assert!(query.contains("CONSTRUCT"));
}

// ============================================================================
// Query Structure Manipulation Tests
// ============================================================================

#[test]
fn test_closing_brace_blocked() {
    let malicious = "test } DROP ALL";
    let result = SparqlSanitizer::escape_string(malicious);
    assert!(result.is_err());
    assert!(matches!(result, Err(SparqlSecurityError::StructureManipulation)));
}

#[test]
fn test_opening_brace_blocked() {
    let malicious = "test { SELECT";
    let result = SparqlSanitizer::escape_string(malicious);
    assert!(result.is_err());
}

#[test]
fn test_query_builder_rejects_destructive_where_clause() {
    let query = QueryBuilder::select()
        .variable("?person")
        .where_clause("?person a :Person")
        .where_clause("DROP GRAPH <http://example.org>")  // Should be ignored
        .build()
        .unwrap();

    assert!(!query.contains("DROP"));
}

// ============================================================================
// Real-World Attack Scenarios
// ============================================================================

#[test]
fn test_sql_injection_style_attack() {
    // Classic SQL injection pattern adapted for SPARQL
    let malicious = "' OR 1=1 --";
    let result = SparqlSanitizer::escape_string(malicious);
    // Should fail on comment injection
    assert!(result.is_err());
}

#[test]
fn test_tautology_injection() {
    let malicious = "' } FILTER(1=1) { '";
    let result = SparqlSanitizer::escape_string(malicious);
    assert!(result.is_err());
}

#[test]
fn test_blind_injection_timing() {
    let malicious = "' } FILTER(SLEEP(5)) { '";
    let result = SparqlSanitizer::escape_string(malicious);
    assert!(result.is_err());
}

#[test]
fn test_information_disclosure_via_error() {
    let malicious = "' } SELECT ?password WHERE { ?user :password ?password } { '";
    let result = SparqlSanitizer::escape_string(malicious);
    assert!(result.is_err());
}

#[test]
fn test_safe_user_input_integration() {
    // Demonstrate safe handling of user input
    let user_input = "John O'Brien";
    
    let safe_literal = SafeLiteralBuilder::string(user_input).build();
    
    let query = QueryBuilder::select()
        .prefix("foaf", "http://xmlns.com/foaf/0.1/")
        .variable("?person")
        .where_clause("?person a foaf:Person")
        .where_clause(&format!("?person foaf:name {}", safe_literal))
        .build()
        .unwrap();
    
    // Query should contain escaped apostrophe
    assert!(query.contains("John O\\'Brien"));
    // Should not allow query structure manipulation
    assert!(query.contains("WHERE"));
}

#[test]
fn test_parameterized_query_with_multiple_inputs() {
    let name = "Alice";
    let age = 30;
    let city = "Paris";
    
    let name_lit = SafeLiteralBuilder::string(name).build();
    let age_lit = SafeLiteralBuilder::integer(age).build();
    let city_lit = SafeLiteralBuilder::string(city).language("en").build();
    
    let query = QueryBuilder::select()
        .variable("?person")
        .where_clause("?person :name " + &name_lit)
        .where_clause("?person :age " + &age_lit)
        .where_clause("?person :city " + &city_lit)
        .build();
    
    assert!(query.is_ok());
}

// ============================================================================
// Edge Cases and Boundary Tests
// ============================================================================

#[test]
fn test_empty_string_literal() {
    let lit = SafeLiteralBuilder::string("").build();
    assert_eq!(lit, "\"\"");
}

#[test]
fn test_empty_iri_invalid() {
    let result = IriValidator::validate("");
    assert!(result.is_err());
}

#[test]
fn test_unicode_in_literal() {
    let lit = SafeLiteralBuilder::string("こんにちは").language("ja").build();
    assert!(lit.contains("こんにちは"));
    assert!(lit.contains("@ja"));
}

#[test]
fn test_very_long_valid_string() {
    let long_string = "a".repeat(10000);
    let result = SparqlSanitizer::escape_string(&long_string);
    assert!(result.is_ok());
}

#[test]
fn test_null_byte_in_string() {
    let with_null = "test\0string";
    let result = SparqlSanitizer::escape_string(with_null);
    // Should successfully escape
    assert!(result.is_ok());
}

#[test]
fn test_iri_escape_special_chars() {
    let iri = "http://example.org/resource|pipe";
    let result = SparqlSanitizer::escape_iri(iri);
    assert!(result.is_ok());
    assert!(result.unwrap().contains("%7C")); // Pipe encoded
}
