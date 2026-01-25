//! Comprehensive Chicago-style TDD Test Harness for Tera Template Rendering
//!
//! This harness provides:
//! - Template rendering from strings and files
//! - Template syntax validation
//! - Context population and verification
//! - Generated code validation
//! - Golden file (snapshot) testing
//! - Code quality assertions
//! - Template variable usage verification
//!
//! Chicago-style TDD principles:
//! - Test actual behavior, not implementation
//! - Verify side effects and state changes
//! - Test integration with real dependencies
//! - Focus on observable outcomes

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tera::{Context as TeraContext, Tera};

// ============================================================================
// MAIN TEST HARNESS
// ============================================================================

/// Main test harness for Tera template rendering and validation
#[derive(Debug)]
pub struct TemplateTestHarness {
    /// Tera template engine instance
    tera: Tera,

    /// Base directory for templates
    template_dir: PathBuf,

    /// Base directory for fixtures
    fixture_dir: PathBuf,

    /// Configuration options
    config: HarnessConfig,

    /// Cache of rendered outputs for testing
    render_cache: HashMap<String, String>,
}

/// Configuration for the test harness
#[derive(Debug, Clone)]
pub struct HarnessConfig {
    /// Whether to validate syntax of rendered output
    pub validate_syntax: bool,

    /// Whether to check for security issues
    pub security_checks: bool,

    /// Whether to verify all variables are used
    pub check_variable_usage: bool,

    /// Whether to update golden files on mismatch
    pub update_golden_files: bool,

    /// Whether to compile rendered Rust code
    pub compile_check: bool,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            validate_syntax: true,
            security_checks: true,
            check_variable_usage: true,
            update_golden_files: false,
            compile_check: false, // Can be expensive
        }
    }
}

impl TemplateTestHarness {
    /// Creates a new test harness instance
    ///
    /// # Arguments
    /// * `template_dir` - Directory containing Tera templates
    /// * `fixture_dir` - Directory containing test fixtures
    pub fn new<P: AsRef<Path>>(template_dir: P, fixture_dir: P) -> Result<Self> {
        let template_dir = template_dir.as_ref().to_path_buf();
        let fixture_dir = fixture_dir.as_ref().to_path_buf();

        // Initialize Tera with all templates
        let template_pattern = template_dir.join("**/*.tera");
        let mut tera = Tera::new(
            template_pattern
                .to_str()
                .ok_or_else(|| anyhow!("Invalid template path"))?,
        )
        .context("Failed to initialize Tera")?;

        // Configure Tera settings
        tera.autoescape_on(vec![]);

        Ok(Self {
            tera,
            template_dir,
            fixture_dir,
            config: HarnessConfig::default(),
            render_cache: HashMap::new(),
        })
    }

    /// Creates a harness with custom configuration
    pub fn with_config<P: AsRef<Path>>(
        template_dir: P,
        fixture_dir: P,
        config: HarnessConfig,
    ) -> Result<Self> {
        let mut harness = Self::new(template_dir, fixture_dir)?;
        harness.config = config;
        Ok(harness)
    }

    // ========================================================================
    // TEMPLATE RENDERING
    // ========================================================================

    /// Renders a template from a string
    ///
    /// # Arguments
    /// * `template_name` - Name to give the template
    /// * `template_str` - Template content as string
    /// * `context` - Context for rendering
    pub fn render_from_string(
        &mut self,
        template_name: &str,
        template_str: &str,
        context: &TeraContext,
    ) -> Result<String> {
        self.tera
            .add_raw_template(template_name, template_str)
            .context("Failed to add template")?;

        let output = self
            .tera
            .render(template_name, context)
            .context("Failed to render template")?;

        self.render_cache
            .insert(template_name.to_string(), output.clone());

        Ok(output)
    }

    /// Renders a template from a file
    ///
    /// # Arguments
    /// * `template_file` - Relative path to template file
    /// * `context` - Context for rendering
    pub fn render_from_file(
        &mut self,
        template_file: &str,
        context: &TeraContext,
    ) -> Result<String> {
        let output = self
            .tera
            .render(template_file, context)
            .context(format!("Failed to render template: {}", template_file))?;

        self.render_cache
            .insert(template_file.to_string(), output.clone());

        Ok(output)
    }

    /// Renders a template with a JSON context file
    ///
    /// # Arguments
    /// * `template_file` - Template file name
    /// * `context_file` - JSON context file name (relative to fixture_dir)
    pub fn render_with_context_file(
        &mut self,
        template_file: &str,
        context_file: &str,
    ) -> Result<String> {
        let context = self.load_context_from_file(context_file)?;
        self.render_from_file(template_file, &context)
    }

    // ========================================================================
    // CONTEXT MANAGEMENT
    // ========================================================================

    /// Loads a context from a JSON file
    pub fn load_context_from_file(&self, context_file: &str) -> Result<TeraContext> {
        let path = self.fixture_dir.join("contexts").join(context_file);
        let content = fs::read_to_string(&path)
            .context(format!("Failed to read context file: {:?}", path))?;

        let json: JsonValue =
            serde_json::from_str(&content).context("Failed to parse context JSON")?;

        TeraContext::from_serialize(&json).context("Failed to create Tera context")
    }

    /// Creates a context from a JSON string
    pub fn context_from_json(&self, json_str: &str) -> Result<TeraContext> {
        let json: JsonValue = serde_json::from_str(json_str)?;
        Ok(TeraContext::from_serialize(&json)?)
    }

    // ========================================================================
    // TEMPLATE VALIDATION
    // ========================================================================

    /// Validates template syntax without rendering
    pub fn validate_template_syntax(&mut self, template_str: &str) -> Result<ValidationResult> {
        let temp_name = format!("__validation_temp_{}", uuid::Uuid::new_v4());

        match self.tera.add_raw_template(&temp_name, template_str) {
            Ok(_) => Ok(ValidationResult {
                valid: true,
                errors: Vec::new(),
                warnings: Vec::new(),
            }),
            Err(e) => Ok(ValidationResult {
                valid: false,
                errors: vec![format!("Syntax error: {}", e)],
                warnings: Vec::new(),
            }),
        }
    }

    /// Checks if a template file exists
    pub fn template_exists(&self, template_file: &str) -> bool {
        self.tera
            .get_template_names()
            .any(|name| name == template_file)
    }

    /// Lists all available templates
    pub fn list_templates(&self) -> Vec<String> {
        self.tera.get_template_names().map(String::from).collect()
    }

    /// Extracts all variables used in a template
    pub fn extract_template_variables(&self, template_name: &str) -> Result<HashSet<String>> {
        let template = self
            .tera
            .get_template(template_name)
            .context("Template not found")?;

        let mut variables = HashSet::new();

        // Parse the template AST to extract variable references
        // This is a simplified version - real implementation would traverse AST
        let source = &template.source;
        extract_variables_from_source(source, &mut variables);

        Ok(variables)
    }

    /// Verifies that all variables in context are used in template
    pub fn verify_context_usage(
        &self,
        template_name: &str,
        context: &TeraContext,
    ) -> Result<UsageReport> {
        let template_vars = self.extract_template_variables(template_name)?;
        let context_vars: HashSet<String> = context
            .clone()
            .into_json()
            .as_object()
            .map(|obj| obj.keys().cloned().collect())
            .unwrap_or_default();

        let unused_context_vars: Vec<String> =
            context_vars.difference(&template_vars).cloned().collect();

        let missing_template_vars: Vec<String> =
            template_vars.difference(&context_vars).cloned().collect();

        Ok(UsageReport {
            unused_context_vars,
            missing_template_vars,
        })
    }

    // ========================================================================
    // GENERATED CODE VALIDATION
    // ========================================================================

    /// Validates that generated output is valid Rust code
    pub fn validate_rust_syntax(&self, code: &str) -> Result<CodeValidation> {
        let mut validation = CodeValidation {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            metrics: CodeMetrics::default(),
        };

        // Check balanced delimiters
        let delimiter_check = check_balanced_delimiters(code);
        if let Err(e) = delimiter_check {
            validation.valid = false;
            validation.errors.push(e);
        }

        // Check for basic Rust patterns
        if self.config.validate_syntax {
            validation.warnings.extend(check_rust_patterns(code));
        }

        // Security checks
        if self.config.security_checks {
            validation.warnings.extend(check_security_patterns(code));
        }

        // Calculate metrics
        validation.metrics = calculate_code_metrics(code);

        Ok(validation)
    }

    /// Attempts to compile generated Rust code (expensive operation)
    #[cfg(feature = "compile-check")]
    pub fn compile_check(&self, code: &str) -> Result<CompileResult> {
        use std::process::Command;

        let temp_dir = tempfile::tempdir()?;
        let src_file = temp_dir.path().join("lib.rs");

        fs::write(&src_file, code)?;

        let output = Command::new("rustc")
            .arg("--crate-type=lib")
            .arg("--emit=metadata")
            .arg(&src_file)
            .arg("--out-dir")
            .arg(temp_dir.path())
            .output()?;

        Ok(CompileResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    // ========================================================================
    // GOLDEN FILE TESTING (SNAPSHOT TESTING)
    // ========================================================================

    /// Compares rendered output against a golden file
    ///
    /// # Arguments
    /// * `golden_file` - Name of golden file in fixtures/expected/
    /// * `rendered` - The rendered output to compare
    pub fn assert_matches_golden(&self, golden_file: &str, rendered: &str) -> Result<()> {
        let golden_path = self.fixture_dir.join("expected").join(golden_file);

        if !golden_path.exists() {
            if self.config.update_golden_files {
                // Create the golden file
                if let Some(parent) = golden_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&golden_path, rendered)?;
                println!("Created golden file: {:?}", golden_path);
                return Ok(());
            } else {
                return Err(anyhow!(
                    "Golden file does not exist: {:?}\nRun with update_golden_files=true to create it",
                    golden_path
                ));
            }
        }

        let expected = fs::read_to_string(&golden_path)
            .context(format!("Failed to read golden file: {:?}", golden_path))?;

        if normalize_whitespace(&expected) != normalize_whitespace(rendered) {
            if self.config.update_golden_files {
                fs::write(&golden_path, rendered)?;
                println!("Updated golden file: {:?}", golden_path);
                Ok(())
            } else {
                // Generate diff for better error message
                let diff = generate_diff(&expected, rendered);
                Err(anyhow!(
                    "Output does not match golden file: {:?}\n\n{}",
                    golden_path,
                    diff
                ))
            }
        } else {
            Ok(())
        }
    }

    // ========================================================================
    // BEHAVIOR VERIFICATION
    // ========================================================================

    /// Verifies that template renders without errors
    pub fn verify_renders_successfully(
        &mut self,
        template_file: &str,
        context: &TeraContext,
    ) -> Result<()> {
        self.render_from_file(template_file, context)?;
        Ok(())
    }

    /// Verifies that rendered output contains expected strings
    pub fn verify_contains(&self, template_name: &str, expected: &[&str]) -> Result<()> {
        let output = self
            .render_cache
            .get(template_name)
            .ok_or_else(|| anyhow!("Template not in cache: {}", template_name))?;

        for &exp in expected {
            if !output.contains(exp) {
                return Err(anyhow!(
                    "Output does not contain expected string: '{}'",
                    exp
                ));
            }
        }

        Ok(())
    }

    /// Verifies that rendered output does not contain strings
    pub fn verify_not_contains(&self, template_name: &str, forbidden: &[&str]) -> Result<()> {
        let output = self
            .render_cache
            .get(template_name)
            .ok_or_else(|| anyhow!("Template not in cache: {}", template_name))?;

        for &forb in forbidden {
            if output.contains(forb) {
                return Err(anyhow!("Output contains forbidden string: '{}'", forb));
            }
        }

        Ok(())
    }

    /// Verifies conditional blocks work correctly
    pub fn verify_conditionals(
        &mut self,
        template_str: &str,
        true_context: &TeraContext,
        false_context: &TeraContext,
        expected_when_true: &str,
        expected_when_false: &str,
    ) -> Result<()> {
        let true_output = self.render_from_string("cond_true", template_str, true_context)?;
        let false_output = self.render_from_string("cond_false", template_str, false_context)?;

        if !true_output.contains(expected_when_true) {
            return Err(anyhow!(
                "Conditional true case failed: expected '{}' not found",
                expected_when_true
            ));
        }

        if !false_output.contains(expected_when_false) {
            return Err(anyhow!(
                "Conditional false case failed: expected '{}' not found",
                expected_when_false
            ));
        }

        Ok(())
    }

    /// Verifies loops iterate correctly
    pub fn verify_loop_iteration(
        &mut self,
        template_str: &str,
        context: &TeraContext,
        expected_count: usize,
    ) -> Result<()> {
        let output = self.render_from_string("loop_test", template_str, context)?;

        // Count iterations (this is simplified - real impl would be more sophisticated)
        let actual_count = output.lines().filter(|l| !l.trim().is_empty()).count();

        if actual_count != expected_count {
            return Err(anyhow!(
                "Loop iteration count mismatch: expected {}, got {}",
                expected_count,
                actual_count
            ));
        }

        Ok(())
    }

    /// Verifies filters are applied correctly
    pub fn verify_filter_applied(
        &mut self,
        template_str: &str,
        context: &TeraContext,
        expected_transformation: &str,
    ) -> Result<()> {
        let output = self.render_from_string("filter_test", template_str, context)?;

        if !output.contains(expected_transformation) {
            return Err(anyhow!(
                "Filter not applied correctly: expected '{}' not found in output",
                expected_transformation
            ));
        }

        Ok(())
    }

    /// Gets the last rendered output for a template
    pub fn get_rendered(&self, template_name: &str) -> Option<&String> {
        self.render_cache.get(template_name)
    }
}

// ============================================================================
// CONTEXT BUILDER
// ============================================================================

/// Builder for creating test contexts fluently
#[derive(Debug, Default)]
pub struct TemplateContextBuilder {
    data: serde_json::Map<String, JsonValue>,
}

impl TemplateContextBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets an entity name
    pub fn entity(mut self, name: &str) -> Self {
        self.data.insert(
            "entity_name".to_string(),
            JsonValue::String(name.to_string()),
        );
        self
    }

    /// Adds a field to the fields array
    pub fn field(mut self, name: &str, rust_type: &str) -> Self {
        let field = serde_json::json!({
            "name": name,
            "rust_type": rust_type,
            "description": format!("{} field", name),
            "required": true
        });

        let fields = self
            .data
            .entry("fields".to_string())
            .or_insert_with(|| JsonValue::Array(Vec::new()));

        if let JsonValue::Array(ref mut arr) = fields {
            arr.push(field);
        }

        self
    }

    /// Sets a boolean flag
    pub fn flag(mut self, name: &str, value: bool) -> Self {
        self.data.insert(name.to_string(), JsonValue::Bool(value));
        self
    }

    /// Sets a string value
    pub fn value(mut self, name: &str, value: &str) -> Self {
        self.data
            .insert(name.to_string(), JsonValue::String(value.to_string()));
        self
    }

    /// Sets a custom JSON value
    pub fn custom<T: Serialize>(mut self, name: &str, value: T) -> Result<Self> {
        let json = serde_json::to_value(value)?;
        self.data.insert(name.to_string(), json);
        Ok(self)
    }

    /// Builds the Tera context
    pub fn build(self) -> Result<TeraContext> {
        Ok(TeraContext::from_serialize(&self.data)?)
    }
}

// ============================================================================
// RESULT TYPES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageReport {
    pub unused_context_vars: Vec<String>,
    pub missing_template_vars: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeValidation {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub metrics: CodeMetrics,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodeMetrics {
    pub line_count: usize,
    pub char_count: usize,
    pub has_imports: bool,
    pub has_docs: bool,
    pub has_tests: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Extracts variable names from template source (simplified)
fn extract_variables_from_source(source: &str, variables: &mut HashSet<String>) {
    // Simple regex-based extraction (in production, would use AST)
    let var_pattern = regex::Regex::new(r"\{\{\s*([a-zA-Z_][a-zA-Z0-9_]*)").expect("Invalid regex");

    for cap in var_pattern.captures_iter(source) {
        if let Some(var_name) = cap.get(1) {
            variables.insert(var_name.as_str().to_string());
        }
    }
}

/// Checks for balanced delimiters in code
fn check_balanced_delimiters(code: &str) -> Result<()> {
    let mut stack = Vec::new();
    let pairs = [('(', ')'), ('[', ']'), ('{', '}')];

    // Simple character-by-character check (ignores strings/comments)
    for ch in code.chars() {
        match ch {
            '(' | '[' | '{' => stack.push(ch),
            ')' | ']' | '}' => {
                if let Some(last) = stack.pop() {
                    let expected = pairs
                        .iter()
                        .find(|(open, _)| *open == last)
                        .map(|(_, close)| close);

                    if expected != Some(&ch) {
                        return Err(anyhow!(
                            "Mismatched delimiter: expected {:?}, got {}",
                            expected,
                            ch
                        ));
                    }
                } else {
                    return Err(anyhow!("Unmatched closing delimiter: {}", ch));
                }
            }
            _ => {}
        }
    }

    if !stack.is_empty() {
        return Err(anyhow!("Unclosed delimiters: {:?}", stack));
    }

    Ok(())
}

/// Checks for common Rust patterns
fn check_rust_patterns(code: &str) -> Vec<String> {
    let mut warnings = Vec::new();

    // Check for common issues
    if !code.contains("use ") && code.len() > 100 {
        warnings.push("No import statements found".to_string());
    }

    if code.contains("pub struct") || code.contains("pub fn") {
        if !code.contains("///") && !code.contains("//!") {
            warnings.push("Public items lack documentation comments".to_string());
        }
    }

    warnings
}

/// Checks for security patterns
fn check_security_patterns(code: &str) -> Vec<String> {
    let mut warnings = Vec::new();

    let dangerous_patterns = [
        ("unsafe {", "Contains unsafe code"),
        ("std::process::Command", "Uses process execution"),
        ("std::fs::remove", "Uses file deletion"),
        ("std::fs::write", "Uses file writing"),
    ];

    for (pattern, message) in &dangerous_patterns {
        if code.contains(pattern) {
            warnings.push(message.to_string());
        }
    }

    warnings
}

/// Calculates code metrics
fn calculate_code_metrics(code: &str) -> CodeMetrics {
    CodeMetrics {
        line_count: code.lines().count(),
        char_count: code.len(),
        has_imports: code.contains("use "),
        has_docs: code.contains("///") || code.contains("//!"),
        has_tests: code.contains("#[test]") || code.contains("#[cfg(test)]"),
    }
}

/// Normalizes whitespace for comparison
fn normalize_whitespace(s: &str) -> String {
    s.lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Generates a diff between two strings
fn generate_diff(expected: &str, actual: &str) -> String {
    use similar::{ChangeTag, TextDiff};

    let diff = TextDiff::from_lines(expected, actual);
    let mut output = String::new();

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        output.push_str(&format!("{}{}", sign, change));
    }

    output
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_template_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates")
    }

    fn test_fixture_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tera")
    }

    #[test]
    fn test_harness_creation() {
        let harness = TemplateTestHarness::new(test_template_dir(), test_fixture_dir());
        assert!(harness.is_ok(), "Should create harness successfully");
    }

    #[test]
    fn test_context_builder() {
        let context = TemplateContextBuilder::new()
            .entity("User")
            .field("name", "String")
            .field("email", "Email")
            .flag("has_id", true)
            .value("description", "User entity")
            .build();

        assert!(context.is_ok(), "Should build context successfully");
    }

    #[test]
    fn test_render_from_string() {
        let mut harness = TemplateTestHarness::new(test_template_dir(), test_fixture_dir())
            .expect("Failed to create harness");

        let template = "Hello {{ name }}!";
        let mut context = TeraContext::new();
        context.insert("name", "World");

        let result = harness.render_from_string("test", template, &context);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello World!");
    }

    #[test]
    fn test_validate_template_syntax() {
        let mut harness = TemplateTestHarness::new(test_template_dir(), test_fixture_dir())
            .expect("Failed to create harness");

        let valid_template = "{% if true %}Valid{% endif %}";
        let result = harness.validate_template_syntax(valid_template);
        assert!(result.is_ok());
        assert!(result.unwrap().valid);

        let invalid_template = "{% if true %}Missing endif";
        let result = harness.validate_template_syntax(invalid_template);
        assert!(result.is_ok());
        assert!(!result.unwrap().valid);
    }

    #[test]
    fn test_balanced_delimiters() {
        let valid_code = "fn main() { let x = vec![1, 2, 3]; }";
        assert!(check_balanced_delimiters(valid_code).is_ok());

        let invalid_code = "fn main() { let x = vec![1, 2, 3; }";
        assert!(check_balanced_delimiters(invalid_code).is_err());
    }

    #[test]
    fn test_normalize_whitespace() {
        let input = "  line 1  \n  line 2\n\n  line 3  ";
        let expected = "line 1\n  line 2\n\n  line 3";
        assert_eq!(normalize_whitespace(input), expected);
    }
}
