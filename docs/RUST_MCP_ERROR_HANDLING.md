# Rust MCP Error Handling Best Practices for ggen-mcp

## Table of Contents

1. [Error Type Strategy](#error-type-strategy)
2. [Error Context](#error-context)
3. [MCP Error Reporting](#mcp-error-reporting)
4. [Recovery Patterns](#recovery-patterns)
5. [Error Testing](#error-testing)
6. [Performance Considerations](#performance-considerations)
7. [TPS Jidoka Principles](#tps-jidoka-principles)
8. [Real-World Examples](#real-world-examples)

---

## Error Type Strategy

### When to Use `anyhow::Result`

Use `anyhow::Result<T>` for **application-level errors** where:
- The error is propagated through multiple layers
- You need rich error context and backtraces
- The exact error type doesn't matter to the caller
- You're building error chains with `.context()`

**Current Usage in ggen-mcp:**
```rust
// File I/O operations
pub async fn load_workbook(&self, path: &Path) -> Result<Workbook> {
    tokio::fs::read(path)
        .await
        .context("Failed to read workbook file")?;
    // ...
}

// Complex operations with multiple failure points
pub async fn process_sparql_query(&self, query: &str) -> Result<QueryResults> {
    let parsed = self.parse_query(query)
        .context("Failed to parse SPARQL query")?;

    let validated = self.validate_query(&parsed)
        .context("Query validation failed")?;

    self.execute_query(validated)
        .await
        .context("Query execution failed")
}
```

### When to Use `thiserror` Custom Errors

Use `thiserror::Error` for **domain-specific errors** where:
- The error type carries semantic meaning
- Callers need to match on specific error variants
- You want structured error information
- The error represents a well-defined failure mode

**Current Usage in ggen-mcp:**

```rust
use thiserror::Error;

/// Template parameter validation errors
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ValidationError {
    #[error("missing required parameter: {0}")]
    MissingRequired(String),

    #[error("type mismatch for parameter '{name}': expected {expected}, got {actual}")]
    TypeMismatch {
        name: String,
        expected: String,
        actual: String,
    },

    #[error("validation rule failed for parameter '{name}': {message}")]
    RuleFailed { name: String, message: String },

    #[error("template not found: {0}")]
    TemplateNotFound(String),
}

/// Graph integrity errors
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum IntegrityError {
    #[error("Dangling reference: {0} references {1} which does not exist")]
    DanglingReference(String, String),

    #[error("Missing required property: {0} missing {1}")]
    MissingProperty(String, String),

    #[error("Type inconsistency: {0} has incompatible types {1} and {2}")]
    TypeInconsistency(String, String, String),

    #[error("Circular reference detected: {0}")]
    CircularReference(String),
}
```

### Error Hierarchies for MCP Tools

**Best Practice:** Create a hierarchy of errors from generic to specific:

```rust
use thiserror::Error;

/// Top-level MCP server errors
#[derive(Debug, Error)]
pub enum McpServerError {
    #[error("Tool error: {0}")]
    Tool(#[from] ToolError),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Tool-specific errors
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("tool '{tool_name}' is disabled")]
    ToolDisabled { tool_name: String },

    #[error("tool '{tool_name}' timed out after {timeout_ms}ms")]
    Timeout { tool_name: String, timeout_ms: u64 },

    #[error("response too large: {size} bytes > {limit} bytes")]
    ResponseTooLarge { size: usize, limit: usize },

    #[error("invalid parameters: {0}")]
    InvalidParams(String),
}

/// Validation errors
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("schema validation failed: {0}")]
    Schema(#[from] SchemaValidationError),

    #[error("SPARQL validation failed: {0}")]
    Sparql(#[from] SparqlValidationError),

    #[error("template validation failed: {0}")]
    Template(#[from] TemplateValidationError),
}
```

### Domain-Specific Error Types

Create focused error types for each domain:

```rust
/// SPARQL query validation errors
#[derive(Debug, Error, Clone, PartialEq)]
pub enum SparqlValidationError {
    #[error("Expected variable '{0}' not found in results")]
    MissingVariable(String),

    #[error("Variable '{0}' has unexpected type. Expected {1}, got {2}")]
    TypeMismatch(String, String, String),

    #[error("Cardinality constraint violated for '{0}': {1}")]
    CardinalityViolation(String, String),

    #[error("Unbound value for required variable '{0}'")]
    UnboundRequired(String),
}

/// Schema validation errors
#[derive(Debug, Error)]
pub enum SchemaValidationError {
    #[error("Schema validation failed for tool '{tool}': {errors}")]
    ValidationFailed {
        tool: String,
        errors: Vec<String>,
    },

    #[error("Missing required field '{field}' in tool '{tool}'")]
    MissingRequiredField {
        tool: String,
        field: String,
    },

    #[error("Invalid type for field '{field}' in tool '{tool}': expected {expected}, got {actual}")]
    InvalidType {
        tool: String,
        field: String,
        expected: String,
        actual: String,
    },
}
```

---

## Error Context

### Using `.context()` and `.with_context()`

**Static context** with `.context()`:
```rust
use anyhow::{Context, Result};

pub fn load_config(path: &Path) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .context("Failed to read configuration file")?;

    serde_json::from_str(&content)
        .context("Failed to parse configuration JSON")
}
```

**Dynamic context** with `.with_context()`:
```rust
pub fn process_workbook(workbook_id: &str) -> Result<ProcessedWorkbook> {
    let workbook = self.get_workbook(workbook_id)
        .with_context(|| format!("Failed to load workbook '{}'", workbook_id))?;

    let sheets = workbook.sheets()
        .with_context(|| format!("Failed to enumerate sheets in workbook '{}'", workbook_id))?;

    for sheet in sheets {
        process_sheet(&sheet)
            .with_context(|| format!(
                "Failed to process sheet '{}' in workbook '{}'",
                sheet.name(),
                workbook_id
            ))?;
    }

    Ok(ProcessedWorkbook { /* ... */ })
}
```

### Building Informative Error Messages

**Good error messages** are:
- **Specific**: Include relevant identifiers (IDs, names, paths)
- **Actionable**: Suggest what went wrong and how to fix it
- **Contextual**: Provide the full context chain

```rust
// ❌ BAD
Err(anyhow!("Invalid value"))

// ✅ GOOD
Err(anyhow!(
    "Invalid value for parameter 'max_rows': expected positive integer, got '{}'",
    value
))

// ❌ BAD
.context("Error")?

// ✅ GOOD
.with_context(|| format!(
    "Failed to validate SPARQL query for template '{}': missing required variable 'subject'",
    template_name
))?
```

### Preserving Error Chains

Always preserve the full error chain for debugging:

```rust
use anyhow::{Context, Result};

pub async fn execute_pipeline(config: &PipelineConfig) -> Result<PipelineResult> {
    // Each step adds context while preserving the underlying error
    let ontology = load_ontology(&config.ontology_path)
        .context("Failed to load ontology")?;

    let template = load_template(&config.template_path)
        .context("Failed to load template")?;

    let validated = validate_template(&template, &ontology)
        .context("Template validation failed")?;

    generate_code(&validated)
        .await
        .context("Code generation failed")
}
```

### Backtrace Configuration

Enable backtraces for development:

```bash
# In development
export RUST_BACKTRACE=1

# For detailed backtraces
export RUST_BACKTRACE=full

# In production (disabled by default for performance)
# No environment variable needed
```

Configure backtrace capture in code:

```rust
use anyhow::{Context, Result};

pub fn main() -> Result<()> {
    // Backtraces are captured automatically when RUST_BACKTRACE is set
    // They're included in the error chain

    risky_operation()
        .context("Operation failed")?;

    Ok(())
}

// When an error occurs, the backtrace is preserved:
// Error: Operation failed
//
// Caused by:
//     Underlying error message
//
// Stack backtrace:
//     0: std::backtrace::Backtrace::create
//     1: anyhow::error::<impl anyhow::Error>::msg
//     ...
```

---

## MCP Error Reporting

### Mapping Rust Errors to MCP Error Codes

MCP defines several error codes. Map your errors appropriately:

```rust
use rmcp::ErrorData as McpError;
use anyhow::Error;

/// Convert anyhow errors to MCP errors
pub fn to_mcp_error(error: Error) -> McpError {
    // Check for specific error types
    if let Some(tool_error) = error.downcast_ref::<ToolDisabledError>() {
        return McpError::invalid_request(
            format!("Tool '{}' is disabled", tool_error.tool_name),
            None
        );
    }

    if let Some(validation_error) = error.downcast_ref::<ValidationError>() {
        return McpError::invalid_params(
            validation_error.to_string(),
            None
        );
    }

    if let Some(timeout_error) = error.downcast_ref::<TimeoutError>() {
        return McpError::internal_error(
            format!("Operation timed out: {}", timeout_error),
            None
        );
    }

    if error.to_string().contains("not found") {
        return McpError::invalid_params(
            error.to_string(),
            None
        );
    }

    // Default to internal error for unexpected errors
    McpError::internal_error(error.to_string(), None)
}
```

### User-Friendly Error Messages

Transform technical errors into user-friendly messages:

```rust
pub fn format_user_error(error: &Error) -> String {
    if let Some(validation_error) = error.downcast_ref::<ValidationError>() {
        match validation_error {
            ValidationError::MissingRequired(param) => {
                format!("Missing required parameter: '{}'. Please provide this parameter.", param)
            }
            ValidationError::TypeMismatch { name, expected, actual } => {
                format!(
                    "Parameter '{}' has the wrong type. Expected {}, but got {}.",
                    name, expected, actual
                )
            }
            ValidationError::TemplateNotFound(name) => {
                format!("Template '{}' not found. Please check the template name.", name)
            }
            _ => validation_error.to_string(),
        }
    } else {
        // Provide a generic user-friendly message
        format!("An error occurred: {}", error)
    }
}
```

### Error Severity Levels

Classify errors by severity:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,      // Informational, no action needed
    Warning,   // Potential issue, operation succeeded
    Error,     // Operation failed, but system is stable
    Critical,  // System integrity compromised
}

pub struct ErrorWithSeverity {
    pub severity: Severity,
    pub error: Error,
    pub context: String,
}

impl ErrorWithSeverity {
    pub fn to_mcp_error(&self) -> McpError {
        match self.severity {
            Severity::Info | Severity::Warning => {
                // Return as successful result with warning
                McpError::internal_error(
                    format!("[{}] {}", self.severity_label(), self.error),
                    None
                )
            }
            Severity::Error => {
                McpError::invalid_request(self.error.to_string(), None)
            }
            Severity::Critical => {
                McpError::internal_error(
                    format!("CRITICAL: {}", self.error),
                    None
                )
            }
        }
    }

    fn severity_label(&self) -> &str {
        match self.severity {
            Severity::Info => "INFO",
            Severity::Warning => "WARNING",
            Severity::Error => "ERROR",
            Severity::Critical => "CRITICAL",
        }
    }
}
```

### Actionable Error Suggestions

Provide suggestions for fixing errors:

```rust
pub struct ActionableError {
    pub error: String,
    pub context: String,
    pub suggestion: Option<String>,
}

impl ActionableError {
    pub fn new(error: impl Into<String>, context: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            context: context.into(),
            suggestion: None,
        }
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn to_mcp_error(&self) -> McpError {
        let message = if let Some(suggestion) = &self.suggestion {
            format!(
                "{}\n\nContext: {}\nSuggestion: {}",
                self.error, self.context, suggestion
            )
        } else {
            format!("{}\n\nContext: {}", self.error, self.context)
        };

        McpError::invalid_request(message, None)
    }
}

// Usage example
fn validate_query(query: &str) -> Result<(), ActionableError> {
    if query.is_empty() {
        return Err(ActionableError::new(
            "Empty SPARQL query",
            "Query validation"
        ).with_suggestion(
            "Provide a valid SPARQL SELECT, CONSTRUCT, or ASK query"
        ));
    }

    Ok(())
}
```

---

## Recovery Patterns

### Retry Logic with Backoff

Implement exponential backoff for transient failures:

```rust
use std::time::Duration;
use anyhow::Result;

pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
    pub jitter: bool,
}

impl RetryConfig {
    pub fn recalc() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

pub trait RetryPolicy {
    fn should_retry(&self, attempt: u32, error: &anyhow::Error) -> bool;
    fn delay(&self, attempt: u32) -> Duration;
}

pub struct ExponentialBackoff {
    config: RetryConfig,
}

impl RetryPolicy for ExponentialBackoff {
    fn should_retry(&self, attempt: u32, error: &anyhow::Error) -> bool {
        if attempt >= self.config.max_attempts {
            return false;
        }

        let error_msg = error.to_string().to_lowercase();

        // Don't retry fatal errors
        if error_msg.contains("permission denied")
            || error_msg.contains("not supported")
            || error_msg.contains("invalid argument")
        {
            return false;
        }

        // Retry transient errors
        error_msg.contains("timeout")
            || error_msg.contains("unavailable")
            || error_msg.contains("busy")
            || error_msg.contains("locked")
    }

    fn delay(&self, attempt: u32) -> Duration {
        let base_delay = self.config.initial_delay.as_millis() as f64;
        let exponential_delay = base_delay * self.config.backoff_multiplier.powi(attempt as i32);

        let mut delay = Duration::from_millis(exponential_delay as u64);

        if delay > self.config.max_delay {
            delay = self.config.max_delay;
        }

        if self.config.jitter {
            // Add up to 25% jitter to prevent thundering herd
            let jitter = (delay.as_millis() as f64 * 0.25 * rand::random::<f64>()) as u64;
            delay += Duration::from_millis(jitter);
        }

        delay
    }
}

pub async fn retry_async_with_policy<T, F, Fut>(
    operation: F,
    policy: &dyn RetryPolicy,
    operation_name: &str,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempt = 0;

    loop {
        attempt += 1;

        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    tracing::debug!(
                        operation = operation_name,
                        attempt = attempt,
                        "operation succeeded after retry"
                    );
                }
                return Ok(result);
            }
            Err(err) => {
                if policy.should_retry(attempt, &err) {
                    let delay = policy.delay(attempt);
                    tracing::warn!(
                        operation = operation_name,
                        attempt = attempt,
                        delay_ms = delay.as_millis(),
                        error = %err,
                        "retrying operation after delay"
                    );
                    tokio::time::sleep(delay).await;
                } else {
                    return Err(err);
                }
            }
        }
    }
}
```

### Fallback Strategies

Implement graceful degradation:

```rust
use anyhow::Result;

pub struct FallbackChain<T> {
    strategies: Vec<Box<dyn Fn() -> Result<T>>>,
    operation_name: String,
}

impl<T> FallbackChain<T> {
    pub fn new(operation_name: impl Into<String>) -> Self {
        Self {
            strategies: Vec::new(),
            operation_name: operation_name.into(),
        }
    }

    pub fn try_with<F>(mut self, strategy: F) -> Self
    where
        F: Fn() -> Result<T> + 'static,
    {
        self.strategies.push(Box::new(strategy));
        self
    }

    pub fn execute(self) -> Result<T> {
        let mut last_error = None;

        for (idx, strategy) in self.strategies.iter().enumerate() {
            match strategy() {
                Ok(result) => {
                    if idx > 0 {
                        tracing::warn!(
                            operation = self.operation_name,
                            fallback_index = idx,
                            "primary strategy failed, succeeded with fallback"
                        );
                    }
                    return Ok(result);
                }
                Err(err) => {
                    tracing::debug!(
                        operation = self.operation_name,
                        strategy_index = idx,
                        error = %err,
                        "strategy failed, trying next fallback"
                    );
                    last_error = Some(err);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("No fallback strategies available")))
    }
}

// Usage
pub fn load_template(name: &str) -> Result<Template> {
    FallbackChain::new("load_template")
        .try_with(|| load_from_cache(name))
        .try_with(|| load_from_filesystem(name))
        .try_with(|| load_from_embedded_resources(name))
        .try_with(|| load_default_template())
        .execute()
}
```

### Partial Success Handling

Handle batch operations with partial failures:

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult<T> {
    pub succeeded: Vec<T>,
    pub failed: Vec<BatchFailure>,
    pub total: usize,
    pub summary: BatchSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchFailure {
    pub index: usize,
    pub item_id: String,
    pub error: String,
    pub is_fatal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSummary {
    pub success_count: usize,
    pub failure_count: usize,
    pub success_rate: f64,
    pub completed: bool,
}

pub struct PartialSuccessHandler {
    pub fail_fast: bool,
    pub max_errors: Option<usize>,
}

impl PartialSuccessHandler {
    pub async fn process_batch<T, I, F, Fut>(
        &self,
        items: Vec<I>,
        mut processor: F,
    ) -> BatchResult<T>
    where
        F: FnMut(usize, I) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let total = items.len();
        let mut result = BatchResult::new();

        for (index, item) in items.into_iter().enumerate() {
            if self.should_stop_processing(&result) {
                return result.finalize(total, false);
            }

            match processor(index, item).await {
                Ok(processed) => {
                    result.add_success(processed);
                }
                Err(err) => {
                    let is_fatal = self.is_fatal_error(&err);
                    result.add_failure(
                        index,
                        format!("item_{}", index),
                        err.to_string(),
                        is_fatal,
                    );

                    if is_fatal || self.fail_fast {
                        return result.finalize(total, false);
                    }
                }
            }
        }

        result.finalize(total, true)
    }

    fn should_stop_processing<T>(&self, result: &BatchResult<T>) -> bool {
        if let Some(max_errors) = self.max_errors {
            result.summary.failure_count >= max_errors
        } else {
            false
        }
    }

    fn is_fatal_error(&self, error: &Error) -> bool {
        let error_msg = error.to_string().to_lowercase();
        error_msg.contains("corrupted")
            || error_msg.contains("permission denied")
            || error_msg.contains("disk full")
    }
}
```

### Circuit Breaker Integration

Protect against cascading failures:

```rust
use std::sync::Arc;
use parking_lot::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerState {
    Closed,    // Normal operation
    Open,      // Failing fast
    HalfOpen,  // Testing recovery
}

pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    inner: Arc<Mutex<CircuitBreakerInner>>,
    name: String,
}

pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout: Duration,
    pub failure_window: Duration,
}

impl CircuitBreaker {
    pub async fn execute_async<T, F, Fut>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        // Check if circuit allows execution
        {
            let mut inner = self.inner.lock();
            match inner.state {
                CircuitBreakerState::Open => {
                    if inner.state_changed_at.elapsed() >= self.config.timeout {
                        inner.state = CircuitBreakerState::HalfOpen;
                        inner.state_changed_at = Instant::now();
                    } else {
                        return Err(anyhow!(
                            "circuit breaker '{}' is open (failing fast)",
                            self.name
                        ));
                    }
                }
                _ => {}
            }
        }

        // Execute operation
        match operation().await {
            Ok(result) => {
                self.on_success();
                Ok(result)
            }
            Err(err) => {
                self.on_failure();
                Err(err)
            }
        }
    }
}
```

---

## Error Testing

### Testing Error Paths

Test both success and failure scenarios:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    #[test]
    fn test_validation_error_missing_required() {
        let error = ValidationError::MissingRequired("name".to_string());
        assert_eq!(
            error.to_string(),
            "missing required parameter: name"
        );
    }

    #[test]
    fn test_retry_policy_transient_error() {
        let policy = ExponentialBackoff::default();
        let error = anyhow!("operation timed out");

        assert!(policy.should_retry(1, &error));
    }

    #[test]
    fn test_retry_policy_fatal_error() {
        let policy = ExponentialBackoff::default();
        let error = anyhow!("permission denied");

        assert!(!policy.should_retry(1, &error));
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::from_millis(100),
            failure_window: Duration::from_secs(60),
        };

        let cb = CircuitBreaker::new("test", config);

        // Trigger failures
        for _ in 0..3 {
            let _ = cb.execute_async(|| async {
                Err::<(), _>(anyhow!("error"))
            }).await;
        }

        assert_eq!(cb.state(), CircuitBreakerState::Open);
    }
}
```

### Error Scenario Coverage

Test comprehensive error scenarios:

```rust
#[cfg(test)]
mod error_scenario_tests {
    use super::*;

    #[test]
    fn test_all_validation_error_variants() {
        let scenarios = vec![
            ValidationError::MissingRequired("param".into()),
            ValidationError::TypeMismatch {
                name: "field".into(),
                expected: "string".into(),
                actual: "number".into(),
            },
            ValidationError::RuleFailed {
                name: "value".into(),
                message: "too short".into(),
            },
            ValidationError::TemplateNotFound("template.tera".into()),
        ];

        for error in scenarios {
            // Ensure all variants format correctly
            let msg = error.to_string();
            assert!(!msg.is_empty());

            // Ensure they can be converted to MCP errors
            let mcp_error = validation_error_to_mcp(&error);
            assert!(!mcp_error.message.is_empty());
        }
    }

    #[tokio::test]
    async fn test_partial_success_all_scenarios() {
        let handler = PartialSuccessHandler::new();

        // All succeed
        let result = handler.process_batch(
            vec![1, 2, 3],
            |_idx, item| async move { Ok(item * 2) }
        ).await;
        assert!(result.is_complete_success());

        // All fail
        let result = handler.process_batch(
            vec![1, 2, 3],
            |_idx, _item| async move { Err::<i32, _>(anyhow!("error")) }
        ).await;
        assert!(result.is_complete_failure());

        // Partial success
        let result = handler.process_batch(
            vec![1, 2, 3],
            |_idx, item| async move {
                if item == 2 {
                    Err(anyhow!("error"))
                } else {
                    Ok(item * 2)
                }
            }
        ).await;
        assert!(result.is_partial_success());
    }
}
```

### Error Message Validation

Validate error message quality:

```rust
#[cfg(test)]
mod message_quality_tests {
    use super::*;

    #[test]
    fn test_error_messages_are_informative() {
        let error = ValidationError::TypeMismatch {
            name: "max_rows".into(),
            expected: "positive integer".into(),
            actual: "string".into(),
        };

        let msg = error.to_string();

        // Message should contain parameter name
        assert!(msg.contains("max_rows"));

        // Message should contain expected type
        assert!(msg.contains("positive integer"));

        // Message should contain actual type
        assert!(msg.contains("string"));

        // Message should be readable
        assert!(msg.len() > 20);
        assert!(msg.len() < 200);
    }

    #[test]
    fn test_error_context_preservation() {
        let result: Result<()> = Err(anyhow!("root cause"))
            .context("step 1 failed")
            .context("step 2 failed")
            .context("operation failed");

        let error = result.unwrap_err();
        let full_error = format!("{:#}", error);

        // Should contain all context layers
        assert!(full_error.contains("operation failed"));
        assert!(full_error.contains("step 2 failed"));
        assert!(full_error.contains("step 1 failed"));
        assert!(full_error.contains("root cause"));
    }
}
```

### Error Propagation Tests

Test error propagation through layers:

```rust
#[cfg(test)]
mod propagation_tests {
    use super::*;

    fn inner_function() -> Result<()> {
        Err(anyhow!("inner error"))
    }

    fn middle_function() -> Result<()> {
        inner_function()
            .context("middle layer failed")
    }

    fn outer_function() -> Result<()> {
        middle_function()
            .context("outer layer failed")
    }

    #[test]
    fn test_error_chain_preservation() {
        let result = outer_function();
        assert!(result.is_err());

        let error = result.unwrap_err();
        let error_chain = format!("{:#}", error);

        // Verify all layers are present
        assert!(error_chain.contains("outer layer failed"));
        assert!(error_chain.contains("middle layer failed"));
        assert!(error_chain.contains("inner error"));
    }
}
```

---

## Performance Considerations

### Zero-Cost Error Handling

Rust's error handling is zero-cost in the success path:

```rust
// No heap allocation in success case
pub fn parse_number(s: &str) -> Result<i64> {
    s.parse::<i64>()
        .context("Failed to parse number")  // Only allocates on error
}

// Use Result<T, E> instead of Option + separate error
pub fn find_item(id: &str) -> Result<Item> {
    self.items.get(id)
        .cloned()
        .ok_or_else(|| anyhow!("Item '{}' not found", id))  // Lazy error creation
}
```

### Avoiding Allocations in Hot Paths

Minimize allocations in performance-critical code:

```rust
use std::borrow::Cow;

// ❌ BAD: Allocates on every call
pub fn validate_fast(value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(anyhow!("Value cannot be empty"));  // String allocation
    }
    Ok(())
}

// ✅ GOOD: Use static strings for common errors
pub fn validate_fast(value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(anyhow!("Value cannot be empty"));  // Still allocates, but unavoidable
    }
    Ok(())
}

// ✅ BETTER: Use custom error types for hot paths
#[derive(Debug, Error)]
pub enum FastValidationError {
    #[error("Value cannot be empty")]
    EmptyValue,

    #[error("Value too long")]
    TooLong,
}

pub fn validate_very_fast(value: &str) -> Result<(), FastValidationError> {
    if value.is_empty() {
        return Err(FastValidationError::EmptyValue);  // No allocation
    }
    if value.len() > 1000 {
        return Err(FastValidationError::TooLong);  // No allocation
    }
    Ok(())
}
```

### Error Caching Patterns

Cache frequently used error messages:

```rust
use once_cell::sync::Lazy;
use std::collections::HashMap;

static ERROR_TEMPLATES: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert("missing_param", "Missing required parameter: {}");
    map.insert("invalid_type", "Invalid type for {}: expected {}, got {}");
    map.insert("not_found", "Resource '{}' not found");
    map
});

pub fn cached_error(template_key: &str, args: &[&str]) -> Error {
    let template = ERROR_TEMPLATES.get(template_key)
        .unwrap_or(&"Unknown error");

    match args.len() {
        1 => anyhow!(template, args[0]),
        2 => anyhow!(template, args[0], args[1]),
        3 => anyhow!(template, args[0], args[1], args[2]),
        _ => anyhow!("Error formatting failed"),
    }
}
```

---

## TPS Jidoka Principles

### Automatic Error Detection (Jidoka)

Jidoka (自働化) - "automation with a human touch" - means building quality into the process by detecting errors immediately.

#### 1. Compile-Time Error Prevention

Use the type system to prevent errors:

```rust
// ❌ BAD: Stringly-typed errors
pub fn process_sheet(sheet_name: &str) -> Result<()> {
    // Runtime validation needed
    if sheet_name.is_empty() {
        return Err(anyhow!("Invalid sheet name"));
    }
    // ...
}

// ✅ GOOD: Type-safe wrappers
#[derive(Debug, Clone)]
pub struct SheetName(String);

impl SheetName {
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        if name.is_empty() {
            return Err(anyhow!("Sheet name cannot be empty"));
        }
        if name.len() > 255 {
            return Err(anyhow!("Sheet name too long (max 255 characters)"));
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// Now invalid sheet names are impossible
pub fn process_sheet(sheet_name: &SheetName) -> Result<()> {
    // No validation needed - type guarantees validity
    // ...
}
```

#### 2. Poka-Yoke (Error-Proofing) in API Design

Design APIs that make errors impossible:

```rust
// ❌ BAD: Easy to misuse
pub fn execute_query(
    query: &str,
    validate: bool,
    check_injection: bool,
    cache: bool,
) -> Result<QueryResults> {
    // Caller might forget to set validate=true
}

// ✅ GOOD: Safe by default, opt-in to unsafe
pub struct QueryExecutor {
    validator: QueryValidator,
    cache: QueryCache,
}

impl QueryExecutor {
    pub fn execute_safe(&self, query: &ValidatedQuery) -> Result<QueryResults> {
        // Query is guaranteed to be validated
        self.execute_internal(query.as_str())
    }

    pub fn execute_unsafe(&self, query: &str) -> Result<QueryResults> {
        // Explicitly marked as unsafe operation
        tracing::warn!("Executing unvalidated query");
        self.execute_internal(query)
    }
}

pub struct ValidatedQuery {
    query: String,
    hash: u64,
}

impl ValidatedQuery {
    pub fn new(query: String, validator: &QueryValidator) -> Result<Self> {
        // Validation happens at construction
        validator.validate(&query)?;
        validator.check_injection(&query)?;

        let hash = calculate_hash(&query);
        Ok(Self { query, hash })
    }

    pub fn as_str(&self) -> &str {
        &self.query
    }
}
```

#### 3. Fail-Fast Validation

Detect errors as early as possible:

```rust
pub struct TemplateContext {
    template_name: String,
    context: HashMap<String, JsonValue>,
    validated: bool,
}

impl TemplateContext {
    pub fn new(template_name: impl Into<String>) -> Self {
        Self {
            template_name: template_name.into(),
            context: HashMap::new(),
            validated: false,
        }
    }

    // Validate immediately after construction
    pub fn build(mut self, validator: &TemplateValidator) -> Result<ValidatedContext> {
        validator.validate(&self)?;
        Ok(ValidatedContext {
            template_name: self.template_name,
            context: self.context,
        })
    }
}

pub struct ValidatedContext {
    template_name: String,
    context: HashMap<String, JsonValue>,
}

impl ValidatedContext {
    // Can only render with validated context
    pub fn render(&self, registry: &TemplateRegistry) -> Result<String> {
        registry.render_internal(&self.template_name, &self.context)
    }
}
```

#### 4. Andon Cord - Stop and Alert

When an error is detected, stop immediately and alert:

```rust
pub struct AndonSystem {
    alerts: Arc<Mutex<Vec<Alert>>>,
    should_stop: Arc<AtomicBool>,
}

pub struct Alert {
    severity: Severity,
    component: String,
    message: String,
    timestamp: Instant,
}

impl AndonSystem {
    pub fn pull_cord(&self, severity: Severity, component: &str, message: &str) {
        let alert = Alert {
            severity,
            component: component.to_string(),
            message: message.to_string(),
            timestamp: Instant::now(),
        };

        self.alerts.lock().unwrap().push(alert.clone());

        if severity >= Severity::Error {
            // Stop processing
            self.should_stop.store(true, Ordering::SeqCst);

            // Log to monitoring system
            tracing::error!(
                severity = ?severity,
                component = component,
                message = message,
                "ANDON: System halted due to error"
            );

            // Trigger alerts
            self.notify_operators(&alert);
        }
    }

    pub fn should_stop(&self) -> bool {
        self.should_stop.load(Ordering::SeqCst)
    }

    pub fn reset(&self) {
        self.should_stop.store(false, Ordering::SeqCst);
        self.alerts.lock().unwrap().clear();
    }
}

// Usage in MCP tool
pub async fn execute_tool(&self, tool_name: &str) -> Result<ToolResult> {
    if self.andon.should_stop() {
        return Err(anyhow!("System halted by Andon - check alerts"));
    }

    let result = self.execute_tool_internal(tool_name).await;

    if let Err(error) = &result {
        let severity = classify_error(error);
        self.andon.pull_cord(severity, tool_name, &error.to_string());
    }

    result
}
```

#### 5. Root Cause Analysis (5 Whys)

Build error messages that support root cause analysis:

```rust
pub struct DetailedError {
    pub error: String,
    pub component: String,
    pub operation: String,
    pub input: Option<String>,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub suggestion: Option<String>,
    pub related_errors: Vec<String>,
}

impl DetailedError {
    pub fn to_error_report(&self) -> String {
        format!(
            r#"
ERROR REPORT
============
Component: {}
Operation: {}
Error: {}

{}

{}

{}

{}
"#,
            self.component,
            self.operation,
            self.error,
            self.input.as_ref().map(|i| format!("Input: {}", i)).unwrap_or_default(),
            self.expected.as_ref().map(|e| format!("Expected: {}", e)).unwrap_or_default(),
            self.actual.as_ref().map(|a| format!("Actual: {}", a)).unwrap_or_default(),
            self.suggestion.as_ref().map(|s| format!("Suggestion: {}", s)).unwrap_or_default(),
        )
    }
}
```

---

## Real-World Examples

### Example 1: SPARQL Query Execution with Full Error Handling

```rust
use anyhow::{Context, Result};
use oxigraph::sparql::QueryResults;

pub async fn execute_sparql_query(
    &self,
    query: &str,
    template_name: &str,
) -> Result<QueryResults> {
    // 1. Validate query syntax
    let parsed_query = self.parse_query(query)
        .with_context(|| format!(
            "Failed to parse SPARQL query for template '{}'",
            template_name
        ))?;

    // 2. Check for injection attempts (Poka-Yoke)
    self.injection_checker.check(&parsed_query)
        .with_context(|| format!(
            "SPARQL injection detected in template '{}'",
            template_name
        ))?;

    // 3. Validate against expected schema
    self.result_validator.validate_query(&parsed_query)
        .with_context(|| format!(
            "Query validation failed for template '{}'",
            template_name
        ))?;

    // 4. Execute with timeout and retry
    let policy = ExponentialBackoff::new(RetryConfig::default());
    let results = retry_async_with_policy(
        || self.store.query(parsed_query.clone()),
        &policy,
        "execute_sparql_query"
    )
    .await
    .with_context(|| format!(
        "Failed to execute SPARQL query for template '{}'",
        template_name
    ))?;

    // 5. Validate results
    self.result_validator.validate_results(&results)
        .with_context(|| format!(
            "Query results validation failed for template '{}'",
            template_name
        ))?;

    Ok(results)
}
```

### Example 2: MCP Tool with Complete Error Handling

```rust
use rmcp::{Json, ErrorData as McpError};

#[tool(name = "generate_code", description = "Generate code from template")]
pub async fn generate_code(
    &self,
    Parameters(params): Parameters<GenerateCodeParams>,
) -> Result<Json<GenerateCodeResponse>, McpError> {
    // 1. Check if tool is enabled
    self.ensure_tool_enabled("generate_code")
        .map_err(to_mcp_error)?;

    // 2. Validate parameters
    let validated_params = self.validator
        .validate_and_deserialize::<GenerateCodeParams>("generate_code", params)
        .map_err(|e| McpError::invalid_params(
            format!("Parameter validation failed: {}", e),
            None
        ))?;

    // 3. Execute with timeout and proper error handling
    let result = self.run_tool_with_timeout(
        "generate_code",
        async {
            // Load ontology
            let ontology = self.ontology_manager
                .load_ontology(&validated_params.ontology_path)
                .await
                .context("Failed to load ontology")?;

            // Validate ontology integrity
            let integrity_report = self.integrity_checker
                .check(&ontology)
                .context("Ontology integrity check failed")?;

            if !integrity_report.is_valid() {
                return Err(anyhow!(
                    "Ontology validation failed: {}",
                    integrity_report.summary()
                ));
            }

            // Load and validate template
            let template = self.template_registry
                .load_template(&validated_params.template_name)
                .await
                .with_context(|| format!(
                    "Failed to load template '{}'",
                    validated_params.template_name
                ))?;

            // Build context
            let context = self.build_template_context(&ontology, &validated_params)
                .context("Failed to build template context")?;

            // Render with error recovery
            let code = FallbackChain::new("render_template")
                .try_with(|| template.render(&context))
                .try_with(|| self.render_with_defaults(&template, &context))
                .execute()
                .context("Template rendering failed")?;

            // Validate generated code
            self.code_validator
                .validate(&code)
                .context("Generated code validation failed")?;

            Ok(GenerateCodeResponse {
                code,
                template_name: validated_params.template_name,
                warnings: integrity_report.warnings(),
            })
        }
    )
    .await
    .map_err(|e| {
        // Convert to appropriate MCP error
        if e.to_string().contains("not found") {
            McpError::invalid_params(e.to_string(), None)
        } else if e.to_string().contains("validation failed") {
            McpError::invalid_request(e.to_string(), None)
        } else {
            McpError::internal_error(e.to_string(), None)
        }
    })?;

    Ok(Json(result))
}
```

### Example 3: Batch Operation with Partial Success

```rust
pub async fn process_batch_edits(
    &self,
    edits: Vec<CellEdit>,
) -> Result<BatchResult<ProcessedEdit>> {
    let handler = PartialSuccessHandler::new()
        .max_errors(Some(10));

    let result = handler.process_batch_async(
        edits,
        |index, edit| async move {
            // Validate edit
            self.validate_edit(&edit)
                .with_context(|| format!(
                    "Validation failed for edit at index {}",
                    index
                ))?;

            // Apply edit with retry
            let policy = ExponentialBackoff::new(RetryConfig::file_io());
            let processed = retry_async_with_policy(
                || self.apply_edit(&edit),
                &policy,
                "apply_edit"
            )
            .await
            .with_context(|| format!(
                "Failed to apply edit at index {}",
                index
            ))?;

            Ok(processed)
        }
    ).await;

    // Log results
    tracing::info!(
        succeeded = result.summary.success_count,
        failed = result.summary.failure_count,
        rate = result.summary.success_rate,
        "Batch processing complete"
    );

    Ok(result)
}
```

---

## Summary

### Key Takeaways

1. **Use the right error type for the job:**
   - `anyhow::Result` for application errors
   - `thiserror::Error` for domain errors

2. **Always add context:**
   - Use `.context()` for static messages
   - Use `.with_context()` for dynamic messages

3. **Map errors appropriately:**
   - Convert to MCP error codes
   - Provide user-friendly messages
   - Include actionable suggestions

4. **Implement recovery patterns:**
   - Retry with exponential backoff
   - Circuit breakers for cascade prevention
   - Partial success for batch operations

5. **Test thoroughly:**
   - Test both success and error paths
   - Validate error messages
   - Test error propagation

6. **Apply Jidoka principles:**
   - Detect errors early (compile-time when possible)
   - Use type-safe APIs (Poka-Yoke)
   - Fail fast with clear signals
   - Stop on critical errors (Andon)

7. **Optimize for performance:**
   - Use zero-cost abstractions
   - Avoid allocations in hot paths
   - Cache common error messages

---

## Additional Resources

- [anyhow documentation](https://docs.rs/anyhow)
- [thiserror documentation](https://docs.rs/thiserror)
- [Rust Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [MCP Protocol Specification](https://spec.modelcontextprotocol.io/)
- [Toyota Production System (TPS)](https://en.wikipedia.org/wiki/Toyota_Production_System)

---

**Document Version:** 1.0
**Last Updated:** 2026-01-20
**Author:** ggen-mcp development team
