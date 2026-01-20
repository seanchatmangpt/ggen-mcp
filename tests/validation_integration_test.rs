//! Integration tests for JSON schema validation
//!
//! These tests verify that the schema validation system correctly validates
//! tool parameters and integrates properly with the rmcp framework.
//!
//! Uses chicago-tdd-tools framework for comprehensive test coverage with:
//! - AAA (Arrange-Act-Assert) pattern enforcement
//! - Result-based error handling
//! - Enhanced assertion helpers

#[cfg(test)]
mod validation_tests {
    use chicago_tdd_tools::prelude::*;
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

    test!(test_schema_generation, {
        use spreadsheet_mcp::validation::SchemaValidator;

        // Arrange: Create validator and register schema
        let mut validator = SchemaValidator::new();
        validator.register_schema::<SimpleParams>("simple_tool");

        let params = json!({
            "name": "test",
            "count": 42
        });

        // Act: Validate parameters
        let result = validator.validate("simple_tool", &params);

        // Assert: Validation succeeds
        assert_ok!(result, "Schema validation should succeed for valid parameters");
    });

    test!(test_valid_parameters, {
        use spreadsheet_mcp::validation::SchemaValidator;

        // Arrange: Create validator with complex schema and valid parameters
        let mut validator = SchemaValidator::new();
        validator.register_schema::<ComplexParams>("complex_tool");

        let params = json!({
            "workbook_id": "wb-123",
            "sheet_name": "Sheet1",
            "range": "A1:C10",
            "limit": 100,
            "tags": ["important", "reviewed"]
        });

        // Act: Validate complex parameters
        let result = validator.validate("complex_tool", &params);

        // Assert: Validation succeeds for all fields
        assert_ok!(result, "Complex parameters should validate successfully");
    });

    test!(test_missing_required_field, {
        use spreadsheet_mcp::validation::{SchemaValidationError, SchemaValidator};

        // Arrange: Create validator and parameters missing required field
        let mut validator = SchemaValidator::new();
        validator.register_schema::<SimpleParams>("simple_tool");

        let params = json!({
            "count": 42
            // Missing 'name' field
        });

        // Act: Validate parameters with missing field
        let result = validator.validate("simple_tool", &params);

        // Assert: Validation fails with appropriate error
        assert_err!(result, "Validation should fail when required field is missing");

        if let Err(SchemaValidationError::ValidationFailed { errors, .. }) = result {
            assert!(
                errors.iter().any(|e| e.contains("name")),
                "Error should mention the missing 'name' field"
            );
        } else {
            panic!("Expected ValidationFailed error");
        }
    });

    test!(test_invalid_type, {
        use spreadsheet_mcp::validation::{SchemaValidationError, SchemaValidator};

        // Arrange: Create validator and parameters with wrong type
        let mut validator = SchemaValidator::new();
        validator.register_schema::<SimpleParams>("simple_tool");

        let params = json!({
            "name": "test",
            "count": "not a number"  // Should be i32
        });

        // Act: Validate parameters with type mismatch
        let result = validator.validate("simple_tool", &params);

        // Assert: Validation fails due to type error
        assert_err!(result, "Validation should fail when field has wrong type");

        if let Err(SchemaValidationError::ValidationFailed { .. }) = result {
            // Expected error type
        } else {
            panic!("Expected ValidationFailed error");
        }
    });

    test!(test_optional_fields, {
        use spreadsheet_mcp::validation::SchemaValidator;

        // Arrange: Create validator with optional field schema
        let mut validator = SchemaValidator::new();
        validator.register_schema::<OptionalParams>("optional_tool");

        // Act & Assert: Test with optional field present
        let params_with = json!({
            "required_field": "value",
            "optional_field": "optional value"
        });
        let result_with = validator.validate("optional_tool", &params_with);
        assert_ok!(result_with, "Validation should succeed with optional field present");

        // Act & Assert: Test with optional field missing
        let params_without = json!({
            "required_field": "value"
        });
        let result_without = validator.validate("optional_tool", &params_without);
        assert_ok!(result_without, "Validation should succeed with optional field missing");

        // Act & Assert: Test with optional field null
        let params_null = json!({
            "required_field": "value",
            "optional_field": null
        });
        let result_null = validator.validate("optional_tool", &params_null);
        assert_ok!(result_null, "Validation should succeed with optional field null");
    });

    test!(test_middleware_creation, {
        use spreadsheet_mcp::validation::{SchemaValidationMiddleware, SchemaValidator};
        use std::sync::Arc;

        // Arrange: Create validator, register schema, and wrap in middleware
        let mut validator = SchemaValidator::new();
        validator.register_schema::<SimpleParams>("simple_tool");
        let middleware = SchemaValidationMiddleware::new(Arc::new(validator));

        let params = json!({
            "name": "test",
            "count": 42
        });

        // Act: Validate through middleware
        let result = middleware.validate_tool_call("simple_tool", &params);

        // Assert: Middleware validation succeeds
        assert_ok!(result, "Middleware should validate tool calls successfully");
    });

    test!(test_validation_builder, {
        use spreadsheet_mcp::validation::SchemaValidatorBuilder;

        // Arrange: Build validator with multiple schemas using builder pattern
        let validator = SchemaValidatorBuilder::new()
            .register::<SimpleParams>("simple_tool")
            .register::<OptionalParams>("optional_tool")
            .register::<ComplexParams>("complex_tool")
            .build();

        // Act & Assert: Test simple tool
        let simple_params = json!({
            "name": "test",
            "count": 42
        });
        let simple_result = validator.validate("simple_tool", &simple_params);
        assert_ok!(simple_result, "Builder-created validator should handle simple tool");

        // Act & Assert: Test optional tool
        let optional_params = json!({
            "required_field": "value"
        });
        let optional_result = validator.validate("optional_tool", &optional_params);
        assert_ok!(optional_result, "Builder-created validator should handle optional tool");

        // Act & Assert: Test complex tool
        let complex_params = json!({
            "workbook_id": "wb-123",
            "sheet_name": "Sheet1"
        });
        let complex_result = validator.validate("complex_tool", &complex_params);
        assert_ok!(complex_result, "Builder-created validator should handle complex tool");
    });

    test!(test_error_message_formatting, {
        use spreadsheet_mcp::validation::{
            SchemaValidationError, SchemaValidator, format_validation_errors,
        };

        // Arrange: Create validator and invalid parameters
        let mut validator = SchemaValidator::new();
        validator.register_schema::<SimpleParams>("simple_tool");

        let params = json!({
            "count": 42
            // Missing 'name'
        });

        // Act: Validate and capture error
        let result = validator.validate("simple_tool", &params);

        // Assert: Error message is properly formatted
        if let Err(SchemaValidationError::ValidationFailed { errors, .. }) = result {
            let formatted = format_validation_errors("simple_tool", &errors);
            assert!(
                formatted.contains("simple_tool"),
                "Formatted error should include tool name"
            );
            assert!(
                formatted.contains("validation failed"),
                "Formatted error should indicate validation failure"
            );
        } else {
            panic!("Expected ValidationFailed error");
        }
    });

    test!(test_validate_and_deserialize, {
        use spreadsheet_mcp::validation::SchemaValidator;

        // Arrange: Create validator and valid parameters
        let mut validator = SchemaValidator::new();
        validator.register_schema::<SimpleParams>("simple_tool");

        let params = json!({
            "name": "test",
            "count": 42
        });

        // Act: Validate and deserialize in one operation
        let result: Result<SimpleParams, _> =
            validator.validate_and_deserialize("simple_tool", params);

        // Assert: Deserialization succeeds and values are correct
        assert_ok!(&result, "Validation and deserialization should succeed");

        if let Ok(deserialized) = result {
            assert_eq!(deserialized.name, "test", "Name field should deserialize correctly");
            assert_eq!(deserialized.count, 42, "Count field should deserialize correctly");
        }
    });

    test!(test_array_validation, {
        use spreadsheet_mcp::validation::SchemaValidator;

        // Arrange: Create validator with array field schema
        let mut validator = SchemaValidator::new();
        validator.register_schema::<ComplexParams>("complex_tool");

        // Act & Assert: Validate array with multiple elements
        let params = json!({
            "workbook_id": "wb-123",
            "sheet_name": "Sheet1",
            "tags": ["tag1", "tag2", "tag3"]
        });
        let result = validator.validate("complex_tool", &params);
        assert_ok!(result, "Validation should succeed with non-empty array");

        // Act & Assert: Validate empty array
        let params_empty = json!({
            "workbook_id": "wb-123",
            "sheet_name": "Sheet1",
            "tags": []
        });
        let result_empty = validator.validate("complex_tool", &params_empty);
        assert_ok!(result_empty, "Validation should succeed with empty array");
    });

    test!(test_integration_validators, {
        use spreadsheet_mcp::validation::integration::create_configured_validator;

        // Arrange: No setup needed for factory function

        // Act: Create pre-configured validator
        let validator = create_configured_validator();

        // Assert: Validator is created successfully
        // The validator should be ready to use
        // In practice, you would test with actual tool parameters here
        drop(validator);
    });

    #[cfg(feature = "recalc")]
    test!(test_recalc_validator, {
        use spreadsheet_mcp::validation::integration::create_configured_validator_with_recalc;

        // Arrange: No setup needed for factory function

        // Act: Create recalc-enabled validator
        let validator = create_configured_validator_with_recalc();

        // Assert: Recalc-enabled validator is created successfully
        drop(validator);
    });
}
