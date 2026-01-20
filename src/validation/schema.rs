use anyhow::{Result, anyhow};
use schemars::{JsonSchema, schema_for};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// JSON schema validation error
#[derive(Debug, Error)]
pub enum SchemaValidationError {
    #[error("Schema validation failed for tool '{tool}': {errors}")]
    ValidationFailed {
        tool: String,
        errors: Vec<String>,
    },

    #[error("Schema generation failed for tool '{tool}': {error}")]
    SchemaGenerationFailed {
        tool: String,
        error: String,
    },

    #[error("Missing required field '{field}' in tool '{tool}'")]
    MissingRequiredField {
        tool: String,
        field: String,
    },

    #[error("Invalid type for field '{field}' in tool '{tool}': expected {expected}, got {actual}")]
    InvalidType {
        tool: String,
        field: String,
        expected: String,
        actual: String,
    },

    #[error("Invalid value for field '{field}' in tool '{tool}': {reason}")]
    InvalidValue {
        tool: String,
        field: String,
        reason: String,
    },

    #[error("Unknown field '{field}' in tool '{tool}'")]
    UnknownField {
        tool: String,
        field: String,
    },
}

/// JSON schema validator for MCP tool parameters
pub struct SchemaValidator {
    /// Cache of generated schemas by tool name
    schemas: HashMap<String, serde_json::Value>,
}

impl SchemaValidator {
    /// Create a new schema validator
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
        }
    }

    /// Register a schema for a tool
    pub fn register_schema<T: JsonSchema>(&mut self, tool_name: &str) {
        let schema = schema_for!(T);
        let schema_json = serde_json::to_value(schema)
            .expect("Failed to serialize schema");
        self.schemas.insert(tool_name.to_string(), schema_json);
    }

    /// Validate parameters against a registered schema
    pub fn validate(&self, tool_name: &str, params: &Value) -> Result<(), SchemaValidationError> {
        let schema = self.schemas.get(tool_name)
            .ok_or_else(|| SchemaValidationError::SchemaGenerationFailed {
                tool: tool_name.to_string(),
                error: "Schema not registered".to_string(),
            })?;

        self.validate_against_schema(tool_name, params, schema)
    }

    /// Validate parameters against a schema and return detailed errors
    fn validate_against_schema(
        &self,
        tool_name: &str,
        params: &Value,
        schema: &Value,
    ) -> Result<(), SchemaValidationError> {
        let mut errors = Vec::new();

        // Extract schema definition
        let schema_obj = schema.as_object()
            .ok_or_else(|| SchemaValidationError::SchemaGenerationFailed {
                tool: tool_name.to_string(),
                error: "Invalid schema structure".to_string(),
            })?;

        // Get the actual schema definition (handle $ref if present)
        let definitions = schema_obj.get("definitions");
        let schema_def = if let Some(ref_path) = schema_obj.get("$ref") {
            // Handle $ref to definitions
            if let Some(ref_str) = ref_path.as_str() {
                if let Some(def_name) = ref_str.strip_prefix("#/definitions/") {
                    definitions
                        .and_then(|d| d.get(def_name))
                        .ok_or_else(|| SchemaValidationError::SchemaGenerationFailed {
                            tool: tool_name.to_string(),
                            error: format!("Definition '{}' not found", def_name),
                        })?
                } else {
                    schema_obj
                }
            } else {
                schema_obj
            }
        } else {
            schema_obj
        };

        // Validate type
        if let Some(schema_type) = schema_def.get("type") {
            if let Err(e) = self.validate_type(tool_name, params, schema_type) {
                errors.push(e);
            }
        }

        // If params is an object, validate its properties
        if let Some(params_obj) = params.as_object() {
            // Get required fields
            let required_fields: Vec<String> = schema_def
                .get("required")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            // Get properties schema
            let properties = schema_def
                .get("properties")
                .and_then(|p| p.as_object());

            // Check required fields
            for required_field in &required_fields {
                if !params_obj.contains_key(required_field) {
                    errors.push(format!("Missing required field: {}", required_field));
                }
            }

            // Validate each property
            if let Some(props) = properties {
                for (key, value) in params_obj {
                    if let Some(prop_schema) = props.get(key) {
                        if let Err(e) = self.validate_property(
                            tool_name,
                            key,
                            value,
                            prop_schema,
                            definitions,
                        ) {
                            errors.push(e);
                        }
                    } else {
                        // Check if additional properties are allowed
                        let additional_props = schema_def
                            .get("additionalProperties")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(true);

                        if !additional_props {
                            errors.push(format!("Unknown field: {}", key));
                        }
                    }
                }
            }
        }

        if !errors.is_empty() {
            return Err(SchemaValidationError::ValidationFailed {
                tool: tool_name.to_string(),
                errors,
            });
        }

        Ok(())
    }

    /// Validate a single property against its schema
    fn validate_property(
        &self,
        tool_name: &str,
        field_name: &str,
        value: &Value,
        prop_schema: &Value,
        definitions: Option<&Value>,
    ) -> Result<(), String> {
        // Handle $ref
        let actual_schema = if let Some(ref_path) = prop_schema.get("$ref") {
            if let Some(ref_str) = ref_path.as_str() {
                if let Some(def_name) = ref_str.strip_prefix("#/definitions/") {
                    definitions
                        .and_then(|d| d.get(def_name))
                        .unwrap_or(prop_schema)
                } else {
                    prop_schema
                }
            } else {
                prop_schema
            }
        } else {
            prop_schema
        };

        // Handle anyOf (for Option<T>)
        if let Some(any_of) = actual_schema.get("anyOf") {
            if let Some(variants) = any_of.as_array() {
                // For Option<T>, one variant is usually "type": "null"
                // Try to validate against non-null variants
                for variant in variants {
                    if let Some(variant_type) = variant.get("type") {
                        if variant_type.as_str() == Some("null") && value.is_null() {
                            return Ok(());
                        }
                    }
                    // Try to validate against this variant
                    if self.validate_property(tool_name, field_name, value, variant, definitions).is_ok() {
                        return Ok(());
                    }
                }
                return Err(format!("Value does not match any variant for field '{}'", field_name));
            }
        }

        // Validate type
        if let Some(expected_type) = actual_schema.get("type") {
            self.validate_type(tool_name, value, expected_type)
                .map_err(|e| format!("Field '{}': {}", field_name, e))?;
        }

        // Validate enum values
        if let Some(enum_values) = actual_schema.get("enum") {
            if let Some(enum_array) = enum_values.as_array() {
                if !enum_array.contains(value) {
                    let allowed: Vec<String> = enum_array
                        .iter()
                        .map(|v| format!("{:?}", v))
                        .collect();
                    return Err(format!(
                        "Field '{}': value must be one of: {}",
                        field_name,
                        allowed.join(", ")
                    ));
                }
            }
        }

        // Validate string constraints
        if value.is_string() {
            if let Some(s) = value.as_str() {
                // Min length
                if let Some(min_len) = actual_schema.get("minLength").and_then(|v| v.as_u64()) {
                    if s.len() < min_len as usize {
                        return Err(format!(
                            "Field '{}': string length {} is less than minimum {}",
                            field_name,
                            s.len(),
                            min_len
                        ));
                    }
                }

                // Max length
                if let Some(max_len) = actual_schema.get("maxLength").and_then(|v| v.as_u64()) {
                    if s.len() > max_len as usize {
                        return Err(format!(
                            "Field '{}': string length {} exceeds maximum {}",
                            field_name,
                            s.len(),
                            max_len
                        ));
                    }
                }

                // Pattern
                if let Some(pattern) = actual_schema.get("pattern").and_then(|v| v.as_str()) {
                    if let Ok(regex) = regex::Regex::new(pattern) {
                        if !regex.is_match(s) {
                            return Err(format!(
                                "Field '{}': value does not match pattern '{}'",
                                field_name,
                                pattern
                            ));
                        }
                    }
                }
            }
        }

        // Validate number constraints
        if value.is_number() {
            if let Some(num) = value.as_f64() {
                // Minimum
                if let Some(min) = actual_schema.get("minimum").and_then(|v| v.as_f64()) {
                    if num < min {
                        return Err(format!(
                            "Field '{}': value {} is less than minimum {}",
                            field_name,
                            num,
                            min
                        ));
                    }
                }

                // Maximum
                if let Some(max) = actual_schema.get("maximum").and_then(|v| v.as_f64()) {
                    if num > max {
                        return Err(format!(
                            "Field '{}': value {} exceeds maximum {}",
                            field_name,
                            num,
                            max
                        ));
                    }
                }

                // Exclusive minimum
                if let Some(min) = actual_schema.get("exclusiveMinimum").and_then(|v| v.as_f64()) {
                    if num <= min {
                        return Err(format!(
                            "Field '{}': value {} must be greater than {}",
                            field_name,
                            num,
                            min
                        ));
                    }
                }

                // Exclusive maximum
                if let Some(max) = actual_schema.get("exclusiveMaximum").and_then(|v| v.as_f64()) {
                    if num >= max {
                        return Err(format!(
                            "Field '{}': value {} must be less than {}",
                            field_name,
                            num,
                            max
                        ));
                    }
                }
            }
        }

        // Validate array constraints
        if let Some(arr) = value.as_array() {
            // Min items
            if let Some(min_items) = actual_schema.get("minItems").and_then(|v| v.as_u64()) {
                if arr.len() < min_items as usize {
                    return Err(format!(
                        "Field '{}': array length {} is less than minimum {}",
                        field_name,
                        arr.len(),
                        min_items
                    ));
                }
            }

            // Max items
            if let Some(max_items) = actual_schema.get("maxItems").and_then(|v| v.as_u64()) {
                if arr.len() > max_items as usize {
                    return Err(format!(
                        "Field '{}': array length {} exceeds maximum {}",
                        field_name,
                        arr.len(),
                        max_items
                    ));
                }
            }

            // Validate items
            if let Some(items_schema) = actual_schema.get("items") {
                for (i, item) in arr.iter().enumerate() {
                    let item_field = format!("{}[{}]", field_name, i);
                    self.validate_property(tool_name, &item_field, item, items_schema, definitions)?;
                }
            }
        }

        Ok(())
    }

    /// Validate type of a value
    fn validate_type(
        &self,
        _tool_name: &str,
        value: &Value,
        expected_type: &Value,
    ) -> Result<(), String> {
        let expected = if let Some(type_str) = expected_type.as_str() {
            type_str
        } else {
            return Ok(()); // Type not specified or complex
        };

        let actual = match value {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        };

        // Special handling for integer
        if expected == "integer" {
            if let Some(num) = value.as_f64() {
                if num.fract() == 0.0 {
                    return Ok(());
                }
            }
            return Err(format!("expected integer, got {}", actual));
        }

        if expected != actual {
            return Err(format!("expected {}, got {}", expected, actual));
        }

        Ok(())
    }

    /// Validate and deserialize parameters
    pub fn validate_and_deserialize<T>(
        &self,
        tool_name: &str,
        params: Value,
    ) -> Result<T>
    where
        T: DeserializeOwned + JsonSchema,
    {
        // First validate against schema
        self.validate(tool_name, &params)
            .map_err(|e| anyhow!(e))?;

        // Then deserialize
        serde_json::from_value(params)
            .map_err(|e| anyhow!("Deserialization failed for tool '{}': {}", tool_name, e))
    }
}

impl Default for SchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe schema validator
pub type SharedSchemaValidator = Arc<SchemaValidator>;

/// Create a pre-configured schema validator with all tool schemas registered
///
/// This function creates a validator with all tool schemas registered.
/// The actual registration is delegated to the integration module.
///
/// # Returns
///
/// A SchemaValidator with all tool parameter schemas registered based on enabled features.
///
/// # Example
///
/// ```rust,ignore
/// use crate::validation::create_validator;
///
/// let validator = create_validator();
/// let params = serde_json::json!({
///     "slug_prefix": "test"
/// });
/// validator.validate("list_workbooks", &params)?;
/// ```
pub fn create_validator() -> SchemaValidator {
    // Delegate to integration module for full validator setup
    // This keeps the schema module focused on validation logic
    // while integration handles tool registration
    crate::validation::integration::create_full_validator()
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::JsonSchema;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, JsonSchema)]
    struct TestParams {
        required_field: String,
        #[serde(default)]
        optional_field: Option<i32>,
    }

    #[test]
    fn test_schema_generation() {
        let mut validator = SchemaValidator::new();
        validator.register_schema::<TestParams>("test_tool");

        assert!(validator.schemas.contains_key("test_tool"));
    }

    #[test]
    fn test_valid_params() {
        let mut validator = SchemaValidator::new();
        validator.register_schema::<TestParams>("test_tool");

        let params = serde_json::json!({
            "required_field": "value",
            "optional_field": 42
        });

        assert!(validator.validate("test_tool", &params).is_ok());
    }

    #[test]
    fn test_missing_required_field() {
        let mut validator = SchemaValidator::new();
        validator.register_schema::<TestParams>("test_tool");

        let params = serde_json::json!({
            "optional_field": 42
        });

        let result = validator.validate("test_tool", &params);
        assert!(result.is_err());

        if let Err(SchemaValidationError::ValidationFailed { errors, .. }) = result {
            assert!(errors.iter().any(|e| e.contains("required_field")));
        }
    }

    #[test]
    fn test_invalid_type() {
        let mut validator = SchemaValidator::new();
        validator.register_schema::<TestParams>("test_tool");

        let params = serde_json::json!({
            "required_field": 123  // Should be string
        });

        let result = validator.validate("test_tool", &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_optional_field_missing() {
        let mut validator = SchemaValidator::new();
        validator.register_schema::<TestParams>("test_tool");

        let params = serde_json::json!({
            "required_field": "value"
        });

        // Should be valid - optional field is optional
        assert!(validator.validate("test_tool", &params).is_ok());
    }
}
