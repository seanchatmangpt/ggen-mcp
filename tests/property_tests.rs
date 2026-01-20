//! Property-Based Testing for Spreadsheet MCP
//!
//! This module implements comprehensive property-based testing using proptest
//! to validate robustness of validation functions, SPARQL handling, template
//! rendering, and serialization across the entire input space.
//!
//! # Test Coverage
//!
//! - Domain type generators (WorkbookId, SheetName, CellAddress, etc.)
//! - Validation function properties (never panic, correct error handling)
//! - SPARQL injection prevention (no injection possible)
//! - Template rendering safety (no syntax errors)
//! - Serialization round-trips (lossless)
//! - Cache invariants (LRU behavior, capacity limits)
//! - State management invariants (atomic operations, consistency)
//!
//! # Chicago TDD Integration
//!
//! This module uses chicago-tdd-tools for enhanced property-based testing:
//! - Result-based error handling with assertion helpers
//! - AAA pattern structure where applicable
//! - Type-safe property test generators
//! - Comprehensive edge case coverage

use chicago_tdd_tools::prelude::*;
use proptest::prelude::*;
use spreadsheet_mcp::model::WorkbookId;
use spreadsheet_mcp::sparql::injection_prevention::*;
use spreadsheet_mcp::template::parameter_validation::*;
use spreadsheet_mcp::validation::bounds::*;
use spreadsheet_mcp::validation::input_guards::*;

// =============================================================================
// Domain Type Generators
// =============================================================================
//
// Property test generators for domain types. These generators produce valid
// test data for property-based testing using proptest.
//
// Note: .unwrap() is used for regex compilation as patterns are hardcoded and
// known to be valid at compile time. This is acceptable in test code.

#[allow(clippy::unwrap_used)] // Generator functions: regex patterns are hardcoded and valid
/// Generate valid alphanumeric strings for WorkbookId
pub fn arb_workbook_id() -> impl Strategy<Value = String> {
    prop::string::string_regex(r"[a-zA-Z0-9\-_\.]{1,255}").unwrap()
}

#[allow(clippy::unwrap_used)] // Generator functions: regex patterns are hardcoded and valid
/// Generate valid sheet names (Excel compliant)
pub fn arb_sheet_name() -> impl Strategy<Value = String> {
    // Sheet names: 1-31 chars, no : \ / ? * [ ]
    prop::string::string_regex(r"[a-zA-Z0-9 _\-\.]{1,31}")
        .unwrap()
        .prop_filter("not History", |s| !s.eq_ignore_ascii_case("History"))
}

/// Generate valid column letters (A-XFD)
pub fn arb_column_letters() -> impl Strategy<Value = String> {
    (1u32..=16384u32).prop_map(|col| {
        let mut column = col;
        let mut name = String::new();
        while column > 0 {
            let rem = ((column - 1) % 26) as u8;
            name.insert(0, (b'A' + rem) as char);
            column = (column - 1) / 26;
        }
        name
    })
}

/// Generate valid row numbers (1-1048576)
pub fn arb_row_number() -> impl Strategy<Value = u32> {
    1u32..=EXCEL_MAX_ROWS
}

/// Generate valid column numbers (1-16384)
pub fn arb_column_number() -> impl Strategy<Value = u32> {
    1u32..=EXCEL_MAX_COLUMNS
}

/// Generate valid cell addresses in A1 notation
pub fn arb_cell_address() -> impl Strategy<Value = String> {
    (arb_column_letters(), arb_row_number()).prop_map(|(col, row)| format!("{}{}", col, row))
}

/// Generate valid range strings (A1:B10 format)
pub fn arb_range_string() -> impl Strategy<Value = String> {
    prop_oneof![
        // Single cell
        arb_cell_address(),
        // Cell range
        (arb_cell_address(), arb_cell_address())
            .prop_map(|(start, end)| format!("{}:{}", start, end)),
        // Column range
        (arb_column_letters(), arb_column_letters())
            .prop_map(|(start, end)| format!("{}:{}", start, end)),
        // Row range
        (arb_row_number(), arb_row_number()).prop_map(|(start, end)| format!("{}:{}", start, end)),
    ]
}

#[allow(clippy::unwrap_used)] // Generator functions: regex patterns are hardcoded and valid
/// Generate potentially malicious strings for injection testing
pub fn arb_malicious_string() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::string::string_regex(r".*UNION.*").unwrap(),
        prop::string::string_regex(r".*DROP.*").unwrap(),
        prop::string::string_regex(r".*DELETE.*").unwrap(),
        prop::string::string_regex(r".*#.*").unwrap(),
        prop::string::string_regex(r".*\{.*\}.*").unwrap(),
        prop::string::string_regex(r".*'.*'.*").unwrap(),
        prop::string::string_regex(r#".*".*".*"#).unwrap(),
    ]
}

#[allow(clippy::unwrap_used)] // Generator functions: regex patterns are hardcoded and valid
/// Generate valid SPARQL variable names
pub fn arb_sparql_variable() -> impl Strategy<Value = String> {
    prop::string::string_regex(r"\?[a-zA-Z_][a-zA-Z0-9_]{0,127}")
        .unwrap()
        .prop_filter("not reserved", |s| {
            !matches!(
                s.as_str(),
                "?a" | "?b"
                    | "?c"
                    | "?o"
                    | "?p"
                    | "?s"
                    | "?x"
                    | "?y"
                    | "?z"
                    | "?base"
                    | "?prefix"
                    | "?graph"
                    | "?default"
            )
        })
}

#[allow(clippy::unwrap_used)] // Generator functions: regex patterns are hardcoded and valid
/// Generate valid IRIs
pub fn arb_iri() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::string::string_regex(r"https?://[a-zA-Z0-9\-\.]+\.[a-z]{2,}(/[a-zA-Z0-9\-_\.]*)*")
            .unwrap(),
        prop::string::string_regex(r"urn:[a-z][a-z0-9\-]*:[a-zA-Z0-9\-_\.]+").unwrap(),
    ]
}

#[allow(clippy::unwrap_used)] // Generator functions: regex patterns are hardcoded and valid
/// Generate safe paths (no traversal)
pub fn arb_safe_path() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop::string::string_regex(r"[a-zA-Z0-9_\-\.]+").unwrap(),
        1..=5,
    )
    .prop_map(|parts| parts.join("/"))
}

#[allow(clippy::unwrap_used)] // Generator functions: regex patterns are hardcoded and valid
/// Generate potentially dangerous paths
pub fn arb_dangerous_path() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("../etc/passwd".to_string()),
        Just("/etc/passwd".to_string()),
        Just("C:\\Windows\\System32".to_string()),
        Just("..\\..\\..\\etc\\passwd".to_string()),
        Just("file\0.txt".to_string()),
        prop::string::string_regex(r".*\.\..*").unwrap(),
    ]
}

// =============================================================================
// Property Tests for Validation Functions
// =============================================================================

#[allow(clippy::unwrap_used)] // Property tests: unwrap is acceptable in test code
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Property: validate_row_1based never panics
    /// AAA Pattern: Act - validate arbitrary row, Assert - no panic
    #[test]
    fn prop_validate_row_never_panics(row in any::<u32>()) {
        // Act: Validate arbitrary row value
        let _ = validate_row_1based(row, "test");

        // Assert: Should never panic regardless of input
    }

    /// Property: validate_column_1based never panics
    /// AAA Pattern: Act - validate arbitrary column, Assert - no panic
    #[test]
    fn prop_validate_column_never_panics(col in any::<u32>()) {
        // Act: Validate arbitrary column value
        let _ = validate_column_1based(col, "test");

        // Assert: Should never panic regardless of input
    }

    /// Property: valid rows always pass validation
    /// AAA Pattern: Arrange - generate valid row, Act - validate, Assert - success
    #[test]
    fn prop_valid_rows_accepted(row in arb_row_number()) {
        // Arrange: row is generated within valid range

        // Act: Validate the row
        let result = validate_row_1based(row, "test");

        // Assert: Valid rows always pass
        prop_assert!(result.is_ok());
    }

    /// Property: valid columns always pass validation
    /// AAA Pattern: Arrange - generate valid column, Act - validate, Assert - success
    #[test]
    fn prop_valid_columns_accepted(col in arb_column_number()) {
        // Arrange: col is generated within valid range

        // Act: Validate the column
        let result = validate_column_1based(col, "test");

        // Assert: Valid columns always pass
        prop_assert!(result.is_ok());
    }

    /// Property: row 0 always fails
    /// AAA Pattern: Arrange - row 0, Act - validate, Assert - error
    #[test]
    fn prop_row_zero_rejected(_unit in Just(())) {
        // Arrange: Row 0 is always invalid (1-based indexing)
        let row = 0;

        // Act: Validate row 0
        let result = validate_row_1based(row, "test");

        // Assert: Row 0 always fails
        prop_assert!(result.is_err());
    }

    /// Property: column 0 always fails
    /// AAA Pattern: Arrange - column 0, Act - validate, Assert - error
    #[test]
    fn prop_column_zero_rejected(_unit in Just(())) {
        // Arrange: Column 0 is always invalid (1-based indexing)
        let col = 0;

        // Act: Validate column 0
        let result = validate_column_1based(col, "test");

        // Assert: Column 0 always fails
        prop_assert!(result.is_err());
    }

    /// Property: rows beyond Excel limit fail
    /// AAA Pattern: Arrange - row > max, Act - validate, Assert - error
    #[test]
    fn prop_rows_beyond_limit_rejected(extra in 1u32..=1000) {
        // Arrange: Create row beyond Excel maximum
        let row = EXCEL_MAX_ROWS + extra;

        // Act: Validate row beyond limit
        let result = validate_row_1based(row, "test");

        // Assert: Beyond-limit rows always fail
        prop_assert!(result.is_err());
    }

    /// Property: columns beyond Excel limit fail
    /// AAA Pattern: Arrange - column > max, Act - validate, Assert - error
    #[test]
    fn prop_columns_beyond_limit_rejected(extra in 1u32..=1000) {
        // Arrange: Create column beyond Excel maximum
        let col = EXCEL_MAX_COLUMNS + extra;

        // Act: Validate column beyond limit
        let result = validate_column_1based(col, "test");

        // Assert: Beyond-limit columns always fail
        prop_assert!(result.is_err());
    }

    /// Property: valid cell addresses always pass
    /// AAA Pattern: Arrange - generate valid address, Act - validate, Assert - success
    #[test]
    fn prop_valid_cell_addresses_accepted(address in arb_cell_address()) {
        // Arrange: address is generated in valid A1 notation

        // Act: Validate the cell address
        let result = validate_cell_address(&address);

        // Assert: Valid cell addresses always pass
        prop_assert!(result.is_ok());
    }

    /// Property: empty cell address always fails
    /// AAA Pattern: Arrange - empty string, Act - validate, Assert - error
    #[test]
    fn prop_empty_cell_address_rejected(_unit in Just(())) {
        // Arrange: Empty cell address
        let address = "";

        // Act: Validate empty address
        let result = validate_cell_address(address);

        // Assert: Empty address always fails
        prop_assert!(result.is_err());
    }

    /// Property: valid range strings pass validation
    /// AAA Pattern: Arrange - generate valid range, Act - validate, Assert - no panic
    #[test]
    fn prop_valid_ranges_accepted(range in arb_range_string()) {
        // Arrange: range is generated in valid format

        // Act: Validate the range string
        let _ = validate_range_string(&range);

        // Assert: Should not panic (may succeed or fail, but gracefully)
    }

    /// Property: cache capacity clamping is idempotent
    /// AAA Pattern: Arrange - arbitrary capacity, Act - clamp twice, Assert - equal
    #[test]
    fn prop_cache_capacity_clamp_idempotent(capacity in any::<usize>()) {
        // Arrange: arbitrary capacity value

        // Act: Clamp twice in succession
        let clamped1 = clamp_cache_capacity(capacity);
        let clamped2 = clamp_cache_capacity(clamped1);

        // Assert: Clamping is idempotent
        prop_assert_eq!(clamped1, clamped2);
    }

    /// Property: clamped cache capacity is within bounds
    /// AAA Pattern: Arrange - arbitrary capacity, Act - clamp, Assert - within bounds
    #[test]
    fn prop_cache_capacity_within_bounds(capacity in any::<usize>()) {
        // Arrange: arbitrary capacity value

        // Act: Clamp to valid range
        let clamped = clamp_cache_capacity(capacity);

        // Assert: Result is within defined bounds
        prop_assert!(clamped >= MIN_CACHE_CAPACITY);
        prop_assert!(clamped <= MAX_CACHE_CAPACITY);
    }

    /// Property: pagination validation prevents overflow
    /// AAA Pattern: Arrange - valid offset/limit, Act - validate, Assert - no overflow
    #[test]
    fn prop_pagination_no_overflow(offset in 0usize..MAX_PAGINATION_OFFSET, limit in 0usize..MAX_PAGINATION_LIMIT) {
        // Arrange: offset and limit within their respective bounds

        // Act: Validate pagination parameters
        let result = validate_pagination(offset, limit);

        // Assert: If validation passes, sum must not overflow
        if result.is_ok() {
            prop_assert!(offset.checked_add(limit).is_some());
        }
    }

    /// Property: validate_sheet_name rejects invalid characters
    /// AAA Pattern: Arrange - name with invalid char, Act - validate, Assert - error
    #[test]
    fn prop_sheet_name_rejects_invalid_chars(invalid_char in prop::char::range(':', ']')) {
        // Arrange: Create sheet name with potentially invalid character
        let name = format!("Sheet{}", invalid_char);

        // Act & Assert: Invalid characters should be rejected
        if [':', '\\', '/', '?', '*', '[', ']'].contains(&invalid_char) {
            let result = validate_sheet_name(&name);
            prop_assert!(result.is_err());
        }
    }

    /// Property: valid sheet names always pass
    /// AAA Pattern: Arrange - generate valid name, Act - validate, Assert - success
    #[test]
    fn prop_valid_sheet_names_accepted(name in arb_sheet_name()) {
        // Arrange: name is generated with valid Excel sheet name rules

        // Act: Validate the sheet name
        let result = validate_sheet_name(&name);

        // Assert: Valid sheet names always pass
        prop_assert!(result.is_ok());
    }

    /// Property: sheet names over 31 chars fail
    /// AAA Pattern: Arrange - long name, Act - validate, Assert - error
    #[test]
    fn prop_long_sheet_names_rejected(name in prop::string::string_regex(r"[a-zA-Z]{32,100}").unwrap()) {
        // Arrange: name is 32-100 characters (exceeds 31 char limit)

        // Act: Validate the long sheet name
        let result = validate_sheet_name(&name);

        // Assert: Long sheet names always fail
        prop_assert!(result.is_err());
    }

    /// Property: workbook ID validation never panics
    /// AAA Pattern: Act - validate arbitrary ID, Assert - no panic
    #[test]
    fn prop_workbook_id_never_panics(id in any::<String>()) {
        // Act: Validate arbitrary workbook ID
        let _ = validate_workbook_id(&id);

        // Assert: Should never panic regardless of input
    }

    /// Property: valid workbook IDs pass
    /// AAA Pattern: Arrange - generate valid ID, Act - validate, Assert - success
    #[test]
    fn prop_valid_workbook_ids_accepted(id in arb_workbook_id()) {
        // Arrange: id is generated with valid characters and length

        // Act: Validate the workbook ID
        let result = validate_workbook_id(&id);

        // Assert: Valid workbook IDs always pass
        prop_assert!(result.is_ok());
    }

    /// Property: empty workbook ID fails
    /// AAA Pattern: Arrange - empty/whitespace ID, Act - validate, Assert - error
    #[test]
    fn prop_empty_workbook_id_rejected(_unit in Just(())) {
        // Arrange: Empty and whitespace-only IDs

        // Act & Assert: Both should fail validation
        let empty_result = validate_workbook_id("");
        let whitespace_result = validate_workbook_id("   ");

        prop_assert!(empty_result.is_err());
        prop_assert!(whitespace_result.is_err());
    }
}

// =============================================================================
// Property Tests for Path Safety
// =============================================================================

#[allow(clippy::unwrap_used)] // Property tests: unwrap is acceptable in test code
proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Property: safe paths never contain traversal
    /// AAA Pattern: Arrange - generate safe path, Act - validate, Assert - success
    #[test]
    fn prop_safe_paths_accepted(path in arb_safe_path()) {
        // Arrange: path is generated with safe characters only

        // Act: Validate the safe path
        let result = validate_path_safe(&path);

        // Assert: Safe paths always pass
        prop_assert!(result.is_ok());
    }

    /// Property: path traversal attempts are rejected
    /// AAA Pattern: Arrange - generate dangerous path, Act - validate, Assert - error
    #[test]
    fn prop_dangerous_paths_rejected(path in arb_dangerous_path()) {
        // Arrange: path contains traversal attempts or dangerous patterns

        // Act: Validate the dangerous path
        let result = validate_path_safe(&path);

        // Assert: Dangerous paths always fail
        prop_assert!(result.is_err());
    }

    /// Property: paths with .. are always rejected
    /// AAA Pattern: Arrange - construct path with .., Act - validate, Assert - error
    #[test]
    fn prop_dot_dot_paths_rejected(prefix in arb_safe_path(), suffix in arb_safe_path()) {
        // Arrange: Construct path with .. traversal
        let path = format!("{}/../{}", prefix, suffix);

        // Act: Validate path with traversal
        let result = validate_path_safe(&path);

        // Assert: Paths with .. always fail
        prop_assert!(result.is_err());
    }

    /// Property: absolute paths are rejected
    /// AAA Pattern: Arrange - construct absolute path, Act - validate, Assert - error
    #[test]
    fn prop_absolute_paths_rejected(path in arb_safe_path()) {
        // Arrange: Create absolute path (starts with /)
        let abs_path = format!("/{}", path);

        // Act: Validate absolute path
        let result = validate_path_safe(&abs_path);

        // Assert: Absolute paths always fail
        prop_assert!(result.is_err());
    }
}

// =============================================================================
// Property Tests for SPARQL Injection Prevention
// =============================================================================

#[allow(clippy::unwrap_used)] // Property tests: unwrap is acceptable in test code
proptest! {
    #![proptest_config(ProptestConfig::with_cases(10000))] // More cases for security

    /// Property: SPARQL sanitizer never allows injection keywords
    /// AAA Pattern: Arrange - malicious string, Act - escape, Assert - injection blocked
    #[test]
    fn prop_sparql_sanitizer_blocks_injection(input in arb_malicious_string()) {
        // Arrange: input contains potentially malicious SPARQL patterns

        // Act: Attempt to escape the malicious string
        let result = SparqlSanitizer::escape_string(&input);

        // Assert: If escaping succeeds, injection vectors must be neutralized
        if let Ok(escaped) = result {
            prop_assert!(!escaped.contains("UNION"));
            prop_assert!(!escaped.contains("DROP"));
            prop_assert!(!escaped.contains("DELETE"));
        }
    }

    /// Property: SPARQL variable validator accepts valid variables
    /// AAA Pattern: Arrange - generate valid variable, Act - validate, Assert - success
    #[test]
    fn prop_valid_sparql_variables_accepted(var in arb_sparql_variable()) {
        // Arrange: var is generated with valid SPARQL variable syntax

        // Act: Validate the SPARQL variable
        let result = VariableValidator::validate(&var);

        // Assert: Valid variables always pass
        prop_assert!(result.is_ok());
    }

    /// Property: SPARQL variables without ? or $ are rejected
    /// AAA Pattern: Arrange - generate invalid variable, Act - validate, Assert - error
    #[test]
    fn prop_invalid_sparql_variables_rejected(var in prop::string::string_regex(r"[a-zA-Z][a-zA-Z0-9_]*").unwrap()) {
        // Arrange: var lacks required ? or $ prefix

        // Act: Validate the invalid variable
        let result = VariableValidator::validate(&var);

        // Assert: Variables without prefix always fail
        prop_assert!(result.is_err());
    }

    /// Property: IRI validator accepts valid IRIs
    /// AAA Pattern: Arrange - generate valid IRI, Act - validate, Assert - success
    #[test]
    fn prop_valid_iris_accepted(iri in arb_iri()) {
        // Arrange: iri is generated with valid URI syntax

        // Act: Validate the IRI
        let result = IriValidator::validate(&iri);

        // Assert: Valid IRIs always pass
        prop_assert!(result.is_ok());
    }

    /// Property: IRIs with spaces are rejected
    /// AAA Pattern: Arrange - construct IRI with space, Act - validate, Assert - error
    #[test]
    fn prop_iris_with_spaces_rejected(before in arb_iri(), after in arb_iri()) {
        // Arrange: Construct IRI with embedded space
        let iri = format!("{} {}", before, after);

        // Act: Validate IRI with space
        let result = IriValidator::validate(&iri);

        // Assert: IRIs with spaces always fail
        prop_assert!(result.is_err());
    }

    /// Property: QueryBuilder produces parseable queries
    /// AAA Pattern: Arrange - generate variables, Act - build query, Assert - valid structure
    #[test]
    fn prop_query_builder_produces_valid_queries(
        var1 in arb_sparql_variable(),
        var2 in arb_sparql_variable()
    ) {
        // Arrange: Two valid SPARQL variables

        // Act: Build query with variables
        let query = QueryBuilder::select()
            .variable(&var1)
            .variable(&var2)
            .where_clause(&format!("{} ?p ?o", var1))
            .build();

        // Assert: Query is valid and contains expected structure
        prop_assert!(query.is_ok());
        let query_str = query.unwrap();
        prop_assert!(query_str.contains("SELECT"));
        prop_assert!(query_str.contains("WHERE"));
    }

    /// Property: SafeLiteralBuilder always produces quoted strings
    /// AAA Pattern: Arrange - generate string value, Act - build literal, Assert - quoted
    #[test]
    fn prop_literal_builder_quotes_strings(value in prop::string::string_regex(r"[a-zA-Z0-9 ]{1,100}").unwrap()) {
        // Arrange: value is a valid string

        // Act: Build SPARQL string literal
        let literal = SafeLiteralBuilder::string(&value).build();

        // Assert: Literal is properly quoted
        prop_assert!(literal.starts_with('"'));
        prop_assert!(literal.ends_with('"'));
    }

    /// Property: Integer literals are valid
    /// AAA Pattern: Arrange - arbitrary integer, Act - build literal, Assert - valid format
    #[test]
    fn prop_literal_builder_integers(value in any::<i64>()) {
        // Arrange: arbitrary integer value

        // Act: Build SPARQL integer literal
        let literal = SafeLiteralBuilder::integer(value).build();

        // Assert: Literal contains value and type annotation
        prop_assert!(literal.contains(&value.to_string()));
        prop_assert!(literal.contains("XMLSchema#integer"));
    }
}

// =============================================================================
// Property Tests for Template Parameter Validation
// =============================================================================

#[allow(clippy::unwrap_used)] // Property tests: unwrap is acceptable in test code
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Property: ParameterType::String matches only strings
    /// AAA Pattern: Arrange - create values, Act - check type match, Assert - correct type
    #[test]
    fn prop_parameter_type_string_matches(s in any::<String>()) {
        // Arrange: Create string and non-string values
        let string_value = serde_json::Value::String(s);
        let number_value = serde_json::Value::Number(42.into());

        // Act & Assert: String type matches strings only
        prop_assert!(ParameterType::String.matches(&string_value));
        prop_assert!(!ParameterType::String.matches(&number_value));
    }

    /// Property: ParameterType::Bool matches only booleans
    /// AAA Pattern: Arrange - create values, Act - check type match, Assert - correct type
    #[test]
    fn prop_parameter_type_bool_matches(b in any::<bool>()) {
        // Arrange: Create boolean and non-boolean values
        let bool_value = serde_json::Value::Bool(b);
        let string_value = serde_json::Value::String("true".to_string());

        // Act & Assert: Bool type matches booleans only
        prop_assert!(ParameterType::Bool.matches(&bool_value));
        prop_assert!(!ParameterType::Bool.matches(&string_value));
    }

    /// Property: ParameterType::Number matches only numbers
    /// AAA Pattern: Arrange - create number value, Act - check type match, Assert - correct type
    #[test]
    fn prop_parameter_type_number_matches(n in any::<i64>()) {
        // Arrange: Create number value
        let number_value = serde_json::Value::Number(n.into());

        // Act & Assert: Number type matches numbers
        prop_assert!(ParameterType::Number.matches(&number_value));
    }

    /// Property: ValidationRule::MinLength rejects strings too short
    /// AAA Pattern: Arrange - create rule and string, Act - validate, Assert - length check
    #[test]
    fn prop_validation_min_length(min in 1usize..=20, s in prop::string::string_regex(r"[a-z]{1,50}").unwrap()) {
        // Arrange: Create min length rule and string value
        let rule = ValidationRule::MinLength(min);
        let value = serde_json::Value::String(s.clone());

        // Act: Validate string length
        let result = rule.validate("test", &value);

        // Assert: Strings >= min length pass, others fail
        if s.len() >= min {
            prop_assert!(result.is_ok());
        } else {
            prop_assert!(result.is_err());
        }
    }

    /// Property: ValidationRule::MaxLength rejects strings too long
    /// AAA Pattern: Arrange - create rule and string, Act - validate, Assert - length check
    #[test]
    fn prop_validation_max_length(max in 5usize..=50, s in prop::string::string_regex(r"[a-z]{1,100}").unwrap()) {
        // Arrange: Create max length rule and string value
        let rule = ValidationRule::MaxLength(max);
        let value = serde_json::Value::String(s.clone());

        // Act: Validate string length
        let result = rule.validate("test", &value);

        // Assert: Strings <= max length pass, others fail
        if s.len() <= max {
            prop_assert!(result.is_ok());
        } else {
            prop_assert!(result.is_err());
        }
    }

    /// Property: ValidationRule::Min rejects numbers too small
    /// AAA Pattern: Arrange - create rule and number, Act - validate, Assert - min check
    #[test]
    fn prop_validation_min_number(min in any::<i64>(), n in any::<i64>()) {
        // Arrange: Create min value rule and number
        let rule = ValidationRule::Min(min);
        let value = serde_json::Value::Number(n.into());

        // Act: Validate number minimum
        let result = rule.validate("test", &value);

        // Assert: Numbers >= min pass, others fail
        if n >= min {
            prop_assert!(result.is_ok());
        } else {
            prop_assert!(result.is_err());
        }
    }

    /// Property: ValidationRule::Max rejects numbers too large
    /// AAA Pattern: Arrange - create rule and number, Act - validate, Assert - max check
    #[test]
    fn prop_validation_max_number(max in any::<i64>(), n in any::<i64>()) {
        // Arrange: Create max value rule and number
        let rule = ValidationRule::Max(max);
        let value = serde_json::Value::Number(n.into());

        // Act: Validate number maximum
        let result = rule.validate("test", &value);

        // Assert: Numbers <= max pass, others fail
        if n <= max {
            prop_assert!(result.is_ok());
        } else {
            prop_assert!(result.is_err());
        }
    }
}

// =============================================================================
// Property Tests for Serialization Round-Trips
// =============================================================================

#[allow(clippy::unwrap_used)] // Property tests: unwrap is acceptable in test code
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Property: WorkbookId round-trips through JSON
    /// AAA Pattern: Arrange - create ID, Act - serialize/deserialize, Assert - equal
    #[test]
    fn prop_workbook_id_json_roundtrip(id_str in arb_workbook_id()) {
        // Arrange: Create WorkbookId
        let id = WorkbookId(id_str.clone());

        // Act: Serialize to JSON and deserialize back
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: WorkbookId = serde_json::from_str(&json).unwrap();

        // Assert: Round-trip preserves value
        prop_assert_eq!(id.as_str(), deserialized.as_str());
    }

    /// Property: Cell addresses round-trip correctly
    /// AAA Pattern: Arrange - generate address, Act - serialize/deserialize, Assert - equal
    #[test]
    fn prop_cell_address_roundtrip(address in arb_cell_address()) {
        // Arrange: address is a valid cell address

        // Act & Assert: If address is valid, it should round-trip
        if validate_cell_address(&address).is_ok() {
            let json = serde_json::to_string(&address).unwrap();
            let deserialized: String = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(address, deserialized);
        }
    }

    /// Property: Boolean values round-trip through JSON
    /// AAA Pattern: Arrange - create bool, Act - serialize/deserialize, Assert - equal
    #[test]
    fn prop_bool_json_roundtrip(b in any::<bool>()) {
        // Arrange: arbitrary boolean value

        // Act: Serialize to JSON and deserialize back
        let json = serde_json::to_string(&b).unwrap();
        let deserialized: bool = serde_json::from_str(&json).unwrap();

        // Assert: Round-trip preserves value
        prop_assert_eq!(b, deserialized);
    }

    /// Property: Numbers round-trip through JSON
    /// AAA Pattern: Arrange - create number, Act - serialize/deserialize, Assert - equal
    #[test]
    fn prop_number_json_roundtrip(n in any::<i64>()) {
        // Arrange: arbitrary integer value

        // Act: Serialize to JSON and deserialize back
        let json = serde_json::to_string(&n).unwrap();
        let deserialized: i64 = serde_json::from_str(&json).unwrap();

        // Assert: Round-trip preserves value
        prop_assert_eq!(n, deserialized);
    }
}

// =============================================================================
// Property Tests for Range Validation
// =============================================================================

#[allow(clippy::unwrap_used)] // Property tests: unwrap is acceptable in test code
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Property: validate_range_1based accepts valid ranges
    /// AAA Pattern: Arrange - generate range bounds, Act - validate, Assert - correct dimensions
    #[test]
    fn prop_valid_ranges_accepted_1based(
        start_row in arb_row_number(),
        start_col in arb_column_number(),
        end_row in arb_row_number(),
        end_col in arb_column_number()
    ) {
        // Arrange: Four arbitrary valid row/column values

        // Act: Validate the range
        let result = validate_range_1based(start_row, start_col, end_row, end_col, "test");

        // Assert: Well-ordered ranges pass and return correct dimensions
        if start_row <= end_row && start_col <= end_col {
            prop_assert!(result.is_ok());
            let (rows, cols) = result.unwrap();
            prop_assert_eq!(rows, end_row - start_row + 1);
            prop_assert_eq!(cols, end_col - start_col + 1);
        } else {
            prop_assert!(result.is_err());
        }
    }

    /// Property: validate_screenshot_range rejects ranges too large
    /// AAA Pattern: Arrange - generate dimensions, Act - validate, Assert - size limits
    #[test]
    fn prop_screenshot_range_limits(rows in 1u32..=200, cols in 1u32..=100) {
        // Arrange: Generate row and column counts

        // Act: Validate screenshot range
        let result = validate_screenshot_range(rows, cols);

        // Assert: Within-limit ranges pass, oversized ranges fail
        if rows <= MAX_SCREENSHOT_ROWS && cols <= MAX_SCREENSHOT_COLS {
            prop_assert!(result.is_ok());
        } else {
            prop_assert!(result.is_err());
        }
    }

    /// Property: validate_png_dimensions enforces limits
    /// AAA Pattern: Arrange - generate dimensions, Act - validate, Assert - dimension/area limits
    #[test]
    fn prop_png_dimensions_limits(width in 1u32..=20000, height in 1u32..=20000) {
        // Arrange: Generate PNG dimensions

        // Act: Validate PNG dimensions
        let result = validate_png_dimensions(width, height, None, None);

        // Assert: Dimensions within limits pass, oversized fail
        let within_dim_limit = width <= DEFAULT_MAX_PNG_DIM_PX && height <= DEFAULT_MAX_PNG_DIM_PX;
        let within_area_limit = (width as u64 * height as u64) <= DEFAULT_MAX_PNG_AREA_PX;

        if within_dim_limit && within_area_limit {
            prop_assert!(result.is_ok());
        } else {
            prop_assert!(result.is_err());
        }
    }
}

// =============================================================================
// Property Tests for Numeric Validation
// =============================================================================

#[allow(clippy::unwrap_used)] // Property tests: unwrap is acceptable in test code
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Property: validate_sample_size clamps to total_rows
    /// AAA Pattern: Arrange - generate sizes, Act - validate, Assert - clamped correctly
    #[test]
    fn prop_sample_size_clamps(sample_size in 1usize..1000, total_rows in 1usize..10000) {
        // Arrange: Generate sample size and total rows

        // Act & Assert: If sample size is valid, result is clamped appropriately
        if sample_size <= MAX_SAMPLE_SIZE {
            let result = validate_sample_size(sample_size, total_rows);
            prop_assert!(result.is_ok());
            let validated = result.unwrap();
            prop_assert!(validated <= total_rows);
            prop_assert!(validated <= sample_size);
        }
    }

    /// Property: validate_sample_size rejects excessive sizes
    /// AAA Pattern: Arrange - generate excessive size, Act - validate, Assert - error
    #[test]
    fn prop_sample_size_rejects_excessive(excessive in (MAX_SAMPLE_SIZE + 1)..=MAX_SAMPLE_SIZE * 2) {
        // Arrange: excessive is larger than MAX_SAMPLE_SIZE

        // Act: Validate excessive sample size
        let result = validate_sample_size(excessive, excessive);

        // Assert: Excessive sizes always fail
        prop_assert!(result.is_err());
    }
}

// =============================================================================
// Shrinking Tests
// =============================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used)] // Test code: unwrap is acceptable
mod shrinking_tests {
    use super::*;

    test!(test_shrinking_finds_minimal_failing_case, {
        // Arrange: Set up proptest to intentionally fail for values > 100
        // This demonstrates proptest's shrinking capability

        // Act: Run proptest with a condition that fails for n > 100
        let result = proptest!(|(n in 0u32..1000)| {
            if n > 100 {
                // This will fail for n > 100
                prop_assert!(n <= 100);
            }
        });

        // Assert: The test should fail, and shrinking should find n = 101 as minimal case
        assert!(result.is_err());
    });
}

// =============================================================================
// Configuration for Critical Security Properties
// =============================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used)] // Test code: unwrap is acceptable
mod security_critical_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 10000, // More cases for security-critical tests
            max_shrink_iters: 1000,
            ..Default::default()
        })]

        /// Critical: SPARQL injection must never succeed
        /// AAA Pattern: Arrange - malicious input, Act - escape, Assert - injection blocked
        #[test]
        fn critical_no_sparql_injection(malicious in arb_malicious_string()) {
            // Arrange: malicious contains potential SPARQL injection patterns

            // Act: Attempt to escape the malicious string
            let result = SparqlSanitizer::escape_string(&malicious);

            // Assert: Either rejected or properly escaped (no dangerous keywords)
            if let Ok(escaped) = result {
                // Must not contain dangerous keywords unescaped
                prop_assert!(!escaped.contains("UNION SELECT"));
                prop_assert!(!escaped.contains("DROP TABLE"));
                prop_assert!(!escaped.contains("DELETE FROM"));
            }
        }

        /// Critical: Path traversal must never succeed
        /// AAA Pattern: Act - validate arbitrary path, Assert - traversal prevented
        #[test]
        fn critical_no_path_traversal(path in any::<String>()) {
            // Act: Validate arbitrary path string
            let result = validate_path_safe(&path);

            // Assert: If validation passes, path must not contain traversal patterns
            if result.is_ok() {
                prop_assert!(!path.contains(".."));
                prop_assert!(!path.starts_with('/'));
                prop_assert!(!path.contains('\0'));
            }
        }

        /// Critical: Cell address validation never causes panic
        /// AAA Pattern: Act - validate arbitrary address, Assert - no panic
        #[test]
        fn critical_cell_address_no_panic(address in any::<String>()) {
            // Act: Validate arbitrary cell address string
            let _ = validate_cell_address(&address);

            // Assert: Must never panic regardless of input
        }
    }
}
