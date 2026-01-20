//! Order Processing Workflow Tests
//!
//! Chicago-style TDD integration tests for the complete order processing workflow.
//!
//! # Workflow Steps
//! 1. Create order aggregate
//! 2. Generate order tools
//! 3. Create order
//! 4. Add items to cart
//! 5. Calculate total
//! 6. Process payment
//! 7. Emit order_placed event

use super::*;
use anyhow::{Context, Result};
use serde_json::json;
use std::path::Path;

/// Run the complete order processing workflow
///
/// This is a Chicago-style integration test that:
/// - Creates order aggregates with real state
/// - Generates and executes order management tools
/// - Processes payments with validation
/// - Emits business events
/// - Maintains audit trail
pub async fn run_order_processing_workflow() -> Result<WorkflowResult> {
    WorkflowBuilder::new("order_processing")?
        .step("load_order_ontology", load_order_ontology)
        .step("generate_order_tools", generate_order_tools)
        .step("create_order", create_order)
        .step("add_items_to_cart", add_items_to_cart)
        .step("calculate_order_total", calculate_order_total)
        .step("validate_payment", validate_payment)
        .step("process_payment", process_payment)
        .step("finalize_order", finalize_order)
        .assert("order_created", assert_order_created)
        .assert("items_added", assert_items_added)
        .assert("total_calculated", assert_total_calculated)
        .assert("payment_processed", assert_payment_processed)
        .assert("order_placed_event", assert_order_placed_event)
        .run()
        .await
}

/// Step 1: Load order ontology from TTL fixture
async fn load_order_ontology(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/workflows/order_processing/01_ontology.ttl");

    let ontology = if fixture_path.exists() {
        load_ontology_fixture(&fixture_path).await?
    } else {
        // Use embedded default ontology for testing
        include_str!("../../../fixtures/workflows/order_processing/01_ontology.ttl").to_string()
    };

    {
        let mut ctx = context.write().await;
        ctx.ontology = Some(ontology.clone());
    }

    harness.emit_event(
        "ontology_loaded",
        json!({ "type": "order", "size_bytes": ontology.len() }),
        "load_order_ontology"
    ).await;

    transition_state(context.clone(), "ontology_loaded", "load_order_ontology").await;

    Ok(())
}

/// Step 2: Generate order management tools
async fn generate_order_tools(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let ontology = {
        let ctx = context.read().await;
        ctx.ontology.clone()
            .ok_or_else(|| anyhow::anyhow!("Ontology not loaded"))?
    };

    // Generate code for order aggregate and tools
    let order_code = generate_order_aggregate_code(&ontology)?;
    let tools_code = generate_order_tools_code(&ontology)?;

    save_generated_code(context.clone(), "order_aggregate", order_code).await;
    save_generated_code(context.clone(), "order_tools", tools_code).await;

    // Register tools
    register_order_tools(context.clone()).await;

    harness.emit_event(
        "tools_generated",
        json!({
            "artifacts": ["order_aggregate", "order_tools"],
            "tools": ["create_order", "add_item", "calculate_total", "process_payment"]
        }),
        "generate_order_tools"
    ).await;

    transition_state(context.clone(), "tools_generated", "generate_order_tools").await;

    Ok(())
}

/// Step 3: Create a new order
async fn create_order(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let order_data = json!({
        "customer_id": "cust_456",
        "currency": "USD"
    });

    let order_id = "order_789";
    store_data(context.clone(), "order_id", json!(order_id)).await;
    store_data(context.clone(), "order_data", order_data.clone()).await;
    store_data(context.clone(), "items", json!([])).await;

    harness.emit_event(
        "order_created",
        json!({
            "order_id": order_id,
            "customer_id": "cust_456"
        }),
        "create_order"
    ).await;

    harness.audit(
        "order_created",
        "system",
        json!({ "order_id": order_id })
    ).await;

    transition_state(context.clone(), "order_created", "create_order").await;

    Ok(())
}

/// Step 4: Add items to the cart
async fn add_items_to_cart(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let items = vec![
        json!({
            "product_id": "prod_1",
            "name": "Widget A",
            "quantity": 2,
            "unit_price": 29.99
        }),
        json!({
            "product_id": "prod_2",
            "name": "Widget B",
            "quantity": 1,
            "unit_price": 49.99
        }),
        json!({
            "product_id": "prod_3",
            "name": "Widget C",
            "quantity": 3,
            "unit_price": 19.99
        }),
    ];

    store_data(context.clone(), "items", json!(items.clone())).await;

    for (index, item) in items.iter().enumerate() {
        harness.emit_event(
            "item_added",
            json!({
                "item_index": index,
                "product_id": item["product_id"],
                "quantity": item["quantity"]
            }),
            "add_items_to_cart"
        ).await;

        harness.audit(
            "item_added_to_order",
            "system",
            item.clone()
        ).await;
    }

    transition_state(context.clone(), "items_added", "add_items_to_cart").await;

    Ok(())
}

/// Step 5: Calculate order total
async fn calculate_order_total(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let items = get_data(context.clone(), "items").await
        .ok_or_else(|| anyhow::anyhow!("Items not found"))?;

    let items_array = items.as_array()
        .ok_or_else(|| anyhow::anyhow!("Items is not an array"))?;

    let mut subtotal = 0.0;
    for item in items_array {
        let quantity = item["quantity"].as_f64().unwrap_or(0.0);
        let unit_price = item["unit_price"].as_f64().unwrap_or(0.0);
        subtotal += quantity * unit_price;
    }

    let tax_rate = 0.08; // 8% tax
    let tax = subtotal * tax_rate;
    let total = subtotal + tax;

    store_data(context.clone(), "subtotal", json!(subtotal)).await;
    store_data(context.clone(), "tax", json!(tax)).await;
    store_data(context.clone(), "total", json!(total)).await;

    harness.emit_event(
        "total_calculated",
        json!({
            "subtotal": subtotal,
            "tax": tax,
            "total": total
        }),
        "calculate_order_total"
    ).await;

    harness.audit(
        "order_total_calculated",
        "system",
        json!({ "total": total })
    ).await;

    transition_state(context.clone(), "total_calculated", "calculate_order_total").await;

    Ok(())
}

/// Step 6: Validate payment information
async fn validate_payment(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let payment_method = json!({
        "type": "credit_card",
        "last_four": "4242",
        "exp_month": 12,
        "exp_year": 2025
    });

    // Validate payment method
    if payment_method["type"].as_str() != Some("credit_card") {
        return Err(anyhow::anyhow!("Invalid payment method type"));
    }

    store_data(context.clone(), "payment_method", payment_method.clone()).await;

    harness.emit_event(
        "payment_validated",
        payment_method,
        "validate_payment"
    ).await;

    transition_state(context.clone(), "payment_validated", "validate_payment").await;

    Ok(())
}

/// Step 7: Process payment
async fn process_payment(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let total = get_data(context.clone(), "total").await
        .and_then(|v| v.as_f64())
        .ok_or_else(|| anyhow::anyhow!("Total not found"))?;

    let payment_method = get_data(context.clone(), "payment_method").await
        .ok_or_else(|| anyhow::anyhow!("Payment method not found"))?;

    // Simulate payment processing
    let transaction_id = "txn_abc123";
    store_data(context.clone(), "transaction_id", json!(transaction_id)).await;
    store_data(context.clone(), "payment_status", json!("completed")).await;

    harness.emit_event(
        "payment_processed",
        json!({
            "transaction_id": transaction_id,
            "amount": total,
            "payment_method": payment_method
        }),
        "process_payment"
    ).await;

    harness.audit(
        "payment_processed",
        "payment_gateway",
        json!({
            "transaction_id": transaction_id,
            "amount": total
        })
    ).await;

    transition_state(context.clone(), "payment_processed", "process_payment").await;

    Ok(())
}

/// Step 8: Finalize order
async fn finalize_order(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let order_id = get_data(context.clone(), "order_id").await
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .ok_or_else(|| anyhow::anyhow!("Order ID not found"))?;

    store_data(context.clone(), "order_status", json!("placed")).await;

    harness.emit_event(
        "order_placed",
        json!({
            "order_id": order_id,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }),
        "finalize_order"
    ).await;

    harness.audit(
        "order_placed",
        "system",
        json!({ "order_id": order_id })
    ).await;

    transition_state(context.clone(), "order_placed", "finalize_order").await;

    Ok(())
}

// =============================================================================
// Assertions
// =============================================================================

async fn assert_order_created(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    _harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let order_id = get_data(context.clone(), "order_id").await
        .ok_or_else(|| anyhow::anyhow!("Order ID not found"))?;

    if !order_id.is_string() {
        return Err(anyhow::anyhow!("Order ID is not a string"));
    }

    Ok(())
}

async fn assert_items_added(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    _harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let items = get_data(context.clone(), "items").await
        .ok_or_else(|| anyhow::anyhow!("Items not found"))?;

    let items_array = items.as_array()
        .ok_or_else(|| anyhow::anyhow!("Items is not an array"))?;

    if items_array.is_empty() {
        return Err(anyhow::anyhow!("No items in cart"));
    }

    if items_array.len() != 3 {
        return Err(anyhow::anyhow!("Expected 3 items, got {}", items_array.len()));
    }

    Ok(())
}

async fn assert_total_calculated(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    _harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let total = get_data(context.clone(), "total").await
        .and_then(|v| v.as_f64())
        .ok_or_else(|| anyhow::anyhow!("Total not found"))?;

    // Expected: (2*29.99 + 1*49.99 + 3*19.99) * 1.08 = 169.94 * 1.08 = 183.5352
    let expected_total = 183.5352;
    if (total - expected_total).abs() > 0.01 {
        return Err(anyhow::anyhow!(
            "Total mismatch: expected {}, got {}",
            expected_total,
            total
        ));
    }

    Ok(())
}

async fn assert_payment_processed(
    context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    _harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let payment_status = get_data(context.clone(), "payment_status").await
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .ok_or_else(|| anyhow::anyhow!("Payment status not found"))?;

    if payment_status != "completed" {
        return Err(anyhow::anyhow!(
            "Payment status is '{}', expected 'completed'",
            payment_status
        ));
    }

    let transaction_id = get_data(context.clone(), "transaction_id").await
        .ok_or_else(|| anyhow::anyhow!("Transaction ID not found"))?;

    if !transaction_id.is_string() {
        return Err(anyhow::anyhow!("Transaction ID is not a string"));
    }

    Ok(())
}

async fn assert_order_placed_event(
    _context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>,
    harness: &IntegrationWorkflowHarness,
) -> Result<()> {
    let events = harness.events().await;

    let order_placed_event = events.iter()
        .find(|e| e.event_type == "order_placed")
        .ok_or_else(|| anyhow::anyhow!("order_placed event not found"))?;

    if order_placed_event.payload["order_id"].as_str().is_none() {
        return Err(anyhow::anyhow!("order_placed event missing order_id"));
    }

    Ok(())
}

// =============================================================================
// Code Generation (Mock Implementation)
// =============================================================================

fn generate_order_aggregate_code(_ontology: &str) -> Result<String> {
    Ok(r#"
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub customer_id: String,
    pub items: Vec<OrderItem>,
    pub subtotal: f64,
    pub tax: f64,
    pub total: f64,
    pub status: OrderStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub product_id: String,
    pub name: String,
    pub quantity: u32,
    pub unit_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderStatus {
    Created,
    Processing,
    Paid,
    Placed,
    Shipped,
    Delivered,
}
"#.to_string())
}

fn generate_order_tools_code(_ontology: &str) -> Result<String> {
    Ok(r#"
pub fn create_order(customer_id: String) -> Result<Order, String> {
    Ok(Order {
        id: uuid::Uuid::new_v4().to_string(),
        customer_id,
        items: Vec::new(),
        subtotal: 0.0,
        tax: 0.0,
        total: 0.0,
        status: OrderStatus::Created,
        created_at: chrono::Utc::now(),
    })
}

pub fn add_item(order: &mut Order, item: OrderItem) {
    order.items.push(item);
}

pub fn calculate_total(order: &mut Order, tax_rate: f64) {
    order.subtotal = order.items.iter()
        .map(|item| item.quantity as f64 * item.unit_price)
        .sum();
    order.tax = order.subtotal * tax_rate;
    order.total = order.subtotal + order.tax;
}
"#.to_string())
}

async fn register_order_tools(context: std::sync::Arc<tokio::sync::RwLock<WorkflowContext>>) {
    let tools = vec![
        ToolRegistration {
            name: "create_order".to_string(),
            description: "Create a new order".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "customer_id": { "type": "string" }
                },
                "required": ["customer_id"]
            }),
            handler: "create_order".to_string(),
        },
        ToolRegistration {
            name: "add_item".to_string(),
            description: "Add item to order".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "product_id": { "type": "string" },
                    "quantity": { "type": "integer" },
                    "unit_price": { "type": "number" }
                },
                "required": ["product_id", "quantity", "unit_price"]
            }),
            handler: "add_item".to_string(),
        },
    ];

    for tool in tools {
        register_tool(context.clone(), tool).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_order_processing_workflow_complete() {
        let result = run_order_processing_workflow().await.unwrap();
        assert!(result.success);
        assert_eq!(result.steps_executed, 8);
    }

    #[tokio::test]
    async fn test_calculate_order_total() {
        let harness = IntegrationWorkflowHarness::new().unwrap();
        let context = harness.context.clone();

        // Setup items
        let items = vec![
            json!({ "quantity": 2.0, "unit_price": 29.99 }),
            json!({ "quantity": 1.0, "unit_price": 49.99 }),
            json!({ "quantity": 3.0, "unit_price": 19.99 }),
        ];
        store_data(context.clone(), "items", json!(items)).await;

        calculate_order_total(context.clone(), &harness).await.unwrap();

        let total = get_data(context.clone(), "total").await
            .and_then(|v| v.as_f64())
            .unwrap();

        assert!((total - 183.5352).abs() < 0.01);
    }
}
