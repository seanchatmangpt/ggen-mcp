# Chicago-style TDD Fixture Library

## Overview

The Fixture Library provides a comprehensive test data management system following **Chicago-style TDD** principles, where fixtures represent real, integrated domain objects with actual implementations rather than mocks or stubs.

### Key Features

- **Master Registry**: Centralized fixture management with lazy loading and caching
- **Builder Patterns**: Fluent APIs for constructing domain objects
- **80/20 Principle**: Focused fixture categories covering essential test scenarios
- **Composability**: Combine multiple fixtures into complex test scenarios
- **Versioning**: Track fixture compatibility across test suite evolution
- **Automatic Cleanup**: Isolated test workspaces with automatic teardown

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      FixtureLibrary                             │
│  (Master Registry with Caching & Lazy Loading)                  │
└────────────────┬────────────────────────────────────────────────┘
                 │
      ┌──────────┴──────────┬──────────────┬──────────────┐
      │                     │              │              │
┌─────▼─────┐      ┌────────▼────┐  ┌─────▼─────┐  ┌────▼──────┐
│  Domain   │      │    Config   │  │ Ontology  │  │ Template  │
│ Fixtures  │      │  Fixtures   │  │ Fixtures  │  │  Context  │
└───────────┘      └─────────────┘  └───────────┘  └───────────┘
     │                    │               │              │
     │                    │               │              │
┌────▼──────────┐  ┌──────▼───────┐ ┌───▼──────┐  ┌───▼────────┐
│  Builders:    │  │  Builders:   │ │Builders: │  │ Builders:  │
│ - Aggregate   │  │ - Config     │ │- Ontology│  │ - Context  │
│ - User        │  │              │ │          │  │            │
│ - Order       │  │              │ │          │  │            │
│ - Product     │  │              │ │          │  │            │
│ - Payment     │  │              │ │          │  │            │
└───────────────┘  └──────────────┘ └──────────┘  └────────────┘
```

## Quick Start

### Basic Usage

```rust
use crate::harness::{Fixtures, AggregateBuilder, ConfigBuilder};

#[test]
fn test_user_creation() {
    // Load pre-configured fixtures
    let user = Fixtures::user().minimal();
    assert_eq!(user.name, "User");
    assert_eq!(user.fields.len(), 3);

    // Build custom fixtures
    let custom_user = AggregateBuilder::new("CustomUser")
        .with_id("custom_001")
        .with_field("name", "String", true)
        .with_field("email", "Email", true)
        .build();

    assert_eq!(custom_user.name, "CustomUser");
}

#[test]
fn test_configuration() {
    // Use pre-configured config
    let config = Fixtures::config().production();
    assert!(config.cache_capacity >= 10);

    // Build custom config
    let custom = ConfigBuilder::new()
        .workspace_root("/tmp/custom")
        .cache_capacity(15)
        .with_recalc()
        .build();

    assert_eq!(custom.cache_capacity, 15);
}
```

### AAA Pattern Helper

```rust
use crate::harness::{AAAPattern, Fixtures};

#[test]
fn test_with_aaa_pattern() {
    AAAPattern::new()
        // Arrange
        .arrange(Fixtures::user().minimal())
        // Act
        .act(|user| {
            // Transform the user
            let mut modified = user;
            modified.fields.push(/* new field */);
            modified
        })
        // Assert
        .assert(|result| {
            let user = result.expect("User should exist");
            assert!(user.fields.len() > 3);
        });
}
```

## Complete Fixture Catalog

### Domain Fixtures

Domain fixtures represent aggregates, entities, and value objects following DDD principles.

#### User Aggregate

```rust
// Minimal user with only required fields
let user = Fixtures::user().minimal();
// Fields: id, name, email
// Commands: CreateUser
// Events: UserCreated

// Complete user with all fields
let user = Fixtures::user().complete();
// Fields: id, name, email, phone, address, created_at, updated_at
// Commands: CreateUser, UpdateUserProfile, ChangeUserEmail, DeleteUser
// Events: UserCreated, UserProfileUpdated, UserEmailChanged, UserDeleted
// Invariants: email must be unique, name must not be empty

// Invalid user for error testing
let user = Fixtures::user().invalid();
// Marked as invalid, no fields
```

#### Order Aggregate

```rust
// Empty order without items
let order = Fixtures::order().empty();
// Fields: id, user_id, status, total
// Commands: CreateOrder
// Events: OrderCreated

// Order with N items
let order = Fixtures::order().with_items(3);
// Fields: id, user_id, status, total, items
// Commands: CreateOrder, AddOrderItem, RemoveOrderItem
// Events: OrderCreated, OrderItemAdded, OrderItemRemoved
// Invariants: Each item must have positive quantity

// Cancelled order
let order = Fixtures::order().cancelled();
// Fields: id, user_id, status, total, cancelled_at, cancellation_reason
// Commands: CancelOrder
// Events: OrderCancelled
```

#### Product Aggregate

```rust
// Product in stock
let product = Fixtures::product().in_stock();
// Fields: id, name, price, quantity
// Invariants: quantity > 0

// Product out of stock
let product = Fixtures::product().out_of_stock();
// Fields: id, name, price, quantity
// Invariants: quantity == 0
```

#### Payment Aggregate

```rust
// Pending payment
let payment = Fixtures::payment().pending();
// Status: Pending

// Completed payment
let payment = Fixtures::payment().completed();
// Status: Completed
// Additional fields: completed_at

// Failed payment
let payment = Fixtures::payment().failed();
// Status: Failed
// Additional fields: failed_at, failure_reason
```

### Configuration Fixtures

Configuration fixtures represent valid and invalid server configurations.

```rust
// Minimal configuration with defaults
let config = Fixtures::config().minimal();
// workspace_root: /tmp/minimal
// cache_capacity: 5
// recalc_enabled: false

// Complete configuration with all features
let config = Fixtures::config().complete();
// All features enabled
// cache_capacity: 10
// recalc_enabled: true
// vba_enabled: true
// max_concurrent_recalcs: 4

// Development configuration
let config = Fixtures::config().development();
// Optimized for development
// Extended timeouts
// Reduced cache

// Production configuration
let config = Fixtures::config().production();
// Optimized for production
// Large cache
// Strict timeouts

// Invalid configurations
let config = Fixtures::config().invalid_cache_too_small();
let config = Fixtures::config().invalid_timeout_too_low();
```

### Ontology Fixtures

Ontology fixtures provide RDF/Turtle ontologies for testing.

```rust
// Single aggregate ontology
let ontology = Fixtures::ontology().single_aggregate();
// Contains: User aggregate, CreateUser command, UserCreated event

// Complete domain ontology
let ontology = Fixtures::ontology().complete_domain();
// Contains: User, Order, Product, Payment aggregates
// Value objects: Email, Money, Address
// All commands and events

// MCP tools ontology
let ontology = Fixtures::ontology().mcp_tools();
// Contains: Tool, Resource, Prompt aggregates
// MCP-specific commands and events

// DDD patterns ontology
let ontology = Fixtures::ontology().ddd_patterns();
// Contains: Aggregate, ValueObject, Entity, Command, DomainEvent

// Invalid ontologies for error testing
let ontology = Fixtures::ontology().invalid_missing_type();
let ontology = Fixtures::ontology().invalid_cyclic_hierarchy();

// Convert to Store for SPARQL queries
let store = ontology.store()?;
```

## Builder API Documentation

### AggregateBuilder

Fluent builder for domain aggregates.

```rust
let aggregate = AggregateBuilder::new("EntityName")
    .with_id("entity_001")                          // Set ID
    .with_field("name", "String", true)             // Add required field
    .with_field("optional", "String", false)        // Add optional field
    .with_field_desc("age", "u32", true, "User age") // Add field with description
    .with_command("CreateEntity")                   // Add command
    .with_event("EntityCreated")                    // Add event
    .with_invariant("name must not be empty")       // Add invariant
    .description("Custom entity description")       // Set description
    .tag("custom")                                  // Add tag
    .invalid()                                      // Mark as invalid (for error tests)
    .build();

// Convert to other formats
let ttl = aggregate.to_ttl();                       // Convert to Turtle/TTL
let context = aggregate.to_context();               // Convert to template context
```

**Methods:**
- `new(name)` - Create builder with entity name
- `with_id(id)` - Set aggregate ID (auto-generated if not set)
- `with_field(name, type, required)` - Add a field
- `with_field_desc(name, type, required, desc)` - Add field with description
- `with_command(name)` - Add a command
- `with_event(name)` - Add a domain event
- `with_invariant(rule)` - Add business rule/invariant
- `description(text)` - Set fixture description
- `tag(tag)` - Add a tag for categorization
- `invalid()` - Mark fixture as invalid (for error testing)
- `build()` - Build the AggregateFixture

### ConfigBuilder

Fluent builder for server configurations.

```rust
let config = ConfigBuilder::new()
    .workspace_root("/path/to/workspace")           // Set workspace root
    .cache_capacity(10)                             // Set cache size
    .with_recalc()                                  // Enable recalc
    .with_vba()                                     // Enable VBA
    .max_concurrent_recalcs(4)                      // Set max recalcs
    .tool_timeout_ms(60_000)                        // Set timeout
    .no_timeout()                                   // Disable timeout
    .max_response_bytes(5_000_000)                  // Set max response size
    .description("Custom configuration")            // Set description
    .invalid()                                      // Mark as invalid
    .build();
```

**Methods:**
- `new()` - Create builder with defaults
- `workspace_root(path)` - Set workspace root directory
- `cache_capacity(n)` - Set workbook cache capacity
- `with_recalc()` - Enable recalculation features
- `with_vba()` - Enable VBA introspection
- `max_concurrent_recalcs(n)` - Set max concurrent recalcs
- `tool_timeout_ms(ms)` - Set tool timeout in milliseconds
- `no_timeout()` - Disable timeout
- `max_response_bytes(n)` - Set max response size
- `description(text)` - Set fixture description
- `invalid()` - Mark as invalid
- `build()` - Build the ConfigFixture

### OntologyBuilder

Fluent builder for RDF ontologies.

```rust
let ontology = OntologyBuilder::new()
    .prefix("ex", "http://example.org/")            // Add namespace prefix
    .prefix("ddd", "https://ddd-patterns.dev/")     // Add DDD namespace
    .add_aggregate("User")                          // Add aggregate
    .add_value_object("Email")                      // Add value object
    .add_command("CreateUser")                      // Add command
    .add_event("UserCreated")                       // Add event
    .add_triple("ex:User", "ex:hasField", "ex:name") // Add custom triple
    .description("Custom ontology")                 // Set description
    .invalid()                                      // Mark as invalid
    .build();                                       // Build OntologyFixture

// Or build just the TTL string
let ttl = OntologyBuilder::new()
    .add_aggregate("Product")
    .build_ttl();                                   // Build TTL string
```

**Methods:**
- `new()` - Create builder with default prefixes (ddd, ex, rdf, rdfs)
- `prefix(prefix, namespace)` - Add namespace prefix
- `add_aggregate(name)` - Add aggregate root
- `add_value_object(name)` - Add value object
- `add_command(name)` - Add command
- `add_event(name)` - Add domain event
- `add_triple(subject, predicate, object)` - Add custom RDF triple
- `description(text)` - Set fixture description
- `invalid()` - Mark as invalid
- `build()` - Build OntologyFixture with Store
- `build_ttl()` - Build TTL string only

### TemplateContextBuilder

Fluent builder for template contexts.

```rust
let context = TemplateContextBuilder::new()
    .entity_name("User")                            // Set entity name
    .add_field("name", "String")                    // Add field
    .add_field("email", "Email")                    // Add another field
    .add_import("serde", vec!["Serialize", "Deserialize"]) // Add import
    .add_import("std::fmt", vec!["Display"])        // Add another import
    .add_custom("async_runtime", json!("tokio"))    // Add custom data
    .description("User template context")           // Set description
    .invalid()                                      // Mark as invalid
    .build();

// Convert to JSON for rendering
let json = context.to_json();
```

**Methods:**
- `new()` - Create empty builder
- `entity_name(name)` - Set entity/aggregate name
- `add_field(name, type)` - Add a field with type
- `add_import(module, items)` - Add import statement
- `add_custom(key, value)` - Add custom JSON data
- `description(text)` - Set fixture description
- `invalid()` - Mark as invalid
- `build()` - Build TemplateContextFixture

## Fixture Composition

Combine multiple fixtures into complex test scenarios:

```rust
use crate::harness::{FixtureComposer, Fixtures};

// Compose multiple aggregates into a complete domain ontology
let domain_ontology = FixtureComposer::new()
    .add(Fixtures::user().minimal())
    .add(Fixtures::order().with_items(2))
    .add(Fixtures::product().in_stock())
    .add(Fixtures::payment().pending())
    .build_ontology()?;

// The resulting ontology contains all aggregates, commands, and events
let store = domain_ontology.store()?;

// Access individual fixtures
let composer = FixtureComposer::new()
    .add(Fixtures::user().complete())
    .add(Fixtures::order().empty());

for fixture in composer.fixtures() {
    println!("Fixture: {}", fixture.metadata().name);
}
```

## Test Data Management

### TestWorkspace

Provides isolated temporary directories with automatic cleanup:

```rust
use crate::harness::TestWorkspace;

#[test]
fn test_with_workspace() -> Result<()> {
    // Create isolated workspace (automatically cleaned up when dropped)
    let workspace = TestWorkspace::new()?;

    // Get root path
    let root = workspace.root();

    // Create files
    let file = workspace.create_file("test.txt", "Hello, world!")?;
    assert!(file.exists());

    // Create paths
    let path = workspace.path("subdir/file.txt");

    // Copy fixtures
    let fixture_path = PathBuf::from("tests/fixtures/example.ttl");
    let copied = workspace.copy_fixture(&fixture_path, "ontology.ttl")?;

    Ok(())
} // Workspace automatically cleaned up here
```

**Methods:**
- `new()` - Create new temporary workspace
- `root()` - Get root path
- `path(name)` - Get path relative to root
- `create_file(name, content)` - Create file with content
- `copy_fixture(src, dest)` - Copy fixture file to workspace

### Fixture Library

Centralized registry with caching:

```rust
use crate::harness::{FixtureLibrary, Fixtures};

// Create library
let library = FixtureLibrary::new();

// Register custom fixtures
library.register("custom_user", Box::new(
    Fixtures::user().minimal()
));

// Get fixture (with caching)
let fixture = library.get("custom_user");

// List all fixtures
let names = library.list();

// Get by category
let domain_fixtures = library.by_category(FixtureCategory::Domain);

// Clear cache
library.clear_cache();
```

## Usage Examples

### Example 1: Testing Aggregate Generation

```rust
use crate::harness::{Fixtures, AAAPattern};

#[test]
fn test_user_aggregate_generation() {
    AAAPattern::new()
        // Arrange: Load user aggregate fixture
        .arrange(Fixtures::user().complete())
        // Act: Convert to TTL
        .act(|user| {
            let ttl = user.to_ttl();
            ttl
        })
        // Assert: Validate TTL structure
        .assert(|result| {
            let ttl = result.expect("TTL should be generated");
            assert!(ttl.contains("@prefix ddd:"));
            assert!(ttl.contains("ex:User a ddd:Aggregate"));
            assert!(ttl.contains("ddd:handlesCommand ex:CreateUser"));
            assert!(ttl.contains("ddd:emitsEvent ex:UserCreated"));
        });
}
```

### Example 2: Testing Configuration Validation

```rust
use crate::harness::{Fixtures, ConfigBuilder};

#[test]
fn test_config_validation() {
    // Valid config should pass
    let valid = Fixtures::config().production();
    assert!(valid.metadata.valid);
    assert!(valid.cache_capacity >= 10);

    // Invalid config should be marked invalid
    let invalid = Fixtures::config().invalid_cache_too_small();
    assert!(!invalid.metadata.valid);
    assert_eq!(invalid.cache_capacity, 0);
}

#[test]
fn test_custom_config() {
    let config = ConfigBuilder::new()
        .workspace_root("/tmp/custom")
        .cache_capacity(15)
        .with_recalc()
        .max_concurrent_recalcs(4)
        .tool_timeout_ms(45_000)
        .build();

    assert_eq!(config.workspace_root.to_str().unwrap(), "/tmp/custom");
    assert_eq!(config.cache_capacity, 15);
    assert!(config.recalc_enabled);
    assert_eq!(config.max_concurrent_recalcs, 4);
}
```

### Example 3: Testing Ontology Consistency

```rust
use crate::harness::Fixtures;

#[test]
fn test_ontology_consistency() -> Result<()> {
    // Valid ontology should load into store
    let ontology = Fixtures::ontology().complete_domain();
    let store = ontology.store()?;

    // Should contain expected triples
    assert!(store.len()? > 0);

    // Invalid ontology should fail validation
    let invalid = Fixtures::ontology().invalid_cyclic_hierarchy();
    assert!(!invalid.metadata.valid);

    Ok(())
}
```

### Example 4: Testing Template Rendering

```rust
use crate::harness::{Fixtures, TemplateContextBuilder};

#[test]
fn test_template_context() {
    // Use pre-configured context
    let user = Fixtures::user().minimal();
    let context = user.to_context();

    assert_eq!(context.entity_name, "User");
    assert!(context.fields.contains_key("name"));
    assert!(context.fields.contains_key("email"));

    // Build custom context
    let custom = TemplateContextBuilder::new()
        .entity_name("CustomEntity")
        .add_field("id", "Uuid")
        .add_field("name", "String")
        .add_import("uuid", vec!["Uuid"])
        .add_custom("derive", json!(["Debug", "Clone"]))
        .build();

    let json = custom.to_json();
    assert!(json.is_object());
}
```

### Example 5: Complex Domain Testing

```rust
use crate::harness::{FixtureComposer, Fixtures};

#[test]
fn test_complete_domain() -> Result<()> {
    // Compose a complete e-commerce domain
    let domain = FixtureComposer::new()
        .add(Fixtures::user().complete())
        .add(Fixtures::order().with_items(3))
        .add(Fixtures::product().in_stock())
        .add(Fixtures::payment().completed())
        .build_ontology()?;

    // Verify domain completeness
    let store = domain.store()?;

    // Should have all aggregates
    let ttl = &domain.ttl;
    assert!(ttl.contains("User"));
    assert!(ttl.contains("Order"));
    assert!(ttl.contains("Product"));
    assert!(ttl.contains("Payment"));

    // Should have all commands
    assert!(ttl.contains("CreateUser"));
    assert!(ttl.contains("CreateOrder"));
    assert!(ttl.contains("CreateProduct"));

    Ok(())
}
```

## Adding New Fixtures

### Step 1: Define Fixture Type

```rust
#[derive(Debug, Clone)]
pub struct CustomFixture {
    pub metadata: FixtureMetadata,
    pub custom_field: String,
}

impl Fixture for CustomFixture {
    fn metadata(&self) -> &FixtureMetadata {
        &self.metadata
    }

    fn clone_box(&self) -> Box<dyn Fixture> {
        Box::new(self.clone())
    }
}
```

### Step 2: Create Builder

```rust
pub struct CustomBuilder {
    custom_field: String,
    description: Option<String>,
    valid: bool,
}

impl CustomBuilder {
    pub fn new() -> Self {
        Self {
            custom_field: String::new(),
            description: None,
            valid: true,
        }
    }

    pub fn custom_field(mut self, value: impl Into<String>) -> Self {
        self.custom_field = value.into();
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn invalid(mut self) -> Self {
        self.valid = false;
        self
    }

    pub fn build(self) -> CustomFixture {
        CustomFixture {
            metadata: FixtureMetadata {
                name: "custom".to_string(),
                category: FixtureCategory::Domain,
                version: FixtureVersion::CURRENT,
                description: self.description.unwrap_or_default(),
                valid: self.valid,
                tags: HashSet::new(),
            },
            custom_field: self.custom_field,
        }
    }
}
```

### Step 3: Add to Fixtures Collection

```rust
impl Fixtures {
    pub fn custom() -> CustomFixtures {
        CustomFixtures
    }
}

pub struct CustomFixtures;

impl CustomFixtures {
    pub fn default() -> CustomFixture {
        CustomBuilder::new()
            .custom_field("default value")
            .description("Default custom fixture")
            .build()
    }
}
```

### Step 4: Register in Library

```rust
let library = FixtureLibrary::new();
library.register("custom_default", Box::new(
    Fixtures::custom().default()
));
```

## Testing Patterns

### Pattern 1: AAA (Arrange-Act-Assert)

```rust
#[test]
fn test_with_aaa() {
    AAAPattern::new()
        .arrange(Fixtures::user().minimal())
        .act(|user| {
            // Transform user
            user
        })
        .assert(|result| {
            // Verify result
        });
}
```

### Pattern 2: Given-When-Then

```rust
#[test]
fn test_given_when_then() {
    // Given: A user aggregate
    let user = Fixtures::user().complete();

    // When: Converting to TTL
    let ttl = user.to_ttl();

    // Then: Should contain expected elements
    assert!(ttl.contains("ddd:Aggregate"));
}
```

### Pattern 3: Table-Driven Tests

```rust
#[test]
fn test_all_user_variants() {
    let test_cases = vec![
        ("minimal", Fixtures::user().minimal()),
        ("complete", Fixtures::user().complete()),
        ("invalid", Fixtures::user().invalid()),
    ];

    for (name, fixture) in test_cases {
        println!("Testing {}", name);
        assert_eq!(fixture.name, "User");
    }
}
```

### Pattern 4: Property-Based Testing

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_any_user_has_name(
        name in "[A-Za-z]+",
        email in "[a-z]+@[a-z]+\\.[a-z]+"
    ) {
        let user = AggregateBuilder::new("User")
            .with_field("name", "String", true)
            .with_field("email", "Email", true)
            .build();

        assert!(!user.fields.is_empty());
    }
}
```

## Best Practices

### 1. Use Pre-configured Fixtures for Common Cases

```rust
// Good: Use pre-configured fixture
let user = Fixtures::user().minimal();

// Avoid: Building from scratch for common cases
let user = AggregateBuilder::new("User")
    .with_field("id", "UserId", true)
    .with_field("name", "String", true)
    .with_field("email", "Email", true)
    .build();
```

### 2. Build Custom Fixtures for Specific Tests

```rust
// Good: Custom fixture for specific test case
let special_user = AggregateBuilder::new("User")
    .with_field("special_property", "SpecialType", true)
    .tag("special_case")
    .build();
```

### 3. Use Composition for Complex Scenarios

```rust
// Good: Compose fixtures
let domain = FixtureComposer::new()
    .add(Fixtures::user().minimal())
    .add(Fixtures::order().with_items(2))
    .build_ontology()?;
```

### 4. Mark Invalid Fixtures Explicitly

```rust
// Good: Explicitly mark invalid fixtures
let invalid = AggregateBuilder::new("Invalid")
    .invalid()
    .description("Invalid for testing error handling")
    .build();

assert!(!invalid.metadata.valid);
```

### 5. Use TestWorkspace for File Operations

```rust
// Good: Use isolated workspace
let workspace = TestWorkspace::new()?;
let file = workspace.create_file("test.ttl", ttl_content)?;

// Avoid: Writing to actual file system in tests
std::fs::write("/tmp/test.ttl", ttl_content)?; // Bad!
```

### 6. Cache Expensive Fixtures

```rust
// Good: Use library with caching
let library = FixtureLibrary::new();
library.register("expensive", Box::new(create_expensive_fixture()));
let fixture = library.get("expensive"); // Cached on subsequent calls
```

### 7. Tag Fixtures for Organization

```rust
// Good: Tag fixtures for categorization
let user = AggregateBuilder::new("User")
    .tag("minimal")
    .tag("valid")
    .tag("smoke_test")
    .build();
```

### 8. Version Fixtures for Compatibility

```rust
// Fixtures are automatically versioned
let user = Fixtures::user().minimal();
assert_eq!(user.metadata.version, FixtureVersion::CURRENT);
```

## Performance Considerations

### Lazy Loading

Fixtures are lazily loaded and cached:

```rust
let library = FixtureLibrary::new();

// Not loaded yet
library.register("user", Box::new(Fixtures::user().minimal()));

// Loaded and cached on first access
let fixture1 = library.get("user"); // Loads
let fixture2 = library.get("user"); // Uses cache
```

### Builder Performance

Builders are zero-cost abstractions:

```rust
// This compiles to direct struct construction
let user = AggregateBuilder::new("User")
    .with_field("name", "String", true)
    .build();
```

### Fixture Composition

Composition creates new fixtures without copying:

```rust
// Uses Arc for shared ownership
let composer = FixtureComposer::new()
    .add(expensive_fixture_1)
    .add(expensive_fixture_2);
```

## Troubleshooting

### Issue: Fixture Not Found

```rust
let library = FixtureLibrary::new();
let fixture = library.get("missing"); // Returns None
```

**Solution**: Register the fixture first:

```rust
library.register("my_fixture", Box::new(my_fixture));
```

### Issue: Invalid Fixture Passed Validation

```rust
let invalid = Fixtures::config().invalid_cache_too_small();
// Test expects this to fail validation, but it doesn't
```

**Solution**: Check the `valid` flag in metadata:

```rust
assert!(!invalid.metadata.valid, "Should be marked invalid");
```

### Issue: TestWorkspace Not Cleaned Up

```rust
let workspace = TestWorkspace::new()?;
// Files not being cleaned up
```

**Solution**: Ensure workspace is not leaked:

```rust
{
    let workspace = TestWorkspace::new()?;
    // Use workspace
} // Cleaned up here when dropped
```

### Issue: Ontology Store Creation Fails

```rust
let ontology = Fixtures::ontology().complete_domain();
let store = ontology.store()?; // Fails
```

**Solution**: Check TTL syntax in the fixture:

```rust
println!("TTL: {}", ontology.ttl);
// Verify TTL is valid Turtle syntax
```

## API Reference

### Core Traits

- `Fixture` - Base trait for all fixtures
- `FixtureMetadata` - Metadata about fixture (name, category, version, etc.)
- `FixtureCategory` - Category enum (Domain, Configuration, Ontology, etc.)

### Fixture Types

- `AggregateFixture` - Domain aggregate
- `ConfigFixture` - Server configuration
- `OntologyFixture` - RDF ontology
- `TemplateContextFixture` - Template context
- `SparqlQueryFixture` - SPARQL query

### Builders

- `AggregateBuilder` - Build aggregates
- `ConfigBuilder` - Build configurations
- `OntologyBuilder` - Build ontologies
- `TemplateContextBuilder` - Build template contexts

### Collections

- `Fixtures` - Main fixture accessor
- `FixtureLibrary` - Master registry
- `FixtureComposer` - Compose multiple fixtures

### Utilities

- `TestWorkspace` - Isolated test workspace
- `AAAPattern` - AAA testing pattern helper

## Contributing

When adding new fixtures:

1. Create the fixture type implementing `Fixture` trait
2. Create a builder with fluent API
3. Add pre-configured fixtures to `Fixtures` collection
4. Document in this file with examples
5. Add tests demonstrating usage

## See Also

- [Chicago-style TDD](https://github.com/testdouble/contributing-tests/wiki/Chicago-School-TDD)
- [Fixture Design Patterns](https://xunitpatterns.com/Fixture%20Design%20Patterns.html)
- [Domain-Driven Design](https://martinfowler.com/tags/domain%20driven%20design.html)
