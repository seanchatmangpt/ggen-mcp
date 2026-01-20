//! Error Handling Patterns for MCP Servers
//!
//! This example demonstrates comprehensive error handling patterns
//! specific to MCP servers, including:
//! - Custom error types with thiserror
//! - Error context with anyhow
//! - MCP error mapping
//! - Retry logic and recovery
//! - Circuit breakers
//! - Partial success handling
//! - Validation error reporting

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use thiserror::Error;

// =============================================================================
// CUSTOM ERROR TYPES
// =============================================================================

/// Domain-specific errors for MCP tools
#[derive(Debug, Error)]
pub enum McpToolError {
    #[error("Tool '{tool_name}' is disabled by configuration")]
    ToolDisabled { tool_name: String },

    #[error("Tool '{tool_name}' timed out after {timeout_ms}ms")]
    Timeout {
        tool_name: String,
        timeout_ms: u64,
    },

    #[error("Response too large: {size} bytes exceeds limit of {limit} bytes")]
    ResponseTooLarge { size: usize, limit: usize },

    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("Resource not found: {resource_type} '{resource_id}'")]
    ResourceNotFound {
        resource_type: String,
        resource_id: String,
    },

    #[error("Rate limit exceeded: {requests} requests in {window_secs} seconds")]
    RateLimitExceeded { requests: usize, window_secs: u64 },

    #[error(transparent)]
    Validation(#[from] ValidationError),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Validation errors for parameters and data
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ValidationError {
    #[error("Missing required field: {field}")]
    MissingRequired { field: String },

    #[error("Invalid type for field '{field}': expected {expected}, got {actual}")]
    TypeMismatch {
        field: String,
        expected: String,
        actual: String,
    },

    #[error("Value out of range for '{field}': {value} not in [{min}, {max}]")]
    OutOfRange {
        field: String,
        value: i64,
        min: i64,
        max: i64,
    },

    #[error("Invalid format for '{field}': {reason}")]
    InvalidFormat { field: String, reason: String },

    #[error("Constraint violation for '{field}': {constraint}")]
    ConstraintViolation { field: String, constraint: String },
}

/// SPARQL-specific errors
#[derive(Debug, Error, Clone, PartialEq)]
pub enum SparqlError {
    #[error("Query syntax error: {0}")]
    SyntaxError(String),

    #[error("SPARQL injection detected: {pattern}")]
    InjectionDetected { pattern: String },

    #[error("Query validation failed: {reason}")]
    ValidationFailed { reason: String },

    #[error("Expected variable '{variable}' not found in results")]
    MissingVariable { variable: String },

    #[error("Type mismatch for variable '{variable}': expected {expected}, got {actual}")]
    TypeMismatch {
        variable: String,
        expected: String,
        actual: String,
    },

    #[error("Cardinality constraint violated: expected {expected}, got {actual} results")]
    CardinalityViolation { expected: String, actual: usize },
}

/// Template rendering errors
#[derive(Debug, Error, Clone)]
pub enum TemplateError {
    #[error("Template not found: {template_name}")]
    NotFound { template_name: String },

    #[error("Template syntax error in {template_name}: {message}")]
    SyntaxError {
        template_name: String,
        message: String,
    },

    #[error("Missing required parameter: {parameter}")]
    MissingParameter { parameter: String },

    #[error("Invalid parameter value for '{parameter}': {reason}")]
    InvalidParameter { parameter: String, reason: String },

    #[error("Rendering failed: {0}")]
    RenderingFailed(String),
}

// =============================================================================
// ERROR CONTEXT PATTERNS
// =============================================================================

/// Example: Building rich error context
pub fn load_and_process_ontology(path: &str) -> Result<ProcessedOntology> {
    // Static context
    let content = std::fs::read_to_string(path)
        .context("Failed to read ontology file")?;

    // Dynamic context with path information
    let parsed = parse_ontology(&content)
        .with_context(|| format!("Failed to parse ontology from file: {}", path))?;

    // Nested context with operation details
    let validated = validate_ontology(&parsed)
        .with_context(|| {
            format!(
                "Ontology validation failed for '{}' (version {})",
                parsed.name,
                parsed.version
            )
        })?;

    process_ontology(validated)
        .with_context(|| format!("Failed to process ontology '{}'", parsed.name))
}

/// Example: Error context in async operations
pub async fn execute_sparql_pipeline(
    query: &str,
    template: &str,
) -> Result<PipelineResult> {
    // Context for parsing
    let parsed_query = parse_sparql(query)
        .with_context(|| format!("Failed to parse SPARQL query for template '{}'", template))?;

    // Context for validation
    validate_sparql(&parsed_query)
        .with_context(|| {
            format!(
                "SPARQL validation failed for template '{}': query contains {} variables",
                template,
                parsed_query.variables.len()
            )
        })?;

    // Context for execution with retry info
    let results = execute_with_retry(&parsed_query)
        .await
        .with_context(|| {
            format!(
                "Failed to execute SPARQL query for template '{}' after multiple retries",
                template
            )
        })?;

    // Context for result processing
    process_results(results, template)
        .with_context(|| format!("Failed to process SPARQL results for template '{}'", template))
}

// =============================================================================
// MCP ERROR MAPPING
// =============================================================================

/// MCP error codes (simplified from MCP protocol)
pub enum McpErrorCode {
    InvalidParams,
    InvalidRequest,
    MethodNotFound,
    InternalError,
    ResourceNotFound,
}

pub struct McpError {
    pub code: McpErrorCode,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl McpError {
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: McpErrorCode::InvalidParams,
            message: message.into(),
            data: None,
        }
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: McpErrorCode::InvalidRequest,
            message: message.into(),
            data: None,
        }
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self {
            code: McpErrorCode::InternalError,
            message: message.into(),
            data: None,
        }
    }

    pub fn resource_not_found(message: impl Into<String>) -> Self {
        Self {
            code: McpErrorCode::ResourceNotFound,
            message: message.into(),
            data: None,
        }
    }
}

/// Convert application errors to MCP errors
pub fn to_mcp_error(error: anyhow::Error) -> McpError {
    // Check for specific error types using downcast_ref
    if let Some(tool_error) = error.downcast_ref::<McpToolError>() {
        return match tool_error {
            McpToolError::ToolDisabled { tool_name } => McpError::invalid_request(format!(
                "Tool '{}' is disabled. Enable it in server configuration.",
                tool_name
            )),
            McpToolError::Timeout {
                tool_name,
                timeout_ms,
            } => McpError::internal_error(format!(
                "Tool '{}' timed out after {}ms. Try reducing the request size.",
                tool_name, timeout_ms
            )),
            McpToolError::ResponseTooLarge { size, limit } => McpError::invalid_request(format!(
                "Response too large ({} bytes > {} bytes). Use pagination or filters.",
                size, limit
            )),
            McpToolError::InvalidParameters(msg) => McpError::invalid_params(msg.clone()),
            McpToolError::ResourceNotFound {
                resource_type,
                resource_id,
            } => McpError::resource_not_found(format!(
                "{} '{}' not found. Verify the identifier.",
                resource_type, resource_id
            )),
            McpToolError::RateLimitExceeded {
                requests,
                window_secs,
            } => McpError::invalid_request(format!(
                "Rate limit exceeded: {} requests in {} seconds. Please slow down.",
                requests, window_secs
            )),
            McpToolError::Validation(validation_error) => {
                validation_error_to_mcp(validation_error)
            }
            McpToolError::Other(_) => McpError::internal_error(tool_error.to_string()),
        };
    }

    if let Some(validation_error) = error.downcast_ref::<ValidationError>() {
        return validation_error_to_mcp(validation_error);
    }

    if let Some(sparql_error) = error.downcast_ref::<SparqlError>() {
        return sparql_error_to_mcp(sparql_error);
    }

    if let Some(template_error) = error.downcast_ref::<TemplateError>() {
        return template_error_to_mcp(template_error);
    }

    // Check error message for common patterns
    let error_msg = error.to_string().to_lowercase();

    if error_msg.contains("not found") {
        return McpError::resource_not_found(error.to_string());
    }

    if error_msg.contains("invalid")
        || error_msg.contains("malformed")
        || error_msg.contains("parse")
    {
        return McpError::invalid_params(error.to_string());
    }

    // Default to internal error
    McpError::internal_error(format!(
        "An unexpected error occurred: {}",
        truncate_error_message(&error.to_string(), 200)
    ))
}

fn validation_error_to_mcp(error: &ValidationError) -> McpError {
    match error {
        ValidationError::MissingRequired { field } => McpError::invalid_params(format!(
            "Missing required field '{}'. Please provide this field.",
            field
        )),
        ValidationError::TypeMismatch {
            field,
            expected,
            actual,
        } => McpError::invalid_params(format!(
            "Field '{}' has wrong type. Expected {}, got {}.",
            field, expected, actual
        )),
        ValidationError::OutOfRange {
            field,
            value,
            min,
            max,
        } => McpError::invalid_params(format!(
            "Field '{}' value {} is out of range [{}, {}].",
            field, value, min, max
        )),
        ValidationError::InvalidFormat { field, reason } => McpError::invalid_params(format!(
            "Field '{}' has invalid format: {}.",
            field, reason
        )),
        ValidationError::ConstraintViolation { field, constraint } => {
            McpError::invalid_params(format!(
                "Field '{}' violates constraint: {}.",
                field, constraint
            ))
        }
    }
}

fn sparql_error_to_mcp(error: &SparqlError) -> McpError {
    match error {
        SparqlError::SyntaxError(msg) => {
            McpError::invalid_params(format!("SPARQL syntax error: {}", msg))
        }
        SparqlError::InjectionDetected { pattern } => McpError::invalid_request(format!(
            "Potential SPARQL injection detected: {}. Query rejected for security.",
            pattern
        )),
        SparqlError::ValidationFailed { reason } => {
            McpError::invalid_params(format!("SPARQL validation failed: {}", reason))
        }
        SparqlError::MissingVariable { variable } => McpError::invalid_request(format!(
            "Expected variable '{}' not found in query results.",
            variable
        )),
        SparqlError::TypeMismatch {
            variable,
            expected,
            actual,
        } => McpError::invalid_request(format!(
            "Variable '{}' has wrong type. Expected {}, got {}.",
            variable, expected, actual
        )),
        SparqlError::CardinalityViolation { expected, actual } => {
            McpError::invalid_request(format!(
                "Query returned {} results, but expected {}.",
                actual, expected
            ))
        }
    }
}

fn template_error_to_mcp(error: &TemplateError) -> McpError {
    match error {
        TemplateError::NotFound { template_name } => McpError::resource_not_found(format!(
            "Template '{}' not found. Check the template name.",
            template_name
        )),
        TemplateError::SyntaxError {
            template_name,
            message,
        } => McpError::internal_error(format!(
            "Template '{}' has syntax error: {}",
            template_name, message
        )),
        TemplateError::MissingParameter { parameter } => McpError::invalid_params(format!(
            "Missing required template parameter: '{}'",
            parameter
        )),
        TemplateError::InvalidParameter { parameter, reason } => {
            McpError::invalid_params(format!("Invalid parameter '{}': {}", parameter, reason))
        }
        TemplateError::RenderingFailed(msg) => {
            McpError::internal_error(format!("Template rendering failed: {}", msg))
        }
    }
}

fn truncate_error_message(msg: &str, max_len: usize) -> String {
    if msg.len() <= max_len {
        msg.to_string()
    } else {
        format!("{}...", &msg[..max_len])
    }
}

// =============================================================================
// RETRY PATTERNS
// =============================================================================

pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryConfig {
    pub fn for_sparql_queries() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(15),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }

    pub fn for_file_operations() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
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

impl ExponentialBackoff {
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }
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
            || error_msg.contains("not found")
            || error_msg.contains("unauthorized")
        {
            return false;
        }

        // Retry transient errors
        error_msg.contains("timeout")
            || error_msg.contains("unavailable")
            || error_msg.contains("busy")
            || error_msg.contains("locked")
            || error_msg.contains("temporary")
            || error_msg.contains("connection")
    }

    fn delay(&self, attempt: u32) -> Duration {
        let base_delay = self.config.initial_delay.as_millis() as f64;
        let exponential_delay = base_delay * self.config.backoff_multiplier.powi(attempt as i32);

        let mut delay = Duration::from_millis(exponential_delay as u64);

        if delay > self.config.max_delay {
            delay = self.config.max_delay;
        }

        if self.config.jitter {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let jitter = (delay.as_millis() as f64 * 0.25 * rng.gen::<f64>()) as u64;
            delay += Duration::from_millis(jitter);
        }

        delay
    }
}

/// Retry an async operation with a policy
pub async fn retry_async<T, F, Fut>(
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
                    println!(
                        "[RETRY] Operation '{}' succeeded after {} attempts",
                        operation_name, attempt
                    );
                }
                return Ok(result);
            }
            Err(err) => {
                if policy.should_retry(attempt, &err) {
                    let delay = policy.delay(attempt);
                    println!(
                        "[RETRY] Operation '{}' failed (attempt {}), retrying after {:?}: {}",
                        operation_name, attempt, delay, err
                    );
                    tokio::time::sleep(delay).await;
                } else {
                    return Err(err.context(format!(
                        "Operation '{}' failed after {} attempts",
                        operation_name, attempt
                    )));
                }
            }
        }
    }
}

// =============================================================================
// CIRCUIT BREAKER PATTERN
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(60),
        }
    }
}

pub struct CircuitBreaker {
    name: String,
    config: CircuitBreakerConfig,
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
}

impl CircuitBreaker {
    pub fn new(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        Self {
            name: name.into(),
            config,
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
        }
    }

    pub async fn execute<T, F, Fut>(&mut self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        // Check circuit state
        match self.state {
            CircuitState::Open => {
                if let Some(last_failure) = self.last_failure_time {
                    if last_failure.elapsed() >= self.config.timeout {
                        println!(
                            "[CIRCUIT] Transitioning '{}' from Open to HalfOpen",
                            self.name
                        );
                        self.state = CircuitState::HalfOpen;
                        self.success_count = 0;
                    } else {
                        return Err(anyhow!(
                            "Circuit breaker '{}' is open (failing fast)",
                            self.name
                        ));
                    }
                }
            }
            CircuitState::Closed | CircuitState::HalfOpen => {
                // Allow execution
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

    fn on_success(&mut self) {
        match self.state {
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.config.success_threshold {
                    println!(
                        "[CIRCUIT] Transitioning '{}' from HalfOpen to Closed",
                        self.name
                    );
                    self.state = CircuitState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                }
            }
            CircuitState::Closed => {
                self.failure_count = 0;
            }
            CircuitState::Open => {}
        }
    }

    fn on_failure(&mut self) {
        self.last_failure_time = Some(Instant::now());

        match self.state {
            CircuitState::HalfOpen => {
                println!(
                    "[CIRCUIT] Failure in HalfOpen state, reopening circuit '{}'",
                    self.name
                );
                self.state = CircuitState::Open;
                self.success_count = 0;
            }
            CircuitState::Closed => {
                self.failure_count += 1;
                if self.failure_count >= self.config.failure_threshold {
                    println!(
                        "[CIRCUIT] Threshold exceeded, opening circuit '{}'",
                        self.name
                    );
                    self.state = CircuitState::Open;
                }
            }
            CircuitState::Open => {}
        }
    }

    pub fn state(&self) -> CircuitState {
        self.state
    }
}

// =============================================================================
// PARTIAL SUCCESS HANDLING
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult<T> {
    pub succeeded: Vec<T>,
    pub failed: Vec<BatchFailure>,
    pub summary: BatchSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchFailure {
    pub index: usize,
    pub item_id: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSummary {
    pub total: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub success_rate: f64,
}

impl<T> BatchResult<T> {
    pub fn new() -> Self {
        Self {
            succeeded: Vec::new(),
            failed: Vec::new(),
            summary: BatchSummary {
                total: 0,
                success_count: 0,
                failure_count: 0,
                success_rate: 0.0,
            },
        }
    }

    pub fn finalize(mut self, total: usize) -> Self {
        self.summary.total = total;
        self.summary.success_count = self.succeeded.len();
        self.summary.failure_count = self.failed.len();
        self.summary.success_rate = if total > 0 {
            (self.succeeded.len() as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        self
    }
}

pub async fn process_batch<T, I, F, Fut>(
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
        match processor(index, item).await {
            Ok(processed) => {
                result.succeeded.push(processed);
            }
            Err(err) => {
                result.failed.push(BatchFailure {
                    index,
                    item_id: format!("item_{}", index),
                    error: err.to_string(),
                });
            }
        }
    }

    result.finalize(total)
}

// =============================================================================
// EXAMPLE USAGE
// =============================================================================

// Stub types for examples
pub struct ProcessedOntology {
    pub name: String,
    pub version: String,
}

pub struct ParsedOntology {
    pub name: String,
    pub version: String,
}

pub struct ValidatedOntology {
    pub name: String,
}

pub struct PipelineResult {
    pub data: String,
}

pub struct ParsedQuery {
    pub variables: Vec<String>,
}

pub struct QueryResults {
    pub rows: Vec<HashMap<String, String>>,
}

fn parse_ontology(content: &str) -> Result<ParsedOntology> {
    if content.is_empty() {
        bail!("Empty ontology content");
    }
    Ok(ParsedOntology {
        name: "Example".to_string(),
        version: "1.0".to_string(),
    })
}

fn validate_ontology(ontology: &ParsedOntology) -> Result<ValidatedOntology> {
    Ok(ValidatedOntology {
        name: ontology.name.clone(),
    })
}

fn process_ontology(ontology: ValidatedOntology) -> Result<ProcessedOntology> {
    Ok(ProcessedOntology {
        name: ontology.name,
        version: "1.0".to_string(),
    })
}

fn parse_sparql(query: &str) -> Result<ParsedQuery> {
    if query.is_empty() {
        bail!("Empty SPARQL query");
    }
    Ok(ParsedQuery {
        variables: vec!["subject".to_string()],
    })
}

fn validate_sparql(query: &ParsedQuery) -> Result<()> {
    if query.variables.is_empty() {
        bail!("Query must have at least one variable");
    }
    Ok(())
}

async fn execute_with_retry(query: &ParsedQuery) -> Result<QueryResults> {
    Ok(QueryResults {
        rows: vec![HashMap::new()],
    })
}

fn process_results(results: QueryResults, template: &str) -> Result<PipelineResult> {
    Ok(PipelineResult {
        data: template.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error_creation() {
        let error = ValidationError::MissingRequired {
            field: "name".to_string(),
        };
        assert_eq!(error.to_string(), "Missing required field: name");
    }

    #[test]
    fn test_mcp_error_mapping() {
        let tool_error = McpToolError::ToolDisabled {
            tool_name: "test_tool".to_string(),
        };
        let error = anyhow::Error::from(tool_error);
        let mcp_error = to_mcp_error(error);

        // Verify the error is mapped correctly
        assert!(mcp_error.message.contains("test_tool"));
        assert!(mcp_error.message.contains("disabled"));
    }

    #[test]
    fn test_retry_policy() {
        let policy = ExponentialBackoff::new(RetryConfig::default());

        // Should retry on transient errors
        let transient_error = anyhow!("connection timeout");
        assert!(policy.should_retry(1, &transient_error));

        // Should not retry on fatal errors
        let fatal_error = anyhow!("permission denied");
        assert!(!policy.should_retry(1, &fatal_error));

        // Should not retry after max attempts
        assert!(!policy.should_retry(5, &transient_error));
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let mut cb = CircuitBreaker::new(
            "test",
            CircuitBreakerConfig {
                failure_threshold: 3,
                success_threshold: 2,
                timeout: Duration::from_millis(100),
            },
        );

        assert_eq!(cb.state(), CircuitState::Closed);

        // Trigger failures
        for _ in 0..3 {
            let _ = cb
                .execute(|| async { Err::<(), _>(anyhow!("error")) })
                .await;
        }

        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[tokio::test]
    async fn test_batch_processing() {
        let items = vec![1, 2, 3, 4, 5];

        let result = process_batch(items, |_idx, item| async move {
            if item == 3 {
                Err(anyhow!("Error processing item 3"))
            } else {
                Ok(item * 2)
            }
        })
        .await;

        assert_eq!(result.summary.success_count, 4);
        assert_eq!(result.summary.failure_count, 1);
        assert!(result.summary.success_rate > 75.0);
    }
}

fn main() {
    println!("Error Handling Patterns Example");
    println!("================================");
    println!();
    println!("This example demonstrates comprehensive error handling patterns for MCP servers.");
    println!("Run tests with: cargo test --example error_handling_patterns");
}
