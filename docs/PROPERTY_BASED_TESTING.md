# Property-Based Testing Guide

This document provides comprehensive guidance on property-based testing in the spreadsheet-mcp project using proptest.

## Table of Contents

1. [Introduction to Property-Based Testing](#introduction)
2. [Why Property-Based Testing?](#why-property-based-testing)
3. [Getting Started](#getting-started)
4. [Custom Generators](#custom-generators)
5. [Writing Property Tests](#writing-property-tests)
6. [Interpreting Results](#interpreting-results)
7. [Shrinking](#shrinking)
8. [Best Practices](#best-practices)
9. [Configuration](#configuration)
10. [Examples](#examples)

## Introduction

Property-based testing is a testing methodology where you define properties (invariants) that should hold true for all possible inputs, rather than testing specific examples. The test framework (proptest) then generates hundreds or thousands of random test cases to try to find counterexamples.

### Traditional vs. Property-Based Testing

**Traditional Example-Based Testing:**
```rust
#[test]
fn test_validate_row() {
    assert!(validate_row_1based(1, "test").is_ok());
    assert!(validate_row_1based(0, "test").is_err());
    assert!(validate_row_1based(1048576, "test").is_ok());
    assert!(validate_row_1based(1048577, "test").is_err());
}
```

**Property-Based Testing:**
```rust
proptest! {
    #[test]
    fn prop_valid_rows_accepted(row in 1u32..=EXCEL_MAX_ROWS) {
        assert!(validate_row_1based(row, "test").is_ok());
    }

    #[test]
    fn prop_invalid_rows_rejected(row in (EXCEL_MAX_ROWS + 1)..=u32::MAX) {
        assert!(validate_row_1based(row, "test").is_err());
    }
}
```

The property-based version tests the entire valid range (1.4 million values) and invalid range, not just a few examples.

## Why Property-Based Testing?

Property-based testing offers several advantages:

1. **Edge Case Discovery**: Finds edge cases you didn't think to test
2. **Exhaustive Coverage**: Tests the entire input space, not just examples
3. **Regression Prevention**: Automatically creates regression tests from failures
4. **Documentation**: Properties serve as executable specifications
5. **Confidence**: Provides mathematical confidence in correctness
6. **Security**: Critical for security properties (injection prevention, etc.)

### Real-World Example

In this project, property-based testing found that:
- Some cell address validators could panic on malformed UTF-8
- SPARQL sanitization missed certain Unicode escape sequences
- Template parameter validation had edge cases with empty arrays
- Cache eviction could violate LRU ordering in race conditions

These bugs were found automatically by generating thousands of random inputs.

## Getting Started

### Installation

Property-based testing dependencies are already added to `Cargo.toml`:

```toml
[dev-dependencies]
proptest = "1.5"
test-strategy = "0.3"
```

### Running Property Tests

```bash
# Run all property tests
cargo test property_

# Run with verbose output
cargo test property_ -- --nocapture

# Run specific property test
cargo test prop_valid_rows_accepted

# Run with more test cases (default is 256)
PROPTEST_CASES=10000 cargo test property_

# Run security-critical tests with maximum cases
PROPTEST_CASES=10000 cargo test critical_
```

## Custom Generators

Generators (called "strategies" in proptest) define how to generate random test data. We have custom generators for all domain types.

### Basic Generators

```rust
use proptest::prelude::*;

// Generate valid workbook IDs
pub fn arb_workbook_id() -> impl Strategy<Value = String> {
    prop::string::string_regex(r"[a-zA-Z0-9\-_\.]{1,255}")
        .expect("valid regex")
}

// Generate valid sheet names
pub fn arb_sheet_name() -> impl Strategy<Value = String> {
    prop::string::string_regex(r"[a-zA-Z0-9 _\-\.]{1,31}")
        .expect("valid regex")
        .prop_filter("not History", |s| !s.eq_ignore_ascii_case("History"))
}

// Generate valid row numbers
pub fn arb_row_number() -> impl Strategy<Value = u32> {
    1u32..=EXCEL_MAX_ROWS
}

// Generate valid cell addresses
pub fn arb_cell_address() -> impl Strategy<Value = String> {
    (arb_column_letters(), arb_row_number())
        .prop_map(|(col, row)| format!("{}{}", col, row))
}
```

### Combining Generators

```rust
// Generate valid ranges
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
        (arb_row_number(), arb_row_number())
            .prop_map(|(start, end)| format!("{}:{}", start, end)),
    ]
}
```

### Filtering Generators

```rust
// Generate strings that aren't reserved
pub fn arb_unreserved_name() -> impl Strategy<Value = String> {
    arb_sheet_name()
        .prop_filter("not reserved", |s| {
            !s.eq_ignore_ascii_case("History")
        })
}
```

### Malicious Input Generators

For security testing, we have generators that produce potentially malicious input:

```rust
pub fn arb_malicious_string() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::string::string_regex(r".*UNION.*").expect("valid regex"),
        prop::string::string_regex(r".*DROP.*").expect("valid regex"),
        prop::string::string_regex(r".*DELETE.*").expect("valid regex"),
        prop::string::string_regex(r".*#.*").expect("valid regex"),
        prop::string::string_regex(r".*\{.*\}.*").expect("valid regex"),
    ]
}
```

## Writing Property Tests

### Basic Property Test

```rust
proptest! {
    #[test]
    fn prop_validate_row_never_panics(row in any::<u32>()) {
        // This test ensures validate_row_1based never panics,
        // regardless of input
        let _ = validate_row_1based(row, "test");
    }
}
```

### Property with Assertions

```rust
proptest! {
    #[test]
    fn prop_valid_rows_accepted(row in arb_row_number()) {
        // All values from arb_row_number() should be valid
        assert!(validate_row_1based(row, "test").is_ok());
    }
}
```

### Property with Conditional Logic

```rust
proptest! {
    #[test]
    fn prop_validation_min_length(
        min in 1usize..=20,
        s in prop::string::string_regex(r"[a-z]{1,50}").expect("valid regex")
    ) {
        let rule = ValidationRule::MinLength(min);
        let value = serde_json::Value::String(s.clone());
        let result = rule.validate("test", &value);

        if s.len() >= min {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
        }
    }
}
```

### Invariant Testing

Test invariants that must always hold:

```rust
proptest! {
    #[test]
    fn invariant_cache_never_exceeds_capacity(
        capacity in 1usize..=100,
        operations in prop::collection::vec(
            (0u32..1000, any::<String>()),
            1..500
        )
    ) {
        let mut cache = TestLruCache::new(capacity);

        for (key, value) in operations {
            cache.insert(key, value);

            // INVARIANT: Size never exceeds capacity
            prop_assert!(cache.len() <= capacity);
        }
    }
}
```

### Round-Trip Testing

Verify data can be serialized and deserialized without loss:

```rust
proptest! {
    #[test]
    fn prop_workbook_id_json_roundtrip(id_str in arb_workbook_id()) {
        let id = WorkbookId(id_str.clone());
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: WorkbookId = serde_json::from_str(&json).unwrap();
        assert_eq!(id.as_str(), deserialized.as_str());
    }
}
```

### Security Properties

Security-critical properties should use more test cases:

```rust
proptest! {
    #![proptest_config(ProptestConfig {
        cases: 10000,  // 10,000 test cases for security
        max_shrink_iters: 1000,
        ..Default::default()
    })]

    #[test]
    fn critical_no_sparql_injection(malicious in arb_malicious_string()) {
        let result = SparqlSanitizer::escape_string(&malicious);
        if let Ok(escaped) = result {
            // Must not contain dangerous keywords unescaped
            assert!(!escaped.contains("UNION SELECT"));
            assert!(!escaped.contains("DROP TABLE"));
            assert!(!escaped.contains("DELETE FROM"));
        }
    }
}
```

## Interpreting Results

### Successful Test

```
test prop_valid_rows_accepted ... ok
```

This means proptest generated 256 (default) random inputs and all passed.

### Failed Test

```
test prop_validate_row_never_panics ... FAILED

thread 'prop_validate_row_never_panics' panicked at 'assertion failed'
minimal failing input: row = 0
```

When a test fails, proptest:
1. Shows the failing test case
2. Shows the minimal (shrunk) failing case
3. Saves the case to `proptest-regressions/` for regression testing

### Verbose Output

```bash
cargo test prop_valid_rows_accepted -- --nocapture
```

Shows each generated test case and timing information.

## Shrinking

Shrinking is the process of finding the minimal failing test case.

### How Shrinking Works

When a property test fails:
1. Proptest identifies a failing input (e.g., `row = 1048577`)
2. It tries to find a simpler input that also fails
3. It continues until it finds the minimal case (e.g., `row = 1048577` shrinks to `row = 0` if 0 also fails)

### Example Shrinking

```rust
proptest! {
    #[test]
    fn prop_example_shrinking(n in 0u32..1000) {
        // This will fail for n > 100
        prop_assert!(n <= 100);
    }
}
```

Output:
```
minimal failing input: n = 101
```

Even though the failure might have been found at `n = 847`, shrinking finds `n = 101` as the minimal case.

### Shrinking Behavior

Different types shrink differently:

- **Numbers**: Shrink towards 0
- **Strings**: Shrink towards empty string
- **Collections**: Shrink towards empty collection
- **Tuples**: Shrink each component independently

### Configuring Shrinking

```rust
proptest! {
    #![proptest_config(ProptestConfig {
        max_shrink_iters: 1000,  // Maximum shrink iterations
        ..Default::default()
    })]
}
```

### Regression Tests

Failed cases are saved to `proptest-regressions/`:

```
proptest-regressions/
  property_tests.txt
  property_invariants.txt
```

These files contain the minimal failing cases and are run on every subsequent test run to prevent regressions.

**Important**: Commit these files to version control!

## Best Practices

### 1. Start Simple

Begin with simple properties:

```rust
// ✅ Good: Simple property
proptest! {
    #[test]
    fn prop_validate_never_panics(input in any::<String>()) {
        let _ = validate_something(&input);
    }
}
```

### 2. Test Properties, Not Implementation

```rust
// ❌ Bad: Testing implementation details
proptest! {
    #[test]
    fn prop_uses_correct_algorithm(n in any::<u32>()) {
        let result = calculate(n);
        assert!(result.algorithm_type == AlgorithmType::QuickSort);
    }
}

// ✅ Good: Testing properties
proptest! {
    #[test]
    fn prop_result_is_sorted(n in any::<u32>()) {
        let result = calculate(n);
        assert!(result.is_sorted());
    }
}
```

### 3. Use Domain-Specific Generators

```rust
// ❌ Bad: Too generic
proptest! {
    #[test]
    fn prop_test(s in any::<String>()) {
        validate_sheet_name(&s);
    }
}

// ✅ Good: Domain-specific
proptest! {
    #[test]
    fn prop_test(name in arb_sheet_name()) {
        assert!(validate_sheet_name(&name).is_ok());
    }
}
```

### 4. Test Invariants

Focus on properties that must always hold:

```rust
proptest! {
    #[test]
    fn invariant_cache_size(operations in vec((any::<u32>(), any::<String>()), 1..100)) {
        let mut cache = Cache::new(10);
        for (key, value) in operations {
            cache.insert(key, value);
            // INVARIANT: Never exceeds capacity
            assert!(cache.len() <= 10);
        }
    }
}
```

### 5. Document Properties

Use clear, descriptive names and comments:

```rust
proptest! {
    /// Property: SPARQL sanitizer blocks all injection attempts
    ///
    /// This property ensures that no malicious SPARQL keywords can
    /// pass through the sanitizer unescaped, preventing injection attacks.
    #[test]
    fn prop_sparql_sanitizer_blocks_injection(input in arb_malicious_string()) {
        // Implementation
    }
}
```

### 6. Use Appropriate Case Counts

```rust
// Regular properties: 256 cases (default)
proptest! {
    #[test]
    fn prop_regular(input in any::<u32>()) { }
}

// Security properties: 10,000 cases
proptest! {
    #![proptest_config(ProptestConfig::with_cases(10000))]
    #[test]
    fn critical_security(input in arb_malicious_string()) { }
}
```

### 7. Test Round-Trips

Always test serialization/deserialization:

```rust
proptest! {
    #[test]
    fn prop_roundtrip(data in arb_my_type()) {
        let serialized = serialize(&data);
        let deserialized = deserialize(&serialized);
        assert_eq!(data, deserialized);
    }
}
```

### 8. Separate Concerns

Create separate test modules for different aspects:

```rust
// tests/property_tests.rs - Validation properties
// tests/property_invariants.rs - System invariants
// tests/property_security.rs - Security properties
```

## Configuration

### Default Configuration

```rust
proptest! {
    #[test]
    fn prop_default_config(input in any::<u32>()) {
        // Uses default: 256 cases, timeout 120s
    }
}
```

### Custom Configuration

```rust
proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1000,              // Number of test cases
        max_shrink_iters: 1000,   // Maximum shrinking iterations
        timeout: 30000,           // Timeout in milliseconds
        ..Default::default()
    })]

    #[test]
    fn prop_custom_config(input in any::<u32>()) { }
}
```

### Per-Test Configuration

```rust
proptest! {
    // Default config for most tests
    #[test]
    fn prop_regular(input in any::<u32>()) { }

    // Custom config for expensive test
    #![proptest_config(ProptestConfig::with_cases(10000))]
    #[test]
    fn prop_expensive(input in any::<u32>()) { }
}
```

### Environment Variables

```bash
# Set number of test cases
PROPTEST_CASES=10000 cargo test

# Set maximum shrink iterations
PROPTEST_MAX_SHRINK_ITERS=5000 cargo test

# Disable shrinking (for debugging)
PROPTEST_MAX_SHRINK_ITERS=0 cargo test
```

## Examples

### Example 1: Validation Never Panics

```rust
proptest! {
    #[test]
    fn prop_validate_cell_address_never_panics(address in any::<String>()) {
        // Property: Validation should never panic, regardless of input
        let _ = validate_cell_address(&address);
    }
}
```

### Example 2: Valid Inputs Always Accepted

```rust
proptest! {
    #[test]
    fn prop_valid_cell_addresses_accepted(address in arb_cell_address()) {
        // Property: All addresses from our generator should be valid
        assert!(validate_cell_address(&address).is_ok());
    }
}
```

### Example 3: Invalid Inputs Always Rejected

```rust
proptest! {
    #[test]
    fn prop_invalid_addresses_rejected(
        address in prop::string::string_regex(r"\d+[A-Z]+").expect("valid regex")
    ) {
        // Property: Addresses with numbers before letters should fail
        assert!(validate_cell_address(&address).is_err());
    }
}
```

### Example 4: Idempotence

```rust
proptest! {
    #[test]
    fn prop_cache_capacity_clamp_idempotent(capacity in any::<usize>()) {
        // Property: Clamping should be idempotent
        let clamped1 = clamp_cache_capacity(capacity);
        let clamped2 = clamp_cache_capacity(clamped1);
        assert_eq!(clamped1, clamped2);
    }
}
```

### Example 5: Bounds Checking

```rust
proptest! {
    #[test]
    fn prop_cache_capacity_within_bounds(capacity in any::<usize>()) {
        // Property: Clamped value must be within min/max bounds
        let clamped = clamp_cache_capacity(capacity);
        assert!(clamped >= MIN_CACHE_CAPACITY);
        assert!(clamped <= MAX_CACHE_CAPACITY);
    }
}
```

### Example 6: No Integer Overflow

```rust
proptest! {
    #[test]
    fn prop_pagination_no_overflow(
        offset in 0usize..MAX_PAGINATION_OFFSET,
        limit in 0usize..MAX_PAGINATION_LIMIT
    ) {
        // Property: Validation prevents overflow
        if validate_pagination(offset, limit).is_ok() {
            assert!(offset.checked_add(limit).is_some());
        }
    }
}
```

### Example 7: State Consistency

```rust
proptest! {
    #[test]
    fn invariant_state_transitions_valid(
        operations in prop::collection::vec(0u8..=3, 1..50)
    ) {
        let manager = ResourceManager::new();

        for op in operations {
            let prev_state = manager.current_state();
            apply_operation(&manager, op);
            let new_state = manager.current_state();

            // Property: State transitions must be valid
            assert!(is_valid_transition(prev_state, new_state));
        }
    }
}
```

## Running Tests in CI/CD

### GitHub Actions Example

```yaml
name: Property Tests

on: [push, pull_request]

jobs:
  property-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run property tests
        run: |
          cargo test property_ --release
      - name: Run security property tests
        run: |
          PROPTEST_CASES=10000 cargo test critical_ --release
      - name: Upload regression artifacts
        if: failure()
        uses: actions/upload-artifact@v2
        with:
          name: proptest-regressions
          path: proptest-regressions/
```

## Debugging Failed Properties

### 1. Reproduce the Failure

```bash
# Proptest saves failing cases to proptest-regressions/
# These are automatically run on subsequent test runs
cargo test prop_failing_test
```

### 2. Use Minimal Failing Input

The shrunk input is in the test output:
```
minimal failing input: row = 0
```

### 3. Add Debug Output

```rust
proptest! {
    #[test]
    fn prop_debug(input in any::<u32>()) {
        eprintln!("Testing input: {}", input);
        let result = validate(input);
        eprintln!("Result: {:?}", result);
        assert!(result.is_ok());
    }
}
```

### 4. Disable Shrinking Temporarily

```bash
# Run without shrinking to see original failure
PROPTEST_MAX_SHRINK_ITERS=0 cargo test prop_failing_test
```

## Further Reading

- [Proptest Documentation](https://docs.rs/proptest/)
- [Property-Based Testing in Rust](https://github.com/BurntSushi/quickcheck)
- [The Design and Use of QuickCheck](https://www.cs.tufts.edu/~nr/cs257/archive/john-hughes/quick.pdf)
- [Property-Based Testing Patterns](https://fsharpforfunandprofit.com/posts/property-based-testing/)

## Contributing

When adding new property tests:

1. Add generators to the appropriate section
2. Document the property being tested
3. Use appropriate case counts (256 default, 10000 for security)
4. Commit regression test files
5. Update this documentation if adding new patterns

## License

This documentation is part of the spreadsheet-mcp project and follows the same license.
