//! Jira Integration Stage for ggen Sync Pipeline
//!
//! Optional compiler stage that integrates Jira after code generation.
//! Delegates to existing jira_unified tool. Zero duplication.
//!
//! ## Modes
//! - DryRun: Generate plan, don't create tickets
//! - Create: Create Jira tickets from generated files
//! - Sync: Bidirectional sync with spreadsheet
//!
//! ## Safety
//! - Optional stage (ggen.toml enabled = true)
//! - Delegates to jira_unified (no code duplication)
//! - Input validation (poka-yoke)
//! - Environment-based auth token

use crate::tools::jira_integration::{ConflictResolution, JiraSyncColumnMapping, SyncReport};
use crate::tools::jira_unified::{
    JiraOperation, ManageJiraParams, ManageJiraResponse, manage_jira_integration,
};
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

// =============================================================================
// Configuration
// =============================================================================

/// Jira integration configuration from ggen.toml
#[derive(Debug, Clone, Deserialize)]
pub struct JiraConfig {
    /// Enable Jira integration stage
    pub enabled: bool,
    /// Operating mode
    pub mode: JiraMode,
    /// Jira project key
    pub project_key: String,
    /// Jira base URL
    pub base_url: String,
    /// Environment variable for auth token
    pub auth_token_env: String,
    /// Column mapping for ticket creation/sync
    pub mapping: ColumnMapping,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JiraMode {
    /// Generate plan only, no API calls
    DryRun,
    /// Create tickets from generated files
    Create,
    /// Bidirectional sync with spreadsheet
    Sync,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ColumnMapping {
    /// Summary column (e.g., "B")
    #[serde(default = "default_summary_column")]
    pub summary_column: String,
    /// Status column (e.g., "C")
    #[serde(default = "default_status_column")]
    pub status_column: String,
    /// Assignee column (e.g., "D")
    #[serde(default = "default_assignee_column")]
    pub assignee_column: String,
    /// Description column (optional)
    pub description_column: Option<String>,
}

fn default_summary_column() -> String {
    "B".to_string()
}

fn default_status_column() -> String {
    "C".to_string()
}

fn default_assignee_column() -> String {
    "D".to_string()
}

impl JiraConfig {
    /// Parse Jira configuration from ggen.toml [jira] section
    pub fn from_toml(toml: &toml::Value) -> Result<Option<Self>> {
        let jira_section = match toml.get("jira") {
            Some(s) => s,
            None => return Ok(None),
        };

        let enabled = jira_section
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !enabled {
            return Ok(None);
        }

        let mode_str = jira_section
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("dry_run");

        let mode = match mode_str {
            "dry_run" => JiraMode::DryRun,
            "create" => JiraMode::Create,
            "sync" => JiraMode::Sync,
            _ => return Err(anyhow!("Invalid jira.mode: {}", mode_str)),
        };

        let auth_token_env = jira_section
            .get("auth_token_env")
            .and_then(|v| v.as_str())
            .unwrap_or("JIRA_TOKEN")
            .to_string();

        let auth_token = std::env::var(&auth_token_env).context(format!(
            "Missing Jira auth token in env var: {}",
            auth_token_env
        ))?;

        let project_key = jira_section
            .get("project_key")
            .and_then(|v| v.as_str())
            .context("Missing jira.project_key")?
            .to_string();

        let base_url = jira_section
            .get("base_url")
            .and_then(|v| v.as_str())
            .context("Missing jira.base_url")?
            .to_string();

        let mapping = if let Some(mapping_section) = jira_section.get("mapping") {
            ColumnMapping {
                summary_column: mapping_section
                    .get("summary_column")
                    .and_then(|v| v.as_str())
                    .unwrap_or("B")
                    .to_string(),
                status_column: mapping_section
                    .get("status_column")
                    .and_then(|v| v.as_str())
                    .unwrap_or("C")
                    .to_string(),
                assignee_column: mapping_section
                    .get("assignee_column")
                    .and_then(|v| v.as_str())
                    .unwrap_or("D")
                    .to_string(),
                description_column: mapping_section
                    .get("description_column")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            }
        } else {
            ColumnMapping {
                summary_column: default_summary_column(),
                status_column: default_status_column(),
                assignee_column: default_assignee_column(),
                description_column: None,
            }
        };

        Ok(Some(JiraConfig {
            enabled: true,
            mode,
            project_key,
            base_url,
            auth_token_env,
            mapping,
        }))
    }

    /// Get auth token from environment
    pub fn get_auth_token(&self) -> Result<String> {
        std::env::var(&self.auth_token_env).context(format!(
            "Missing Jira auth token in env var: {}",
            self.auth_token_env
        ))
    }
}

// =============================================================================
// Stage Execution
// =============================================================================

/// Jira stage executor
pub struct JiraStage;

impl JiraStage {
    /// Execute Jira integration stage
    pub async fn execute(
        state: Arc<crate::state::AppState>,
        ctx: &SyncContext,
        config: &JiraConfig,
    ) -> Result<JiraStageResult> {
        let start = Instant::now();

        match config.mode {
            JiraMode::DryRun => {
                let plan = Self::generate_plan(ctx, config)?;
                Ok(JiraStageResult {
                    mode: config.mode.clone(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    details: JiraStageDetails::DryRun(plan),
                })
            }
            JiraMode::Create => {
                let result = Self::create_tickets(state, ctx, config).await?;
                Ok(JiraStageResult {
                    mode: config.mode.clone(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    details: JiraStageDetails::Created(result),
                })
            }
            JiraMode::Sync => {
                let result = Self::sync_tickets(state, ctx, config).await?;
                Ok(JiraStageResult {
                    mode: config.mode.clone(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    details: JiraStageDetails::Synced(result),
                })
            }
        }
    }

    /// Generate plan for ticket creation (dry-run mode)
    fn generate_plan(ctx: &SyncContext, config: &JiraConfig) -> Result<JiraPlan> {
        let mut tickets = Vec::new();

        // Analyze generated files and create ticket plan
        for file in &ctx.generated_files {
            tickets.push(JiraTicketPlan {
                summary: format!("Implement {}", file.module_name),
                description: format!(
                    "Generated from ontology query: {}\nTemplate: {}\nFile: {}",
                    file.source_query, file.source_template, file.path
                ),
                labels: vec!["generated".to_string(), "ggen".to_string()],
                component: Some("code-generation".to_string()),
            });
        }

        Ok(JiraPlan {
            project_key: config.project_key.clone(),
            tickets,
            dry_run: true,
        })
    }

    /// Create Jira tickets from generated files
    async fn create_tickets(
        state: Arc<crate::state::AppState>,
        ctx: &SyncContext,
        config: &JiraConfig,
    ) -> Result<CreateTicketsResult> {
        let auth_token = config.get_auth_token()?;

        // Use jira_unified tool to create tickets
        let params = ManageJiraParams {
            workbook_or_fork_id: ctx.workbook_id.clone(),
            sheet_name: "GeneratedTickets".to_string(),
            jira_base_url: config.base_url.clone(),
            jira_auth_token: auth_token,
            operation: JiraOperation::CreateTickets {
                jira_project_key: config.project_key.clone(),
                column_mapping: crate::tools::jira_export::JiraColumnMapping {
                    summary_column: config.mapping.summary_column.clone(),
                    description_column: config
                        .mapping
                        .description_column
                        .clone()
                        .unwrap_or_else(|| "E".to_string()),
                    issue_type_column: "Task".to_string(),
                    priority_column: None,
                    assignee_column: Some(config.mapping.assignee_column.clone()),
                    labels_column: Some("generated,ggen".to_string()),
                    epic_link_column: None,
                    story_points_column: None,
                },
                dry_run: false,
                start_row: 2,
                max_tickets: 100,
            },
        };

        let response = manage_jira_integration(state, params).await?;

        // Extract result
        match response.result {
            crate::tools::jira_unified::JiraOperationResult::CreateTickets {
                tickets_created,
                tickets_failed,
                results,
                notes,
            } => Ok(CreateTicketsResult {
                created_count: tickets_created,
                failed_count: tickets_failed,
                ticket_keys: results
                    .iter()
                    .filter_map(|r| r.ticket_key.clone())
                    .collect(),
                notes,
            }),
            _ => Err(anyhow!("Unexpected Jira operation result type")),
        }
    }

    /// Sync tickets bidirectionally with spreadsheet
    async fn sync_tickets(
        state: Arc<crate::state::AppState>,
        ctx: &SyncContext,
        config: &JiraConfig,
    ) -> Result<SyncTicketsResult> {
        let auth_token = config.get_auth_token()?;

        // Use jira_unified tool for bidirectional sync
        let params = ManageJiraParams {
            workbook_or_fork_id: ctx.workbook_id.clone(),
            sheet_name: "Backlog".to_string(),
            jira_base_url: config.base_url.clone(),
            jira_auth_token: auth_token,
            operation: JiraOperation::SyncToSpreadsheet {
                fork_id: ctx.fork_id.clone().unwrap_or_default(),
                jql_query: format!("project = {}", config.project_key),
                column_mapping: JiraSyncColumnMapping {
                    jira_key_column: "A".to_string(),
                    summary_column: config.mapping.summary_column.clone(),
                    status_column: Some(config.mapping.status_column.clone()),
                    assignee_column: Some(config.mapping.assignee_column.clone()),
                    updated_column: Some("F".to_string()),
                    description_column: config.mapping.description_column.clone(),
                    priority_column: None,
                    labels_column: None,
                },
                start_row: 2,
                conflict_resolution: ConflictResolution::JiraWins,
            },
        };

        let response = manage_jira_integration(state, params).await?;

        // Extract sync report
        match response.result {
            crate::tools::jira_unified::JiraOperationResult::Sync { report } => {
                Ok(SyncTicketsResult {
                    created: report.created,
                    updated: report.updated,
                    skipped: report.skipped,
                    conflicts: report.conflicts.len(),
                })
            }
            _ => Err(anyhow!("Unexpected Jira operation result type")),
        }
    }
}

// =============================================================================
// Context & Results
// =============================================================================

/// Sync context passed from main pipeline
pub struct SyncContext {
    /// Workbook ID
    pub workbook_id: String,
    /// Fork ID (optional)
    pub fork_id: Option<String>,
    /// Generated files from sync
    pub generated_files: Vec<GeneratedFileInfo>,
}

#[derive(Debug, Clone)]
pub struct GeneratedFileInfo {
    pub path: String,
    pub module_name: String,
    pub source_query: String,
    pub source_template: String,
}

/// Jira stage execution result
#[derive(Debug, Clone, Serialize)]
pub struct JiraStageResult {
    pub mode: JiraMode,
    pub duration_ms: u64,
    pub details: JiraStageDetails,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JiraStageDetails {
    DryRun(JiraPlan),
    Created(CreateTicketsResult),
    Synced(SyncTicketsResult),
}

#[derive(Debug, Clone, Serialize)]
pub struct JiraPlan {
    pub project_key: String,
    pub tickets: Vec<JiraTicketPlan>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct JiraTicketPlan {
    pub summary: String,
    pub description: String,
    pub labels: Vec<String>,
    pub component: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateTicketsResult {
    pub created_count: usize,
    pub failed_count: usize,
    pub ticket_keys: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncTicketsResult {
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub conflicts: usize,
}

// =============================================================================
// Serialization for JiraMode
// =============================================================================

impl Serialize for JiraMode {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = match self {
            JiraMode::DryRun => "dry_run",
            JiraMode::Create => "create",
            JiraMode::Sync => "sync",
        };
        serializer.serialize_str(s)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jira_config_parsing_disabled() {
        let toml_str = r#"
            [jira]
            enabled = false
        "#;
        let toml: toml::Value = toml::from_str(toml_str).unwrap();
        let config = JiraConfig::from_toml(&toml).unwrap();
        assert!(config.is_none());
    }

    #[test]
    fn test_jira_config_parsing_dry_run() {
        std::env::set_var("JIRA_TOKEN", "test-token");

        let toml_str = r#"
            [jira]
            enabled = true
            mode = "dry_run"
            project_key = "PROJ"
            base_url = "https://company.atlassian.net"
            auth_token_env = "JIRA_TOKEN"

            [jira.mapping]
            summary_column = "B"
            status_column = "C"
            assignee_column = "D"
        "#;
        let toml: toml::Value = toml::from_str(toml_str).unwrap();
        let config = JiraConfig::from_toml(&toml).unwrap().unwrap();

        assert_eq!(config.mode, JiraMode::DryRun);
        assert_eq!(config.project_key, "PROJ");
        assert_eq!(config.base_url, "https://company.atlassian.net");
        assert_eq!(config.mapping.summary_column, "B");
    }

    #[test]
    fn test_jira_config_parsing_create_mode() {
        std::env::set_var("JIRA_API_TOKEN", "test-token-2");

        let toml_str = r#"
            [jira]
            enabled = true
            mode = "create"
            project_key = "DEMO"
            base_url = "https://demo.atlassian.net"
            auth_token_env = "JIRA_API_TOKEN"
        "#;
        let toml: toml::Value = toml::from_str(toml_str).unwrap();
        let config = JiraConfig::from_toml(&toml).unwrap().unwrap();

        assert_eq!(config.mode, JiraMode::Create);
        assert_eq!(config.project_key, "DEMO");
    }

    #[test]
    fn test_jira_config_parsing_missing_token() {
        std::env::remove_var("MISSING_TOKEN");

        let toml_str = r#"
            [jira]
            enabled = true
            mode = "sync"
            project_key = "TEST"
            base_url = "https://test.atlassian.net"
            auth_token_env = "MISSING_TOKEN"
        "#;
        let toml: toml::Value = toml::from_str(toml_str).unwrap();
        let result = JiraConfig::from_toml(&toml);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("MISSING_TOKEN"));
    }

    #[test]
    fn test_generate_plan() {
        std::env::set_var("JIRA_TOKEN", "test-token");

        let config = JiraConfig {
            enabled: true,
            mode: JiraMode::DryRun,
            project_key: "PLAN".to_string(),
            base_url: "https://plan.atlassian.net".to_string(),
            auth_token_env: "JIRA_TOKEN".to_string(),
            mapping: ColumnMapping {
                summary_column: "B".to_string(),
                status_column: "C".to_string(),
                assignee_column: "D".to_string(),
                description_column: None,
            },
        };

        let ctx = SyncContext {
            workbook_id: "test.xlsx".to_string(),
            fork_id: None,
            generated_files: vec![
                GeneratedFileInfo {
                    path: "src/generated/tool.rs".to_string(),
                    module_name: "tool".to_string(),
                    source_query: "tool.rq".to_string(),
                    source_template: "tool.rs.tera".to_string(),
                },
                GeneratedFileInfo {
                    path: "src/generated/handler.rs".to_string(),
                    module_name: "handler".to_string(),
                    source_query: "handler.rq".to_string(),
                    source_template: "handler.rs.tera".to_string(),
                },
            ],
        };

        let plan = JiraStage::generate_plan(&ctx, &config).unwrap();

        assert_eq!(plan.project_key, "PLAN");
        assert_eq!(plan.tickets.len(), 2);
        assert!(plan.dry_run);
        assert_eq!(plan.tickets[0].summary, "Implement tool");
        assert_eq!(plan.tickets[1].summary, "Implement handler");
    }

    #[test]
    fn test_default_column_mappings() {
        assert_eq!(default_summary_column(), "B");
        assert_eq!(default_status_column(), "C");
        assert_eq!(default_assignee_column(), "D");
    }

    #[test]
    fn test_jira_mode_serialization() {
        let dry_run = JiraMode::DryRun;
        let create = JiraMode::Create;
        let sync = JiraMode::Sync;

        assert_eq!(serde_json::to_string(&dry_run).unwrap(), r#""dry_run""#);
        assert_eq!(serde_json::to_string(&create).unwrap(), r#""create""#);
        assert_eq!(serde_json::to_string(&sync).unwrap(), r#""sync""#);
    }
}
