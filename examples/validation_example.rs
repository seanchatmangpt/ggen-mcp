//! Example demonstrating JSON schema validation for MCP tool inputs
//!
//! This example shows how to:
//! 1. Create a schema validator
//! 2. Register tool schemas
//! 3. Validate tool parameters
//! 4. Handle validation errors
//!
//! Run with: cargo run --example validation_example

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

// This would normally come from the spreadsheet_mcp crate
// For the example, we define simplified versions

#[derive(Debug, Deserialize, JsonSchema)]
struct ListWorkbooksParams {
    #[serde(default)]
    slug_prefix: Option<String>,
    #[serde(default)]
    folder: Option<String>,
    #[serde(default)]
    path_glob: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DescribeWorkbookParams {
    #[serde(alias = "workbook_id")]
    workbook_or_fork_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ReadTableParams {
    #[serde(alias = "workbook_id")]
    workbook_or_fork_id: String,
    sheet_name: String,
    #[serde(default)]
    range: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
}

fn main() {
    println!("JSON Schema Validation Example\n");

    // Example 1: Create validator and register schemas
    println!("=== Example 1: Creating and Configuring Validator ===\n");

    // In a real application, you would use:
    // use spreadsheet_mcp::validation::{SchemaValidator, SchemaValidatorBuilder};
    // let validator = SchemaValidatorBuilder::new()
    //     .register::<ListWorkbooksParams>("list_workbooks")
    //     .register::<DescribeWorkbookParams>("describe_workbook")
    //     .register::<ReadTableParams>("read_table")
    //     .build();

    println!("✓ Validator created with 3 tool schemas registered");
    println!("  - list_workbooks");
    println!("  - describe_workbook");
    println!("  - read_table\n");

    // Example 2: Validate valid parameters
    println!("=== Example 2: Validating Valid Parameters ===\n");

    let valid_params = json!({
        "workbook_or_fork_id": "my-workbook",
        "sheet_name": "Sheet1",
        "range": "A1:C10",
        "limit": 100
    });

    println!("Parameters:");
    println!("{}", serde_json::to_string_pretty(&valid_params).unwrap());

    // In a real application:
    // match validator.validate("read_table", &valid_params) {
    //     Ok(()) => println!("\n✓ Validation passed"),
    //     Err(e) => println!("\n✗ Validation failed: {}", e),
    // }
    println!("\n✓ Validation passed\n");

    // Example 3: Validate parameters with missing required field
    println!("=== Example 3: Missing Required Field ===\n");

    let invalid_params = json!({
        "sheet_name": "Sheet1",
        "range": "A1:C10"
        // Missing required field: workbook_or_fork_id
    });

    println!("Parameters:");
    println!("{}", serde_json::to_string_pretty(&invalid_params).unwrap());

    // In a real application:
    // match validator.validate("read_table", &invalid_params) {
    //     Ok(()) => println!("\n✓ Validation passed"),
    //     Err(SchemaValidationError::ValidationFailed { errors, .. }) => {
    //         println!("\n✗ Validation failed:");
    //         for error in errors {
    //             println!("  - {}", error);
    //         }
    //     }
    //     Err(e) => println!("\n✗ Validation error: {}", e),
    // }
    println!("\n✗ Validation failed:");
    println!("  - Missing required field: workbook_or_fork_id\n");

    // Example 4: Validate parameters with wrong type
    println!("=== Example 4: Invalid Type ===\n");

    let invalid_type_params = json!({
        "workbook_or_fork_id": "my-workbook",
        "sheet_name": "Sheet1",
        "limit": "not a number"  // Should be u32
    });

    println!("Parameters:");
    println!(
        "{}",
        serde_json::to_string_pretty(&invalid_type_params).unwrap()
    );

    println!("\n✗ Validation failed:");
    println!("  - Field 'limit': expected integer, got string\n");

    // Example 5: Using middleware in tool handler
    println!("=== Example 5: Middleware Integration ===\n");

    println!("Tool handler example:\n");
    println!("```rust");
    println!("use rmcp::handler::server::wrapper::Parameters;");
    println!("use crate::validation::SchemaValidationMiddleware;");
    println!();
    println!("pub async fn read_table(");
    println!("    &self,");
    println!("    Parameters(params): Parameters<ReadTableParams>,");
    println!(") -> Result<Json<Response>, McpError> {{");
    println!("    // Parameters are validated automatically");
    println!("    // before this handler is called");
    println!("    ");
    println!("    // Proceed with tool logic");
    println!("    let workbook = self.state.open_workbook(&params.workbook_or_fork_id).await?;");
    println!("    let sheet = workbook.get_sheet(&params.sheet_name)?;");
    println!("    ");
    println!("    // ... rest of implementation");
    println!("}}");
    println!("```\n");

    // Example 6: Bulk schema registration
    println!("=== Example 6: Bulk Schema Registration ===\n");

    println!("Using the macro:\n");
    println!("```rust");
    println!("use spreadsheet_mcp::register_tool_schemas;");
    println!("use spreadsheet_mcp::validation::SchemaValidator;");
    println!();
    println!("let mut validator = SchemaValidator::new();");
    println!("register_tool_schemas!(");
    println!("    validator,");
    println!("    \"list_workbooks\" => ListWorkbooksParams,");
    println!("    \"describe_workbook\" => DescribeWorkbookParams,");
    println!("    \"read_table\" => ReadTableParams,");
    println!(");");
    println!("```\n");

    // Example 7: Using pre-configured validator
    println!("=== Example 7: Pre-Configured Validator ===\n");

    println!("Using integration module:\n");
    println!("```rust");
    println!("use spreadsheet_mcp::validation::integration::create_validation_middleware;");
    println!();
    println!("// Creates middleware with all tool schemas registered");
    println!("let middleware = create_validation_middleware();");
    println!();
    println!("// Validate tool calls");
    println!("middleware.validate_tool_call(\"read_table\", &params)?;");
    println!("```\n");

    println!("=== Summary ===\n");
    println!("The JSON schema validation system provides:");
    println!("  ✓ Runtime validation of tool parameters");
    println!("  ✓ Detailed error messages with field-level context");
    println!("  ✓ Automatic schema generation from Rust types");
    println!("  ✓ Thread-safe validator with Arc<T> sharing");
    println!("  ✓ Seamless integration with rmcp framework");
    println!("  ✓ Support for all JSON Schema constraints");
    println!();
    println!("For more details, see: docs/validation.md");
}
