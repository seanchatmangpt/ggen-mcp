//! Domain Model Harness Integration Tests
//!
//! These tests verify the domain model harness functionality and demonstrate
//! Chicago-style TDD for domain-driven design patterns.

mod harness;

use chicago_tdd_tools::prelude::*;
use harness::domain_model_harness::*;

// ============================================================================
// Aggregate Tests
// ============================================================================

test!(test_aggregate_root_controls_access, {
    // Arrange: Set up aggregate with items
    let user_id = UserId::generate();
    let product_id = ProductId::generate();

    // Act: Build order with items through aggregate root
    let order = OrderBuilder::new(user_id)
        .add_item(product_id, 2, Money::usd(1000))
        .calculate_total()
        .build()?;

    // Assert: Aggregate controls access and maintains consistency
    assert_eq!(order.items.len(), 1);
    assert_eq!(order.total.amount, 2000);
});

test!(test_entities_within_boundary, {
    // Arrange: Set up aggregate components
    let user_id = UserId::generate();
    let product_id = ProductId::generate();

    // Act: Create order with entity items
    let order = OrderBuilder::new(user_id)
        .add_item(product_id, 1, Money::usd(500))
        .calculate_total()
        .build()?;

    // Assert: Entities exist within aggregate boundary
    assert!(!order.items.is_empty());
    assert_eq!(order.items[0].product_id, product_id);
});

test!(test_consistency_maintained, {
    // Arrange: Set up order without calculating total
    let user_id = UserId::generate();
    let product_id = ProductId::generate();

    // Act: Attempt to build order without required calculation
    let result = OrderBuilder::new(user_id)
        .add_item(product_id, 2, Money::usd(1000))
        .build();

    // Assert: Consistency rules enforced
    assert_err!(result);
});

test!(test_invariants_enforced, {
    // Arrange: Set up empty order
    let user_id = UserId::generate();

    // Act: Attempt to build order without items
    let result = OrderBuilder::new(user_id).calculate_total().build();

    // Assert: Invariants prevent invalid states
    assert_err!(result);
});

// ============================================================================
// Value Object Tests
// ============================================================================

test!(test_value_object_immutable, {
    // Arrange: Create initial value object
    let email = Email::new("test@example.com".to_string())?;

    // Act: Verify email (creates new instance)
    let verified_email = email.verify();

    // Assert: Original unchanged, new instance has updated state
    assert!(!email.verified);
    assert!(verified_email.verified);
});

test!(test_value_object_validation_on_construction, {
    // Arrange & Act & Assert: Valid email addresses succeed
    assert_ok!(Email::new("test@example.com".to_string()));

    // Assert: Invalid email addresses fail
    assert_err!(Email::new("invalid".to_string()));
    assert_err!(Email::new("@example.com".to_string()));
    assert_err!(Email::new("test@".to_string()));
});

test!(test_value_object_equality, {
    // Arrange: Create value objects with same and different values
    let email1 = Email::new("test@example.com".to_string())?;
    let email2 = Email::new("test@example.com".to_string())?;
    let email3 = Email::new("other@example.com".to_string())?;

    // Assert: Equality based on value, not identity
    assert_eq!(email1, email2);
    assert_ne!(email1, email3);
});

test!(test_value_object_no_identity, {
    // Arrange: Create two money instances with same value
    let money1 = Money::usd(1000);
    let money2 = Money::usd(1000);

    // Assert: Value objects have no identity
    assert_eq!(money1, money2);
});

// ============================================================================
// Command Tests
// ============================================================================

test!(test_command_validation_before_execution, {
    // Arrange: Create command with invalid data
    let cmd = Command::CreateUser {
        email: "invalid".to_string(),
        age: 25,
    };

    // Act: Validate command
    let result = cmd.validate();

    // Assert: Validation fails for invalid data
    assert_err!(result);
});

test!(test_command_state_transitions_valid, {
    // Arrange: Create valid command
    let cmd = Command::CreateUser {
        email: "test@example.com".to_string(),
        age: 25,
    };

    // Act: Validate and execute command
    assert_ok!(cmd.validate());
    let events = cmd.execute()?;

    // Assert: Command produces expected events
    assert_eq!(events.len(), 1);
});

test!(test_command_events_emitted_correctly, {
    // Arrange: Create command
    let cmd = Command::CreateUser {
        email: "test@example.com".to_string(),
        age: 25,
    };

    // Act: Execute command
    let events = cmd.execute()?;

    // Assert: Correct event type emitted
    assert!(matches!(events[0], DomainEvent::UserCreated { .. }));
});

test!(test_command_idempotency_preserved, {
    // Arrange & Assert: CreateUser is idempotent
    let cmd = Command::CreateUser {
        email: "test@example.com".to_string(),
        age: 25,
    };
    assert!(cmd.is_idempotent());

    // Arrange & Assert: AddToCart is not idempotent
    let cmd = Command::AddToCart {
        cart_id: CartId::generate(),
        product_id: ProductId::generate(),
        quantity: 1,
    };
    assert!(!cmd.is_idempotent());
});

// ============================================================================
// Event Tests
// ============================================================================

test!(test_event_immutable_after_creation, {
    // Arrange: Create domain event
    let event = DomainEvent::UserCreated {
        user_id: UserId::generate(),
        email: Email::new("test@example.com".to_string())?,
        age: 25,
    };

    // Act: Clone event
    let _cloned = event.clone();

    // Assert: Event can be cloned (immutable)
});

test!(test_event_serialization_preserves_data, {
    // Arrange: Create event with specific data
    let event = DomainEvent::UserCreated {
        user_id: UserId::new("user_1".to_string()),
        email: Email::new("test@example.com".to_string())?,
        age: 25,
    };

    // Act: Serialize and deserialize
    let json = serde_json::to_string(&event)?;
    let deserialized: DomainEvent = serde_json::from_str(&json)?;

    // Assert: Data preserved through serialization
    assert_eq!(event, deserialized);
});

test!(test_event_version_compatibility, {
    // Arrange: Create event
    let event = DomainEvent::UserCreated {
        user_id: UserId::generate(),
        email: Email::new("test@example.com".to_string())?,
        age: 25,
    };

    // Assert: Event compatible with multiple versions
    assert!(event.is_compatible_with_version(1));
    assert!(event.is_compatible_with_version(2));
});

test!(test_event_replay_produces_same_state, {
    // Arrange: Create initial user state
    let mut user = User {
        id: UserId::new("user_1".to_string()),
        email: Email::new("old@example.com".to_string())?,
        age: 20,
        address: None,
        phone: None,
        status: UserStatus::Pending,
        version: 0,
    };

    // Act: Apply event to user
    let event = DomainEvent::UserCreated {
        user_id: UserId::new("user_1".to_string()),
        email: Email::new("test@example.com".to_string())?,
        age: 25,
    };
    user.apply_event(&event);

    // Assert: State updated correctly and version incremented
    assert_eq!(user.email.value, "test@example.com");
    assert_eq!(user.age, 25);
    assert_eq!(user.version, 1);
});

// ============================================================================
// Business Rule Tests
// ============================================================================

test!(test_age_validation_18_plus, {
    // Arrange & Act: Attempt to create user under 18
    let result = UserBuilder::new().age(17).build();

    // Assert: Age validation enforces minimum age
    assert_err!(result);

    // Arrange & Act: Create user at minimum age
    let result = UserBuilder::new().age(18).build();

    // Assert: Valid age accepted
    assert_ok!(result);
});

test!(test_order_total_calculation, {
    // Arrange: Set up order with multiple items
    let user_id = UserId::generate();
    let product_id = ProductId::generate();

    // Act: Build order with calculated total
    let order = OrderBuilder::new(user_id)
        .add_item(product_id.clone(), 2, Money::usd(1000))
        .add_item(product_id, 1, Money::usd(500))
        .calculate_total()
        .build()?;

    // Assert: Total calculated correctly (2*1000 + 1*500)
    assert_eq!(order.total.amount, 2500);
});

test!(test_stock_availability, {
    // Arrange: Create product with stock
    let mut product = ProductBuilder::new().stock(5).build();

    // Assert: Stock availability checks
    assert!(product.is_available(5));
    assert!(!product.is_available(6));

    // Act: Reserve stock
    let result = product.reserve_stock(3);

    // Assert: Stock reserved successfully
    assert_ok!(result);
    assert_eq!(product.stock, 2);

    // Act: Attempt to reserve more than available
    let result = product.reserve_stock(3);

    // Assert: Insufficient stock prevented
    assert_err!(result);
});

test!(test_payment_authorization, {
    // Arrange: Create pending payment
    let payment_id = PaymentId::generate();
    let order_id = OrderId::generate();
    let mut payment = Payment {
        id: payment_id,
        order_id,
        amount: Money::usd(10000),
        status: PaymentStatus::Pending,
        method: PaymentMethod::CreditCard,
        version: 0,
    };

    // Act: Authorize payment
    let result = PaymentProcessingService::authorize_payment(&mut payment);

    // Assert: Payment authorized successfully
    assert_ok!(result);
    assert_eq!(payment.status, PaymentStatus::Authorized);

    // Act: Attempt to re-authorize
    let result = PaymentProcessingService::authorize_payment(&mut payment);

    // Assert: Double authorization prevented
    assert_err!(result);
});

test!(test_shipping_eligibility, {
    // Arrange: Create valid shipping address
    let address = Address::new(
        "123 Main St".to_string(),
        "Springfield".to_string(),
        "IL".to_string(),
        "62701".to_string(),
        "USA".to_string(),
    )?;

    // Assert: Address valid for shipping
    assert!(ShippingCalculator::is_address_valid_for_shipping(&address));

    // Act: Calculate shipping cost
    let cost = ShippingCalculator::calculate_shipping_cost(&address, 5.0);

    // Assert: Shipping cost calculated correctly
    assert_eq!(cost.amount, 1000);

    // Act: Estimate delivery time
    let days = ShippingCalculator::estimate_delivery_days(&address);

    // Assert: Delivery estimate provided
    assert_eq!(days, 3);
});

// ============================================================================
// Harness Integration Tests
// ============================================================================

test!(test_harness_assert_invariant_holds, {
    // Arrange: Create harness and valid user
    let harness = DomainModelHarness::new();
    let user = UserBuilder::new().age(25).build()?;

    // Act & Assert: Invariant holds for valid user
    harness.assert_invariant_holds(|| user.validate_age_requirement(), "minimum_age");
});

test!(test_harness_assert_invariant_fails, {
    // Arrange: Create harness and user with invalid age
    let harness = DomainModelHarness::new();
    let mut user = UserBuilder::new().age(25).build()?;
    user.age = 17;

    // Act: Attempt to validate invalid invariant
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        harness.assert_invariant_holds(|| user.validate_age_requirement(), "minimum_age");
    }));

    // Assert: Invariant violation detected
    assert!(result.is_err(), "Expected panic for invariant violation");
});

test!(test_harness_assert_rule_enforced, {
    // Arrange: Create harness and paid order
    let harness = DomainModelHarness::new();
    let user_id = UserId::generate();
    let order = OrderBuilder::new(user_id)
        .add_item(ProductId::generate(), 1, Money::usd(1000))
        .calculate_total()
        .payment_status(PaymentStatus::Paid)
        .build()?;

    // Act & Assert: Rule enforced for paid order
    harness.assert_rule_enforced(
        || order.validate_cancellation_allowed(),
        "no_cancel_paid_orders",
    );
});

test!(test_harness_assert_event_emitted, {
    // Arrange: Create harness and command
    let harness = DomainModelHarness::new();
    let cmd = Command::CreateUser {
        email: "test@example.com".to_string(),
        age: 25,
    };

    // Act: Execute command
    let events = cmd.execute()?;

    // Assert: Expected event emitted
    harness.assert_event_emitted(&events, "UserCreated");
});

test!(test_harness_assert_state_transition, {
    // Arrange: Create harness, user, and event
    let harness = DomainModelHarness::new();
    let mut user = UserBuilder::new().age(25).build()?;
    let before = user.clone();

    // Act: Apply state transition
    let event = DomainEvent::EmailVerified {
        user_id: user.id.clone(),
    };
    user.apply_event(&event);

    // Assert: Valid state transition
    harness.assert_state_transition_valid(&before, &user, &event);
});

test!(test_ddd_pattern_validations, {
    // Arrange: Create harness
    let harness = DomainModelHarness::new();

    // Assert: DDD patterns validated
    assert!(harness.validate_aggregate_has_root());
    assert!(harness.validate_value_object_immutable());
    assert!(harness.validate_commands_are_pure());
    assert!(harness.validate_events_past_tense());
});

// ============================================================================
// Domain Service Tests
// ============================================================================

test!(test_order_pricing_service, {
    // Arrange: Create order items
    let product_id = ProductId::generate();
    let items = vec![
        OrderItem {
            product_id: product_id.clone(),
            quantity: 2,
            unit_price: Money::usd(1000),
            subtotal: Money::usd(2000),
        },
        OrderItem {
            product_id,
            quantity: 1,
            unit_price: Money::usd(500),
            subtotal: Money::usd(500),
        },
    ];

    // Act: Calculate total
    let total = OrderPricingService::calculate_total(&items);

    // Assert: Total calculated correctly
    assert_eq!(total.amount, 2500);

    // Act: Apply discount
    let discounted = OrderPricingService::apply_discount(total, 10);

    // Assert: Discount applied correctly (10% off 2500 = 2250)
    assert_eq!(discounted.amount, 2250);

    // Act: Calculate tax
    let tax = OrderPricingService::calculate_tax(total, 0.08);

    // Assert: Tax calculated correctly (8% of 2500 = 200)
    assert_eq!(tax.amount, 200);
});
