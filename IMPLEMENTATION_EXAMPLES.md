# Poka-Yoke Implementation Examples

This document shows concrete before/after examples of the defensive improvements made.

## Example 1: Safe Collection Access

### Before
```rust
let first_region = regions.first().unwrap();
let workbook_id = workbooks["workbooks"][0]["workbook_id"].as_str().unwrap();
```

### After
```rust
use crate::utils::{safe_first, safe_json_str};

let first_region = safe_first(&regions, "detected regions list")?;
let workbook_id = safe_json_str(&workbooks, "workbook_id", "parsing workbook response")?;
```

**Benefit:** Clear error messages like "Failed to get first element: detected regions list" instead of "index out of bounds" or "called Option::unwrap() on None".

---

## Example 2: Date Epoch Construction

### Before
```rust
let epoch_1904 = NaiveDate::from_ymd_opt(1904, 1, 1).unwrap();
```

### After
```rust
let epoch_1904 = NaiveDate::from_ymd_opt(1904, 1, 1)
    .expect("Valid epoch date 1904-01-01 should always be constructible");
```

**Benefit:** Self-documenting code that explains why the unwrap is safe, making it clear this is a validated constant.

---

## Example 3: Empty String Guard

### Before
```rust
fn header_data_penalty(s: &str) -> f32 {
    if s.is_empty() {
        return 0.0;
    }
    let first_char = s.chars().next().unwrap();  // Could still panic on empty UTF-8
    // ...
}
```

### After
```rust
fn header_data_penalty(s: &str) -> f32 {
    if s.is_empty() {
        return 0.0;
    }
    // Safely get first character - we already checked is_empty()
    let Some(first_char) = s.chars().next() else {
        return 0.0;
    };
    // ...
}
```

**Benefit:** Double-guard against edge cases, explicit handling of the None case even after isEmpty check.

---

## Example 4: Empty Collection Guards

### Before
```rust
pub fn compute_sheet_statistics(sheet: &Worksheet, _sample_rows: usize) -> SheetStats {
    let (max_col, max_row) = sheet.get_highest_column_and_row();
    // Immediately starts processing without checking...
}
```

### After
```rust
/// Compute statistics for a worksheet
/// Returns empty stats if sheet has no data (max_col or max_row is 0)
pub fn compute_sheet_statistics(sheet: &Worksheet, _sample_rows: usize) -> SheetStats {
    let (max_col, max_row) = sheet.get_highest_column_and_row();

    // Guard against empty sheets
    if max_col == 0 || max_row == 0 {
        return SheetStats::default();
    }
    // ... process sheet data
}
```

**Benefit:** Prevents processing invalid data, returns safe defaults for edge cases.

---

## Example 5: Division by Zero Guard

### Before
```rust
let mean = Some(numeric_values.iter().sum::<f64>() / numeric_values.len() as f64);
```

### After
```rust
// Guard against empty numeric_values collection before computing stats
let mean = if numeric_values.is_empty() {
    None
} else {
    Some(numeric_values.iter().sum::<f64>() / numeric_values.len() as f64)
};
```

**Benefit:** Prevents NaN values and division by zero panics.

---

## Example 6: Coordinate Parsing Safety

### Before
```rust
fn coord_abs_flags(coord: &str) -> CoordFlags {
    let bytes = coord.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let leading_dollar = i < len && bytes[i] == b'$';  // Could access empty array
    // ...
}
```

### After
```rust
/// Parse absolute/relative flags from a cell coordinate (e.g., "$A$1", "A1", "$A1", "A$1")
/// Returns CoordFlags indicating which parts are absolute references
fn coord_abs_flags(coord: &str) -> CoordFlags {
    let bytes = coord.as_bytes();
    let len = bytes.len();

    // Guard against empty coordinate
    if len == 0 {
        return CoordFlags {
            abs_col: false,
            abs_row: false,
        };
    }

    let mut i = 0;
    let leading_dollar = i < len && bytes[i] == b'$';
    // ...
}
```

**Benefit:** Explicit handling of empty input, prevents array access errors.

---

## Example 7: Regex Compilation

### Before
```rust
let pattern = Regex::new(r"^ont-[a-z0-9]{10}$").unwrap();
```

### After
```rust
// Regex pattern is static and should always compile successfully
let pattern = Regex::new(r"^ont-[a-z0-9]{10}$")
    .expect("Valid regex pattern should always compile");
```

**Benefit:** Makes it clear this is a static pattern that should never fail, not a dynamic user input.

---

## Example 8: Trim Bounds Function

### Before
```rust
fn trim_bounds_by_cells(
    entries: &[(u32, usize)],
    trim_cells: usize,
    default_start: u32,
    default_end: u32,
) -> (u32, u32) {
    if entries.is_empty() {
        return (default_start, default_end);
    }
    // ... process entries
}
```

### After
```rust
/// Trim bounds by removing sparse cells from edges
/// Returns (start, end) tuple representing trimmed bounds
fn trim_bounds_by_cells(
    entries: &[(u32, usize)],
    trim_cells: usize,
    default_start: u32,
    default_end: u32,
) -> (u32, u32) {
    // Guard against empty entries
    if entries.is_empty() {
        return (default_start, default_end);
    }
    // ... process entries
}
```

**Benefit:** Clear documentation of behavior and purpose, explicit edge case handling.

---

## Example 9: Using New Utilities in Test Code

### Before
```rust
#[test]
fn test_fork_workflow() {
    let workbook_id = workbooks["workbooks"][0]["workbook_id"].as_str().unwrap();
    let fork_id = fork["fork_id"].as_str().unwrap();
}
```

### After
```rust
#[test]
fn test_fork_workflow() {
    let workbook_id = workbooks["workbooks"][0]["workbook_id"]
        .as_str()
        .expect("Test response should contain workbook_id");
    let fork_id = fork["fork_id"]
        .as_str()
        .expect("Fork response should contain fork_id");
}
```

**Benefit:** Test failures show exactly what was expected, making debugging much faster.

---

## Summary of Improvements

| Category | Before | After | Benefit |
|----------|--------|-------|---------|
| Collection Access | `unwrap()` | `safe_first()`, `safe_get()` | Meaningful error messages |
| JSON Extraction | `.as_str().unwrap()` | `safe_json_str()` | Context-aware errors |
| String Operations | `.unwrap()` | `let Some(...) else` | Explicit None handling |
| Division | Direct division | Zero guard | Prevents NaN/panic |
| Parsing | `.unwrap()` | `.expect()` with message | Self-documenting |
| Empty Checks | Missing | Early returns | Prevents invalid processing |

## Key Takeaways

1. **Error messages matter** - "Failed to get first element: detected regions list" is much more helpful than "index out of bounds"

2. **Self-documenting code** - Using `expect()` with a message explains why the unwrap is safe

3. **Guard clauses** - Check for edge cases early and return safe defaults

4. **Utility functions** - Centralized safe unwrapping patterns reduce duplication

5. **Even tests benefit** - Better error messages in tests make debugging faster

These patterns make the codebase more robust, maintainable, and developer-friendly.
