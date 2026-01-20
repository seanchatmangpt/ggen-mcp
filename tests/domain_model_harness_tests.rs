//! Domain Model Harness Integration Tests
//!
//! These tests verify the domain model harness functionality and demonstrate
//! Chicago-style TDD for domain-driven design patterns.

mod harness;

use harness::domain_model_harness::*;

// ============================================================================
// Aggregate Tests
// ============================================================================

#[test]
fn test_aggregate_root_controls_access() {
    let user_id = UserId::generate();
    let product_id = ProductId::generate();

    let order = OrderBuilder::new(user_id)
        .add_item(product_id, 2, Money::usd(1000))
        .calculate_total()
        .build()
        .unwrap();

    assert_eq!(order.items.len(), 1);
    assert_eq!(order.total.amount, 2000);
}

#[test]
fn test_entities_within_boundary() {
    let user_id = UserId::generate();
    let product_id = ProductId::generate();

    let order = OrderBuilder::new(user_id)
        .add_item(product_id, 1, Money::usd(500))
        .calculate_total()
        .build()
        .unwrap();

    assert!(!order.items.is_empty());
    assert_eq!(order.items[0].product_id, product_id);
}

#[test]
fn test_consistency_maintained() {
    let user_id = UserId::generate();
    let product_id = ProductId::generate();

    let result = OrderBuilder::new(user_id)
        .add_item(product_id, 2, Money::usd(1000))
        .build();

    assert!(result.is_err());
}

#[test]
fn test_invariants_enforced() {
    let user_id = UserId::generate();

    let result = OrderBuilder::new(user_id).calculate_total().build();

    assert!(result.is_err());
}

// ============================================================================
// Value Object Tests
// ============================================================================

#[test]
fn test_value_object_immutable() {
    let email = Email::new("test@example.com".to_string()).unwrap();
    assert!(!email.verified);

    let verified_email = email.verify();
    assert!(!email.verified);
    assert!(verified_email.verified);
}

#[test]
fn test_value_object_validation_on_construction() {
    assert!(Email::new("test@example.com".to_string()).is_ok());
    assert!(Email::new("invalid".to_string()).is_err());
    assert!(Email::new("@example.com".to_string()).is_err());
    assert!(Email::new("test@".to_string()).is_err());
}

#[test]
fn test_value_object_equality() {
    let email1 = Email::new("test@example.com".to_string()).unwrap();
    let email2 = Email::new("test@example.com".to_string()).unwrap();
    let email3 = Email::new("other@example.com".to_string()).unwrap();

    assert_eq!(email1, email2);
    assert_ne!(email1, email3);
}

#[test]
fn test_value_object_no_identity() {
    let money1 = Money::usd(1000);
    let money2 = Money::usd(1000);

    assert_eq!(money1, money2);
}

// ============================================================================
// Command Tests
// ============================================================================

#[test]
fn test_command_validation_before_execution() {
    let cmd = Command::CreateUser {
        email: "invalid".to_string(),
        age: 25,
    };

    assert!(cmd.validate().is_err());
}

#[test]
fn test_command_state_transitions_valid() {
    let cmd = Command::CreateUser {
        email: "test@example.com".to_string(),
        age: 25,
    };

    assert!(cmd.validate().is_ok());
    let events = cmd.execute().unwrap();
    assert_eq!(events.len(), 1);
}

#[test]
fn test_command_events_emitted_correctly() {
    let cmd = Command::CreateUser {
        email: "test@example.com".to_string(),
        age: 25,
    };

    let events = cmd.execute().unwrap();
    assert!(matches!(events[0], DomainEvent::UserCreated { .. }));
}

#[test]
fn test_command_idempotency_preserved() {
    let cmd = Command::CreateUser {
        email: "test@example.com".to_string(),
        age: 25,
    };

    assert!(cmd.is_idempotent());

    let cmd = Command::AddToCart {
        cart_id: CartId::generate(),
        product_id: ProductId::generate(),
        quantity: 1,
    };

    assert!(!cmd.is_idempotent());
}

// ============================================================================
// Event Tests
// ============================================================================

#[test]
fn test_event_immutable_after_creation() {
    let event = DomainEvent::UserCreated {
        user_id: UserId::generate(),
        email: Email::new("test@example.com".to_string()).unwrap(),
        age: 25,
    };

    let _cloned = event.clone();
}

#[test]
fn test_event_serialization_preserves_data() {
    let event = DomainEvent::UserCreated {
        user_id: UserId::new("user_1".to_string()),
        email: Email::new("test@example.com".to_string()).unwrap(),
        age: 25,
    };

    let json = serde_json::to_string(&event).unwrap();
    let deserialized: DomainEvent = serde_json::from_str(&json).unwrap();

    assert_eq!(event, deserialized);
}

#[test]
fn test_event_version_compatibility() {
    let event = DomainEvent::UserCreated {
        user_id: UserId::generate(),
        email: Email::new("test@example.com".to_string()).unwrap(),
        age: 25,
    };

    assert!(event.is_compatible_with_version(1));
    assert!(event.is_compatible_with_version(2));
}

#[test]
fn test_event_replay_produces_same_state() {
    let mut user = User {
        id: UserId::new("user_1".to_string()),
        email: Email::new("old@example.com".to_string()).unwrap(),
        age: 20,
        address: None,
        phone: None,
        status: UserStatus::Pending,
        version: 0,
    };

    let event = DomainEvent::UserCreated {
        user_id: UserId::new("user_1".to_string()),
        email: Email::new("test@example.com".to_string()).unwrap(),
        age: 25,
    };

    user.apply_event(&event);

    assert_eq!(user.email.value, "test@example.com");
    assert_eq!(user.age, 25);
    assert_eq!(user.version, 1);
}

// ============================================================================
// Business Rule Tests
// ============================================================================

#[test]
fn test_age_validation_18_plus() {
    let result = UserBuilder::new().age(17).build();
    assert!(result.is_err());

    let result = UserBuilder::new().age(18).build();
    assert!(result.is_ok());
}

#[test]
fn test_order_total_calculation() {
    let user_id = UserId::generate();
    let product_id = ProductId::generate();

    let order = OrderBuilder::new(user_id)
        .add_item(product_id.clone(), 2, Money::usd(1000))
        .add_item(product_id, 1, Money::usd(500))
        .calculate_total()
        .build()
        .unwrap();

    assert_eq!(order.total.amount, 2500);
}

#[test]
fn test_stock_availability() {
    let mut product = ProductBuilder::new().stock(5).build();

    assert!(product.is_available(5));
    assert!(!product.is_available(6));

    assert!(product.reserve_stock(3).is_ok());
    assert_eq!(product.stock, 2);

    assert!(product.reserve_stock(3).is_err());
}

#[test]
fn test_payment_authorization() {
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

    assert!(PaymentProcessingService::authorize_payment(&mut payment).is_ok());
    assert_eq!(payment.status, PaymentStatus::Authorized);

    assert!(PaymentProcessingService::authorize_payment(&mut payment).is_err());
}

#[test]
fn test_shipping_eligibility() {
    let address = Address::new(
        "123 Main St".to_string(),
        "Springfield".to_string(),
        "IL".to_string(),
        "62701".to_string(),
        "USA".to_string(),
    )
    .unwrap();

    assert!(ShippingCalculator::is_address_valid_for_shipping(&address));

    let cost = ShippingCalculator::calculate_shipping_cost(&address, 5.0);
    assert_eq!(cost.amount, 1000);

    let days = ShippingCalculator::estimate_delivery_days(&address);
    assert_eq!(days, 3);
}

// ============================================================================
// Harness Integration Tests
// ============================================================================

#[test]
fn test_harness_assert_invariant_holds() {
    let harness = DomainModelHarness::new();
    let user = UserBuilder::new().age(25).build().unwrap();

    harness.assert_invariant_holds(|| user.validate_age_requirement(), "minimum_age");
}

#[test]
#[should_panic(expected = "Invariant 'minimum_age' violated")]
fn test_harness_assert_invariant_fails() {
    let harness = DomainModelHarness::new();
    let mut user = UserBuilder::new().age(25).build().unwrap();
    user.age = 17;

    harness.assert_invariant_holds(|| user.validate_age_requirement(), "minimum_age");
}

#[test]
fn test_harness_assert_rule_enforced() {
    let harness = DomainModelHarness::new();
    let user_id = UserId::generate();
    let order = OrderBuilder::new(user_id)
        .add_item(ProductId::generate(), 1, Money::usd(1000))
        .calculate_total()
        .payment_status(PaymentStatus::Paid)
        .build()
        .unwrap();

    harness.assert_rule_enforced(
        || order.validate_cancellation_allowed(),
        "no_cancel_paid_orders",
    );
}

#[test]
fn test_harness_assert_event_emitted() {
    let harness = DomainModelHarness::new();
    let cmd = Command::CreateUser {
        email: "test@example.com".to_string(),
        age: 25,
    };

    let events = cmd.execute().unwrap();
    harness.assert_event_emitted(&events, "UserCreated");
}

#[test]
fn test_harness_assert_state_transition() {
    let harness = DomainModelHarness::new();
    let mut user = UserBuilder::new().age(25).build().unwrap();
    let before = user.clone();

    let event = DomainEvent::EmailVerified {
        user_id: user.id.clone(),
    };
    user.apply_event(&event);

    harness.assert_state_transition_valid(&before, &user, &event);
}

#[test]
fn test_ddd_pattern_validations() {
    let harness = DomainModelHarness::new();

    assert!(harness.validate_aggregate_has_root());
    assert!(harness.validate_value_object_immutable());
    assert!(harness.validate_commands_are_pure());
    assert!(harness.validate_events_past_tense());
}

// ============================================================================
// Domain Service Tests
// ============================================================================

#[test]
fn test_order_pricing_service() {
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

    let total = OrderPricingService::calculate_total(&items);
    assert_eq!(total.amount, 2500);

    let discounted = OrderPricingService::apply_discount(total, 10);
    assert_eq!(discounted.amount, 2250);

    let tax = OrderPricingService::calculate_tax(total, 0.08);
    assert_eq!(tax.amount, 200);
}
