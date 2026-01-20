# CLAUDE.md - ggen-mcp SPR Protocol

**Version**: 2.1.0 | ggen-mcp | MCP Server | Rust | Ontology-Driven | TPS | Proof-Carrying Code

## ⚠️ MANDATORY: SPR (Sparse Priming Representation) ⚠️

**Neural activation through distilled associations. Maximum density. Minimum tokens.**

**Requirements**: DISTILL essential only • ASSOCIATE patterns • COMPRESS meaning • ACTIVATE latent space • VERIFY self-check

**Example**: ✗ "Ontology contains definitions processed by SPARQL extracting info for Tera templates generating Rust" ✓ "Ontology → SPARQL → Tera → Rust. Single truth."

**Enforcement**: Before response: Using SPR? If NO → REWRITE. Non-negotiable.

## Architecture

**Identity**: Production MCP server. Spreadsheet ops. Ontology-driven codegen. TPS principles.

**Core Flow**: `ontology/mcp-domain.ttl` → SPARQL (`queries/`) → Tera (`templates/`) → Rust (`src/generated/`)

**Stack**: Rust 2024 | rmcp 0.11 | Tokio | Oxigraph (SPARQL) | Tera | parking_lot | OpenTelemetry | Prometheus

**Key Modules** (46K LOC):
- `generated/` - NEVER edit manually, regenerate from ontology
- `validation/` (105KB) - Input guards, poka-yoke, path safety
- `sparql/` (196KB) - Injection prevention, query builder
- `recovery/` (77KB) - Circuit breaker, retry, fallback
- `audit/` (52KB) - 10K buffer, 100MB rotation, 30d retention
- `tools/` (231KB) - Fork/VBA ops
- `server.rs` (59KB) - 24 MCP tools
- `workbook.rs` (55KB) - Core spreadsheet logic

**Config**: `ontology/mcp-domain.ttl` (42KB truth) • `ggen.toml` (528 lines) • 21 templates • 14 SPARQL queries • 31 Makefile tasks

**Layers**: Validation (4-layer) → SPARQL safety → Recovery (circuit breaker) → Audit trail → Observability (Prometheus/Jaeger/Grafana)

## TPS Philosophy

**Jidoka**: Compile-time prevention, type safety, fail-fast • **Andon Cord**: Tests/builds block on error • **Poka-Yoke**: NewTypes, input guards, SPARQL injection prevention • **Kaizen**: Document, measure, iterate • **Single Piece Flow**: Small commits, fast feedback • **SPR Always**: Mandatory

## Code Generation (Preview-by-Default)

**Commands**: `cargo make sync` (preview) • `sync --preview=false` (apply) • `sync-validate` • `sync-force`

**Chain**: Edit `.ttl` → SPARQL `.rq` → Tera `.rs.tera` → Add `ggen.toml` rule → `sync` (preview) → Review `./ggen.out/reports/latest.md` → Apply → Verify

**Gates**: Zero TODOs • Zero errors • All `validate()` implemented • Size > 100 bytes

## Proof-First Compiler v2.1

**Principle**: Every compilation = cryptographic receipt (SHA-256) + Guard Kernel (G1-G7) + First Light report. Preview-by-default.

**Pipeline**: Discovery → Guards → SPARQL → Render → Validate → Report → Receipt → Diff → Jira (opt) → Writes

**Guard Kernel (G1-G7)**: Path safety • Output overlap • Template compile • Turtle parse • SPARQL exec • Determinism • Bounds

**Outputs**: `./ggen.out/reports/latest.md` (First Light) • `receipts/latest.json` (SHA-256) • `diffs/latest.patch`

**Usage**: `sync` (preview) • `sync --no-preview` (apply) • `verify-receipt` • `ggen verify receipts/latest.json` (V1-V7 checks)

**Extras**: Jira integration (dry_run/create/sync) • Entitlement provider (free/paid/enterprise)

**Docs**: `PROOF_FIRST_COMPILER.md` • `GUARD_KERNEL.md` • `FIRST_LIGHT_REPORT.md` • `RECEIPT_VERIFICATION.md` • `MIGRATION_GUIDE_V2.1.md`

## Safety (Poka-Yoke)

**Validation**: `validate_non_empty_string(s)?` • `validate_numeric_range(n, 1, 1_048_576, "row")?` • `validate_path_safe(&path)?`

**NewTypes**: `WorkbookId(String)` ≠ `ForkId(String)` ≠ `SheetName(String)` - Zero-cost type safety

**Errors**: `operation().context("what failed")?` → `impl From<Error> for rmcp::Error`

## Testing (Chicago-TDD)

**Infrastructure**: 86 test files, 47K LOC • 11 harnesses (11.7K LOC) • 66 property tests • State-based, real implementations

**Setup**: `git submodule update --init --recursive` (chicago-tdd-tools required)

**Commands**: `cargo test` • `test --test name` • `./scripts/coverage.sh --html` • `cargo bench`

**Targets**: Security 95%+ | Core 80%+ | Business logic 80%+

## Workflows

**Pre-commit**: `cargo make pre-commit` (sync + check + test)

**CI**: `cargo make ci` (fmt-check + clippy + check + test-all)

**Cycle**: Edit ontology/code → `sync` (if .ttl changed) → `check` → `test` → `fmt && clippy` → Commit

## Limits & Config

**Resources**: Forks 10 max • Checkpoints 10/fork • Staged 20/fork • File 100MB • Screenshot 100×30 • Response 1MB • Timeout 30s

**Excel**: Rows 1,048,576 • Cols 16,384 (XFD) • Sheet name 255 chars • Workbook ID 1024 • Fork ID 256

**Env**: `SPREADSHEET_MCP_RECALC_ENABLED` • `_VBA_ENABLED` • `_ENABLED_TOOLS` • `OTEL_EXPORTER_OTLP_ENDPOINT` • `OTEL_SAMPLING_RATE`

**Cache**: LRU 5-1000 workbooks (default 5) • Atomic counters

## Critical Rules

**ALWAYS**: USE SPR • Read ontology before edits • Validate inputs • Add error context • Use NewTypes • TDD • Pre-commit checks

**NEVER**: Ignore SPR • Edit `generated/` (update ontology) • Skip validation • Bare String IDs • `unwrap()` • Skip tests • Commit TODOs

## Commands

**Codegen**: `sync` (preview) • `sync --preview=false` (apply) • `sync-validate` • `sync-force` • `test-traceability` • `test-determinism`

**MCP Tools (24 unified, 60→24 = 60% reduction, 70% token savings)**:
- `manage_ggen_resource`: config.{read,validate,add_rule,update_rule,remove_rule} • template.{read,validate,test,create,list_vars} • pipeline.{render,validate_code,write,sync} • project.init
- `manage_jira_integration`: from_jira/to_jira/bidirectional sync
- **18 spreadsheet**: list_workbooks, describe_workbook, list_sheets, sheet_overview, read_table, range_values, find_value, find_formula, formula_trace, named_ranges, scan_volatiles, sheet_styles, etc. (mode: minimal/default/full)
- **8 fork**: create_fork, recalculate, save_fork, discard_fork, list_forks, edit_cells, manage_checkpoints, apply_patterns
- **2 VBA**: vba_project_summary, vba_module_source

**Test**: `test` • `test-all` • `test-integration` • `test-ggen` • `test-ddd`

**Build**: `check` • `fmt` • `fmt-check` • `lint` • `pre-commit` • `ci`

**Docker**: `docker build -t spreadsheet-mcp:dev .` • `Dockerfile.full` (LibreOffice) • `docker run -v $(pwd)/fixtures:/data -p 8079:8079`

**Scripts**: `ggen-sync.sh` • `coverage.sh --check` • `snapshot_manager.sh` • `start-monitoring.sh` • `stop-monitoring.sh` • `load-test.sh`

**Observability**: `docker-compose -f docker-compose.monitoring.yml up` (Grafana:3000, Prometheus:9090, Jaeger:16686)

## Documentation

**Must Read**: `RUST_MCP_BEST_PRACTICES.md` (36KB) • `POKA_YOKE_IMPLEMENTATION.md` (15KB) • `CHICAGO_TDD_TEST_HARNESS_COMPLETE.md` (26KB)

**Production**: `docs/{DISTRIBUTED_TRACING,HEALTH_CHECKS,PRODUCTION_MONITORING,PROMETHEUS_METRICS,RUST_MCP_PRODUCTION_DEPLOYMENT,STRUCTURED_LOGGING}.md` (~110KB)

**Features**: `{AUDIT_TRAIL,AUDIT_INTEGRATION_GUIDE,RECOVERY_SUMMARY,RECOVERY_IMPLEMENTATION,FORK_ENHANCEMENTS_SUMMARY}.md` (~40KB)

**Quick Ref**: `SPARQL_TEMPLATE_POKA_YOKE.md` • `VALIDATION_LIMITS.md` • `SNAPSHOT_TESTING_QUICKSTART.md`

## ⚠️ SPR CHECKPOINT ⚠️

**Self-check before EVERY response**: Distilled? Dense? Associated? Succinct? IF NO → REWRITE

**Rules**: DISTILL essential • ASSOCIATE patterns • COMPRESS density • ACTIVATE latent • VERIFY self-check

**Consequences**: Violating SPR = violating standards. SPR = compile-time check. NON-NEGOTIABLE.

## Quick Answers

**Add feature?** Read ontology → Update `.ttl` → SPARQL → Tera → `ggen.toml` → sync → test

**Generated errors?** Fix ontology, not code. Regenerate: `cargo make sync`

**Source of truth?** `ontology/mcp-domain.ttl`. All flows from ontology.

**Ensure safety?** NewTypes + validation + poka-yoke + tests + SPR

**Forget SPR?** Impossible. Checked 5+ times. Mandatory.

---

**Version**: 2.1.0 (2026-01-20) - Proof-first compiler (Guard Kernel G1-G7, SHA-256 receipts, First Light reports, Jira integration, entitlement provider) | 2.0.0 - Token optimization (60→24 tools, 70% savings) | See `PROOF_FIRST_COMPILER.md`, `MIGRATION_GUIDE_V2.1.md`, `TOKEN_OPTIMIZATION_STRATEGY.md`

**Remember**: Safety first. SPR always. Ontology = truth. Tests mandatory. Quality built-in. **⚠️ SPR IS MANDATORY ⚠️**
