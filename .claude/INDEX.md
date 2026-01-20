# .claude/ Directory Index (SPR-Optimized)

**Navigation guide. Quick access. Essential only.**

---

## Directory Structure (Visual Map)

```
.claude/
├── INDEX.md                    # This file (navigation)
├── README.md                   # Status monitoring (tokens, git, agents)
├── settings.json               # Configuration. Model selection. Hooks. Quality gates.
├── state.json                  # Real-time session state
│
├── rules/                      # Development standards (enforceable)
│   ├── spr.md                  # Sparse Priming Representation protocol
│   ├── tps.md                  # Toyota Production System principles
│   ├── rust.md                 # Rust patterns. Type safety. Error handling.
│   └── testing.md              # Chicago-style TDD. Real implementations.
│
├── agents/                     # Workflow orchestrators (specialized)
│   ├── code-review-agent.md    # Enforce compliance. Verify patterns. Gate commits.
│   ├── codegen-agent.md        # Ontology → SPARQL → Tera → Rust. Deterministic generation.
│   ├── test-agent.md           # TDD workflow. Coverage. Benchmarks.
│   ├── ontology-sync.md        # Ontology schema sync. RDF validation.
│   └── [others]                # Coverage analyzer, test runner, code reviewer
│
├── hooks/                      # Lifecycle automation (quality gates)
│   ├── pre-commit.sh           # Format. Lint. Compile. Test. Zero TODOs. (Andon Cord)
│   ├── post-generation.sh      # Generated code quality gates. Fail-fast.
│   └── [others]                # Git checks, shell initialization
│
├── scripts/                    # Utilities (status, monitoring)
│   ├── status-line.sh          # Real-time token/cost/git/agent display
│   └── [others]                # Shell initialization
│
└── docs/                       # Supplemental (optional)
    └── mcp-integration.md      # MCP server integration patterns
```

---

## File-to-Purpose Mapping (Quick Lookup)

### I need to understand...

**SPR Protocol** → `.claude/rules/spr.md`
- What: Sparse Priming Representation (distilled communication)
- Why: Maximize concept density. Minimize tokens.
- Use: Before every response. Self-check mandatory.

**TPS Principles** → `.claude/rules/tps.md`
- What: Toyota Production System adapted to software
- Why: Fail-fast (Jidoka). Build stops on error (Andon Cord). Error-proof (Poka-yoke).
- Use: Development philosophy reference. Understand fail-fast mentality.

**Rust Patterns** → `.claude/rules/rust.md`
- What: Type safety. NewTypes. Error handling. Validation.
- Why: Compile-time prevention > runtime debugging.
- Use: Before writing Rust code. Reference for domain IDs, error context.

**Testing Strategy** → `.claude/rules/testing.md`
- What: Chicago-style TDD. State-based. Real implementations.
- Why: Verify behavior, not implementation details. Integration-focused.
- Use: Before writing tests. Coverage targets: security 95%+, core 80%+.

---

## Workflow Triggers (Agent Use Cases)

### Code Review Workflow
```
Trigger: Before commit
Agent: .claude/agents/code-review-agent.md
Checklist: SPR compliance, type safety, testing, validation, pre-commit checks
Output: ✓ Ready to commit OR ❌ Issues found
Hook: .claude/hooks/pre-commit.sh (automated)
```

### Code Generation Workflow
```
Trigger: After ontology change
Agent: .claude/agents/codegen-agent.md
Process: Update ontology → SPARQL → Tera → ggen.toml → cargo make sync
Quality Gates: Zero TODOs, compiles, tests pass, file sizes valid
Hook: .claude/hooks/post-generation.sh (automated)
```

### Testing Workflow
```
Trigger: Before commit or test changes
Agent: .claude/agents/test-agent.md
Process: Write test → implement until pass → cover error cases → verify coverage
Coverage: Security 95%+, core 80%+, generated 85%+
Hook: cargo test (part of pre-commit)
```

---

## Configuration (settings.json)

### Models
- **default**: Sonnet 4.5 (fast, capable, cost-efficient)
- **subagents**: Haiku 4.5 (10x faster, validation/analysis)
- **Principle**: Haiku for speed; escalate to Sonnet if needed (Jidoka)

### Hooks
- **pre-commit**: Format → lint → compile → test → verify
- **post-generation**: Zero TODOs, compile, file sizes, tests
- **Principle**: Fail-fast. Build stops on error (Andon Cord).

### Quality Gates
```
Generated code:  TODO count = 0, file size >= 100B, coverage >= 85%
Commits:         format + lint + compile + test + coverage >= 80%
Coverage targets: Security 95%+, core 80%+, generated 85%+
```

### SPR Enforcement
- Enabled: Yes (mandatory)
- Check frequency: Every response
- Principles: DISTILL, ASSOCIATE, COMPRESS, ACTIVATE, VERIFY
- Reasoning: SPR = compile-time check for communication

### TPS Framework
- Jidoka: NewTypes, validation, generated code
- Andon Cord: Tests/TODOs/lints block build
- Poka-yoke: Validation guards, no unwrap()
- Kaizen: Measure (coverage), document (decisions), iterate
- Single Piece Flow: One component per commit

---

## Decision Tree (Where Do I Go?)

```
I need to...
├── Write code
│   └── Check .claude/rules/rust.md (NewTypes, errors, validation)
├── Write tests
│   └── Check .claude/rules/testing.md (Chicago-TDD, coverage targets)
├── Generate code from ontology
│   └── Check .claude/agents/codegen-agent.md (workflow steps)
├── Review code before commit
│   └── Check .claude/agents/code-review-agent.md (checklist)
├── Understand communication style
│   └── Check .claude/rules/spr.md (distilled, dense, concise)
├── Understand philosophy
│   └── Check .claude/rules/tps.md (fail-fast, quality gates)
└── Monitor session (tokens, git, agents)
    └── Check .claude/README.md (status-line.sh)
```

---

## Key Constraints (Non-Negotiable)

1. **SPR Mandatory** - Distilled communication. No exceptions. Self-check required.
2. **Generated Code Untouchable** - Edit ontology, not generated code. Regenerate via sync.
3. **Zero TODOs** - Incomplete generated code blocks commits. Run hooks.
4. **Type Safety** - NewTypes for domain IDs. No bare strings. Compiler enforces.
5. **Test Coverage** - Security 95%+, core 80%+. Coverage targets drive quality.
6. **Error Context** - All Err() must have context("what failed and why").
7. **No unwrap()** - Production code uses Result<T>. Tests may be lenient.
8. **Pre-Commit Checks** - Format, lint, compile, test must pass. Build stops otherwise.

---

## Performance Metrics

- **status-line.sh**: < 100ms
- **pre-commit hook**: < 60s (format + lint + compile + test)
- **post-generation hook**: < 45s (verify + compile + test)
- **Parallel subagents**: 3 optimal (Amdahl's Law, N=4 cores)

---

## Quick Commands

```bash
# Show status (tokens, git, agents)
./.claude/scripts/status-line.sh render

# Pre-commit verification (run before git commit)
./.claude/hooks/pre-commit.sh

# Code generation (after ontology change)
cargo make sync && ./.claude/hooks/post-generation.sh

# Run tests
cargo test

# Coverage report
./scripts/coverage.sh --html

# Code generation preview (dry-run)
cargo make sync-dry-run
```

---

## Rule File Quick Reference (Content Summary)

| File | Lines | Focus | Reference |
|------|-------|-------|-----------|
| spr.md | 74 | Sparse Priming. DISTILL, ASSOCIATE, COMPRESS. Self-check checklist. | Rules.spr |
| tps.md | 137 | Toyota Production System. Five pillars. Fail-fast. Quality gates. | Rules.tps |
| rust.md | 240 | Type safety, NewTypes, error handling, validation, async, testing patterns. | Rules.rust |
| testing.md | 176 | Chicago-TDD. State-based. Real impls. Coverage targets (95/80/85). | Rules.testing |

## Agent File Quick Reference (Content Summary)

| File | Lines | Trigger | Workflow | Output |
|------|-------|---------|----------|--------|
| code-review-agent.md | 84 | Before commit | Verify SPR/type/tests/validation | ✓/❌ verdict |
| codegen-agent.md | 167 | Ontology change | Ontology→SPARQL→Tera→ggen→sync | Generated code |
| test-agent.md | 268 | Test changes | TDD workflow. Coverage check. | Test results |
| ontology-sync.md | 122 | Schema sync | RDF validation. Turtle syntax. | ✓/❌ sync |

---

**SPR Applied**: All essential. No fluff. Maximum density. Navigation optimized. (74 lines for this index.)**

