//! Comprehensive TOML Configuration Tests using Chicago-style TDD
//!
//! These tests demonstrate the Chicago-style TDD approach:
//! - State-based verification (not mocks)
//! - Testing real configuration parsing
//! - Behavior verification through state inspection
//! - Property-based testing principles

mod harness;

use harness::toml_config_harness::*;

// ============================================================================
// Valid Configuration Tests
// ============================================================================

#[test]
fn test_minimal_config_is_valid() {
    let harness = ConfigTestHarness::from_fixture("valid/minimal.toml");
    harness.assert_valid();
}

#[test]
fn test_minimal_config_has_required_fields() {
    let harness = ConfigTestHarness::from_fixture("valid/minimal.toml");
    harness.assert_valid();
    harness.assert_project_name("test-project");
    harness.assert_project_version("0.1.0");
    harness.assert_ontology_source("ontology/test.ttl");
    harness.assert_base_uri("https://test.dev/domain#");
}

#[test]
fn test_complete_config_is_valid() {
    let harness = ConfigTestHarness::from_fixture("valid/complete.toml");
    harness.assert_valid();
}

#[test]
fn test_complete_config_has_all_fields() {
    let harness = ConfigTestHarness::from_fixture("valid/complete.toml");
    harness.assert_valid();

    // Project fields
    harness.assert_project_name("complete-test");
    harness.assert_project_version("1.0.0");

    // SPARQL configuration
    harness.assert_sparql_timeout(60);
    harness.assert_sparql_max_results(10000);

    // Performance
    harness.assert_max_workers(4);

    // Logging
    harness.assert_log_level("debug");

    // Inference
    harness.assert_inference_enabled();
    harness.assert_inference_rule_count(1);

    // Generation
    harness.assert_generation_rule_count(1);
}

#[test]
fn test_config_with_defaults() {
    let harness = ConfigTestHarness::from_fixture("valid/with_defaults.toml");
    harness.assert_valid();
    harness.assert_project_name("defaults-test");
}

#[test]
fn test_config_with_env_vars() {
    let harness = ConfigTestHarness::from_fixture("valid/with_env_vars.toml");
    harness.assert_valid();

    // Check environment overrides exist
    harness.assert_has_env_override("development", "logging.level");
    harness.assert_has_env_override("ci", "logging.format");
    harness.assert_has_env_override("production", "performance.max_workers");
}

// ============================================================================
// Invalid Configuration Tests
// ============================================================================

#[test]
fn test_missing_required_fields_is_invalid() {
    let harness = ConfigTestHarness::from_fixture("invalid/missing_required.toml");
    harness.assert_invalid();
}

#[test]
fn test_invalid_types_is_invalid() {
    let harness = ConfigTestHarness::from_fixture("invalid/invalid_types.toml");
    harness.assert_invalid();
}

#[test]
fn test_malformed_syntax_is_invalid() {
    let harness = ConfigTestHarness::from_fixture("invalid/malformed_syntax.toml");
    harness.assert_invalid();
}

// ============================================================================
// Default Value Tests
// ============================================================================

#[test]
fn test_sparql_defaults() {
    let toml = r#"
[project]
name = "test"
version = "0.1.0"

[ontology]
source = "test.ttl"
base_uri = "https://test.dev/#"
format = "turtle"

[rdf]
base_uri = "https://test.dev/"
default_format = "turtle"
"#;

    let harness = ConfigTestHarness::from_str(toml);
    harness.assert_valid();
    harness.assert_default_sparql_timeout();
}

#[test]
fn test_logging_defaults() {
    let config = ConfigBuilder::new().build();
    let toml_str = toml::to_string(&config).unwrap();
    let harness = ConfigTestHarness::from_str(toml_str);

    harness.assert_valid();
    harness.assert_default_log_level();
}

#[test]
fn test_all_defaults_applied() {
    let config = ConfigBuilder::new().build();
    let harness = ConfigTestHarness::from_str(toml::to_string(&config).unwrap());

    harness.assert_valid();
    harness.assert_defaults_applied();
}

// ============================================================================
// Serialization Round-trip Tests
// ============================================================================

#[test]
fn test_minimal_config_round_trip() {
    let harness = ConfigTestHarness::from_fixture("valid/minimal.toml");
    harness.assert_valid();
    harness.assert_round_trip();
}

#[test]
fn test_complete_config_round_trip() {
    let harness = ConfigTestHarness::from_fixture("valid/complete.toml");
    harness.assert_valid();
    harness.assert_round_trip();
}

#[test]
fn test_builder_config_round_trip() {
    let config = ConfigBuilder::new()
        .project_name("round-trip")
        .project_version("2.0.0")
        .sparql_timeout(45)
        .sparql_max_results(8000)
        .enable_inference()
        .enable_validation()
        .max_workers(6)
        .log_level("debug")
        .build();

    let serialized = toml::to_string(&config).unwrap();
    let harness = ConfigTestHarness::from_str(serialized);

    harness.assert_valid();
    harness.assert_round_trip();
}

// ============================================================================
// Builder Pattern Tests
// ============================================================================

#[test]
fn test_builder_minimal() {
    let config = ConfigBuilder::new().build();

    assert_eq!(config.project.name, "test-project");
    assert_eq!(config.project.version, "0.1.0");
    assert_eq!(config.ontology.format, "turtle");
}

#[test]
fn test_builder_with_custom_values() {
    let config = ConfigBuilder::new()
        .project_name("custom")
        .project_version("3.0.0")
        .ontology_source("custom.ttl")
        .base_uri("https://custom.dev/#")
        .sparql_timeout(120)
        .sparql_max_results(20000)
        .max_workers(16)
        .log_level("trace")
        .build();

    assert_eq!(config.project.name, "custom");
    assert_eq!(config.project.version, "3.0.0");
    assert_eq!(config.ontology.source, "custom.ttl");
    assert_eq!(config.sparql.timeout, 120);
    assert_eq!(config.sparql.max_results, 20000);
    assert_eq!(config.performance.max_workers, 16);
    assert_eq!(config.logging.level, "trace");
}

#[test]
fn test_builder_with_features() {
    let config = ConfigBuilder::new()
        .enable_inference()
        .enable_validation()
        .build();

    assert!(config.inference.enabled);
    assert!(config.validation.validate_syntax);
    assert!(config.validation.no_unsafe);
}

// ============================================================================
// Field Validation Tests
// ============================================================================

#[test]
fn test_project_name_validation() {
    let harness = ConfigTestHarness::from_fixture("valid/minimal.toml");
    harness.assert_valid();
    harness.assert_project_name("test-project");
}

#[test]
fn test_sparql_timeout_validation() {
    let config = ConfigBuilder::new()
        .sparql_timeout(90)
        .build();

    let harness = ConfigTestHarness::from_str(toml::to_string(&config).unwrap());
    harness.assert_valid();
    harness.assert_sparql_timeout(90);
}

#[test]
fn test_max_workers_validation() {
    let config = ConfigBuilder::new()
        .max_workers(12)
        .build();

    let harness = ConfigTestHarness::from_str(toml::to_string(&config).unwrap());
    harness.assert_valid();
    harness.assert_max_workers(12);
}

// ============================================================================
// Inference Configuration Tests
// ============================================================================

#[test]
fn test_inference_disabled_by_default() {
    let config = ConfigBuilder::new().build();
    let harness = ConfigTestHarness::from_str(toml::to_string(&config).unwrap());

    harness.assert_valid();
    harness.assert_inference_disabled();
}

#[test]
fn test_inference_can_be_enabled() {
    let config = ConfigBuilder::new()
        .enable_inference()
        .build();

    let harness = ConfigTestHarness::from_str(toml::to_string(&config).unwrap());
    harness.assert_valid();
    harness.assert_inference_enabled();
}

#[test]
fn test_complete_config_has_inference_rules() {
    let harness = ConfigTestHarness::from_fixture("valid/complete.toml");
    harness.assert_valid();
    harness.assert_inference_rule_count(1);
}

// ============================================================================
// Environment Override Tests
// ============================================================================

#[test]
fn test_development_env_overrides() {
    let harness = ConfigTestHarness::from_fixture("valid/with_env_vars.toml");
    harness.assert_valid();

    harness.assert_has_env_override("development", "logging.level");
    harness.assert_env_override_value(
        "development",
        "logging.level",
        &toml::Value::String("debug".to_string())
    );
}

#[test]
fn test_ci_env_overrides() {
    let harness = ConfigTestHarness::from_fixture("valid/with_env_vars.toml");
    harness.assert_valid();

    harness.assert_has_env_override("ci", "logging.format");
    harness.assert_env_override_value(
        "ci",
        "logging.format",
        &toml::Value::String("json".to_string())
    );
}

#[test]
fn test_production_env_overrides() {
    let harness = ConfigTestHarness::from_fixture("valid/with_env_vars.toml");
    harness.assert_valid();

    harness.assert_has_env_override("production", "performance.max_workers");
    harness.assert_env_override_value(
        "production",
        "performance.max_workers",
        &toml::Value::Integer(16)
    );
}

// ============================================================================
// Standalone Assertion Helper Tests
// ============================================================================

#[test]
fn test_assert_config_valid_helper() {
    let toml = r#"
[project]
name = "helper-test"
version = "1.0.0"

[ontology]
source = "test.ttl"
base_uri = "https://test.dev/#"
format = "turtle"

[rdf]
base_uri = "https://test.dev/"
default_format = "turtle"
"#;

    assert_config_valid(toml);
}

#[test]
#[should_panic]
fn test_assert_config_invalid_helper() {
    let toml = r#"
[project
name = "bad"
"#;

    assert_config_invalid(toml, "expected");
}

#[test]
fn test_assert_field_equals_helper() {
    let config = ConfigBuilder::new()
        .project_name("field-test")
        .build();

    assert_field_equals(&config, |c| &c.project.name, &"field-test".to_string());
}

#[test]
fn test_assert_defaults_applied_helper() {
    let config = ConfigBuilder::new().build();
    assert_defaults_applied(&config);
}

// ============================================================================
// Property-Based Test Patterns
// ============================================================================

#[test]
fn test_any_valid_config_parses() {
    // Test various valid configurations
    let configs = vec![
        ConfigBuilder::new().build(),
        ConfigBuilder::new().sparql_timeout(60).build(),
        ConfigBuilder::new().enable_inference().build(),
        ConfigBuilder::new().max_workers(8).build(),
        ConfigBuilder::new().log_level("debug").build(),
    ];

    for config in configs {
        let toml_str = toml::to_string(&config).unwrap();
        let harness = ConfigTestHarness::from_str(toml_str);
        harness.assert_valid();
    }
}

#[test]
fn test_round_trip_property() {
    // Property: Any valid config should survive serialize → deserialize
    let configs = vec![
        ConfigBuilder::new().build(),
        ConfigBuilder::new()
            .project_name("prop1")
            .sparql_timeout(45)
            .build(),
        ConfigBuilder::new()
            .enable_inference()
            .enable_validation()
            .build(),
    ];

    for config in configs {
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: TomlConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }
}

#[test]
fn test_defaults_always_valid() {
    // Property: Default configuration is always valid
    let config = ConfigBuilder::new().build();
    let toml_str = toml::to_string(&config).unwrap();
    let harness = ConfigTestHarness::from_str(toml_str);
    harness.assert_valid();
}

// ============================================================================
// Behavior Verification Tests
// ============================================================================

#[test]
fn test_configuration_loads_successfully() {
    // Behavior: Configuration should load from valid TOML
    let harness = ConfigTestHarness::from_fixture("valid/minimal.toml");
    harness.assert_valid();

    let config = harness.unwrap_config();
    assert!(!config.project.name.is_empty());
    assert!(!config.project.version.is_empty());
}

#[test]
fn test_validation_errors_caught() {
    // Behavior: Invalid TOML should produce error
    let harness = ConfigTestHarness::from_fixture("invalid/malformed_syntax.toml");
    harness.assert_invalid();
}

#[test]
fn test_precedence_file_over_defaults() {
    // Behavior: File values should override defaults
    let harness = ConfigTestHarness::from_fixture("valid/complete.toml");
    harness.assert_valid();

    let config = harness.unwrap_config();
    // These are explicitly set in complete.toml, not defaults
    assert_eq!(config.sparql.timeout, 60); // Not the default 30
    assert_eq!(config.logging.level, "debug"); // Not the default "info"
}

#[test]
fn test_serialization_preserves_structure() {
    // Behavior: Serialization should preserve structure
    let config = ConfigBuilder::new()
        .project_name("preserve")
        .sparql_timeout(77)
        .max_workers(9)
        .build();

    let serialized = toml::to_string(&config).unwrap();
    let deserialized: TomlConfig = toml::from_str(&serialized).unwrap();

    assert_eq!(config.project.name, deserialized.project.name);
    assert_eq!(config.sparql.timeout, deserialized.sparql.timeout);
    assert_eq!(config.performance.max_workers, deserialized.performance.max_workers);
}

// ============================================================================
// Edge Cases and Corner Cases
// ============================================================================

#[test]
fn test_empty_arrays_are_valid() {
    let config = ConfigBuilder::new().build();

    assert!(config.ontology.imports.is_empty());
    assert!(config.generation.protected_paths.is_empty());
    assert!(config.security.allowed_domains.is_empty());
}

#[test]
fn test_optional_fields_can_be_none() {
    let config = ConfigBuilder::new().build();

    assert!(config.project.description.is_none());
    assert!(config.project.authors.is_none());
    assert!(config.project.license.is_none());
}

#[test]
fn test_nested_tables_parse_correctly() {
    let harness = ConfigTestHarness::from_fixture("valid/complete.toml");
    harness.assert_valid();

    let config = harness.unwrap_config();
    assert!(!config.ontology.prefixes.is_empty());
    assert!(!config.rdf.prefixes.is_empty());
}

#[test]
fn test_arrays_of_tables_parse_correctly() {
    let harness = ConfigTestHarness::from_fixture("valid/complete.toml");
    harness.assert_valid();

    let config = harness.unwrap_config();
    assert!(!config.inference.rules.is_empty());
    assert!(!config.generation.rules.is_empty());
}

// ============================================================================
// Schema Validation Tests
// ============================================================================

#[test]
fn test_required_fields_must_be_present() {
    // Missing project.name
    let toml = r#"
[project]
version = "1.0.0"

[ontology]
source = "test.ttl"
base_uri = "https://test.dev/#"
format = "turtle"

[rdf]
base_uri = "https://test.dev/"
default_format = "turtle"
"#;

    let harness = ConfigTestHarness::from_str(toml);
    harness.assert_invalid();
}

#[test]
fn test_types_must_match_schema() {
    // Wrong type for timeout (string instead of number)
    let toml = r#"
[project]
name = "test"
version = "1.0.0"

[ontology]
source = "test.ttl"
base_uri = "https://test.dev/#"
format = "turtle"

[rdf]
base_uri = "https://test.dev/"
default_format = "turtle"

[sparql]
timeout = "not-a-number"
"#;

    let harness = ConfigTestHarness::from_str(toml);
    harness.assert_invalid();
}
