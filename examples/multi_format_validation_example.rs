//! Multi-Format Validation Example
//!
//! Demonstrates TypeScript, YAML, JSON, and OpenAPI validation capabilities
//! without requiring external compilers or dependencies beyond serde.

use spreadsheet_mcp::template::multi_format_validator::{
    JsonValidator, OpenApiValidator, TypeScriptValidator, YamlValidator,
};

fn main() {
    println!("=== Multi-Format Validation Example ===\n");

    // TypeScript Validation
    typescript_validation_example();

    // YAML Validation
    yaml_validation_example();

    // JSON Validation
    json_validation_example();

    // OpenAPI Validation
    openapi_validation_example();
}

fn typescript_validation_example() {
    println!("--- TypeScript Validation ---");

    let mut validator = TypeScriptValidator::new();

    // Valid TypeScript
    let valid_code = r#"
import { Component } from 'react';

interface Props {
    title: string;
    count: number;
}

export function MyComponent(props: Props) {
    return <div>{props.title}: {props.count}</div>;
}
"#;

    match validator.validate(valid_code, "example.tsx") {
        Ok(report) => {
            println!("✓ Valid TypeScript: {} errors, {} warnings",
                     report.error_count, report.warning_count);
        }
        Err(e) => println!("✗ Validation failed: {}", e),
    }

    // Invalid TypeScript (unbalanced braces)
    validator.reset();
    let invalid_code = "function test() { return { key: 'value'; }";
    match validator.validate(invalid_code, "invalid.ts") {
        Ok(report) => {
            println!("✓ Invalid TypeScript detected: {} errors", report.error_count);
            for issue in &report.issues {
                println!("  - {}", issue.message);
            }
        }
        Err(e) => println!("✗ Validation failed: {}", e),
    }

    println!();
}

fn yaml_validation_example() {
    println!("--- YAML Validation ---");

    let validator = YamlValidator::new();

    // Valid YAML
    let valid_yaml = r#"
name: My Project
version: 1.0.0
dependencies:
  - lodash
  - react
  - typescript
config:
  port: 3000
  debug: true
"#;

    match validator.validate(valid_yaml, "config.yaml") {
        Ok(report) => {
            println!("✓ Valid YAML: {} errors, {} warnings",
                     report.error_count, report.warning_count);
        }
        Err(e) => println!("✗ Validation failed: {}", e),
    }

    // Invalid YAML (tab indentation)
    let invalid_yaml = "key: value\n\tindented: with_tab";
    match validator.validate(invalid_yaml, "invalid.yaml") {
        Ok(report) => {
            println!("✓ Invalid YAML detected: {} errors", report.error_count);
            for issue in &report.issues {
                println!("  - {}: {}", issue.severity.as_str(), issue.message);
                if let Some(suggestion) = &issue.suggestion {
                    println!("    Suggestion: {}", suggestion);
                }
            }
        }
        Err(e) => println!("✗ Validation failed: {}", e),
    }

    println!();
}

fn json_validation_example() {
    println!("--- JSON Validation ---");

    let validator = JsonValidator::new();

    // Valid JSON
    let valid_json = r#"
{
  "name": "My API",
  "version": "1.0.0",
  "endpoints": [
    {
      "path": "/users",
      "methods": ["GET", "POST"]
    }
  ],
  "config": {
    "timeout": 3000
  }
}
"#;

    match validator.validate(valid_json, "api.json") {
        Ok(report) => {
            println!("✓ Valid JSON: {} errors", report.error_count);
        }
        Err(e) => println!("✗ Validation failed: {}", e),
    }

    // Invalid JSON (trailing comma)
    let invalid_json = r#"{"key": "value",}"#;
    match validator.validate(invalid_json, "invalid.json") {
        Ok(report) => {
            println!("✓ Invalid JSON detected: {} errors", report.error_count);
            for issue in &report.issues {
                println!("  - {}", issue.message);
                if let Some(suggestion) = &issue.suggestion {
                    println!("    Suggestion: {}", suggestion);
                }
            }
        }
        Err(e) => println!("✗ Validation failed: {}", e),
    }

    println!();
}

fn openapi_validation_example() {
    println!("--- OpenAPI Validation ---");

    let validator = OpenApiValidator::new();

    // Valid OpenAPI
    let valid_openapi = r#"
openapi: 3.0.0
info:
  title: Example API
  description: An example API specification
  version: 1.0.0

paths:
  /users:
    get:
      summary: List all users
      responses:
        '200':
          description: Successful response
"#;

    match validator.validate(valid_openapi, "openapi.yaml") {
        Ok(report) => {
            println!("✓ Valid OpenAPI: {} errors, {} warnings",
                     report.error_count, report.warning_count);
        }
        Err(e) => println!("✗ Validation failed: {}", e),
    }

    // Invalid OpenAPI (missing required field)
    let invalid_openapi = r#"
info:
  title: Example API
  version: 1.0.0
paths: {}
"#;

    match validator.validate(invalid_openapi, "invalid_openapi.yaml") {
        Ok(report) => {
            println!("✓ Invalid OpenAPI detected: {} errors", report.error_count);
            for issue in &report.issues {
                println!("  - {}", issue.message);
                if let Some(suggestion) = &issue.suggestion {
                    println!("    Suggestion: {}", suggestion);
                }
            }
        }
        Err(e) => println!("✗ Validation failed: {}", e),
    }

    println!();
}

// Helper trait for displaying severity
trait SeverityDisplay {
    fn as_str(&self) -> &str;
}

impl SeverityDisplay for spreadsheet_mcp::codegen::validation::ValidationSeverity {
    fn as_str(&self) -> &str {
        match self {
            Self::Error => "ERROR",
            Self::Warning => "WARNING",
            Self::Info => "INFO",
        }
    }
}
