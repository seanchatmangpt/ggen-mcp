# Code Generation Pipeline Test Harness - Quick Reference

**One-page reference for the Chicago-style TDD code generation pipeline harness**

## Quick Start

```rust
use harness::*;

#[test]
fn test_my_pipeline() -> Result<()> {
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true);

    let result = harness.run_complete_pipeline()?;
    harness.assert_all_stages_succeeded(&result);

    Ok(())
}
```

## Pipeline Stages

| Stage | Purpose | Input | Output |
|-------|---------|-------|--------|
| 1. Ontology | Load TTL | `.ttl` file | RDF Store |
| 2. SPARQL | Extract entities | SPARQL query | Domain entities |
| 3. Template | Generate code | Templates + entities | Rust code strings |
| 4. Validation | Verify syntax | Rust code | Validated code |
| 5. File Writing | Persist | Validated code | Written files |

## Common Test Patterns

### Basic Pipeline Test
```rust
#[test]
fn test_basic() -> Result<()> {
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate");

    let result = harness.run_complete_pipeline()?;

    assert!(result.success);
    Ok(())
}
```

### With Golden Files
```rust
#[test]
fn test_golden() -> Result<()> {
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_golden_files(true);

    let result = harness.run_complete_pipeline()?;
    let report = harness.compare_golden_files(&result)?;

    assert!(report.is_perfect_match());
    Ok(())
}
```

### Stage-by-Stage Validation
```rust
#[test]
fn test_stages() -> Result<()> {
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate");

    let result = harness.run_complete_pipeline()?;

    // Stage 1: Ontology
    assert!(result.ontology_result.triple_count > 0);

    // Stage 2: SPARQL
    assert!(!result.sparql_result.entities.is_empty());

    // Stage 3: Template
    assert!(!result.template_result.rendered_code.is_empty());

    // Stage 4: Validation
    assert!(result.validation_result.all_valid);

    // Stage 5: Files
    assert!(!result.file_result.written_files.is_empty());

    Ok(())
}
```

### Performance Testing
```rust
#[test]
fn test_perf() -> Result<()> {
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate");

    let result = harness.run_complete_pipeline()?;

    assert!(result.duration.as_millis() < 5000);
    harness.metrics.print_summary();

    Ok(())
}
```

## Assertions

```rust
// All stages
harness.assert_all_stages_succeeded(&result);

// Code quality
harness.assert_code_compiles(code)?;
harness.assert_all_imports_valid(code)?;
harness.assert_no_unused_code(code)?;

// Golden files
harness.assert_output_matches_golden(code, &path)?;

// Stage results
assert!(result.ontology_result.triple_count > 0);
assert_eq!(result.sparql_result.entities.len(), 5);
assert!(result.validation_result.all_valid);
```

## Fixture Structure

```
tests/fixtures/pipeline/my_fixture/
├── input/
│   ├── ontology.ttl          # REQUIRED
│   ├── queries.sparql        # optional
│   └── templates/            # optional
└── expected/
    └── *.rs                  # golden files
```

## Ontology Patterns

### Aggregate
```turtle
ggen:User a ddd:AggregateRoot ;
    rdfs:label "User" ;
    ddd:hasProperty ggen:User_id .

ggen:User_id a ddd:Property ;
    rdfs:label "id" ;
    ddd:propertyType "Uuid" .
```

### Command
```turtle
ggen:CreateUser a ddd:Command ;
    rdfs:label "CreateUser" ;
    ddd:targetsAggregate ggen:User ;
    ddd:hasParameter ggen:CreateUser_email .
```

### Value Object
```turtle
ggen:Money a ddd:ValueObject ;
    rdfs:label "Money" ;
    ddd:hasProperty ggen:Money_amount .
```

### MCP Tool
```turtle
ggen:ReadFile a mcp:Tool ;
    rdfs:label "read_file" ;
    mcp:description "Read a file" ;
    mcp:hasParameter ggen:ReadFile_path .
```

## Commands

```bash
# Run all tests
cargo test --test codegen_pipeline_integration_tests

# Run specific test
cargo test test_simple_aggregate

# Update golden files
cargo test test_update_golden_files -- --ignored

# Performance tests
cargo test test_.*_performance

# With output
cargo test test_simple_aggregate -- --nocapture
```

## Configuration Options

| Method | Purpose | Default |
|--------|---------|---------|
| `with_fixture(name)` | Set fixture | Required |
| `with_validation(bool)` | Enable validation | true |
| `with_golden_files(bool)` | Enable golden file comparison | true |
| `with_incremental(bool)` | Enable incremental updates | false |

## Result Types

```rust
PipelineResult {
    fixture: String,
    ontology_result: OntologyResult,
    sparql_result: SparqlResult,
    template_result: TemplateResult,
    validation_result: ValidationResult,
    file_result: FileResult,
    duration: Duration,
    success: bool,
}
```

## Common Errors

| Error | Cause | Solution |
|-------|-------|----------|
| "Fixture not found" | Missing directory | Check path exists |
| "Failed to parse TTL" | Invalid Turtle | Validate syntax |
| "Code does not compile" | Invalid Rust | Check generated code |
| "Golden file mismatch" | Output changed | Review and update |

## Performance Thresholds

| Scenario | Threshold |
|----------|-----------|
| Simple aggregate | < 5 seconds |
| Complete domain | < 10 seconds |
| Ontology loading | < 100 ms |
| SPARQL query | < 50 ms |

## Best Practices

✅ **Do:**
- Use real collaborators (Chicago TDD)
- Test complete flows
- Keep fixtures focused
- Validate golden files
- Check performance

❌ **Don't:**
- Mock core components
- Test implementation details
- Create test-specific fixtures
- Skip validation
- Ignore performance

## Files

| File | Purpose |
|------|---------|
| `tests/harness/codegen_pipeline_harness.rs` | Main harness implementation |
| `tests/codegen_pipeline_integration_tests.rs` | Integration tests |
| `tests/fixtures/pipeline/*/` | Test fixtures |
| `docs/TDD_CODEGEN_PIPELINE_HARNESS.md` | Full documentation |
| `examples/codegen_pipeline_harness_example.rs` | Usage examples |

## Example Output

```
🧪 Test: Simple Aggregate - Complete Pipeline
  📚 Stage 1: Ontology Loading
    ✓ Loaded 12 triples
    ⏱️  Duration: 12ms
  🔍 Stage 2: SPARQL Query Execution
    ✓ Extracted 2 entities
    ⏱️  Duration: 8ms
  📝 Stage 3: Template Rendering
    ✓ Loaded 5 templates
    ✓ Rendered 2 code files
    ⏱️  Duration: 15ms
  ✅ Stage 4: Code Validation
    🔍 Validating User.rs
      ✓ Syntax valid
    🔍 Validating CreateUser.rs
      ✓ Syntax valid
    ⏱️  Duration: 45ms
  💾 Stage 5: File Writing
    ✓ Wrote /path/to/User.rs
    ✓ Wrote /path/to/CreateUser.rs
    ⏱️  Duration: 5ms

📊 Pipeline Performance Metrics:
  Ontology Loading:    12ms
  SPARQL Query:        8ms
  Template Rendering:  15ms
  Code Validation:     45ms
  File Writing:        5ms
  ─────────────────────────────────
  Total:               85ms
```

## Quick Links

- 📖 [Full Documentation](TDD_CODEGEN_PIPELINE_HARNESS.md)
- 📋 [Fixture README](../tests/fixtures/pipeline/README.md)
- 💡 [Examples](../examples/codegen_pipeline_harness_example.rs)
- 🧪 [Integration Tests](../tests/codegen_pipeline_integration_tests.rs)
