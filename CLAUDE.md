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
Rust 2024 | rmcp v0.11 | Tokio async | Oxigraph RDF/SPARQL | Tera templates | OpenTelemetry observability

---

## Critical File Structure

```
src/
├── generated/        # NEVER edit manually - regenerate from ontology
├── validation/       # Input guards, poka-yoke
├── domain/          # NewTypes, value objects
├── ontology/        # RDF/SPARQL engine
├── workbook.rs      # Core spreadsheet logic (55KB)
├── server.rs        # MCP tools (43KB)
└── error.rs         # Typed errors (27KB)

ontology/mcp-domain.ttl    # Source of truth (42KB)
ggen.toml                  # Generation config
templates/*.rs.tera        # Code generators
queries/*.rq              # SPARQL extractors
```

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

### Test Commands
```bash
cargo test                    # All tests
cargo test --test name        # Specific suite
./scripts/coverage.sh --html  # Coverage report
cargo bench                   # Benchmarks
```

### Coverage Targets
Security: 95%+ | Core handlers: 80%+ | Business logic: 80%+

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

### Code Generation
```bash
ggen sync --manifest ggen.toml
grep -r "TODO" src/generated/  # Must be empty
cargo check                     # Must pass
```

### Testing
```bash
cargo test
./scripts/coverage.sh --check
```

### Docker
```bash
docker build -t spreadsheet-mcp:dev .
docker run -v $(pwd)/fixtures:/data -p 8079:8079 spreadsheet-mcp:dev
```

### Scripts
```bash
./scripts/coverage.sh          # Coverage analysis
./scripts/ggen-sync.sh         # Sync with validation
./verify_poka_yoke.sh          # Verify safety patterns
```

---

## Documentation Map (Core)

**Must Read**:
- `RUST_MCP_BEST_PRACTICES.md` (36KB) - Comprehensive Rust patterns
- `POKA_YOKE_IMPLEMENTATION.md` (15KB) - Error-proofing guide
- `CHICAGO_TDD_TEST_HARNESS_COMPLETE.md` (26KB) - Testing infrastructure

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
- 1.2.0 (2026-01-20): SPR-optimized 80/20 distillation (200 LOC)
- 1.1.0 (2026-01-20): Added SPR protocol enforcement
- 1.0.0 (2026-01-20): Initial comprehensive guide

**Remember**: Safety first. SPR always. Ontology is truth. Tests are mandatory. Quality is built-in.

**⚠️ SPR IS MANDATORY ⚠️**
