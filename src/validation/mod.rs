//! Input validation and boundary checks.
//!
//! This module provides comprehensive validation for all numeric parameters
//! used throughout the spreadsheet MCP server, including:
//! - Excel row/column limits
//! - Cache capacity bounds
//! - Sample size limits
//! - Pagination overflow protection
//! - PNG dimension constraints
//! - String and identifier validation (poka-yoke guards)
//! - Path traversal protection
//! - JSON schema validation for MCP tool inputs
//!
//! # JSON Schema Validation
//!
//! Runtime JSON schema validation for MCP tool inputs:
//! - Generate JSON schemas from schemars annotations
//! - Validate tool parameters against schemas before execution
//! - Detailed validation error messages with field-level context
//! - Middleware that validates all tool calls
//! - Thread-safe schema validation
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::validation::{SchemaValidator, SchemaValidationMiddleware};
//! use schemars::JsonSchema;
//! use serde::Deserialize;
//! use std::sync::Arc;
//!
//! #[derive(Debug, Deserialize, JsonSchema)]
//! struct MyToolParams {
//!     required_field: String,
//!     optional_field: Option<i32>,
//! }
//!
//! // Create and configure validator
//! let mut validator = SchemaValidator::new();
//! validator.register_schema::<MyToolParams>("my_tool");
//!
//! // Create middleware
//! let middleware = SchemaValidationMiddleware::new(Arc::new(validator));
//!
//! // Validate parameters
//! let params = serde_json::json!({
//!     "required_field": "value",
//!     "optional_field": 42
//! });
//!
//! middleware.validate_tool_call("my_tool", &params)?;
//! ```

pub mod bounds;
pub mod enhanced_bounds;
pub mod input_guards;
pub mod integration;
pub mod middleware;
pub mod schema;

pub use bounds::{
    ABSOLUTE_MAX_PNG_AREA_PX,

    ABSOLUTE_MAX_PNG_DIM_PX,
    DEFAULT_CACHE_CAPACITY,

    DEFAULT_MAX_PNG_AREA_PX,
    DEFAULT_MAX_PNG_DIM_PX,
    EXCEL_MAX_CELLS,

    EXCEL_MAX_COLUMN_INDEX,
    // Excel limits
    EXCEL_MAX_COLUMNS,
    EXCEL_MAX_ROW_INDEX,
    EXCEL_MAX_ROWS,
    MAX_CACHE_CAPACITY,
    MAX_PAGINATION_LIMIT,
    MAX_PAGINATION_OFFSET,

    // Sample and pagination limits
    MAX_SAMPLE_SIZE,
    MAX_SCREENSHOT_CELLS,
    MAX_SCREENSHOT_COLS,
    // Screenshot limits
    MAX_SCREENSHOT_ROWS,
    // Cache limits
    MIN_CACHE_CAPACITY,
    clamp_cache_capacity,
    validate_cache_capacity,
    validate_cell_1based,
    validate_column_1based,
    validate_pagination,
    validate_png_dimensions,
    validate_range_1based,
    // Validation functions
    validate_row_1based,
    validate_sample_size,
    validate_screenshot_range,
};

pub use enhanced_bounds::{
    validate_column_enhanced, validate_pagination_enhanced, validate_range_enhanced,
    validate_row_enhanced, validate_sample_size_enhanced, validate_screenshot_range_enhanced,
};

pub use input_guards::{
    ValidationError, ValidationResult, validate_cell_address, validate_non_empty_string,
    validate_numeric_range, validate_optional_numeric_range, validate_path_safe,
    validate_range_string, validate_sheet_name, validate_workbook_id,
};

// JSON Schema validation exports
pub use schema::{SchemaValidationError, SchemaValidator, SharedSchemaValidator, create_validator};

pub use middleware::{
    ValidateParams, ValidationMiddleware as SchemaValidationMiddleware, format_validation_errors,
    validate_with_details,
};

use schemars::JsonSchema;
use serde_json::Value;
use std::sync::Arc;

/// Create a fully configured validation middleware with all tool schemas
pub fn create_validation_middleware() -> SchemaValidationMiddleware {
    let validator = Arc::new(create_validator());
    SchemaValidationMiddleware::new(validator)
}

/// Convenience function to validate a tool call
pub fn validate_tool_params(
    validator: &SchemaValidator,
    tool_name: &str,
    params: &Value,
) -> Result<(), SchemaValidationError> {
    validator.validate(tool_name, params)
}

/// Macro to register multiple tool schemas at once
///
/// # Example
///
/// ```rust,ignore
/// use crate::register_tool_schemas;
/// use crate::validation::SchemaValidator;
///
/// let mut validator = SchemaValidator::new();
/// register_tool_schemas!(
///     validator,
///     "list_workbooks" => ListWorkbooksParams,
///     "describe_workbook" => DescribeWorkbookParams,
///     "read_table" => ReadTableParams,
/// );
/// ```
#[macro_export]
macro_rules! register_tool_schemas {
    ($validator:expr, $($tool_name:expr => $param_type:ty),* $(,)?) => {
        $(
            $validator.register_schema::<$param_type>($tool_name);
        )*
    };
}

/// Builder for creating a configured schema validator
pub struct SchemaValidatorBuilder {
    validator: SchemaValidator,
}

impl SchemaValidatorBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            validator: SchemaValidator::new(),
        }
    }

    /// Register a schema for a tool
    pub fn register<T: JsonSchema>(mut self, tool_name: &str) -> Self {
        self.validator.register_schema::<T>(tool_name);
        self
    }

    /// Build the validator
    pub fn build(self) -> SchemaValidator {
        self.validator
    }

    /// Build and wrap in Arc for thread-safe sharing
    pub fn build_shared(self) -> SharedSchemaValidator {
        Arc::new(self.validator)
    }
}

impl Default for SchemaValidatorBuilder {
    fn default() -> Self {
        Self::new()
    }
}
