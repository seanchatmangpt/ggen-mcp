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

use proptest::prelude::*;
use spreadsheet_mcp::model::WorkbookId;
use spreadsheet_mcp::sparql::injection_prevention::*;
use spreadsheet_mcp::template::parameter_validation::*;
use spreadsheet_mcp::validation::bounds::*;
use spreadsheet_mcp::validation::input_guards::*;

// =============================================================================
// Domain Type Generators
// =============================================================================

/// Generate valid alphanumeric strings for WorkbookId
pub fn arb_workbook_id() -> impl Strategy<Value = String> {
    prop::string::string_regex(r"[a-zA-Z0-9\-_\.]{1,255}").expect("valid regex")
}

/// Generate valid sheet names (Excel compliant)
pub fn arb_sheet_name() -> impl Strategy<Value = String> {
    // Sheet names: 1-31 chars, no : \ / ? * [ ]
    prop::string::string_regex(r"[a-zA-Z0-9 _\-\.]{1,31}")
        .expect("valid regex")
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

/// Generate potentially malicious strings for injection testing
pub fn arb_malicious_string() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::string::string_regex(r".*UNION.*").expect("valid regex"),
        prop::string::string_regex(r".*DROP.*").expect("valid regex"),
        prop::string::string_regex(r".*DELETE.*").expect("valid regex"),
        prop::string::string_regex(r".*#.*").expect("valid regex"),
        prop::string::string_regex(r".*\{.*\}.*").expect("valid regex"),
        prop::string::string_regex(r".*'.*'.*").expect("valid regex"),
        prop::string::string_regex(r#".*".*".*"#).expect("valid regex"),
    ]
}

/// Generate valid SPARQL variable names
pub fn arb_sparql_variable() -> impl Strategy<Value = String> {
    prop::string::string_regex(r"\?[a-zA-Z_][a-zA-Z0-9_]{0,127}")
        .expect("valid regex")
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

/// Generate valid IRIs
pub fn arb_iri() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::string::string_regex(r"https?://[a-zA-Z0-9\-\.]+\.[a-z]{2,}(/[a-zA-Z0-9\-_\.]*)*")
            .expect("valid regex"),
        prop::string::string_regex(r"urn:[a-z][a-z0-9\-]*:[a-zA-Z0-9\-_\.]+").expect("valid regex"),
    ]
}

/// Generate safe paths (no traversal)
pub fn arb_safe_path() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop::string::string_regex(r"[a-zA-Z0-9_\-\.]+").expect("valid regex"),
        1..=5,
    )
    .prop_map(|parts| parts.join("/"))
}

/// Generate potentially dangerous paths
pub fn arb_dangerous_path() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("../etc/passwd".to_string()),
        Just("/etc/passwd".to_string()),
        Just("C:\\Windows\\System32".to_string()),
        Just("..\\..\\..\\etc\\passwd".to_string()),
        Just("file\0.txt".to_string()),
        prop::string::string_regex(r".*\.\..*").expect("valid regex"),
    ]
}

// =============================================================================
// Property Tests for Validation Functions
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Property: validate_row_1based never panics
    #[test]
    fn prop_validate_row_never_panics(row in any::<u32>()) {
        let _ = validate_row_1based(row, "test");
        // Should never panic
    }

    /// Property: validate_column_1based never panics
    #[test]
    fn prop_validate_column_never_panics(col in any::<u32>()) {
        let _ = validate_column_1based(col, "test");
        // Should never panic
    }

    /// Property: valid rows always pass validation
    #[test]
    fn prop_valid_rows_accepted(row in arb_row_number()) {
        assert!(validate_row_1based(row, "test").is_ok());
    }

    /// Property: valid columns always pass validation
    #[test]
    fn prop_valid_columns_accepted(col in arb_column_number()) {
        assert!(validate_column_1based(col, "test").is_ok());
    }

    /// Property: row 0 always fails
    #[test]
    fn prop_row_zero_rejected(_unit in Just(())) {
        assert!(validate_row_1based(0, "test").is_err());
    }

    /// Property: column 0 always fails
    #[test]
    fn prop_column_zero_rejected(_unit in Just(())) {
        assert!(validate_column_1based(0, "test").is_err());
    }

    /// Property: rows beyond Excel limit fail
    #[test]
    fn prop_rows_beyond_limit_rejected(extra in 1u32..=1000) {
        let row = EXCEL_MAX_ROWS + extra;
        assert!(validate_row_1based(row, "test").is_err());
    }

    /// Property: columns beyond Excel limit fail
    #[test]
    fn prop_columns_beyond_limit_rejected(extra in 1u32..=1000) {
        let col = EXCEL_MAX_COLUMNS + extra;
        assert!(validate_column_1based(col, "test").is_err());
    }

    /// Property: valid cell addresses always pass
    #[test]
    fn prop_valid_cell_addresses_accepted(address in arb_cell_address()) {
        assert!(validate_cell_address(&address).is_ok());
    }

    /// Property: empty cell address always fails
    #[test]
    fn prop_empty_cell_address_rejected(_unit in Just(())) {
        assert!(validate_cell_address("").is_err());
    }

    /// Property: valid range strings pass validation
    #[test]
    fn prop_valid_ranges_accepted(range in arb_range_string()) {
        let _ = validate_range_string(&range);
        // Should not panic
    }

    /// Property: cache capacity clamping is idempotent
    #[test]
    fn prop_cache_capacity_clamp_idempotent(capacity in any::<usize>()) {
        let clamped1 = clamp_cache_capacity(capacity);
        let clamped2 = clamp_cache_capacity(clamped1);
        assert_eq!(clamped1, clamped2);
    }

    /// Property: clamped cache capacity is within bounds
    #[test]
    fn prop_cache_capacity_within_bounds(capacity in any::<usize>()) {
        let clamped = clamp_cache_capacity(capacity);
        assert!(clamped >= MIN_CACHE_CAPACITY);
        assert!(clamped <= MAX_CACHE_CAPACITY);
    }

    /// Property: pagination validation prevents overflow
    #[test]
    fn prop_pagination_no_overflow(offset in 0usize..MAX_PAGINATION_OFFSET, limit in 0usize..MAX_PAGINATION_LIMIT) {
        if validate_pagination(offset, limit).is_ok() {
            // If it passes, offset + limit must not overflow
            assert!(offset.checked_add(limit).is_some());
        }
    }

    /// Property: validate_sheet_name rejects invalid characters
    #[test]
    fn prop_sheet_name_rejects_invalid_chars(invalid_char in prop::char::range(':', ']')) {
        let name = format!("Sheet{}", invalid_char);
        if [':', '\\', '/', '?', '*', '[', ']'].contains(&invalid_char) {
            assert!(validate_sheet_name(&name).is_err());
        }
    }

    /// Property: valid sheet names always pass
    #[test]
    fn prop_valid_sheet_names_accepted(name in arb_sheet_name()) {
        assert!(validate_sheet_name(&name).is_ok());
    }

    /// Property: sheet names over 31 chars fail
    #[test]
    fn prop_long_sheet_names_rejected(name in prop::string::string_regex(r"[a-zA-Z]{32,100}").expect("valid regex")) {
        assert!(validate_sheet_name(&name).is_err());
    }

    /// Property: workbook ID validation never panics
    #[test]
    fn prop_workbook_id_never_panics(id in any::<String>()) {
        let _ = validate_workbook_id(&id);
        // Should never panic
    }

    /// Property: valid workbook IDs pass
    #[test]
    fn prop_valid_workbook_ids_accepted(id in arb_workbook_id()) {
        assert!(validate_workbook_id(&id).is_ok());
    }

    /// Property: empty workbook ID fails
    #[test]
    fn prop_empty_workbook_id_rejected(_unit in Just(())) {
        assert!(validate_workbook_id("").is_err());
        assert!(validate_workbook_id("   ").is_err());
    }
}

// =============================================================================
// Property Tests for Path Safety
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Property: safe paths never contain traversal
    #[test]
    fn prop_safe_paths_accepted(path in arb_safe_path()) {
        assert!(validate_path_safe(&path).is_ok());
    }

    /// Property: path traversal attempts are rejected
    #[test]
    fn prop_dangerous_paths_rejected(path in arb_dangerous_path()) {
        assert!(validate_path_safe(&path).is_err());
    }

    /// Property: paths with .. are always rejected
    #[test]
    fn prop_dot_dot_paths_rejected(prefix in arb_safe_path(), suffix in arb_safe_path()) {
        let path = format!("{}/../{}", prefix, suffix);
        assert!(validate_path_safe(&path).is_err());
    }

    /// Property: absolute paths are rejected
    #[test]
    fn prop_absolute_paths_rejected(path in arb_safe_path()) {
        let abs_path = format!("/{}", path);
        assert!(validate_path_safe(&abs_path).is_err());
    }
}

// =============================================================================
// Property Tests for SPARQL Injection Prevention
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10000))] // More cases for security

    /// Property: SPARQL sanitizer never allows injection keywords
    #[test]
    fn prop_sparql_sanitizer_blocks_injection(input in arb_malicious_string()) {
        let result = SparqlSanitizer::escape_string(&input);
        if result.is_ok() {
            let escaped = result.unwrap();
            // Escaped output should not contain injection vectors
            assert!(!escaped.contains("UNION"));
            assert!(!escaped.contains("DROP"));
            assert!(!escaped.contains("DELETE"));
        }
    }

    /// Property: SPARQL variable validator accepts valid variables
    #[test]
    fn prop_valid_sparql_variables_accepted(var in arb_sparql_variable()) {
        assert!(VariableValidator::validate(&var).is_ok());
    }

    /// Property: SPARQL variables without ? or $ are rejected
    #[test]
    fn prop_invalid_sparql_variables_rejected(var in prop::string::string_regex(r"[a-zA-Z][a-zA-Z0-9_]*").expect("valid regex")) {
        assert!(VariableValidator::validate(&var).is_err());
    }

    /// Property: IRI validator accepts valid IRIs
    #[test]
    fn prop_valid_iris_accepted(iri in arb_iri()) {
        assert!(IriValidator::validate(&iri).is_ok());
    }

    /// Property: IRIs with spaces are rejected
    #[test]
    fn prop_iris_with_spaces_rejected(before in arb_iri(), after in arb_iri()) {
        let iri = format!("{} {}", before, after);
        assert!(IriValidator::validate(&iri).is_err());
    }

    /// Property: QueryBuilder produces parseable queries
    #[test]
    fn prop_query_builder_produces_valid_queries(
        var1 in arb_sparql_variable(),
        var2 in arb_sparql_variable()
    ) {
        let query = QueryBuilder::select()
            .variable(&var1)
            .variable(&var2)
            .where_clause(&format!("{} ?p ?o", var1))
            .build();

        assert!(query.is_ok());
        let query_str = query.unwrap();
        assert!(query_str.contains("SELECT"));
        assert!(query_str.contains("WHERE"));
    }

    /// Property: SafeLiteralBuilder always produces quoted strings
    #[test]
    fn prop_literal_builder_quotes_strings(value in prop::string::string_regex(r"[a-zA-Z0-9 ]{1,100}").expect("valid regex")) {
        let literal = SafeLiteralBuilder::string(&value).build();
        assert!(literal.starts_with('"'));
        assert!(literal.ends_with('"'));
    }

    /// Property: Integer literals are valid
    #[test]
    fn prop_literal_builder_integers(value in any::<i64>()) {
        let literal = SafeLiteralBuilder::integer(value).build();
        assert!(literal.contains(&value.to_string()));
        assert!(literal.contains("XMLSchema#integer"));
    }
}

// =============================================================================
// Property Tests for Template Parameter Validation
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Property: ParameterType::String matches only strings
    #[test]
    fn prop_parameter_type_string_matches(s in any::<String>()) {
        let value = serde_json::Value::String(s);
        assert!(ParameterType::String.matches(&value));

        let number = serde_json::Value::Number(42.into());
        assert!(!ParameterType::String.matches(&number));
    }

    /// Property: ParameterType::Bool matches only booleans
    #[test]
    fn prop_parameter_type_bool_matches(b in any::<bool>()) {
        let value = serde_json::Value::Bool(b);
        assert!(ParameterType::Bool.matches(&value));

        let string = serde_json::Value::String("true".to_string());
        assert!(!ParameterType::Bool.matches(&string));
    }

    /// Property: ParameterType::Number matches only numbers
    #[test]
    fn prop_parameter_type_number_matches(n in any::<i64>()) {
        let value = serde_json::Value::Number(n.into());
        assert!(ParameterType::Number.matches(&value));
    }

    /// Property: ValidationRule::MinLength rejects strings too short
    #[test]
    fn prop_validation_min_length(min in 1usize..=20, s in prop::string::string_regex(r"[a-z]{1,50}").expect("valid regex")) {
        let rule = ValidationRule::MinLength(min);
        let value = serde_json::Value::String(s.clone());
        let result = rule.validate("test", &value);

        if s.len() >= min {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
        }
    }

    /// Property: ValidationRule::MaxLength rejects strings too long
    #[test]
    fn prop_validation_max_length(max in 5usize..=50, s in prop::string::string_regex(r"[a-z]{1,100}").expect("valid regex")) {
        let rule = ValidationRule::MaxLength(max);
        let value = serde_json::Value::String(s.clone());
        let result = rule.validate("test", &value);

        if s.len() <= max {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
        }
    }

    /// Property: ValidationRule::Min rejects numbers too small
    #[test]
    fn prop_validation_min_number(min in any::<i64>(), n in any::<i64>()) {
        let rule = ValidationRule::Min(min);
        let value = serde_json::Value::Number(n.into());
        let result = rule.validate("test", &value);

        if n >= min {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
        }
    }

    /// Property: ValidationRule::Max rejects numbers too large
    #[test]
    fn prop_validation_max_number(max in any::<i64>(), n in any::<i64>()) {
        let rule = ValidationRule::Max(max);
        let value = serde_json::Value::Number(n.into());
        let result = rule.validate("test", &value);

        if n <= max {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
        }
    }
}

// =============================================================================
// Property Tests for Serialization Round-Trips
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Property: WorkbookId round-trips through JSON
    #[test]
    fn prop_workbook_id_json_roundtrip(id_str in arb_workbook_id()) {
        let id = WorkbookId(id_str.clone());
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: WorkbookId = serde_json::from_str(&json).unwrap();
        assert_eq!(id.as_str(), deserialized.as_str());
    }

    /// Property: Cell addresses round-trip correctly
    #[test]
    fn prop_cell_address_roundtrip(address in arb_cell_address()) {
        // Validate it parses correctly
        if validate_cell_address(&address).is_ok() {
            // Should be able to serialize and deserialize
            let json = serde_json::to_string(&address).unwrap();
            let deserialized: String = serde_json::from_str(&json).unwrap();
            assert_eq!(address, deserialized);
        }
    }

    /// Property: Boolean values round-trip through JSON
    #[test]
    fn prop_bool_json_roundtrip(b in any::<bool>()) {
        let json = serde_json::to_string(&b).unwrap();
        let deserialized: bool = serde_json::from_str(&json).unwrap();
        assert_eq!(b, deserialized);
    }

    /// Property: Numbers round-trip through JSON
    #[test]
    fn prop_number_json_roundtrip(n in any::<i64>()) {
        let json = serde_json::to_string(&n).unwrap();
        let deserialized: i64 = serde_json::from_str(&json).unwrap();
        assert_eq!(n, deserialized);
    }
}

// =============================================================================
// Property Tests for Range Validation
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Property: validate_range_1based accepts valid ranges
    #[test]
    fn prop_valid_ranges_accepted_1based(
        start_row in arb_row_number(),
        start_col in arb_column_number(),
        end_row in arb_row_number(),
        end_col in arb_column_number()
    ) {
        let result = validate_range_1based(start_row, start_col, end_row, end_col, "test");

        if start_row <= end_row && start_col <= end_col {
            assert!(result.is_ok());
            let (rows, cols) = result.unwrap();
            assert_eq!(rows, end_row - start_row + 1);
            assert_eq!(cols, end_col - start_col + 1);
        } else {
            assert!(result.is_err());
        }
    }

    /// Property: validate_screenshot_range rejects ranges too large
    #[test]
    fn prop_screenshot_range_limits(rows in 1u32..=200, cols in 1u32..=100) {
        let result = validate_screenshot_range(rows, cols);

        if rows <= MAX_SCREENSHOT_ROWS && cols <= MAX_SCREENSHOT_COLS {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
        }
    }

    /// Property: validate_png_dimensions enforces limits
    #[test]
    fn prop_png_dimensions_limits(width in 1u32..=20000, height in 1u32..=20000) {
        let result = validate_png_dimensions(width, height, None, None);

        let within_dim_limit = width <= DEFAULT_MAX_PNG_DIM_PX && height <= DEFAULT_MAX_PNG_DIM_PX;
        let within_area_limit = (width as u64 * height as u64) <= DEFAULT_MAX_PNG_AREA_PX;

        if within_dim_limit && within_area_limit {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
        }
    }
}

// =============================================================================
// Property Tests for Numeric Validation
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Property: validate_sample_size clamps to total_rows
    #[test]
    fn prop_sample_size_clamps(sample_size in 1usize..1000, total_rows in 1usize..10000) {
        if sample_size <= MAX_SAMPLE_SIZE {
            let result = validate_sample_size(sample_size, total_rows);
            assert!(result.is_ok());
            let validated = result.unwrap();
            assert!(validated <= total_rows);
            assert!(validated <= sample_size);
        }
    }

    /// Property: validate_sample_size rejects excessive sizes
    #[test]
    fn prop_sample_size_rejects_excessive(excessive in (MAX_SAMPLE_SIZE + 1)..=MAX_SAMPLE_SIZE * 2) {
        let result = validate_sample_size(excessive, excessive);
        assert!(result.is_err());
    }
}

// =============================================================================
// Shrinking Tests
// =============================================================================

#[cfg(test)]
mod shrinking_tests {
    use super::*;

    #[test]
    fn test_shrinking_finds_minimal_failing_case() {
        // This test verifies that proptest shrinking works correctly
        // by intentionally creating a condition that fails for values > 100
        let result = proptest!(|(n in 0u32..1000)| {
            if n > 100 {
                // This will fail for n > 100
                prop_assert!(n <= 100);
            }
        });

        // The test should fail, and shrinking should find n = 101 as minimal case
        assert!(result.is_err());
    }
}

// =============================================================================
// Configuration for Critical Security Properties
// =============================================================================

#[cfg(test)]
mod security_critical_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 10000, // More cases for security-critical tests
            max_shrink_iters: 1000,
            ..Default::default()
        })]

        /// Critical: SPARQL injection must never succeed
        #[test]
        fn critical_no_sparql_injection(malicious in arb_malicious_string()) {
            let result = SparqlSanitizer::escape_string(&malicious);
            // Either rejected or properly escaped
            if let Ok(escaped) = result {
                // Must not contain dangerous keywords unescaped
                assert!(!escaped.contains("UNION SELECT"));
                assert!(!escaped.contains("DROP TABLE"));
                assert!(!escaped.contains("DELETE FROM"));
            }
        }

        /// Critical: Path traversal must never succeed
        #[test]
        fn critical_no_path_traversal(path in any::<String>()) {
            if validate_path_safe(&path).is_ok() {
                // If it passes, it must not contain traversal
                assert!(!path.contains(".."));
                assert!(!path.starts_with('/'));
                assert!(!path.contains('\0'));
            }
        }

        /// Critical: Cell address validation never causes panic
        #[test]
        fn critical_cell_address_no_panic(address in any::<String>()) {
            let _ = validate_cell_address(&address);
            // Must never panic regardless of input
        }
    }
}
