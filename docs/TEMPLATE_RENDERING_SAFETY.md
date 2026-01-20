# Template Rendering Safety Guide

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Components](#components)
4. [Safe Rendering Patterns](#safe-rendering-patterns)
5. [Configuration](#configuration)
6. [Error Handling](#error-handling)
7. [Output Validation](#output-validation)
8. [Testing](#testing)
9. [Troubleshooting](#troubleshooting)
10. [Best Practices](#best-practices)

## Overview

The template rendering safety system implements **poka-yoke** (error-proofing) principles from the Toyota Production System to prevent errors during code generation from Tera templates.

### Design Principles

1. **Prevention over Detection**: Stop errors before they occur
2. **Graceful Degradation**: Provide partial results when possible
3. **Clear Feedback**: Detailed error messages with suggestions
4. **Resource Limits**: Prevent resource exhaustion
5. **Defense in Depth**: Multiple layers of protection

### Safety Guarantees

The system provides the following safety guarantees:

- ✅ **Timeout Protection**: Templates that run too long are terminated
- ✅ **Memory Limits**: Output size is capped to prevent memory exhaustion
- ✅ **Recursion Limits**: Deep template nesting is prevented
- ✅ **Syntax Validation**: Generated code is checked for valid Rust syntax
- ✅ **Security Checks**: Dangerous patterns are detected
- ✅ **Resource Cleanup**: Temporary files are automatically removed

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   SafeRenderer                      │
│  ┌─────────────────────────────────────────────┐  │
│  │         RenderConfig                        │  │
│  │  - Timeouts, Limits, Whitelist             │  │
│  └─────────────────────────────────────────────┘  │
│                      │                             │
│         ┌────────────┴────────────┐                │
│         ▼                         ▼                │
│  ┌─────────────┐         ┌─────────────┐          │
│  │RenderContext│         │  Tera Engine│          │
│  │- Variables  │         │  - Templates│          │
│  │- Scoping    │         │  - Rendering│          │
│  │- Limits     │         └─────────────┘          │
│  └─────────────┘                 │                │
│         │                        │                │
│         └────────────┬───────────┘                │
│                      ▼                             │
│             ┌─────────────────┐                   │
│             │  Output         │                   │
│             └─────────────────┘                   │
│                      │                             │
│         ┌────────────┴────────────┐                │
│         ▼                         ▼                │
│  ┌─────────────┐         ┌─────────────┐          │
│  │OutputValidator│       │ErrorRecovery│          │
│  │- Syntax Check│       │- Collect Errors│       │
│  │- Security    │       │- Suggestions   │       │
│  └─────────────┘       └─────────────┘          │
│                      │                             │
│                ┌─────┴─────┐                      │
│                ▼           ▼                      │
│         ┌──────────┐ ┌──────────┐                │
│         │Valid Code│ │Error Report│              │
│         └──────────┘ └──────────┘                │
└─────────────────────────────────────────────────────┘
```

## Components

### 1. SafeRenderer

The main entry point for safe template rendering.

```rust
use spreadsheet_mcp::template::rendering_safety::{SafeRenderer, RenderConfig, RenderContext};

// Create renderer with custom config
let config = RenderConfig::default()
    .with_timeout_ms(10_000)
    .with_max_recursion_depth(15)
    .with_syntax_validation(true)
    .with_security_checks(true);

let renderer = SafeRenderer::new(config)?;

// Add templates
renderer.add_template("my_template", template_content)?;

// Create context with variables
let mut context = RenderContext::new();
context.insert("name", &"Example")?;
context.insert("items", &vec!["a", "b", "c"])?;

// Render safely
let output = renderer.render_safe("my_template", &context)?;
```

### 2. RenderContext

Isolated rendering environment with variable scoping.

```rust
use spreadsheet_mcp::template::rendering_safety::RenderContext;
use std::sync::Arc;

// Root context
let mut root = RenderContext::new();
root.insert("global_var", &"value")?;

// Child context (for nested rendering)
let root_arc = Arc::new(root);
let mut child = RenderContext::child(root_arc.clone());
child.insert("local_var", &"local")?;

// Child can access parent variables
assert!(child.get("global_var").is_some());

// Parent cannot access child variables
assert!(root_arc.get("local_var").is_none());
```

### 3. OutputValidator

Validates generated code for syntax and security issues.

```rust
use spreadsheet_mcp::template::rendering_safety::{OutputValidator, ValidationSeverity};

let validator = OutputValidator::new(
    true,  // validate_rust_syntax
    true,  // security_checks
);

let output = "fn main() { unsafe { /* ... */ } }";
let errors = validator.validate(output)?;

for error in &errors {
    match error.severity {
        ValidationSeverity::Error => {
            eprintln!("ERROR: {}", error.message);
        }
        ValidationSeverity::Warning => {
            println!("WARNING: {}", error.message);
        }
        ValidationSeverity::Info => {
            println!("INFO: {}", error.message);
        }
    }
}
```

### 4. ErrorRecovery

Handles graceful failure with detailed diagnostics.

```rust
use spreadsheet_mcp::template::rendering_safety::ErrorRecovery;

let mut recovery = ErrorRecovery::new(true); // allow_partial = true

// Errors are recorded during rendering
if recovery.has_errors() {
    // Get error report with suggestions
    let report = recovery.error_report();
    eprintln!("{}", report);

    // Get suggestions for fixes
    let suggestions = recovery.suggest_fixes();
    for suggestion in suggestions {
        println!("Suggestion: {}", suggestion);
    }

    // Get partial output if available
    if let Some(partial) = recovery.partial_output() {
        println!("Partial output available: {} bytes", partial.len());
    }
}
```

### 5. RenderGuard

RAII guard for automatic resource cleanup.

```rust
use spreadsheet_mcp::template::rendering_safety::RenderGuard;

let mut guard = RenderGuard::new();

// Register temporary files
let temp_file = std::env::temp_dir().join("temp.rs");
std::fs::write(&temp_file, "// temp content")?;
guard.register_temp_file(temp_file.clone());

// If rendering succeeds, commit the guard
let metrics = guard.commit();
println!("Rendering took: {:?}", metrics.duration);

// If guard is dropped without commit, temp files are cleaned up automatically
```

## Safe Rendering Patterns

### Pattern 1: Basic Safe Rendering

```rust
use spreadsheet_mcp::template::rendering_safety::{
    SafeRenderer, RenderConfig, RenderContext
};

fn render_template(template_name: &str, data: &MyData) -> Result<String> {
    let config = RenderConfig::default();
    let renderer = SafeRenderer::new(config)?;

    let mut context = RenderContext::new();
    context.insert("data", data)?;

    renderer.render_safe(template_name, &context)
}
```

### Pattern 2: Batch Rendering with Timeout

```rust
use spreadsheet_mcp::template::rendering_safety::{
    SafeRenderer, RenderConfig, RenderContext
};

fn batch_render(templates: &[&str]) -> Result<Vec<String>> {
    let config = RenderConfig::default()
        .with_timeout_ms(2000);  // 2s per template

    let renderer = SafeRenderer::new(config)?;
    let mut results = Vec::new();

    for template_name in templates {
        let context = RenderContext::new();
        match renderer.render_safe(template_name, &context) {
            Ok(output) => results.push(output),
            Err(e) => {
                eprintln!("Failed to render {}: {}", template_name, e);
                // Continue with other templates
            }
        }
    }

    Ok(results)
}
```

### Pattern 3: Nested Template Rendering

```rust
use spreadsheet_mcp::template::rendering_safety::{
    SafeRenderer, RenderConfig, RenderContext
};
use std::sync::Arc;

fn render_with_includes(
    renderer: &SafeRenderer,
    parent_template: &str,
    parent_context: Arc<RenderContext>,
) -> Result<String> {
    // Check recursion depth before rendering
    parent_context.check_recursion_depth(
        renderer.config().max_recursion_depth
    )?;

    // Create child context for nested template
    let mut child_context = RenderContext::child(parent_context.clone());
    child_context.insert("nested", &true)?;

    renderer.render_safe(parent_template, &child_context)
}
```

### Pattern 4: Validation with Error Recovery

```rust
use spreadsheet_mcp::template::rendering_safety::{
    SafeRenderer, RenderConfig, RenderContext, ErrorRecovery,
};

fn render_with_recovery(template: &str) -> Result<(String, Vec<String>)> {
    let config = RenderConfig::default()
        .with_syntax_validation(true)
        .with_security_checks(true);

    let renderer = SafeRenderer::new(config)?;
    let context = RenderContext::new();

    let mut warnings = Vec::new();

    match renderer.render_safe(template, &context) {
        Ok(output) => Ok((output, warnings)),
        Err(e) => {
            // Collect error details
            let mut recovery = ErrorRecovery::new(false);
            recovery.record_error(e);

            // Get suggestions
            warnings.extend(recovery.suggest_fixes());

            Err(anyhow::anyhow!(recovery.error_report()))
        }
    }
}
```

## Configuration

### RenderConfig Options

```rust
use spreadsheet_mcp::template::rendering_safety::RenderConfig;
use std::path::PathBuf;

let config = RenderConfig::builder()
    // Timeout for rendering (default: 5000ms, max: 30000ms)
    .timeout_ms(10_000)

    // Maximum recursion depth (default: 10, max: 100)
    .max_recursion_depth(20)

    // Maximum output size in bytes (default: 10MB, max: 100MB)
    .max_output_size(50 * 1024 * 1024)

    // Maximum macro expansions (default: 1000)
    .max_macro_expansions(5000)

    // Enable syntax validation
    .validate_syntax(true)

    // Enable security checks
    .security_checks(true)

    // Add include file to whitelist
    .add_include_whitelist("templates/common/header.tera")
    .add_include_whitelist("templates/common/footer.tera")

    // Allow partial rendering on error
    .allow_partial_rendering(true)

    // Collect rendering metrics
    .collect_metrics(true)

    .build();
```

### Recommended Configurations

#### Development Configuration

```rust
let dev_config = RenderConfig::default()
    .with_timeout_ms(10_000)  // Longer timeout for debugging
    .with_syntax_validation(true)
    .with_security_checks(true);
```

#### Production Configuration

```rust
let prod_config = RenderConfig::default()
    .with_timeout_ms(5_000)   // Strict timeout
    .with_max_recursion_depth(10)
    .with_max_output_size(10 * 1024 * 1024)  // 10MB limit
    .with_syntax_validation(true)
    .with_security_checks(true);
```

#### Testing Configuration

```rust
let test_config = RenderConfig::default()
    .with_timeout_ms(30_000)  // Very long timeout
    .with_syntax_validation(false)  // Allow invalid syntax for testing
    .with_security_checks(false);
```

## Error Handling

### Error Types

The system defines several error types:

1. **Timeout Errors**: Template rendering exceeded time limit
2. **Size Errors**: Output exceeded size limit
3. **Recursion Errors**: Template nesting too deep
4. **Validation Errors**: Generated code has syntax errors
5. **Security Errors**: Dangerous patterns detected

### Handling Errors

```rust
use spreadsheet_mcp::template::rendering_safety::{
    SafeRenderer, RenderingError, RenderContext
};

fn handle_render_errors(
    renderer: &SafeRenderer,
    template: &str,
    context: &RenderContext,
) -> Result<String> {
    match renderer.render_safe(template, context) {
        Ok(output) => Ok(output),

        Err(RenderingError::Timeout { timeout_ms }) => {
            eprintln!("Template timed out after {}ms", timeout_ms);
            // Option 1: Increase timeout
            // Option 2: Simplify template
            // Option 3: Use cached result
            Err(anyhow::anyhow!("Template timeout"))
        }

        Err(RenderingError::OutputSizeExceeded { size, limit }) => {
            eprintln!("Output too large: {} > {}", size, limit);
            // Option 1: Split into multiple templates
            // Option 2: Increase limit
            Err(anyhow::anyhow!("Output too large"))
        }

        Err(RenderingError::ValidationFailed { errors }) => {
            eprintln!("Validation failed:");
            for error in &errors {
                eprintln!("  - {}", error);
            }
            // Option 1: Fix template
            // Option 2: Disable validation (not recommended)
            Err(anyhow::anyhow!("Validation failed"))
        }

        Err(e) => {
            eprintln!("Rendering error: {}", e);
            Err(anyhow::anyhow!("Rendering failed: {}", e))
        }
    }
}
```

## Output Validation

### Syntax Validation

The validator checks for:

- ✅ Balanced braces, brackets, and parentheses
- ✅ Valid Rust identifiers
- ✅ Common syntax errors (empty structs, consecutive semicolons)
- ✅ String literal handling
- ✅ Comment handling

```rust
use spreadsheet_mcp::template::rendering_safety::OutputValidator;

let validator = OutputValidator::new(true, false);

let code = r#"
    fn main() {
        let x = vec![1, 2, 3];
        println!("{:?}", x);
    }
"#;

let errors = validator.validate(code)?;
assert!(errors.is_empty(), "Code should be valid");
```

### Security Checks

The validator detects:

- ⚠️ Unsafe code blocks
- ⚠️ System command execution
- ℹ️ File system modifications
- ⚠️ Potential SQL injection

```rust
let validator = OutputValidator::new(false, true);

let unsafe_code = r#"
    unsafe {
        std::ptr::write(ptr, value);
    }
"#;

let errors = validator.validate(unsafe_code)?;
assert!(!errors.is_empty(), "Should detect unsafe code");
```

### Custom Validation

You can extend validation by wrapping the validator:

```rust
use spreadsheet_mcp::template::rendering_safety::{
    OutputValidator, ValidationError, ValidationSeverity
};

struct CustomValidator {
    inner: OutputValidator,
}

impl CustomValidator {
    fn validate(&self, output: &str) -> Result<Vec<ValidationError>> {
        let mut errors = self.inner.validate(output)?;

        // Add custom checks
        if output.contains("TODO") {
            errors.push(ValidationError {
                line: None,
                column: None,
                message: "TODO comment found in generated code".to_string(),
                severity: ValidationSeverity::Warning,
            });
        }

        Ok(errors)
    }
}
```

## Testing

### Unit Testing Templates

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use spreadsheet_mcp::template::rendering_safety::{
        SafeRenderer, RenderConfig, RenderContext
    };

    #[test]
    fn test_template_rendering() {
        let config = RenderConfig::default();
        let renderer = SafeRenderer::new(config).unwrap();

        renderer.add_template("test", "Hello {{ name }}!").unwrap();

        let mut context = RenderContext::new();
        context.insert("name", &"World").unwrap();

        let output = renderer.render_safe("test", &context).unwrap();
        assert_eq!(output, "Hello World!");
    }

    #[test]
    fn test_timeout_enforcement() {
        let config = RenderConfig::default().with_timeout_ms(100);
        let renderer = SafeRenderer::new(config).unwrap();

        // Infinite loop template (would timeout in real implementation)
        renderer.add_template(
            "infinite",
            "{% for i in range(start=0, end=999999) %}{{ i }}{% endfor %}"
        ).unwrap();

        let context = RenderContext::new();
        // This should timeout
        // let result = renderer.render_safe("infinite", &context);
        // assert!(matches!(result, Err(RenderingError::Timeout { .. })));
    }
}
```

### Integration Testing

```rust
#[test]
fn test_complete_generation_pipeline() {
    let config = RenderConfig::default()
        .with_syntax_validation(true)
        .with_security_checks(true);

    let renderer = SafeRenderer::from_directory("templates", config).unwrap();

    let mut context = RenderContext::new();
    context.insert("aggregate_name", &"Order").unwrap();
    context.insert("fields", &vec![
        Field { name: "id".into(), type_: "String".into() },
        Field { name: "total".into(), type_: "f64".into() },
    ]).unwrap();

    let output = renderer.render_safe("aggregate.rs.tera", &context).unwrap();

    // Verify output contains expected patterns
    assert!(output.contains("pub struct Order"));
    assert!(output.contains("pub id: String"));
    assert!(output.contains("pub total: f64"));

    // Verify syntax is valid
    let validator = OutputValidator::new(true, true);
    let errors = validator.validate(&output).unwrap();
    assert!(errors.is_empty(), "Generated code should be valid");
}
```

### Testing Malicious Templates

See `tests/template_rendering_tests.rs` for comprehensive malicious template tests.

## Troubleshooting

### Common Issues

#### Issue 1: Template Timeout

**Symptom**: `RenderingError::Timeout`

**Causes**:
- Template has too many loops
- Large data sets being rendered
- Complex template logic

**Solutions**:
1. Simplify template logic
2. Reduce data set size
3. Increase timeout: `.with_timeout_ms(10_000)`
4. Split into multiple smaller templates

#### Issue 2: Output Size Exceeded

**Symptom**: `RenderingError::OutputSizeExceeded`

**Causes**:
- Generating very large files
- Excessive repetition in loops

**Solutions**:
1. Split generation across multiple files
2. Reduce data being rendered
3. Increase limit: `.with_max_output_size(50 * 1024 * 1024)`

#### Issue 3: Recursion Depth Exceeded

**Symptom**: `RenderingError::RecursionDepthExceeded`

**Causes**:
- Too many template includes
- Deep template inheritance
- Circular includes

**Solutions**:
1. Flatten template structure
2. Reduce use of includes/extends
3. Increase limit: `.with_max_recursion_depth(20)`
4. Check for circular includes

#### Issue 4: Validation Errors

**Symptom**: `RenderingError::ValidationFailed`

**Causes**:
- Template generates invalid Rust syntax
- Unbalanced delimiters
- Invalid identifiers

**Solutions**:
1. Check template syntax
2. Verify data passed to template
3. Use frozen sections for complex code
4. Temporarily disable validation for debugging

### Debugging Tips

#### Enable Detailed Logging

```rust
use tracing_subscriber;

tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .init();
```

#### Inspect Context Variables

```rust
let context = RenderContext::new();
context.insert("debug", &true)?;

// In template:
// {% if debug %}
// {{ __tera_context }}  // Prints entire context
// {% endif %}
```

#### Validate Templates Before Rendering

```rust
use tera::Tera;

let tera = Tera::default();
match tera.add_raw_template("test", template_content) {
    Ok(_) => println!("Template syntax is valid"),
    Err(e) => eprintln!("Template syntax error: {}", e),
}
```

## Best Practices

### 1. Always Use SafeRenderer

❌ **Bad**:
```rust
let tera = Tera::default();
let output = tera.render("template", &context)?;
```

✅ **Good**:
```rust
let renderer = SafeRenderer::new(RenderConfig::default())?;
let output = renderer.render_safe("template", &context)?;
```

### 2. Set Appropriate Limits

❌ **Bad**:
```rust
let config = RenderConfig::default()
    .with_timeout_ms(30_000)  // Too long
    .with_max_output_size(100 * 1024 * 1024);  // Too large
```

✅ **Good**:
```rust
let config = RenderConfig::default()
    .with_timeout_ms(5_000)   // Reasonable
    .with_max_output_size(10 * 1024 * 1024);  // Reasonable
```

### 3. Always Enable Validation in Production

❌ **Bad**:
```rust
let config = RenderConfig::default()
    .with_syntax_validation(false)
    .with_security_checks(false);
```

✅ **Good**:
```rust
let config = RenderConfig::default()
    .with_syntax_validation(true)
    .with_security_checks(true);
```

### 4. Handle Errors Gracefully

❌ **Bad**:
```rust
let output = renderer.render_safe("template", &context).unwrap();
```

✅ **Good**:
```rust
match renderer.render_safe("template", &context) {
    Ok(output) => process_output(output),
    Err(e) => {
        eprintln!("Rendering failed: {}", e);
        // Log error, use fallback, or return error
    }
}
```

### 5. Use Context Scoping for Nested Templates

❌ **Bad**:
```rust
let context = RenderContext::new();
// Variables leak across templates
```

✅ **Good**:
```rust
let root = Arc::new(RenderContext::new());
let child = RenderContext::child(root);
// Clear scoping, no pollution
```

### 6. Test Templates Thoroughly

```rust
#[test]
fn test_all_templates() {
    let renderer = SafeRenderer::from_directory("templates", config)?;
    let test_data = load_test_data();

    for template_name in template_names {
        let context = create_context(&test_data);
        let result = renderer.render_safe(template_name, &context);
        assert!(result.is_ok(), "Template {} failed", template_name);
    }
}
```

### 7. Use Whitelist for Includes

```rust
let config = RenderConfig::default()
    .with_include_whitelist("templates/common/header.tera")
    .with_include_whitelist("templates/common/footer.tera");
```

### 8. Monitor Rendering Metrics

```rust
let guard = RenderGuard::new();
// ... rendering ...
let metrics = guard.commit();

println!("Rendering metrics:");
println!("  Duration: {:?}", metrics.duration);
println!("  Output size: {} bytes", metrics.output_size);
println!("  Recursion depth: {}", metrics.max_recursion_reached);
println!("  Validation errors: {}", metrics.validation_errors);
```

## Examples

See the `tests/template_rendering_tests.rs` file for comprehensive examples of:

- Basic safe rendering
- Timeout handling
- Output size limits
- Recursion depth limits
- Validation error handling
- Security violation detection
- Malicious template prevention
- Error recovery
- Resource cleanup

## Further Reading

- [Tera Template Documentation](https://tera.netlify.app/docs/)
- [Toyota Production System: Poka-Yoke](../docs/TOYOTA_PRODUCTION_SYSTEM.md)
- [Domain-Driven Design Code Generation](../docs/DDD_CODE_GENERATION.md)
