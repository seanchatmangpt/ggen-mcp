//! Comprehensive SPARQL Injection Prevention Tests
//!
//! This test suite validates the security measures implemented in the
//! sparql::injection_prevention module using chicago-tdd-tools framework.
//! It includes real-world attack scenarios and edge cases.
//!
//! Test Categories:
//! 1. Comment Injection Attacks
//! 2. Union-Based Injection
//! 3. Filter Manipulation
//! 4. IRI Injection
//! 5. Literal Escaping
//! 6. Variable Validation
//! 7. Query Structure Manipulation
//!
//! All tests follow Chicago-style TDD principles:
//! - AAA Pattern (Arrange-Act-Assert)
//! - Result-based error handling
//! - State-based verification

use chicago_tdd_tools::prelude::*;
use spreadsheet_mcp::sparql::{
    IriValidator, QueryBuilder, SafeLiteralBuilder, SparqlSanitizer, SparqlSecurityError,
    VariableValidator,
};

// ============================================================================
// Comment Injection Tests
// ============================================================================

test!(test_hash_comment_injection_blocked, {
    // Arrange: Prepare malicious input with hash comment
    let malicious = "admin' # comment out rest";

    // Act: Attempt to escape string
    let result = SparqlSanitizer::escape_string(malicious);

    // Assert: Verify comment injection is blocked
    assert_err!(&result);
    assert!(matches!(result, Err(SparqlSecurityError::CommentInjection)));
});

test!(test_double_slash_comment_injection_blocked, {
    // Arrange: Prepare malicious input with double slash comment
    let malicious = "admin' // comment";

    // Act: Attempt to escape string
    let result = SparqlSanitizer::escape_string(malicious);

    // Assert: Verify comment injection is blocked
    assert_err!(&result);
    assert!(matches!(result, Err(SparqlSecurityError::CommentInjection)));
});

test!(test_fragment_identifier_allowed_in_iri, {
    // Arrange: Fragment identifiers (#) are valid in IRIs
    let valid_iri = "http://example.org/resource#fragment";

    // Act: Validate IRI
    let result = IriValidator::validate(valid_iri);

    // Assert: Verify fragment identifier is allowed
    assert_ok!(&result);
});

// ============================================================================
// Union-Based Injection Tests
// ============================================================================

test!(test_union_injection_attempt_blocked, {
    // Arrange: Prepare UNION injection attempt
    let malicious = "' } UNION { ?s ?p ?o }";

    // Act: Attempt to escape string
    let result = SparqlSanitizer::escape_string(malicious);

    // Assert: Verify UNION injection is blocked
    assert_err!(&result);
    assert!(matches!(
        result,
        Err(SparqlSecurityError::MaliciousPattern(_))
    ));
});

test!(test_union_keyword_in_literal_blocked, {
    // Arrange: Prepare string with UNION keyword
    let malicious = "European UNION policies";

    // Act: Attempt to escape string
    let result = SparqlSanitizer::escape_string(malicious);

    // Assert: Verify UNION keyword is blocked even in normal text
    assert_err!(&result);
});

test!(test_case_insensitive_union_detection, {
    // Arrange: Prepare case-variant UNION injection
    let malicious = "test uNiOn select";

    // Act: Attempt to escape string
    let result = SparqlSanitizer::escape_string(malicious);

    // Assert: Verify case-insensitive detection
    assert_err!(&result);
});

// ============================================================================
// Filter Manipulation Tests
// ============================================================================

test!(test_filter_injection_blocked, {
    // Arrange: Prepare FILTER injection attempt
    let malicious = "' } FILTER (?age > 0) { ";

    // Act: Attempt to escape string
    let result = SparqlSanitizer::escape_string(malicious);

    // Assert: Verify FILTER injection is blocked
    assert_err!(&result);
    assert!(matches!(
        result,
        Err(SparqlSecurityError::MaliciousPattern(_))
    ));
});

test!(test_optional_injection_blocked, {
    // Arrange: Prepare OPTIONAL injection attempt
    let malicious = "' } OPTIONAL { ?s ?p ?o }";

    // Act: Attempt to escape string
    let result = SparqlSanitizer::escape_string(malicious);

    // Assert: Verify OPTIONAL injection is blocked
    assert_err!(&result);
});

// ============================================================================
// Destructive Query Tests
// ============================================================================

test!(test_insert_query_blocked, {
    // Arrange: Prepare INSERT query injection
    let malicious = "'; INSERT DATA { <http://evil.com> }";

    // Act: Attempt to escape string
    let result = SparqlSanitizer::escape_string(malicious);

    // Assert: Verify INSERT query is blocked
    assert_err!(&result);
});

test!(test_delete_query_blocked, {
    // Arrange: Prepare DELETE query injection
    let malicious = "'; DELETE WHERE { ?s ?p ?o }";

    // Act: Attempt to escape string
    let result = SparqlSanitizer::escape_string(malicious);

    // Assert: Verify DELETE query is blocked
    assert_err!(&result);
});

test!(test_drop_graph_blocked, {
    // Arrange: Prepare DROP GRAPH injection
    let malicious = "'; DROP GRAPH <http://example.org/graph>";

    // Act: Attempt to escape string
    let result = SparqlSanitizer::escape_string(malicious);

    // Assert: Verify DROP GRAPH is blocked
    assert_err!(&result);
});

test!(test_clear_graph_blocked, {
    // Arrange: Prepare CLEAR GRAPH injection
    let malicious = "'; CLEAR GRAPH <http://example.org/graph>";

    // Act: Attempt to escape string
    let result = SparqlSanitizer::escape_string(malicious);

    // Assert: Verify CLEAR GRAPH is blocked
    assert_err!(&result);
});

// ============================================================================
// IRI Injection Tests
// ============================================================================

test!(test_angle_brackets_in_iri_blocked, {
    // Arrange: Prepare IRI with angle brackets
    let malicious = "http://example.org/<script>";

    // Act: Validate IRI
    let result = IriValidator::validate(malicious);

    // Assert: Verify angle brackets are blocked
    assert_err!(&result);
});

test!(test_spaces_in_iri_blocked, {
    // Arrange: Prepare IRI with spaces
    let malicious = "http://example.org/my resource";

    // Act: Validate IRI
    let result = IriValidator::validate(malicious);

    // Assert: Verify spaces in IRI are blocked
    assert_err!(&result);
    assert!(matches!(result, Err(SparqlSecurityError::InvalidIri(_))));
});

test!(test_javascript_scheme_blocked, {
    // Arrange: Prepare javascript: scheme URI (XSS attack)
    let malicious = "javascript:alert(1)";

    // Act: Validate IRI
    let result = IriValidator::validate(malicious);

    // Assert: Verify javascript scheme is blocked
    assert_err!(&result);
    assert!(matches!(result, Err(SparqlSecurityError::InvalidScheme(_))));
});

test!(test_data_scheme_allowed, {
    // Arrange: Prepare data: scheme URI (legitimate use)
    let valid = "data:text/plain,hello";

    // Act: Validate IRI
    let result = IriValidator::validate(valid);

    // Assert: Verify data scheme is allowed
    assert_ok!(&result);
});

test!(test_valid_http_iri, {
    // Arrange: Prepare valid HTTP IRI
    let valid = "http://example.org/resource";

    // Act: Validate IRI
    let result = IriValidator::validate(valid);

    // Assert: Verify HTTP IRI is valid
    assert_ok!(&result);
});

test!(test_valid_https_iri, {
    // Arrange: Prepare valid HTTPS IRI with query string
    let valid = "https://example.org/resource?query=value";

    // Act: Validate IRI
    let result = IriValidator::validate(valid);

    // Assert: Verify HTTPS IRI with query is valid
    assert_ok!(&result);
});

test!(test_valid_urn_iri, {
    // Arrange: Prepare valid URN IRI
    let valid = "urn:isbn:0451450523";

    // Act: Validate IRI
    let result = IriValidator::validate(valid);

    // Assert: Verify URN IRI is valid
    assert_ok!(&result);
});

test!(test_relative_uri_requires_absolute, {
    // Arrange: Prepare relative URI
    let relative = "path/to/resource";

    // Act: Require absolute URI
    let result = IriValidator::require_absolute(relative);

    // Assert: Verify relative URI is rejected
    assert_err!(&result);
    assert!(matches!(
        result,
        Err(SparqlSecurityError::RelativeUriInUnsafeContext)
    ));
});

test!(test_absolute_uri_check, {
    // Arrange: Prepare test URIs
    let absolute = "http://example.org";
    let relative = "relative/path";
    let fragment = ":fragment";

    // Act: Check if URIs are absolute
    let is_abs_absolute = IriValidator::is_absolute(absolute);
    let is_abs_relative = IriValidator::is_absolute(relative);
    let is_abs_fragment = IriValidator::is_absolute(fragment);

    // Assert: Verify absolute/relative detection
    assert!(is_abs_absolute, "HTTP URI should be absolute");
    assert!(!is_abs_relative, "Relative path should not be absolute");
    assert!(!is_abs_fragment, "Fragment should not be absolute");
});

// ============================================================================
// Literal Escaping Tests
// ============================================================================

test!(test_single_quote_escaped, {
    // Arrange: String with single quote
    let input = "O'Reilly";

    // Act: Escape string
    let result = SparqlSanitizer::escape_string(input);

    // Assert: Verify single quote is properly escaped
    assert_ok!(&result);
    if let Ok(escaped) = result {
        assert_eq!(escaped, "O\\'Reilly");
    }
});

test!(test_double_quote_escaped, {
    // Arrange: String with double quotes
    let input = "He said \"hello\"";

    // Act: Escape string
    let result = SparqlSanitizer::escape_string(input);

    // Assert: Verify double quotes are properly escaped
    assert_ok!(&result);
    if let Ok(escaped) = result {
        assert_eq!(escaped, "He said \\\"hello\\\"");
    }
});

test!(test_backslash_escaped, {
    // Arrange: String with backslashes (Windows path)
    let input = "C:\\path\\file";

    // Act: Escape string
    let result = SparqlSanitizer::escape_string(input);

    // Assert: Verify backslashes are properly escaped
    assert_ok!(&result);
    if let Ok(escaped) = result {
        assert_eq!(escaped, "C:\\\\path\\\\file");
    }
});

test!(test_newline_escaped, {
    // Arrange: String with newline
    let input = "line1\nline2";

    // Act: Escape string
    let result = SparqlSanitizer::escape_string(input);

    // Assert: Verify newline is properly escaped
    assert_ok!(&result);
    if let Ok(escaped) = result {
        assert_eq!(escaped, "line1\\nline2");
    }
});

test!(test_tab_escaped, {
    // Arrange: String with tab character
    let input = "col1\tcol2";

    // Act: Escape string
    let result = SparqlSanitizer::escape_string(input);

    // Assert: Verify tab is properly escaped
    assert_ok!(&result);
    if let Ok(escaped) = result {
        assert_eq!(escaped, "col1\\tcol2");
    }
});

test!(test_carriage_return_escaped, {
    // Arrange: String with carriage return
    let input = "line1\rline2";

    // Act: Escape string
    let result = SparqlSanitizer::escape_string(input);

    // Assert: Verify carriage return is properly escaped
    assert_ok!(&result);
    if let Ok(escaped) = result {
        assert_eq!(escaped, "line1\\rline2");
    }
});

test!(test_multiple_escapes, {
    // Arrange: String with multiple special characters
    let input = "O'Reilly\n\"quote\"";

    // Act: Escape string
    let result = SparqlSanitizer::escape_string(input);

    // Assert: Verify all characters are properly escaped
    assert_ok!(&result);
    if let Ok(escaped) = result {
        assert_eq!(escaped, "O\\'Reilly\\n\\\"quote\\\"");
    }
});

// ============================================================================
// Variable Validation Tests
// ============================================================================

test!(test_valid_question_mark_variable, {
    // Arrange: Valid question mark variables
    let var1 = "?person";
    let var2 = "?name123";
    let var3 = "?my_var";

    // Act: Validate variables
    let result1 = VariableValidator::validate(var1);
    let result2 = VariableValidator::validate(var2);
    let result3 = VariableValidator::validate(var3);

    // Assert: Verify all are valid
    assert_ok!(&result1);
    assert_ok!(&result2);
    assert_ok!(&result3);
});

test!(test_valid_dollar_variable, {
    // Arrange: Valid dollar sign variables
    let var1 = "$person";
    let var2 = "$name";

    // Act: Validate variables
    let result1 = VariableValidator::validate(var1);
    let result2 = VariableValidator::validate(var2);

    // Assert: Verify both are valid
    assert_ok!(&result1);
    assert_ok!(&result2);
});

test!(test_variable_without_prefix_invalid, {
    // Arrange: Variable without prefix
    let var = "person";

    // Act: Validate variable
    let result = VariableValidator::validate(var);

    // Assert: Verify variable without prefix is invalid
    assert_err!(&result);
    assert!(matches!(
        result,
        Err(SparqlSecurityError::InvalidVariable(_))
    ));
});

test!(test_variable_with_space_invalid, {
    // Arrange: Variable with space
    let var = "?my var";

    // Act: Validate variable
    let result = VariableValidator::validate(var);

    // Assert: Verify variable with space is invalid
    assert_err!(&result);
});

test!(test_variable_with_special_chars_invalid, {
    // Arrange: Variables with special characters
    let var1 = "?name@domain";
    let var2 = "?name-var";
    let var3 = "?name.var";

    // Act: Validate variables
    let result1 = VariableValidator::validate(var1);
    let result2 = VariableValidator::validate(var2);
    let result3 = VariableValidator::validate(var3);

    // Assert: Verify all special characters are invalid
    assert_err!(&result1);
    assert_err!(&result2);
    assert_err!(&result3);
});

test!(test_variable_too_long_invalid, {
    // Arrange: Variable exceeding maximum length
    let long_var = format!("?{}", "a".repeat(128));

    // Act: Validate variable
    let result = VariableValidator::validate(&long_var);

    // Assert: Verify too-long variable is invalid
    assert_err!(&result);
    assert!(matches!(
        result,
        Err(SparqlSecurityError::VariableNameTooLong)
    ));
});

test!(test_reserved_variable_names, {
    // Arrange: Common single-letter variables (reserved)
    let vars = vec!["?s", "?p", "?o", "?x"];

    // Act & Assert: Verify all reserved variables are invalid
    for var in vars {
        let result = VariableValidator::validate(var);
        assert_err!(&result);
    }
});

test!(test_variable_with_numbers, {
    // Arrange: Variables with numbers in different positions
    let var1 = "?var123";
    let var2 = "?123var";
    let var3 = "?123";

    // Act: Validate variables
    let result1 = VariableValidator::validate(var1);
    let result2 = VariableValidator::validate(var2);
    let result3 = VariableValidator::validate(var3);

    // Assert: Verify numbers in variables are valid
    assert_ok!(&result1);
    assert_ok!(&result2);
    assert_ok!(&result3);
});

// ============================================================================
// Safe Literal Builder Tests
// ============================================================================

test!(test_string_literal_builder, {
    // Arrange: Simple string value
    let value = "Hello, World!";

    // Act: Build string literal
    let lit = SafeLiteralBuilder::string(value).build();

    // Assert: Verify proper string literal format
    assert_eq!(lit, "\"Hello, World!\"");
});

test!(test_string_literal_with_quotes, {
    // Arrange: String with apostrophe
    let value = "O'Reilly";

    // Act: Build string literal
    let lit = SafeLiteralBuilder::string(value).build();

    // Assert: Verify apostrophe is escaped
    assert!(lit.contains("O\\'Reilly"));
});

test!(test_integer_literal_builder, {
    // Arrange: Integer value
    let value = 42;

    // Act: Build integer literal
    let lit = SafeLiteralBuilder::integer(value).build();

    // Assert: Verify integer literal with datatype
    assert!(lit.contains("42"));
    assert!(lit.contains("XMLSchema#integer"));
});

test!(test_negative_integer_literal, {
    // Arrange: Negative integer value
    let value = -123;

    // Act: Build integer literal
    let lit = SafeLiteralBuilder::integer(value).build();

    // Assert: Verify negative integer is preserved
    assert!(lit.contains("-123"));
});

test!(test_decimal_literal_builder, {
    // Arrange: Decimal value
    let value = 3.14159;

    // Act: Build decimal literal
    let lit = SafeLiteralBuilder::decimal(value).build();

    // Assert: Verify decimal literal with datatype
    assert!(lit.contains("3.14159"));
    assert!(lit.contains("XMLSchema#decimal"));
});

test!(test_boolean_true_literal, {
    // Arrange: Boolean true value
    let value = true;

    // Act: Build boolean literal
    let lit = SafeLiteralBuilder::boolean(value).build();

    // Assert: Verify boolean literal with datatype
    assert!(lit.contains("true"));
    assert!(lit.contains("XMLSchema#boolean"));
});

test!(test_boolean_false_literal, {
    // Arrange: Boolean false value
    let value = false;

    // Act: Build boolean literal
    let lit = SafeLiteralBuilder::boolean(value).build();

    // Assert: Verify false literal
    assert!(lit.contains("false"));
});

test!(test_datetime_literal_builder, {
    // Arrange: DateTime ISO 8601 string
    let value = "2024-01-20T12:00:00Z";

    // Act: Build datetime literal
    let lit = SafeLiteralBuilder::datetime(value).build();

    // Assert: Verify datetime literal with datatype
    assert!(lit.contains("2024-01-20T12:00:00Z"));
    assert!(lit.contains("XMLSchema#dateTime"));
});

test!(test_language_tagged_literal, {
    // Arrange: French string
    let value = "Bonjour";
    let lang = "fr";

    // Act: Build language-tagged literal
    let lit = SafeLiteralBuilder::string(value).language(lang).build();

    // Assert: Verify language tag is applied
    assert!(lit.contains("Bonjour"));
    assert!(lit.contains("@fr"));
});

test!(test_language_tagged_literal_with_region, {
    // Arrange: English string with US region
    let value = "Hello";
    let lang = "en-US";

    // Act: Build language-tagged literal with region
    let lit = SafeLiteralBuilder::string(value).language(lang).build();

    // Assert: Verify region-specific language tag
    assert!(lit.contains("@en-US"));
});

test!(test_invalid_language_tag_ignored, {
    // Arrange: String with invalid language tag
    let value = "Test";
    let invalid_lang = "invalid123";

    // Act: Build literal with invalid language tag
    let lit = SafeLiteralBuilder::string(value).language(invalid_lang).build();

    // Assert: Verify invalid language tag is ignored
    assert!(!lit.contains("@invalid123"));
});

test!(test_custom_datatype, {
    // Arrange: String with custom datatype
    let value = "custom";
    let datatype = "http://example.org/customType";

    // Act: Build literal with custom datatype
    let lit = SafeLiteralBuilder::string(value).with_datatype(datatype).build();

    // Assert: Verify custom datatype is applied
    assert!(lit.contains("http://example.org/customType"));
});

// ============================================================================
// Number Validation Tests
// ============================================================================

test!(test_valid_integer, {
    // Arrange: Valid integer formats
    let int1 = "123";
    let int2 = "-456";
    let int3 = "+789";

    // Act: Validate numbers
    let result1 = SparqlSanitizer::escape_number(int1);
    let result2 = SparqlSanitizer::escape_number(int2);
    let result3 = SparqlSanitizer::escape_number(int3);

    // Assert: Verify all integer formats are valid
    assert_ok!(&result1);
    assert_ok!(&result2);
    assert_ok!(&result3);
});

test!(test_valid_decimal, {
    // Arrange: Valid decimal formats
    let dec1 = "3.14";
    let dec2 = "-0.5";
    let dec3 = ".5";
    let dec4 = "5.";

    // Act: Validate decimals
    let result1 = SparqlSanitizer::escape_number(dec1);
    let result2 = SparqlSanitizer::escape_number(dec2);
    let result3 = SparqlSanitizer::escape_number(dec3);
    let result4 = SparqlSanitizer::escape_number(dec4);

    // Assert: Verify all decimal formats are valid
    assert_ok!(&result1);
    assert_ok!(&result2);
    assert_ok!(&result3);
    assert_ok!(&result4);
});

test!(test_valid_scientific_notation, {
    // Arrange: Valid scientific notation formats
    let sci1 = "1e10";
    let sci2 = "1.5E-5";
    let sci3 = "-3.2e+10";

    // Act: Validate scientific notation
    let result1 = SparqlSanitizer::escape_number(sci1);
    let result2 = SparqlSanitizer::escape_number(sci2);
    let result3 = SparqlSanitizer::escape_number(sci3);

    // Assert: Verify all scientific notation formats are valid
    assert_ok!(&result1);
    assert_ok!(&result2);
    assert_ok!(&result3);
});

test!(test_invalid_number_format, {
    // Arrange: Invalid number formats
    let invalid1 = "abc";
    let invalid2 = "12.34.56";
    let invalid3 = "1e2e3";

    // Act: Validate invalid numbers
    let result1 = SparqlSanitizer::escape_number(invalid1);
    let result2 = SparqlSanitizer::escape_number(invalid2);
    let result3 = SparqlSanitizer::escape_number(invalid3);

    // Assert: Verify all invalid formats are rejected
    assert_err!(&result1);
    assert_err!(&result2);
    assert_err!(&result3);
});

// ============================================================================
// Query Builder Integration Tests
// ============================================================================

test!(test_query_builder_simple_select, {
    // Arrange: Query builder configuration
    let builder = QueryBuilder::select()
        .variable("?person")
        .variable("?name")
        .where_clause("?person a foaf:Person")
        .where_clause("?person foaf:name ?name");

    // Act: Build query
    let result = builder.build();

    // Assert: Verify query structure
    assert_ok!(&result);
    if let Ok(query) = result {
        assert!(query.contains("SELECT ?person ?name"));
        assert!(query.contains("WHERE"));
        assert!(query.contains("?person a foaf:Person"));
    }
});

test!(test_query_builder_with_prefix, {
    // Arrange: Query builder with PREFIX declaration
    let builder = QueryBuilder::select()
        .prefix("foaf", "http://xmlns.com/foaf/0.1/")
        .variable("?person")
        .where_clause("?person a foaf:Person");

    // Act: Build query
    let result = builder.build();

    // Assert: Verify PREFIX is included
    assert_ok!(&result);
    if let Ok(query) = result {
        assert!(query.contains("PREFIX foaf: <http://xmlns.com/foaf/0.1/>"));
    }
});

test!(test_query_builder_with_filter, {
    // Arrange: Query builder with FILTER clause
    let builder = QueryBuilder::select()
        .variable("?person")
        .variable("?age")
        .where_clause("?person :age ?age")
        .filter("(?age > 18)");

    // Act: Build query
    let result = builder.build();

    // Assert: Verify FILTER is included
    assert_ok!(&result);
    if let Ok(query) = result {
        assert!(query.contains("FILTER (?age > 18)"));
    }
});

test!(test_query_builder_with_order_by, {
    // Arrange: Query builder with ORDER BY clause
    let builder = QueryBuilder::select()
        .variable("?person")
        .variable("?name")
        .where_clause("?person :name ?name")
        .order_by("?name");

    // Act: Build query
    let result = builder.build();

    // Assert: Verify ORDER BY is included
    assert_ok!(&result);
    if let Ok(query) = result {
        assert!(query.contains("ORDER BY ?name"));
    }
});

test!(test_query_builder_with_limit, {
    // Arrange: Query builder with LIMIT clause
    let builder = QueryBuilder::select()
        .variable("?person")
        .where_clause("?person a :Person")
        .limit(10);

    // Act: Build query
    let result = builder.build();

    // Assert: Verify LIMIT is included
    assert_ok!(&result);
    if let Ok(query) = result {
        assert!(query.contains("LIMIT 10"));
    }
});

test!(test_query_builder_with_offset, {
    // Arrange: Query builder with LIMIT and OFFSET clauses
    let builder = QueryBuilder::select()
        .variable("?person")
        .where_clause("?person a :Person")
        .limit(10)
        .offset(20);

    // Act: Build query
    let result = builder.build();

    // Assert: Verify LIMIT and OFFSET are included
    assert_ok!(&result);
    if let Ok(query) = result {
        assert!(query.contains("LIMIT 10"));
        assert!(query.contains("OFFSET 20"));
    }
});

test!(test_query_builder_distinct, {
    // Arrange: Query builder with DISTINCT modifier
    let builder = QueryBuilder::select()
        .distinct()
        .variable("?type")
        .where_clause("?s a ?type");

    // Act: Build query
    let result = builder.build();

    // Assert: Verify DISTINCT is included
    assert_ok!(&result);
    if let Ok(query) = result {
        assert!(query.contains("SELECT DISTINCT ?type"));
    }
});

test!(test_query_builder_select_all, {
    // Arrange: Query builder with SELECT * (no variables)
    let builder = QueryBuilder::select()
        .where_clause("?s ?p ?o");

    // Act: Build query
    let result = builder.build();

    // Assert: Verify SELECT * is used
    assert_ok!(&result);
    if let Ok(query) = result {
        assert!(query.contains("SELECT *"));
    }
});

test!(test_query_builder_ask, {
    // Arrange: ASK query builder
    let builder = QueryBuilder::ask()
        .where_clause("?person a foaf:Person");

    // Act: Build ASK query
    let result = builder.build();

    // Assert: Verify ASK query structure
    assert_ok!(&result);
    if let Ok(query) = result {
        assert!(query.contains("ASK"));
        assert!(query.contains("WHERE"));
    }
});

test!(test_query_builder_describe, {
    // Arrange: DESCRIBE query builder
    let builder = QueryBuilder::describe()
        .variable("?person")
        .where_clause("?person a foaf:Person");

    // Act: Build DESCRIBE query
    let result = builder.build();

    // Assert: Verify DESCRIBE query structure
    assert_ok!(&result);
    if let Ok(query) = result {
        assert!(query.contains("DESCRIBE ?person"));
    }
});

test!(test_query_builder_construct, {
    // Arrange: CONSTRUCT query builder
    let builder = QueryBuilder::construct()
        .where_clause("?s :name ?name")
        .where_clause("?s :age ?age");

    // Act: Build CONSTRUCT query
    let result = builder.build();

    // Assert: Verify CONSTRUCT query structure
    assert_ok!(&result);
    if let Ok(query) = result {
        assert!(query.contains("CONSTRUCT"));
    }
});

// ============================================================================
// Query Structure Manipulation Tests
// ============================================================================

test!(test_closing_brace_blocked, {
    // Arrange: String with closing brace (structure manipulation attempt)
    let malicious = "test } DROP ALL";

    // Act: Attempt to escape string
    let result = SparqlSanitizer::escape_string(malicious);

    // Assert: Verify structure manipulation is blocked
    assert_err!(&result);
    assert!(matches!(
        result,
        Err(SparqlSecurityError::StructureManipulation)
    ));
});

test!(test_opening_brace_blocked, {
    // Arrange: String with opening brace (structure manipulation attempt)
    let malicious = "test { SELECT";

    // Act: Attempt to escape string
    let result = SparqlSanitizer::escape_string(malicious);

    // Assert: Verify structure manipulation is blocked
    assert_err!(&result);
});

test!(test_query_builder_rejects_destructive_where_clause, {
    // Arrange: Query builder with destructive WHERE clause
    let builder = QueryBuilder::select()
        .variable("?person")
        .where_clause("?person a :Person")
        .where_clause("DROP GRAPH <http://example.org>"); // Should be sanitized

    // Act: Build query
    let result = builder.build();

    // Assert: Verify DROP is not in final query
    assert_ok!(&result);
    if let Ok(query) = result {
        assert!(!query.contains("DROP"));
    }
});

// ============================================================================
// Real-World Attack Scenarios
// ============================================================================

test!(test_sql_injection_style_attack, {
    // Arrange: Classic SQL injection pattern adapted for SPARQL
    let malicious = "' OR 1=1 --";

    // Act: Attempt to escape string
    let result = SparqlSanitizer::escape_string(malicious);

    // Assert: Should fail on comment injection
    assert_err!(&result);
});

test!(test_tautology_injection, {
    // Arrange: Tautology injection (always true condition)
    let malicious = "' } FILTER(1=1) { '";

    // Act: Attempt to escape string
    let result = SparqlSanitizer::escape_string(malicious);

    // Assert: Verify tautology injection is blocked
    assert_err!(&result);
});

test!(test_blind_injection_timing, {
    // Arrange: Blind injection using timing attack (SLEEP)
    let malicious = "' } FILTER(SLEEP(5)) { '";

    // Act: Attempt to escape string
    let result = SparqlSanitizer::escape_string(malicious);

    // Assert: Verify timing attack is blocked
    assert_err!(&result);
});

test!(test_information_disclosure_via_error, {
    // Arrange: Information disclosure attempt via password query
    let malicious = "' } SELECT ?password WHERE { ?user :password ?password } { '";

    // Act: Attempt to escape string
    let result = SparqlSanitizer::escape_string(malicious);

    // Assert: Verify information disclosure is blocked
    assert_err!(&result);
});

test!(test_safe_user_input_integration, {
    // Arrange: Legitimate user input with apostrophe
    let user_input = "John O'Brien";
    let safe_literal = SafeLiteralBuilder::string(user_input).build();

    let builder = QueryBuilder::select()
        .prefix("foaf", "http://xmlns.com/foaf/0.1/")
        .variable("?person")
        .where_clause("?person a foaf:Person")
        .where_clause(&format!("?person foaf:name {}", safe_literal));

    // Act: Build query with safe user input
    let result = builder.build();

    // Assert: Verify query is safely constructed
    assert_ok!(&result);
    if let Ok(query) = result {
        // Query should contain escaped apostrophe
        assert!(query.contains("John O\\'Brien"));
        // Should not allow query structure manipulation
        assert!(query.contains("WHERE"));
    }
});

test!(test_parameterized_query_with_multiple_inputs, {
    // Arrange: Multiple safe inputs of different types
    let name = "Alice";
    let age = 30;
    let city = "Paris";

    let name_lit = SafeLiteralBuilder::string(name).build();
    let age_lit = SafeLiteralBuilder::integer(age).build();
    let city_lit = SafeLiteralBuilder::string(city).language("en").build();

    let builder = QueryBuilder::select()
        .variable("?person")
        .where_clause(&("?person :name ".to_string() + &name_lit))
        .where_clause(&("?person :age ".to_string() + &age_lit))
        .where_clause(&("?person :city ".to_string() + &city_lit));

    // Act: Build parameterized query
    let result = builder.build();

    // Assert: Verify query builds successfully
    assert_ok!(&result);
});

// ============================================================================
// Edge Cases and Boundary Tests
// ============================================================================

test!(test_empty_string_literal, {
    // Arrange: Empty string
    let value = "";

    // Act: Build empty string literal
    let lit = SafeLiteralBuilder::string(value).build();

    // Assert: Verify empty literal format
    assert_eq!(lit, "\"\"");
});

test!(test_empty_iri_invalid, {
    // Arrange: Empty IRI string
    let iri = "";

    // Act: Validate empty IRI
    let result = IriValidator::validate(iri);

    // Assert: Verify empty IRI is invalid
    assert_err!(&result);
});

test!(test_unicode_in_literal, {
    // Arrange: Japanese Unicode string
    let value = "こんにちは";
    let lang = "ja";

    // Act: Build Unicode literal with language tag
    let lit = SafeLiteralBuilder::string(value).language(lang).build();

    // Assert: Verify Unicode is preserved with language tag
    assert!(lit.contains("こんにちは"));
    assert!(lit.contains("@ja"));
});

test!(test_very_long_valid_string, {
    // Arrange: Very long string (10000 characters)
    let long_string = "a".repeat(10000);

    // Act: Escape long string
    let result = SparqlSanitizer::escape_string(&long_string);

    // Assert: Verify long string is accepted
    assert_ok!(&result);
});

test!(test_null_byte_in_string, {
    // Arrange: String with null byte
    let with_null = "test\0string";

    // Act: Escape string with null byte
    let result = SparqlSanitizer::escape_string(with_null);

    // Assert: Verify null byte handling
    assert_ok!(&result);
});

test!(test_iri_escape_special_chars, {
    // Arrange: IRI with pipe character
    let iri = "http://example.org/resource|pipe";

    // Act: Escape IRI
    let result = SparqlSanitizer::escape_iri(iri);

    // Assert: Verify special characters are percent-encoded
    assert_ok!(&result);
    if let Ok(escaped) = result {
        assert!(escaped.contains("%7C")); // Pipe encoded
    }
});
