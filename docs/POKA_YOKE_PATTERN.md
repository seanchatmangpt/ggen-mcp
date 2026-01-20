# Poka-Yoke Pattern: NewType Wrappers for Type Safety

## Overview

This document describes the implementation of the **NewType wrapper pattern** (also known as Poka-Yoke) in the ggen-mcp codebase. This pattern prevents type confusion bugs by creating distinct types for semantically different domain primitives.

## What is Poka-Yoke?

Poka-Yoke (ポカヨケ, Japanese for "mistake-proofing") is a lean manufacturing technique that prevents errors by making it impossible to make mistakes. In software, the NewType pattern is a form of Poka-Yoke that uses the type system to prevent bugs at compile time.

### Without NewType Pattern (Unsafe)

```rust
// Everything is just a String - easy to mix up!
fn create_fork(workbook_id: String) -> String { /* ... */ }
fn delete_fork(fork_id: String) { /* ... */ }

// Bug: Easy to accidentally swap arguments!
let workbook = "wb123".to_string();
let fork = create_fork(workbook);
delete_fork(workbook);  // ❌ BUG! Should be fork, not workbook
                        // ✗ Compiles without error!
```

### With NewType Pattern (Safe)

```rust
// Each type is distinct - compiler enforces correctness!
fn create_fork(workbook_id: WorkbookId) -> ForkId { /* ... */ }
fn delete_fork(fork_id: ForkId) { /* ... */ }

// Compiler prevents bugs:
let workbook = WorkbookId::new("wb123".to_string())?;
let fork = create_fork(workbook.clone());
delete_fork(workbook);  // ❌ COMPILE ERROR!
                        // ✓ Type mismatch caught at compile time!
```

## Implemented NewTypes

### 1. WorkbookId

**Purpose**: Unique identifier for workbooks
**Prevents mixing with**: ForkId, generic strings

```rust
use crate::domain::value_objects::{WorkbookId, ValidationError};

// Create with validation
let id = WorkbookId::new("workbook-abc123".to_string())?;

// Access the value
assert_eq!(id.as_str(), "workbook-abc123");

// Validation rules:
// - Must not be empty
// - Maximum length: 1024 characters

// Fails validation
assert!(WorkbookId::new("".to_string()).is_err());
assert!(WorkbookId::new("a".repeat(2000)).is_err());
```

### 2. ForkId

**Purpose**: Unique identifier for workbook forks
**Prevents mixing with**: WorkbookId, generic strings

```rust
use crate::domain::value_objects::{ForkId, ValidationError};

// Create with validation
let fork_id = ForkId::new("fork-xyz789".to_string())?;

// Access the value
assert_eq!(fork_id.as_str(), "fork-xyz789");

// Validation rules:
// - Must not be empty
// - Maximum length: 256 characters

// Type safety in action
fn process_fork(fork_id: ForkId) { /* ... */ }
let workbook_id = WorkbookId::new("wb1".to_string())?;
// process_fork(workbook_id); // ❌ Compile error!
```

### 3. SheetName

**Purpose**: Name of a worksheet in a workbook
**Prevents mixing with**: Generic strings, file paths

```rust
use crate::domain::value_objects::{SheetName, ValidationError};

// Create with validation
let sheet = SheetName::new("Q1 Revenue".to_string())?;

// Access the value
assert_eq!(sheet.as_str(), "Q1 Revenue");

// Validation rules:
// - Must not be empty
// - Maximum length: 255 characters
// - Cannot contain: [ ] : * ? / \

// Fails validation
assert!(SheetName::new("Invalid[Sheet]".to_string()).is_err());
assert!(SheetName::new("Invalid:Sheet".to_string()).is_err());
assert!(SheetName::new("Invalid*Sheet".to_string()).is_err());
```

### 4. RegionId

**Purpose**: Identifier for a region within a worksheet
**Prevents mixing with**: Row/column indices, generic integers

```rust
use crate::domain::value_objects::{RegionId, ValidationError};

// Create with validation
let region = RegionId::new(42)?;

// Access the value
assert_eq!(region.value(), 42);

// Validation rules:
// - Must be positive (> 0)

// Type safety example
fn analyze_region(region_id: RegionId, row: u32, col: u32) { /* ... */ }

let region = RegionId::new(5)?;
let row = 10u32;
let col = 3u32;

analyze_region(region, row, col); // ✓ OK
// analyze_region(row, region, col); // ❌ Compile error!
```

### 5. CellAddress

**Purpose**: Address of a cell in A1 notation
**Prevents**: Invalid cell references

```rust
use crate::domain::value_objects::{CellAddress, ValidationError};

// Parse from A1 notation
let addr = CellAddress::parse("B5")?;
assert_eq!(addr.as_str(), "B5");
assert_eq!(addr.column(), 2);  // B = column 2
assert_eq!(addr.row(), 5);

// Create from indices
let addr = CellAddress::from_indices(26, 99)?; // Z99
assert_eq!(addr.as_str(), "Z99");

// Validation rules:
// - Must be valid A1 notation
// - Column: A-XFD (1-16384)
// - Row: 1-1048576

// Fails validation
assert!(CellAddress::parse("").is_err());
assert!(CellAddress::parse("1A").is_err());  // Row before column
assert!(CellAddress::parse("A0").is_err());  // Row must be >= 1
```

## Serialization Support

All NewType wrappers support `serde` serialization with the `#[serde(transparent)]` attribute, meaning they serialize as their underlying type:

```rust
use serde_json;

let workbook_id = WorkbookId::new("wb123".to_string())?;

// Serializes as a plain string
let json = serde_json::to_string(&workbook_id)?;
assert_eq!(json, r#""wb123""#);

// Deserializes back to WorkbookId
let deserialized: WorkbookId = serde_json::from_str(&json)?;
assert_eq!(deserialized, workbook_id);
```

## Conversion Traits

### Display

All types implement `Display` for easy string conversion:

```rust
let id = WorkbookId::new("wb123".to_string())?;
println!("Workbook ID: {}", id);  // "Workbook ID: wb123"
```

### From / Into

Convert to underlying types:

```rust
let id = WorkbookId::new("wb123".to_string())?;
let s: String = id.into();  // Consumes the wrapper
assert_eq!(s, "wb123");
```

### AsRef

Borrow the underlying value:

```rust
let id = WorkbookId::new("wb123".to_string())?;
let s: &str = id.as_ref();  // Non-consuming borrow
assert_eq!(s, "wb123");
```

### FromStr (for applicable types)

Parse from strings:

```rust
use std::str::FromStr;

let sheet = SheetName::from_str("Sheet1")?;
assert_eq!(sheet.as_str(), "Sheet1");

let addr = CellAddress::from_str("A1")?;
assert_eq!(addr.column(), 1);
assert_eq!(addr.row(), 1);
```

## Error Handling

All validation errors are represented by the `ValidationError` enum:

```rust
pub enum ValidationError {
    Empty(&'static str),
    TooLong { field: &'static str, max: usize, actual: usize },
    InvalidCharacter { field: &'static str, character: char },
    Invalid { field: &'static str, reason: &'static str },
    OutOfRange { field: &'static str, min: u32, max: u32, actual: u32 },
}
```

Example error handling:

```rust
match WorkbookId::new(input) {
    Ok(id) => println!("Valid ID: {}", id),
    Err(ValidationError::Empty(_)) => eprintln!("ID cannot be empty"),
    Err(ValidationError::TooLong { max, actual, .. }) => {
        eprintln!("ID too long: {} chars (max: {})", actual, max)
    }
    Err(e) => eprintln!("Validation error: {}", e),
}
```

## Migration Guide

### Migrating from String-based IDs

**Before:**
```rust
fn get_workbook(id: String) -> Result<Workbook> {
    if id.is_empty() {
        return Err(anyhow!("ID cannot be empty"));
    }
    // ...
}
```

**After:**
```rust
fn get_workbook(id: WorkbookId) -> Result<Workbook> {
    // Validation already done by WorkbookId::new()
    // Just use id.as_str() to access the string
    // ...
}

// Call site:
let id = WorkbookId::new(input)?; // Validation happens here
let workbook = get_workbook(id)?;
```

### Gradual Migration Strategy

1. **Start with new code**: Use NewTypes for all new functions and APIs
2. **Wrapper functions**: Create compatibility wrappers for existing code
3. **Incremental updates**: Update existing functions one at a time
4. **Deprecation**: Mark old string-based functions as deprecated

Example compatibility wrapper:

```rust
// New type-safe function
pub fn create_fork_safe(workbook_id: WorkbookId) -> Result<ForkId> {
    // Implementation
}

// Compatibility wrapper (deprecated)
#[deprecated(note = "Use create_fork_safe with WorkbookId instead")]
pub fn create_fork(workbook_id: String) -> Result<String> {
    let id = WorkbookId::new(workbook_id)?;
    let fork_id = create_fork_safe(id)?;
    Ok(fork_id.into_inner())
}
```

## Benefits

### 1. Compile-Time Safety

The compiler prevents mixing incompatible types:

```rust
fn process(workbook: WorkbookId, fork: ForkId) { /* ... */ }

let wb = WorkbookId::new("wb1".to_string())?;
let fk = ForkId::new("fork1".to_string())?;

process(wb, fk);     // ✓ OK
// process(fk, wb);  // ❌ Compile error!
```

### 2. Self-Documenting Code

Function signatures are more explicit:

```rust
// Before: What do these strings represent?
fn merge(a: String, b: String) -> String { /* ... */ }

// After: Crystal clear!
fn merge_forks(fork1: ForkId, fork2: ForkId) -> ForkId { /* ... */ }
```

### 3. Centralized Validation

Validation logic lives in one place:

```rust
// Validation happens once at construction
let id = WorkbookId::new(input)?;

// All subsequent uses are guaranteed valid
store.save(id.clone());
cache.insert(id.clone());
log_access(id);
// No need to re-validate!
```

### 4. Zero Runtime Cost

NewTypes are zero-cost abstractions - they compile to the same code as raw types:

```rust
// These have identical runtime performance:
let s1: String = "test".to_string();
let s2: WorkbookId = WorkbookId::new_unchecked("test".to_string());
```

## Best Practices

### 1. Validate at Boundaries

Convert to NewTypes as early as possible:

```rust
// Good: Validate at API boundary
async fn handle_request(req: CreateForkRequest) -> Result<Response> {
    let workbook_id = WorkbookId::new(req.workbook_id)?;  // Validate immediately
    let fork_id = service.create_fork(workbook_id).await?;
    Ok(Response { fork_id: fork_id.into_inner() })
}

// Bad: Pass raw strings internally
async fn handle_request(req: CreateForkRequest) -> Result<Response> {
    let fork_id = service.create_fork(req.workbook_id).await?;  // Validation deep inside
    Ok(Response { fork_id })
}
```

### 2. Use `new_unchecked` Sparingly

Only use `new_unchecked` when you're absolutely certain the value is valid:

```rust
// Good: When loading from trusted database
let id = WorkbookId::new_unchecked(db_record.workbook_id);

// Bad: When receiving user input
let id = WorkbookId::new_unchecked(user_input);  // ❌ NO! Use new() instead
```

### 3. Prefer Owned Types in APIs

Use owned types in public APIs for simplicity:

```rust
// Good: Simple and clear
pub fn get_workbook(id: WorkbookId) -> Result<Workbook> { /* ... */ }

// Acceptable: For performance-critical code
pub fn get_workbook_ref(id: &WorkbookId) -> Result<&Workbook> { /* ... */ }
```

### 4. Provide Convenience Methods

Add helper methods for common operations:

```rust
impl WorkbookId {
    /// Returns a short display version (first 8 chars)
    pub fn short(&self) -> &str {
        &self.0[..self.0.len().min(8)]
    }

    /// Checks if this is a special system workbook
    pub fn is_system(&self) -> bool {
        self.0.starts_with("sys_")
    }
}
```

## Testing

All NewType wrappers include comprehensive tests:

```bash
# Run all value object tests
cargo test domain::value_objects

# Run specific test
cargo test domain::value_objects::test_workbook_id_validation
```

Example test:

```rust
#[test]
fn test_type_safety() {
    let workbook_id = WorkbookId::new("wb1".to_string()).unwrap();
    let fork_id = ForkId::new("fork1".to_string()).unwrap();

    // These are different types
    assert_ne!(
        std::any::TypeId::of::<WorkbookId>(),
        std::any::TypeId::of::<ForkId>()
    );

    // But we can compare their string values if needed
    assert_ne!(workbook_id.as_str(), fork_id.as_str());
}
```

## Related Patterns

- **Phantom Types**: For encoding state in types (e.g., `File<Open>` vs `File<Closed>`)
- **Type State Pattern**: For enforcing state machine invariants
- **Builder Pattern**: For constructing complex validated objects

## References

- [Rust Design Patterns: NewType](https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html)
- [Parse, don't validate](https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/)
- [Making Illegal States Unrepresentable](https://ybogomolov.me/making-illegal-states-unrepresentable)
