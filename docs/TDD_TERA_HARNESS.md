# Tera Template Test Harness - Chicago-Style TDD

## Overview

This document describes the comprehensive Chicago-style TDD test harness for Tera template rendering and validation in the ggen-mcp project. The harness provides production-ready tools for testing all 17+ templates with behavior verification, code quality assertions, and golden file testing.

## Chicago-Style TDD Principles

The harness follows Chicago-style TDD (also known as Classical TDD) principles:

1. **Test Actual Behavior** - Tests verify observable outcomes and side effects, not internal implementation
2. **Real Dependencies** - Uses actual Tera engine and file system, not mocks
3. **Integration Focus** - Tests how components work together in realistic scenarios
4. **State Verification** - Validates system state after operations complete

## Architecture

### Core Components

#### 1. TemplateTestHarness

The main test harness providing comprehensive template testing capabilities:

```rust
use harness::{TemplateTestHarness, HarnessConfig};

let mut harness = TemplateTestHarness::new(
    "templates/",      // Template directory
    "tests/fixtures/"  // Fixture directory
)?;
```

**Features:**
- Template rendering from strings and files
- Template syntax validation
- Context management and verification
- Generated code validation
- Golden file (snapshot) testing
- Behavior verification

#### 2. TemplateContextBuilder

Fluent API for building test contexts:

```rust
use harness::TemplateContextBuilder;

let context = TemplateContextBuilder::new()
    .entity("User")
    .field("name", "String")
    .field("email", "Email")
    .flag("has_id", true)
    .flag("has_timestamps", true)
    .value("description", "User entity")
    .build()?;
```

#### 3. HarnessConfig

Configuration for test behavior:

```rust
let config = HarnessConfig {
    validate_syntax: true,      // Validate Rust syntax
    security_checks: true,       // Check for security issues
    check_variable_usage: true,  // Verify context usage
    update_golden_files: false,  // Update golden files on mismatch
    compile_check: false,        // Run rustc (expensive)
};
```

## Template Coverage

The harness provides comprehensive coverage for all 17+ templates:

### Core Domain Templates

1. **aggregate.rs.tera** - Domain aggregate roots
2. **command.rs.tera** - CQRS commands
3. **domain_entity.rs.tera** - Rich domain entities (11,982 bytes)
4. **domain_mod.rs.tera** - Domain module structure
5. **domain_service.rs.tera** - Domain services (14,226 bytes)
6. **value_object.rs.tera** - Value objects
7. **value_objects.rs.tera** - Value object collections

### Infrastructure Templates

8. **repositories.rs.tera** - Repository patterns
9. **services.rs.tera** - Application services
10. **handlers.rs.tera** - Request handlers
11. **policies.rs.tera** - Business policies
12. **tests.rs.tera** - Test modules

### MCP Tool Templates

13. **mcp_tool_handler.rs.tera** - MCP tool handlers (13,514 bytes)
14. **mcp_tool_params.rs.tera** - Parameter structs (4,016 bytes)
15. **mcp_tools.rs.tera** - Tool definitions (3,777 bytes)
16. **mcp_resource_handler.rs.tera** - Resource handlers (18,645 bytes)

### Application Templates

17. **application_mod.rs.tera** - Application layer module

## Usage Guide

### Basic Template Rendering

```rust
#[test]
fn test_render_domain_entity() {
    let mut harness = create_harness()?;

    let context = TemplateContextBuilder::new()
        .entity("User")
        .field("name", "String")
        .field("email", "Email")
        .flag("has_id", true)
        .flag("has_validation", true)
        .build()?;

    let output = harness.render_from_file(
        "domain_entity.rs.tera",
        &context
    )?;

    assert!(output.contains("pub struct User"));
}
```

### Rendering with Context Files

```rust
#[test]
fn test_render_with_fixture() {
    let mut harness = create_harness()?;

    // Loads context from tests/fixtures/tera/contexts/user_aggregate.json
    let output = harness.render_with_context_file(
        "domain_entity.rs.tera",
        "user_aggregate.json"
    )?;

    assert!(output.contains("pub struct User"));
}
```

### Template Syntax Validation

```rust
#[test]
fn test_validate_syntax() {
    let mut harness = create_harness()?;

    let template = "{% if has_id %}pub id: String{% endif %}";

    let result = harness.validate_template_syntax(template)?;
    assert!(result.valid);
    assert!(result.errors.is_empty());
}
```

### Context Usage Verification

```rust
#[test]
fn test_verify_context_usage() {
    let mut harness = create_harness()?;

    let template = "Entity: {{ entity_name }}";
    harness.render_from_string("test", template, &context)?;

    let report = harness.verify_context_usage("test", &context)?;

    // Check for unused context variables
    assert!(report.unused_context_vars.is_empty());

    // Check for missing required variables
    assert!(report.missing_template_vars.is_empty());
}
```

### Code Quality Validation

```rust
#[test]
fn test_validate_generated_code() {
    let harness = create_harness()?;

    let validation = harness.validate_rust_syntax(&generated_code)?;

    assert!(validation.valid);
    assert!(validation.metrics.has_imports);
    assert!(validation.metrics.has_docs);
    assert!(validation.metrics.has_tests);
}
```

### Golden File Testing

```rust
#[test]
fn test_golden_file_comparison() {
    let mut harness = create_harness()?;

    let output = harness.render_with_context_file(
        "domain_entity.rs.tera",
        "user_aggregate.json"
    )?;

    // Compares against tests/fixtures/tera/expected/UserAggregate.rs
    harness.assert_matches_golden("UserAggregate.rs", &output)?;
}
```

### Behavior Verification

#### Conditional Testing

```rust
#[test]
fn test_conditionals() {
    let mut harness = create_harness()?;

    let template = "{% if enabled %}ON{% else %}OFF{% endif %}";

    let mut true_ctx = TeraContext::new();
    true_ctx.insert("enabled", &true);

    let mut false_ctx = TeraContext::new();
    false_ctx.insert("enabled", &false);

    harness.verify_conditionals(
        template,
        &true_ctx,
        &false_ctx,
        "ON",    // Expected when true
        "OFF"    // Expected when false
    )?;
}
```

#### Loop Iteration Testing

```rust
#[test]
fn test_loop_iteration() {
    let mut harness = create_harness()?;

    let template = "{% for item in items %}{{ item }}{% endfor %}";

    let mut context = TeraContext::new();
    context.insert("items", &vec!["A", "B", "C"]);

    harness.verify_loop_iteration(
        template,
        &context,
        3  // Expected iteration count
    )?;
}
```

#### Filter Application Testing

```rust
#[test]
fn test_filters() {
    let mut harness = create_harness()?;

    let template = "Name: {{ name | upper }}";

    let mut context = TeraContext::new();
    context.insert("name", "john");

    harness.verify_filter_applied(
        template,
        &context,
        "JOHN"  // Expected transformation
    )?;
}
```

### Content Verification

```rust
#[test]
fn test_output_contains() {
    let mut harness = create_harness()?;

    harness.render_from_file("template.rs.tera", &context)?;

    // Verify output contains expected strings
    harness.verify_contains("template.rs.tera", &[
        "pub struct User",
        "impl User",
        "pub fn new"
    ])?;

    // Verify output doesn't contain forbidden strings
    harness.verify_not_contains("template.rs.tera", &[
        "TODO",
        "FIXME",
        "panic!"
    ])?;
}
```

## Test Fixtures

### Directory Structure

```
tests/fixtures/tera/
├── contexts/           # JSON context files
│   ├── user_aggregate.json
│   ├── mcp_tool.json
│   ├── domain_service.json
│   └── list_tools.json
└── expected/           # Golden files for snapshot testing
    ├── UserAggregate.rs
    └── CreateUserTool.rs
```

### Context File Format

Context files are JSON documents with template variables:

```json
{
  "entity_name": "User",
  "description": "User aggregate root",
  "has_id": true,
  "has_timestamps": true,
  "has_validation": true,
  "has_builder": true,
  "fields": [
    {
      "name": "username",
      "rust_type": "String",
      "description": "Unique username",
      "required": true
    }
  ],
  "invariants": [
    {
      "description": "Username length check",
      "expression": "self.username.len() >= 3",
      "message": "Username must be at least 3 characters"
    }
  ]
}
```

## Code Quality Assertions

The harness validates multiple aspects of generated code:

### 1. Balanced Delimiters

Ensures all brackets, braces, and parentheses are properly matched:

```rust
assert!(validation.valid);  // No unbalanced delimiters
```

### 2. Rust Patterns

Checks for common Rust code patterns:

- Import statements (`use` declarations)
- Documentation comments (`///`, `//!`)
- Type annotations
- Proper struct/enum/trait definitions

### 3. Security Checks

Warns about potentially dangerous patterns:

- `unsafe` blocks
- `std::process::Command` usage
- File system operations (`fs::write`, `fs::remove`)
- Network operations

### 4. Code Metrics

Calculates quality metrics:

```rust
pub struct CodeMetrics {
    pub line_count: usize,
    pub char_count: usize,
    pub has_imports: bool,
    pub has_docs: bool,
    pub has_tests: bool,
}
```

## Best Practices

### 1. Use Context Builder for Clarity

✅ **Good:**
```rust
let context = TemplateContextBuilder::new()
    .entity("User")
    .field("name", "String")
    .flag("has_id", true)
    .build()?;
```

❌ **Avoid:**
```rust
let mut context = TeraContext::new();
context.insert("entity_name", "User");
context.insert("has_id", true);
// ... manual insertion
```

### 2. Use Golden Files for Complex Templates

For templates generating >100 lines, use golden file testing:

```rust
harness.assert_matches_golden("ComplexOutput.rs", &rendered)?;
```

### 3. Test Template Features Independently

Test conditionals, loops, and filters separately before integration:

```rust
#[test]
fn test_conditional_logic() { /* ... */ }

#[test]
fn test_loop_iteration() { /* ... */ }

#[test]
fn test_filter_application() { /* ... */ }
```

### 4. Verify Both Success and Error Paths

```rust
#[test]
fn test_valid_template() {
    assert!(result.is_ok());
}

#[test]
fn test_invalid_template() {
    assert!(result.is_err());
}
```

## Example Test Suites

### Full Domain Entity Test

```rust
#[test]
fn test_full_domain_entity_generation() {
    let mut harness = create_harness()?;

    let context = TemplateContextBuilder::new()
        .entity("Order")
        .field("order_number", "String")
        .field("customer_id", "String")
        .field("total_amount", "Decimal")
        .field("status", "OrderStatus")
        .flag("has_id", true)
        .flag("has_timestamps", true)
        .flag("has_validation", true)
        .flag("has_builder", true)
        .value("description", "Order aggregate root")
        .build()?;

    let output = harness.render_from_file(
        "domain_entity.rs.tera",
        &context
    )?;

    // Verify structure
    assert!(output.contains("pub struct Order"));
    assert!(output.contains("pub enum OrderError"));
    assert!(output.contains("impl Order"));
    assert!(output.contains("pub struct OrderBuilder"));

    // Validate code quality
    let validation = harness.validate_rust_syntax(&output)?;
    assert!(validation.valid);
    assert!(validation.metrics.has_docs);
    assert!(validation.metrics.has_tests);

    // Compare against golden file
    harness.assert_matches_golden("Order.rs", &output)?;
}
```

### Full MCP Tool Test

```rust
#[test]
fn test_full_mcp_tool_generation() {
    let mut harness = create_harness()?;

    let output = harness.render_with_context_file(
        "mcp_tool_handler.rs.tera",
        "mcp_tool.json"
    )?;

    // Verify all components present
    harness.verify_contains("mcp_tool_handler.rs.tera", &[
        "pub struct CreateUserParams",
        "pub struct CreateUserResponse",
        "pub struct CreateUserMetadata",
        "pub async fn create_user",
        "fn validate_params",
    ])?;

    // Validate code
    let validation = harness.validate_rust_syntax(&output)?;
    assert!(validation.valid);

    // No security issues
    let security_warnings: Vec<_> = validation.warnings.iter()
        .filter(|w| w.contains("unsafe") || w.contains("Command"))
        .collect();
    assert!(security_warnings.is_empty());
}
```

## Performance Considerations

### Baseline Performance

The harness includes performance baseline tests:

```rust
#[test]
fn test_render_performance() {
    let mut harness = create_harness()?;
    let start = Instant::now();

    for _ in 0..100 {
        harness.render_from_file("template.rs.tera", &context)?;
    }

    let avg_ms = start.elapsed().as_millis() / 100;
    assert!(avg_ms < 50, "Average render time should be < 50ms");
}
```

### Optimization Tips

1. **Reuse harness instances** - Creating a new harness is expensive
2. **Cache contexts** - Build contexts once, reuse for multiple tests
3. **Parallel test execution** - Tests are independent and thread-safe
4. **Skip expensive checks** - Set `compile_check: false` for most tests

## Troubleshooting

### Template Not Found

**Error:** `Template not found: domain_entity.rs.tera`

**Solution:** Ensure template path is relative to the template directory:
```rust
harness.render_from_file("domain_entity.rs.tera", &context)
// Not: "templates/domain_entity.rs.tera"
```

### Context Variable Missing

**Error:** `Variable 'entity_name' not found`

**Solution:** Ensure all required variables are in context:
```rust
let context = TemplateContextBuilder::new()
    .entity("User")  // Sets 'entity_name' variable
    .build()?;
```

### Golden File Mismatch

**Error:** `Output does not match golden file`

**Solution:**
1. Review the diff in the error message
2. If change is intentional, set `update_golden_files: true`
3. Run test again to update the golden file
4. Reset `update_golden_files: false` for normal testing

### Unbalanced Delimiters

**Error:** `Unmatched closing delimiter: }`

**Solution:** Check for balanced braces in conditionals and loops:
```tera
{% if condition %}
    {# Content here #}
{% endif %}  {# Don't forget the endif! #}
```

## API Reference

### TemplateTestHarness

#### Constructor Methods

- `new(template_dir, fixture_dir) -> Result<Self>` - Create new harness
- `with_config(template_dir, fixture_dir, config) -> Result<Self>` - Create with custom config

#### Rendering Methods

- `render_from_string(name, template, context) -> Result<String>` - Render from string
- `render_from_file(template_file, context) -> Result<String>` - Render from file
- `render_with_context_file(template_file, context_file) -> Result<String>` - Render with JSON context

#### Validation Methods

- `validate_template_syntax(template) -> Result<ValidationResult>` - Check template syntax
- `validate_rust_syntax(code) -> Result<CodeValidation>` - Check generated code
- `verify_context_usage(template_name, context) -> Result<UsageReport>` - Check context variable usage

#### Behavior Verification Methods

- `verify_renders_successfully(template_file, context) -> Result<()>` - Check rendering succeeds
- `verify_contains(template_name, expected) -> Result<()>` - Check output contains strings
- `verify_not_contains(template_name, forbidden) -> Result<()>` - Check output excludes strings
- `verify_conditionals(template, true_ctx, false_ctx, expected_true, expected_false) -> Result<()>`
- `verify_loop_iteration(template, context, expected_count) -> Result<()>`
- `verify_filter_applied(template, context, expected_transformation) -> Result<()>`

#### Golden File Methods

- `assert_matches_golden(golden_file, rendered) -> Result<()>` - Compare against golden file

#### Utility Methods

- `template_exists(template_file) -> bool` - Check if template is loaded
- `list_templates() -> Vec<String>` - Get all template names
- `extract_template_variables(template_name) -> Result<HashSet<String>>` - Get used variables
- `load_context_from_file(context_file) -> Result<TeraContext>` - Load JSON context
- `context_from_json(json_str) -> Result<TeraContext>` - Parse JSON to context

### TemplateContextBuilder

- `new() -> Self` - Create new builder
- `entity(name: &str) -> Self` - Set entity name
- `field(name: &str, rust_type: &str) -> Self` - Add field
- `flag(name: &str, value: bool) -> Self` - Set boolean flag
- `value(name: &str, value: &str) -> Self` - Set string value
- `custom<T: Serialize>(name: &str, value: T) -> Result<Self>` - Set custom value
- `build() -> Result<TeraContext>` - Build the context

## Integration with CI/CD

### GitHub Actions Example

```yaml
name: Template Tests

on: [push, pull_request]

jobs:
  template-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run template harness tests
        run: cargo test --test tera_harness_tests

      - name: Check golden files are up to date
        run: |
          cargo test --test tera_harness_tests
          git diff --exit-code tests/fixtures/tera/expected/
```

## Contributing

When adding new templates:

1. Create corresponding test in `tests/tera_harness_tests.rs`
2. Add context fixture in `tests/fixtures/tera/contexts/`
3. Generate golden file with `update_golden_files: true`
4. Add template to coverage test
5. Document template-specific features

## References

- [Tera Template Documentation](https://tera.netlify.app/)
- [Chicago-Style TDD](https://github.com/testdouble/contributing-tests/wiki/Chicago-style-TDD)
- [Snapshot Testing Best Practices](https://kentcdodds.com/blog/effective-snapshot-testing)

## License

This test harness is part of the ggen-mcp project and is licensed under Apache-2.0.
