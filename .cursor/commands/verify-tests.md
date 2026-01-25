# Verify Tests Before Completion - Multi-Step Workflow

## Purpose

This command guides agents through the complete workflow of running tests, identifying failures, fixing issues, and ensuring all tests pass before marking work as complete. It breaks down the complex process into clear, sequential steps with validation checkpoints.

**ggen-mcp Context**: Tests verify ontology sync, SPARQL queries, template rendering, and code generation. All tests must pass before completing ontology/template changes.

## Workflow Overview

```
Step 1: Run Tests (with Measurement) → Step 2: Analyze Results → Step 3: Fix Failures → Step 4: Re-Run Tests → Step 5: Verify Completion (with Measurement & Control)
```

## Step-by-Step Instructions

### Step 1: Run Test Suite

**Action**: Run all tests to identify any failures.

```bash
cargo make test
```

**What this does**:
- Runs all unit tests
- Runs all integration tests
- Runs all example tests
- Runs ontology sync tests
- Runs SPARQL validation tests
- Runs template rendering tests
- Runs code generation tests
- Runs property-based tests (if feature enabled)

**Expected Result**: All tests pass (exit code 0)

**If this step fails**: Proceed to Step 2 (Analyze Results)

**If this step succeeds**: Skip to Step 5 (Verify Completion)

**Note**: Always use `cargo make test`, never `cargo test` directly. See [ggen-mcp Standards](../rules/ggen-mcp-standards.mdc).

**ggen-mcp specific**: After ontology/template changes, run `cargo make sync-dry-run` first, then `cargo make test` to verify generated code.

#### 1.1: Collect Baseline Data (DMAIC Measurement)

**Action**: Measure current test state to establish baseline.

**Data to collect**:
- **Test count**: How many tests exist?
- **Failure count**: How many tests fail?
- **Failure rate**: What percentage of tests fail?
- **Failure types**: What types of failures (compilation, test, panic, timeout)?
- **ggen-mcp specific**: Ontology sync failures, SPARQL query failures, template rendering failures

**Action**: Collect baseline data

```bash
# Count total tests
cargo make test 2>&1 | grep -c "test.*\.\.\."
# Output: 226 tests total

# Count failures
cargo make test 2>&1 | grep -c "FAILED"
# Output: 5 failures

# Calculate failure rate
# 5 failures / 226 tests = 2.2% failure rate

# Categorize failures
# Compilation errors: 2
# Test failures: 2
# Panics: 1
# Timeouts: 0
# Ontology sync failures: 1
```

**Example baseline data**:
```markdown
## Baseline Data (ggen-mcp)

**Total Tests**: 226
**Failures**: 5
**Failure Rate**: 2.2% (5/226)

**By Type**:
- Compilation errors: 2 (40%)
- Test failures: 2 (40%)
- Panics: 1 (20%)
- Timeouts: 0 (0%)

**By Category**:
- Ontology sync tests: 1 failure
- SPARQL tests: 0 failures
- Template tests: 1 failure
- Code generation tests: 1 failure
- Integration tests: 2 failures
```

---

### Step 2: Analyze Test Results

**Action**: Parse test output to identify all failures and categorize them.

#### 2.1: Extract Failure Information

**Look for these patterns in output**:

**Compilation Errors**:
```
error[E...]: <description>
  --> src/file.rs:line:column
```

**Test Failures**:
```
test test_name ... FAILED
```

**Panics**:
```
thread 'test_name' panicked at '<message>', src/file.rs:line:column
```

**Timeouts**:
```
test test_name ... timeout
```

**ggen-mcp specific failures**:
- **Ontology sync failures**: `test_ontology_sync ... FAILED`
- **SPARQL query failures**: `test_sparql_query ... FAILED`
- **Template rendering failures**: `test_template_rendering ... FAILED`
- **Code generation failures**: `test_code_generation ... FAILED`

#### 2.2: Categorize Failures

**Create failure list**:

```markdown
## Test Failures (ggen-mcp)

### Compilation Errors
- [ ] `src/generated/entities.rs:123` - Error: `expected type, found ...`
- [ ] `src/tools/ontology_sparql.rs:456` - Error: `cannot find function ...`

### Test Failures
- [ ] `test_ontology_sync_generates_code` - Error: `assertion failed: expected 21 files, got 20`
- [ ] `test_sparql_query_execution` - Error: `Query execution failed`

### Panics
- [ ] `test_template_rendering` - Panic: `called Result::unwrap() on an Err value`

### Timeouts
- [ ] `test_large_ontology_sync` - Timeout after 30s

### Ontology Sync Failures
- [ ] `test_ontology_sync` - Error: `SHACL validation failed`
```

#### 2.3: Prioritize Fixes

**Priority Order**:
1. **Compilation errors** - Must fix first (blocks everything)
2. **Ontology sync failures** - Fix before other tests (affects generated code)
3. **Test failures** - Fix by test importance (critical path first)
4. **Panics** - Fix immediately (indicates bugs)
5. **Timeouts** - Fix or optimize (may indicate performance issues)

---

### Step 3: Fix Test Failures

**Action**: Systematically fix each failure category.

#### 3.1: Fix Compilation Errors

**For each compilation error**:

**Step 3.1.1**: Read error message carefully
- Understand what the compiler is complaining about
- Identify the root cause
- **ggen-mcp specific**: Check if error is in generated code (fix ontology/template, not generated code)

**Step 3.1.2**: Fix the error
- **If error in generated code**: Fix ontology/template, regenerate code
- **If error in source code**: Update code to resolve compilation issue
- Ensure type safety
- Fix import statements if needed

**Step 3.1.3**: Verify fix
```bash
# If error in generated code
cargo make sync-dry-run  # Preview changes
cargo make sync          # Regenerate code
cargo make check         # Verify compilation

# If error in source code
cargo make check         # Verify compilation
```

**Step 3.1.4**: Repeat until all compilation errors fixed

**Common Fixes**:
- Missing imports: Add `use` statements
- Type mismatches: Fix type annotations
- Missing features: Enable feature flags in `Cargo.toml`
- Syntax errors: Fix syntax issues
- **Generated code errors**: Fix ontology/template, regenerate

**CRITICAL**: Never edit `src/generated/` manually. Fix ontology/template instead.

#### 3.2: Fix Test Failures

**For each test failure**:

**Step 3.2.1**: Read test failure message
- Understand what the test expected vs. what it got
- Identify the root cause
- **ggen-mcp specific**: Check if failure is due to ontology/template changes

**Step 3.2.2**: Determine if test or implementation is wrong
- Review test logic
- Review implementation logic
- Review ontology/template (if test involves code generation)
- Check if test needs updating or implementation needs fixing

**Step 3.2.3**: Fix the issue
- **If ontology/template issue**: Fix ontology/template, regenerate code
- Update test if test is wrong
- Update implementation if implementation is wrong
- Ensure test follows AAA pattern (see [Chicago TDD Standards](../rules/ggen-mcp-standards.mdc))

**Step 3.2.4**: Verify fix
```bash
# If ontology/template change
cargo make sync-dry-run  # Preview
cargo make sync          # Apply
cargo make test test_name  # Test specific test

# If source code change
cargo make test test_name  # Test specific test
```

**Step 3.2.5**: Repeat for each failing test

**Common Fixes**:
- Wrong expected values: Update assertions
- Missing setup: Add Arrange phase
- Async issues: Ensure proper async handling
- Feature flags: Enable required features
- **Ontology changes**: Update ontology, regenerate code, update test expectations

**Example - Ontology Sync Test Failure**:
```rust
// Test failure: Expected 21 files, got 20
// Root cause: Added new entity to ontology, but test expects old count

// Fix: Update test expectation
#[tokio::test]
async fn test_ontology_sync_generates_code() {
    // Arrange
    let ontology_path = "tests/fixtures/test_ontology.ttl";
    
    // Act
    let result = sync_ontology(ontology_path).await?;
    
    // Assert: Updated expectation
    assert_eq!(result.generated_files.len(), 22); // Updated from 21
    // Verify: All files compile
    assert!(result.compiles);
}
```

#### 3.3: Fix Panics

**For each panic**:

**Step 3.3.1**: Identify panic source
- Read stack trace
- Find the exact line causing panic
- **ggen-mcp specific**: Check if panic is in generated code (fix ontology/template)

**Step 3.3.2**: Fix panic source
- Replace `unwrap()`/`expect()` with proper error handling
- Add null checks
- Fix index out of bounds
- Handle edge cases
- **If in generated code**: Fix ontology/template, regenerate

**Step 3.3.3**: Verify fix
```bash
cargo make test test_name
```

**Step 3.3.4**: Repeat for each panic

**Common Fixes**:
- `unwrap()` on `None`: Use `match` or `?` operator
- Index out of bounds: Add bounds checking
- Division by zero: Add zero checks
- Null pointer: Add null checks
- **Generated code panics**: Fix ontology/template, regenerate

#### 3.4: Fix Timeouts

**For each timeout**:

**Step 3.4.1**: Identify slow operation
- Review test code
- Find the operation taking too long
- **ggen-mcp specific**: Check if SPARQL query is too complex, ontology too large

**Step 3.4.2**: Optimize or mock
- Optimize slow code
- Optimize SPARQL query (add LIMIT, simplify)
- Mock external dependencies
- Increase timeout if legitimate
- Use test fixtures for setup

**Step 3.4.3**: Verify fix
```bash
cargo make test test_name
```

**Step 3.4.4**: Repeat for each timeout

**Common Fixes**:
- Mock external APIs
- Optimize SPARQL queries (add LIMIT clause)
- Use test fixtures
- Optimize algorithms
- Increase timeout for legitimate slow operations

---

### Step 4: Re-Run Tests

**Action**: Run tests again to verify all fixes worked.

```bash
cargo make test
```

**Expected Result**: All tests pass (exit code 0)

**If this step fails**: 
- Return to Step 2
- Identify remaining failures
- Fix them in Step 3
- Repeat until all tests pass

**If this step succeeds**: Proceed to Step 5

**CRITICAL**: Do not mark work as complete until Step 4 passes completely.

**ggen-mcp specific**: After fixing ontology/template issues, verify sync still works:
```bash
cargo make sync-dry-run  # Preview
cargo make sync          # Apply
cargo make test          # Verify all tests pass
```

---

### Step 5: Verify Completion

**Action**: Final verification that work is complete.

#### 5.1: Verify All Tests Pass

```bash
cargo make test
```

**Expected**: Exit code 0, all tests pass

#### 5.2: Verify Compilation

```bash
cargo make check
```

**Expected**: Exit code 0, no compilation errors

#### 5.3: Verify Ontology Sync (ggen-mcp Specific)

**Action**: Verify ontology sync works correctly.

```bash
# Preview sync
cargo make sync-dry-run

# Apply sync
cargo make sync

# Verify generated code compiles
cargo make check
```

**Expected**: Sync succeeds, generated code compiles

#### 5.4: Verify No Pending Test Fixes

**Check**: Review todo list for any pending test fixes

**Action**: Remove completed test fixes from todo list

**Expected**: No pending test fixes remain

#### 5.5: Measure Improvement (DMAIC Measurement)

**Action**: Measure improvement against baseline data.

**Measurement**:
- Re-count failures after fixes
- Compare to baseline
- Calculate improvement percentage
- Verify success criteria met

**Action**: Measure improvement

```bash
# Re-count failures after fixes
cargo make test 2>&1 | grep -c "FAILED"
# Output: 0 failures (down from 5)

# Calculate improvement
# Baseline: 5 failures (2.2% failure rate)
# After fixes: 0 failures (0% failure rate)
# Improvement: 100% (5/5 failures fixed)
```

**Example improvement measurement**:
```markdown
## Improvement Measurement (ggen-mcp)

**Baseline**: 5 failures (2.2% failure rate)
**After Fixes**: 0 failures (0% failure rate)
**Improvement**: 100% (5/5 failures fixed)

**By Type**:
- Compilation errors: 2 → 0 (100% improvement)
- Test failures: 2 → 0 (100% improvement)
- Panics: 1 → 0 (100% improvement)

**By Category**:
- Ontology sync tests: 1 → 0 (100% improvement)
- Template tests: 1 → 0 (100% improvement)
- Code generation tests: 1 → 0 (100% improvement)

**Success Criteria Met**: ✅
- All tests pass: 226/226 (100%) ✅
- No compilation errors ✅
- No test failures ✅
- Ontology sync works ✅
```

#### 5.6: Mark Work Complete

**Only when**:
- ✅ All tests pass (`cargo make test` exits with code 0)
- ✅ No compilation errors (`cargo make check` succeeds)
- ✅ No test failures
- ✅ Ontology sync works (`cargo make sync` succeeds)
- ✅ Generated code compiles
- ✅ No pending test fixes in todo list
- ✅ Improvement measured and verified

**Then**: Mark work as complete

#### 5.7: Establish Controls (DMAIC Control)

**Action**: Set up controls to prevent test failures from returning.

**Controls**:
- **CI/CD**: Run tests automatically on every commit
- **Pre-commit hooks**: Run tests before commits
- **Monitoring**: Track test failure rate over time
- **Alerts**: Set up alerts if failure rate increases
- **ggen-mcp specific**: Verify ontology sync in CI

**Action**: Create todo list for controls (10+ items)

```markdown
## Test Verification Control Todos (10+ items) - ggen-mcp

**CI/CD Controls**:
- [ ] Add CI check: Run all tests on every commit
- [ ] Add CI check: Run ontology sync before tests
- [ ] Configure CI to fail if tests fail
- [ ] Add test failure rate tracking to CI
- [ ] Verify CI checks work correctly

**Pre-commit Controls**:
- [ ] Add pre-commit hook: Run tests before commit
- [ ] Add pre-commit hook: Run ontology sync validation
- [ ] Configure hook to prevent commit if tests fail
- [ ] Verify pre-commit hooks work correctly
- [ ] Document hook usage

**Monitoring Controls**:
- [ ] Set up test failure rate tracking dashboard
- [ ] Configure alerts if failure rate > 1%
- [ ] Track ontology sync success rate
- [ ] Review test failure trends weekly
- [ ] Document failure patterns

**Standards Controls**:
- [ ] Add standard: All tests must pass before commit
- [ ] Add standard: Test failure rate must be < 1%
- [ ] Add standard: Ontology sync must succeed before commit
- [ ] Update team documentation with standards
- [ ] Verify standards are followed
```

**Execution**:
1. Create todos using `todo_write` tool (10+ items minimum)
2. Execute todos one by one (implement controls)
3. Mark todos as completed as controls are implemented
4. Verify each control works before moving to next
5. Continue until all controls implemented

**Principle**: Implement controls to prevent test failures, don't just document them. Todos track progress, controls prevent recurrence.

#### 5.8: Monitor (DMAIC Control)

**Action**: Monitor to ensure test failures don't return.

**Monitoring**:
- Track test failure rate over time
- Set up alerts for regression
- Review trends periodically
- Adjust controls if needed

**Action**: Set up monitoring

```bash
# Monitor test failure rate
# Run daily: cargo make test 2>&1 | grep -c "FAILED"
# Alert if failure rate > 1%

# Monitor ontology sync success
# Run daily: cargo make sync-dry-run 2>&1 | grep -c "error"
# Alert if sync fails

# Track trends
# Week 1: 5 failures (2.2% failure rate - baseline)
# Week 2: 0 failures (0% failure rate - after fixes)
# Week 3: 0 failures (0% failure rate - controls working)
# Week 4: 0 failures (0% failure rate - sustained)
```

---

## Advanced: Running Specific Test Suites (ggen-mcp)

### Run Unit Tests Only

```bash
cargo make test-unit
```

**Use when**: Quick feedback during development

### Run Integration Tests

```bash
cargo make test-integration
```

**Use when**: Testing ontology sync, SPARQL queries, template rendering

### Run Ontology Sync Tests

```bash
cargo make test-ggen
```

**Use when**: Testing code generation pipeline

### Run Example Tests

```bash
cargo make test-examples
```

**Use when**: Verifying example code works

### Run Single Test

```bash
cargo make test test_ontology_sync_generates_code
```

**Use when**: Debugging specific test failure

---

## Failure Pattern Reference (ggen-mcp)

### Compilation Errors in Generated Code

**Pattern**: `error[E...]: <description>` in `src/generated/`

**Example**:
```
error[E0425]: cannot find type `Entity` in this scope
  --> src/generated/entities.rs:10:5
   |
10 |     pub entity: Entity,
   |                 ^^^^^^^ not found in this scope
```

**Fix**: Fix ontology (add missing entity definition), regenerate code:
```bash
# Fix ontology
# Edit ontology/mcp-domain.ttl: Add entity definition

# Regenerate code
cargo make sync-dry-run  # Preview
cargo make sync          # Apply
cargo make check         # Verify
```

**CRITICAL**: Never edit `src/generated/` manually. Fix ontology instead.

### Ontology Sync Failures

**Pattern**: `test_ontology_sync ... FAILED`

**Example**:
```
test test_ontology_sync_generates_code ... FAILED

---- test_ontology_sync_generates_code stdout ----
thread 'test_ontology_sync_generates_code' panicked at 'assertion failed: 
  left: `20`,
 right: `21`', tests/ontology_sync_tests.rs:45:5
```

**Fix**: Update test expectation or fix ontology:
- If ontology changed: Update test expectation
- If ontology incomplete: Fix ontology, regenerate

### SPARQL Query Failures

**Pattern**: `test_sparql_query ... FAILED`

**Example**:
```
test test_sparql_query_execution ... FAILED

---- test_sparql_query_execution stdout ----
Error: Query execution failed: SPARQL syntax error
```

**Fix**: Fix SPARQL query syntax, validate query:
```bash
# Validate SPARQL query
cargo make validate-sparql queries/tools.rq

# Fix query syntax
# Edit queries/tools.rq: Fix syntax error

# Retry test
cargo make test test_sparql_query_execution
```

### Template Rendering Failures

**Pattern**: `test_template_rendering ... FAILED`

**Example**:
```
test test_template_rendering ... FAILED

---- test_template_rendering stdout ----
Error: Template rendering failed: Variable 'entity_name' not found
```

**Fix**: Ensure SPARQL query provides template variable:
- Check SPARQL query extracts `entity_name`
- Update query if missing
- Or add default value in template

---

## Complete Workflow Example (ggen-mcp)

```bash
# Step 1: Run Tests
cargo make test
# Output: 2 tests failed

# Step 2: Analyze Results
# Found:
# - test_ontology_sync: FAILED - Expected 21 files, got 20
# - test_sparql_query: FAILED - Query syntax error

# Step 3: Fix Failures
# Fix test_ontology_sync: Update expectation (added new entity)
# Fix test_sparql_query: Fix SPARQL syntax error

# Step 4: Re-Run Tests
cargo make test
# All tests pass ✅

# Step 5: Verify Completion
cargo make sync-dry-run  # Preview OK ✅
cargo make sync          # Sync OK ✅
cargo make check         # Compilation OK ✅
cargo make test          # All tests pass ✅
# Mark work complete ✅
```

## Error Handling (ggen-mcp Specific)

### If Tests Fail After Ontology Change

**Symptoms**: Tests fail after modifying ontology

**Fix**:
1. Regenerate code: `cargo make sync`
2. Update test expectations if needed
3. Verify generated code compiles
4. Retry tests

### If Generated Code Has Errors

**Symptoms**: Compilation errors in `src/generated/`

**Fix**:
1. **DO NOT** edit generated code manually
2. Fix ontology/template (source of truth)
3. Regenerate code: `cargo make sync`
4. Verify fix

### If SPARQL Query Fails

**Symptoms**: SPARQL query execution errors

**Fix**:
1. Validate query syntax: `cargo make validate-sparql queries/*.rq`
2. Check query provides all template variables
3. Fix query syntax
4. Retry test

### If Template Rendering Fails

**Symptoms**: Template rendering errors

**Fix**:
1. Check template variables provided by SPARQL
2. Add `{{ error() }}` guards if needed
3. Fix template syntax
4. Retry test

## Best Practices (ggen-mcp)

1. **Run tests frequently** - Don't wait until the end
2. **Fix immediately** - Address failures as they occur
3. **One fix at a time** - Fix and verify each issue separately
4. **Verify after fixes** - Always re-run tests after fixes
5. **Never edit generated code** - Fix ontology/template instead
6. **Preview before applying** - Use `sync-dry-run` before `sync`
7. **Verify sync works** - Ensure ontology sync succeeds
8. **Document failures** - Add to todo list if not immediately fixable
9. **Never skip validation** - All tests must pass before completion

## Integration with Other Commands

- **[Ontology Sync](./ontology-sync.md)** - Verify sync before running tests
- **[SPARQL Validation](./sparql-validation.md)** - Validate queries before tests
- **[Template Rendering](./template-rendering.md)** - Validate templates before tests
- **[Code Generation](./code-generation.md)** - Verify codegen pipeline before tests
- **[Chicago TDD Standards](../rules/ggen-mcp-standards.mdc)** - Testing standards
- **[DMAIC Problem Solving](./dmaic-problem-solving.md)** - Use DMAIC measurement and control steps integrated into this workflow

## Documentation References

- **[ggen-mcp Standards](../rules/ggen-mcp-standards.mdc)** - Project standards
- **[CLAUDE.md](../../CLAUDE.md)** - SPR protocol and architecture
- **[RUST_MCP_BEST_PRACTICES.md](../../RUST_MCP_BEST_PRACTICES.md)** - MCP best practices
- **[Makefile.toml](../../Makefile.toml)** - Build tasks

## Quick Reference (ggen-mcp)

```bash
# Full workflow
cargo make sync-dry-run          # Preview ontology sync
cargo make sync                  # Apply sync
cargo make test                  # Step 1: Run tests
# Analyze failures               # Step 2: Analyze
# Fix failures                   # Step 3: Fix
cargo make sync                  # Regenerate if ontology changed
cargo make test                  # Step 4: Re-run
cargo make check                 # Step 5: Verify compilation
# Mark complete                  # Step 5: Verify completion
```
