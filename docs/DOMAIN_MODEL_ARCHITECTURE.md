# Domain Model Architecture

Visual guide to the domain model test harness architecture and relationships.

## System Architecture

```text
┌─────────────────────────────────────────────────────────────────────┐
│                         Test Layer                                  │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │         domain_model_harness_tests.rs                        │   │
│  │  - Integration Tests (30+ tests)                             │   │
│  │  - Workflow Examples                                         │   │
│  │  - Pattern Validation                                        │   │
│  └──────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     Harness Layer                                   │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │         DomainModelHarness                                   │   │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐             │   │
│  │  │  Fixture   │  │ Assertion  │  │   DDD      │             │   │
│  │  │  Loading   │  │  Helpers   │  │  Pattern   │             │   │
│  │  │            │  │            │  │ Validators │             │   │
│  │  └────────────┘  └────────────┘  └────────────┘             │   │
│  └──────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Builder Layer                                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐             │
│  │ UserBuilder  │  │OrderBuilder  │  │ProductBuilder│             │
│  │              │  │              │  │              │             │
│  │ - Fluent API │  │ - Item Mgmt  │  │ - Defaults   │             │
│  │ - Validation │  │ - Auto Total │  │ - No Validate│             │
│  └──────────────┘  └──────────────┘  └──────────────┘             │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Domain Model Layer                               │
│  ┌────────────────────────────────────────────────────────────┐    │
│  │                     Aggregates                             │    │
│  │  ┌──────┐  ┌───────┐  ┌─────────┐  ┌──────┐  ┌─────────┐  │    │
│  │  │ User │  │ Order │  │ Product │  │ Cart │  │ Payment │  │    │
│  │  └──────┘  └───────┘  └─────────┘  └──────┘  └─────────┘  │    │
│  │  ┌──────────┐                                               │    │
│  │  │ Shipment │                                               │    │
│  │  └──────────┘                                               │    │
│  └────────────────────────────────────────────────────────────┘    │
│                                                                     │
│  ┌────────────────────────────────────────────────────────────┐    │
│  │                   Value Objects                            │    │
│  │  ┌───────┐  ┌───────┐  ┌─────────┐  ┌─────────────┐       │    │
│  │  │ Email │  │ Money │  │ Address │  │ PhoneNumber │       │    │
│  │  └───────┘  └───────┘  └─────────┘  └─────────────┘       │    │
│  └────────────────────────────────────────────────────────────┘    │
│                                                                     │
│  ┌────────────────────────────────────────────────────────────┐    │
│  │                      Commands                              │    │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐            │    │
│  │  │CreateUser  │  │PlaceOrder  │  │AddToCart   │            │    │
│  │  └────────────┘  └────────────┘  └────────────┘            │    │
│  │  ┌────────────────┐                                         │    │
│  │  │ProcessPayment  │                                         │    │
│  │  └────────────────┘                                         │    │
│  └────────────────────────────────────────────────────────────┘    │
│                                                                     │
│  ┌────────────────────────────────────────────────────────────┐    │
│  │                       Events                               │    │
│  │  ┌────────────┐  ┌────────────┐  ┌──────────────┐          │    │
│  │  │UserCreated │  │OrderPlaced │  │EmailVerified │          │    │
│  │  └────────────┘  └────────────┘  └──────────────┘          │    │
│  │  ┌──────────────────┐                                       │    │
│  │  │PaymentProcessed  │                                       │    │
│  │  └──────────────────┘                                       │    │
│  └────────────────────────────────────────────────────────────┘    │
│                                                                     │
│  ┌────────────────────────────────────────────────────────────┐    │
│  │                  Domain Services                           │    │
│  │  ┌────────────────────┐  ┌──────────────────────┐          │    │
│  │  │OrderPricingService │  │PaymentProcessingServ.│          │    │
│  │  └────────────────────┘  └──────────────────────┘          │    │
│  │  ┌──────────────────┐                                       │    │
│  │  │ShippingCalculator│                                       │    │
│  │  └──────────────────┘                                       │    │
│  └────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     Fixture Layer                                   │
│  ┌──────────────┐  ┌──────────┐  ┌────────┐  ┌──────────────┐     │
│  │  Aggregates  │  │ Commands │  │ Events │  │Value Objects │     │
│  │  (5 files)   │  │(6 files) │  │(4 files)│  │  (3 files)   │     │
│  └──────────────┘  └──────────┘  └────────┘  └──────────────┘     │
└─────────────────────────────────────────────────────────────────────┘
```

## Aggregate Boundaries

```text
┌─────────────────────────────────────────────────────────┐
│                    User Aggregate                       │
│  ┌─────────────────────────────────────────────────┐    │
│  │ User (Root)                                     │    │
│  │ ├─ id: UserId                                   │    │
│  │ ├─ email: Email (VO)                            │    │
│  │ ├─ age: u8                                      │    │
│  │ ├─ address: Option<Address> (VO)                │    │
│  │ ├─ phone: Option<PhoneNumber> (VO)              │    │
│  │ ├─ status: UserStatus                           │    │
│  │ └─ version: u32                                 │    │
│  │                                                 │    │
│  │ Invariants:                                     │    │
│  │ • Age >= 18                                     │    │
│  │ • Active users must have verified email        │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│                   Order Aggregate                       │
│  ┌─────────────────────────────────────────────────┐    │
│  │ Order (Root)                                    │    │
│  │ ├─ id: OrderId                                  │    │
│  │ ├─ user_id: UserId (Reference)                  │    │
│  │ ├─ items: Vec<OrderItem> (Entities)             │    │
│  │ ├─ total: Money (VO)                            │    │
│  │ ├─ status: OrderStatus                          │    │
│  │ ├─ payment_status: PaymentStatus                │    │
│  │ └─ version: u32                                 │    │
│  │                                                 │    │
│  │ ┌───────────────────────────────────────────┐   │    │
│  │ │ OrderItem (Entity - within boundary)      │   │    │
│  │ │ ├─ product_id: ProductId                  │   │    │
│  │ │ ├─ quantity: u32                          │   │    │
│  │ │ ├─ unit_price: Money (VO)                 │   │    │
│  │ │ └─ subtotal: Money (VO)                   │   │    │
│  │ └───────────────────────────────────────────┘   │    │
│  │                                                 │    │
│  │ Invariants:                                     │    │
│  │ • Must have at least one item                   │    │
│  │ • Total must match sum of item subtotals        │    │
│  │ • Paid orders cannot be cancelled               │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│                  Product Aggregate                      │
│  ┌─────────────────────────────────────────────────┐    │
│  │ Product (Root)                                  │    │
│  │ ├─ id: ProductId                                │    │
│  │ ├─ name: String                                 │    │
│  │ ├─ price: Money (VO)                            │    │
│  │ ├─ stock: u32                                   │    │
│  │ ├─ status: ProductStatus                        │    │
│  │ └─ version: u32                                 │    │
│  │                                                 │    │
│  │ Invariants:                                     │    │
│  │ • Active products must have stock               │    │
│  │ • Price must be positive                        │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

## Command → Event → State Flow

```text
Command Validation                Event Emission              State Update
─────────────────                ───────────────             ─────────────

┌─────────────┐                  ┌─────────────┐            ┌─────────────┐
│ CreateUser  │                  │UserCreated  │            │    User     │
│             │                  │             │            │             │
│ Validate:   │                  │ Contains:   │            │ Apply:      │
│ • Email     │  ─────────────▶  │ • user_id   │  ───────▶  │ • Set id    │
│ • Age >= 18 │     Execute      │ • email     │   Replay   │ • Set email │
│             │                  │ • age       │            │ • Set age   │
└─────────────┘                  └─────────────┘            │ • Pending   │
                                                            │ • version++ │
                                                            └─────────────┘

┌─────────────┐                  ┌─────────────┐            ┌─────────────┐
│ PlaceOrder  │                  │OrderPlaced  │            │   Order     │
│             │                  │             │            │             │
│ Validate:   │                  │ Contains:   │            │ Apply:      │
│ • Has items │  ─────────────▶  │ • order_id  │  ───────▶  │ • Create    │
│ • Products  │     Execute      │ • user_id   │   Replay   │ • Add items │
│   exist     │                  │ • item_count│            │ • Pending   │
└─────────────┘                  └─────────────┘            │ • version++ │
                                                            └─────────────┘

┌─────────────┐                  ┌─────────────┐            ┌─────────────┐
│ProcessPaymt │                  │PaymentProc. │            │  Payment    │
│             │                  │             │            │             │
│ Validate:   │                  │ Contains:   │            │ Apply:      │
│ • Amount>0  │  ─────────────▶  │ • payment_id│  ───────▶  │ • Set id    │
│ • Method OK │     Execute      │ • order_id  │   Replay   │ • Set amount│
│             │                  │ • amount    │            │ • Authorized│
└─────────────┘                  │ • method    │            │ • version++ │
                                 └─────────────┘            └─────────────┘
```

## Value Object Immutability

```text
┌──────────────────────────────────────────────────────────┐
│                 Email Value Object                       │
│                                                          │
│  ┌────────────────┐         ┌────────────────┐          │
│  │ email: Email   │         │ verified_email │          │
│  │ verified=false │  ────▶  │ verified=true  │          │
│  └────────────────┘ verify  └────────────────┘          │
│        │                            ▲                    │
│        │                            │                    │
│        └────────────────────────────┘                    │
│              Original unchanged                          │
│              (Immutability proven)                       │
└──────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────┐
│                 Money Value Object                       │
│                                                          │
│  ┌────────────┐  ┌────────────┐   ┌────────────┐        │
│  │ price      │  │ shipping   │   │ total      │        │
│  │ $29.99     │  │ $5.00      │   │ $34.99     │        │
│  └────────────┘  └────────────┘   └────────────┘        │
│        │              │                   ▲              │
│        └──────────────┴───────add()───────┘              │
│                                                          │
│  Both original values unchanged                          │
│  New value created (Immutability)                        │
└──────────────────────────────────────────────────────────┘
```

## Builder Pattern Flow

```text
┌─────────────────────────────────────────────────────────┐
│                 UserBuilder Flow                        │
│                                                         │
│  UserBuilder::new()                                     │
│       │                                                 │
│       ▼                                                 │
│  .email("user@example.com")  ─────┐                     │
│       │                           │                     │
│       ▼                           │                     │
│  .age(25)                         │  Fluent             │
│       │                           │  Chaining           │
│       ▼                           │                     │
│  .with_address(address)           │                     │
│       │                           │                     │
│       ▼                           │                     │
│  .with_phone(phone)               │                     │
│       │                           │                     │
│       ▼                           │                     │
│  .status(UserStatus::Active) ─────┘                     │
│       │                                                 │
│       ▼                                                 │
│  .build()? ──────────────────────┐                      │
│       │                          │                      │
│       ▼                          ▼                      │
│  Construct User            Validate Invariants          │
│       │                          │                      │
│       │                          │                      │
│       │          OK?             │                      │
│       │    ┌─────┴──────┐        │                      │
│       ▼    ▼            ▼        │                      │
│    Result<User>   Err(DomainError)                      │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## Test Assertion Flow

```text
┌─────────────────────────────────────────────────────────┐
│            Harness Assertion Flow                       │
│                                                         │
│  Test Code                                              │
│  ┌──────────────────────────────────────────┐           │
│  │ harness.assert_invariant_holds(          │           │
│  │     || user.validate_age_requirement(),  │           │
│  │     "minimum_age"                        │           │
│  │ );                                       │           │
│  └──────────────────────────────────────────┘           │
│       │                                                 │
│       ▼                                                 │
│  Execute validation closure                             │
│       │                                                 │
│       ▼                                                 │
│  Check result                                           │
│       │                                                 │
│       ├─────────────┬─────────────┐                     │
│       ▼             ▼             ▼                     │
│    Ok(_)     Err(expected)  Err(unexpected)             │
│       │             │             │                     │
│       ▼             ▼             ▼                     │
│    Pass          Panic         Panic                    │
│                  with           with                    │
│                 message       message                   │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## Fixture Loading Flow

```text
┌─────────────────────────────────────────────────────────┐
│              Fixture Loading Flow                       │
│                                                         │
│  Test Code                                              │
│  ┌──────────────────────────────────────────┐           │
│  │ let harness = DomainModelHarness::new(); │           │
│  │ let user: User =                         │           │
│  │   harness.load_fixture(                  │           │
│  │     "aggregates",                        │           │
│  │     "valid_user"                         │           │
│  │   )?;                                    │           │
│  └──────────────────────────────────────────┘           │
│       │                                                 │
│       ▼                                                 │
│  Build path: fixtures/domain/aggregates/valid_user.json│
│       │                                                 │
│       ▼                                                 │
│  Read file                                              │
│       │                                                 │
│       ▼                                                 │
│  Parse JSON                                             │
│       │                                                 │
│       ▼                                                 │
│  Deserialize to User                                    │
│       │                                                 │
│       ▼                                                 │
│  Return Result<User>                                    │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## Domain Service Interactions

```text
┌─────────────────────────────────────────────────────────┐
│            OrderPricingService Flow                     │
│                                                         │
│  Input: Order with items                                │
│       │                                                 │
│       ▼                                                 │
│  ┌────────────────────────────┐                         │
│  │ OrderPricingService        │                         │
│  │                            │                         │
│  │ calculate_total(items) ────┼────▶ Sum item subtotals│
│  │                            │                         │
│  │ apply_discount(total, %) ──┼────▶ Calculate discount│
│  │                            │                         │
│  │ calculate_tax(subtotal) ───┼────▶ Apply tax rate    │
│  │                            │                         │
│  └────────────────────────────┘                         │
│       │                                                 │
│       ▼                                                 │
│  Output: Money values                                   │
│                                                         │
│  Note: Service is stateless                             │
│        All functions are pure                           │
│        No side effects                                  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## DDD Pattern Validation

```text
┌─────────────────────────────────────────────────────────┐
│            Pattern Validators                           │
│                                                         │
│  ✓ validate_aggregate_has_root()                        │
│    ├─ Checks: Order is aggregate root                   │
│    ├─ Checks: OrderItem is entity within               │
│    └─ Checks: Access through root only                 │
│                                                         │
│  ✓ validate_value_object_immutable()                    │
│    ├─ Checks: No setters on Email                      │
│    ├─ Checks: Changes return new instances             │
│    └─ Checks: Original values unchanged                │
│                                                         │
│  ✓ validate_commands_are_pure()                         │
│    ├─ Checks: Commands validate input                  │
│    ├─ Checks: Commands produce events                  │
│    └─ Checks: No side effects                          │
│                                                         │
│  ✓ validate_events_past_tense()                         │
│    ├─ Checks: UserCreated (not CreateUser)             │
│    ├─ Checks: OrderPlaced (not PlaceOrder)             │
│    └─ Checks: PaymentProcessed (not Process)           │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## Full Test Workflow Example

```text
┌─────────────────────────────────────────────────────────┐
│         Complete User Registration Workflow             │
│                                                         │
│  1. Create Command                                      │
│     ┌──────────────────────────┐                        │
│     │ Command::CreateUser      │                        │
│     │ email: "user@ex.com"     │                        │
│     │ age: 25                  │                        │
│     └──────────────────────────┘                        │
│              │                                          │
│              ▼                                          │
│  2. Validate Command                                    │
│     ┌──────────────────────────┐                        │
│     │ cmd.validate()           │                        │
│     │ ✓ Email format valid     │                        │
│     │ ✓ Age >= 18              │                        │
│     └──────────────────────────┘                        │
│              │                                          │
│              ▼                                          │
│  3. Execute Command                                     │
│     ┌──────────────────────────┐                        │
│     │ cmd.execute()            │                        │
│     │ → Vec<DomainEvent>       │                        │
│     └──────────────────────────┘                        │
│              │                                          │
│              ▼                                          │
│  4. Verify Event Emitted                                │
│     ┌──────────────────────────┐                        │
│     │ harness.assert_event_    │                        │
│     │   emitted(&events,       │                        │
│     │   "UserCreated")         │                        │
│     └──────────────────────────┘                        │
│              │                                          │
│              ▼                                          │
│  5. Build User from Event                               │
│     ┌──────────────────────────┐                        │
│     │ if let UserCreated {..}  │                        │
│     │   user = User::new(..)   │                        │
│     └──────────────────────────┘                        │
│              │                                          │
│              ▼                                          │
│  6. Verify Invariants                                   │
│     ┌──────────────────────────┐                        │
│     │ harness.assert_invariant_│                        │
│     │   holds(                 │                        │
│     │   || user.validate(),    │                        │
│     │   "user_invariants"      │                        │
│     │ )                        │                        │
│     └──────────────────────────┘                        │
│              │                                          │
│              ▼                                          │
│  ✓ Test Complete                                        │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

---

This architecture demonstrates:
- Clear separation of concerns
- DDD pattern compliance
- Immutability where required
- State-based testing approach
- Event sourcing capability
- Business rule enforcement
- Type safety throughout
