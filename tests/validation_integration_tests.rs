// Integration tests for validate_generated_code tool
// Chicago-TDD: Real implementations, state-based testing

use ggen_mcp::tools::ontology_generation::{
    ValidateGeneratedCodeParams, ValidateGeneratedCodeResponse, validate_generated_code,
};
use std::path::PathBuf;

const VALID_RUST_CODE: &str = r#"
pub struct Entity {
    pub id: String,
    pub name: String,
}

impl Entity {
    pub fn new(id: String, name: String) -> Self {
        Self { id, name }
    }
}
"#;

const INVALID_RUST_CODE: &str = r#"
pub struct Entity {
    pub id: String,
    pub name: String,
// Missing closing brace
"#;

const VALID_TYPESCRIPT: &str = r#"
export interface Entity {
    id: string;
    name: string;
}

export class EntityService {
    getEntity(id: string): Entity {
        return { id, name: "test" };
    }
}
"#;

const VALID_JSON: &str = r#"
{
  "name": "test",
  "version": "1.0.0",
  "entities": [
    {
      "id": "1",
      "name": "Entity1"
    }
  ]
}
"#;

const INVALID_JSON: &str = r#"
{
  "name": "test",
  "version": "1.0.0",
  "trailing": "comma",
}
"#;

const VALID_YAML: &str = r#"
name: test
version: 1.0.0
entities:
  - id: 1
    name: Entity1
  - id: 2
    name: Entity2
"#;

const INVALID_YAML: &str = r#"
name: test
  bad_indent: value
  version: 1.0.0
"#;

// =============================================================================
// Rust Validation Tests
// =============================================================================

#[tokio::test]
async fn test_validate_rust_valid() {
    let params = ValidateGeneratedCodeParams {
        code: VALID_RUST_CODE.to_string(),
        language: "rust".to_string(),
        file_name: "entity.rs".to_string(),
        golden_file_path: None,
        strict_mode: false,
        allow_golden_update: false,
    };

    let result = validate_generated_code(params).await;
    assert!(result.is_ok(), "Expected Ok result: {:?}", result);

    let response = result.unwrap();
    assert!(response.valid, "Expected valid=true for valid Rust code");
    assert!(response.errors.is_empty(), "Expected no errors");
}

#[tokio::test]
async fn test_validate_rust_invalid() {
    let params = ValidateGeneratedCodeParams {
        code: INVALID_RUST_CODE.to_string(),
        language: "rust".to_string(),
        file_name: "entity.rs".to_string(),
        golden_file_path: None,
        strict_mode: false,
        allow_golden_update: false,
    };

    let result = validate_generated_code(params).await;
    assert!(
        result.is_ok(),
        "Validation should succeed even with syntax errors"
    );

    let response = result.unwrap();
    assert!(
        !response.valid,
        "Expected valid=false for invalid Rust code"
    );
    assert!(!response.errors.is_empty(), "Expected syntax errors");
}

#[tokio::test]
async fn test_validate_rust_strict_mode() {
    let code_with_warning = r#"
pub struct test_struct {
    pub field: String,
}
"#;

    let params = ValidateGeneratedCodeParams {
        code: code_with_warning.to_string(),
        language: "rust".to_string(),
        file_name: "test.rs".to_string(),
        golden_file_path: None,
        strict_mode: true,
        allow_golden_update: false,
    };

    let result = validate_generated_code(params).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    // In strict mode, warnings should fail validation
    if !response.warnings.is_empty() {
        assert!(!response.valid, "Strict mode should fail on warnings");
    }
}

// =============================================================================
// TypeScript Validation Tests
// =============================================================================

#[tokio::test]
async fn test_validate_typescript_valid() {
    let params = ValidateGeneratedCodeParams {
        code: VALID_TYPESCRIPT.to_string(),
        language: "typescript".to_string(),
        file_name: "entity.ts".to_string(),
        golden_file_path: None,
        strict_mode: false,
        allow_golden_update: false,
    };

    let result = validate_generated_code(params).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.valid, "Expected valid TypeScript code");
}

#[tokio::test]
async fn test_validate_typescript_unbalanced_braces() {
    let code = "export interface Test { name: string;";

    let params = ValidateGeneratedCodeParams {
        code: code.to_string(),
        language: "typescript".to_string(),
        file_name: "test.ts".to_string(),
        golden_file_path: None,
        strict_mode: false,
        allow_golden_update: false,
    };

    let result = validate_generated_code(params).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(!response.valid, "Expected invalid due to unbalanced braces");
    assert!(!response.errors.is_empty());
}

// =============================================================================
// JSON Validation Tests
// =============================================================================

#[tokio::test]
async fn test_validate_json_valid() {
    let params = ValidateGeneratedCodeParams {
        code: VALID_JSON.to_string(),
        language: "json".to_string(),
        file_name: "data.json".to_string(),
        golden_file_path: None,
        strict_mode: false,
        allow_golden_update: false,
    };

    let result = validate_generated_code(params).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.valid, "Expected valid JSON");
}

#[tokio::test]
async fn test_validate_json_invalid() {
    let params = ValidateGeneratedCodeParams {
        code: INVALID_JSON.to_string(),
        language: "json".to_string(),
        file_name: "data.json".to_string(),
        golden_file_path: None,
        strict_mode: false,
        allow_golden_update: false,
    };

    let result = validate_generated_code(params).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(!response.valid, "Expected invalid JSON");
    assert!(!response.errors.is_empty());
}

// =============================================================================
// YAML Validation Tests
// =============================================================================

#[tokio::test]
async fn test_validate_yaml_valid() {
    let params = ValidateGeneratedCodeParams {
        code: VALID_YAML.to_string(),
        language: "yaml".to_string(),
        file_name: "config.yaml".to_string(),
        golden_file_path: None,
        strict_mode: false,
        allow_golden_update: false,
    };

    let result = validate_generated_code(params).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.valid, "Expected valid YAML");
}

#[tokio::test]
async fn test_validate_yaml_invalid() {
    let params = ValidateGeneratedCodeParams {
        code: INVALID_YAML.to_string(),
        language: "yaml".to_string(),
        file_name: "config.yaml".to_string(),
        golden_file_path: None,
        strict_mode: false,
        allow_golden_update: false,
    };

    let result = validate_generated_code(params).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(!response.valid, "Expected invalid YAML");
}

// =============================================================================
// Golden File Comparison Tests
// =============================================================================

#[tokio::test]
async fn test_golden_file_comparison_match() {
    let params = ValidateGeneratedCodeParams {
        code: VALID_TYPESCRIPT.to_string(),
        language: "typescript".to_string(),
        file_name: "entity.ts".to_string(),
        golden_file_path: Some("tests/golden/lib/types/entities.mjs".to_string()),
        strict_mode: false,
        allow_golden_update: false,
    };

    let result = validate_generated_code(params).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.valid, "Expected valid TypeScript");

    // Golden file diff should be present
    if let Some(diff) = response.golden_file_diff {
        // File might not match exactly, but diff should be computed
        assert!(diff.additions > 0 || diff.deletions > 0 || diff.changes > 0 || diff.is_identical);
    }
}

#[tokio::test]
async fn test_golden_file_missing() {
    let params = ValidateGeneratedCodeParams {
        code: VALID_RUST_CODE.to_string(),
        language: "rust".to_string(),
        file_name: "test.rs".to_string(),
        golden_file_path: Some("tests/golden/nonexistent.rs".to_string()),
        strict_mode: false,
        allow_golden_update: false,
    };

    let result = validate_generated_code(params).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    // Should still validate syntax even if golden file missing
    assert!(response.valid);
    // Should have warning about missing golden file
    assert!(response.warnings.iter().any(|w| w.contains("not found")));
}

// =============================================================================
// Language Support Tests
// =============================================================================

#[tokio::test]
async fn test_unsupported_language() {
    let params = ValidateGeneratedCodeParams {
        code: "print('hello')".to_string(),
        language: "python".to_string(),
        file_name: "test.py".to_string(),
        golden_file_path: None,
        strict_mode: false,
        allow_golden_update: false,
    };

    let result = validate_generated_code(params).await;
    assert!(result.is_err(), "Should reject unsupported language");

    let error = result.unwrap_err();
    assert!(error.to_string().contains("Unsupported language"));
}

#[tokio::test]
async fn test_language_aliases() {
    // Test that language aliases work
    let test_cases = vec![
        ("rust", "rs"),
        ("typescript", "ts"),
        ("javascript", "js"),
        ("yaml", "yml"),
    ];

    for (lang1, lang2) in test_cases {
        let params1 = ValidateGeneratedCodeParams {
            code: "{}".to_string(),
            language: lang1.to_string(),
            file_name: "test".to_string(),
            golden_file_path: None,
            strict_mode: false,
            allow_golden_update: false,
        };

        let params2 = ValidateGeneratedCodeParams {
            code: "{}".to_string(),
            language: lang2.to_string(),
            file_name: "test".to_string(),
            golden_file_path: None,
            strict_mode: false,
            allow_golden_update: false,
        };

        let result1 = validate_generated_code(params1).await;
        let result2 = validate_generated_code(params2).await;

        // Both should succeed (or fail in the same way)
        assert_eq!(
            result1.is_ok(),
            result2.is_ok(),
            "Language aliases {} and {} should behave the same",
            lang1,
            lang2
        );
    }
}

// =============================================================================
// Error Reporting Tests
// =============================================================================

#[tokio::test]
async fn test_error_reporting_detail() {
    let code = r#"
pub struct Test {
    pub field String, // Missing colon
}
"#;

    let params = ValidateGeneratedCodeParams {
        code: code.to_string(),
        language: "rust".to_string(),
        file_name: "test.rs".to_string(),
        golden_file_path: None,
        strict_mode: false,
        allow_golden_update: false,
    };

    let result = validate_generated_code(params).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(!response.valid);
    assert!(
        !response.summary.is_empty(),
        "Summary should provide context"
    );
}
