# Chicago-Style TDD Integration Workflow Harness

## Overview

This document describes the comprehensive integration workflow test harness for ggen-mcp, implementing Chicago-style Test-Driven Development (TDD) principles.

**Chicago-style TDD** emphasizes testing with real dependencies rather than mocks, focusing on:
- Real MCP protocol communication
- Real Docker containers
- Real state persistence
- Complete end-to-end workflows
- Behavior verification over implementation details

## Philosophy

### Chicago vs. London School TDD

| Aspect | London School (Mockist) | Chicago School (Classicist) |
|--------|------------------------|----------------------------|
| **Dependencies** | Mock everything | Use real objects when possible |
| **Focus** | Interactions | State and behavior |
| **Tests** | White-box | Black-box |
| **Refactoring** | Fragile (mocks coupled to implementation) | Robust (tests coupled to behavior) |
| **Confidence** | Lower (mocks may not match reality) | Higher (tests use real components) |

Our harness follows the **Chicago school** because:
1. **Higher confidence**: Tests use real MCP protocol, real Docker, real persistence
2. **Better refactoring**: Tests remain valid as internal implementation changes
3. **Find integration issues**: Real components reveal real problems
4. **Simpler tests**: No complex mock setup, just test actual behavior

### 80/20 Principle

We focus on the 20% of workflows that provide 80% of value:

1. **User Registration**: Most common CRUD workflow
2. **Order Processing**: Complex multi-step business workflow
3. **MCP Tool Lifecycle**: Core infrastructure workflow

## Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────┐
│         IntegrationWorkflowHarness                  │
├─────────────────────────────────────────────────────┤
│  - Workspace Management                             │
│  - Event Tracking                                   │
│  - Audit Logging                                    │
│  - Docker Integration                               │
└───────────────┬─────────────────────────────────────┘
                │
    ┌───────────┴──────────────┬──────────────────────┐
    │                          │                      │
┌───▼────────┐      ┌──────────▼──────┐    ┌─────────▼────────┐
│ Workflow   │      │   Workflow      │    │   Assertions     │
│ Builder    │      │   Context       │    │   & Helpers      │
└────────────┘      └─────────────────┘    └──────────────────┘
```

### Core Components

#### 1. IntegrationWorkflowHarness

The main harness providing:
- Temporary workspace for test isolation
- Event emission and tracking
- Audit log management
- Docker container lifecycle
- Cleanup on test completion

```rust
let harness = IntegrationWorkflowHarness::new()?;
let workspace = harness.workspace_path(); // Isolated temp directory
harness.emit_event("user_created", payload, "step_name").await;
harness.audit("action", "actor", details).await;
```

#### 2. WorkflowBuilder

Fluent API for defining workflows:

```rust
WorkflowBuilder::new("workflow_name")?
    .step("step_1", |ctx, harness| async move {
        // Execute step logic
        Ok(())
    })
    .step("step_2", |ctx, harness| async move {
        // Execute step logic
        Ok(())
    })
    .assert("assertion_1", |ctx, harness| async move {
        // Verify expected state
        Ok(())
    })
    .run()
    .await?
```

#### 3. WorkflowContext

Shared state across workflow steps:

```rust
pub struct WorkflowContext {
    workflow_name: String,
    data: HashMap<String, Value>,
    state_history: Vec<StateTransition>,
    current_state: String,
    ontology: Option<String>,
    generated_code: HashMap<String, String>,
    tool_registrations: HashMap<String, ToolRegistration>,
    storage_path: PathBuf,
}
```

Access via helper functions:
```rust
store_data(ctx.clone(), "key", json!("value")).await;
let value = get_data(ctx.clone(), "key").await;
transition_state(ctx.clone(), "new_state", "trigger").await;
```

## Workflows

### 1. User Registration Workflow

Complete user registration from ontology to persistence.

**Steps:**
1. Load user ontology (TTL)
2. Generate user aggregate code (Rust)
3. Compile generated code
4. Register create_user MCP tool
5. Execute create_user tool
6. Verify user persisted to storage

**Fixtures:**
- `fixtures/workflows/user_registration/01_ontology.ttl` - User aggregate definition
- `fixtures/workflows/user_registration/02_expected_code.rs` - Expected generated code
- `fixtures/workflows/user_registration/03_tool_request.json` - MCP tool request
- `fixtures/workflows/user_registration/04_expected_response.json` - Expected response

**Usage:**
```rust
use crate::harness::user_registration_workflow::run_user_registration_workflow;

#[tokio::test]
async fn test_complete_user_registration() {
    let result = run_user_registration_workflow().await.unwrap();
    assert!(result.success);
    assert_eq!(result.steps_executed, 6);
}
```

**Events Emitted:**
- `ontology_loaded` - Ontology loaded from fixture
- `code_generated` - User aggregate code generated
- `code_compiled` - Code compilation succeeded
- `tool_registered` - create_user tool registered
- `user_created` - User successfully created
- `persistence_verified` - User persisted to storage

### 2. Order Processing Workflow

Complex multi-step business workflow with state transitions.

**Steps:**
1. Load order ontology
2. Generate order management tools
3. Create new order
4. Add items to cart (multiple)
5. Calculate order total with tax
6. Validate payment method
7. Process payment
8. Finalize and place order

**Fixtures:**
- `fixtures/workflows/order_processing/01_ontology.ttl` - Order aggregate definition
- `fixtures/workflows/order_processing/02_expected_code.rs` - Generated order code
- `fixtures/workflows/order_processing/03_tool_requests.json` - Sequence of tool calls

**Usage:**
```rust
use crate::harness::order_processing_workflow::run_order_processing_workflow;

#[tokio::test]
async fn test_complete_order_processing() {
    let result = run_order_processing_workflow().await.unwrap();
    assert!(result.success);
    assert_eq!(result.steps_executed, 8);
}
```

**Events Emitted:**
- `ontology_loaded` - Order ontology loaded
- `tools_generated` - Order tools generated
- `order_created` - New order created
- `item_added` - Item added to cart (multiple)
- `total_calculated` - Order total calculated
- `payment_validated` - Payment method validated
- `payment_processed` - Payment successfully processed
- `order_placed` - Order finalized and placed

**Assertions:**
- Order total calculation: `(2×$29.99 + 1×$49.99 + 3×$19.99) × 1.08 = $183.54`
- Payment status: `completed`
- Order status: `placed`
- All events emitted in correct sequence

### 3. MCP Tool Workflow

MCP tool definition, generation, registration, and invocation.

**Steps:**
1. Define tool in ontology
2. Generate tool handler code
3. Compile handler
4. Register tool with MCP server
5. Invoke tool via JSON-RPC 2.0
6. Validate response structure
7. Verify audit log completeness

**Fixtures:**
- `fixtures/workflows/mcp_tool/01_ontology.ttl` - Tool definition ontology
- `fixtures/workflows/mcp_tool/02_expected_handler.rs` - Expected handler code
- `fixtures/workflows/mcp_tool/03_tool_request.json` - JSON-RPC request
- `fixtures/workflows/mcp_tool/04_expected_response.json` - Expected JSON-RPC response

**Usage:**
```rust
use crate::harness::mcp_tool_workflow::run_mcp_tool_workflow;

#[tokio::test]
async fn test_complete_mcp_tool_lifecycle() {
    let result = run_mcp_tool_workflow().await.unwrap();
    assert!(result.success);
    assert_eq!(result.steps_executed, 7);
}
```

**Events Emitted:**
- `tool_defined` - Tool defined in ontology
- `handler_generated` - Handler code generated
- `handler_compiled` - Handler compilation succeeded
- `tool_registered` - Tool registered with MCP
- `tool_invoked` - Tool invoked via protocol
- `response_validated` - Response validated
- `audit_verified` - Audit log verified

## Integration Assertions

### State Assertions

#### `assert_step_state(context, step_name, expected_state)`

Verifies a step resulted in the expected state.

```rust
assert_step_state(ctx.clone(), "create_order", "order_created").await?;
```

#### `assert_state_consistent(context)`

Verifies state transition history is consistent:
- Each transition's `to_state` matches next transition's `from_state`
- Current state matches last transition's `to_state`

```rust
assert_state_consistent(ctx.clone()).await?;
```

### Event Assertions

#### `assert_event_sequence(events, expected_sequence)`

Verifies events were emitted in the expected order.

```rust
let expected = vec!["order_created", "item_added", "order_placed"];
assert_event_sequence(&events, &expected).await?;
```

### Audit Assertions

#### `assert_audit_trail_complete(audit_log)`

Verifies audit log:
- Not empty
- All entries have valid timestamps
- Entries in chronological order

```rust
let audit_log = harness.audit_log().await;
assert_audit_trail_complete(&audit_log).await?;
```

### Workflow Assertions

#### `assert_workflow_succeeds(result)`

Verifies workflow completed successfully.

```rust
let result = workflow.run().await?;
assert_workflow_succeeds(&result).await?;
```

## Real MCP Protocol Testing

### McpProtocolTester

Provides JSON-RPC 2.0 communication with MCP servers.

```rust
let tester = McpProtocolTester::new("http://localhost:8080");

// Invoke tool
let response = tester.invoke_tool("create_user", json!({
    "username": "john_doe",
    "email": "john@example.com"
})).await?;

// Access resource
let resource = tester.access_resource("user://john_doe").await?;

// Send progress notification
tester.send_progress("token_123", 50, 100).await?;

// Send error response
let error = tester.send_error(-32600, "Invalid Request", None).await?;
```

### JSON-RPC 2.0 Validation

All responses are validated for:
- Correct `jsonrpc: "2.0"` field
- Valid `id` matching request
- Either `result` or `error` field (not both)
- Proper error structure with `code`, `message`, `data`

## Docker Integration

### Enabling Docker

```rust
WorkflowBuilder::new("workflow")?
    .with_docker("test-network")
    .step("setup_container", setup_docker)
    .run()
    .await?
```

### Container Lifecycle

1. **Start**: Container started when workflow begins
2. **Execute**: Steps run against container
3. **Cleanup**: Container stopped and removed on workflow completion (success or failure)

### Volume Mounts

```rust
let workspace = harness.workspace_path();
// Workspace automatically mounted to container
// Generated code written to workspace is accessible in container
```

### Network Communication

```rust
// Test services communicate via Docker network
let endpoint = format!("http://mcp-server:8080");
let tester = McpProtocolTester::new(endpoint);
```

## Error Handling

### Happy Path

All steps complete successfully:
- State transitions correctly
- Events emitted in order
- Audit trail complete
- Final assertions pass

```rust
let result = workflow.run().await?;
assert!(result.success);
```

### Error Cases

#### Step Failure

If a step fails:
1. Workflow stops immediately
2. `step_failed` event emitted
3. Docker containers cleaned up
4. Error returned with context

```rust
match workflow.run().await {
    Ok(result) => assert!(result.success),
    Err(e) => {
        // Step failed with clear error message
        assert!(e.to_string().contains("Step 'load_ontology' failed"));
    }
}
```

#### Assertion Failure

If assertions fail after successful steps:
1. All steps executed
2. Assertions collected
3. Failed assertions reported
4. Workflow marked as failed

```rust
Err: Workflow 'user_registration' completed but 2 assertion(s) failed:
  - user_created: User ID not found in context
  - events_emitted: Required event 'persistence_verified' not found
```

#### Partial Rollback

For workflows supporting rollback:

```rust
.step("create_order", create_order)
.step("add_items", add_items)
.on_error("rollback", |ctx, harness| async move {
    // Rollback order creation
    delete_order(ctx.clone()).await?;
    Ok(())
})
```

### Concurrent Workflows

#### Running Multiple Workflows

```rust
use tokio::join;

let (user_result, order_result) = join!(
    run_user_registration_workflow(),
    run_order_processing_workflow()
);

assert!(user_result?.success);
assert!(order_result?.success);
```

#### Race Condition Testing

```rust
// Test concurrent user creation
let handles: Vec<_> = (0..10)
    .map(|i| {
        tokio::spawn(async move {
            let mut workflow = WorkflowBuilder::new(format!("user_{}", i))?;
            workflow.step("create", create_unique_user).run().await
        })
    })
    .collect();

for handle in handles {
    assert!(handle.await??.success);
}
```

#### Lock Verification

```rust
.assert("no_deadlocks", |ctx, harness| async move {
    // Verify no locks held after workflow
    let locks = get_active_locks(ctx.clone()).await?;
    assert_eq!(locks.len(), 0, "Locks still held: {:?}", locks);
    Ok(())
})
```

## Performance Considerations

### Workflow Duration

Track workflow execution time:

```rust
let result = workflow.run().await?;
println!("Workflow completed in {} ms", result.duration_ms);

// Assert performance SLA
assert!(result.duration_ms < 5000, "Workflow too slow: {} ms", result.duration_ms);
```

### Event Volume

Monitor event emission:

```rust
let events = harness.events().await;
assert!(events.len() < 100, "Too many events emitted: {}", events.len());
```

### Memory Usage

```rust
// Use workspace for large files, not in-memory storage
let large_file = workspace.join("large_data.bin");
tokio::fs::write(&large_file, data).await?;
store_data(ctx.clone(), "large_file_path", json!(large_file)).await;
```

## Testing Patterns

### Setup-Execute-Verify

```rust
#[tokio::test]
async fn test_pattern() {
    // Setup
    let workflow = WorkflowBuilder::new("test")?
        .step("setup", setup_data);

    // Execute
    let result = workflow
        .step("execute", execute_action)
        .run()
        .await?;

    // Verify
    assert!(result.success);
    assert_audit_trail_complete(&result.audit_log).await?;
}
```

### Given-When-Then

```rust
#[tokio::test]
async fn test_gherkin_style() {
    // Given: User ontology loaded
    let workflow = WorkflowBuilder::new("test")?
        .step("given_ontology", load_ontology);

    // When: User is created
    let result = workflow
        .step("when_create_user", create_user)
        // Then: User is persisted
        .assert("then_user_persisted", verify_persistence)
        .run()
        .await?;

    assert!(result.success);
}
```

### Parameterized Tests

```rust
#[tokio::test]
async fn test_multiple_orders() {
    for item_count in [1, 5, 10, 50] {
        let result = WorkflowBuilder::new(format!("order_{}", item_count))?
            .step("create", |ctx, h| create_order(ctx, h, item_count))
            .assert("correct_count", move |ctx, h| verify_items(ctx, h, item_count))
            .run()
            .await?;

        assert!(result.success);
    }
}
```

## Debugging

### Event Inspection

```rust
let events = harness.events().await;
for event in events {
    eprintln!("[{}] {}: {:?}",
        event.timestamp,
        event.event_type,
        event.payload
    );
}
```

### Audit Log Review

```rust
let audit = harness.audit_log().await;
for entry in audit {
    eprintln!("[{}] {} by {}: {:?}",
        entry.timestamp,
        entry.action,
        entry.actor,
        entry.details
    );
}
```

### State Transitions

```rust
let ctx = context.read().await;
for transition in &ctx.state_history {
    eprintln!("{} -> {} (triggered by: {})",
        transition.from_state,
        transition.to_state,
        transition.trigger
    );
}
```

### Workspace Contents

```rust
let workspace = harness.workspace_path();
for entry in std::fs::read_dir(workspace)? {
    let path = entry?.path();
    eprintln!("Generated: {:?}", path);
}
```

## Best Practices

### 1. Test Behavior, Not Implementation

❌ **Bad**: Testing internal implementation details
```rust
.assert("uses_hashmap", |ctx, h| async move {
    let ctx = ctx.read().await;
    assert!(ctx.data.contains_key("internal_cache"));
    Ok(())
})
```

✅ **Good**: Testing observable behavior
```rust
.assert("user_created", |ctx, h| async move {
    let user = get_data(ctx.clone(), "created_user").await;
    assert!(user.is_some());
    Ok(())
})
```

### 2. Use Fixtures for Complex Data

❌ **Bad**: Hardcoding large ontologies in tests
```rust
let ontology = r#"@prefix : <http://...> ... (1000 lines) ..."#;
```

✅ **Good**: Loading from fixture files
```rust
let ontology = load_ontology_fixture(Path::new(
    "fixtures/workflows/user_registration/01_ontology.ttl"
)).await?;
```

### 3. Clear Step Names

❌ **Bad**: Vague step names
```rust
.step("do_stuff", do_stuff)
.step("check", check)
```

✅ **Good**: Descriptive step names
```rust
.step("load_user_ontology", load_user_ontology)
.step("verify_user_persisted", verify_user_persisted)
```

### 4. Comprehensive Assertions

❌ **Bad**: Single assertion
```rust
.assert("works", |ctx, h| async move {
    assert!(get_data(ctx.clone(), "result").await.is_some());
    Ok(())
})
```

✅ **Good**: Multiple specific assertions
```rust
.assert("order_created", assert_order_created)
.assert("items_added", assert_items_added)
.assert("total_calculated", assert_total_calculated)
.assert("payment_processed", assert_payment_processed)
```

### 5. Clean Error Messages

❌ **Bad**: Generic error
```rust
return Err(anyhow!("failed"));
```

✅ **Good**: Contextual error
```rust
return Err(anyhow!(
    "Failed to process payment: invalid card number '{}'. Expected 16 digits, got {}",
    card_number,
    card_number.len()
));
```

## Examples

### Complete User Registration Test

```rust
#[tokio::test]
async fn test_user_registration_e2e() {
    let result = WorkflowBuilder::new("user_registration")?
        .step("load_ontology", |ctx, harness| async move {
            let ontology = load_ontology_fixture(Path::new(
                "fixtures/workflows/user_registration/01_ontology.ttl"
            )).await?;

            let mut c = ctx.write().await;
            c.ontology = Some(ontology);
            Ok(())
        })
        .step("generate_code", |ctx, harness| async move {
            let ontology = ctx.read().await.ontology.clone().unwrap();
            let code = generate_user_code(&ontology)?;
            save_generated_code(ctx.clone(), "user", code).await;
            Ok(())
        })
        .step("create_user", |ctx, harness| async move {
            let user = User::new("john_doe".into(), "john@example.com".into());
            store_data(ctx.clone(), "user", serde_json::to_value(user)?).await;
            harness.emit_event("user_created", json!({}), "create_user").await;
            Ok(())
        })
        .assert("user_exists", |ctx, harness| async move {
            let user = get_data(ctx.clone(), "user").await;
            assert!(user.is_some());
            Ok(())
        })
        .assert("event_emitted", |ctx, harness| async move {
            let events = harness.events().await;
            assert!(events.iter().any(|e| e.event_type == "user_created"));
            Ok(())
        })
        .run()
        .await?;

    assert!(result.success);
    assert_eq!(result.steps_executed, 3);
}
```

## Troubleshooting

### Common Issues

#### 1. Docker Container Not Cleaned Up

**Symptom**: Containers still running after test failure

**Solution**: Ensure cleanup in test
```rust
#[tokio::test]
async fn test_with_docker() {
    let mut harness = IntegrationWorkflowHarness::new()?
        .with_docker("test-net");

    let result = /* run workflow */;

    // Explicit cleanup
    harness.cleanup_docker().await?;

    result
}
```

#### 2. State Not Persisting

**Symptom**: Data stored in context not accessible later

**Solution**: Use Arc<RwLock<>> correctly
```rust
// Clone the Arc, not the data
let ctx_clone = ctx.clone();
store_data(ctx_clone, "key", value).await;
```

#### 3. Events Out of Order

**Symptom**: Event sequence assertion failing

**Solution**: Ensure events emitted in correct step order
```rust
// Events emitted in step execution order
.step("step1", |ctx, h| async move {
    h.emit_event("event1", json!({}), "step1").await;
    Ok(())
})
.step("step2", |ctx, h| async move {
    h.emit_event("event2", json!({}), "step2").await;
    Ok(())
})
```

## Future Enhancements

### Planned Features

1. **Snapshot Testing Integration**
   - Capture workflow state snapshots
   - Compare against golden files
   - Update mode for accepting changes

2. **Performance Benchmarking**
   - Built-in performance assertions
   - Comparison against baselines
   - Regression detection

3. **Parallel Step Execution**
   - Execute independent steps concurrently
   - Dependency graph resolution
   - Optimized workflow execution

4. **Cloud Integration**
   - Test against cloud services
   - Managed Docker orchestration
   - Distributed workflow testing

5. **Visual Workflow Reports**
   - HTML reports with step visualization
   - Event timeline graphs
   - State transition diagrams

## References

- [Chicago vs London TDD](https://martinfowler.com/articles/mocksArentStubs.html)
- [MCP Protocol Specification](https://modelcontextprotocol.io/)
- [Rust Testing Best Practices](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Docker Test Containers](https://www.testcontainers.org/)

## Contributing

When adding new workflows:

1. Create workflow module in `tests/harness/`
2. Add fixtures in `fixtures/workflows/<workflow_name>/`
3. Document workflow in this file
4. Add usage examples
5. Include comprehensive tests

## License

Same as parent project (Apache-2.0)
