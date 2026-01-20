# Domain Model Test Fixtures

This directory contains JSON fixtures for testing domain models following Domain-Driven Design (DDD) patterns.

## Directory Structure

```text
domain/
├── aggregates/      # Full aggregate root states
├── commands/        # Command inputs (valid and invalid)
├── events/          # Domain events
└── value_objects/   # Value object examples
```

## Fixture Categories

### Aggregates

**Purpose:** Complete aggregate states for testing invariants and state transitions.

Files:
- `valid_user.json` - A valid User aggregate with all fields populated
- `invalid_user.json` - An invalid User (age < 18, unverified email for active user)
- `valid_order.json` - A valid Order with multiple items and correct total
- `invalid_order.json` - An invalid Order (empty items list)
- `valid_product.json` - A valid Product with stock available

**Usage:**
```rust
let harness = DomainModelHarness::new();
let user: User = harness.load_fixture("aggregates", "valid_user")?;
assert!(user.validate_invariants().is_ok());
```

### Commands

**Purpose:** Command payloads for testing command validation and execution.

Files:
- `create_user_valid.json` - Valid CreateUser command
- `create_user_invalid.json` - Invalid CreateUser (bad email, age < 18)
- `place_order_valid.json` - Valid PlaceOrder command with items
- `place_order_invalid.json` - Invalid PlaceOrder (empty items)
- `add_to_cart_valid.json` - Valid AddToCart command
- `add_to_cart_invalid.json` - Invalid AddToCart (quantity = 0)

**Usage:**
```rust
let cmd: Command = harness.load_fixture("commands", "create_user_valid")?;
assert!(cmd.validate().is_ok());
```

### Events

**Purpose:** Domain events for testing event sourcing and state reconstruction.

Files:
- `user_created.json` - UserCreated event
- `order_placed.json` - OrderPlaced event
- `payment_processed.json` - PaymentProcessed event
- `email_verified.json` - EmailVerified event

**Usage:**
```rust
let event: DomainEvent = harness.load_fixture("events", "user_created")?;
user.apply_event(&event);
```

### Value Objects

**Purpose:** Value object examples for testing immutability and validation.

Files:
- `valid_email.json` - A valid Email value object
- `valid_address.json` - A valid Address value object
- `valid_money.json` - A valid Money value object

**Usage:**
```rust
let email: Email = harness.load_fixture("value_objects", "valid_email")?;
assert!(email.is_valid());
```

## Fixture Format

All fixtures are JSON files following the structure of their corresponding domain types:

### Example: User Aggregate

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

### Example: Command

```json
{
  "CreateUser": {
    "email": "jane.smith@example.com",
    "age": 28
  }
}
```

### Example: Event

```json
{
  "UserCreated": {
    "user_id": "user_new_001",
    "email": {
      "value": "new.user@example.com",
      "verified": false
    },
    "age": 25
  }
}
```

## Adding New Fixtures

When adding new fixtures:

1. **Follow naming conventions:**
   - `valid_*.json` for fixtures that should pass validation
   - `invalid_*.json` for fixtures that should fail validation
   - `edge_case_*.json` for boundary conditions

2. **Maintain consistency:**
   - Use consistent IDs across related fixtures (e.g., `user_123` appears in both user and order fixtures)
   - Use realistic data that reflects actual use cases
   - Include all required fields

3. **Document special cases:**
   - Add comments in this README explaining non-obvious fixtures
   - Reference the corresponding test cases

4. **Test both paths:**
   - Always create both valid and invalid variants
   - Cover edge cases (empty strings, zero values, boundaries)

## Testing Patterns

### Pattern 1: Load and Validate
```rust
#[test]
fn test_valid_fixture() {
    let harness = DomainModelHarness::new();
    let user: User = harness.load_fixture("aggregates", "valid_user").unwrap();
    assert!(user.validate_invariants().is_ok());
}
```

### Pattern 2: Load and Expect Failure
```rust
#[test]
fn test_invalid_fixture() {
    let harness = DomainModelHarness::new();
    let user: User = harness.load_fixture("aggregates", "invalid_user").unwrap();
    assert!(user.validate_invariants().is_err());
}
```

### Pattern 3: Load and Transform
```rust
#[test]
fn test_command_execution() {
    let harness = DomainModelHarness::new();
    let cmd: Command = harness.load_fixture("commands", "create_user_valid").unwrap();
    let events = cmd.execute().unwrap();
    assert!(!events.is_empty());
}
```

### Pattern 4: Event Replay
```rust
#[test]
fn test_event_sourcing() {
    let harness = DomainModelHarness::new();
    let mut user = User::default();

    let event: DomainEvent = harness.load_fixture("events", "user_created").unwrap();
    user.apply_event(&event);

    assert_eq!(user.status, UserStatus::Pending);
}
```

## Maintenance

These fixtures are maintained alongside the domain model code. When updating domain model types:

1. Update corresponding fixtures to match new structure
2. Add new fixtures for new domain types
3. Deprecate fixtures for removed types (don't delete immediately)
4. Run fixture validation tests to catch breaking changes

## Related Documentation

- [Domain Model Harness Documentation](../../../docs/TDD_DOMAIN_MODEL_HARNESS.md)
- [DDD Patterns Guide](../../../docs/DDD_PATTERNS.md)
- [Test Data Builders](../../../docs/TEST_BUILDERS.md)

---

**Last Updated:** 2026-01-20
**Maintainer:** ggen-mcp Test Team
