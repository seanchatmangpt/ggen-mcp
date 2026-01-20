# Domain Model Harness Quick Reference

Quick reference for using the Chicago-style TDD Domain Model Test Harness.

## Import

```rust
use harness::domain_model_harness::*;
```

## Builders

### User
```rust
let user = UserBuilder::new()
    .email("user@example.com")
    .age(25)
    .with_address(address)
    .with_phone(phone)
    .status(UserStatus::Active)
    .build()?;
```

### Order
```rust
let order = OrderBuilder::new(user_id)
    .add_item(product_id, quantity: 2, Money::usd(1000))
    .calculate_total()
    .status(OrderStatus::Confirmed)
    .payment_status(PaymentStatus::Paid)
    .build()?;
```

### Product
```rust
let product = ProductBuilder::new()
    .name("Product Name")
    .price(Money::usd(2999))
    .stock(100)
    .status(ProductStatus::Active)
    .build();
```

## Harness Assertions

### Invariant Testing
```rust
let harness = DomainModelHarness::new();
harness.assert_invariant_holds(
    || user.validate_age_requirement(),
    "minimum_age"
);
```

### Business Rule Testing
```rust
harness.assert_rule_enforced(
    || order.validate_cancellation_allowed(),
    "no_cancel_paid_orders"
);
```

### Event Testing
```rust
let events = command.execute()?;
harness.assert_event_emitted(&events, "UserCreated");
```

### State Transition Testing
```rust
let before = user.clone();
user.apply_event(&event);
harness.assert_state_transition_valid(&before, &user, &event);
```

## Value Objects

### Email
```rust
let email = Email::new("user@example.com".to_string())?;
assert!(email.is_valid());
let verified = email.verify(); // Returns new instance
```

### Money
```rust
let price = Money::usd(2999);  // Amount in cents
let total = price.add(&shipping)?;
```

### Address
```rust
let address = Address::new(
    "123 Main St".to_string(),
    "City".to_string(),
    "State".to_string(),
    "12345".to_string(),
    "USA".to_string()
)?;
assert!(address.is_domestic());
```

### PhoneNumber
```rust
let phone = PhoneNumber::new("+1-555-0123".to_string())?;
```

## Commands

### CreateUser
```rust
let cmd = Command::CreateUser {
    email: "user@example.com".to_string(),
    age: 25,
};
cmd.validate()?;
let events = cmd.execute()?;
```

### PlaceOrder
```rust
let cmd = Command::PlaceOrder {
    user_id,
    items: vec![(product_id, quantity)],
};
let events = cmd.execute()?;
```

### AddToCart
```rust
let cmd = Command::AddToCart {
    cart_id,
    product_id,
    quantity: 3,
};
let events = cmd.execute()?;
```

### ProcessPayment
```rust
let cmd = Command::ProcessPayment {
    order_id,
    amount: Money::usd(10000),
    method: PaymentMethod::CreditCard,
};
let events = cmd.execute()?;
```

## Events

### Apply Event to Aggregate
```rust
let event = DomainEvent::UserCreated { user_id, email, age };
user.apply_event(&event);
```

### Event Validation
```rust
event.validate()?;
assert!(event.is_compatible_with_version(1));
```

### Event Serialization
```rust
let json = serde_json::to_string(&event)?;
let deserialized: DomainEvent = serde_json::from_str(&json)?;
```

## Domain Services

### OrderPricingService
```rust
let total = OrderPricingService::calculate_total(&items);
let discounted = OrderPricingService::apply_discount(total, 10);
let tax = OrderPricingService::calculate_tax(subtotal, 0.08);
```

### PaymentProcessingService
```rust
PaymentProcessingService::authorize_payment(&mut payment)?;
assert!(PaymentProcessingService::validate_payment_method(method));
```

### ShippingCalculator
```rust
let cost = ShippingCalculator::calculate_shipping_cost(&address, weight);
let days = ShippingCalculator::estimate_delivery_days(&address);
assert!(ShippingCalculator::is_address_valid_for_shipping(&address));
```

## Fixtures

### Loading Fixtures
```rust
let harness = DomainModelHarness::new();
let user: User = harness.load_fixture("aggregates", "valid_user")?;
let cmd: Command = harness.load_fixture("commands", "create_user_valid")?;
let event: DomainEvent = harness.load_fixture("events", "user_created")?;
```

## Common Test Patterns

### Test Invariant
```rust
#[test]
fn test_user_age_minimum() {
    let result = UserBuilder::new().age(17).build();
    assert!(result.is_err());
}
```

### Test State Transition
```rust
#[test]
fn test_email_verification() {
    let mut user = UserBuilder::new().build()?;
    let event = DomainEvent::EmailVerified { user_id: user.id.clone() };
    user.apply_event(&event);
    assert!(user.email.verified);
}
```

### Test Command Flow
```rust
#[test]
fn test_create_user_flow() {
    let cmd = Command::CreateUser { email: "test@example.com".to_string(), age: 25 };
    cmd.validate()?;
    let events = cmd.execute()?;
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], DomainEvent::UserCreated { .. }));
}
```

### Test Business Rule
```rust
#[test]
fn test_paid_order_cannot_be_cancelled() {
    let harness = DomainModelHarness::new();
    let order = OrderBuilder::new(user_id)
        .add_item(product_id, 1, Money::usd(1000))
        .calculate_total()
        .payment_status(PaymentStatus::Paid)
        .build()?;

    harness.assert_rule_enforced(
        || order.validate_cancellation_allowed(),
        "no_cancel_paid_orders"
    );
}
```

## DDD Pattern Checks

```rust
let harness = DomainModelHarness::new();

// Verify aggregates have roots
assert!(harness.validate_aggregate_has_root());

// Verify value objects are immutable
assert!(harness.validate_value_object_immutable());

// Verify commands are pure
assert!(harness.validate_commands_are_pure());

// Verify events are past tense
assert!(harness.validate_events_past_tense());
```

## Error Handling

### Domain Errors
```rust
match result {
    Err(DomainError::ValidationError { field, message }) => {
        // Handle validation error
    }
    Err(DomainError::InvariantViolation { invariant, message }) => {
        // Handle invariant violation
    }
    Err(DomainError::BusinessRuleViolation { rule, message }) => {
        // Handle business rule violation
    }
    Err(DomainError::NotFound { entity, id }) => {
        // Handle not found
    }
    Ok(_) => {
        // Success
    }
}
```

## Running Tests

```bash
# All domain model tests
cargo test domain_model_harness_tests

# Specific category
cargo test test_aggregate
cargo test test_value_object
cargo test test_command
cargo test test_event
cargo test test_.*_validation

# With output
cargo test domain_model_harness_tests -- --nocapture

# Single test
cargo test test_age_validation_18_plus
```

## Best Practices

1. **Use Builders**: Always use builders for test data construction
2. **Test State**: Focus on final state, not implementation
3. **Name Clearly**: Test names should describe behavior
4. **One Concept**: One assertion per logical concept
5. **Use Harness**: Use harness assertions for domain concepts

## Full Documentation

See [TDD_DOMAIN_MODEL_HARNESS.md](../../../docs/TDD_DOMAIN_MODEL_HARNESS.md) for complete documentation.
