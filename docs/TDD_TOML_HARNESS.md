# Chicago-style TDD Test Harness for TOML Configuration

## Overview

This document describes the comprehensive Chicago-style TDD test harness for TOML configuration parsing and validation in the ggen-mcp project. The harness provides state-based testing infrastructure that verifies configuration behavior through real object state inspection rather than mocks.

## Chicago-style TDD Principles

### What is Chicago-style TDD?

Chicago-style TDD (also known as "Classical TDD" or "Detroit School") emphasizes:

1. **Real Objects**: Tests use actual production objects, not mocks or stubs
2. **State Verification**: Tests verify the final state of objects after operations
3. **Behavior Testing**: Tests verify observable behavior through state changes
4. **No Mocks**: Direct testing against real implementations
5. **Inside-Out Development**: Build from domain objects outward

### Contrast with London-style TDD

| Aspect | Chicago-style | London-style |
|--------|--------------|--------------|
| Dependencies | Use real objects | Use mocks/stubs |
| Verification | State-based | Interaction-based |
| Isolation | Test behavior | Test collaborations |
| Focus | End-to-end behavior | Component interactions |

### Why Chicago-style for Configuration?

Configuration testing is ideal for Chicago-style TDD because:

- **State-based**: Configuration is inherently about state (settings, values)
- **No side effects**: Parsing TOML has no external side effects
- **Deterministic**: Same input always produces same output
- **Self-contained**: Configuration parsing is independent
- **Verifiable**: Easy to verify final configuration state

## Architecture

### Directory Structure

```
tests/
├── harness/
│   ├── mod.rs
│   └── toml_config_harness.rs    # Main harness implementation
├── fixtures/
│   └── toml/
│       ├── valid/
│       │   ├── minimal.toml
│       │   ├── complete.toml
│       │   ├── with_defaults.toml
│       │   └── with_env_vars.toml
│       └── invalid/
│           ├── missing_required.toml
│           ├── invalid_types.toml
│           ├── out_of_range.toml
│           ├── invalid_enum.toml
│           ├── malformed_syntax.toml
│           └── conflicting_settings.toml
└── toml_config_tests.rs          # Comprehensive test suite
```

### Core Components

#### 1. Configuration Structures (`TomlConfig`)

Complete Rust structures representing the TOML configuration schema:

```rust
pub struct TomlConfig {
    pub project: ProjectConfig,
    pub ontology: OntologyConfig,
    pub rdf: RdfConfig,
    pub sparql: SparqlConfig,
    pub inference: InferenceConfig,
    pub generation: GenerationConfig,
    pub validation: ValidationConfig,
    pub lifecycle: LifecycleConfig,
    pub security: SecurityConfig,
    pub performance: PerformanceConfig,
    pub logging: LoggingConfig,
    pub templates: TemplatesConfig,
    pub env: HashMap<String, HashMap<String, toml::Value>>,
    pub features: FeaturesConfig,
}
```

#### 2. Test Builder (`ConfigBuilder`)

Fluent builder for constructing test configurations:

```rust
let config = ConfigBuilder::new()
    .project_name("my-project")
    .sparql_timeout(60)
    .enable_inference()
    .max_workers(8)
    .build();
```

#### 3. Test Harness (`ConfigTestHarness`)

Main testing interface with comprehensive assertions:

```rust
let harness = ConfigTestHarness::from_fixture("valid/minimal.toml");
harness.assert_valid();
harness.assert_project_name("test-project");
harness.assert_sparql_timeout(30);
```

## Test Coverage Matrix

### Valid Configurations (80/20 Principle)

| Test Case | Coverage | Purpose |
|-----------|----------|---------|
| **minimal.toml** | Required fields only | Verify minimum valid config |
| **complete.toml** | All possible fields | Verify comprehensive parsing |
| **with_defaults.toml** | Omitted optional fields | Verify default application |
| **with_env_vars.toml** | Environment overrides | Verify precedence rules |

### Invalid Configurations

| Test Case | Error Type | Purpose |
|-----------|------------|---------|
| **missing_required.toml** | Missing fields | Verify required field validation |
| **invalid_types.toml** | Type mismatches | Verify type checking |
| **out_of_range.toml** | Value constraints | Verify range validation |
| **invalid_enum.toml** | Invalid enum values | Verify enum validation |
| **malformed_syntax.toml** | TOML syntax errors | Verify syntax error handling |
| **conflicting_settings.toml** | Logic conflicts | Verify semantic validation |

### Test Categories

#### 1. Parsing Tests
- Valid TOML syntax parsing
- Invalid TOML syntax rejection
- Nested table parsing
- Array of tables parsing
- Type coercion

#### 2. Validation Tests
- Required field presence
- Type correctness
- Value range constraints
- Enum value validity
- Semantic consistency

#### 3. Default Value Tests
- Default application for optional fields
- Override of defaults with explicit values
- Nested default propagation
- Feature flag defaults

#### 4. Serialization Tests
- Round-trip (serialize → deserialize)
- Format preservation
- Structure preservation
- Value preservation

#### 5. Behavior Tests
- Configuration loads successfully
- Validation errors are caught
- Defaults applied correctly
- Precedence rules (file > env > default)
- Environment overrides work

#### 6. Property-Based Tests
- Any valid TOML should parse
- Round-trip equality
- Defaults always valid
- Invalid values always rejected

## Usage Guide

### Basic Testing

```rust
use crate::harness::toml_config_harness::*;

#[test]
fn test_my_config() {
    // From fixture
    let harness = ConfigTestHarness::from_fixture("valid/minimal.toml");
    harness.assert_valid();
    harness.assert_project_name("test-project");
}
```

### Using the Builder

```rust
#[test]
fn test_custom_config() {
    let config = ConfigBuilder::new()
        .project_name("custom")
        .project_version("2.0.0")
        .sparql_timeout(45)
        .enable_inference()
        .build();

    let toml_str = toml::to_string(&config).unwrap();
    let harness = ConfigTestHarness::from_str(toml_str);

    harness.assert_valid();
    harness.assert_sparql_timeout(45);
}
```

### Testing Validation

```rust
#[test]
fn test_invalid_config() {
    let toml = r#"
[project]
# Missing required 'name' field
version = "1.0.0"
"#;

    let harness = ConfigTestHarness::from_str(toml);
    harness.assert_invalid();
    harness.assert_error_contains("missing field");
}
```

### Testing Defaults

```rust
#[test]
fn test_defaults() {
    let config = ConfigBuilder::new().build();
    let harness = ConfigTestHarness::from_str(toml::to_string(&config).unwrap());

    harness.assert_valid();
    harness.assert_default_sparql_timeout();
    harness.assert_default_log_level();
}
```

### Testing Round-trips

```rust
#[test]
fn test_round_trip() {
    let harness = ConfigTestHarness::from_fixture("valid/complete.toml");
    harness.assert_valid();
    harness.assert_round_trip();
}
```

## Assertion Reference

### Validation Assertions

```rust
harness.assert_valid()                          // Config is valid
harness.assert_invalid()                        // Config is invalid
harness.assert_error_contains("expected text")  // Error contains text
```

### Field Assertions

```rust
harness.assert_project_name("name")             // Project name
harness.assert_project_version("1.0.0")         // Project version
harness.assert_ontology_source("path.ttl")      // Ontology source
harness.assert_base_uri("https://...")          // Base URI
harness.assert_sparql_timeout(30)               // SPARQL timeout
harness.assert_sparql_max_results(5000)         // SPARQL max results
harness.assert_log_level("info")                // Log level
harness.assert_max_workers(8)                   // Max workers
```

### Default Value Assertions

```rust
harness.assert_defaults_applied()               // All defaults correct
harness.assert_default_sparql_timeout()         // SPARQL timeout default
harness.assert_default_log_level()              // Log level default
```

### Feature Assertions

```rust
harness.assert_inference_enabled()              // Inference is on
harness.assert_inference_disabled()             // Inference is off
harness.assert_inference_rule_count(5)          // Rule count
harness.assert_generation_rule_count(10)        // Generation rule count
```

### Serialization Assertions

```rust
harness.assert_round_trip()                     // Serialize/deserialize works
```

### Environment Override Assertions

```rust
harness.assert_has_env_override("dev", "key")   // Override exists
harness.assert_env_override_value(              // Override value matches
    "production",
    "logging.level",
    &toml::Value::String("error".to_string())
)
```

### Standalone Helper Functions

```rust
assert_config_valid(toml_str)                   // Standalone valid check
assert_config_invalid(toml_str, "error text")   // Standalone invalid check
assert_field_equals(&config, |c| &c.name, &"val") // Field equality
assert_defaults_applied(&config)                // Defaults applied
```

## Adding New Tests

### 1. Add a Test Fixture

Create a new TOML file in the appropriate fixtures directory:

```bash
# Valid configuration
tests/fixtures/toml/valid/my_feature.toml

# Invalid configuration
tests/fixtures/toml/invalid/my_error_case.toml
```

### 2. Write Test Cases

```rust
#[test]
fn test_my_new_feature() {
    let harness = ConfigTestHarness::from_fixture("valid/my_feature.toml");
    harness.assert_valid();
    // Add feature-specific assertions
    harness.assert_project_name("expected");
}

#[test]
fn test_my_error_case() {
    let harness = ConfigTestHarness::from_fixture("invalid/my_error_case.toml");
    harness.assert_invalid();
    harness.assert_error_contains("expected error message");
}
```

### 3. Add Custom Assertions (if needed)

Extend `ConfigTestHarness` with new assertions:

```rust
impl ConfigTestHarness {
    pub fn assert_my_custom_field(&self, expected: &str) {
        let config = self.unwrap_config();
        assert_eq!(
            config.my_section.my_field,
            expected,
            "My field should be '{}'",
            expected
        );
    }
}
```

### 4. Add Builder Methods (if needed)

Extend `ConfigBuilder` with new builder methods:

```rust
impl ConfigBuilder {
    pub fn my_custom_field(mut self, value: impl Into<String>) -> Self {
        self.config.my_section.my_field = value.into();
        self
    }
}
```

## Best Practices

### 1. Test Naming

Use descriptive test names that explain what is being tested:

```rust
// Good
#[test]
fn test_minimal_config_is_valid() { ... }

#[test]
fn test_missing_required_fields_is_invalid() { ... }

// Bad
#[test]
fn test_config() { ... }

#[test]
fn test_error() { ... }
```

### 2. Test Organization

Group related tests together:

```rust
// ============================================================================
// Valid Configuration Tests
// ============================================================================

#[test]
fn test_minimal_config() { ... }

#[test]
fn test_complete_config() { ... }

// ============================================================================
// Invalid Configuration Tests
// ============================================================================

#[test]
fn test_missing_fields() { ... }
```

### 3. Use Fixtures for Complex Cases

For complex configurations, use fixtures rather than inline TOML:

```rust
// Good - fixture file
#[test]
fn test_complex_config() {
    let harness = ConfigTestHarness::from_fixture("valid/complex.toml");
    harness.assert_valid();
}

// Acceptable - simple inline
#[test]
fn test_simple_field() {
    let toml = r#"
[project]
name = "test"
version = "1.0.0"
"#;
    let harness = ConfigTestHarness::from_str(toml);
    harness.assert_valid();
}
```

### 4. Use Builder for Programmatic Tests

When testing behavior rather than parsing, use the builder:

```rust
#[test]
fn test_feature_interaction() {
    let config = ConfigBuilder::new()
        .enable_inference()
        .enable_validation()
        .build();

    // Test behavior based on state
    assert!(config.inference.enabled);
    assert!(config.validation.validate_syntax);
}
```

### 5. Test Both Positive and Negative Cases

Always test both valid and invalid scenarios:

```rust
#[test]
fn test_valid_timeout() {
    let config = ConfigBuilder::new().sparql_timeout(60).build();
    // Verify it works
}

#[test]
fn test_invalid_timeout() {
    let toml = r#"
[sparql]
timeout = -10
"#;
    let harness = ConfigTestHarness::from_str(toml);
    // Verify it's rejected
}
```

### 6. Property-Based Testing

Test properties that should always hold:

```rust
#[test]
fn test_round_trip_property() {
    // Property: Any valid config survives serialize → deserialize
    let configs = vec![
        ConfigBuilder::new().build(),
        ConfigBuilder::new().sparql_timeout(45).build(),
        // ... more variations
    ];

    for config in configs {
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: TomlConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }
}
```

## Integration with CI/CD

### Running Tests

```bash
# Run all TOML config tests
cargo test toml_config

# Run specific test
cargo test test_minimal_config_is_valid

# Run with verbose output
cargo test toml_config -- --nocapture

# Run and show ignored tests
cargo test toml_config -- --ignored
```

### Coverage

```bash
# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage
```

### Performance

```bash
# Run tests with timing
cargo test toml_config -- --show-output --test-threads=1
```

## Common Patterns

### Testing Environment Overrides

```rust
#[test]
fn test_environment_precedence() {
    let harness = ConfigTestHarness::from_fixture("valid/with_env_vars.toml");
    harness.assert_valid();

    // Development overrides
    harness.assert_has_env_override("development", "logging.level");
    harness.assert_env_override_value(
        "development",
        "logging.level",
        &toml::Value::String("debug".to_string())
    );

    // Production overrides
    harness.assert_has_env_override("production", "performance.max_workers");
}
```

### Testing Nested Structures

```rust
#[test]
fn test_nested_configuration() {
    let harness = ConfigTestHarness::from_fixture("valid/complete.toml");
    harness.assert_valid();

    let config = harness.unwrap_config();

    // Test nested prefixes
    assert!(config.ontology.prefixes.contains_key("mcp"));
    assert_eq!(
        config.ontology.prefixes.get("mcp").unwrap(),
        "https://ggen-mcp.dev/mcp#"
    );
}
```

### Testing Arrays of Tables

```rust
#[test]
fn test_inference_rules() {
    let harness = ConfigTestHarness::from_fixture("valid/complete.toml");
    harness.assert_valid();

    let config = harness.unwrap_config();

    // Test rule count
    assert!(!config.inference.rules.is_empty());

    // Test specific rule
    let rule = &config.inference.rules[0];
    assert_eq!(rule.name, "test-rule");
    assert_eq!(rule.description, "Test inference rule");
}
```

## Troubleshooting

### Test Fails to Parse Fixture

**Problem**: `ConfigTestHarness::from_fixture()` panics with "file not found"

**Solution**: Ensure you're using the correct relative path from `tests/fixtures/toml/`:

```rust
// Correct
ConfigTestHarness::from_fixture("valid/minimal.toml")

// Incorrect
ConfigTestHarness::from_fixture("tests/fixtures/toml/valid/minimal.toml")
```

### Assertion Fails with Unexpected Value

**Problem**: `assert_project_name()` fails but the TOML looks correct

**Solution**: Check for:
- Whitespace in TOML strings
- Case sensitivity
- Default value application

Use `harness.unwrap_config()` to inspect the actual parsed value:

```rust
let config = harness.unwrap_config();
println!("Actual name: {:?}", config.project.name);
```

### Round-trip Test Fails

**Problem**: `assert_round_trip()` fails with "configurations don't match"

**Solution**: This usually indicates:
- Missing `#[serde(default)]` on optional fields
- Custom default functions not matching
- Ordering issues in collections

Check that your struct definitions have proper defaults:

```rust
#[derive(Deserialize, Serialize, PartialEq)]
pub struct MyConfig {
    #[serde(default)]
    pub optional_field: Option<String>,

    #[serde(default = "default_value")]
    pub field_with_default: String,
}
```

## Future Enhancements

### Planned Features

1. **Property-based testing integration**: Use `proptest` to generate random valid configs
2. **Fuzzing**: Fuzz test TOML parsing with invalid inputs
3. **Snapshot testing**: Capture and compare serialized configurations
4. **Migration testing**: Test configuration upgrades across versions
5. **Performance benchmarks**: Measure parsing performance

### Contributing

To add new test capabilities:

1. Add test fixtures in `tests/fixtures/toml/`
2. Add assertions to `ConfigTestHarness`
3. Add builder methods to `ConfigBuilder`
4. Add comprehensive test cases to `toml_config_tests.rs`
5. Update this documentation

## References

- [Chicago vs London TDD](https://medium.com/@adrianbooth/test-driven-development-wars-detroit-vs-london-classicist-vs-mockist-9956c78ae95f)
- [TOML Specification](https://toml.io/en/)
- [Serde TOML](https://docs.rs/toml/latest/toml/)
- [State-based Testing](https://martinfowler.com/articles/mocksArentStubs.html)

## Summary

This Chicago-style TDD test harness provides:

- **Comprehensive coverage** of TOML configuration parsing
- **State-based verification** without mocks
- **Fluent test builders** for easy test construction
- **Rich assertions** for common test patterns
- **Property-based testing** support
- **Production-ready** test infrastructure

Use this harness to ensure robust configuration handling with confidence that your TOML parsing and validation work correctly in all scenarios.
