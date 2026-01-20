# Ontology Generation Test Infrastructure

**Version**: 1.0.0
**Coverage Target**: >80%
**Pattern**: Chicago-style TDD (state-based, real implementations)

## Overview

Comprehensive test infrastructure for ontology-driven code generation workflows:
- **Load** RDF ontology → **Query** SPARQL → **Render** Tera templates → **Validate** output

## Architecture

### Test Harness

**File**: `tests/harness/ontology_generation_harness.rs` (730+ lines)

Core harness implementing:
- Ontology loading (Turtle/RDF)
- SPARQL query execution with caching
- Tera template rendering
- Output validation (non-empty, no TODOs, min size)
- Golden file comparison
- Setup/teardown lifecycle
- Workflow metrics collection

Key methods:
```rust
OntologyGenerationHarness::new()
    .with_fixture("test-api")
    .with_preview_mode(false)
    .with_golden_comparison(true)

harness.load_ontology(name)
harness.register_query(name, file)
harness.register_template(name, file)
harness.execute_workflow() -> Result<WorkflowResult>
harness.verify_output(&result)
harness.compare_golden_files(&result)
harness.test_cache_hit()
```

### Test Fixtures

#### Ontology: `tests/fixtures/ontology/test-api.ttl`
Minimal test ontology defining:
- **Entities**: User, Product
- **Value Objects**: Email
- **Commands**: CreateUser
- **Properties**: IDs, names, email, price
- **Invariants**: Email validation, positive price

#### SPARQL Queries: `tests/fixtures/queries/`
- `test-entities.rq` - Extract entity definitions with properties
- `test-valueobjects.rq` - Extract value objects
- `test-commands.rq` - Extract command definitions

#### Tera Templates: `tests/fixtures/templates/`
- `test-schema.tera` - Rust struct generation (entities, value objects, commands)
- `test-types.tera` - TypeScript type definitions
- `test-openapi.tera` - OpenAPI 3.0 YAML spec

### Golden Files

**Location**: `tests/golden/`

Expected outputs for validation:
- `test-schema.rs` - Expected Rust code
- `test-types.mjs` - Expected TypeScript types
- `test-openapi.yaml` - Expected OpenAPI spec

Golden files enable regression testing: generated output must match byte-for-byte.

### Integration Tests

**File**: `tests/ontology_generation_integration_tests.rs` (600+ lines)

15 comprehensive tests covering:

#### Full Workflow Tests
1. **test_full_workflow** - Complete end-to-end pipeline
   - Load → Register queries/templates → Execute → Verify
   - Validates all stages succeed
   - Checks metrics populated
   - Ensures no TODOs in output

2. **test_preview_mode** - Render without writing
   - Templates render successfully
   - No files written to disk
   - Validation still runs

3. **test_cache_hit** - Query result caching
   - First execution populates cache
   - Second execution uses cache
   - Verify all queries cached

4. **test_golden_file_comparison** - Output regression testing
   - Compare generated vs. expected
   - Report differences
   - Line-by-line diff on mismatch

#### Error Recovery Tests
5. **test_error_recovery_missing_ontology** - Handle missing TTL files
6. **test_error_recovery_missing_query** - Handle missing SPARQL files
7. **test_error_recovery_missing_template** - Handle missing Tera files
8. **test_error_recovery_no_fixture_set** - Fail-fast without fixture

#### State-Based Tests (Chicago-TDD)
9. **test_state_after_load** - Verify harness state after operations
10. **test_deterministic_output** - Same input → same output (idempotency)

#### Property-Based Tests
11. **test_property_all_queries_have_results** - Every query returns data
12. **test_property_all_templates_render_non_empty** - Every template produces output

## Usage

### Run All Tests
```bash
cargo test --test ontology_generation_integration_tests
```

### Run Specific Test
```bash
cargo test --test ontology_generation_integration_tests test_full_workflow
```

### Run with Output
```bash
cargo test --test ontology_generation_integration_tests -- --nocapture
```

### Generate Coverage Report
```bash
./scripts/coverage.sh --html
# View: target/coverage/html/index.html
```

## Test Patterns

### Chicago-Style TDD
- **State-based assertions**: Verify object state changes, not interaction sequences
- **Real collaborators**: Use actual OntologyEngine, SPARQL, Tera (no mocks)
- **Integration focus**: Test complete workflows, not isolated units

### Example
```rust
// State-based: Verify result state
let result = harness.execute_workflow()?;
assert!(!result.query_results.is_empty());
assert!(result.validation_report.valid);

// NOT interaction-based (no "expect query() called 3 times")
```

### Validation Layers
1. **Non-empty output** - Generated content exists
2. **No TODOs** - No incomplete code markers
3. **Minimum size** - Output >100 bytes (detect empty generation)
4. **Golden file match** - Byte-for-byte comparison with expected

## Workflow Stages

### 1. Ontology Loading
- Parse Turtle/RDF into Oxigraph store
- Metric: `ontology_load_time`

### 2. SPARQL Execution
- Execute registered queries against store
- Cache results for performance
- Metric: `query_execution_time`

### 3. Template Rendering
- Build Tera context from query results
- Render all registered templates
- Metric: `template_render_time`

### 4. Output Writing
- Write rendered content to files (unless preview mode)
- Record written file paths

### 5. Validation
- Check non-empty, no TODOs, minimum size
- Compare against golden files (if enabled)
- Metric: `total_workflow_time`

## Directory Structure

```
tests/
├── harness/
│   ├── ontology_generation_harness.rs     # Main harness (730 lines)
│   └── mod.rs                             # Export harness types
├── fixtures/
│   ├── ontology/
│   │   └── test-api.ttl                   # Test ontology (3.9KB)
│   ├── queries/
│   │   ├── test-entities.rq               # Entity extraction
│   │   ├── test-valueobjects.rq           # Value object extraction
│   │   └── test-commands.rq               # Command extraction
│   └── templates/
│       ├── test-schema.tera               # Rust code gen
│       ├── test-types.tera                # TypeScript types
│       └── test-openapi.tera              # OpenAPI spec
├── golden/
│   ├── test-schema.rs                     # Expected Rust output
│   ├── test-types.mjs                     # Expected TS output
│   └── test-openapi.yaml                  # Expected OpenAPI output
└── ontology_generation_integration_tests.rs  # Integration tests (600 lines)
```

## Exported Types

From `tests/harness/mod.rs`:

```rust
pub use ontology_generation_harness::{
    OntologyGenerationHarness,    // Main harness
    WorkflowResult,                // Complete workflow result
    QueryResult,                   // SPARQL query result
    RenderedOutput,                // Template render result
    ValidationReport,              // Validation checks
    ValidationCheck,               // Individual check
    GoldenFileComparison,          // Golden file comparison result
    FileComparison,                // Single file comparison
    CacheTestResult,               // Cache test result
    WorkflowMetrics,               // Performance metrics
};
```

## Metrics Collection

All workflows collect performance metrics:
- `ontology_load_time: Duration` - Time to parse and load TTL
- `query_execution_time: Duration` - Total SPARQL execution time
- `template_render_time: Duration` - Total Tera rendering time
- `total_workflow_time: Duration` - End-to-end workflow time

Access via `result.metrics.*`

## Error Handling

All operations return `Result<T>`:
- Ontology parse errors
- Query execution errors
- Template rendering errors
- File I/O errors
- Validation failures

Context added at each layer:
```rust
harness.load_ontology("test-api")
    .context("Failed to load test-api ontology")?;
```

## Coverage Goals

| Category | Target | Status |
|----------|--------|--------|
| Harness core | 80%+ | ✓ Created |
| Workflow execution | 80%+ | ✓ Tested |
| Error paths | 80%+ | ✓ Tested |
| State transitions | 100% | ✓ Tested |
| Integration flows | 80%+ | ✓ Tested |

## Quality Gates

Tests enforce:
1. **Zero TODOs** in generated code
2. **Non-empty** output (>100 bytes)
3. **Successful validation** report
4. **Deterministic** output (idempotent)
5. **Cache effectiveness** (second query uses cache)

## Future Enhancements

Potential additions:
- [ ] Performance benchmarks (large ontologies)
- [ ] Parallel query execution tests
- [ ] Incremental generation tests
- [ ] Error recovery snapshots
- [ ] Template hot-reload testing
- [ ] Multi-ontology merge testing
- [ ] Schema evolution testing
- [ ] Custom validation rules

## Dependencies

Core dependencies used:
- `oxigraph` - RDF store and SPARQL engine
- `tera` - Template engine
- `serde` - Serialization
- `anyhow` - Error handling
- `spreadsheet_mcp::ontology` - Ontology validation
- `spreadsheet_mcp::sparql` - Query builders, caching
- `spreadsheet_mcp::template` - Safe rendering

## Related Documentation

- `CHICAGO_TDD_TEST_HARNESS_COMPLETE.md` - Test harness patterns
- `RUST_MCP_BEST_PRACTICES.md` - Rust coding standards
- `POKA_YOKE_IMPLEMENTATION.md` - Error-proofing patterns
- `CLAUDE.md` - Project guidelines (SPR, TPS principles)

## Maintenance

### Adding New Tests
1. Create fixture in `tests/fixtures/`
2. Add SPARQL query in `queries/`
3. Add Tera template in `templates/`
4. Create golden file in `golden/`
5. Write test in `ontology_generation_integration_tests.rs`

### Updating Golden Files
After intentional changes to generation:
```bash
# Run test to generate new output
cargo test test_full_workflow -- --nocapture

# Copy generated to golden
cp target/test_ontology_output/*.rs tests/golden/
cp target/test_ontology_output/*.mjs tests/golden/
cp target/test_ontology_output/*.yaml tests/golden/
```

### Debugging Test Failures
1. Run with `--nocapture` to see println! output
2. Check `target/test_ontology_output/` for generated files
3. Compare against golden files manually
4. Use `harness.verify_output()` for detailed errors
5. Check metrics for performance issues

---

**Test Philosophy**: Real implementations. State-based assertions. End-to-end flows. Golden file regression. Chicago-style TDD throughout.
