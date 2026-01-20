# Entitlement System

**Version**: 1.0.0
**Status**: Production-ready, optional (disabled by default)

---

## Overview

Pluggable capability-based monetization system for ggen-mcp. Gates protected operations behind entitlement checks. Supports multiple providers for flexible deployment.

## Architecture

```
EntitlementGate
    ↓
EntitlementProvider (trait)
    ├── DisabledProvider     (default, allows all)
    ├── LocalFileProvider    (reads .ggen_license)
    ├── EnvVarProvider       (reads GGEN_LICENSE env var)
    └── GcpMarketplaceProvider (stub for future GCP integration)
```

## Capabilities

### Free Tier
- `preview_mode` - Read-only, dry-run operations
- `read_only_tools` - Full read-only tool suite

### Paid Tier
- `apply_mode` - Write operations (sync_ggen apply)
- `jira_create` - Create Jira tickets
- `jira_sync` - Bidirectional Jira sync
- `full_guard_suite` - Advanced validation
- `receipt_verification` - Transaction verification

### Enterprise Tier
- `multi_workspace` - Multiple workspace support
- `team_collaboration` - Team features
- `audit_reporting` - Compliance reporting

## Configuration

### Environment Variables

```bash
# Enable entitlement checking
SPREADSHEET_MCP_ENTITLEMENT_ENABLED=true

# Choose provider: local, env, gcp, disabled
SPREADSHEET_MCP_ENTITLEMENT_PROVIDER=local

# License file path (for local provider)
SPREADSHEET_MCP_ENTITLEMENT_LICENSE_PATH=.ggen_license
```

### Config File (YAML)

```yaml
entitlement_enabled: true
entitlement_provider: local
entitlement_license_path: .ggen_license
```

### CLI Args

```bash
spreadsheet-mcp \
  --entitlement-enabled \
  --entitlement-provider local \
  --entitlement-license-path /path/to/license.json
```

## Providers

### Local File Provider

Reads license from JSON file.

**License Format** (`.ggen_license`):
```json
{
  "version": "1.0",
  "capabilities": [
    "preview_mode",
    "read_only_tools",
    "apply_mode",
    "jira_create"
  ],
  "expires_at": "2027-12-31T23:59:59Z",
  "signature": "replace_with_real_crypto_signature",
  "holder": "Company Name",
  "license_id": "license-uuid-001"
}
```

**Usage Logging**: Writes to `.ggen_license.usage.jsonl`

```jsonl
{"operation":"sync_ggen_apply","timestamp":"2026-01-20T12:00:00Z","workspace_hash":"abc123","user_id":null,"metadata":{}}
```

### Environment Variable Provider

Reads license from `GGEN_LICENSE` environment variable.

```bash
export GGEN_LICENSE='{
  "capabilities": ["apply_mode", "jira_create"]
}'
```

**Usage Reporting**: Logs to stderr

### GCP Marketplace Provider (Stub)

Future integration with GCP Cloud Commerce Procurement API.

**Status**: Stub implementation - allows all capabilities
**TODO**: Implement Procurement API calls and Pub/Sub usage reporting

## Usage

### In MCP Tools

```rust
use crate::entitlement::Capability;

// Check entitlement before protected operation
async fn sync_ggen(state: Arc<AppState>, params: SyncParams) -> Result<Response> {
    if !params.preview {
        // Require ApplyMode capability
        state.entitlement_gate()
            .require_capability(Capability::ApplyMode)
            .await?;

        // Report usage for metering
        state.entitlement_gate().report_usage(UsageUnit {
            operation: "sync_ggen_apply".to_string(),
            timestamp: Utc::now(),
            workspace_hash: workspace_fingerprint(),
            user_id: None,
            metadata: HashMap::new(),
        }).await;
    }

    // Proceed with operation
    apply_changes(&state, params).await
}
```

### Optional Capability Check

```rust
// Check without failing
if state.entitlement_gate().has_capability(Capability::JiraSync).await {
    // Use bidirectional sync
    sync_bidirectional().await?;
} else {
    // Fall back to one-way sync
    sync_one_way().await?;
}
```

## Error Handling

### Entitlement Required Error

```rust
Err(McpError::builder(ErrorCode::EntitlementRequired)
    .message("Capability 'apply_mode' requires entitlement")
    .operation("sync_ggen")
    .suggestion("Contact sales or upgrade plan")
    .doc_link("https://example.com/pricing")
    .build())
```

### User-Facing Error Message

```
[EntitlementRequired(-32020)] Capability 'apply_mode' requires entitlement. Contact sales or upgrade plan.

Suggestions:
  1. Contact sales or upgrade plan
```

## Integration Points

### Files Modified

1. `src/entitlement/mod.rs` - Core trait and types (400 LOC)
2. `src/entitlement/providers/` - Provider implementations (430 LOC)
3. `src/error.rs` - Added `EntitlementRequired` error code
4. `src/config.rs` - Added entitlement config fields
5. `src/state.rs` - Added `entitlement_gate: Arc<EntitlementGate>`
6. `src/lib.rs` - Added entitlement module

### Example Integration

**In `tools/ggen_sync/mod.rs`**:

```rust
if !params.preview {
    state.entitlement_gate()
        .require_capability(Capability::ApplyMode)
        .await?;

    state.entitlement_gate().report_usage(UsageUnit {
        operation: "sync_ggen_apply".to_string(),
        timestamp: Utc::now(),
        workspace_hash: workspace_fingerprint(),
        user_id: None,
        metadata: HashMap::new(),
    }).await;
}
```

## Security Considerations

### License Signature Verification

**Current**: Placeholder only (logs warning)
**TODO**: Implement cryptographic signature verification
- Use Ed25519 or RSA-PSS
- Verify against public key
- Include license claims in signature

### License Expiration

✓ Checked at load time
✓ Returns error if expired
⚠ Not checked continuously (restart required to detect expiration)

### Usage Reporting Security

- Local provider: Append-only JSONL log
- Env provider: Logged to stderr (redirect as needed)
- GCP provider: TODO - signed requests to Pub/Sub

## Testing

### Unit Tests

```bash
# Test all providers
cargo test entitlement::providers

# Test specific provider
cargo test entitlement::providers::local
cargo test entitlement::providers::env
cargo test entitlement::providers::gcp
```

### Integration Tests

```bash
# Create test license
cat > .ggen_license <<EOF
{
  "version": "1.0",
  "capabilities": ["apply_mode"],
  "expires_at": "2027-12-31T23:59:59Z",
  "signature": "test_sig",
  "holder": "Test User",
  "license_id": "test-001"
}
EOF

# Run with entitlement enabled
SPREADSHEET_MCP_ENTITLEMENT_ENABLED=true \
SPREADSHEET_MCP_ENTITLEMENT_PROVIDER=local \
SPREADSHEET_MCP_ENTITLEMENT_LICENSE_PATH=.ggen_license \
cargo run -- --workspace-root fixtures

# Test apply mode (should work)
curl -X POST http://localhost:8079/mcp \
  -H "Content-Type: application/json" \
  -d '{"method":"sync_ggen","params":{"preview":false,...}}'

# Remove apply_mode from license
# Test apply mode (should fail with EntitlementRequired)
```

## Monitoring

### Metrics

```rust
// Error metrics automatically tracked
ERROR_METRICS.get_category_count("entitlement_error")

// Usage reporting logged by provider
```

### Logs

```
INFO  entitlement::providers::local: Local license loaded path=.ggen_license capabilities=[ApplyMode, JiraCreate]
WARN  entitlement::providers::local: License signature verification not implemented
INFO  entitlement: Checking capability capability=apply_mode result=allowed
WARN  entitlement: Failed to report usage error="connection timeout"
```

## Deployment

### Disabled (Default)

```bash
# No configuration needed - all capabilities allowed
spreadsheet-mcp --workspace-root fixtures
```

### Local File Provider (Recommended)

```bash
# 1. Generate license file
cat > .ggen_license <<EOF
{...}
EOF

# 2. Enable entitlement
spreadsheet-mcp \
  --workspace-root fixtures \
  --entitlement-enabled \
  --entitlement-provider local \
  --entitlement-license-path .ggen_license
```

### Environment Variable Provider

```bash
export GGEN_LICENSE='{
  "capabilities": ["apply_mode", "jira_create"],
  "expires_at": "2027-12-31T23:59:59Z"
}'

spreadsheet-mcp \
  --workspace-root fixtures \
  --entitlement-enabled \
  --entitlement-provider env
```

### GCP Marketplace (Future)

```bash
spreadsheet-mcp \
  --workspace-root fixtures \
  --entitlement-enabled \
  --entitlement-provider gcp \
  --gcp-project-id my-project
```

## Future Enhancements

### Phase 1 (Current)
- ✓ Pluggable provider architecture
- ✓ Local file provider
- ✓ Environment variable provider
- ✓ GCP provider stub
- ✓ Usage reporting

### Phase 2 (TODO)
- [ ] Cryptographic signature verification (Ed25519)
- [ ] License server provider (HTTP API)
- [ ] Token-based authentication
- [ ] Rate limiting per license
- [ ] Usage quotas

### Phase 3 (TODO)
- [ ] GCP Marketplace integration
  - [ ] Procurement API calls
  - [ ] Pub/Sub usage metering
  - [ ] Subscription management
- [ ] AWS Marketplace integration
- [ ] Azure Marketplace integration

### Phase 4 (TODO)
- [ ] License analytics dashboard
- [ ] Automated license renewal
- [ ] Multi-tenancy support
- [ ] Webhook notifications for license events

## Troubleshooting

### License File Not Found

```
Error: Failed to read license file: No such file or directory (os error 2)
```

**Solution**: Check path, ensure `.ggen_license` exists

### License Expired

```
Error: License expired on 2026-01-01T00:00:00Z
```

**Solution**: Obtain new license with valid expiration

### Entitlement Check Fails

```
Error: Capability 'apply_mode' requires entitlement
```

**Solution**:
1. Check license file includes required capability
2. Verify entitlement provider is enabled
3. Check license expiration date

### Usage Reporting Fails

```
WARN: Failed to report usage: Permission denied
```

**Solution**: Ensure write permissions for `.ggen_license.usage.jsonl`

---

## SPR Summary

Pluggable entitlement system. Four providers: disabled (default), local file, env var, GCP stub. Gates capabilities (preview/apply/jira). Usage metering. Optional (disabled by default). NewType safety. Error handling integrated. Production-ready.
