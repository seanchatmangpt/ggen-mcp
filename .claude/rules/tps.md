# TPS (Toyota Production System) Principles

**Version**: 1.2.0 | Core Development Philosophy

## Five Pillars (ggen-mcp Applied)

### 1. Jidoka (Automation with Human Touch)
**Compile-time prevention over runtime debugging.**

```
Patterns                          Why
─────────────────────────────────────────────
NewTypes prevent ID mixing        Type system enforces domain logic
Validation at boundaries          Catch errors at API entry
Poka-yoke input guards           Fail-fast, no silent failures
Generated code (ontology-driven)  Single source of truth prevents divergence
```

**Implementation**: NewTypes (WorkbookId ≠ ForkId), validate_non_empty_string(), auto-generated code from ontology.

### 2. Andon Cord (Stop and Call for Help)
**Tests fail → build stops. Errors block progress. Never ship broken.**

```bash
cargo make pre-commit   # sync + check + test (MUST pass)
cargo make ci           # fmt-check + clippy + check + test-all
```

**Applied**: Zero TODOs in generated code. Compilation errors block commits. Test coverage > 80% for security paths.

### 3. Poka-Yoke (Error-Proofing)
**Make wrong action impossible, right action obvious.**

```rust
// NewType prevents confusion
WorkbookId(String)           // Cannot accidentally use as ForkId
validate_sheet_name(&name)?  // Input guard at boundary
context("what failed")?       // Mandatory error context
```

**Applied**: Validation limits, path safety checks, SPARQL injection prevention.

### 4. Kaizen (Continuous Improvement)
**Measure. Document. Iterate.**

```
Metrics tracked: Test coverage, build latency, security scans
Decisions documented: CLAUDE.md, architecture ADRs
Feedback loop: Pre-commit checks → CI → deployment
```

### 5. Single Piece Flow (One Component at a Time)
**Small commits. Fast feedback. Work in progress limits: 1.**

```bash
git log --oneline  # Should show focused commits
# ✗ "Add features X, Y, Z and fix bugs A, B in one commit"
# ✓ "feat: Add feature X. Reason: improves user experience"
```

---

## Applied Patterns

### Code Generation Workflow (Self-Describing System)
```
1. Update ontology/mcp-domain.ttl (source of truth)
2. Create SPARQL query (queries/*.rq) — extracts domain intent
3. Create Tera template (templates/*.rs.tera) — generates code
4. Add rule to ggen.toml — maps query → template → output
5. Run: cargo make sync
6. Verify: no TODOs, compiles, tests pass
```

### Quality Gates (Andon Cord Implementation)
```
Zero TODOs in generated code    ← Jidoka: prevent incomplete code
Zero compile errors             ← Andon: stop on error
validate() functions complete   ← Poka-yoke: no unwrap()
File size > 100 bytes          ← Detect empty generation (fail-fast)
```

### Safety Patterns (Poka-Yoke Details)
```rust
// Input Validation (first line of defense)
validate_non_empty_string(s)?;
validate_numeric_range(n, 1, 1_048_576, "row")?;
validate_path_safe(&path)?;

// NewTypes (zero-cost type safety)
pub struct WorkbookId(String);   // ≠ ForkId(String)
pub struct ForkId(String);

// Error Context (mandatory)
operation().context("What failed and why")?;

// Mapping to MCP
impl From<Error> for rmcp::Error { ... }
```

---

## Development Cycle (TPS Rhythm)

```
1. Edit: code or ontology
2. If ontology changed: cargo make sync
3. cargo check
4. cargo test
5. cargo fmt && cargo clippy
6. Small, focused commit
7. Repeat
```

## Critical DO/DON'T (Jidoka Enforcement)

### ALWAYS DO
- Use SPR — distilled communication mandatory
- Read ontology before editing generated code
- Validate all inputs at boundaries
- Add error context to failures
- Use NewTypes for domain concepts
- Write tests first (TDD)
- Run pre-commit checks

### NEVER DO
- Ignore SPR — non-negotiable
- Edit generated code (update ontology instead)
- Skip validation
- Use bare String for domain IDs
- Use unwrap() in production code
- Skip tests
- Commit with TODOs in generated code

---

**TPS applied to software: fail-fast, type-safe, measurable, continuous improvement.**
