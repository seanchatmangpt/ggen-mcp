//! Example showing how to integrate JSON schema validation with the MCP server
//!
//! This example demonstrates the complete integration of the validation system
//! with the SpreadsheetServer and rmcp framework.

/// # Server Integration Pattern
///
/// The validation system integrates with the MCP server at multiple levels:
///
/// 1. **Schema Registration at Startup**
///    - Create a SchemaValidator with all tool schemas
///    - Wrap in Arc for thread-safe sharing
///    - Store in server state or as middleware
///
/// 2. **Parameter Validation in Tool Handlers**
///    - Parameters are validated before deserialization
///    - Errors are returned as MCP errors to the client
///    - Valid parameters are passed to tool logic
///
/// 3. **Error Handling**
///    - Validation errors are formatted as user-friendly messages
///    - Errors are returned with proper MCP error codes
///
/// ## Example Integration Code
///
/// ```rust,ignore
/// use spreadsheet_mcp::validation::integration::create_validation_middleware;
/// use spreadsheet_mcp::validation::SchemaValidationMiddleware;
/// use std::sync::Arc;
///
/// // In server initialization:
/// pub struct SpreadsheetServer {
///     state: Arc<AppState>,
///     tool_router: ToolRouter<SpreadsheetServer>,
///     validator: Arc<SchemaValidationMiddleware>,
/// }
///
/// impl SpreadsheetServer {
///     pub async fn new(config: Arc<ServerConfig>) -> Result<Self> {
///         config.ensure_workspace_root()?;
///         let state = Arc::new(AppState::new(config));
///
///         // Create validation middleware with all tool schemas
///         let validator = Arc::new(create_validation_middleware());
///
///         Ok(Self::from_state(state, validator))
///     }
///
///     pub fn from_state(
///         state: Arc<AppState>,
///         validator: Arc<SchemaValidationMiddleware>
///     ) -> Self {
///         let router = Self::tool_router();
///
///         Self {
///             state,
///             tool_router: router,
///             validator,
///         }
///     }
/// }
/// ```
///
/// ## Tool Handler Pattern
///
/// ```rust,ignore
/// use rmcp::handler::server::wrapper::Parameters;
/// use rmcp::{Json, ErrorData as McpError};
/// use serde_json::Value;
///
/// // Option 1: Validate before Parameters extraction
/// pub async fn read_table(
///     &self,
///     params_value: Value,
/// ) -> Result<Json<Response>, McpError> {
///     // Validate first
///     self.validator
///         .validate_tool_call("read_table", &params_value)
///         .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
///
///     // Then deserialize
///     let params: ReadTableParams = serde_json::from_value(params_value)
///         .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
///
///     // Proceed with tool logic
///     self.run_tool_with_timeout(
///         "read_table",
///         tools::read_table(self.state.clone(), params),
///     )
///     .await
///     .map(Json)
///     .map_err(to_mcp_error)
/// }
///
/// // Option 2: Use validate_and_deserialize
/// pub async fn read_table(
///     &self,
///     params_value: Value,
/// ) -> Result<Json<Response>, McpError> {
///     // Validate and deserialize in one step
///     let params: ReadTableParams = self.validator
///         .validate_and_deserialize("read_table", params_value)
///         .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
///
///     // Proceed with tool logic
///     self.run_tool_with_timeout(
///         "read_table",
///         tools::read_table(self.state.clone(), params),
///     )
///     .await
///     .map(Json)
///     .map_err(to_mcp_error)
/// }
/// ```
///
/// ## Error Conversion
///
/// ```rust,ignore
/// use spreadsheet_mcp::validation::SchemaValidationError;
/// use rmcp::ErrorData as McpError;
///
/// fn validation_error_to_mcp(error: SchemaValidationError) -> McpError {
///     match error {
///         SchemaValidationError::ValidationFailed { tool, errors } => {
///             let message = format!(
///                 "Tool '{}' parameter validation failed:\n{}",
///                 tool,
///                 errors
///                     .iter()
///                     .enumerate()
///                     .map(|(i, e)| format!("  {}. {}", i + 1, e))
///                     .collect::<Vec<_>>()
///                     .join("\n")
///             );
///             McpError::invalid_params(message, None)
///         }
///         SchemaValidationError::SchemaGenerationFailed { tool, error } => {
///             let message = format!(
///                 "Schema generation failed for tool '{}': {}",
///                 tool, error
///             );
///             McpError::internal_error(message, None)
///         }
///         _ => {
///             McpError::invalid_params(error.to_string(), None)
///         }
///     }
/// }
/// ```
///
/// ## Configuration-Based Validation
///
/// ```rust,ignore
/// // In ServerConfig
/// pub struct ServerConfig {
///     // ... existing fields ...
///
///     /// Enable strict schema validation
///     pub strict_validation: bool,
///
///     /// Enable validation logging
///     pub log_validation: bool,
/// }
///
/// // In tool handler
/// pub async fn read_table(
///     &self,
///     Parameters(params): Parameters<ReadTableParams>,
/// ) -> Result<Json<Response>, McpError> {
///     // Optionally validate based on config
///     if self.state.config().strict_validation {
///         // Perform additional validation beyond schema
///         validate_business_rules(&params)?;
///     }
///
///     if self.state.config().log_validation {
///         tracing::debug!(
///             tool = "read_table",
///             params = ?params,
///             "validated tool parameters"
///         );
///     }
///
///     // Proceed with tool logic
///     // ...
/// }
/// ```
///
/// ## Testing Integration
///
/// ```rust,ignore
/// #[cfg(test)]
/// mod tests {
///     use super::*;
///     use spreadsheet_mcp::validation::integration::create_validation_middleware;
///
///     #[tokio::test]
///     async fn test_server_with_validation() {
///         let config = Arc::new(ServerConfig::default());
///         let server = SpreadsheetServer::new(config).await.unwrap();
///
///         // Test that validator is properly initialized
///         let params = serde_json::json!({
///             "workbook_or_fork_id": "test-id"
///         });
///
///         assert!(server.validator.validate_tool_call(
///             "describe_workbook",
///             &params
///         ).is_ok());
///     }
///
///     #[tokio::test]
///     async fn test_invalid_parameters_rejected() {
///         let config = Arc::new(ServerConfig::default());
///         let server = SpreadsheetServer::new(config).await.unwrap();
///
///         // Test that invalid parameters are rejected
///         let params = serde_json::json!({
///             // Missing required field
///         });
///
///         assert!(server.validator.validate_tool_call(
///             "describe_workbook",
///             &params
///         ).is_err());
///     }
/// }
/// ```

fn main() {
    println!("Server Integration Example");
    println!();
    println!("This example demonstrates how to integrate the JSON schema");
    println!("validation system with the SpreadsheetServer.");
    println!();
    println!("See the inline documentation above for detailed patterns and examples.");
    println!();
    println!("Key integration points:");
    println!("  1. Schema registration at server startup");
    println!("  2. Parameter validation in tool handlers");
    println!("  3. Error conversion to MCP errors");
    println!("  4. Configuration-based validation");
    println!("  5. Testing with validation");
    println!();
    println!("For implementation details, see:");
    println!("  - src/validation/integration.rs");
    println!("  - src/server.rs");
    println!("  - docs/validation.md");
}
