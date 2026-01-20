# NewType Pattern - Quick Reference

## Import

```rust
use crate::domain::value_objects::{
    WorkbookId, ForkId, SheetName, RegionId, CellAddress, ValidationError
};
```

## Quick Usage

### WorkbookId

```rust
// Create
let id = WorkbookId::new("workbook-123".to_string())?;

// Use
println!("{}", id);              // Display
let s: &str = id.as_ref();       // Borrow as &str
let s: String = id.into_inner(); // Consume to String

// Rules
// ✓ 1-1024 characters
// ✗ Empty strings
```

### ForkId

```rust
// Create
let fork = ForkId::new("fork-xyz".to_string())?;

// Use
println!("{}", fork);
let s: &str = fork.as_ref();
let s: String = fork.into_inner();

// Rules
// ✓ 1-256 characters
// ✗ Empty strings
```

### SheetName

```rust
// Create
let sheet = SheetName::new("Q1 Revenue".to_string())?;

// Parse
let sheet = "Sheet1".parse::<SheetName>()?;

// Use
println!("{}", sheet);
let s: &str = sheet.as_ref();
let s: String = sheet.into_inner();

// Rules
// ✓ 1-255 characters
// ✗ Empty strings
// ✗ Characters: [ ] : * ? / \
```

### RegionId

```rust
// Create
let region = RegionId::new(42)?;

// Convert
let region: RegionId = 42u32.try_into()?;

// Use
let value: u32 = region.value();
let index: usize = region.as_usize();

// Rules
// ✓ Positive integers (> 0)
// ✗ Zero
```

### CellAddress

```rust
// Parse from A1 notation
let addr = CellAddress::parse("B5")?;

// Create from indices (1-based)
let addr = CellAddress::from_indices(2, 5)?;  // B5

// Parse trait
let addr: CellAddress = "AA42".parse()?;

// Use
println!("{}", addr);              // "B5"
let s: &str = addr.as_ref();
let col: u32 = addr.column();      // 2 (1-based)
let row: u32 = addr.row();         // 5 (1-based)

// Rules
// ✓ Format: Letters then numbers (e.g., "A1", "AA42")
// ✓ Column: A-XFD (1-16384)
// ✓ Row: 1-1048576
// ✗ Empty, numbers first, row 0
```

## Error Handling

```rust
use ValidationError;

match WorkbookId::new(input) {
    Ok(id) => { /* use id */ },
    Err(ValidationError::Empty(field)) => {
        eprintln!("{} is empty", field);
    },
    Err(ValidationError::TooLong { field, max, actual }) => {
        eprintln!("{} too long: {} > {}", field, actual, max);
    },
    Err(e) => eprintln!("{}", e),
}
```

## Common Patterns

### API Boundary Validation

```rust
async fn handle_request(req: CreateForkRequest) -> Result<Response> {
    // Validate at boundary
    let workbook_id = WorkbookId::new(req.workbook_id)?;

    // Pass type-safe values internally
    let fork_id = service.create_fork(workbook_id).await?;

    // Convert back for response
    Ok(Response { fork_id: fork_id.into_inner() })
}
```

### Struct Fields

```rust
#[derive(Debug, Serialize, Deserialize)]
struct WorkbookState {
    workbook_id: WorkbookId,
    active_sheet: SheetName,
    cursor: CellAddress,
}
```

### Function Signatures

```rust
// Type-safe - compiler enforces correctness
fn create_fork(workbook_id: WorkbookId) -> Result<ForkId>
fn get_sheet(workbook_id: WorkbookId, sheet: SheetName) -> Result<Sheet>
fn edit_cell(addr: CellAddress, value: String) -> Result<()>
```

### Collections

```rust
use std::collections::HashMap;

let mut sheets: HashMap<SheetName, Sheet> = HashMap::new();
sheets.insert(SheetName::new("Sheet1".to_string())?, sheet);

let mut regions: HashMap<RegionId, Region> = HashMap::new();
regions.insert(RegionId::new(1)?, region);
```

## Serialization

```rust
#[derive(Serialize, Deserialize)]
struct Data {
    id: WorkbookId,  // Serializes as string
    addr: CellAddress,  // Serializes as string
}

// JSON
let json = serde_json::to_string(&data)?;
// {"id":"wb123","addr":"A1"}

let data: Data = serde_json::from_str(&json)?;
```

## Cheat Sheet

| Type | Validate | Access | Convert | Max Length | Special |
|------|----------|--------|---------|------------|---------|
| `WorkbookId` | `::new()` | `.as_ref()` | `.into_inner()` | 1024 | - |
| `ForkId` | `::new()` | `.as_ref()` | `.into_inner()` | 256 | - |
| `SheetName` | `::new()` | `.as_ref()` | `.into_inner()` | 255 | No `[]:\*?/\\` |
| `RegionId` | `::new()` | `.value()` | `.into()` | - | Must be > 0 |
| `CellAddress` | `::parse()` | `.as_ref()` | - | - | A1 notation |

## Migration Checklist

- [ ] Import NewTypes from `domain::value_objects`
- [ ] Update function signatures to use NewTypes
- [ ] Add validation at API boundaries (`::new()`, `::parse()`)
- [ ] Update struct fields to use NewTypes
- [ ] Remove manual validation code
- [ ] Update tests to use NewTypes
- [ ] Handle `ValidationError` appropriately

## Tips

1. **Validate Early**: Convert to NewTypes at API boundaries
2. **Use `new_unchecked`**: Only when 100% certain the value is valid
3. **Let It Fail**: Don't catch validation errors unless you can handle them
4. **Type Safety**: Let the compiler help you - don't fight it
5. **Documentation**: Function signatures are now self-documenting

## Examples

Full examples: `/home/user/ggen-mcp/examples/newtype_integration.rs`

Full docs: `/home/user/ggen-mcp/docs/POKA_YOKE_PATTERN.md`
