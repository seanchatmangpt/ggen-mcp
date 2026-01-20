# Proof-First Compiler (ggen v2.1)

**Version**: 2.1.0 | Proof-Carrying Code | Cryptographic Receipts | Preview-First
**Status**: Production | TPS-Aligned | SPR-Documented

---

## Overview

ggen v2.1 introduces **proof-first code generation**: every compilation emits cryptographic receipts, comprehensive reports, and guard verdicts. Changes are **preview by default**, explicit application required.

**Core Principle**: Code generation = auditable transformation with cryptographic proof chain.

**Key Features**:
- Preview by default (no writes without explicit approval)
- First Light Report (1-page markdown/JSON summary)
- Cryptographic receipts (SHA-256 hashes, deterministic builds)
- Guard Kernel (7 safety checks, fail-fast)
- Receipt verification (standalone tool, 7-check audit)
- Jira compiler stage (optional, workflow integration)
- Entitlement gates (capability-based licensing)

---

## Architecture

### Compilation Pipeline (10 Stages)

```
┌────────────────────────────────────────────────────────────┐
│ Stage 1: Discovery                                          │
│ - Find ggen.toml, ontologies, queries, templates           │
│ - Compute workspace fingerprint (SHA-256)                  │
└────────────────────────────────────────────────────────────┘
         ↓
┌────────────────────────────────────────────────────────────┐
│ Stage 2: Guard Kernel                                       │
│ - Run 7 safety checks (G1-G7)                              │
│ - Fail-fast on guard failure (unless force: true)         │
└────────────────────────────────────────────────────────────┘
         ↓
┌────────────────────────────────────────────────────────────┐
│ Stage 3: SPARQL Extraction                                  │
│ - Execute queries against ontologies (parallel)            │
│ - Cache results (SHA-256 keyed)                            │
└────────────────────────────────────────────────────────────┘
         ↓
┌────────────────────────────────────────────────────────────┐
│ Stage 4: Template Rendering                                 │
│ - Generate code via Tera templates (parallel)              │
│ - Deterministic rendering (stable iteration order)         │
└────────────────────────────────────────────────────────────┘
         ↓
┌────────────────────────────────────────────────────────────┐
│ Stage 5: Validation                                         │
│ - Multi-language syntax checks (Rust, TypeScript, YAML)    │
│ - Linting (clippy, eslint)                                 │
└────────────────────────────────────────────────────────────┘
         ↓
┌────────────────────────────────────────────────────────────┐
│ Stage 6: First Light Report                                 │
│ - Emit markdown/JSON summary (1-page)                      │
│ - Sections: inputs, guards, changes, validation, perf      │
└────────────────────────────────────────────────────────────┘
         ↓
┌────────────────────────────────────────────────────────────┐
│ Stage 7: Cryptographic Receipt                              │
│ - Emit receipt.json (workspace fingerprint + hashes)       │
│ - Schema: schemas/receipt.json (stable v1.0.0)            │
└────────────────────────────────────────────────────────────┘
         ↓
┌────────────────────────────────────────────────────────────┐
│ Stage 8: Diff Generation                                    │
│ - Emit unified diff (preview mode)                         │
│ - Format: Git-compatible patch file                        │
└────────────────────────────────────────────────────────────┘
         ↓
┌────────────────────────────────────────────────────────────┐
│ Stage 9: Jira Sync (Optional)                              │
│ - Create/sync Jira tickets (if enabled in ggen.toml)      │
│ - Modes: dry_run, create, sync                            │
└────────────────────────────────────────────────────────────┘
         ↓
┌────────────────────────────────────────────────────────────┐
│ Stage 10: Atomic Writes (Apply Mode Only)                  │
│ - Write files (if preview: false)                          │
│ - Atomic rename (temp → final)                             │
└────────────────────────────────────────────────────────────┘
```

**Fail-Fast**: Any stage failure stops pipeline (unless `force: true`).
**Parallelism**: SPARQL/rendering stages execute in parallel (bounded by CPU cores).
**Determinism**: SHA-256 verification ensures same inputs → same outputs.

---

## Key Features

### 1. Preview by Default

**Principle**: Prevent accidental overwrites. Explicit opt-in for writes.

**API**:
```json
// Preview (default) - generates report, no writes
{
  "tool": "sync_ggen",
  "params": {
    "workspace_root": "."
  }
}

// Apply - writes files after guards pass
{
  "tool": "sync_ggen",
  "params": {
    "workspace_root": ".",
    "preview": false
  }
}
```

**Output**:
```bash
# Preview mode
./ggen.out/
├── reports/2026-01-20-163000.md    # First Light Report
├── receipts/7f83b165.json          # Cryptographic receipt
└── diffs/7f83b165.patch            # Unified diff

# Apply mode (preview: false)
./ggen.out/
├── reports/2026-01-20-163000.md
├── receipts/7f83b165.json
├── diffs/7f83b165.patch
└── (files written to src/generated/)
```

**Workflow**:
1. Run preview → Review report/diff
2. If satisfied → Run apply (`preview: false`)
3. Verify receipt → Commit changes

---

### 2. First Light Report

**Purpose**: 1-page summary of every compilation. Human-readable markdown or machine-readable JSON.

**Sections**:

#### A. Inputs Discovered
```markdown
## Inputs Discovered (Stage 1)

- **Config**: ggen.toml (SHA-256: 5f4c9c9d...)
- **Ontologies**:
  - ontology/mcp-domain.ttl (42KB, 1,247 triples)
- **Queries**: 14 SPARQL files
- **Templates**: 21 Tera templates
- **Workspace Fingerprint**: a3d9f7e2... (root + config + ontologies)
```

#### B. Guard Verdicts
```markdown
## Guard Verdicts (Stage 2)

| Guard | Verdict | Diagnostic |
|-------|---------|------------|
| G1: Path Safety | ✅ PASS | All paths validated |
| G2: Output Overlap | ✅ PASS | No duplicate outputs |
| G3: Template Compilation | ✅ PASS | 21/21 templates valid |
| G4: Turtle Parse | ✅ PASS | 1,247 triples parsed |
| G5: SPARQL Execution | ✅ PASS | 14/14 queries executed |
| G6: Determinism | ✅ PASS | SHA-256 match (cache hit) |
| G7: Bounds | ✅ PASS | 24 files, 47KB total |

**Result**: All guards passed. Safe to apply.
```

#### C. Changes
```markdown
## Changes (Stage 4-5)

| File | Status | LOC | Language | Validation |
|------|--------|-----|----------|------------|
| src/generated/tools.rs | Modified | +324 | Rust | ✅ rustc, clippy |
| src/generated/schema.rs | Modified | +145 | Rust | ✅ rustc, clippy |
| src/generated/types.ts | Added | +287 | TypeScript | ✅ tsc, eslint |

**Summary**: 3 files changed, +756 LOC added, 0 errors, 0 warnings.
```

#### D. Validation
```markdown
## Validation (Stage 5)

### Rust Files (2)
- **rustc**: 0 errors, 0 warnings
- **clippy**: 0 warnings (strict mode)

### TypeScript Files (1)
- **tsc**: 0 errors
- **eslint**: 0 warnings (airbnb config)

**Result**: All validation checks passed.
```

#### E. Performance
```markdown
## Performance (Stage 1-10)

| Stage | Duration | Notes |
|-------|----------|-------|
| Discovery | 12ms | 57 files scanned |
| Guards | 87ms | 7 checks executed |
| SPARQL | 234ms | 14 queries (parallel) |
| Rendering | 156ms | 21 templates (parallel) |
| Validation | 1,245ms | rustc + clippy + tsc |
| Reports | 34ms | Markdown + JSON |
| **Total** | **1,768ms** | 1.8s end-to-end |

**Cache Hit Rate**: 85% (SPARQL results cached from prior run)
```

#### F. Receipts
```markdown
## Receipts (Stage 7-8)

- **Report**: ./ggen.out/reports/2026-01-20-163000.md (this file)
- **Receipt**: ./ggen.out/receipts/7f83b165.json (cryptographic proof)
- **Diff**: ./ggen.out/diffs/7f83b165.patch (unified diff, 847 lines)

**Verification**: Run `verify_receipt { receipt_path: "./ggen.out/receipts/7f83b165.json" }`
```

**Formats**:
- `markdown`: Human-readable (default)
- `json`: Machine-readable (structured)
- `none`: No report (receipts only)

**Configuration** (ggen.toml):
```toml
[output]
report_format = "markdown"  # markdown | json | none
emit_receipt = true
emit_diff = true
```

---

### 3. Guard Kernel

**Purpose**: 7 safety checks prevent unsafe code generation. Fail-fast by default.

#### G1: Path Safety
**Check**: No path traversal (`../`), no absolute paths outside workspace.
**Remediation**: Use relative paths within workspace root.

**Example Failure**:
```markdown
❌ FAIL: G1 Path Safety

**Issue**: Output path '../../../etc/passwd' attempts path traversal.

**Remediation**:
1. Use paths relative to workspace root: 'src/generated/entity.rs'
2. Verify ggen.toml output paths do not contain '..'
3. Check template variables for user-supplied paths
```

#### G2: Output Overlap
**Check**: No duplicate output paths (two templates → same file).
**Remediation**: Ensure unique output paths per generation rule.

**Example Failure**:
```markdown
❌ FAIL: G2 Output Overlap

**Issue**: Multiple rules write to 'src/generated/entity.rs':
- Rule 'generate_entities' (templates/entity.rs.tera)
- Rule 'generate_models' (templates/model.rs.tera)

**Remediation**:
1. Rename one output file: 'src/generated/entity_model.rs'
2. Or consolidate rules into single template
3. Check ggen.toml [[generate]] sections for duplicate output_path
```

#### G3: Template Compilation
**Check**: All Tera templates compile without syntax errors.
**Remediation**: Fix template syntax errors.

**Example Failure**:
```markdown
❌ FAIL: G3 Template Compilation

**Issue**: Template 'templates/entity.rs.tera' failed to compile:
  Line 42: Unexpected token '}}', expected filter or tag end.

**Remediation**:
1. Check template syntax at line 42
2. Common error: Unmatched braces '{{ }}' vs control tags '{% %}'
3. Run: validate_tera_template { template: "templates/entity.rs.tera" }
```

#### G4: Turtle Parse
**Check**: All RDF ontologies parse as valid Turtle syntax.
**Remediation**: Fix Turtle syntax errors.

**Example Failure**:
```markdown
❌ FAIL: G4 Turtle Parse

**Issue**: Ontology 'ontology/mcp-domain.ttl' failed to parse:
  Line 127: Expected '.' at end of triple, found ':'.

**Remediation**:
1. Check Turtle syntax at line 127
2. Common error: Missing trailing '.' after triple
3. Run: validate_ontology { ontology_path: "ontology/mcp-domain.ttl" }
```

#### G5: SPARQL Execution
**Check**: All SPARQL queries execute without errors.
**Remediation**: Fix SPARQL syntax or query logic.

**Example Failure**:
```markdown
❌ FAIL: G5 SPARQL Execution

**Issue**: Query 'queries/entities.rq' failed to execute:
  Variable ?name used but not bound in query.

**Remediation**:
1. Check SPARQL syntax in queries/entities.rq
2. Ensure all variables are bound via SELECT or BIND
3. Test query: sparql_query { query: "queries/entities.rq" }
```

#### G6: Determinism
**Check**: SHA-256 hash of output matches prior run (same inputs).
**Remediation**: Fix non-deterministic template logic.

**Example Failure**:
```markdown
❌ FAIL: G6 Determinism

**Issue**: Output hash mismatch for 'src/generated/entity.rs':
  Expected: 7f83b165a3d9f7e2...
  Got:      9a2c1d4f8b6e3a7c...

**Possible Causes**:
1. Template uses random values (UUID, timestamps)
2. Template iterates HashMap (unstable order)
3. System clock or environment variables in output

**Remediation**:
1. Use stable iteration (BTreeMap, sorted Vec)
2. Replace random values with deterministic seeds
3. Avoid timestamps in generated code
```

#### G7: Bounds
**Check**: Total output size/file count within limits.
**Remediation**: Reduce generated code size or increase limits.

**Example Failure**:
```markdown
❌ FAIL: G7 Bounds

**Issue**: Total output size (127MB) exceeds limit (100MB).

**Remediation**:
1. Reduce number of generated files (current: 547)
2. Increase limit: ggen.toml [limits] max_output_bytes = 200000000
3. Split generation rules across multiple syncs
```

**Configuration** (ggen.toml):
```toml
[guards]
enabled = ["G1", "G2", "G3", "G4", "G5", "G6", "G7"]
fail_fast = true  # Stop on first failure

[limits]
max_output_files = 1000
max_output_bytes = 104857600  # 100MB
max_template_bytes = 1048576  # 1MB per template
```

**Force Mode** (bypass guards):
```json
{
  "tool": "sync_ggen",
  "params": {
    "workspace_root": ".",
    "force": true  // ⚠️ Bypasses guards. Use with caution.
  }
}
```

---

### 4. Cryptographic Receipts

**Purpose**: Proof-carrying code. Every compilation emits receipt with SHA-256 hashes.

**Schema** (schemas/receipt.json v1.0.0):
```json
{
  "version": "1.0.0",
  "timestamp": "2026-01-20T16:30:00Z",
  "compiler_version": "ggen-v2.1.0",
  "mode": "preview",

  "workspace": {
    "root": "/home/user/ggen-mcp",
    "fingerprint": "a3d9f7e2c1b5f4d8e6a9c3b1f2e7d4a8"
  },

  "inputs": {
    "config": {
      "path": "ggen.toml",
      "hash": "5f4c9c9dc40aae69f519c8dfea66b",
      "size": 17508
    },
    "ontologies": [
      {
        "path": "ontology/mcp-domain.ttl",
        "hash": "c40aae69f519c8dfea66b5f4c9c9d",
        "size": 42738,
        "triples": 1247
      }
    ],
    "queries": [
      {
        "path": "queries/entities.rq",
        "hash": "9f519c8dfea66b5f4c9c9dc40aae6"
      }
    ],
    "templates": [
      {
        "path": "templates/entity.rs.tera",
        "hash": "dfea66b5f4c9c9dc40aae69f519c8"
      }
    ]
  },

  "guards": [
    {
      "name": "G1_PathSafety",
      "verdict": "PASS",
      "diagnostic": "All paths validated"
    },
    {
      "name": "G2_OutputOverlap",
      "verdict": "PASS",
      "diagnostic": "No duplicate outputs"
    }
  ],

  "outputs": [
    {
      "path": "src/generated/tools.rs",
      "hash": "7f83b165a3d9f7e2c1b5f4d8e6a9c3b1",
      "size": 14523,
      "status": "modified",
      "language": "rust"
    }
  ],

  "performance": {
    "total_duration_ms": 1768,
    "cache_hit_rate": 0.85,
    "stages": {
      "discovery": 12,
      "guards": 87,
      "sparql": 234,
      "rendering": 156,
      "validation": 1245
    }
  },

  "artifacts": {
    "report": "./ggen.out/reports/2026-01-20-163000.md",
    "diff": "./ggen.out/diffs/7f83b165.patch"
  }
}
```

**Properties**:
- **Immutable**: Receipt cannot be modified without invalidating hashes
- **Deterministic**: Same inputs → same fingerprint
- **Verifiable**: Standalone verification tool (see Receipt Verification)
- **Auditable**: Persistent trail for compliance (SOC2, ISO 27001)

**Use Cases**:
1. **Supply Chain Security**: Verify generated code matches declared inputs
2. **Audit Compliance**: Prove generation process followed policy
3. **Reproducible Builds**: Guarantee same inputs → same outputs
4. **Multi-Party Verification**: Third parties can verify receipts

---

### 5. Receipt Verification

**Tool**: `verify_receipt`

**Purpose**: Standalone verification of cryptographic receipts. 7-check audit.

**API**:
```json
{
  "tool": "verify_receipt",
  "params": {
    "receipt_path": "./ggen.out/receipts/7f83b165.json"
  }
}
```

**7 Verification Checks**:

#### V1: Schema Version
**Check**: Receipt schema version is supported.
**Expected**: `"version": "1.0.0"`
**Failure**: Unsupported schema version (upgrade verifier).

#### V2: Workspace Fingerprint
**Check**: Workspace fingerprint matches current workspace state.
**Expected**: SHA-256(workspace_root + config + ontologies) matches `workspace.fingerprint`.
**Failure**: Workspace changed since receipt generated (ontologies/config modified).

#### V3: Input File Hashes
**Check**: All input file hashes match current file contents.
**Expected**: SHA-256(file_content) matches `inputs.*.hash`.
**Failure**: Input file(s) modified since receipt generated.

#### V4: Output File Hashes
**Check**: All output file hashes match current file contents.
**Expected**: SHA-256(file_content) matches `outputs.*.hash`.
**Failure**: Generated file(s) modified after generation.

#### V5: Guard Verdicts Integrity
**Check**: All guards passed (no FAIL verdicts).
**Expected**: All `guards[].verdict == "PASS"`.
**Failure**: Receipt indicates guard failure (unsafe generation).

#### V6: Metadata Consistency
**Check**: Receipt metadata is internally consistent.
**Expected**: Timestamps valid, compiler version matches format.
**Failure**: Receipt corrupted or tampered.

#### V7: Signature (Optional)
**Check**: Cryptographic signature validates (if present).
**Expected**: ECDSA signature validates against public key.
**Failure**: Signature invalid (receipt tampered).
**Note**: v2.1 supports unsigned receipts. Signature planned for v2.2.

**Response**:
```json
{
  "status": "success",
  "verification": {
    "receipt_path": "./ggen.out/receipts/7f83b165.json",
    "checks": [
      {
        "check": "V1_SchemaVersion",
        "verdict": "PASS",
        "message": "Schema version 1.0.0 supported"
      },
      {
        "check": "V2_WorkspaceFingerprint",
        "verdict": "PASS",
        "message": "Workspace fingerprint matches"
      },
      {
        "check": "V3_InputFileHashes",
        "verdict": "PASS",
        "message": "All 57 input files verified"
      },
      {
        "check": "V4_OutputFileHashes",
        "verdict": "PASS",
        "message": "All 24 output files verified"
      },
      {
        "check": "V5_GuardVerdictsIntegrity",
        "verdict": "PASS",
        "message": "All 7 guards passed"
      },
      {
        "check": "V6_MetadataConsistency",
        "verdict": "PASS",
        "message": "Metadata valid"
      },
      {
        "check": "V7_Signature",
        "verdict": "SKIP",
        "message": "Receipt not signed (optional)"
      }
    ],
    "result": "VERIFIED",
    "summary": "Receipt valid. Generated code matches declared inputs."
  }
}
```

**Failure Example**:
```json
{
  "status": "success",
  "verification": {
    "checks": [
      {
        "check": "V4_OutputFileHashes",
        "verdict": "FAIL",
        "message": "Hash mismatch: src/generated/tools.rs (expected 7f83b165..., got 9a2c1d4f...)"
      }
    ],
    "result": "FAILED",
    "summary": "Receipt invalid. Output file(s) modified after generation."
  }
}
```

**CLI Usage**:
```bash
# Verify specific receipt
ggen verify ./ggen.out/receipts/7f83b165.json

# Verify all receipts in directory
ggen verify ./ggen.out/receipts/*.json

# CI integration
ggen verify --fail-fast ./ggen.out/receipts/*.json || exit 1
```

---

### 6. Jira Compiler Stage (Optional)

**Purpose**: Integrate Jira ticket management into compilation pipeline.

**3 Modes**:

#### A. Dry Run (Default)
Generate ticket plan, don't create.

```toml
[jira]
enabled = true
mode = "dry_run"
project_key = "PROJ"
base_url = "https://company.atlassian.net"
auth_token_env = "JIRA_TOKEN"
```

**Output** (First Light Report):
```markdown
## Jira Tickets (Stage 9) - Dry Run

**Tickets to Create**: 13

| Summary | Type | Labels | Assignee |
|---------|------|--------|----------|
| Implement User entity | Task | codegen, user | unassigned |
| Add Product schema | Task | codegen, product | unassigned |
| Update API types | Task | codegen, api | unassigned |

**Note**: Dry run mode. No tickets created. Set mode='create' to apply.
```

#### B. Create Mode
Create Jira tickets from generated files.

```toml
[jira]
enabled = true
mode = "create"
project_key = "PROJ"
base_url = "https://company.atlassian.net"
auth_token_env = "JIRA_TOKEN"

[jira.mapping]
# Map generated file metadata → Jira fields
summary_template = "Implement {{ entity_name }} entity"
type = "Task"
labels = ["codegen", "{{ module }}"]
```

**Output** (First Light Report):
```markdown
## Jira Tickets (Stage 9) - Created

**Tickets Created**: 13

| Ticket | Summary | Status | Link |
|--------|---------|--------|------|
| PROJ-456 | Implement User entity | To Do | https://company.atlassian.net/browse/PROJ-456 |
| PROJ-457 | Add Product schema | To Do | https://company.atlassian.net/browse/PROJ-457 |
| PROJ-458 | Update API types | To Do | https://company.atlassian.net/browse/PROJ-458 |

**Total**: 13 tickets created in 2.3s
```

#### C. Sync Mode
Bidirectional sync with spreadsheet.

```toml
[jira]
enabled = true
mode = "sync"
project_key = "PROJ"
base_url = "https://company.atlassian.net"
auth_token_env = "JIRA_TOKEN"

[jira.spreadsheet_sync]
workbook_id = "wb-codegen-tracker"
sheet_name = "Generated Code"
field_mapping = {
  summary = "B",
  status = "C",
  assignee = "D",
  file_path = "E"
}
conflict_resolution = "jira_wins"  # jira_wins | spreadsheet_wins | manual
```

**Workflow**:
1. Generate code → Extract metadata
2. Query Jira for existing tickets (JQL: `project = PROJ AND labels = codegen`)
3. Sync bidirectionally: Jira ↔ Spreadsheet
4. Create new tickets for new files
5. Update existing tickets with file changes

**Output** (First Light Report):
```markdown
## Jira Sync (Stage 9)

**Tickets Synced**: 13
**Tickets Created**: 3
**Tickets Updated**: 10
**Conflicts Resolved**: 2 (jira_wins strategy)

| Ticket | Action | Details |
|--------|--------|---------|
| PROJ-456 | Updated | Status: In Progress (from Jira) |
| PROJ-457 | Updated | Assignee: alice@example.com (from Jira) |
| PROJ-458 | Conflict | File path mismatch (resolved: Jira wins) |

**Spreadsheet**: Updated 13 rows in 'Generated Code' sheet
```

**Environment**:
```bash
export JIRA_TOKEN="your-jira-api-token"
ggen sync  # Jira stage runs if enabled in ggen.toml
```

---

### 7. Entitlement Gate (Optional)

**Purpose**: Pluggable capability gating for monetization.

**Capabilities**:
```rust
pub enum Capability {
    // Free tier
    PreviewMode,
    ReadOnlyTools,
    BasicValidation,

    // Paid tier
    ApplyMode,
    JiraCreate,
    JiraSync,
    FullGuardSuite,
    CryptographicReceipts,

    // Enterprise tier
    MultiWorkspace,
    TeamCollaboration,
    AuditReporting,
    SignedReceipts,
    CustomGuards,
}
```

**Providers**:

#### A. Local File Provider
Read from `.ggen_license` file.

```json
// .ggen_license
{
  "version": "1.0.0",
  "tier": "paid",
  "capabilities": [
    "PreviewMode",
    "ApplyMode",
    "JiraCreate",
    "FullGuardSuite",
    "CryptographicReceipts"
  ],
  "expires_at": "2027-01-20T00:00:00Z",
  "licensee": "Acme Corp",
  "signature": "..."
}
```

#### B. Env Var Provider
Read from `GGEN_LICENSE` environment variable.

```bash
export GGEN_LICENSE="eyJ0aWVyIjoicGFpZCIsImNhcGFiaWxpdGllcyI6WyJBcHBseU1vZGUiLCJKaXJhQ3JlYXRlIl19"
```

#### C. GCP Marketplace Provider (Future)
Integrate with GCP Marketplace for billing.

**Configuration** (ggen.toml):
```toml
[entitlement]
enabled = true
provider = "local"  # local | env | gcp_marketplace
license_path = ".ggen_license"  # For local provider
grace_period_days = 30  # Allow 30 days after expiry
```

**Behavior**:

**Free Tier (Default)**:
```json
{
  "tool": "sync_ggen",
  "params": {
    "workspace_root": ".",
    "preview": false  // ❌ Denied: ApplyMode requires paid tier
  }
}

// Response:
{
  "error": "EntitlementError",
  "message": "Capability 'ApplyMode' not available in your license.",
  "current_tier": "free",
  "upgrade_url": "https://ggen.dev/pricing"
}
```

**Paid Tier**:
```json
{
  "tool": "sync_ggen",
  "params": {
    "workspace_root": ".",
    "preview": false  // ✅ Allowed: ApplyMode available
  }
}

// Response: Normal compilation proceeds
```

**Usage Reporting**:
```markdown
## Usage Report (Stage 6)

**License**: Paid (expires 2027-01-20)
**Capabilities Used**: ApplyMode, JiraCreate, CryptographicReceipts
**Usage This Month**: 127 syncs (limit: 1000)
**Remaining**: 873 syncs
```

---

## Configuration Reference

### ggen.toml Extensions (v2.1)

```toml
# Output configuration
[output]
report_format = "markdown"  # markdown | json | none
emit_receipt = true
emit_diff = true
output_dir = "./ggen.out"  # Default: ./ggen.out

# Guard configuration
[guards]
enabled = ["G1", "G2", "G3", "G4", "G5", "G6", "G7"]
fail_fast = true  # Stop on first guard failure

# Limits (G7 bounds check)
[limits]
max_output_files = 1000
max_output_bytes = 104857600  # 100MB
max_template_bytes = 1048576  # 1MB per template

# Jira integration (optional)
[jira]
enabled = true
mode = "dry_run"  # dry_run | create | sync
project_key = "PROJ"
base_url = "https://company.atlassian.net"
auth_token_env = "JIRA_TOKEN"

[jira.mapping]
summary_template = "Implement {{ entity_name }}"
type = "Task"
labels = ["codegen", "{{ module }}"]

[jira.spreadsheet_sync]
workbook_id = "wb-codegen-tracker"
sheet_name = "Generated Code"
field_mapping = { summary = "B", status = "C", assignee = "D" }
conflict_resolution = "jira_wins"  # jira_wins | spreadsheet_wins | manual

# Entitlement (optional)
[entitlement]
enabled = false  # Default: false (free tier)
provider = "local"  # local | env | gcp_marketplace
license_path = ".ggen_license"
grace_period_days = 30
```

---

## Workflows

### Workflow 1: Basic Preview → Apply

```bash
# 1. Preview (default)
{
  "tool": "sync_ggen",
  "params": {
    "workspace_root": "."
  }
}

# 2. Review report
cat ./ggen.out/reports/latest.md

# 3. Review diff
less ./ggen.out/diffs/7f83b165.patch

# 4. Apply if satisfied
{
  "tool": "sync_ggen",
  "params": {
    "workspace_root": ".",
    "preview": false
  }
}

# 5. Verify receipt
{
  "tool": "verify_receipt",
  "params": {
    "receipt_path": "./ggen.out/receipts/7f83b165.json"
  }
}

# 6. Commit
git add src/generated/
git commit -m "feat: Generate code from ontology (receipt: 7f83b165)"
```

---

### Workflow 2: Jira Integration

```bash
# 1. Enable Jira in ggen.toml
[jira]
enabled = true
mode = "create"
project_key = "PROJ"

# 2. Dry run first (preview Jira tickets)
{
  "tool": "sync_ggen",
  "params": {
    "workspace_root": ".",
    "preview": true
  }
}

# 3. Review Jira section in report
cat ./ggen.out/reports/latest.md | grep -A 20 "## Jira Tickets"

# 4. Apply (creates Jira tickets)
{
  "tool": "sync_ggen",
  "params": {
    "workspace_root": ".",
    "preview": false
  }
}

# 5. Check Jira results in report
cat ./ggen.out/reports/latest.md | grep "Tickets Created"
# Output: "Tickets Created: 13"
```

---

### Workflow 3: CI/CD Integration

```yaml
# .github/workflows/codegen.yml
name: Code Generation

on:
  push:
    paths:
      - 'ontology/**'
      - 'queries/**'
      - 'templates/**'
      - 'ggen.toml'

jobs:
  generate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install ggen
        run: cargo install ggen-mcp

      - name: Preview generation
        run: ggen sync --preview

      - name: Upload report
        uses: actions/upload-artifact@v3
        with:
          name: first-light-report
          path: ./ggen.out/reports/*.md

      - name: Apply generation
        run: ggen sync --no-preview

      - name: Verify receipt
        run: ggen verify ./ggen.out/receipts/*.json

      - name: Commit generated code
        run: |
          git config user.name "GitHub Actions"
          git config user.email "actions@github.com"
          git add src/generated/
          git commit -m "chore: Update generated code (receipt: $(ls -t ./ggen.out/receipts/*.json | head -1 | xargs basename .json))"
          git push
```

---

### Workflow 4: Audit Compliance

```bash
# 1. Generate code with receipt
ggen sync --no-preview

# 2. Export receipt for audit
cp ./ggen.out/receipts/7f83b165.json ./audit/receipts/2026-01-20.json

# 3. Verify receipt (auditor can run independently)
ggen verify ./audit/receipts/2026-01-20.json

# 4. Generate audit report
ggen audit-report --receipt ./audit/receipts/2026-01-20.json --format pdf
# Output: ./audit/reports/2026-01-20.pdf (SOC2-compliant format)
```

---

### Workflow 5: Multi-Workspace (Enterprise)

```bash
# Requires Enterprise tier license

# 1. Configure workspaces
[workspaces]
default = "./workspace-a"
additional = ["./workspace-b", "./workspace-c"]

# 2. Sync all workspaces
ggen sync --all-workspaces

# 3. Verify all receipts
ggen verify ./ggen.out/receipts/**/*.json

# 4. Generate consolidated report
ggen consolidate-reports --output ./ggen.out/consolidated.md
```

---

## API Reference

### Tool: `sync_ggen`

**Purpose**: Run code generation pipeline with proof-first guarantees.

**Parameters**:
```typescript
interface SyncGgenParams {
  workspace_root: string;           // Required: Workspace root path
  preview?: boolean;                // Default: true (no writes)
  force?: boolean;                  // Default: false (bypass guards)
  validate?: boolean;               // Default: true (run validation)
  dry_run?: boolean;                // Alias for preview: true
  mode?: "minimal" | "default" | "full";  // Response detail level
}
```

**Response**:
```typescript
interface SyncGgenResponse {
  status: "success" | "error";
  compilation: {
    workspace_fingerprint: string;  // SHA-256 hash
    mode: "preview" | "apply";
    guards_passed: number;
    guards_failed: number;
    files_generated: number;
    lines_of_code: number;
    duration_ms: number;
  };
  artifacts: {
    report: string;    // Path to First Light Report
    receipt: string;   // Path to cryptographic receipt
    diff?: string;     // Path to unified diff (preview mode)
  };
  guards: Array<{
    name: string;
    verdict: "PASS" | "FAIL";
    diagnostic: string;
  }>;
  jira?: {
    tickets_created: number;
    tickets_updated: number;
    conflicts_resolved: number;
  };
}
```

**Error Codes**:
- `GuardFailure`: Guard check failed (see `guards` array)
- `EntitlementError`: Capability not available in license
- `ValidationError`: Generated code failed validation
- `PathTraversalError`: Output path attempts directory traversal

---

### Tool: `verify_receipt`

**Purpose**: Verify cryptographic receipt.

**Parameters**:
```typescript
interface VerifyReceiptParams {
  receipt_path: string;             // Required: Path to receipt.json
  fail_fast?: boolean;              // Default: false (run all checks)
}
```

**Response**:
```typescript
interface VerifyReceiptResponse {
  status: "success" | "error";
  verification: {
    receipt_path: string;
    checks: Array<{
      check: string;                // V1-V7
      verdict: "PASS" | "FAIL" | "SKIP";
      message: string;
    }>;
    result: "VERIFIED" | "FAILED";
    summary: string;
  };
}
```

---

## Best Practices

### 1. Always Preview First
```bash
# ❌ Don't: Apply directly
ggen sync --no-preview

# ✅ Do: Preview → Review → Apply
ggen sync                  # Preview
cat ./ggen.out/reports/latest.md
ggen sync --no-preview     # Apply after review
```

### 2. Commit Receipts with Code
```bash
git add src/generated/ ./ggen.out/receipts/
git commit -m "feat: Generate code (receipt: 7f83b165)"
```

### 3. Verify Receipts in CI
```yaml
- name: Verify receipts
  run: ggen verify ./ggen.out/receipts/*.json || exit 1
```

### 4. Use Jira Dry Run First
```toml
[jira]
mode = "dry_run"  # Preview tickets before creating
```

### 5. Review Guard Failures Carefully
```markdown
If G6 (Determinism) fails:
→ Investigate non-deterministic template logic
→ Fix before applying

If G7 (Bounds) fails:
→ Reduce generated code size or increase limits
```

---

## Troubleshooting

### Issue 1: Guard G1 Fails (Path Safety)

**Symptom**:
```markdown
❌ FAIL: G1 Path Safety
Issue: Output path '../../../etc/passwd' attempts path traversal.
```

**Solution**:
1. Check ggen.toml output paths
2. Ensure all paths relative to workspace root
3. Review template variables for user-supplied paths

---

### Issue 2: Guard G6 Fails (Determinism)

**Symptom**:
```markdown
❌ FAIL: G6 Determinism
Issue: Output hash mismatch for 'src/generated/entity.rs'
```

**Solution**:
1. Check template for random values (UUID, timestamps)
2. Use `BTreeMap` instead of `HashMap` (stable iteration)
3. Replace `now()` with deterministic placeholder

---

### Issue 3: Jira Integration Fails

**Symptom**:
```markdown
Error: JiraApiError: Authentication failed (401 Unauthorized)
```

**Solution**:
1. Verify `JIRA_TOKEN` environment variable set
2. Check token has correct permissions (create issues, read project)
3. Verify `base_url` in ggen.toml is correct

---

### Issue 4: Receipt Verification Fails (V4)

**Symptom**:
```markdown
❌ FAIL: V4 Output File Hashes
Issue: Hash mismatch: src/generated/tools.rs
```

**Solution**:
1. Generated file was modified after generation
2. Re-run `ggen sync --no-preview` to regenerate
3. Or discard manual changes and restore from receipt

---

### Issue 5: Entitlement Error

**Symptom**:
```json
{
  "error": "EntitlementError",
  "message": "Capability 'ApplyMode' not available in your license."
}
```

**Solution**:
1. Check license file `.ggen_license` exists
2. Verify license not expired: `jq .expires_at .ggen_license`
3. Upgrade license at https://ggen.dev/pricing

---

## Performance Optimization

### 1. Parallel SPARQL Queries
```toml
[performance]
max_parallel_queries = 8  # Default: CPU cores
```

### 2. Cache SPARQL Results
```toml
[cache]
enabled = true
ttl_seconds = 3600  # Cache for 1 hour
```

### 3. Skip Validation (Faster, Less Safe)
```bash
ggen sync --no-validate  # ⚠️ Skips rustc/clippy/tsc
```

---

## Migration from v2.0

See [MIGRATION_GUIDE_V2.1.md](../MIGRATION_GUIDE_V2.1.md) for complete migration guide.

**Key Changes**:
1. Preview is now default (was apply by default)
2. Receipts always emitted (was optional)
3. Guard kernel runs by default (was opt-in)
4. First Light Report always generated

**Backward Compatibility**:
- Set `preview: false` to restore v2.0 apply-by-default behavior
- Set `[output] emit_receipt = false` to disable receipts
- Set `[guards] enabled = []` to disable guards

---

## References

- **Guard Kernel**: [docs/GUARD_KERNEL.md](./GUARD_KERNEL.md)
- **First Light Report**: [docs/FIRST_LIGHT_REPORT.md](./FIRST_LIGHT_REPORT.md)
- **Receipt Verification**: [docs/RECEIPT_VERIFICATION.md](./RECEIPT_VERIFICATION.md)
- **Entitlement Provider**: [docs/ENTITLEMENT_PROVIDER.md](./ENTITLEMENT_PROVIDER.md)
- **Migration Guide**: [MIGRATION_GUIDE_V2.1.md](../MIGRATION_GUIDE_V2.1.md)
- **CLAUDE.md**: [CLAUDE.md](../CLAUDE.md) (v2.1 update)

---

**End of PROOF_FIRST_COMPILER.md**
