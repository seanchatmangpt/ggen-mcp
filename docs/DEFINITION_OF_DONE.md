# Definition of Done (DoD) System

**Version**: 1.0.0 | ggen-mcp Quality Assurance | Profile-Based Validation

## Overview

### What is DoD?

Definition of Done (DoD) = automated quality gate validating compilation readiness. Ensures code meets production standards before deployment through 15+ category-specific checks across 8 domains.

**Core Principle**: Ship-ready code = passes all required checks. Binary verdict: Ready (PASS) or NotReady (FAIL).

### Why DoD?

**Problem**: Manual pre-deployment checklists → human error, inconsistency, forgotten steps.

**Solution**: Automated, profile-based validation system with cryptographic receipts.

**Benefits**:
- **Consistency**: Same checks, every time, no exceptions
- **Speed**: Parallel execution, 2-5 min validation (Fast mode)
- **Evidence**: Cryptographic receipts, audit trails, reproducible results
- **Remediation**: Actionable fix suggestions with automation commands
- **Profiles**: Dev vs Enterprise, customize thresholds

### How DoD Works

```
Input → Profile Selection → Check Execution → Scoring → Verdict → Receipt
```

1. **Profile Selection**: Choose `default-dev` (lenient) or `enterprise-strict` (production)
2. **Check Execution**: Run 15+ checks in parallel with dependency ordering
3. **Scoring**: Weighted category scores → overall readiness score (0-100)
4. **Verdict**: `Ready` (≥ threshold) or `NotReady` (< threshold)
5. **Receipt**: SHA-256 hash, evidence bundles, remediation suggestions

---

## 15 Core Checks (Categories A-H)

### Category C: Tool Registry (WHAT)
**Purpose**: Verify MCP tools match OpenAPI spec.

| Check ID | Description | Severity |
|----------|-------------|----------|
| `TOOL_REGISTRY` | Validates server.rs tool declarations match OpenAPI | Fatal |

**Pass Criteria**: All tools declared in OpenAPI exist in server.rs, signatures match.

**Remediation**: Update server.rs or OpenAPI spec to align tool definitions.

---

### Category D: Build Correctness
**Purpose**: Ensure code compiles and meets Rust standards.

| Check ID | Description | Severity |
|----------|-------------|----------|
| `BUILD_FMT` | `cargo fmt --check` passes | Fatal |
| `BUILD_CLIPPY` | `cargo clippy` warnings < threshold | Warning |
| `BUILD_CHECK` | `cargo check` succeeds | Fatal |

**Pass Criteria**:
- `BUILD_FMT`: Zero formatting violations
- `BUILD_CLIPPY`: Warnings ≤ max_warnings (profile-dependent)
- `BUILD_CHECK`: Compilation succeeds, zero errors

**Remediation**:
```bash
cargo fmt              # Fix formatting
cargo clippy --fix     # Auto-fix clippy issues
cargo check            # Verify compilation
```

---

### Category E: Test Truth
**Purpose**: Validate test coverage and correctness.

| Check ID | Description | Severity |
|----------|-------------|----------|
| `TEST_UNIT` | Unit tests pass | Fatal |
| `TEST_INTEGRATION` | Integration tests pass | Fatal |
| `TEST_SNAPSHOT` | Snapshot tests match golden files | Warning |

**Pass Criteria**:
- `TEST_UNIT`: All unit tests pass
- `TEST_INTEGRATION`: All integration tests pass
- `TEST_SNAPSHOT`: Snapshot diffs acceptable or require update

**Remediation**:
```bash
cargo test                    # Run all tests
cargo test --test name        # Run specific integration test
cargo insta review           # Update snapshots if needed
```

---

### Category F: ggen Pipeline
**Purpose**: Validate ontology-driven code generation.

| Check ID | Description | Severity |
|----------|-------------|----------|
| `GGEN_ONTOLOGY` | Ontology .ttl files valid | Fatal |
| `GGEN_SPARQL` | SPARQL queries execute | Fatal |
| `GGEN_DRY_RUN` | Preview mode succeeds | Fatal |
| `GGEN_RENDER` | Rendered code compiles | Fatal |

**Pass Criteria**:
- `GGEN_ONTOLOGY`: Turtle syntax valid, no parse errors
- `GGEN_SPARQL`: All SPARQL queries execute without errors
- `GGEN_DRY_RUN`: `cargo make sync` preview succeeds
- `GGEN_RENDER`: Generated code has zero TODOs, compiles

**Remediation**:
```bash
cargo make sync              # Preview generation
cargo make sync --no-preview # Apply changes
cargo make sync-validate     # Validate generated code
```

---

### Category G: Safety Invariants
**Purpose**: Enforce security and safety requirements.

| Check ID | Description | Severity |
|----------|-------------|----------|
| `G8_SECRETS` | No exposed secrets (API keys, tokens) | Fatal |
| `G8_LICENSE` | License headers present | Warning |
| `G8_DEPS` | Dependencies pass security audit | Warning |

**Pass Criteria**:
- `G8_SECRETS`: Zero hardcoded credentials
- `G8_LICENSE`: All source files have license headers
- `G8_DEPS`: `cargo audit` clean or known exceptions

**Remediation**:
```bash
# Secrets
git-secrets --scan           # Scan for secrets
# Move secrets to .env and add to .gitignore

# Licenses
# Add SPDX headers to all source files

# Dependencies
cargo audit                  # Check for vulnerabilities
cargo audit fix              # Update vulnerable deps
```

---

### Category H: Deployment Readiness
**Purpose**: Verify production deployment artifacts.

| Check ID | Description | Severity |
|----------|-------------|----------|
| `DEPLOY_RELEASE` | Release build succeeds | Fatal |

**Pass Criteria**:
- `DEPLOY_RELEASE`: `cargo build --release --locked` succeeds

**Remediation**:
```bash
cargo build --release --locked
# Fix any release-specific compilation issues
```

---

## Profiles

### Default Dev Profile (`default-dev`)

**Use Case**: Local development, rapid iteration.

**Thresholds**:
- Min readiness score: 70%
- Max warnings: 20
- Require all tests pass: No
- Fail on clippy warnings: No

**Required Checks**:
- `TOOL_REGISTRY`
- `BUILD_FMT`
- `BUILD_CHECK`
- `TEST_UNIT`
- `GGEN_DRY_RUN`

**Optional Checks**:
- `BUILD_CLIPPY`
- `TEST_INTEGRATION`
- `GGEN_RENDER`

**Parallelism**: Auto (based on CPU cores)

**Timeouts**:
- Build: 10 minutes
- Tests: 15 minutes
- ggen: 5 minutes

---

### Enterprise Strict Profile (`enterprise-strict`)

**Use Case**: Production deployments, CI/CD gates.

**Thresholds**:
- Min readiness score: 90%
- Max warnings: 5
- Require all tests pass: Yes
- Fail on clippy warnings: Yes

**Required Checks** (all 15):
- `TOOL_REGISTRY`
- `BUILD_FMT`, `BUILD_CLIPPY`, `BUILD_CHECK`
- `TEST_UNIT`, `TEST_INTEGRATION`, `TEST_SNAPSHOT`
- `GGEN_ONTOLOGY`, `GGEN_SPARQL`, `GGEN_DRY_RUN`, `GGEN_RENDER`
- `G8_SECRETS`, `G8_LICENSE`, `G8_DEPS`
- `DEPLOY_RELEASE`

**Parallelism**: Auto

**Timeouts**:
- Build: 10 minutes
- Tests: 30 minutes
- ggen: 10 minutes

---

## Usage Examples

### MCP Tool: validate_definition_of_done

**Purpose**: Programmatic DoD validation via MCP interface.

**Request**:
```json
{
  "method": "tools/call",
  "params": {
    "name": "validate_definition_of_done",
    "arguments": {
      "profile": "default-dev",
      "mode": "fast",
      "workspace_root": "/path/to/ggen-mcp"
    }
  }
}
```

**Response**:
```json
{
  "verdict": "Ready",
  "readiness_score": 85.5,
  "profile": "default-dev",
  "mode": "fast",
  "summary": {
    "checks_total": 8,
    "checks_passed": 7,
    "checks_failed": 0,
    "checks_warned": 1,
    "checks_skipped": 0
  },
  "artifacts": {
    "receipt_path": "./ggen.out/dod/receipts/latest.json",
    "report_path": "./ggen.out/dod/reports/latest.md",
    "bundle_path": "./ggen.out/dod/bundles/latest.tar.gz"
  },
  "duration_ms": 125000
}
```

---

### CLI: cargo make validate-dod

**Basic Usage**:
```bash
# Default dev profile
cargo make validate-dod

# Enterprise strict profile
cargo make validate-dod-strict

# Custom profile
cargo make validate-dod PROFILE=custom-profile
```

**Output**:
```
[DoD] Starting validation (profile: default-dev, mode: fast)
[DoD] Running 8 checks...
  ✓ TOOL_REGISTRY (0.8s)
  ✓ BUILD_FMT (1.2s)
  ✓ BUILD_CHECK (5.3s)
  ✓ TEST_UNIT (8.1s)
  ⚠ BUILD_CLIPPY (2.4s) - 3 warnings
  ✓ GGEN_DRY_RUN (3.7s)
[DoD] Readiness Score: 85.5% (threshold: 70%)
[DoD] Verdict: READY ✓
[DoD] Receipt: ./ggen.out/dod/receipts/2026-01-24T12-34-56.json
[DoD] Report: ./ggen.out/dod/reports/2026-01-24T12-34-56.md
```

---

### Programmatic Usage (Rust)

```rust
use ggen_mcp::dod::{CheckExecutor, CheckContext, DodProfile};
use ggen_mcp::dod::checks::create_registry;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create registry with all checks
    let registry = create_registry();

    // Load profile
    let profile = DodProfile::default_dev();

    // Create executor
    let executor = CheckExecutor::new(registry, profile);

    // Create context
    let context = CheckContext::new(PathBuf::from("."))
        .with_timeout(120_000);

    // Execute all checks
    let results = executor.execute_all(&context).await?;

    // Analyze results
    for result in &results {
        println!("{}: {:?}", result.id, result.status);
    }

    Ok(())
}
```

---

## Remediation Guide

### Workflow: Fix → Validate → Ship

1. **Run Validation**:
   ```bash
   cargo make validate-dod
   ```

2. **Review Report**:
   ```bash
   cat ./ggen.out/dod/reports/latest.md
   ```

3. **Fix Issues** (prioritized by severity):
   - **Critical**: Fix immediately (build/test/security)
   - **High**: Fix before merge (clippy warnings)
   - **Medium**: Fix when convenient (docs)
   - **Low**: Nice to have (cosmetic)

4. **Apply Automation**:
   ```bash
   # Report includes automation commands
   cargo fmt
   cargo clippy --fix
   cargo test
   cargo make sync
   ```

5. **Re-validate**:
   ```bash
   cargo make validate-dod
   ```

6. **Verify Receipt**:
   ```bash
   cargo make verify-dod-receipt
   ```

---

### Example Remediation: BUILD_CLIPPY Failed

**Check Output**:
```
✗ BUILD_CLIPPY (2.4s)
  Status: FAIL
  Message: Clippy detected 12 warnings (threshold: 5)
  Evidence: src/server.rs:145: unused variable 'foo'
  Remediation:
    1. Run: cargo clippy --fix
    2. Review and fix remaining warnings
    3. Re-run validation
```

**Fix Steps**:
```bash
# Auto-fix warnings
cargo clippy --fix

# Manual review
cargo clippy -- -D warnings

# Re-validate
cargo make validate-dod
```

---

### Example Remediation: GGEN_RENDER Failed

**Check Output**:
```
✗ GGEN_RENDER (3.7s)
  Status: FAIL
  Message: Generated code contains TODOs
  Evidence: src/generated/mcp_tools.rs:42: TODO: implement validate()
  Remediation:
    1. Update ontology/mcp-domain.ttl
    2. Ensure all generated functions have implementations
    3. Run: cargo make sync --no-preview
    4. Verify: grep -r "TODO" src/generated/
```

**Fix Steps**:
```bash
# Update ontology
vim ontology/mcp-domain.ttl

# Regenerate
cargo make sync

# Review preview
cat ./ggen.out/reports/latest.md

# Apply
cargo make sync --no-preview

# Verify
grep -r "TODO" src/generated/  # Should be empty

# Re-validate
cargo make validate-dod
```

---

## Receipt Verification

### What are Receipts?

Cryptographic proof of DoD validation. Contains:
- SHA-256 hash of all check results
- Evidence bundles (command outputs, file hashes)
- Timestamp, profile, verdict
- Reproducible validation state

### Receipt Structure

```json
{
  "version": "1.0.0",
  "timestamp": "2026-01-24T12:34:56Z",
  "profile": "default-dev",
  "mode": "fast",
  "verdict": "Ready",
  "readiness_score": 85.5,
  "hash": "sha256:abc123...",
  "checks": [
    {
      "id": "BUILD_FMT",
      "status": "Pass",
      "hash": "sha256:def456...",
      "duration_ms": 1200
    }
  ],
  "evidence_bundle": "bundles/2026-01-24T12-34-56.tar.gz"
}
```

### Verify Receipt

```bash
# Verify receipt integrity
cargo make verify-dod-receipt RECEIPT=./ggen.out/dod/receipts/latest.json

# Output:
# ✓ Receipt signature valid
# ✓ All check hashes verified
# ✓ Evidence bundle intact
# ✓ Verdict: Ready (score: 85.5%)
```

---

## Integration with CI/CD

### GitHub Actions

```yaml
name: DoD Validation

on: [push, pull_request]

jobs:
  validate-dod:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Run DoD Validation
        run: cargo make validate-dod-strict

      - name: Upload Receipt
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: dod-receipt
          path: ./ggen.out/dod/receipts/latest.json

      - name: Upload Report
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: dod-report
          path: ./ggen.out/dod/reports/latest.md

      - name: Fail on NotReady
        run: |
          if ! grep -q '"verdict": "Ready"' ./ggen.out/dod/receipts/latest.json; then
            echo "DoD validation failed"
            exit 1
          fi
```

### GitLab CI

```yaml
dod-validation:
  stage: test
  script:
    - cargo make validate-dod-strict
  artifacts:
    when: always
    paths:
      - ./ggen.out/dod/receipts/latest.json
      - ./ggen.out/dod/reports/latest.md
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
```

---

## Troubleshooting

### Issue: Timeouts on Slow CI

**Symptom**: Checks timeout in CI but pass locally.

**Solution**: Increase timeouts in profile.

```toml
# profiles/ci.toml
[timeouts_ms]
build = 1200000  # 20 min
tests = 3600000  # 60 min
ggen = 600000    # 10 min
```

### Issue: False Positives on GGEN_RENDER

**Symptom**: GGEN_RENDER fails on valid generated code.

**Solution**: Ensure preview mode succeeds before applying.

```bash
cargo make sync          # Preview first
# Review ./ggen.out/reports/latest.md
cargo make sync --no-preview  # Apply if preview looks good
```

### Issue: Parallel Execution Flakiness

**Symptom**: Tests pass serially but fail in parallel.

**Solution**: Switch to serial mode or fix race conditions.

```toml
# profiles/serial.toml
[parallelism]
mode = "serial"
```

### Issue: Receipt Verification Fails

**Symptom**: Receipt hash doesn't match evidence.

**Solution**: Re-run validation, don't tamper with artifacts.

```bash
# Re-validate from clean state
git clean -fdx
cargo make validate-dod
```

---

## Category Weights

Readiness score = weighted average of category scores.

**Default Weights**:
- Build Correctness: 25%
- Test Truth: 25%
- ggen Pipeline: 20%
- Tool Registry: 15%
- Safety Invariants: 10%
- Intent Alignment: 5%

**Custom Weights** (profiles/*.toml):
```toml
[category_weights]
BuildCorrectness = 0.30
TestTruth = 0.30
GgenPipeline = 0.20
ToolRegistry = 0.10
SafetyInvariants = 0.10
IntentAlignment = 0.00  # Disable
```

---

## Advanced Usage

### Custom Profiles

**Create**: `profiles/custom.toml`

```toml
name = "custom-profile"
description = "Custom thresholds for feature branch"

required_checks = [
  "BUILD_FMT",
  "BUILD_CHECK",
  "TEST_UNIT",
]

optional_checks = [
  "BUILD_CLIPPY",
  "GGEN_DRY_RUN",
]

[category_weights]
BuildCorrectness = 0.40
TestTruth = 0.40
GgenPipeline = 0.20

[parallelism]
mode = "auto"

[timeouts_ms]
build = 600000
tests = 900000
ggen = 300000
default = 60000

[thresholds]
min_readiness_score = 75.0
max_warnings = 10
require_all_tests_pass = false
fail_on_clippy_warnings = false
```

**Use**:
```bash
cargo make validate-dod PROFILE=custom
```

### Validation Modes

- **Fast**: Skip expensive checks, optimize for speed (2-5 min)
- **Strict**: All checks, normal timeouts (5-10 min)
- **Paranoid**: All checks + extended timeouts + re-runs (10-30 min)

```bash
cargo make validate-dod MODE=fast
cargo make validate-dod MODE=strict
cargo make validate-dod MODE=paranoid
```

---

## Metrics & Observability

### Prometheus Metrics

```
dod_checks_total{profile="default-dev",status="pass"} 7
dod_checks_total{profile="default-dev",status="fail"} 0
dod_checks_total{profile="default-dev",status="warn"} 1
dod_readiness_score{profile="default-dev"} 85.5
dod_duration_seconds{profile="default-dev"} 125.3
```

### Grafana Dashboard

Import `grafana/dod-dashboard.json` for:
- Readiness score trend
- Check failure rates
- Validation duration
- Category score breakdown

---

## References

- **API Documentation**: [DEFINITION_OF_DONE_API.md](DEFINITION_OF_DONE_API.md)
- **Example Code**: [examples/dod_validation.rs](../examples/dod_validation.rs)
- **Source Code**: [src/dod/](../src/dod/)
- **Tests**: [tests/dod_tests.rs](../tests/dod_tests.rs)

---

**Version**: 1.0.0 (2026-01-24)

**Total Documentation**: 1000+ LOC

**SPR Summary**: DoD = automated quality gate. 15 checks, 8 categories, profile-based thresholds. Cryptographic receipts, parallel execution, actionable remediation. Ship-ready code = Ready verdict. Essential for CI/CD, production deployments, TPS compliance.
