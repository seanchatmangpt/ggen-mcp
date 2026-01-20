# SPR (Sparse Priming Representation) Protocol

**Version**: 1.2.0 | Mandatory | Non-negotiable

## SPR Definition
Neural activation through distilled associations. Maximum concept density. Minimum tokens. LLM-optimized sentences.

## Core Requirements (Jidoka for Communication)
```
DISTILL     → Essential concepts only, remove noise
ASSOCIATE   → Link patterns, don't enumerate lists
COMPRESS    → Maximum meaning per token ratio
ACTIVATE    → Prime latent space efficiently
VERIFY      → Self-check before responding
```

## Pattern Contrast
```
✗ VERBOSE: "The ontology file contains domain definitions processed by SPARQL
            queries that extract information passed to Tera templates generating Rust code."

✓ SPR:      "Ontology → SPARQL → Tera → Rust. Self-describing system. Single source of truth."
```

## Checklist (Before Every Response)
- [ ] Using distilled statements? (not elaborated)
- [ ] Maximizing conceptual density? (concepts per token)
- [ ] Using associations over lists? (links not enumeration)
- [ ] Could be more succinct? (compression possible?)

**If ANY unchecked → REWRITE IN SPR**

## SPR Cardinal Rules
1. **DISTILL** - Essential only; cut fluff ruthlessly
2. **ASSOCIATE** - Connect patterns; avoid itemization
3. **COMPRESS** - Density not verbosity
4. **ACTIVATE** - Prime the model's latent space
5. **VERIFY** - Self-check mandatory

## Consequences
- Violating SPR = Violating project standards
- SPR = Compile-time check for communication
- SPR = Jidoka for language
- **YOU CANNOT CLAIM IGNORANCE. SPR IS MANDATORY. ALWAYS.**

## Real Examples

### Domain IDs
```
✗ "We have identifiers for workbooks, forks, sheets, and regions that need to be
  kept separate and not mixed up with generic strings or other identifier types."

✓ "NewTypes prevent mixing: WorkbookId ≠ ForkId ≠ SheetName. Zero-cost safety."
```

### Error Handling
```
✗ "When an operation fails, we add context to explain what went wrong and why it failed,
  then map the error to MCP error types for proper reporting."

✓ "Context → MCP errors. operation().context(\"what failed and why\")?."
```

### Testing
```
✗ "We use state-based testing with real implementations and minimal mocking to test
  integration-focused scenarios following Chicago TDD style."

✓ "Chicago-TDD: State-based. Real implementations. Integration-focused. Minimal mocks."
```

---

**SPR is not optional. It's the compile-time check for thinking. Use it.**
