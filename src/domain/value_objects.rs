//! Domain Value Objects with NewType pattern for type safety
//!
//! This module implements the NewType wrapper pattern (Poka-Yoke) to prevent
//! type confusion bugs by creating distinct types for domain primitives.
//!
//! # Poka-Yoke Pattern
//!
//! The NewType pattern creates zero-cost abstractions that prevent mixing
//! semantically different values at compile time:
//!
//! ```rust,ignore
//! // Prevents mixing WorkbookId with ForkId
//! let workbook_id = WorkbookId::new("wb123");
//! let fork_id = ForkId::new("fork456");
//! // function(workbook_id) ✓ OK
//! // function(fork_id)     ✗ Compile error!
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// ============================================================================
// WorkbookId - Unique identifier for workbooks
// ============================================================================

/// Unique identifier for a workbook.
///
/// Prevents accidental mixing with ForkId or other string identifiers.
///
/// # Validation
/// - Must not be empty
/// - Maximum length: 1024 characters
///
/// # Example
/// ```rust,ignore
/// let id = WorkbookId::new("workbook-abc123".to_string())?;
/// assert_eq!(id.as_str(), "workbook-abc123");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkbookId(String);

impl WorkbookId {
    const MAX_LENGTH: usize = 1024;

    /// Creates a new WorkbookId with validation.
    ///
    /// # Errors
    /// Returns `Err` if the ID is empty or exceeds maximum length.
    pub fn new(id: String) -> Result<Self, ValidationError> {
        if id.is_empty() {
            return Err(ValidationError::Empty("WorkbookId"));
        }
        if id.len() > Self::MAX_LENGTH {
            return Err(ValidationError::TooLong {
                field: "WorkbookId",
                max: Self::MAX_LENGTH,
                actual: id.len(),
            });
        }
        Ok(Self(id))
    }

    /// Creates a WorkbookId without validation (use with caution).
    ///
    /// # Safety
    /// Caller must ensure the ID is valid.
    pub fn new_unchecked(id: String) -> Self {
        Self(id)
    }

    /// Returns the ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes self and returns the inner String.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for WorkbookId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<WorkbookId> for String {
    fn from(id: WorkbookId) -> String {
        id.0
    }
}

impl AsRef<str> for WorkbookId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ============================================================================
// ForkId - Unique identifier for workbook forks
// ============================================================================

/// Unique identifier for a workbook fork.
///
/// Prevents accidental mixing with WorkbookId or other string identifiers.
///
/// # Validation
/// - Must not be empty
/// - Maximum length: 256 characters
/// - Typically short random IDs (8-16 chars)
///
/// # Example
/// ```rust,ignore
/// let fork_id = ForkId::new("fork-xyz789".to_string())?;
/// assert_eq!(fork_id.as_str(), "fork-xyz789");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ForkId(String);

impl ForkId {
    const MAX_LENGTH: usize = 256;

    /// Creates a new ForkId with validation.
    ///
    /// # Errors
    /// Returns `Err` if the ID is empty or exceeds maximum length.
    pub fn new(id: String) -> Result<Self, ValidationError> {
        if id.is_empty() {
            return Err(ValidationError::Empty("ForkId"));
        }
        if id.len() > Self::MAX_LENGTH {
            return Err(ValidationError::TooLong {
                field: "ForkId",
                max: Self::MAX_LENGTH,
                actual: id.len(),
            });
        }
        Ok(Self(id))
    }

    /// Creates a ForkId without validation (use with caution).
    ///
    /// # Safety
    /// Caller must ensure the ID is valid.
    pub fn new_unchecked(id: String) -> Self {
        Self(id)
    }

    /// Returns the ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes self and returns the inner String.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for ForkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<ForkId> for String {
    fn from(id: ForkId) -> String {
        id.0
    }
}

impl AsRef<str> for ForkId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ============================================================================
// SheetName - Name of a worksheet
// ============================================================================

/// Name of a worksheet in a workbook.
///
/// Prevents accidental mixing with generic strings.
///
/// # Validation
/// - Must not be empty
/// - Maximum length: 255 characters (Excel limit: 31, but we support more)
/// - Cannot contain: [ ] : * ? / \
///
/// # Example
/// ```rust,ignore
/// let sheet = SheetName::new("Q1 Revenue".to_string())?;
/// assert_eq!(sheet.as_str(), "Q1 Revenue");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SheetName(String);

impl SheetName {
    const MAX_LENGTH: usize = 255;
    const FORBIDDEN_CHARS: &'static [char] = &['[', ']', ':', '*', '?', '/', '\\'];

    /// Creates a new SheetName with validation.
    ///
    /// # Errors
    /// Returns `Err` if the name is empty, too long, or contains forbidden characters.
    pub fn new(name: String) -> Result<Self, ValidationError> {
        if name.is_empty() {
            return Err(ValidationError::Empty("SheetName"));
        }
        if name.len() > Self::MAX_LENGTH {
            return Err(ValidationError::TooLong {
                field: "SheetName",
                max: Self::MAX_LENGTH,
                actual: name.len(),
            });
        }
        if let Some(ch) = name.chars().find(|c| Self::FORBIDDEN_CHARS.contains(c)) {
            return Err(ValidationError::InvalidCharacter {
                field: "SheetName",
                character: ch,
            });
        }
        Ok(Self(name))
    }

    /// Creates a SheetName without validation (use with caution).
    ///
    /// # Safety
    /// Caller must ensure the name is valid.
    pub fn new_unchecked(name: String) -> Self {
        Self(name)
    }

    /// Returns the name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes self and returns the inner String.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for SheetName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<SheetName> for String {
    fn from(name: SheetName) -> String {
        name.0
    }
}

impl AsRef<str> for SheetName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl FromStr for SheetName {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}

// ============================================================================
// RegionId - Identifier for a region within a sheet
// ============================================================================

/// Identifier for a region within a worksheet.
///
/// Prevents accidental mixing with row/column indices.
///
/// # Validation
/// - Must be positive (> 0)
/// - Maximum value: 2^31 - 1 (i32::MAX)
///
/// # Example
/// ```rust,ignore
/// let region = RegionId::new(42)?;
/// assert_eq!(region.value(), 42);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RegionId(u32);

impl RegionId {
    /// Creates a new RegionId with validation.
    ///
    /// # Errors
    /// Returns `Err` if the ID is zero (regions start at 1).
    pub fn new(id: u32) -> Result<Self, ValidationError> {
        if id == 0 {
            return Err(ValidationError::Invalid {
                field: "RegionId",
                reason: "RegionId must be positive (> 0)",
            });
        }
        Ok(Self(id))
    }

    /// Creates a RegionId without validation (use with caution).
    ///
    /// # Safety
    /// Caller must ensure the ID is valid (> 0).
    pub fn new_unchecked(id: u32) -> Self {
        Self(id)
    }

    /// Returns the region ID value.
    pub fn value(self) -> u32 {
        self.0
    }

    /// Returns the region ID as usize.
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for RegionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<RegionId> for u32 {
    fn from(id: RegionId) -> u32 {
        id.0
    }
}

impl TryFrom<u32> for RegionId {
    type Error = ValidationError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

// ============================================================================
// CellAddress - Address of a cell in A1 notation
// ============================================================================

/// Address of a cell in A1 notation (e.g., "A1", "Z100", "AA42").
///
/// Prevents invalid cell references at compile time.
///
/// # Validation
/// - Must be valid A1 notation (letter(s) followed by number(s))
/// - Column: A-XFD (1-16384 in Excel)
/// - Row: 1-1048576 (Excel limit)
///
/// # Example
/// ```rust,ignore
/// let addr = CellAddress::parse("B5")?;
/// assert_eq!(addr.as_str(), "B5");
/// assert_eq!(addr.column(), 2);
/// assert_eq!(addr.row(), 5);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CellAddress(String);

impl CellAddress {
    const MAX_COLUMN: u32 = 16384; // Excel XFD
    const MAX_ROW: u32 = 1048576; // Excel limit

    /// Parses a cell address from A1 notation.
    ///
    /// # Errors
    /// Returns `Err` if the address is not valid A1 notation.
    ///
    /// # Example
    /// ```rust,ignore
    /// let addr = CellAddress::parse("A1")?;
    /// assert_eq!(addr.column(), 1);
    /// assert_eq!(addr.row(), 1);
    /// ```
    pub fn parse(s: &str) -> Result<Self, ValidationError> {
        // Find the split between letters and numbers
        let split_idx =
            s.find(|c: char| c.is_ascii_digit())
                .ok_or_else(|| ValidationError::Invalid {
                    field: "CellAddress",
                    reason: "must contain row number",
                })?;

        if split_idx == 0 {
            return Err(ValidationError::Invalid {
                field: "CellAddress",
                reason: "must start with column letter(s)",
            });
        }

        let (col_str, row_str) = s.split_at(split_idx);

        // Parse row
        let row = row_str
            .parse::<u32>()
            .map_err(|_| ValidationError::Invalid {
                field: "CellAddress",
                reason: "invalid row number",
            })?;

        if row == 0 || row > Self::MAX_ROW {
            return Err(ValidationError::OutOfRange {
                field: "CellAddress row",
                min: 1,
                max: Self::MAX_ROW,
                actual: row,
            });
        }

        // Parse column
        let col = Self::column_from_letters(col_str)?;

        if col == 0 || col > Self::MAX_COLUMN {
            return Err(ValidationError::OutOfRange {
                field: "CellAddress column",
                min: 1,
                max: Self::MAX_COLUMN,
                actual: col,
            });
        }

        Ok(Self(s.to_uppercase()))
    }

    /// Creates a CellAddress from column and row indices.
    ///
    /// # Errors
    /// Returns `Err` if column or row is out of valid range.
    pub fn from_indices(col: u32, row: u32) -> Result<Self, ValidationError> {
        if col == 0 || col > Self::MAX_COLUMN {
            return Err(ValidationError::OutOfRange {
                field: "CellAddress column",
                min: 1,
                max: Self::MAX_COLUMN,
                actual: col,
            });
        }
        if row == 0 || row > Self::MAX_ROW {
            return Err(ValidationError::OutOfRange {
                field: "CellAddress row",
                min: 1,
                max: Self::MAX_ROW,
                actual: row,
            });
        }

        let col_str = Self::column_to_letters(col);
        Ok(Self(format!("{}{}", col_str, row)))
    }

    /// Returns the cell address as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the column index (1-based).
    pub fn column(&self) -> u32 {
        let split_idx = self.0.find(|c: char| c.is_ascii_digit()).unwrap();
        let col_str = &self.0[..split_idx];
        Self::column_from_letters(col_str).unwrap()
    }

    /// Returns the row index (1-based).
    pub fn row(&self) -> u32 {
        let split_idx = self.0.find(|c: char| c.is_ascii_digit()).unwrap();
        let row_str = &self.0[split_idx..];
        row_str.parse().unwrap()
    }

    /// Converts column letters to column index (1-based).
    fn column_from_letters(s: &str) -> Result<u32, ValidationError> {
        let mut col = 0u32;
        for ch in s.chars() {
            if !ch.is_ascii_alphabetic() {
                return Err(ValidationError::Invalid {
                    field: "CellAddress column",
                    reason: "must contain only letters",
                });
            }
            let digit = ch.to_ascii_uppercase() as u32 - 'A' as u32 + 1;
            col = col
                .checked_mul(26)
                .and_then(|c| c.checked_add(digit))
                .ok_or_else(|| ValidationError::Invalid {
                    field: "CellAddress column",
                    reason: "column overflow",
                })?;
        }
        Ok(col)
    }

    /// Converts column index to letters (1-based).
    fn column_to_letters(mut col: u32) -> String {
        let mut result = String::new();
        while col > 0 {
            col -= 1;
            let remainder = (col % 26) as u8;
            result.insert(0, (b'A' + remainder) as char);
            col /= 26;
        }
        result
    }
}

impl fmt::Display for CellAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for CellAddress {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl AsRef<str> for CellAddress {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ============================================================================
// ValidationError - Errors for value object validation
// ============================================================================

/// Validation errors for domain value objects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Field is empty but must not be.
    Empty(&'static str),

    /// Field exceeds maximum length.
    TooLong {
        field: &'static str,
        max: usize,
        actual: usize,
    },

    /// Field contains invalid character.
    InvalidCharacter {
        field: &'static str,
        character: char,
    },

    /// Field value is invalid for specified reason.
    Invalid {
        field: &'static str,
        reason: &'static str,
    },

    /// Field value is out of valid range.
    OutOfRange {
        field: &'static str,
        min: u32,
        max: u32,
        actual: u32,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::Empty(field) => write!(f, "{} cannot be empty", field),
            ValidationError::TooLong { field, max, actual } => {
                write!(f, "{} too long (max: {}, actual: {})", field, max, actual)
            }
            ValidationError::InvalidCharacter { field, character } => {
                write!(f, "{} contains invalid character: '{}'", field, character)
            }
            ValidationError::Invalid { field, reason } => {
                write!(f, "{} is invalid: {}", field, reason)
            }
            ValidationError::OutOfRange {
                field,
                min,
                max,
                actual,
            } => {
                write!(
                    f,
                    "{} out of range (min: {}, max: {}, actual: {})",
                    field, min, max, actual
                )
            }
        }
    }
}

impl std::error::Error for ValidationError {}

// ============================================================================
// Legacy Value Objects (preserved for compatibility)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OntologyId {
    pub id: String,
}

impl OntologyId {
    pub fn new(id: String) -> Self {
        assert!(!id.is_empty(), "OntologyId cannot be empty");
        Self { id }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReceiptId {
    pub receipt_id: String,
}

impl ReceiptId {
    pub fn new(receipt_id: String) -> Self {
        assert!(!receipt_id.is_empty(), "ReceiptId cannot be empty");
        Self { receipt_id }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workbook_id_validation() {
        assert!(WorkbookId::new("valid-id".to_string()).is_ok());
        assert!(WorkbookId::new("".to_string()).is_err());
        assert!(WorkbookId::new("a".repeat(2000)).is_err());
    }

    #[test]
    fn test_fork_id_validation() {
        assert!(ForkId::new("fork123".to_string()).is_ok());
        assert!(ForkId::new("".to_string()).is_err());
        assert!(ForkId::new("a".repeat(300)).is_err());
    }

    #[test]
    fn test_sheet_name_validation() {
        assert!(SheetName::new("Sheet1".to_string()).is_ok());
        assert!(SheetName::new("Q1 Revenue".to_string()).is_ok());
        assert!(SheetName::new("".to_string()).is_err());
        assert!(SheetName::new("Invalid[Sheet]".to_string()).is_err());
        assert!(SheetName::new("Invalid:Sheet".to_string()).is_err());
        assert!(SheetName::new("Invalid*Sheet".to_string()).is_err());
        assert!(SheetName::new("Invalid?Sheet".to_string()).is_err());
        assert!(SheetName::new("Invalid/Sheet".to_string()).is_err());
        assert!(SheetName::new("Invalid\\Sheet".to_string()).is_err());
    }

    #[test]
    fn test_region_id_validation() {
        assert!(RegionId::new(1).is_ok());
        assert!(RegionId::new(42).is_ok());
        assert!(RegionId::new(0).is_err());
    }

    #[test]
    fn test_cell_address_parsing() {
        let addr = CellAddress::parse("A1").unwrap();
        assert_eq!(addr.column(), 1);
        assert_eq!(addr.row(), 1);
        assert_eq!(addr.as_str(), "A1");

        let addr = CellAddress::parse("Z99").unwrap();
        assert_eq!(addr.column(), 26);
        assert_eq!(addr.row(), 99);

        let addr = CellAddress::parse("AA1").unwrap();
        assert_eq!(addr.column(), 27);
        assert_eq!(addr.row(), 1);

        let addr = CellAddress::parse("ab10").unwrap(); // lowercase should work
        assert_eq!(addr.as_str(), "AB10"); // normalized to uppercase
        assert_eq!(addr.column(), 28);
        assert_eq!(addr.row(), 10);
    }

    #[test]
    fn test_cell_address_from_indices() {
        let addr = CellAddress::from_indices(1, 1).unwrap();
        assert_eq!(addr.as_str(), "A1");

        let addr = CellAddress::from_indices(26, 99).unwrap();
        assert_eq!(addr.as_str(), "Z99");

        let addr = CellAddress::from_indices(27, 1).unwrap();
        assert_eq!(addr.as_str(), "AA1");

        assert!(CellAddress::from_indices(0, 1).is_err());
        assert!(CellAddress::from_indices(1, 0).is_err());
        assert!(CellAddress::from_indices(20000, 1).is_err());
        assert!(CellAddress::from_indices(1, 2000000).is_err());
    }

    #[test]
    fn test_cell_address_invalid() {
        assert!(CellAddress::parse("").is_err());
        assert!(CellAddress::parse("1").is_err());
        assert!(CellAddress::parse("A").is_err());
        assert!(CellAddress::parse("A0").is_err());
        assert!(CellAddress::parse("1A").is_err());
    }

    #[test]
    fn test_type_safety() {
        let workbook_id = WorkbookId::new("wb1".to_string()).unwrap();
        let fork_id = ForkId::new("fork1".to_string()).unwrap();

        // These are different types, so the following won't compile:
        // let _: WorkbookId = fork_id; // Compile error!
        // let _: ForkId = workbook_id; // Compile error!

        // But we can compare the underlying strings if needed:
        assert_ne!(workbook_id.as_str(), fork_id.as_str());
    }

    #[test]
    fn test_serde_serialization() {
        let workbook_id = WorkbookId::new("wb123".to_string()).unwrap();
        let json = serde_json::to_string(&workbook_id).unwrap();
        assert_eq!(json, "\"wb123\"");

        let deserialized: WorkbookId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, workbook_id);

        let addr = CellAddress::parse("B5").unwrap();
        let json = serde_json::to_string(&addr).unwrap();
        assert_eq!(json, "\"B5\"");

        let deserialized: CellAddress = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, addr);
    }
}
