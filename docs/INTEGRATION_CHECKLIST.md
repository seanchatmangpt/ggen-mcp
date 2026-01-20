# Integration Checklist for JSON Schema Validation

This checklist guides the integration of the JSON schema validation system into the MCP server.

## ✅ Completed

- [x] Create `src/validation/schema.rs` with JSON schema validation logic
- [x] Create `src/validation/middleware.rs` with validation middleware
- [x] Create `src/validation/integration.rs` with pre-configured validators
- [x] Update `src/validation/mod.rs` to export new modules
- [x] Add `pub mod validation;` to `src/lib.rs` (already present)
- [x] Create comprehensive documentation in `docs/validation.md`
- [x] Create quick start guide in `src/validation/README.md`
- [x] Create integration tests in `tests/validation_integration_test.rs`
- [x] Create usage examples in `examples/validation_example.rs`
- [x] Create server integration example in `examples/server_integration_example.rs`

## 🔄 Optional Integration Steps

These steps are optional but recommended for full integration:

### 1. Add Validation to Server State

**File**: `src/state.rs`

```rust
use crate::validation::SchemaValidationMiddleware;
use std::sync::Arc;

pub struct AppState {
    // ... existing fields ...

    /// Schema validator for tool parameters
    validator: Arc<SchemaValidationMiddleware>,
}

impl AppState {
    pub fn new(config: Arc<ServerConfig>) -> Self {
        use crate::validation::integration::create_validation_middleware;

        Self {
            // ... existing initialization ...
            validator: Arc::new(create_validation_middleware()),
        }
    }

    pub fn validator(&self) -> &Arc<SchemaValidationMiddleware> {
        &self.validator
    }
}
```

### 2. Add Validation to Tool Handlers

**File**: `src/server.rs`

Add validation before tool execution:

```rust
use crate::validation::format_validation_errors;

pub async fn read_table(
    &self,
    Parameters(params): Parameters<tools::ReadTableParams>,
) -> Result<Json<ReadTableResponse>, McpError> {
    // Optional: explicit validation (Parameters wrapper doesn't validate yet)
    // This could be added as a pre-processing step

    self.ensure_tool_enabled("read_table")
        .map_err(to_mcp_error)?;
    self.run_tool_with_timeout(
        "read_table",
        tools::read_table(self.state.clone(), params),
    )
    .await
    .map(Json)
    .map_err(to_mcp_error)
}
```

### 3. Add Configuration Options

**File**: `src/config.rs`

```rust
pub struct ServerConfig {
    // ... existing fields ...

    /// Enable strict JSON schema validation
    #[clap(long, env = "SPREADSHEET_MCP_STRICT_VALIDATION", default_value = "true")]
    pub strict_validation: bool,

    /// Log validation results
    #[clap(long, env = "SPREADSHEET_MCP_LOG_VALIDATION", default_value = "false")]
    pub log_validation: bool,
}
```

### 4. Add Validation Metrics

**File**: `src/state.rs` or new `src/metrics.rs`

```rust
use std::sync::atomic::{AtomicU64, Ordering};

pub struct ValidationMetrics {
    total_validations: AtomicU64,
    failed_validations: AtomicU64,
    validation_time_ms: AtomicU64,
}

impl ValidationMetrics {
    pub fn record_validation(&self, success: bool, duration_ms: u64) {
        self.total_validations.fetch_add(1, Ordering::Relaxed);
        if !success {
            self.failed_validations.fetch_add(1, Ordering::Relaxed);
        }
        self.validation_time_ms.fetch_add(duration_ms, Ordering::Relaxed);
    }
}
```

### 5. Add Validation Error Handler

**File**: `src/server.rs`

```rust
use crate::validation::SchemaValidationError;

fn validation_error_to_mcp(error: SchemaValidationError) -> McpError {
    match error {
        SchemaValidationError::ValidationFailed { tool, errors } => {
            let message = format_validation_errors(&tool, &errors);
            McpError::invalid_params(message, None)
        }
        SchemaValidationError::MissingRequiredField { tool, field } => {
            McpError::invalid_params(
                format!("Missing required field '{}' in tool '{}'", field, tool),
                None
            )
        }
        SchemaValidationError::InvalidType { tool, field, expected, actual } => {
            McpError::invalid_params(
                format!(
                    "Invalid type for field '{}' in tool '{}': expected {}, got {}",
                    field, tool, expected, actual
                ),
                None
            )
        }
        _ => McpError::invalid_params(error.to_string(), None),
    }
}
```

## 📋 Testing Checklist

- [ ] Run unit tests: `cargo test --lib validation`
- [ ] Run integration tests: `cargo test --test validation_integration_test`
- [ ] Run examples:
  - [ ] `cargo run --example validation_example`
  - [ ] `cargo run --example server_integration_example`
- [ ] Run full test suite: `cargo test`
- [ ] Run with features:
  - [ ] `cargo test --features recalc`
  - [ ] `cargo test --all-features`
- [ ] Check code formatting: `cargo fmt --check`
- [ ] Check lints: `cargo clippy -- -D warnings`
- [ ] Generate documentation: `cargo doc --no-deps --open`

## 🔍 Verification Steps

### 1. Verify Schema Registration

```bash
# Create a test program that prints registered schemas
cargo run --example validation_example
```

### 2. Verify Validation Works

```rust
// In a test or example
let validator = create_validation_middleware();

// Valid params - should succeed
let valid = json!({ "workbook_or_fork_id": "test" });
assert!(validator.validate_tool_call("describe_workbook", &valid).is_ok());

// Invalid params - should fail
let invalid = json!({});
assert!(validator.validate_tool_call("describe_workbook", &invalid).is_err());
```

### 3. Verify Error Messages

```bash
# Run integration test and check error output
cargo test test_missing_required_field -- --nocapture
```

### 4. Verify Performance

```rust
// Benchmark schema lookup and validation
use std::time::Instant;

let validator = create_validation_middleware();
let params = json!({ "workbook_or_fork_id": "test" });

let start = Instant::now();
for _ in 0..1000 {
    validator.validate_tool_call("describe_workbook", &params).unwrap();
}
let duration = start.elapsed();

println!("Average validation time: {:?}", duration / 1000);
// Should be < 1ms per validation
```

## 📚 Documentation Checklist

- [x] API documentation (inline doc comments)
- [x] Module-level documentation (`src/validation/mod.rs`)
- [x] Comprehensive guide (`docs/validation.md`)
- [x] Quick start guide (`src/validation/README.md`)
- [x] Integration examples (`examples/`)
- [x] Implementation summary (`IMPLEMENTATION_SUMMARY.md`)
- [ ] Update main README.md to mention validation
- [ ] Update CHANGELOG.md with validation feature

## 🚀 Deployment Checklist

- [ ] Merge validation implementation to main branch
- [ ] Update version number
- [ ] Update CHANGELOG.md
- [ ] Run full test suite in CI/CD
- [ ] Deploy to staging environment
- [ ] Verify in staging:
  - [ ] All tools accept valid parameters
  - [ ] Invalid parameters are rejected with clear errors
  - [ ] Performance is acceptable
- [ ] Deploy to production
- [ ] Monitor error rates and validation failures

## 🎯 Success Criteria

The integration is successful when:

- ✅ All tool parameters are validated before execution
- ✅ Invalid parameters are rejected with detailed error messages
- ✅ Validation performance impact is < 1ms per request
- ✅ All tests pass
- ✅ Documentation is complete and accurate
- ✅ Examples demonstrate proper usage
- ✅ Error messages are clear and actionable

## 🐛 Troubleshooting

### Issue: Schemas not registered

**Solution**: Ensure `create_validation_middleware()` is called at server startup and stored in server state.

### Issue: Validation errors not showing field names

**Solution**: Check that error formatting is using `format_validation_errors()` function.

### Issue: Optional fields failing validation

**Solution**: Ensure structs use `#[serde(default)]` for optional fields and `Option<T>` type.

### Issue: Slow validation

**Solution**:
1. Verify schemas are cached (registered once at startup)
2. Check that validator is wrapped in `Arc` for sharing
3. Profile validation code to find bottlenecks

### Issue: Compilation errors

**Solution**:
1. Ensure all dependencies are up to date
2. Run `cargo clean && cargo build`
3. Check that feature flags are consistent

## 📞 Support

For questions or issues with the validation implementation:

1. Check documentation: `docs/validation.md`
2. Review examples: `examples/validation_example.rs`
3. Check tests: `tests/validation_integration_test.rs`
4. Review implementation: `src/validation/`

## 🔗 Related Resources

- [JSON Schema Specification](https://json-schema.org/)
- [schemars Documentation](https://docs.rs/schemars/)
- [rmcp Framework](https://docs.rs/rmcp/)
- [MCP Protocol](https://modelcontextprotocol.io/)
- [Implementation Summary](../IMPLEMENTATION_SUMMARY.md)
- [Validation Guide](validation.md)
