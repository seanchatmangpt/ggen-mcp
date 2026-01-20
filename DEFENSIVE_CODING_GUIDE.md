# Defensive Coding Guide (Poka-Yoke Implementation)

This guide documents the defensive programming patterns (poka-yoke) implemented throughout the codebase to prevent null/empty value errors and provide meaningful error messages.

## Overview

Poka-yoke (mistake-proofing) is a defensive programming practice that guards against common errors by:
- Adding null/empty checks before processing data
- Replacing `unwrap()` with `expect()` or proper error handling
- Providing meaningful error messages
- Using utility functions for safe unwrapping

## Safe Unwrapping Utilities

All safe unwrapping utilities are located in `src/utils.rs`. These functions provide descriptive error messages when operations fail.

### Collection Access

```rust
use crate::utils::{safe_first, safe_last, safe_get, ensure_not_empty};

// Instead of:
let first = my_vec.first().unwrap();

// Use:
let first = safe_first(&my_vec, "processing user input")?;

// Instead of:
let item = my_vec[5];  // panics if out of bounds

// Use:
let item = safe_get(&my_vec, 5, "accessing cached results")?;

// Guard against empty collections:
ensure_not_empty(&my_vec, "transaction list must not be empty")?;
```

### Option Unwrapping

```rust
use crate::utils::expect_some;

// Instead of:
let value = option.unwrap();

// Use:
let value = expect_some(option, "configuration value must be present")?;
```

### JSON Value Extraction

```rust
use crate::utils::{safe_json_str, safe_json_array, safe_json_object};

// Instead of:
let workbook_id = json["workbook_id"].as_str().unwrap();

// Use:
let workbook_id = safe_json_str(&json, "workbook_id", "parsing workbook response")?;

// For arrays:
let changes = safe_json_array(&json, "changes", "parsing changeset")?;

// For objects:
let metadata = safe_json_object(&json, "metadata", "parsing workbook metadata")?;
```

### String Operations

```rust
use crate::utils::{safe_strip_prefix, ensure_non_empty_str, safe_parse};

// Instead of:
let id = s.strip_prefix("wb-").unwrap();

// Use:
let id = safe_strip_prefix(s, "wb-", "parsing workbook ID")?;

// Guard against empty strings:
let name = ensure_non_empty_str(&input, "sheet name cannot be empty")?;

// Safe parsing:
let port: u16 = safe_parse("8080", "parsing port number")?;
```

## Defensive Patterns by Domain

### Spreadsheet Cell Operations

**Always check for null cells before accessing values:**

```rust
// Instead of:
let cell = sheet.get_cell("A1").unwrap();
let value = cell.get_value();

// Use:
let cell = sheet.get_cell("A1")
    .ok_or_else(|| anyhow!("Cell A1 not found in sheet {}", sheet_name))?;
let value = cell.get_value();

// Or use expect() with context:
let cell = sheet.get_cell("A1")
    .expect("Cell A1 should exist in validated sheet");
```

**Check sheet existence:**

```rust
// Instead of:
let sheet = book.get_sheet_by_name_mut("Sheet1").unwrap();

// Use:
let sheet = book.get_sheet_by_name_mut("Sheet1")
    .ok_or_else(|| anyhow!("Sheet 'Sheet1' not found in workbook"))?;

// Or with expect:
let sheet = book.get_sheet_by_name_mut("Sheet1")
    .expect("Sheet1 must exist in test workbook");
```

### Formula Parsing

**Guard against empty formulas:**

```rust
pub fn parse_base_formula(formula: &str) -> Result<ASTNode> {
    let trimmed = formula.trim();
    if trimmed.is_empty() {
        bail!("Formula cannot be empty");
    }

    let with_equals = if trimmed.starts_with('=') {
        trimmed.to_string()
    } else {
        format!("={}", trimmed)
    };

    formualizer_parse::parse(&with_equals)
        .map_err(|e| anyhow!("Failed to parse formula '{}': {}", formula, e.message))
}
```

**Safe coordinate parsing:**

```rust
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

    // ... rest of parsing logic
}
```

### Collection Processing

**Always check isEmpty() before processing:**

```rust
pub fn compute_sheet_statistics(sheet: &Worksheet, _sample_rows: usize) -> SheetStats {
    let (max_col, max_row) = sheet.get_highest_column_and_row();

    // Guard against empty sheets
    if max_col == 0 || max_row == 0 {
        return SheetStats::default();
    }

    // ... process sheet data
}
```

**Guard against division by zero:**

```rust
let total_cells = (max_col * max_row) as f32;

// Guard against division by zero when computing density
let density = if total_cells == 0.0 {
    0.0
} else {
    filled_cells as f32 / total_cells
};
```

**Safe collection statistics:**

```rust
// Guard against empty numeric_values collection before computing stats
let mean = if numeric_values.is_empty() {
    None
} else {
    Some(numeric_values.iter().sum::<f64>() / numeric_values.len() as f64)
};
```

## When to Use unwrap() vs expect() vs ?

### Use `?` operator (preferred)
When the function already returns `Result` and the error can propagate:

```rust
pub fn load_workbook(path: &Path) -> Result<Workbook> {
    let metadata = fs::metadata(path)?;  // ✓ Good
    let spreadsheet = xlsx::read(path)?;  // ✓ Good
    Ok(Workbook { ... })
}
```

### Use `expect()` with meaningful message
When the unwrap is guaranteed to succeed based on program logic or validation:

```rust
// Static regex patterns that are always valid
let pattern = Regex::new(r"^wb-[a-z0-9]{10}$")
    .expect("Valid regex pattern should always compile");

// Validated date construction
let epoch = NaiveDate::from_ymd_opt(1899, 12, 31)
    .expect("Valid epoch date 1899-12-31 should always be constructible");

// After explicit checks
if s.is_empty() {
    return 0.0;
}
let first_char = s.chars().next()
    .expect("String is not empty, first char must exist");
```

### Never use bare `unwrap()`
Replace all bare `unwrap()` calls with one of the above patterns.

## Test Code Guidelines

Even in test code, prefer `expect()` over `unwrap()` for better error messages:

```rust
#[test]
fn test_sheet_operations() {
    let mut book = Spreadsheet::new();

    // Instead of:
    let sheet = book.get_sheet_by_name_mut("Sheet1").unwrap();

    // Use:
    let sheet = book.get_sheet_by_name_mut("Sheet1")
        .expect("Test workbook should have Sheet1");

    // For JSON in tests:
    let workbook_id = workbooks["workbooks"][0]["workbook_id"]
        .as_str()
        .expect("Workbook ID should be present in test response");
}
```

## Error Message Best Practices

1. **Be specific about what failed:**
   ```rust
   // Bad
   .expect("failed")

   // Good
   .expect("Failed to parse workbook ID from response")
   ```

2. **Include context:**
   ```rust
   // Bad
   .ok_or_else(|| anyhow!("Not found"))?

   // Good
   .ok_or_else(|| anyhow!("Sheet '{}' not found in workbook '{}'",
                          sheet_name, workbook_id))?
   ```

3. **Indicate expected state:**
   ```rust
   // Good
   .expect("Cell A1 should exist in validated input range")
   .expect("Regex pattern is static and should always compile")
   .expect("Configuration file must contain server port")
   ```

## Common Patterns

### Pattern: Option Chain with Context

```rust
// Instead of chained unwraps:
let value = data.get("key").unwrap().as_str().unwrap();

// Use:
let value = data
    .get("key")
    .and_then(|v| v.as_str())
    .ok_or_else(|| anyhow!("Missing or invalid 'key' in data"))?;
```

### Pattern: Safe First/Last Element

```rust
// Instead of:
let first_region = regions.first().unwrap();
let last_change = changes.last().unwrap();

// Use:
let first_region = safe_first(&regions, "detected regions list")?;
let last_change = safe_last(&changes, "changeset history")?;
```

### Pattern: Fallback with Warning

For cases where you want to continue with a default but log the issue:

```rust
use crate::utils::unwrap_or_default_with_warning;

let config = unwrap_or_default_with_warning(
    optional_config,
    "server configuration not found, using defaults"
);
```

## Migration Checklist

When updating code to use defensive patterns:

- [ ] Replace `unwrap()` with `expect()` + meaningful message
- [ ] Replace `expect()` with `?` operator where appropriate
- [ ] Add `isEmpty()` checks before collection processing
- [ ] Add null checks before accessing optional values
- [ ] Add guards for division by zero
- [ ] Add guards for empty strings
- [ ] Use safe utility functions from `utils.rs`
- [ ] Provide context in all error messages
- [ ] Update tests to use `expect()` instead of `unwrap()`

## Summary

By following these defensive coding patterns:
- **Errors are caught early** with meaningful messages
- **Debugging is faster** with descriptive error context
- **Code is more robust** against edge cases
- **Maintenance is easier** with self-documenting error handling

Always ask: "What could go wrong here?" and add appropriate guards.
