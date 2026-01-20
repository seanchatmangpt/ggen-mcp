//! Example: Integrating NewType wrappers with existing code
//!
//! This example demonstrates how to use the NewType wrapper pattern
//! for type-safe domain primitives in the ggen-mcp codebase.

#![allow(dead_code, unused_imports)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Import our NewType wrappers
use spreadsheet_mcp::domain::value_objects::{
    CellAddress, ForkId, RegionId, SheetName, ValidationError, WorkbookId,
};

// ============================================================================
// Example 1: Type-Safe API Design
// ============================================================================

/// Type-safe function that prevents mixing WorkbookId with ForkId
fn create_fork(workbook_id: WorkbookId) -> Result<ForkId> {
    // Implementation would create a fork and return its ID
    let fork_id_str = format!("fork-{}", uuid::Uuid::new_v4());
    Ok(ForkId::new_unchecked(fork_id_str))
}

/// Type-safe function that only accepts ForkId
fn delete_fork(fork_id: ForkId) -> Result<()> {
    println!("Deleting fork: {}", fork_id);
    Ok(())
}

/// Compiler prevents type confusion
fn example_type_safety() -> Result<()> {
    let workbook = WorkbookId::new("workbook-123".to_string())?;
    let fork = create_fork(workbook.clone())?;

    // This works:
    delete_fork(fork)?;

    // This would be a compile error:
    // delete_fork(workbook)?;  // ❌ Type mismatch!

    Ok(())
}

// ============================================================================
// Example 2: Validation at API Boundaries
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct CreateForkRequest {
    workbook_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateForkResponse {
    fork_id: String,
    workbook_id: String,
}

/// API handler that validates at the boundary
async fn handle_create_fork(req: CreateForkRequest) -> Result<CreateForkResponse> {
    // Validate immediately at the API boundary
    let workbook_id = WorkbookId::new(req.workbook_id)
        .map_err(|e| anyhow::anyhow!("Invalid workbook_id: {}", e))?;

    // Now we can pass type-safe values to internal functions
    let fork_id = create_fork(workbook_id.clone())?;

    // Convert back to raw types for JSON response
    Ok(CreateForkResponse {
        fork_id: fork_id.into_inner(),
        workbook_id: workbook_id.into_inner(),
    })
}

// ============================================================================
// Example 3: Working with Cell Addresses
// ============================================================================

#[derive(Debug)]
struct CellEdit {
    address: CellAddress,
    value: String,
    is_formula: bool,
}

impl CellEdit {
    /// Creates a new cell edit with validated address
    fn new(address: &str, value: String, is_formula: bool) -> Result<Self> {
        let address = CellAddress::parse(address)
            .map_err(|e| anyhow::anyhow!("Invalid cell address: {}", e))?;

        Ok(Self {
            address,
            value,
            is_formula,
        })
    }

    /// Get the cell coordinates
    fn coordinates(&self) -> (u32, u32) {
        (self.address.column(), self.address.row())
    }
}

fn example_cell_edits() -> Result<()> {
    // Valid cell addresses
    let edit1 = CellEdit::new("A1", "100".to_string(), false)?;
    let edit2 = CellEdit::new("B5", "=SUM(A1:A10)".to_string(), true)?;
    let edit3 = CellEdit::new("AA42", "Hello".to_string(), false)?;

    println!("Edit 1: {:?} at {:?}", edit1.value, edit1.coordinates());
    println!("Edit 2: {:?} at {:?}", edit2.value, edit2.coordinates());
    println!("Edit 3: {:?} at {:?}", edit3.value, edit3.coordinates());

    // Invalid cell addresses would fail:
    // let bad = CellEdit::new("A0", "value".to_string(), false)?;   // Row 0 invalid
    // let bad = CellEdit::new("1A", "value".to_string(), false)?;   // Wrong format
    // let bad = CellEdit::new("", "value".to_string(), false)?;     // Empty

    Ok(())
}

// ============================================================================
// Example 4: Sheet Operations with Type Safety
// ============================================================================

struct Sheet {
    name: SheetName,
    cells: HashMap<CellAddress, String>,
}

impl Sheet {
    fn new(name: &str) -> Result<Self> {
        Ok(Self {
            name: SheetName::new(name.to_string())?,
            cells: HashMap::new(),
        })
    }

    fn set_cell(&mut self, address: &str, value: String) -> Result<()> {
        let addr = CellAddress::parse(address)?;
        self.cells.insert(addr, value);
        Ok(())
    }

    fn get_cell(&self, address: &str) -> Result<Option<&String>> {
        let addr = CellAddress::parse(address)?;
        Ok(self.cells.get(&addr))
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }
}

fn example_sheets() -> Result<()> {
    // Create sheets with validated names
    let mut sheet1 = Sheet::new("Q1 Revenue")?;
    let mut sheet2 = Sheet::new("Q2 Revenue")?;

    // These would fail validation:
    // let bad = Sheet::new("Invalid[Sheet]")?;  // Contains '['
    // let bad = Sheet::new("Invalid:Sheet")?;   // Contains ':'
    // let bad = Sheet::new("")?;                 // Empty name

    // Set cells with validated addresses
    sheet1.set_cell("A1", "Revenue".to_string())?;
    sheet1.set_cell("B1", "100000".to_string())?;

    sheet2.set_cell("A1", "Revenue".to_string())?;
    sheet2.set_cell("B1", "150000".to_string())?;

    println!("Sheet '{}' has {} cells", sheet1.name(), sheet1.cells.len());
    println!("Sheet '{}' has {} cells", sheet2.name(), sheet2.cells.len());

    Ok(())
}

// ============================================================================
// Example 5: Region Operations
// ============================================================================

#[derive(Debug)]
struct Region {
    id: RegionId,
    sheet: SheetName,
    bounds: String,
}

impl Region {
    fn new(id: u32, sheet: &str, bounds: String) -> Result<Self> {
        Ok(Self {
            id: RegionId::new(id)?,
            sheet: SheetName::new(sheet.to_string())?,
            bounds,
        })
    }

    fn id(&self) -> RegionId {
        self.id
    }
}

fn example_regions() -> Result<()> {
    // Create regions with validated IDs
    let region1 = Region::new(1, "Data", "A1:D10".to_string())?;
    let region2 = Region::new(2, "Calculations", "A1:Z100".to_string())?;

    // This would fail:
    // let bad = Region::new(0, "Data", "A1:D10".to_string())?;  // ID must be > 0

    println!(
        "Region {} on sheet '{}'",
        region1.id(),
        region1.sheet.as_str()
    );
    println!(
        "Region {} on sheet '{}'",
        region2.id(),
        region2.sheet.as_str()
    );

    // Type safety prevents mixing region IDs with row/col indices
    fn process_region(region_id: RegionId, row: u32, col: u32) {
        println!(
            "Processing region {} at row {}, col {}",
            region_id, row, col
        );
    }

    let region_id = RegionId::new(5)?;
    let row = 10u32;
    let col = 3u32;

    process_region(region_id, row, col); // ✓ OK

    // This would fail to compile:
    // process_region(row, region_id, col);  // ❌ Type error!

    Ok(())
}

// ============================================================================
// Example 6: Serialization and Deserialization
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct WorkbookState {
    workbook_id: WorkbookId,
    fork_id: Option<ForkId>,
    active_sheet: SheetName,
    cursor: CellAddress,
}

fn example_serialization() -> Result<()> {
    let state = WorkbookState {
        workbook_id: WorkbookId::new("wb-123".to_string())?,
        fork_id: Some(ForkId::new("fork-456".to_string())?),
        active_sheet: SheetName::new("Sheet1".to_string())?,
        cursor: CellAddress::parse("B5")?,
    };

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&state)?;
    println!("Serialized state:\n{}", json);

    // Deserialize back
    let deserialized: WorkbookState = serde_json::from_str(&json)?;
    println!("Deserialized workbook_id: {}", deserialized.workbook_id);
    println!("Deserialized cursor: {}", deserialized.cursor);

    Ok(())
}

// ============================================================================
// Example 7: Migration from String-based to NewType
// ============================================================================

mod legacy {
    use anyhow::Result;

    /// Old function using raw strings
    pub fn get_workbook_info(workbook_id: String) -> Result<String> {
        if workbook_id.is_empty() {
            return Err(anyhow::anyhow!("Workbook ID cannot be empty"));
        }
        Ok(format!("Info for workbook: {}", workbook_id))
    }
}

mod modern {
    use super::WorkbookId;
    use anyhow::Result;

    /// New function using NewType
    pub fn get_workbook_info(workbook_id: WorkbookId) -> Result<String> {
        // No need to validate - WorkbookId guarantees validity
        Ok(format!("Info for workbook: {}", workbook_id))
    }
}

/// Wrapper for gradual migration
pub fn get_workbook_info_compat(workbook_id: String) -> Result<String> {
    // Convert and validate
    let id = WorkbookId::new(workbook_id)?;

    // Call the type-safe version
    modern::get_workbook_info(id)
}

fn example_migration() -> Result<()> {
    // Old style (deprecated)
    let info1 = legacy::get_workbook_info("wb-123".to_string())?;
    println!("Legacy: {}", info1);

    // New style (recommended)
    let id = WorkbookId::new("wb-123".to_string())?;
    let info2 = modern::get_workbook_info(id)?;
    println!("Modern: {}", info2);

    // Compatibility wrapper
    let info3 = get_workbook_info_compat("wb-123".to_string())?;
    println!("Compat: {}", info3);

    Ok(())
}

// ============================================================================
// Example 8: Error Handling
// ============================================================================

fn example_error_handling() {
    use ValidationError;

    // WorkbookId validation
    match WorkbookId::new("".to_string()) {
        Ok(id) => println!("Valid ID: {}", id),
        Err(ValidationError::Empty(field)) => {
            eprintln!("Error: {} is empty", field);
        }
        Err(e) => eprintln!("Validation error: {}", e),
    }

    // SheetName validation
    match SheetName::new("Invalid[Sheet]".to_string()) {
        Ok(name) => println!("Valid name: {}", name),
        Err(ValidationError::InvalidCharacter { field, character }) => {
            eprintln!(
                "Error: {} contains invalid character '{}'",
                field, character
            );
        }
        Err(e) => eprintln!("Validation error: {}", e),
    }

    // CellAddress validation
    match CellAddress::parse("A0") {
        Ok(addr) => println!("Valid address: {}", addr),
        Err(ValidationError::OutOfRange {
            field,
            min,
            max,
            actual,
        }) => {
            eprintln!(
                "Error: {} is out of range (min: {}, max: {}, actual: {})",
                field, min, max, actual
            );
        }
        Err(e) => eprintln!("Validation error: {}", e),
    }

    // RegionId validation
    match RegionId::new(0) {
        Ok(id) => println!("Valid region: {}", id),
        Err(ValidationError::Invalid { field, reason }) => {
            eprintln!("Error: {} - {}", field, reason);
        }
        Err(e) => eprintln!("Validation error: {}", e),
    }
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== NewType Integration Examples ===\n");

    println!("1. Type Safety:");
    example_type_safety()?;

    println!("\n2. Cell Edits:");
    example_cell_edits()?;

    println!("\n3. Sheets:");
    example_sheets()?;

    println!("\n4. Regions:");
    example_regions()?;

    println!("\n5. Serialization:");
    example_serialization()?;

    println!("\n6. Migration:");
    example_migration()?;

    println!("\n7. Error Handling:");
    example_error_handling();

    Ok(())
}
