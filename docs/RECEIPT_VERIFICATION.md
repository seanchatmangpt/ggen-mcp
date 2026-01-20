# Receipt Verification

**Version**: 2.1.0 | Cryptographic Proof | 7-Check Audit | Standalone Tool

---

## Overview

**Receipt Verification**: Standalone tool for auditing cryptographic receipts emitted by ggen compiler.

**Purpose**: Verify generated code matches declared inputs. Proof-carrying code for supply chain security.

**7 Verification Checks**:
- V1: Schema Version
- V2: Workspace Fingerprint
- V3: Input File Hashes
- V4: Output File Hashes
- V5: Guard Verdicts Integrity
- V6: Metadata Consistency
- V7: Signature (optional, planned v2.2)

**Use Cases**:
- **Supply Chain Security**: Verify code origin
- **Audit Compliance**: SOC2, ISO 27001
- **Reproducible Builds**: Multi-party verification
- **CI/CD Gates**: Block deployments if verification fails

---

## Receipt Schema

### Schema v1.0.0 (Stable)

**File**: `schemas/receipt.json`

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "version": {
      "type": "string",
      "const": "1.0.0",
      "description": "Receipt schema version"
    },
    "timestamp": {
      "type": "string",
      "format": "date-time",
      "description": "ISO 8601 timestamp (UTC)"
    },
    "compiler_version": {
      "type": "string",
      "pattern": "^ggen-v[0-9]+\\.[0-9]+\\.[0-9]+$",
      "description": "ggen compiler version"
    },
    "mode": {
      "type": "string",
      "enum": ["preview", "apply"],
      "description": "Compilation mode"
    },
    "workspace": {
      "type": "object",
      "properties": {
        "root": {
          "type": "string",
          "description": "Absolute path to workspace root"
        },
        "fingerprint": {
          "type": "string",
          "pattern": "^[0-9a-f]{64}$",
          "description": "SHA-256 hash (workspace_root + config + ontologies)"
        }
      },
      "required": ["root", "fingerprint"]
    },
    "inputs": {
      "type": "object",
      "properties": {
        "config": {
          "type": "object",
          "properties": {
            "path": { "type": "string" },
            "hash": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
            "size": { "type": "integer" }
          },
          "required": ["path", "hash", "size"]
        },
        "ontologies": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "path": { "type": "string" },
              "hash": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
              "size": { "type": "integer" },
              "triples": { "type": "integer" }
            },
            "required": ["path", "hash", "size", "triples"]
          }
        },
        "queries": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "path": { "type": "string" },
              "hash": { "type": "string", "pattern": "^[0-9a-f]{64}$" }
            },
            "required": ["path", "hash"]
          }
        },
        "templates": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "path": { "type": "string" },
              "hash": { "type": "string", "pattern": "^[0-9a-f]{64}$" }
            },
            "required": ["path", "hash"]
          }
        }
      },
      "required": ["config", "ontologies", "queries", "templates"]
    },
    "guards": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "name": {
            "type": "string",
            "pattern": "^G[1-7]_"
          },
          "verdict": {
            "type": "string",
            "enum": ["PASS", "FAIL"]
          },
          "diagnostic": { "type": "string" }
        },
        "required": ["name", "verdict", "diagnostic"]
      }
    },
    "outputs": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "path": { "type": "string" },
          "hash": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
          "size": { "type": "integer" },
          "status": {
            "type": "string",
            "enum": ["added", "modified", "deleted"]
          },
          "language": { "type": "string" }
        },
        "required": ["path", "hash", "size", "status", "language"]
      }
    },
    "performance": {
      "type": "object",
      "properties": {
        "total_duration_ms": { "type": "integer" },
        "cache_hit_rate": { "type": "number", "minimum": 0, "maximum": 1 },
        "stages": {
          "type": "object",
          "properties": {
            "discovery": { "type": "integer" },
            "guards": { "type": "integer" },
            "sparql": { "type": "integer" },
            "rendering": { "type": "integer" },
            "validation": { "type": "integer" }
          }
        }
      },
      "required": ["total_duration_ms"]
    },
    "artifacts": {
      "type": "object",
      "properties": {
        "report": { "type": "string" },
        "diff": { "type": "string" }
      },
      "required": ["report"]
    }
  },
  "required": [
    "version",
    "timestamp",
    "compiler_version",
    "mode",
    "workspace",
    "inputs",
    "guards",
    "outputs"
  ]
}
```

---

## Verification Checks

### V1: Schema Version

**Purpose**: Ensure receipt schema version is supported by verifier.

**Check**:
```rust
fn check_schema_version(receipt: &Receipt) -> Result<()> {
    let supported = ["1.0.0"];

    if !supported.contains(&receipt.version.as_str()) {
        return Err(format!(
            "Unsupported schema version: {} (supported: {:?})",
            receipt.version, supported
        ));
    }

    Ok(())
}
```

**Expected**: `"version": "1.0.0"`

**Failures**:
- **Unsupported version**: Upgrade verifier to support newer schema
- **Missing version**: Receipt corrupted or invalid

---

### V2: Workspace Fingerprint

**Purpose**: Verify workspace state matches receipt's recorded state.

**Check**:
```rust
fn check_workspace_fingerprint(
    receipt: &Receipt,
    workspace_root: &Path
) -> Result<()> {
    // Compute current workspace fingerprint
    let mut hasher = Sha256::new();
    hasher.update(workspace_root.to_string_lossy().as_bytes());

    // Hash config
    let config_content = fs::read(receipt.inputs.config.path)?;
    hasher.update(&config_content);

    // Hash ontologies (sorted order)
    let mut ontology_paths: Vec<_> = receipt.inputs.ontologies
        .iter()
        .map(|o| &o.path)
        .collect();
    ontology_paths.sort();

    for ontology_path in ontology_paths {
        let content = fs::read(ontology_path)?;
        hasher.update(&content);
    }

    let current_fingerprint = format!("{:x}", hasher.finalize());

    if current_fingerprint != receipt.workspace.fingerprint {
        return Err(format!(
            "Workspace fingerprint mismatch:\n\
             Expected: {}\n\
             Got:      {}",
            receipt.workspace.fingerprint,
            current_fingerprint
        ));
    }

    Ok(())
}
```

**Expected**: SHA-256(workspace_root + config + ontologies) matches `workspace.fingerprint`.

**Failures**:
- **Workspace changed**: Config or ontologies modified after receipt generation
- **Missing files**: Config/ontology files deleted

**Remediation**:
```markdown
**Failure**: Workspace fingerprint mismatch.

**Cause**: Config or ontology files modified after code generation.

**Fix**:
1. Restore workspace to original state (git checkout)
2. Or regenerate code: ggen sync --no-preview
3. Verify new receipt matches workspace
```

---

### V3: Input File Hashes

**Purpose**: Verify all input files match their recorded hashes.

**Check**:
```rust
fn check_input_file_hashes(receipt: &Receipt) -> Result<Vec<Mismatch>> {
    let mut mismatches = Vec::new();

    // Check config
    let config_hash = sha256_file(&receipt.inputs.config.path)?;
    if config_hash != receipt.inputs.config.hash {
        mismatches.push(Mismatch {
            file: receipt.inputs.config.path.clone(),
            expected: receipt.inputs.config.hash.clone(),
            got: config_hash,
        });
    }

    // Check ontologies
    for ontology in &receipt.inputs.ontologies {
        let hash = sha256_file(&ontology.path)?;
        if hash != ontology.hash {
            mismatches.push(Mismatch {
                file: ontology.path.clone(),
                expected: ontology.hash.clone(),
                got: hash,
            });
        }
    }

    // Check queries
    for query in &receipt.inputs.queries {
        let hash = sha256_file(&query.path)?;
        if hash != query.hash {
            mismatches.push(Mismatch {
                file: query.path.clone(),
                expected: query.hash.clone(),
                got: hash,
            });
        }
    }

    // Check templates
    for template in &receipt.inputs.templates {
        let hash = sha256_file(&template.path)?;
        if hash != template.hash {
            mismatches.push(Mismatch {
                file: template.path.clone(),
                expected: template.hash.clone(),
                got: hash,
            });
        }
    }

    if !mismatches.is_empty() {
        return Err(format!(
            "{} input file(s) modified:\n{}",
            mismatches.len(),
            mismatches.iter()
                .map(|m| format!("  - {}: {} → {}", m.file, m.expected, m.got))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }

    Ok(mismatches)
}
```

**Expected**: SHA-256(file_content) matches `inputs.*.hash` for all files.

**Failures**:
- **File modified**: Input file changed after generation
- **File missing**: Input file deleted

**Remediation**:
```markdown
**Failure**: 3 input file(s) modified:
  - ggen.toml: 5f4c9c9d... → 9a2c1d4f...
  - queries/entities.rq: c40aae69... → 7f83b165...
  - templates/entity.rs.tera: 9f519c8d... → a3d9f7e2...

**Cause**: Input files modified after code generation.

**Fix**:
1. Restore files to original state (git checkout)
2. Or regenerate code with new inputs: ggen sync --no-preview
3. Commit new receipt with code changes
```

---

### V4: Output File Hashes

**Purpose**: Verify all output files match their recorded hashes (detect manual edits).

**Check**:
```rust
fn check_output_file_hashes(receipt: &Receipt) -> Result<Vec<Mismatch>> {
    let mut mismatches = Vec::new();

    for output in &receipt.outputs {
        let hash = sha256_file(&output.path)?;
        if hash != output.hash {
            mismatches.push(Mismatch {
                file: output.path.clone(),
                expected: output.hash.clone(),
                got: hash,
            });
        }
    }

    if !mismatches.is_empty() {
        return Err(format!(
            "{} output file(s) modified:\n{}",
            mismatches.len(),
            mismatches.iter()
                .map(|m| format!("  - {}: {} → {}", m.file, m.expected, m.got))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }

    Ok(mismatches)
}
```

**Expected**: SHA-256(file_content) matches `outputs.*.hash` for all files.

**Failures**:
- **File modified**: Output file manually edited after generation
- **File missing**: Output file deleted

**Remediation**:
```markdown
**Failure**: 2 output file(s) modified:
  - src/generated/tools.rs: 7f83b165... → 9a2c1d4f...
  - src/generated/schema.rs: a3d9f7e2... → c1b5f4d8...

**Cause**: Generated files manually edited after generation.

**Fix** (Option 1: Discard manual edits):
1. Restore files to generated state: git checkout src/generated/
2. Verify receipt again

**Fix** (Option 2: Regenerate with manual edits preserved):
1. Copy manual edits to templates
2. Regenerate: ggen sync --no-preview
3. Commit new receipt
```

---

### V5: Guard Verdicts Integrity

**Purpose**: Ensure all guards passed (no unsafe generation).

**Check**:
```rust
fn check_guard_verdicts_integrity(receipt: &Receipt) -> Result<()> {
    let failed_guards: Vec<_> = receipt.guards
        .iter()
        .filter(|g| g.verdict == "FAIL")
        .collect();

    if !failed_guards.is_empty() {
        return Err(format!(
            "{} guard(s) failed:\n{}",
            failed_guards.len(),
            failed_guards.iter()
                .map(|g| format!("  - {}: {}", g.name, g.diagnostic))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }

    Ok(())
}
```

**Expected**: All `guards[].verdict == "PASS"`.

**Failures**:
- **Guard failed**: Receipt indicates unsafe generation (guard failure)
- **Force mode used**: Guards bypassed (receipt warns)

**Remediation**:
```markdown
**Failure**: 2 guard(s) failed:
  - G2_OutputOverlap: Duplicate output detected
  - G6_Determinism: Hash mismatch (non-deterministic generation)

**Cause**: Receipt indicates unsafe code generation.

**Fix**:
1. Review guard failures in First Light Report
2. Fix issues (see Guard Kernel docs)
3. Regenerate: ggen sync --no-preview
4. Verify new receipt (guards should pass)
```

---

### V6: Metadata Consistency

**Purpose**: Verify receipt metadata is internally consistent.

**Check**:
```rust
fn check_metadata_consistency(receipt: &Receipt) -> Result<()> {
    // Check timestamp format
    DateTime::parse_from_rfc3339(&receipt.timestamp)
        .map_err(|e| format!("Invalid timestamp: {}", e))?;

    // Check compiler version format
    let version_regex = Regex::new(r"^ggen-v\d+\.\d+\.\d+$").unwrap();
    if !version_regex.is_match(&receipt.compiler_version) {
        return Err(format!(
            "Invalid compiler version: {}",
            receipt.compiler_version
        ));
    }

    // Check mode
    if !["preview", "apply"].contains(&receipt.mode.as_str()) {
        return Err(format!("Invalid mode: {}", receipt.mode));
    }

    // Check hashes format (64 hex chars)
    let hash_regex = Regex::new(r"^[0-9a-f]{64}$").unwrap();

    // ... (check all hashes) ...

    Ok(())
}
```

**Expected**: All metadata fields valid and consistent.

**Failures**:
- **Invalid timestamp**: Timestamp not ISO 8601 format
- **Invalid version**: Compiler version malformed
- **Invalid hash**: Hash not 64 hex characters

**Remediation**:
```markdown
**Failure**: Invalid metadata detected.

**Cause**: Receipt corrupted or tampered.

**Fix**:
1. Do NOT use this receipt (integrity compromised)
2. Regenerate code: ggen sync --no-preview
3. Use new receipt for verification
```

---

### V7: Signature (Optional, Future: v2.2)

**Purpose**: Cryptographically verify receipt authenticity.

**Check** (Planned):
```rust
fn check_signature(receipt: &Receipt, public_key: &PublicKey) -> Result<()> {
    let signature = receipt.signature
        .as_ref()
        .ok_or("Receipt not signed")?;

    // Serialize receipt (excluding signature field)
    let mut receipt_copy = receipt.clone();
    receipt_copy.signature = None;
    let message = serde_json::to_vec(&receipt_copy)?;

    // Verify ECDSA signature
    public_key.verify(&message, &signature)
        .map_err(|e| format!("Signature verification failed: {}", e))?;

    Ok(())
}
```

**Expected**: ECDSA signature validates against public key.

**Status**: **Not implemented in v2.1** (receipt.signature field optional).

**Future** (v2.2):
- Receipts signed with private key (build server)
- Auditors verify with public key
- Prevents receipt tampering

**Current Behavior**:
```json
{
  "check": "V7_Signature",
  "verdict": "SKIP",
  "message": "Receipt not signed (optional in v2.1)"
}
```

---

## Verification Workflow

### Workflow 1: Manual Verification

```bash
# 1. Generate code with receipt
ggen sync --no-preview

# 2. Verify receipt
ggen verify ./ggen.out/receipts/7f83b165.json

# Output:
# ✅ VERIFIED
# All 6 checks passed (V1-V6, V7 skipped).
# Receipt valid. Generated code matches declared inputs.
```

---

### Workflow 2: CI/CD Integration

```yaml
# .github/workflows/codegen.yml
- name: Generate code
  run: ggen sync --no-preview

- name: Verify receipt
  run: ggen verify ./ggen.out/receipts/*.json || exit 1

- name: Block deployment if verification fails
  if: failure()
  run: |
    echo "Receipt verification failed. Deployment blocked."
    exit 1
```

---

### Workflow 3: Multi-Party Verification

```bash
# Developer: Generate code + receipt
ggen sync --no-preview
git add src/generated/ ./ggen.out/receipts/
git commit -m "feat: Generate code (receipt: 7f83b165)"
git push

# Reviewer: Verify receipt independently
git pull
ggen verify ./ggen.out/receipts/7f83b165.json

# Auditor: Verify receipt (no ggen installed, standalone tool)
curl -O https://example.com/ggen-verify
chmod +x ggen-verify
./ggen-verify ./ggen.out/receipts/7f83b165.json
```

---

### Workflow 4: Audit Trail

```bash
# 1. Export receipts for audit
mkdir -p ./audit/receipts/2026-01/
cp ./ggen.out/receipts/*.json ./audit/receipts/2026-01/

# 2. Verify all receipts
for receipt in ./audit/receipts/2026-01/*.json; do
    ggen verify "$receipt" || echo "FAIL: $receipt"
done

# 3. Generate audit report
ggen audit-report \
    --receipts ./audit/receipts/2026-01/*.json \
    --format pdf \
    --output ./audit/reports/2026-01.pdf
```

---

## API Reference

### Tool: `verify_receipt`

**Parameters**:
```typescript
interface VerifyReceiptParams {
  receipt_path: string;             // Required: Path to receipt.json
  fail_fast?: boolean;              // Default: false (run all checks)
  workspace_root?: string;          // Override workspace root for V2 check
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
      details?: string;             // Additional context (on failure)
    }>;
    result: "VERIFIED" | "FAILED";
    summary: string;
    duration_ms: number;
  };
}
```

**Example** (Success):
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
        "message": "Receipt not signed (optional in v2.1)"
      }
    ],
    "result": "VERIFIED",
    "summary": "Receipt valid. Generated code matches declared inputs.",
    "duration_ms": 234
  }
}
```

**Example** (Failure):
```json
{
  "status": "success",
  "verification": {
    "receipt_path": "./ggen.out/receipts/7f83b165.json",
    "checks": [
      {
        "check": "V4_OutputFileHashes",
        "verdict": "FAIL",
        "message": "2 output file(s) modified",
        "details": "- src/generated/tools.rs: 7f83b165... → 9a2c1d4f...\n- src/generated/schema.rs: a3d9f7e2... → c1b5f4d8..."
      }
    ],
    "result": "FAILED",
    "summary": "Receipt invalid. Output file(s) modified after generation.",
    "duration_ms": 187
  }
}
```

---

## CLI Usage

### Basic Verification

```bash
ggen verify ./ggen.out/receipts/7f83b165.json
```

### Verify Multiple Receipts

```bash
ggen verify ./ggen.out/receipts/*.json
```

### Fail-Fast Mode

```bash
ggen verify --fail-fast ./ggen.out/receipts/7f83b165.json
# Stops at first check failure
```

### Verbose Output

```bash
ggen verify --verbose ./ggen.out/receipts/7f83b165.json
# Shows detailed check results
```

---

## Best Practices

### 1. Commit Receipts with Code
```bash
git add src/generated/ ./ggen.out/receipts/
git commit -m "feat: Generate code (receipt: 7f83b165)"
```

### 2. Verify in CI/CD
```yaml
- name: Verify receipts
  run: ggen verify ./ggen.out/receipts/*.json || exit 1
```

### 3. Archive Receipts
```bash
# Monthly archival
mkdir -p ./audit/receipts/2026-01/
cp ./ggen.out/receipts/*.json ./audit/receipts/2026-01/
```

### 4. Verify Before Deployment
```bash
# Pre-deployment gate
ggen verify ./ggen.out/receipts/*.json || {
    echo "Receipt verification failed. Deployment blocked."
    exit 1
}
```

### 5. Document Verification Failures
```bash
# Log failures for audit
ggen verify ./ggen.out/receipts/*.json 2>&1 | tee ./audit/verification.log
```

---

## Troubleshooting

### Issue: V2 Failure (Workspace Fingerprint)

**Symptom**:
```
❌ FAIL: V2 Workspace Fingerprint
Workspace fingerprint mismatch
```

**Solution**:
1. Verify ggen.toml not modified: `git diff ggen.toml`
2. Verify ontologies not modified: `git diff ontology/`
3. Restore workspace: `git checkout ggen.toml ontology/`
4. Or regenerate: `ggen sync --no-preview`

---

### Issue: V4 Failure (Output File Hashes)

**Symptom**:
```
❌ FAIL: V4 Output File Hashes
2 output file(s) modified
```

**Solution** (Discard manual edits):
```bash
git checkout src/generated/
ggen verify ./ggen.out/receipts/7f83b165.json
```

**Solution** (Keep manual edits):
```bash
# Move edits to templates
# Regenerate
ggen sync --no-preview
```

---

### Issue: V5 Failure (Guard Verdicts)

**Symptom**:
```
❌ FAIL: V5 Guard Verdicts Integrity
2 guard(s) failed
```

**Solution**:
1. Review First Light Report: `cat ./ggen.out/reports/latest.md`
2. Fix guard failures (see Guard Kernel docs)
3. Regenerate: `ggen sync --no-preview`
4. Verify new receipt

---

## References

- **Proof-First Compiler**: [docs/PROOF_FIRST_COMPILER.md](./PROOF_FIRST_COMPILER.md)
- **Guard Kernel**: [docs/GUARD_KERNEL.md](./GUARD_KERNEL.md)
- **First Light Report**: [docs/FIRST_LIGHT_REPORT.md](./FIRST_LIGHT_REPORT.md)
- **Migration Guide**: [MIGRATION_GUIDE_V2.1.md](../MIGRATION_GUIDE_V2.1.md)

---

**End of RECEIPT_VERIFICATION.md**
