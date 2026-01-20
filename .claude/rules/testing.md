# Testing Rules (Chicago-Style TDD)

**Version**: 1.2.0 | State-based, Real implementations, Minimal mocking

## Core Principles
```
State-based     → Test object state changes, not call sequences
Real impl       → Use actual implementations, minimal mocks
Integration     → Test component interactions end-to-end
Chicago-style   → Focus on behavior, verify domain properties
```

## Test Structure

### Unit Tests (Poka-Yoke Layer)
```rust
#[cfg(test)]
mod validation_tests {
    use super::*;

    // State-based: test what changed, not how
    #[test]
    fn validate_non_empty_string_accepts_valid() {
        let result = validate_non_empty_string("valid");
        assert!(result.is_ok());
    }

    #[test]
    fn validate_non_empty_string_rejects_empty() {
        let result = validate_non_empty_string("");
        assert!(result.is_err());
    }

    // NewType safety
    #[test]
    fn workbook_id_type_safety() {
        let id: WorkbookId = WorkbookId("test".to_string());
        // Cannot accidentally use as ForkId — type system prevents
        // Verify: conversion tests exist but never implicit coercion
    }
}
```

### Integration Tests (Real Implementation)
```rust
#[test]
fn spreadsheet_operation_flow() {
    // Real Workbook instance, no mocks
    let mut wb = Workbook::new("test-workbook").unwrap();
    
    // Assert state changes, not call sequences
    assert_eq!(wb.sheet_count(), 0);
    wb.add_sheet("Sheet1").unwrap();
    assert_eq!(wb.sheet_count(), 1);
    
    // Verify domain property: sheets are ordered
    assert_eq!(wb.sheet_at(0).unwrap().name(), "Sheet1");
}
```

### Coverage Targets
```
Security paths      → 95%+ (validation, auth, error handling)
Core handlers       → 80%+ (business logic)
Generated code      → 85%+ (ensure ggen quality)
Edge cases          → Boundary testing (off-by-one, empty, max)
```

## Commands

```bash
cargo test                      # All tests
cargo test --test name          # Specific integration suite
./scripts/coverage.sh --html    # Coverage report (HTML)
./scripts/coverage.sh --check   # Check against targets
cargo test -- --ignored        # Run ignored (slow) tests
cargo bench                     # Performance benchmarks
```

## Patterns (Do's)

### Do: Property-Based Testing
```rust
#[test]
fn sheet_names_never_empty_property() {
    // Property: any valid sheet has non-empty name
    let sheet = Sheet::new("Valid").unwrap();
    assert!(!sheet.name().is_empty());
}
```

### Do: Test Error Paths
```rust
#[test]
fn workbook_rejects_duplicate_sheet_names() {
    let mut wb = Workbook::new("test").unwrap();
    wb.add_sheet("Sheet").unwrap();
    
    let result = wb.add_sheet("Sheet");
    assert!(result.is_err());
    assert_eq!(wb.sheet_count(), 1);  // Verify: state unchanged on error
}
```

### Do: Use Fixtures
```rust
fn create_test_workbook() -> Workbook {
    let mut wb = Workbook::new("test").unwrap();
    wb.add_sheet("Data").unwrap();
    wb
}

#[test]
fn operation_with_fixture() {
    let wb = create_test_workbook();
    assert_eq!(wb.sheet_count(), 1);
}
```

## Patterns (Don'ts)

### Don't: Mock Core Types
```rust
// ✗ Don't mock
let mock_workbook = MockWorkbook::new();

// ✓ Use real Workbook
let wb = Workbook::new("test").unwrap();
```

### Don't: Test Implementation Details
```rust
// ✗ Test call sequence
expect_fn(call_count).times(3);

// ✓ Test state change
assert_eq!(wb.sheet_count(), 3);
```

### Don't: Over-Test Happy Path
```rust
// ✗ Only happy path tests
#[test]
fn valid_input_works() { ... }

// ✓ Test error cases equally
#[test]
fn rejects_invalid_input() { ... }
#[test]
fn recovers_after_error() { ... }
```

## Generated Code Testing

```bash
# Verify generated code has tests
cargo test --test generated_*

# Check TODO count (must be zero)
grep -r "TODO" src/generated/
# Expected output: (empty)
```

## Snapshot Testing (Validation Output)
```rust
#[test]
fn generated_code_snapshot() {
    let generated = generate_from_ontology();
    insta::assert_snapshot!(generated);
    // Verify: format preserved, no unwanted diffs on sync
}
```

---

**Test philosophy: Real implementations. Domain properties. Error paths matter equally.**
