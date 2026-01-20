# Chicago-Style TDD Property-Based Testing Harness

## Overview

This document describes the comprehensive property-based testing harness for the ggen-mcp system. The harness implements **Chicago-style Test-Driven Development** principles combined with **property-based testing** to achieve high confidence in system correctness across the entire input space.

## Table of Contents

1. [Philosophy](#philosophy)
2. [Architecture](#architecture)
3. [Input Generators](#input-generators)
4. [System Properties](#system-properties)
5. [Test Categories](#test-categories)
6. [Usage Guide](#usage-guide)
7. [Shrinking and Debugging](#shrinking-and-debugging)
8. [Performance Considerations](#performance-considerations)
9. [Extending the Harness](#extending-the-harness)

---

## Philosophy

### Chicago-Style TDD

The Chicago School (also called "Classical" or "State-Based") TDD emphasizes:

- **Test through public interfaces**: Don't test implementation details
- **State-based verification**: Assert on outcomes and observable state changes
- **Real collaborators**: Avoid excessive mocking; test real integrations
- **Focus on behavior**: What the system does, not how it does it

### Property-Based Testing

Property-based testing complements TDD by:

- **Automatic input generation**: Test framework generates hundreds of inputs
- **Universal properties**: Define properties that must hold for all inputs
- **Shrinking**: Automatically find minimal failing examples
- **Coverage**: Reach edge cases that manual testing misses

### The 80/20 Principle

The harness focuses on the 20% of input types that cover 80% of system behavior:

1. **TOML Configuration** - System configuration and settings
2. **Turtle/RDF** - Ontology definitions and semantic data
3. **Tera Templates** - Code generation templates and contexts
4. **SPARQL Queries** - Semantic queries and data extraction

---

## Architecture

### Module Structure

```
tests/harness/
├── mod.rs                       # Module exports
└── property_input_harness.rs    # Main harness implementation
```

### Test Organization

```rust
// Configuration constants
const STANDARD_CASES: u32 = 256;
const SECURITY_CASES: u32 = 10_000;
const PERFORMANCE_CASES: u32 = 1_000;

// Input generators (arb_* functions)
// System properties (prop_* tests)
// Invariants (invariant_* tests)
// Shrinking tests
```

---

## Input Generators

### TOML Configuration Generators

#### Valid Configurations

```rust
pub fn arb_valid_toml_config() -> impl Strategy<Value = String>
```

Generates all valid combinations of configuration parameters:
- Workspace root paths
- Cache capacities (within bounds)
- Extension lists
- Transport settings (HTTP/stdio)
- Timeouts and limits
- Feature flags

**Example Generated Config:**
```toml
workspace_root = "/tmp/workspace"
cache_capacity = 42
extensions = ["xlsx", "xlsm"]
transport = "http"
http_bind = "127.0.0.1:8080"
recalc_enabled = true
```

#### Invalid Configurations

```rust
pub fn arb_invalid_toml_config() -> impl Strategy<Value = String>
```

Generates configurations that should fail validation:
- Syntax errors (unclosed brackets, quotes)
- Type errors (strings for numbers, etc.)
- Out-of-bounds values (negative capacities, etc.)
- Missing required fields

**Error Classes Covered:**
- Parse errors
- Type mismatches
- Constraint violations
- Schema violations

#### Edge Cases

```rust
pub fn arb_edge_case_toml_config() -> impl Strategy<Value = String>
```

Tests boundary conditions:
- Minimal valid config (capacity = 1)
- Maximal values (at upper bounds)
- Empty strings and null values
- Special characters and Unicode
- Very long strings

### Turtle/RDF Ontology Generators

#### Valid Ontologies

```rust
pub fn arb_valid_turtle_ontology() -> impl Strategy<Value = String>
```

Generates valid RDF graphs following DDD patterns:
- Standard prefix declarations
- Entity definitions (Aggregates, Entities, Value Objects)
- Relationship triples
- Property constraints
- Type hierarchies

**Example Generated Ontology:**
```turtle
@prefix : <http://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix ddd: <https://ddd-patterns.dev/schema#> .

:OrderAggregate
    a ddd:AggregateRoot ;
    rdfs:label "Order Aggregate" ;
    ddd:hasInvariant :TotalMustBePositive .
```

#### Invalid Ontologies

```rust
pub fn arb_invalid_turtle_ontology() -> impl Strategy<Value = String>
```

Tests constraint violation handling:
- Syntax errors (missing dots, braces)
- Undefined prefixes
- Invalid IRIs (spaces, special chars)
- Malformed literals
- Type errors

#### Edge Cases

```rust
pub fn arb_edge_case_turtle_ontology() -> impl Strategy<Value = String>
```

Boundary and stress tests:
- Empty graphs
- Minimal graphs (single triple)
- Large graphs (1000+ triples)
- Unicode in literals and IRIs
- Very long property values
- Deeply nested structures

### Tera Template Context Generators

#### Valid Contexts

```rust
pub fn arb_valid_tera_context() -> impl Strategy<Value = JsonValue>
```

Generates template rendering contexts:
- Nested JSON objects
- Arrays of various types
- Mixed data types
- Template variables and values
- Metadata and configuration

#### Invalid Contexts

```rust
pub fn arb_invalid_tera_context() -> impl Strategy<Value = JsonValue>
```

Tests error handling:
- Missing required fields
- Wrong value types
- Null where not expected
- Non-object when object required

#### Edge Cases

```rust
pub fn arb_edge_case_tera_context() -> impl Strategy<Value = JsonValue>
```

Special conditions:
- Empty objects and arrays
- Deeply nested structures (10+ levels)
- Large arrays (100+ elements)
- Special characters in strings
- Unicode and control characters
- Very long string values

### SPARQL Query Generators

#### Valid Queries

```rust
pub fn arb_valid_sparql_select() -> impl Strategy<Value = String>
pub fn arb_valid_sparql_construct() -> impl Strategy<Value = String>
pub fn arb_valid_sparql_ask() -> impl Strategy<Value = String>
```

Generates valid queries for all forms:
- SELECT with multiple variables
- CONSTRUCT with triple patterns
- ASK for boolean queries
- FILTER expressions
- OPTIONAL patterns
- UNION clauses

**Example Generated Query:**
```sparql
SELECT ?x ?y ?z WHERE {
    ?x rdf:type :Entity .
    ?x :hasProperty ?y .
    OPTIONAL { ?x :hasMethod ?z }
}
```

#### Invalid Queries

```rust
pub fn arb_invalid_sparql_query() -> impl Strategy<Value = String>
```

Tests error cases:
- Syntax errors (missing braces, keywords)
- Invalid variables (no ? prefix)
- Malformed IRIs
- Injection attempts
- Type errors

#### Edge Cases

```rust
pub fn arb_edge_case_sparql_query() -> impl Strategy<Value = String>
```

Boundary tests:
- Empty result queries
- Many variables (50+)
- Very long queries
- Complex filters
- Unicode in literals
- Nested subqueries

---

## System Properties

### Parsing Properties

#### Property: Valid Input Always Parses

```rust
#[test]
fn prop_toml_valid_always_parses(config_str in arb_valid_toml_config()) {
    let result = serde_yaml::from_str::<Value>(&config_str);
    prop_assert!(result.is_ok(), "Valid TOML should parse");
}
```

**Rationale**: Any input generated as "valid" must successfully parse without errors.

#### Property: Invalid Input Errors Gracefully

```rust
#[test]
fn prop_toml_invalid_errors_gracefully(config_str in arb_invalid_toml_config()) {
    let result = serde_yaml::from_str::<Value>(&config_str);
    // Should not panic - either Ok or Err is fine
}
```

**Rationale**: System must never panic on invalid input; must return proper error.

#### Property: Parse Errors Are Helpful

```rust
#[test]
fn invariant_validation_errors_helpful(input in ".*") {
    let result = validate_cell_address(&input);
    if let Err(err) = result {
        let error_msg = err.to_string();
        prop_assert!(!error_msg.is_empty());
        prop_assert!(error_msg.len() > 10, "Error should be descriptive");
    }
}
```

**Rationale**: Error messages must provide actionable information for debugging.

#### Property: Parsing Is Deterministic

```rust
#[test]
fn prop_toml_parsing_deterministic(config_str in arb_valid_toml_config()) {
    let result1 = parse(&config_str);
    let result2 = parse(&config_str);
    prop_assert_eq!(result1.is_ok(), result2.is_ok());
}
```

**Rationale**: Same input must always produce same result (no randomness).

### Validation Properties

#### Property: Valid Inputs Pass Validation

```rust
#[test]
fn prop_valid_ranges_accepted(
    start_row in arb_row_number(),
    start_col in arb_column_number(),
    end_row in arb_row_number(),
    end_col in arb_column_number()
) {
    if start_row <= end_row && start_col <= end_col {
        let result = validate_range_1based(start_row, start_col, end_row, end_col, "test");
        prop_assert!(result.is_ok());
    }
}
```

**Rationale**: Validation must accept all inputs that meet the specification.

#### Property: Invalid Inputs Fail Validation

```rust
#[test]
fn prop_row_zero_rejected(_unit in Just(())) {
    assert!(validate_row_1based(0, "test").is_err());
}
```

**Rationale**: Validation must reject inputs that violate constraints.

#### Property: Validation Errors Are Specific

```rust
#[test]
fn prop_validation_errors_specific(input in ".*") {
    if let Err(err) = validate_workbook_id(&input) {
        let msg = err.to_string();
        prop_assert!(
            msg.contains("workbook") || msg.contains("id"),
            "Error should mention the validated field"
        );
    }
}
```

**Rationale**: Users need to know *what* failed validation, not just *that* it failed.

#### Property: Validation Is Consistent

```rust
#[test]
fn prop_validation_consistent(input in ".*") {
    let result1 = validate_sheet_name(&input);
    let result2 = validate_sheet_name(&input);
    prop_assert_eq!(result1.is_ok(), result2.is_ok());
}
```

**Rationale**: Validation must be deterministic across multiple calls.

### Generation Properties

#### Property: Generated Code Always Compiles

```rust
#[test]
fn prop_generated_code_compiles(ontology in arb_valid_turtle_ontology()) {
    let code = generate_code_from_ontology(&ontology)?;
    assert_compiles(&code);
}
```

**Rationale**: Code generator must never produce syntactically invalid Rust.

#### Property: Generated Code Passes Clippy

```rust
#[test]
fn prop_generated_code_passes_clippy(ontology in arb_valid_turtle_ontology()) {
    let code = generate_code_from_ontology(&ontology)?;
    assert_passes_clippy(&code);
}
```

**Rationale**: Generated code should follow Rust best practices and conventions.

#### Property: Generated Code Matches Schema

```rust
#[test]
fn prop_generated_matches_schema(ontology in arb_valid_turtle_ontology()) {
    let code = generate_code_from_ontology(&ontology)?;
    let parsed = syn::parse_file(&code)?;
    verify_matches_ontology(&parsed, &ontology);
}
```

**Rationale**: Generated code must accurately reflect the ontology specification.

#### Property: Code Generation Is Deterministic

```rust
#[test]
fn prop_generation_deterministic(ontology in arb_valid_turtle_ontology()) {
    let code1 = generate_code_from_ontology(&ontology)?;
    let code2 = generate_code_from_ontology(&ontology)?;
    prop_assert_eq!(code1, code2);
}
```

**Rationale**: Same ontology must always generate identical code (reproducible builds).

### Round-Trip Properties

#### Property: TOML Round-Trip

```rust
#[test]
fn prop_toml_roundtrip(config_str in arb_valid_toml_config()) {
    let parsed1 = parse_toml(&config_str)?;
    let serialized = serialize_toml(&parsed1)?;
    let parsed2 = parse_toml(&serialized)?;
    prop_assert_eq!(parsed1, parsed2);
}
```

**Rationale**: Parse → Serialize → Parse must preserve all data (lossless).

#### Property: Turtle Round-Trip

```rust
#[test]
fn prop_turtle_roundtrip(ttl in arb_valid_turtle_ontology()) {
    let store1 = parse_turtle(&ttl)?;
    let serialized = serialize_turtle(&store1)?;
    let store2 = parse_turtle(&serialized)?;
    prop_assert_eq!(store1.len(), store2.len());
}
```

**Rationale**: RDF serialization must preserve graph structure and semantics.

#### Property: Code Round-Trip

```rust
#[test]
fn prop_code_roundtrip(ontology in arb_valid_turtle_ontology()) {
    let code1 = generate_code(&ontology)?;
    let parsed = parse_rust(&code1)?;
    let ontology2 = extract_ontology(&parsed)?;
    let code2 = generate_code(&ontology2)?;
    prop_assert_eq!(code1, code2);
}
```

**Rationale**: Code generation should be reversible (for round-trip tooling).

---

## Test Categories

### 1. Standard Property Tests

**Configuration**: 256 test cases per property

**Purpose**: Verify core system behavior across typical inputs

**Coverage**:
- Valid input acceptance
- Invalid input rejection
- Edge case handling
- Basic round-trips

**Example**:
```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn prop_valid_cell_addresses_accepted(address in arb_cell_address()) {
        assert!(validate_cell_address(&address).is_ok());
    }
}
```

### 2. Security Critical Tests

**Configuration**: 10,000 test cases per property

**Purpose**: Exhaustively test security-critical code paths

**Coverage**:
- SPARQL injection prevention
- Path traversal prevention
- No panics on malicious input
- Sanitization effectiveness

**Example**:
```rust
proptest! {
    #![proptest_config(ProptestConfig {
        cases: 10_000,
        ..Default::default()
    })]

    #[test]
    fn critical_no_sparql_injection(malicious in ".*") {
        let result = SparqlSanitizer::escape_string(&malicious);
        if let Ok(escaped) = result {
            assert!(!escaped.contains("'; DROP"));
        }
    }
}
```

### 3. Performance Tests

**Configuration**: 1,000 test cases with timeout checks

**Purpose**: Ensure operations complete within time bounds

**Coverage**:
- Parse time linear in input size
- Generation time bounded
- Memory usage reasonable
- No exponential blowup

**Example**:
```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(1_000))]

    #[test]
    fn prop_parsing_time_bounded(input in arb_valid_toml_config()) {
        let start = Instant::now();
        let _ = parse_toml(&input);
        let elapsed = start.elapsed();
        prop_assert!(elapsed < Duration::from_millis(100));
    }
}
```

### 4. Invariant Tests

**Configuration**: Various (property-dependent)

**Purpose**: Verify system invariants always hold

**Coverage**:
- State consistency
- No memory leaks
- No panics
- No data corruption
- No security violations

**Example**:
```rust
proptest! {
    #[test]
    fn invariant_system_state_consistent(
        operations in vec((bool, String), 1..=10)
    ) {
        let store = Store::new()?;
        for (should_load, ttl) in operations {
            if should_load {
                let _ = store.load_turtle(&ttl);
            }
            // INVARIANT: Store is always queryable
            prop_assert!(store.len().is_ok());
        }
    }
}
```

### 5. Shrinking Tests

**Configuration**: Explicit test runner configuration

**Purpose**: Verify proptest shrinking finds minimal failing cases

**Coverage**:
- Minimal examples produced
- Shrunk cases still fail
- Shrinking completes in time
- Property preserved during shrinking

**Example**:
```rust
#[test]
fn test_shrinking_finds_minimal_failing_case() {
    let result = proptest!(|(n in 0u32..1000)| {
        if n > 100 {
            prop_assert!(n <= 100);
        }
    });

    // Should fail at n = 101 (minimal failing case)
    assert!(result.is_err());
}
```

---

## Usage Guide

### Running All Property Tests

```bash
# Run full harness
cargo test --test property_input_harness

# Run with verbose output
cargo test --test property_input_harness -- --nocapture

# Run specific test
cargo test --test property_input_harness prop_toml_valid_always_parses
```

### Configuring Test Cases

Use environment variables to override defaults:

```bash
# Run with 10,000 cases per test
PROPTEST_CASES=10000 cargo test --test property_input_harness

# Enable verbose shrinking
PROPTEST_VERBOSE=1 cargo test --test property_input_harness

# Set maximum shrinking iterations
PROPTEST_MAX_SHRINK_ITERS=10000 cargo test --test property_input_harness

# Disable shrinking (faster but less useful for debugging)
PROPTEST_MAX_SHRINK_ITERS=0 cargo test --test property_input_harness
```

### Targeting Specific Property Categories

```bash
# Run only TOML tests
cargo test --test property_input_harness prop_toml

# Run only security tests
cargo test --test property_input_harness critical_

# Run only invariants
cargo test --test property_input_harness invariant_

# Run only round-trip tests
cargo test --test property_input_harness roundtrip
```

### Continuous Integration

Add to `.github/workflows/tests.yml`:

```yaml
- name: Run Property-Based Tests
  run: |
    # Standard tests (fast)
    cargo test --test property_input_harness

    # Security tests (more cases)
    PROPTEST_CASES=10000 cargo test --test property_input_harness critical_

    # Performance tests
    cargo test --test property_input_harness prop_.*_time_bounded
```

---

## Shrinking and Debugging

### Understanding Shrinking

When a property test fails, proptest automatically **shrinks** the failing input to find the minimal example that still fails.

**Example**:
```
thread 'prop_validate_row_number' panicked at 'assertion failed'

Shrinking input:
  Attempt 1: row = 500 ✓ passes
  Attempt 2: row = 250 ✓ passes
  Attempt 3: row = 125 ✓ passes
  Attempt 4: row = 62 ✗ fails
  Attempt 5: row = 93 ✓ passes
  ...
  Attempt 42: row = 101 ✗ fails
  Attempt 43: row = 100 ✓ passes

Minimal failing case: row = 101
```

### Reading Shrinking Output

When a test fails, you'll see:

```
thread 'prop_toml_valid_always_parses' panicked at 'assertion failed'

minimal failing input: config_str = "cache_capacity = 0"
```

This is the **simplest input** that triggers the bug.

### Debugging Shrunk Cases

1. **Copy the minimal input** from test output
2. **Create a unit test** with that exact input:

```rust
#[test]
fn debug_cache_capacity_zero() {
    let config = "cache_capacity = 0";
    let result = parse_toml(config);
    // Set breakpoint here
    assert!(result.is_ok());
}
```

3. **Run with debugger** to step through code
4. **Fix the bug**
5. **Verify fix** by re-running property test

### Preserving Failing Cases

To save a failing case for regression testing:

```rust
#[test]
fn regression_cache_capacity_zero() {
    // Discovered by property test on 2024-01-20
    let config = "cache_capacity = 0";
    let result = parse_toml(config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must be at least 1"));
}
```

### Controlling Shrinking

```rust
proptest! {
    #![proptest_config(ProptestConfig {
        max_shrink_iters: 10_000, // More iterations = smaller example
        max_shrink_time: 60_000,   // Stop after 60 seconds
        ..Default::default()
    })]

    #[test]
    fn prop_test_with_custom_shrinking(input in strategy) {
        // ...
    }
}
```

---

## Performance Considerations

### Test Execution Time

**Standard Configuration** (256 cases):
- TOML tests: ~2-5 seconds
- Turtle tests: ~10-20 seconds (parsing overhead)
- SPARQL tests: ~5-10 seconds
- Tera tests: ~1-3 seconds
- **Total**: ~20-40 seconds

**Security Configuration** (10,000 cases):
- SPARQL injection tests: ~60-120 seconds
- Path traversal tests: ~30-60 seconds
- **Total**: ~2-3 minutes

### Optimization Strategies

#### 1. Parallel Execution

```bash
# Run tests in parallel (default)
cargo test --test property_input_harness

# Control parallelism
cargo test --test property_input_harness -- --test-threads=4
```

#### 2. Targeted Testing

Run expensive tests only in CI:

```rust
#[cfg_attr(not(feature = "ci"), ignore)]
#[test]
fn expensive_security_test() {
    // Only runs with --features ci
}
```

#### 3. Caching

Proptest automatically caches successful shrinking attempts:

```bash
# Cache location
.proptest-regressions/
└── property_input_harness.txt
```

Don't `.gitignore` this - it preserves discovered edge cases!

#### 4. Profiling

Find slow tests:

```bash
# Run with timing
cargo test --test property_input_harness -- --nocapture --show-output

# Profile with flamegraph
cargo flamegraph --test property_input_harness
```

### Memory Usage

**Expected Memory**:
- Standard tests: ~50-100 MB
- Security tests: ~200-500 MB (more cases)
- Turtle tests: ~100-300 MB (RDF store overhead)

**Memory Leaks**:
- The `invariant_no_memory_leaks` test verifies this
- Run with `valgrind` for deep analysis:

```bash
valgrind --leak-check=full \
  cargo test --test property_input_harness prop_test_name
```

---

## Extending the Harness

### Adding New Input Types

1. **Create generator**:

```rust
pub fn arb_my_input() -> impl Strategy<Value = MyType> {
    // Define generation strategy
    prop::string::string_regex(r"[a-z]+")
        .expect("valid regex")
        .prop_map(|s| MyType::new(s))
}
```

2. **Add property tests**:

```rust
proptest! {
    #[test]
    fn prop_my_input_valid_parses(input in arb_my_input()) {
        let result = parse_my_input(&input);
        prop_assert!(result.is_ok());
    }
}
```

3. **Add edge cases**:

```rust
pub fn arb_my_input_edge_cases() -> impl Strategy<Value = MyType> {
    prop_oneof![
        Just(MyType::empty()),
        Just(MyType::minimal()),
        Just(MyType::maximal()),
    ]
}
```

### Adding New Properties

1. **Identify universal property**:
   - "All valid X should Y"
   - "No X should ever Z"
   - "X then Y then X should return to original state"

2. **Write property test**:

```rust
proptest! {
    #[test]
    fn prop_my_new_property(input in arb_my_input()) {
        let result = my_operation(&input);
        prop_assert!(my_invariant_holds(&result));
    }
}
```

3. **Document the property**:

```rust
/// Property: My operation preserves data integrity
///
/// Rationale: Users depend on data not being corrupted
/// during this operation, so we test that all data
/// remains valid after processing.
#[test]
fn prop_my_new_property(...) {
    // ...
}
```

### Adding Custom Shrinking

For complex types, implement custom shrinking:

```rust
impl Arbitrary for MyComplexType {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        // Generation strategy
        (arb_field1(), arb_field2())
            .prop_map(|(f1, f2)| MyComplexType { f1, f2 })
            .boxed()
    }
}

impl MyComplexType {
    fn shrink(&self) -> impl Iterator<Item = Self> {
        // Custom shrinking logic
        // Return simpler versions of self
        vec![
            Self::simplified_version_1(),
            Self::simplified_version_2(),
        ].into_iter()
    }
}
```

---

## Best Practices

### 1. Write Properties, Not Examples

**Bad** (example-based):
```rust
#[test]
fn test_parse_config() {
    let config = "cache_capacity = 10";
    assert!(parse_config(config).is_ok());
}
```

**Good** (property-based):
```rust
proptest! {
    #[test]
    fn prop_valid_configs_parse(config in arb_valid_config()) {
        prop_assert!(parse_config(&config).is_ok());
    }
}
```

### 2. Test One Property Per Test

**Bad** (multiple properties):
```rust
#[test]
fn prop_parse_and_validate(config in arb_config()) {
    let parsed = parse(config)?;
    assert!(parsed.is_valid());
    assert!(parsed.capacity > 0);
    assert!(serialize(parsed).is_ok());
}
```

**Good** (focused properties):
```rust
#[test]
fn prop_parse_succeeds(config in arb_valid_config()) {
    prop_assert!(parse(config).is_ok());
}

#[test]
fn prop_parsed_is_valid(config in arb_valid_config()) {
    let parsed = parse(config)?;
    prop_assert!(parsed.is_valid());
}
```

### 3. Use Descriptive Property Names

**Bad**:
```rust
fn prop_test1(x in any::<u32>()) { ... }
```

**Good**:
```rust
fn prop_cache_capacity_never_exceeds_limit(capacity in 1..=1000) { ... }
```

### 4. Document Rationale

```rust
/// Property: Validation errors contain field name
///
/// Rationale: When validation fails, users need to know
/// WHICH field failed, not just that validation failed.
/// This property ensures all error messages include context.
#[test]
fn prop_validation_errors_include_field_name(...) {
    // ...
}
```

### 5. Preserve Failing Cases

When a property test finds a bug:

1. Fix the bug
2. Add the failing case as a regression test
3. Document what was discovered

```rust
/// Regression test for issue #123
///
/// Property test discovered that cache_capacity=0 was accepted
/// but caused division by zero. This test ensures the fix holds.
#[test]
fn regression_cache_capacity_zero_rejected() {
    let config = "cache_capacity = 0";
    assert!(parse_config(config).is_err());
}
```

---

## Troubleshooting

### Test Failures

#### "Shrinking timed out"

**Cause**: Shrinking is taking too long (>60 seconds default)

**Solution**: Increase timeout or simplify input space

```rust
proptest! {
    #![proptest_config(ProptestConfig {
        max_shrink_time: 120_000, // 2 minutes
        ..Default::default()
    })]
}
```

#### "Too many rejections"

**Cause**: Generator produces many invalid inputs that are filtered out

**Solution**: Make generator more precise

```rust
// Bad: Many rejections
prop::string::string_regex(".*")
    .prop_filter("valid cell", |s| validate_cell(s).is_ok())

// Good: Generate only valid inputs
arb_column_letters()
```

#### "No failures with this seed"

**Cause**: Proptest uses deterministic PRNG; different seed = different inputs

**Solution**: Save the seed from failure output, replay it

```bash
# Failure shows: seed = 0x1234567890abcdef
PROPTEST_SEED=0x1234567890abcdef cargo test --test property_input_harness
```

### Performance Issues

#### Tests Taking Too Long

**Diagnosis**:
```bash
# Profile tests
cargo test --test property_input_harness -- --nocapture

# Find slow tests
cargo test --test property_input_harness -- --test-threads=1 --nocapture
```

**Solutions**:
- Reduce `PROPTEST_CASES`
- Simplify generators
- Run expensive tests only in CI
- Use `#[ignore]` for slow tests

#### Memory Usage High

**Diagnosis**:
```bash
# Monitor memory
/usr/bin/time -v cargo test --test property_input_harness
```

**Solutions**:
- Check for memory leaks with `valgrind`
- Reduce size of generated inputs
- Run memory-intensive tests separately

---

## References

### Papers and Books

- **Property-Based Testing**: [QuickCheck paper (Claessen & Hughes, 2000)](https://www.cs.tufts.edu/~nr/cs257/archive/john-hughes/quick.pdf)
- **Shrinking**: [Integrated Shrinking (Li et al., 2019)](https://www.cs.tufts.edu/~nr/cs257/archive/john-hughes/quick.pdf)
- **Chicago TDD**: [Growing Object-Oriented Software (Freeman & Pryce)](http://www.growing-object-oriented-software.com/)

### Tools

- [proptest documentation](https://docs.rs/proptest/)
- [proptest-derive](https://docs.rs/proptest-derive/) for automatic `Arbitrary` derivation
- [test-strategy](https://docs.rs/test-strategy/) for convenient macros

### Project Resources

- [VALIDATION_ARCHITECTURE.md](./VALIDATION_ARCHITECTURE.md) - Validation system design
- [TPS_IMPLEMENTATION.md](./TPS_IMPLEMENTATION.md) - Toyota Production System principles
- [POKA_YOKE.md](./POKA_YOKE.md) - Error-proofing mechanisms

---

## Appendix: Property Test Patterns

### Pattern: Idempotence

```rust
proptest! {
    #[test]
    fn prop_operation_idempotent(input in arb_input()) {
        let result1 = operation(&input);
        let result2 = operation(&result1);
        prop_assert_eq!(result1, result2);
    }
}
```

### Pattern: Commutativity

```rust
proptest! {
    #[test]
    fn prop_operation_commutative(a in arb_input(), b in arb_input()) {
        let result1 = operation(a, b);
        let result2 = operation(b, a);
        prop_assert_eq!(result1, result2);
    }
}
```

### Pattern: Associativity

```rust
proptest! {
    #[test]
    fn prop_operation_associative(
        a in arb_input(),
        b in arb_input(),
        c in arb_input()
    ) {
        let result1 = operation(operation(a, b), c);
        let result2 = operation(a, operation(b, c));
        prop_assert_eq!(result1, result2);
    }
}
```

### Pattern: Inverse

```rust
proptest! {
    #[test]
    fn prop_operation_has_inverse(input in arb_input()) {
        let transformed = operation(&input);
        let restored = inverse_operation(&transformed);
        prop_assert_eq!(input, restored);
    }
}
```

### Pattern: Invariant Preservation

```rust
proptest! {
    #[test]
    fn prop_operation_preserves_invariant(input in arb_input()) {
        prop_assert!(check_invariant(&input));
        let result = operation(&input);
        prop_assert!(check_invariant(&result));
    }
}
```

### Pattern: Oracle (Model-Based)

```rust
proptest! {
    #[test]
    fn prop_matches_reference_implementation(input in arb_input()) {
        let actual = fast_implementation(&input);
        let expected = reference_implementation(&input);
        prop_assert_eq!(actual, expected);
    }
}
```

---

## Changelog

### 2024-01-20 - Initial Release

- Implemented comprehensive property-based test harness
- Added generators for all major input types (TOML, Turtle, Tera, SPARQL)
- Defined system properties for parsing, validation, generation, round-trips
- Configured standard (256), security (10K), and performance (1K) test suites
- Added invariant tests for system state consistency
- Implemented shrinking verification tests
- Documented usage, extension, and troubleshooting

---

**Maintained by**: ggen-mcp contributors
**Last Updated**: 2024-01-20
**Status**: Production Ready
