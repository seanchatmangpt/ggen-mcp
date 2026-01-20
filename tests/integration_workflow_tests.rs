//! Integration Workflow Tests
//!
//! Comprehensive tests demonstrating the Chicago-style TDD integration workflow harness.
//!
//! These tests execute complete end-to-end workflows using real dependencies:
//! - Real MCP protocol
//! - Real code generation
//! - Real state persistence
//! - Real event emission
//! - Real audit logging

mod harness;

use harness::*;
use anyhow::Result;

// =============================================================================
// User Registration Workflow Tests
// =============================================================================

#[tokio::test]
async fn test_user_registration_workflow_complete() -> Result<()> {
    let result = user_registration_workflow::run_user_registration_workflow().await?;

    // Verify workflow succeeded
    assert_workflow_succeeds(&result).await?;

    // Verify all steps executed
    assert_eq!(result.steps_executed, 6);

    // Verify events emitted
    let required_events = vec![
        "ontology_loaded",
        "code_generated",
        "code_compiled",
        "tool_registered",
        "user_created",
        "persistence_verified",
    ];
    assert_event_sequence(&result.events, &required_events).await?;

    // Verify audit trail
    assert_audit_trail_complete(&result.audit_log).await?;

    Ok(())
}

#[tokio::test]
async fn test_user_registration_workflow_events() -> Result<()> {
    let result = user_registration_workflow::run_user_registration_workflow().await?;

    // Check specific event payloads
    let user_created_event = result.events.iter()
        .find(|e| e.event_type == "user_created")
        .expect("user_created event not found");

    assert_eq!(user_created_event.payload["username"], "john_doe");
    assert!(user_created_event.payload["user_id"].is_string());

    Ok(())
}

#[tokio::test]
async fn test_user_registration_workflow_audit() -> Result<()> {
    let result = user_registration_workflow::run_user_registration_workflow().await?;

    // Verify specific audit entries
    let audit_actions: Vec<&str> = result.audit_log.iter()
        .map(|entry| entry.action.as_str())
        .collect();

    // Should contain workflow steps in audit log
    assert!(audit_actions.iter().any(|a| a.contains("step_started")));
    assert!(audit_actions.iter().any(|a| a.contains("step_completed")));

    Ok(())
}

// =============================================================================
// Order Processing Workflow Tests
// =============================================================================

#[tokio::test]
async fn test_order_processing_workflow_complete() -> Result<()> {
    let result = order_processing_workflow::run_order_processing_workflow().await?;

    // Verify workflow succeeded
    assert_workflow_succeeds(&result).await?;

    // Verify all steps executed
    assert_eq!(result.steps_executed, 8);

    // Verify events emitted in correct order
    let events = result.events;
    assert!(events.iter().position(|e| e.event_type == "order_created") <
            events.iter().position(|e| e.event_type == "item_added"));
    assert!(events.iter().position(|e| e.event_type == "item_added") <
            events.iter().position(|e| e.event_type == "total_calculated"));
    assert!(events.iter().position(|e| e.event_type == "total_calculated") <
            events.iter().position(|e| e.event_type == "payment_processed"));
    assert!(events.iter().position(|e| e.event_type == "payment_processed") <
            events.iter().position(|e| e.event_type == "order_placed"));

    Ok(())
}

#[tokio::test]
async fn test_order_processing_workflow_calculation() -> Result<()> {
    let result = order_processing_workflow::run_order_processing_workflow().await?;

    // Find the total_calculated event
    let total_event = result.events.iter()
        .find(|e| e.event_type == "total_calculated")
        .expect("total_calculated event not found");

    // Verify calculation is correct
    let total = total_event.payload["total"].as_f64().expect("total not found");
    let expected_total = 183.5352; // (2*29.99 + 1*49.99 + 3*19.99) * 1.08

    assert!((total - expected_total).abs() < 0.01,
        "Total mismatch: expected {}, got {}", expected_total, total);

    Ok(())
}

#[tokio::test]
async fn test_order_processing_workflow_payment() -> Result<()> {
    let result = order_processing_workflow::run_order_processing_workflow().await?;

    // Verify payment was processed
    let payment_event = result.events.iter()
        .find(|e| e.event_type == "payment_processed")
        .expect("payment_processed event not found");

    assert!(payment_event.payload["transaction_id"].is_string());
    assert!(payment_event.payload["amount"].is_f64());

    Ok(())
}

// =============================================================================
// MCP Tool Workflow Tests
// =============================================================================

#[tokio::test]
async fn test_mcp_tool_workflow_complete() -> Result<()> {
    let result = mcp_tool_workflow::run_mcp_tool_workflow().await?;

    // Verify workflow succeeded
    assert_workflow_succeeds(&result).await?;

    // Verify all steps executed
    assert_eq!(result.steps_executed, 7);

    // Verify tool lifecycle events
    let required_events = vec![
        "tool_defined",
        "handler_generated",
        "handler_compiled",
        "tool_registered",
        "tool_invoked",
        "response_validated",
        "audit_verified",
    ];
    assert_event_sequence(&result.events, &required_events).await?;

    Ok(())
}

#[tokio::test]
async fn test_mcp_tool_workflow_protocol() -> Result<()> {
    let result = mcp_tool_workflow::run_mcp_tool_workflow().await?;

    // Verify tool was invoked via protocol
    let invoked_event = result.events.iter()
        .find(|e| e.event_type == "tool_invoked")
        .expect("tool_invoked event not found");

    assert_eq!(invoked_event.payload["tool_name"], "process_data");
    assert!(invoked_event.payload["request_id"].is_number());

    Ok(())
}

#[tokio::test]
async fn test_mcp_tool_workflow_audit() -> Result<()> {
    let result = mcp_tool_workflow::run_mcp_tool_workflow().await?;

    // Verify required audit entries
    let required_actions = vec![
        "tool_defined_in_ontology",
        "tool_registered_with_mcp",
        "tool_invoked_via_protocol",
    ];

    for required in &required_actions {
        assert!(
            result.audit_log.iter().any(|entry| entry.action == *required),
            "Required audit action '{}' not found",
            required
        );
    }

    Ok(())
}

// =============================================================================
// Concurrent Workflow Tests
// =============================================================================

#[tokio::test]
async fn test_concurrent_user_registrations() -> Result<()> {
    use tokio::task::JoinSet;

    let mut set = JoinSet::new();

    // Run 5 user registrations concurrently
    for i in 0..5 {
        set.spawn(async move {
            user_registration_workflow::run_user_registration_workflow().await
        });
    }

    // Wait for all to complete
    let mut results = Vec::new();
    while let Some(res) = set.join_next().await {
        results.push(res??);
    }

    // Verify all succeeded
    assert_eq!(results.len(), 5);
    for result in results {
        assert_workflow_succeeds(&result).await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_concurrent_order_processing() -> Result<()> {
    use tokio::join;

    // Run two orders concurrently
    let (result1, result2) = join!(
        order_processing_workflow::run_order_processing_workflow(),
        order_processing_workflow::run_order_processing_workflow()
    );

    // Both should succeed
    assert_workflow_succeeds(&result1?).await?;
    assert_workflow_succeeds(&result2?).await?;

    Ok(())
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[tokio::test]
async fn test_workflow_step_failure_cleanup() -> Result<()> {
    use std::sync::Arc;
    use tokio::sync::RwLock;

    // Create a workflow that fails in a step
    let result = WorkflowBuilder::new("failing_workflow")?
        .step("step1", |ctx, _harness| async move {
            Ok(())
        })
        .step("failing_step", |ctx, _harness| async move {
            Err(anyhow::anyhow!("Intentional failure"))
        })
        .step("step3", |ctx, _harness| async move {
            Ok(())
        })
        .run()
        .await;

    // Should fail
    assert!(result.is_err());

    // Error should mention which step failed
    let err = result.unwrap_err();
    assert!(err.to_string().contains("failing_step"));

    Ok(())
}

#[tokio::test]
async fn test_workflow_assertion_failure() -> Result<()> {
    // Create a workflow that succeeds but fails assertion
    let result = WorkflowBuilder::new("assertion_failing")?
        .step("step1", |ctx, _harness| async move {
            store_data(ctx.clone(), "value", serde_json::json!(42)).await;
            Ok(())
        })
        .assert("wrong_value", |ctx, _harness| async move {
            let value = get_data(ctx.clone(), "value").await
                .and_then(|v| v.as_i64())
                .expect("value not found");

            if value != 100 {
                return Err(anyhow::anyhow!("Expected 100, got {}", value));
            }
            Ok(())
        })
        .run()
        .await;

    // Should fail
    assert!(result.is_err());

    // Error should mention assertion failure
    let err = result.unwrap_err();
    assert!(err.to_string().contains("assertion"));

    Ok(())
}

// =============================================================================
// Helper Function Tests
// =============================================================================

#[tokio::test]
async fn test_store_and_retrieve_data() -> Result<()> {
    let harness = IntegrationWorkflowHarness::new()?;
    let ctx = harness.context.clone();

    // Store data
    store_data(ctx.clone(), "test_key", serde_json::json!("test_value")).await;

    // Retrieve data
    let value = get_data(ctx.clone(), "test_key").await;
    assert_eq!(value, Some(serde_json::json!("test_value")));

    // Non-existent key
    let missing = get_data(ctx.clone(), "missing_key").await;
    assert_eq!(missing, None);

    Ok(())
}

#[tokio::test]
async fn test_state_transitions() -> Result<()> {
    let harness = IntegrationWorkflowHarness::new()?;
    let ctx = harness.context.clone();

    // Initial state
    {
        let c = ctx.read().await;
        assert_eq!(c.current_state, "initial");
    }

    // Transition to state1
    transition_state(ctx.clone(), "state1", "trigger1").await;
    {
        let c = ctx.read().await;
        assert_eq!(c.current_state, "state1");
        assert_eq!(c.state_history.len(), 1);
    }

    // Transition to state2
    transition_state(ctx.clone(), "state2", "trigger2").await;
    {
        let c = ctx.read().await;
        assert_eq!(c.current_state, "state2");
        assert_eq!(c.state_history.len(), 2);
    }

    // Verify consistency
    assert_state_consistent(ctx.clone()).await?;

    Ok(())
}

#[tokio::test]
async fn test_event_emission() -> Result<()> {
    let harness = IntegrationWorkflowHarness::new()?;

    // Emit events
    harness.emit_event("event1", serde_json::json!({"data": 1}), "source1").await;
    harness.emit_event("event2", serde_json::json!({"data": 2}), "source2").await;
    harness.emit_event("event3", serde_json::json!({"data": 3}), "source3").await;

    // Verify events
    let events = harness.events().await;
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].event_type, "event1");
    assert_eq!(events[1].event_type, "event2");
    assert_eq!(events[2].event_type, "event3");

    Ok(())
}

#[tokio::test]
async fn test_audit_logging() -> Result<()> {
    let harness = IntegrationWorkflowHarness::new()?;

    // Add audit entries
    harness.audit("action1", "actor1", serde_json::json!({"detail": 1})).await;
    harness.audit("action2", "actor2", serde_json::json!({"detail": 2})).await;

    // Verify audit log
    let log = harness.audit_log().await;
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].action, "action1");
    assert_eq!(log[1].action, "action2");

    // Verify completeness
    assert_audit_trail_complete(&log).await?;

    Ok(())
}

// =============================================================================
// MCP Protocol Tests
// =============================================================================

#[tokio::test]
async fn test_mcp_protocol_tester_tool_invocation() -> Result<()> {
    let tester = McpProtocolTester::new("http://localhost:8080");

    let response = tester.invoke_tool(
        "test_tool",
        serde_json::json!({"arg": "value"})
    ).await?;

    // Verify JSON-RPC 2.0 structure
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["id"].is_number());
    assert!(response["result"].is_object());

    Ok(())
}

#[tokio::test]
async fn test_mcp_protocol_tester_resource_access() -> Result<()> {
    let tester = McpProtocolTester::new("http://localhost:8080");

    let response = tester.access_resource("test://resource").await?;

    // Verify JSON-RPC 2.0 structure
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["id"].is_number());

    Ok(())
}

#[tokio::test]
async fn test_mcp_protocol_tester_progress_notification() -> Result<()> {
    let tester = McpProtocolTester::new("http://localhost:8080");

    // Should not error
    tester.send_progress("token_123", 50, 100).await?;

    Ok(())
}

#[tokio::test]
async fn test_mcp_protocol_tester_error_response() -> Result<()> {
    let tester = McpProtocolTester::new("http://localhost:8080");

    let response = tester.send_error(
        -32600,
        "Invalid Request",
        Some(serde_json::json!({"detail": "test"}))
    ).await?;

    // Verify error structure
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["id"].is_number());
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32600);
    assert_eq!(response["error"]["message"], "Invalid Request");

    Ok(())
}

// =============================================================================
// Performance Tests
// =============================================================================

#[tokio::test]
async fn test_workflow_performance_user_registration() -> Result<()> {
    let result = user_registration_workflow::run_user_registration_workflow().await?;

    // Workflow should complete in reasonable time (< 5 seconds)
    assert!(
        result.duration_ms < 5000,
        "User registration workflow too slow: {} ms",
        result.duration_ms
    );

    Ok(())
}

#[tokio::test]
async fn test_workflow_performance_order_processing() -> Result<()> {
    let result = order_processing_workflow::run_order_processing_workflow().await?;

    // Workflow should complete in reasonable time (< 10 seconds)
    assert!(
        result.duration_ms < 10000,
        "Order processing workflow too slow: {} ms",
        result.duration_ms
    );

    Ok(())
}

// =============================================================================
// Fixture Loading Tests
// =============================================================================

#[tokio::test]
async fn test_load_ontology_fixture_user() -> Result<()> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/workflows/user_registration/01_ontology.ttl");

    if !path.exists() {
        // Skip if fixture not available
        return Ok(());
    }

    let ontology = load_ontology_fixture(&path).await?;

    // Verify ontology loaded
    assert!(!ontology.is_empty());
    assert!(ontology.contains("@prefix"));

    Ok(())
}

#[tokio::test]
async fn test_load_ontology_fixture_order() -> Result<()> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/workflows/order_processing/01_ontology.ttl");

    if !path.exists() {
        // Skip if fixture not available
        return Ok(());
    }

    let ontology = load_ontology_fixture(&path).await?;

    // Verify ontology loaded
    assert!(!ontology.is_empty());
    assert!(ontology.contains("Order"));

    Ok(())
}
