//! Multi-Format Validation Tests
//!
//! Comprehensive tests for TypeScript, YAML, JSON, and OpenAPI validators
//! following Chicago-style TDD principles (state-based, real implementations)

use spreadsheet_mcp::template::multi_format_validator::{
    JsonValidator, OpenApiValidator, TypeScriptValidator, YamlValidator,
};

// =============================================================================
// TypeScript Validator Tests
// =============================================================================

#[test]
fn typescript_validates_balanced_braces() {
    let mut validator = TypeScriptValidator::new();

    let valid_code = r#"
function test() {
    return { key: 'value' };
}
"#;
    let report = validator.validate(valid_code, "test.ts").unwrap();
    assert!(!report.has_errors(), "Valid code should not have errors");
}

#[test]
fn typescript_detects_unbalanced_braces() {
    let mut validator = TypeScriptValidator::new();

    let invalid_code = r#"
function test() {
    return { key: 'value';
}
"#;
    let report = validator.validate(invalid_code, "test.ts").unwrap();
    assert!(report.has_errors(), "Unbalanced braces should cause error");
    assert!(
        report.error_count >= 1,
        "Should report unbalanced brace error"
    );
}

#[test]
fn typescript_detects_unbalanced_brackets() {
    let mut validator = TypeScriptValidator::new();

    let invalid_code = "const arr = [1, 2, 3;";
    let report = validator.validate(invalid_code, "test.ts").unwrap();
    assert!(report.has_errors(), "Unbalanced brackets should cause error");
}

#[test]
fn typescript_detects_unbalanced_parens() {
    let mut validator = TypeScriptValidator::new();

    let invalid_code = "function test(arg1, arg2 {";
    let report = validator.validate(invalid_code, "test.ts").unwrap();
    assert!(
        report.has_errors(),
        "Unbalanced parentheses should cause error"
    );
}

#[test]
fn typescript_validates_import_syntax() {
    let mut validator = TypeScriptValidator::new();

    let valid_imports = r#"
import { Component } from 'react';
import type { Props } from './types';
import * as Utils from './utils';
"#;
    let report = validator.validate(valid_imports, "test.ts").unwrap();
    assert!(!report.has_errors(), "Valid imports should not have errors");
}

#[test]
fn typescript_detects_import_typo() {
    let mut validator = TypeScriptValidator::new();

    let invalid_import = "import { Component } form 'react';";
    let report = validator.validate(invalid_import, "test.ts").unwrap();
    assert!(
        report.has_errors(),
        "Typo 'form' instead of 'from' should cause error"
    );
}

#[test]
fn typescript_validates_export_syntax() {
    let mut validator = TypeScriptValidator::new();

    let valid_exports = r#"
export function test() {}
export const value = 42;
export default class MyClass {}
"#;
    let report = validator.validate(valid_exports, "test.ts").unwrap();
    assert!(!report.has_errors(), "Valid exports should not have errors");
}

#[test]
fn typescript_detects_incomplete_export() {
    let mut validator = TypeScriptValidator::new();

    let invalid_export = "export";
    let report = validator.validate(invalid_export, "test.ts").unwrap();
    assert!(
        report.has_errors(),
        "Incomplete export should cause error"
    );
}

#[test]
fn typescript_detects_reserved_word_as_identifier() {
    let mut validator = TypeScriptValidator::new();

    let invalid_code = "const class = 'test';";
    let report = validator.validate(invalid_code, "test.ts").unwrap();
    assert!(
        report.has_errors(),
        "Reserved word 'class' should cause error"
    );
}

#[test]
fn typescript_validates_interface_naming() {
    let mut validator = TypeScriptValidator::new();

    let lowercase_interface = "interface myInterface { key: string; }";
    let report = validator
        .validate(lowercase_interface, "test.ts")
        .unwrap();
    assert!(
        report.warning_count > 0,
        "Lowercase interface name should warn"
    );

    validator.reset();
    let pascalcase_interface = "interface MyInterface { key: string; }";
    let report = validator
        .validate(pascalcase_interface, "test.ts")
        .unwrap();
    assert_eq!(report.warning_count, 0, "PascalCase should not warn");
}

#[test]
fn typescript_validates_type_alias_naming() {
    let mut validator = TypeScriptValidator::new();

    let lowercase_type = "type myType = string | number;";
    let report = validator.validate(lowercase_type, "test.ts").unwrap();
    assert!(
        report.warning_count > 0,
        "Lowercase type alias should warn"
    );

    validator.reset();
    let pascalcase_type = "type MyType = string | number;";
    let report = validator.validate(pascalcase_type, "test.ts").unwrap();
    assert_eq!(report.warning_count, 0, "PascalCase should not warn");
}

#[test]
fn typescript_validates_function_naming() {
    let mut validator = TypeScriptValidator::new();

    let uppercase_function = "function MyFunction() {}";
    let report = validator.validate(uppercase_function, "test.ts").unwrap();
    assert!(
        report.warning_count > 0,
        "Uppercase function name should warn"
    );

    validator.reset();
    let camelcase_function = "function myFunction() {}";
    let report = validator.validate(camelcase_function, "test.ts").unwrap();
    assert_eq!(report.warning_count, 0, "camelCase should not warn");
}

#[test]
fn typescript_detects_duplicate_type_identifiers() {
    let mut validator = TypeScriptValidator::new();

    let duplicate_interfaces = r#"
interface User { name: string; }
interface User { email: string; }
"#;
    let report = validator.validate(duplicate_interfaces, "test.ts").unwrap();
    assert!(
        report.warning_count > 0,
        "Duplicate interface should warn"
    );
}

#[test]
fn typescript_detects_any_type_usage() {
    let mut validator = TypeScriptValidator::new();

    let code_with_any = "function test(param: any) {}";
    let report = validator.validate(code_with_any, "test.ts").unwrap();
    assert!(
        report.info_count > 0,
        "Using 'any' type should generate info message"
    );
}

#[test]
fn typescript_validates_complex_code() {
    let mut validator = TypeScriptValidator::new();

    let complex_code = r#"
import { Component } from 'react';

interface Props {
    title: string;
    onClick: () => void;
}

export class MyComponent extends Component<Props> {
    handleClick = () => {
        console.log('Clicked');
        this.props.onClick();
    };

    render() {
        return (
            <button onClick={this.handleClick}>
                {this.props.title}
            </button>
        );
    }
}

export default MyComponent;
"#;
    let report = validator.validate(complex_code, "component.tsx").unwrap();
    assert!(
        !report.has_errors(),
        "Valid complex TypeScript should pass"
    );
}

// =============================================================================
// YAML Validator Tests
// =============================================================================

#[test]
fn yaml_validates_simple_structure() {
    let validator = YamlValidator::new();

    let valid_yaml = r#"
key: value
number: 42
list:
  - item1
  - item2
  - item3
"#;
    let report = validator.validate(valid_yaml, "test.yaml").unwrap();
    assert!(!report.has_errors(), "Valid YAML should not have errors");
}

#[test]
fn yaml_validates_nested_structure() {
    let validator = YamlValidator::new();

    let nested_yaml = r#"
root:
  level1:
    level2:
      key: value
      array:
        - item1
        - item2
"#;
    let report = validator.validate(nested_yaml, "test.yaml").unwrap();
    assert!(
        !report.has_errors(),
        "Valid nested YAML should not have errors"
    );
}

#[test]
fn yaml_detects_tab_indentation() {
    let validator = YamlValidator::new();

    let yaml_with_tabs = "key: value\n\tindented: with_tab";
    let report = validator.validate(yaml_with_tabs, "test.yaml").unwrap();
    assert!(
        report.has_errors(),
        "YAML with tabs should cause error"
    );
}

#[test]
fn yaml_warns_on_inconsistent_indentation() {
    let validator = YamlValidator::new();

    let inconsistent_yaml = "key: value\n   indented: odd_spaces";
    let report = validator.validate(inconsistent_yaml, "test.yaml").unwrap();
    assert!(
        report.warning_count > 0,
        "Inconsistent indentation should warn"
    );
}

#[test]
fn yaml_detects_syntax_error() {
    let validator = YamlValidator::new();

    let invalid_yaml = r#"
key: value
- invalid_mix
  nested: structure
"#;
    let report = validator.validate(invalid_yaml, "test.yaml").unwrap();
    assert!(report.has_errors(), "Invalid YAML syntax should error");
}

#[test]
fn yaml_validates_arrays() {
    let validator = YamlValidator::new();

    let yaml_array = r#"
items:
  - name: item1
    value: 1
  - name: item2
    value: 2
"#;
    let report = validator.validate(yaml_array, "test.yaml").unwrap();
    assert!(!report.has_errors(), "Valid YAML array should pass");
}

#[test]
fn yaml_validates_multiline_strings() {
    let validator = YamlValidator::new();

    let multiline_yaml = r#"
description: |
  This is a multiline
  string in YAML
  format.
another: >
  This is a folded
  multiline string.
"#;
    let report = validator.validate(multiline_yaml, "test.yaml").unwrap();
    assert!(
        !report.has_errors(),
        "Valid multiline YAML should pass"
    );
}

// =============================================================================
// JSON Validator Tests
// =============================================================================

#[test]
fn json_validates_simple_object() {
    let validator = JsonValidator::new();

    let valid_json = r#"{"key": "value", "number": 42}"#;
    let report = validator.validate(valid_json, "test.json").unwrap();
    assert!(!report.has_errors(), "Valid JSON should not have errors");
}

#[test]
fn json_validates_nested_structure() {
    let validator = JsonValidator::new();

    let nested_json = r#"
{
  "root": {
    "nested": {
      "key": "value",
      "array": [1, 2, 3]
    }
  }
}
"#;
    let report = validator.validate(nested_json, "test.json").unwrap();
    assert!(
        !report.has_errors(),
        "Valid nested JSON should not have errors"
    );
}

#[test]
fn json_detects_trailing_comma() {
    let validator = JsonValidator::new();

    let json_with_trailing_comma = r#"{"key": "value",}"#;
    let report = validator
        .validate(json_with_trailing_comma, "test.json")
        .unwrap();
    assert!(
        report.has_errors(),
        "JSON with trailing comma should error"
    );
}

#[test]
fn json_detects_missing_quotes() {
    let validator = JsonValidator::new();

    let json_without_quotes = r#"{key: "value"}"#;
    let report = validator
        .validate(json_without_quotes, "test.json")
        .unwrap();
    assert!(
        report.has_errors(),
        "JSON keys must be in quotes"
    );
}

#[test]
fn json_detects_unclosed_structure() {
    let validator = JsonValidator::new();

    let incomplete_json = r#"{"key": "value""#;
    let report = validator.validate(incomplete_json, "test.json").unwrap();
    assert!(
        report.has_errors(),
        "Incomplete JSON should error"
    );
}

#[test]
fn json_validates_array() {
    let validator = JsonValidator::new();

    let json_array = r#"[1, 2, 3, "four", {"five": 5}]"#;
    let report = validator.validate(json_array, "test.json").unwrap();
    assert!(!report.has_errors(), "Valid JSON array should pass");
}

#[test]
fn json_validates_complex_structure() {
    let validator = JsonValidator::new();

    let complex_json = r#"
{
  "name": "Test API",
  "version": "1.0.0",
  "endpoints": [
    {
      "path": "/users",
      "methods": ["GET", "POST"],
      "auth": true
    },
    {
      "path": "/posts",
      "methods": ["GET"],
      "auth": false
    }
  ],
  "config": {
    "timeout": 3000,
    "retries": 3
  }
}
"#;
    let report = validator.validate(complex_json, "api.json").unwrap();
    assert!(
        !report.has_errors(),
        "Valid complex JSON should pass"
    );
}

// =============================================================================
// OpenAPI Validator Tests
// =============================================================================

#[test]
fn openapi_validates_minimal_spec() {
    let validator = OpenApiValidator::new();

    let minimal_openapi = r#"
openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths: {}
"#;
    let report = validator.validate(minimal_openapi, "openapi.yaml").unwrap();
    assert!(
        !report.has_errors(),
        "Minimal valid OpenAPI should pass"
    );
}

#[test]
fn openapi_detects_missing_version() {
    let validator = OpenApiValidator::new();

    let missing_version = r#"
info:
  title: Test API
  version: 1.0.0
paths: {}
"#;
    let report = validator.validate(missing_version, "openapi.yaml").unwrap();
    assert!(
        report.has_errors(),
        "Missing openapi version should error"
    );
}

#[test]
fn openapi_detects_missing_info() {
    let validator = OpenApiValidator::new();

    let missing_info = r#"
openapi: 3.0.0
paths: {}
"#;
    let report = validator.validate(missing_info, "openapi.yaml").unwrap();
    assert!(
        report.has_errors(),
        "Missing info section should error"
    );
}

#[test]
fn openapi_detects_missing_info_title() {
    let validator = OpenApiValidator::new();

    let missing_title = r#"
openapi: 3.0.0
info:
  version: 1.0.0
paths: {}
"#;
    let report = validator.validate(missing_title, "openapi.yaml").unwrap();
    assert!(
        report.has_errors(),
        "Missing info.title should error"
    );
}

#[test]
fn openapi_detects_missing_info_version() {
    let validator = OpenApiValidator::new();

    let missing_info_version = r#"
openapi: 3.0.0
info:
  title: Test API
paths: {}
"#;
    let report = validator
        .validate(missing_info_version, "openapi.yaml")
        .unwrap();
    assert!(
        report.has_errors(),
        "Missing info.version should error"
    );
}

#[test]
fn openapi_warns_on_missing_paths() {
    let validator = OpenApiValidator::new();

    let no_paths = r#"
openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
"#;
    let report = validator.validate(no_paths, "openapi.yaml").unwrap();
    assert!(
        report.warning_count > 0,
        "Missing paths section should warn"
    );
}

#[test]
fn openapi_validates_complete_spec() {
    let validator = OpenApiValidator::new();

    let complete_spec = r#"
openapi: 3.0.0
info:
  title: Test API
  description: A test API specification
  version: 1.0.0
  contact:
    name: API Support
    email: support@example.com

servers:
  - url: https://api.example.com/v1
    description: Production server

paths:
  /users:
    get:
      summary: List users
      responses:
        '200':
          description: Successful response
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/User'

components:
  schemas:
    User:
      type: object
      required:
        - id
        - name
      properties:
        id:
          type: integer
        name:
          type: string
        email:
          type: string
"#;
    let report = validator.validate(complete_spec, "openapi.yaml").unwrap();
    assert!(
        !report.has_errors(),
        "Complete valid OpenAPI spec should pass"
    );
}

#[test]
fn openapi_warns_on_old_version() {
    let validator = OpenApiValidator::new();

    let old_version = r#"
openapi: 2.0.0
info:
  title: Test API
  version: 1.0.0
paths: {}
"#;
    let report = validator.validate(old_version, "openapi.yaml").unwrap();
    assert!(
        report.warning_count > 0,
        "Old OpenAPI version should warn"
    );
}

#[test]
fn openapi_rejects_invalid_yaml() {
    let validator = OpenApiValidator::new();

    let invalid_yaml = r#"
openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  - invalid list syntax
"#;
    let report = validator.validate(invalid_yaml, "openapi.yaml").unwrap();
    assert!(
        report.has_errors(),
        "Invalid YAML in OpenAPI should error"
    );
}

// =============================================================================
// Integration Tests
// =============================================================================

#[test]
fn typescript_validator_resets_state() {
    let mut validator = TypeScriptValidator::new();

    // First validation
    let code1 = "interface User { name: string; }";
    validator.validate(code1, "test1.ts").unwrap();

    // Reset and second validation with same identifier
    validator.reset();
    let code2 = "interface User { email: string; }";
    let report = validator.validate(code2, "test2.ts").unwrap();

    // Should not warn about duplicate since we reset
    assert_eq!(
        report.warning_count, 0,
        "Reset should clear seen identifiers"
    );
}

#[test]
fn validators_provide_helpful_suggestions() {
    let mut ts_validator = TypeScriptValidator::new();
    let json_validator = JsonValidator::new();
    let yaml_validator = YamlValidator::new();

    // TypeScript suggestions
    let ts_report = ts_validator
        .validate("import { X } form 'y';", "test.ts")
        .unwrap();
    assert!(
        ts_report.issues[0].suggestion.is_some(),
        "TypeScript validator should provide suggestions"
    );

    // JSON suggestions
    let json_report = json_validator
        .validate(r#"{"key": "value",}"#, "test.json")
        .unwrap();
    assert!(
        json_report.issues[0].suggestion.is_some(),
        "JSON validator should provide suggestions"
    );

    // YAML suggestions
    let yaml_report = yaml_validator
        .validate("key: value\n\ttabbed: indent", "test.yaml")
        .unwrap();
    assert!(
        yaml_report.issues[0].suggestion.is_some(),
        "YAML validator should provide suggestions"
    );
}

#[test]
fn validators_include_line_numbers() {
    let mut validator = TypeScriptValidator::new();

    let code = "line1\nline2\nline3 { unmatched\nline4";
    let report = validator.validate(code, "test.ts").unwrap();

    assert!(report.has_errors());
    // Check that location includes line number
    if let Some(location) = &report.issues[0].location {
        assert!(location.contains(':'), "Location should include line number");
    }
}
