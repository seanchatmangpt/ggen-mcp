//! Enhanced boundary validation with rich error context and actionable messages.
//!
//! This module wraps the basic bounds validation with our comprehensive error handling
//! system to provide better error messages, suggestions, and telemetry.

use crate::error::{ErrorCode, McpError};
use crate::validation::bounds::*;
use anyhow::Result;

/// Enhanced row validation with rich error context
pub fn validate_row_enhanced(
    row: u32,
    operation: &str,
    workbook_id: Option<&str>,
    sheet_name: Option<&str>,
) -> Result<()> {
    if row == 0 {
        let mut builder = McpError::validation()
            .message("Row number must be at least 1 (rows are 1-indexed in Excel)")
            .operation(operation)
            .param("row", row)
            .suggestion("Row numbers start at 1, not 0")
            .suggestion("Use 1 for the first row");

        if let Some(wb) = workbook_id {
            builder = builder.workbook_id(wb);
        }
        if let Some(sheet) = sheet_name {
            builder = builder.sheet_name(sheet);
        }

        return Err(builder.build_and_track().into_anyhow());
    }

    if row > EXCEL_MAX_ROWS {
        let mut builder = McpError::validation()
            .message(format!(
                "Row number {} exceeds Excel maximum of {}",
                row, EXCEL_MAX_ROWS
            ))
            .operation(operation)
            .param("row", row)
            .param("max_rows", EXCEL_MAX_ROWS)
            .suggestion(format!("Row must be between 1 and {}", EXCEL_MAX_ROWS))
            .suggestion("Use sheet_overview to check actual sheet dimensions")
            .doc_link("https://support.microsoft.com/en-us/office/excel-specifications-and-limits");

        if let Some(wb) = workbook_id {
            builder = builder.workbook_id(wb);
        }
        if let Some(sheet) = sheet_name {
            builder = builder.sheet_name(sheet);
        }

        return Err(builder.build_and_track().into_anyhow());
    }

    Ok(())
}

/// Enhanced column validation with rich error context
pub fn validate_column_enhanced(
    col: u32,
    operation: &str,
    workbook_id: Option<&str>,
    sheet_name: Option<&str>,
) -> Result<()> {
    if col == 0 {
        let mut builder = McpError::validation()
            .message("Column number must be at least 1 (columns are 1-indexed in Excel)")
            .operation(operation)
            .param("column", col)
            .suggestion("Column numbers start at 1, not 0")
            .suggestion("Use 1 for the first column (column A)");

        if let Some(wb) = workbook_id {
            builder = builder.workbook_id(wb);
        }
        if let Some(sheet) = sheet_name {
            builder = builder.sheet_name(sheet);
        }

        return Err(builder.build_and_track().into_anyhow());
    }

    if col > EXCEL_MAX_COLUMNS {
        let mut builder = McpError::validation()
            .message(format!(
                "Column number {} exceeds Excel maximum of {} (column XFD)",
                col, EXCEL_MAX_COLUMNS
            ))
            .operation(operation)
            .param("column", col)
            .param("max_columns", EXCEL_MAX_COLUMNS)
            .suggestion(format!(
                "Column must be between 1 and {} (XFD)",
                EXCEL_MAX_COLUMNS
            ))
            .suggestion("Use sheet_overview to check actual sheet dimensions")
            .doc_link("https://support.microsoft.com/en-us/office/excel-specifications-and-limits");

        if let Some(wb) = workbook_id {
            builder = builder.workbook_id(wb);
        }
        if let Some(sheet) = sheet_name {
            builder = builder.sheet_name(sheet);
        }

        return Err(builder.build_and_track().into_anyhow());
    }

    Ok(())
}

/// Enhanced range validation with rich error context
pub fn validate_range_enhanced(
    start_row: u32,
    start_col: u32,
    end_row: u32,
    end_col: u32,
    operation: &str,
    workbook_id: Option<&str>,
    sheet_name: Option<&str>,
    range_str: Option<&str>,
) -> Result<(u32, u32)> {
    // Validate start cell
    validate_row_enhanced(start_row, operation, workbook_id, sheet_name)?;
    validate_column_enhanced(start_col, operation, workbook_id, sheet_name)?;

    // Validate end cell
    validate_row_enhanced(end_row, operation, workbook_id, sheet_name)?;
    validate_column_enhanced(end_col, operation, workbook_id, sheet_name)?;

    // Check that start <= end
    if start_row > end_row {
        let mut builder = McpError::builder(ErrorCode::InvalidRange)
            .message(format!(
                "Invalid range: start row {} is greater than end row {}",
                start_row, end_row
            ))
            .operation(operation)
            .param("start_row", start_row)
            .param("end_row", end_row)
            .suggestion("Ensure start row is less than or equal to end row")
            .suggestion("Range should be specified as top-left to bottom-right");

        if let Some(range) = range_str {
            builder = builder.range(range);
        }
        if let Some(wb) = workbook_id {
            builder = builder.workbook_id(wb);
        }
        if let Some(sheet) = sheet_name {
            builder = builder.sheet_name(sheet);
        }

        return Err(builder.build_and_track().into_anyhow());
    }

    if start_col > end_col {
        let mut builder = McpError::builder(ErrorCode::InvalidRange)
            .message(format!(
                "Invalid range: start column {} is greater than end column {}",
                start_col, end_col
            ))
            .operation(operation)
            .param("start_col", start_col)
            .param("end_col", end_col)
            .suggestion("Ensure start column is less than or equal to end column")
            .suggestion("Range should be specified as top-left to bottom-right");

        if let Some(range) = range_str {
            builder = builder.range(range);
        }
        if let Some(wb) = workbook_id {
            builder = builder.workbook_id(wb);
        }
        if let Some(sheet) = sheet_name {
            builder = builder.sheet_name(sheet);
        }

        return Err(builder.build_and_track().into_anyhow());
    }

    let rows = end_row - start_row + 1;
    let cols = end_col - start_col + 1;

    Ok((rows, cols))
}

/// Enhanced pagination validation with rich error context
pub fn validate_pagination_enhanced(offset: usize, limit: usize, operation: &str) -> Result<()> {
    if offset > MAX_PAGINATION_OFFSET {
        return Err(McpError::validation()
            .message(format!(
                "Offset {} exceeds maximum allowed offset of {}",
                offset, MAX_PAGINATION_OFFSET
            ))
            .operation(operation)
            .param("offset", offset)
            .param("max_offset", MAX_PAGINATION_OFFSET)
            .suggestion(format!("Offset must be at most {}", MAX_PAGINATION_OFFSET))
            .suggestion("Use filters to narrow results instead of large offsets")
            .build_and_track()
            .into_anyhow());
    }

    if limit > MAX_PAGINATION_LIMIT {
        return Err(McpError::validation()
            .message(format!(
                "Limit {} exceeds maximum allowed limit of {}",
                limit, MAX_PAGINATION_LIMIT
            ))
            .operation(operation)
            .param("limit", limit)
            .param("max_limit", MAX_PAGINATION_LIMIT)
            .suggestion(format!("Limit must be at most {}", MAX_PAGINATION_LIMIT))
            .suggestion("Use pagination with smaller limits")
            .suggestion("Consider using summary_only=true for large datasets")
            .build_and_track()
            .into_anyhow());
    }

    // Check for overflow
    if offset.checked_add(limit).is_none() {
        return Err(McpError::validation()
            .message(format!(
                "Offset + limit would overflow: {} + {}",
                offset, limit
            ))
            .operation(operation)
            .param("offset", offset)
            .param("limit", limit)
            .suggestion("Reduce offset or limit to prevent arithmetic overflow")
            .build_and_track()
            .into_anyhow());
    }

    Ok(())
}

/// Enhanced screenshot range validation with rich error context
pub fn validate_screenshot_range_enhanced(
    rows: u32,
    cols: u32,
    operation: &str,
    range_str: Option<&str>,
) -> Result<()> {
    if rows > MAX_SCREENSHOT_ROWS || cols > MAX_SCREENSHOT_COLS {
        let row_tiles = div_ceil(rows, MAX_SCREENSHOT_ROWS);
        let col_tiles = div_ceil(cols, MAX_SCREENSHOT_COLS);
        let total_tiles = row_tiles * col_tiles;

        let mut builder = McpError::builder(ErrorCode::ResourceExhausted)
            .message(format!(
                "Range too large for screenshot: {} rows x {} cols (max {} x {})",
                rows, cols, MAX_SCREENSHOT_ROWS, MAX_SCREENSHOT_COLS
            ))
            .operation(operation)
            .param("rows", rows)
            .param("cols", cols)
            .param("max_rows", MAX_SCREENSHOT_ROWS)
            .param("max_cols", MAX_SCREENSHOT_COLS)
            .param("required_tiles", total_tiles)
            .suggestion(format!(
                "Reduce range to at most {} rows x {} columns",
                MAX_SCREENSHOT_ROWS, MAX_SCREENSHOT_COLS
            ))
            .suggestion("Break large areas into smaller screenshots")
            .suggestion("Use sheet_overview to identify regions of interest");

        if let Some(range) = range_str {
            builder = builder.range(range);
            builder = builder.suggestion(format!(
                "Try a smaller range like the first {} rows",
                MAX_SCREENSHOT_ROWS
            ));
        }

        return Err(builder.build_and_track().into_anyhow());
    }

    let cell_count = rows as u64 * cols as u64;
    if cell_count > MAX_SCREENSHOT_CELLS as u64 {
        let mut builder = McpError::builder(ErrorCode::ResourceExhausted)
            .message(format!(
                "Range contains {} cells, exceeds maximum of {} cells for screenshots",
                cell_count, MAX_SCREENSHOT_CELLS
            ))
            .operation(operation)
            .param("cell_count", cell_count)
            .param("max_cells", MAX_SCREENSHOT_CELLS)
            .suggestion(format!(
                "Reduce range to at most {} cells",
                MAX_SCREENSHOT_CELLS
            ))
            .suggestion("Focus on specific areas of the sheet");

        if let Some(range) = range_str {
            builder = builder.range(range);
        }

        return Err(builder.build_and_track().into_anyhow());
    }

    Ok(())
}

/// Enhanced sample size validation with rich error context
pub fn validate_sample_size_enhanced(
    sample_size: usize,
    total_rows: usize,
    operation: &str,
) -> Result<usize> {
    if sample_size > MAX_SAMPLE_SIZE {
        return Err(McpError::validation()
            .message(format!(
                "Sample size {} exceeds maximum allowed sample size of {}",
                sample_size, MAX_SAMPLE_SIZE
            ))
            .operation(operation)
            .param("sample_size", sample_size)
            .param("max_sample_size", MAX_SAMPLE_SIZE)
            .param("total_rows", total_rows)
            .suggestion(format!("Sample size must be at most {}", MAX_SAMPLE_SIZE))
            .suggestion("Use pagination with limit/offset for large datasets")
            .suggestion("Consider using summary statistics instead of full sampling")
            .build_and_track()
            .into_anyhow());
    }

    if sample_size > total_rows {
        // Clamp to total_rows (warning in logs, but not an error)
        tracing::debug!(
            operation = operation,
            sample_size = sample_size,
            total_rows = total_rows,
            "Sample size exceeds total rows, clamping to total"
        );
        Ok(total_rows)
    } else {
        Ok(sample_size)
    }
}

// Re-export the ceiling division utility
const fn div_ceil(n: u32, d: u32) -> u32 {
    (n + d - 1) / d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_row_validation_enhanced_zero() {
        let result = validate_row_enhanced(0, "test_op", Some("test.xlsx"), Some("Sheet1"));
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.to_lowercase().contains("at least 1"));
    }

    #[test]
    fn test_row_validation_enhanced_too_large() {
        let result = validate_row_enhanced(
            EXCEL_MAX_ROWS + 1,
            "test_op",
            Some("test.xlsx"),
            Some("Sheet1"),
        );
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains(&EXCEL_MAX_ROWS.to_string()));
    }

    #[test]
    fn test_row_validation_enhanced_valid() {
        assert!(validate_row_enhanced(1, "test_op", Some("test.xlsx"), Some("Sheet1")).is_ok());
        assert!(
            validate_row_enhanced(EXCEL_MAX_ROWS, "test_op", Some("test.xlsx"), Some("Sheet1"))
                .is_ok()
        );
    }

    #[test]
    fn test_range_validation_enhanced_inverted() {
        let result = validate_range_enhanced(
            10,
            10,
            5,
            5,
            "test_op",
            Some("test.xlsx"),
            Some("Sheet1"),
            Some("J10:E5"),
        );
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.to_lowercase().contains("greater than"));
    }

    #[test]
    fn test_pagination_validation_enhanced_overflow() {
        let result = validate_pagination_enhanced(usize::MAX - 5, 10, "test_op");
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.to_lowercase().contains("overflow"));
    }

    #[test]
    fn test_screenshot_range_validation_enhanced_too_large() {
        let result = validate_screenshot_range_enhanced(
            MAX_SCREENSHOT_ROWS + 1,
            10,
            "screenshot_sheet",
            Some("A1:J200"),
        );
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.to_lowercase().contains("too large"));
    }

    #[test]
    fn test_sample_size_validation_enhanced_too_large() {
        let result = validate_sample_size_enhanced(MAX_SAMPLE_SIZE + 1, 1_000_000, "test_op");
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains(&MAX_SAMPLE_SIZE.to_string()));
    }

    #[test]
    fn test_sample_size_validation_enhanced_clamping() {
        // Should clamp to total_rows
        let result = validate_sample_size_enhanced(100, 50, "test_op");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 50);
    }
}
