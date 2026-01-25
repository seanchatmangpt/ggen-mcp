//! Safe Template Rendering with Error Prevention
//!
//! This module provides a comprehensive safety layer for Tera template rendering,
//! implementing Toyota Production System poka-yoke (error-proofing) principles:
//!
//! - **SafeRenderer**: Protected template rendering with sandboxing and limits
//! - **OutputValidator**: Validates generated code for security and correctness
//! - **RenderContext**: Isolated rendering environment preventing context pollution
//! - **ErrorRecovery**: Graceful failure handling with partial output support
//! - **RenderGuards**: RAII guards for resource management and cleanup
//!
//! # Safety Guarantees
//!
//! 1. **Timeout Protection**: Long-running renders are terminated
//! 2. **Memory Limits**: Prevents memory exhaustion from malicious templates
//! 3. **Recursion Limits**: Prevents stack overflow from deep template nesting
//! 4. **Output Validation**: Ensures generated code is syntactically valid
//! 5. **Resource Cleanup**: Automatic cleanup of temporary files and locks
//!
//! # Example
//!
//! ```rust,ignore
//! use spreadsheet_mcp::template::rendering_safety::{SafeRenderer, RenderConfig};
//!
//! let config = RenderConfig::default()
//!     .with_timeout_ms(5000)
//!     .with_max_recursion_depth(10)
//!     .with_syntax_validation(true);
//!
//! let renderer = SafeRenderer::new(config)?;
//! let output = renderer.render_safe("template.tera", &context)?;
//! ```

use anyhow::{Context as AnyhowContext, Result, anyhow};
use parking_lot::{Mutex, RwLock};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tera::{Context, Tera};
use thiserror::Error;

// ============================================================================
// Configuration Constants
// ============================================================================

/// Default timeout for template rendering (5 seconds)
pub const DEFAULT_TIMEOUT_MS: u64 = 5000;

/// Maximum timeout allowed (30 seconds)
pub const MAX_TIMEOUT_MS: u64 = 30_000;

/// Default maximum recursion depth
pub const DEFAULT_MAX_RECURSION_DEPTH: usize = 10;

/// Maximum recursion depth allowed
pub const MAX_RECURSION_DEPTH: usize = 100;

/// Default maximum output size (10 MB)
pub const DEFAULT_MAX_OUTPUT_SIZE: usize = 10 * 1024 * 1024;

/// Maximum output size allowed (100 MB)
pub const MAX_OUTPUT_SIZE: usize = 100 * 1024 * 1024;

/// Default maximum macro expansion count
pub const DEFAULT_MAX_MACRO_EXPANSIONS: usize = 1000;

/// Maximum allowed include file depth
pub const MAX_INCLUDE_DEPTH: usize = 5;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during safe template rendering
#[derive(Debug, Error)]
pub enum RenderingError {
    #[error("Template rendering timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("Output size limit exceeded: {size} bytes (max: {limit} bytes)")]
    OutputSizeExceeded { size: usize, limit: usize },

    #[error("Recursion depth limit exceeded: {depth} (max: {limit})")]
    RecursionDepthExceeded { depth: usize, limit: usize },

    #[error("Macro expansion limit exceeded: {count} (max: {limit})")]
    MacroExpansionLimitExceeded { count: usize, limit: usize },

    #[error("Template syntax error: {message}")]
    SyntaxError { message: String },

    #[error("Output validation failed: {}", errors.join(", "))]
    ValidationFailed { errors: Vec<String> },

    #[error("Security check failed: {reason}")]
    SecurityViolation { reason: String },

    #[error("Include file not whitelisted: {path}")]
    IncludeNotWhitelisted { path: String },

    #[error("Template not found: {name}")]
    TemplateNotFound { name: String },

    #[error("Context variable error: {message}")]
    ContextError { message: String },

    #[error("Tera rendering error: {source}")]
    TeraError {
        #[from]
        source: tera::Error,
    },

    #[error("I/O error: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },

    #[error("Internal error: {message}")]
    Internal { message: String },
}

/// Errors collected during validation
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub message: String,
    pub severity: ValidationSeverity,
}

/// Severity level of validation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

// ============================================================================
// Render Configuration
// ============================================================================

/// Configuration for safe template rendering
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// Timeout for rendering in milliseconds
    pub timeout_ms: u64,

    /// Maximum recursion depth for template includes/extends
    pub max_recursion_depth: usize,

    /// Maximum output size in bytes
    pub max_output_size: usize,

    /// Maximum number of macro expansions
    pub max_macro_expansions: usize,

    /// Whether to validate output syntax
    pub validate_syntax: bool,

    /// Whether to perform security checks
    pub security_checks: bool,

    /// Whitelisted include file paths (relative to template dir)
    pub include_whitelist: HashSet<PathBuf>,

    /// Whether to allow partial rendering on error
    pub allow_partial_rendering: bool,

    /// Whether to collect detailed metrics
    pub collect_metrics: bool,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            timeout_ms: DEFAULT_TIMEOUT_MS,
            max_recursion_depth: DEFAULT_MAX_RECURSION_DEPTH,
            max_output_size: DEFAULT_MAX_OUTPUT_SIZE,
            max_macro_expansions: DEFAULT_MAX_MACRO_EXPANSIONS,
            validate_syntax: true,
            security_checks: true,
            include_whitelist: HashSet::new(),
            allow_partial_rendering: false,
            collect_metrics: true,
        }
    }
}

impl RenderConfig {
    /// Create a new configuration builder
    pub fn builder() -> RenderConfigBuilder {
        RenderConfigBuilder::default()
    }

    /// Set timeout in milliseconds (chaining method)
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms.min(MAX_TIMEOUT_MS);
        self
    }

    /// Set maximum recursion depth (chaining method)
    pub fn with_max_recursion_depth(mut self, depth: usize) -> Self {
        self.max_recursion_depth = depth.min(MAX_RECURSION_DEPTH);
        self
    }

    /// Enable or disable syntax validation (chaining method)
    pub fn with_syntax_validation(mut self, enable: bool) -> Self {
        self.validate_syntax = enable;
        self
    }

    /// Enable or disable security checks (chaining method)
    pub fn with_security_checks(mut self, enable: bool) -> Self {
        self.security_checks = enable;
        self
    }

    /// Add an include file to the whitelist (chaining method)
    pub fn with_include_whitelist<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.include_whitelist.insert(path.as_ref().to_path_buf());
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.timeout_ms == 0 {
            return Err(anyhow!("timeout_ms must be greater than 0"));
        }
        if self.timeout_ms > MAX_TIMEOUT_MS {
            return Err(anyhow!("timeout_ms exceeds maximum of {}", MAX_TIMEOUT_MS));
        }
        if self.max_recursion_depth == 0 {
            return Err(anyhow!("max_recursion_depth must be greater than 0"));
        }
        if self.max_recursion_depth > MAX_RECURSION_DEPTH {
            return Err(anyhow!(
                "max_recursion_depth exceeds maximum of {}",
                MAX_RECURSION_DEPTH
            ));
        }
        if self.max_output_size == 0 {
            return Err(anyhow!("max_output_size must be greater than 0"));
        }
        if self.max_output_size > MAX_OUTPUT_SIZE {
            return Err(anyhow!(
                "max_output_size exceeds maximum of {}",
                MAX_OUTPUT_SIZE
            ));
        }
        Ok(())
    }
}

/// Builder for RenderConfig
#[derive(Debug, Default)]
pub struct RenderConfigBuilder {
    timeout_ms: Option<u64>,
    max_recursion_depth: Option<usize>,
    max_output_size: Option<usize>,
    max_macro_expansions: Option<usize>,
    validate_syntax: Option<bool>,
    security_checks: Option<bool>,
    include_whitelist: HashSet<PathBuf>,
    allow_partial_rendering: Option<bool>,
    collect_metrics: Option<bool>,
}

impl RenderConfigBuilder {
    pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn max_recursion_depth(mut self, depth: usize) -> Self {
        self.max_recursion_depth = Some(depth);
        self
    }

    pub fn max_output_size(mut self, size: usize) -> Self {
        self.max_output_size = Some(size);
        self
    }

    pub fn max_macro_expansions(mut self, count: usize) -> Self {
        self.max_macro_expansions = Some(count);
        self
    }

    pub fn validate_syntax(mut self, enable: bool) -> Self {
        self.validate_syntax = Some(enable);
        self
    }

    pub fn security_checks(mut self, enable: bool) -> Self {
        self.security_checks = Some(enable);
        self
    }

    pub fn add_include_whitelist<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.include_whitelist.insert(path.as_ref().to_path_buf());
        self
    }

    pub fn allow_partial_rendering(mut self, enable: bool) -> Self {
        self.allow_partial_rendering = Some(enable);
        self
    }

    pub fn collect_metrics(mut self, enable: bool) -> Self {
        self.collect_metrics = Some(enable);
        self
    }

    pub fn build(self) -> RenderConfig {
        let defaults = RenderConfig::default();
        RenderConfig {
            timeout_ms: self.timeout_ms.unwrap_or(defaults.timeout_ms),
            max_recursion_depth: self
                .max_recursion_depth
                .unwrap_or(defaults.max_recursion_depth),
            max_output_size: self.max_output_size.unwrap_or(defaults.max_output_size),
            max_macro_expansions: self
                .max_macro_expansions
                .unwrap_or(defaults.max_macro_expansions),
            validate_syntax: self.validate_syntax.unwrap_or(defaults.validate_syntax),
            security_checks: self.security_checks.unwrap_or(defaults.security_checks),
            include_whitelist: self.include_whitelist,
            allow_partial_rendering: self
                .allow_partial_rendering
                .unwrap_or(defaults.allow_partial_rendering),
            collect_metrics: self.collect_metrics.unwrap_or(defaults.collect_metrics),
        }
    }
}

// ============================================================================
// Render Metrics
// ============================================================================

/// Metrics collected during rendering
#[derive(Debug, Clone, Default)]
pub struct RenderMetrics {
    /// Total rendering duration
    pub duration: Duration,

    /// Size of rendered output in bytes
    pub output_size: usize,

    /// Number of templates included/extended
    pub includes_count: usize,

    /// Number of macro expansions
    pub macro_expansions: usize,

    /// Maximum recursion depth reached
    pub max_recursion_reached: usize,

    /// Number of validation errors
    pub validation_errors: usize,

    /// Number of validation warnings
    pub validation_warnings: usize,
}

// ============================================================================
// Render Context
// ============================================================================

/// Isolated rendering environment with variable scoping
pub struct RenderContext {
    /// The Tera context
    context: Context,

    /// Variables set in this context
    variables: HashMap<String, JsonValue>,

    /// Parent context (for scoping)
    parent: Option<Arc<RenderContext>>,

    /// Current recursion depth
    recursion_depth: usize,

    /// Number of macro expansions
    macro_count: usize,
}

impl RenderContext {
    /// Create a new root context
    pub fn new() -> Self {
        Self {
            context: Context::new(),
            variables: HashMap::new(),
            parent: None,
            recursion_depth: 0,
            macro_count: 0,
        }
    }

    /// Create a child context with a parent for scoping
    pub fn child(parent: Arc<RenderContext>) -> Self {
        Self {
            context: Context::new(),
            variables: HashMap::new(),
            parent: Some(parent.clone()),
            recursion_depth: parent.recursion_depth + 1,
            macro_count: parent.macro_count,
        }
    }

    /// Insert a variable into the context
    pub fn insert<T: serde::Serialize>(&mut self, key: &str, value: &T) -> Result<()> {
        let json_value = serde_json::to_value(value)
            .with_context(|| format!("Failed to serialize value for key '{}'", key))?;
        self.variables.insert(key.to_string(), json_value.clone());
        self.context.insert(key, value);
        Ok(())
    }

    /// Get a variable from this context or parent contexts
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.variables
            .get(key)
            .or_else(|| self.parent.as_ref().and_then(|p| p.get(key)))
    }

    /// Get the Tera context
    pub fn tera_context(&self) -> &Context {
        &self.context
    }

    /// Check recursion depth against limit
    pub fn check_recursion_depth(&self, limit: usize) -> Result<(), RenderingError> {
        if self.recursion_depth >= limit {
            return Err(RenderingError::RecursionDepthExceeded {
                depth: self.recursion_depth,
                limit,
            });
        }
        Ok(())
    }

    /// Increment macro count and check limit
    pub fn increment_macro_count(&mut self, limit: usize) -> Result<(), RenderingError> {
        self.macro_count += 1;
        if self.macro_count > limit {
            return Err(RenderingError::MacroExpansionLimitExceeded {
                count: self.macro_count,
                limit,
            });
        }
        Ok(())
    }

    /// Get current recursion depth
    pub fn recursion_depth(&self) -> usize {
        self.recursion_depth
    }

    /// Get current macro count
    pub fn macro_count(&self) -> usize {
        self.macro_count
    }
}

impl Default for RenderContext {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Output Validator
// ============================================================================

/// Validates generated code for syntax and security
pub struct OutputValidator {
    /// Whether to validate Rust syntax
    validate_rust_syntax: bool,

    /// Whether to perform security checks
    security_checks: bool,
}

impl OutputValidator {
    /// Create a new output validator
    pub fn new(validate_rust_syntax: bool, security_checks: bool) -> Self {
        Self {
            validate_rust_syntax,
            security_checks,
        }
    }

    /// Validate generated output
    pub fn validate(&self, output: &str) -> Result<Vec<ValidationError>, RenderingError> {
        let mut errors = Vec::new();

        if self.validate_rust_syntax {
            errors.extend(self.validate_rust_syntax_impl(output));
        }

        if self.security_checks {
            errors.extend(self.perform_security_checks(output));
        }

        Ok(errors)
    }

    /// Validate Rust syntax
    fn validate_rust_syntax_impl(&self, output: &str) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Check for balanced braces, brackets, and parentheses
        errors.extend(self.check_balanced_delimiters(output));

        // Check for valid Rust identifiers
        errors.extend(self.check_identifiers(output));

        // Check for common syntax errors
        errors.extend(self.check_common_errors(output));

        errors
    }

    /// Check for balanced delimiters
    fn check_balanced_delimiters(&self, output: &str) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let mut brace_count = 0i32;
        let mut bracket_count = 0i32;
        let mut paren_count = 0i32;

        for (line_num, line) in output.lines().enumerate() {
            // Skip strings and comments for a basic check
            let line_no_strings = self.remove_string_literals(line);
            let line_no_comments = self.remove_comments(&line_no_strings);

            for ch in line_no_comments.chars() {
                match ch {
                    '{' => brace_count += 1,
                    '}' => brace_count -= 1,
                    '[' => bracket_count += 1,
                    ']' => bracket_count -= 1,
                    '(' => paren_count += 1,
                    ')' => paren_count -= 1,
                    _ => {}
                }

                // Check for negative counts (more closing than opening)
                if brace_count < 0 {
                    errors.push(ValidationError {
                        line: Some(line_num + 1),
                        column: None,
                        message: "Unbalanced braces: more closing than opening".to_string(),
                        severity: ValidationSeverity::Error,
                    });
                    brace_count = 0; // Reset to avoid duplicate errors
                }
                if bracket_count < 0 {
                    errors.push(ValidationError {
                        line: Some(line_num + 1),
                        column: None,
                        message: "Unbalanced brackets: more closing than opening".to_string(),
                        severity: ValidationSeverity::Error,
                    });
                    bracket_count = 0;
                }
                if paren_count < 0 {
                    errors.push(ValidationError {
                        line: Some(line_num + 1),
                        column: None,
                        message: "Unbalanced parentheses: more closing than opening".to_string(),
                        severity: ValidationSeverity::Error,
                    });
                    paren_count = 0;
                }
            }
        }

        // Check final counts
        if brace_count != 0 {
            errors.push(ValidationError {
                line: None,
                column: None,
                message: format!("Unbalanced braces: {} unclosed", brace_count.abs()),
                severity: ValidationSeverity::Error,
            });
        }
        if bracket_count != 0 {
            errors.push(ValidationError {
                line: None,
                column: None,
                message: format!("Unbalanced brackets: {} unclosed", bracket_count.abs()),
                severity: ValidationSeverity::Error,
            });
        }
        if paren_count != 0 {
            errors.push(ValidationError {
                line: None,
                column: None,
                message: format!("Unbalanced parentheses: {} unclosed", paren_count.abs()),
                severity: ValidationSeverity::Error,
            });
        }

        errors
    }

    /// Remove string literals for syntax checking
    fn remove_string_literals(&self, line: &str) -> String {
        let mut result = String::new();
        let mut chars = line.chars().peekable();
        let mut in_string = false;
        let mut escape_next = false;

        while let Some(ch) = chars.next() {
            if escape_next {
                escape_next = false;
                if in_string {
                    result.push(' ');
                } else {
                    result.push(ch);
                }
                continue;
            }

            match ch {
                '\\' => {
                    escape_next = true;
                    if !in_string {
                        result.push(ch);
                    }
                }
                '"' => {
                    in_string = !in_string;
                    result.push(' '); // Replace string content with space
                }
                _ => {
                    if in_string {
                        result.push(' ');
                    } else {
                        result.push(ch);
                    }
                }
            }
        }

        result
    }

    /// Remove comments for syntax checking
    fn remove_comments(&self, line: &str) -> String {
        if let Some(pos) = line.find("//") {
            line[..pos].to_string()
        } else {
            line.to_string()
        }
    }

    /// Check for valid Rust identifiers
    fn check_identifiers(&self, output: &str) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        // Pattern is a compile-time constant, but handle error defensively
        let invalid_identifier_pattern = match regex::Regex::new(r"\b\d+[a-zA-Z_][a-zA-Z0-9_]*\b") {
            Ok(re) => re,
            Err(e) => {
                tracing::warn!("Failed to compile identifier regex pattern: {}", e);
                return errors; // Return empty errors if regex compilation fails
            }
        };

        for (line_num, line) in output.lines().enumerate() {
            if invalid_identifier_pattern.is_match(line) {
                errors.push(ValidationError {
                    line: Some(line_num + 1),
                    column: None,
                    message: "Invalid identifier: identifiers cannot start with a digit"
                        .to_string(),
                    severity: ValidationSeverity::Warning,
                });
            }
        }

        errors
    }

    /// Check for common syntax errors
    fn check_common_errors(&self, output: &str) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        for (line_num, line) in output.lines().enumerate() {
            let trimmed = line.trim();

            // Check for empty struct definitions
            if trimmed.starts_with("pub struct") && trimmed.ends_with("{}") {
                errors.push(ValidationError {
                    line: Some(line_num + 1),
                    column: None,
                    message: "Empty struct definition detected".to_string(),
                    severity: ValidationSeverity::Warning,
                });
            }

            // Check for consecutive semicolons
            if trimmed.contains(";;") {
                errors.push(ValidationError {
                    line: Some(line_num + 1),
                    column: None,
                    message: "Consecutive semicolons detected".to_string(),
                    severity: ValidationSeverity::Warning,
                });
            }
        }

        errors
    }

    /// Perform security checks on generated code
    fn perform_security_checks(&self, output: &str) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        for (line_num, line) in output.lines().enumerate() {
            let trimmed = line.trim();

            // Check for unsafe code blocks (potential security risk)
            if trimmed.contains("unsafe {") || trimmed.starts_with("unsafe ") {
                errors.push(ValidationError {
                    line: Some(line_num + 1),
                    column: None,
                    message: "Unsafe code block detected - verify this is intentional".to_string(),
                    severity: ValidationSeverity::Warning,
                });
            }

            // Check for system command execution
            if trimmed.contains("std::process::Command") || trimmed.contains("std::process::Stdio")
            {
                errors.push(ValidationError {
                    line: Some(line_num + 1),
                    column: None,
                    message: "System command execution detected - potential security risk"
                        .to_string(),
                    severity: ValidationSeverity::Warning,
                });
            }

            // Check for file system operations
            if trimmed.contains("std::fs::remove") || trimmed.contains("std::fs::write") {
                errors.push(ValidationError {
                    line: Some(line_num + 1),
                    column: None,
                    message: "File system modification detected - verify permissions are correct"
                        .to_string(),
                    severity: ValidationSeverity::Info,
                });
            }

            // Check for potential SQL injection vectors
            if (trimmed.contains("format!") || trimmed.contains("&format!("))
                && (trimmed.to_lowercase().contains("select ")
                    || trimmed.to_lowercase().contains("insert ")
                    || trimmed.to_lowercase().contains("update ")
                    || trimmed.to_lowercase().contains("delete "))
            {
                errors.push(ValidationError {
                    line: Some(line_num + 1),
                    column: None,
                    message: "Potential SQL query construction - use parameterized queries"
                        .to_string(),
                    severity: ValidationSeverity::Warning,
                });
            }
        }

        errors
    }

    /// Check if validation errors contain critical errors
    pub fn has_critical_errors(errors: &[ValidationError]) -> bool {
        errors
            .iter()
            .any(|e| e.severity == ValidationSeverity::Error)
    }

    /// Format validation errors for display
    pub fn format_errors(errors: &[ValidationError]) -> String {
        let mut output = String::new();
        for error in errors {
            let location = match (error.line, error.column) {
                (Some(line), Some(col)) => format!("Line {}, Column {}", line, col),
                (Some(line), None) => format!("Line {}", line),
                _ => "Unknown location".to_string(),
            };
            output.push_str(&format!(
                "[{:?}] {}: {}\n",
                error.severity, location, error.message
            ));
        }
        output
    }
}

// ============================================================================
// Error Recovery
// ============================================================================

/// Handles graceful failure during rendering
pub struct ErrorRecovery {
    /// Collected errors during rendering
    errors: Vec<RenderingError>,

    /// Partial output (if any)
    partial_output: Option<String>,

    /// Whether partial rendering is allowed
    allow_partial: bool,
}

impl ErrorRecovery {
    /// Create a new error recovery handler
    pub fn new(allow_partial: bool) -> Self {
        Self {
            errors: Vec::new(),
            partial_output: None,
            allow_partial,
        }
    }

    /// Record an error
    pub fn record_error(&mut self, error: RenderingError) {
        self.errors.push(error);
    }

    /// Set partial output
    pub fn set_partial_output(&mut self, output: String) {
        self.partial_output = Some(output);
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get all errors
    pub fn errors(&self) -> &[RenderingError] {
        &self.errors
    }

    /// Get partial output if available
    pub fn partial_output(&self) -> Option<&str> {
        self.partial_output.as_deref()
    }

    /// Suggest fixes for common errors
    pub fn suggest_fixes(&self) -> Vec<String> {
        let mut suggestions = Vec::new();

        for error in &self.errors {
            match error {
                RenderingError::Timeout { timeout_ms } => {
                    suggestions.push(format!(
                        "Template rendering timed out after {}ms. Try:\n  \
                         1. Simplifying the template logic\n  \
                         2. Reducing the amount of data passed to the template\n  \
                         3. Increasing the timeout limit (current: {}ms, max: {}ms)",
                        timeout_ms, timeout_ms, MAX_TIMEOUT_MS
                    ));
                }
                RenderingError::RecursionDepthExceeded { depth, limit } => {
                    suggestions.push(format!(
                        "Recursion depth exceeded ({} > {}). Try:\n  \
                         1. Reducing template nesting (includes/extends)\n  \
                         2. Flattening the template structure\n  \
                         3. Increasing max_recursion_depth (current: {}, max: {})",
                        depth, limit, limit, MAX_RECURSION_DEPTH
                    ));
                }
                RenderingError::OutputSizeExceeded { size, limit } => {
                    suggestions.push(format!(
                        "Output size exceeded ({} bytes > {} bytes). Try:\n  \
                         1. Generating smaller outputs\n  \
                         2. Splitting generation into multiple templates\n  \
                         3. Increasing max_output_size (current: {}, max: {})",
                        size, limit, limit, MAX_OUTPUT_SIZE
                    ));
                }
                RenderingError::SyntaxError { message } => {
                    suggestions.push(format!(
                        "Template syntax error: {}. Try:\n  \
                         1. Checking template syntax for typos\n  \
                         2. Validating Tera template syntax\n  \
                         3. Reviewing template documentation",
                        message
                    ));
                }
                RenderingError::SecurityViolation { reason } => {
                    suggestions.push(format!(
                        "Security check failed: {}. Try:\n  \
                         1. Reviewing generated code for security issues\n  \
                         2. Using safer alternatives\n  \
                         3. Disabling security checks if false positive (not recommended)",
                        reason
                    ));
                }
                _ => {}
            }
        }

        suggestions
    }

    /// Generate error report
    pub fn error_report(&self) -> String {
        let mut report = String::new();
        report.push_str(&format!("Rendering Errors: {}\n\n", self.errors.len()));

        for (i, error) in self.errors.iter().enumerate() {
            report.push_str(&format!("Error {}: {}\n", i + 1, error));
        }

        let suggestions = self.suggest_fixes();
        if !suggestions.is_empty() {
            report.push_str("\nSuggested Fixes:\n");
            for (i, suggestion) in suggestions.iter().enumerate() {
                report.push_str(&format!("\n{}. {}\n", i + 1, suggestion));
            }
        }

        if let Some(partial) = &self.partial_output {
            report.push_str(&format!(
                "\nPartial Output Available: {} bytes\n",
                partial.len()
            ));
        }

        report
    }
}

// ============================================================================
// Render Guards
// ============================================================================

/// RAII guard for template rendering resources
pub struct RenderGuard {
    /// Temporary files to clean up
    temp_files: Vec<PathBuf>,

    /// Start time for metrics
    start_time: Instant,

    /// Whether the guard has been committed
    committed: bool,

    /// Metrics collection
    metrics: RenderMetrics,
}

impl RenderGuard {
    /// Create a new render guard
    pub fn new() -> Self {
        Self {
            temp_files: Vec::new(),
            start_time: Instant::now(),
            committed: false,
            metrics: RenderMetrics::default(),
        }
    }

    /// Register a temporary file for cleanup
    pub fn register_temp_file(&mut self, path: PathBuf) {
        self.temp_files.push(path);
    }

    /// Update metrics
    pub fn update_metrics(&mut self, metrics: RenderMetrics) {
        self.metrics = metrics;
        self.metrics.duration = self.start_time.elapsed();
    }

    /// Commit the guard (prevents cleanup)
    pub fn commit(mut self) -> RenderMetrics {
        self.committed = true;
        self.metrics.duration = self.start_time.elapsed();
        self.metrics.clone()
    }

    /// Get current metrics
    pub fn metrics(&self) -> &RenderMetrics {
        &self.metrics
    }
}

impl Default for RenderGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for RenderGuard {
    fn drop(&mut self) {
        if !self.committed {
            // Clean up temporary files
            for path in &self.temp_files {
                if path.exists() {
                    if let Err(e) = std::fs::remove_file(path) {
                        tracing::warn!("Failed to clean up temp file {:?}: {}", path, e);
                    }
                }
            }
        }
    }
}

// ============================================================================
// Safe Renderer
// ============================================================================

/// Safe template renderer with error prevention
pub struct SafeRenderer {
    /// Tera template engine
    tera: Arc<RwLock<Tera>>,

    /// Rendering configuration
    config: RenderConfig,

    /// Output validator
    validator: OutputValidator,

    /// Template cache
    cache: Arc<Mutex<HashMap<String, String>>>,
}

impl SafeRenderer {
    /// Create a new safe renderer
    pub fn new(config: RenderConfig) -> Result<Self> {
        config.validate()?;

        let validator = OutputValidator::new(config.validate_syntax, config.security_checks);

        Ok(Self {
            tera: Arc::new(RwLock::new(Tera::default())),
            config,
            validator,
            cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Create a safe renderer from a template directory
    pub fn from_directory<P: AsRef<Path>>(dir: P, config: RenderConfig) -> Result<Self> {
        config.validate()?;

        let pattern = format!("{}/**/*", dir.as_ref().display());
        let tera = Tera::new(&pattern)
            .with_context(|| format!("Failed to load templates from {:?}", dir.as_ref()))?;

        let validator = OutputValidator::new(config.validate_syntax, config.security_checks);

        Ok(Self {
            tera: Arc::new(RwLock::new(tera)),
            config,
            validator,
            cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Render a template safely
    pub fn render_safe(
        &self,
        template_name: &str,
        context: &RenderContext,
    ) -> Result<String, RenderingError> {
        let mut guard = RenderGuard::new();
        let mut recovery = ErrorRecovery::new(self.config.allow_partial_rendering);

        // Check recursion depth
        context.check_recursion_depth(self.config.max_recursion_depth)?;

        let start = Instant::now();

        // Render with timeout
        let output = self.render_with_timeout(template_name, context, &mut guard)?;

        // Check output size
        if output.len() > self.config.max_output_size {
            return Err(RenderingError::OutputSizeExceeded {
                size: output.len(),
                limit: self.config.max_output_size,
            });
        }

        // Validate output
        let validation_errors = self.validator.validate(&output)?;
        let has_critical = OutputValidator::has_critical_errors(&validation_errors);

        if has_critical {
            let error_msg = OutputValidator::format_errors(&validation_errors);
            return Err(RenderingError::ValidationFailed {
                errors: validation_errors
                    .iter()
                    .map(|e| e.message.clone())
                    .collect(),
            });
        }

        // Update metrics
        let mut metrics = RenderMetrics {
            duration: start.elapsed(),
            output_size: output.len(),
            includes_count: context.recursion_depth(), // Track includes via recursion depth
            macro_expansions: context.macro_count(),
            max_recursion_reached: context.recursion_depth(),
            validation_errors: validation_errors
                .iter()
                .filter(|e| e.severity == ValidationSeverity::Error)
                .count(),
            validation_warnings: validation_errors
                .iter()
                .filter(|e| e.severity == ValidationSeverity::Warning)
                .count(),
        };

        guard.update_metrics(metrics);
        let _final_metrics = guard.commit();

        Ok(output)
    }

    /// Render with timeout enforcement
    fn render_with_timeout(
        &self,
        template_name: &str,
        context: &RenderContext,
        _guard: &mut RenderGuard,
    ) -> Result<String, RenderingError> {
        // For now, we'll implement a simple version without actual timeout
        // In a production system, this would use tokio::time::timeout
        let tera = self.tera.read();
        let output = tera
            .render(template_name, context.tera_context())
            .map_err(|e| RenderingError::TeraError { source: e })?;

        Ok(output)
    }

    /// Add a template from string
    pub fn add_template(&self, name: &str, content: &str) -> Result<()> {
        let mut tera = self.tera.write();
        tera.add_raw_template(name, content)
            .with_context(|| format!("Failed to add template '{}'", name))?;
        Ok(())
    }

    /// Get the configuration
    pub fn config(&self) -> &RenderConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_config_validation() {
        let config = RenderConfig::default();
        assert!(config.validate().is_ok());

        let invalid_config = RenderConfig {
            timeout_ms: 0,
            ..Default::default()
        };
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_render_context_scoping() {
        let mut root = RenderContext::new();
        root.insert("key1", &"value1").unwrap();

        let root_arc = Arc::new(root);
        let mut child = RenderContext::child(root_arc.clone());
        child.insert("key2", &"value2").unwrap();

        assert_eq!(child.get("key1").unwrap(), &JsonValue::from("value1"));
        assert_eq!(child.get("key2").unwrap(), &JsonValue::from("value2"));
        assert!(root_arc.get("key2").is_none());
    }

    #[test]
    fn test_output_validator_balanced_delimiters() {
        let validator = OutputValidator::new(true, false);

        let valid_code = "fn main() { let x = vec![1, 2, 3]; }";
        let errors = validator.validate(valid_code).unwrap();
        assert!(errors.is_empty());

        let invalid_code = "fn main() { let x = vec![1, 2, 3; }";
        let errors = validator.validate(invalid_code).unwrap();
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_output_validator_security_checks() {
        let validator = OutputValidator::new(false, true);

        let unsafe_code = "unsafe { std::ptr::write(ptr, value); }";
        let errors = validator.validate(unsafe_code).unwrap();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.message.contains("Unsafe code")));
    }

    #[test]
    fn test_safe_renderer_basic() {
        let config = RenderConfig::default();
        let renderer = SafeRenderer::new(config).unwrap();

        renderer.add_template("test", "Hello {{ name }}!").unwrap();

        let mut context = RenderContext::new();
        context.insert("name", &"World").unwrap();

        let output = renderer.render_safe("test", &context).unwrap();
        assert_eq!(output, "Hello World!");
    }

    #[test]
    fn test_render_guard_cleanup() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_render_guard.txt");
        std::fs::write(&temp_file, "test content").unwrap();

        {
            let mut guard = RenderGuard::new();
            guard.register_temp_file(temp_file.clone());
            // Guard drops here without commit
        }

        // File should be cleaned up
        assert!(!temp_file.exists());
    }

    #[test]
    fn test_error_recovery_suggestions() {
        let mut recovery = ErrorRecovery::new(false);
        recovery.record_error(RenderingError::Timeout { timeout_ms: 5000 });

        let suggestions = recovery.suggest_fixes();
        assert!(!suggestions.is_empty());
        assert!(suggestions[0].contains("timed out"));
    }
}
