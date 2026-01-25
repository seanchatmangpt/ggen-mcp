//! Fixture Library Usage Examples
//!
//! This test file demonstrates comprehensive usage of the fixture library
//! following Chicago-style TDD principles.

mod harness;

use anyhow::Result;
use harness::{
    AAAPattern, AggregateBuilder, ConfigBuilder, FixtureComposer, Fixtures, OntologyBuilder,
    TemplateContextBuilder, TestWorkspace,
};

// =============================================================================
// Domain Fixture Examples
// =============================================================================

#[test]
fn example_user_fixtures() {
    // Use pre-configured minimal user
    let user = Fixtures::user().minimal();
    assert_eq!(user.name, "User");
    assert_eq!(user.fields.len(), 3); // id, name, email
    assert_eq!(user.commands.len(), 1); // CreateUser
    assert_eq!(user.events.len(), 1); // UserCreated
    assert!(user.metadata.valid);

    // Use pre-configured complete user
    let user = Fixtures::user().complete();
    assert_eq!(user.fields.len(), 7); // All fields
    assert_eq!(user.commands.len(), 4); // All commands
    assert_eq!(user.events.len(), 4); // All events
    assert_eq!(user.invariants.len(), 2); // All invariants

    // Use invalid user for error testing
    let user = Fixtures::user().invalid();
    assert!(!user.metadata.valid);
    assert_eq!(user.fields.len(), 0);
}

#[test]
fn example_order_fixtures() {
    // Empty order
    let order = Fixtures::order().empty();
    assert_eq!(order.fields.len(), 4); // id, user_id, status, total
    assert!(!order.fields.iter().any(|f| f.name == "items"));

    // Order with 3 items
    let order = Fixtures::order().with_items(3);
    assert!(order.fields.iter().any(|f| f.name == "items"));
    assert_eq!(order.invariants.len(), 3); // One per item

    // Cancelled order
    let order = Fixtures::order().cancelled();
    assert!(order.fields.iter().any(|f| f.name == "cancelled_at"));
    assert!(order.fields.iter().any(|f| f.name == "cancellation_reason"));
}

#[test]
fn example_product_fixtures() {
    // In stock product
    let product = Fixtures::product().in_stock();
    assert!(
        product
            .invariants
            .iter()
            .any(|i| i.contains("quantity > 0"))
    );

    // Out of stock product
    let product = Fixtures::product().out_of_stock();
    assert!(
        product
            .invariants
            .iter()
            .any(|i| i.contains("quantity == 0"))
    );
}

#[test]
fn example_payment_fixtures() {
    // Test all payment states
    let pending = Fixtures::payment().pending();
    assert!(pending.metadata.tags.contains("pending"));

    let completed = Fixtures::payment().completed();
    assert!(completed.fields.iter().any(|f| f.name == "completed_at"));

    let failed = Fixtures::payment().failed();
    assert!(failed.fields.iter().any(|f| f.name == "failure_reason"));
}

// =============================================================================
// Builder Examples
// =============================================================================

#[test]
fn example_aggregate_builder_basic() {
    let aggregate = AggregateBuilder::new("Product")
        .with_id("prod_001")
        .with_field("name", "String", true)
        .with_field("price", "Money", true)
        .with_field("description", "String", false)
        .build();

    assert_eq!(aggregate.name, "Product");
    assert_eq!(aggregate.id, "prod_001");
    assert_eq!(aggregate.fields.len(), 3);
    assert!(aggregate.metadata.valid);
}

#[test]
fn example_aggregate_builder_complete() {
    let aggregate = AggregateBuilder::new("Customer")
        .with_id("customer_001")
        .with_field("id", "CustomerId", true)
        .with_field("name", "String", true)
        .with_field("email", "Email", true)
        .with_field_desc("loyalty_points", "u32", false, "Customer loyalty points")
        .with_command("CreateCustomer")
        .with_command("UpdateCustomer")
        .with_command("AwardLoyaltyPoints")
        .with_event("CustomerCreated")
        .with_event("CustomerUpdated")
        .with_event("LoyaltyPointsAwarded")
        .with_invariant("email must be unique")
        .with_invariant("loyalty_points >= 0")
        .description("Customer aggregate with loyalty program")
        .tag("loyalty")
        .tag("customer_management")
        .build();

    assert_eq!(aggregate.name, "Customer");
    assert_eq!(aggregate.fields.len(), 4);
    assert_eq!(aggregate.commands.len(), 3);
    assert_eq!(aggregate.events.len(), 3);
    assert_eq!(aggregate.invariants.len(), 2);
    assert!(aggregate.metadata.tags.contains("loyalty"));
}

#[test]
fn example_config_builder() {
    let config = ConfigBuilder::new()
        .workspace_root("/tmp/test-workspace")
        .cache_capacity(15)
        .with_recalc()
        .with_vba()
        .max_concurrent_recalcs(4)
        .tool_timeout_ms(45_000)
        .max_response_bytes(2_000_000)
        .description("Custom test configuration")
        .build();

    assert_eq!(
        config.workspace_root.to_str().unwrap(),
        "/tmp/test-workspace"
    );
    assert_eq!(config.cache_capacity, 15);
    assert!(config.recalc_enabled);
    assert!(config.vba_enabled);
    assert_eq!(config.max_concurrent_recalcs, 4);
    assert_eq!(config.tool_timeout_ms, Some(45_000));
}

#[test]
fn example_ontology_builder() {
    let ontology = OntologyBuilder::new()
        .prefix("shop", "http://shop.example.org/")
        .add_aggregate("Product")
        .add_aggregate("Category")
        .add_value_object("Price")
        .add_command("CreateProduct")
        .add_command("AssignCategory")
        .add_event("ProductCreated")
        .add_event("CategoryAssigned")
        .description("Shopping domain ontology")
        .build();

    assert!(ontology.ttl.contains("@prefix shop:"));
    assert!(ontology.ttl.contains("ex:Product a ddd:Aggregate"));
    assert!(ontology.ttl.contains("ex:Category a ddd:Aggregate"));
    assert!(ontology.ttl.contains("ex:Price a ddd:ValueObject"));
}

#[test]
fn example_template_context_builder() {
    let context = TemplateContextBuilder::new()
        .entity_name("OrderItem")
        .add_field("product_id", "ProductId")
        .add_field("quantity", "u32")
        .add_field("unit_price", "Money")
        .add_import("serde", vec!["Deserialize", "Serialize"])
        .add_import("uuid", vec!["Uuid"])
        .add_custom(
            "derive_traits",
            serde_json::json!(["Debug", "Clone", "PartialEq"]),
        )
        .description("Order item template context")
        .build();

    assert_eq!(context.entity_name, "OrderItem");
    assert_eq!(context.fields.len(), 3);
    assert_eq!(context.imports.len(), 2);
    assert!(context.custom.contains_key("derive_traits"));

    let json = context.to_json();
    assert!(json.is_object());
}

// =============================================================================
// Fixture Composition Examples
// =============================================================================

#[test]
fn example_simple_composition() -> Result<()> {
    let ontology = FixtureComposer::new()
        .add(Fixtures::user().minimal())
        .add(Fixtures::product().in_stock())
        .build_ontology()?;

    let ttl = &ontology.ttl;
    assert!(ttl.contains("User"));
    assert!(ttl.contains("Product"));
    assert!(ttl.contains("CreateUser"));
    assert!(ttl.contains("CreateProduct"));

    Ok(())
}

#[test]
fn example_complex_composition() -> Result<()> {
    // Compose a complete e-commerce domain
    let domain = FixtureComposer::new()
        .add(Fixtures::user().complete())
        .add(Fixtures::order().with_items(3))
        .add(Fixtures::product().in_stock())
        .add(Fixtures::payment().completed())
        .build_ontology()?;

    // Verify all aggregates present
    let ttl = &domain.ttl;
    assert!(ttl.contains("User"));
    assert!(ttl.contains("Order"));
    assert!(ttl.contains("Product"));
    assert!(ttl.contains("Payment"));

    // Should have commands from all aggregates
    assert!(ttl.contains("CreateUser"));
    assert!(ttl.contains("CreateOrder"));
    assert!(ttl.contains("AddOrderItem"));
    assert!(ttl.contains("CreateProduct"));
    assert!(ttl.contains("CompletePayment"));

    // Load into store
    let store = domain.store()?;
    assert!(store.len()? > 0);

    Ok(())
}

#[test]
fn example_custom_composition() -> Result<()> {
    // Build custom aggregates and compose them
    let inventory = AggregateBuilder::new("Inventory")
        .with_field("product_id", "ProductId", true)
        .with_field("quantity", "u32", true)
        .with_command("AddStock")
        .with_command("RemoveStock")
        .with_event("StockAdded")
        .with_event("StockRemoved")
        .build();

    let warehouse = AggregateBuilder::new("Warehouse")
        .with_field("id", "WarehouseId", true)
        .with_field("location", "Address", true)
        .with_command("CreateWarehouse")
        .with_event("WarehouseCreated")
        .build();

    let ontology = FixtureComposer::new()
        .add(inventory)
        .add(warehouse)
        .add(Fixtures::product().in_stock())
        .build_ontology()?;

    let ttl = &ontology.ttl;
    assert!(ttl.contains("Inventory"));
    assert!(ttl.contains("Warehouse"));
    assert!(ttl.contains("Product"));

    Ok(())
}

// =============================================================================
// Configuration Examples
// =============================================================================

#[test]
fn example_configuration_fixtures() {
    // Minimal config
    let minimal = Fixtures::config().minimal();
    assert_eq!(minimal.cache_capacity, 5);
    assert!(!minimal.recalc_enabled);

    // Development config
    let dev = Fixtures::config().development();
    assert_eq!(dev.cache_capacity, 3);
    assert!(dev.recalc_enabled);

    // Production config
    let prod = Fixtures::config().production();
    assert_eq!(prod.cache_capacity, 20);
    assert!(prod.recalc_enabled);
    assert_eq!(prod.max_concurrent_recalcs, 8);

    // Invalid configs
    let invalid = Fixtures::config().invalid_cache_too_small();
    assert!(!invalid.metadata.valid);
    assert_eq!(invalid.cache_capacity, 0);
}

// =============================================================================
// Ontology Examples
// =============================================================================

#[test]
fn example_ontology_fixtures() -> Result<()> {
    // Single aggregate
    let single = Fixtures::ontology().single_aggregate();
    assert!(single.ttl.contains("User"));
    assert!(single.ttl.contains("CreateUser"));

    // Complete domain
    let complete = Fixtures::ontology().complete_domain();
    assert!(complete.ttl.contains("User"));
    assert!(complete.ttl.contains("Order"));
    assert!(complete.ttl.contains("Product"));
    assert!(complete.ttl.contains("Payment"));

    // MCP tools
    let mcp = Fixtures::ontology().mcp_tools();
    assert!(mcp.prefixes.contains_key("mcp"));
    assert!(mcp.ttl.contains("Tool"));
    assert!(mcp.ttl.contains("Resource"));

    // DDD patterns
    let ddd = Fixtures::ontology().ddd_patterns();
    assert!(ddd.ttl.contains("Aggregate"));
    assert!(ddd.ttl.contains("ValueObject"));
    assert!(ddd.ttl.contains("Entity"));

    Ok(())
}

#[test]
fn example_ontology_store() -> Result<()> {
    let ontology = Fixtures::ontology().complete_domain();
    let store = ontology.store()?;

    // Store should contain triples
    assert!(store.len()? > 0);

    // Can query the store
    // (Add SPARQL queries here if needed)

    Ok(())
}

// =============================================================================
// Test Workspace Examples
// =============================================================================

#[test]
fn example_test_workspace() -> Result<()> {
    let workspace = TestWorkspace::new()?;

    // Get root path
    let root = workspace.root();
    assert!(root.exists());

    // Create file
    let file = workspace.create_file("test.txt", "Hello, world!")?;
    assert!(file.exists());
    let content = std::fs::read_to_string(&file)?;
    assert_eq!(content, "Hello, world!");

    // Create nested file
    let nested = workspace.create_file("subdir/nested.txt", "Nested content")?;
    assert!(nested.exists());

    // Get path
    let path = workspace.path("another_file.txt");
    assert!(path.parent().unwrap().exists());

    Ok(())
} // Workspace automatically cleaned up here

#[test]
fn example_workspace_with_ontology() -> Result<()> {
    let workspace = TestWorkspace::new()?;

    // Create ontology
    let ontology = Fixtures::ontology().complete_domain();

    // Save to workspace
    let ttl_path = workspace.create_file("domain.ttl", &ontology.ttl)?;
    assert!(ttl_path.exists());

    // Read back
    let content = std::fs::read_to_string(&ttl_path)?;
    assert!(content.contains("@prefix ddd:"));

    Ok(())
}

// =============================================================================
// AAA Pattern Examples
// =============================================================================

#[test]
fn example_aaa_pattern_basic() {
    AAAPattern::new()
        // Arrange
        .arrange(Fixtures::user().minimal())
        // Act
        .act(|user| {
            // Convert to TTL
            user.to_ttl()
        })
        // Assert
        .assert(|result| {
            let ttl = result.expect("Should produce TTL");
            assert!(ttl.contains("@prefix ddd:"));
            assert!(ttl.contains("ex:User a ddd:Aggregate"));
        });
}

#[test]
fn example_aaa_pattern_transformation() {
    AAAPattern::new()
        // Arrange: Start with minimal user
        .arrange(Fixtures::user().minimal())
        // Act: Transform to template context
        .act(|user| user.to_context())
        // Assert: Verify context
        .assert(|result| {
            let context = result.expect("Should produce context");
            assert_eq!(context.entity_name, "User");
            assert!(context.fields.contains_key("name"));
            assert!(context.fields.contains_key("email"));
        });
}

#[test]
fn example_aaa_pattern_complex() {
    AAAPattern::new()
        // Arrange: Create order with items
        .arrange(Fixtures::order().with_items(5))
        // Act: Count invariants
        .act(|order| order.invariants.len())
        // Assert: Should have one per item
        .assert(|result| {
            let count = result.expect("Should have count");
            assert_eq!(count, 5);
        });
}

// =============================================================================
// Advanced Examples
// =============================================================================

#[test]
fn example_fixture_conversion() {
    // Start with aggregate
    let user = Fixtures::user().complete();

    // Convert to TTL
    let ttl = user.to_ttl();
    assert!(ttl.contains("ddd:Aggregate"));

    // Convert to template context
    let context = user.to_context();
    assert_eq!(context.entity_name, "User");
    assert!(context.fields.len() > 0);

    // Context can be converted to JSON
    let json = context.to_json();
    assert!(json.is_object());
}

#[test]
fn example_invalid_fixtures() {
    // Test error handling with invalid fixtures

    // Invalid user
    let user = Fixtures::user().invalid();
    assert!(!user.metadata.valid);

    // Invalid config
    let config = Fixtures::config().invalid_cache_too_small();
    assert!(!config.metadata.valid);

    // Invalid ontology
    let ontology = Fixtures::ontology().invalid_missing_type();
    assert!(!ontology.metadata.valid);
}

#[test]
fn example_fixture_metadata() {
    let user = Fixtures::user().minimal();

    // Check metadata
    assert_eq!(user.metadata.name, "User");
    assert!(user.metadata.valid);
    assert!(user.metadata.tags.contains("minimal"));
    assert!(!user.metadata.description.is_empty());

    // Version tracking
    use harness::FixtureVersion;
    assert_eq!(user.metadata.version, FixtureVersion::CURRENT);
}

#[test]
fn example_builder_chaining() {
    // Demonstrate fluent builder chaining
    let aggregate = AggregateBuilder::new("ComplexAggregate")
        .with_id("complex_001")
        .with_field("field1", "Type1", true)
        .with_field("field2", "Type2", false)
        .with_field("field3", "Type3", true)
        .with_command("Command1")
        .with_command("Command2")
        .with_event("Event1")
        .with_event("Event2")
        .with_invariant("invariant1")
        .with_invariant("invariant2")
        .description("A complex aggregate for testing")
        .tag("complex")
        .tag("test")
        .build();

    assert_eq!(aggregate.fields.len(), 3);
    assert_eq!(aggregate.commands.len(), 2);
    assert_eq!(aggregate.events.len(), 2);
    assert_eq!(aggregate.invariants.len(), 2);
    assert_eq!(aggregate.metadata.tags.len(), 2);
}

// =============================================================================
// Real-World Scenario Examples
// =============================================================================

#[test]
fn example_ecommerce_scenario() -> Result<()> {
    // Build a complete e-commerce scenario

    // 1. Create domain aggregates
    let customer = AggregateBuilder::new("Customer")
        .with_field("id", "CustomerId", true)
        .with_field("email", "Email", true)
        .with_field("shipping_address", "Address", false)
        .with_command("RegisterCustomer")
        .with_event("CustomerRegistered")
        .build();

    let cart = AggregateBuilder::new("ShoppingCart")
        .with_field("id", "CartId", true)
        .with_field("customer_id", "CustomerId", true)
        .with_field("items", "Vec<CartItem>", true)
        .with_command("AddToCart")
        .with_command("RemoveFromCart")
        .with_command("Checkout")
        .with_event("ItemAdded")
        .with_event("ItemRemoved")
        .with_event("CheckoutInitiated")
        .build();

    // 2. Compose with existing fixtures
    let domain = FixtureComposer::new()
        .add(customer)
        .add(cart)
        .add(Fixtures::product().in_stock())
        .add(Fixtures::order().with_items(3))
        .add(Fixtures::payment().completed())
        .build_ontology()?;

    // 3. Verify complete domain
    let ttl = &domain.ttl;
    assert!(ttl.contains("Customer"));
    assert!(ttl.contains("ShoppingCart"));
    assert!(ttl.contains("Product"));
    assert!(ttl.contains("Order"));
    assert!(ttl.contains("Payment"));

    Ok(())
}

#[test]
fn example_test_data_pipeline() -> Result<()> {
    // Demonstrate a complete test data pipeline

    let workspace = TestWorkspace::new()?;

    // 1. Create domain ontology
    let ontology = Fixtures::ontology().complete_domain();

    // 2. Save to workspace
    let ontology_path = workspace.create_file("domain.ttl", &ontology.ttl)?;

    // 3. Create template context
    let context = Fixtures::user().complete().to_context();
    let context_json = serde_json::to_string_pretty(&context.to_json())?;

    // 4. Save context
    let context_path = workspace.create_file("user_context.json", &context_json)?;

    // 5. Create config
    let config = Fixtures::config().development();

    // 6. Verify all files exist
    assert!(ontology_path.exists());
    assert!(context_path.exists());

    Ok(())
}

#[test]
fn example_validation_workflow() -> Result<()> {
    // Test validation workflow with valid and invalid fixtures

    // Valid fixtures should pass
    let valid_user = Fixtures::user().minimal();
    assert!(valid_user.metadata.valid);

    let valid_config = Fixtures::config().production();
    assert!(valid_config.metadata.valid);

    let valid_ontology = Fixtures::ontology().complete_domain();
    assert!(valid_ontology.metadata.valid);

    // Invalid fixtures should be marked
    let invalid_user = Fixtures::user().invalid();
    assert!(!invalid_user.metadata.valid);

    let invalid_config = Fixtures::config().invalid_cache_too_small();
    assert!(!invalid_config.metadata.valid);

    let invalid_ontology = Fixtures::ontology().invalid_cyclic_hierarchy();
    assert!(!invalid_ontology.metadata.valid);

    Ok(())
}

// =============================================================================
// Performance Examples
// =============================================================================

#[test]
fn example_fixture_reuse() {
    // Demonstrate fixture reuse for performance

    // Create fixture once
    let user_template = Fixtures::user().complete();

    // Reuse in multiple tests
    for i in 0..10 {
        let user = user_template.clone();
        assert_eq!(user.name, "User");
        assert_eq!(user.fields.len(), 7);
    }
}

#[test]
fn example_lazy_composition() -> Result<()> {
    // Composition is lazy - doesn't build until needed

    let composer = FixtureComposer::new()
        .add(Fixtures::user().minimal())
        .add(Fixtures::order().empty());

    // Not built yet
    assert_eq!(composer.fixtures().len(), 2);

    // Build when needed
    let ontology = composer.build_ontology()?;
    assert!(!ontology.ttl.is_empty());

    Ok(())
}
