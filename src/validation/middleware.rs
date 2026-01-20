use crate::validation::schema::{SchemaValidator, SchemaValidationError, SharedSchemaValidator};
use anyhow::{Result, anyhow};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, error, warn};

/// Middleware for validating MCP tool calls
pub struct ValidationMiddleware {
    validator: SharedSchemaValidator,
}

impl ValidationMiddleware {
    /// Create a new validation middleware
    pub fn new(validator: SharedSchemaValidator) -> Self {
        Self { validator }
    }

    /// Validate tool parameters before execution
    pub fn validate_tool_call(
        &self,
        tool_name: &str,
        params: &Value,
    ) -> Result<(), SchemaValidationError> {
        debug!(tool = tool_name, "validating tool parameters");

        match self.validator.validate(tool_name, params) {
            Ok(()) => {
                debug!(tool = tool_name, "tool parameters validation passed");
                Ok(())
            }
            Err(e) => {
                warn!(
                    tool = tool_name,
                    error = %e,
                    "tool parameters validation failed"
                );
                Err(e)
            }
        }
    }

    /// Validate and deserialize tool parameters
    pub fn validate_and_deserialize<T>(
        &self,
        tool_name: &str,
        params: Value,
    ) -> Result<T>
    where
        T: DeserializeOwned + JsonSchema,
    {
        debug!(tool = tool_name, "validating and deserializing tool parameters");

        match self.validator.validate_and_deserialize(tool_name, params) {
            Ok(deserialized) => {
                debug!(tool = tool_name, "tool parameters validated and deserialized successfully");
                Ok(deserialized)
            }
            Err(e) => {
                error!(
                    tool = tool_name,
                    error = %e,
                    "tool parameters validation or deserialization failed"
                );
                Err(e)
            }
        }
    }
}

/// Validation result with detailed error information
#[derive(Debug)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub tool_name: String,
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn success(tool_name: String) -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            tool_name,
        }
    }

    /// Create a failed validation result
    pub fn failure(tool_name: String, errors: Vec<String>) -> Self {
        Self {
            is_valid: false,
            errors,
            tool_name,
        }
    }

    /// Convert to Result
    pub fn into_result(self) -> Result<()> {
        if self.is_valid {
            Ok(())
        } else {
            Err(anyhow!(
                "Validation failed for tool '{}': {}",
                self.tool_name,
                self.errors.join("; ")
            ))
        }
    }
}

/// Extension trait for validating tool parameters
pub trait ValidateParams {
    /// Validate parameters against a schema
    fn validate_params(
        &self,
        tool_name: &str,
        params: &Value,
    ) -> Result<ValidationResult>;
}

impl ValidateParams for SchemaValidator {
    fn validate_params(
        &self,
        tool_name: &str,
        params: &Value,
    ) -> Result<ValidationResult> {
        match self.validate(tool_name, params) {
            Ok(()) => Ok(ValidationResult::success(tool_name.to_string())),
            Err(SchemaValidationError::ValidationFailed { errors, .. }) => {
                Ok(ValidationResult::failure(tool_name.to_string(), errors))
            }
            Err(e) => Err(anyhow!(e)),
        }
    }
}

/// Helper function to create validation error messages
pub fn format_validation_errors(tool_name: &str, errors: &[String]) -> String {
    let error_list = errors
        .iter()
        .enumerate()
        .map(|(i, e)| format!("  {}. {}", i + 1, e))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "Tool '{}' parameter validation failed:\n{}",
        tool_name,
        error_list
    )
}

/// Validate parameters with detailed error reporting
pub fn validate_with_details(
    validator: &SchemaValidator,
    tool_name: &str,
    params: &Value,
) -> Result<()> {
    match validator.validate(tool_name, params) {
        Ok(()) => Ok(()),
        Err(SchemaValidationError::ValidationFailed { errors, .. }) => {
            let message = format_validation_errors(tool_name, &errors);
            Err(anyhow!(message))
        }
        Err(e) => Err(anyhow!(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::schema::SchemaValidator;
    use schemars::JsonSchema;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, JsonSchema)]
    struct TestParams {
        name: String,
        count: i32,
    }

    #[test]
    fn test_middleware_validation() {
        let mut validator = SchemaValidator::new();
        validator.register_schema::<TestParams>("test_tool");

        let middleware = ValidationMiddleware::new(Arc::new(validator));

        let valid_params = serde_json::json!({
            "name": "test",
            "count": 42
        });

        assert!(middleware.validate_tool_call("test_tool", &valid_params).is_ok());

        let invalid_params = serde_json::json!({
            "name": "test"
            // missing count
        });

        assert!(middleware.validate_tool_call("test_tool", &invalid_params).is_err());
    }

    #[test]
    fn test_validation_result() {
        let success = ValidationResult::success("test".to_string());
        assert!(success.is_valid);
        assert!(success.into_result().is_ok());

        let failure = ValidationResult::failure(
            "test".to_string(),
            vec!["error 1".to_string(), "error 2".to_string()],
        );
        assert!(!failure.is_valid);
        assert!(failure.into_result().is_err());
    }

    #[test]
    fn test_format_validation_errors() {
        let errors = vec![
            "Missing required field: name".to_string(),
            "Invalid type for field: count".to_string(),
        ];

        let formatted = format_validation_errors("test_tool", &errors);
        assert!(formatted.contains("test_tool"));
        assert!(formatted.contains("Missing required field: name"));
        assert!(formatted.contains("Invalid type for field: count"));
    }
}
