# Code Review Agent

**Purpose**: Enforce SPR, TPS, and Rust safety patterns during code review

**Trigger**: Manual invocation before commit

## Checklist

### SPR Compliance
- [ ] Is communication distilled? (no verbosity)
- [ ] Maximum conceptual density per token?
- [ ] Uses associations over lists?
- [ ] Self-check: could be more succinct?

### Type Safety (Jidoka)
- [ ] NewTypes used for domain IDs? (WorkbookId ≠ ForkId)
- [ ] No bare String for domain concepts?
- [ ] All error types use Result<T>?
- [ ] Error context added to failures?

### Testing (Chicago-TDD)
- [ ] Unit tests validate behavior (not implementation)?
- [ ] Error paths tested as thoroughly as happy path?
- [ ] Real implementations used (minimal mocks)?
- [ ] Coverage targets met (security 95%+, core 80%+)?

### Generated Code
- [ ] Zero TODOs in src/generated/?
- [ ] Changes from ontology update only?
- [ ] Regenerated with cargo make sync?
- [ ] Compiles without warnings?

### Validation (Poka-Yoke)
- [ ] All inputs validated at boundaries?
- [ ] Path safety checks in place?
- [ ] Range validation for numeric inputs?
- [ ] No unwrap() in production code?
- [ ] SPARQL queries include LIMIT clause? (prevent unbounded queries)

### Pre-Commit (Andon Cord)
- [ ] cargo fmt passed?
- [ ] cargo clippy -- -D warnings passed?
- [ ] cargo test all passed?
- [ ] No Cargo.lock modifications (unless dependencies changed)?

## Report Template
```
## SPR Compliance
- [x] Distilled communication
- [x] Conceptual density maximized
- [ ] NEEDS: Remove verbose explanation in function docstring

## Type Safety
- [x] NewTypes enforced
- [x] Result<T> used throughout
- [ ] NEEDS: Add context to line 42 error mapping

## Testing
- [x] Error paths covered
- [x] State-based assertions
- [ ] NEEDS: Add test for boundary case (input > 1048576)

## Validation
- [x] Boundary checks present
- [x] No unwrap() in production
- [x] Path safety validated

## Generated Code
- [x] Zero TODOs
- [x] Compiles clean
- [x] Regenerated from ontology

## Pre-Commit
- [x] fmt check passed
- [x] clippy clean
- [x] tests green

**VERDICT**: ✓ Ready to commit
```

## Command
```bash
# Interactive code review
claude-code review-agent
```
