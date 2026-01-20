//! Integration examples and utilities for JSON schema validation with rmcp
//!
//! This module provides examples and utilities for integrating JSON schema validation
//! with the rmcp MCP server framework.

use crate::tools;
use crate::validation::{SchemaValidationMiddleware, SchemaValidator, SchemaValidatorBuilder};
use std::sync::Arc;

/// Create a fully configured schema validator with all tool schemas registered
///
/// This function registers all tool parameter schemas from the application.
/// It should be called once at server startup to create a shared validator.
///
/// # Example
///
/// ```rust,ignore
/// use crate::validation::integration::create_configured_validator;
///
/// let validator = create_configured_validator();
/// let middleware = SchemaValidationMiddleware::new(Arc::new(validator));
/// ```
pub fn create_configured_validator() -> SchemaValidator {
    let validator = SchemaValidatorBuilder::new()
        // Core spreadsheet tools
        .register::<tools::ListWorkbooksParams>("list_workbooks")
        .register::<tools::DescribeWorkbookParams>("describe_workbook")
        .register::<tools::ListSheetsParams>("list_sheets")
        .register::<tools::SheetOverviewParams>("sheet_overview")
        .register::<tools::SheetPageParams>("sheet_page")
        .register::<tools::FindValueParams>("find_value")
        .register::<tools::ReadTableParams>("read_table")
        .register::<tools::TableProfileParams>("table_profile")
        .register::<tools::RangeValuesParams>("range_values")
        .register::<tools::SheetStatisticsParams>("sheet_statistics")
        .register::<tools::SheetFormulaMapParams>("sheet_formula_map")
        .register::<tools::FormulaTraceParams>("formula_trace")
        .register::<tools::NamedRangesParams>("named_ranges")
        .register::<tools::FindFormulaParams>("find_formula")
        .register::<tools::ScanVolatilesParams>("scan_volatiles")
        .register::<tools::SheetStylesParams>("sheet_styles")
        .register::<tools::WorkbookStyleSummaryParams>("workbook_style_summary")
        .register::<tools::ManifestStubParams>("get_manifest_stub")
        .register::<tools::CloseWorkbookParams>("close_workbook")
        .register::<tools::WorkbookSummaryParams>("workbook_summary")
        .build();

    validator
}

/// Create a fully configured schema validator including VBA tools
///
/// This variant includes VBA-specific tool schemas.
///
/// # Example
///
/// ```rust,ignore
/// use crate::validation::integration::create_configured_validator_with_vba;
///
/// let validator = create_configured_validator_with_vba();
/// ```
#[cfg(feature = "vba")]
pub fn create_configured_validator_with_vba() -> SchemaValidator {
    let validator = SchemaValidatorBuilder::new()
        // Core spreadsheet tools
        .register::<tools::ListWorkbooksParams>("list_workbooks")
        .register::<tools::DescribeWorkbookParams>("describe_workbook")
        .register::<tools::ListSheetsParams>("list_sheets")
        .register::<tools::SheetOverviewParams>("sheet_overview")
        .register::<tools::SheetPageParams>("sheet_page")
        .register::<tools::FindValueParams>("find_value")
        .register::<tools::ReadTableParams>("read_table")
        .register::<tools::TableProfileParams>("table_profile")
        .register::<tools::RangeValuesParams>("range_values")
        .register::<tools::SheetStatisticsParams>("sheet_statistics")
        .register::<tools::SheetFormulaMapParams>("sheet_formula_map")
        .register::<tools::FormulaTraceParams>("formula_trace")
        .register::<tools::NamedRangesParams>("named_ranges")
        .register::<tools::FindFormulaParams>("find_formula")
        .register::<tools::ScanVolatilesParams>("scan_volatiles")
        .register::<tools::SheetStylesParams>("sheet_styles")
        .register::<tools::WorkbookStyleSummaryParams>("workbook_style_summary")
        .register::<tools::ManifestStubParams>("get_manifest_stub")
        .register::<tools::CloseWorkbookParams>("close_workbook")
        .register::<tools::WorkbookSummaryParams>("workbook_summary")
        // VBA tools
        .register::<tools::vba::VbaProjectSummaryParams>("vba_project_summary")
        .register::<tools::vba::VbaModuleSourceParams>("vba_module_source")
        .build();

    validator
}

/// Create a fully configured schema validator including fork/recalc tools
///
/// This variant includes fork and recalculation tool schemas.
///
/// # Example
///
/// ```rust,ignore
/// use crate::validation::integration::create_configured_validator_with_recalc;
///
/// let validator = create_configured_validator_with_recalc();
/// ```
#[cfg(feature = "recalc")]
pub fn create_configured_validator_with_recalc() -> SchemaValidator {
    let validator = SchemaValidatorBuilder::new()
        // Core spreadsheet tools
        .register::<tools::ListWorkbooksParams>("list_workbooks")
        .register::<tools::DescribeWorkbookParams>("describe_workbook")
        .register::<tools::ListSheetsParams>("list_sheets")
        .register::<tools::SheetOverviewParams>("sheet_overview")
        .register::<tools::SheetPageParams>("sheet_page")
        .register::<tools::FindValueParams>("find_value")
        .register::<tools::ReadTableParams>("read_table")
        .register::<tools::TableProfileParams>("table_profile")
        .register::<tools::RangeValuesParams>("range_values")
        .register::<tools::SheetStatisticsParams>("sheet_statistics")
        .register::<tools::SheetFormulaMapParams>("sheet_formula_map")
        .register::<tools::FormulaTraceParams>("formula_trace")
        .register::<tools::NamedRangesParams>("named_ranges")
        .register::<tools::FindFormulaParams>("find_formula")
        .register::<tools::ScanVolatilesParams>("scan_volatiles")
        .register::<tools::SheetStylesParams>("sheet_styles")
        .register::<tools::WorkbookStyleSummaryParams>("workbook_style_summary")
        .register::<tools::ManifestStubParams>("get_manifest_stub")
        .register::<tools::CloseWorkbookParams>("close_workbook")
        .register::<tools::WorkbookSummaryParams>("workbook_summary")
        // Fork/recalc tools
        .register::<tools::fork::CreateForkParams>("create_fork")
        .register::<tools::fork::EditBatchParams>("edit_batch")
        .register::<tools::fork::TransformBatchParams>("transform_batch")
        .register::<tools::fork::StyleBatchParams>("style_batch")
        .register::<tools::fork::ApplyFormulaPatternParams>("apply_formula_pattern")
        .register::<tools::fork::StructureBatchParams>("structure_batch")
        .register::<tools::fork::GetEditsParams>("get_edits")
        .register::<tools::fork::GetChangesetParams>("get_changeset")
        .register::<tools::fork::RecalculateParams>("recalculate")
        .register::<tools::fork::ListForksParams>("list_forks")
        .register::<tools::fork::DiscardForkParams>("discard_fork")
        .register::<tools::fork::SaveForkParams>("save_fork")
        .register::<tools::fork::CheckpointForkParams>("checkpoint_fork")
        .register::<tools::fork::ListCheckpointsParams>("list_checkpoints")
        .register::<tools::fork::RestoreCheckpointParams>("restore_checkpoint")
        .register::<tools::fork::DeleteCheckpointParams>("delete_checkpoint")
        .register::<tools::fork::ListStagedChangesParams>("list_staged_changes")
        .register::<tools::fork::ApplyStagedChangeParams>("apply_staged_change")
        .register::<tools::fork::DiscardStagedChangeParams>("discard_staged_change")
        .register::<tools::fork::ScreenshotSheetParams>("screenshot_sheet")
        .build();

    validator
}

/// Create a fully configured schema validator with all available tools
///
/// This variant includes all tool schemas based on enabled features.
///
/// # Example
///
/// ```rust,ignore
/// use crate::validation::integration::create_full_validator;
///
/// let validator = create_full_validator();
/// let middleware = SchemaValidationMiddleware::new(Arc::new(validator));
/// ```
pub fn create_full_validator() -> SchemaValidator {
    let mut builder = SchemaValidatorBuilder::new()
        // Core spreadsheet tools
        .register::<tools::ListWorkbooksParams>("list_workbooks")
        .register::<tools::DescribeWorkbookParams>("describe_workbook")
        .register::<tools::ListSheetsParams>("list_sheets")
        .register::<tools::SheetOverviewParams>("sheet_overview")
        .register::<tools::SheetPageParams>("sheet_page")
        .register::<tools::FindValueParams>("find_value")
        .register::<tools::ReadTableParams>("read_table")
        .register::<tools::TableProfileParams>("table_profile")
        .register::<tools::RangeValuesParams>("range_values")
        .register::<tools::SheetStatisticsParams>("sheet_statistics")
        .register::<tools::SheetFormulaMapParams>("sheet_formula_map")
        .register::<tools::FormulaTraceParams>("formula_trace")
        .register::<tools::NamedRangesParams>("named_ranges")
        .register::<tools::FindFormulaParams>("find_formula")
        .register::<tools::ScanVolatilesParams>("scan_volatiles")
        .register::<tools::SheetStylesParams>("sheet_styles")
        .register::<tools::WorkbookStyleSummaryParams>("workbook_style_summary")
        .register::<tools::ManifestStubParams>("get_manifest_stub")
        .register::<tools::CloseWorkbookParams>("close_workbook")
        .register::<tools::WorkbookSummaryParams>("workbook_summary");

    // Conditionally add VBA tools
    builder = builder
        .register::<tools::vba::VbaProjectSummaryParams>("vba_project_summary")
        .register::<tools::vba::VbaModuleSourceParams>("vba_module_source");

    // Conditionally add fork/recalc tools
    #[cfg(feature = "recalc")]
    {
        builder = builder
            .register::<tools::fork::CreateForkParams>("create_fork")
            .register::<tools::fork::EditBatchParams>("edit_batch")
            .register::<tools::fork::TransformBatchParams>("transform_batch")
            .register::<tools::fork::StyleBatchParams>("style_batch")
            .register::<tools::fork::ApplyFormulaPatternParams>("apply_formula_pattern")
            .register::<tools::fork::StructureBatchParams>("structure_batch")
            .register::<tools::fork::GetEditsParams>("get_edits")
            .register::<tools::fork::GetChangesetParams>("get_changeset")
            .register::<tools::fork::RecalculateParams>("recalculate")
            .register::<tools::fork::ListForksParams>("list_forks")
            .register::<tools::fork::DiscardForkParams>("discard_fork")
            .register::<tools::fork::SaveForkParams>("save_fork")
            .register::<tools::fork::CheckpointForkParams>("checkpoint_fork")
            .register::<tools::fork::ListCheckpointsParams>("list_checkpoints")
            .register::<tools::fork::RestoreCheckpointParams>("restore_checkpoint")
            .register::<tools::fork::DeleteCheckpointParams>("delete_checkpoint")
            .register::<tools::fork::ListStagedChangesParams>("list_staged_changes")
            .register::<tools::fork::ApplyStagedChangeParams>("apply_staged_change")
            .register::<tools::fork::DiscardStagedChangeParams>("discard_staged_change")
            .register::<tools::fork::ScreenshotSheetParams>("screenshot_sheet");
    }

    builder.build()
}

/// Create a validation middleware with all tools registered
///
/// This is a convenience function that creates a fully configured middleware
/// ready to use for validating tool calls.
///
/// # Example
///
/// ```rust,ignore
/// use crate::validation::integration::create_validation_middleware;
///
/// let middleware = create_validation_middleware();
///
/// // In a tool handler:
/// middleware.validate_tool_call("list_workbooks", &params)?;
/// ```
pub fn create_validation_middleware() -> SchemaValidationMiddleware {
    let validator = Arc::new(create_full_validator());
    SchemaValidationMiddleware::new(validator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_configured_validator() {
        let validator = create_configured_validator();
        // Test that a core tool is registered
        let params = json!({
            "slug_prefix": "test"
        });
        assert!(validator.validate("list_workbooks", &params).is_ok());
    }

    #[test]
    fn test_create_full_validator() {
        let validator = create_full_validator();
        // Test that a core tool is registered
        let params = json!({
            "workbook_or_fork_id": "test-id"
        });
        assert!(validator.validate("describe_workbook", &params).is_ok());
    }

    #[cfg(feature = "recalc")]
    #[test]
    fn test_create_validator_with_recalc() {
        let validator = create_configured_validator_with_recalc();
        // Test that a fork tool is registered
        let params = json!({
            "workbook_id": "test-id"
        });
        assert!(validator.validate("create_fork", &params).is_ok());
    }

    #[test]
    fn test_create_validation_middleware() {
        let middleware = create_validation_middleware();
        // Just ensure it creates without panic
        drop(middleware);
    }
}
