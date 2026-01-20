# Migration Guide: v2.0 → v2.1 (Proof-First Compiler)

**Version**: 2.1.0 | Proof-First Edition | Breaking Changes | Migration Path
**Date**: 2026-01-20

---

## Overview

ggen-mcp v2.1 introduces **proof-first code generation**: cryptographic receipts, guard kernel, and preview-by-default workflow. This guide covers migration from v2.0.

**Key Changes**:
1. **Preview by default** (breaking): Was apply-by-default in v2.0
2. **Guard kernel** (new): 7 safety checks run before generation
3. **Cryptographic receipts** (new): SHA-256 proof-carrying code
4. **First Light reports** (new): 1-page markdown/JSON summaries
5. **Receipt verification** (new): Standalone verification tool
6. **Jira compiler stage** (new): Optional Jira integration in pipeline
7. **Entitlement provider** (new): Capability-based licensing

**Backward Compatibility**: 90% compatible. See breaking changes below.

---

## Breaking Changes

### Breaking Change 1: Preview by Default

**Impact**: `sync_ggen` now previews by default (was apply in v2.0).

#### BEFORE (v2.0)
```json
{
  "tool": "sync_ggen",
  "params": {
    "workspace_root": "."
  }
}
// Applied changes immediately (wrote files)
```

#### AFTER (v2.1)
```json
{
  "tool": "sync_ggen",
  "params": {
    "workspace_root": "."
  }
}
// Previews changes only (no writes)

// To apply, explicitly set preview: false
{
  "tool": "sync_ggen",
  "params": {
    "workspace_root": ".",
    "preview": false  // ⚠️ Required for writes in v2.1
  }
}
```

**Migration**: Add `preview: false` to existing scripts if you want immediate writes.

**Rationale**: Prevent accidental overwrites. Explicit opt-in for writes = safer workflow.

---

### Breaking Change 2: New Output Directory

**Impact**: Reports/receipts/diffs now written to `./ggen.out/` (was no structured output in v2.0).

#### BEFORE (v2.0)
- No structured output directory
- No reports generated
- No receipts emitted

#### AFTER (v2.1)
```
./ggen.out/
├── reports/
│   ├── 2026-01-20-163000.md     # First Light Report (markdown)
│   ├── 2026-01-20-163000.json   # First Light Report (JSON)
│   └── latest.md                # Symlink to latest report
├── receipts/
│   ├── 7f83b165.json            # Cryptographic receipt
│   └── latest.json              # Symlink to latest receipt
└── diffs/
    ├── 7f83b165.patch           # Unified diff
    └── latest.patch             # Symlink to latest diff
```

**Migration**: Update `.gitignore` if you want to exclude `ggen.out/`.

```bash
# .gitignore
ggen.out/reports/*.md  # Exclude markdown reports
ggen.out/diffs/*.patch # Exclude diffs

# But commit receipts for audit
# ggen.out/receipts/*.json (keep tracked)
```

---

### Breaking Change 3: Guards Run by Default

**Impact**: 7 safety checks now run before generation (was opt-in in v2.0).

#### BEFORE (v2.0)
- No guard checks (unless explicitly enabled)
- Compilation could proceed with unsafe configurations

#### AFTER (v2.1)
- **Guards always run** (G1-G7)
- Compilation **stops on guard failure** (fail-fast)
- Use `force: true` to bypass (not recommended)

**Migration**: Fix guard failures before applying.

```bash
# Run sync to see guard verdicts
ggen sync

# Check report for guard failures
cat ./ggen.out/reports/latest.md | grep -A 10 "Guard Verdicts"

# Fix issues (see Guard Kernel docs)
# Re-run until all guards pass
ggen sync
```

**Disable guards** (not recommended):
```toml
# ggen.toml
[guards]
enabled = []  # ⚠️ Disables all guards (unsafe)
```

---

## New Features (Non-Breaking)

### Feature 1: First Light Report

**Automatically generated on every sync**. No action required.

**Location**: `./ggen.out/reports/latest.md`

**Example**:
```bash
ggen sync
cat ./ggen.out/reports/latest.md
```

**Customize**:
```toml
# ggen.toml
[output]
report_format = "json"  # markdown (default) | json | none
```

---

### Feature 2: Cryptographic Receipts

**Automatically generated on every sync**. No action required.

**Location**: `./ggen.out/receipts/latest.json`

**Verify**:
```bash
ggen verify ./ggen.out/receipts/latest.json
```

**Disable** (not recommended for production):
```toml
# ggen.toml
[output]
emit_receipt = false  # ⚠️ Disables receipts
```

---

### Feature 3: Guard Kernel

**7 safety checks** (G1-G7) run before generation.

**Checks**:
1. G1: Path Safety
2. G2: Output Overlap
3. G3: Template Compilation
4. G4: Turtle Parse
5. G5: SPARQL Execution
6. G6: Determinism
7. G7: Bounds

**View verdicts**:
```bash
cat ./ggen.out/reports/latest.md | grep -A 10 "Guard Verdicts"
```

**Customize**:
```toml
# ggen.toml
[guards]
enabled = ["G1", "G2", "G3"]  # Only run first 3 guards
fail_fast = false             # Run all guards, report all failures
```

---

### Feature 4: Receipt Verification

**New tool**: `verify_receipt`

**Usage**:
```bash
# Verify specific receipt
ggen verify ./ggen.out/receipts/7f83b165.json

# Verify all receipts
ggen verify ./ggen.out/receipts/*.json
```

**MCP API**:
```json
{
  "tool": "verify_receipt",
  "params": {
    "receipt_path": "./ggen.out/receipts/7f83b165.json"
  }
}
```

---

### Feature 5: Jira Compiler Stage (Optional)

**Opt-in via ggen.toml**:
```toml
[jira]
enabled = true
mode = "dry_run"  # dry_run | create | sync
project_key = "PROJ"
base_url = "https://company.atlassian.net"
auth_token_env = "JIRA_TOKEN"
```

**Usage**:
```bash
export JIRA_TOKEN="your-jira-api-token"
ggen sync
```

**Result**: Jira tickets created/synced automatically during compilation.

---

### Feature 6: Entitlement Provider (Optional)

**Opt-in via ggen.toml**:
```toml
[entitlement]
enabled = true
provider = "local"
license_path = ".ggen_license"
```

**Free tier** (default): Preview mode only.

**Paid tier**: Apply mode + Jira + receipts.

**Enterprise tier**: Unlimited + signatures + multi-workspace.

---

## Migration Path

### Step 1: Update ggen

```bash
# Backup current version
cargo install ggen-mcp --version 2.0.0 --root ./backup/

# Install v2.1
cargo install ggen-mcp --version 2.1.0
ggen --version
# Output: ggen-v2.1.0
```

---

### Step 2: Test Preview Mode

```bash
# Run sync in preview mode (new default)
ggen sync

# Review report
cat ./ggen.out/reports/latest.md

# Review diff
less ./ggen.out/diffs/latest.patch

# Verify guards passed
grep "Guard Verdicts" ./ggen.out/reports/latest.md
```

**Expected Output**:
```markdown
## Guard Verdicts (Stage 2)

| Guard | Verdict | Diagnostic |
|-------|---------|------------|
| G1: Path Safety | ✅ PASS | All paths validated |
| G2: Output Overlap | ✅ PASS | No duplicate outputs |
| ... | ... | ... |

**Result**: ✅ All guards passed. Safe to apply.
```

---

### Step 3: Update Scripts (If Needed)

**If you have CI/CD scripts that rely on immediate writes**:

#### BEFORE (v2.0)
```bash
# CI/CD script (v2.0)
ggen sync  # Applied changes immediately
git add src/generated/
git commit -m "chore: Update generated code"
```

#### AFTER (v2.1)
```bash
# CI/CD script (v2.1)
ggen sync --no-preview  # Explicit apply (or use: ggen sync --preview=false via API)
git add src/generated/ ./ggen.out/receipts/
git commit -m "chore: Update generated code (receipt: $(basename ./ggen.out/receipts/latest.json .json))"
```

**API equivalent**:
```json
{
  "tool": "sync_ggen",
  "params": {
    "workspace_root": ".",
    "preview": false  // ⚠️ Add this to restore v2.0 behavior
  }
}
```

---

### Step 4: Enable Optional Features

#### Feature: Jira Integration

```toml
# ggen.toml
[jira]
enabled = true
mode = "dry_run"  # Start with dry_run to preview tickets
project_key = "PROJ"
base_url = "https://company.atlassian.net"
auth_token_env = "JIRA_TOKEN"
```

```bash
export JIRA_TOKEN="your-jira-api-token"
ggen sync
cat ./ggen.out/reports/latest.md | grep -A 20 "Jira"
```

#### Feature: Entitlement Gating

```toml
# ggen.toml
[entitlement]
enabled = false  # Disabled by default (free tier)
```

**Enable paid tier**:
```bash
# Install license
cp your-license.ggen_license .ggen_license

# Enable entitlement
[entitlement]
enabled = true
provider = "local"
license_path = ".ggen_license"

# Verify
ggen verify-license
```

---

### Step 5: Integrate Receipt Verification (Optional)

```bash
# Add to CI pipeline
ggen sync --no-preview
ggen verify ./ggen.out/receipts/*.json || exit 1
```

**GitHub Actions**:
```yaml
- name: Generate code
  run: ggen sync --no-preview

- name: Verify receipt
  run: ggen verify ./ggen.out/receipts/*.json

- name: Block deployment if verification fails
  if: failure()
  run: |
    echo "Receipt verification failed. Deployment blocked."
    exit 1
```

---

### Step 6: Commit Receipts with Code

```bash
# Update .gitignore
echo "ggen.out/reports/*.md" >> .gitignore
echo "ggen.out/diffs/*.patch" >> .gitignore
# But DO commit receipts:
# ggen.out/receipts/*.json (tracked)

# Commit generated code + receipts
ggen sync --no-preview
git add src/generated/ ./ggen.out/receipts/
git commit -m "feat: Generate code from ontology (receipt: $(basename ./ggen.out/receipts/latest.json .json))"
git push
```

---

## Configuration Changes

### ggen.toml (v2.1 Extensions)

```toml
# Output configuration (new in v2.1)
[output]
report_format = "markdown"  # markdown | json | none
emit_receipt = true
emit_diff = true
output_dir = "./ggen.out"

# Guard configuration (new in v2.1)
[guards]
enabled = ["G1", "G2", "G3", "G4", "G5", "G6", "G7"]
fail_fast = true

# Limits (G7 bounds check, new in v2.1)
[limits]
max_output_files = 1000
max_output_bytes = 104857600  # 100MB

# Jira integration (optional, new in v2.1)
[jira]
enabled = false
mode = "dry_run"
project_key = "PROJ"
base_url = "https://company.atlassian.net"
auth_token_env = "JIRA_TOKEN"

# Entitlement (optional, new in v2.1)
[entitlement]
enabled = false
provider = "local"
license_path = ".ggen_license"
grace_period_days = 30
```

---

## Testing Your Migration

### Test 1: Preview → Apply Workflow

```bash
# 1. Preview
ggen sync

# 2. Verify no files written
git status
# Should be clean (no changes in src/generated/)

# 3. Apply
ggen sync --no-preview

# 4. Verify files written
git status
# Should show changes in src/generated/

# 5. Verify receipt
ggen verify ./ggen.out/receipts/*.json
```

---

### Test 2: Guard Failures

```bash
# Introduce a guard failure (G2: Output Overlap)
# Edit ggen.toml: duplicate output_path

[[generate]]
name = "rule1"
output_path = "src/generated/entity.rs"

[[generate]]
name = "rule2"
output_path = "src/generated/entity.rs"  # Duplicate

# Run sync
ggen sync

# Expected: G2 failure
cat ./ggen.out/reports/latest.md | grep -A 10 "Guard Verdicts"
# Output:
# | G2: Output Overlap | ❌ FAIL | Duplicate output detected |

# Fix: Remove duplicate
# Re-run
ggen sync
```

---

### Test 3: Receipt Verification

```bash
# Generate code
ggen sync --no-preview

# Verify receipt
ggen verify ./ggen.out/receipts/*.json

# Expected: ✅ VERIFIED

# Manually edit generated file
echo "// Manual edit" >> src/generated/entity.rs

# Re-verify receipt
ggen verify ./ggen.out/receipts/*.json

# Expected: ❌ FAILED (V4: Output file hash mismatch)
```

---

### Test 4: Jira Integration (If Enabled)

```bash
# Enable Jira dry run
[jira]
enabled = true
mode = "dry_run"

export JIRA_TOKEN="your-token"

# Run sync
ggen sync

# Check report for Jira section
cat ./ggen.out/reports/latest.md | grep -A 20 "Jira"

# Expected: "Tickets to Create: 13" (dry run, not created)

# Switch to create mode
[jira]
mode = "create"

# Run sync again
ggen sync --no-preview

# Expected: "Tickets Created: 13"
```

---

## Backward Compatibility

### What's Compatible

✅ **ggen.toml format** (v2.0 configs work in v2.1)
✅ **SPARQL queries** (no syntax changes)
✅ **Tera templates** (no syntax changes)
✅ **Ontology format** (Turtle/RDF unchanged)
✅ **CLI flags** (all v2.0 flags still work)
✅ **MCP tool names** (sync_ggen, etc. unchanged)

### What's NOT Compatible

❌ **Default behavior** (preview vs. apply)
❌ **Output directory** (no structured output in v2.0)
❌ **Guard checks** (didn't exist in v2.0)

**Workaround**: Set `preview: false` to restore v2.0 behavior.

---

## Grace Period

**v2.1 → v2.2**: 6 months (until 2026-07-20)
**v2.2 → v2.3**: 3 months (preview-by-default becomes permanent)

**During grace period**:
- Legacy behavior available via `preview: false`
- Deprecation warnings emitted
- Migration guide updated

**After grace period** (v2.3+):
- Preview-by-default mandatory
- `preview: false` still works (explicit opt-in)
- No more deprecation warnings

---

## Troubleshooting

### Issue 1: "Guard Failure" Error

**Symptom**:
```
Error: Guard G2 (Output Overlap) failed.
Compilation stopped.
```

**Solution**:
1. Review guard failure in report: `cat ./ggen.out/reports/latest.md`
2. Fix issue (see Guard Kernel docs)
3. Re-run: `ggen sync`

**Bypass** (not recommended):
```bash
ggen sync --force  # ⚠️ Bypasses guards
```

---

### Issue 2: Files Not Written

**Symptom**: `ggen sync` completes but files not written.

**Cause**: Preview mode (default in v2.1).

**Solution**:
```bash
# Explicitly apply
ggen sync --no-preview

# Or via API
{
  "tool": "sync_ggen",
  "params": {
    "workspace_root": ".",
    "preview": false
  }
}
```

---

### Issue 3: Receipt Verification Fails

**Symptom**:
```
Error: V4 Output File Hashes
Hash mismatch: src/generated/entity.rs
```

**Cause**: Generated file manually edited after generation.

**Solution** (Option 1: Discard edits):
```bash
git checkout src/generated/entity.rs
ggen verify ./ggen.out/receipts/*.json
```

**Solution** (Option 2: Regenerate):
```bash
# Move edits to template
# Regenerate
ggen sync --no-preview
```

---

### Issue 4: Jira Integration Error

**Symptom**:
```
Error: JiraApiError: Authentication failed (401)
```

**Solution**:
1. Verify `JIRA_TOKEN` env var set: `echo $JIRA_TOKEN`
2. Verify token has correct permissions (create issues)
3. Verify `base_url` in ggen.toml is correct

---

## Migration Checklist

- [ ] **Backup** current v2.0 installation
- [ ] **Install** ggen v2.1
- [ ] **Test** preview mode (default behavior)
- [ ] **Update** scripts to use `preview: false` (if needed)
- [ ] **Review** guard verdicts (fix failures)
- [ ] **Enable** optional features (Jira, entitlement)
- [ ] **Integrate** receipt verification (CI/CD)
- [ ] **Update** .gitignore (exclude reports/diffs, commit receipts)
- [ ] **Commit** receipts with generated code
- [ ] **Update** documentation (reference new features)
- [ ] **Train** team (preview → apply workflow)

---

## Support

**Questions**: Open issue at https://github.com/example/ggen-mcp/issues
**Documentation**:
- [PROOF_FIRST_COMPILER.md](docs/PROOF_FIRST_COMPILER.md)
- [GUARD_KERNEL.md](docs/GUARD_KERNEL.md)
- [FIRST_LIGHT_REPORT.md](docs/FIRST_LIGHT_REPORT.md)
- [RECEIPT_VERIFICATION.md](docs/RECEIPT_VERIFICATION.md)
- [ENTITLEMENT_PROVIDER.md](docs/ENTITLEMENT_PROVIDER.md)

**Slack**: #ggen-mcp-migration

---

## Changelog (v2.0 → v2.1)

### Added
- ✅ Preview-by-default workflow
- ✅ Guard Kernel (7 safety checks: G1-G7)
- ✅ Cryptographic receipts (SHA-256)
- ✅ First Light reports (markdown/JSON)
- ✅ Receipt verification tool (7 checks: V1-V7)
- ✅ Jira compiler stage (dry_run/create/sync modes)
- ✅ Entitlement provider (free/paid/enterprise tiers)
- ✅ Unified diff generation (preview mode)
- ✅ Workspace fingerprinting (SHA-256)
- ✅ Usage tracking (syncs per month)

### Changed
- ⚠️ **Breaking**: Default behavior changed to preview (was apply)
- ⚠️ **Breaking**: Structured output directory (./ggen.out/)
- ⚠️ **Breaking**: Guards run by default (was opt-in)

### Fixed
- 🐛 Non-deterministic generation (G6 check enforces determinism)
- 🐛 Path traversal vulnerabilities (G1 check prevents)
- 🐛 Output file collisions (G2 check prevents)

---

**End of MIGRATION_GUIDE_V2.1.md**
