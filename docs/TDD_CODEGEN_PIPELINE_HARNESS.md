# Chicago-Style TDD Code Generation Pipeline Harness

**Comprehensive test harness for the complete code generation pipeline: TTL → SPARQL → Template → Rust Code**

## Overview

This document describes the comprehensive Chicago-style TDD test harness for validating the complete code generation pipeline in `ggen-mcp`. The harness implements state-based testing with real collaborators following Test-Driven Development best practices.

## Architecture

### Pipeline Flow

```
┌─────────────┐     ┌──────────┐     ┌──────────┐     ┌────────────┐     ┌───────────┐
│ TTL Ontology│────▶│  SPARQL  │────▶│ Template │────▶│ Validation │────▶│   File    │
│   Loading   │     │  Query   │     │ Rendering│     │  & Syntax  │     │  Writing  │
└─────────────┘     └──────────┘     └──────────┘     └────────────┘     └───────────┘
     Stage 1           Stage 2          Stage 3           Stage 4           Stage 5
```

### Five Pipeline Stages

#### Stage 1: Ontology Loading
- **Purpose**: Load and validate TTL ontology files
- **Input**: `.ttl` files (RDF/Turtle format)
- **Output**: Populated RDF graph (Oxigraph Store)
- **Validation**:
  - Triple count verification
  - Graph integrity checking
  - Consistency validation
  - SHACL constraints (optional)

#### Stage 2: SPARQL Query Execution
- **Purpose**: Extract domain entities from RDF graph
- **Input**: SPARQL queries + RDF Store
- **Output**: List of `DomainEntity` objects
- **Validation**:
  - Query result verification
  - Type safety checks
  - Entity extraction completeness

#### Stage 3: Template Rendering
- **Purpose**: Generate Rust code from templates
- **Input**: Templates + Domain entities
- **Output**: Rendered Rust code strings
- **Validation**:
  - Template compilation
  - Context population
  - Output generation

#### Stage 4: Code Validation
- **Purpose**: Verify generated code quality
- **Input**: Rendered Rust code
- **Output**: Validated code + validation reports
- **Validation**:
  - Syntax checking (syn parser)
  - Import verification
  - Code structure validation
  - rustfmt compliance (optional)

#### Stage 5: File Writing
- **Purpose**: Safely persist generated code
- **Input**: Validated code
- **Output**: Written files + receipts
- **Validation**:
  - Atomic write operations
  - Backup creation
  - Artifact tracking
  - Dependency management

## Test Harness Components

### CodegenPipelineHarness

Main test harness orchestrating the complete pipeline.

```rust
use codegen_pipeline_harness::*;

let mut harness = CodegenPipelineHarness::new()
    .with_fixture("simple_aggregate")
    .with_validation(true)
    .with_golden_files(true);

let result = harness.run_complete_pipeline()?;
harness.assert_all_stages_succeeded(&result);
```

**Configuration Options:**
- `with_fixture(name)` - Set fixture directory
- `with_validation(bool)` - Enable/disable validation
- `with_golden_files(bool)` - Enable/disable golden file comparison
- `with_incremental(bool)` - Enable/disable incremental updates

### Pipeline Result Types

```rust
pub struct PipelineResult {
    pub fixture: String,
    pub ontology_result: OntologyResult,
    pub sparql_result: SparqlResult,
    pub template_result: TemplateResult,
    pub validation_result: ValidationResult,
    pub file_result: FileResult,
    pub duration: Duration,
    pub success: bool,
}
```

Each stage returns detailed results:

**OntologyResult**:
- RDF Store
- Triple count
- Integrity report
- Consistency report

**SparqlResult**:
- Extracted entities
- Query execution time
- Entity metadata

**TemplateResult**:
- Rendered code map
- Template count
- Render statistics

**ValidationResult**:
- Validated code
- Validation reports
- Syntax check results

**FileResult**:
- Written file paths
- Generation receipts
- Artifact metadata

## Test Fixtures

### Directory Structure

```
tests/fixtures/pipeline/
├── simple_aggregate/
│   ├── input/
│   │   ├── ontology.ttl
│   │   ├── queries.sparql
│   │   └── templates/  (optional)
│   └── expected/
│       ├── User.rs
│       └── CreateUser.rs
├── complete_domain/
│   ├── input/
│   │   └── ontology.ttl
│   └── expected/
│       ├── aggregates/
│       │   ├── User.rs
│       │   ├── Product.rs
│       │   └── Order.rs
│       ├── commands/
│       └── value_objects/
├── mcp_tool/
│   ├── input/
│   │   └── ontology.ttl
│   └── expected/
│       └── tools/
│           └── read_file.rs
└── error_scenarios/
    └── input/
        └── ontology.ttl  (intentionally invalid)
```

### Creating Test Fixtures

1. **Create fixture directory**:
   ```bash
   mkdir -p tests/fixtures/pipeline/my_fixture/input
   mkdir -p tests/fixtures/pipeline/my_fixture/expected
   ```

2. **Add ontology** (`input/ontology.ttl`):
   ```turtle
   @prefix ddd: <http://example.org/ddd#> .
   @prefix ggen: <http://example.org/ggen#> .

   ggen:MyAggregate a ddd:AggregateRoot ;
       rdfs:label "MyAggregate" ;
       ddd:hasProperty ggen:MyAggregate_id .
   ```

3. **Add expected output** (`expected/MyAggregate.rs`):
   ```rust
   pub struct MyAggregate {
       pub id: Uuid,
   }
   ```

4. **Write test**:
   ```rust
   #[test]
   fn test_my_fixture() -> Result<()> {
       let mut harness = CodegenPipelineHarness::new()
           .with_fixture("my_fixture");

       let result = harness.run_complete_pipeline()?;
       harness.assert_all_stages_succeeded(&result);

       Ok(())
   }
   ```

## Test Scenarios

### 1. Simple Scenarios

**Single Aggregate**:
```rust
#[test]
fn test_simple_aggregate() -> Result<()> {
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate");

    let result = harness.run_complete_pipeline()?;

    assert!(result.success);
    assert_eq!(result.sparql_result.entities.len(), 2); // User + CreateUser

    Ok(())
}
```

**Single Value Object**:
```rust
#[test]
fn test_value_object() -> Result<()> {
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_value_object");

    let result = harness.run_complete_pipeline()?;

    // Verify value object generated
    assert!(result.template_result.rendered_code.contains_key("Money.rs"));

    Ok(())
}
```

### 2. Complex Scenarios

**Complete Domain (User, Order, Product)**:
```rust
#[test]
fn test_complete_domain() -> Result<()> {
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("complete_domain");

    let result = harness.run_complete_pipeline()?;

    // Verify multiple entities
    assert!(result.sparql_result.entities.len() >= 3);

    // Verify all code compiles
    for (_, code) in &result.validation_result.validated_code {
        harness.assert_code_compiles(code)?;
    }

    Ok(())
}
```

**Complete MCP Server**:
```rust
#[test]
fn test_mcp_server() -> Result<()> {
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("mcp_server_complete");

    let result = harness.run_complete_pipeline()?;

    // Verify tools generated
    assert!(!result.template_result.rendered_code.is_empty());

    Ok(())
}
```

### 3. Error Scenarios

**Invalid Ontology**:
```rust
#[test]
fn test_invalid_ontology() {
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("error_scenarios");

    let result = harness.run_complete_pipeline();

    // Should fail or produce validation errors
    assert!(result.is_err() || !result.unwrap().validation_result.all_valid);
}
```

**Missing Template**:
```rust
#[test]
fn test_missing_template() -> Result<()> {
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("missing_template_fixture");

    // Should fallback to default templates
    let result = harness.run_complete_pipeline()?;
    assert!(!result.template_result.rendered_code.is_empty());

    Ok(())
}
```

## Assertions

### Stage Assertions

```rust
// All stages succeeded
harness.assert_all_stages_succeeded(&result);

// Code compiles
harness.assert_code_compiles(code)?;

// Imports valid
harness.assert_all_imports_valid(code)?;

// No unused code
harness.assert_no_unused_code(code)?;
```

### Golden File Assertions

```rust
// Compare against golden file
harness.assert_output_matches_golden(
    generated_code,
    &PathBuf::from("expected/User.rs")
)?;

// Comprehensive comparison
let report = harness.compare_golden_files(&result)?;
assert!(report.is_perfect_match());
```

### Custom Assertions

```rust
// Verify entity count
assert_eq!(result.sparql_result.entities.len(), 5);

// Verify file count
assert_eq!(result.file_result.written_files.len(), 5);

// Verify performance
assert!(result.duration.as_millis() < 1000);
```

## Golden File Testing

### Workflow

1. **Generate initial output**:
   ```bash
   cargo test test_simple_aggregate -- --ignored
   ```

2. **Review generated code**:
   ```bash
   cat target/test_output/simple_aggregate/User.rs
   ```

3. **Update golden files** (if correct):
   ```bash
   cargo test test_update_golden_files -- --ignored
   ```

4. **Run comparison**:
   ```bash
   cargo test test_golden_file_comparison
   ```

### Golden File Report

```rust
let report = harness.compare_golden_files(&result)?;

report.print_summary();
// Output:
// 📋 Golden File Comparison:
//   ✓ Matches:    3
//   ✗ Mismatches: 1
//   ? Missing:    0
//
//   Mismatched files:
//     - User.rs

if !report.is_perfect_match() {
    println!("Run with UPDATE_GOLDEN=1 to update");
}
```

## Incremental Testing

### Change Detection

```rust
#[test]
fn test_incremental_updates() -> Result<()> {
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_incremental(true);

    // First run - generates all files
    let result1 = harness.run_complete_pipeline()?;
    let count1 = result1.file_result.written_files.len();

    // Second run - should detect no changes
    let result2 = harness.run_complete_pipeline()?;
    let count2 = result2.file_result.written_files.len();

    assert_eq!(count1, count2);

    Ok(())
}
```

### Dependency Tracking

The harness uses `ArtifactTracker` to manage dependencies:

```rust
// Automatically tracked during file writing
self.artifact_tracker.track_artifact(
    &output_path,
    code.as_bytes(),
    dependencies,
)?;

// Query dependencies later
let deps = self.artifact_tracker.get_dependencies(&file_path)?;
```

## Performance Benchmarks

### Pipeline Metrics

```rust
#[test]
fn test_performance() -> Result<()> {
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate");

    let result = harness.run_complete_pipeline()?;

    // Print detailed metrics
    harness.metrics.print_summary();
    // Output:
    // 📊 Pipeline Performance Metrics:
    //   Ontology Loading:    12ms
    //   SPARQL Query:        8ms
    //   Template Rendering:  15ms
    //   Code Validation:     45ms
    //   File Writing:        5ms
    //   ─────────────────────────────────
    //   Total:               85ms

    Ok(())
}
```

### Performance Assertions

```rust
// Simple aggregate should complete quickly
assert!(result.duration.as_millis() < 5000);

// Complex domain has higher threshold
assert!(result.duration.as_millis() < 10000);

// Individual stage thresholds
assert!(result.ontology_result.duration.as_millis() < 100);
assert!(result.sparql_result.duration.as_millis() < 50);
```

## Integration Points

### Programmatic API

```rust
use codegen_pipeline_harness::*;

fn generate_code_from_ontology(ttl_path: &Path) -> Result<Vec<PathBuf>> {
    let mut harness = CodegenPipelineHarness::new()
        .with_validation(true);

    // Custom fixture from runtime path
    // ... configure harness ...

    let result = harness.run_complete_pipeline()?;

    Ok(result.file_result.written_files)
}
```

### CLI Integration

```bash
# Run specific fixture
cargo test test_simple_aggregate

# Run all pipeline tests
cargo test codegen_pipeline

# Run with golden file updates
cargo test test_update_golden_files -- --ignored

# Performance benchmarks
cargo test test_.*_performance
```

### CI/CD Integration

```yaml
# .github/workflows/test.yml
- name: Run Pipeline Tests
  run: |
    cargo test --test codegen_pipeline_integration_tests

- name: Verify Golden Files
  run: |
    cargo test test_golden_file_comparison

- name: Performance Check
  run: |
    cargo test test_pipeline_performance
```

## Best Practices

### 80/20 Principle

Focus on the 20% of functionality that provides 80% of value:

1. **Core pipeline stages** (must work)
2. **Common use cases** (simple aggregate, basic commands)
3. **Error handling** (invalid input, missing files)
4. **Performance** (reasonable completion times)

### Chicago TDD Guidelines

1. **Use real collaborators**: Test with actual Oxigraph, Tera, syn
2. **State-based assertions**: Verify actual state changes
3. **Minimal mocking**: Only mock external I/O
4. **End-to-end flows**: Test complete user scenarios

### Test Organization

```rust
// ✅ Good - One clear purpose
#[test]
fn test_ontology_loading_succeeds() { }

// ✅ Good - Tests complete flow
#[test]
fn test_simple_aggregate_pipeline() { }

// ❌ Bad - Tests implementation details
#[test]
fn test_internal_parser_state() { }
```

### Fixture Management

```rust
// ✅ Good - Reusable fixture
tests/fixtures/pipeline/simple_aggregate/

// ✅ Good - Specific scenario
tests/fixtures/pipeline/complete_domain/

// ❌ Bad - Test-specific fixture
tests/fixtures/pipeline/test_123_temp/
```

## Troubleshooting

### Common Issues

**Issue**: Test fails with "Fixture not found"
```rust
// Solution: Verify fixture directory exists
let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("tests/fixtures/pipeline/my_fixture");
assert!(path.exists());
```

**Issue**: Golden file mismatch
```bash
# Solution: Update golden files
cargo test test_update_golden_files -- --ignored

# Or manually inspect diff
diff expected/User.rs target/test_output/simple_aggregate/User.rs
```

**Issue**: Syntax validation fails
```rust
// Solution: Check generated code
println!("{}", code);

// Verify with rustfmt
let formatted = Command::new("rustfmt")
    .arg("--check")
    .arg(&file_path)
    .output()?;
```

## Examples

### Complete Test Example

```rust
#[test]
fn test_user_aggregate_generation() -> Result<()> {
    println!("\n🧪 Testing User Aggregate Generation");

    // Arrange
    let mut harness = CodegenPipelineHarness::new()
        .with_fixture("simple_aggregate")
        .with_validation(true)
        .with_golden_files(true);

    // Act
    let result = harness.run_complete_pipeline()?;

    // Assert - Stage by stage
    println!("  ✓ Stage 1: Loaded {} triples",
        result.ontology_result.triple_count);
    assert!(result.ontology_result.triple_count > 0);

    println!("  ✓ Stage 2: Extracted {} entities",
        result.sparql_result.entities.len());
    assert_eq!(result.sparql_result.entities.len(), 2);

    println!("  ✓ Stage 3: Rendered {} files",
        result.template_result.rendered_code.len());
    assert!(result.template_result.rendered_code.contains_key("User.rs"));

    println!("  ✓ Stage 4: All code valid");
    assert!(result.validation_result.all_valid);

    println!("  ✓ Stage 5: Wrote {} files",
        result.file_result.written_files.len());
    assert_eq!(result.file_result.written_files.len(), 2);

    // Assert - Golden files
    let report = harness.compare_golden_files(&result)?;
    report.print_summary();

    // Assert - Performance
    harness.metrics.print_summary();
    assert!(result.duration.as_millis() < 5000);

    println!("\n  ✅ All checks passed!");

    Ok(())
}
```

## References

- [Chicago TDD Style](https://martinfowler.com/articles/mocksArentStubs.html)
- [Oxigraph RDF Store](https://github.com/oxigraph/oxigraph)
- [Tera Templates](https://tera.netlify.app/)
- [syn Rust Parser](https://docs.rs/syn/)

## Summary

The Chicago-style TDD Code Generation Pipeline Harness provides:

✅ **Complete pipeline validation** (TTL → Rust)
✅ **State-based testing** with real components
✅ **Golden file comparison** for regression prevention
✅ **Performance benchmarks** for optimization
✅ **Incremental testing** for change detection
✅ **Comprehensive fixtures** for all scenarios
✅ **Integration points** for CLI, API, and CI/CD

**Key Principles**:
- Test complete user flows, not implementation details
- Use real collaborators, minimal mocking
- Focus on the 80/20 - core functionality first
- Provide clear, actionable error messages
- Make tests fast and reliable
