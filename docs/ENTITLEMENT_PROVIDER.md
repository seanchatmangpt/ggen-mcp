# Entitlement Provider

**Version**: 2.1.0 | Capability Gating | Pluggable Licensing | Usage Reporting

---

## Overview

**Entitlement Provider**: Pluggable system for capability-based licensing. Gates access to advanced features (preview/apply modes, Jira integration, cryptographic receipts).

**Principle**: Freemium model. Core features free. Advanced features gated by entitlement.

**Providers**:
1. **Local File**: Read license from `.ggen_license` file
2. **Environment Variable**: Read license from `GGEN_LICENSE` env var
3. **GCP Marketplace**: Integrate with GCP billing (future: v2.2)

**Use Cases**:
- **Monetization**: Free tier → Paid tier → Enterprise tier
- **Feature Flags**: Gradual rollout of new features
- **Usage Tracking**: Monitor API usage per license
- **Audit Compliance**: Enforce licensing policies

---

## Capabilities

### Free Tier (Default)

**Capabilities**:
- `PreviewMode`: Generate code in preview mode (no writes)
- `ReadOnlyTools`: MCP read-only tools (list_workbooks, read_table, etc.)
- `BasicValidation`: Input validation, schema checks

**Limitations**:
- ❌ No `ApplyMode` (cannot write files)
- ❌ No `JiraIntegration`
- ❌ No `CryptographicReceipts`
- ❌ Single workspace only

**Configuration**:
```toml
# ggen.toml
[entitlement]
enabled = false  # Free tier (default)
```

---

### Paid Tier

**Capabilities**:
- **Free Tier** (all)
- `ApplyMode`: Write files (preview → apply workflow)
- `JiraCreate`: Create Jira tickets from generated code
- `FullGuardSuite`: All 7 guards (G1-G7)
- `CryptographicReceipts`: SHA-256 receipts for audit

**Limitations**:
- ❌ No `JiraSync` (bidirectional sync)
- ❌ No `SignedReceipts` (ECDSA signatures)
- ❌ No `MultiWorkspace` (single workspace)
- ✅ Usage limit: 1,000 syncs/month

**License File** (`.ggen_license`):
```json
{
  "version": "1.0.0",
  "tier": "paid",
  "capabilities": [
    "PreviewMode",
    "ApplyMode",
    "ReadOnlyTools",
    "JiraCreate",
    "FullGuardSuite",
    "CryptographicReceipts"
  ],
  "limits": {
    "syncs_per_month": 1000,
    "workspaces": 1
  },
  "expires_at": "2027-01-20T00:00:00Z",
  "licensee": "Acme Corp",
  "email": "devops@acme.com",
  "signature": "3045022100d9e8f7..."
}
```

---

### Enterprise Tier

**Capabilities**:
- **Paid Tier** (all)
- `JiraSync`: Bidirectional Jira ↔ Spreadsheet sync
- `SignedReceipts`: ECDSA-signed receipts (tamper-proof)
- `MultiWorkspace`: Multiple workspace roots
- `TeamCollaboration`: Shared licenses across team
- `AuditReporting`: PDF audit reports (SOC2-compliant)
- `CustomGuards`: Plugin custom guard logic
- `PrioritySupport`: Dedicated support channel

**Limitations**:
- ✅ Unlimited syncs
- ✅ Unlimited workspaces

**License File** (`.ggen_license`):
```json
{
  "version": "1.0.0",
  "tier": "enterprise",
  "capabilities": [
    "PreviewMode",
    "ApplyMode",
    "ReadOnlyTools",
    "JiraCreate",
    "JiraSync",
    "FullGuardSuite",
    "CryptographicReceipts",
    "SignedReceipts",
    "MultiWorkspace",
    "TeamCollaboration",
    "AuditReporting",
    "CustomGuards",
    "PrioritySupport"
  ],
  "limits": {
    "syncs_per_month": null,  // Unlimited
    "workspaces": null        // Unlimited
  },
  "expires_at": "2027-01-20T00:00:00Z",
  "licensee": "Acme Corp",
  "team_members": 50,
  "email": "devops@acme.com",
  "signature": "3045022100d9e8f7..."
}
```

---

## Providers

### Provider 1: Local File

**Configuration** (ggen.toml):
```toml
[entitlement]
enabled = true
provider = "local"
license_path = ".ggen_license"  # Default: .ggen_license
grace_period_days = 30  # Allow 30 days after expiry
```

**License File Format**:
```json
{
  "version": "1.0.0",
  "tier": "paid",
  "capabilities": ["PreviewMode", "ApplyMode", "JiraCreate"],
  "limits": {
    "syncs_per_month": 1000
  },
  "expires_at": "2027-01-20T00:00:00Z",
  "licensee": "Acme Corp",
  "email": "devops@acme.com",
  "signature": "3045022100..."  // ECDSA signature (optional in v2.1)
}
```

**Verification**:
```rust
fn verify_local_license(path: &Path) -> Result<License> {
    // 1. Read file
    let content = fs::read_to_string(path)?;
    let license: License = serde_json::from_str(&content)?;

    // 2. Check version
    if license.version != "1.0.0" {
        return Err("Unsupported license version");
    }

    // 3. Check expiry (with grace period)
    let expires_at = DateTime::parse_from_rfc3339(&license.expires_at)?;
    let grace_period = Duration::days(30);
    let cutoff = expires_at + grace_period;

    if Utc::now() > cutoff {
        return Err("License expired");
    }

    // 4. Verify signature (optional)
    if let Some(signature) = &license.signature {
        verify_signature(&license, signature)?;
    }

    Ok(license)
}
```

---

### Provider 2: Environment Variable

**Configuration** (ggen.toml):
```toml
[entitlement]
enabled = true
provider = "env"
env_var_name = "GGEN_LICENSE"  # Default: GGEN_LICENSE
```

**Environment Variable** (base64-encoded JSON):
```bash
export GGEN_LICENSE="eyJ2ZXJzaW9uIjoiMS4wLjAiLCJ0aWVyIjoicGFpZCIsImNhcGFiaWxpdGllcyI6WyJQcmV2aWV3TW9kZSIsIkFwcGx5TW9kZSIsIkppcmFDcmVhdGUiXSwibGltaXRzIjp7InN5bmNzX3Blcl9tb250aCI6MTAwMH0sImV4cGlyZXNfYXQiOiIyMDI3LTAxLTIwVDAwOjAwOjAwWiIsImxpY2Vuc2VlIjoiQWNtZSBDb3JwIiwiZW1haWwiOiJkZXZvcHNAYWNtZS5jb20iLCJzaWduYXR1cmUiOiIzMDQ1MDIyMTAwZDllOGY3Li4uIn0="
```

**Decode**:
```bash
echo "$GGEN_LICENSE" | base64 -d | jq .
```

**Verification**:
```rust
fn verify_env_license(env_var_name: &str) -> Result<License> {
    // 1. Read env var
    let encoded = env::var(env_var_name)
        .map_err(|_| format!("Env var {} not set", env_var_name))?;

    // 2. Decode base64
    let decoded = base64::decode(&encoded)?;

    // 3. Parse JSON
    let license: License = serde_json::from_slice(&decoded)?;

    // 4. Verify (same as local file)
    // ...

    Ok(license)
}
```

---

### Provider 3: GCP Marketplace (Future: v2.2)

**Configuration** (ggen.toml):
```toml
[entitlement]
enabled = true
provider = "gcp_marketplace"
project_id = "my-gcp-project"
entitlement_name = "ggen-pro"
```

**Workflow**:
1. User subscribes to "ggen-pro" in GCP Marketplace
2. GCP provisions entitlement (linked to project)
3. ggen queries GCP API for entitlement status
4. Capabilities granted based on subscription tier

**API Integration**:
```rust
async fn verify_gcp_entitlement(
    project_id: &str,
    entitlement_name: &str
) -> Result<License> {
    // 1. Get GCP credentials
    let token = get_access_token().await?;

    // 2. Query entitlement API
    let url = format!(
        "https://cloudcommerceprocurement.googleapis.com/v1/projects/{}/entitlements/{}",
        project_id, entitlement_name
    );

    let response = reqwest::Client::new()
        .get(&url)
        .bearer_auth(&token)
        .send()
        .await?;

    let entitlement: GcpEntitlement = response.json().await?;

    // 3. Map entitlement → capabilities
    let capabilities = match entitlement.plan.as_str() {
        "ggen-free" => vec!["PreviewMode", "ReadOnlyTools"],
        "ggen-pro" => vec!["PreviewMode", "ApplyMode", "JiraCreate"],
        "ggen-enterprise" => vec!["PreviewMode", "ApplyMode", "JiraCreate", "JiraSync", "MultiWorkspace"],
        _ => return Err("Unknown plan"),
    };

    Ok(License {
        tier: entitlement.plan.clone(),
        capabilities,
        expires_at: entitlement.end_time,
        // ...
    })
}
```

**Status**: Planned for v2.2 (not implemented in v2.1).

---

## Capability Checks

### Check at Runtime

```rust
fn check_capability(capability: &str, license: &License) -> Result<()> {
    if !license.capabilities.contains(&capability.to_string()) {
        return Err(format!(
            "Capability '{}' not available in your license.\n\
             Current tier: {}\n\
             Upgrade at: https://ggen.dev/pricing",
            capability, license.tier
        ));
    }

    Ok(())
}
```

### Example: ApplyMode Check

```rust
// In sync_ggen handler
if !params.preview {
    // User requested apply mode
    check_capability("ApplyMode", &license)?;
}

// If check passes, proceed with apply mode
// If check fails, return EntitlementError
```

---

## Error Responses

### Free Tier Denied

**Request**:
```json
{
  "tool": "sync_ggen",
  "params": {
    "workspace_root": ".",
    "preview": false  // ❌ Denied: ApplyMode requires paid tier
  }
}
```

**Response**:
```json
{
  "error": "EntitlementError",
  "message": "Capability 'ApplyMode' not available in your license.",
  "details": {
    "capability": "ApplyMode",
    "current_tier": "free",
    "required_tier": "paid",
    "upgrade_url": "https://ggen.dev/pricing"
  }
}
```

---

### License Expired

**Request**:
```json
{
  "tool": "sync_ggen",
  "params": {
    "workspace_root": ".",
    "preview": false
  }
}
```

**Response**:
```json
{
  "error": "EntitlementError",
  "message": "License expired on 2026-01-20. Grace period ended 2026-02-19.",
  "details": {
    "expires_at": "2026-01-20T00:00:00Z",
    "grace_period_days": 30,
    "renew_url": "https://ggen.dev/renew"
  }
}
```

---

### Usage Limit Exceeded

**Request**:
```json
{
  "tool": "sync_ggen",
  "params": {
    "workspace_root": ".",
    "preview": false
  }
}
```

**Response**:
```json
{
  "error": "EntitlementError",
  "message": "Monthly sync limit exceeded (1000/1000).",
  "details": {
    "syncs_this_month": 1000,
    "limit": 1000,
    "resets_at": "2026-02-01T00:00:00Z",
    "upgrade_url": "https://ggen.dev/pricing"
  }
}
```

---

## Usage Reporting

### Track Usage

```rust
struct UsageTracker {
    license: License,
    usage_file: PathBuf,
}

impl UsageTracker {
    fn record_sync(&mut self) -> Result<()> {
        // 1. Load usage data
        let mut usage: UsageData = self.load_usage()?;

        // 2. Increment counter
        usage.syncs_this_month += 1;

        // 3. Check limit
        if let Some(limit) = self.license.limits.syncs_per_month {
            if usage.syncs_this_month > limit {
                return Err("Monthly sync limit exceeded");
            }
        }

        // 4. Save usage data
        self.save_usage(&usage)?;

        Ok(())
    }

    fn reset_monthly(&mut self) -> Result<()> {
        let mut usage: UsageData = self.load_usage()?;

        // Reset counter on first of month
        if Utc::now().day() == 1 {
            usage.syncs_this_month = 0;
            self.save_usage(&usage)?;
        }

        Ok(())
    }
}
```

---

### Usage Report (First Light Report)

**Section**: Usage Report

```markdown
## Usage Report (Entitlement)

**License**:
- **Tier**: Paid
- **Licensee**: Acme Corp
- **Email**: devops@acme.com
- **Expires**: 2027-01-20 (353 days remaining)

**Capabilities Enabled**:
- ✅ PreviewMode
- ✅ ApplyMode
- ✅ JiraCreate
- ✅ FullGuardSuite
- ✅ CryptographicReceipts

**Usage This Month** (January 2026):
- **Syncs**: 127 / 1,000 (12.7%)
- **Remaining**: 873 syncs
- **Resets**: 2026-02-01

**Capabilities Used This Sync**:
- ApplyMode (write files)
- JiraCreate (13 tickets created)
- CryptographicReceipts (receipt: 7f83b165)
```

---

## License Management

### Generate License (Vendor)

```bash
# Generate signed license (vendor-side)
ggen-admin generate-license \
    --tier paid \
    --capabilities "PreviewMode,ApplyMode,JiraCreate" \
    --limits "syncs_per_month:1000" \
    --expires "2027-01-20" \
    --licensee "Acme Corp" \
    --email "devops@acme.com" \
    --sign-with ./vendor-private-key.pem \
    --output acme-corp.ggen_license
```

**Output** (`acme-corp.ggen_license`):
```json
{
  "version": "1.0.0",
  "tier": "paid",
  "capabilities": ["PreviewMode", "ApplyMode", "JiraCreate"],
  "limits": { "syncs_per_month": 1000 },
  "expires_at": "2027-01-20T00:00:00Z",
  "licensee": "Acme Corp",
  "email": "devops@acme.com",
  "signature": "3045022100d9e8f7..."
}
```

---

### Install License (Customer)

```bash
# Option 1: Local file
cp acme-corp.ggen_license .ggen_license

# Option 2: Environment variable
export GGEN_LICENSE=$(cat acme-corp.ggen_license | base64)

# Verify license
ggen verify-license
```

**Output**:
```
✅ License valid
Tier: Paid
Expires: 2027-01-20 (353 days remaining)
Capabilities: PreviewMode, ApplyMode, JiraCreate, FullGuardSuite, CryptographicReceipts
```

---

### Renew License

```bash
# Automatic renewal (30-day grace period)
# Old license expires: 2026-01-20
# Grace period ends: 2026-02-19

# Install new license before grace period ends
cp acme-corp-renewed.ggen_license .ggen_license

# Verify
ggen verify-license
```

---

## Configuration Reference

### ggen.toml

```toml
[entitlement]
enabled = true                     # Enable entitlement checks
provider = "local"                 # local | env | gcp_marketplace
license_path = ".ggen_license"     # For local provider
env_var_name = "GGEN_LICENSE"      # For env provider
grace_period_days = 30             # Allow 30 days after expiry
usage_tracking = true              # Track usage metrics
usage_file = ".ggen_usage.json"    # Usage data file

[entitlement.gcp]                  # For gcp_marketplace provider
project_id = "my-gcp-project"
entitlement_name = "ggen-pro"
```

---

## Best Practices

### 1. Commit License (Local Provider)
```bash
# Add license to version control
git add .ggen_license
git commit -m "chore: Add ggen license"
```

### 2. Secure License (Env Provider)
```bash
# Use CI secrets for env var
# GitHub Actions
- name: Set license
  env:
    GGEN_LICENSE: ${{ secrets.GGEN_LICENSE }}
  run: ggen sync
```

### 3. Monitor Expiry
```bash
# Check license status daily (cron)
0 9 * * * ggen verify-license --warn-before 30 || mail -s "ggen license expiring" admin@example.com
```

### 4. Track Usage
```bash
# Export usage report monthly
ggen usage-report --month 2026-01 --output usage-2026-01.json
```

### 5. Renew Before Expiry
```bash
# Renew 60 days before expiry (avoid grace period)
ggen verify-license --warn-before 60
```

---

## Troubleshooting

### Issue: License Not Found

**Symptom**:
```
Error: License file '.ggen_license' not found.
```

**Solution**:
1. Verify file exists: `ls -la .ggen_license`
2. Check path in ggen.toml: `[entitlement] license_path = ".ggen_license"`
3. Or use env provider: `export GGEN_LICENSE="..."`

---

### Issue: License Expired

**Symptom**:
```
Error: License expired on 2026-01-20. Grace period ended 2026-02-19.
```

**Solution**:
1. Contact vendor for renewal
2. Install new license: `cp renewed.ggen_license .ggen_license`
3. Verify: `ggen verify-license`

---

### Issue: Capability Denied

**Symptom**:
```
Error: Capability 'ApplyMode' not available in your license.
Current tier: free
```

**Solution**:
1. Check current tier: `ggen verify-license`
2. Upgrade license: https://ggen.dev/pricing
3. Install new license: `cp paid.ggen_license .ggen_license`

---

### Issue: Usage Limit Exceeded

**Symptom**:
```
Error: Monthly sync limit exceeded (1000/1000).
Resets at: 2026-02-01
```

**Solution** (Option 1: Wait for reset):
```bash
# Wait until next month (auto-reset)
```

**Solution** (Option 2: Upgrade tier):
```bash
# Upgrade to higher tier (e.g., enterprise = unlimited)
# Install new license
cp enterprise.ggen_license .ggen_license
```

---

### Issue: Signature Verification Failed

**Symptom**:
```
Error: License signature verification failed.
```

**Solution**:
1. License may be corrupted or tampered
2. Download fresh license from vendor
3. Verify vendor public key is correct

---

## Pricing (Example)

| Tier | Price | Capabilities | Limits |
|------|-------|--------------|--------|
| **Free** | $0 | PreviewMode, ReadOnlyTools | Preview only, no writes |
| **Paid** | $49/month | + ApplyMode, JiraCreate, Receipts | 1,000 syncs/month, 1 workspace |
| **Enterprise** | $499/month | + JiraSync, SignedReceipts, MultiWorkspace | Unlimited syncs, unlimited workspaces |

**Volume Discounts**:
- 10-49 users: 10% off
- 50-99 users: 20% off
- 100+ users: Contact sales

**Annual Billing**:
- Pay annually: 2 months free (16.67% discount)

---

## References

- **Proof-First Compiler**: [docs/PROOF_FIRST_COMPILER.md](./PROOF_FIRST_COMPILER.md)
- **Guard Kernel**: [docs/GUARD_KERNEL.md](./GUARD_KERNEL.md)
- **Receipt Verification**: [docs/RECEIPT_VERIFICATION.md](./RECEIPT_VERIFICATION.md)
- **Migration Guide**: [MIGRATION_GUIDE_V2.1.md](../MIGRATION_GUIDE_V2.1.md)

---

**End of ENTITLEMENT_PROVIDER.md**
