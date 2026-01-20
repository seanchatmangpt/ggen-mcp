//! Integration tests for JSON schema validation
//!
//! These tests verify that the schema validation system correctly validates
//! tool parameters and integrates properly with the rmcp framework.

#[cfg(test)]
mod validation_tests {
    use schemars::JsonSchema;
    use serde::Deserialize;
    use serde_json::json;

    // Test parameter structs
    #[derive(Debug, Deserialize, JsonSchema)]
    struct SimpleParams {
        name: String,
        count: i32,
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    struct OptionalParams {
        required_field: String,
        #[serde(default)]
        optional_field: Option<String>,
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    struct ComplexParams {
        workbook_id: String,
        sheet_name: String,
        #[serde(default)]
        range: Option<String>,
        #[serde(default)]
        limit: Option<u32>,
        #[serde(default)]
        tags: Vec<String>,
    }

    #[test]
    fn test_schema_generation() {
        use spreadsheet_mcp::validation::SchemaValidator;

        let mut validator = SchemaValidator::new();
        validator.register_schema::<SimpleParams>("simple_tool");

        // Verify schema is registered
        let params = json!({
            "name": "test",
            "count": 42
        });

        assert!(validator.validate("simple_tool", &params).is_ok());
    }

    #[test]
    fn test_valid_parameters() {
        use spreadsheet_mcp::validation::SchemaValidator;

        let mut validator = SchemaValidator::new();
        validator.register_schema::<ComplexParams>("complex_tool");

        let params = json!({
            "workbook_id": "wb-123",
            "sheet_name": "Sheet1",
            "range": "A1:C10",
            "limit": 100,
            "tags": ["important", "reviewed"]
        });

        let result = validator.validate("complex_tool", &params);
        assert!(result.is_ok(), "Expected validation to pass, got: {:?}", result);
    }

    #[test]
    fn test_missing_required_field() {
        use spreadsheet_mcp::validation::{SchemaValidator, SchemaValidationError};

        let mut validator = SchemaValidator::new();
        validator.register_schema::<SimpleParams>("simple_tool");

        let params = json!({
            "count": 42
            // Missing 'name' field
        });

        let result = validator.validate("simple_tool", &params);
        assert!(result.is_err());

        match result {
            Err(SchemaValidationError::ValidationFailed { errors, .. }) => {
                assert!(errors.iter().any(|e| e.contains("name")));
            }
            _ => panic!("Expected ValidationFailed error"),
        }
    }

    #[test]
    fn test_invalid_type() {
        use spreadsheet_mcp::validation::{SchemaValidator, SchemaValidationError};

        let mut validator = SchemaValidator::new();
        validator.register_schema::<SimpleParams>("simple_tool");

        let params = json!({
            "name": "test",
            "count": "not a number"  // Should be i32
        });

        let result = validator.validate("simple_tool", &params);
        assert!(result.is_err());

        match result {
            Err(SchemaValidationError::ValidationFailed { .. }) => {
                // Expected
            }
            _ => panic!("Expected ValidationFailed error"),
        }
    }

    #[test]
    fn test_optional_fields() {
        use spreadsheet_mcp::validation::SchemaValidator;

        let mut validator = SchemaValidator::new();
        validator.register_schema::<OptionalParams>("optional_tool");

        // Test with optional field present
        let params_with = json!({
            "required_field": "value",
            "optional_field": "optional value"
        });
        assert!(validator.validate("optional_tool", &params_with).is_ok());

        // Test with optional field missing
        let params_without = json!({
            "required_field": "value"
        });
        assert!(validator.validate("optional_tool", &params_without).is_ok());

        // Test with optional field null
        let params_null = json!({
            "required_field": "value",
            "optional_field": null
        });
        assert!(validator.validate("optional_tool", &params_null).is_ok());
    }

    #[test]
    fn test_middleware_creation() {
        use spreadsheet_mcp::validation::{SchemaValidator, SchemaValidationMiddleware};
        use std::sync::Arc;

        let mut validator = SchemaValidator::new();
        validator.register_schema::<SimpleParams>("simple_tool");

        let middleware = SchemaValidationMiddleware::new(Arc::new(validator));

        let params = json!({
            "name": "test",
            "count": 42
        });

        assert!(middleware.validate_tool_call("simple_tool", &params).is_ok());
    }

    #[test]
    fn test_validation_builder() {
        use spreadsheet_mcp::validation::SchemaValidatorBuilder;

        let validator = SchemaValidatorBuilder::new()
            .register::<SimpleParams>("simple_tool")
            .register::<OptionalParams>("optional_tool")
            .register::<ComplexParams>("complex_tool")
            .build();

        // Test simple tool
        let simple_params = json!({
            "name": "test",
            "count": 42
        });
        assert!(validator.validate("simple_tool", &simple_params).is_ok());

        // Test optional tool
        let optional_params = json!({
            "required_field": "value"
        });
        assert!(validator.validate("optional_tool", &optional_params).is_ok());

        // Test complex tool
        let complex_params = json!({
            "workbook_id": "wb-123",
            "sheet_name": "Sheet1"
        });
        assert!(validator.validate("complex_tool", &complex_params).is_ok());
    }

    #[test]
    fn test_error_message_formatting() {
        use spreadsheet_mcp::validation::{SchemaValidator, SchemaValidationError, format_validation_errors};

        let mut validator = SchemaValidator::new();
        validator.register_schema::<SimpleParams>("simple_tool");

        let params = json!({
            "count": 42
            // Missing 'name'
        });

        match validator.validate("simple_tool", &params) {
            Err(SchemaValidationError::ValidationFailed { errors, .. }) => {
                let formatted = format_validation_errors("simple_tool", &errors);
                assert!(formatted.contains("simple_tool"));
                assert!(formatted.contains("validation failed"));
            }
            _ => panic!("Expected ValidationFailed error"),
        }
    }

    #[test]
    fn test_validate_and_deserialize() {
        use spreadsheet_mcp::validation::SchemaValidator;

        let mut validator = SchemaValidator::new();
        validator.register_schema::<SimpleParams>("simple_tool");

        let params = json!({
            "name": "test",
            "count": 42
        });

        let result: Result<SimpleParams, _> = validator.validate_and_deserialize("simple_tool", params);
        assert!(result.is_ok());

        let deserialized = result.unwrap();
        assert_eq!(deserialized.name, "test");
        assert_eq!(deserialized.count, 42);
    }

    #[test]
    fn test_array_validation() {
        use spreadsheet_mcp::validation::SchemaValidator;

        let mut validator = SchemaValidator::new();
        validator.register_schema::<ComplexParams>("complex_tool");

        // Valid array
        let params = json!({
            "workbook_id": "wb-123",
            "sheet_name": "Sheet1",
            "tags": ["tag1", "tag2", "tag3"]
        });
        assert!(validator.validate("complex_tool", &params).is_ok());

        // Empty array (should be valid)
        let params_empty = json!({
            "workbook_id": "wb-123",
            "sheet_name": "Sheet1",
            "tags": []
        });
        assert!(validator.validate("complex_tool", &params_empty).is_ok());
    }

    #[test]
    fn test_integration_validators() {
        use spreadsheet_mcp::validation::integration::create_configured_validator;

        // This test verifies that the pre-configured validator can be created
        // In a full test, we would validate against actual tool parameters
        let validator = create_configured_validator();

        // The validator should be ready to use
        // In practice, you would test with actual tool parameters here
        drop(validator);
    }

    #[test]
    #[cfg(feature = "recalc")]
    fn test_recalc_validator() {
        use spreadsheet_mcp::validation::integration::create_configured_validator_with_recalc;

        // Test that the recalc-enabled validator can be created
        let validator = create_configured_validator_with_recalc();
        drop(validator);
    }
}
