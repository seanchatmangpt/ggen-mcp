# MIGRATION GUIDE: Token-Optimized MCP Tools

**Version**: 2.0.0 | Breaking Changes | Token Optimization Release
**Date**: 2026-01-20
**Migration Path**: v1.x → v2.0

---

## Overview

ggen-mcp v2.0 consolidates 60 tools → 24 tools for 70% token reduction. This guide covers migration from legacy tools to unified interfaces.

**Key Changes**:
- 15 ggen tools → 1 `manage_ggen_resource`
- 2 Jira tools → 1 `manage_jira_integration`
- 20 fork tools → 8 consolidated tools
- All tools support `mode` parameter (minimal/default/full)
- Smart defaults reduce parameter overhead by 60%

---

## Breaking Changes

### 1. Ggen Tools Consolidation

**DEPRECATED** (15 tools removed):
```
read_ggen_config
validate_ggen_config
add_generation_rule
update_generation_rule
remove_generation_rule
read_tera_template
validate_tera_template
test_tera_template
create_tera_template
list_template_variables
render_template
validate_generated_code
write_generated_artifact
init_ggen_project
sync_ggen
```

**REPLACEMENT**: `manage_ggen_resource`

**Migration Examples**:

#### BEFORE: `read_ggen_config`
```json
{
  "tool": "read_ggen_config",
  "params": {
    "config_path": "ggen.toml"
  }
}
```

#### AFTER: `manage_ggen_resource`
```json
{
  "tool": "manage_ggen_resource",
  "params": {
    "action": "config.read",
    "resource": "ggen.toml",
    "mode": "default"
  }
}
```

---

#### BEFORE: `sync_ggen`
```json
{
  "tool": "sync_ggen",
  "params": {
    "config_path": "ggen.toml",
    "dry_run": true,
    "validate": true
  }
}
```

#### AFTER: `manage_ggen_resource`
```json
{
  "tool": "manage_ggen_resource",
  "params": {
    "action": "pipeline.sync",
    "resource": "ggen.toml",
    "dry_run": true,
    "validate": true,
    "mode": "default"
  }
}
```

---

### 2. Preview Mode: Default Behavior (Safety-First)

**BREAKING**: Sync operations now default to **preview mode** (no file writes).

**Impact**: Existing scripts/workflows that rely on automatic file writes will need updates.

**Migration**:

#### BEFORE: Automatic Apply
```json
// v1.x: Applied changes immediately
{
  "tool": "sync_ggen",
  "params": {
    "workspace_root": ".",
    "force": false
  }
}
```

#### AFTER: Explicit Opt-In for Writes
```json
// v2.0+: Preview by default (safe)
{
  "tool": "manage_ggen_resource",
  "params": {
    "action": "pipeline.sync",
    "resource": "ggen.toml",
    "preview": true  // DEFAULT - set false to apply changes
  }
}

// To apply changes (explicitly opt-out of preview):
{
  "tool": "manage_ggen_resource",
  "params": {
    "action": "pipeline.sync",
    "resource": "ggen.toml",
    "preview": false  // Explicitly apply changes
  }
}
```

**Rationale**: "Explicit is better than implicit." Preview-first prevents accidental overwrites and aligns with TPS safety principles.

**Recommended Workflow**:
1. **Preview**: Run with `preview: true` (default) → Check report
2. **Apply**: Run with `preview: false` (explicit) → Verify

---

#### BEFORE: `render_template`
```json
{
  "tool": "render_template",
  "params": {
    "template": "entity.rs.tera",
    "context": {"name": "User", "fields": [...]},
    "output_format": "rust"
  }
}
```

#### AFTER: `manage_ggen_resource`
```json
{
  "tool": "manage_ggen_resource",
  "params": {
    "action": "pipeline.render",
    "resource": "entity.rs.tera",
    "params": {
      "context": {"name": "User", "fields": [...]},
      "output_format": "rust"
    }
  }
}
```

---

### 2. Jira Tools Consolidation

**DEPRECATED** (2 tools removed):
```
sync_jira_to_spreadsheet
sync_spreadsheet_to_jira
```

**REPLACEMENT**: `manage_jira_integration`

**Migration Examples**:

#### BEFORE: `sync_jira_to_spreadsheet`
```json
{
  "tool": "sync_jira_to_spreadsheet",
  "params": {
    "jql": "project = DEMO",
    "workbook_id": "wb123",
    "sheet_name": "Issues",
    "field_mapping": {
      "summary": "A",
      "status": "B",
      "assignee": "C"
    }
  }
}
```

#### AFTER: `manage_jira_integration`
```json
{
  "tool": "manage_jira_integration",
  "params": {
    "direction": "from_jira",
    "jira_source": "project = DEMO",
    "spreadsheet_target": {
      "workbook_id": "wb123",
      "sheet_name": "Issues"
    },
    "field_mapping": {
      "summary": "A",
      "status": "B",
      "assignee": "C"
    }
  }
}
```

---

### 3. Fork Tools Consolidation

**DEPRECATED** (12 tools consolidated):
```
edit_batch              → edit_cells
transform_batch         → edit_cells
checkpoint_fork         → manage_checkpoints
restore_checkpoint      → manage_checkpoints
delete_checkpoint       → manage_checkpoints
list_staged_changes     → manage_staged
apply_staged_change     → manage_staged
discard_staged_change   → manage_staged
```

**Migration Examples**:

#### BEFORE: `checkpoint_fork`
```json
{
  "tool": "checkpoint_fork",
  "params": {
    "fork_id": "fork-abc123",
    "checkpoint_name": "before_edits"
  }
}
```

#### AFTER: `manage_checkpoints`
```json
{
  "tool": "manage_checkpoints",
  "params": {
    "action": "create",
    "fork_id": "fork-abc123",
    "checkpoint_name": "before_edits"
  }
}
```

---

#### BEFORE: `list_staged_changes`
```json
{
  "tool": "list_staged_changes",
  "params": {
    "fork_id": "fork-abc123"
  }
}
```

#### AFTER: `manage_staged`
```json
{
  "tool": "manage_staged",
  "params": {
    "action": "list",
    "fork_id": "fork-abc123",
    "mode": "minimal"
  }
}
```

---

### 4. Response Mode Parameter (All Tools)

**NEW FEATURE**: All tools now support `mode` parameter.

**Values**:
- `minimal`: IDs, counts, status only (10% of fields, 90% use case)
- `default`: + metadata, summaries (40% of fields, 8% use case)
- `full`: All fields (100% of fields, 2% use case)

**Example**:

#### BEFORE (no mode, always full response):
```json
{
  "tool": "workbook_summary",
  "params": {
    "workbook_id": "wb123"
  }
}
// Response: 2KB (all fields)
```

#### AFTER (with mode):
```json
{
  "tool": "workbook_summary",
  "params": {
    "workbook_id": "wb123",
    "mode": "minimal"
  }
}
// Response: 400 bytes (summary only)
```

---

### 5. Smart Defaults (Parameter Reduction)

**NEW FEATURE**: Context-aware parameter inference.

**Inferred Parameters**:
- `workbook_id`: Defaults to last-used workbook
- `sheet_name`: Defaults to first sheet or last-used sheet
- `limit`: Auto-calculated based on response size budget
- `format`: Inferred from file extension or cell format

**Example**:

#### BEFORE (explicit parameters):
```json
{
  "tool": "read_table",
  "params": {
    "workbook_id": "wb123",
    "sheet_name": "Data",
    "range": "A1:Z100",
    "limit": 1000,
    "format": "json"
  }
}
```

#### AFTER (smart defaults):
```json
{
  "tool": "read_table",
  "params": {
    "range": "A1:Z100"
  }
  // workbook_id, sheet_name, limit, format inferred from context
}
```

**Note**: Explicit parameters always override defaults.

---

## Compatibility Shims

### Backward Compatibility Mode

**Enable**: Set environment variable `SPREADSHEET_MCP_LEGACY_TOOLS=true`

**Effect**: Deprecated tools remain available but emit warnings.

**Example**:
```bash
SPREADSHEET_MCP_LEGACY_TOOLS=true ./spreadsheet-mcp
```

**Response Warning**:
```json
{
  "status": "success",
  "warning": "Tool 'read_ggen_config' is deprecated. Use 'manage_ggen_resource' with action='config.read'.",
  "result": {...}
}
```

---

### Automatic Migration Helper

**Tool**: `migrate_tool_call` (available in v2.0 only)

**Purpose**: Converts legacy tool calls to v2.0 format.

**Usage**:
```json
{
  "tool": "migrate_tool_call",
  "params": {
    "legacy_tool": "read_ggen_config",
    "legacy_params": {
      "config_path": "ggen.toml"
    }
  }
}
```

**Response**:
```json
{
  "migrated_tool": "manage_ggen_resource",
  "migrated_params": {
    "action": "config.read",
    "resource": "ggen.toml",
    "mode": "default"
  },
  "suggestion": "Replace your tool call with the migrated version."
}
```

---

## Tool Mapping Reference

### Complete Old → New Mapping

#### Ggen/Tera Tools
| Old Tool | New Tool | Action | Notes |
|----------|----------|--------|-------|
| `read_ggen_config` | `manage_ggen_resource` | `config.read` | |
| `validate_ggen_config` | `manage_ggen_resource` | `config.validate` | |
| `add_generation_rule` | `manage_ggen_resource` | `config.add_rule` | |
| `update_generation_rule` | `manage_ggen_resource` | `config.update_rule` | |
| `remove_generation_rule` | `manage_ggen_resource` | `config.remove_rule` | |
| `read_tera_template` | `manage_ggen_resource` | `template.read` | |
| `validate_tera_template` | `manage_ggen_resource` | `template.validate` | |
| `test_tera_template` | `manage_ggen_resource` | `template.test` | |
| `create_tera_template` | `manage_ggen_resource` | `template.create` | |
| `list_template_variables` | `manage_ggen_resource` | `template.list_vars` | |
| `render_template` | `manage_ggen_resource` | `pipeline.render` | |
| `validate_generated_code` | `manage_ggen_resource` | `pipeline.validate_code` | |
| `write_generated_artifact` | `manage_ggen_resource` | `pipeline.write` | |
| `sync_ggen` | `manage_ggen_resource` | `pipeline.sync` | |
| `init_ggen_project` | `manage_ggen_resource` | `project.init` | |

#### Jira Tools
| Old Tool | New Tool | Direction | Notes |
|----------|----------|-----------|-------|
| `sync_jira_to_spreadsheet` | `manage_jira_integration` | `from_jira` | |
| `sync_spreadsheet_to_jira` | `manage_jira_integration` | `to_jira` | |

#### Fork Tools
| Old Tool | New Tool | Action | Notes |
|----------|----------|--------|-------|
| `checkpoint_fork` | `manage_checkpoints` | `create` | |
| `restore_checkpoint` | `manage_checkpoints` | `restore` | |
| `delete_checkpoint` | `manage_checkpoints` | `delete` | |
| `list_checkpoints` | `manage_checkpoints` | `list` | |
| `list_staged_changes` | `manage_staged` | `list` | |
| `apply_staged_change` | `manage_staged` | `apply` | |
| `discard_staged_change` | `manage_staged` | `discard` | |
| `edit_batch` | `edit_cells` | N/A | Unified with transform_batch |
| `transform_batch` | `edit_cells` | N/A | Unified with edit_batch |

---

## Deprecation Timeline

### Phase 1: Soft Deprecation (v2.0 - v2.3, 6 months)
- **Status**: Legacy tools available with warnings
- **Recommendation**: Migrate to new tools
- **Breaking Changes**: None (backward compatible)

### Phase 2: Hard Deprecation (v2.4 - v2.6, 3 months)
- **Status**: Legacy tools disabled by default
- **Recommendation**: Must set `SPREADSHEET_MCP_LEGACY_TOOLS=true`
- **Breaking Changes**: Legacy tools require explicit opt-in

### Phase 3: Removal (v3.0+, after 9 months)
- **Status**: Legacy tools removed entirely
- **Recommendation**: Must migrate to v2.0 tools
- **Breaking Changes**: Legacy tools no longer available

**Timeline**:
```
v2.0 (2026-02):  Soft deprecation begins
v2.4 (2026-08):  Hard deprecation (opt-in required)
v3.0 (2026-11):  Legacy tools removed
```

---

## Testing Your Migration

### 1. Unit Test Migration

**Example Test Migration**:

#### BEFORE:
```rust
#[test]
fn test_read_ggen_config() {
    let response = read_ggen_config(ReadGgenConfigParams {
        config_path: "ggen.toml".into(),
    }).await.unwrap();
    assert!(response.rules.len() > 0);
}
```

#### AFTER:
```rust
#[test]
fn test_manage_ggen_resource_read_config() {
    let response = manage_ggen_resource(ManageGgenResourceParams {
        action: "config.read".into(),
        resource: Some("ggen.toml".into()),
        mode: ResponseMode::Default,
        ..Default::default()
    }).await.unwrap();
    assert!(response.result["rules"].as_array().unwrap().len() > 0);
}
```

---

### 2. Integration Test Migration

**Example Workflow Migration**:

#### BEFORE (5 tool calls):
```rust
// 1. Validate config
let valid = validate_ggen_config(...).await?;
assert!(valid.is_valid);

// 2. Read template
let template = read_tera_template(...).await?;

// 3. Test template
let test = test_tera_template(...).await?;
assert!(test.errors.is_empty());

// 4. Render template
let rendered = render_template(...).await?;

// 5. Sync
let sync = sync_ggen(...).await?;
```

#### AFTER (1 tool call with batched actions):
```rust
let response = manage_ggen_resource(ManageGgenResourceParams {
    action: "pipeline.sync".into(),
    resource: Some("ggen.toml".into()),
    validate: true,
    dry_run: true,
    mode: ResponseMode::Full,
    ..Default::default()
}).await?;

assert_eq!(response.status, "success");
assert!(response.validation.unwrap().is_valid);
```

---

### 3. Migration Validation Script

**Run**: `./scripts/validate_migration.sh`

**Checks**:
1. No references to deprecated tools in codebase
2. All tool calls use v2.0 format
3. All tests pass with new tools
4. Token usage metrics show expected reduction

**Example Output**:
```
✓ No deprecated tool calls found
✓ All tests passing (120/120)
✓ Token usage: 45,000 → 18,000 (60% reduction)
✓ Migration complete
```

---

## Common Migration Patterns

### Pattern 1: Config Read → Validate → Sync

**BEFORE**:
```python
# 3 separate tool calls
config = read_ggen_config({"config_path": "ggen.toml"})
validation = validate_ggen_config({"config_path": "ggen.toml"})
if validation["is_valid"]:
    sync = sync_ggen({"config_path": "ggen.toml", "dry_run": True})
```

**AFTER**:
```python
# 1 tool call with built-in validation
result = manage_ggen_resource({
    "action": "pipeline.sync",
    "resource": "ggen.toml",
    "validate": True,
    "dry_run": True
})
# Validation happens automatically before sync
```

---

### Pattern 2: Template Test → Render → Validate → Write

**BEFORE**:
```python
# 4 separate tool calls
test = test_tera_template({
    "template": "entity.rs.tera",
    "context": {"name": "User"}
})
rendered = render_template({
    "template": "entity.rs.tera",
    "context": {"name": "User"}
})
validation = validate_generated_code({
    "code": rendered["output"],
    "language": "rust"
})
if validation["is_valid"]:
    write = write_generated_artifact({
        "content": rendered["output"],
        "output_path": "src/generated/user.rs"
    })
```

**AFTER**:
```python
# 1 tool call (or 2 if you want explicit test first)
result = manage_ggen_resource({
    "action": "pipeline.render",
    "resource": "entity.rs.tera",
    "params": {
        "context": {"name": "User"},
        "output_path": "src/generated/user.rs",
        "validate": True,
        "write": True
    }
})
# Test, render, validate, write all in one atomic operation
```

---

### Pattern 3: Jira Bidirectional Sync

**BEFORE**:
```python
# 2 separate tool calls, manual conflict resolution
jira_to_sheet = sync_jira_to_spreadsheet({
    "jql": "project = DEMO",
    "workbook_id": "wb123",
    "sheet_name": "Issues"
})
# Manual conflict check
sheet_to_jira = sync_spreadsheet_to_jira({
    "workbook_id": "wb123",
    "sheet_name": "Issues",
    "jql": "project = DEMO"
})
```

**AFTER**:
```python
# 1 tool call with automatic conflict resolution
result = manage_jira_integration({
    "direction": "bidirectional",
    "jira_source": "project = DEMO",
    "spreadsheet_target": {
        "workbook_id": "wb123",
        "sheet_name": "Issues"
    },
    "conflict_resolution": "jira_wins"
})
# Conflicts detected and resolved automatically
```

---

## Troubleshooting

### Issue 1: "Unknown action" Error

**Error**:
```json
{
  "error": "UnknownAction: 'config.reads' is not a valid action for manage_ggen_resource"
}
```

**Cause**: Typo in action name.

**Fix**: Check action name against mapping table. Correct: `config.read` (not `config.reads`).

---

### Issue 2: Missing `params` Field

**Error**:
```json
{
  "error": "InvalidParams: Action 'pipeline.render' requires 'params.context' field"
}
```

**Cause**: Action-specific parameters must be nested under `params` field.

**Fix**:
```json
// WRONG:
{
  "action": "pipeline.render",
  "context": {...}
}

// CORRECT:
{
  "action": "pipeline.render",
  "params": {
    "context": {...}
  }
}
```

---

### Issue 3: Unexpected Response Structure

**Error**: Response structure changed from v1.x.

**Cause**: Unified tools use consistent response format.

**Fix**: Update response parsing logic:

```python
# BEFORE (v1.x):
if response["is_valid"]:
    rules = response["rules"]

# AFTER (v2.0):
if response["status"] == "success":
    rules = response["result"]["rules"]
```

---

## Migration Checklist

- [ ] **Review** TOKEN_OPTIMIZATION_STRATEGY.md for context
- [ ] **Identify** all deprecated tool calls in your codebase
- [ ] **Update** tool calls to use v2.0 format (see mapping table)
- [ ] **Add** `mode` parameter to optimize token usage
- [ ] **Remove** redundant parameters (let smart defaults infer)
- [ ] **Update** tests to validate new tool responses
- [ ] **Run** migration validation script
- [ ] **Measure** token usage before/after (expect 60-70% reduction)
- [ ] **Update** documentation and examples
- [ ] **Deploy** and monitor for errors

---

## Support

**Questions**: Open issue at https://github.com/example/ggen-mcp/issues
**Documentation**: https://docs.example.com/ggen-mcp/v2.0/migration
**Slack**: #ggen-mcp-migration

---

**End of MIGRATION_GUIDE.md**
