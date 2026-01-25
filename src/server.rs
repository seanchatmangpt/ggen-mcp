use crate::config::ServerConfig;
use crate::error::{ErrorCode, McpError as CustomMcpError, to_rmcp_error};
use crate::model::{
    CloseWorkbookResponse, FindFormulaResponse, FindValueResponse, FormulaTraceResponse,
    ManifestStubResponse, NamedRangesResponse, RangeValuesResponse, ReadTableResponse,
    SheetFormulaMapResponse, SheetListResponse, SheetOverviewResponse, SheetPageResponse,
    SheetStatisticsResponse, SheetStylesResponse, TableProfileResponse, VolatileScanResponse,
    WorkbookDescription, WorkbookListResponse, WorkbookStyleSummaryResponse,
    WorkbookSummaryResponse,
};
use crate::state::AppState;
use crate::tools;
use anyhow::{Result, anyhow};
use rmcp::{
    ErrorData as McpError, Json, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::stdio,
};
use serde::Serialize;
use std::future::Future;
use std::sync::Arc;
use thiserror::Error;

const BASE_INSTRUCTIONS: &str = "\
Spreadsheet MCP: optimized for spreadsheet analysis.

WORKFLOW:
1) list_workbooks → list_sheets → workbook_summary for orientation
2) sheet_overview for region detection (ids/bounds/kind/confidence)
3) For structured data: table_profile for quick column sense, then read_table with region_id/range, filters, sampling
4) For spot checks: range_values or find_value (label mode for key-value sheets)

TOOL SELECTION:
- table_profile: Fast column/type summary before wide reads.
- read_table: Structured table extraction. Prefer region_id or tight range; use limit + sample_mode.
- sheet_formula_map: Get formula overview. Use limit param for large sheets (e.g., limit=10). \
Use sort_by='complexity' for most complex formulas first, or 'count' for most repeated. \
Use range param to scope to specific region.
- formula_trace: Trace ONE cell's precedents/dependents. Use AFTER formula_map \
to dive deep on specific outputs (e.g., trace the total cell to understand calc flow).
- sheet_page: Raw cell dump. Use ONLY when region detection fails or for \
unstructured sheets. Prefer read_table for tabular data.
- find_value with mode='label': For key-value layouts (label in col A, value in col B). \
Use direction='right' or 'below' hints.
- find_formula: Search formulas. Default returns no context and only first 50 matches. \
Use include_context=true for header+cell snapshots, and use limit/offset to page.

RANGES: Use A1 notation (e.g., A1:C10). Prefer region_id when available.

DATES: Cells with date formats return ISO-8601 strings (YYYY-MM-DD).

Keep payloads small. Page through large sheets.";

const VBA_INSTRUCTIONS: &str = "

VBA TOOLS (enabled):
Read-only VBA project inspection for .xlsm workbooks.

WORKFLOW:
1) list_workbooks → describe_workbook to find candidate .xlsm
2) vba_project_summary to list modules
3) vba_module_source to page module code

TOOLS:
- vba_project_summary: Parse and summarize the embedded vbaProject.bin (modules + metadata).
- vba_module_source: Return paged source for one module (use offset_lines/limit_lines).

SAFETY:
- Treat VBA as untrusted code. Tools only read and return text.
- Responses are size-limited; page through module source.
";

const WRITE_INSTRUCTIONS: &str = "

WRITE/RECALC TOOLS (enabled):
Fork-based editing allows 'what-if' analysis without modifying original files.

WORKFLOW:
1) create_fork: Create editable copy of a workbook. Returns fork_id.
2) Optional: checkpoint_fork before large edits.
3) edit_batch/transform_batch/style_batch/structure_batch/apply_formula_pattern: Apply edits to the fork.
4) recalculate: Trigger LibreOffice to recompute all formulas.
5) get_changeset: Diff fork against original. Use filters/limit/offset to keep it small.
   Optional: screenshot_sheet to capture a visual view of a range (original or fork).
6) save_fork: Write changes to file.
7) discard_fork: Delete fork without saving.

SAFETY:
- checkpoint_fork before large/structural edits; restore_checkpoint to rollback if needed.
- Tools with mode='preview' create staged changes (transform_batch/style_batch/structure_batch/apply_formula_pattern); use list_staged_changes + apply_staged_change/discard_staged_change.

TOOL DETAILS:
- create_fork: Only .xlsx supported. Returns fork_id for subsequent operations.
- edit_batch: {fork_id, sheet_name, edits:[{address, value, is_formula}]}. \
Formulas should NOT include leading '='.
- transform_batch: Range-first clear/fill/replace. Prefer for bulk edits (blank/fill/rename) to avoid per-cell edit_batch bloat.
- recalculate: Required after edit_batch to update formula results. \
May take several seconds for complex workbooks.
- get_changeset: Returns a paged diff + summary. Use limit/offset to page. \
Use include_types/exclude_types/include_subtypes/exclude_subtypes to filter (e.g. exclude_subtypes=['recalc_result']). \
Use summary_only=true when you only need counts.
- screenshot_sheet: {workbook_or_fork_id, sheet_name, range?}. Renders a cropped PNG for inspecting an area visually.
  workbook_or_fork_id may be either a real workbook_id OR a fork_id (to screenshot an edited fork).
  Returns a file:// URI under workspace_root/screenshots/ (Docker default: /data/screenshots/).
  DO NOT call save_fork just to get a screenshot.
  If formulas changed, run recalculate on the fork first.
- save_fork: Requires target_path for new file location.
  If target_path is relative, it is resolved under workspace_root (Docker default: `/data`).
  Overwriting original requires server --allow-overwrite flag.
  Use drop_fork=false to keep fork active after saving (default: true drops fork).
  Validates base file unchanged since fork creation.
- get_edits: List all edits applied to a fork (before recalculate).
- list_forks: See all active forks.
- checkpoint_fork: Snapshot a fork to a checkpoint for high-fidelity undo.
- list_checkpoints: List checkpoints for a fork.
- restore_checkpoint: Restore a fork to a checkpoint (overwrites fork file; clears newer staged changes).
- delete_checkpoint: Delete a checkpoint.
- list_staged_changes: List staged (previewed) changes for a fork.
- apply_staged_change: Apply a staged change to the fork.
- discard_staged_change: Discard a staged change.

BEST PRACTICES:
- Always recalculate after edit_batch before get_changeset.
- Review changeset before save_fork to verify expected changes.
- Use screenshot_sheet for quick visual inspection; save_fork is ONLY for exporting a workbook file.
- Discard forks when done to free resources (auto-cleanup after 1 hour).
- For large edits, batch multiple cells in single edit_batch call.";

fn build_instructions(recalc_enabled: bool, vba_enabled: bool) -> String {
    let mut instructions = BASE_INSTRUCTIONS.to_string();

    if vba_enabled {
        instructions.push_str(VBA_INSTRUCTIONS);
    } else {
        instructions
            .push_str("\n\nVBA tools disabled. Set SPREADSHEET_MCP_VBA_ENABLED=true to enable.");
    }

    if recalc_enabled {
        instructions.push_str(WRITE_INSTRUCTIONS);
    } else {
        instructions.push_str("\n\nRead-only mode. Write/recalc tools disabled.");
    }
    instructions
}

#[derive(Clone)]
pub struct SpreadsheetServer {
    pub state: Arc<AppState>,
    tool_router: ToolRouter<SpreadsheetServer>,
}

impl SpreadsheetServer {
    pub async fn new(config: Arc<ServerConfig>) -> Result<Self> {
        config.ensure_workspace_root()?;
        let state = Arc::new(AppState::new(config));
        Ok(Self::from_state(state))
    }

    pub fn from_state(state: Arc<AppState>) -> Self {
        #[allow(unused_mut)]
        let mut router = Self::tool_router();

        #[cfg(feature = "recalc")]
        {
            router.merge(Self::fork_tool_router());
        }

        if state.config().vba_enabled {
            router.merge(Self::vba_tool_router());
        }

        // Ontology generation tools (always available)
        router.merge(Self::ontology_tool_router());

        Self {
            state,
            tool_router: router,
        }
    }

    pub async fn run_stdio(self) -> Result<()> {
        let service = self
            .serve(stdio())
            .await
            .inspect_err(|error| tracing::error!("serving error: {:?}", error))?;
        service.waiting().await?;
        Ok(())
    }

    pub async fn run(self) -> Result<()> {
        self.run_stdio().await
    }

    pub fn ensure_tool_enabled(&self, tool: &str) -> Result<()> {
        tracing::info!(tool = tool, "tool invocation requested");
        if self.state.config().is_tool_enabled(tool) {
            Ok(())
        } else {
            Err(ToolDisabledError::new(tool).into())
        }
    }

    fn ensure_vba_enabled(&self, tool: &str) -> Result<()> {
        self.ensure_tool_enabled(tool)?;
        if self.state.config().vba_enabled {
            Ok(())
        } else {
            Err(VbaDisabledError.into())
        }
    }

    #[cfg(feature = "recalc")]
    fn ensure_recalc_enabled(&self, tool: &str) -> Result<()> {
        self.ensure_tool_enabled(tool)?;
        if self.state.config().recalc_enabled {
            Ok(())
        } else {
            Err(RecalcDisabledError.into())
        }
    }

    pub async fn run_tool_with_timeout<T, F>(&self, tool: &str, fut: F) -> Result<T>
    where
        F: Future<Output = Result<T>>,
        T: Serialize,
    {
        use crate::metrics::{METRICS, RequestMetrics, classify_error};

        let metrics = RequestMetrics::new(tool);

        let result = if let Some(timeout_duration) = self.state.config().tool_timeout() {
            match tokio::time::timeout(timeout_duration, fut).await {
                Ok(result) => result,
                Err(_) => {
                    let err = anyhow!(
                        "tool '{}' timed out after {}ms",
                        tool,
                        timeout_duration.as_millis()
                    );
                    metrics.timeout();
                    return Err(err);
                }
            }
        } else {
            fut.await
        };

        match result {
            Ok(response) => {
                if let Err(e) = self.ensure_response_size(tool, &response) {
                    metrics.error(classify_error(&e));
                    return Err(e);
                }
                metrics.success();
                Ok(response)
            }
            Err(e) => {
                metrics.error(classify_error(&e));
                Err(e)
            }
        }
    }

    fn ensure_response_size<T: Serialize>(&self, tool: &str, value: &T) -> Result<()> {
        let Some(limit) = self.state.config().max_response_bytes() else {
            return Ok(());
        };
        let payload = serde_json::to_vec(value)
            .map_err(|e| anyhow!("failed to serialize response for {}: {}", tool, e))?;
        if payload.len() > limit {
            return Err(ResponseTooLargeError::new(tool, payload.len(), limit).into());
        }
        Ok(())
    }
}

#[tool_router]
impl SpreadsheetServer {
    #[tool(
        name = "list_workbooks",
        description = "List spreadsheet files in the workspace"
    )]
    pub async fn list_workbooks(
        &self,
        Parameters(params): Parameters<tools::ListWorkbooksParams>,
    ) -> Result<Json<WorkbookListResponse>, McpError> {
        self.ensure_tool_enabled("list_workbooks")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "list_workbooks",
            tools::list_workbooks(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(name = "describe_workbook", description = "Describe workbook metadata")]
    pub async fn describe_workbook(
        &self,
        Parameters(params): Parameters<tools::DescribeWorkbookParams>,
    ) -> Result<Json<WorkbookDescription>, McpError> {
        self.ensure_tool_enabled("describe_workbook")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "describe_workbook",
            tools::describe_workbook(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "workbook_summary",
        description = "Summarize workbook regions and entry points"
    )]
    pub async fn workbook_summary(
        &self,
        Parameters(params): Parameters<tools::WorkbookSummaryParams>,
    ) -> Result<Json<WorkbookSummaryResponse>, McpError> {
        self.ensure_tool_enabled("workbook_summary")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "workbook_summary",
            tools::workbook_summary(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(name = "list_sheets", description = "List sheets with summaries")]
    pub async fn list_sheets(
        &self,
        Parameters(params): Parameters<tools::ListSheetsParams>,
    ) -> Result<Json<SheetListResponse>, McpError> {
        self.ensure_tool_enabled("list_sheets")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "list_sheets",
            tools::list_sheets(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "sheet_overview",
        description = "Get narrative overview for a sheet"
    )]
    pub async fn sheet_overview(
        &self,
        Parameters(params): Parameters<tools::SheetOverviewParams>,
    ) -> Result<Json<SheetOverviewResponse>, McpError> {
        self.ensure_tool_enabled("sheet_overview")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "sheet_overview",
            tools::sheet_overview(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(name = "sheet_page", description = "Page through sheet cells")]
    pub async fn sheet_page(
        &self,
        Parameters(params): Parameters<tools::SheetPageParams>,
    ) -> Result<Json<SheetPageResponse>, McpError> {
        self.ensure_tool_enabled("sheet_page")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout("sheet_page", tools::sheet_page(self.state.clone(), params))
            .await
            .map(Json)
            .map_err(to_mcp_error)
    }

    #[tool(name = "find_value", description = "Search cell values or labels")]
    pub async fn find_value(
        &self,
        Parameters(params): Parameters<tools::FindValueParams>,
    ) -> Result<Json<FindValueResponse>, McpError> {
        self.ensure_tool_enabled("find_value")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout("find_value", tools::find_value(self.state.clone(), params))
            .await
            .map(Json)
            .map_err(to_mcp_error)
    }

    #[tool(
        name = "read_table",
        description = "Read structured data from a range or table"
    )]
    pub async fn read_table(
        &self,
        Parameters(params): Parameters<tools::ReadTableParams>,
    ) -> Result<Json<ReadTableResponse>, McpError> {
        self.ensure_tool_enabled("read_table")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout("read_table", tools::read_table(self.state.clone(), params))
            .await
            .map(Json)
            .map_err(to_mcp_error)
    }

    #[tool(name = "table_profile", description = "Profile a region or table")]
    pub async fn table_profile(
        &self,
        Parameters(params): Parameters<tools::TableProfileParams>,
    ) -> Result<Json<TableProfileResponse>, McpError> {
        self.ensure_tool_enabled("table_profile")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "table_profile",
            tools::table_profile(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "range_values",
        description = "Fetch raw values for specific ranges"
    )]
    pub async fn range_values(
        &self,
        Parameters(params): Parameters<tools::RangeValuesParams>,
    ) -> Result<Json<RangeValuesResponse>, McpError> {
        self.ensure_tool_enabled("range_values")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "range_values",
            tools::range_values(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "sheet_statistics",
        description = "Get aggregated sheet statistics"
    )]
    pub async fn sheet_statistics(
        &self,
        Parameters(params): Parameters<tools::SheetStatisticsParams>,
    ) -> Result<Json<SheetStatisticsResponse>, McpError> {
        self.ensure_tool_enabled("sheet_statistics")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "sheet_statistics",
            tools::sheet_statistics(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "sheet_formula_map",
        description = "Summarize formula groups across a sheet"
    )]
    pub async fn sheet_formula_map(
        &self,
        Parameters(params): Parameters<tools::SheetFormulaMapParams>,
    ) -> Result<Json<SheetFormulaMapResponse>, McpError> {
        self.ensure_tool_enabled("sheet_formula_map")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "sheet_formula_map",
            tools::sheet_formula_map(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "formula_trace",
        description = "Trace formula precedents or dependents"
    )]
    pub async fn formula_trace(
        &self,
        Parameters(params): Parameters<tools::FormulaTraceParams>,
    ) -> Result<Json<FormulaTraceResponse>, McpError> {
        self.ensure_tool_enabled("formula_trace")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "formula_trace",
            tools::formula_trace(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(name = "named_ranges", description = "List named ranges and tables")]
    pub async fn named_ranges(
        &self,
        Parameters(params): Parameters<tools::NamedRangesParams>,
    ) -> Result<Json<NamedRangesResponse>, McpError> {
        self.ensure_tool_enabled("named_ranges")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "named_ranges",
            tools::named_ranges(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "find_formula",
        description = "Search formulas containing text. Defaults: include_context=false, limit=50; use offset for paging."
    )]
    pub async fn find_formula(
        &self,
        Parameters(params): Parameters<tools::FindFormulaParams>,
    ) -> Result<Json<FindFormulaResponse>, McpError> {
        self.ensure_tool_enabled("find_formula")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "find_formula",
            tools::find_formula(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(name = "scan_volatiles", description = "Scan for volatile formulas")]
    pub async fn scan_volatiles(
        &self,
        Parameters(params): Parameters<tools::ScanVolatilesParams>,
    ) -> Result<Json<VolatileScanResponse>, McpError> {
        self.ensure_tool_enabled("scan_volatiles")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "scan_volatiles",
            tools::scan_volatiles(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "sheet_styles",
        description = "Summarise style usage and properties for a sheet"
    )]
    pub async fn sheet_styles(
        &self,
        Parameters(params): Parameters<tools::SheetStylesParams>,
    ) -> Result<Json<SheetStylesResponse>, McpError> {
        self.ensure_tool_enabled("sheet_styles")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "sheet_styles",
            tools::sheet_styles(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "workbook_style_summary",
        description = "Summarise style usage, theme colors, and conditional formats across a workbook"
    )]
    pub async fn workbook_style_summary(
        &self,
        Parameters(params): Parameters<tools::WorkbookStyleSummaryParams>,
    ) -> Result<Json<WorkbookStyleSummaryResponse>, McpError> {
        self.ensure_tool_enabled("workbook_style_summary")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "workbook_style_summary",
            tools::workbook_style_summary(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "get_manifest_stub",
        description = "Generate manifest scaffold for workbook"
    )]
    pub async fn get_manifest_stub(
        &self,
        Parameters(params): Parameters<tools::ManifestStubParams>,
    ) -> Result<Json<ManifestStubResponse>, McpError> {
        self.ensure_tool_enabled("get_manifest_stub")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "get_manifest_stub",
            tools::get_manifest_stub(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(name = "close_workbook", description = "Evict a workbook from cache")]
    pub async fn close_workbook(
        &self,
        Parameters(params): Parameters<tools::CloseWorkbookParams>,
    ) -> Result<Json<CloseWorkbookResponse>, McpError> {
        self.ensure_tool_enabled("close_workbook")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "close_workbook",
            tools::close_workbook(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }
}

#[tool_router(router = vba_tool_router)]
impl SpreadsheetServer {
    #[tool(
        name = "vba_project_summary",
        description = "Summarize embedded VBA project (xlsm)"
    )]
    pub async fn vba_project_summary(
        &self,
        Parameters(params): Parameters<tools::vba::VbaProjectSummaryParams>,
    ) -> Result<Json<crate::model::VbaProjectSummaryResponse>, McpError> {
        self.ensure_vba_enabled("vba_project_summary")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "vba_project_summary",
            tools::vba::vba_project_summary(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "vba_module_source",
        description = "Read VBA module source (paged)"
    )]
    pub async fn vba_module_source(
        &self,
        Parameters(params): Parameters<tools::vba::VbaModuleSourceParams>,
    ) -> Result<Json<crate::model::VbaModuleSourceResponse>, McpError> {
        self.ensure_vba_enabled("vba_module_source")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "vba_module_source",
            tools::vba::vba_module_source(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }
}

// =============================================================================
// Ontology Generation Tools
// =============================================================================

#[tool_router(router = ontology_tool_router)]
impl SpreadsheetServer {
    #[tool(
        name = "render_template",
        description = "Render a Tera template with context data for code generation"
    )]
    pub async fn render_template(
        &self,
        Parameters(params): Parameters<tools::ontology_generation::RenderTemplateParams>,
    ) -> Result<Json<tools::ontology_generation::RenderTemplateResponse>, McpError> {
        self.ensure_tool_enabled("render_template")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "render_template",
            tools::ontology_generation::render_template(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "write_generated_artifact",
        description = "Write generated code to file with validation and audit trail"
    )]
    pub async fn write_generated_artifact(
        &self,
        Parameters(params): Parameters<tools::ontology_generation::WriteGeneratedArtifactParams>,
    ) -> Result<Json<tools::ontology_generation::WriteGeneratedArtifactResponse>, McpError> {
        self.ensure_tool_enabled("write_generated_artifact")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "write_generated_artifact",
            tools::ontology_generation::write_generated_artifact(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "init_ggen_project",
        description = "Initialize a new ggen project with template scaffolding (rust-mcp-server, api-server, domain-model)"
    )]
    pub async fn init_ggen_project(
        &self,
        Parameters(params): Parameters<tools::ggen_init::InitGgenProjectParams>,
    ) -> Result<Json<tools::ggen_init::InitGgenProjectResponse>, McpError> {
        self.ensure_tool_enabled("init_ggen_project")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "init_ggen_project",
            tools::ggen_init::init_ggen_project(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "read_ggen_config",
        description = "Read and parse ggen.toml configuration, returning structured JSON"
    )]
    pub async fn read_ggen_config(
        &self,
        Parameters(params): Parameters<tools::ggen_config::ReadGgenConfigParams>,
    ) -> Result<Json<tools::ggen_config::ReadGgenConfigResponse>, McpError> {
        self.ensure_tool_enabled("read_ggen_config")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "read_ggen_config",
            tools::ggen_config::read_ggen_config(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "validate_ggen_config",
        description = "Validate ggen.toml configuration (syntax, file refs, circular deps, path overlaps)"
    )]
    pub async fn validate_ggen_config(
        &self,
        Parameters(params): Parameters<tools::ggen_config::ValidateGgenConfigParams>,
    ) -> Result<Json<tools::ggen_config::ValidateGgenConfigResponse>, McpError> {
        self.ensure_tool_enabled("validate_ggen_config")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "validate_ggen_config",
            tools::ggen_config::validate_ggen_config(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "add_generation_rule",
        description = "Add new generation rule to ggen.toml atomically with backup"
    )]
    pub async fn add_generation_rule(
        &self,
        Parameters(params): Parameters<tools::ggen_config::AddGenerationRuleParams>,
    ) -> Result<Json<tools::ggen_config::AddGenerationRuleResponse>, McpError> {
        self.ensure_tool_enabled("add_generation_rule")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "add_generation_rule",
            tools::ggen_config::add_generation_rule(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "update_generation_rule",
        description = "Update existing generation rule in ggen.toml by name"
    )]
    pub async fn update_generation_rule(
        &self,
        Parameters(params): Parameters<tools::ggen_config::UpdateGenerationRuleParams>,
    ) -> Result<Json<tools::ggen_config::UpdateGenerationRuleResponse>, McpError> {
        self.ensure_tool_enabled("update_generation_rule")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "update_generation_rule",
            tools::ggen_config::update_generation_rule(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "remove_generation_rule",
        description = "Remove generation rule from ggen.toml by name"
    )]
    pub async fn remove_generation_rule(
        &self,
        Parameters(params): Parameters<tools::ggen_config::RemoveGenerationRuleParams>,
    ) -> Result<Json<tools::ggen_config::RemoveGenerationRuleResponse>, McpError> {
        self.ensure_tool_enabled("remove_generation_rule")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "remove_generation_rule",
            tools::ggen_config::remove_generation_rule(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "sync_ggen",
        description = "Atomic ontology-driven code generation - 13-stage pipeline (discover → query → render → validate → write)"
    )]
    pub async fn sync_ggen_tool(
        &self,
        Parameters(params): Parameters<tools::ggen_sync::SyncGgenParams>,
    ) -> Result<Json<tools::ggen_sync::SyncGgenResponse>, McpError> {
        self.ensure_tool_enabled("sync_ggen")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "sync_ggen",
            tools::ggen_sync::sync_ggen(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "verify_receipt",
        description = "Verify cryptographic integrity of ggen generation receipt (7 checks: schema, workspace, inputs, outputs, guards, metadata, receipt ID)"
    )]
    pub async fn verify_receipt_tool(
        &self,
        Parameters(params): Parameters<tools::verify_receipt::VerifyReceiptParams>,
    ) -> Result<Json<tools::verify_receipt::VerifyReceiptResponse>, McpError> {
        self.ensure_tool_enabled("verify_receipt")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "verify_receipt",
            tools::verify_receipt::verify_receipt(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "validate_definition_of_done",
        description = "Validate Definition of Done: 15 checks (workspace, build, tests, ggen, safety). Returns deployment readiness verdict with evidence bundle."
    )]
    pub async fn validate_definition_of_done_tool(
        &self,
        Parameters(params): Parameters<tools::dod::ValidateDefinitionOfDoneParams>,
    ) -> Result<Json<tools::dod::ValidateDefinitionOfDoneResponse>, McpError> {
        self.ensure_tool_enabled("validate_definition_of_done")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "validate_definition_of_done",
            tools::dod::validate_definition_of_done(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    // ========================================================================
    // Tera Template Authoring Tools
    // ========================================================================

    #[tool(
        name = "read_tera_template",
        description = "Read and analyze Tera template (variables, filters, control structures, blocks, macros)"
    )]
    pub async fn read_tera_template(
        &self,
        Parameters(params): Parameters<tools::tera_authoring::ReadTeraTemplateParams>,
    ) -> Result<Json<tools::tera_authoring::ReadTeraTemplateResponse>, McpError> {
        self.ensure_tool_enabled("read_tera_template")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "read_tera_template",
            async {
                tools::tera_authoring::read_tera_template(self.state.clone(), params)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            },
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "validate_tera_template",
        description = "Validate Tera template syntax, variables, filters, and balanced blocks"
    )]
    pub async fn validate_tera_template(
        &self,
        Parameters(params): Parameters<tools::tera_authoring::ValidateTeraParams>,
    ) -> Result<Json<tools::tera_authoring::ValidateTeraResponse>, McpError> {
        self.ensure_tool_enabled("validate_tera_template")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "validate_tera_template",
            async {
                tools::tera_authoring::validate_tera_template(self.state.clone(), params)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            },
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "test_tera_template",
        description = "Test Tera template rendering with sample context (shows errors, performance metrics)"
    )]
    pub async fn test_tera_template(
        &self,
        Parameters(params): Parameters<tools::tera_authoring::TestTeraParams>,
    ) -> Result<Json<tools::tera_authoring::TestTeraResponse>, McpError> {
        self.ensure_tool_enabled("test_tera_template")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "test_tera_template",
            async {
                tools::tera_authoring::test_tera_template(self.state.clone(), params)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            },
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "create_tera_template",
        description = "Scaffold Tera template from pattern (struct, endpoint, schema, interface)"
    )]
    pub async fn create_tera_template(
        &self,
        Parameters(params): Parameters<tools::tera_authoring::CreateTeraParams>,
    ) -> Result<Json<tools::tera_authoring::CreateTeraResponse>, McpError> {
        self.ensure_tool_enabled("create_tera_template")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "create_tera_template",
            async {
                tools::tera_authoring::create_tera_template(self.state.clone(), params)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            },
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "list_template_variables",
        description = "Extract all variables from Tera template (usage count, filters, type hints)"
    )]
    pub async fn list_template_variables(
        &self,
        Parameters(params): Parameters<tools::tera_authoring::ListTemplateVariablesParams>,
    ) -> Result<Json<tools::tera_authoring::ListTemplateVariablesResponse>, McpError> {
        self.ensure_tool_enabled("list_template_variables")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "list_template_variables",
            async {
                tools::tera_authoring::list_template_variables(self.state.clone(), params)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            },
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    // ========================================================================
    // Unified Ggen Resource Management Tool
    // ========================================================================

    #[tool(
        name = "manage_ggen_resource",
        description = "Unified ggen resource management: config (5 ops), ontology (5 ops), templates (5 ops). Single MCP tool consolidating 15 authoring operations."
    )]
    pub async fn manage_ggen_resource(
        &self,
        Parameters(params): Parameters<tools::ggen_unified::ManageGgenResourceParams>,
    ) -> Result<Json<tools::ggen_unified::ManageGgenResourceResponse>, McpError> {
        self.ensure_tool_enabled("manage_ggen_resource")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "manage_ggen_resource",
            tools::ggen_unified::manage_ggen_resource(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }
}

#[cfg(feature = "recalc")]
#[tool_router(router = fork_tool_router)]
impl SpreadsheetServer {
    #[tool(
        name = "create_fork",
        description = "Create a temporary editable copy of a workbook for what-if analysis"
    )]
    pub async fn create_fork(
        &self,
        Parameters(params): Parameters<tools::fork::CreateForkParams>,
    ) -> Result<Json<tools::fork::CreateForkResponse>, McpError> {
        self.ensure_recalc_enabled("create_fork")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "create_fork",
            tools::fork::create_fork(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "edit_batch",
        description = "Apply batch edits (values or formulas) to a fork"
    )]
    pub async fn edit_batch(
        &self,
        Parameters(params): Parameters<tools::fork::EditBatchParams>,
    ) -> Result<Json<tools::fork::EditBatchResponse>, McpError> {
        self.ensure_recalc_enabled("edit_batch")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "edit_batch",
            tools::fork::edit_batch(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "transform_batch",
        description = "Range-oriented transforms for a fork (clear/fill/replace). Supports targets by range, region_id, or explicit cells. \
Mode: preview or apply (default apply)."
    )]
    pub async fn transform_batch(
        &self,
        Parameters(params): Parameters<tools::fork::TransformBatchParams>,
    ) -> Result<Json<tools::fork::TransformBatchResponse>, McpError> {
        self.ensure_recalc_enabled("transform_batch")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "transform_batch",
            tools::fork::transform_batch(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "style_batch",
        description = "Apply batch style edits to a fork. Supports targets by range, region_id, or explicit cells. \
Mode: preview or apply (default apply). Op mode: merge (default), set, or clear."
    )]
    pub async fn style_batch(
        &self,
        Parameters(params): Parameters<tools::fork::StyleBatchParams>,
    ) -> Result<Json<tools::fork::StyleBatchResponse>, McpError> {
        self.ensure_recalc_enabled("style_batch")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "style_batch",
            tools::fork::style_batch(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "apply_formula_pattern",
        description = "Autofill-like formula pattern application over a target range in a fork. \
Provide base_formula at anchor_cell, then fill across target_range. \
Mode: preview or apply (default apply). relative_mode: excel (default), abs_cols, abs_rows. \
fill_direction: down, right, both (default both)."
    )]
    pub async fn apply_formula_pattern(
        &self,
        Parameters(params): Parameters<tools::fork::ApplyFormulaPatternParams>,
    ) -> Result<Json<tools::fork::ApplyFormulaPatternResponse>, McpError> {
        self.ensure_recalc_enabled("apply_formula_pattern")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "apply_formula_pattern",
            tools::fork::apply_formula_pattern(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "structure_batch",
        description = "Apply structural edits to a fork (rows/cols/sheets). \
Mode: preview or apply (default apply). Note: structural edits may not fully rewrite formulas/named ranges like Excel; \
run recalculate and review get_changeset after applying."
    )]
    pub async fn structure_batch(
        &self,
        Parameters(params): Parameters<tools::fork::StructureBatchParams>,
    ) -> Result<Json<tools::fork::StructureBatchResponse>, McpError> {
        self.ensure_recalc_enabled("structure_batch")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "structure_batch",
            tools::fork::structure_batch(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(name = "get_edits", description = "List all edits applied to a fork")]
    pub async fn get_edits(
        &self,
        Parameters(params): Parameters<tools::fork::GetEditsParams>,
    ) -> Result<Json<tools::fork::GetEditsResponse>, McpError> {
        self.ensure_recalc_enabled("get_edits")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "get_edits",
            tools::fork::get_edits(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "get_changeset",
        description = "Calculate diff between fork and base workbook. Defaults: limit=200. Supports limit/offset paging and type/subtype filters; returns summary."
    )]
    pub async fn get_changeset(
        &self,
        Parameters(params): Parameters<tools::fork::GetChangesetParams>,
    ) -> Result<Json<tools::fork::GetChangesetResponse>, McpError> {
        self.ensure_recalc_enabled("get_changeset")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "get_changeset",
            tools::fork::get_changeset(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "recalculate",
        description = "Recalculate all formulas in a fork using LibreOffice"
    )]
    pub async fn recalculate(
        &self,
        Parameters(params): Parameters<tools::fork::RecalculateParams>,
    ) -> Result<Json<tools::fork::RecalculateResponse>, McpError> {
        self.ensure_recalc_enabled("recalculate")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "recalculate",
            tools::fork::recalculate(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(name = "list_forks", description = "List all active forks")]
    pub async fn list_forks(
        &self,
        Parameters(params): Parameters<tools::fork::ListForksParams>,
    ) -> Result<Json<tools::fork::ListForksResponse>, McpError> {
        self.ensure_recalc_enabled("list_forks")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "list_forks",
            tools::fork::list_forks(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(name = "discard_fork", description = "Discard a fork without saving")]
    pub async fn discard_fork(
        &self,
        Parameters(params): Parameters<tools::fork::DiscardForkParams>,
    ) -> Result<Json<tools::fork::DiscardForkResponse>, McpError> {
        self.ensure_recalc_enabled("discard_fork")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "discard_fork",
            tools::fork::discard_fork(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "save_fork",
        description = "Save fork changes to target path (defaults to overwriting original)"
    )]
    pub async fn save_fork(
        &self,
        Parameters(params): Parameters<tools::fork::SaveForkParams>,
    ) -> Result<Json<tools::fork::SaveForkResponse>, McpError> {
        self.ensure_recalc_enabled("save_fork")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "save_fork",
            tools::fork::save_fork(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "checkpoint_fork",
        description = "Create a high-fidelity checkpoint snapshot of a fork"
    )]
    pub async fn checkpoint_fork(
        &self,
        Parameters(params): Parameters<tools::fork::CheckpointForkParams>,
    ) -> Result<Json<tools::fork::CheckpointForkResponse>, McpError> {
        self.ensure_recalc_enabled("checkpoint_fork")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "checkpoint_fork",
            tools::fork::checkpoint_fork(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(name = "list_checkpoints", description = "List checkpoints for a fork")]
    pub async fn list_checkpoints(
        &self,
        Parameters(params): Parameters<tools::fork::ListCheckpointsParams>,
    ) -> Result<Json<tools::fork::ListCheckpointsResponse>, McpError> {
        self.ensure_recalc_enabled("list_checkpoints")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "list_checkpoints",
            tools::fork::list_checkpoints(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "restore_checkpoint",
        description = "Restore a fork to a checkpoint"
    )]
    pub async fn restore_checkpoint(
        &self,
        Parameters(params): Parameters<tools::fork::RestoreCheckpointParams>,
    ) -> Result<Json<tools::fork::RestoreCheckpointResponse>, McpError> {
        self.ensure_recalc_enabled("restore_checkpoint")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "restore_checkpoint",
            tools::fork::restore_checkpoint(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "delete_checkpoint",
        description = "Delete a checkpoint from a fork"
    )]
    pub async fn delete_checkpoint(
        &self,
        Parameters(params): Parameters<tools::fork::DeleteCheckpointParams>,
    ) -> Result<Json<tools::fork::DeleteCheckpointResponse>, McpError> {
        self.ensure_recalc_enabled("delete_checkpoint")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "delete_checkpoint",
            tools::fork::delete_checkpoint(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "list_staged_changes",
        description = "List previewed/staged changes for a fork"
    )]
    pub async fn list_staged_changes(
        &self,
        Parameters(params): Parameters<tools::fork::ListStagedChangesParams>,
    ) -> Result<Json<tools::fork::ListStagedChangesResponse>, McpError> {
        self.ensure_recalc_enabled("list_staged_changes")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "list_staged_changes",
            tools::fork::list_staged_changes(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "apply_staged_change",
        description = "Apply a staged change to a fork"
    )]
    pub async fn apply_staged_change(
        &self,
        Parameters(params): Parameters<tools::fork::ApplyStagedChangeParams>,
    ) -> Result<Json<tools::fork::ApplyStagedChangeResponse>, McpError> {
        self.ensure_recalc_enabled("apply_staged_change")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "apply_staged_change",
            tools::fork::apply_staged_change(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "discard_staged_change",
        description = "Discard a staged change without applying it"
    )]
    pub async fn discard_staged_change(
        &self,
        Parameters(params): Parameters<tools::fork::DiscardStagedChangeParams>,
    ) -> Result<Json<tools::fork::DiscardStagedChangeResponse>, McpError> {
        self.ensure_recalc_enabled("discard_staged_change")
            .map_err(to_mcp_error)?;
        self.run_tool_with_timeout(
            "discard_staged_change",
            tools::fork::discard_staged_change(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "screenshot_sheet",
        description = "Capture a visual screenshot of a spreadsheet region as PNG. \
	Returns file URI. Max range: 100 rows x 30 columns. Default: A1:M40."
    )]
    pub async fn screenshot_sheet(
        &self,
        Parameters(params): Parameters<tools::fork::ScreenshotSheetParams>,
    ) -> Result<rmcp::model::CallToolResult, McpError> {
        use base64::Engine;
        use rmcp::model::Content;

        self.ensure_recalc_enabled("screenshot_sheet")
            .map_err(to_mcp_error)?;

        let result = async {
            let response = self
                .run_tool_with_timeout(
                    "screenshot_sheet",
                    tools::fork::screenshot_sheet(self.state.clone(), params),
                )
                .await?;

            let mut content = Vec::new();

            let fs_path = response
                .output_path
                .strip_prefix("file://")
                .ok_or_else(|| anyhow!("unexpected screenshot output_path"))?;
            let bytes = tokio::fs::read(fs_path)
                .await
                .map_err(|e| anyhow!("failed to read screenshot: {}", e))?;

            if let Some(limit) = self.state.config().max_response_bytes() {
                let encoded_len = ((bytes.len() + 2) / 3) * 4;
                let meta = serde_json::to_vec(&response)
                    .map_err(|e| anyhow!("failed to serialize response: {}", e))?;
                let estimated = encoded_len + meta.len() + response.output_path.len();
                if estimated > limit {
                    return Err(
                        ResponseTooLargeError::new("screenshot_sheet", estimated, limit).into(),
                    );
                }
            }

            let data = base64::engine::general_purpose::STANDARD.encode(bytes);
            content.push(Content::image(data, "image/png"));

            // Always include a small text hint for clients that ignore structured_content.
            content.push(Content::text(response.output_path.clone()));

            let structured_content = serde_json::to_value(&response)
                .map_err(|e| anyhow!("failed to serialize response: {}", e))?;

            Ok(rmcp::model::CallToolResult {
                content,
                structured_content: Some(structured_content),
                is_error: Some(false),
                meta: None,
            })
        }
        .await;

        result.map_err(to_mcp_error)
    }

    #[tool(
        name = "validate_generated_code",
        description = "Validate generated code with multi-language syntax checking and optional golden file comparison. \
Supports: Rust, TypeScript, YAML, JSON. Returns detailed errors, warnings, and suggestions."
    )]
    pub async fn validate_generated_code(
        &self,
        Parameters(params): Parameters<tools::ontology_generation::ValidateGeneratedCodeParams>,
    ) -> Result<Json<tools::ontology_generation::ValidateGeneratedCodeResponse>, McpError> {
        // No tool enablement check - validation is always available
        self.run_tool_with_timeout(
            "validate_generated_code",
            tools::ontology_generation::validate_generated_code(params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }
}

// =============================================================================
// Jira Integration Tools
// =============================================================================

#[tool_router(router = jira_tool_router)]
impl SpreadsheetServer {
    #[tool(
        name = "sync_jira_to_spreadsheet",
        description = "Sync Jira tickets to spreadsheet rows (Jira → Spreadsheet). Query tickets via JQL, update fork. Timestamp-based conflict resolution."
    )]
    pub async fn sync_jira_to_spreadsheet(
        &self,
        Parameters(params): Parameters<tools::jira_integration::SyncJiraToSpreadsheetParams>,
    ) -> Result<Json<tools::jira_integration::SyncJiraToSpreadsheetResponse>, McpError> {
        // Requires fork feature (atomic transactions)
        #[cfg(not(feature = "recalc"))]
        {
            return Err(to_mcp_error(anyhow::anyhow!(
                "Jira sync requires fork support (enable recalc feature)"
            )));
        }

        #[cfg(feature = "recalc")]
        {
            self.ensure_recalc_enabled("sync_jira_to_spreadsheet")
                .map_err(to_mcp_error)?;
            self.run_tool_with_timeout(
                "sync_jira_to_spreadsheet",
                tools::jira_integration::sync_jira_to_spreadsheet(self.state.clone(), params),
            )
            .await
            .map(Json)
            .map_err(to_mcp_error)
        }
    }

    #[tool(
        name = "sync_spreadsheet_to_jira",
        description = "Sync spreadsheet rows to Jira tickets (Spreadsheet → Jira). Create/update tickets. Timestamp-based conflict resolution."
    )]
    pub async fn sync_spreadsheet_to_jira(
        &self,
        Parameters(params): Parameters<tools::jira_integration::SyncSpreadsheetToJiraParams>,
    ) -> Result<Json<tools::jira_integration::SyncSpreadsheetToJiraResponse>, McpError> {
        self.run_tool_with_timeout(
            "sync_spreadsheet_to_jira",
            tools::jira_integration::sync_spreadsheet_to_jira(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }

    #[tool(
        name = "manage_jira_integration",
        description = "Unified Jira integration tool. Consolidates 6 operations: QueryTickets, CreateTickets, ImportTickets, SyncToSpreadsheet, SyncToJira, CreateDashboard. \
Token-efficient (250 token savings). Operation-based dispatch."
    )]
    pub async fn manage_jira_integration(
        &self,
        Parameters(params): Parameters<tools::jira_unified::ManageJiraParams>,
    ) -> Result<Json<tools::jira_unified::ManageJiraResponse>, McpError> {
        // Check if fork feature required for certain operations
        match &params.operation {
            tools::jira_unified::JiraOperation::SyncToSpreadsheet { .. } => {
                #[cfg(not(feature = "recalc"))]
                {
                    return Err(to_mcp_error(anyhow::anyhow!(
                        "SyncToSpreadsheet requires fork support (enable recalc feature)"
                    )));
                }
                #[cfg(feature = "recalc")]
                {
                    self.ensure_recalc_enabled("manage_jira_integration")
                        .map_err(to_mcp_error)?;
                }
            }
            _ => {}
        }

        self.run_tool_with_timeout(
            "manage_jira_integration",
            tools::jira_unified::manage_jira_integration(self.state.clone(), params),
        )
        .await
        .map(Json)
        .map_err(to_mcp_error)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for SpreadsheetServer {
    fn get_info(&self) -> ServerInfo {
        let recalc_enabled = {
            #[cfg(feature = "recalc")]
            {
                self.state.config().recalc_enabled
            }
            #[cfg(not(feature = "recalc"))]
            {
                false
            }
        };

        let vba_enabled = self.state.config().vba_enabled;

        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(build_instructions(recalc_enabled, vba_enabled)),
            ..ServerInfo::default()
        }
    }
}

fn to_mcp_error(error: anyhow::Error) -> McpError {
    // Check for specific error types first
    if let Some(tool_disabled) = error.downcast_ref::<ToolDisabledError>() {
        let custom_error = CustomMcpError::builder(ErrorCode::ToolDisabled)
            .message(format!(
                "Tool '{}' is disabled by server configuration",
                tool_disabled.tool_name
            ))
            .operation(&tool_disabled.tool_name)
            .suggestion("Enable the tool in server configuration")
            .suggestion("Check SPREADSHEET_MCP_ENABLED_TOOLS environment variable")
            .build_and_track();
        return to_rmcp_error(custom_error);
    }

    if let Some(response_too_large) = error.downcast_ref::<ResponseTooLargeError>() {
        let custom_error = CustomMcpError::builder(ErrorCode::ResponseTooLarge)
            .message(format!(
                "Tool '{}' response too large ({} bytes > {} bytes)",
                response_too_large.tool_name, response_too_large.size, response_too_large.limit
            ))
            .operation(&response_too_large.tool_name)
            .param("size", response_too_large.size)
            .param("limit", response_too_large.limit)
            .suggestion("Use limit and offset parameters for pagination")
            .suggestion("Narrow the range or apply filters")
            .suggestion("Use summary_only=true for changesets")
            .build_and_track();
        return to_rmcp_error(custom_error);
    }

    if error.downcast_ref::<VbaDisabledError>().is_some() {
        let custom_error = CustomMcpError::builder(ErrorCode::ToolDisabled)
            .message("VBA tools are disabled")
            .suggestion("Set SPREADSHEET_MCP_VBA_ENABLED=true to enable VBA tools")
            .build_and_track();
        return to_rmcp_error(custom_error);
    }

    #[cfg(feature = "recalc")]
    if error.downcast_ref::<RecalcDisabledError>().is_some() {
        let custom_error = CustomMcpError::builder(ErrorCode::ToolDisabled)
            .message("Recalc/write tools are disabled")
            .suggestion("Set SPREADSHEET_MCP_RECALC_ENABLED=true to enable recalc tools")
            .build_and_track();
        return to_rmcp_error(custom_error);
    }

    // Use the comprehensive error conversion from error module
    let custom_error = crate::error::to_mcp_error(error);
    to_rmcp_error(custom_error)
}

#[derive(Debug, Error)]
#[error("tool '{tool_name}' is disabled by server configuration")]
struct ToolDisabledError {
    tool_name: String,
}

impl ToolDisabledError {
    fn new(tool_name: &str) -> Self {
        Self {
            tool_name: tool_name.to_ascii_lowercase(),
        }
    }
}

#[derive(Debug, Error)]
#[error(
    "tool '{tool_name}' response too large ({size} bytes > {limit} bytes); reduce request size or page results"
)]
struct ResponseTooLargeError {
    tool_name: String,
    size: usize,
    limit: usize,
}

impl ResponseTooLargeError {
    fn new(tool_name: &str, size: usize, limit: usize) -> Self {
        Self {
            tool_name: tool_name.to_ascii_lowercase(),
            size,
            limit,
        }
    }
}

#[derive(Debug, Error)]
#[error("VBA tools are disabled (set SPREADSHEET_MCP_VBA_ENABLED=true)")]
struct VbaDisabledError;

#[cfg(feature = "recalc")]
#[derive(Debug, Error)]
#[error("recalc/write tools are disabled (set SPREADSHEET_MCP_RECALC_ENABLED=true)")]
struct RecalcDisabledError;
