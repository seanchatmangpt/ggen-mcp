# First Light Report

**Version**: 2.1.0 | 1-Page Compilation Summary | Human + Machine Readable

---

## Overview

**First Light Report**: 1-page summary emitted after every ggen compilation. Provides immediate visibility into:
- What was discovered (inputs)
- What was checked (guards)
- What was changed (outputs)
- What was validated (quality)
- How long it took (performance)

**Principle**: Compilation transparency. No hidden state. Every sync = auditable report.

**Formats**:
- **Markdown** (default): Human-readable, GitHub-friendly
- **JSON**: Machine-readable, CI/CD integration
- **None**: Disable reports (receipts only)

---

## Report Sections

### Section A: Inputs Discovered (Stage 1)

**Purpose**: Document all inputs consumed by compilation.

**Example** (Markdown):
```markdown
## Inputs Discovered (Stage 1)

**Timestamp**: 2026-01-20 16:30:00 UTC
**Compiler Version**: ggen-v2.1.0
**Mode**: Preview

### Configuration
- **File**: ggen.toml
- **SHA-256**: 5f4c9c9dc40aae69f519c8dfea66b
- **Size**: 17.5 KB
- **Rules**: 24 generation rules

### Ontologies (1 file)
| File | Size | Triples | SHA-256 |
|------|------|---------|---------|
| ontology/mcp-domain.ttl | 42.7 KB | 1,247 | c40aae69f519c8... |

### SPARQL Queries (14 files)
| File | Lines | SHA-256 |
|------|-------|---------|
| queries/entities.rq | 47 | 9f519c8dfea66b... |
| queries/operations.rq | 82 | dfea66b5f4c9c9... |
| queries/relationships.rq | 63 | 5f4c9c9dc40aae... |
| *(11 more files)* | | |

### Tera Templates (21 files)
| File | Lines | Variables | SHA-256 |
|------|-------|-----------|---------|
| templates/entity.rs.tera | 124 | 7 | 7f83b165a3d9f7... |
| templates/schema.rs.tera | 89 | 5 | a3d9f7e2c1b5f4... |
| *(19 more files)* | | | |

### Workspace Fingerprint
- **SHA-256**: a3d9f7e2c1b5f4d8e6a9c3b1f2e7d4a8
- **Components**: workspace_root + config + ontologies
```

**Example** (JSON):
```json
{
  "inputs": {
    "timestamp": "2026-01-20T16:30:00Z",
    "compiler_version": "ggen-v2.1.0",
    "mode": "preview",
    "config": {
      "path": "ggen.toml",
      "hash": "5f4c9c9dc40aae69f519c8dfea66b",
      "size": 17508,
      "rules": 24
    },
    "ontologies": [
      {
        "path": "ontology/mcp-domain.ttl",
        "size": 42738,
        "triples": 1247,
        "hash": "c40aae69f519c8dfea66b5f4c9c9d"
      }
    ],
    "queries": [
      {
        "path": "queries/entities.rq",
        "lines": 47,
        "hash": "9f519c8dfea66b5f4c9c9dc40aae6"
      }
    ],
    "templates": [
      {
        "path": "templates/entity.rs.tera",
        "lines": 124,
        "variables": 7,
        "hash": "7f83b165a3d9f7e2c1b5f4d8e6a9c3"
      }
    ],
    "workspace_fingerprint": "a3d9f7e2c1b5f4d8e6a9c3b1f2e7d4a8"
  }
}
```

---

### Section B: Guard Verdicts (Stage 2)

**Purpose**: Document safety checks (7 guards).

**Example** (All Passing):
```markdown
## Guard Verdicts (Stage 2)

| Guard | Verdict | Diagnostic |
|-------|---------|------------|
| G1: Path Safety | ✅ PASS | All 24 output paths validated |
| G2: Output Overlap | ✅ PASS | No duplicate output paths detected |
| G3: Template Compilation | ✅ PASS | 21/21 templates compiled successfully |
| G4: Turtle Parse | ✅ PASS | 1,247 triples parsed from 1 ontology |
| G5: SPARQL Execution | ✅ PASS | 14/14 queries executed successfully |
| G6: Determinism | ✅ PASS | SHA-256 match (cache hit: 85%) |
| G7: Bounds | ✅ PASS | 24 files, 47.2 KB total (under 100 MB limit) |

**Result**: ✅ All guards passed. Safe to apply.
```

**Example** (Failure):
```markdown
## Guard Verdicts (Stage 2)

| Guard | Verdict | Diagnostic |
|-------|---------|------------|
| G1: Path Safety | ✅ PASS | All 24 output paths validated |
| G2: Output Overlap | ❌ FAIL | **Duplicate output: src/generated/entity.rs** |
| G3: Template Compilation | ⏸️ SKIPPED | Blocked by G2 failure |
| G4: Turtle Parse | ⏸️ SKIPPED | Blocked by G2 failure |
| G5: SPARQL Execution | ⏸️ SKIPPED | Blocked by G2 failure |
| G6: Determinism | ⏸️ SKIPPED | Blocked by G2 failure |
| G7: Bounds | ⏸️ SKIPPED | Blocked by G2 failure |

**Result**: ❌ Guard failure. Compilation stopped.

### G2 Failure Details

**Issue**: Multiple generation rules write to the same output path.

**Conflicting Rules**:
1. Rule `generate_entities` (templates/entity.rs.tera) → src/generated/entity.rs
2. Rule `generate_models` (templates/model.rs.tera) → src/generated/entity.rs

**Remediation**:
1. Rename one output file: `src/generated/entity_model.rs`
2. Or consolidate rules into single template
3. Check ggen.toml `[[generate]]` sections for duplicate `output_path`
```

---

### Section C: Changes (Stage 4-5)

**Purpose**: Document files generated/modified.

**Example** (Markdown):
```markdown
## Changes (Stage 4-5)

### Files Generated (24)

| File | Status | LOC | Language | Validation |
|------|--------|-----|----------|------------|
| src/generated/tools.rs | Modified | +324 | Rust | ✅ rustc, clippy |
| src/generated/schema.rs | Modified | +145 | Rust | ✅ rustc, clippy |
| src/generated/types.ts | Added | +287 | TypeScript | ✅ tsc, eslint |
| src/generated/api.rs | Modified | +198 | Rust | ✅ rustc, clippy |
| src/generated/models/ | Directory | - | - | - |
| src/generated/models/user.rs | Added | +78 | Rust | ✅ rustc, clippy |
| src/generated/models/product.rs | Added | +92 | Rust | ✅ rustc, clippy |
| *(18 more files)* | | | | |

### Summary
- **Files Added**: 13
- **Files Modified**: 11
- **Files Deleted**: 0
- **Total LOC**: +2,847 lines added
- **Languages**: Rust (18), TypeScript (5), YAML (1)
```

**Example** (JSON):
```json
{
  "changes": {
    "files": [
      {
        "path": "src/generated/tools.rs",
        "status": "modified",
        "loc_added": 324,
        "loc_deleted": 87,
        "language": "rust",
        "validation": {
          "rustc": "passed",
          "clippy": "passed"
        }
      }
    ],
    "summary": {
      "files_added": 13,
      "files_modified": 11,
      "files_deleted": 0,
      "total_loc_added": 2847,
      "total_loc_deleted": 234,
      "languages": ["rust", "typescript", "yaml"]
    }
  }
}
```

---

### Section D: Validation (Stage 5)

**Purpose**: Document syntax/lint checks.

**Example** (All Passing):
```markdown
## Validation (Stage 5)

### Rust Files (18)
- **rustc**: ✅ 0 errors, 0 warnings
- **clippy**: ✅ 0 warnings (strict mode: `-D warnings`)

### TypeScript Files (5)
- **tsc**: ✅ 0 errors
- **eslint**: ✅ 0 warnings (airbnb config)

### YAML Files (1)
- **yamllint**: ✅ 0 errors

**Result**: ✅ All validation checks passed.
```

**Example** (Errors):
```markdown
## Validation (Stage 5)

### Rust Files (18)
- **rustc**: ❌ 3 errors, 0 warnings
- **clippy**: ⏸️ SKIPPED (rustc errors must be fixed first)

**Errors**:
```
error[E0412]: cannot find type `User` in this scope
  --> src/generated/api.rs:42:15
   |
42 |     fn get_user() -> User {
   |                      ^^^^ not found in this scope

error[E0425]: cannot find value `user_id` in this scope
  --> src/generated/api.rs:43:9
   |
43 |         user_id
   |         ^^^^^^^ not found in this scope
```

**Remediation**:
1. Check template logic for missing imports
2. Verify SPARQL query returns expected fields
3. Re-run: `ggen sync --validate`
```

---

### Section E: Performance (Stage 1-10)

**Purpose**: Document execution timings.

**Example** (Markdown):
```markdown
## Performance (Stage 1-10)

| Stage | Duration | Details |
|-------|----------|---------|
| 1. Discovery | 12 ms | 57 files scanned |
| 2. Guards | 87 ms | 7 checks executed |
| 3. SPARQL | 234 ms | 14 queries (8 parallel, 6 cache hits) |
| 4. Rendering | 156 ms | 21 templates (8 parallel) |
| 5. Validation | 1,245 ms | rustc + clippy + tsc + eslint |
| 6. Report | 34 ms | Markdown + JSON |
| 7. Receipt | 18 ms | SHA-256 hashing |
| 8. Diff | 42 ms | Unified diff (847 lines) |
| 9. Jira | 523 ms | 13 tickets created |
| 10. Writes | 0 ms | *(Preview mode: no writes)* |
| **Total** | **2,351 ms** | 2.4s end-to-end |

### Cache Performance
- **SPARQL Cache Hit Rate**: 85% (11/13 queries cached)
- **Template Cache Hit Rate**: 62% (13/21 templates cached)
- **Cache Savings**: ~1.2s (from 3.6s → 2.4s)

### Parallelism
- **Max Parallel SPARQL**: 8 (CPU cores)
- **Max Parallel Rendering**: 8 (CPU cores)
- **Speedup**: 3.2x (from 7.5s serial → 2.4s parallel)
```

---

### Section F: Receipts (Stage 7-8)

**Purpose**: Document artifact locations.

**Example** (Markdown):
```markdown
## Receipts (Stage 7-8)

### Artifacts Generated
- **Report (Markdown)**: ./ggen.out/reports/2026-01-20-163000.md *(this file)*
- **Report (JSON)**: ./ggen.out/reports/2026-01-20-163000.json
- **Receipt**: ./ggen.out/receipts/7f83b165.json *(cryptographic proof)*
- **Diff**: ./ggen.out/diffs/7f83b165.patch *(unified diff, 847 lines)*

### Receipt Fingerprint
- **SHA-256**: 7f83b165a3d9f7e2c1b5f4d8e6a9c3b1f2e7d4a8c9f3e1b6d2a7c5e8f4b9
- **Workspace**: a3d9f7e2c1b5f4d8e6a9c3b1f2e7d4a8
- **Inputs**: 57 files (config + ontologies + queries + templates)
- **Outputs**: 24 files (generated code)

### Verification
Run the following command to verify receipt:
```bash
ggen verify ./ggen.out/receipts/7f83b165.json
```

Or use MCP tool:
```json
{
  "tool": "verify_receipt",
  "params": {
    "receipt_path": "./ggen.out/receipts/7f83b165.json"
  }
}
```
```

---

### Section G: Jira Integration (Optional, Stage 9)

**Purpose**: Document Jira ticket creation/sync.

**Example** (Dry Run):
```markdown
## Jira Integration (Stage 9) - Dry Run

**Mode**: dry_run (preview only, no tickets created)

### Tickets to Create (13)

| Summary | Type | Labels | Assignee | Priority |
|---------|------|--------|----------|----------|
| Implement User entity | Task | codegen, user | unassigned | Medium |
| Add Product schema | Task | codegen, product | unassigned | Medium |
| Update API types | Task | codegen, api | unassigned | Medium |
| Generate models directory | Task | codegen, models | unassigned | Low |
| *(9 more tickets)* | | | | |

**Note**: Dry run mode. No tickets created. Set `mode = "create"` in ggen.toml to apply.
```

**Example** (Create Mode):
```markdown
## Jira Integration (Stage 9) - Created

**Mode**: create (tickets created)

### Tickets Created (13)

| Ticket | Summary | Status | Link |
|--------|---------|--------|------|
| PROJ-456 | Implement User entity | To Do | [View](https://company.atlassian.net/browse/PROJ-456) |
| PROJ-457 | Add Product schema | To Do | [View](https://company.atlassian.net/browse/PROJ-457) |
| PROJ-458 | Update API types | To Do | [View](https://company.atlassian.net/browse/PROJ-458) |
| *(10 more tickets)* | | | |

### Performance
- **API Calls**: 15 (1 auth + 13 create + 1 bulk verify)
- **Duration**: 523 ms
- **Success Rate**: 100% (13/13 created)
```

**Example** (Sync Mode):
```markdown
## Jira Integration (Stage 9) - Synced

**Mode**: sync (bidirectional with spreadsheet)

### Sync Summary
- **Tickets Queried**: 47 (JQL: `project = PROJ AND labels = codegen`)
- **Tickets Created**: 3 (new files)
- **Tickets Updated**: 10 (file changes)
- **Tickets Unchanged**: 34
- **Conflicts Resolved**: 2 (strategy: jira_wins)

### Actions Taken

| Ticket | Action | Details |
|--------|--------|---------|
| PROJ-456 | Updated | Status: In Progress ← Jira (was: To Do ← Sheet) |
| PROJ-457 | Updated | Assignee: alice@example.com ← Jira |
| PROJ-458 | Conflict | File path mismatch (resolved: Jira wins) |
| PROJ-459 | Created | New file: src/generated/models/order.rs |
| PROJ-460 | Created | New file: src/generated/models/invoice.rs |
| PROJ-461 | Created | New file: src/generated/models/payment.rs |

### Spreadsheet Sync
- **Workbook**: wb-codegen-tracker
- **Sheet**: Generated Code
- **Rows Updated**: 13
- **Rows Added**: 3
```

---

## Report Formats

### Format 1: Markdown (Default)

**File**: `./ggen.out/reports/2026-01-20-163000.md`

**Characteristics**:
- Human-readable
- GitHub-friendly (renders in PR comments)
- ~200 lines (~8 KB)
- Sections A-G (all)

**Use Cases**:
- Code review (paste in PR)
- Developer debugging
- Quick inspection

**Configuration**:
```toml
[output]
report_format = "markdown"
```

---

### Format 2: JSON (Machine-Readable)

**File**: `./ggen.out/reports/2026-01-20-163000.json`

**Characteristics**:
- Machine-readable
- Structured data (CI/CD integration)
- ~4 KB (minified)
- All fields (no truncation)

**Use Cases**:
- CI/CD pipelines (parse metrics)
- Monitoring dashboards
- API integrations

**Configuration**:
```toml
[output]
report_format = "json"
```

**Schema**:
```json
{
  "version": "2.1.0",
  "timestamp": "2026-01-20T16:30:00Z",
  "inputs": { /* Section A */ },
  "guards": [ /* Section B */ ],
  "changes": { /* Section C */ },
  "validation": { /* Section D */ },
  "performance": { /* Section E */ },
  "receipts": { /* Section F */ },
  "jira": { /* Section G (optional) */ }
}
```

---

### Format 3: None (Receipts Only)

**Configuration**:
```toml
[output]
report_format = "none"
emit_receipt = true  # Still emit receipts
```

**Use Cases**:
- Minimal output (CI/CD)
- Receipts-only verification
- Performance-critical scenarios

---

## Customization

### 1. Output Directory

**Default**: `./ggen.out/`

**Custom**:
```toml
[output]
output_dir = "./build/ggen-reports"
```

**Structure**:
```
./build/ggen-reports/
├── reports/
│   ├── 2026-01-20-163000.md
│   ├── 2026-01-20-163000.json
│   └── latest.md (symlink)
├── receipts/
│   ├── 7f83b165.json
│   └── latest.json (symlink)
└── diffs/
    ├── 7f83b165.patch
    └── latest.patch (symlink)
```

---

### 2. Retention Policy

**Configuration**:
```toml
[output]
retention_days = 30      # Keep reports for 30 days
max_reports = 100        # Keep last 100 reports max
auto_cleanup = true      # Auto-delete old reports
```

**Cleanup**:
```bash
# Manual cleanup
ggen cleanup --older-than 30d

# List reports
ggen list-reports --sort-by date --limit 10
```

---

### 3. Report Sections (Selective)

**Configuration**:
```toml
[output.sections]
inputs = true
guards = true
changes = true
validation = true
performance = false  # Disable performance section
receipts = true
jira = true
```

---

### 4. Verbosity Levels

**Configuration**:
```toml
[output]
verbosity = "default"  # minimal | default | verbose
```

**Levels**:
- **Minimal**: Summary only (inputs count, guards pass/fail, changes count)
- **Default**: Full sections A-G (current behavior)
- **Verbose**: + Template variables, SPARQL results, validation details

---

## Integration Examples

### Example 1: CI/CD (GitHub Actions)

```yaml
- name: Generate code
  run: ggen sync --no-preview

- name: Parse report
  run: |
    LOC=$(jq '.changes.summary.total_loc_added' ./ggen.out/reports/latest.json)
    echo "Generated $LOC lines of code"

- name: Comment on PR
  uses: actions/github-script@v6
  with:
    script: |
      const fs = require('fs');
      const report = fs.readFileSync('./ggen.out/reports/latest.md', 'utf8');
      github.rest.issues.createComment({
        issue_number: context.issue.number,
        owner: context.repo.owner,
        repo: context.repo.repo,
        body: `## Code Generation Report\n\n${report}`
      });
```

---

### Example 2: Slack Notification

```bash
#!/bin/bash
# scripts/notify-slack.sh

REPORT_JSON="./ggen.out/reports/latest.json"
GUARDS_PASSED=$(jq -r '.guards | map(select(.verdict == "PASS")) | length' "$REPORT_JSON")
TOTAL_GUARDS=$(jq -r '.guards | length' "$REPORT_JSON")
LOC=$(jq -r '.changes.summary.total_loc_added' "$REPORT_JSON")

if [ "$GUARDS_PASSED" -eq "$TOTAL_GUARDS" ]; then
  STATUS="✅ Success"
else
  STATUS="❌ Failure"
fi

curl -X POST https://hooks.slack.com/services/YOUR/WEBHOOK/URL \
  -H 'Content-Type: application/json' \
  -d "{
    \"text\": \"$STATUS: Code generation complete\",
    \"blocks\": [
      {
        \"type\": \"section\",
        \"text\": {
          \"type\": \"mrkdwn\",
          \"text\": \"*Guards*: $GUARDS_PASSED/$TOTAL_GUARDS passed\n*LOC*: +$LOC lines\n*Report*: <file://./ggen.out/reports/latest.md|View Report>\"
        }
      }
    ]
  }"
```

---

### Example 3: Prometheus Metrics

```bash
# Export metrics to Prometheus pushgateway

REPORT_JSON="./ggen.out/reports/latest.json"
DURATION=$(jq -r '.performance.total_duration_ms' "$REPORT_JSON")
GUARDS_PASSED=$(jq -r '.guards | map(select(.verdict == "PASS")) | length' "$REPORT_JSON")
LOC=$(jq -r '.changes.summary.total_loc_added' "$REPORT_JSON")

cat <<EOF | curl --data-binary @- http://pushgateway:9091/metrics/job/ggen
# TYPE ggen_compilation_duration_ms gauge
ggen_compilation_duration_ms $DURATION
# TYPE ggen_guards_passed gauge
ggen_guards_passed $GUARDS_PASSED
# TYPE ggen_lines_of_code_generated gauge
ggen_lines_of_code_generated $LOC
EOF
```

---

## Troubleshooting

### Issue 1: Report Not Generated

**Symptom**: `./ggen.out/reports/` directory empty.

**Causes**:
1. `[output] report_format = "none"` set
2. Guard failure (compilation stopped before report stage)
3. Permissions issue (can't write to output directory)

**Solutions**:
1. Check ggen.toml: `report_format = "markdown"`
2. Fix guard failures (see guard verdicts)
3. Verify write permissions: `mkdir -p ./ggen.out/reports`

---

### Issue 2: Report Too Large

**Symptom**: Report file > 1 MB.

**Causes**:
1. Many files generated (>1000)
2. Verbose mode enabled
3. Large diffs included

**Solutions**:
1. Use `verbosity = "minimal"`
2. Disable diffs: `[output] emit_diff = false`
3. Filter changes: `[output.sections] changes = false`

---

### Issue 3: JSON Parse Error

**Symptom**: `jq: parse error` when parsing report.

**Causes**:
1. Report incomplete (compilation crashed)
2. Invalid JSON (bug in report generation)

**Solutions**:
1. Check report file exists: `ls -lh ./ggen.out/reports/latest.json`
2. Validate JSON: `jq . ./ggen.out/reports/latest.json`
3. Report bug with reproduction steps

---

## Best Practices

### 1. Always Review Reports in Preview Mode
```bash
ggen sync                  # Preview
cat ./ggen.out/reports/latest.md
ggen sync --no-preview     # Apply after review
```

### 2. Commit Reports with Code
```bash
git add src/generated/ ./ggen.out/reports/ ./ggen.out/receipts/
git commit -m "feat: Generate code (receipt: 7f83b165)"
```

### 3. Automate Report Distribution
```bash
# Add to CI/CD
- Upload report to PR comments
- Send Slack notification
- Export metrics to monitoring
```

### 4. Retention Policy
```bash
# Clean up old reports (keep last 30 days)
ggen cleanup --older-than 30d
```

### 5. Use JSON for Automation
```bash
# Parse JSON report in scripts
LOC=$(jq '.changes.summary.total_loc_added' ./ggen.out/reports/latest.json)
if [ "$LOC" -gt 10000 ]; then
  echo "Warning: Large code generation detected"
fi
```

---

## References

- **Proof-First Compiler**: [docs/PROOF_FIRST_COMPILER.md](./PROOF_FIRST_COMPILER.md)
- **Guard Kernel**: [docs/GUARD_KERNEL.md](./GUARD_KERNEL.md)
- **Receipt Verification**: [docs/RECEIPT_VERIFICATION.md](./RECEIPT_VERIFICATION.md)
- **Migration Guide**: [MIGRATION_GUIDE_V2.1.md](../MIGRATION_GUIDE_V2.1.md)

---

**End of FIRST_LIGHT_REPORT.md**
