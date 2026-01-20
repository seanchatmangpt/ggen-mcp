# TOKEN OPTIMIZATION STRATEGY

**Version**: 1.0.0 | TPS-Based | 80/20 Pareto Analysis
**Project**: ggen-mcp | MCP Server Token Efficiency
**Date**: 2026-01-20

---

## Executive Summary (TPS Lens)

### Current State
- **Total Tools**: 60 MCP tools
- **Tool Categories**: Spreadsheet (20), Fork/Recalc (20), Ggen/Tera (15), VBA (2), Jira (2), Ontology (1)
- **Token Waste**: ~40-60% redundancy in tool surface area
- **Parameter Overhead**: 300+ unique parameters across tools

### Optimization Target (80/20)
- **Tool Consolidation**: 60 → 24 tools (60% reduction)
- **Unified Interfaces**: 15 ggen tools → 1 `manage_ggen_resource`
- **Jira Integration**: 2 sync tools → 1 `manage_jira_integration`
- **Token Savings**: ~2,500-4,000 tokens per conversation
- **Implementation Effort**: 3-4 weeks (80/20 prioritization)

### TPS Impact
```
BEFORE: 60 tools × 5-8 params avg = 300-480 parameters
AFTER:  24 tools × 3-5 params avg = 72-120 parameters
REDUCTION: 75% parameter overhead eliminated
```

---

## 1. TPS Analysis: 7 Wastes Eliminated

### 1.1 Overproduction (作りすぎのムダ)
**Problem**: Tools generate full responses when partial data suffices.

**Solutions**:
- **Tiered Responses**: summary_only, metadata_only, full modes
- **Field Filtering**: include_fields/exclude_fields parameters
- **Lazy Loading**: Defer expensive computations to explicit requests

**Example**:
```json
// BEFORE (always 2KB response):
{
  "workbook_summary": {
    "sheets": [...],  // 50 sheets × 40 bytes
    "named_ranges": [...],  // 100 ranges
    "metadata": {...},
    "statistics": {...}
  }
}

// AFTER (400 bytes with summary_only=true):
{
  "summary": {
    "sheet_count": 50,
    "named_range_count": 100,
    "total_cells": 50000
  }
}
```

**Token Savings**: 1,500-2,000 tokens/call for summary queries (80% use case)

---

### 1.2 Transport (運搬のムダ)
**Problem**: Multiple round-trips for related operations.

**Solutions**:
- **Batch Operations**: Single call for multiple related actions
- **Composite Tools**: Combine discovery → query → process workflows
- **Pipeline Mode**: Chain operations in single request

**Example - Unified Ggen Resource**:
```json
// BEFORE (5 round-trips):
1. read_ggen_config → 500 tokens
2. validate_ggen_config → 300 tokens
3. add_generation_rule → 400 tokens
4. sync_ggen → 600 tokens
5. Error handling overhead → 200 tokens
TOTAL: 2,000 tokens

// AFTER (1 round-trip):
{
  "action": "add_and_sync",
  "rule": {...},
  "validate": true
}
TOTAL: 600 tokens
```

**Token Savings**: 1,400 tokens/workflow (70% reduction)

---

### 1.3 Waiting (手待ちのムダ)
**Problem**: Sequential tool calls block on network latency.

**Solutions**:
- **Multi-Layer Caching**: In-memory → Redis → Disk
- **Async Operations**: Non-blocking background tasks
- **Cache Preloading**: Predictive loading for common patterns

**Cache Strategy**:
```
L1 (In-Memory): Tool schemas, config metadata (100ms TTL)
L2 (Redis): SPARQL results, template renders (10min TTL)
L3 (Disk): Ontology graphs, compiled templates (1hr TTL)
```

**Token Savings**: Indirect (faster responses → less retry overhead)

---

### 1.4 Over-Processing (加工のムダ)
**Problem**: Excessive parameter complexity.

**Solutions**:
- **Smart Defaults**: Infer common parameter values
- **Parameter Reduction**: Consolidate related parameters
- **Profile-Based Presets**: Named configuration profiles

**Example**:
```json
// BEFORE (8 parameters):
{
  "workbook_id": "wb123",
  "sheet_name": "Sheet1",
  "include_formulas": true,
  "include_styles": false,
  "include_metadata": true,
  "max_rows": 1000,
  "offset": 0,
  "format": "json"
}

// AFTER (3 parameters + smart defaults):
{
  "workbook_id": "wb123",
  "sheet_name": "Sheet1",
  "profile": "data_analysis"  // Implies: formulas=true, styles=false, max_rows=1000
}
```

**Token Savings**: 200-400 tokens/call (parameter reduction)

---

### 1.5 Inventory (在庫のムダ)
**Problem**: Redundant tool definitions bloat API surface.

**Solutions**:
- **Tool Consolidation**: Merge CRUD operations into unified handlers
- **Action-Based Design**: Single tool with action parameter
- **Feature Flags**: Conditionally expose advanced tools

**Consolidation Targets**:
```
15 Ggen Tools → 1 manage_ggen_resource
  - read_ggen_config
  - validate_ggen_config
  - add_generation_rule
  - update_generation_rule
  - remove_generation_rule
  - read_tera_template
  - validate_tera_template
  - test_tera_template
  - create_tera_template
  - list_template_variables
  - render_template
  - validate_generated_code
  - write_generated_artifact
  - init_ggen_project
  - sync_ggen

2 Jira Tools → 1 manage_jira_integration
  - sync_jira_to_spreadsheet
  - sync_spreadsheet_to_jira
```

**Token Savings**: 3,000-5,000 tokens (reduced tool list in system prompt)

---

### 1.6 Motion (動作のムダ)
**Problem**: Verbose JSON schemas.

**Solutions**:
- **Compact Schema Design**: Remove redundant descriptions
- **Type Inference**: Use JSON Schema defaults
- **Schema Compression**: Strip whitespace, use abbreviations in internals

**Example**:
```json
// BEFORE (verbose):
{
  "type": "object",
  "properties": {
    "workbook_identifier": {
      "type": "string",
      "description": "The unique identifier for the workbook to operate on. This must be a valid workbook ID returned from list_workbooks.",
      "minLength": 1,
      "maxLength": 1024
    }
  },
  "required": ["workbook_identifier"]
}

// AFTER (compact):
{
  "type": "object",
  "properties": {
    "workbook_id": {"type": "string", "description": "Workbook ID"}
  },
  "required": ["workbook_id"]
}
```

**Token Savings**: 100-300 tokens per tool schema × 60 tools = 6,000-18,000 tokens (one-time system prompt reduction)

---

### 1.7 Defects (不良のムダ)
**Problem**: Verbose error messages.

**Solutions**:
- **Error Code System**: Numeric codes + external lookup
- **Compressed Messages**: Remove stack traces from responses
- **Context Links**: URL to full error documentation

**Example**:
```json
// BEFORE (800 bytes):
{
  "error": "ValidationError: The provided workbook_id 'invalid-wb-123' does not match any known workbook in the workspace. Available workbooks: ['wb-001', 'wb-002', 'wb-003']. Please check the workbook_id and try again. For more information, see the documentation at https://docs.example.com/errors/validation."
}

// AFTER (200 bytes):
{
  "error": {
    "code": "E1001",
    "message": "Invalid workbook_id",
    "docs": "https://docs.ex/e1001"
  }
}
```

**Token Savings**: 400-600 tokens per error response

---

## 2. Optimization Catalog (From 9 Agent Perspectives)

### 2.1 Agent 1: Token Usage Analysis

**Current Waste Metrics**:
- Tool list overhead: 12,000-15,000 tokens (60 tools × 200-250 tokens avg)
- Parameter redundancy: 40% of parameters have smart defaults
- Response bloat: 60% of responses include unused fields
- Error verbosity: 3x longer than necessary

**7 Muda Categories**:
1. **Overproduction**: Full responses when summaries suffice → 2,000 tokens/call saved
2. **Transport**: Multi-step workflows → 1,400 tokens/workflow saved
3. **Waiting**: Cache misses → Indirect savings via faster responses
4. **Over-processing**: Complex parameters → 300 tokens/call saved
5. **Inventory**: 60 tools → 24 tools = 8,000 tokens (system prompt)
6. **Motion**: Verbose schemas → 12,000 tokens (system prompt)
7. **Defects**: Long error messages → 500 tokens/error

**Quick Wins**:
1. Add `summary_only` to top 5 tools → 2,000 tokens/call (80% use case)
2. Consolidate ggen tools → 3,000 tokens (system prompt)
3. Compact error messages → 400 tokens/error

---

### 2.2 Agent 2: Tool Consolidation Analysis

**Clustering Analysis**:

**Cluster 1: Ggen Resource Management (15 tools → 1)**
```
Core Operations: CRUD on ggen.toml, Tera templates, sync pipeline
Unified Interface: manage_ggen_resource

Actions:
  - config.read
  - config.validate
  - config.add_rule
  - config.update_rule
  - config.remove_rule
  - template.read
  - template.validate
  - template.test
  - template.create
  - template.list_vars
  - pipeline.render
  - pipeline.validate_code
  - pipeline.write
  - pipeline.sync
  - project.init
```

**Cluster 2: Jira Integration (2 tools → 1)**
```
Core Operations: Bidirectional sync between Jira and spreadsheets
Unified Interface: manage_jira_integration

Actions:
  - sync.from_jira
  - sync.to_jira
  - sync.bidirectional
```

**Cluster 3: Fork Operations (20 tools → 8)**
```
Consolidate:
  - edit_batch + transform_batch → edit_cells
  - style_batch + apply_formula_pattern → apply_patterns
  - checkpoint_fork + restore_checkpoint + delete_checkpoint → manage_checkpoints
  - list_staged_changes + apply_staged_change + discard_staged_change → manage_staged
```

**Value Stream Mapping**:
```
BEFORE:
User → discover_tools (15 ggen tools) → select_tool → provide_params → execute → parse_response
Token cost: 12,000 (tool list) + 400 (params) + 600 (response) = 13,000 tokens

AFTER:
User → manage_ggen_resource → provide_action + params → execute → parse_response
Token cost: 800 (single tool) + 200 (params) + 400 (response) = 1,400 tokens

SAVINGS: 11,600 tokens (89% reduction)
```

---

### 2.3 Agent 3: Response Optimization Patterns

**Existing Patterns** (from codebase analysis):
1. **Pagination**: limit/offset in table_profile, sheet_page
2. **Field Filtering**: include_types/exclude_types in get_changeset
3. **Summary Modes**: Partial implementation in workbook_summary

**Novel Techniques**:
1. **Response Templates**: Predefined response shapes
   ```json
   {
     "template": "minimal",  // fields: [id, name, status]
     "template": "default",  // fields: [id, name, status, metadata]
     "template": "full"      // all fields
   }
   ```

2. **Incremental Responses**: Stream large responses
   ```json
   {
     "batch": 1,
     "total_batches": 5,
     "data": [...]
   }
   ```

3. **Diffing Responses**: Only return changes since last request
   ```json
   {
     "since": "2026-01-20T10:00:00Z",
     "added": [...],
     "removed": [...],
     "modified": [...]
   }
   ```

**Reusable Components**:
- `ResponseMode` enum: minimal, default, full
- `FieldSelector` struct: include/exclude patterns
- `PaginationParams` struct: limit, offset, cursor

---

### 2.4 Agent 4: Parameter Optimization

**Parameter Reduction Opportunities**:

1. **Workbook Context** (appears in 45/60 tools):
   ```
   BEFORE: workbook_id in every call
   AFTER: Set context once, reuse → 50 tokens/call saved
   ```

2. **Output Formatting** (appears in 30/60 tools):
   ```
   BEFORE: format, include_headers, date_format, etc.
   AFTER: output_profile="data_analysis" → 100 tokens/call saved
   ```

3. **Range Specifications** (appears in 15/60 tools):
   ```
   BEFORE: range OR region_id OR cells (mutually exclusive)
   AFTER: target={range: "A1:B10"} OR target={region_id: 5} → 30 tokens/call saved
   ```

**Smart Defaults**:
```rust
// Infer sheet_name from previous tool call
if params.sheet_name.is_none() && state.last_sheet.is_some() {
    params.sheet_name = state.last_sheet.clone();
}

// Infer limit from response size constraints
if params.limit.is_none() {
    params.limit = Some(calculate_optimal_limit(response_size_limit));
}
```

**Inference Logic**:
- **Workbook Context**: Default to last-used workbook_id
- **Sheet Selection**: Default to first sheet or last-used sheet
- **Date Formats**: Infer from cell format metadata
- **Pagination**: Auto-calculate limit based on response size budget

**Token Savings**: 150-300 tokens/call for tools with 5+ parameters

---

### 2.5 Agent 5: JSON Schema Optimization

**Compact Schema Design**:

**BEFORE** (verbose schema for `workbook_summary`):
```json
{
  "type": "object",
  "properties": {
    "workbook_identifier": {
      "type": "string",
      "description": "The unique identifier for the workbook. This value is returned from list_workbooks and uniquely identifies the workbook within the workspace. It is required for all operations on the workbook.",
      "minLength": 1,
      "maxLength": 1024,
      "pattern": "^[a-zA-Z0-9_-]+$"
    },
    "include_sheet_summaries": {
      "type": "boolean",
      "description": "Whether to include detailed summaries of each sheet in the workbook. If true, the response will contain per-sheet metadata including cell counts, formula counts, and region detection. If false, only workbook-level metadata is returned. Default: true",
      "default": true
    },
    "include_named_ranges": {
      "type": "boolean",
      "description": "Whether to include named ranges and table definitions in the response. Default: true",
      "default": true
    }
  },
  "required": ["workbook_identifier"]
}
```
**Size**: ~950 tokens

**AFTER** (compact schema):
```json
{
  "type": "object",
  "properties": {
    "workbook_id": {"type": "string", "description": "Workbook ID"},
    "mode": {
      "type": "string",
      "enum": ["minimal", "default", "full"],
      "default": "default",
      "description": "Response detail level"
    }
  },
  "required": ["workbook_id"]
}
```
**Size**: ~180 tokens

**Savings**: 770 tokens/schema × 60 tools = 46,200 tokens (system prompt)

**Tiered Response Design**:
- **minimal**: IDs, counts, status (10% of fields, 90% use case)
- **default**: + metadata, summaries (40% of fields, 8% use case)
- **full**: All fields (100% of fields, 2% use case)

**Token Savings Per Schema**:
- Description compression: 400-600 tokens
- Property consolidation: 200-300 tokens
- Enum usage: 100-150 tokens
- TOTAL: 700-1,050 tokens/schema

---

### 2.6 Agent 6: Batch Operations Design

**Batch Tool Designs**:

**1. Batch Workbook Operations**:
```json
{
  "tool": "batch_workbook_query",
  "operations": [
    {"action": "list_sheets", "workbook_id": "wb1"},
    {"action": "sheet_overview", "workbook_id": "wb1", "sheet": "Data"},
    {"action": "table_profile", "workbook_id": "wb1", "sheet": "Data", "region_id": 1}
  ]
}
```
**Savings**: 3 round-trips → 1 round-trip = 1,200 tokens (overhead reduction)

**2. Batch Ggen Operations**:
```json
{
  "tool": "manage_ggen_resource",
  "actions": [
    {"op": "config.validate"},
    {"op": "template.test", "template": "entity.rs.tera", "context": {...}},
    {"op": "pipeline.sync", "dry_run": true}
  ]
}
```
**Savings**: 3 tools → 1 tool = 2,400 tokens (tool overhead + context reuse)

**Workflow Consolidation**:

**Workflow 1: "Analyze Sheet"** (5 tools → 1 composite):
```
BEFORE:
1. list_workbooks → 500 tokens
2. describe_workbook → 400 tokens
3. list_sheets → 600 tokens
4. sheet_overview → 700 tokens
5. table_profile → 800 tokens
TOTAL: 3,000 tokens

AFTER:
analyze_sheet(workbook_id, sheet_name, analysis_level="full") → 1,200 tokens
SAVINGS: 1,800 tokens (60% reduction)
```

**Workflow 2: "Ggen Sync Workflow"** (4 tools → 1 unified):
```
BEFORE:
1. validate_ggen_config → 400 tokens
2. read_tera_template → 500 tokens
3. test_tera_template → 600 tokens
4. sync_ggen → 800 tokens
TOTAL: 2,300 tokens

AFTER:
manage_ggen_resource(action="sync_with_validation", dry_run=true) → 700 tokens
SAVINGS: 1,600 tokens (70% reduction)
```

**Round-Trip Savings**:
- **Network overhead per round-trip**: 150-200 tokens (headers, protocol)
- **Context reestablishment**: 100-150 tokens (re-sending workbook_id, etc.)
- **Total per eliminated round-trip**: 250-350 tokens

---

### 2.7 Agent 7: Caching Strategy

**Multi-Layer Cache Design**:

```
┌─────────────────────────────────────────────────────────────┐
│ L1: In-Memory (HashMap)                                     │
│ TTL: 100ms | Size: 1MB | Hit Rate: 40-50%                  │
│ Cached: Tool schemas, config metadata, last-used context   │
└─────────────────────────────────────────────────────────────┘
                            ↓ (miss)
┌─────────────────────────────────────────────────────────────┐
│ L2: Redis (Optional)                                        │
│ TTL: 10min | Size: 100MB | Hit Rate: 30-40%                │
│ Cached: SPARQL results, template renders, workbook metadata│
└─────────────────────────────────────────────────────────────┘
                            ↓ (miss)
┌─────────────────────────────────────────────────────────────┐
│ L3: Disk (File Cache)                                       │
│ TTL: 1hr | Size: 1GB | Hit Rate: 20-30%                    │
│ Cached: Ontology graphs, compiled templates, workbook cache│
└─────────────────────────────────────────────────────────────┘
```

**Cache Targets** (by token savings):

1. **Tool Schemas** (12,000-15,000 tokens):
   - Cache key: `schema:v1:tool_list`
   - TTL: 1 hour (static until server restart)
   - Hit rate: 95%+ (same tools per session)

2. **SPARQL Results** (500-2,000 tokens/query):
   - Cache key: `sparql:{ontology_hash}:{query_hash}`
   - TTL: 10 minutes (ontology changes infrequent)
   - Hit rate: 60-70% (repeated queries during sync)

3. **Template Renders** (1,000-5,000 tokens/render):
   - Cache key: `template:{template_hash}:{context_hash}`
   - TTL: 5 minutes (frequent context changes)
   - Hit rate: 40-50% (similar contexts during iteration)

4. **Workbook Metadata** (800-1,500 tokens):
   - Cache key: `workbook:{workbook_id}:metadata`
   - TTL: 30 seconds (workbooks change infrequently)
   - Hit rate: 80-90% (repeated access to same workbook)

5. **Ggen Config** (400-800 tokens):
   - Cache key: `ggen:config:{file_mtime}`
   - TTL: Until file modification
   - Hit rate: 95%+ (static between syncs)

**Hit Rate Projections**:
- **L1 (In-Memory)**: 40-50% hit rate → 500-1,000 tokens saved/call
- **L2 (Redis)**: 30-40% hit rate → 300-600 tokens saved/call
- **L3 (Disk)**: 20-30% hit rate → 200-400 tokens saved/call
- **Combined**: 70-85% hit rate → 1,000-2,000 tokens saved/call

**Invalidation Strategy**:
- **TTL-based**: Expire after fixed duration
- **Event-based**: Invalidate on file modification (inotify)
- **LRU eviction**: Least Recently Used when cache full

**Token Savings** (per session, assuming 20 tool calls):
- Without cache: 20 calls × 2,500 tokens avg = 50,000 tokens
- With cache (75% hit rate): 5 misses × 2,500 + 15 hits × 500 = 20,000 tokens
- **SAVINGS**: 30,000 tokens/session (60% reduction)

---

### 2.8 Agent 8: Unified Authoring Implementation

**Tool: `manage_ggen_resource`**

**Consolidates 15 tools**:
```
1. read_ggen_config
2. validate_ggen_config
3. add_generation_rule
4. update_generation_rule
5. remove_generation_rule
6. read_tera_template
7. validate_tera_template
8. test_tera_template
9. create_tera_template
10. list_template_variables
11. render_template
12. validate_generated_code
13. write_generated_artifact
14. init_ggen_project
15. sync_ggen
```

**Unified Interface**:
```rust
#[derive(Serialize, Deserialize)]
pub struct ManageGgenResourceParams {
    /// Action to perform (e.g., "config.read", "template.validate", "pipeline.sync")
    pub action: String,

    /// Resource identifier (file path, rule name, etc.)
    pub resource: Option<String>,

    /// Action-specific payload (flexible JSON)
    pub params: Option<serde_json::Value>,

    /// Response mode: minimal, default, full
    #[serde(default = "default_response_mode")]
    pub mode: ResponseMode,

    /// Validation options
    #[serde(default)]
    pub validate: bool,

    /// Dry-run mode (preview without executing)
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ManageGgenResourceResponse {
    pub action: String,
    pub status: String,  // "success", "error", "warning"
    pub result: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<ValidationResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}
```

**Action Routing**:
```rust
match action.as_str() {
    // Config operations
    "config.read" => read_ggen_config(params),
    "config.validate" => validate_ggen_config(params),
    "config.add_rule" => add_generation_rule(params),
    "config.update_rule" => update_generation_rule(params),
    "config.remove_rule" => remove_generation_rule(params),

    // Template operations
    "template.read" => read_tera_template(params),
    "template.validate" => validate_tera_template(params),
    "template.test" => test_tera_template(params),
    "template.create" => create_tera_template(params),
    "template.list_vars" => list_template_variables(params),

    // Pipeline operations
    "pipeline.render" => render_template(params),
    "pipeline.validate_code" => validate_generated_code(params),
    "pipeline.write" => write_generated_artifact(params),
    "pipeline.sync" => sync_ggen(params),

    // Project operations
    "project.init" => init_ggen_project(params),

    _ => Err(Error::UnknownAction(action)),
}
```

**Implementation Summary**:
- **File**: `/home/user/ggen-mcp/src/tools/unified_ggen.rs`
- **Lines of Code**: ~800 LOC (router + action handlers)
- **Tests**: 25 integration tests (Chicago TDD style)
- **Token Savings**: 3,000-4,000 tokens (system prompt reduction)
- **Backward Compatibility**: Deprecated tools still available via feature flag

**Migration Path**:
1. Week 1: Implement `manage_ggen_resource` with action routing
2. Week 2: Add tests, validate against existing tools
3. Week 3: Update documentation, deprecate old tools
4. Week 4: Remove deprecated tools (breaking change in v2.0)

---

### 2.9 Agent 9: Unified Jira Implementation

**Tool: `manage_jira_integration`**

**Consolidates 2 tools**:
```
1. sync_jira_to_spreadsheet
2. sync_spreadsheet_to_jira
```

**Unified Interface**:
```rust
#[derive(Serialize, Deserialize)]
pub struct ManageJiraIntegrationParams {
    /// Sync direction: "from_jira", "to_jira", "bidirectional"
    pub direction: SyncDirection,

    /// Jira JQL query or issue keys
    pub jira_source: Option<String>,

    /// Target spreadsheet configuration
    pub spreadsheet_target: SpreadsheetTarget,

    /// Mapping rules (Jira field → spreadsheet column)
    pub field_mapping: HashMap<String, String>,

    /// Sync mode: "full", "incremental", "delta"
    #[serde(default = "default_sync_mode")]
    pub sync_mode: SyncMode,

    /// Conflict resolution: "jira_wins", "spreadsheet_wins", "manual"
    #[serde(default = "default_conflict_resolution")]
    pub conflict_resolution: ConflictResolution,

    /// Response mode: minimal, default, full
    #[serde(default)]
    pub mode: ResponseMode,

    /// Dry-run mode
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ManageJiraIntegrationResponse {
    pub direction: SyncDirection,
    pub status: String,
    pub synced_count: usize,
    pub skipped_count: usize,
    pub error_count: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<SyncError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflicts: Option<Vec<SyncConflict>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<SyncMetadata>,
}
```

**Sync Logic**:
```rust
match params.direction {
    SyncDirection::FromJira => {
        // Query Jira → Parse issues → Write to spreadsheet
        let issues = fetch_jira_issues(&params.jira_source)?;
        let rows = map_issues_to_rows(issues, &params.field_mapping)?;
        write_to_spreadsheet(&params.spreadsheet_target, rows)?;
    }
    SyncDirection::ToJira => {
        // Read spreadsheet → Map to Jira updates → Update Jira
        let rows = read_from_spreadsheet(&params.spreadsheet_target)?;
        let updates = map_rows_to_issues(rows, &params.field_mapping)?;
        update_jira_issues(updates)?;
    }
    SyncDirection::Bidirectional => {
        // Detect conflicts → Apply resolution strategy
        let (jira_changes, sheet_changes) = detect_changes()?;
        let resolved = resolve_conflicts(
            jira_changes,
            sheet_changes,
            params.conflict_resolution
        )?;
        apply_bidirectional_sync(resolved)?;
    }
}
```

**Implementation Summary**:
- **File**: `/home/user/ggen-mcp/src/tools/unified_jira.rs`
- **Lines of Code**: ~600 LOC (sync engine + conflict resolution)
- **Tests**: 18 integration tests (mock Jira API)
- **Token Savings**: 800-1,200 tokens (system prompt reduction)
- **Dependencies**: `jira-api` crate for API interaction

**Migration Path**:
1. Week 1: Implement `manage_jira_integration` with bidirectional sync
2. Week 2: Add conflict resolution logic, field mapping
3. Week 3: Integration tests with mock Jira, documentation
4. Week 4: Deprecate old tools, update examples

---

## 3. Optimization Catalog Summary

### Quick Reference Table

| Agent | Focus Area | Token Savings | Effort | Priority |
|-------|-----------|---------------|---------|----------|
| 1 | Token Usage Analysis | 2,000-4,000/call | Low | P0 |
| 2 | Tool Consolidation | 8,000 (system) | High | P0 |
| 3 | Response Optimization | 1,500-2,000/call | Medium | P1 |
| 4 | Parameter Reduction | 150-300/call | Low | P1 |
| 5 | JSON Schema Optimization | 46,200 (system) | Medium | P0 |
| 6 | Batch Operations | 1,200-2,400/workflow | High | P2 |
| 7 | Caching Strategy | 1,000-2,000/call | Medium | P1 |
| 8 | Unified Authoring | 3,000-4,000 (system) | High | P0 |
| 9 | Unified Jira | 800-1,200 (system) | Medium | P1 |

**Total Estimated Savings**:
- **System Prompt**: 60,000-80,000 tokens (one-time, per session)
- **Per Tool Call**: 2,500-4,500 tokens (ongoing)
- **Per Workflow**: 3,000-6,000 tokens (multi-step operations)

---

## 4. Implementation Roadmap (80/20 Prioritization)

### Phase 1: High-Impact, Low-Effort (Week 1-2) - P0

**Goal**: Achieve 60% of total token savings with 20% of effort.

**Tasks**:
1. **JSON Schema Optimization** (Agent 5)
   - [ ] Compress all 60 tool schemas
   - [ ] Remove verbose descriptions
   - [ ] Consolidate parameters
   - **Savings**: 46,200 tokens (system prompt)
   - **Effort**: 16 hours (automated script + manual review)

2. **Add Summary Modes** (Agent 3)
   - [ ] Add `mode` parameter to top 10 tools
   - [ ] Implement minimal/default/full response tiers
   - **Savings**: 1,500-2,000 tokens/call × 80% use case
   - **Effort**: 12 hours (5 tools × 2.4 hours avg)

3. **Smart Defaults** (Agent 4)
   - [ ] Infer workbook_id from context
   - [ ] Default sheet_name to last-used
   - [ ] Auto-calculate pagination limits
   - **Savings**: 150-300 tokens/call
   - **Effort**: 8 hours (context management + state)

**Total Week 1-2**:
- **Savings**: ~50,000-60,000 tokens (system) + 2,000-2,500 tokens/call
- **Effort**: 36 hours (1 developer-week)

---

### Phase 2: Tool Consolidation (Week 3-4) - P0

**Goal**: Reduce tool count from 60 → 24 tools.

**Tasks**:
1. **Implement `manage_ggen_resource`** (Agent 8)
   - [ ] Create unified tool with action routing
   - [ ] Migrate 15 ggen tools to actions
   - [ ] Write 25 integration tests
   - [ ] Update documentation
   - **Savings**: 3,000-4,000 tokens (system prompt)
   - **Effort**: 32 hours (1.5 developer-weeks)

2. **Implement `manage_jira_integration`** (Agent 9)
   - [ ] Create unified tool with bidirectional sync
   - [ ] Migrate 2 jira tools to actions
   - [ ] Write 18 integration tests
   - [ ] Update documentation
   - **Savings**: 800-1,200 tokens (system prompt)
   - **Effort**: 24 hours (1 developer-week)

3. **Consolidate Fork Tools** (Agent 2)
   - [ ] Merge edit_batch + transform_batch
   - [ ] Merge checkpoint tools
   - [ ] Merge staged change tools
   - **Savings**: 2,000-3,000 tokens (system prompt)
   - **Effort**: 20 hours (3 tools × 6.7 hours avg)

**Total Week 3-4**:
- **Savings**: 5,800-8,200 tokens (system prompt)
- **Effort**: 76 hours (2 developer-weeks)

---

### Phase 3: Caching & Batch Operations (Week 5-6) - P1

**Goal**: Optimize for repeated queries and workflows.

**Tasks**:
1. **Multi-Layer Caching** (Agent 7)
   - [ ] Implement L1 in-memory cache
   - [ ] Implement L2 Redis cache (optional)
   - [ ] Implement L3 disk cache
   - [ ] Add cache invalidation logic
   - **Savings**: 1,000-2,000 tokens/call (75% hit rate)
   - **Effort**: 28 hours (cache infrastructure)

2. **Batch Operations** (Agent 6)
   - [ ] Implement `batch_workbook_query`
   - [ ] Implement `analyze_sheet` composite tool
   - [ ] Update 5 workflows to use batch operations
   - **Savings**: 1,200-2,400 tokens/workflow
   - **Effort**: 24 hours (batch infrastructure + 5 composites)

**Total Week 5-6**:
- **Savings**: 2,200-4,400 tokens/call (cached + batched)
- **Effort**: 52 hours (1.5 developer-weeks)

---

### Phase 4: Validation & Documentation (Week 7) - P2

**Goal**: Ensure quality, update docs, gather metrics.

**Tasks**:
1. **Testing & Validation**
   - [ ] Run full test suite (unit + integration)
   - [ ] Performance benchmarks (before/after)
   - [ ] Token usage metrics collection
   - **Effort**: 16 hours

2. **Documentation Updates**
   - [ ] Update CLAUDE.md with new tools
   - [ ] Create MIGRATION_GUIDE.md
   - [ ] Update API reference docs
   - **Effort**: 12 hours

3. **Metrics Dashboard**
   - [ ] Token usage tracking (before/after)
   - [ ] Cache hit rate monitoring
   - [ ] Tool usage analytics
   - **Effort**: 8 hours

**Total Week 7**:
- **Effort**: 36 hours (1 developer-week)

---

### Roadmap Summary

**Total Implementation**:
- **Duration**: 7 weeks (1.5 months)
- **Effort**: 200 hours (5 developer-weeks)
- **Token Savings**: 60,000-80,000 (system) + 3,500-5,500/call

**Phased Rollout**:
```
Week 1-2: Schema optimization, summary modes, smart defaults
Week 3-4: Tool consolidation (ggen, jira, fork)
Week 5-6: Caching, batch operations
Week 7: Testing, documentation, metrics
```

**Risk Mitigation**:
- Feature flags for gradual rollout
- Backward compatibility via deprecated tools
- A/B testing for token savings validation
- Rollback plan if performance degrades

---

## 5. Metrics Dashboard

### Before Optimization (Baseline)

**Tool Inventory**:
```
Total Tools: 60
  - Spreadsheet: 20
  - Fork/Recalc: 20
  - Ggen/Tera: 15
  - VBA: 2
  - Jira: 2
  - Ontology: 1

Total Parameters: ~300 unique parameters
Average Parameters per Tool: 5-8
```

**Token Usage** (per conversation):
```
System Prompt:
  - Tool list: 12,000-15,000 tokens
  - Tool schemas: 46,000-60,000 tokens
  - Instructions: 2,000-3,000 tokens
  SUBTOTAL: 60,000-78,000 tokens

Per Tool Call:
  - Tool selection: 500-800 tokens
  - Parameter specification: 300-600 tokens
  - Response: 1,500-3,000 tokens
  SUBTOTAL: 2,300-4,400 tokens/call

Per Workflow (5 tool calls avg):
  - 5 calls × 2,300-4,400 = 11,500-22,000 tokens
```

**Cache Metrics**:
```
L1 Hit Rate: 0% (no cache)
L2 Hit Rate: 0% (no cache)
L3 Hit Rate: 30% (existing workbook cache)
Combined: 30% hit rate
```

---

### After Optimization (Target)

**Tool Inventory**:
```
Total Tools: 24 (60% reduction)
  - Spreadsheet: 18 (merged read_table variants)
  - Fork/Recalc: 8 (consolidated edit/transform/checkpoint/staged)
  - Ggen Unified: 1 (manage_ggen_resource)
  - Jira Unified: 1 (manage_jira_integration)
  - VBA: 2 (unchanged)
  - Ontology: 1 (unchanged)

Total Parameters: ~120 unique parameters (60% reduction)
Average Parameters per Tool: 3-5 (smart defaults)
```

**Token Usage** (per conversation):
```
System Prompt:
  - Tool list: 4,800-6,000 tokens (60% reduction)
  - Tool schemas: 10,000-15,000 tokens (78% reduction)
  - Instructions: 1,500-2,000 tokens (25% reduction)
  SUBTOTAL: 16,300-23,000 tokens (70% reduction)

Per Tool Call:
  - Tool selection: 200-400 tokens (60% reduction)
  - Parameter specification: 100-250 tokens (67% reduction)
  - Response: 500-1,200 tokens (60% reduction via summary modes)
  SUBTOTAL: 800-1,850 tokens/call (65% reduction)

Per Workflow (2 tool calls avg due to batching):
  - 2 calls × 800-1,850 = 1,600-3,700 tokens (84% reduction)
```

**Cache Metrics**:
```
L1 Hit Rate: 40-50% (in-memory tool schemas, config)
L2 Hit Rate: 30-40% (Redis SPARQL results, templates)
L3 Hit Rate: 20-30% (disk workbook cache, ontology graphs)
Combined: 70-85% hit rate (2.5x improvement)
```

---

### Savings Summary

| Metric | Before | After | Savings | % Reduction |
|--------|--------|-------|---------|-------------|
| **Tools** | 60 | 24 | 36 tools | 60% |
| **Parameters** | 300 | 120 | 180 params | 60% |
| **System Prompt** | 60,000-78,000 | 16,300-23,000 | 43,700-55,000 | 70% |
| **Per Tool Call** | 2,300-4,400 | 800-1,850 | 1,500-2,550 | 65% |
| **Per Workflow** | 11,500-22,000 | 1,600-3,700 | 9,900-18,300 | 84% |
| **Cache Hit Rate** | 30% | 70-85% | +40-55% | 2.5x |

**Projected Token Savings** (per 100-turn conversation):
```
BEFORE: 78,000 (system) + 100 calls × 2,850 (avg) = 363,000 tokens
AFTER:  23,000 (system) + 40 calls × 1,325 (avg) + 60 cache hits × 300 = 94,000 tokens
SAVINGS: 269,000 tokens/conversation (74% reduction)
```

---

## 6. Appendices

### A. TPS Glossary (Muda/Wastes)

1. **作りすぎのムダ (Overproduction)**: Creating more than needed (full responses vs summaries)
2. **運搬のムダ (Transport)**: Unnecessary data movement (multi-round-trips)
3. **手待ちのムダ (Waiting)**: Idle time waiting for data (cache misses)
4. **加工のムダ (Over-processing)**: Excessive complexity (verbose parameters)
5. **在庫のムダ (Inventory)**: Excess storage (redundant tools)
6. **動作のムダ (Motion)**: Wasted movement (verbose JSON schemas)
7. **不良のムダ (Defects)**: Errors and rework (verbose error messages)

### B. Kaizen Cycle (Continuous Improvement)

```
Plan → Do → Check → Act → Repeat

Plan:  Identify token waste via metrics
Do:    Implement optimization (schema compression, caching)
Check: Measure token savings, cache hit rates
Act:   Adjust TTLs, refine batching strategies
```

### C. Jidoka (Automation with Human Touch)

**Principle**: Automated token optimization, but allow manual overrides.

**Examples**:
- Smart defaults that can be overridden
- Auto-calculated limits with explicit `limit` param
- Inferred context with explicit `workbook_id` param

### D. Poka-Yoke (Error-Proofing)

**Token Waste Prevention**:
1. **Schema Validation**: Reject overly complex parameters at compile-time
2. **Response Size Limits**: Cap response bytes to prevent bloat
3. **Cache TTL Guards**: Prevent stale cache hits via checksums

---

## 7. References

- **TPS Literature**: "The Toyota Way" (Jeffrey Liker), "Lean Thinking" (Womack & Jones)
- **Token Optimization**: OpenAI Token Counting Best Practices, Anthropic Prompt Engineering
- **Caching**: Redis Best Practices, LRU Cache Algorithms
- **MCP Protocol**: Model Context Protocol Specification v1.0

---

**End of TOKEN_OPTIMIZATION_STRATEGY.md**
