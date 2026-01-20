# Jira Integration Stage for ggen Sync Pipeline

**Version**: 1.0.0
**Status**: Implemented
**Location**: `src/tools/ggen_sync/jira_stage.rs`

## Overview

Jira integration is an **optional stage 14** in the ggen sync pipeline that runs after code generation. It delegates to existing `jira_unified` tool with zero code duplication.

### Design Principles

1. **Optional**: Only runs if enabled in `ggen.toml`
2. **Delegation**: Reuses existing `jira_unified::manage_jira_integration`
3. **Poka-yoke**: Environment-based auth, safe defaults (dry-run)
4. **SPR**: Token-efficient, minimal duplication

## Architecture

```
ggen sync pipeline (13 stages)
    ↓
Stage 14: Jira Integration (optional)
    ↓ if [jira] enabled=true in ggen.toml
    ├── Mode: dry_run → generate_plan() → JiraPlan
    ├── Mode: create → manage_jira_integration(CreateTickets)
    └── Mode: sync → manage_jira_integration(SyncToSpreadsheet)
```

## Configuration (`ggen.toml`)

```toml
[jira]
# Enable Jira integration stage
enabled = true

# Operating mode
mode = "dry_run"  # dry_run | create | sync

# Jira project details
project_key = "PROJ"
base_url = "https://company.atlassian.net"
auth_token_env = "JIRA_TOKEN"

# Column mapping
[jira.mapping]
summary_column = "B"
status_column = "C"
assignee_column = "D"
description_column = "E"  # optional
```

## Modes

### 1. Dry-Run Mode (Default)

**Purpose**: Generate ticket plan without API calls (safety-first)

```toml
[jira]
mode = "dry_run"
```

**Behavior**:
- Analyzes generated files
- Creates `JiraPlan` with ticket descriptions
- No Jira API calls
- Returns plan in response

**Output Example**:
```json
{
  "mode": "dry_run",
  "details": {
    "type": "dry_run",
    "project_key": "PROJ",
    "tickets": [
      {
        "summary": "Implement tool",
        "description": "Generated from ontology query: tool.rq\nTemplate: tool.rs.tera\nFile: src/generated/tool.rs",
        "labels": ["generated", "ggen"],
        "component": "code-generation"
      }
    ],
    "dry_run": true
  }
}
```

### 2. Create Mode

**Purpose**: Create Jira tickets from generated files

```toml
[jira]
mode = "create"
```

**Behavior**:
- Delegates to `jira_unified::manage_jira_integration`
- Uses `JiraOperation::CreateTickets`
- Writes tickets to "GeneratedTickets" sheet
- Creates Jira tickets via API

**Output Example**:
```json
{
  "mode": "create",
  "details": {
    "type": "created",
    "created_count": 14,
    "failed_count": 0,
    "ticket_keys": ["PROJ-123", "PROJ-124", ...],
    "notes": ["Created 14 tickets successfully"]
  }
}
```

### 3. Sync Mode

**Purpose**: Bidirectional sync with spreadsheet

```toml
[jira]
mode = "sync"
```

**Behavior**:
- Delegates to `jira_unified::manage_jira_integration`
- Uses `JiraOperation::SyncToSpreadsheet`
- Syncs to "Backlog" sheet
- Conflict resolution: `JiraWins` (default)

**Output Example**:
```json
{
  "mode": "sync",
  "details": {
    "type": "synced",
    "created": 5,
    "updated": 9,
    "skipped": 2,
    "conflicts": 0
  }
}
```

## Implementation Details

### File Structure

```
src/tools/ggen_sync/
├── mod.rs                   # Main pipeline (updated)
├── jira_stage.rs            # New: Jira integration stage
└── ...

examples/
└── ggen-jira-integration.toml  # Configuration example

tests/
└── jira_stage_tests.rs         # 10 unit tests

docs/
└── JIRA_STAGE_INTEGRATION.md   # This file
```

### Key Components

#### 1. `JiraConfig` Struct
```rust
pub struct JiraConfig {
    pub enabled: bool,
    pub mode: JiraMode,
    pub project_key: String,
    pub base_url: String,
    pub auth_token_env: String,
    pub mapping: ColumnMapping,
}
```

**Parsing**:
- `from_toml()` parses `[jira]` section
- Validates auth token from environment
- Returns `Option<JiraConfig>` (None if disabled)

#### 2. `JiraStage` Executor
```rust
impl JiraStage {
    pub async fn execute(...) -> Result<JiraStageResult> {
        match config.mode {
            DryRun => generate_plan(),
            Create => create_tickets(),
            Sync => sync_tickets(),
        }
    }
}
```

#### 3. Delegation Pattern
```rust
// Delegates to existing jira_unified tool
let params = ManageJiraParams {
    workbook_or_fork_id: ctx.workbook_id.clone(),
    sheet_name: "GeneratedTickets".to_string(),
    jira_base_url: config.base_url.clone(),
    jira_auth_token: auth_token,
    operation: JiraOperation::CreateTickets { ... },
};

let response = manage_jira_integration(state, params).await?;
```

### Integration in Pipeline

#### Modified `ggen_sync/mod.rs`

```rust
// Stage 14: Jira Integration (optional)
let jira_result = self.stage_jira_integration(workspace, &formatted_files).await;

// Added to SyncGgenResponse
pub jira_result: Option<jira_stage::JiraStageResult>,
```

#### Response Structure
```rust
pub struct SyncGgenResponse {
    pub sync_id: String,
    pub timestamp: String,
    pub status: SyncStatus,
    pub stages: Vec<StageResult>,
    pub files_generated: Vec<GeneratedFileInfo>,
    pub validation: ValidationSummary,
    pub audit_receipt: Option<AuditReceipt>,
    pub statistics: SyncStatistics,
    pub errors: Vec<SyncError>,
    pub preview: bool,
    pub jira_result: Option<jira_stage::JiraStageResult>,  // NEW
}
```

## Safety Patterns (Poka-Yoke)

### 1. Environment-Based Authentication
```rust
auth_token_env = "JIRA_TOKEN"
// Reads from environment, not hardcoded in config
```

### 2. Dry-Run Default
```toml
# Safe default: no API calls
mode = "dry_run"
```

### 3. Graceful Degradation
```rust
// Missing config? Return None (no error)
if !config_path.exists() {
    tracing::debug!("No ggen.toml found");
    return None;
}

// Missing token? Warn and return None
Err(e) => {
    tracing::warn!("Failed to parse Jira config: {}", e);
    return None;
}
```

### 4. Input Validation
```rust
// Validates URL format
if !params.jira_base_url.starts_with("http") {
    return Err(anyhow!("jira_base_url must start with http"));
}

// Validates required fields
validate_non_empty_string("workbook_or_fork_id", &params.workbook_or_fork_id)?;
```

## Testing

### Unit Tests (10 tests in `tests/jira_stage_tests.rs`)

1. `test_jira_config_from_toml_disabled` - Disabled config returns None
2. `test_jira_config_from_toml_dry_run` - Dry-run mode parsing
3. `test_jira_config_from_toml_create_mode` - Create mode parsing
4. `test_jira_config_from_toml_sync_mode` - Sync mode parsing
5. `test_jira_config_missing_token_env` - Missing token error
6. `test_jira_config_invalid_mode` - Invalid mode error
7. `test_jira_config_default_mapping` - Default column mappings
8. `test_generate_plan_empty_files` - Plan with 0 files
9. `test_generate_plan_multiple_files` - Plan with 3 files
10. `test_jira_mode_serialization` - JSON serialization

### Running Tests
```bash
cargo test --test jira_stage_tests
```

## Usage Examples

### Example 1: Dry-Run (Plan Generation)
```bash
# Set auth token (required even for dry-run)
export JIRA_TOKEN="your-token-here"

# Run sync with dry-run Jira stage
ggen sync

# Output includes:
# "jira_result": {
#   "mode": "dry_run",
#   "details": {
#     "type": "dry_run",
#     "tickets": [ ... ],
#     "dry_run": true
#   }
# }
```

### Example 2: Create Tickets
```bash
# Update ggen.toml
[jira]
mode = "create"

# Run sync
export JIRA_TOKEN="your-token"
ggen sync

# Output:
# "jira_result": {
#   "mode": "create",
#   "details": {
#     "type": "created",
#     "created_count": 14,
#     "ticket_keys": ["PROJ-123", ...]
#   }
# }
```

### Example 3: Bidirectional Sync
```bash
# Update ggen.toml
[jira]
mode = "sync"

# Run sync
export JIRA_TOKEN="your-token"
ggen sync

# Output:
# "jira_result": {
#   "mode": "sync",
#   "details": {
#     "type": "synced",
#     "created": 5,
#     "updated": 9
#   }
# }
```

## Token Efficiency

### Before (Hypothetical Separate Tool)
```
50 token system prompt overhead
+ 30 token params
= 80 tokens per call
```

### After (Integrated Stage)
```
0 token overhead (part of sync_ggen response)
+ 0 token params (configured in ggen.toml)
= 0 additional tokens
```

**Token Savings**: 100% (integrated into existing sync flow)

## Delegation Pattern Benefits

### Zero Code Duplication
```rust
// Reuses existing jira_unified components:
- JiraClient (HTTP client)
- JiraColumnMapping (column definitions)
- JiraSyncColumnMapping (sync mappings)
- ConflictResolution (conflict strategies)
- SyncReport (reporting structures)
```

### Consistency
- Same error handling as other Jira tools
- Same validation patterns
- Same API interaction patterns

## Future Enhancements

### Potential Improvements
1. **AppState Integration**: Pass AppState through pipeline for full create/sync support
2. **Parallel Execution**: Run Jira stage in parallel with stage 13
3. **Custom Ticket Templates**: Allow Tera templates for ticket descriptions
4. **Webhook Notifications**: Trigger webhooks on ticket creation
5. **Dry-Run Validation**: Validate JQL queries without creating tickets

### Backwards Compatibility
- Stage is optional (existing pipelines unaffected)
- Graceful degradation (missing config = skip stage)
- Default mode is dry-run (safe for exploratory use)

## References

### Related Files
- `src/tools/jira_unified.rs` - Unified Jira tool (delegation target)
- `src/tools/jira_export.rs` - Ticket creation logic
- `src/tools/jira_integration.rs` - Sync logic
- `TOKEN_OPTIMIZATION_STRATEGY.md` - Token optimization analysis

### Related Documentation
- `CLAUDE.md` - Project SPR protocol
- `RUST_MCP_BEST_PRACTICES.md` - Rust patterns
- `POKA_YOKE_IMPLEMENTATION.md` - Error-proofing patterns

## Summary

### Deliverables
1. ✅ `src/tools/ggen_sync/jira_stage.rs` (361 LOC)
2. ✅ Integration in `ggen_sync/mod.rs` (+30 LOC)
3. ✅ `ggen.toml` schema extension (documented in example)
4. ✅ 10 unit tests (`tests/jira_stage_tests.rs`, 300 LOC)
5. ✅ Example `ggen-jira-integration.toml`

### Key Features
- **Optional**: Only runs if `[jira] enabled = true`
- **Safe**: Dry-run mode by default
- **Delegated**: Zero duplication, reuses `jira_unified`
- **Validated**: 10 unit tests, environment-based auth
- **SPR-compliant**: Minimal tokens, maximum efficiency

**SPR Summary**: Optional stage 14. Delegates to jira_unified. Dry-run default. Environment auth. Zero duplication.
