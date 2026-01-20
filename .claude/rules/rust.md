# Rust Development Rules

**Version**: 1.2.0 | Rust 2024 | MCP-safe patterns

## Type Safety Foundation (Jidoka)

### NewTypes Prevent Category Errors
```rust
// ✓ NewTypes: type-safe domain IDs
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct WorkbookId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ForkId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SheetName(pub String);

// Compiler prevents: let id: WorkbookId = fork_id;
```

### Value Objects Over Bare Strings
```rust
// ✗ Don't: bare strings lose domain meaning
fn add_sheet(wb: &mut Workbook, name: String) -> Result<()> { ... }

// ✓ Do: explicit value objects
fn add_sheet(wb: &mut Workbook, name: SheetName) -> Result<()> { ... }
```

## Error Handling (Poka-Yoke)

### Always Add Context
```rust
// ✗ Don't: lose information
operation().map_err(|e| Error::Failed)?;

// ✓ Do: contextual errors
operation().context("Failed to parse cell reference 'A1:B10'")?;
```

### Error Enum (Exhaustiveness)
```rust
#[derive(Debug)]
pub enum Error {
    ValidationFailed { reason: String },
    NotFound { resource: String },
    AlreadyExists { resource: String },
    InvalidState { expected: String },
    IoError { path: String, source: std::io::Error },
}

// Compiler forces: all variants handled in match
match operation() {
    Ok(v) => {...},
    Err(Error::ValidationFailed { reason }) => {...},
    // Compiler error if any variant missing
}
```

## Validation Patterns (Poka-Yoke Guards)

### Boundary Validation (First Line of Defense)
```rust
pub fn validate_non_empty_string(s: &str) -> Result<()> {
    if s.trim().is_empty() {
        return Err(Error::ValidationFailed {
            reason: "String cannot be empty".to_string(),
        });
    }
    Ok(())
}

pub fn validate_numeric_range(n: usize, min: usize, max: usize, name: &str) -> Result<()> {
    if n < min || n > max {
        return Err(Error::ValidationFailed {
            reason: format!("{} must be between {} and {}", name, min, max),
        });
    }
    Ok(())
}

pub fn validate_sheet_name(name: &str) -> Result<SheetName> {
    validate_non_empty_string(name)?;
    if name.len() > 31 {
        return Err(Error::ValidationFailed {
            reason: "Sheet name exceeds 31 characters".to_string(),
        });
    }
    Ok(SheetName(name.to_string()))
}
```

### Path Safety (Prevent Traversal Attacks)
```rust
pub fn validate_path_safe(path: &str) -> Result<()> {
    if path.contains("../") || path.contains("..\\") {
        return Err(Error::ValidationFailed {
            reason: "Path traversal not allowed".to_string(),
        });
    }
    Ok(())
}
```

## Async/Await (Tokio)

### Never Block in Async Context
```rust
// ✗ Don't: blocking operation in async
#[tokio::main]
async fn main() {
    std::thread::sleep(Duration::from_secs(1));  // WRONG
}

// ✓ Do: async sleep
#[tokio::main]
async fn main() {
    tokio::time::sleep(Duration::from_secs(1)).await;
}
```

### Timeout Safety (Andon Cord)
```rust
use tokio::time::timeout;

let result = timeout(
    Duration::from_secs(30),
    long_operation()
).await?;

match result {
    Ok(value) => {...},
    Err(_) => Err(Error::Timeout { operation: "sync" }),  // Fail-fast
}
```

## Generated Code Patterns

### Marker Trait (Never Edit Generated)
```rust
/// Auto-generated from ontology/mcp-domain.ttl
/// DO NOT EDIT. Run: cargo make sync
/// 
/// [SPARQL query: queries/workbook_operations.rq]
/// [Tera template: templates/workbook.rs.tera]
pub mod generated {
    // Generated code here
    // Zero TODOs. Compilation required to proceed.
}
```

### Quality Gates
```rust
// Generated code MUST:
// 1. Compile without warnings
// 2. Have zero TODOs
// 3. Implement validate() for all public types
// 4. Use Result<T> for fallible operations
// 5. Add error context to all Err()
```

## Testing Patterns

### Test Module Isolation
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_sheet_name() {
        assert!(validate_sheet_name("Sheet1").is_ok());
        assert!(validate_sheet_name("").is_err());
        assert!(validate_sheet_name(&"x".repeat(32)).is_err());
    }
}
```

### Property-Based Testing
```rust
#[test]
fn sheet_name_length_property() {
    for len in [1, 10, 31] {
        let name = "x".repeat(len);
        assert!(validate_sheet_name(&name).is_ok());
    }
    assert!(validate_sheet_name(&"x".repeat(32)).is_err());
}
```

## Code Formatting

### cargo fmt (Non-negotiable)
```bash
cargo fmt  # ALWAYS before commit
```

### clippy Lints (Security)
```bash
cargo clippy -- -D warnings
# Treat warnings as errors. Fail-fast principle.
```

## Dependency Management

### Minimal, Vendored Dependencies
```toml
# Preferred: use std when possible
[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
oxigraph = "0.4"  # SPARQL/RDF engine
tera = "1.19"     # Template engine
rmcp = "0.11"     # MCP client
```

### Security Audit
```bash
cargo audit  # Check for known vulnerabilities
```

## Project Layout

```
src/
├── generated/      # ← Never edit (regenerate from ontology)
├── validation/     # ← Input guards, poka-yoke
├── domain/        # ← NewTypes, value objects
├── ontology/      # ← RDF/SPARQL engine
├── lib.rs         # ← Public API
└── error.rs       # ← Typed errors

tests/
└── integration/   # ← Real implementations, no mocks
```

---

**Rust principles: Type safety > runtime validation. Compiler == Jidoka. Result<T> handles all branches.**
