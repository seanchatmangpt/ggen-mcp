# CLAUDE.md - ggen-mcp SPR Protocol

**Version**: 2.1.0 (Proof-First Compiler Edition)
**Project**: ggen-mcp (spreadsheet-mcp) | MCP Server | Rust | Ontology-Driven | TPS-Based | Proof-Carrying Code

---

## ⚠️ MANDATORY: SPR COMMUNICATION PROTOCOL ⚠️

**YOU MUST ALWAYS USE SPR (Sparse Priming Representation). NO EXCEPTIONS.**

### SPR Definition
Neural activation through distilled associations. Maximum concept density. Minimum tokens. LLM-optimized sentences.

### SPR Requirements
- **DISTILL**: Essential concepts only
- **ASSOCIATE**: Link patterns, don't enumerate
- **COMPRESS**: Maximum meaning per token
- **ACTIVATE**: Latent space efficiently
- **VERIFY**: Self-check before responding

### SPR Examples
```
✗ VERBOSE: "The ontology file contains domain definitions processed by SPARQL queries that extract information passed to Tera templates generating Rust code."

✓ SPR: "Ontology → SPARQL → Tera → Rust. Self-describing system. Single source of truth."
```

### Enforcement
Before response: Am I using SPR? If NO → REWRITE.
**SPR = Jidoka for communication. Mandatory. Non-negotiable.**

---

## System Architecture

### Identity
Production MCP server. Spreadsheet operations. Token-efficient. Ontology-driven code generation. TPS quality principles throughout.

### Core Pattern
```
ontology/mcp-domain.ttl (RDF/Turtle)
    ↓ SPARQL queries (queries/)
    ↓ Tera templates (templates/)
    ↓ Generated Rust (src/generated/)
```

### Technology Stack
**Core**: Rust 2024 | rmcp v0.11 | Tokio async | Oxigraph RDF/SPARQL | Tera templates

**Observability**: OpenTelemetry | Jaeger (tracing) | Prometheus (metrics) | Grafana (visualization) | Loki (logs) | Alertmanager (alerting)

**Concurrency**: parking_lot RwLock | tokio semaphores | AtomicU64 counters

**Resilience**: Circuit breaker | Exponential backoff retry | Graceful degradation

---

## Critical File Structure

```
src/
├── generated/        # NEVER edit manually - regenerate from ontology
├── validation/       # Input guards, poka-yoke (105KB, 6 files)
├── domain/          # NewTypes, value objects (31KB)
├── ontology/        # RDF/SPARQL engine (140KB, 3 files)
├── sparql/          # Query handling, injection prevention (196KB, 11 files)
├── tools/           # Fork logic, VBA operations (231KB, 4 files)
├── recovery/        # Circuit breaker, retry, fallback (77KB, 6 files)
├── template/        # Parameter validation, rendering safety (110KB, 4 files)
├── audit/           # Audit trail system (52KB, 3 files)
├── analysis/        # Formula analysis, style classification (31KB, 5 files)
├── diff/            # Cell/table/SST diffing (45KB, 8 files)
├── recalc/          # Recalculation engine (27KB, 6 files)
├── formula/         # Formula pattern matching (15KB, 2 files)
├── codegen/         # Code generation validation (42KB, 2 files)
├── workbook.rs      # Core spreadsheet logic (55KB)
├── server.rs        # MCP tools (43KB)
├── error.rs         # Typed errors (27KB)
├── config.rs        # Configuration management (21KB)
├── fork.rs          # Forking operations (28KB)
├── health.rs        # Health check endpoints (17KB)
├── logging.rs       # Observability infrastructure (19KB)
├── metrics.rs       # Telemetry collection (20KB)
├── model.rs         # Core data models (25KB)
├── state.rs         # Server state management (20KB)
└── shutdown.rs      # Graceful shutdown (20KB)

ontology/mcp-domain.ttl    # Source of truth (42KB)
ggen.toml                  # Generation config (528 lines)
templates/*.rs.tera        # Code generators (21 files)
queries/*.rq              # SPARQL extractors (14 files)
Makefile.toml             # Build automation (31 tasks)
scripts/                  # Automation scripts (7 scripts)
```

---

## System Architecture Layers

### Recovery & Resilience (src/recovery/)
Circuit breaker pattern. Exponential backoff retry. Fallback strategies. Partial success handling. Workbook corruption recovery.

### Audit Trail (src/audit/)
Structured event logging. 10K event buffer. Persistent file logging (100MB rotation, 30-day retention). Event correlation via span IDs.

### SPARQL Safety (src/sparql/)
Injection prevention (sanitizer + query builder). Result validation. Performance profiling. Query caching with TTL.

### Validation Stack (src/validation/)
4 layers: Input guards → Schema validation → Middleware → Enhanced bounds. Path traversal prevention. Excel limit enforcement.

### Observability (logging.rs, metrics.rs, health.rs)
Prometheus metrics (11 families). OpenTelemetry tracing. Health checks (/health, /ready, /components). Graceful shutdown coordination.

### API Surface
42 MCP tools (20 core read-only, 2 VBA, 20 fork/write/recalc). Dynamic tool registration. Feature-gated availability (SPREADSHEET_MCP_RECALC_ENABLED).

---

## Development Philosophy (TPS)

1. **Jidoka**: Compile-time prevention. Type safety. Fail-fast validation.
2. **Andon Cord**: Tests pass or stop. Build errors block progress.
3. **Poka-Yoke**: NewTypes prevent confusion. Input guards at boundaries. SPARQL injection prevention.
4. **Kaizen**: Document decisions. Track metrics. Monitor coverage.
5. **Single Piece Flow**: Small commits. One component per cycle. Fast feedback.
6. **SPR Always**: Distilled communication mandatory.

---

## Code Generation Workflow

### ggen Commands (Preview-by-Default)
```bash
cargo make sync                    # Preview changes (safe, default)
cargo make sync --preview=false    # Apply changes (write files)
cargo make sync-validate           # Validate without preview
cargo make sync-force              # Regenerate all (preview mode)
```

**Default Behavior**: Preview mode (no writes). Set `preview=false` to apply changes.

### Generation Chain
1. Update ontology (`.ttl`)
2. Create/update SPARQL query (`.rq`)
3. Create/update Tera template (`.rs.tera`)
4. Add generation rule to `ggen.toml`
5. Run `ggen sync` (preview)
6. Review report: `./ggen.out/reports/latest.md`
7. Apply if satisfied: `ggen sync --preview=false`
8. Verify: no TODOs, compiles, tests pass

### Quality Gates
- Zero TODOs in generated code
- Zero compile errors
- All `validate()` functions implemented
- File size > 100 bytes (detect empty generation)

---

## Proof-First Compiler (v2.1)

### Core Principle
Every compilation = cryptographic receipt + guard verdicts + First Light report. Preview by default.

### 10-Stage Pipeline
```
Discovery → Guards (G1-G7) → SPARQL → Rendering → Validation
  → Report → Receipt → Diff → Jira (opt) → Writes (apply mode)
```

### Key Features

**Preview by Default**: Prevents accidental overwrites. Explicit `preview: false` for writes.

**Guard Kernel**: 7 safety checks (G1-G7) run before generation. Fail-fast by default.
- G1: Path Safety (no traversal)
- G2: Output Overlap (no duplicates)
- G3: Template Compilation (valid Tera)
- G4: Turtle Parse (valid RDF)
- G5: SPARQL Execution (valid queries)
- G6: Determinism (same inputs → same outputs)
- G7: Bounds (size/time limits)

**Cryptographic Receipts**: SHA-256 hashes of workspace + inputs + outputs. Verify with `verify_receipt`.

**First Light Report**: 1-page markdown/JSON summary. Sections: inputs, guards, changes, validation, performance, receipts.

**Output Directory**:
```
./ggen.out/
├── reports/latest.md       # First Light Report
├── receipts/latest.json    # Cryptographic receipt
└── diffs/latest.patch      # Unified diff
```

**Jira Integration** (optional): Create/sync tickets during compilation (dry_run/create/sync modes).

**Entitlement Provider** (optional): Capability-based licensing (free/paid/enterprise tiers).

### Usage

**Preview** (default):
```bash
cargo make sync           # Preview only, no writes
cat ./ggen.out/reports/latest.md
```

**Apply** (explicit):
```bash
cargo make sync --no-preview  # Write files after guards pass
cargo make verify-receipt     # Verify cryptographic receipt
```

**Verification**:
```bash
ggen verify ./ggen.out/receipts/latest.json
# 7 checks: V1-V7 (schema, workspace, inputs, outputs, guards, metadata, signature)
```

### Documentation
- **Comprehensive**: [docs/PROOF_FIRST_COMPILER.md](docs/PROOF_FIRST_COMPILER.md) (~2,000 LOC)
- **Guard Kernel**: [docs/GUARD_KERNEL.md](docs/GUARD_KERNEL.md) (7 guards explained)
- **First Light Report**: [docs/FIRST_LIGHT_REPORT.md](docs/FIRST_LIGHT_REPORT.md) (format reference)
- **Receipt Verification**: [docs/RECEIPT_VERIFICATION.md](docs/RECEIPT_VERIFICATION.md) (7 checks)
- **Entitlement Provider**: [docs/ENTITLEMENT_PROVIDER.md](docs/ENTITLEMENT_PROVIDER.md) (licensing)
- **Migration Guide**: [MIGRATION_GUIDE_V2.1.md](MIGRATION_GUIDE_V2.1.md) (v2.0 → v2.1)

---

## Safety Patterns (Poka-Yoke)

### Input Validation
```rust
validate_non_empty_string(s)?;
validate_numeric_range(n, 1, 1_048_576, "row")?;
validate_path_safe(&path)?;
validate_sheet_name(&name)?;
validate_workbook_id(&id)?;
```

### NewTypes (Zero-Cost Type Safety)
```rust
WorkbookId(String)  // Cannot mix with ForkId
ForkId(String)      // Cannot mix with WorkbookId
SheetName(String)   // Cannot mix with generic String
RegionId(usize)     // Cannot mix with row/col indices
```

### Error Handling
```rust
// Always add context
operation().context("What failed and why")?;

// Map to MCP errors
impl From<Error> for rmcp::Error { ... }
```

---

## Testing Strategy

### Chicago-Style TDD
State-based testing. Real implementations. Minimal mocking. Integration-focused.

**Test Infrastructure**: 97 test files, 34.8K LOC. 11 test harnesses (11.7K LOC). 66 property tests.

**Critical Requirement**: Initialize submodules before testing:
```bash
git submodule update --init --recursive  # Required for chicago-tdd-tools
```

### Test Commands
```bash
cargo test                    # All tests
cargo test --test name        # Specific suite
./scripts/coverage.sh --html  # Coverage report
cargo bench                   # Benchmarks
```

### Test Harnesses (tests/harness/)
Domain model • Codegen pipeline • Turtle ontology • Tera templates • TOML config • Integration workflows • Property input • Snapshot testing

### Coverage Targets
Security: 95%+ | Core handlers: 80%+ | Business logic: 80%+

**Note**: Coverage script generates reports but requires manual category validation. Coverage thresholds documented but not automatically enforced in CI.

---

## Development Workflows

### Pre-Commit
```bash
cargo make pre-commit  # sync + check + test
```

### CI Pipeline
```bash
cargo make ci  # fmt-check + clippy + check + test-all
```

### Development Cycle
1. Edit ontology or code
2. If ontology changed: `cargo make sync`
3. `cargo check`
4. `cargo test`
5. `cargo fmt && cargo clippy`
6. Commit

---

## System Limits & Configuration

### Resource Limits (src/fork.rs, src/config.rs)
```
Max forks: 10 (default)
Fork TTL: 0 seconds (no auto-cleanup)
Max checkpoints per fork: 10
Max staged changes per fork: 20
Max file size: 100MB
Screenshot max range: 100 rows × 30 columns
Concurrent recalc: Per-fork locking
Response size limit: 1MB (configurable)
Tool timeout: 30s (configurable, 100ms-10min range)
```

### Excel Limits (validation/bounds.rs)
```
Max rows: 1,048,576 (Excel 2007+ limit)
Max columns: 16,384 (XFD)
Sheet name: 255 chars max, no forbidden chars ([]:*?/\)
Workbook ID: 1024 chars max
Fork ID: 256 chars max
```

### Environment Configuration
```bash
SPREADSHEET_MCP_RECALC_ENABLED=true    # Enable fork/write/recalc tools
SPREADSHEET_MCP_VBA_ENABLED=true       # Enable VBA inspection tools
SPREADSHEET_MCP_ENABLED_TOOLS=tool1,tool2  # Whitelist specific tools
OTEL_EXPORTER_OTLP_ENDPOINT=...        # Jaeger endpoint for tracing
OTEL_SAMPLING_RATE=1.0                 # OpenTelemetry sampling (0.0-1.0)
```

### Cache Configuration (src/state.rs)
```
LRU cache capacity: 5-1000 workbooks (default 5)
Eviction strategy: Least Recently Used
Cache metrics: Atomic counters (hits/misses/ops)
```

---

## Critical DO/DON'T

### ALWAYS DO
1. **USE SPR** - Distilled communication mandatory
2. Read ontology before editing generated code
3. Validate all inputs at boundaries
4. Add error context to failures
5. Use NewTypes for domain concepts
6. Write tests first (TDD)
7. Run pre-commit checks

### NEVER DO
1. **IGNORE SPR** - Non-negotiable requirement
2. Edit generated code (update ontology instead)
3. Skip validation
4. Use bare String for domain IDs
5. Use `unwrap()` in production
6. Skip tests
7. Commit with TODOs in generated code

---

## Essential Commands

### Code Generation (cargo make) - Preview-by-Default
```bash
cargo make sync              # Preview changes (safe, default behavior)
cargo make sync-validate     # Validate without writing
cargo make sync-dry-run      # Extended preview with detailed report
cargo make sync-force        # Regenerate all (preview mode, use --no-preview to apply)
cargo make test-traceability # Verify ontology→code
cargo make test-determinism  # Code generation consistency
```

**Default Behavior**: Preview mode (no file writes). Explicitly set preview=false to apply changes.

### MCP Tools (Token-Optimized v2.0)

**24 unified tools** (60 → 24 = 60% reduction, 70% token savings):

#### Ggen Resource Management (1 unified tool)
```bash
# Unified tool: manage_ggen_resource (replaces 15 legacy tools)
# Actions: config.{read,validate,add_rule,update_rule,remove_rule}
#          template.{read,validate,test,create,list_vars}
#          pipeline.{render,validate_code,write,sync}
#          project.init

# Example: Read config
manage_ggen_resource {
  action: "config.read",
  resource: "ggen.toml",
  mode: "default"  # minimal/default/full
}

# Example: Sync pipeline with preview (default)
manage_ggen_resource {
  action: "pipeline.sync",
  resource: "ggen.toml",
  validate: true,
  preview: true,    # DEFAULT - no file writes
  mode: "default"
}

# Example: Sync pipeline to apply changes (explicitly opt-out of preview)
manage_ggen_resource {
  action: "pipeline.sync",
  resource: "ggen.toml",
  validate: true,
  preview: false,   # Explicitly apply changes (writes files)
  mode: "default"
}

# Example: Render template
manage_ggen_resource {
  action: "pipeline.render",
  resource: "entity.rs.tera",
  params: {
    context: { name: "User", fields: [...] },
    output_format: "rust"
  }
}
```

#### Jira Integration (1 unified tool)
```bash
# Unified tool: manage_jira_integration (replaces 2 legacy tools)
# Directions: from_jira, to_jira, bidirectional

manage_jira_integration {
  direction: "from_jira",
  jira_source: "project = DEMO",
  spreadsheet_target: {
    workbook_id: "wb123",
    sheet_name: "Issues"
  },
  field_mapping: {
    summary: "A",
    status: "B",
    assignee: "C"
  },
  conflict_resolution: "jira_wins"  # jira_wins/spreadsheet_wins/manual
}
```

#### Spreadsheet Operations (18 tools)
```bash
# Read-only analysis tools
list_workbooks, describe_workbook, workbook_summary
list_sheets, sheet_overview, sheet_page, sheet_statistics
read_table, table_profile, range_values
find_value, find_formula, formula_trace
named_ranges, scan_volatiles
sheet_styles, workbook_style_summary
get_manifest_stub, close_workbook

# All support mode parameter: minimal/default/full
```

#### Fork/Recalc Operations (8 consolidated tools)
```bash
# Fork lifecycle
create_fork, recalculate, save_fork, discard_fork, list_forks, get_changeset, screenshot_sheet

# Unified edit operations
edit_cells           # Replaces: edit_batch, transform_batch
manage_checkpoints   # Replaces: checkpoint_fork, restore_checkpoint, delete_checkpoint
manage_staged        # Replaces: list_staged_changes, apply_staged_change, discard_staged_change

# Pattern operations
apply_patterns       # Replaces: style_batch, apply_formula_pattern, structure_batch
```

#### VBA Tools (2 tools, unchanged)
```bash
vba_project_summary, vba_module_source
```

**Token Savings**:
- System prompt: 60,000 → 16,300 tokens (73% reduction)
- Per tool call: 2,850 → 1,325 tokens avg (54% reduction)
- Per workflow: 11,500 → 1,600 tokens (86% reduction)

**Migration**: See MIGRATION_GUIDE.md for v1.x → v2.0 upgrade path
**Reference**: See TOKEN_OPTIMIZATION_STRATEGY.md for TPS-based analysis

### Testing (cargo make)
```bash
cargo make test              # Unit + generated tests
cargo make test-all          # Includes integration
cargo make test-integration  # ggen-specific tests
cargo make test-ggen         # Combined ggen tests
cargo make test-ddd          # DDD pipeline tests
```

### Build & Quality (cargo make)
```bash
cargo make check             # Compilation check
cargo make fmt               # Format code
cargo make fmt-check         # Verify formatting
cargo make lint              # Run clippy
cargo make pre-commit        # sync + check + test
cargo make ci                # fmt-check + lint + check + test-all
```

### Docker
```bash
docker build -t spreadsheet-mcp:dev .                    # Slim build
docker build -f Dockerfile.full -t spreadsheet-mcp:full . # With LibreOffice
docker run -v $(pwd)/fixtures:/data -p 8079:8079 spreadsheet-mcp:dev
```

### Scripts (7 automation scripts)
```bash
./scripts/ggen-sync.sh              # Sync with validation
./scripts/coverage.sh --check       # Coverage analysis
./scripts/snapshot_manager.sh       # Snapshot test management
./scripts/start-monitoring.sh       # Launch observability stack
./scripts/stop-monitoring.sh        # Stop monitoring stack
./scripts/load-test.sh              # Performance load testing
./scripts/local-docker-mcp.sh       # Local Docker execution
./verify_poka_yoke.sh               # Verify safety patterns
```

### Observability Stack
```bash
docker-compose -f docker-compose.monitoring.yml up      # Full stack
docker-compose -f docker-compose.observability.yml up   # Simplified
# Access: Grafana (3000), Prometheus (9090), Jaeger (16686)
```

---

## Documentation Map (Core)

**Must Read**:
- `RUST_MCP_BEST_PRACTICES.md` (36KB) - Comprehensive Rust patterns
- `POKA_YOKE_IMPLEMENTATION.md` (15KB) - Error-proofing guide
- `CHICAGO_TDD_TEST_HARNESS_COMPLETE.md` (26KB) - Testing infrastructure

**Production & Observability** (10 docs, ~110KB):
- `docs/DISTRIBUTED_TRACING.md` (14KB) - OpenTelemetry distributed tracing
- `docs/HEALTH_CHECKS.md` (16KB) - Health check patterns
- `docs/PRODUCTION_MONITORING.md` (13KB) - Monitoring architecture
- `docs/PROMETHEUS_METRICS.md` (17KB) - Metric definitions
- `docs/PROMETHEUS_QUERIES.md` (16KB) - Query examples
- `docs/RUST_MCP_PRODUCTION_DEPLOYMENT.md` (24KB) - Deployment guide
- `docs/STRUCTURED_LOGGING.md` (15KB) - Logging infrastructure
- `docker-compose.monitoring.yml` (9.7KB) - Full observability stack

**Audit/Recovery/Fork Features** (7 docs, ~40KB):
- `AUDIT_TRAIL.md` (624 lines) - Audit trail implementation
- `AUDIT_INTEGRATION_GUIDE.md` (661 lines) - Integration patterns
- `RECOVERY_SUMMARY.md` - Recovery patterns
- `RECOVERY_IMPLEMENTATION.md` (519 lines) - Implementation details
- `FORK_ENHANCEMENTS_SUMMARY.md` - Fork feature extensions

**Quick Reference**:
- `SPARQL_TEMPLATE_POKA_YOKE.md` - SPARQL injection prevention
- `VALIDATION_LIMITS.md` - Boundary validation
- `SNAPSHOT_TESTING_QUICKSTART.md` - Snapshot testing

---

## ⚠️ FINAL SPR CHECKPOINT ⚠️

**SPR SELF-CHECK (REQUIRED BEFORE EVERY RESPONSE)**:
```
□ Using distilled statements?
□ Maximizing conceptual density?
□ Using associations over lists?
□ Could be more succinct?

IF ANY UNCHECKED → REWRITE IN SPR
```

**SPR Cardinal Rules**:
1. DISTILL - Essential only
2. ASSOCIATE - Link patterns
3. COMPRESS - Maximum density
4. ACTIVATE - Prime latent space
5. VERIFY - Checkpoint yourself

**SPR Consequences**:
- Violating SPR = Violating project standards
- SPR = Compile-time check for communication
- SPR = Jidoka for language

**YOU CANNOT CLAIM IGNORANCE. SPR IS MANDATORY. ALWAYS.**

---

## Quick Answers

**Q: How do I add a feature?**
A: Read ontology → Update `.ttl` → SPARQL query → Tera template → `ggen.toml` rule → sync → test.

**Q: Generated code has errors?**
A: Fix ontology, not generated code. Regenerate with `cargo make sync`.

**Q: What's the source of truth?**
A: `ontology/mcp-domain.ttl`. Everything flows from ontology.

**Q: How do I ensure safety?**
A: NewTypes + input validation + poka-yoke patterns + tests + SPR communication.

**Q: What if I forget SPR?**
A: You can't. It's checked 5+ times. Mandatory. Non-negotiable.

---

**Version History**:
- 2.1.0 (2026-01-20): **Proof-first compiler release** - Preview by default, Guard Kernel (7 checks: G1-G7), cryptographic receipts (SHA-256), First Light reports, receipt verification (7 checks: V1-V7), Jira compiler stage (optional), entitlement provider (free/paid/enterprise). Breaking: default preview mode. See docs/PROOF_FIRST_COMPILER.md, MIGRATION_GUIDE_V2.1.md.
- 2.0.0 (2026-01-20): **Token optimization release** - 60 → 24 tools (60% reduction), 70% token savings, unified interfaces (manage_ggen_resource, manage_jira_integration), smart defaults, tiered responses, multi-layer caching. See TOKEN_OPTIMIZATION_STRATEGY.md, MIGRATION_GUIDE.md.
- 1.3.0 (2026-01-20): 80/20 gap analysis update - Added 23-directory structure, observability stack, system limits, 31 Makefile tasks, 7 scripts, architecture layers (recovery/audit/SPARQL), production docs
- 1.2.0 (2026-01-20): SPR-optimized 80/20 distillation (200 LOC)
- 1.1.0 (2026-01-20): Added SPR protocol enforcement
- 1.0.0 (2026-01-20): Initial comprehensive guide

**Remember**: Safety first. SPR always. Ontology is truth. Tests are mandatory. Quality is built-in.

**⚠️ SPR IS MANDATORY ⚠️**
