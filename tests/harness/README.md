# Integration Workflow Test Harness

Chicago-style TDD test harness for complete integration workflows.

## Quick Start

```rust
use crate::harness::*;

#[tokio::test]
async fn test_my_workflow() {
    let result = WorkflowBuilder::new("my_workflow")?
        .step("load_data", |ctx, harness| async move {
            // Load ontology
            let ontology = load_ontology_fixture(path).await?;
            let mut c = ctx.write().await;
            c.ontology = Some(ontology);
            Ok(())
        })
        .step("process", |ctx, harness| async move {
            // Process data
            store_data(ctx.clone(), "result", json!(42)).await;
            harness.emit_event("processed", json!({}), "process").await;
            Ok(())
        })
        .assert("result_correct", |ctx, harness| async move {
            let result = get_data(ctx.clone(), "result").await;
            assert_eq!(result, Some(json!(42)));
            Ok(())
        })
        .run()
        .await?;

    assert_workflow_succeeds(&result).await?;
}
```

## Available Workflows

### 1. User Registration
Complete user registration workflow

### 2. Order Processing
Complex multi-step business workflow

### 3. MCP Tool Lifecycle
Tool definition, generation, and invocation

## Documentation

Full documentation: `docs/TDD_INTEGRATION_WORKFLOW_HARNESS.md`

## License

Same as parent project (Apache-2.0)
