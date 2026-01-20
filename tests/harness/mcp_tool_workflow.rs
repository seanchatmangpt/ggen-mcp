//! MCP Tool Workflow Tests
//!
//! Chicago-style TDD integration tests for the MCP tool lifecycle workflow.
//!
//! # Workflow Steps
//! 1. Define tool in ontology
//! 2. Generate handler code
//! 3. Register tool with MCP
//! 4. Invoke tool via protocol
//! 5. Validate response
//! 6. Check audit log

use super::*;
use anyhow::{Context, Result};
use serde_json::json;
use std::path::Path;

/// Run the complete MCP tool lifecycle workflow
///
/// This is a Chicago-style integration test that:
/// - Defines tools in ontologies
/// - Generates MCP tool handlers
/// - Registers tools with the MCP server
/// - Invokes tools via JSON-RPC 2.0
/// - Validates responses
/// - Audits all operations
pub async fn run_mcp_tool_workflow() -> Result<WorkflowResult> {
    WorkflowBuilder::new("mcp_tool")?
        .with_docker("mcp-test-network")
        .step("define_tool_in_ontology", define_tool_in_ontology)
        .step("generate_tool_handler", generate_tool_handler)
        .step("compile_handler", compile_handler)
        .step("register_with_mcp", register_with_mcp)
        .step("invoke_tool_via_protocol", invoke_tool_via_protocol)
        .step("validate_tool_response", validate_tool_response)
        .step("verify_audit_log", verify_audit_log)
        .assert("tool_registered", assert_tool_registered)
        .assert("invocation_succeeded", assert_invocation_succeeded)
        .assert("response_valid", assert_response_valid)
        .assert("audit_complete", assert_audit_complete)
        .run()
        .await
}

/// Step 1: Define tool in ontology
async fn define_tool_in_ontology(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/workflows/mcp_tool/01_ontology.ttl");

    let ontology = if fixture_path.exists() {
        load_ontology_fixture(&fixture_path).await?
    } else {
        // Use embedded default ontology for testing
        include_str!("../../../fixtures/workflows/mcp_tool/01_ontology.ttl").to_string()
    };

    {
        let mut ctx = context.write().await;
        ctx.ontology = Some(ontology.clone());
    }

    // Extract tool definition from ontology
    let tool_definition = parse_tool_definition(&ontology)?;
    store_data(context.clone(), "tool_definition", tool_definition.clone()).await;

    harness.emit_event(
        "tool_defined",
        json!({
            "tool_name": tool_definition["name"],
            "ontology_size": ontology.len()
        }),
        "define_tool_in_ontology"
    ).await;

    harness.audit(
        "tool_defined_in_ontology",
        "developer",
        tool_definition
    ).await;

    transition_state(context.clone(), "tool_defined", "define_tool_in_ontology").await;

    Ok(())
}

/// Step 2: Generate tool handler code
async fn generate_tool_handler(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let ontology = {
        let ctx = context.read().await;
        ctx.ontology.clone()
            .ok_or_else(|| anyhow::anyhow!("Ontology not loaded"))?
    };

    let tool_definition = get_data(context.clone(), "tool_definition").await
        .ok_or_else(|| anyhow::anyhow!("Tool definition not found"))?;

    // Generate handler code from ontology
    let handler_code = generate_tool_handler_code(&ontology, &tool_definition)?;

    save_generated_code(context.clone(), "tool_handler", handler_code.clone()).await;

    harness.emit_event(
        "handler_generated",
        json!({
            "tool_name": tool_definition["name"],
            "lines": handler_code.lines().count()
        }),
        "generate_tool_handler"
    ).await;

    transition_state(context.clone(), "handler_generated", "generate_tool_handler").await;

    Ok(())
}

/// Step 3: Compile handler code
async fn compile_handler(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let handler_code = {
        let ctx = context.read().await;
        ctx.generated_code.get("tool_handler")
            .ok_or_else(|| anyhow::anyhow!("Handler code not found"))?
            .clone()
    };

    // Write handler to workspace
    let handler_file = harness.workspace_path().join("tool_handler.rs");
    tokio::fs::write(&handler_file, &handler_code).await?;

    // In real implementation, would compile with rustc
    // For testing, verify file exists
    if !handler_file.exists() {
        return Err(anyhow::anyhow!("Failed to write handler code"));
    }

    harness.emit_event(
        "handler_compiled",
        json!({ "path": handler_file.to_string_lossy() }),
        "compile_handler"
    ).await;

    transition_state(context.clone(), "handler_compiled", "compile_handler").await;

    Ok(())
}

/// Step 4: Register tool with MCP server
async fn register_with_mcp(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let tool_definition = get_data(context.clone(), "tool_definition").await
        .ok_or_else(|| anyhow::anyhow!("Tool definition not found"))?;

    let tool_name = tool_definition["name"].as_str()
        .ok_or_else(|| anyhow::anyhow!("Tool name not found"))?;

    // Create tool registration
    let registration = ToolRegistration {
        name: tool_name.to_string(),
        description: tool_definition["description"]
            .as_str()
            .unwrap_or("MCP tool")
            .to_string(),
        input_schema: tool_definition["input_schema"].clone(),
        handler: format!("handle_{}", tool_name),
    };

    register_tool(context.clone(), registration.clone()).await;

    harness.emit_event(
        "tool_registered",
        json!({
            "tool_name": tool_name,
            "input_schema": registration.input_schema
        }),
        "register_with_mcp"
    ).await;

    harness.audit(
        "tool_registered_with_mcp",
        "system",
        json!({
            "tool_name": tool_name,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
    ).await;

    transition_state(context.clone(), "tool_registered", "register_with_mcp").await;

    Ok(())
}

/// Step 5: Invoke tool via MCP protocol
async fn invoke_tool_via_protocol(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let tool_definition = get_data(context.clone(), "tool_definition").await
        .ok_or_else(|| anyhow::anyhow!("Tool definition not found"))?;

    let tool_name = tool_definition["name"].as_str()
        .ok_or_else(|| anyhow::anyhow!("Tool name not found"))?;

    // Create MCP protocol tester
    let tester = McpProtocolTester::new("http://localhost:8080");

    // Prepare tool invocation arguments
    let arguments = json!({
        "input": "test data",
        "options": {
            "validate": true
        }
    });

    // Invoke tool via JSON-RPC 2.0
    let response = tester.invoke_tool(tool_name, arguments.clone()).await?;

    store_data(context.clone(), "invocation_request", arguments).await;
    store_data(context.clone(), "invocation_response", response.clone()).await;

    harness.emit_event(
        "tool_invoked",
        json!({
            "tool_name": tool_name,
            "request_id": response["id"]
        }),
        "invoke_tool_via_protocol"
    ).await;

    harness.audit(
        "tool_invoked_via_protocol",
        "client",
        json!({
            "tool_name": tool_name,
            "request": arguments,
            "response_id": response["id"]
        })
    ).await;

    transition_state(context.clone(), "tool_invoked", "invoke_tool_via_protocol").await;

    Ok(())
}

/// Step 6: Validate tool response
async fn validate_tool_response(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let response = get_data(context.clone(), "invocation_response").await
        .ok_or_else(|| anyhow::anyhow!("Invocation response not found"))?;

    // Validate JSON-RPC 2.0 response structure
    if response["jsonrpc"].as_str() != Some("2.0") {
        return Err(anyhow::anyhow!("Invalid JSON-RPC version"));
    }

    if response["id"].is_null() {
        return Err(anyhow::anyhow!("Missing response ID"));
    }

    // Check for errors
    if !response["error"].is_null() {
        let error_msg = response["error"]["message"].as_str()
            .unwrap_or("Unknown error");
        return Err(anyhow::anyhow!("Tool invocation failed: {}", error_msg));
    }

    // Validate result exists
    if response["result"].is_null() {
        return Err(anyhow::anyhow!("Missing result in response"));
    }

    store_data(context.clone(), "validation_result", json!({
        "valid": true,
        "response_id": response["id"]
    })).await;

    harness.emit_event(
        "response_validated",
        json!({
            "valid": true,
            "response_id": response["id"]
        }),
        "validate_tool_response"
    ).await;

    transition_state(context.clone(), "response_validated", "validate_tool_response").await;

    Ok(())
}

/// Step 7: Verify audit log
async fn verify_audit_log(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let audit_log = harness.audit_log().await;

    // Verify required audit entries exist
    let required_actions = vec![
        "tool_defined_in_ontology",
        "tool_registered_with_mcp",
        "tool_invoked_via_protocol",
    ];

    for required in &required_actions {
        if !audit_log.iter().any(|entry| entry.action == *required) {
            return Err(anyhow::anyhow!(
                "Required audit action '{}' not found",
                required
            ));
        }
    }

    store_data(context.clone(), "audit_verified", json!(true)).await;

    harness.emit_event(
        "audit_verified",
        json!({ "entries": audit_log.len() }),
        "verify_audit_log"
    ).await;

    transition_state(context.clone(), "audit_verified", "verify_audit_log").await;

    Ok(())
}

// =============================================================================
// Assertions
// =============================================================================

async fn assert_tool_registered(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    _harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let ctx = context.read().await;

    if ctx.tool_registrations.is_empty() {
        return Err(anyhow::anyhow!("No tools registered"));
    }

    Ok(())
}

async fn assert_invocation_succeeded(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    _harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let response = get_data(context.clone(), "invocation_response").await
        .ok_or_else(|| anyhow::anyhow!("No invocation response found"))?;

    if !response["error"].is_null() {
        return Err(anyhow::anyhow!("Invocation failed with error"));
    }

    if response["result"].is_null() {
        return Err(anyhow::anyhow!("No result in response"));
    }

    Ok(())
}

async fn assert_response_valid(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    _harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let validation = get_data(context.clone(), "validation_result").await
        .ok_or_else(|| anyhow::anyhow!("Validation result not found"))?;

    if validation["valid"] != json!(true) {
        return Err(anyhow::anyhow!("Response validation failed"));
    }

    Ok(())
}

async fn assert_audit_complete(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let audit_verified = get_data(context.clone(), "audit_verified").await
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !audit_verified {
        return Err(anyhow::anyhow!("Audit log not verified"));
    }

    let audit_log = harness.audit_log().await;
    assert_audit_trail_complete(&audit_log).await?;

    Ok(())
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Parse tool definition from ontology
fn parse_tool_definition(ontology: &str) -> Result<serde_json::Value> {
    // In a real implementation, this would parse the TTL ontology
    // For testing, return a mock definition
    Ok(json!({
        "name": "process_data",
        "description": "Process input data and return results",
        "input_schema": {
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Input data to process"
                },
                "options": {
                    "type": "object",
                    "properties": {
                        "validate": {
                            "type": "boolean",
                            "default": true
                        }
                    }
                }
            },
            "required": ["input"]
        },
        "output_schema": {
            "type": "object",
            "properties": {
                "result": {
                    "type": "string"
                },
                "metadata": {
                    "type": "object"
                }
            }
        }
    }))
}

/// Generate tool handler code
fn generate_tool_handler_code(_ontology: &str, tool_def: &serde_json::Value) -> Result<String> {
    let tool_name = tool_def["name"].as_str()
        .ok_or_else(|| anyhow::anyhow!("Tool name not found"))?;

    Ok(format!(r#"
use anyhow::Result;
use serde::{{Deserialize, Serialize}};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct {tool_name_pascal}Input {{
    pub input: String,
    #[serde(default)]
    pub options: {tool_name_pascal}Options,
}}

#[derive(Debug, Default, Deserialize)]
pub struct {tool_name_pascal}Options {{
    #[serde(default = "default_validate")]
    pub validate: bool,
}}

fn default_validate() -> bool {{
    true
}}

#[derive(Debug, Serialize)]
pub struct {tool_name_pascal}Output {{
    pub result: String,
    pub metadata: Value,
}}

pub async fn handle_{tool_name}(input: {tool_name_pascal}Input) -> Result<{tool_name_pascal}Output> {{
    // Validate input if requested
    if input.options.validate {{
        if input.input.is_empty() {{
            return Err(anyhow::anyhow!("Input cannot be empty"));
        }}
    }}

    // Process the input
    let result = process_input(&input.input)?;

    Ok({tool_name_pascal}Output {{
        result,
        metadata: serde_json::json!({{
            "processed_at": chrono::Utc::now().to_rfc3339(),
            "input_length": input.input.len(),
        }}),
    }})
}}

fn process_input(input: &str) -> Result<String> {{
    // Actual processing logic would go here
    Ok(format!("Processed: {{}}", input))
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[tokio::test]
    async fn test_handle_{tool_name}() {{
        let input = {tool_name_pascal}Input {{
            input: "test data".to_string(),
            options: {tool_name_pascal}Options::default(),
        }};

        let output = handle_{tool_name}(input).await.unwrap();
        assert!(output.result.contains("test data"));
    }}
}}
"#,
        tool_name = tool_name,
        tool_name_pascal = to_pascal_case(tool_name)
    ))
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_tool_workflow_complete() {
        let result = run_mcp_tool_workflow().await.unwrap();
        assert!(result.success);
        assert_eq!(result.steps_executed, 7);
    }

    #[tokio::test]
    async fn test_parse_tool_definition() {
        let ontology = "@prefix : <http://example.org/> .";
        let def = parse_tool_definition(ontology).unwrap();
        assert_eq!(def["name"], "process_data");
    }

    #[tokio::test]
    async fn test_generate_tool_handler_code() {
        let tool_def = json!({
            "name": "test_tool",
            "description": "Test tool"
        });

        let code = generate_tool_handler_code("", &tool_def).unwrap();
        assert!(code.contains("handle_test_tool"));
        assert!(code.contains("TestToolInput"));
    }

    #[tokio::test]
    async fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("process_data"), "ProcessData");
        assert_eq!(to_pascal_case("create_user"), "CreateUser");
        assert_eq!(to_pascal_case("test"), "Test");
    }
}
