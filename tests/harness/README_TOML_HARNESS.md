# TOML Configuration Test Harness

## Overview

This directory contains a comprehensive Chicago-style TDD test harness for TOML configuration parsing and validation. The harness provides production-ready testing infrastructure that follows state-based testing principles.

## Files Created

### Core Harness
- **`toml_config_harness.rs`** - Main test harness implementation (1000+ lines)
  - Configuration structures
  - Test builders (ConfigBuilder)
  - Test harness (ConfigTestHarness)
  - Comprehensive assertions
  - Default value functions

### Test Suite
- **`../toml_config_tests.rs`** - Comprehensive test suite (500+ lines)
  - 50+ test cases
  - Valid configuration tests
  - Invalid configuration tests
  - Default value tests
  - Serialization round-trip tests
  - Builder pattern tests
  - Field validation tests
  - Inference configuration tests
  - Environment override tests
  - Property-based test patterns
  - Behavior verification tests
  - Edge cases and corner cases
  - Schema validation tests

### Test Fixtures

#### Valid Configurations (4 fixtures)
```
fixtures/toml/valid/
├── minimal.toml         - Minimal valid config (required fields only)
├── complete.toml        - Complete config (all fields)
├── with_defaults.toml   - Config with defaults applied
└── with_env_vars.toml   - Config with environment overrides
```

#### Invalid Configurations (6 fixtures)
```
fixtures/toml/invalid/
├── missing_required.toml      - Missing required fields
├── invalid_types.toml         - Wrong field types
├── out_of_range.toml          - Values out of range
├── invalid_enum.toml          - Invalid enum values
├── malformed_syntax.toml      - Malformed TOML syntax
└── conflicting_settings.toml  - Conflicting configuration
```

### Documentation
- **`../../docs/TDD_TOML_HARNESS.md`** - Comprehensive documentation (500+ lines)
  - Chicago-style TDD principles
  - Architecture overview
  - Test coverage matrix
  - Usage guide
  - Assertion reference
  - Best practices
  - Troubleshooting guide
  - Integration with CI/CD

### Example
- **`../../examples/toml_config_harness_demo.rs`** - Demo showing usage

## Quick Start

### Running Tests

```bash
# Run all TOML config tests
cargo test toml_config

# Run specific test
cargo test test_minimal_config_is_valid

# Run with output
cargo test toml_config -- --nocapture
```

### Using the Harness

```rust
use crate::harness::toml_config_harness::*;

#[test]
fn test_my_config() {
    // From fixture
    let harness = ConfigTestHarness::from_fixture("valid/minimal.toml");
    harness.assert_valid();
    harness.assert_project_name("test-project");

    // From string
    let toml = r#"
    [project]
    name = "test"
    version = "1.0.0"
    "#;
    let harness = ConfigTestHarness::from_str(toml);
    harness.assert_valid();

    // Using builder
    let config = ConfigBuilder::new()
        .project_name("my-project")
        .sparql_timeout(60)
        .enable_inference()
        .build();
}
```

## Architecture

### Configuration Structures

Complete representation of the TOML configuration schema:

- **TomlConfig** - Root configuration
- **ProjectConfig** - Project metadata
- **OntologyConfig** - Ontology settings
- **RdfConfig** - RDF store configuration
- **SparqlConfig** - SPARQL query settings
- **InferenceConfig** - Inference rules
- **GenerationConfig** - Code generation settings
- **ValidationConfig** - Validation rules
- **LifecycleConfig** - Lifecycle management
- **SecurityConfig** - Security settings
- **PerformanceConfig** - Performance tuning
- **LoggingConfig** - Logging configuration
- **TemplatesConfig** - Template settings
- **FeaturesConfig** - Feature flags

### Builder Pattern

Fluent API for constructing test configurations:

```rust
let config = ConfigBuilder::new()
    .project_name("my-project")
    .project_version("2.0.0")
    .ontology_source("custom.ttl")
    .base_uri("https://custom.dev/#")
    .sparql_timeout(120)
    .sparql_max_results(20000)
    .enable_inference()
    .enable_validation()
    .max_workers(16)
    .log_level("trace")
    .build();
```

### Test Harness

Comprehensive assertion API:

```rust
let harness = ConfigTestHarness::from_fixture("valid/complete.toml");

// Validation
harness.assert_valid();
harness.assert_invalid();
harness.assert_error_contains("message");

// Fields
harness.assert_project_name("name");
harness.assert_sparql_timeout(30);
harness.assert_log_level("info");

// Defaults
harness.assert_defaults_applied();
harness.assert_default_sparql_timeout();

// Features
harness.assert_inference_enabled();
harness.assert_generation_rule_count(10);

// Serialization
harness.assert_round_trip();

// Environment
harness.assert_has_env_override("dev", "key");
```

## Test Coverage

### Test Categories (50+ tests)

1. **Valid Configuration Tests** (8 tests)
   - Minimal configuration
   - Complete configuration
   - With defaults
   - With environment variables

2. **Invalid Configuration Tests** (6 tests)
   - Missing required fields
   - Invalid types
   - Out of range values
   - Invalid enum values
   - Malformed syntax
   - Conflicting settings

3. **Default Value Tests** (4 tests)
   - SPARQL defaults
   - Logging defaults
   - All defaults applied
   - Performance defaults

4. **Serialization Tests** (3 tests)
   - Minimal round-trip
   - Complete round-trip
   - Builder round-trip

5. **Builder Pattern Tests** (3 tests)
   - Minimal builder
   - Custom values builder
   - Features builder

6. **Field Validation Tests** (3 tests)
   - Project name
   - SPARQL timeout
   - Max workers

7. **Inference Tests** (3 tests)
   - Disabled by default
   - Can be enabled
   - Rule count validation

8. **Environment Override Tests** (3 tests)
   - Development overrides
   - CI overrides
   - Production overrides

9. **Assertion Helper Tests** (4 tests)
   - Valid helper
   - Invalid helper
   - Field equals helper
   - Defaults helper

10. **Property-Based Tests** (3 tests)
    - Any valid config parses
    - Round-trip property
    - Defaults always valid

11. **Behavior Tests** (5 tests)
    - Configuration loads
    - Validation errors caught
    - Precedence rules
    - Serialization preserves structure

12. **Edge Cases** (4 tests)
    - Empty arrays
    - Optional fields
    - Nested tables
    - Arrays of tables

13. **Schema Validation** (2 tests)
    - Required fields
    - Type matching

## Coverage Matrix

| Category | Test Count | Fixtures | Assertions |
|----------|-----------|----------|------------|
| Valid configs | 8 | 4 | 25+ |
| Invalid configs | 6 | 6 | 12+ |
| Defaults | 4 | 2 | 10+ |
| Serialization | 3 | 3 | 6+ |
| Builder | 3 | 0 | 8+ |
| Fields | 3 | 2 | 6+ |
| Inference | 3 | 2 | 6+ |
| Environment | 3 | 1 | 9+ |
| Helpers | 4 | 0 | 8+ |
| Properties | 3 | 0 | 6+ |
| Behavior | 5 | 3 | 10+ |
| Edge cases | 4 | 2 | 8+ |
| Schema | 2 | 0 | 4+ |
| **Total** | **51** | **10** | **118+** |

## Chicago-style TDD Principles

### What We Apply

1. **Real Objects**: All tests use actual `TomlConfig` structs, not mocks
2. **State Verification**: Tests verify configuration state, not interactions
3. **Behavior Testing**: Tests verify parsing and validation behavior
4. **No Mocks**: Direct testing against `toml::from_str()`
5. **Inside-Out**: Build from configuration objects outward

### Why It Works

- **Deterministic**: TOML parsing is pure, no side effects
- **State-based**: Configuration is fundamentally about state
- **Self-contained**: No external dependencies
- **Verifiable**: Easy to inspect final configuration state

## Key Features

### 80/20 Principle Applied

Fixtures cover the most important cases:
- ✅ Minimal valid (20% of fields, 80% of usage)
- ✅ Complete valid (100% of fields for reference)
- ✅ Defaults (common pattern)
- ✅ Environment overrides (production pattern)

### Comprehensive Assertions

118+ assertions covering:
- Validation (valid/invalid/error messages)
- Field values (project, SPARQL, logging, etc.)
- Default values (all configuration sections)
- Features (inference, generation, validation)
- Serialization (round-trip testing)
- Environment (override precedence)

### Property-Based Testing

Tests verify properties that always hold:
- Any valid config should parse
- Round-trip preserves equality
- Defaults are always valid
- Invalid values are always rejected

### Test Builders

Fluent API for constructing test data:
```rust
ConfigBuilder::new()
    .project_name("test")
    .enable_inference()
    .build()
```

## Dependencies

Added to `Cargo.toml`:
```toml
[dev-dependencies]
toml = "0.8"
num_cpus = "1.16"
```

## Integration

### With CI/CD

```bash
# In CI pipeline
cargo test toml_config --no-fail-fast
cargo tarpaulin --out Html --output-dir coverage
```

### With Development Workflow

```bash
# Watch mode during development
cargo watch -x "test toml_config"

# Before commit
cargo test toml_config && cargo fmt && cargo clippy
```

## Adding New Tests

1. **Add fixture** in `tests/fixtures/toml/valid/` or `invalid/`
2. **Write test** in `tests/toml_config_tests.rs`
3. **Add assertion** (if needed) to `ConfigTestHarness`
4. **Add builder method** (if needed) to `ConfigBuilder`
5. **Update documentation** in `docs/TDD_TOML_HARNESS.md`

## Best Practices

### ✅ Do

- Use fixtures for complex configurations
- Use builder for simple tests
- Test both positive and negative cases
- Verify round-trip serialization
- Use descriptive test names
- Group related tests

### ❌ Don't

- Use inline TOML for complex configs
- Test parsing implementation details
- Duplicate fixture content
- Skip negative test cases
- Use generic test names

## Documentation

See **`../../docs/TDD_TOML_HARNESS.md`** for:
- Detailed architecture
- Complete assertion reference
- Usage examples
- Troubleshooting guide
- Contributing guidelines

## Summary

This comprehensive Chicago-style TDD test harness provides:

- ✅ **1000+ lines** of production-ready test infrastructure
- ✅ **51 test cases** covering all scenarios
- ✅ **10 test fixtures** (4 valid, 6 invalid)
- ✅ **118+ assertions** for comprehensive verification
- ✅ **State-based testing** without mocks
- ✅ **Fluent test builders** for easy construction
- ✅ **Property-based patterns** for robustness
- ✅ **Complete documentation** with examples
- ✅ **Production-ready** code quality

**Use this harness to ensure robust TOML configuration handling with confidence!**
