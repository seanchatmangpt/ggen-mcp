# Multi-Format Validation Quick Start

## 5-Minute Guide

### Installation

No additional dependencies required. Uses existing:
- `serde_json`
- `serde_yaml`
- `regex`
- `anyhow`

### Basic Usage

#### TypeScript

```rust
use spreadsheet_mcp::template::TypeScriptValidator;

let mut validator = TypeScriptValidator::new();
let code = r#"
import { Component } from 'react';
export function MyComponent() {
    return <div>Hello</div>;
}
"#;

let report = validator.validate(code, "component.tsx")?;
if report.has_errors() {
    eprintln!("Errors found:");
    for issue in &report.issues {
        eprintln!("  {}: {}", issue.location.unwrap_or_default(), issue.message);
    }
}
```

#### YAML

```rust
use spreadsheet_mcp::template::YamlValidator;

let validator = YamlValidator::new();
let yaml = "name: project\nversion: 1.0.0";

let report = validator.validate(yaml, "config.yaml")?;
assert!(!report.has_errors());
```

#### JSON

```rust
use spreadsheet_mcp::template::JsonValidator;

let validator = JsonValidator::new();
let json = r#"{"key": "value"}"#;

let report = validator.validate(json, "data.json")?;
assert!(!report.has_errors());
```

#### OpenAPI

```rust
use spreadsheet_mcp::template::OpenApiValidator;

let validator = OpenApiValidator::new();
let spec = r#"
openapi: 3.0.0
info:
  title: API
  version: 1.0.0
paths: {}
"#;

let report = validator.validate(spec, "openapi.yaml")?;
assert!(!report.has_errors());
```

### Via GeneratedCodeValidator

```rust
use spreadsheet_mcp::codegen::validation::GeneratedCodeValidator;

let validator = GeneratedCodeValidator::new();

// One validator, multiple formats
let ts_report = validator.validate_typescript_syntax(ts_code, "file.ts")?;
let yaml_report = validator.validate_yaml_syntax(yaml_content, "config.yaml")?;
let json_report = validator.validate_json_syntax(json_content, "data.json")?;
let openapi_report = validator.validate_openapi_spec(spec_content, "openapi.yaml")?;
```

## Common Patterns

### Pattern 1: Validate and Fail Fast

```rust
let report = validator.validate(code, filename)?;
if report.has_errors() {
    return Err(anyhow!("Validation failed: {} errors", report.error_count));
}
// Continue with valid code
```

### Pattern 2: Validate with Warnings

```rust
let report = validator.validate(code, filename)?;
if report.has_errors() {
    return Err(anyhow!("Validation failed"));
}
if report.warning_count > 0 {
    tracing::warn!("{} warnings in {}", report.warning_count, filename);
}
```

### Pattern 3: Detailed Error Reporting

```rust
let report = validator.validate(code, filename)?;
for issue in &report.issues {
    let level = match issue.severity {
        ValidationSeverity::Error => "ERROR",
        ValidationSeverity::Warning => "WARN",
        ValidationSeverity::Info => "INFO",
    };

    eprintln!("[{}] {}: {}",
              level,
              issue.location.as_deref().unwrap_or(""),
              issue.message);

    if let Some(suggestion) = &issue.suggestion {
        eprintln!("      → {}", suggestion);
    }
}
```

### Pattern 4: Reset State Between Files

```rust
let mut validator = TypeScriptValidator::new();

for file in files {
    let code = fs::read_to_string(file)?;
    let report = validator.validate(&code, file)?;
    // Process report...
    validator.reset();  // Clear state before next file
}
```

## Testing

### Run Tests

```bash
# All multi-format validation tests (44 tests)
cargo test --test multi_format_validation_tests

# Specific validator
cargo test --test multi_format_validation_tests typescript
cargo test --test multi_format_validation_tests yaml

# Run example
cargo run --example multi_format_validation_example
```

### Test Output Example

```
running 44 tests
test typescript_validates_balanced_braces ... ok
test typescript_detects_unbalanced_braces ... ok
test yaml_validates_simple_structure ... ok
test yaml_detects_tab_indentation ... ok
test json_validates_simple_object ... ok
test json_detects_trailing_comma ... ok
test openapi_validates_minimal_spec ... ok
test openapi_detects_missing_version ... ok

test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured
```

## Common Errors and Solutions

### TypeScript

**Error**: "Unmatched closing brace '}'"
- **Cause**: Missing opening brace
- **Fix**: Add `{` before the closing brace

**Error**: "Typo in import: 'form' should be 'from'"
- **Cause**: Common typo
- **Fix**: Change `form` to `from`

**Warning**: "Interface 'myInterface' should use PascalCase"
- **Cause**: Naming convention violation
- **Fix**: Rename to `MyInterface`

### YAML

**Error**: "YAML does not allow tabs for indentation"
- **Cause**: Tab characters in YAML
- **Fix**: Replace tabs with spaces

**Warning**: "Inconsistent indentation (should be multiples of 2)"
- **Cause**: Odd-number indentation
- **Fix**: Use 2, 4, 6, etc. spaces

### JSON

**Error**: "Trailing comma not allowed in JSON"
- **Cause**: Extra comma before `}` or `]`
- **Fix**: Remove trailing comma

**Error**: "JSON keys must be strings in double quotes"
- **Cause**: Unquoted key
- **Fix**: Add quotes: `"key": value`

### OpenAPI

**Error**: "Missing required field 'openapi'"
- **Cause**: No version specified
- **Fix**: Add `openapi: 3.0.0` at top

**Error**: "Missing required field 'info.title'"
- **Cause**: Incomplete info section
- **Fix**: Add `title` field under `info`

## Integration with Code Generation

### Example: Validate Generated Code

```rust
use spreadsheet_mcp::codegen::validation::GeneratedCodeValidator;

// In your code generation pipeline
let rendered_code = template.render(&context)?;

// Determine format from file extension
let validator = GeneratedCodeValidator::new();
let report = match output_file.extension().and_then(|s| s.to_str()) {
    Some("ts") | Some("tsx") => {
        validator.validate_typescript_syntax(&rendered_code, output_file.to_str().unwrap())?
    }
    Some("yaml") | Some("yml") => {
        validator.validate_yaml_syntax(&rendered_code, output_file.to_str().unwrap())?
    }
    Some("json") => {
        validator.validate_json_syntax(&rendered_code, output_file.to_str().unwrap())?
    }
    Some("rs") => {
        validator.validate_code(&rendered_code, output_file.to_str().unwrap())?
    }
    _ => ValidationReport::new(),  // Skip unknown formats
};

if report.has_errors() {
    return Err(anyhow!("Generated code validation failed: {} errors", report.error_count));
}

// Write validated code
fs::write(output_file, rendered_code)?;
```

## Performance Tips

1. **Reuse Validators**: Create once, validate many times
2. **Reset State**: Call `reset()` for stateful validators (TypeScript)
3. **Parallel Validation**: Validators are independent, can run in parallel
4. **Skip Unknown Formats**: Return empty report for unsupported extensions

## Documentation

- **Full Guide**: `docs/MULTI_FORMAT_VALIDATION.md`
- **Summary**: `MULTI_FORMAT_VALIDATION_SUMMARY.md`
- **Example**: `examples/multi_format_validation_example.rs`
- **Tests**: `tests/multi_format_validation_tests.rs`

## Quick Reference

| Validator | Format | Primary Check | Speed |
|-----------|--------|---------------|-------|
| TypeScriptValidator | .ts, .tsx | Pattern-based syntax | ~1-2ms |
| YamlValidator | .yaml, .yml | serde_yaml parser | ~0.5-1ms |
| JsonValidator | .json | serde_json parser | ~0.5-1ms |
| OpenApiValidator | .yaml, .yml | YAML + OpenAPI schema | ~1-3ms |

## SPR Summary

Multi-format validators ready. Import from `spreadsheet_mcp::template`. Validate TS/YAML/JSON/OpenAPI. Fail-fast pattern. No external compilers. 44 tests pass. Zero new dependencies.

**Quick start**: `use spreadsheet_mcp::template::{TypeScriptValidator, YamlValidator, JsonValidator, OpenApiValidator};`

**Pattern**: `validator.validate(code, filename)? → ValidationReport → check .has_errors()`

---

**Get Started**: `cargo run --example multi_format_validation_example`
