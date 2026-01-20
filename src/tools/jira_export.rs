//! Jira Export Tool - Create Jira tickets from spreadsheet data
//!
//! Reads spreadsheet rows → Creates Jira tickets via REST API v3
//!
//! ## Features
//! - Batch ticket creation from spreadsheet
//! - Column mapping to Jira fields
//! - Bearer/Basic Auth support
//! - Rate limiting (100ms delay)
//! - Per-ticket error handling
//! - Dry-run validation
//!
//! ## Safety
//! - Input validation (poka-yoke)
//! - HTTP timeout (30s)
//! - Rate limiting (anti-abuse)
//! - Error context per row

use crate::audit::integration::audit_tool;
use crate::error::{ErrorCode, McpError as CustomMcpError};
use crate::model::WorkbookId;
use crate::state::AppState;
use crate::validation::{validate_non_empty_string, validate_numeric_range};
use anyhow::{Context, Result, anyhow};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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
const MAX_COLUMN_NAME_LENGTH: usize = 10;

// =============================================================================
// Parameter Structs
// =============================================================================

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct CreateJiraTicketsParams {
    /// Workbook ID
    pub workbook_id: WorkbookId,
    /// Sheet name
    pub sheet_name: String,
    /// Jira project key
    pub jira_project_key: String,
    /// Jira URL
    pub jira_url: String,
    /// Auth credentials
    pub jira_auth: JiraAuth,
    /// Column mapping
    pub column_mapping: JiraColumnMapping,
    /// Dry run (default: false)
    #[serde(default)]
    pub dry_run: bool,
    /// Start row (default: 2)
    #[serde(default = "default_start_row")]
    pub start_row: u32,
    /// Max tickets (default: 100)
    #[serde(default = "default_max_tickets")]
    pub max_tickets: usize,
}

fn default_start_row() -> u32 {
    2
}

fn default_max_tickets() -> usize {
    100
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JiraAuth {
    Bearer {
        token: String,
        #[serde(default)]
        email: Option<String>,
    },
    Basic {
        username: String,
        password: String,
    },
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct JiraColumnMapping {
    pub summary_column: String,
    pub description_column: String,
    pub issue_type_column: String,
    #[serde(default)]
    pub priority_column: Option<String>,
    #[serde(default)]
    pub assignee_column: Option<String>,
    #[serde(default)]
    pub labels_column: Option<String>,
    #[serde(default)]
    pub epic_link_column: Option<String>,
    #[serde(default)]
    pub story_points_column: Option<String>,
}

// =============================================================================
// Response Structs
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateJiraTicketsResponse {
    pub workbook_id: WorkbookId,
    pub sheet_name: String,
    pub dry_run: bool,
    pub total_rows_processed: usize,
    pub tickets_created: usize,
    pub tickets_failed: usize,
    pub results: Vec<JiraTicketResult>,
    pub notes: Vec<String>,
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

// =============================================================================
// Internal Structures
// =============================================================================

#[derive(Debug, Clone)]
struct JiraTicketData {
    row: u32,
    summary: String,
    description: String,
    issue_type: String,
    priority: Option<String>,
    assignee: Option<String>,
    labels: Vec<String>,
    epic_link: Option<String>,
    story_points: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct JiraCreateResponse {
    key: String,
    #[serde(rename = "self")]
    self_link: String,
}

#[derive(Debug, Serialize)]
struct JiraCreateRequest {
    fields: JiraFields,
}

#[derive(Debug, Serialize)]
struct JiraFields {
    project: JiraProject,
    summary: String,
    description: JiraDescription,
    issuetype: JiraIssueType,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<JiraPriority>,
    #[serde(skip_serializing_if = "Option::is_none")]
    assignee: Option<JiraAssignee>,
    #[serde(skip_serializing_if = "Option::is_none")]
    labels: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct JiraProject {
    key: String,
}

#[derive(Debug, Serialize)]
struct JiraIssueType {
    name: String,
}

#[derive(Debug, Serialize)]
struct JiraPriority {
    name: String,
}

#[derive(Debug, Serialize)]
struct JiraAssignee {
    name: String,
}

#[derive(Debug, Serialize)]
struct JiraDescription {
    #[serde(rename = "type")]
    doc_type: String,
    version: u32,
    content: Vec<JiraContent>,
}

#[derive(Debug, Serialize)]
struct JiraContent {
    #[serde(rename = "type")]
    content_type: String,
    content: Vec<JiraText>,
}

#[derive(Debug, Serialize)]
struct JiraText {
    #[serde(rename = "type")]
    text_type: String,
    text: String,
}

// =============================================================================
// Validation
// =============================================================================

impl CreateJiraTicketsParams {
    fn validate(&self) -> Result<()> {
        validate_non_empty_string("workbook_id", &self.workbook_id.0)?;
        validate_non_empty_string("sheet_name", &self.sheet_name)?;
        validate_non_empty_string("jira_project_key", &self.jira_project_key)?;
        validate_non_empty_string("jira_url", &self.jira_url)?;

        if !self.jira_url.starts_with("http://") && !self.jira_url.starts_with("https://") {
            return Err(anyhow!("jira_url must start with http:// or https://"));
        }

        self.jira_auth.validate()?;
        self.column_mapping.validate()?;

        validate_numeric_range("start_row", self.start_row as usize, 1, 1_048_576)?;

        if self.max_tickets == 0 || self.max_tickets > MAX_BATCH_SIZE {
            return Err(anyhow!(
                "max_tickets must be between 1 and {}",
                MAX_BATCH_SIZE
            ));
        }

        Ok(())
    }
}

impl JiraAuth {
    fn validate(&self) -> Result<()> {
        match self {
            JiraAuth::Bearer { token, email } => {
                validate_non_empty_string("bearer_token", token)?;
                if let Some(e) = email {
                    validate_non_empty_string("email", e)?;
                }
            }
            JiraAuth::Basic { username, password } => {
                validate_non_empty_string("username", username)?;
                validate_non_empty_string("password", password)?;
            }
        }
        Ok(())
    }

    fn to_auth_header(&self) -> String {
        match self {
            JiraAuth::Bearer { token, email } => {
                if let Some(email) = email {
                    let credentials = format!("{}:{}", email, token);
                    let encoded = base64_encode(credentials.as_bytes());
                    format!("Basic {}", encoded)
                } else {
                    format!("Bearer {}", token)
                }
            }
            JiraAuth::Basic { username, password } => {
                let credentials = format!("{}:{}", username, password);
                let encoded = base64_encode(credentials.as_bytes());
                format!("Basic {}", encoded)
            }
        }
    }
}

impl JiraColumnMapping {
    fn validate(&self) -> Result<()> {
        Self::validate_column_name(&self.summary_column, "summary_column")?;
        Self::validate_column_name(&self.description_column, "description_column")?;
        Self::validate_column_name(&self.issue_type_column, "issue_type_column")?;

        if let Some(ref col) = self.priority_column {
            Self::validate_column_name(col, "priority_column")?;
        }
        if let Some(ref col) = self.assignee_column {
            Self::validate_column_name(col, "assignee_column")?;
        }
        if let Some(ref col) = self.labels_column {
            Self::validate_column_name(col, "labels_column")?;
        }
        if let Some(ref col) = self.epic_link_column {
            Self::validate_column_name(col, "epic_link_column")?;
        }
        if let Some(ref col) = self.story_points_column {
            Self::validate_column_name(col, "story_points_column")?;
        }

        Ok(())
    }

    fn validate_column_name(col: &str, field_name: &str) -> Result<()> {
        validate_non_empty_string(field_name, col)?;

        if col.len() > MAX_COLUMN_NAME_LENGTH {
            return Err(anyhow!(
                "{} exceeds max length of {}",
                field_name,
                MAX_COLUMN_NAME_LENGTH
            ));
        }

        if !col.chars().all(|c| c.is_ascii_uppercase()) {
            return Err(anyhow!(
                "{} must contain only uppercase letters (A-ZZZ)",
                field_name
            ));
        }

        Ok(())
    }
}

// =============================================================================
// Main Tool Function
// =============================================================================

pub async fn create_jira_tickets_from_spreadsheet(
    state: Arc<AppState>,
    params: CreateJiraTicketsParams,
) -> Result<CreateJiraTicketsResponse> {
    let _span = audit_tool("create_jira_tickets_from_spreadsheet", &params);

    params.validate().context("parameter validation failed")?;

    info!(
        "Creating Jira tickets from {} sheet {} (dry_run: {})",
        params.workbook_id.0, params.sheet_name, params.dry_run
    );

    let workbook = state
        .open_workbook(&params.workbook_id)
        .await
        .context("failed to open workbook")?;

    let ticket_data = extract_ticket_data_from_sheet(&workbook, &params)
        .context("failed to extract ticket data")?;

    if ticket_data.is_empty() {
        return Ok(CreateJiraTicketsResponse {
            workbook_id: params.workbook_id,
            sheet_name: params.sheet_name,
            dry_run: params.dry_run,
            total_rows_processed: 0,
            tickets_created: 0,
            tickets_failed: 0,
            results: vec![],
            notes: vec!["No ticket data found".to_string()],
        });
    }

    info!("Extracted {} ticket(s)", ticket_data.len());

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(JIRA_API_TIMEOUT_SECS))
        .build()
        .context("failed to create HTTP client")?;

    let mut results = Vec::new();
    let mut tickets_created = 0;
    let mut tickets_failed = 0;

    for ticket in ticket_data {
        if params.dry_run {
            results.push(JiraTicketResult {
                row: ticket.row,
                success: true,
                ticket_key: None,
                ticket_url: None,
                summary: ticket.summary.clone(),
                error: None,
            });
            tickets_created += 1;
        } else {
            match create_jira_ticket(&client, &params, &ticket).await {
                Ok((key, url)) => {
                    info!("Created {} for row {}", key, ticket.row);
                    results.push(JiraTicketResult {
                        row: ticket.row,
                        success: true,
                        ticket_key: Some(key),
                        ticket_url: Some(url),
                        summary: ticket.summary.clone(),
                        error: None,
                    });
                    tickets_created += 1;
                }
                Err(e) => {
                    error!("Failed row {}: {}", ticket.row, e);
                    results.push(JiraTicketResult {
                        row: ticket.row,
                        success: false,
                        ticket_key: None,
                        ticket_url: None,
                        summary: ticket.summary.clone(),
                        error: Some(format!("{:#}", e)),
                    });
                    tickets_failed += 1;
                }
            }

            sleep(Duration::from_millis(JIRA_RATE_LIMIT_DELAY_MS)).await;
        }
    }

    let mut notes = Vec::new();
    if params.dry_run {
        notes.push("Dry run: no tickets created".to_string());
    }
    if tickets_failed > 0 {
        notes.push(format!("{} ticket(s) failed", tickets_failed));
    }

    Ok(CreateJiraTicketsResponse {
        workbook_id: params.workbook_id,
        sheet_name: params.sheet_name,
        dry_run: params.dry_run,
        total_rows_processed: results.len(),
        tickets_created,
        tickets_failed,
        results,
        notes,
    })
}

// =============================================================================
// Helper Functions
// =============================================================================

fn extract_ticket_data_from_sheet(
    workbook: &crate::workbook::WorkbookContext,
    params: &CreateJiraTicketsParams,
) -> Result<Vec<JiraTicketData>> {
    workbook.with_sheet(&params.sheet_name, |sheet| {
        let mut tickets = Vec::new();

        let start_row = params.start_row;
        let max_row = start_row + params.max_tickets as u32;

        for row_idx in start_row..=max_row {
            let summary = get_cell_value(sheet, &params.column_mapping.summary_column, row_idx);
            let description =
                get_cell_value(sheet, &params.column_mapping.description_column, row_idx);
            let issue_type =
                get_cell_value(sheet, &params.column_mapping.issue_type_column, row_idx);

            if summary.trim().is_empty() {
                continue;
            }

            if issue_type.trim().is_empty() {
                warn!("Row {}: issue_type empty, skipping", row_idx);
                continue;
            }

            let priority = params
                .column_mapping
                .priority_column
                .as_ref()
                .map(|col| get_cell_value(sheet, col, row_idx))
                .filter(|s| !s.trim().is_empty());

            let assignee = params
                .column_mapping
                .assignee_column
                .as_ref()
                .map(|col| get_cell_value(sheet, col, row_idx))
                .filter(|s| !s.trim().is_empty());

            let labels = params
                .column_mapping
                .labels_column
                .as_ref()
                .map(|col| {
                    let raw = get_cell_value(sheet, col, row_idx);
                    raw.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let epic_link = params
                .column_mapping
                .epic_link_column
                .as_ref()
                .map(|col| get_cell_value(sheet, col, row_idx))
                .filter(|s| !s.trim().is_empty());

            let story_points = params
                .column_mapping
                .story_points_column
                .as_ref()
                .and_then(|col| {
                    let raw = get_cell_value(sheet, col, row_idx);
                    raw.trim().parse::<f64>().ok()
                });

            tickets.push(JiraTicketData {
                row: row_idx,
                summary,
                description,
                issue_type,
                priority,
                assignee,
                labels,
                epic_link,
                story_points,
            });
        }

        Ok(tickets)
    })?
}

fn get_cell_value(sheet: &umya_spreadsheet::Worksheet, column: &str, row: u32) -> String {
    let cell_ref = format!("{}{}", column, row);
    sheet
        .get_cell(&cell_ref)
        .and_then(|cell| cell.get_value().as_ref().map(|v| v.to_string()))
        .unwrap_or_default()
}

async fn create_jira_ticket(
    client: &reqwest::Client,
    params: &CreateJiraTicketsParams,
    ticket: &JiraTicketData,
) -> Result<(String, String)> {
    let url = format!("{}/rest/api/3/issue", params.jira_url.trim_end_matches('/'));

    let request = build_jira_request(params, ticket);

    debug!("Creating {} in {}", ticket.summary, params.jira_project_key);

    let response = client
        .post(&url)
        .header("Authorization", params.jira_auth.to_auth_header())
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .context("failed to send Jira API request")?;

    let status = response.status();

    if !status.is_success() {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "unable to read error body".to_string());
        return Err(anyhow!("Jira API status {}: {}", status, error_body));
    }

    let jira_response: JiraCreateResponse = response
        .json()
        .await
        .context("failed to parse Jira response")?;

    let ticket_url = format!(
        "{}/browse/{}",
        params.jira_url.trim_end_matches('/'),
        jira_response.key
    );

    Ok((jira_response.key, ticket_url))
}

fn build_jira_request(
    params: &CreateJiraTicketsParams,
    ticket: &JiraTicketData,
) -> JiraCreateRequest {
    let description = JiraDescription {
        doc_type: "doc".to_string(),
        version: 1,
        content: vec![JiraContent {
            content_type: "paragraph".to_string(),
            content: vec![JiraText {
                text_type: "text".to_string(),
                text: ticket.description.clone(),
            }],
        }],
    };

    let fields = JiraFields {
        project: JiraProject {
            key: params.jira_project_key.clone(),
        },
        summary: ticket.summary.clone(),
        description,
        issuetype: JiraIssueType {
            name: ticket.issue_type.clone(),
        },
        priority: ticket.priority.as_ref().map(|p| JiraPriority {
            name: p.clone(),
        }),
        assignee: ticket.assignee.as_ref().map(|a| JiraAssignee {
            name: a.clone(),
        }),
        labels: if ticket.labels.is_empty() {
            None
        } else {
            Some(ticket.labels.clone())
        },
    };

    JiraCreateRequest { fields }
}

// =============================================================================
// Base64 Encoding
// =============================================================================

fn base64_encode(input: &[u8]) -> String {
    use std::fmt::Write;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::new();
    let mut i = 0;

    while i + 2 < input.len() {
        let b1 = input[i];
        let b2 = input[i + 1];
        let b3 = input[i + 2];

        let _ = write!(
            &mut result,
            "{}{}{}{}",
            CHARS[(b1 >> 2) as usize] as char,
            CHARS[(((b1 & 0x03) << 4) | (b2 >> 4)) as usize] as char,
            CHARS[(((b2 & 0x0f) << 2) | (b3 >> 6)) as usize] as char,
            CHARS[(b3 & 0x3f) as usize] as char
        );

        i += 3;
    }

    if i < input.len() {
        let b1 = input[i];
        let b2 = if i + 1 < input.len() {
            input[i + 1]
        } else {
            0
        };

        let _ = write!(
            &mut result,
            "{}{}",
            CHARS[(b1 >> 2) as usize] as char,
            CHARS[(((b1 & 0x03) << 4) | (b2 >> 4)) as usize] as char
        );

        if i + 1 < input.len() {
            let _ = write!(
                &mut result,
                "{}",
                CHARS[((b2 & 0x0f) << 2) as usize] as char
            );
        } else {
            result.push('=');
        }

        result.push('=');
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
    fn test_params_validation_success() {
        let params = CreateJiraTicketsParams {
            workbook_id: WorkbookId("test.xlsx".to_string()),
            sheet_name: "Sheet1".to_string(),
            jira_project_key: "PROJ".to_string(),
            jira_url: "https://company.atlassian.net".to_string(),
            jira_auth: JiraAuth::Bearer {
                token: "test-token".to_string(),
                email: None,
            },
            column_mapping: JiraColumnMapping {
                summary_column: "A".to_string(),
                description_column: "B".to_string(),
                issue_type_column: "C".to_string(),
                priority_column: Some("D".to_string()),
                assignee_column: None,
                labels_column: None,
                epic_link_column: None,
                story_points_column: None,
            },
            dry_run: false,
            start_row: 2,
            max_tickets: 10,
        };

        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_params_validation_empty_workbook_id() {
        let params = CreateJiraTicketsParams {
            workbook_id: WorkbookId("".to_string()),
            sheet_name: "Sheet1".to_string(),
            jira_project_key: "PROJ".to_string(),
            jira_url: "https://company.atlassian.net".to_string(),
            jira_auth: JiraAuth::Bearer {
                token: "test-token".to_string(),
                email: None,
            },
            column_mapping: JiraColumnMapping {
                summary_column: "A".to_string(),
                description_column: "B".to_string(),
                issue_type_column: "C".to_string(),
                priority_column: None,
                assignee_column: None,
                labels_column: None,
                epic_link_column: None,
                story_points_column: None,
            },
            dry_run: false,
            start_row: 2,
            max_tickets: 10,
        };

        assert!(params.validate().is_err());
    }

    #[test]
    fn test_params_validation_invalid_url() {
        let params = CreateJiraTicketsParams {
            workbook_id: WorkbookId("test.xlsx".to_string()),
            sheet_name: "Sheet1".to_string(),
            jira_project_key: "PROJ".to_string(),
            jira_url: "invalid-url".to_string(),
            jira_auth: JiraAuth::Bearer {
                token: "test-token".to_string(),
                email: None,
            },
            column_mapping: JiraColumnMapping {
                summary_column: "A".to_string(),
                description_column: "B".to_string(),
                issue_type_column: "C".to_string(),
                priority_column: None,
                assignee_column: None,
                labels_column: None,
                epic_link_column: None,
                story_points_column: None,
            },
            dry_run: false,
            start_row: 2,
            max_tickets: 10,
        };

        assert!(params.validate().is_err());
    }

    #[test]
    fn test_jira_auth_bearer_header() {
        let auth = JiraAuth::Bearer {
            token: "test-token-123".to_string(),
            email: None,
        };

        let header = auth.to_auth_header();
        assert_eq!(header, "Bearer test-token-123");
    }

    #[test]
    fn test_jira_auth_basic_header() {
        let auth = JiraAuth::Basic {
            username: "user".to_string(),
            password: "pass".to_string(),
        };

        let header = auth.to_auth_header();
        assert!(header.starts_with("Basic "));
    }

    #[test]
    fn test_column_mapping_validation() {
        let mapping = JiraColumnMapping {
            summary_column: "A".to_string(),
            description_column: "B".to_string(),
            issue_type_column: "C".to_string(),
            priority_column: Some("D".to_string()),
            assignee_column: Some("E".to_string()),
            labels_column: Some("F".to_string()),
            epic_link_column: Some("G".to_string()),
            story_points_column: Some("H".to_string()),
        };

        assert!(mapping.validate().is_ok());
    }

    #[test]
    fn test_column_mapping_invalid_column() {
        let mapping = JiraColumnMapping {
            summary_column: "A1".to_string(),
            description_column: "B".to_string(),
            issue_type_column: "C".to_string(),
            priority_column: None,
            assignee_column: None,
            labels_column: None,
            epic_link_column: None,
            story_points_column: None,
        };

        assert!(mapping.validate().is_err());
    }

    #[test]
    fn test_base64_encoding() {
        let encoded = base64_encode(b"user:pass");
        assert_eq!(encoded, "dXNlcjpwYXNz");

        let encoded2 = base64_encode(b"test@example.com:api-token");
        assert!(!encoded2.is_empty());
    }
}
