# Test Agent

**Purpose**: Orchestrate Chicago-style TDD and coverage verification

**Trigger**: Manual invocation or before pre-commit

## Workflow (Test-First)

### 1. Define Behavior
```rust
// Before implementation, define what should happen
#[test]
fn validates_sheet_name_length() {
    // Should accept <= 31 chars
    assert!(validate_sheet_name("SheetName").is_ok());
    
    // Should reject > 31 chars
    assert!(validate_sheet_name(&"x".repeat(32)).is_err());
}
```

### 2. Implement Until Test Passes
```rust
pub fn validate_sheet_name(name: &str) -> Result<SheetName> {
    if name.len() > 31 {
        return Err(Error::ValidationFailed {
            reason: "Sheet name exceeds 31 characters".to_string(),
        });
    }
    Ok(SheetName(name.to_string()))
}
// cargo test → ✓ PASS
```

### 3. Refine and Cover Error Cases
```rust
#[test]
fn rejects_empty_sheet_name() {
    assert!(validate_sheet_name("").is_err());
}

#[test]
fn rejects_whitespace_only_name() {
    assert!(validate_sheet_name("   ").is_err());
}
```

## Test Commands

```bash
# All tests
cargo test

# Specific module
cargo test validation::tests

# Integration tests
cargo test --test integration

# Show output
cargo test -- --nocapture

# Single test
cargo test validate_sheet_name

# With backtrace
RUST_BACKTRACE=1 cargo test
```

## Coverage Verification

```bash
# Generate HTML coverage report
./scripts/coverage.sh --html

# Check against targets
./scripts/coverage.sh --check

# Expected results:
# Security paths: 95%+
# Core handlers: 80%+
# Generated code: 85%+
```

## Patterns (Do's)

### Do: State-Based Testing
```rust
#[test]
fn workbook_sheet_count_increases() {
    let mut wb = Workbook::new("test").unwrap();
    assert_eq!(wb.sheet_count(), 0);
    
    wb.add_sheet("Sheet1").unwrap();
    assert_eq!(wb.sheet_count(), 1);  // Verify state changed
    
    wb.add_sheet("Sheet2").unwrap();
    assert_eq!(wb.sheet_count(), 2);  // Not just "called twice"
}
```

### Do: Test Error Paths
```rust
#[test]
fn duplicate_sheet_names_rejected() {
    let mut wb = Workbook::new("test").unwrap();
    wb.add_sheet("Sheet").unwrap();
    
    // Should fail
    let result = wb.add_sheet("Sheet");
    assert!(result.is_err());
    
    // Verify: state unchanged on error (transaction-like)
    assert_eq!(wb.sheet_count(), 1);
}
```

### Do: Use Fixtures
```rust
fn setup_workbook() -> Workbook {
    let mut wb = Workbook::new("test").unwrap();
    wb.add_sheet("Data").unwrap();
    wb
}

#[test]
fn operation_with_fixture() {
    let mut wb = setup_workbook();
    // Test with pre-initialized state
    assert_eq!(wb.sheet_count(), 1);
}
```

### Do: Property-Based Tests
```rust
#[test]
fn sheet_name_length_property() {
    for len in [1, 5, 15, 31] {
        let name = "x".repeat(len);
        assert!(validate_sheet_name(&name).is_ok(),
                "Should accept {} chars", len);
    }
}
```

## Patterns (Don'ts)

### Don't: Mock Core Types
```rust
// ✗ Don't
let mock_wb = MockWorkbook::new();

// ✓ Use real implementation
let wb = Workbook::new("test").unwrap();
```

### Don't: Test Implementation Details
```rust
// ✗ Don't
expect(process_called).times(3);

// ✓ Test observable behavior
assert_eq!(wb.sheet_count(), 3);
```

### Don't: Ignore Error Cases
```rust
// ✗ Don't (happy path only)
#[test]
fn add_sheet_works() { ... }

// ✓ Test failures equally
#[test]
fn rejects_duplicate_names() { ... }
#[test]
fn recovers_from_error() { ... }
```

### Don't: Over-Specialize Tests
```rust
// ✗ Don't (brittle to refactoring)
#[test]
fn uses_string_interning() { ... }

// ✓ Test domain behavior
#[test]
fn sheet_names_unique() { ... }
```

## Generated Code Testing

### Auto-Tests from Generation
```bash
# Generated code includes tests
cargo test --test generated_*

# Verify TODO count (must be zero)
grep -r "TODO" src/generated/
# Expected: (empty)
```

### Snapshot Testing
```rust
#[test]
fn generated_code_preserves_format() {
    let generated = generate_from_ontology();
    insta::assert_snapshot!(generated);
    // Fail on unwanted diffs
}
```

## Benchmarks

```bash
cargo bench                    # Run all benchmarks
cargo bench --bench name       # Specific benchmark

# Example benchmark
#[bench]
fn bench_add_sheet(b: &mut Bencher) {
    let mut wb = Workbook::new("test").unwrap();
    b.iter(|| wb.add_sheet("Sheet"));
}
```

## Agent Commands
```bash
# Run all tests with coverage
claude-code test-agent verify

# Run specific test suite
claude-code test-agent run --suite validation

# Check coverage targets
claude-code test-agent coverage-check

# Generate HTML report
claude-code test-agent coverage-report

# Run benchmarks
claude-code test-agent bench
```

## Output Template
```
## Test Execution
- Command: cargo test
- Duration: 2.5s
- Results: 120 passed, 0 failed
- Skipped: 0

## Coverage Report
- Security paths: 96% (target: 95%+) ✓
- Core handlers: 82% (target: 80%+) ✓
- Generated code: 87% (target: 85%+) ✓
- Overall: 84%

## Benchmarks (if applicable)
- add_sheet: 1.2µs (baseline)
- validate_name: 150ns (baseline)
- No regressions detected

**VERDICT**: ✓ All tests pass. Coverage targets met. Ready to commit.
```

---

**Testing philosophy: Real implementations. Domain properties. Error cases first. Coverage metrics drive quality.**
