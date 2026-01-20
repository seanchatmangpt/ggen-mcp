//! Unified Jira Integration Tool
//!
//! Consolidates 6 Jira tools into single `manage_jira_integration` tool.
//! Reduces token usage: 6×50=300 tokens → 1×50=50 tokens (250 token savings).
//!
//! ## Operations
//! - QueryTickets: Execute JQL query, return results
//! - CreateTickets: Create tickets from spreadsheet rows
//! - ImportTickets: Import Jira tickets to spreadsheet
//! - SyncToSpreadsheet: Jira → Spreadsheet (fork-based)
//! - SyncToJira: Spreadsheet → Jira
//! - CreateDashboard: Generate dashboard spreadsheet
//!
//! ## Design
//! Single tool signature. Enum-based operation dispatch. Reuse existing components.
//!
//! ## Safety
//! Input validation. Poka-yoke patterns. Fork-based atomic transactions.

use crate::audit::integration::audit_tool;
use crate::error::{ErrorCode, McpError as CustomMcpError};
use crate::model::WorkbookId;
use crate::state::AppState;
use crate::tools::jira_export::{JiraAuth, JiraColumnMapping};
use crate::tools::jira_integration::{
    ConflictResolution, JiraClient, JiraSyncColumnMapping, SyncReport,
};
use crate::validation::{validate_non_empty_string, validate_numeric_range};
use anyhow::{Context, Result, anyhow, bail};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

// =============================================================================
// Constants
// =============================================================================

const JIRA_API_TIMEOUT_SECS: u64 = 30;
const JIRA_RATE_LIMIT_DELAY_MS: u64 = 100;
const MAX_BATCH_SIZE: usize = 100;
const MAX_RESULTS_DEFAULT: usize = 100;

// =============================================================================
// Unified Tool Parameters
// =============================================================================

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ManageJiraParams {
    /// Workbook or Fork ID
    pub workbook_or_fork_id: String,
    /// Sheet name
    pub sheet_name: String,
    /// Jira base URL
    pub jira_base_url: String,
    /// Jira auth token
    pub jira_auth_token: String,
    /// Operation to perform
    pub operation: JiraOperation,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JiraOperation {
    /// Execute JQL query, return ticket data
    QueryTickets {
        jql_query: String,
        #[serde(default = "default_max_results")]
        max_results: usize,
        #[serde(default)]
        fields: Vec<String>,
    },

    /// Create Jira tickets from spreadsheet rows
    CreateTickets {
        jira_project_key: String,
        column_mapping: JiraColumnMapping,
        #[serde(default)]
        dry_run: bool,
        #[serde(default = "default_start_row")]
        start_row: u32,
        #[serde(default = "default_max_tickets")]
        max_tickets: usize,
    },

    /// Import Jira tickets to spreadsheet (selective fields)
    ImportTickets {
        jql_query: String,
        #[serde(default)]
        fields: Vec<String>,
        #[serde(default = "default_start_row_usize")]
        start_row: usize,
    },

    /// Sync Jira → Spreadsheet (fork-based)
    SyncToSpreadsheet {
        fork_id: String,
        jql_query: String,
        #[serde(default)]
        column_mapping: JiraSyncColumnMapping,
        #[serde(default = "default_start_row_usize")]
        start_row: usize,
        #[serde(default)]
        conflict_resolution: ConflictResolution,
    },

    /// Sync Spreadsheet → Jira
    SyncToJira {
        jira_project_key: String,
        #[serde(default)]
        column_mapping: JiraSyncColumnMapping,
        #[serde(default = "default_start_row_usize")]
        start_row: usize,
        #[serde(default)]
        end_row: Option<usize>,
        #[serde(default)]
        conflict_resolution: ConflictResolution,
    },

    /// Create dashboard spreadsheet from Jira data
    CreateDashboard {
        jql_query: String,
        #[serde(default)]
        views: Vec<DashboardView>,
    },
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DashboardView {
    Summary,
    ByStatus,
    ByPriority,
    ByAssignee,
    Timeline,
}

fn default_max_results() -> usize {
    MAX_RESULTS_DEFAULT
}

fn default_start_row() -> u32 {
    2
}

fn default_max_tickets() -> usize {
    100
}

fn default_start_row_usize() -> usize {
    2
}

// =============================================================================
// Unified Tool Response
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ManageJiraResponse {
    /// Operation performed
    pub operation: String,
    /// Operation-specific result
    pub result: JiraOperationResult,
    /// Execution metrics
    pub metrics: OperationMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JiraOperationResult {
    Query {
        tickets: Vec<JiraTicketSummary>,
        total_count: usize,
    },
    CreateTickets {
        tickets_created: usize,
        tickets_failed: usize,
        results: Vec<JiraTicketResult>,
        notes: Vec<String>,
    },
    Import {
        rows_imported: usize,
        fields_imported: Vec<String>,
    },
    Sync {
        report: SyncReport,
    },
    Dashboard {
        sheet_name: String,
        views_created: Vec<String>,
        total_rows: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JiraTicketSummary {
    pub key: String,
    pub summary: String,
    pub status: String,
    pub assignee: Option<String>,
    pub created: String,
    pub updated: String,
    pub fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JiraTicketResult {
    pub row: u32,
    pub success: bool,
    pub ticket_key: Option<String>,
    pub ticket_url: Option<String>,
    pub summary: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OperationMetrics {
    pub duration_ms: u64,
    pub items_processed: usize,
    pub api_calls: usize,
}

// =============================================================================
// Main Tool Function
// =============================================================================

pub async fn manage_jira_integration(
    state: Arc<AppState>,
    params: ManageJiraParams,
) -> Result<ManageJiraResponse> {
    let _span = audit_tool("manage_jira_integration", &params);
    let start = std::time::Instant::now();

    // Validate common parameters
    validate_common_params(&params)?;

    // Create Jira client
    let jira_client = JiraClient::new(
        params.jira_base_url.clone(),
        params.jira_auth_token.clone(),
    )?;

    // Dispatch to operation-specific handler
    let (operation_name, result, api_calls) = match params.operation.clone() {
        JiraOperation::QueryTickets {
            jql_query,
            max_results,
            fields,
        } => {
            let (result, calls) = handle_query_tickets(&jira_client, jql_query, max_results, fields).await?;
            ("query_tickets".to_string(), result, calls)
        }
        JiraOperation::CreateTickets {
            jira_project_key,
            column_mapping,
            dry_run,
            start_row,
            max_tickets,
        } => {
            let (result, calls) = handle_create_tickets(
                state.clone(),
                &params.workbook_or_fork_id,
                &params.sheet_name,
                &params.jira_base_url,
                &jira_client,
                jira_project_key,
                column_mapping,
                dry_run,
                start_row,
                max_tickets,
            )
            .await?;
            ("create_tickets".to_string(), result, calls)
        }
        JiraOperation::ImportTickets {
            jql_query,
            fields,
            start_row,
        } => {
            let (result, calls) = handle_import_tickets(
                state.clone(),
                &params.workbook_or_fork_id,
                &params.sheet_name,
                &jira_client,
                jql_query,
                fields,
                start_row,
            )
            .await?;
            ("import_tickets".to_string(), result, calls)
        }
        JiraOperation::SyncToSpreadsheet {
            fork_id,
            jql_query,
            column_mapping,
            start_row,
            conflict_resolution,
        } => {
            let (result, calls) = handle_sync_to_spreadsheet(
                state.clone(),
                &fork_id,
                &params.sheet_name,
                &jira_client,
                jql_query,
                column_mapping,
                start_row,
                conflict_resolution,
            )
            .await?;
            ("sync_to_spreadsheet".to_string(), result, calls)
        }
        JiraOperation::SyncToJira {
            jira_project_key,
            column_mapping,
            start_row,
            end_row,
            conflict_resolution,
        } => {
            let (result, calls) = handle_sync_to_jira(
                state.clone(),
                &params.workbook_or_fork_id,
                &params.sheet_name,
                &jira_client,
                jira_project_key,
                column_mapping,
                start_row,
                end_row,
                conflict_resolution,
            )
            .await?;
            ("sync_to_jira".to_string(), result, calls)
        }
        JiraOperation::CreateDashboard { jql_query, views } => {
            let (result, calls) = handle_create_dashboard(
                state.clone(),
                &params.workbook_or_fork_id,
                &params.sheet_name,
                &jira_client,
                jql_query,
                views,
            )
            .await?;
            ("create_dashboard".to_string(), result, calls)
        }
    };

    let duration_ms = start.elapsed().as_millis() as u64;
    let items_processed = match &result {
        JiraOperationResult::Query { total_count, .. } => *total_count,
        JiraOperationResult::CreateTickets { tickets_created, tickets_failed, .. } => tickets_created + tickets_failed,
        JiraOperationResult::Import { rows_imported, .. } => *rows_imported,
        JiraOperationResult::Sync { report } => report.created + report.updated + report.skipped,
        JiraOperationResult::Dashboard { total_rows, .. } => *total_rows,
    };

    Ok(ManageJiraResponse {
        operation: operation_name,
        result,
        metrics: OperationMetrics {
            duration_ms,
            items_processed,
            api_calls,
        },
    })
}

// =============================================================================
// Operation Handlers
// =============================================================================

async fn handle_query_tickets(
    client: &JiraClient,
    jql_query: String,
    max_results: usize,
    fields: Vec<String>,
) -> Result<(JiraOperationResult, usize)> {
    info!("Querying Jira: {}", jql_query);

    let default_fields = vec!["summary".to_string(), "status".to_string(), "created".to_string(), "updated".to_string()];
    let query_fields = if fields.is_empty() {
        default_fields
    } else {
        fields
    };

    let issues = client.search_issues(&jql_query, &query_fields).await?;
    let total = issues.len().min(max_results);

    let tickets: Vec<JiraTicketSummary> = issues
        .into_iter()
        .take(max_results)
        .map(|issue| {
            let summary = issue.fields.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let status = issue.fields.get("status").and_then(|v| v.get("name")).and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
            let assignee = issue.fields.get("assignee").and_then(|v| v.get("displayName")).and_then(|v| v.as_str()).map(|s| s.to_string());
            let created = issue.fields.get("created").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let updated = issue.fields.get("updated").and_then(|v| v.as_str()).unwrap_or("").to_string();

            let mut fields_map = HashMap::new();
            for (key, value) in issue.fields.as_object().unwrap_or(&serde_json::Map::new()) {
                fields_map.insert(key.clone(), value.clone());
            }

            JiraTicketSummary {
                key: issue.key,
                summary,
                status,
                assignee,
                created,
                updated,
                fields: fields_map,
            }
        })
        .collect();

    Ok((
        JiraOperationResult::Query {
            tickets,
            total_count: total,
        },
        1, // 1 API call
    ))
}

async fn handle_create_tickets(
    state: Arc<AppState>,
    workbook_id: &str,
    sheet_name: &str,
    jira_url: &str,
    client: &JiraClient,
    project_key: String,
    column_mapping: JiraColumnMapping,
    dry_run: bool,
    start_row: u32,
    max_tickets: usize,
) -> Result<(JiraOperationResult, usize)> {
    info!("Creating Jira tickets from {} sheet {} (dry_run: {})", workbook_id, sheet_name, dry_run);

    // Delegate to existing jira_export logic
    let params = crate::tools::jira_export::CreateJiraTicketsParams {
        workbook_id: WorkbookId(workbook_id.to_string()),
        sheet_name: sheet_name.to_string(),
        jira_project_key: project_key,
        jira_url: jira_url.to_string(),
        jira_auth: JiraAuth::Bearer {
            token: client.auth_token.clone(),
            email: None,
        },
        column_mapping,
        dry_run,
        start_row,
        max_tickets,
    };

    let response = crate::tools::jira_export::create_jira_tickets_from_spreadsheet(state, params).await?;

    let results = response
        .results
        .into_iter()
        .map(|r| JiraTicketResult {
            row: r.row,
            success: r.success,
            ticket_key: r.ticket_key,
            ticket_url: r.ticket_url,
            summary: r.summary,
            error: r.error,
        })
        .collect();

    Ok((
        JiraOperationResult::CreateTickets {
            tickets_created: response.tickets_created,
            tickets_failed: response.tickets_failed,
            results,
            notes: response.notes,
        },
        response.tickets_created + response.tickets_failed,
    ))
}

async fn handle_import_tickets(
    state: Arc<AppState>,
    workbook_or_fork_id: &str,
    sheet_name: &str,
    client: &JiraClient,
    jql_query: String,
    fields: Vec<String>,
    start_row: usize,
) -> Result<(JiraOperationResult, usize)> {
    info!("Importing Jira tickets to {} sheet {}", workbook_or_fork_id, sheet_name);

    let default_fields = vec!["summary".to_string(), "status".to_string(), "priority".to_string()];
    let import_fields = if fields.is_empty() {
        default_fields
    } else {
        fields
    };

    let issues = client.search_issues(&jql_query, &import_fields).await?;

    // Open workbook and write to sheet
    let workbook_id = WorkbookId(workbook_or_fork_id.to_string());
    let workbook = state.open_workbook(&workbook_id).await?;

    tokio::task::spawn_blocking({
        let work_path = workbook.path.clone();
        let sheet_name = sheet_name.to_string();
        let issues = issues.clone();
        let import_fields = import_fields.clone();
        move || write_tickets_to_sheet(&work_path, &sheet_name, &issues, &import_fields, start_row)
    })
    .await??;

    Ok((
        JiraOperationResult::Import {
            rows_imported: issues.len(),
            fields_imported: import_fields,
        },
        1, // 1 API call
    ))
}

async fn handle_sync_to_spreadsheet(
    state: Arc<AppState>,
    fork_id: &str,
    sheet_name: &str,
    _client: &JiraClient,
    jql_query: String,
    column_mapping: JiraSyncColumnMapping,
    start_row: usize,
    conflict_resolution: ConflictResolution,
) -> Result<(JiraOperationResult, usize)> {
    info!("Syncing Jira → Spreadsheet (fork: {}, sheet: {})", fork_id, sheet_name);

    // Delegate to existing jira_integration logic
    let params = crate::tools::jira_integration::SyncJiraToSpreadsheetParams {
        fork_id: fork_id.to_string(),
        sheet_name: sheet_name.to_string(),
        jira_base_url: _client.base_url.clone(),
        jira_auth_token: _client.auth_token.clone(),
        jql_query,
        column_mapping,
        start_row,
        conflict_resolution,
    };

    let response = crate::tools::jira_integration::sync_jira_to_spreadsheet(state, params).await?;

    let api_calls = response.report.created + response.report.updated;

    Ok((
        JiraOperationResult::Sync {
            report: response.report,
        },
        api_calls,
    ))
}

async fn handle_sync_to_jira(
    state: Arc<AppState>,
    workbook_or_fork_id: &str,
    sheet_name: &str,
    _client: &JiraClient,
    jira_project_key: String,
    column_mapping: JiraSyncColumnMapping,
    start_row: usize,
    end_row: Option<usize>,
    conflict_resolution: ConflictResolution,
) -> Result<(JiraOperationResult, usize)> {
    info!("Syncing Spreadsheet → Jira (workbook: {}, sheet: {})", workbook_or_fork_id, sheet_name);

    // Delegate to existing jira_integration logic
    let params = crate::tools::jira_integration::SyncSpreadsheetToJiraParams {
        workbook_or_fork_id: WorkbookId(workbook_or_fork_id.to_string()),
        sheet_name: sheet_name.to_string(),
        jira_base_url: _client.base_url.clone(),
        jira_auth_token: _client.auth_token.clone(),
        jira_project_key,
        column_mapping,
        start_row,
        end_row,
        conflict_resolution,
    };

    let response = crate::tools::jira_integration::sync_spreadsheet_to_jira(state, params).await?;

    let api_calls = response.report.created + response.report.updated;

    Ok((
        JiraOperationResult::Sync {
            report: response.report,
        },
        api_calls,
    ))
}

async fn handle_create_dashboard(
    state: Arc<AppState>,
    workbook_or_fork_id: &str,
    sheet_name: &str,
    client: &JiraClient,
    jql_query: String,
    views: Vec<DashboardView>,
) -> Result<(JiraOperationResult, usize)> {
    info!("Creating Jira dashboard in {} sheet {}", workbook_or_fork_id, sheet_name);

    let default_views = if views.is_empty() {
        vec![DashboardView::Summary, DashboardView::ByStatus]
    } else {
        views
    };

    let fields = vec![
        "summary".to_string(),
        "status".to_string(),
        "priority".to_string(),
        "assignee".to_string(),
        "created".to_string(),
        "updated".to_string(),
    ];

    let issues = client.search_issues(&jql_query, &fields).await?;

    // Open workbook
    let workbook_id = WorkbookId(workbook_or_fork_id.to_string());
    let workbook = state.open_workbook(&workbook_id).await?;

    // Generate dashboard views
    let views_created = tokio::task::spawn_blocking({
        let work_path = workbook.path.clone();
        let sheet_name = sheet_name.to_string();
        let issues = issues.clone();
        let views = default_views.clone();
        move || create_dashboard_views(&work_path, &sheet_name, &issues, &views)
    })
    .await??;

    Ok((
        JiraOperationResult::Dashboard {
            sheet_name: sheet_name.to_string(),
            views_created,
            total_rows: issues.len(),
        },
        1, // 1 API call
    ))
}

// =============================================================================
// Helper Functions
// =============================================================================

fn validate_common_params(params: &ManageJiraParams) -> Result<()> {
    validate_non_empty_string("workbook_or_fork_id", &params.workbook_or_fork_id)?;
    validate_non_empty_string("sheet_name", &params.sheet_name)?;
    validate_non_empty_string("jira_base_url", &params.jira_base_url)?;
    validate_non_empty_string("jira_auth_token", &params.jira_auth_token)?;

    if !params.jira_base_url.starts_with("http://") && !params.jira_base_url.starts_with("https://") {
        return Err(anyhow!("jira_base_url must start with http:// or https://"));
    }

    Ok(())
}

fn write_tickets_to_sheet(
    path: &std::path::Path,
    sheet_name: &str,
    issues: &[crate::tools::jira_integration::JiraIssue],
    fields: &[String],
    start_row: usize,
) -> Result<()> {
    let mut book = if path.exists() {
        umya_spreadsheet::reader::xlsx::read(path).context("read spreadsheet failed")?
    } else {
        umya_spreadsheet::new_file()
    };

    // Get or create sheet
    let sheet = if book.get_sheet_by_name(sheet_name).is_some() {
        book.get_sheet_by_name_mut(sheet_name).unwrap()
    } else {
        book.new_sheet(sheet_name).map_err(|e| anyhow!("Failed to create sheet: {}", e))?;
        book.get_sheet_by_name_mut(sheet_name).unwrap()
    };

    // Write header row
    let header_row = start_row - 1;
    sheet.get_cell_mut(&format!("A{}", header_row)).set_value("Key");
    for (idx, field) in fields.iter().enumerate() {
        let col = column_letter(idx + 1);
        sheet.get_cell_mut(&format!("{}{}", col, header_row)).set_value(field);
    }

    // Write data rows
    for (row_idx, issue) in issues.iter().enumerate() {
        let row_num = start_row + row_idx;
        sheet.get_cell_mut(&format!("A{}", row_num)).set_value(&issue.key);

        for (col_idx, field) in fields.iter().enumerate() {
            let col = column_letter(col_idx + 1);
            let value = issue.fields.get(field).and_then(|v| {
                if let Some(s) = v.as_str() {
                    Some(s.to_string())
                } else if let Some(obj) = v.as_object() {
                    obj.get("name").and_then(|n| n.as_str()).map(|s| s.to_string())
                } else {
                    Some(v.to_string())
                }
            }).unwrap_or_default();

            sheet.get_cell_mut(&format!("{}{}", col, row_num)).set_value(value);
        }
    }

    umya_spreadsheet::writer::xlsx::write(&book, path).context("write spreadsheet failed")
}

fn create_dashboard_views(
    path: &std::path::Path,
    sheet_name: &str,
    issues: &[crate::tools::jira_integration::JiraIssue],
    views: &[DashboardView],
) -> Result<Vec<String>> {
    let mut book = if path.exists() {
        umya_spreadsheet::reader::xlsx::read(path).context("read spreadsheet failed")?
    } else {
        umya_spreadsheet::new_file()
    };

    let mut views_created = Vec::new();

    for view in views {
        let view_sheet_name = format!("{}_{:?}", sheet_name, view);
        views_created.push(view_sheet_name.clone());

        let sheet = if book.get_sheet_by_name(&view_sheet_name).is_some() {
            book.get_sheet_by_name_mut(&view_sheet_name).unwrap()
        } else {
            book.new_sheet(&view_sheet_name).map_err(|e| anyhow!("Failed to create sheet: {}", e))?;
            book.get_sheet_by_name_mut(&view_sheet_name).unwrap()
        };

        match view {
            DashboardView::Summary => {
                create_summary_view(sheet, issues)?;
            }
            DashboardView::ByStatus => {
                create_by_status_view(sheet, issues)?;
            }
            DashboardView::ByPriority => {
                create_by_priority_view(sheet, issues)?;
            }
            DashboardView::ByAssignee => {
                create_by_assignee_view(sheet, issues)?;
            }
            DashboardView::Timeline => {
                create_timeline_view(sheet, issues)?;
            }
        }
    }

    umya_spreadsheet::writer::xlsx::write(&book, path).context("write spreadsheet failed")?;

    Ok(views_created)
}

fn create_summary_view(
    sheet: &mut umya_spreadsheet::Worksheet,
    issues: &[crate::tools::jira_integration::JiraIssue],
) -> Result<()> {
    sheet.get_cell_mut("A1").set_value("Metric");
    sheet.get_cell_mut("B1").set_value("Count");

    sheet.get_cell_mut("A2").set_value("Total Tickets");
    sheet.get_cell_mut("B2").set_value(issues.len().to_string());

    let open_count = issues.iter().filter(|i| {
        i.fields.get("status").and_then(|v| v.get("name")).and_then(|v| v.as_str()) == Some("Open")
    }).count();
    sheet.get_cell_mut("A3").set_value("Open");
    sheet.get_cell_mut("B3").set_value(open_count.to_string());

    let in_progress_count = issues.iter().filter(|i| {
        i.fields.get("status").and_then(|v| v.get("name")).and_then(|v| v.as_str()) == Some("In Progress")
    }).count();
    sheet.get_cell_mut("A4").set_value("In Progress");
    sheet.get_cell_mut("B4").set_value(in_progress_count.to_string());

    Ok(())
}

fn create_by_status_view(
    sheet: &mut umya_spreadsheet::Worksheet,
    issues: &[crate::tools::jira_integration::JiraIssue],
) -> Result<()> {
    let mut status_counts: HashMap<String, usize> = HashMap::new();
    for issue in issues {
        let status = issue.fields.get("status")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();
        *status_counts.entry(status).or_insert(0) += 1;
    }

    sheet.get_cell_mut("A1").set_value("Status");
    sheet.get_cell_mut("B1").set_value("Count");

    for (idx, (status, count)) in status_counts.iter().enumerate() {
        let row = idx + 2;
        sheet.get_cell_mut(&format!("A{}", row)).set_value(status);
        sheet.get_cell_mut(&format!("B{}", row)).set_value(count.to_string());
    }

    Ok(())
}

fn create_by_priority_view(
    sheet: &mut umya_spreadsheet::Worksheet,
    issues: &[crate::tools::jira_integration::JiraIssue],
) -> Result<()> {
    let mut priority_counts: HashMap<String, usize> = HashMap::new();
    for issue in issues {
        let priority = issue.fields.get("priority")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();
        *priority_counts.entry(priority).or_insert(0) += 1;
    }

    sheet.get_cell_mut("A1").set_value("Priority");
    sheet.get_cell_mut("B1").set_value("Count");

    for (idx, (priority, count)) in priority_counts.iter().enumerate() {
        let row = idx + 2;
        sheet.get_cell_mut(&format!("A{}", row)).set_value(priority);
        sheet.get_cell_mut(&format!("B{}", row)).set_value(count.to_string());
    }

    Ok(())
}

fn create_by_assignee_view(
    sheet: &mut umya_spreadsheet::Worksheet,
    issues: &[crate::tools::jira_integration::JiraIssue],
) -> Result<()> {
    let mut assignee_counts: HashMap<String, usize> = HashMap::new();
    for issue in issues {
        let assignee = issue.fields.get("assignee")
            .and_then(|v| v.get("displayName"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unassigned")
            .to_string();
        *assignee_counts.entry(assignee).or_insert(0) += 1;
    }

    sheet.get_cell_mut("A1").set_value("Assignee");
    sheet.get_cell_mut("B1").set_value("Count");

    for (idx, (assignee, count)) in assignee_counts.iter().enumerate() {
        let row = idx + 2;
        sheet.get_cell_mut(&format!("A{}", row)).set_value(assignee);
        sheet.get_cell_mut(&format!("B{}", row)).set_value(count.to_string());
    }

    Ok(())
}

fn create_timeline_view(
    sheet: &mut umya_spreadsheet::Worksheet,
    issues: &[crate::tools::jira_integration::JiraIssue],
) -> Result<()> {
    sheet.get_cell_mut("A1").set_value("Key");
    sheet.get_cell_mut("B1").set_value("Summary");
    sheet.get_cell_mut("C1").set_value("Created");
    sheet.get_cell_mut("D1").set_value("Updated");
    sheet.get_cell_mut("E1").set_value("Status");

    for (idx, issue) in issues.iter().enumerate() {
        let row = idx + 2;
        sheet.get_cell_mut(&format!("A{}", row)).set_value(&issue.key);

        let summary = issue.fields.get("summary").and_then(|v| v.as_str()).unwrap_or("");
        sheet.get_cell_mut(&format!("B{}", row)).set_value(summary);

        let created = issue.fields.get("created").and_then(|v| v.as_str()).unwrap_or("");
        sheet.get_cell_mut(&format!("C{}", row)).set_value(created);

        let updated = issue.fields.get("updated").and_then(|v| v.as_str()).unwrap_or("");
        sheet.get_cell_mut(&format!("D{}", row)).set_value(updated);

        let status = issue.fields.get("status")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        sheet.get_cell_mut(&format!("E{}", row)).set_value(status);
    }

    Ok(())
}

fn column_letter(n: usize) -> String {
    let mut result = String::new();
    let mut n = n;
    while n > 0 {
        let rem = (n - 1) % 26;
        result.insert(0, (b'A' + rem as u8) as char);
        n = (n - 1) / 26;
    }
    result
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_common_params_success() {
        let params = ManageJiraParams {
            workbook_or_fork_id: "test.xlsx".to_string(),
            sheet_name: "Sheet1".to_string(),
            jira_base_url: "https://company.atlassian.net".to_string(),
            jira_auth_token: "test-token".to_string(),
            operation: JiraOperation::QueryTickets {
                jql_query: "project = TEST".to_string(),
                max_results: 100,
                fields: vec![],
            },
        };

        assert!(validate_common_params(&params).is_ok());
    }

    #[test]
    fn test_validate_common_params_invalid_url() {
        let params = ManageJiraParams {
            workbook_or_fork_id: "test.xlsx".to_string(),
            sheet_name: "Sheet1".to_string(),
            jira_base_url: "invalid-url".to_string(),
            jira_auth_token: "test-token".to_string(),
            operation: JiraOperation::QueryTickets {
                jql_query: "project = TEST".to_string(),
                max_results: 100,
                fields: vec![],
            },
        };

        assert!(validate_common_params(&params).is_err());
    }

    #[test]
    fn test_column_letter() {
        assert_eq!(column_letter(1), "A");
        assert_eq!(column_letter(2), "B");
        assert_eq!(column_letter(26), "Z");
        assert_eq!(column_letter(27), "AA");
        assert_eq!(column_letter(28), "AB");
    }

    #[test]
    fn test_operation_metrics_structure() {
        let metrics = OperationMetrics {
            duration_ms: 1000,
            items_processed: 10,
            api_calls: 2,
        };

        assert_eq!(metrics.duration_ms, 1000);
        assert_eq!(metrics.items_processed, 10);
        assert_eq!(metrics.api_calls, 2);
    }
}
