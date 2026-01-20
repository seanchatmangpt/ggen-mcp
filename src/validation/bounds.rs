//! Boundary and range validation guards for numeric parameters.
//!
//! This module provides compile-time constants and runtime validation functions
//! to ensure numeric parameters stay within safe and reasonable bounds.

use anyhow::{anyhow, bail, Result};

// ============================================================================
// Excel Limits (Microsoft Excel 2007+)
// ============================================================================

/// Maximum number of rows in an Excel worksheet (2^20)
pub const EXCEL_MAX_ROWS: u32 = 1_048_576;

/// Maximum number of columns in an Excel worksheet (2^14)
pub const EXCEL_MAX_COLUMNS: u32 = 16_384;

/// Maximum column index (0-based)
pub const EXCEL_MAX_COLUMN_INDEX: u32 = EXCEL_MAX_COLUMNS - 1;

/// Maximum row index (0-based)
pub const EXCEL_MAX_ROW_INDEX: u32 = EXCEL_MAX_ROWS - 1;

/// Maximum total cells in a worksheet (theoretical limit)
pub const EXCEL_MAX_CELLS: u64 = EXCEL_MAX_ROWS as u64 * EXCEL_MAX_COLUMNS as u64;

// ============================================================================
// Cache Configuration Limits
// ============================================================================

/// Minimum reasonable cache capacity
pub const MIN_CACHE_CAPACITY: usize = 1;

/// Maximum reasonable cache capacity
pub const MAX_CACHE_CAPACITY: usize = 100;

/// Default cache capacity if none specified
pub const DEFAULT_CACHE_CAPACITY: usize = 5;

// ============================================================================
// Screenshot Limits
// ============================================================================

/// Maximum number of rows in a single screenshot
pub const MAX_SCREENSHOT_ROWS: u32 = 100;

/// Maximum number of columns in a single screenshot
pub const MAX_SCREENSHOT_COLS: u32 = 30;

/// Maximum total cells in a screenshot
pub const MAX_SCREENSHOT_CELLS: u32 = MAX_SCREENSHOT_ROWS * MAX_SCREENSHOT_COLS;

/// Default maximum PNG dimension in pixels (width or height)
pub const DEFAULT_MAX_PNG_DIM_PX: u32 = 4096;

/// Default maximum PNG area in pixels (width * height)
pub const DEFAULT_MAX_PNG_AREA_PX: u64 = 12_000_000;

/// Maximum reasonable PNG dimension (prevents resource exhaustion)
pub const ABSOLUTE_MAX_PNG_DIM_PX: u32 = 16_384;

/// Maximum reasonable PNG area (prevents resource exhaustion)
pub const ABSOLUTE_MAX_PNG_AREA_PX: u64 = 100_000_000;

// ============================================================================
// Sample and Pagination Limits
// ============================================================================

/// Maximum reasonable sample size for statistics
pub const MAX_SAMPLE_SIZE: usize = 100_000;

/// Maximum reasonable limit value for pagination
pub const MAX_PAGINATION_LIMIT: usize = 10_000;

/// Maximum reasonable offset value for pagination
pub const MAX_PAGINATION_OFFSET: usize = 1_000_000;

// ============================================================================
// Validation Functions
// ============================================================================

/// Validates that a row index is within Excel limits.
///
/// # Arguments
/// * `row` - 1-based row number (as used in Excel)
/// * `context` - Description of what's being validated (for error messages)
///
/// # Returns
/// Ok(()) if valid, Err with descriptive message if invalid
#[inline]
pub fn validate_row_1based(row: u32, context: &str) -> Result<()> {
    if row == 0 {
        bail!("{}: row number must be at least 1", context);
    }
    if row > EXCEL_MAX_ROWS {
        bail!(
            "{}: row {} exceeds Excel limit of {} rows",
            context,
            row,
            EXCEL_MAX_ROWS
        );
    }
    Ok(())
}

/// Validates that a column index is within Excel limits.
///
/// # Arguments
/// * `col` - 1-based column number (as used in Excel)
/// * `context` - Description of what's being validated (for error messages)
///
/// # Returns
/// Ok(()) if valid, Err with descriptive message if invalid
#[inline]
pub fn validate_column_1based(col: u32, context: &str) -> Result<()> {
    if col == 0 {
        bail!("{}: column number must be at least 1", context);
    }
    if col > EXCEL_MAX_COLUMNS {
        bail!(
            "{}: column {} exceeds Excel limit of {} columns",
            context,
            col,
            EXCEL_MAX_COLUMNS
        );
    }
    Ok(())
}

/// Validates a cell reference (row, col) with 1-based indices.
#[inline]
pub fn validate_cell_1based(row: u32, col: u32, context: &str) -> Result<()> {
    validate_row_1based(row, context)?;
    validate_column_1based(col, context)?;
    Ok(())
}

/// Validates a range defined by two cell references.
///
/// # Arguments
/// * `start_row`, `start_col` - Top-left cell (1-based)
/// * `end_row`, `end_col` - Bottom-right cell (1-based)
/// * `context` - Description of what's being validated
///
/// # Returns
/// Ok((rows, cols)) - the dimensions of the range
#[inline]
pub fn validate_range_1based(
    start_row: u32,
    start_col: u32,
    end_row: u32,
    end_col: u32,
    context: &str,
) -> Result<(u32, u32)> {
    validate_cell_1based(start_row, start_col, context)?;
    validate_cell_1based(end_row, end_col, context)?;

    if start_row > end_row {
        bail!(
            "{}: start row {} is greater than end row {}",
            context,
            start_row,
            end_row
        );
    }
    if start_col > end_col {
        bail!(
            "{}: start column {} is greater than end column {}",
            context,
            start_col,
            end_col
        );
    }

    let rows = end_row - start_row + 1;
    let cols = end_col - start_col + 1;

    Ok((rows, cols))
}

/// Validates cache capacity is within reasonable bounds.
#[inline]
pub fn validate_cache_capacity(capacity: usize) -> Result<usize> {
    if capacity < MIN_CACHE_CAPACITY {
        bail!(
            "cache capacity {} is below minimum of {}",
            capacity,
            MIN_CACHE_CAPACITY
        );
    }
    if capacity > MAX_CACHE_CAPACITY {
        bail!(
            "cache capacity {} exceeds maximum of {}",
            capacity,
            MAX_CACHE_CAPACITY
        );
    }
    Ok(capacity)
}

/// Clamps cache capacity to valid range, returning the clamped value.
#[inline]
pub fn clamp_cache_capacity(capacity: usize) -> usize {
    capacity.clamp(MIN_CACHE_CAPACITY, MAX_CACHE_CAPACITY)
}

/// Validates that a sample size doesn't exceed the total row count.
///
/// # Arguments
/// * `sample_size` - Requested sample size
/// * `total_rows` - Total number of rows available
///
/// # Returns
/// Ok(effective_sample_size) - The validated (possibly clamped) sample size
#[inline]
pub fn validate_sample_size(sample_size: usize, total_rows: usize) -> Result<usize> {
    if sample_size > MAX_SAMPLE_SIZE {
        bail!(
            "sample size {} exceeds maximum allowed sample size of {}",
            sample_size,
            MAX_SAMPLE_SIZE
        );
    }
    if sample_size > total_rows {
        // Clamp to total_rows rather than failing
        Ok(total_rows)
    } else {
        Ok(sample_size)
    }
}

/// Validates pagination parameters (offset, limit).
///
/// # Arguments
/// * `offset` - Starting position
/// * `limit` - Maximum number of items to return
///
/// # Returns
/// Ok(()) if valid, Err if parameters would cause overflow or exceed limits
#[inline]
pub fn validate_pagination(offset: usize, limit: usize) -> Result<()> {
    if offset > MAX_PAGINATION_OFFSET {
        bail!(
            "offset {} exceeds maximum allowed offset of {}",
            offset,
            MAX_PAGINATION_OFFSET
        );
    }
    if limit > MAX_PAGINATION_LIMIT {
        bail!(
            "limit {} exceeds maximum allowed limit of {}",
            limit,
            MAX_PAGINATION_LIMIT
        );
    }

    // Check for overflow when adding offset + limit
    offset
        .checked_add(limit)
        .ok_or_else(|| anyhow!("offset + limit would overflow: {} + {}", offset, limit))?;

    Ok(())
}

/// Validates PNG dimensions.
///
/// # Arguments
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
/// * `max_dim` - Maximum allowed dimension (optional, uses DEFAULT_MAX_PNG_DIM_PX if None)
/// * `max_area` - Maximum allowed area (optional, uses DEFAULT_MAX_PNG_AREA_PX if None)
///
/// # Returns
/// Ok(()) if valid, Err with descriptive message if limits exceeded
#[inline]
pub fn validate_png_dimensions(
    width: u32,
    height: u32,
    max_dim: Option<u32>,
    max_area: Option<u64>,
) -> Result<()> {
    let max_dim = max_dim.unwrap_or(DEFAULT_MAX_PNG_DIM_PX);
    let max_area = max_area.unwrap_or(DEFAULT_MAX_PNG_AREA_PX);

    // Enforce absolute limits even if custom limits are higher
    let effective_max_dim = max_dim.min(ABSOLUTE_MAX_PNG_DIM_PX);
    let effective_max_area = max_area.min(ABSOLUTE_MAX_PNG_AREA_PX);

    if width > effective_max_dim {
        bail!(
            "PNG width {} exceeds maximum dimension of {} pixels",
            width,
            effective_max_dim
        );
    }
    if height > effective_max_dim {
        bail!(
            "PNG height {} exceeds maximum dimension of {} pixels",
            height,
            effective_max_dim
        );
    }

    let area = width as u64 * height as u64;
    if area > effective_max_area {
        bail!(
            "PNG area {} pixels ({}x{}) exceeds maximum area of {} pixels",
            area,
            width,
            height,
            effective_max_area
        );
    }

    Ok(())
}

/// Validates that a range is suitable for screenshots.
///
/// # Arguments
/// * `rows` - Number of rows in the range
/// * `cols` - Number of columns in the range
///
/// # Returns
/// Ok(()) if within screenshot limits, Err with suggestion if too large
#[inline]
pub fn validate_screenshot_range(rows: u32, cols: u32) -> Result<()> {
    if rows > MAX_SCREENSHOT_ROWS || cols > MAX_SCREENSHOT_COLS {
        let row_tiles = div_ceil(rows, MAX_SCREENSHOT_ROWS);
        let col_tiles = div_ceil(cols, MAX_SCREENSHOT_COLS);
        let total_tiles = row_tiles * col_tiles;

        bail!(
            "Range is too large for a single screenshot ({} rows x {} cols; max {} x {}). \
             Would require {} tile(s) ({} row tiles x {} col tiles).",
            rows,
            cols,
            MAX_SCREENSHOT_ROWS,
            MAX_SCREENSHOT_COLS,
            total_tiles,
            row_tiles,
            col_tiles
        );
    }

    let cell_count = rows as u64 * cols as u64;
    if cell_count > MAX_SCREENSHOT_CELLS as u64 {
        bail!(
            "Range contains {} cells, exceeds maximum of {} cells for screenshots",
            cell_count,
            MAX_SCREENSHOT_CELLS
        );
    }

    Ok(())
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Ceiling division: (n + d - 1) / d
#[inline]
const fn div_ceil(n: u32, d: u32) -> u32 {
    (n + d - 1) / d
}

// ============================================================================
// Compile-time Checks
// ============================================================================

/// Compile-time assertion that Excel limits are sensible
const _: () = {
    assert!(EXCEL_MAX_ROWS > 0, "EXCEL_MAX_ROWS must be positive");
    assert!(EXCEL_MAX_COLUMNS > 0, "EXCEL_MAX_COLUMNS must be positive");
    assert!(
        EXCEL_MAX_ROWS == 1_048_576,
        "EXCEL_MAX_ROWS must match Excel 2007+ limit"
    );
    assert!(
        EXCEL_MAX_COLUMNS == 16_384,
        "EXCEL_MAX_COLUMNS must match Excel 2007+ limit"
    );
};

/// Compile-time assertion that cache limits are sensible
const _: () = {
    assert!(
        MIN_CACHE_CAPACITY > 0,
        "MIN_CACHE_CAPACITY must be positive"
    );
    assert!(
        MAX_CACHE_CAPACITY >= MIN_CACHE_CAPACITY,
        "MAX_CACHE_CAPACITY must be >= MIN_CACHE_CAPACITY"
    );
    assert!(
        DEFAULT_CACHE_CAPACITY >= MIN_CACHE_CAPACITY,
        "DEFAULT_CACHE_CAPACITY must be >= MIN_CACHE_CAPACITY"
    );
    assert!(
        DEFAULT_CACHE_CAPACITY <= MAX_CACHE_CAPACITY,
        "DEFAULT_CACHE_CAPACITY must be <= MAX_CACHE_CAPACITY"
    );
};

/// Compile-time assertion that screenshot limits are sensible
const _: () = {
    assert!(
        MAX_SCREENSHOT_ROWS > 0,
        "MAX_SCREENSHOT_ROWS must be positive"
    );
    assert!(
        MAX_SCREENSHOT_COLS > 0,
        "MAX_SCREENSHOT_COLS must be positive"
    );
    assert!(
        MAX_SCREENSHOT_ROWS <= EXCEL_MAX_ROWS,
        "MAX_SCREENSHOT_ROWS must not exceed EXCEL_MAX_ROWS"
    );
    assert!(
        MAX_SCREENSHOT_COLS <= EXCEL_MAX_COLUMNS,
        "MAX_SCREENSHOT_COLS must not exceed EXCEL_MAX_COLUMNS"
    );
};

/// Compile-time assertion that PNG limits are sensible
const _: () = {
    assert!(
        DEFAULT_MAX_PNG_DIM_PX > 0,
        "DEFAULT_MAX_PNG_DIM_PX must be positive"
    );
    assert!(
        DEFAULT_MAX_PNG_AREA_PX > 0,
        "DEFAULT_MAX_PNG_AREA_PX must be positive"
    );
    assert!(
        ABSOLUTE_MAX_PNG_DIM_PX >= DEFAULT_MAX_PNG_DIM_PX,
        "ABSOLUTE_MAX_PNG_DIM_PX must be >= DEFAULT_MAX_PNG_DIM_PX"
    );
    assert!(
        ABSOLUTE_MAX_PNG_AREA_PX >= DEFAULT_MAX_PNG_AREA_PX,
        "ABSOLUTE_MAX_PNG_AREA_PX must be >= DEFAULT_MAX_PNG_AREA_PX"
    );
};

/// Compile-time assertion that pagination limits are sensible
const _: () = {
    assert!(
        MAX_SAMPLE_SIZE > 0,
        "MAX_SAMPLE_SIZE must be positive"
    );
    assert!(
        MAX_PAGINATION_LIMIT > 0,
        "MAX_PAGINATION_LIMIT must be positive"
    );
    assert!(
        MAX_PAGINATION_OFFSET < usize::MAX / 2,
        "MAX_PAGINATION_OFFSET must allow safe addition"
    );
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_row_1based() {
        assert!(validate_row_1based(0, "test").is_err());
        assert!(validate_row_1based(1, "test").is_ok());
        assert!(validate_row_1based(EXCEL_MAX_ROWS, "test").is_ok());
        assert!(validate_row_1based(EXCEL_MAX_ROWS + 1, "test").is_err());
    }

    #[test]
    fn test_validate_column_1based() {
        assert!(validate_column_1based(0, "test").is_err());
        assert!(validate_column_1based(1, "test").is_ok());
        assert!(validate_column_1based(EXCEL_MAX_COLUMNS, "test").is_ok());
        assert!(validate_column_1based(EXCEL_MAX_COLUMNS + 1, "test").is_err());
    }

    #[test]
    fn test_validate_range_1based() {
        // Valid range
        let result = validate_range_1based(1, 1, 10, 10, "test");
        assert!(result.is_ok());
        let (rows, cols) = result.unwrap();
        assert_eq!(rows, 10);
        assert_eq!(cols, 10);

        // Invalid: start > end
        assert!(validate_range_1based(10, 10, 5, 5, "test").is_err());

        // Invalid: exceeds Excel limits
        assert!(validate_range_1based(1, 1, EXCEL_MAX_ROWS + 1, 1, "test").is_err());
    }

    #[test]
    fn test_validate_cache_capacity() {
        assert!(validate_cache_capacity(0).is_err());
        assert!(validate_cache_capacity(1).is_ok());
        assert!(validate_cache_capacity(50).is_ok());
        assert!(validate_cache_capacity(100).is_ok());
        assert!(validate_cache_capacity(101).is_err());
    }

    #[test]
    fn test_clamp_cache_capacity() {
        assert_eq!(clamp_cache_capacity(0), MIN_CACHE_CAPACITY);
        assert_eq!(clamp_cache_capacity(1), 1);
        assert_eq!(clamp_cache_capacity(50), 50);
        assert_eq!(clamp_cache_capacity(100), 100);
        assert_eq!(clamp_cache_capacity(1000), MAX_CACHE_CAPACITY);
    }

    #[test]
    fn test_validate_sample_size() {
        assert!(validate_sample_size(10, 100).is_ok());
        assert_eq!(validate_sample_size(10, 100).unwrap(), 10);

        // Sample size larger than total rows gets clamped
        assert_eq!(validate_sample_size(100, 50).unwrap(), 50);

        // Exceeds maximum sample size
        assert!(validate_sample_size(MAX_SAMPLE_SIZE + 1, 1_000_000).is_err());
    }

    #[test]
    fn test_validate_pagination() {
        assert!(validate_pagination(0, 10).is_ok());
        assert!(validate_pagination(100, 50).is_ok());

        // Exceeds offset limit
        assert!(validate_pagination(MAX_PAGINATION_OFFSET + 1, 10).is_err());

        // Exceeds limit limit
        assert!(validate_pagination(0, MAX_PAGINATION_LIMIT + 1).is_err());

        // Would overflow
        assert!(validate_pagination(usize::MAX - 5, 10).is_err());
    }

    #[test]
    fn test_validate_png_dimensions() {
        assert!(validate_png_dimensions(1920, 1080, None, None).is_ok());
        assert!(validate_png_dimensions(4096, 4096, None, None).is_ok());

        // Exceeds dimension limit
        assert!(validate_png_dimensions(5000, 1000, Some(4096), None).is_err());

        // Exceeds area limit
        assert!(validate_png_dimensions(5000, 5000, Some(10000), Some(10_000_000)).is_err());

        // Custom limits respected but clamped to absolute limits
        assert!(validate_png_dimensions(20000, 1000, Some(30000), None).is_err());
    }

    #[test]
    fn test_validate_screenshot_range() {
        assert!(validate_screenshot_range(10, 10).is_ok());
        assert!(validate_screenshot_range(MAX_SCREENSHOT_ROWS, MAX_SCREENSHOT_COLS).is_ok());

        // Exceeds row limit
        assert!(validate_screenshot_range(MAX_SCREENSHOT_ROWS + 1, 10).is_err());

        // Exceeds column limit
        assert!(validate_screenshot_range(10, MAX_SCREENSHOT_COLS + 1).is_err());
    }

    #[test]
    fn test_div_ceil() {
        assert_eq!(div_ceil(10, 3), 4);
        assert_eq!(div_ceil(9, 3), 3);
        assert_eq!(div_ceil(0, 5), 0);
        assert_eq!(div_ceil(1, 1), 1);
    }
}
