//! Input validation guards for MCP tool parameters
//!
//! This module implements comprehensive validation functions following poka-yoke
//! (mistake-proofing) principles to prevent invalid inputs from causing errors
//! or security issues.

use thiserror::Error;

/// Result type for validation operations
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Validation error types
///
/// These errors are raised when input parameters fail validation checks.
/// They provide clear, actionable error messages to help users correct their input.
#[derive(Debug, Error)]
pub enum ValidationError {
    /// String parameter is empty or contains only whitespace
    #[error("parameter '{parameter}' cannot be empty or whitespace-only")]
    EmptyString { parameter: String },

    /// Numeric parameter is outside valid range
    #[error("parameter '{parameter}' value {value} is outside valid range [{min}, {max}]")]
    NumericOutOfRange {
        parameter: String,
        value: i64,
        min: i64,
        max: i64,
    },

    /// Numeric parameter is outside valid range (floating point)
    #[error("parameter '{parameter}' value {value} is outside valid range [{min}, {max}]")]
    NumericOutOfRangeF64 {
        parameter: String,
        value: f64,
        min: f64,
        max: f64,
    },

    /// Path contains potential traversal attempt
    #[error("path '{path}' contains potential path traversal pattern")]
    PathTraversal { path: String },

    /// Invalid sheet name
    #[error("invalid sheet name '{name}': {reason}")]
    InvalidSheetName { name: String, reason: String },

    /// Invalid workbook ID
    #[error("invalid workbook ID '{id}': {reason}")]
    InvalidWorkbookId { id: String, reason: String },

    /// Invalid cell address
    #[error("invalid cell address '{address}': {reason}")]
    InvalidCellAddress { address: String, reason: String },

    /// Invalid range string
    #[error("invalid range '{range}': {reason}")]
    InvalidRange { range: String, reason: String },

    /// Generic validation error
    #[error("{message}")]
    Generic { message: String },
}

/// Validates that a string parameter is not empty or whitespace-only
///
/// # Arguments
///
/// * `parameter_name` - Name of the parameter being validated (for error messages)
/// * `value` - The string value to validate
///
/// # Returns
///
/// Returns the validated string if valid, otherwise returns ValidationError::EmptyString
///
/// # Examples
///
/// ```
/// use spreadsheet_mcp::validation::validate_non_empty_string;
///
/// assert!(validate_non_empty_string("workbook_id", "my-workbook").is_ok());
/// assert!(validate_non_empty_string("workbook_id", "").is_err());
/// assert!(validate_non_empty_string("workbook_id", "   ").is_err());
/// ```
pub fn validate_non_empty_string<'a>(
    parameter_name: &str,
    value: &'a str,
) -> ValidationResult<&'a str> {
    if value.trim().is_empty() {
        Err(ValidationError::EmptyString {
            parameter: parameter_name.to_string(),
        })
    } else {
        Ok(value)
    }
}

/// Validates that a numeric parameter is within a specified range
///
/// # Arguments
///
/// * `parameter_name` - Name of the parameter being validated
/// * `value` - The numeric value to validate
/// * `min` - Minimum acceptable value (inclusive)
/// * `max` - Maximum acceptable value (inclusive)
///
/// # Returns
///
/// Returns the validated value if within range, otherwise returns ValidationError::NumericOutOfRange
///
/// # Examples
///
/// ```
/// use spreadsheet_mcp::validation::validate_numeric_range;
///
/// assert!(validate_numeric_range("limit", 10, 1, 100).is_ok());
/// assert!(validate_numeric_range("limit", 0, 1, 100).is_err());
/// assert!(validate_numeric_range("limit", 101, 1, 100).is_err());
/// ```
pub fn validate_numeric_range<T>(
    parameter_name: &str,
    value: T,
    min: T,
    max: T,
) -> ValidationResult<T>
where
    T: PartialOrd + Copy + Into<i64>,
{
    if value < min || value > max {
        Err(ValidationError::NumericOutOfRange {
            parameter: parameter_name.to_string(),
            value: value.into(),
            min: min.into(),
            max: max.into(),
        })
    } else {
        Ok(value)
    }
}

/// Validates that an optional numeric parameter is within a specified range if present
///
/// # Arguments
///
/// * `parameter_name` - Name of the parameter being validated
/// * `value` - Optional numeric value to validate
/// * `min` - Minimum acceptable value (inclusive)
/// * `max` - Maximum acceptable value (inclusive)
///
/// # Returns
///
/// Returns the validated optional value if None or within range
pub fn validate_optional_numeric_range<T>(
    parameter_name: &str,
    value: Option<T>,
    min: T,
    max: T,
) -> ValidationResult<Option<T>>
where
    T: PartialOrd + Copy + Into<i64>,
{
    match value {
        Some(v) => validate_numeric_range(parameter_name, v, min, max).map(Some),
        None => Ok(None),
    }
}

/// Validates that a path does not contain traversal attempts
///
/// This function checks for common path traversal patterns that could be used
/// to access files outside the intended workspace directory.
///
/// # Arguments
///
/// * `path` - The path string to validate
///
/// # Returns
///
/// Returns the validated path if safe, otherwise returns ValidationError::PathTraversal
///
/// # Security
///
/// This function checks for:
/// - `..` (parent directory traversal)
/// - Absolute paths (starting with `/` on Unix or drive letters on Windows)
/// - Null bytes
/// - Backslash sequences on Unix systems
///
/// # Examples
///
/// ```
/// use spreadsheet_mcp::validation::validate_path_safe;
///
/// assert!(validate_path_safe("data/file.xlsx").is_ok());
/// assert!(validate_path_safe("../etc/passwd").is_err());
/// assert!(validate_path_safe("/etc/passwd").is_err());
/// ```
pub fn validate_path_safe(path: &str) -> ValidationResult<&str> {
    // Check for null bytes
    if path.contains('\0') {
        return Err(ValidationError::PathTraversal {
            path: path.to_string(),
        });
    }

    // Check for parent directory traversal
    if path.contains("..") {
        return Err(ValidationError::PathTraversal {
            path: path.to_string(),
        });
    }

    // Check for absolute paths (Unix-style)
    if path.starts_with('/') {
        return Err(ValidationError::PathTraversal {
            path: path.to_string(),
        });
    }

    // Check for absolute paths (Windows-style drive letters)
    if path.len() >= 2 && path.chars().nth(1) == Some(':') {
        let first_char = path.chars().next().unwrap();
        if first_char.is_ascii_alphabetic() {
            return Err(ValidationError::PathTraversal {
                path: path.to_string(),
            });
        }
    }

    // Check for backslash on Unix (could indicate Windows-style path injection)
    #[cfg(unix)]
    if path.contains('\\') {
        return Err(ValidationError::PathTraversal {
            path: path.to_string(),
        });
    }

    Ok(path)
}

/// Validates a sheet name
///
/// Sheet names must:
/// - Not be empty or whitespace-only
/// - Not exceed 31 characters (Excel limit)
/// - Not contain invalid characters: `:`, `\\`, `/`, `?`, `*`, `[`, `]`
/// - Not be named 'History' (Excel reserved)
///
/// # Arguments
///
/// * `name` - The sheet name to validate
///
/// # Returns
///
/// Returns the validated sheet name if valid
///
/// # Examples
///
/// ```
/// use spreadsheet_mcp::validation::validate_sheet_name;
///
/// assert!(validate_sheet_name("Sheet1").is_ok());
/// assert!(validate_sheet_name("").is_err());
/// assert!(validate_sheet_name("Sheet[1]").is_err());
/// ```
pub fn validate_sheet_name(name: &str) -> ValidationResult<&str> {
    // Check for empty or whitespace
    if name.trim().is_empty() {
        return Err(ValidationError::InvalidSheetName {
            name: name.to_string(),
            reason: "sheet name cannot be empty or whitespace-only".to_string(),
        });
    }

    // Check length (Excel limit is 31 characters)
    if name.len() > 31 {
        return Err(ValidationError::InvalidSheetName {
            name: name.to_string(),
            reason: format!(
                "sheet name exceeds maximum length of 31 characters (got {})",
                name.len()
            ),
        });
    }

    // Check for invalid characters
    const INVALID_CHARS: &[char] = &[':', '\\', '/', '?', '*', '[', ']'];
    if let Some(invalid_char) = name.chars().find(|c| INVALID_CHARS.contains(c)) {
        return Err(ValidationError::InvalidSheetName {
            name: name.to_string(),
            reason: format!("sheet name contains invalid character '{}'", invalid_char),
        });
    }

    // Check for reserved name (Excel reserves 'History')
    if name.eq_ignore_ascii_case("History") {
        return Err(ValidationError::InvalidSheetName {
            name: name.to_string(),
            reason: "'History' is a reserved sheet name".to_string(),
        });
    }

    Ok(name)
}

/// Validates a workbook ID
///
/// Workbook IDs must:
/// - Not be empty or whitespace-only
/// - Not exceed 255 characters
/// - Contain only safe characters (alphanumeric, `-`, `_`, `.`)
///
/// # Arguments
///
/// * `id` - The workbook ID to validate
///
/// # Returns
///
/// Returns the validated workbook ID if valid
///
/// # Examples
///
/// ```
/// use spreadsheet_mcp::validation::validate_workbook_id;
///
/// assert!(validate_workbook_id("my-workbook-123").is_ok());
/// assert!(validate_workbook_id("").is_err());
/// assert!(validate_workbook_id("my/workbook").is_err());
/// ```
pub fn validate_workbook_id(id: &str) -> ValidationResult<&str> {
    // Check for empty or whitespace
    if id.trim().is_empty() {
        return Err(ValidationError::InvalidWorkbookId {
            id: id.to_string(),
            reason: "workbook ID cannot be empty or whitespace-only".to_string(),
        });
    }

    // Check length
    if id.len() > 255 {
        return Err(ValidationError::InvalidWorkbookId {
            id: id.to_string(),
            reason: format!(
                "workbook ID exceeds maximum length of 255 characters (got {})",
                id.len()
            ),
        });
    }

    // Check for safe characters only
    // Allow: alphanumeric, hyphen, underscore, period, colon (for fork IDs)
    let is_safe = id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ':');

    if !is_safe {
        return Err(ValidationError::InvalidWorkbookId {
            id: id.to_string(),
            reason:
                "workbook ID contains invalid characters (only alphanumeric, -, _, ., : allowed)"
                    .to_string(),
        });
    }

    Ok(id)
}

/// Validates a cell address in A1 notation
///
/// Valid cell addresses follow Excel A1 notation:
/// - Column letters (A-Z, AA-ZZ, etc.)
/// - Row number (1-1048576)
///
/// # Arguments
///
/// * `address` - The cell address to validate
///
/// # Returns
///
/// Returns the validated cell address if valid
///
/// # Examples
///
/// ```
/// use spreadsheet_mcp::validation::validate_cell_address;
///
/// assert!(validate_cell_address("A1").is_ok());
/// assert!(validate_cell_address("XFD1048576").is_ok());
/// assert!(validate_cell_address("").is_err());
/// assert!(validate_cell_address("123").is_err());
/// ```
pub fn validate_cell_address(address: &str) -> ValidationResult<&str> {
    if address.is_empty() {
        return Err(ValidationError::InvalidCellAddress {
            address: address.to_string(),
            reason: "cell address cannot be empty".to_string(),
        });
    }

    // Split into column and row parts
    let mut col_end = 0;
    for (i, c) in address.chars().enumerate() {
        if c.is_ascii_alphabetic() {
            col_end = i + 1;
        } else {
            break;
        }
    }

    if col_end == 0 {
        return Err(ValidationError::InvalidCellAddress {
            address: address.to_string(),
            reason: "cell address must start with column letters".to_string(),
        });
    }

    if col_end == address.len() {
        return Err(ValidationError::InvalidCellAddress {
            address: address.to_string(),
            reason: "cell address must include a row number".to_string(),
        });
    }

    let col_part = &address[..col_end];
    let row_part = &address[col_end..];

    // Validate column part (A-XFD for Excel, max 16384 columns)
    if col_part.len() > 3 {
        return Err(ValidationError::InvalidCellAddress {
            address: address.to_string(),
            reason: "column exceeds maximum (XFD)".to_string(),
        });
    }

    // Validate row part is a number
    let row_num = row_part
        .parse::<u32>()
        .map_err(|_| ValidationError::InvalidCellAddress {
            address: address.to_string(),
            reason: "row must be a valid number".to_string(),
        })?;

    // Excel max row is 1048576
    if row_num == 0 || row_num > 1048576 {
        return Err(ValidationError::InvalidCellAddress {
            address: address.to_string(),
            reason: format!("row number {} is outside valid range [1, 1048576]", row_num),
        });
    }

    Ok(address)
}

/// Validates a range string in A1 notation
///
/// Valid ranges:
/// - Single cell: "A1"
/// - Range: "A1:B10"
/// - Column range: "A:A"
/// - Row range: "1:10"
///
/// # Arguments
///
/// * `range` - The range string to validate
///
/// # Returns
///
/// Returns the validated range string if valid
///
/// # Examples
///
/// ```
/// use spreadsheet_mcp::validation::validate_range_string;
///
/// assert!(validate_range_string("A1").is_ok());
/// assert!(validate_range_string("A1:B10").is_ok());
/// assert!(validate_range_string("A:A").is_ok());
/// assert!(validate_range_string("").is_err());
/// ```
pub fn validate_range_string(range: &str) -> ValidationResult<&str> {
    if range.is_empty() {
        return Err(ValidationError::InvalidRange {
            range: range.to_string(),
            reason: "range cannot be empty".to_string(),
        });
    }

    // Check if it's a range (contains :)
    if range.contains(':') {
        let parts: Vec<&str> = range.split(':').collect();
        if parts.len() != 2 {
            return Err(ValidationError::InvalidRange {
                range: range.to_string(),
                reason: "range must have exactly one colon separator".to_string(),
            });
        }

        let start = parts[0];
        let end = parts[1];

        // Check if it's a column range (A:A) or row range (1:10)
        let is_col_range = start.chars().all(|c| c.is_ascii_alphabetic())
            && end.chars().all(|c| c.is_ascii_alphabetic());
        let is_row_range =
            start.chars().all(|c| c.is_ascii_digit()) && end.chars().all(|c| c.is_ascii_digit());

        if is_col_range || is_row_range {
            // Valid column or row range
            Ok(range)
        } else {
            // Should be cell range, validate both parts
            validate_cell_address(start).map_err(|_| ValidationError::InvalidRange {
                range: range.to_string(),
                reason: format!("invalid start cell '{}'", start),
            })?;
            validate_cell_address(end).map_err(|_| ValidationError::InvalidRange {
                range: range.to_string(),
                reason: format!("invalid end cell '{}'", end),
            })?;
            Ok(range)
        }
    } else {
        // Single cell
        validate_cell_address(range).map_err(|_| ValidationError::InvalidRange {
            range: range.to_string(),
            reason: "invalid cell address".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_non_empty_string() {
        assert!(validate_non_empty_string("test", "hello").is_ok());
        assert!(validate_non_empty_string("test", "").is_err());
        assert!(validate_non_empty_string("test", "   ").is_err());
        assert!(validate_non_empty_string("test", "\t\n").is_err());
    }

    #[test]
    fn test_validate_numeric_range() {
        assert!(validate_numeric_range("limit", 50u32, 1u32, 100u32).is_ok());
        assert!(validate_numeric_range("limit", 1u32, 1u32, 100u32).is_ok());
        assert!(validate_numeric_range("limit", 100u32, 1u32, 100u32).is_ok());
        assert!(validate_numeric_range("limit", 0u32, 1u32, 100u32).is_err());
        assert!(validate_numeric_range("limit", 101u32, 1u32, 100u32).is_err());
    }

    #[test]
    fn test_validate_optional_numeric_range() {
        assert!(validate_optional_numeric_range("limit", Some(50u32), 1u32, 100u32).is_ok());
        assert!(validate_optional_numeric_range("limit", None::<u32>, 1u32, 100u32).is_ok());
        assert!(validate_optional_numeric_range("limit", Some(0u32), 1u32, 100u32).is_err());
    }

    #[test]
    fn test_validate_path_safe() {
        // Valid paths
        assert!(validate_path_safe("data/file.xlsx").is_ok());
        assert!(validate_path_safe("folder/subfolder/file.xlsx").is_ok());
        assert!(validate_path_safe("file.xlsx").is_ok());

        // Invalid paths - traversal
        assert!(validate_path_safe("../etc/passwd").is_err());
        assert!(validate_path_safe("data/../etc/passwd").is_err());
        assert!(validate_path_safe("..\\windows\\system32").is_err());

        // Invalid paths - absolute
        assert!(validate_path_safe("/etc/passwd").is_err());
        #[cfg(windows)]
        assert!(validate_path_safe("C:\\Windows\\System32").is_err());

        // Invalid paths - null bytes
        assert!(validate_path_safe("file\0.xlsx").is_err());
    }

    #[test]
    fn test_validate_sheet_name() {
        // Valid names
        assert!(validate_sheet_name("Sheet1").is_ok());
        assert!(validate_sheet_name("Data Analysis").is_ok());
        assert!(validate_sheet_name("Report_2024").is_ok());

        // Invalid - empty
        assert!(validate_sheet_name("").is_err());
        assert!(validate_sheet_name("   ").is_err());

        // Invalid - too long
        assert!(validate_sheet_name("ThisSheetNameIsWayTooLongForExcel123456789").is_err());

        // Invalid - special characters
        assert!(validate_sheet_name("Sheet:1").is_err());
        assert!(validate_sheet_name("Sheet[1]").is_err());
        assert!(validate_sheet_name("Sheet/1").is_err());
        assert!(validate_sheet_name("Sheet\\1").is_err());
        assert!(validate_sheet_name("Sheet?").is_err());
        assert!(validate_sheet_name("Sheet*").is_err());

        // Invalid - reserved name
        assert!(validate_sheet_name("History").is_err());
        assert!(validate_sheet_name("history").is_err());
    }

    #[test]
    fn test_validate_workbook_id() {
        // Valid IDs
        assert!(validate_workbook_id("my-workbook").is_ok());
        assert!(validate_workbook_id("workbook_123").is_ok());
        assert!(validate_workbook_id("wb.xlsx").is_ok());
        assert!(validate_workbook_id("fork:123").is_ok());

        // Invalid - empty
        assert!(validate_workbook_id("").is_err());
        assert!(validate_workbook_id("   ").is_err());

        // Invalid - special characters
        assert!(validate_workbook_id("my/workbook").is_err());
        assert!(validate_workbook_id("my\\workbook").is_err());
        assert!(validate_workbook_id("my workbook").is_err());
    }

    #[test]
    fn test_validate_cell_address() {
        // Valid addresses
        assert!(validate_cell_address("A1").is_ok());
        assert!(validate_cell_address("Z99").is_ok());
        assert!(validate_cell_address("AA100").is_ok());
        assert!(validate_cell_address("XFD1048576").is_ok());

        // Invalid - empty
        assert!(validate_cell_address("").is_err());

        // Invalid - no column
        assert!(validate_cell_address("123").is_err());

        // Invalid - no row
        assert!(validate_cell_address("ABC").is_err());

        // Invalid - row out of range
        assert!(validate_cell_address("A0").is_err());
        assert!(validate_cell_address("A1048577").is_err());
    }

    #[test]
    fn test_validate_range_string() {
        // Valid ranges
        assert!(validate_range_string("A1").is_ok());
        assert!(validate_range_string("A1:B10").is_ok());
        assert!(validate_range_string("A:A").is_ok());
        assert!(validate_range_string("A:Z").is_ok());
        assert!(validate_range_string("1:10").is_ok());

        // Invalid - empty
        assert!(validate_range_string("").is_err());

        // Invalid - multiple colons
        assert!(validate_range_string("A1:B2:C3").is_err());

        // Invalid - bad cell addresses
        assert!(validate_range_string("A1:123").is_err());
        assert!(validate_range_string("ABC:B10").is_err());
    }
}
