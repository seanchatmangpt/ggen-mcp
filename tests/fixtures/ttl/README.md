# TTL Test Fixtures

This directory contains Turtle (TTL) ontology test fixtures for the TDD test harness.

## Directory Structure

```
ttl/
├── valid/              # Valid ontologies that should parse and validate
└── invalid/            # Invalid ontologies for testing error detection
```

## Valid Fixtures

### `user_aggregate.ttl`

**Purpose**: Complete DDD user aggregate with full lifecycle

**Contains**:
- Aggregate Root: `User`
- Value Objects: `UserId`, `Email`, `Username`, `UserProfile`
- Commands: `CreateUser`, `UpdateUserProfile`, `ChangeEmail`, `DeleteUser`
- Events: `UserCreated`, `UserProfileUpdated`, `EmailChanged`, `UserDeleted`
- Repository: `UserRepository`
- Invariants: Email validation, username length check
- SHACL Shapes: Property cardinality constraints

**Test Coverage**:
- ✅ Complete aggregate structure
- ✅ Value object validation patterns
- ✅ Email regex validation
- ✅ Username pattern matching
- ✅ Command/Event pairs
- ✅ Repository-aggregate linking

**Example Usage**:
```rust
let harness = OntologyTestHarness::parse_from_file(
    "tests/fixtures/ttl/valid/user_aggregate.ttl"
)?;
harness.assert_aggregate_structure("user:User");
```

### `order_aggregate.ttl`

**Purpose**: E-commerce order aggregate with entities and business rules

**Contains**:
- Aggregate Root: `Order`
- Entities: `LineItem` (within aggregate)
- Value Objects: `OrderId`, `Money`, `Quantity`, `OrderStatus`
- Commands: `PlaceOrder`, `AddLineItem`, `ConfirmOrder`, `CancelOrder`
- Events: `OrderPlaced`, `OrderConfirmed`, `OrderCancelled`
- Repository: `OrderRepository`
- Business Invariants: Order must have items, total must match line items
- Status Enum: Pending, Confirmed, Shipped, Delivered, Cancelled

**Test Coverage**:
- ✅ Entities within aggregates
- ✅ Business rule invariants
- ✅ Money value object pattern
- ✅ Quantity validation
- ✅ Order lifecycle states
- ✅ Multi-entity aggregate

**Example Usage**:
```rust
let harness = OntologyTestHarness::parse_from_file(
    "tests/fixtures/ttl/valid/order_aggregate.ttl"
)?;
harness.assert_class_is_aggregate_root("order:Order");
harness.assert_class_defined("order:LineItem");
```

### `mcp_tools.ttl`

**Purpose**: MCP tool, resource, and prompt definitions

**Contains**:
- Tools: `read_file`, `write_file`, `list_directory`, `search_files`
- Resources: `FileResource`, `DirectoryResource`
- Prompts: `code_review` with arguments
- JSON Schemas for tool inputs
- SHACL validation for tool structure

**Test Coverage**:
- ✅ MCP tool definitions
- ✅ Tool input schemas
- ✅ Resource URIs
- ✅ Prompt arguments
- ✅ Tool naming conventions

**Example Usage**:
```rust
let harness = OntologyTestHarness::parse_from_file(
    "tests/fixtures/ttl/valid/mcp_tools.ttl"
)?;
let tools = harness.query("SELECT ?tool WHERE { ?tool a mcp:Tool }")?;
```

### `complete_domain.ttl`

**Purpose**: Comprehensive bounded context with multiple aggregates

**Contains**:
- 3 Aggregates: `Customer`, `Product`, `Order`
- Multiple Value Objects per aggregate
- Domain Services: `OrderingService`, `PricingService`
- Factories: `OrderFactory`, `ProductFactory`
- Policies: `OrderConfirmationPolicy`
- Queries: `GetCustomerOrdersQuery`, `SearchProductsQuery`
- Handlers: `PlaceOrderHandler`, `RegisterCustomerHandler`
- Cross-aggregate references

**Test Coverage**:
- ✅ Multiple aggregate boundaries
- ✅ Domain services
- ✅ Factories
- ✅ Policies (saga pattern)
- ✅ CQRS queries
- ✅ Command handlers
- ✅ Cross-aggregate relationships

**Example Usage**:
```rust
let harness = OntologyTestHarness::parse_from_file(
    "tests/fixtures/ttl/valid/complete_domain.ttl"
)?;
let aggregates = harness.get_aggregate_roots()?;
assert_eq!(aggregates.len(), 3);
```

## Invalid Fixtures

### `syntax_error.ttl`

**Purpose**: Test TTL parser error handling

**Contains**:
- Unclosed string literals
- Missing statement terminators (dots)
- Invalid prefix usage
- Malformed triples
- Invalid URIs

**Expected Behavior**: Should fail to parse with clear error message

**Example Usage**:
```rust
let result = OntologyTestHarness::parse_from_file(
    "tests/fixtures/ttl/invalid/syntax_error.ttl"
);
assert!(result.is_err(), "Should fail to parse");
```

### `missing_properties.ttl`

**Purpose**: Test DDD pattern compliance validation

**Contains**:
- Aggregate without `ddd:hasProperty` - INVALID
- Value Object without properties - INVALID
- Command without properties - INVALID
- Repository without `ddd:forAggregate` - INVALID
- Invariant without `ddd:check` - INVALID

**Expected Behavior**: Should parse but fail validation

**Example Usage**:
```rust
let harness = OntologyTestHarness::parse_from_file(
    "tests/fixtures/ttl/invalid/missing_properties.ttl"
)?;
let result = harness.validate();
assert!(!result.is_valid(), "Should detect missing properties");
```

### `circular_dependencies.ttl`

**Purpose**: Test cyclic hierarchy detection

**Contains**:
- Class hierarchy cycle: A → B → C → A
- Self-referential class: `SelfReferential rdfs:subClassOf SelfReferential`
- Circular property domain/range

**Expected Behavior**: Should detect and report cycles

**Example Usage**:
```rust
let harness = OntologyTestHarness::parse_from_file(
    "tests/fixtures/ttl/invalid/circular_dependencies.ttl"
)?;
let report = harness.validate_consistency();
assert!(!report.valid, "Should detect cycle");
assert!(report.errors.iter().any(|e| e.contains("cycle")));
```

### `broken_references.ttl`

**Purpose**: Test undefined reference detection

**Contains**:
- `rdfs:subClassOf` to undefined class
- Property with undefined domain/range
- Repository for non-existent aggregate
- Instance of undefined class
- Property reference to undefined property

**Expected Behavior**: Should detect broken references (warnings or errors)

**Example Usage**:
```rust
let harness = OntologyTestHarness::parse_from_file(
    "tests/fixtures/ttl/invalid/broken_references.ttl"
)?;
let result = harness.validate();
let has_issues = !result.errors().is_empty() || !result.warnings().is_empty();
assert!(has_issues, "Should detect broken references");
```

### `type_mismatches.ttl`

**Purpose**: Test SHACL constraint validation

**Contains**:
- Missing required property (`sh:minCount` violation)
- Multiple values where one expected (`sh:maxCount` violation)
- Pattern mismatch (email format)
- Datatype mismatch (string instead of integer)
- Range violation (value exceeds `sh:maxInclusive`)

**Expected Behavior**: Should detect SHACL constraint violations

**Example Usage**:
```rust
let harness = OntologyTestHarness::parse_from_file(
    "tests/fixtures/ttl/invalid/type_mismatches.ttl"
)?;
let result = harness.validate();
// Note: SHACL validation implementation may vary
```

## Fixture Usage Guidelines

### When to Use Which Fixture

**Use `user_aggregate.ttl` when testing**:
- Basic DDD patterns
- Value object validation
- Simple aggregate structure
- String validation patterns

**Use `order_aggregate.ttl` when testing**:
- Complex aggregates with entities
- Business rule invariants
- Numeric value objects (Money, Quantity)
- State enumerations

**Use `mcp_tools.ttl` when testing**:
- MCP-specific structures
- Tool definitions
- Resource URIs
- JSON schema integration

**Use `complete_domain.ttl` when testing**:
- Multiple aggregates
- Cross-aggregate communication
- Domain services
- Full bounded context

**Use invalid fixtures when testing**:
- Error handling
- Validation rules
- Parser robustness
- Error messages

### Creating New Fixtures

When creating a new fixture:

1. **Choose appropriate directory**:
   - `valid/` if it should parse and validate
   - `invalid/` if it tests error detection

2. **Document in this README**:
   - Purpose
   - What it contains
   - Test coverage
   - Example usage

3. **Follow naming conventions**:
   - Use lowercase with underscores
   - Descriptive names (e.g., `payment_aggregate.ttl`)
   - Suffix indicates purpose (e.g., `*_error.ttl`)

4. **Add comprehensive comments**:
   - Header explaining purpose
   - Comments for each section
   - For invalid fixtures, mark what's wrong

5. **Create corresponding test**:
   - Add test in `turtle_harness_integration_tests.rs`
   - Use fixture to verify behavior

## Maintenance

### Updating Fixtures

When updating fixtures:

1. Update the fixture file
2. Update this README if structure changes
3. Run tests to ensure no breakage: `cargo test turtle`
4. Update documentation if behavior changes

### Adding Test Coverage

To expand test coverage:

1. Identify untested DDD patterns
2. Create fixture demonstrating pattern
3. Document in this README
4. Write integration test
5. Update main harness documentation

## Quick Reference

| Fixture | Lines | Purpose | Key Features |
|---------|-------|---------|--------------|
| `user_aggregate.ttl` | 200 | User management | Email validation, profiles |
| `order_aggregate.ttl` | 150 | E-commerce | Money, quantities, states |
| `mcp_tools.ttl` | 100 | MCP tools | Tools, resources, prompts |
| `complete_domain.ttl` | 250 | Full context | 3 aggregates, services, policies |
| `syntax_error.ttl` | 30 | Parser errors | Malformed TTL |
| `missing_properties.ttl` | 40 | DDD violations | Missing required elements |
| `circular_dependencies.ttl` | 40 | Cycle detection | Class hierarchy cycles |
| `broken_references.ttl` | 30 | Reference errors | Undefined classes |
| `type_mismatches.ttl` | 50 | SHACL violations | Constraint violations |

## Running Tests with Fixtures

```bash
# Test all valid fixtures
cargo test test_user_aggregate_is_valid
cargo test test_order_aggregate_is_valid
cargo test test_mcp_tools_structure

# Test all invalid fixtures
cargo test test_syntax_error_detection
cargo test test_missing_properties_detection
cargo test test_circular_dependency_detection
cargo test test_broken_references_detection
cargo test test_type_mismatch_detection

# Run all fixture tests
cargo test --test turtle_harness_integration_tests
```
