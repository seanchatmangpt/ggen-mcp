# MCP Integration Guide - ggen-mcp (spreadsheet-mcp)

**Version**: 1.0.0 SPR | Production MCP deployment patterns | Rust rmcp v0.11

---

## Transport Layer

### Deployment Matrix

```
┌─ STDIO (Default)
│  ├─ Use: Single MCP client (Claude, IDEs)
│  ├─ Launch: `spreadsheet-mcp --workspace-root /data --transport stdio`
│  └─ Protocol: JSON-RPC over stdin/stdout
│
└─ HTTP (Streaming)
   ├─ Use: Multiple concurrent agents, Docker orchestration
   ├─ Launch: `spreadsheet-mcp --workspace-root /data --transport http --http-bind 0.0.0.0:8079`
   ├─ Endpoint: http://host:8079/mcp (StreamableHttpService)
   └─ Features: session-local workbooks, metrics, health checks
```

### Docker Patterns

**Multi-stage build** (distroless, nonroot):
```bash
docker build -t spreadsheet-mcp:latest .
docker run -v $(pwd)/workbooks:/data -p 8079:8079 \
  -e SPREADSHEET_MCP_WORKSPACE=/data \
  spreadsheet-mcp --transport http --http-bind 0.0.0.0:8079
```

**Environment override** (stdio):
```bash
docker run -v $(pwd)/workbooks:/data \
  spreadsheet-mcp --transport stdio --workspace-root /data
```

**Health checks** (HTTP only):
```bash
# Liveness
curl http://localhost:8079/health

# Readiness
curl http://localhost:8079/ready

# Component status
curl http://localhost:8079/health/components

# Prometheus metrics
curl http://localhost:8079/metrics
```

---

## Tool Exposure Strategy

### Conditional Tool Sets

**Tier 1: Discovery & Analysis** (Always enabled)
```
CORE (20 tools) → Read-only workbook introspection
├─ list_workbooks: Scan workspace, filter by slug/folder/glob
├─ describe_workbook: Metadata (format, size, sheet count, VBA flag)
├─ workbook_summary: Quick region classification + entry points
├─ list_sheets: Sheet names + counts + classification
└─ ... (15 more analysis tools)
```

**Tier 2: Editing (Conditional)**
```
FORK-BASED (20 tools) → Require --recalc-enabled or cargo build --features recalc
├─ create_fork: Copy workbook, returns fork_id
├─ edit_batch/transform_batch: Bulk cell edits
├─ recalculate: Trigger LibreOffice formula recompute
├─ get_changeset: Diff fork vs original
├─ checkpoint_fork: Save rollback point
├─ save_fork: Write to file
└─ ... (14 more fork tools)

Gate: #[cfg(feature = "recalc")] blocks ALL fork_ tools
Disabled error: RecalcDisabledError (MCP error code)
```

**Tier 3: VBA Inspection (Conditional)**
```
VBA (2 tools) → Require --vba-enabled
├─ vba_project_summary: List modules + metadata from .xlsm
└─ vba_module_source: Paginated source code (offset_lines/limit_lines)

Gate: if !config.vba_enabled → VbaDisabledError
Use case: Read-only VBA audit (code never executed)
```

### Tool Enable/Disable Configuration

**Default**: All available tools enabled.

**Selective enable** (config file or CLI):
```yaml
enabled_tools:
  - list_workbooks
  - list_sheets
  - read_table
  - table_profile
  # Disables: find_value, formula_trace, VBA, fork tools
```

**Implementation** (`is_tool_enabled`):
```rust
if enabled_tools.is_some() {
  enabled_tools.contains(tool_name) // whitelist
} else {
  true // no filter
}
```

---

## Configuration Examples

### File Format (YAML/JSON)

**`.mcp.json` for Claude Desktop**:
```json
{
  "mcpServers": {
    "spreadsheet-mcp": {
      "command": "spreadsheet-mcp",
      "args": [
        "--workspace-root", "/Users/alice/workbooks",
        "--cache-capacity", "10",
        "--transport", "http",
        "--http-bind", "127.0.0.1:8079",
        "--recalc-enabled",
        "--vba-enabled",
        "--tool-timeout-ms", "30000",
        "--max-response-bytes", "1000000"
      ]
    }
  }
}
```

**Server config file** (`server-config.yaml`):
```yaml
workspace_root: /mnt/spreadsheets

# Caching
cache_capacity: 20

# File filtering
extensions:
  - xlsx
  - xlsm
  - xls

# Transport
transport: http
http_bind: 0.0.0.0:8079

# Conditional features
recalc_enabled: true
vba_enabled: false  # Read-only VBA disables if not trusted

max_concurrent_recalcs: 4
tool_timeout_ms: 30000       # 30s per tool
max_response_bytes: 1000000  # 1MB response cap

enabled_tools:
  - list_workbooks
  - list_sheets
  - sheet_overview
  - read_table
  - table_profile
  - find_value
  - sheet_formula_map
  - formula_trace
  - create_fork
  - edit_batch
  - recalculate
  - get_changeset
  - save_fork
  - discard_fork
  - checkpoint_fork
  - restore_checkpoint

allow_overwrite: false
graceful_shutdown_timeout_secs: 45
```

**Command-line override**:
```bash
spreadsheet-mcp \
  --config server-config.yaml \
  --workspace-root /data \
  --cache-capacity 15 \
  --enabled-tools list_workbooks list_sheets read_table \
  --recalc-enabled \
  --transport http \
  --http-bind 127.0.0.1:9000
```

---

## Common Agent Workflows

### Workflow 1: Discovery → Profile → Read

**Goal**: Understand workbook structure before deep dive.

```
STEP 1: list_workbooks(slug_prefix="report")
├─ Response: [{ path: "sales_report.xlsx", size: 2.5MB, ... }]
└─ Output: Workbook slug for next steps

STEP 2: describe_workbook(workbook_id="sales_report")
├─ Response: { format: "xlsx", sheets: 5, has_vba: false, ... }
└─ Output: Format + metadata check

STEP 3: workbook_summary(workbook_id="sales_report")
├─ Response: { entry_points: [...], breakdown: {...}, ... }
└─ Output: Quick region classification (calculator/data/metadata sheets)

STEP 4: sheet_overview(workbook_id="sales_report", sheet_name="Data")
├─ Response: { regions: [{id: "REG_001", bounds: "A1:C100", kind: "table", ...}], ... }
└─ Output: Detected region IDs + bounds (confidence scores)

STEP 5a: table_profile(region_id="REG_001")
├─ Response: { columns: [{name: "Date", type: "date", non_empty: 98}, ...], rows: 100 }
└─ Output: Column types + stats (decide sampling strategy)

STEP 5b: read_table(region_id="REG_001", limit=50, sample_mode="first")
├─ Response: { headers: [...], rows: [[...], ...], ... }
└─ Output: Actual data for analysis

STEP 6 (optional): formula_trace(workbook_id="sales_report", sheet_name="Calc", cell="F10")
└─ Output: Precedent cells + formula flow (for audit/understanding)
```

**Key pattern**: Region detection eliminates manual range guessing. `table_profile` gates full read cost.

---

### Workflow 2: Edit with Checkpoint Recovery

**Goal**: Make changes to workbook with rollback safety.

```
STEP 1: create_fork(workbook_id="budget.xlsx")
├─ Response: { fork_id: "fork_abc123", ... }
└─ Output: Editable copy of original

STEP 2 (optional): checkpoint_fork(fork_id="fork_abc123", label="before_adjustments")
├─ Response: { checkpoint_id: "ckpt_789", timestamp: "2026-01-20T..." }
└─ Output: Restore point (auto-saves state)

STEP 3: edit_batch(fork_id="fork_abc123", sheet_name="Budget", edits=[...])
├─ Edits: [{ address: "B2", value: "=A2*1.1", is_formula: true }, ...]
└─ Output: Staged edits (not saved yet)

STEP 4: transform_batch(fork_id="fork_abc123", sheet_name="Budget",
  operations=[{ range: "C:C", op: "fill", value: 0 }])
└─ Output: Bulk clear/fill applied

STEP 5: recalculate(fork_id="fork_abc123")
├─ Triggers: LibreOffice formula engine (may take seconds)
└─ Output: Formula cells recalculated

STEP 6a (success): get_changeset(fork_id="fork_abc123", limit=100)
├─ Response: { cell_changes: [...], formula_changes: [...], ... }
└─ Output: Review changes before commit

STEP 6b (success): save_fork(fork_id="fork_abc123", filename="budget_updated.xlsx")
└─ Output: Writes to workspace, original untouched

STEP 6c (rollback): restore_checkpoint(fork_id="fork_abc123", checkpoint_id="ckpt_789")
├─ Response: { restored: true, message: "..." }
└─ Output: Reverts to saved checkpoint

STEP 7: discard_fork(fork_id="fork_abc123")
└─ Output: Cleanup (frees memory)
```

**Safety pattern**: Checkpoints before risky bulk edits. `get_changeset` for verification. Discard on cleanup.

---

### Workflow 3: VBA Code Audit (Read-Only)

**Goal**: Inspect VBA macros without execution risk.

```
STEP 1: describe_workbook(workbook_id="macro_file.xlsm")
├─ Response: { has_vba: true, format: "xlsm" }
└─ Output: Confirm VBA presence

STEP 2: vba_project_summary(workbook_id="macro_file.xlsm")
├─ Response: { modules: [
│   { name: "Module1", type: "Standard", function_count: 3 },
│   { name: "ThisWorkbook", type: "Class", ... }
│ ], ... }
└─ Output: Module manifest

STEP 3: vba_module_source(workbook_id="macro_file.xlsm", module_name="Module1",
  offset_lines=0, limit_lines=50)
├─ Response: { source: "Sub Calculate()...", offset: 0, limit: 50, total_lines: 200 }
└─ Output: Paged source (repeat with offset+=50 for continuation)

STEP 4 (analysis): Pattern match for:
├─ Suspicious APIs (CreateObject, ActiveXObject)
├─ File I/O (CreateObject("ADODB.Stream"))
├─ External calls (ShellExecute, etc.)
└─ Output: Risk assessment
```

**Containment**: VBA tools are read-only. Code never executes. `vba_enabled: false` disables tool.

---

## Error Patterns & Handling

### Common Errors

**1. Tool Disabled**
```
ErrorCode: INVALID_REQUEST
Message: "tool 'create_fork' is disabled by server configuration"
Cause: Recalc feature not compiled OR --recalc-enabled not set
Fix: Rebuild with --features recalc OR enable via CLI/config
```

**2. Recalc Disabled**
```
ErrorCode: INVALID_REQUEST
Message: "recalculation is not enabled"
Cause: Fork tool called but recalc feature missing
Fix: docker build --build-arg FEATURES=recalc
```

**3. VBA Disabled**
```
ErrorCode: INVALID_REQUEST
Message: "VBA tools not available"
Cause: --vba-enabled not set
Fix: Enable in config or CLI args
```

**4. Tool Timeout**
```
ErrorCode: INTERNAL_ERROR
Message: "tool 'read_table' timed out after 30000ms"
Cause: Large workbook, complex region detection
Fix: Increase --tool-timeout-ms OR use limit/sample_mode
```

**5. Response Too Large**
```
ErrorCode: INTERNAL_ERROR
Message: "response from 'read_table' exceeds 1000000 bytes"
Cause: Too many rows/columns requested
Fix: Use limit, sample_mode='first', or increase --max-response-bytes
```

**6. Workbook Not Found**
```
ErrorCode: INVALID_REQUEST
Message: "workbook 'missing.xlsx' not found in workspace"
Cause: File doesn't exist or wrong slug
Fix: list_workbooks to find correct name
```

**7. Region Detection Failed**
```
ErrorCode: INVALID_REQUEST
Message: "no region detected at range 'A1:C10'"
Cause: Range is empty or unstructured
Fix: Use sheet_overview to find valid regions OR use sheet_page for raw cells
```

**8. Formula Cycle Detected**
```
ErrorCode: INVALID_REQUEST
Message: "circular reference in formula at B5"
Cause: Workbook has circular formula dependency
Fix: Audit formulas OR skip recalculate
```

### Resilience Patterns

**Retry logic** (fork operations):
```
On RecalcTimeout → retry recalculate up to 3×
On DiskError → wait 2s, retry save_fork
On WorkbookLocked → wait, retry list_forks
```

**Degradation** (if VBA disabled):
```
vba_project_summary → ToolDisabledError
→ Agent skips VBA audit, continues analysis
```

**Response pagination** (large reads):
```
read_table(region_id="REG_001", limit=100, offset=0)
If response >= max_response_bytes:
  → Agent fetches next page with offset=100
  → Concatenate results
```

---

## Configuration Checklist

### Pre-Deployment

- [ ] Feature flag correct: `--features recalc` if editing needed
- [ ] Workspace accessible: `--workspace-root` points to readable dir
- [ ] Transport valid: `--transport stdio` or `--transport http`
- [ ] HTTP bind not privileged: Port ≥ 1024 unless root
- [ ] Timeout reasonable: `--tool-timeout-ms` ≥ 5000 for complex sheets
- [ ] Response cap set: `--max-response-bytes` ≤ 100MB
- [ ] Cache sized right: `--cache-capacity` ≥ max concurrent agents
- [ ] VBA trusted: `--vba-enabled false` by default (code audit only)
- [ ] Extensions filtered: `--extensions xlsx xlsm xls` (reject unknown)
- [ ] Recalc pool sized: `--max-concurrent-recalcs` ≤ CPU cores

### Docker Notes

- Base image: `gcr.io/distroless/static-debian12:nonroot` (security)
- Healthcheck: `CMD ["/usr/local/bin/spreadsheet-mcp", "--version"]`
- ENV override: `SPREADSHEET_MCP_WORKSPACE=/data` (workspace root)
- Volumes: Mount workbooks at `/data` (read/write if fork enabled)

### Agent Integration

- Provide `BASE_INSTRUCTIONS` from server for agent orientation
- Enable `VBA_INSTRUCTIONS` only if `--vba-enabled`
- Enable `WRITE_INSTRUCTIONS` only if `--recalc-enabled`
- Tool discovery via `tools/list` (MCP standard)
- Timeout handling: Agents should retry on timeout

---

## SPR Reference

**Distilled**: Transport layer (stdio/HTTP) → Conditional tool tiers (analysis/fork/VBA) → Config precedence (CLI > file > default) → Workflow patterns (discovery→profile→read, edit→checkpoint→verify) → Error recovery.

**Key associations**:
- RECALC feature gates fork tools entirely (compile-time)
- VBA disabled by default (threat model: read-only code)
- Tool timeout prevents runaway queries
- Response cap prevents OOM on agent side
- Checkpoint/rollback ensures edit safety

