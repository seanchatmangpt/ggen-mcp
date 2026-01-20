# Code Coverage Guide

This document describes the code coverage tracking and improvement process for the ggen-mcp project.

## Table of Contents

- [Overview](#overview)
- [Coverage Targets](#coverage-targets)
- [Running Coverage Locally](#running-coverage-locally)
- [CI/CD Integration](#cicd-integration)
- [Interpreting Coverage Reports](#interpreting-coverage-reports)
- [Improving Coverage](#improving-coverage)
- [Best Practices](#best-practices)

## Overview

Code coverage measures how much of our codebase is exercised by our test suite. We use `cargo-llvm-cov` for accurate, LLVM-based coverage tracking.

### Why Coverage Matters

- **Quality Assurance**: Higher coverage reduces the likelihood of undetected bugs
- **Confidence**: Well-tested code can be refactored with confidence
- **Documentation**: Tests serve as executable documentation
- **Security**: Critical security code must be thoroughly tested

## Coverage Targets

We maintain different coverage targets based on the criticality of code:

| Category | Target | Priority | Rationale |
|----------|--------|----------|-----------|
| **Security Code** | **95%+** | **Critical** | SPARQL injection prevention, input validation, path traversal prevention |
| **Core Handlers** | **80%+** | **High** | MCP tool handlers, state management, fork operations, cache operations |
| **Error Paths** | **70%+** | **High** | Error handling, validation failures, recovery mechanisms |
| **Business Logic** | **80%+** | **Medium** | Domain logic, transformations, calculations |
| **Utilities** | **60%+** | **Medium** | Helper functions, formatting, conversions |
| **Generated Code** | **40%+** | **Low** | Auto-generated code (lower priority, may be excluded) |

### Security Code Coverage

Security-critical code requires the highest coverage:

- **SPARQL Injection Prevention** (`src/sparql/injection_prevention.rs`) → 95%+
  - Test all injection patterns
  - Test escape mechanisms
  - Test validation edge cases

- **Input Validation** (`src/validation/`) → 95%+
  - Test all validation rules
  - Test boundary conditions
  - Test malformed inputs

- **Path Traversal Prevention** → 95%+
  - Test directory traversal attempts
  - Test path normalization
  - Test symlink handling

## Running Coverage Locally

### Prerequisites

```bash
# Install cargo-llvm-cov
cargo install cargo-llvm-cov

# Verify installation
cargo llvm-cov --version
```

### Quick Start

```bash
# Generate HTML coverage report
./scripts/coverage.sh --html --open

# Generate LCOV report for CI integration
./scripts/coverage.sh --lcov

# Generate text report to stdout
./scripts/coverage.sh --text

# Check coverage thresholds
./scripts/coverage.sh --check
```

### Script Options

```bash
./scripts/coverage.sh [OPTIONS]

OPTIONS:
  --html          Generate HTML report (default)
  --lcov          Generate LCOV report for CI/codecov
  --json          Generate JSON report
  --text          Generate text report to stdout
  --open          Open HTML report in browser
  --clean         Clean coverage data before running
  --check         Check coverage against target thresholds
  --verbose, -v   Verbose output
  --help, -h      Show help message
```

### Manual Coverage Commands

```bash
# Basic coverage report
cargo llvm-cov --all-features --workspace

# HTML report
cargo llvm-cov --all-features --workspace --html

# LCOV format (for CI)
cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info

# JSON format
cargo llvm-cov --all-features --workspace --json --output-path coverage.json

# Coverage for specific package
cargo llvm-cov --package spreadsheet-mcp

# Coverage for specific test
cargo llvm-cov --test sparql_injection_tests
```

## CI/CD Integration

### GitHub Actions Workflow

Coverage is automatically generated on:
- Push to `main` branch
- Pull requests
- Daily schedule (2 AM UTC)

The workflow:
1. Generates LCOV coverage report
2. Uploads to Codecov (if configured)
3. Generates HTML report as artifact
4. Posts coverage summary on PRs

### Viewing CI Coverage Reports

1. **Codecov Dashboard** (if configured)
   - Visit https://codecov.io/gh/YOUR_ORG/ggen-mcp
   - View coverage trends over time
   - See coverage diff on PRs

2. **GitHub Artifacts**
   - Go to Actions → Coverage workflow
   - Download the `coverage-report` artifact
   - Open `index.html` in your browser

### Setting Up Codecov

1. Sign up at https://codecov.io
2. Add your repository
3. Add `CODECOV_TOKEN` to GitHub Secrets
4. Coverage will be uploaded automatically

## Interpreting Coverage Reports

### HTML Report Structure

```
target/llvm-cov/html/
├── index.html          # Overall coverage summary
├── src/                # Source file coverage
│   ├── sparql/
│   │   └── injection_prevention.rs.html
│   └── ...
└── tests/              # Test file coverage (optional)
```

### Coverage Metrics

- **Line Coverage**: Percentage of code lines executed
- **Function Coverage**: Percentage of functions called
- **Branch Coverage**: Percentage of decision branches taken

### Color Coding

- **Green**: Well-covered code (>80%)
- **Yellow**: Partially covered code (50-80%)
- **Red**: Poorly covered code (<50%)

### Finding Gaps

1. Open the HTML report
2. Sort files by coverage percentage
3. Focus on:
   - Red/yellow files in critical categories
   - Security-related code below 95%
   - Core handlers below 80%
4. Click on files to see uncovered lines

## Improving Coverage

### Step-by-Step Process

1. **Identify Gaps**
   ```bash
   ./scripts/coverage.sh --html --open
   ```

2. **Prioritize**
   - Start with security code
   - Then core handlers
   - Then error paths
   - Finally utilities

3. **Write Tests**
   - Unit tests for functions
   - Integration tests for workflows
   - Edge case tests for boundary conditions

4. **Verify Improvement**
   ```bash
   ./scripts/coverage.sh --check
   ```

### Example: Adding Tests for SPARQL Injection

```rust
// File: tests/sparql_injection_coverage_tests.rs

#[test]
fn test_sql_injection_patterns() {
    let validator = InjectionValidator::new();

    // Test basic SQL injection patterns
    assert!(validator.detect_injection("'; DROP TABLE--").is_err());
    assert!(validator.detect_injection("1' OR '1'='1").is_err());

    // Test SPARQL-specific patterns
    assert!(validator.detect_injection("} INSERT DATA {").is_err());
    assert!(validator.detect_injection("} DELETE WHERE {").is_err());

    // Test encoded patterns
    assert!(validator.detect_injection("%7D%20INSERT%20DATA%20%7B").is_err());
}

#[test]
fn test_escape_mechanisms() {
    let escaper = QueryEscaper::new();

    // Test single quote escaping
    assert_eq!(escaper.escape("O'Reilly"), "O\\'Reilly");

    // Test double quote escaping
    assert_eq!(escaper.escape("\"quoted\""), "\\\"quoted\\\"");

    // Test backslash escaping
    assert_eq!(escaper.escape("C:\\path"), "C:\\\\path");
}

#[test]
fn test_validation_edge_cases() {
    let validator = InjectionValidator::new();

    // Test empty string
    assert!(validator.detect_injection("").is_ok());

    // Test very long input
    let long_input = "a".repeat(10000);
    assert!(validator.detect_injection(&long_input).is_ok());

    // Test unicode characters
    assert!(validator.detect_injection("안녕하세요").is_ok());
}
```

### Coverage Testing Patterns

#### 1. Happy Path Tests
```rust
#[test]
fn test_happy_path() {
    let result = function_under_test(valid_input());
    assert!(result.is_ok());
}
```

#### 2. Error Path Tests
```rust
#[test]
fn test_error_conditions() {
    // Test invalid input
    assert!(function_under_test(invalid_input()).is_err());

    // Test boundary conditions
    assert!(function_under_test(boundary_case()).is_err());

    // Test null/empty inputs
    assert!(function_under_test("").is_err());
}
```

#### 3. Edge Case Tests
```rust
#[test]
fn test_edge_cases() {
    // Test maximum values
    assert!(function_under_test(i64::MAX).is_ok());

    // Test minimum values
    assert!(function_under_test(i64::MIN).is_ok());

    // Test zero
    assert!(function_under_test(0).is_ok());
}
```

#### 4. Concurrent Scenario Tests
```rust
#[tokio::test]
async fn test_concurrent_access() {
    let state = Arc::new(SharedState::new());
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let state = Arc::clone(&state);
            tokio::spawn(async move {
                state.update(i).await
            })
        })
        .collect();

    for handle in handles {
        assert!(handle.await.is_ok());
    }
}
```

## Best Practices

### DO

✅ **Prioritize security code coverage**
- Aim for 95%+ coverage on security-critical code
- Test all injection patterns and edge cases

✅ **Test error paths**
- Don't just test happy paths
- Test failure scenarios and error handling

✅ **Use coverage to find missing tests**
- Run coverage regularly during development
- Add tests for uncovered critical code

✅ **Write meaningful tests**
- Test behavior, not implementation
- Focus on edge cases and boundary conditions

✅ **Keep tests maintainable**
- Use helper functions for common setup
- Keep tests focused and readable

### DON'T

❌ **Don't chase 100% coverage blindly**
- Focus on meaningful coverage, not just numbers
- Some code (e.g., trivial getters) may not need tests

❌ **Don't test trivial code over critical code**
- Prioritize security and core logic
- Generated code can have lower coverage

❌ **Don't write tests just for coverage**
- Tests should verify behavior
- Coverage is a byproduct, not the goal

❌ **Don't ignore failing tests**
- Fix the code or fix the test
- Commented-out tests reduce effective coverage

## Coverage Enforcement

### Pre-commit Hook

```bash
#!/bin/sh
# .git/hooks/pre-commit

# Run coverage check before commit
if ! ./scripts/coverage.sh --check; then
    echo "Coverage check failed. Please add tests for uncovered code."
    echo "Run: ./scripts/coverage.sh --html --open to see coverage report."
    exit 1
fi
```

### CI Enforcement

The CI workflow checks coverage on every PR:
- Generates coverage report
- Posts summary on PR
- Fails if critical code is under-tested (configurable)

## Troubleshooting

### Coverage Not Accurate

**Problem**: Coverage report shows unexpected results

**Solutions**:
1. Clean coverage data:
   ```bash
   cargo llvm-cov clean
   ./scripts/coverage.sh --clean
   ```

2. Ensure all tests run:
   ```bash
   cargo test --all-features
   ```

3. Check for conditional compilation:
   - Some code may be excluded by feature flags
   - Use `--all-features` to include everything

### Tests Pass But Coverage Fails

**Problem**: Tests pass locally but coverage CI fails

**Solutions**:
1. Run coverage locally:
   ```bash
   ./scripts/coverage.sh --check
   ```

2. Check CI logs for specific failures

3. Ensure all features are enabled in CI

### Slow Coverage Generation

**Problem**: Coverage takes too long to generate

**Solutions**:
1. Run coverage on specific packages:
   ```bash
   cargo llvm-cov --package spreadsheet-mcp
   ```

2. Exclude slow tests:
   ```bash
   cargo llvm-cov --exclude-test recalc_docker
   ```

3. Use parallel test execution (default)

## Resources

- [cargo-llvm-cov Documentation](https://github.com/taiki-e/cargo-llvm-cov)
- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Codecov Documentation](https://docs.codecov.io/)
- [LLVM Coverage Mapping](https://llvm.org/docs/CoverageMappingFormat.html)

## Contributing

When adding new features:

1. Write tests alongside the code
2. Run coverage to verify test adequacy
3. Ensure critical code meets coverage targets
4. Include coverage report in PR description

## Maintenance

- Review coverage trends monthly
- Update coverage targets as needed
- Remove obsolete tests
- Refactor tests for maintainability

---

Last updated: 2026-01-20
