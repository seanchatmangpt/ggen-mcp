//! Comprehensive serialization patterns for Rust MCP servers
//!
//! This example demonstrates best practices for serialization/deserialization
//! in MCP server implementations, based on patterns from ggen-mcp.
//!
//! Run with: cargo run --example serialization_patterns

use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fmt;

// =============================================================================
// SECTION 1: BASIC PATTERNS
// =============================================================================

/// Example 1.1: Standard parameter struct with required and optional fields
///
/// This is the standard pattern for ALL MCP tool parameters:
/// - Required fields first (no Option)
/// - Optional fields last with #[serde(default)]
/// - All fields have JsonSchema for automatic schema generation
#[derive(Debug, Deserialize, JsonSchema)]
pub struct StandardParams {
    /// Required workbook identifier
    pub workbook_id: String,

    /// Required sheet name
    pub sheet_name: String,

    /// Optional limit (defaults to None if not provided)
    #[serde(default)]
    pub limit: Option<u32>,

    /// Optional offset for pagination
    #[serde(default)]
    pub offset: Option<u32>,
}

/// Example 1.2: Standard response struct
///
/// Response structs should:
/// - Include Serialize (and optionally Deserialize for testing)
/// - Include Clone for sharing across async boundaries
/// - Have consistent structure across all tools
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StandardResponse {
    /// Standard identification fields
    pub workbook_id: String,
    pub workbook_short_id: String,

    /// Tool-specific data
    pub items: Vec<DataItem>,

    /// Pagination metadata
    pub total_count: u32,
    pub has_more: bool,

    /// Skip None values in JSON output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DataItem {
    pub id: String,
    pub value: String,
}

// =============================================================================
// SECTION 2: NEWTYPE WRAPPERS (POKA-YOKE)
// =============================================================================

/// Example 2.1: Transparent NewType wrapper for type safety
///
/// Benefits:
/// - Prevents mixing WorkbookId with plain String
/// - Zero serialization overhead (transparent)
/// - Can add validation and helper methods
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(transparent)]
pub struct WorkbookId(pub String);

impl WorkbookId {
    /// Create a new WorkbookId with validation
    pub fn new(s: String) -> Result<Self, String> {
        if s.is_empty() {
            return Err("WorkbookId cannot be empty".to_string());
        }
        if s.len() > 255 {
            return Err("WorkbookId too long".to_string());
        }
        Ok(Self(s))
    }

    /// Get the inner string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and return inner string
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for WorkbookId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for WorkbookId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Example 2.2: Validated NewType wrapper
///
/// For types that need validation during deserialization
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct ValidatedEmail(String);

impl ValidatedEmail {
    pub fn new(s: String) -> Result<Self, String> {
        if !s.contains('@') || !s.contains('.') {
            return Err("Invalid email format".to_string());
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// Custom deserialize with validation
impl<'de> Deserialize<'de> for ValidatedEmail {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        ValidatedEmail::new(s).map_err(serde::de::Error::custom)
    }
}

// =============================================================================
// SECTION 3: ENUMS AND VARIANTS
// =============================================================================

/// Example 3.1: Simple enum with snake_case serialization
///
/// Standard pattern for classification enums
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SheetClassification {
    Data,
    Calculator,
    Mixed,
    Metadata,
    Empty,
}

impl fmt::Display for SheetClassification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Data => write!(f, "data"),
            Self::Calculator => write!(f, "calculator"),
            Self::Mixed => write!(f, "mixed"),
            Self::Metadata => write!(f, "metadata"),
            Self::Empty => write!(f, "empty"),
        }
    }
}

/// Example 3.2: Enum with custom variant names
///
/// Use when JSON names differ from Rust names
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum RegionKind {
    #[serde(rename = "likely_table")]
    Table,
    #[serde(rename = "likely_data")]
    Data,
    #[serde(rename = "likely_parameters")]
    Parameters,
    #[serde(rename = "likely_outputs")]
    Outputs,
    #[serde(rename = "unknown")]
    Other,
}

/// Example 3.3: Internally tagged enum with content
///
/// Produces clean JSON with explicit type discrimination:
/// {"kind": "Text", "value": "hello"}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value")]
pub enum CellValue {
    Text(String),
    Number(f64),
    Bool(bool),
    Error(String),
    Date(String),
}

/// Example 3.4: Adjacently tagged enum
///
/// Flattens variant fields into parent object:
/// {"kind": "pattern", "pattern_type": "solid", ...}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FillDescriptor {
    Pattern(PatternFill),
    Gradient(GradientFill),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PatternFill {
    pub pattern_type: String,
    pub foreground_color: Option<String>,
    pub background_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GradientFill {
    pub degree: f64,
    pub stops: Vec<GradientStop>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GradientStop {
    pub position: f64,
    pub color: String,
}

// =============================================================================
// SECTION 4: FIELD ATTRIBUTES
// =============================================================================

/// Example 4.1: Field aliases for backwards compatibility
#[derive(Debug, Deserialize, JsonSchema)]
pub struct BackwardsCompatibleParams {
    /// Accepts both "workbook_or_fork_id" and "workbook_id"
    #[serde(alias = "workbook_id")]
    pub workbook_or_fork_id: String,

    /// Accepts both "sheet_name" and "sheetName"
    #[serde(alias = "sheetName")]
    pub sheet_name: String,
}

/// Example 4.2: Conditional serialization
#[derive(Debug, Serialize, JsonSchema)]
pub struct ConditionalResponse {
    pub always_included: String,

    /// Only included if not None
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_field: Option<String>,

    /// Only included if not empty
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,

    /// Only included if true
    #[serde(skip_serializing_if = "is_false")]
    pub flag: bool,
}

fn is_false(b: &bool) -> bool {
    !b
}

/// Example 4.3: Default values
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ParamsWithDefaults {
    pub required_field: String,

    /// Uses Default::default() if not provided
    #[serde(default)]
    pub optional_with_default: String,

    /// Uses custom default function
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,

    /// Uses custom default function
    #[serde(default = "default_retries")]
    pub max_retries: u32,
}

fn default_timeout() -> u64 {
    30_000
}

fn default_retries() -> u32 {
    3
}

// =============================================================================
// SECTION 5: VALIDATION PATTERNS
// =============================================================================

/// Example 5.1: Post-deserialization validation
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ValidatedParams {
    pub workbook_id: String,
    pub sheet_name: String,

    #[serde(default)]
    pub limit: Option<u32>,

    #[serde(default)]
    pub offset: Option<u32>,
}

impl ValidatedParams {
    /// Validate parameters after deserialization
    pub fn validate(&self) -> Result<(), String> {
        // Validate workbook_id
        if self.workbook_id.is_empty() {
            return Err("workbook_id cannot be empty".to_string());
        }

        // Validate sheet_name
        if self.sheet_name.is_empty() {
            return Err("sheet_name cannot be empty".to_string());
        }

        // Validate limit
        if let Some(limit) = self.limit {
            if limit == 0 {
                return Err("limit must be greater than 0".to_string());
            }
            if limit > 10_000 {
                return Err("limit cannot exceed 10,000".to_string());
            }
        }

        // Validate offset
        if let Some(offset) = self.offset {
            if offset > 1_000_000 {
                return Err("offset too large".to_string());
            }
        }

        Ok(())
    }
}

/// Example 5.2: Type-safe builder pattern
pub struct QueryBuilder {
    workbook_id: WorkbookId,
    sheet_name: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
}

impl QueryBuilder {
    /// Create builder with required fields
    pub fn new(workbook_id: WorkbookId) -> Self {
        Self {
            workbook_id,
            sheet_name: None,
            limit: None,
            offset: None,
        }
    }

    /// Set sheet name (required before build)
    pub fn sheet_name(mut self, name: String) -> Self {
        self.sheet_name = Some(name);
        self
    }

    /// Set optional limit
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set optional offset
    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Build the query parameters
    pub fn build(self) -> Result<StandardParams, String> {
        let sheet_name = self.sheet_name.ok_or("sheet_name is required")?;

        Ok(StandardParams {
            workbook_id: self.workbook_id.into_string(),
            sheet_name,
            limit: self.limit,
            offset: self.offset,
        })
    }
}

// =============================================================================
// SECTION 6: SCHEMA GENERATION
// =============================================================================

/// Example 6.1: Schema generation with custom attributes
#[derive(Debug, Deserialize, JsonSchema)]
pub struct WellDocumentedParams {
    /// The unique identifier for the workbook
    ///
    /// This can be obtained from the `list_workbooks` tool.
    /// Format: alphanumeric slug with hyphens
    #[schemars(description = "Unique identifier for the workbook")]
    pub workbook_id: String,

    /// The name of the sheet to query
    ///
    /// Sheet names are case-sensitive and may contain spaces.
    #[schemars(description = "Name of the sheet (case-sensitive)")]
    pub sheet_name: String,

    /// Optional limit on the number of rows to return
    ///
    /// If not specified, returns all rows. Maximum value is 10,000.
    #[schemars(description = "Maximum number of rows to return (max: 10,000)")]
    #[serde(default)]
    pub limit: Option<u32>,
}

// =============================================================================
// SECTION 7: ERROR HANDLING
// =============================================================================

/// Example 7.1: Custom error types for validation
#[derive(Debug)]
pub enum ValidationError {
    EmptyField {
        field: String,
    },
    OutOfRange {
        field: String,
        min: i64,
        max: i64,
        actual: i64,
    },
    InvalidFormat {
        field: String,
        expected: String,
        actual: String,
    },
    MissingRequiredField {
        field: String,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyField { field } => {
                write!(f, "Field '{}' cannot be empty", field)
            }
            Self::OutOfRange {
                field,
                min,
                max,
                actual,
            } => {
                write!(
                    f,
                    "Field '{}' out of range: {} not in [{}, {}]",
                    field, actual, min, max
                )
            }
            Self::InvalidFormat {
                field,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Field '{}' has invalid format: expected {}, got {}",
                    field, expected, actual
                )
            }
            Self::MissingRequiredField { field } => {
                write!(f, "Missing required field: {}", field)
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// Example 7.2: Response size validation
pub struct ResponseSizeValidator {
    max_bytes: usize,
}

impl ResponseSizeValidator {
    pub fn new(max_bytes: usize) -> Self {
        Self { max_bytes }
    }

    pub fn validate<T: Serialize>(&self, value: &T, tool_name: &str) -> Result<Vec<u8>, String> {
        let payload = serde_json::to_vec(value)
            .map_err(|e| format!("Failed to serialize {}: {}", tool_name, e))?;

        if payload.len() > self.max_bytes {
            return Err(format!(
                "Response for {} too large: {} bytes (limit: {} bytes)",
                tool_name,
                payload.len(),
                self.max_bytes
            ));
        }

        Ok(payload)
    }
}

// =============================================================================
// SECTION 8: PAGINATION PATTERNS
// =============================================================================

/// Example 8.1: Standard pagination parameters
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PaginatedParams {
    pub workbook_id: String,

    /// Maximum number of items to return
    #[serde(default = "default_page_size")]
    pub limit: u32,

    /// Number of items to skip
    #[serde(default)]
    pub offset: u32,
}

fn default_page_size() -> u32 {
    100
}

/// Example 8.2: Paginated response
#[derive(Debug, Serialize, JsonSchema)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total_count: u32,
    pub limit: u32,
    pub offset: u32,
    pub has_more: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u32>,
}

impl<T> PaginatedResponse<T> {
    pub fn new(items: Vec<T>, total_count: u32, limit: u32, offset: u32) -> Self {
        let has_more = offset + items.len() as u32 > total_count;
        let next_offset = if has_more {
            Some(offset + items.len() as u32)
        } else {
            None
        };

        Self {
            items,
            total_count,
            limit,
            offset,
            has_more,
            next_offset,
        }
    }
}

// =============================================================================
// SECTION 9: COMPLEX NESTED STRUCTURES
// =============================================================================

/// Example 9.1: Nested structure with multiple levels
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WorkbookSummary {
    pub workbook_id: WorkbookId,
    pub sheet_count: usize,
    pub breakdown: WorkbookBreakdown,
    pub region_counts: RegionCountSummary,
    pub entry_points: Vec<EntryPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct WorkbookBreakdown {
    pub data_sheets: u32,
    pub calculator_sheets: u32,
    pub parameter_sheets: u32,
    pub metadata_sheets: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct RegionCountSummary {
    pub data: u32,
    pub parameters: u32,
    pub outputs: u32,
    pub calculator: u32,
    pub metadata: u32,
    pub other: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EntryPoint {
    pub sheet_name: String,
    pub region_id: Option<u32>,
    pub bounds: Option<String>,
    pub rationale: String,
}

// =============================================================================
// SECTION 10: TPS STANDARDIZED WORK
// =============================================================================

/// Example 10.1: Complete tool implementation pattern
///
/// This demonstrates the full standardized work sequence for a tool:
/// 1. Parameter struct with validation
/// 2. Response struct with metadata
/// 3. Tool logic with error handling
/// 4. Size validation
pub struct ToolImplementationExample;

impl ToolImplementationExample {
    /// Execute tool with standardized pattern
    pub async fn execute_tool(params: StandardParams) -> Result<StandardResponse, String> {
        // Step 1: Validate parameters
        Self::validate_params(&params)?;

        // Step 2: Execute business logic
        let items = Self::fetch_data(&params)?;

        // Step 3: Build response
        let response = Self::build_response(params, items);

        // Step 4: Validate response size
        Self::validate_response_size(&response)?;

        Ok(response)
    }

    fn validate_params(params: &StandardParams) -> Result<(), String> {
        if params.workbook_id.is_empty() {
            return Err("workbook_id cannot be empty".to_string());
        }
        if params.sheet_name.is_empty() {
            return Err("sheet_name cannot be empty".to_string());
        }
        if let Some(limit) = params.limit {
            if limit == 0 || limit > 10_000 {
                return Err("limit must be between 1 and 10,000".to_string());
            }
        }
        Ok(())
    }

    fn fetch_data(_params: &StandardParams) -> Result<Vec<DataItem>, String> {
        // Simulate data fetching
        Ok(vec![
            DataItem {
                id: "1".to_string(),
                value: "Value 1".to_string(),
            },
            DataItem {
                id: "2".to_string(),
                value: "Value 2".to_string(),
            },
        ])
    }

    fn build_response(params: StandardParams, items: Vec<DataItem>) -> StandardResponse {
        let total_count = items.len() as u32;
        let has_more = false;

        StandardResponse {
            workbook_id: params.workbook_id,
            workbook_short_id: "short-id".to_string(),
            items,
            total_count,
            has_more,
            next_offset: None,
        }
    }

    fn validate_response_size(response: &StandardResponse) -> Result<(), String> {
        let validator = ResponseSizeValidator::new(5_000_000); // 5MB limit
        validator.validate(response, "example_tool")?;
        Ok(())
    }
}

// =============================================================================
// MAIN FUNCTION - DEMONSTRATIONS
// =============================================================================

fn main() {
    println!("=== Rust MCP Server Serialization Patterns ===\n");

    // Demo 1: Basic serialization
    println!("1. Basic Serialization");
    demo_basic_serialization();
    println!();

    // Demo 2: NewType wrappers
    println!("2. NewType Wrappers");
    demo_newtype_wrappers();
    println!();

    // Demo 3: Enums
    println!("3. Enum Serialization");
    demo_enum_serialization();
    println!();

    // Demo 4: Validation
    println!("4. Validation Patterns");
    demo_validation();
    println!();

    // Demo 5: Schema generation
    println!("5. Schema Generation");
    demo_schema_generation();
    println!();

    // Demo 6: Error handling
    println!("6. Error Handling");
    demo_error_handling();
    println!();

    // Demo 7: Pagination
    println!("7. Pagination");
    demo_pagination();
    println!();

    println!("=== All Demonstrations Complete ===");
}

fn demo_basic_serialization() {
    let params = StandardParams {
        workbook_id: "my-workbook".to_string(),
        sheet_name: "Sheet1".to_string(),
        limit: Some(100),
        offset: None,
    };

    let json = serde_json::to_string_pretty(&json!({
        "workbook_id": params.workbook_id,
        "sheet_name": params.sheet_name,
        "limit": params.limit,
        "offset": params.offset,
    }))
    .unwrap();

    println!("Serialized parameters:");
    println!("{}", json);
}

fn demo_newtype_wrappers() {
    let id = WorkbookId::new("my-workbook".to_string()).unwrap();

    let json = serde_json::to_string(&id).unwrap();
    println!("NewType serialization: {}", json);
    println!("  Note: Transparent serialization produces plain string");

    let deserialized: WorkbookId = serde_json::from_str(&json).unwrap();
    println!("  Deserialized back: {}", deserialized.as_str());
}

fn demo_enum_serialization() {
    let classification = SheetClassification::Calculator;
    let json = serde_json::to_string(&classification).unwrap();
    println!("Enum (rename_all): {}", json);

    let cell_value = CellValue::Text("Hello".to_string());
    let json = serde_json::to_string(&cell_value).unwrap();
    println!("Enum (tagged): {}", json);

    let region = RegionKind::Table;
    let json = serde_json::to_string(&region).unwrap();
    println!("Enum (custom rename): {}", json);
}

fn demo_validation() {
    let params = ValidatedParams {
        workbook_id: "my-workbook".to_string(),
        sheet_name: "Sheet1".to_string(),
        limit: Some(100),
        offset: Some(0),
    };

    match params.validate() {
        Ok(()) => println!("✓ Validation passed"),
        Err(e) => println!("✗ Validation failed: {}", e),
    }

    let invalid_params = ValidatedParams {
        workbook_id: "".to_string(),
        sheet_name: "Sheet1".to_string(),
        limit: None,
        offset: None,
    };

    match invalid_params.validate() {
        Ok(()) => println!("✓ Validation passed"),
        Err(e) => println!("✗ Validation failed: {}", e),
    }
}

fn demo_schema_generation() {
    let schema = schema_for!(WellDocumentedParams);
    let json = serde_json::to_string_pretty(&schema).unwrap();

    println!("Generated JSON Schema:");
    println!("{}", json);
}

fn demo_error_handling() {
    let error = ValidationError::OutOfRange {
        field: "limit".to_string(),
        min: 1,
        max: 10_000,
        actual: 50_000,
    };

    println!("Validation error: {}", error);

    let validator = ResponseSizeValidator::new(1000);
    let large_response = StandardResponse {
        workbook_id: "test".to_string(),
        workbook_short_id: "test".to_string(),
        items: vec![DataItem {
            id: "1".to_string(),
            value: "x".repeat(2000),
        }],
        total_count: 1,
        has_more: false,
        next_offset: None,
    };

    match validator.validate(&large_response, "test_tool") {
        Ok(_) => println!("✓ Response size OK"),
        Err(e) => println!("✗ {}", e),
    }
}

fn demo_pagination() {
    let items = vec![
        DataItem {
            id: "1".to_string(),
            value: "Item 1".to_string(),
        },
        DataItem {
            id: "2".to_string(),
            value: "Item 2".to_string(),
        },
        DataItem {
            id: "3".to_string(),
            value: "Item 3".to_string(),
        },
    ];

    let response = PaginatedResponse::new(items, 10, 3, 0);
    let json = serde_json::to_string_pretty(&response).unwrap();

    println!("Paginated response:");
    println!("{}", json);
}
