# Entitlement System Quick Start

**30-second setup guide for monetization**

---

## 1. Create License File

```bash
cat > .ggen_license <<'EOF'
{
  "version": "1.0",
  "capabilities": [
    "preview_mode",
    "read_only_tools",
    "apply_mode",
    "jira_create",
    "jira_sync"
  ],
  "expires_at": "2027-12-31T23:59:59Z",
  "signature": "example_signature",
  "holder": "My Company",
  "license_id": "license-001"
}
EOF
```

## 2. Enable Entitlement

```bash
# Via environment variable
export SPREADSHEET_MCP_ENTITLEMENT_ENABLED=true
export SPREADSHEET_MCP_ENTITLEMENT_PROVIDER=local
export SPREADSHEET_MCP_ENTITLEMENT_LICENSE_PATH=.ggen_license

# Or via CLI
cargo run -- \
  --workspace-root fixtures \
  --entitlement-enabled \
  --entitlement-provider local \
  --entitlement-license-path .ggen_license
```

## 3. Integration Example

Add to your MCP tool handler:

```rust
use crate::entitlement::{Capability, UsageUnit};
use chrono::Utc;
use std::collections::HashMap;

async fn sync_ggen_handler(state: Arc<AppState>, params: SyncParams) -> Result<Response> {
    // Check entitlement before apply
    if !params.preview {
        state.entitlement_gate()
            .require_capability(Capability::ApplyMode)
            .await?;

        // Report usage
        state.entitlement_gate().report_usage(UsageUnit {
            operation: "sync_ggen_apply".to_string(),
            timestamp: Utc::now(),
            workspace_hash: "workspace_hash_here".to_string(),
            user_id: None,
            metadata: HashMap::new(),
        }).await;
    }

    // Proceed with operation
    apply_changes(state, params).await
}
```

## 4. Test

```bash
# Preview mode (free) - should work
curl -X POST http://localhost:8079/mcp \
  -d '{"method":"sync_ggen","params":{"preview":true}}'

# Apply mode (paid) - requires entitlement
curl -X POST http://localhost:8079/mcp \
  -d '{"method":"sync_ggen","params":{"preview":false}}'
```

## 5. Disable (Default)

```bash
# Simply don't set SPREADSHEET_MCP_ENTITLEMENT_ENABLED
# Or explicitly disable:
export SPREADSHEET_MCP_ENTITLEMENT_ENABLED=false
```

---

## Quick Reference

### Capabilities

| Capability | Tier | Description |
|-----------|------|-------------|
| `preview_mode` | Free | Read-only, dry-run |
| `read_only_tools` | Free | Full read-only access |
| `apply_mode` | Paid | Write operations |
| `jira_create` | Paid | Create Jira tickets |
| `jira_sync` | Paid | Bidirectional Jira sync |
| `full_guard_suite` | Paid | Advanced validation |
| `multi_workspace` | Enterprise | Multiple workspaces |
| `team_collaboration` | Enterprise | Team features |
| `audit_reporting` | Enterprise | Compliance reporting |

### Providers

| Provider | Use Case | Config |
|---------|----------|--------|
| `disabled` | Default, all allowed | (none) |
| `local` | Local license file | `--entitlement-license-path` |
| `env` | Environment variable | `GGEN_LICENSE='{...}'` |
| `gcp` | GCP Marketplace | `--gcp-project-id` (stub) |

### Error Code

- `-32020` = `EntitlementRequired`
- Category: `entitlement_error`
- Retryable: No

---

See [ENTITLEMENT_SYSTEM.md](docs/ENTITLEMENT_SYSTEM.md) for full documentation.
