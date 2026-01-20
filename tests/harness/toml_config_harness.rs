//! Chicago-style TDD Test Harness for TOML Configuration Parsing and Validation
//!
//! This harness provides comprehensive testing infrastructure for TOML configuration
//! parsing and validation using Chicago-style TDD (state-based testing with real objects).
//!
//! # Chicago-style TDD Principles Applied
//!
//! 1. **Real Objects**: We use actual configuration structs, not mocks
//! 2. **State Verification**: Tests verify the final state of configuration objects
//! 3. **Behavior Testing**: Tests verify the behavior of parsing, validation, and defaults
//! 4. **No Mocks**: Direct testing against real TOML parsing and validation logic
//!
//! # Usage
//!
//! ```rust
//! use crate::harness::toml_config_harness::*;
//!
//! #[test]
//! fn test_minimal_config() {
//!     let harness = ConfigTestHarness::from_fixture("valid/minimal.toml");
//!     harness.assert_valid();
//!     harness.assert_has_project_name("test-project");
//! }
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ============================================================================
// Configuration Structures
// ============================================================================

/// Main TOML configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TomlConfig {
    pub project: ProjectConfig,
    pub ontology: OntologyConfig,
    pub rdf: RdfConfig,
    #[serde(default)]
    pub sparql: SparqlConfig,
    #[serde(default)]
    pub inference: InferenceConfig,
    #[serde(default)]
    pub generation: GenerationConfig,
    #[serde(default)]
    pub validation: ValidationConfig,
    #[serde(default)]
    pub lifecycle: LifecycleConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub performance: PerformanceConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub templates: TemplatesConfig,
    #[serde(default)]
    pub env: HashMap<String, HashMap<String, toml::Value>>,
    #[serde(default)]
    pub features: FeaturesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectConfig {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub authors: Option<Vec<String>>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub repository: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OntologyConfig {
    pub source: String,
    pub base_uri: String,
    pub format: String,
    #[serde(default)]
    pub imports: Vec<String>,
    #[serde(default)]
    pub prefixes: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RdfConfig {
    pub base_uri: String,
    pub default_format: String,
    #[serde(default)]
    pub cache_queries: bool,
    #[serde(default = "default_rdf_store_path")]
    pub store_path: String,
    #[serde(default)]
    pub prefixes: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SparqlConfig {
    #[serde(default = "default_sparql_timeout")]
    pub timeout: u32,
    #[serde(default = "default_sparql_max_results")]
    pub max_results: usize,
    #[serde(default)]
    pub cache_enabled: bool,
    #[serde(default = "default_sparql_cache_ttl")]
    pub cache_ttl: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InferenceConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub materialize_results: bool,
    #[serde(default)]
    pub cache_intermediate: bool,
    #[serde(default)]
    pub fail_on_error: bool,
    #[serde(default)]
    pub log_statistics: bool,
    #[serde(default)]
    pub rules: Vec<InferenceRule>,
    #[serde(default)]
    pub rule_order: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InferenceRule {
    pub name: String,
    pub description: String,
    #[serde(flatten)]
    pub query: InferenceQuery,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub priority: u32,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum InferenceQuery {
    Inline { construct: String },
    File { query_file: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenerationConfig {
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
    #[serde(default)]
    pub require_audit_trail: bool,
    #[serde(default)]
    pub protected_paths: Vec<String>,
    #[serde(default)]
    pub regenerate_paths: Vec<String>,
    #[serde(default)]
    pub generated_header: Option<String>,
    #[serde(default)]
    pub rules: Vec<GenerationRule>,
    #[serde(default)]
    pub poka_yoke: PokaYokeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenerationRule {
    pub name: String,
    pub description: String,
    pub query: QuerySpec,
    pub template: TemplateSpec,
    pub output_file: String,
    #[serde(default = "default_generation_mode")]
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum QuerySpec {
    File { file: String },
    Inline { query: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum TemplateSpec {
    File { file: String },
    Inline { template: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PokaYokeConfig {
    #[serde(default)]
    pub warning_headers: bool,
    #[serde(default)]
    pub gitignore_generated: bool,
    #[serde(default)]
    pub gitattributes_generated: bool,
    #[serde(default)]
    pub validate_imports: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationConfig {
    #[serde(default)]
    pub validate_syntax: bool,
    #[serde(default)]
    pub no_unsafe: bool,
    #[serde(default)]
    pub require_doc_comments: bool,
    #[serde(default = "default_max_line_length")]
    pub max_line_length: usize,
    #[serde(default)]
    pub shacl: ShaclValidationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ShaclValidationConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub shapes_file: Option<String>,
    #[serde(default)]
    pub fail_on_violation: bool,
    #[serde(default)]
    pub fail_on_warning: bool,
    #[serde(default)]
    pub fail_on_info: bool,
    #[serde(default = "default_shacl_report_format")]
    pub report_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LifecycleConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub config_file: Option<String>,
    #[serde(default)]
    pub cache_directory: Option<String>,
    #[serde(default)]
    pub state_file: Option<String>,
    #[serde(default)]
    pub phases: HashMap<String, PhaseConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PhaseConfig {
    Simple(Vec<String>),
    Complex {
        scripts: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecurityConfig {
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,
    #[serde(default = "default_true")]
    pub validate_ssl: bool,
    #[serde(default = "default_true")]
    pub path_traversal_protection: bool,
    #[serde(default = "default_true")]
    pub shell_injection_protection: bool,
    #[serde(default = "default_true")]
    pub template_sandboxing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PerformanceConfig {
    #[serde(default = "default_true")]
    pub parallel_execution: bool,
    #[serde(default = "default_max_workers")]
    pub max_workers: usize,
    #[serde(default = "default_true")]
    pub cache_templates: bool,
    #[serde(default = "default_true")]
    pub incremental_build: bool,
    #[serde(default = "default_memory_limit")]
    pub memory_limit_mb: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_format")]
    pub format: String,
    #[serde(default = "default_log_output")]
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplatesConfig {
    #[serde(default = "default_templates_dir")]
    pub directory: String,
    #[serde(default = "default_output_dir")]
    pub output_directory: String,
    #[serde(default)]
    pub backup_enabled: bool,
    #[serde(default = "default_true")]
    pub idempotent: bool,
    #[serde(default)]
    pub rust: RustTemplateConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RustTemplateConfig {
    #[serde(default = "default_rust_style")]
    pub style: String,
    #[serde(default = "default_error_handling")]
    pub error_handling: String,
    #[serde(default = "default_logging_lib")]
    pub logging: String,
    #[serde(default = "default_async_runtime")]
    pub async_runtime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FeaturesConfig {
    #[serde(default)]
    pub sparql_queries: bool,
    #[serde(default)]
    pub inference_rules: bool,
    #[serde(default)]
    pub lifecycle_management: bool,
    #[serde(default)]
    pub template_validation: bool,
    #[serde(default)]
    pub audit_trails: bool,
    #[serde(default)]
    pub deterministic_outputs: bool,
}

// ============================================================================
// Default Functions
// ============================================================================

fn default_rdf_store_path() -> String {
    ".ggen/rdf-store".to_string()
}

fn default_sparql_timeout() -> u32 {
    30
}

fn default_sparql_max_results() -> usize {
    5000
}

fn default_sparql_cache_ttl() -> u32 {
    3600
}

fn default_output_dir() -> String {
    ".".to_string()
}

fn default_generation_mode() -> String {
    "Overwrite".to_string()
}

fn default_max_line_length() -> usize {
    120
}

fn default_shacl_report_format() -> String {
    "text".to_string()
}

fn default_max_file_size() -> u64 {
    10485760 // 10MB
}

fn default_true() -> bool {
    true
}

fn default_max_workers() -> usize {
    num_cpus::get()
}

fn default_memory_limit() -> usize {
    512
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "pretty".to_string()
}

fn default_log_output() -> String {
    "stderr".to_string()
}

fn default_templates_dir() -> String {
    "templates".to_string()
}

fn default_rust_style() -> String {
    "core-team".to_string()
}

fn default_error_handling() -> String {
    "thiserror".to_string()
}

fn default_logging_lib() -> String {
    "tracing".to_string()
}

fn default_async_runtime() -> String {
    "tokio".to_string()
}

// ============================================================================
// Defaults Implementations
// ============================================================================

impl Default for SparqlConfig {
    fn default() -> Self {
        Self {
            timeout: default_sparql_timeout(),
            max_results: default_sparql_max_results(),
            cache_enabled: false,
            cache_ttl: default_sparql_cache_ttl(),
        }
    }
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            materialize_results: false,
            cache_intermediate: false,
            fail_on_error: false,
            log_statistics: false,
            rules: Vec::new(),
            rule_order: Vec::new(),
        }
    }
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            output_dir: default_output_dir(),
            require_audit_trail: false,
            protected_paths: Vec::new(),
            regenerate_paths: Vec::new(),
            generated_header: None,
            rules: Vec::new(),
            poka_yoke: PokaYokeConfig::default(),
        }
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            validate_syntax: false,
            no_unsafe: false,
            require_doc_comments: false,
            max_line_length: default_max_line_length(),
            shacl: ShaclValidationConfig::default(),
        }
    }
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            config_file: None,
            cache_directory: None,
            state_file: None,
            phases: HashMap::new(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allowed_domains: Vec::new(),
            max_file_size: default_max_file_size(),
            validate_ssl: true,
            path_traversal_protection: true,
            shell_injection_protection: true,
            template_sandboxing: true,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            parallel_execution: true,
            max_workers: default_max_workers(),
            cache_templates: true,
            incremental_build: true,
            memory_limit_mb: default_memory_limit(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            output: default_log_output(),
        }
    }
}

impl Default for TemplatesConfig {
    fn default() -> Self {
        Self {
            directory: default_templates_dir(),
            output_directory: default_output_dir(),
            backup_enabled: false,
            idempotent: true,
            rust: RustTemplateConfig::default(),
        }
    }
}

// ============================================================================
// Config Builder (Test Builder Pattern)
// ============================================================================

/// Builder for constructing test configurations
pub struct ConfigBuilder {
    config: TomlConfig,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: TomlConfig {
                project: ProjectConfig {
                    name: "test-project".to_string(),
                    version: "0.1.0".to_string(),
                    description: None,
                    authors: None,
                    license: None,
                    repository: None,
                },
                ontology: OntologyConfig {
                    source: "ontology/test.ttl".to_string(),
                    base_uri: "https://test.dev/domain#".to_string(),
                    format: "turtle".to_string(),
                    imports: Vec::new(),
                    prefixes: HashMap::new(),
                },
                rdf: RdfConfig {
                    base_uri: "https://test.dev/".to_string(),
                    default_format: "turtle".to_string(),
                    cache_queries: false,
                    store_path: default_rdf_store_path(),
                    prefixes: HashMap::new(),
                },
                sparql: SparqlConfig::default(),
                inference: InferenceConfig::default(),
                generation: GenerationConfig::default(),
                validation: ValidationConfig::default(),
                lifecycle: LifecycleConfig::default(),
                security: SecurityConfig::default(),
                performance: PerformanceConfig::default(),
                logging: LoggingConfig::default(),
                templates: TemplatesConfig::default(),
                env: HashMap::new(),
                features: FeaturesConfig::default(),
            },
        }
    }

    pub fn project_name(mut self, name: impl Into<String>) -> Self {
        self.config.project.name = name.into();
        self
    }

    pub fn project_version(mut self, version: impl Into<String>) -> Self {
        self.config.project.version = version.into();
        self
    }

    pub fn ontology_source(mut self, source: impl Into<String>) -> Self {
        self.config.ontology.source = source.into();
        self
    }

    pub fn base_uri(mut self, uri: impl Into<String>) -> Self {
        let uri = uri.into();
        self.config.ontology.base_uri = uri.clone();
        self.config.rdf.base_uri = uri;
        self
    }

    pub fn sparql_timeout(mut self, timeout: u32) -> Self {
        self.config.sparql.timeout = timeout;
        self
    }

    pub fn sparql_max_results(mut self, max: usize) -> Self {
        self.config.sparql.max_results = max;
        self
    }

    pub fn enable_inference(mut self) -> Self {
        self.config.inference.enabled = true;
        self
    }

    pub fn enable_validation(mut self) -> Self {
        self.config.validation.validate_syntax = true;
        self.config.validation.no_unsafe = true;
        self
    }

    pub fn max_workers(mut self, workers: usize) -> Self {
        self.config.performance.max_workers = workers;
        self
    }

    pub fn log_level(mut self, level: impl Into<String>) -> Self {
        self.config.logging.level = level.into();
        self
    }

    pub fn build(self) -> TomlConfig {
        self.config
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Config Test Harness
// ============================================================================

/// Main test harness for TOML configuration testing
pub struct ConfigTestHarness {
    config: Result<TomlConfig>,
    raw_toml: String,
}

impl ConfigTestHarness {
    /// Create harness from TOML string
    pub fn from_str(toml: impl Into<String>) -> Self {
        let raw_toml = toml.into();
        let config = toml::from_str(&raw_toml).context("Failed to parse TOML");
        Self { config, raw_toml }
    }

    /// Create harness from file path
    pub fn from_file(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        let raw_toml = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))
            .unwrap();
        let config = toml::from_str(&raw_toml)
            .with_context(|| format!("Failed to parse TOML file: {}", path.display()));
        Self { config, raw_toml }
    }

    /// Create harness from test fixture
    pub fn from_fixture(fixture_name: impl AsRef<str>) -> Self {
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/toml")
            .join(fixture_name.as_ref());
        Self::from_file(fixture_path)
    }

    /// Get the configuration if valid
    pub fn config(&self) -> &Result<TomlConfig> {
        &self.config
    }

    /// Get the configuration, panicking if invalid
    pub fn unwrap_config(&self) -> &TomlConfig {
        self.config.as_ref().expect("Configuration should be valid")
    }

    /// Get the raw TOML string
    pub fn raw_toml(&self) -> &str {
        &self.raw_toml
    }

    // ========================================================================
    // Validation Assertions
    // ========================================================================

    /// Assert that the configuration is valid
    pub fn assert_valid(&self) {
        assert!(
            self.config.is_ok(),
            "Configuration should be valid, but got error: {:?}",
            self.config.as_ref().err()
        );
    }

    /// Assert that the configuration is invalid
    pub fn assert_invalid(&self) {
        assert!(
            self.config.is_err(),
            "Configuration should be invalid, but it parsed successfully"
        );
    }

    /// Assert that the error message contains a specific string
    pub fn assert_error_contains(&self, expected: &str) {
        match &self.config {
            Ok(_) => panic!("Expected error containing '{}', but config is valid", expected),
            Err(e) => {
                let error_msg = format!("{:?}", e);
                assert!(
                    error_msg.contains(expected),
                    "Error message should contain '{}', but got: {}",
                    expected,
                    error_msg
                );
            }
        }
    }

    // ========================================================================
    // Field Assertions
    // ========================================================================

    /// Assert project name
    pub fn assert_project_name(&self, expected: &str) {
        let config = self.unwrap_config();
        assert_eq!(
            config.project.name, expected,
            "Project name should be '{}'",
            expected
        );
    }

    /// Assert project version
    pub fn assert_project_version(&self, expected: &str) {
        let config = self.unwrap_config();
        assert_eq!(
            config.project.version, expected,
            "Project version should be '{}'",
            expected
        );
    }

    /// Assert ontology source
    pub fn assert_ontology_source(&self, expected: &str) {
        let config = self.unwrap_config();
        assert_eq!(
            config.ontology.source, expected,
            "Ontology source should be '{}'",
            expected
        );
    }

    /// Assert base URI
    pub fn assert_base_uri(&self, expected: &str) {
        let config = self.unwrap_config();
        assert_eq!(
            config.ontology.base_uri, expected,
            "Base URI should be '{}'",
            expected
        );
    }

    /// Assert SPARQL timeout
    pub fn assert_sparql_timeout(&self, expected: u32) {
        let config = self.unwrap_config();
        assert_eq!(
            config.sparql.timeout, expected,
            "SPARQL timeout should be {}",
            expected
        );
    }

    /// Assert SPARQL max results
    pub fn assert_sparql_max_results(&self, expected: usize) {
        let config = self.unwrap_config();
        assert_eq!(
            config.sparql.max_results, expected,
            "SPARQL max results should be {}",
            expected
        );
    }

    /// Assert log level
    pub fn assert_log_level(&self, expected: &str) {
        let config = self.unwrap_config();
        assert_eq!(
            config.logging.level, expected,
            "Log level should be '{}'",
            expected
        );
    }

    /// Assert max workers
    pub fn assert_max_workers(&self, expected: usize) {
        let config = self.unwrap_config();
        assert_eq!(
            config.performance.max_workers, expected,
            "Max workers should be {}",
            expected
        );
    }

    // ========================================================================
    // Default Value Assertions
    // ========================================================================

    /// Assert that defaults are applied correctly
    pub fn assert_defaults_applied(&self) {
        let config = self.unwrap_config();

        // SPARQL defaults
        assert_eq!(config.sparql.timeout, default_sparql_timeout());
        assert_eq!(config.sparql.max_results, default_sparql_max_results());
        assert_eq!(config.sparql.cache_ttl, default_sparql_cache_ttl());

        // Logging defaults
        assert_eq!(config.logging.level, default_log_level());
        assert_eq!(config.logging.format, default_log_format());
        assert_eq!(config.logging.output, default_log_output());

        // Performance defaults
        assert_eq!(config.performance.memory_limit_mb, default_memory_limit());

        // Security defaults
        assert_eq!(config.security.max_file_size, default_max_file_size());
        assert!(config.security.validate_ssl);
        assert!(config.security.path_traversal_protection);
    }

    /// Assert specific default value
    pub fn assert_default_sparql_timeout(&self) {
        let config = self.unwrap_config();
        assert_eq!(config.sparql.timeout, default_sparql_timeout());
    }

    pub fn assert_default_log_level(&self) {
        let config = self.unwrap_config();
        assert_eq!(config.logging.level, default_log_level());
    }

    // ========================================================================
    // Feature Assertions
    // ========================================================================

    /// Assert inference is enabled
    pub fn assert_inference_enabled(&self) {
        let config = self.unwrap_config();
        assert!(
            config.inference.enabled,
            "Inference should be enabled"
        );
    }

    /// Assert inference is disabled
    pub fn assert_inference_disabled(&self) {
        let config = self.unwrap_config();
        assert!(
            !config.inference.enabled,
            "Inference should be disabled"
        );
    }

    /// Assert number of inference rules
    pub fn assert_inference_rule_count(&self, expected: usize) {
        let config = self.unwrap_config();
        assert_eq!(
            config.inference.rules.len(),
            expected,
            "Should have {} inference rules",
            expected
        );
    }

    /// Assert number of generation rules
    pub fn assert_generation_rule_count(&self, expected: usize) {
        let config = self.unwrap_config();
        assert_eq!(
            config.generation.rules.len(),
            expected,
            "Should have {} generation rules",
            expected
        );
    }

    // ========================================================================
    // Serialization Assertions
    // ========================================================================

    /// Assert round-trip serialization (serialize → deserialize → equals)
    pub fn assert_round_trip(&self) {
        let config = self.unwrap_config();
        let serialized = toml::to_string(&config).expect("Should serialize to TOML");
        let deserialized: TomlConfig = toml::from_str(&serialized)
            .expect("Should deserialize back from TOML");
        assert_eq!(
            config, &deserialized,
            "Configuration should round-trip correctly"
        );
    }

    // ========================================================================
    // Environment Override Assertions
    // ========================================================================

    /// Assert environment override exists
    pub fn assert_has_env_override(&self, env: &str, key: &str) {
        let config = self.unwrap_config();
        assert!(
            config.env.contains_key(env),
            "Should have environment '{}'",
            env
        );
        assert!(
            config.env[env].contains_key(key),
            "Environment '{}' should have key '{}'",
            env,
            key
        );
    }

    /// Assert environment override value
    pub fn assert_env_override_value(&self, env: &str, key: &str, expected: &toml::Value) {
        let config = self.unwrap_config();
        let actual = &config.env[env][key];
        assert_eq!(
            actual, expected,
            "Environment '{}' key '{}' should have value {:?}",
            env, key, expected
        );
    }
}

// ============================================================================
// Standalone Assertion Helpers
// ============================================================================

/// Assert that a TOML string is valid
pub fn assert_config_valid(toml_str: &str) {
    let harness = ConfigTestHarness::from_str(toml_str);
    harness.assert_valid();
}

/// Assert that a TOML string is invalid with a specific error
pub fn assert_config_invalid(toml_str: &str, error_contains: &str) {
    let harness = ConfigTestHarness::from_str(toml_str);
    harness.assert_invalid();
    harness.assert_error_contains(error_contains);
}

/// Assert that a field equals a value
pub fn assert_field_equals<F, V>(config: &TomlConfig, field_accessor: F, expected: V)
where
    F: Fn(&TomlConfig) -> V,
    V: PartialEq + std::fmt::Debug,
{
    let actual = field_accessor(config);
    assert_eq!(actual, expected, "Field should equal expected value");
}

/// Assert that defaults are applied
pub fn assert_defaults_applied(config: &TomlConfig) {
    assert_eq!(config.sparql.timeout, default_sparql_timeout());
    assert_eq!(config.logging.level, default_log_level());
}

// ============================================================================
// Module Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_minimal() {
        let config = ConfigBuilder::new()
            .project_name("test")
            .project_version("1.0.0")
            .build();

        assert_eq!(config.project.name, "test");
        assert_eq!(config.project.version, "1.0.0");
    }

    #[test]
    fn test_builder_with_sparql() {
        let config = ConfigBuilder::new()
            .sparql_timeout(60)
            .sparql_max_results(10000)
            .build();

        assert_eq!(config.sparql.timeout, 60);
        assert_eq!(config.sparql.max_results, 10000);
    }

    #[test]
    fn test_from_str_minimal() {
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
        harness.assert_project_name("test");
    }

    #[test]
    fn test_defaults() {
        let config = ConfigBuilder::new().build();
        let harness = ConfigTestHarness::from_str(toml::to_string(&config).unwrap());
        harness.assert_valid();
        harness.assert_default_sparql_timeout();
        harness.assert_default_log_level();
    }

    #[test]
    fn test_invalid_toml() {
        let toml = r#"
[project
name = "test"
"#;
        let harness = ConfigTestHarness::from_str(toml);
        harness.assert_invalid();
    }

    #[test]
    fn test_round_trip() {
        let config = ConfigBuilder::new()
            .project_name("round-trip-test")
            .sparql_timeout(45)
            .build();

        let serialized = toml::to_string(&config).unwrap();
        let harness = ConfigTestHarness::from_str(serialized);
        harness.assert_valid();
        harness.assert_round_trip();
    }
}
