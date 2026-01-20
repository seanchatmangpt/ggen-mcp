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
//!
//! Refactored to use chicago-tdd-tools framework for:
//! - Proper AAA (Arrange-Act-Assert) pattern enforcement
//! - Type-safe async testing with timeout controls
//! - Result-based error handling
//! - Consistent test structure and maintainability

mod harness;

use chicago_tdd_tools::async_test_with_timeout;
use harness::*;
use anyhow::Result;

// =============================================================================
// User Registration Workflow Tests
// =============================================================================

async_test_with_timeout!(test_user_registration_workflow_complete, 30, {
    // Arrange: No setup required - workflow is self-contained

    // Act: Execute complete user registration workflow
    let result = user_registration_workflow::run_user_registration_workflow().await?;

    // Assert: Verify workflow succeeded with correct steps and events
    assert_workflow_succeeds(&result).await?;
    assert_eq!(result.steps_executed, 6);

    let required_events = vec![
        "ontology_loaded",
        "code_generated",
        "code_compiled",
        "tool_registered",
        "user_created",
        "persistence_verified",
    ];
    assert_event_sequence(&result.events, &required_events).await?;
    assert_audit_trail_complete(&result.audit_log).await?;

    Ok::<(), anyhow::Error>(())
});

async_test_with_timeout!(test_user_registration_workflow_events, 30, {
    // Arrange: No setup required - workflow is self-contained

    // Act: Execute workflow to generate events
    let result = user_registration_workflow::run_user_registration_workflow().await?;

    // Assert: Verify specific event payloads
    let user_created_event = result.events.iter()
        .find(|e| e.event_type == "user_created")
        .ok_or_else(|| anyhow::anyhow!("user_created event not found"))?;

    assert_eq!(user_created_event.payload["username"], "john_doe");
    assert!(user_created_event.payload["user_id"].is_string());

    Ok::<(), anyhow::Error>(())
});

async_test_with_timeout!(test_user_registration_workflow_audit, 30, {
    // Arrange: No setup required - workflow is self-contained

    // Act: Execute workflow to generate audit log
    let result = user_registration_workflow::run_user_registration_workflow().await?;

    // Assert: Verify specific audit entries exist
    let audit_actions: Vec<&str> = result.audit_log.iter()
        .map(|entry| entry.action.as_str())
        .collect();

    assert!(audit_actions.iter().any(|a| a.contains("step_started")));
    assert!(audit_actions.iter().any(|a| a.contains("step_completed")));

    Ok::<(), anyhow::Error>(())
});

// =============================================================================
// Order Processing Workflow Tests
// =============================================================================

async_test_with_timeout!(test_order_processing_workflow_complete, 30, {
    // Arrange: No setup required - workflow is self-contained

    // Act: Execute complete order processing workflow
    let result = order_processing_workflow::run_order_processing_workflow().await?;

    // Assert: Verify workflow succeeded with correct step count and event ordering
    assert_workflow_succeeds(&result).await?;
    assert_eq!(result.steps_executed, 8);

    let events = result.events;
    assert!(events.iter().position(|e| e.event_type == "order_created") <
            events.iter().position(|e| e.event_type == "item_added"));
    assert!(events.iter().position(|e| e.event_type == "item_added") <
            events.iter().position(|e| e.event_type == "total_calculated"));
    assert!(events.iter().position(|e| e.event_type == "total_calculated") <
            events.iter().position(|e| e.event_type == "payment_processed"));
    assert!(events.iter().position(|e| e.event_type == "payment_processed") <
            events.iter().position(|e| e.event_type == "order_placed"));

    Ok::<(), anyhow::Error>(())
});

async_test_with_timeout!(test_order_processing_workflow_calculation, 30, {
    // Arrange: No setup required - workflow is self-contained

    // Act: Execute workflow to generate calculation events
    let result = order_processing_workflow::run_order_processing_workflow().await?;

    // Assert: Verify total calculation is correct
    let total_event = result.events.iter()
        .find(|e| e.event_type == "total_calculated")
        .ok_or_else(|| anyhow::anyhow!("total_calculated event not found"))?;

    let total = total_event.payload["total"].as_f64()
        .ok_or_else(|| anyhow::anyhow!("total not found in payload"))?;
    let expected_total = 183.5352; // (2*29.99 + 1*49.99 + 3*19.99) * 1.08

    assert!((total - expected_total).abs() < 0.01,
        "Total mismatch: expected {}, got {}", expected_total, total);

    Ok::<(), anyhow::Error>(())
});

async_test_with_timeout!(test_order_processing_workflow_payment, 30, {
    // Arrange: No setup required - workflow is self-contained

    // Act: Execute workflow to generate payment events
    let result = order_processing_workflow::run_order_processing_workflow().await?;

    // Assert: Verify payment was processed correctly
    let payment_event = result.events.iter()
        .find(|e| e.event_type == "payment_processed")
        .ok_or_else(|| anyhow::anyhow!("payment_processed event not found"))?;

    assert!(payment_event.payload["transaction_id"].is_string());
    assert!(payment_event.payload["amount"].is_f64());

    Ok::<(), anyhow::Error>(())
});

// =============================================================================
// MCP Tool Workflow Tests
// =============================================================================

async_test_with_timeout!(test_mcp_tool_workflow_complete, 30, {
    // Arrange: No setup required - workflow is self-contained

    // Act: Execute complete MCP tool workflow
    let result = mcp_tool_workflow::run_mcp_tool_workflow().await?;

    // Assert: Verify workflow succeeded with correct steps and tool lifecycle events
    assert_workflow_succeeds(&result).await?;
    assert_eq!(result.steps_executed, 7);

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

    Ok::<(), anyhow::Error>(())
});

async_test_with_timeout!(test_mcp_tool_workflow_protocol, 30, {
    // Arrange: No setup required - workflow is self-contained

    // Act: Execute workflow to generate protocol events
    let result = mcp_tool_workflow::run_mcp_tool_workflow().await?;

    // Assert: Verify tool was invoked via protocol
    let invoked_event = result.events.iter()
        .find(|e| e.event_type == "tool_invoked")
        .ok_or_else(|| anyhow::anyhow!("tool_invoked event not found"))?;

    assert_eq!(invoked_event.payload["tool_name"], "process_data");
    assert!(invoked_event.payload["request_id"].is_number());

    Ok::<(), anyhow::Error>(())
});

async_test_with_timeout!(test_mcp_tool_workflow_audit, 30, {
    // Arrange: Define required audit actions
    let required_actions = vec![
        "tool_defined_in_ontology",
        "tool_registered_with_mcp",
        "tool_invoked_via_protocol",
    ];

    // Act: Execute workflow to generate audit log
    let result = mcp_tool_workflow::run_mcp_tool_workflow().await?;

    // Assert: Verify all required audit entries exist
    for required in &required_actions {
        assert!(
            result.audit_log.iter().any(|entry| entry.action == *required),
            "Required audit action '{}' not found",
            required
        );
    }

    Ok::<(), anyhow::Error>(())
});

// =============================================================================
// Concurrent Workflow Tests
// =============================================================================

async_test_with_timeout!(test_concurrent_user_registrations, 30, {
    use tokio::task::JoinSet;

    // Arrange: Create JoinSet for concurrent execution
    let mut set = JoinSet::new();

    // Act: Spawn 5 concurrent user registration workflows
    for _i in 0..5 {
        set.spawn(async move {
            user_registration_workflow::run_user_registration_workflow().await
        });
    }

    // Wait for all workflows to complete
    let mut results = Vec::new();
    while let Some(res) = set.join_next().await {
        results.push(res??);
    }

    // Assert: Verify all workflows succeeded
    assert_eq!(results.len(), 5);
    for result in results {
        assert_workflow_succeeds(&result).await?;
    }

    Ok::<(), anyhow::Error>(())
});

async_test_with_timeout!(test_concurrent_order_processing, 30, {
    use tokio::join;

    // Arrange & Act: Run two orders concurrently
    let (result1, result2) = join!(
        order_processing_workflow::run_order_processing_workflow(),
        order_processing_workflow::run_order_processing_workflow()
    );

    // Assert: Verify both workflows succeeded
    assert_workflow_succeeds(&result1?).await?;
    assert_workflow_succeeds(&result2?).await?;

    Ok::<(), anyhow::Error>(())
});

// =============================================================================
// Error Handling Tests
// =============================================================================

async_test_with_timeout!(test_workflow_step_failure_cleanup, 30, {
    use std::sync::Arc;
    use tokio::sync::RwLock;

    // Arrange: Create a workflow with an intentional failure in middle step
    let workflow_builder = WorkflowBuilder::new("failing_workflow")?
        .step("step1", |_ctx, _harness| async move {
            Ok(())
        })
        .step("failing_step", |_ctx, _harness| async move {
            Err(anyhow::anyhow!("Intentional failure"))
        })
        .step("step3", |_ctx, _harness| async move {
            Ok(())
        });

    // Act: Run the workflow (expecting failure)
    let result = workflow_builder.run().await;

    // Assert: Verify workflow failed with correct error message
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("failing_step"));

    Ok::<(), anyhow::Error>(())
});

async_test_with_timeout!(test_workflow_assertion_failure, 30, {
    // Arrange: Create a workflow with a failing assertion
    let workflow_builder = WorkflowBuilder::new("assertion_failing")?
        .step("step1", |ctx, _harness| async move {
            store_data(ctx.clone(), "value", serde_json::json!(42)).await;
            Ok(())
        })
        .assert("wrong_value", |ctx, _harness| async move {
            let value = get_data(ctx.clone(), "value").await
                .and_then(|v| v.as_i64())
                .ok_or_else(|| anyhow::anyhow!("value not found"))?;

            if value != 100 {
                return Err(anyhow::anyhow!("Expected 100, got {}", value));
            }
            Ok(())
        });

    // Act: Run the workflow (expecting assertion failure)
    let result = workflow_builder.run().await;

    // Assert: Verify workflow failed due to assertion
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("assertion") || err.to_string().contains("Expected 100"));

    Ok::<(), anyhow::Error>(())
});

// =============================================================================
// Helper Function Tests
// =============================================================================

async_test_with_timeout!(test_store_and_retrieve_data, 30, {
    // Arrange: Create harness and context
    let harness = IntegrationWorkflowHarness::new()?;
    let ctx = harness.context.clone();

    // Act: Store data in context
    store_data(ctx.clone(), "test_key", serde_json::json!("test_value")).await;

    // Assert: Verify data can be retrieved correctly
    let value = get_data(ctx.clone(), "test_key").await;
    assert_eq!(value, Some(serde_json::json!("test_value")));

    let missing = get_data(ctx.clone(), "missing_key").await;
    assert_eq!(missing, None);

    Ok::<(), anyhow::Error>(())
});

async_test_with_timeout!(test_state_transitions, 30, {
    // Arrange: Create harness and context with initial state
    let harness = IntegrationWorkflowHarness::new()?;
    let ctx = harness.context.clone();

    {
        let c = ctx.read().await;
        assert_eq!(c.current_state, "initial");
    }

    // Act: Perform state transitions
    transition_state(ctx.clone(), "state1", "trigger1").await;
    {
        let c = ctx.read().await;
        assert_eq!(c.current_state, "state1");
        assert_eq!(c.state_history.len(), 1);
    }

    transition_state(ctx.clone(), "state2", "trigger2").await;

    // Assert: Verify final state and history
    {
        let c = ctx.read().await;
        assert_eq!(c.current_state, "state2");
        assert_eq!(c.state_history.len(), 2);
    }

    assert_state_consistent(ctx.clone()).await?;

    Ok::<(), anyhow::Error>(())
});

async_test_with_timeout!(test_event_emission, 30, {
    // Arrange: Create harness for event emission
    let harness = IntegrationWorkflowHarness::new()?;

    // Act: Emit multiple events
    harness.emit_event("event1", serde_json::json!({"data": 1}), "source1").await;
    harness.emit_event("event2", serde_json::json!({"data": 2}), "source2").await;
    harness.emit_event("event3", serde_json::json!({"data": 3}), "source3").await;

    // Assert: Verify events were recorded in correct order
    let events = harness.events().await;
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].event_type, "event1");
    assert_eq!(events[1].event_type, "event2");
    assert_eq!(events[2].event_type, "event3");

    Ok::<(), anyhow::Error>(())
});

async_test_with_timeout!(test_audit_logging, 30, {
    // Arrange: Create harness for audit logging
    let harness = IntegrationWorkflowHarness::new()?;

    // Act: Add audit entries
    harness.audit("action1", "actor1", serde_json::json!({"detail": 1})).await;
    harness.audit("action2", "actor2", serde_json::json!({"detail": 2})).await;

    // Assert: Verify audit log entries
    let log = harness.audit_log().await;
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].action, "action1");
    assert_eq!(log[1].action, "action2");

    assert_audit_trail_complete(&log).await?;

    Ok::<(), anyhow::Error>(())
});

// =============================================================================
// MCP Protocol Tests
// =============================================================================

async_test_with_timeout!(test_mcp_protocol_tester_tool_invocation, 30, {
    // Arrange: Create MCP protocol tester
    let tester = McpProtocolTester::new("http://localhost:8080");

    // Act: Invoke a tool via protocol
    let response = tester.invoke_tool(
        "test_tool",
        serde_json::json!({"arg": "value"})
    ).await?;

    // Assert: Verify JSON-RPC 2.0 structure
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["id"].is_number());
    assert!(response["result"].is_object());

    Ok::<(), anyhow::Error>(())
});

async_test_with_timeout!(test_mcp_protocol_tester_resource_access, 30, {
    // Arrange: Create MCP protocol tester
    let tester = McpProtocolTester::new("http://localhost:8080");

    // Act: Access a resource via protocol
    let response = tester.access_resource("test://resource").await?;

    // Assert: Verify JSON-RPC 2.0 structure
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["id"].is_number());

    Ok::<(), anyhow::Error>(())
});

async_test_with_timeout!(test_mcp_protocol_tester_progress_notification, 30, {
    // Arrange: Create MCP protocol tester
    let tester = McpProtocolTester::new("http://localhost:8080");

    // Act & Assert: Send progress notification (should not error)
    tester.send_progress("token_123", 50, 100).await?;

    Ok::<(), anyhow::Error>(())
});

async_test_with_timeout!(test_mcp_protocol_tester_error_response, 30, {
    // Arrange: Create MCP protocol tester and error parameters
    let tester = McpProtocolTester::new("http://localhost:8080");

    // Act: Send error response
    let response = tester.send_error(
        -32600,
        "Invalid Request",
        Some(serde_json::json!({"detail": "test"}))
    ).await?;

    // Assert: Verify error structure
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["id"].is_number());
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32600);
    assert_eq!(response["error"]["message"], "Invalid Request");

    Ok::<(), anyhow::Error>(())
});

// =============================================================================
// Performance Tests
// =============================================================================

async_test_with_timeout!(test_workflow_performance_user_registration, 30, {
    // Arrange: No setup required - workflow is self-contained

    // Act: Execute workflow and measure duration
    let result = user_registration_workflow::run_user_registration_workflow().await?;

    // Assert: Verify workflow completes within performance SLA (< 5 seconds)
    assert!(
        result.duration_ms < 5000,
        "User registration workflow too slow: {} ms",
        result.duration_ms
    );

    Ok::<(), anyhow::Error>(())
});

async_test_with_timeout!(test_workflow_performance_order_processing, 30, {
    // Arrange: No setup required - workflow is self-contained

    // Act: Execute workflow and measure duration
    let result = order_processing_workflow::run_order_processing_workflow().await?;

    // Assert: Verify workflow completes within performance SLA (< 10 seconds)
    assert!(
        result.duration_ms < 10000,
        "Order processing workflow too slow: {} ms",
        result.duration_ms
    );

    Ok::<(), anyhow::Error>(())
});

// =============================================================================
// Fixture Loading Tests
// =============================================================================

async_test_with_timeout!(test_load_ontology_fixture_user, 30, {
    // Arrange: Construct path to user registration ontology fixture
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/workflows/user_registration/01_ontology.ttl");

    if !path.exists() {
        // Skip test if fixture not available
        return Ok(());
    }

    // Act: Load ontology from fixture
    let ontology = load_ontology_fixture(&path).await?;

    // Assert: Verify ontology loaded correctly
    assert!(!ontology.is_empty());
    assert!(ontology.contains("@prefix"));

    Ok::<(), anyhow::Error>(())
});

async_test_with_timeout!(test_load_ontology_fixture_order, 30, {
    // Arrange: Construct path to order processing ontology fixture
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/workflows/order_processing/01_ontology.ttl");

    if !path.exists() {
        // Skip test if fixture not available
        return Ok(());
    }

    // Act: Load ontology from fixture
    let ontology = load_ontology_fixture(&path).await?;

    // Assert: Verify ontology loaded correctly with Order entity
    assert!(!ontology.is_empty());
    assert!(ontology.contains("Order"));

    Ok::<(), anyhow::Error>(())
});
