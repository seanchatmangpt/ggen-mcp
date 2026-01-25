//! Bidirectional Jira ↔ Spreadsheet synchronization
//!
//! ## Tools
//! - sync_jira_to_spreadsheet: Jira → Spreadsheet
//! - sync_spreadsheet_to_jira: Spreadsheet → Jira
//!
//! ## Strategy
//! Primary key: Jira Key column. Timestamp-based conflicts.
//!
//! ## Safety
//! Fork-based atomic transactions. Input validation.

use crate::model::EditOp;
use crate::model::WorkbookId;
use crate::state::AppState;
use anyhow::{Context, Result, anyhow, bail};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// Jira REST API v3 client
#[derive(Debug, Clone)]
pub struct JiraClient {
    pub base_url: String,
    pub auth_token: String,
    client: reqwest::Client,
}

impl JiraClient {
    pub fn new(base_url: String, auth_token: String) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("failed to build HTTP client")?;
        Ok(Self {
            base_url,
            auth_token,
            client,
        })
    }

    pub async fn search_issues(&self, jql: &str, fields: &[String]) -> Result<Vec<JiraIssue>> {
        let url = format!("{}/rest/api/3/search", self.base_url);
        let body = serde_json::json!({ "jql": jql, "fields": fields, "maxResults": 1000 });
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.auth_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("search request failed")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Jira API error ({}): {}", status, body);
        }
        let result: JiraSearchResult = resp.json().await.context("parse search response")?;
        Ok(result.issues)
    }

    pub async fn get_issue(&self, key: &str, fields: &[String]) -> Result<JiraIssue> {
        let url = format!(
            "{}/rest/api/3/issue/{}?fields={}",
            self.base_url,
            key,
            fields.join(",")
        );
        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.auth_token))
            .send()
            .await
            .context("get issue failed")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Jira API error ({}): {}", status, body);
        }
        resp.json().await.context("parse issue response")
    }

    pub async fn update_issue(&self, key: &str, fields: &JiraFieldUpdate) -> Result<()> {
        let url = format!("{}/rest/api/3/issue/{}", self.base_url, key);
        let body = serde_json::json!({ "fields": fields.to_json() });
        let resp = self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", self.auth_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("update issue failed")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Jira API error ({}): {}", status, body);
        }
        Ok(())
    }

    pub async fn create_issue(
        &self,
        project_key: &str,
        fields: &JiraFieldUpdate,
    ) -> Result<String> {
        let url = format!("{}/rest/api/3/issue", self.base_url);
        let mut fields_json = fields.to_json();
        fields_json["project"] = serde_json::json!({"key": project_key});
        fields_json["issuetype"] = serde_json::json!({"name": "Task"});
        let body = serde_json::json!({ "fields": fields_json });
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.auth_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("create issue failed")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Jira API error ({}): {}", status, body);
        }
        let result: JiraCreateResult = resp.json().await.context("parse create response")?;
        Ok(result.key)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct JiraSearchResult {
    pub issues: Vec<JiraIssue>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JiraIssue {
    pub key: String,
    pub fields: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JiraCreateResult {
    pub key: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct JiraFieldUpdate {
    pub summary: Option<String>,
    pub status: Option<String>,
    pub assignee: Option<String>,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub labels: Option<Vec<String>>,
    #[serde(flatten)]
    pub custom_fields: HashMap<String, serde_json::Value>,
}

impl JiraFieldUpdate {
    fn to_json(&self) -> serde_json::Value {
        let mut fields = serde_json::Map::new();
        if let Some(ref s) = self.summary {
            fields.insert("summary".to_string(), serde_json::json!(s));
        }
        if let Some(ref s) = self.status {
            fields.insert("status".to_string(), serde_json::json!({"name": s}));
        }
        if let Some(ref a) = self.assignee {
            fields.insert("assignee".to_string(), serde_json::json!({"name": a}));
        }
        if let Some(ref d) = self.description {
            fields.insert("description".to_string(), serde_json::json!(d));
        }
        if let Some(ref p) = self.priority {
            fields.insert("priority".to_string(), serde_json::json!({"name": p}));
        }
        if let Some(ref l) = self.labels {
            fields.insert("labels".to_string(), serde_json::json!(l));
        }
        for (k, v) in &self.custom_fields {
            fields.insert(k.clone(), v.clone());
        }
        serde_json::Value::Object(fields)
    }
}

/// Column mapping for Jira sync
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct JiraSyncColumnMapping {
    pub jira_key_column: String,
    pub summary_column: String,
    #[serde(default)]
    pub status_column: Option<String>,
    #[serde(default)]
    pub assignee_column: Option<String>,
    #[serde(default)]
    pub updated_column: Option<String>,
    #[serde(default)]
    pub description_column: Option<String>,
    #[serde(default)]
    pub priority_column: Option<String>,
    #[serde(default)]
    pub labels_column: Option<String>,
}

impl Default for JiraSyncColumnMapping {
    fn default() -> Self {
        Self {
            jira_key_column: "A".to_string(),
            summary_column: "B".to_string(),
            status_column: Some("C".to_string()),
            assignee_column: Some("D".to_string()),
            updated_column: Some("E".to_string()),
            description_column: None,
            priority_column: None,
            labels_column: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    JiraWins,
    SpreadsheetWins,
    Skip,
}

impl Default for ConflictResolution {
    fn default() -> Self {
        Self::JiraWins
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SyncReport {
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: Vec<SyncError>,
    pub conflicts: Vec<ConflictReport>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SyncError {
    pub row: usize,
    pub jira_key: Option<String>,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ConflictReport {
    pub row: usize,
    pub jira_key: String,
    pub reason: String,
    pub resolution: String,
}

impl SyncReport {
    fn new() -> Self {
        Self {
            created: 0,
            updated: 0,
            skipped: 0,
            errors: Vec::new(),
            conflicts: Vec::new(),
        }
    }
}

// TOOL 1: Jira → Spreadsheet
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SyncJiraToSpreadsheetParams {
    pub fork_id: String,
    pub sheet_name: String,
    pub jira_base_url: String,
    pub jira_auth_token: String,
    pub jql_query: String,
    #[serde(default)]
    pub column_mapping: JiraSyncColumnMapping,
    #[serde(default = "default_start_row")]
    pub start_row: usize,
    #[serde(default)]
    pub conflict_resolution: ConflictResolution,
}

fn default_start_row() -> usize {
    2
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SyncJiraToSpreadsheetResponse {
    pub fork_id: String,
    pub report: SyncReport,
}

#[cfg(feature = "recalc")]
pub async fn sync_jira_to_spreadsheet(
    state: Arc<AppState>,
    params: SyncJiraToSpreadsheetParams,
) -> Result<SyncJiraToSpreadsheetResponse> {
    let registry = state
        .fork_registry()
        .ok_or_else(|| anyhow!("fork registry not available"))?
        .clone();
    let fork_ctx = registry.get_fork(&params.fork_id)?;
    let work_path = fork_ctx.work_path.clone();

    let jira_client =
        JiraClient::new(params.jira_base_url.clone(), params.jira_auth_token.clone())?;

    let mut fields = vec!["summary".to_string(), "updated".to_string()];
    if params.column_mapping.status_column.is_some() {
        fields.push("status".to_string());
    }
    if params.column_mapping.assignee_column.is_some() {
        fields.push("assignee".to_string());
    }
    if params.column_mapping.description_column.is_some() {
        fields.push("description".to_string());
    }
    if params.column_mapping.priority_column.is_some() {
        fields.push("priority".to_string());
    }
    if params.column_mapping.labels_column.is_some() {
        fields.push("labels".to_string());
    }

    let jira_issues = jira_client
        .search_issues(&params.jql_query, &fields)
        .await
        .context("query Jira failed")?;

    let existing_rows = tokio::task::spawn_blocking({
        let work_path = work_path.clone();
        let sheet_name = params.sheet_name.clone();
        let mapping = params.column_mapping.clone();
        let start_row = params.start_row;
        move || read_spreadsheet_rows(&work_path, &sheet_name, &mapping, start_row)
    })
    .await??;

    let mut key_to_row: HashMap<String, usize> = HashMap::new();
    for (row_idx, row) in existing_rows.iter().enumerate() {
        if let Some(key) = &row.jira_key {
            if !key.trim().is_empty() {
                key_to_row.insert(key.clone(), params.start_row + row_idx);
            }
        }
    }

    let mut edits: Vec<EditOp> = Vec::new();
    let mut report = SyncReport::new();

    for issue in jira_issues {
        let jira_key = issue.key.clone();
        let jira_updated = parse_jira_timestamp(&issue.fields, "updated");

        if let Some(&row_num) = key_to_row.get(&jira_key) {
            let existing_row = &existing_rows[row_num - params.start_row];
            if let (Some(jira_ts), Some(sheet_ts)) = (jira_updated, existing_row.updated) {
                if sheet_ts > jira_ts {
                    match params.conflict_resolution {
                        ConflictResolution::JiraWins => {
                            report.conflicts.push(ConflictReport {
                                row: row_num,
                                jira_key: jira_key.clone(),
                                reason: format!(
                                    "Spreadsheet newer ({}) vs Jira ({})",
                                    sheet_ts, jira_ts
                                ),
                                resolution: "Jira wins".to_string(),
                            });
                        }
                        ConflictResolution::SpreadsheetWins => {
                            report.conflicts.push(ConflictReport {
                                row: row_num,
                                jira_key: jira_key.clone(),
                                reason: format!(
                                    "Spreadsheet newer ({}) vs Jira ({})",
                                    sheet_ts, jira_ts
                                ),
                                resolution: "Spreadsheet wins".to_string(),
                            });
                            report.skipped += 1;
                            continue;
                        }
                        ConflictResolution::Skip => {
                            report.conflicts.push(ConflictReport {
                                row: row_num,
                                jira_key: jira_key.clone(),
                                reason: format!(
                                    "Spreadsheet newer ({}) vs Jira ({})",
                                    sheet_ts, jira_ts
                                ),
                                resolution: "Skipped".to_string(),
                            });
                            report.skipped += 1;
                            continue;
                        }
                    }
                }
            }
            edits.extend(build_row_edits(
                &params.sheet_name,
                row_num,
                &params.column_mapping,
                &issue,
            ));
            report.updated += 1;
        } else {
            let new_row = params.start_row + existing_rows.len() + report.created;
            edits.extend(build_row_edits(
                &params.sheet_name,
                new_row,
                &params.column_mapping,
                &issue,
            ));
            report.created += 1;
        }
    }

    if !edits.is_empty() {
        let edits_for_fork = edits.clone();
        tokio::task::spawn_blocking({
            let work_path = work_path.clone();
            let sheet_name = params.sheet_name.clone();
            move || apply_edits_to_spreadsheet(&work_path, &sheet_name, &edits)
        })
        .await??;
        registry.with_fork_mut(&params.fork_id, |ctx| {
            ctx.edits.extend(edits_for_fork);
            Ok(())
        })?;
    }

    let fork_workbook_id = WorkbookId(params.fork_id.clone());
    let _ = state.close_workbook(&fork_workbook_id);

    Ok(SyncJiraToSpreadsheetResponse {
        fork_id: params.fork_id,
        report,
    })
}

// TOOL 2: Spreadsheet → Jira
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SyncSpreadsheetToJiraParams {
    pub workbook_or_fork_id: WorkbookId,
    pub sheet_name: String,
    pub jira_base_url: String,
    pub jira_auth_token: String,
    pub jira_project_key: String,
    #[serde(default)]
    pub column_mapping: JiraSyncColumnMapping,
    #[serde(default = "default_start_row")]
    pub start_row: usize,
    #[serde(default)]
    pub end_row: Option<usize>,
    #[serde(default)]
    pub conflict_resolution: ConflictResolution,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SyncSpreadsheetToJiraResponse {
    pub report: SyncReport,
}

pub async fn sync_spreadsheet_to_jira(
    state: Arc<AppState>,
    params: SyncSpreadsheetToJiraParams,
) -> Result<SyncSpreadsheetToJiraResponse> {
    let workbook = state.open_workbook(&params.workbook_or_fork_id).await?;
    let work_path = workbook.path.clone();

    let jira_client =
        JiraClient::new(params.jira_base_url.clone(), params.jira_auth_token.clone())?;

    let spreadsheet_rows = tokio::task::spawn_blocking({
        let work_path = work_path.clone();
        let sheet_name = params.sheet_name.clone();
        let mapping = params.column_mapping.clone();
        let start_row = params.start_row;
        move || read_spreadsheet_rows(&work_path, &sheet_name, &mapping, start_row)
    })
    .await??;

    let mut report = SyncReport::new();
    let jira_fields = vec!["summary".to_string(), "updated".to_string()];

    for (row_idx, row) in spreadsheet_rows.iter().enumerate() {
        let row_num = params.start_row + row_idx;
        if row.summary.is_none() || row.summary.as_ref().unwrap().trim().is_empty() {
            continue;
        }

        if let Some(ref jira_key) = row.jira_key {
            if !jira_key.trim().is_empty() {
                match jira_client.get_issue(jira_key, &jira_fields).await {
                    Ok(existing_issue) => {
                        let jira_updated = parse_jira_timestamp(&existing_issue.fields, "updated");
                        if let (Some(jira_ts), Some(sheet_ts)) = (jira_updated, row.updated) {
                            if jira_ts > sheet_ts {
                                match params.conflict_resolution {
                                    ConflictResolution::SpreadsheetWins => {
                                        report.conflicts.push(ConflictReport {
                                            row: row_num,
                                            jira_key: jira_key.clone(),
                                            reason: format!(
                                                "Jira newer ({}) vs Spreadsheet ({})",
                                                jira_ts, sheet_ts
                                            ),
                                            resolution: "Spreadsheet wins".to_string(),
                                        });
                                    }
                                    ConflictResolution::JiraWins => {
                                        report.conflicts.push(ConflictReport {
                                            row: row_num,
                                            jira_key: jira_key.clone(),
                                            reason: format!(
                                                "Jira newer ({}) vs Spreadsheet ({})",
                                                jira_ts, sheet_ts
                                            ),
                                            resolution: "Jira wins".to_string(),
                                        });
                                        report.skipped += 1;
                                        continue;
                                    }
                                    ConflictResolution::Skip => {
                                        report.conflicts.push(ConflictReport {
                                            row: row_num,
                                            jira_key: jira_key.clone(),
                                            reason: format!(
                                                "Jira newer ({}) vs Spreadsheet ({})",
                                                jira_ts, sheet_ts
                                            ),
                                            resolution: "Skipped".to_string(),
                                        });
                                        report.skipped += 1;
                                        continue;
                                    }
                                }
                            }
                        }
                        let field_update = build_field_update(&row);
                        match jira_client.update_issue(jira_key, &field_update).await {
                            Ok(_) => {
                                report.updated += 1;
                            }
                            Err(e) => {
                                report.errors.push(SyncError {
                                    row: row_num,
                                    jira_key: Some(jira_key.clone()),
                                    error: format!("Update failed: {}", e),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        report.errors.push(SyncError {
                            row: row_num,
                            jira_key: Some(jira_key.clone()),
                            error: format!("Fetch failed: {}", e),
                        });
                    }
                }
            } else {
                let field_update = build_field_update(&row);
                match jira_client
                    .create_issue(&params.jira_project_key, &field_update)
                    .await
                {
                    Ok(new_key) => {
                        report.created += 1;
                        tracing::info!(row = row_num, jira_key = %new_key, "created Jira issue");
                    }
                    Err(e) => {
                        report.errors.push(SyncError {
                            row: row_num,
                            jira_key: None,
                            error: format!("Create failed: {}", e),
                        });
                    }
                }
            }
        } else {
            let field_update = build_field_update(&row);
            match jira_client
                .create_issue(&params.jira_project_key, &field_update)
                .await
            {
                Ok(new_key) => {
                    report.created += 1;
                    tracing::info!(row = row_num, jira_key = %new_key, "created Jira issue");
                }
                Err(e) => {
                    report.errors.push(SyncError {
                        row: row_num,
                        jira_key: None,
                        error: format!("Create failed: {}", e),
                    });
                }
            }
        }
    }

    Ok(SyncSpreadsheetToJiraResponse { report })
}

// Helper functions
#[derive(Debug, Clone)]
struct SpreadsheetRow {
    jira_key: Option<String>,
    summary: Option<String>,
    status: Option<String>,
    assignee: Option<String>,
    updated: Option<DateTime<Utc>>,
    description: Option<String>,
    priority: Option<String>,
    labels: Option<Vec<String>>,
}

fn read_spreadsheet_rows(
    path: &std::path::Path,
    sheet_name: &str,
    mapping: &JiraSyncColumnMapping,
    start_row: usize,
) -> Result<Vec<SpreadsheetRow>> {
    let book = umya_spreadsheet::reader::xlsx::read(path).context("read spreadsheet failed")?;
    let sheet = book
        .get_sheet_by_name(sheet_name)
        .ok_or_else(|| anyhow!("sheet '{}' not found", sheet_name))?;
    let mut rows = Vec::new();
    let mut row_num = start_row;
    loop {
        let jira_key = get_cell_value(sheet, &mapping.jira_key_column, row_num);
        let summary = get_cell_value(sheet, &mapping.summary_column, row_num);
        if summary.is_none() || summary.as_ref().unwrap().trim().is_empty() {
            break;
        }
        let status = mapping
            .status_column
            .as_ref()
            .and_then(|col| get_cell_value(sheet, col, row_num));
        let assignee = mapping
            .assignee_column
            .as_ref()
            .and_then(|col| get_cell_value(sheet, col, row_num));
        let updated = mapping
            .updated_column
            .as_ref()
            .and_then(|col| get_cell_value(sheet, col, row_num))
            .and_then(|s| parse_timestamp(&s));
        let description = mapping
            .description_column
            .as_ref()
            .and_then(|col| get_cell_value(sheet, col, row_num));
        let priority = mapping
            .priority_column
            .as_ref()
            .and_then(|col| get_cell_value(sheet, col, row_num));
        let labels = mapping
            .labels_column
            .as_ref()
            .and_then(|col| get_cell_value(sheet, col, row_num))
            .map(|s| s.split(',').map(|l| l.trim().to_string()).collect());
        rows.push(SpreadsheetRow {
            jira_key,
            summary,
            status,
            assignee,
            updated,
            description,
            priority,
            labels,
        });
        row_num += 1;
    }
    Ok(rows)
}

fn get_cell_value(sheet: &umya_spreadsheet::Worksheet, column: &str, row: usize) -> Option<String> {
    let address = format!("{}{}", column, row);
    sheet
        .get_cell(address.as_str())
        .map(|cell| cell.get_value().to_string())
}

fn build_row_edits(
    sheet_name: &str,
    row_num: usize,
    mapping: &JiraSyncColumnMapping,
    issue: &JiraIssue,
) -> Vec<EditOp> {
    let mut edits = Vec::new();
    let now = Utc::now();
    edits.push(EditOp {
        timestamp: now,
        sheet: sheet_name.to_string(),
        address: format!("{}{}", mapping.jira_key_column, row_num),
        value: issue.key.clone(),
        is_formula: false,
    });
    if let Some(summary) = issue.fields.get("summary").and_then(|v| v.as_str()) {
        edits.push(EditOp {
            timestamp: now,
            sheet: sheet_name.to_string(),
            address: format!("{}{}", mapping.summary_column, row_num),
            value: summary.to_string(),
            is_formula: false,
        });
    }
    if let Some(ref col) = mapping.status_column {
        if let Some(status) = issue
            .fields
            .get("status")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
        {
            edits.push(EditOp {
                timestamp: now,
                sheet: sheet_name.to_string(),
                address: format!("{}{}", col, row_num),
                value: status.to_string(),
                is_formula: false,
            });
        }
    }
    if let Some(ref col) = mapping.assignee_column {
        if let Some(assignee) = issue
            .fields
            .get("assignee")
            .and_then(|v| v.get("displayName"))
            .and_then(|v| v.as_str())
        {
            edits.push(EditOp {
                timestamp: now,
                sheet: sheet_name.to_string(),
                address: format!("{}{}", col, row_num),
                value: assignee.to_string(),
                is_formula: false,
            });
        }
    }
    if let Some(ref col) = mapping.updated_column {
        if let Some(updated) = issue.fields.get("updated").and_then(|v| v.as_str()) {
            edits.push(EditOp {
                timestamp: now,
                sheet: sheet_name.to_string(),
                address: format!("{}{}", col, row_num),
                value: updated.to_string(),
                is_formula: false,
            });
        }
    }
    if let Some(ref col) = mapping.description_column {
        if let Some(desc) = issue.fields.get("description").and_then(|v| v.as_str()) {
            edits.push(EditOp {
                timestamp: now,
                sheet: sheet_name.to_string(),
                address: format!("{}{}", col, row_num),
                value: desc.to_string(),
                is_formula: false,
            });
        }
    }
    if let Some(ref col) = mapping.priority_column {
        if let Some(priority) = issue
            .fields
            .get("priority")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
        {
            edits.push(EditOp {
                timestamp: now,
                sheet: sheet_name.to_string(),
                address: format!("{}{}", col, row_num),
                value: priority.to_string(),
                is_formula: false,
            });
        }
    }
    if let Some(ref col) = mapping.labels_column {
        if let Some(labels) = issue.fields.get("labels").and_then(|v| v.as_array()) {
            let labels_str = labels
                .iter()
                .filter_map(|l| l.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            edits.push(EditOp {
                timestamp: now,
                sheet: sheet_name.to_string(),
                address: format!("{}{}", col, row_num),
                value: labels_str,
                is_formula: false,
            });
        }
    }
    edits
}

fn build_field_update(row: &SpreadsheetRow) -> JiraFieldUpdate {
    JiraFieldUpdate {
        summary: row.summary.clone(),
        status: row.status.clone(),
        assignee: row.assignee.clone(),
        description: row.description.clone(),
        priority: row.priority.clone(),
        labels: row.labels.clone(),
        custom_fields: HashMap::new(),
    }
}

fn apply_edits_to_spreadsheet(
    path: &std::path::Path,
    sheet_name: &str,
    edits: &[EditOp],
) -> Result<()> {
    let mut book = umya_spreadsheet::reader::xlsx::read(path).context("read spreadsheet failed")?;
    let sheet = book
        .get_sheet_by_name_mut(sheet_name)
        .ok_or_else(|| anyhow!("sheet '{}' not found", sheet_name))?;
    for edit in edits {
        let cell = sheet.get_cell_mut(edit.address.as_str());
        if edit.is_formula {
            cell.set_formula(edit.value.clone());
        } else {
            cell.set_value(edit.value.clone());
        }
    }
    umya_spreadsheet::writer::xlsx::write(&book, path).context("write spreadsheet failed")
}

fn parse_jira_timestamp(fields: &serde_json::Value, field_name: &str) -> Option<DateTime<Utc>> {
    fields
        .get(field_name)
        .and_then(|v| v.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
}

fn parse_timestamp(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_column_mapping() {
        let mapping = JiraSyncColumnMapping::default();
        assert_eq!(mapping.jira_key_column, "A");
        assert_eq!(mapping.summary_column, "B");
        assert_eq!(mapping.status_column, Some("C".to_string()));
    }

    #[test]
    fn test_conflict_resolution_default() {
        assert!(matches!(
            ConflictResolution::default(),
            ConflictResolution::JiraWins
        ));
    }

    #[test]
    fn test_parse_timestamp_rfc3339() {
        assert!(parse_timestamp("2024-01-01T12:00:00Z").is_some());
    }

    #[test]
    fn test_parse_timestamp_naive() {
        assert!(parse_timestamp("2024-01-01 12:00:00").is_some());
    }

    #[test]
    fn test_sync_report_new() {
        let report = SyncReport::new();
        assert_eq!(report.created, 0);
        assert_eq!(report.updated, 0);
        assert_eq!(report.skipped, 0);
        assert!(report.errors.is_empty());
        assert!(report.conflicts.is_empty());
    }

    #[test]
    fn test_field_update_to_json() {
        let update = JiraFieldUpdate {
            summary: Some("Test".to_string()),
            status: Some("Done".to_string()),
            assignee: None,
            description: None,
            priority: None,
            labels: Some(vec!["label1".to_string()]),
            custom_fields: HashMap::new(),
        };
        let json = update.to_json();
        assert_eq!(json.get("summary").unwrap(), "Test");
        assert_eq!(json.get("status").unwrap().get("name").unwrap(), "Done");
        assert_eq!(json.get("labels").unwrap().as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_build_field_update() {
        let row = SpreadsheetRow {
            jira_key: Some("PROJ-1".to_string()),
            summary: Some("Test task".to_string()),
            status: Some("In Progress".to_string()),
            assignee: Some("user@example.com".to_string()),
            updated: None,
            description: None,
            priority: None,
            labels: None,
        };
        let update = build_field_update(&row);
        assert_eq!(update.summary, Some("Test task".to_string()));
        assert_eq!(update.status, Some("In Progress".to_string()));
        assert_eq!(update.assignee, Some("user@example.com".to_string()));
    }

    #[test]
    fn test_default_start_row() {
        assert_eq!(default_start_row(), 2);
    }
}
