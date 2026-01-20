# Chicago-Style TDD Domain Model Test Harness

## Overview

This document describes the comprehensive Chicago-style Test-Driven Development (TDD) test harness for domain model validation in the ggen-mcp project.

## Table of Contents

1. [Philosophy](#philosophy)
2. [Architecture](#architecture)
3. [Domain Model Coverage](#domain-model-coverage)
4. [Test Fixtures](#test-fixtures)
5. [Domain Builders](#domain-builders)
6. [Behavior Verification](#behavior-verification)
7. [Domain Rule Assertions](#domain-rule-assertions)
8. [DDD Pattern Validation](#ddd-pattern-validation)
9. [Business Rule Testing](#business-rule-testing)
10. [Usage Examples](#usage-examples)
11. [Running Tests](#running-tests)

## Philosophy

### Chicago-Style TDD vs. London-Style TDD

**Chicago-Style (Classical) TDD** emphasizes:
- **State-based verification**: Testing the final state of the system
- **Testing observable behavior**: Focus on what the system produces, not how
- **Minimal mocking**: Use real objects when possible
- **Integration-friendly**: Tests naturally cover multiple components

This is ideal for **domain models** because:
- Domain models are all about **state and invariants**
- We care about **business rules being enforced**, not implementation details
- Domain logic is **deterministic** - same inputs produce same outputs
- **Aggregate boundaries** are natural test boundaries

**Key Principles:**
1. **Arrange**: Set up the domain state using builders
2. **Act**: Execute domain behavior (commands, state transitions)
3. **Assert**: Verify final state, invariants, and emitted events

## Architecture

```text
┌─────────────────────────────────────────────────────────────────┐
│                    DomainModelHarness                           │
│                  (Chicago-Style TDD Core)                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌────────────────┐  ┌────────────────┐  ┌─────────────────┐  │
│  │   Aggregates   │  │ Value Objects  │  │    Commands     │  │
│  │    Testing     │  │    Testing     │  │    Testing      │  │
│  ├────────────────┤  ├────────────────┤  ├─────────────────┤  │
│  │ - User         │  │ - Email        │  │ - CreateUser    │  │
│  │ - Order        │  │ - Money        │  │ - PlaceOrder    │  │
│  │ - Product      │  │ - Address      │  │ - AddToCart     │  │
│  │ - Cart         │  │ - PhoneNumber  │  │ - ProcessPayment│  │
│  │ - Payment      │  └────────────────┘  └─────────────────┘  │
│  │ - Shipment     │                                            │
│  └────────────────┘                                            │
│                                                                 │
│  ┌────────────────┐  ┌────────────────┐  ┌─────────────────┐  │
│  │     Events     │  │ Domain Services│  │ Business Rules  │  │
│  │    Testing     │  │    Testing     │  │    Testing      │  │
│  ├────────────────┤  ├────────────────┤  ├─────────────────┤  │
│  │ - UserCreated  │  │ - OrderPricing │  │ - Age >= 18     │  │
│  │ - OrderPlaced  │  │ - PaymentProc. │  │ - Email unique  │  │
│  │ - PaymentProc. │  │ - ShippingCalc │  │ - Stock avail.  │  │
│  └────────────────┘  └────────────────┘  └─────────────────┘  │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              Test Fixtures & Builders                    │  │
│  │  - JSON fixtures for valid/invalid states                │  │
│  │  - Builder pattern for test data construction            │  │
│  │  - Assertion helpers for invariants and rules            │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Domain Model Coverage

Following the **80/20 principle**, we model a common e-commerce domain that covers the most important DDD patterns:

### Aggregates (80% of Business Logic)

#### 1. User Aggregate
```rust
pub struct User {
    pub id: UserId,              // Aggregate root identity
    pub email: Email,            // Value object
    pub age: u8,                 // Primitive
    pub address: Option<Address>, // Value object
    pub phone: Option<PhoneNumber>, // Value object
    pub status: UserStatus,      // Enumeration
    pub version: u32,            // Optimistic locking
}
```

**Invariants:**
- Age must be 18 or older
- Active users must have verified email
- Email must be unique (enforced by repository)

**State Transitions:**
- Pending → Active (when email verified)
- Active → Suspended (violation)
- Active/Suspended → Deleted (hard delete)

#### 2. Order Aggregate
```rust
pub struct Order {
    pub id: OrderId,             // Aggregate root
    pub user_id: UserId,         // Reference to User aggregate
    pub items: Vec<OrderItem>,   // Entities within boundary
    pub total: Money,            // Value object
    pub status: OrderStatus,
    pub payment_status: PaymentStatus,
    pub version: u32,
}
```

**Invariants:**
- Order must have at least one item
- Total must match sum of item subtotals
- Paid orders cannot be cancelled

**Aggregate Boundary:**
- OrderItem entities are only accessible through Order
- Order is the consistency boundary

#### 3. Product Aggregate
```rust
pub struct Product {
    pub id: ProductId,
    pub name: String,
    pub price: Money,
    pub stock: u32,
    pub status: ProductStatus,
    pub version: u32,
}
```

**Invariants:**
- Active products must have stock available
- Price must be positive

#### 4. Cart Aggregate
```rust
pub struct Cart {
    pub id: CartId,
    pub user_id: UserId,
    pub items: HashMap<ProductId, CartItem>,
    pub version: u32,
}
```

**Behavior:**
- Add items with quantity
- Remove items
- Calculate total
- Convert to Order

#### 5. Payment Aggregate
```rust
pub struct Payment {
    pub id: PaymentId,
    pub order_id: OrderId,
    pub amount: Money,
    pub status: PaymentStatus,
    pub method: PaymentMethod,
    pub version: u32,
}
```

**Invariants:**
- Payment amount must be positive
- Can only authorize pending payments

#### 6. Shipment Aggregate
```rust
pub struct Shipment {
    pub id: ShipmentId,
    pub order_id: OrderId,
    pub address: Address,
    pub status: ShipmentStatus,
    pub tracking_number: Option<String>,
    pub version: u32,
}
```

**Invariants:**
- Shipped shipments must have tracking number

### Value Objects (Immutability and Validation)

#### 1. Email
```rust
pub struct Email {
    pub value: String,
    pub verified: bool,
}
```

**Properties:**
- Immutable after creation
- Validation on construction
- Value equality (no identity)

#### 2. Money
```rust
pub struct Money {
    pub amount: i64,    // Cents to avoid floating point
    pub currency: Currency,
}
```

**Properties:**
- Immutable
- Cannot add different currencies
- Value equality

#### 3. Address
```rust
pub struct Address {
    pub street: String,
    pub city: String,
    pub state: String,
    pub zip_code: String,
    pub country: String,
}
```

**Properties:**
- All fields required
- Immutable
- Domestic/international distinction

#### 4. PhoneNumber
```rust
pub struct PhoneNumber {
    pub value: String,
}
```

**Validation:**
- 10-15 digits
- International format support

### Commands (Intent to Change State)

#### 1. CreateUser
```rust
Command::CreateUser {
    email: String,
    age: u8,
}
```

**Validation:**
- Email format valid
- Age >= 18

**Events Emitted:**
- UserCreated

**Idempotency:** Yes (same email = same user)

#### 2. PlaceOrder
```rust
Command::PlaceOrder {
    user_id: UserId,
    items: Vec<(ProductId, u32)>,
}
```

**Validation:**
- Items not empty
- Products exist and available

**Events Emitted:**
- OrderPlaced

**Idempotency:** No (each order is unique)

#### 3. AddToCart
```rust
Command::AddToCart {
    cart_id: CartId,
    product_id: ProductId,
    quantity: u32,
}
```

**Validation:**
- Quantity > 0

**Events Emitted:**
- ItemAddedToCart

**Idempotency:** No (quantities accumulate)

#### 4. ProcessPayment
```rust
Command::ProcessPayment {
    order_id: OrderId,
    amount: Money,
    method: PaymentMethod,
}
```

**Validation:**
- Amount > 0
- Method supported

**Events Emitted:**
- PaymentProcessed

**Idempotency:** Yes (same order = same payment)

### Events (Things That Happened - Past Tense)

#### Event Naming Convention
All events are named in **past tense** to indicate they represent facts:

- ✅ **UserCreated** (correct - past tense)
- ❌ CreateUser (wrong - imperative)
- ❌ UserCreate (wrong - present tense)

#### 1. UserCreated
```rust
DomainEvent::UserCreated {
    user_id: UserId,
    email: Email,
    age: u8,
}
```

#### 2. OrderPlaced
```rust
DomainEvent::OrderPlaced {
    order_id: OrderId,
    user_id: UserId,
    item_count: usize,
}
```

#### 3. PaymentProcessed
```rust
DomainEvent::PaymentProcessed {
    payment_id: PaymentId,
    order_id: OrderId,
    amount: Money,
    method: PaymentMethod,
}
```

**Event Properties:**
- Immutable after creation
- Serializable (for event sourcing)
- Version compatible
- Replay produces same state

### Domain Services (Stateless Business Logic)

#### 1. OrderPricingService
```rust
impl OrderPricingService {
    pub fn calculate_total(items: &[OrderItem]) -> Money;
    pub fn apply_discount(total: Money, discount_percent: u8) -> Money;
    pub fn calculate_tax(subtotal: Money, tax_rate: f64) -> Money;
}
```

**Characteristics:**
- Stateless
- Pure functions
- Domain logic that doesn't belong to a single aggregate

#### 2. PaymentProcessingService
```rust
impl PaymentProcessingService {
    pub fn authorize_payment(payment: &mut Payment) -> Result<(), DomainError>;
    pub fn validate_payment_method(method: PaymentMethod) -> bool;
}
```

#### 3. ShippingCalculator
```rust
impl ShippingCalculator {
    pub fn calculate_shipping_cost(address: &Address, weight: f64) -> Money;
    pub fn estimate_delivery_days(address: &Address) -> u8;
    pub fn is_address_valid_for_shipping(address: &Address) -> bool;
}
```

## Test Fixtures

### Directory Structure

```text
tests/fixtures/domain/
├── aggregates/
│   ├── valid_user.json
│   ├── invalid_user.json
│   ├── valid_order.json
│   ├── invalid_order.json
│   └── valid_product.json
├── commands/
│   ├── create_user_valid.json
│   ├── create_user_invalid.json
│   ├── place_order_valid.json
│   ├── place_order_invalid.json
│   ├── add_to_cart_valid.json
│   └── add_to_cart_invalid.json
├── events/
│   ├── user_created.json
│   ├── order_placed.json
│   ├── payment_processed.json
│   └── email_verified.json
└── value_objects/
    ├── valid_email.json
    ├── valid_address.json
    └── valid_money.json
```

### Example Fixtures

#### Valid User Aggregate
```json
{
  "id": "user_123",
  "email": {
    "value": "john.doe@example.com",
    "verified": true
  },
  "age": 30,
  "address": {
    "street": "123 Main Street",
    "city": "Springfield",
    "state": "IL",
    "zip_code": "62701",
    "country": "USA"
  },
  "phone": {
    "value": "+1-555-0123"
  },
  "status": "Active",
  "version": 1
}
```

#### Valid Order Aggregate
```json
{
  "id": "order_789",
  "user_id": "user_123",
  "items": [
    {
      "product_id": "product_001",
      "quantity": 2,
      "unit_price": {
        "amount": 2999,
        "currency": "USD"
      },
      "subtotal": {
        "amount": 5998,
        "currency": "USD"
      }
    }
  ],
  "total": {
    "amount": 5998,
    "currency": "USD"
  },
  "status": "Confirmed",
  "payment_status": "Paid",
  "version": 3
}
```

### Loading Fixtures

```rust
let harness = DomainModelHarness::new();
let user: User = harness.load_fixture("aggregates", "valid_user")?;
```

## Domain Builders

Domain builders follow the **Test Data Builder Pattern** for fluent, readable test data construction.

### UserBuilder

```rust
let user = UserBuilder::new()
    .id(UserId::new("user_123".to_string()))
    .email("john@example.com")
    .age(30)
    .with_address(address)
    .with_phone(phone)
    .status(UserStatus::Active)
    .with_valid_state()
    .build()?;
```

**Features:**
- Fluent API
- Sensible defaults
- Validation on build
- `with_valid_state()` helper for common valid configurations

### OrderBuilder

```rust
let order = OrderBuilder::new(user_id)
    .add_item(product_id, 2, Money::usd(1000))
    .add_item(product_id2, 1, Money::usd(500))
    .calculate_total()
    .status(OrderStatus::Confirmed)
    .payment_status(PaymentStatus::Paid)
    .build()?;
```

**Features:**
- Automatic total calculation
- Item management
- State configuration

### ProductBuilder

```rust
let product = ProductBuilder::new()
    .name("Premium Headphones")
    .price(Money::usd(29999))
    .stock(100)
    .status(ProductStatus::Active)
    .build();
```

## Behavior Verification

### Aggregate Tests

#### Test: Aggregate Root Controls Access
```rust
#[test]
fn test_aggregate_root_controls_access() {
    let user_id = UserId::generate();
    let product_id = ProductId::generate();

    let order = OrderBuilder::new(user_id)
        .add_item(product_id, 2, Money::usd(1000))
        .calculate_total()
        .build()
        .unwrap();

    // OrderItems are only accessible through Order
    assert_eq!(order.items.len(), 1);
    assert_eq!(order.total.amount, 2000);
}
```

#### Test: Entities Within Boundary
```rust
#[test]
fn test_entities_within_boundary() {
    // OrderItem entities exist only within Order aggregate
    let user_id = UserId::generate();
    let order = OrderBuilder::new(user_id)
        .add_item(ProductId::generate(), 1, Money::usd(500))
        .calculate_total()
        .build()
        .unwrap();

    assert!(!order.items.is_empty());
}
```

#### Test: Consistency Maintained
```rust
#[test]
fn test_consistency_maintained() {
    let user_id = UserId::generate();
    let result = OrderBuilder::new(user_id)
        .add_item(ProductId::generate(), 2, Money::usd(1000))
        // Wrong total intentionally
        .build();

    // Should fail because total doesn't match items
    assert!(result.is_err());
}
```

#### Test: Invariants Enforced
```rust
#[test]
fn test_invariants_enforced() {
    let user_id = UserId::generate();

    // Order without items violates invariant
    let result = OrderBuilder::new(user_id)
        .calculate_total()
        .build();

    assert!(result.is_err());
}
```

### Value Object Tests

#### Test: Immutable Once Created
```rust
#[test]
fn test_value_object_immutable() {
    let email = Email::new("test@example.com".to_string()).unwrap();
    assert!(!email.verified);

    // verify() returns new instance, doesn't mutate
    let verified_email = email.verify();
    assert!(!email.verified);      // Original unchanged
    assert!(verified_email.verified); // New instance verified
}
```

#### Test: Validation on Construction
```rust
#[test]
fn test_value_object_validation() {
    assert!(Email::new("test@example.com".to_string()).is_ok());
    assert!(Email::new("invalid".to_string()).is_err());
}
```

#### Test: Value Equality
```rust
#[test]
fn test_value_object_equality() {
    let email1 = Email::new("test@example.com".to_string()).unwrap();
    let email2 = Email::new("test@example.com".to_string()).unwrap();

    assert_eq!(email1, email2); // Value equality
}
```

### Command Tests

#### Test: Validation Before Execution
```rust
#[test]
fn test_command_validation() {
    let cmd = Command::CreateUser {
        email: "invalid".to_string(),
        age: 25,
    };

    assert!(cmd.validate().is_err());
}
```

#### Test: Events Emitted Correctly
```rust
#[test]
fn test_command_events() {
    let cmd = Command::CreateUser {
        email: "test@example.com".to_string(),
        age: 25,
    };

    let events = cmd.execute().unwrap();
    assert!(matches!(events[0], DomainEvent::UserCreated { .. }));
}
```

#### Test: Idempotency Preserved
```rust
#[test]
fn test_command_idempotency() {
    let cmd = Command::CreateUser {
        email: "test@example.com".to_string(),
        age: 25,
    };

    assert!(cmd.is_idempotent());
}
```

### Event Tests

#### Test: Serialization Preserves Data
```rust
#[test]
fn test_event_serialization() {
    let event = DomainEvent::UserCreated {
        user_id: UserId::new("user_1".to_string()),
        email: Email::new("test@example.com".to_string()).unwrap(),
        age: 25,
    };

    let json = serde_json::to_string(&event).unwrap();
    let deserialized: DomainEvent = serde_json::from_str(&json).unwrap();

    assert_eq!(event, deserialized);
}
```

#### Test: Replay Produces Same State
```rust
#[test]
fn test_event_replay() {
    let mut user = /* create initial user */;
    let event = DomainEvent::EmailVerified {
        user_id: user.id.clone(),
    };

    user.apply_event(&event);

    assert!(user.email.verified);
    assert_eq!(user.version, 1);
}
```

## Domain Rule Assertions

The harness provides specialized assertion helpers for domain rules:

### assert_invariant_holds

```rust
harness.assert_invariant_holds(
    || user.validate_age_requirement(),
    "minimum_age"
);
```

Verifies that an invariant is maintained. Panics with a clear message if violated.

### assert_rule_enforced

```rust
harness.assert_rule_enforced(
    || order.validate_cancellation_allowed(),
    "no_cancel_paid_orders"
);
```

Verifies that a business rule prevents an invalid operation. Expects the check to fail.

### assert_event_emitted

```rust
let events = command.execute().unwrap();
harness.assert_event_emitted(&events, "UserCreated");
```

Verifies that a specific event type was emitted during command execution.

### assert_state_transition_valid

```rust
let before = user.clone();
user.apply_event(&event);
harness.assert_state_transition_valid(&before, &user, &event);
```

Verifies that applying an event changes the aggregate state.

## DDD Pattern Validation

### Pattern: Aggregates Have Roots

```rust
#[test]
fn test_validate_aggregate_has_root() {
    let harness = DomainModelHarness::new();
    assert!(harness.validate_aggregate_has_root());
}
```

Validates that:
- Order is an aggregate root
- OrderItem is an entity within Order
- Access to OrderItem goes through Order

### Pattern: Value Objects Are Immutable

```rust
#[test]
fn test_validate_value_object_immutable() {
    let harness = DomainModelHarness::new();
    assert!(harness.validate_value_object_immutable());
}
```

Validates that:
- Email, Money, Address have no setters
- Changes return new instances
- Original instances remain unchanged

### Pattern: Commands Are Pure

```rust
#[test]
fn test_validate_commands_are_pure() {
    let harness = DomainModelHarness::new();
    assert!(harness.validate_commands_are_pure());
}
```

Validates that:
- Commands validate input
- Commands produce events
- No side effects during command processing

### Pattern: Events Are Past Tense

```rust
#[test]
fn test_validate_events_past_tense() {
    let harness = DomainModelHarness::new();
    assert!(harness.validate_events_past_tense());
}
```

Validates that:
- UserCreated (not CreateUser)
- OrderPlaced (not PlaceOrder)
- PaymentProcessed (not ProcessPayment)

## Business Rule Testing

### Rule: Age Validation (18+)

```rust
#[test]
fn test_age_validation_18_plus() {
    let result = UserBuilder::new().age(17).build();
    assert!(result.is_err());

    let result = UserBuilder::new().age(18).build();
    assert!(result.is_ok());
}
```

### Rule: Order Total Calculation

```rust
#[test]
fn test_order_total_calculation() {
    let user_id = UserId::generate();
    let order = OrderBuilder::new(user_id)
        .add_item(ProductId::generate(), 2, Money::usd(1000))
        .add_item(ProductId::generate(), 1, Money::usd(500))
        .calculate_total()
        .build()
        .unwrap();

    assert_eq!(order.total.amount, 2500);
}
```

### Rule: Stock Availability

```rust
#[test]
fn test_stock_availability() {
    let mut product = ProductBuilder::new().stock(5).build();

    assert!(product.is_available(5));
    assert!(!product.is_available(6));

    product.reserve_stock(3).unwrap();
    assert_eq!(product.stock, 2);
}
```

### Rule: Payment Authorization

```rust
#[test]
fn test_payment_authorization() {
    let mut payment = /* create payment */;

    assert!(PaymentProcessingService::authorize_payment(&mut payment).is_ok());
    assert_eq!(payment.status, PaymentStatus::Authorized);

    // Cannot authorize twice
    assert!(PaymentProcessingService::authorize_payment(&mut payment).is_err());
}
```

### Rule: Shipping Eligibility

```rust
#[test]
fn test_shipping_eligibility() {
    let address = Address::new(
        "123 Main St".to_string(),
        "Springfield".to_string(),
        "IL".to_string(),
        "62701".to_string(),
        "USA".to_string(),
    ).unwrap();

    assert!(ShippingCalculator::is_address_valid_for_shipping(&address));

    let cost = ShippingCalculator::calculate_shipping_cost(&address, 5.0);
    assert_eq!(cost.amount, 1000); // 500 base + 500 weight
}
```

## Usage Examples

### Example 1: Complete User Registration Flow

```rust
#[test]
fn test_user_registration_flow() {
    let harness = DomainModelHarness::new();

    // 1. Create user command
    let cmd = Command::CreateUser {
        email: "new.user@example.com".to_string(),
        age: 25,
    };

    // 2. Validate command
    assert!(cmd.validate().is_ok());

    // 3. Execute and get events
    let events = cmd.execute().unwrap();
    harness.assert_event_emitted(&events, "UserCreated");

    // 4. Build user from event
    let user = if let DomainEvent::UserCreated { user_id, email, age } = &events[0] {
        User {
            id: user_id.clone(),
            email: email.clone(),
            age: *age,
            address: None,
            phone: None,
            status: UserStatus::Pending,
            version: 0,
        }
    } else {
        panic!("Expected UserCreated event");
    };

    // 5. Verify invariants
    harness.assert_invariant_holds(
        || user.validate_invariants(),
        "user_invariants"
    );
}
```

### Example 2: Order Placement with Business Rules

```rust
#[test]
fn test_order_placement_with_rules() {
    let harness = DomainModelHarness::new();

    // Arrange: Create products with stock
    let mut product1 = ProductBuilder::new()
        .id(ProductId::new("prod_1".to_string()))
        .stock(10)
        .price(Money::usd(2999))
        .build();

    // Act: Reserve stock
    product1.reserve_stock(2).unwrap();

    // Arrange: Create order
    let user_id = UserId::generate();
    let order = OrderBuilder::new(user_id)
        .add_item(ProductId::new("prod_1".to_string()), 2, Money::usd(2999))
        .calculate_total()
        .build()
        .unwrap();

    // Assert: Verify state
    assert_eq!(product1.stock, 8);
    assert_eq!(order.total.amount, 5998);

    // Assert: Verify invariants
    harness.assert_invariant_holds(
        || order.validate_invariants(),
        "order_invariants"
    );
}
```

### Example 3: Payment Processing Flow

```rust
#[test]
fn test_payment_processing_flow() {
    let harness = DomainModelHarness::new();

    // Arrange: Create order
    let user_id = UserId::generate();
    let order = OrderBuilder::new(user_id.clone())
        .add_item(ProductId::generate(), 1, Money::usd(10000))
        .calculate_total()
        .build()
        .unwrap();

    // Act: Process payment command
    let cmd = Command::ProcessPayment {
        order_id: order.id.clone(),
        amount: order.total,
        method: PaymentMethod::CreditCard,
    };

    let events = cmd.execute().unwrap();

    // Assert: Payment event emitted
    harness.assert_event_emitted(&events, "PaymentProcessed");

    // Assert: Idempotency
    assert!(cmd.is_idempotent());
}
```

## Running Tests

### Run All Domain Model Tests

```bash
cargo test --test harness::domain_model_harness
```

### Run Specific Test Category

```bash
# Aggregate tests
cargo test --test harness::domain_model_harness test_aggregate

# Value object tests
cargo test --test harness::domain_model_harness test_value_object

# Command tests
cargo test --test harness::domain_model_harness test_command

# Event tests
cargo test --test harness::domain_model_harness test_event

# Business rule tests
cargo test --test harness::domain_model_harness test_.*_validation
```

### Run with Output

```bash
cargo test --test harness::domain_model_harness -- --nocapture
```

### Run with Coverage

```bash
cargo tarpaulin --test harness::domain_model_harness
```

## Benefits of This Approach

### 1. **Clear Domain Model**
- Explicit aggregates, entities, value objects
- Clear boundaries and invariants
- Self-documenting business rules

### 2. **Comprehensive Coverage**
- State-based testing covers all paths naturally
- Invariants tested at boundaries
- Business rules enforced consistently

### 3. **Maintainable Tests**
- Builders make test data construction easy
- Fixtures provide realistic scenarios
- Assertion helpers make intent clear

### 4. **Refactoring Safety**
- Tests focus on behavior, not implementation
- Can refactor internals without breaking tests
- Invariants catch regression bugs

### 5. **Living Documentation**
- Tests demonstrate how domain works
- Fixtures show valid/invalid states
- Business rules are executable specs

## Best Practices

### 1. **Use Builders for Test Data**
```rust
// ✅ Good - Readable, maintainable
let user = UserBuilder::new()
    .age(25)
    .email("test@example.com")
    .build()?;

// ❌ Bad - Verbose, fragile
let user = User {
    id: UserId::new("user_1".to_string()),
    email: Email::new("test@example.com".to_string()).unwrap(),
    age: 25,
    address: None,
    phone: None,
    status: UserStatus::Pending,
    version: 0,
};
```

### 2. **Test State, Not Implementation**
```rust
// ✅ Good - Tests observable behavior
assert_eq!(order.total.amount, 2500);
assert_eq!(order.status, OrderStatus::Confirmed);

// ❌ Bad - Tests implementation details
assert!(order.calculate_total_called);
```

### 3. **Name Tests After Behavior**
```rust
// ✅ Good - Describes what should happen
#[test]
fn test_order_total_matches_sum_of_items() { }

// ❌ Bad - Describes implementation
#[test]
fn test_calculate_total_function() { }
```

### 4. **One Assertion Per Concept**
```rust
// ✅ Good - Clear what's being tested
#[test]
fn test_user_age_must_be_18_or_older() {
    assert!(UserBuilder::new().age(17).build().is_err());
}

#[test]
fn test_user_with_valid_age_succeeds() {
    assert!(UserBuilder::new().age(18).build().is_ok());
}
```

### 5. **Use Harness Assertions**
```rust
// ✅ Good - Expressive, domain-specific
harness.assert_invariant_holds(
    || user.validate_age_requirement(),
    "minimum_age"
);

// ❌ Bad - Generic, unclear
assert!(user.validate_age_requirement().is_ok());
```

## Conclusion

This Chicago-style TDD test harness provides comprehensive domain model validation through:

- **State-based testing** focusing on observable behavior
- **Complete DDD pattern coverage** (aggregates, value objects, commands, events)
- **Business rule validation** ensuring domain integrity
- **Test fixtures and builders** for maintainable test data
- **Domain-specific assertions** for clear test intent

By following these patterns, we ensure our domain model is:
- **Correct**: Invariants always hold
- **Complete**: All business rules enforced
- **Maintainable**: Tests survive refactoring
- **Documented**: Tests demonstrate usage

## Related Documentation

- [Chicago-Style TDD Philosophy](./TDD_PHILOSOPHY.md)
- [Domain-Driven Design Patterns](./DDD_PATTERNS.md)
- [Test Data Builders](./TEST_BUILDERS.md)
- [Event Sourcing Guide](./EVENT_SOURCING.md)

---

**File:** `/home/user/ggen-mcp/tests/harness/domain_model_harness.rs`
**Fixtures:** `/home/user/ggen-mcp/tests/fixtures/domain/`
**Documentation:** `/home/user/ggen-mcp/docs/TDD_DOMAIN_MODEL_HARNESS.md`
