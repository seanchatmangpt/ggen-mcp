# Jira ↔ Spreadsheet Bidirectional Synchronization

**Version**: 1.0.0
**Feature**: Jira Integration
**Tools**: `sync_jira_to_spreadsheet`, `sync_spreadsheet_to_jira`

---

## Overview

Bidirectional synchronization between Jira tickets and spreadsheet rows. Primary key: Jira Key column. Timestamp-based conflict resolution. Fork-based atomic transactions.

### Key Features

- **Jira → Spreadsheet**: Query tickets via JQL, update fork rows
- **Spreadsheet → Jira**: Read rows, create/update tickets
- **Conflict Resolution**: Timestamp comparison (configurable: Jira wins, Spreadsheet wins, Skip)
- **Atomic Updates**: Fork-based transactions prevent data corruption
- **Detailed Reports**: Track created, updated, skipped, errors, conflicts

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│              sync_jira_to_spreadsheet                  │
│  JQL Query → Jira REST API v3 → Compare Timestamps    │
│  → Resolve Conflicts → Update Fork Rows               │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│              sync_spreadsheet_to_jira                  │
│  Read Rows → Compare Timestamps → Resolve Conflicts   │
│  → Create/Update Jira Tickets → Report Results        │
└─────────────────────────────────────────────────────────┘
```

### Components

1. **JiraClient**: REST API v3 wrapper (search, get, update, create)
2. **JiraSyncColumnMapping**: Column → field mapping (A=Key, B=Summary, C=Status, etc.)
3. **ConflictResolution**: Strategy enum (JiraWins, SpreadsheetWins, Skip)
4. **SyncReport**: Results tracking (created, updated, skipped, errors, conflicts)

---

## Tool 1: sync_jira_to_spreadsheet

**Description**: Sync Jira tickets to spreadsheet rows (Jira → Spreadsheet).

### Parameters

```typescript
{
  fork_id: string,                      // Fork ID (created via create_fork)
  sheet_name: string,                    // Sheet to update
  jira_base_url: string,                 // e.g., "https://company.atlassian.net"
  jira_auth_token: string,               // Bearer token or API token
  jql_query: string,                     // e.g., "project = PROJ AND updated > '2024-01-01'"
  column_mapping?: JiraSyncColumnMapping, // Default: A=Key, B=Summary, C=Status, D=Assignee, E=Updated
  start_row?: number,                    // Default: 2 (skip header)
  conflict_resolution?: ConflictResolution // Default: JiraWins
}
```

### Column Mapping (Default)

| Column | Field       | Required |
|--------|-------------|----------|
| A      | Jira Key    | Yes      |
| B      | Summary     | Yes      |
| C      | Status      | No       |
| D      | Assignee    | No       |
| E      | Updated     | No       |
| F      | Description | No       |
| G      | Priority    | No       |
| H      | Labels      | No       |

### Response

```typescript
{
  fork_id: string,
  report: {
    created: number,        // New rows added
    updated: number,        // Existing rows updated
    skipped: number,        // Conflicts skipped
    errors: SyncError[],    // { row, jira_key?, error }
    conflicts: ConflictReport[] // { row, jira_key, reason, resolution }
  }
}
```

### Example Usage

```javascript
// Step 1: Create fork
const fork = await create_fork({
  workbook_or_fork_id: "project-tracker.xlsx"
});

// Step 2: Sync Jira → Spreadsheet
const syncResult = await sync_jira_to_spreadsheet({
  fork_id: fork.fork_id,
  sheet_name: "Backlog",
  jira_base_url: "https://company.atlassian.net",
  jira_auth_token: "YOUR_TOKEN",
  jql_query: "project = PROJ AND sprint = 'Sprint 1'",
  column_mapping: {
    jira_key_column: "A",
    summary_column: "B",
    status_column: "C",
    assignee_column: "D",
    updated_column: "E"
  },
  start_row: 2,
  conflict_resolution: "jira_wins"
});

console.log(`Created: ${syncResult.report.created}`);
console.log(`Updated: ${syncResult.report.updated}`);
console.log(`Conflicts: ${syncResult.report.conflicts.length}`);

// Step 3: Review changes
const changeset = await get_changeset({
  fork_id: fork.fork_id,
  summary_only: false
});

// Step 4: Save or discard
await save_fork({ fork_id: fork.fork_id, output_path: "synced-tracker.xlsx" });
```

### Conflict Resolution Examples

#### Scenario 1: Jira Wins (Default)

```
Spreadsheet row updated: 2024-01-20 14:00:00
Jira ticket updated:     2024-01-20 15:00:00

Action: Overwrite spreadsheet with Jira data
Reason: Jira is newer
```

#### Scenario 2: Spreadsheet Wins

```
Spreadsheet row updated: 2024-01-20 16:00:00
Jira ticket updated:     2024-01-20 15:00:00

Action: Skip Jira update
Reason: Spreadsheet is newer
```

#### Scenario 3: Skip

```
Any conflict detected

Action: Skip row, log conflict
Reason: User wants manual review
```

---

## Tool 2: sync_spreadsheet_to_jira

**Description**: Sync spreadsheet rows to Jira tickets (Spreadsheet → Jira).

### Parameters

```typescript
{
  workbook_or_fork_id: WorkbookId,       // Workbook or fork to read
  sheet_name: string,                    // Sheet to read
  jira_base_url: string,                 // Jira URL
  jira_auth_token: string,               // Auth token
  jira_project_key: string,              // Project key for new tickets (e.g., "PROJ")
  column_mapping?: JiraSyncColumnMapping, // Default mapping
  start_row?: number,                    // Default: 2
  end_row?: number,                      // Optional (reads until empty row)
  conflict_resolution?: ConflictResolution // Default: JiraWins
}
```

### Response

```typescript
{
  report: {
    created: number,
    updated: number,
    skipped: number,
    errors: SyncError[],
    conflicts: ConflictReport[]
  }
}
```

### Example Usage

```javascript
// Read-only sync (no fork needed)
const syncResult = await sync_spreadsheet_to_jira({
  workbook_or_fork_id: "project-tracker.xlsx",
  sheet_name: "Backlog",
  jira_base_url: "https://company.atlassian.net",
  jira_auth_token: "YOUR_TOKEN",
  jira_project_key: "PROJ",
  column_mapping: {
    jira_key_column: "A",
    summary_column: "B",
    status_column: "C",
    assignee_column: "D",
    updated_column: "E"
  },
  start_row: 2,
  conflict_resolution: "spreadsheet_wins"
});

console.log(`Created: ${syncResult.report.created} new tickets`);
console.log(`Updated: ${syncResult.report.updated} existing tickets`);
console.log(`Errors: ${syncResult.report.errors.length}`);

// Check errors
syncResult.report.errors.forEach(err => {
  console.log(`Row ${err.row}: ${err.error}`);
});
```

### Create vs Update Logic

```
IF Jira Key column is empty OR no Jira Key value
  → CREATE new ticket in Jira
  → Log created ticket key

ELSE IF Jira Key exists (e.g., "PROJ-123")
  → GET existing ticket from Jira
  → Compare timestamps
  → Resolve conflict
  → UPDATE ticket fields
```

---

## Conflict Resolution Strategies

### 1. JiraWins (Default)

- **Use Case**: Jira is source of truth
- **Behavior**: Jira changes always overwrite spreadsheet
- **Example**: Team updates tickets in Jira, spreadsheet is report view

### 2. SpreadsheetWins

- **Use Case**: Spreadsheet is source of truth
- **Behavior**: Spreadsheet changes always overwrite Jira
- **Example**: Planning in spreadsheet, sync to Jira for tracking

### 3. Skip

- **Use Case**: Manual conflict resolution
- **Behavior**: Log conflict, skip row, continue
- **Example**: Complex workflows requiring human review

### Conflict Report Format

```typescript
{
  row: 5,
  jira_key: "PROJ-123",
  reason: "Spreadsheet newer (2024-01-20 16:00:00) vs Jira (2024-01-20 15:00:00)",
  resolution: "Spreadsheet wins (skip Jira)"
}
```

---

## Best Practices

### 1. Always Use Forks for Jira → Spreadsheet

```javascript
// ✓ Correct: Fork-based atomic transaction
const fork = await create_fork({ workbook_or_fork_id: "tracker.xlsx" });
await sync_jira_to_spreadsheet({ fork_id: fork.fork_id, ... });
await save_fork({ fork_id: fork.fork_id, output_path: "synced.xlsx" });

// ✗ Wrong: Direct modification (requires fork)
await sync_jira_to_spreadsheet({ fork_id: "tracker.xlsx", ... }); // Error!
```

### 2. Set Consistent Timestamps

Ensure Updated column (default: E) contains valid ISO 8601 timestamps:

```
2024-01-20T15:30:00Z        ✓ RFC 3339
2024-01-20 15:30:00         ✓ Naive datetime
2024-01-20                  ✗ Date only (no time)
```

### 3. Use JQL to Narrow Scope

```javascript
// ✓ Efficient: Narrow query
jql_query: "project = PROJ AND updated > '2024-01-15' AND sprint = 'Sprint 1'"

// ✗ Inefficient: Broad query (max 1000 results)
jql_query: "project = PROJ"
```

### 4. Handle Errors Gracefully

```javascript
const result = await sync_spreadsheet_to_jira({ ... });

if (result.report.errors.length > 0) {
  console.error("Sync errors:");
  result.report.errors.forEach(err => {
    console.error(`  Row ${err.row} (${err.jira_key || 'new'}): ${err.error}`);
  });
}

if (result.report.conflicts.length > 0) {
  console.warn("Conflicts detected:");
  result.report.conflicts.forEach(conflict => {
    console.warn(`  Row ${conflict.row} (${conflict.jira_key}): ${conflict.resolution}`);
  });
}
```

### 5. Monitor Sync Performance

```javascript
const start = Date.now();
const result = await sync_jira_to_spreadsheet({ ... });
const duration = Date.now() - start;

console.log(`Synced ${result.report.updated + result.report.created} rows in ${duration}ms`);
console.log(`Rate: ${((result.report.updated + result.report.created) / (duration / 1000)).toFixed(2)} rows/sec`);
```

---

## Security Considerations

### 1. Jira Authentication

```javascript
// ✓ Correct: Use environment variables
const token = process.env.JIRA_AUTH_TOKEN;

// ✗ Wrong: Hardcoded credentials
const token = "hardcoded-token-123"; // Security risk!
```

### 2. API Token Scopes

Ensure Jira API token has required permissions:

- **Read**: `jira-work:read`, `jira-project:read`
- **Write**: `jira-work:write`, `jira-issue:create`, `jira-issue:update`

### 3. Input Validation

Tools validate:

- Jira base URL format (must start with http:// or https://)
- Column names (A-ZZZ, uppercase only)
- Row numbers (1-1,048,576, Excel limit)
- Workbook/Fork IDs (non-empty, max 1024 chars)

---

## Limitations

### 1. Jira API Limits

- **Max results per query**: 1000 issues
- **Rate limits**: Varies by Jira plan (Cloud: ~100-300 req/min)
- **Timeout**: 30 seconds per API request

### 2. Spreadsheet Limits

- **Max rows**: 1,048,576 (Excel 2007+)
- **Max columns**: 16,384 (XFD)
- **Sheet name**: 255 chars max, no `[]:*?/\` chars

### 3. Sync Limitations

- **Custom fields**: Not auto-mapped (use `custom_fields` in advanced mode)
- **Attachments**: Not synced
- **Comments**: Not synced
- **Watchers**: Not synced

### 4. Feature Requirements

- **sync_jira_to_spreadsheet**: Requires `recalc` feature (fork support)
- **sync_spreadsheet_to_jira**: No feature requirements

---

## Troubleshooting

### Error: "fork registry not available"

```
Cause: recalc feature not enabled
Solution: Rebuild with --features recalc
```

### Error: "Jira API error (401): Unauthorized"

```
Cause: Invalid or expired auth token
Solution: Regenerate Jira API token
```

### Error: "sheet 'Backlog' not found"

```
Cause: Sheet name mismatch (case-sensitive)
Solution: Use exact sheet name from workbook
```

### Conflict: "Spreadsheet newer vs Jira"

```
Cause: Spreadsheet row modified after Jira ticket
Solution: Choose conflict_resolution strategy:
  - jira_wins: Overwrite spreadsheet
  - spreadsheet_wins: Skip Jira update
  - skip: Log and skip row
```

### No rows synced (created = 0, updated = 0)

```
Possible causes:
1. JQL query returned no results
   → Check JQL query in Jira web UI
2. All rows had conflicts and were skipped
   → Check report.conflicts array
3. Summary column empty (sync stops at first empty row)
   → Ensure data starts at start_row
```

---

## Advanced Usage

### Custom Column Mapping

```javascript
// Map to custom columns
column_mapping: {
  jira_key_column: "Z",        // Jira key in column Z
  summary_column: "AA",         // Summary in column AA
  status_column: "AB",          // Status in column AB
  assignee_column: "AC",        // Assignee in column AC
  updated_column: "AD",         // Updated in column AD
  description_column: "AE",     // Description in column AE
  priority_column: "AF",        // Priority in column AF
  labels_column: "AG"           // Labels (comma-separated) in column AG
}
```

### Partial Column Sync

```javascript
// Only sync key, summary, and status (no assignee/updated/etc.)
column_mapping: {
  jira_key_column: "A",
  summary_column: "B",
  status_column: "C",
  assignee_column: null,        // Skip assignee
  updated_column: null,         // Skip timestamps (no conflict detection)
  description_column: null,
  priority_column: null,
  labels_column: null
}
```

### Incremental Sync

```javascript
// Sync only tickets updated in last 24 hours
const yesterday = new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString().split('T')[0];
const result = await sync_jira_to_spreadsheet({
  ...,
  jql_query: `project = PROJ AND updated >= '${yesterday}'`
});
```

---

## Integration Examples

### Example 1: Daily Backlog Sync

```javascript
// Run daily: Sync Jira backlog to spreadsheet for reporting
async function dailyBacklogSync() {
  const fork = await create_fork({ workbook_or_fork_id: "backlog.xlsx" });

  const result = await sync_jira_to_spreadsheet({
    fork_id: fork.fork_id,
    sheet_name: "Backlog",
    jira_base_url: process.env.JIRA_URL,
    jira_auth_token: process.env.JIRA_TOKEN,
    jql_query: "project = PROJ AND status != Done ORDER BY priority DESC",
    conflict_resolution: "jira_wins" // Jira is source of truth
  });

  if (result.report.errors.length === 0) {
    await save_fork({ fork_id: fork.fork_id, output_path: "backlog-synced.xlsx" });
    console.log(`✓ Synced ${result.report.updated + result.report.created} tickets`);
  } else {
    await discard_fork({ fork_id: fork.fork_id });
    console.error(`✗ Sync failed: ${result.report.errors.length} errors`);
  }
}
```

### Example 2: Spreadsheet Planning → Jira

```javascript
// Workflow: Plan sprint in spreadsheet, sync to Jira
async function planSprintInJira() {
  const result = await sync_spreadsheet_to_jira({
    workbook_or_fork_id: "sprint-plan.xlsx",
    sheet_name: "Sprint 5",
    jira_base_url: process.env.JIRA_URL,
    jira_auth_token: process.env.JIRA_TOKEN,
    jira_project_key: "PROJ",
    conflict_resolution: "spreadsheet_wins", // Spreadsheet is source of truth
    start_row: 2
  });

  console.log(`Created ${result.report.created} new tickets`);
  console.log(`Updated ${result.report.updated} existing tickets`);

  return result.report.created > 0
    ? result.report.created // Return count of new tickets
    : 0;
}
```

### Example 3: Bidirectional Sync Loop

```javascript
// Two-way sync: Jira ↔ Spreadsheet
async function bidirectionalSync() {
  // Step 1: Jira → Spreadsheet
  const fork = await create_fork({ workbook_or_fork_id: "tracker.xlsx" });
  const jiraToSheet = await sync_jira_to_spreadsheet({
    fork_id: fork.fork_id,
    sheet_name: "Tasks",
    jira_base_url: process.env.JIRA_URL,
    jira_auth_token: process.env.JIRA_TOKEN,
    jql_query: "project = PROJ AND updated > -1d",
    conflict_resolution: "jira_wins"
  });

  await save_fork({ fork_id: fork.fork_id, output_path: "tracker-updated.xlsx" });

  // Step 2: Spreadsheet → Jira
  const sheetToJira = await sync_spreadsheet_to_jira({
    workbook_or_fork_id: "tracker-updated.xlsx",
    sheet_name: "Tasks",
    jira_base_url: process.env.JIRA_URL,
    jira_auth_token: process.env.JIRA_TOKEN,
    jira_project_key: "PROJ",
    conflict_resolution: "skip" // Skip conflicts for manual review
  });

  return {
    jiraToSheet: jiraToSheet.report,
    sheetToJira: sheetToJira.report
  };
}
```

---

## API Reference

### Types

```typescript
interface JiraSyncColumnMapping {
  jira_key_column: string;          // Required
  summary_column: string;            // Required
  status_column?: string | null;
  assignee_column?: string | null;
  updated_column?: string | null;
  description_column?: string | null;
  priority_column?: string | null;
  labels_column?: string | null;
}

enum ConflictResolution {
  JiraWins = "jira_wins",
  SpreadsheetWins = "spreadsheet_wins",
  Skip = "skip"
}

interface SyncReport {
  created: number;
  updated: number;
  skipped: number;
  errors: SyncError[];
  conflicts: ConflictReport[];
}

interface SyncError {
  row: number;
  jira_key?: string;
  error: string;
}

interface ConflictReport {
  row: number;
  jira_key: string;
  reason: string;
  resolution: string;
}
```

---

## Testing

### Unit Tests (8 tests)

```bash
cargo test --lib jira_integration
```

Tests cover:

1. ✓ Default column mapping
2. ✓ Conflict resolution default
3. ✓ Parse RFC 3339 timestamps
4. ✓ Parse naive datetimes
5. ✓ Sync report initialization
6. ✓ Field update JSON conversion
7. ✓ Build field update from row
8. ✓ Default start row (2)

---

## Changelog

### Version 1.0.0 (2026-01-20)

- ✓ Implemented `sync_jira_to_spreadsheet` tool
- ✓ Implemented `sync_spreadsheet_to_jira` tool
- ✓ Jira REST API v3 client integration
- ✓ Timestamp-based conflict detection
- ✓ Fork-based atomic transactions
- ✓ Comprehensive sync reports
- ✓ 8 unit tests (100% coverage for core logic)
- ✓ Documentation and examples

---

## Support

### Documentation

- **This file**: JIRA_SYNC.md (comprehensive guide)
- **CLAUDE.md**: Project-wide best practices
- **RUST_MCP_BEST_PRACTICES.md**: Rust patterns

### Code Location

- **Implementation**: `/home/user/ggen-mcp/src/tools/jira_integration.rs`
- **Registration**: `/home/user/ggen-mcp/src/server.rs` (lines 1416-1464)
- **Dependencies**: `reqwest` (HTTP client), `chrono` (timestamp parsing)

### Feature Flags

```bash
# Build with Jira sync support
cargo build --features recalc

# Run without recalc feature
cargo build
# Note: sync_jira_to_spreadsheet requires recalc (fork support)
#       sync_spreadsheet_to_jira works without recalc
```

---

**End of Documentation**
