# Error Handling Documentation

This directory contains comprehensive documentation and examples for error handling in ggen-mcp.

## Documents

### 1. [RUST_MCP_ERROR_HANDLING.md](./RUST_MCP_ERROR_HANDLING.md)
**Comprehensive Best Practices Guide**

A complete guide to error handling in Rust MCP servers, covering:
- Error type strategy (anyhow vs thiserror)
- Error context patterns
- MCP error reporting
- Recovery patterns (retry, circuit breaker, partial success)
- Error testing strategies
- Performance considerations
- TPS Jidoka principles for error prevention
- Real-world examples

**Target Audience:** All developers working on ggen-mcp

**Use Cases:**
- Learning error handling best practices
- Reference for implementing new features
- Code review guidelines
- Onboarding new team members

### 2. [ERROR_HANDLING_ANALYSIS.md](./ERROR_HANDLING_ANALYSIS.md)
**Current State Analysis and Improvement Roadmap**

Detailed analysis of the current error handling implementation:
- Current patterns and usage statistics
- Strengths and weaknesses
- Improvement opportunities (prioritized)
- Metrics and goals
- Actionable recommendations

**Target Audience:** Technical leads, architects, senior developers

**Use Cases:**
- Sprint planning
- Technical debt management
- Architecture reviews
- Performance optimization

## Examples

### [examples/error_handling_patterns.rs](../examples/error_handling_patterns.rs)
**Runnable Examples**

Practical, runnable examples demonstrating:
- Custom error type definitions
- Error context building
- MCP error mapping
- Retry logic with exponential backoff
- Circuit breaker pattern
- Partial success handling
- Comprehensive test examples

**How to run:**
```bash
# Run the example
cargo run --example error_handling_patterns

# Run the tests
cargo test --example error_handling_patterns
```

## Quick Start

### For New Developers

1. **Read** [RUST_MCP_ERROR_HANDLING.md](./RUST_MCP_ERROR_HANDLING.md) sections 1-3
2. **Review** existing error types in:
   - `src/validation/schema.rs` (SchemaValidationError)
   - `src/template/parameter_validation.rs` (ValidationError)
   - `src/sparql/result_validation.rs` (ValidationError)
   - `src/ontology/graph_integrity.rs` (IntegrityError)
3. **Run** the error handling examples
4. **Practice** by adding error handling to a small feature

### For Experienced Developers

1. **Skim** [RUST_MCP_ERROR_HANDLING.md](./RUST_MCP_ERROR_HANDLING.md) for patterns
2. **Review** [ERROR_HANDLING_ANALYSIS.md](./ERROR_HANDLING_ANALYSIS.md) for current state
3. **Check** improvement opportunities relevant to your work
4. **Contribute** by implementing recommended improvements

### For Code Reviewers

1. **Reference** [RUST_MCP_ERROR_HANDLING.md](./RUST_MCP_ERROR_HANDLING.md) during reviews
2. **Check** that:
   - Errors use appropriate types (anyhow vs thiserror)
   - Error context is provided
   - MCP errors are mapped correctly
   - Recovery patterns are used where appropriate
   - Error paths are tested

## Common Patterns

### 1. Application-Level Errors
```rust
use anyhow::{Context, Result};

pub fn load_file(path: &str) -> Result<String> {
    std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path))
}
```

### 2. Domain-Specific Errors
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Missing required field: {0}")]
    MissingRequired(String),

    #[error("Invalid type for '{field}': expected {expected}, got {actual}")]
    TypeMismatch {
        field: String,
        expected: String,
        actual: String,
    },
}
```

### 3. MCP Error Mapping
```rust
fn to_mcp_error(error: Error) -> McpError {
    if let Some(validation_error) = error.downcast_ref::<ValidationError>() {
        return McpError::invalid_params(
            validation_error.to_string(),
            None
        );
    }

    McpError::internal_error(error.to_string(), None)
}
```

### 4. Retry Pattern
```rust
use crate::recovery::retry::{retry_async_with_policy, ExponentialBackoff, RetryConfig};

let policy = ExponentialBackoff::new(RetryConfig::default());
let result = retry_async_with_policy(
    || async { perform_operation().await },
    &policy,
    "operation_name"
).await?;
```

### 5. Circuit Breaker
```rust
use crate::recovery::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};

let mut cb = CircuitBreaker::new("service_name", CircuitBreakerConfig::default());
let result = cb.execute_async(|| async {
    call_external_service().await
}).await?;
```

## Error Handling Checklist

Use this checklist when implementing error handling:

- [ ] **Error Type**: Chose appropriate error type (anyhow vs thiserror)
- [ ] **Context**: Added error context with `.context()` or `.with_context()`
- [ ] **MCP Mapping**: Mapped to appropriate MCP error code
- [ ] **User Message**: Error message is user-friendly and actionable
- [ ] **Suggestion**: Included suggestion for fixing (where applicable)
- [ ] **Recovery**: Implemented retry/fallback if appropriate
- [ ] **Testing**: Added tests for error paths
- [ ] **Logging**: Added appropriate tracing/logging
- [ ] **Documentation**: Documented error conditions in function docs

## Resources

### Internal
- [Recovery Module](../src/recovery/) - Retry, circuit breaker, partial success
- [Validation Module](../src/validation/) - Schema and input validation
- [SPARQL Validation](../src/sparql/) - Query validation and result checking
- [Template Validation](../src/template/) - Template parameter validation

### External
- [anyhow documentation](https://docs.rs/anyhow)
- [thiserror documentation](https://docs.rs/thiserror)
- [Rust Error Handling Book](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [MCP Specification](https://spec.modelcontextprotocol.io/)

## Contributing

When adding new error handling patterns:

1. **Document** the pattern in RUST_MCP_ERROR_HANDLING.md
2. **Add example** to examples/error_handling_patterns.rs
3. **Update analysis** in ERROR_HANDLING_ANALYSIS.md if significant
4. **Add tests** demonstrating the pattern
5. **Review** with the team before merging

## Questions?

- **General questions**: Ask in #dev-general
- **Architecture questions**: Ask in #architecture
- **MCP-specific questions**: Ask in #mcp-development

---

**Last Updated:** 2026-01-20
**Maintainer:** Development Team
