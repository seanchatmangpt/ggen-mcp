// Example: Integrating Input Validation Guards into Tool Handlers
//
// This file shows concrete examples of how to integrate the validation guards
// into your tool handlers following the poka-yoke (mistake-proofing) pattern.
//
// NOTE: This is a reference implementation for documentation purposes.
// These examples should be adapted and integrated into the actual tool handlers
// in src/tools/mod.rs and src/generated/mcp_tools.rs

use crate::model::*;
use crate::state::AppState;
use crate::validation::*;
use anyhow::{Result, anyhow};
use std::sync::Arc;

// ============================================================================
// Example 1: Basic String Validation
// ============================================================================

/// Example: sheet_overview with comprehensive validation
pub async fn sheet_overview_validated(
    state: Arc<AppState>,
    params: SheetOverviewParams,
) -> Result<SheetOverviewResponse> {
    // Validate workbook ID - ensures it's not empty and contains safe characters
    validate_workbook_id(params.workbook_or_fork_id.as_str())
        .map_err(|e| anyhow!("Invalid workbook_id: {}", e))?;

    // Validate sheet name - ensures it follows Excel naming rules
    validate_sheet_name(&params.sheet_name)
        .map_err(|e| anyhow!("Invalid sheet_name: {}", e))?;

    // Validate numeric ranges for optional parameters
    let max_regions = validate_optional_numeric_range(
        "max_regions",
        params.max_regions,
        1u32,
        1000u32,
    )
    .map_err(|e| anyhow!(e))?;

    let max_headers = validate_optional_numeric_range(
        "max_headers",
        params.max_headers,
        1u32,
        500u32,
    )
    .map_err(|e| anyhow!(e))?;

    // All parameters validated - proceed with business logic
    let workbook = state.open_workbook(&params.workbook_or_fork_id).await?;
    let sheet_name = params.sheet_name.clone();

    let mut overview =
        tokio::task::spawn_blocking(move || workbook.sheet_overview(&sheet_name)).await??;

    // ... rest of implementation
    Ok(overview)
}

// ============================================================================
// Example 2: Path Validation
// ============================================================================

#[cfg(feature = "recalc")]
pub async fn save_fork_validated(
    state: Arc<AppState>,
    params: SaveForkParams,
) -> Result<SaveForkResponse> {
    // Validate fork ID
    validate_workbook_id(&params.fork_id)
        .map_err(|e| anyhow!("Invalid fork_id: {}", e))?;

    // Validate target path - prevents path traversal attacks
    validate_path_safe(&params.target_path)
        .map_err(|e| anyhow!("Invalid target_path: {}", e))?;

    // Proceed with validated path
    // ... rest of save_fork implementation

    Ok(SaveForkResponse {
        fork_id: params.fork_id.clone(),
        saved_path: params.target_path.clone(),
        message: "Fork saved successfully".to_string(),
    })
}

// ============================================================================
// Example 3: Cell Address and Range Validation
// ============================================================================

pub async fn formula_trace_validated(
    state: Arc<AppState>,
    params: FormulaTraceParams,
) -> Result<FormulaTraceResponse> {
    // Validate workbook ID
    validate_workbook_id(params.workbook_or_fork_id.as_str())
        .map_err(|e| anyhow!("Invalid workbook_id: {}", e))?;

    // Validate sheet name
    validate_sheet_name(&params.sheet_name)
        .map_err(|e| anyhow!("Invalid sheet_name: {}", e))?;

    // Validate cell address (A1 notation)
    validate_cell_address(&params.cell_address)
        .map_err(|e| anyhow!("Invalid cell_address: {}", e))?;

    // Validate numeric parameters
    let depth = validate_optional_numeric_range(
        "depth",
        params.depth,
        1u32,
        10u32, // Reasonable depth limit
    )
    .map_err(|e| anyhow!(e))?;

    let limit = validate_optional_numeric_range(
        "limit",
        params.limit,
        1u32,
        1000u32,
    )
    .map_err(|e| anyhow!(e))?;

    // ... rest of formula_trace implementation
    unimplemented!("Example only")
}

// ============================================================================
// Example 4: Range String Validation
// ============================================================================

pub async fn read_table_validated(
    state: Arc<AppState>,
    params: ReadTableParams,
) -> Result<ReadTableResponse> {
    // Validate workbook ID
    validate_workbook_id(params.workbook_or_fork_id.as_str())
        .map_err(|e| anyhow!("Invalid workbook_id: {}", e))?;

    // Validate optional sheet name
    if let Some(ref sheet_name) = params.sheet_name {
        validate_non_empty_string("sheet_name", sheet_name)
            .map_err(|e| anyhow!(e))?;
        validate_sheet_name(sheet_name)
            .map_err(|e| anyhow!("Invalid sheet_name: {}", e))?;
    }

    // Validate optional range string
    if let Some(ref range) = params.range {
        validate_range_string(range)
            .map_err(|e| anyhow!("Invalid range: {}", e))?;
    }

    // Validate pagination parameters
    let limit = validate_optional_numeric_range(
        "limit",
        params.limit,
        1u32,
        100000u32,
    )
    .map_err(|e| anyhow!(e))?;

    let offset = validate_optional_numeric_range(
        "offset",
        params.offset,
        0u32,
        u32::MAX,
    )
    .map_err(|e| anyhow!(e))?;

    // ... rest of read_table implementation
    unimplemented!("Example only")
}

// ============================================================================
// Example 5: Batch Validation Helper
// ============================================================================

/// Helper struct for batch validation of common parameters
pub struct ToolParamsValidator;

impl ToolParamsValidator {
    /// Validate common workbook-related parameters
    pub fn validate_workbook_params(
        workbook_id: &str,
        sheet_name: Option<&str>,
    ) -> Result<()> {
        validate_workbook_id(workbook_id)
            .map_err(|e| anyhow!("Invalid workbook_id: {}", e))?;

        if let Some(name) = sheet_name {
            validate_sheet_name(name)
                .map_err(|e| anyhow!("Invalid sheet_name: {}", e))?;
        }

        Ok(())
    }

    /// Validate pagination parameters
    pub fn validate_pagination(
        limit: Option<u32>,
        offset: Option<u32>,
        max_limit: u32,
    ) -> Result<(Option<u32>, Option<u32>)> {
        let validated_limit = validate_optional_numeric_range(
            "limit",
            limit,
            1u32,
            max_limit,
        )
        .map_err(|e| anyhow!(e))?;

        let validated_offset = validate_optional_numeric_range(
            "offset",
            offset,
            0u32,
            u32::MAX,
        )
        .map_err(|e| anyhow!(e))?;

        Ok((validated_limit, validated_offset))
    }

    /// Validate range-related parameters
    pub fn validate_range_params(
        range: Option<&str>,
        region_id: Option<u32>,
    ) -> Result<()> {
        if let Some(r) = range {
            validate_range_string(r)
                .map_err(|e| anyhow!("Invalid range: {}", e))?;
        }

        if let Some(rid) = region_id {
            validate_numeric_range("region_id", rid, 0u32, 10000u32)
                .map_err(|e| anyhow!(e))?;
        }

        Ok(())
    }
}

// ============================================================================
// Example 6: Using the Batch Validator
// ============================================================================

pub async fn sheet_page_validated(
    state: Arc<AppState>,
    params: SheetPageParams,
) -> Result<SheetPageResponse> {
    // Use the batch validator for common parameters
    ToolParamsValidator::validate_workbook_params(
        params.workbook_or_fork_id.as_str(),
        Some(&params.sheet_name),
    )?;

    // Validate pagination
    let (limit, offset) = ToolParamsValidator::validate_pagination(
        Some(params.page_size),
        Some(params.start_row),
        10000u32,
    )?;

    // Validate column names if provided
    if let Some(ref columns) = params.columns {
        for col in columns {
            validate_non_empty_string("column", col)
                .map_err(|e| anyhow!("Invalid column name: {}", e))?;
        }
    }

    // ... rest of sheet_page implementation
    unimplemented!("Example only")
}

// ============================================================================
// Example 7: Integration with Generated Tool Handlers
// ============================================================================

// In src/generated/mcp_tools.rs, you would add validation like this:

use rmcp::{ErrorData as McpError, Json, handler::server::wrapper::Parameters, tool};

#[tool(
    name = "sheet_overview",
    description = "Get narrative overview for a sheet"
)]
pub async fn sheet_overview_handler(
    server: &crate::server::SpreadsheetServer,
    Parameters(params): Parameters<SheetOverviewParams>,
) -> Result<Json<SheetOverviewResponse>, McpError> {
    server
        .ensure_tool_enabled("sheet_overview")
        .map_err(to_mcp_error)?;

    // Add input validation before processing
    validate_workbook_id(params.workbook_or_fork_id.as_str())
        .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

    validate_sheet_name(&params.sheet_name)
        .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

    validate_optional_numeric_range("max_regions", params.max_regions, 1u32, 1000u32)
        .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

    validate_optional_numeric_range("max_headers", params.max_headers, 1u32, 500u32)
        .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

    server
        .run_tool_with_timeout(
            "sheet_overview",
            tools::sheet_overview(server.state.clone(), params.into()),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
}

// ============================================================================
// Example 8: Custom Validation Function
// ============================================================================

/// Example of creating a custom validation function for domain-specific needs
pub fn validate_sample_mode(mode: &str) -> ValidationResult<&str> {
    const VALID_MODES: &[&str] = &["head", "tail", "random", "spread"];

    if !VALID_MODES.contains(&mode) {
        return Err(ValidationError::Generic {
            message: format!(
                "Invalid sample_mode '{}'. Must be one of: {}",
                mode,
                VALID_MODES.join(", ")
            ),
        });
    }

    Ok(mode)
}

// Usage example
pub async fn table_profile_validated(
    state: Arc<AppState>,
    params: TableProfileParams,
) -> Result<TableProfileResponse> {
    // Standard validations
    validate_workbook_id(params.workbook_or_fork_id.as_str())
        .map_err(|e| anyhow!("Invalid workbook_id: {}", e))?;

    if let Some(ref sheet_name) = params.sheet_name {
        validate_sheet_name(sheet_name)
            .map_err(|e| anyhow!("Invalid sheet_name: {}", e))?;
    }

    // Custom domain-specific validation
    if let Some(ref mode) = params.sample_mode {
        validate_sample_mode(mode)
            .map_err(|e| anyhow!(e))?;
    }

    // ... rest of implementation
    unimplemented!("Example only")
}

// ============================================================================
// Helper function for MCP error conversion
// ============================================================================

fn to_mcp_error(error: anyhow::Error) -> McpError {
    // Check if it's a validation error
    if let Some(validation_err) = error.downcast_ref::<ValidationError>() {
        return McpError::invalid_params(validation_err.to_string(), None);
    }

    // Default to internal error for other errors
    McpError::internal_error(error.to_string(), None)
}

// ============================================================================
// Testing Examples
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workbook_params_validation() {
        // Valid parameters
        assert!(ToolParamsValidator::validate_workbook_params(
            "valid-workbook-id",
            Some("ValidSheet")
        )
        .is_ok());

        // Invalid workbook ID
        assert!(ToolParamsValidator::validate_workbook_params(
            "",
            Some("ValidSheet")
        )
        .is_err());

        // Invalid sheet name
        assert!(ToolParamsValidator::validate_workbook_params(
            "valid-workbook-id",
            Some("Sheet[Invalid]")
        )
        .is_err());
    }

    #[test]
    fn test_pagination_validation() {
        // Valid pagination
        let result = ToolParamsValidator::validate_pagination(
            Some(100),
            Some(0),
            1000,
        );
        assert!(result.is_ok());
        let (limit, offset) = result.unwrap();
        assert_eq!(limit, Some(100));
        assert_eq!(offset, Some(0));

        // Limit exceeds maximum
        assert!(ToolParamsValidator::validate_pagination(
            Some(10000),
            Some(0),
            1000,
        )
        .is_err());
    }

    #[test]
    fn test_custom_sample_mode_validation() {
        // Valid modes
        assert!(validate_sample_mode("head").is_ok());
        assert!(validate_sample_mode("tail").is_ok());
        assert!(validate_sample_mode("random").is_ok());
        assert!(validate_sample_mode("spread").is_ok());

        // Invalid mode
        assert!(validate_sample_mode("invalid").is_err());
    }
}
