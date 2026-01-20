//! Integration tests for Jira integration tools
//!
//! Chicago-style TDD: State-based verification, mocked Jira API (wiremock), minimal mocking.
//! Tests complete Jira workflows: create tickets, sync bidirectionally, query, import, conflict resolution.

use anyhow::{Context, Result};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock;

// =============================================================================
// Mock Jira API Server (Replaces wiremock for lightweight testing)
// =============================================================================

struct MockJiraServer {
    tickets: Arc<RwLock<HashMap<String, JiraTicket>>>,
    next_id: Arc<RwLock<u32>>,
    base_url: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct JiraTicket {
    key: String,
    summary: String,
    description: String,
    status: String,
    priority: String,
    assignee: Option<String>,
    created: String,
    updated: String,
}

impl MockJiraServer {
    fn new() -> Self {
        Self {
            tickets: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(1)),
            base_url: "http://localhost:8080/jira".to_string(),
        }
    }

    async fn create_ticket(&self, summary: &str, description: &str, priority: &str) -> Result<JiraTicket> {
        let mut next_id = self.next_id.write().await;
        let id = *next_id;
        *next_id += 1;

        let ticket = JiraTicket {
            key: format!("TEST-{}", id),
            summary: summary.to_string(),
            description: description.to_string(),
            status: "Open".to_string(),
            priority: priority.to_string(),
            assignee: None,
            created: chrono::Utc::now().to_rfc3339(),
            updated: chrono::Utc::now().to_rfc3339(),
        };

        self.tickets.write().await.insert(ticket.key.clone(), ticket.clone());

        Ok(ticket)
    }

    async fn get_ticket(&self, key: &str) -> Result<JiraTicket> {
        let tickets = self.tickets.read().await;
        tickets.get(key).cloned().ok_or_else(|| anyhow::anyhow!("Ticket not found: {}", key))
    }

    async fn update_ticket(&self, key: &str, updates: JiraTicketUpdate) -> Result<JiraTicket> {
        let mut tickets = self.tickets.write().await;
        let ticket = tickets.get_mut(key).ok_or_else(|| anyhow::anyhow!("Ticket not found: {}", key))?;

        if let Some(summary) = updates.summary {
            ticket.summary = summary;
        }
        if let Some(description) = updates.description {
            ticket.description = description;
        }
        if let Some(status) = updates.status {
            ticket.status = status;
        }
        if let Some(priority) = updates.priority {
            ticket.priority = priority;
        }
        if let Some(assignee) = updates.assignee {
            ticket.assignee = Some(assignee);
        }

        ticket.updated = chrono::Utc::now().to_rfc3339();

        Ok(ticket.clone())
    }

    async fn query_tickets(&self, jql: &str) -> Result<Vec<JiraTicket>> {
        let tickets = self.tickets.read().await;

        // Simple JQL parsing (in production would use proper parser)
        let results: Vec<JiraTicket> = if jql.contains("status=Open") {
            tickets.values().filter(|t| t.status == "Open").cloned().collect()
        } else if jql.contains("priority=High") {
            tickets.values().filter(|t| t.priority == "High").cloned().collect()
        } else {
            tickets.values().cloned().collect()
        };

        Ok(results)
    }

    async fn delete_ticket(&self, key: &str) -> Result<()> {
        self.tickets.write().await.remove(key);
        Ok(())
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[derive(Debug, Default)]
struct JiraTicketUpdate {
    summary: Option<String>,
    description: Option<String>,
    status: Option<String>,
    priority: Option<String>,
    assignee: Option<String>,
}

// =============================================================================
// Test Harness for Jira Integration
// =============================================================================

struct JiraIntegrationHarness {
    workspace: TempDir,
    mock_server: MockJiraServer,
}

impl JiraIntegrationHarness {
    fn new() -> Result<Self> {
        Ok(Self {
            workspace: tempfile::tempdir()?,
            mock_server: MockJiraServer::new(),
        })
    }

    fn spreadsheet_path(&self) -> std::path::PathBuf {
        self.workspace.path().join("jira_tickets.xlsx")
    }

    fn workspace_path(&self) -> &Path {
        self.workspace.path()
    }

    async fn create_test_ticket(&self, summary: &str) -> Result<JiraTicket> {
        self.mock_server.create_ticket(summary, "Test description", "Medium").await
    }
}

// =============================================================================
// Fixtures
// =============================================================================

fn sample_spreadsheet_rows() -> Vec<HashMap<String, String>> {
    vec![
        HashMap::from([
            ("Key".to_string(), "TEST-1".to_string()),
            ("Summary".to_string(), "Implement login".to_string()),
            ("Description".to_string(), "Add user login feature".to_string()),
            ("Status".to_string(), "Open".to_string()),
            ("Priority".to_string(), "High".to_string()),
        ]),
        HashMap::from([
            ("Key".to_string(), "TEST-2".to_string()),
            ("Summary".to_string(), "Fix bug in dashboard".to_string()),
            ("Description".to_string(), "Dashboard not loading".to_string()),
            ("Status".to_string(), "In Progress".to_string()),
            ("Priority".to_string(), "Critical".to_string()),
        ]),
    ]
}

// =============================================================================
// Tests: create_jira_tickets_from_spreadsheet
// =============================================================================

#[tokio::test]
async fn test_create_tickets_from_spreadsheet() -> Result<()> {
    // GIVEN: Spreadsheet with ticket data
    let harness = JiraIntegrationHarness::new()?;
    let rows = sample_spreadsheet_rows();

    // WHEN: We create tickets from spreadsheet
    let result = simulate_create_tickets_from_spreadsheet(
        &harness.mock_server,
        rows,
    )
    .await?;

    // THEN: Tickets created successfully
    assert_eq!(result.created_count, 2);
    assert_eq!(result.failed_count, 0);

    // AND: Tickets exist in Jira
    let ticket1 = harness.mock_server.get_ticket("TEST-1").await?;
    assert_eq!(ticket1.summary, "Implement login");
    assert_eq!(ticket1.priority, "High");

    Ok(())
}

#[tokio::test]
async fn test_create_tickets_skips_existing() -> Result<()> {
    // GIVEN: Spreadsheet with some existing tickets
    let harness = JiraIntegrationHarness::new()?;
    harness.create_test_ticket("Existing ticket").await?;

    let mut rows = sample_spreadsheet_rows();
    rows[0].insert("Key".to_string(), "TEST-1".to_string());  // Already exists

    // WHEN: We create tickets (with skip_existing flag)
    let result = simulate_create_tickets_from_spreadsheet(
        &harness.mock_server,
        rows,
    )
    .await?;

    // THEN: Only new tickets created
    assert_eq!(result.created_count, 1);  // Only TEST-2
    assert_eq!(result.skipped_count, 1);  // TEST-1 skipped

    Ok(())
}

#[tokio::test]
async fn test_create_tickets_with_validation_errors() -> Result<()> {
    // GIVEN: Spreadsheet with invalid data
    let harness = JiraIntegrationHarness::new()?;
    let rows = vec![
        HashMap::from([
            ("Summary".to_string(), "".to_string()),  // Empty summary
            ("Description".to_string(), "Test".to_string()),
        ]),
    ];

    // WHEN: We try to create tickets
    let result = simulate_create_tickets_from_spreadsheet(
        &harness.mock_server,
        rows,
    )
    .await?;

    // THEN: Creation fails with validation errors
    assert_eq!(result.created_count, 0);
    assert_eq!(result.failed_count, 1);
    assert!(result.errors.len() > 0);
    assert!(result.errors[0].contains("summary") || result.errors[0].contains("empty"));

    Ok(())
}

// =============================================================================
// Tests: sync_jira_to_spreadsheet
// =============================================================================

#[tokio::test]
async fn test_sync_jira_to_spreadsheet() -> Result<()> {
    // GIVEN: Jira tickets exist
    let harness = JiraIntegrationHarness::new()?;
    harness.mock_server.create_ticket("Ticket 1", "Description 1", "High").await?;
    harness.mock_server.create_ticket("Ticket 2", "Description 2", "Medium").await?;

    // WHEN: We sync from Jira to spreadsheet
    let result = simulate_sync_jira_to_spreadsheet(
        &harness.mock_server,
        harness.spreadsheet_path().as_path(),
        "project = TEST",
    )
    .await?;

    // THEN: Spreadsheet updated with tickets
    assert_eq!(result.synced_count, 2);
    assert_eq!(result.errors.len(), 0);

    // AND: Spreadsheet file created
    assert!(harness.spreadsheet_path().exists());

    Ok(())
}

#[tokio::test]
async fn test_sync_jira_to_spreadsheet_filters_by_jql() -> Result<()> {
    // GIVEN: Mix of open and closed tickets
    let harness = JiraIntegrationHarness::new()?;
    harness.mock_server.create_ticket("Open ticket", "Description", "High").await?;

    let closed_ticket = harness.mock_server.create_ticket("Closed ticket", "Description", "Low").await?;
    harness.mock_server.update_ticket(&closed_ticket.key, JiraTicketUpdate {
        status: Some("Closed".to_string()),
        ..Default::default()
    }).await?;

    // WHEN: We sync only open tickets
    let result = simulate_sync_jira_to_spreadsheet(
        &harness.mock_server,
        harness.spreadsheet_path().as_path(),
        "status=Open",
    )
    .await?;

    // THEN: Only open tickets synced
    assert_eq!(result.synced_count, 1);

    Ok(())
}

#[tokio::test]
async fn test_sync_jira_to_spreadsheet_updates_existing() -> Result<()> {
    // GIVEN: Spreadsheet with older ticket data
    let harness = JiraIntegrationHarness::new()?;
    let ticket = harness.mock_server.create_ticket("Original summary", "Original desc", "Low").await?;

    // Create initial spreadsheet
    simulate_sync_jira_to_spreadsheet(
        &harness.mock_server,
        harness.spreadsheet_path().as_path(),
        "project = TEST",
    )
    .await?;

    // WHEN: Ticket updated in Jira
    harness.mock_server.update_ticket(&ticket.key, JiraTicketUpdate {
        summary: Some("Updated summary".to_string()),
        priority: Some("High".to_string()),
        ..Default::default()
    }).await?;

    // AND: We sync again
    let result = simulate_sync_jira_to_spreadsheet(
        &harness.mock_server,
        harness.spreadsheet_path().as_path(),
        "project = TEST",
    )
    .await?;

    // THEN: Spreadsheet updated with latest data
    assert_eq!(result.updated_count, 1);

    Ok(())
}

// =============================================================================
// Tests: sync_spreadsheet_to_jira
// =============================================================================

#[tokio::test]
async fn test_sync_spreadsheet_to_jira_creates_new() -> Result<()> {
    // GIVEN: Spreadsheet with new ticket (no Key)
    let harness = JiraIntegrationHarness::new()?;
    let rows = vec![
        HashMap::from([
            ("Summary".to_string(), "New ticket from spreadsheet".to_string()),
            ("Description".to_string(), "Description from spreadsheet".to_string()),
            ("Priority".to_string(), "High".to_string()),
        ]),
    ];

    // WHEN: We sync to Jira
    let result = simulate_sync_spreadsheet_to_jira(
        &harness.mock_server,
        rows,
    )
    .await?;

    // THEN: Ticket created in Jira
    assert_eq!(result.created_count, 1);
    assert_eq!(result.updated_count, 0);

    // AND: Ticket exists with correct data
    let all_tickets = harness.mock_server.query_tickets("").await?;
    assert_eq!(all_tickets.len(), 1);
    assert_eq!(all_tickets[0].summary, "New ticket from spreadsheet");

    Ok(())
}

#[tokio::test]
async fn test_sync_spreadsheet_to_jira_updates_existing() -> Result<()> {
    // GIVEN: Existing ticket in Jira
    let harness = JiraIntegrationHarness::new()?;
    let ticket = harness.create_test_ticket("Original summary").await?;

    // AND: Spreadsheet with updated data
    let rows = vec![
        HashMap::from([
            ("Key".to_string(), ticket.key.clone()),
            ("Summary".to_string(), "Updated from spreadsheet".to_string()),
            ("Status".to_string(), "In Progress".to_string()),
        ]),
    ];

    // WHEN: We sync to Jira
    let result = simulate_sync_spreadsheet_to_jira(
        &harness.mock_server,
        rows,
    )
    .await?;

    // THEN: Ticket updated
    assert_eq!(result.updated_count, 1);
    assert_eq!(result.created_count, 0);

    // AND: Changes reflected in Jira
    let updated_ticket = harness.mock_server.get_ticket(&ticket.key).await?;
    assert_eq!(updated_ticket.summary, "Updated from spreadsheet");
    assert_eq!(updated_ticket.status, "In Progress");

    Ok(())
}

#[tokio::test]
async fn test_sync_spreadsheet_to_jira_conflict_detection() -> Result<()> {
    // GIVEN: Ticket updated in both Jira and spreadsheet
    let harness = JiraIntegrationHarness::new()?;
    let ticket = harness.create_test_ticket("Original").await?;

    // Update in Jira
    harness.mock_server.update_ticket(&ticket.key, JiraTicketUpdate {
        summary: Some("Updated in Jira".to_string()),
        ..Default::default()
    }).await?;

    // Spreadsheet has different update
    let rows = vec![
        HashMap::from([
            ("Key".to_string(), ticket.key.clone()),
            ("Summary".to_string(), "Updated in spreadsheet".to_string()),
            ("_LastSyncTime".to_string(), "2024-01-01T00:00:00Z".to_string()),  // Old sync time
        ]),
    ];

    // WHEN: We sync to Jira
    let result = simulate_sync_spreadsheet_to_jira(
        &harness.mock_server,
        rows,
    )
    .await?;

    // THEN: Conflict detected
    assert_eq!(result.conflict_count, 1);
    assert!(result.conflicts.len() > 0);
    assert!(result.conflicts[0].contains(&ticket.key));

    Ok(())
}

// =============================================================================
// Tests: query_jira_tickets
// =============================================================================

#[tokio::test]
async fn test_query_tickets_by_jql() -> Result<()> {
    // GIVEN: Mix of tickets with different priorities
    let harness = JiraIntegrationHarness::new()?;
    harness.mock_server.create_ticket("High priority", "Desc", "High").await?;
    harness.mock_server.create_ticket("Medium priority", "Desc", "Medium").await?;
    harness.mock_server.create_ticket("Another high", "Desc", "High").await?;

    // WHEN: We query for high priority tickets
    let result = simulate_query_jira_tickets(
        &harness.mock_server,
        "priority=High",
    )
    .await?;

    // THEN: Only high priority tickets returned
    assert_eq!(result.tickets.len(), 2);
    assert!(result.tickets.iter().all(|t| t.priority == "High"));

    Ok(())
}

#[tokio::test]
async fn test_query_tickets_returns_empty_on_no_matches() -> Result<()> {
    // GIVEN: Tickets exist but don't match query
    let harness = JiraIntegrationHarness::new()?;
    harness.create_test_ticket("Test ticket").await?;

    // WHEN: We query for non-matching criteria
    let result = simulate_query_jira_tickets(
        &harness.mock_server,
        "status=Closed",
    )
    .await?;

    // THEN: No tickets returned
    assert_eq!(result.tickets.len(), 0);

    Ok(())
}

// =============================================================================
// Tests: import_jira_to_spreadsheet
// =============================================================================

#[tokio::test]
async fn test_import_all_fields() -> Result<()> {
    // GIVEN: Jira tickets with complete data
    let harness = JiraIntegrationHarness::new()?;
    let mut ticket = harness.mock_server.create_ticket("Complete ticket", "Full description", "High").await?;
    ticket.assignee = Some("user@example.com".to_string());
    harness.mock_server.update_ticket(&ticket.key, JiraTicketUpdate {
        assignee: Some("user@example.com".to_string()),
        ..Default::default()
    }).await?;

    // WHEN: We import to spreadsheet
    let result = simulate_import_jira_to_spreadsheet(
        &harness.mock_server,
        harness.spreadsheet_path().as_path(),
        &["Key", "Summary", "Description", "Status", "Priority", "Assignee"],
    )
    .await?;

    // THEN: All fields imported
    assert_eq!(result.imported_count, 1);
    assert!(result.fields_imported.contains(&"Assignee".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_import_selective_fields() -> Result<()> {
    // GIVEN: Jira tickets
    let harness = JiraIntegrationHarness::new()?;
    harness.create_test_ticket("Ticket 1").await?;
    harness.create_test_ticket("Ticket 2").await?;

    // WHEN: We import only specific fields
    let result = simulate_import_jira_to_spreadsheet(
        &harness.mock_server,
        harness.spreadsheet_path().as_path(),
        &["Key", "Summary", "Status"],
    )
    .await?;

    // THEN: Only selected fields imported
    assert_eq!(result.fields_imported.len(), 3);
    assert!(result.fields_imported.contains(&"Key".to_string()));
    assert!(result.fields_imported.contains(&"Summary".to_string()));
    assert!(result.fields_imported.contains(&"Status".to_string()));
    assert!(!result.fields_imported.contains(&"Description".to_string()));

    Ok(())
}

// =============================================================================
// Mock Implementation Helpers (Replace with real MCP tool calls)
// =============================================================================

#[derive(Debug)]
struct CreateTicketsResult {
    created_count: usize,
    skipped_count: usize,
    failed_count: usize,
    errors: Vec<String>,
}

#[derive(Debug)]
struct SyncResult {
    synced_count: usize,
    updated_count: usize,
    created_count: usize,
    conflict_count: usize,
    conflicts: Vec<String>,
    errors: Vec<String>,
}

#[derive(Debug)]
struct QueryResult {
    tickets: Vec<JiraTicket>,
}

#[derive(Debug)]
struct ImportResult {
    imported_count: usize,
    fields_imported: Vec<String>,
}

async fn simulate_create_tickets_from_spreadsheet(
    server: &MockJiraServer,
    rows: Vec<HashMap<String, String>>,
) -> Result<CreateTicketsResult> {
    let mut created_count = 0;
    let mut skipped_count = 0;
    let mut failed_count = 0;
    let mut errors = Vec::new();

    for row in rows {
        let summary = row.get("Summary").map(|s| s.as_str()).unwrap_or("");
        let description = row.get("Description").map(|s| s.as_str()).unwrap_or("");
        let priority = row.get("Priority").map(|s| s.as_str()).unwrap_or("Medium");

        // Validation
        if summary.is_empty() {
            errors.push("Summary cannot be empty".to_string());
            failed_count += 1;
            continue;
        }

        // Check if already exists
        if let Some(key) = row.get("Key") {
            if server.get_ticket(key).await.is_ok() {
                skipped_count += 1;
                continue;
            }
        }

        match server.create_ticket(summary, description, priority).await {
            Ok(_) => created_count += 1,
            Err(e) => {
                errors.push(e.to_string());
                failed_count += 1;
            }
        }
    }

    Ok(CreateTicketsResult {
        created_count,
        skipped_count,
        failed_count,
        errors,
    })
}

async fn simulate_sync_jira_to_spreadsheet(
    server: &MockJiraServer,
    _spreadsheet_path: &Path,
    jql: &str,
) -> Result<SyncResult> {
    let tickets = server.query_tickets(jql).await?;

    // Simulate spreadsheet write
    Ok(SyncResult {
        synced_count: tickets.len(),
        updated_count: 0,  // Would track updates in real implementation
        created_count: tickets.len(),
        conflict_count: 0,
        conflicts: Vec::new(),
        errors: Vec::new(),
    })
}

async fn simulate_sync_spreadsheet_to_jira(
    server: &MockJiraServer,
    rows: Vec<HashMap<String, String>>,
) -> Result<SyncResult> {
    let mut created_count = 0;
    let mut updated_count = 0;
    let mut conflict_count = 0;
    let mut conflicts = Vec::new();

    for row in rows {
        if let Some(key) = row.get("Key") {
            // Update existing
            if server.get_ticket(key).await.is_ok() {
                // Check for conflicts
                if let Some(last_sync) = row.get("_LastSyncTime") {
                    let ticket = server.get_ticket(key).await?;
                    if ticket.updated > *last_sync {
                        conflict_count += 1;
                        conflicts.push(format!("Conflict detected for {}", key));
                        continue;
                    }
                }

                server.update_ticket(key, JiraTicketUpdate {
                    summary: row.get("Summary").cloned(),
                    description: row.get("Description").cloned(),
                    status: row.get("Status").cloned(),
                    priority: row.get("Priority").cloned(),
                    assignee: row.get("Assignee").cloned(),
                }).await?;
                updated_count += 1;
            }
        } else {
            // Create new
            let summary = row.get("Summary").unwrap_or(&String::new());
            let description = row.get("Description").unwrap_or(&String::new());
            let priority = row.get("Priority").unwrap_or(&"Medium".to_string());

            server.create_ticket(summary, description, priority).await?;
            created_count += 1;
        }
    }

    Ok(SyncResult {
        synced_count: created_count + updated_count,
        created_count,
        updated_count,
        conflict_count,
        conflicts,
        errors: Vec::new(),
    })
}

async fn simulate_query_jira_tickets(
    server: &MockJiraServer,
    jql: &str,
) -> Result<QueryResult> {
    let tickets = server.query_tickets(jql).await?;
    Ok(QueryResult { tickets })
}

async fn simulate_import_jira_to_spreadsheet(
    server: &MockJiraServer,
    _spreadsheet_path: &Path,
    fields: &[&str],
) -> Result<ImportResult> {
    let tickets = server.query_tickets("").await?;

    Ok(ImportResult {
        imported_count: tickets.len(),
        fields_imported: fields.iter().map(|s| s.to_string()).collect(),
    })
}
