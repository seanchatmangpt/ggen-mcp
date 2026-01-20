# CLAUDE.md - ggen-mcp SPR Protocol

**Version**: 1.2.0 (SPR-Optimized 80/20)
**Project**: ggen-mcp (spreadsheet-mcp) | MCP Server | Rust | Ontology-Driven

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

### ggen Commands
```bash
cargo make sync           # Generate from ontology
cargo make sync-validate  # Check without writing
cargo make sync-dry-run   # Preview changes
cargo make sync-force     # Regenerate all
```

### Generation Chain
1. Update ontology (`.ttl`)
2. Create/update SPARQL query (`.rq`)
3. Create/update Tera template (`.rs.tera`)
4. Add generation rule to `ggen.toml`
5. Run `ggen sync`
6. Verify: no TODOs, compiles, tests pass

### Quality Gates
- Zero TODOs in generated code
- Zero compile errors
- All `validate()` functions implemented
- File size > 100 bytes (detect empty generation)

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

### Code Generation (cargo make)
```bash
cargo make sync              # Generate from ontology
cargo make sync-validate     # Check without writing
cargo make sync-dry-run      # Preview changes
cargo make sync-force        # Regenerate all
cargo make test-traceability # Verify ontology→code
cargo make test-determinism  # Code generation consistency
```

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
- 1.3.0 (2026-01-20): 80/20 gap analysis update - Added 23-directory structure, observability stack, system limits, 31 Makefile tasks, 7 scripts, architecture layers (recovery/audit/SPARQL), production docs
- 1.2.0 (2026-01-20): SPR-optimized 80/20 distillation (200 LOC)
- 1.1.0 (2026-01-20): Added SPR protocol enforcement
- 1.0.0 (2026-01-20): Initial comprehensive guide

**Remember**: Safety first. SPR always. Ontology is truth. Tests are mandatory. Quality is built-in.

**⚠️ SPR IS MANDATORY ⚠️**
