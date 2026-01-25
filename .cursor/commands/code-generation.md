# Code Generation Pipeline Workflow - Multi-Step Command

## Purpose

This command guides agents through the complete code generation pipeline with 4-layer validation (Poka-Yoke), determinism verification, golden file regression testing, and audit trail generation.

**Core Principle**: Generated code must pass all validation layers before writing. Never edit generated code manually. All changes flow from ontology through SPARQL queries and templates.

## Workflow Overview

```
Step 1: Layer 1 (Input Validation) → Step 2: Layer 2 (Ontology Validation) → Step 3: Layer 3 (Generation Validation) → Step 4: Layer 4 (Runtime Validation) → Step 5: Write Artifacts (with Measurement & Control)
```

## 4-Layer Validation Architecture

### Overview

```
Request → Layer 1 → Layer 2 → Layer 3 → Layer 4 → Response
          ↓         ↓         ↓         ↓
       Params    Ontology  Generated  Runtime
       Guards    SHACL     Quality    Safety
                           Gates
```

**Principle**: Fail-fast at earliest layer possible. Each layer is independent and verifiable.

## Step-by-Step Instructions

### Step 1: Layer 1 - Input Validation (Parameter Guards)

**Action**: Validate tool parameters before any processing begins.

**Validates**:
- Required parameters present
- Parameter types correct
- Value constraints satisfied (ranges, lengths, patterns)
- File paths safe (no traversal)
- Enum values valid

**Using Input Guards**:
```rust
use spreadsheet_mcp::validation::input_guards::*;

// Validate non-empty string
validate_non_empty_string("entity_name", &params.entity_name)?;

// Validate path safety
validate_path_safe(&params.output_path)?;

// Validate enum value
validate_enum_value("operation", &params.operation, &["create", "update", "delete"])?;
```

**Example**:
```rust
pub async fn generate_code(
    params: GenerateCodeParams,
) -> Result<GenerateCodeResponse> {
    // Layer 1: Input validation
    validate_non_empty_string("entity_name", &params.entity_name)?;
    validate_path_safe(&params.output_path)?;
    
    // Proceed to Layer 2...
}
```

**If validation fails**: Reject request, return validation error

**If validation passes**: Proceed to Step 2

### Step 2: Layer 2 - Ontology Validation (SHACL)

**Action**: Validate ontology conforms to SHACL shapes before generation.

**Validates**:
- Required properties present
- Value types match expected types
- Constraints satisfied (min/max, patterns)
- Relationships correctly formed

**Using SHACL Validation**:
```rust
use spreadsheet_mcp::ontology::shacl::ShapeValidator;

let validator = ShapeValidator::from_turtle(&ontology_content)?;
let report = validator.validate(&store)?;

if !report.is_valid() {
    return Err(Error::OntologyValidationFailed {
        violations: report.violations,
    });
}
```

**SHACL Shape Example**:
```turtle
mcp:ToolShape
    a sh:NodeShape ;
    sh:targetClass mcp:Tool ;
    sh:property [
        sh:path mcp:name ;
        sh:minCount 1 ;
        sh:datatype xsd:string ;
    ] ;
    sh:property [
        sh:path mcp:inputSchema ;
        sh:nodeKind sh:IRI ;
    ] .
```

**If validation fails**: Stop pipeline, return SHACL violations

**If validation passes**: Proceed to Step 3

### Step 3: Layer 3 - Generation Validation (Output Quality Gates)

**Action**: Validate generated code quality before writing.

**Validates**:
- Syntax valid (parses correctly)
- Semantics correct (follows conventions)
- No TODO comments
- Deterministic (same inputs → same outputs)
- Matches golden files (if present)

**Using GeneratedCodeValidator**:
```rust
use spreadsheet_mcp::codegen::validation::GeneratedCodeValidator;

let validator = GeneratedCodeValidator::new();
let report = validator.validate(&generated_code, &output_path)?;

if !report.is_valid() {
    return Err(Error::CodeValidationFailed {
        errors: report.errors,
    });
}
```

**Validation checks**:
- Rust syntax validation
- Clippy checks (optional)
- Compilation smoke test (optional)
- Golden file comparison
- Determinism verification

**Determinism check**:
```rust
// Generate twice with same inputs
let output1 = generate_code(&inputs)?;
let output2 = generate_code(&inputs)?;

if output1 != output2 {
    return Err(Error::NonDeterministicGeneration);
}
```

**Golden file comparison**:
```rust
use spreadsheet_mcp::codegen::validation::GoldenFileValidator;

let validator = GoldenFileValidator::new();
let comparison = validator.compare_with_golden(
    &generated_code,
    "expected/entity.rs"
)?;

if comparison.has_mismatches() {
    // Review differences, update golden file if intentional
    return Err(Error::GoldenFileMismatch {
        mismatches: comparison.mismatches,
    });
}
```

**If validation fails**: Rollback changes, return validation errors

**If validation passes**: Proceed to Step 4

### Step 4: Layer 4 - Runtime Validation (Production Safety)

**Action**: Validate code works correctly at runtime.

**Validates**:
- Code compiles
- Tests pass
- No runtime errors
- Performance acceptable

**Using CodeGenPipeline**:
```rust
use spreadsheet_mcp::codegen::validation::CodeGenPipeline;

let mut pipeline = CodeGenPipeline::new();
pipeline.run_rustfmt = true;
pipeline.run_clippy = true;
pipeline.run_compile_check = true;

let result = pipeline.execute(
    &template_content,
    &rendered_code,
    &output_path
)?;

if !result.success {
    return Err(Error::PipelineFailed {
        errors: result.errors,
    });
}
```

**Compilation check**:
```bash
cargo make check
# Should compile without errors
```

**Test execution**:
```bash
cargo make test
# All tests should pass
```

**If validation fails**: Fix issues, regenerate, retry

**If validation passes**: Proceed to Step 5

### Step 5: Write Artifacts (with Measurement & Control)

**Action**: Write generated code atomically with audit trail.

**Using SafeCodeWriter**:
```rust
use spreadsheet_mcp::codegen::validation::SafeCodeWriter;

let writer = SafeCodeWriter::new();
let receipt = writer.write_atomic(
    &output_path,
    &generated_code,
    &metadata
)?;
```

**Atomic write process**:
1. Create backup of existing file (if exists)
2. Write to temporary file
3. Validate written file
4. Atomically rename to final path
5. Generate receipt with hashes

**Receipt generation**:
```rust
use spreadsheet_mcp::codegen::validation::GenerationReceipt;

let receipt = GenerationReceipt::new()
    .with_ontology_hash(&ontology_hash)
    .with_template_hash(&template_hash)
    .with_artifact_hash(&artifact_hash)
    .with_timestamp()
    .build()?;

// Save receipt
receipt.save_to_file("receipts/generation-2026-01-20.json")?;
```

**Measurement**:
- Generation time
- File size
- Validation errors
- Performance metrics

**Control**:
- Receipt verification
- Audit trail logging
- Rollback capability
- Incremental regeneration

**If write fails**: Rollback to backup, return error

**If write succeeds**: Generation complete ✅

## Complete Workflow Example

```rust
// Step 1: Layer 1 - Input Validation
validate_non_empty_string("entity_name", &params.entity_name)?;
validate_path_safe(&params.output_path)?;
// Input valid ✅

// Step 2: Layer 2 - Ontology Validation
let validator = ShapeValidator::from_turtle(&ontology)?;
let report = validator.validate(&store)?;
// Ontology valid ✅

// Step 3: Layer 3 - Generation Validation
let validator = GeneratedCodeValidator::new();
let report = validator.validate(&generated_code, &output_path)?;
// Generated code valid ✅

// Step 4: Layer 4 - Runtime Validation
let mut pipeline = CodeGenPipeline::new();
pipeline.run_compile_check = true;
let result = pipeline.execute(&template, &code, &output_path)?;
// Runtime valid ✅

// Step 5: Write Artifacts
let writer = SafeCodeWriter::new();
let receipt = writer.write_atomic(&output_path, &code, &metadata)?;
// Written ✅
```

## Integration with Ontology Sync

Code generation integrates with ontology sync workflow:

**During Sync**:
- All 4 layers validated automatically
- Generated code validated before writing
- Receipts created for all artifacts
- Audit trail recorded

**After Sync**:
- Receipts verified
- Golden files compared
- Tests run
- Metrics recorded

## Error Handling

### If Layer 1 Fails

**Symptoms**: Parameter validation errors

**Fix**:
1. Check parameter requirements
2. Fix parameter values
3. Retry request

### If Layer 2 Fails

**Symptoms**: SHACL validation errors

**Fix**:
1. Review SHACL violations
2. Fix ontology
3. Retry validation

### If Layer 3 Fails

**Symptoms**: Code validation errors, golden file mismatches

**Fix**:
1. Review validation errors
2. Fix template/query
3. Update golden file if intentional
4. Regenerate code

### If Layer 4 Fails

**Symptoms**: Compilation errors, test failures

**Fix**:
1. **DO NOT** edit generated code manually
2. Fix ontology/template/query
3. Regenerate code
4. Retry validation

## Best Practices

1. **Never Edit Generated Code**: Only edit ontology/queries/templates
2. **Validate All Layers**: Don't skip validation layers
3. **Use Golden Files**: Compare against expected output
4. **Verify Determinism**: Same inputs → same outputs
5. **Generate Receipts**: Track provenance of all artifacts
6. **Test After Generation**: Run tests after every sync
7. **Monitor Metrics**: Track generation performance

## Integration with Other Commands

- **[Ontology Sync](./ontology-sync.md)** - Full sync workflow with validation
- **[SPARQL Validation](./sparql-validation.md)** - Validate queries before generation
- **[Template Rendering](./template-rendering.md)** - Validate templates before generation
- **[Poka-Yoke Design](./poka-yoke-design.md)** - Prevent errors through design

## Documentation References

- **[VALIDATION_GUIDE.md](../../docs/VALIDATION_GUIDE.md)** - Detailed validation guide
- **[CODE_GENERATION_VALIDATION.md](../../docs/CODE_GENERATION_VALIDATION.md)** - Validation documentation
- **[src/codegen/validation.rs](../../src/codegen/validation.rs)** - Source code
- **[CODEGEN_VALIDATION_IMPLEMENTATION.md](../../CODEGEN_VALIDATION_IMPLEMENTATION.md)** - Implementation details

## Quick Reference

```rust
// Full code generation pipeline
validate_inputs(&params)?;                          // Layer 1: Input

let validator = ShapeValidator::from_turtle(&ont)?; // Layer 2: Ontology
validator.validate(&store)?;

let validator = GeneratedCodeValidator::new();      // Layer 3: Generated
validator.validate(&code, &path)?;

let mut pipeline = CodeGenPipeline::new();         // Layer 4: Runtime
pipeline.execute(&template, &code, &path)?;

let writer = SafeCodeWriter::new();                 // Write: Artifacts
writer.write_atomic(&path, &code, &metadata)?;
```
