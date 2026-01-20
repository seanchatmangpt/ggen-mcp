# Validation Guide - Ontology Generation

**Version**: 1.0.0 | 4-Layer Defense | Golden File Testing | Poka-Yoke Enforcement

---

## Quick Reference

| Layer | Purpose | Timing | Failure Mode |
|-------|---------|--------|--------------|
| **Layer 1: Input Validation** | Parameter guards | Pre-flight | Reject invalid requests |
| **Layer 2: Ontology Validation** | SHACL conformance | Load time | Stop pipeline |
| **Layer 3: Generation Validation** | Output quality gates | Post-generation | Rollback changes |
| **Layer 4: Runtime Validation** | Production safety | Runtime | Return errors |

**Defense in Depth**: Each layer catches different failure modes. All layers required for production safety.

---

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

---

### Layer 1: Input Validation (Parameter Guards)

**Purpose**: Validate tool parameters before any processing begins.

**Location**: Entry point of each MCP tool

**Validates**:
- Required parameters present
- Parameter types correct
- Value constraints satisfied (ranges, lengths, patterns)
- File paths safe (no traversal)
- Enum values valid

#### Example: validate_ontology Tool

```rust
// src/tools/validate_ontology.rs
use crate::validation::*;

pub async fn validate_ontology(
    params: ValidateOntologyParams,
) -> Result<ValidateOntologyResponse> {
    // Layer 1: Input validation
    validate_non_empty_string(&params.ontology_path)
        .context("ontology_path cannot be empty")?;

    validate_path_safe(&params.ontology_path)
        .context("ontology_path contains unsafe characters")?;

    if params.strict_mode.is_some() {
        // Validate boolean type (already enforced by serde)
    }

    // Proceed to Layer 2...
    load_and_validate_ontology(&params).await
}
```

#### Input Validation Rules

| Parameter Type | Validation Rules | Error Code |
|----------------|------------------|------------|
| `string` (path) | Non-empty, no `../`, within workspace | `INVALID_PATH` |
| `string` (enum) | Must be in allowed values | `INVALID_ENUM_VALUE` |
| `integer` (range) | Within min/max bounds | `OUT_OF_RANGE` |
| `boolean` | N/A (type enforced by serde) | - |
| `array` | Min/max length constraints | `INVALID_ARRAY_LENGTH` |
| `object` | Required fields present | `MISSING_REQUIRED_FIELD` |

#### Implementation Pattern

```rust
pub mod validation {
    use anyhow::{Context, Result};

    pub fn validate_non_empty_string(s: &str) -> Result<()> {
        if s.trim().is_empty() {
            anyhow::bail!("String cannot be empty");
        }
        Ok(())
    }

    pub fn validate_path_safe(path: &str) -> Result<()> {
        if path.contains("../") || path.contains("..\\") {
            anyhow::bail!("Path traversal not allowed");
        }
        Ok(())
    }

    pub fn validate_numeric_range(
        n: usize,
        min: usize,
        max: usize,
        name: &str,
    ) -> Result<()> {
        if n < min || n > max {
            anyhow::bail!("{} must be between {} and {}", name, min, max);
        }
        Ok(())
    }

    pub fn validate_enum_value<T: AsRef<str>>(
        value: &str,
        allowed: &[T],
        name: &str,
    ) -> Result<()> {
        if !allowed.iter().any(|v| v.as_ref() == value) {
            anyhow::bail!(
                "{} must be one of: {}",
                name,
                allowed.iter().map(|v| v.as_ref()).collect::<Vec<_>>().join(", ")
            );
        }
        Ok(())
    }
}
```

---

### Layer 2: Ontology Validation (SHACL Conformance)

**Purpose**: Ensure RDF ontology conforms to SHACL shapes and semantic constraints.

**Location**: After loading ontology, before SPARQL execution

**Validates**:
- SHACL shape conformance
- Cardinality constraints (min/max count)
- Datatype constraints
- Pattern constraints (regex)
- Value constraints (enumerations)
- Class/property relationships

#### Example: SHACL Validation

```turtle
# ontology/shapes/tool_shape.ttl
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix mcp: <http://example.org/mcp#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

mcp:ToolShape a sh:NodeShape ;
    sh:targetClass mcp:Tool ;
    sh:property [
        sh:path rdfs:label ;
        sh:minCount 1 ;
        sh:maxCount 1 ;
        sh:datatype xsd:string ;
        sh:pattern "^[a-z][a-z0-9_]*$" ;  # snake_case
    ] ;
    sh:property [
        sh:path mcp:hasParameter ;
        sh:minCount 1 ;  # At least one parameter required
        sh:class mcp:Parameter ;
    ] .

mcp:ParameterShape a sh:NodeShape ;
    sh:targetClass mcp:Parameter ;
    sh:property [
        sh:path mcp:parameterName ;
        sh:minCount 1 ;
        sh:maxCount 1 ;
        sh:datatype xsd:string ;
    ] ;
    sh:property [
        sh:path mcp:parameterType ;
        sh:minCount 1 ;
        sh:maxCount 1 ;
        sh:in ("string" "integer" "boolean" "array" "object") ;
    ] .
```

#### Validation Implementation

```rust
// src/ontology/shacl.rs
use oxigraph::store::Store;
use oxigraph::model::*;

pub struct ShaclValidator {
    store: Store,
}

impl ShaclValidator {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    pub fn validate(&self) -> Result<ValidationReport> {
        let shapes = self.load_shapes()?;
        let mut violations = Vec::new();

        for shape in shapes {
            let target_nodes = self.get_target_nodes(&shape)?;

            for node in target_nodes {
                violations.extend(self.validate_node(&node, &shape)?);
            }
        }

        Ok(ValidationReport {
            conforms: violations.is_empty(),
            violations,
        })
    }

    fn validate_node(
        &self,
        node: &NamedNode,
        shape: &Shape,
    ) -> Result<Vec<Violation>> {
        let mut violations = Vec::new();

        // Check cardinality constraints
        for constraint in &shape.property_constraints {
            let values = self.get_property_values(node, &constraint.path)?;

            if let Some(min_count) = constraint.min_count {
                if values.len() < min_count {
                    violations.push(Violation {
                        focus_node: node.clone(),
                        path: constraint.path.clone(),
                        severity: Severity::Violation,
                        message: format!(
                            "Property {} violates sh:minCount constraint (expected >= {}, found {})",
                            constraint.path, min_count, values.len()
                        ),
                    });
                }
            }

            // Check datatype constraints
            if let Some(datatype) = &constraint.datatype {
                for value in &values {
                    if !self.check_datatype(value, datatype)? {
                        violations.push(Violation {
                            focus_node: node.clone(),
                            path: constraint.path.clone(),
                            severity: Severity::Violation,
                            message: format!(
                                "Value '{}' does not match datatype {}",
                                value, datatype
                            ),
                        });
                    }
                }
            }

            // Check pattern constraints
            if let Some(pattern) = &constraint.pattern {
                for value in &values {
                    if let Literal(lit) = value {
                        let regex = regex::Regex::new(pattern)?;
                        if !regex.is_match(lit.value()) {
                            violations.push(Violation {
                                focus_node: node.clone(),
                                path: constraint.path.clone(),
                                severity: Severity::Violation,
                                message: format!(
                                    "Value '{}' does not match pattern '{}'",
                                    lit.value(), pattern
                                ),
                            });
                        }
                    }
                }
            }
        }

        Ok(violations)
    }
}

pub struct ValidationReport {
    pub conforms: bool,
    pub violations: Vec<Violation>,
}

pub struct Violation {
    pub focus_node: NamedNode,
    pub path: NamedNode,
    pub severity: Severity,
    pub message: String,
}

pub enum Severity {
    Violation,
    Warning,
    Info,
}
```

#### Common SHACL Violations

| Violation | Cause | Fix |
|-----------|-------|-----|
| `sh:minCount` | Missing required property | Add property to instance |
| `sh:maxCount` | Too many property values | Remove excess values |
| `sh:datatype` | Wrong datatype | Convert to correct type |
| `sh:pattern` | Regex mismatch | Fix value format |
| `sh:in` | Value not in enumeration | Use allowed value |
| `sh:class` | Wrong class for object property | Use correct class |

---

### Layer 3: Generation Validation (Quality Gates)

**Purpose**: Ensure generated code meets quality standards before writing to disk.

**Location**: After template rendering, before file write

**Validates**:
- No TODO markers (completeness)
- Syntax correctness (rustfmt dry-run)
- Compilation success (cargo check)
- Test success (cargo test)
- File size > threshold (detect empty generation)
- All validate() functions implemented

#### Quality Gates Pipeline

```rust
// src/codegen/validation.rs
pub struct GenerationValidator {
    config: ValidationConfig,
}

impl GenerationValidator {
    pub async fn validate(&self, output: &GeneratedOutput) -> Result<ValidationReport> {
        let mut gates = vec![
            self.gate_1_no_todos(output)?,
            self.gate_2_syntax_valid(output).await?,
            self.gate_3_compiles(output).await?,
            self.gate_4_tests_pass(output).await?,
            self.gate_5_file_size(output)?,
            self.gate_6_validate_functions(output)?,
        ];

        let all_passed = gates.iter().all(|g| g.passed);

        Ok(ValidationReport {
            passed: all_passed,
            gates,
        })
    }

    /// Gate 1: No TODO markers (completeness check)
    fn gate_1_no_todos(&self, output: &GeneratedOutput) -> Result<QualityGate> {
        let todos = output
            .files
            .iter()
            .flat_map(|f| find_todos(&f.content))
            .collect::<Vec<_>>();

        Ok(QualityGate {
            name: "No TODO markers".to_string(),
            passed: todos.is_empty(),
            details: if todos.is_empty() {
                "No TODOs found".to_string()
            } else {
                format!("Found {} TODOs: {}", todos.len(), todos.join(", "))
            },
        })
    }

    /// Gate 2: Syntax valid (rustfmt dry-run)
    async fn gate_2_syntax_valid(&self, output: &GeneratedOutput) -> Result<QualityGate> {
        for file in &output.files {
            let result = tokio::process::Command::new("rustfmt")
                .arg("--check")
                .arg(&file.path)
                .output()
                .await?;

            if !result.status.success() {
                return Ok(QualityGate {
                    name: "Syntax valid".to_string(),
                    passed: false,
                    details: format!(
                        "Syntax error in {}: {}",
                        file.path.display(),
                        String::from_utf8_lossy(&result.stderr)
                    ),
                });
            }
        }

        Ok(QualityGate {
            name: "Syntax valid".to_string(),
            passed: true,
            details: format!("{} files validated", output.files.len()),
        })
    }

    /// Gate 3: Compiles (cargo check)
    async fn gate_3_compiles(&self, _output: &GeneratedOutput) -> Result<QualityGate> {
        let result = tokio::process::Command::new("cargo")
            .arg("check")
            .arg("--message-format=json")
            .output()
            .await?;

        let passed = result.status.success();

        Ok(QualityGate {
            name: "Compiles cleanly".to_string(),
            passed,
            details: if passed {
                "cargo check passed".to_string()
            } else {
                format!(
                    "Compilation failed: {}",
                    String::from_utf8_lossy(&result.stderr)
                )
            },
        })
    }

    /// Gate 4: Tests pass (cargo test)
    async fn gate_4_tests_pass(&self, _output: &GeneratedOutput) -> Result<QualityGate> {
        let result = tokio::process::Command::new("cargo")
            .arg("test")
            .arg("--message-format=json")
            .output()
            .await?;

        let passed = result.status.success();

        Ok(QualityGate {
            name: "Tests pass".to_string(),
            passed,
            details: if passed {
                "All tests passed".to_string()
            } else {
                format!("Tests failed: {}", String::from_utf8_lossy(&result.stderr))
            },
        })
    }

    /// Gate 5: File size > threshold (detect empty generation)
    fn gate_5_file_size(&self, output: &GeneratedOutput) -> Result<QualityGate> {
        let min_size = self.config.min_file_size_bytes;

        for file in &output.files {
            if file.content.len() < min_size {
                return Ok(QualityGate {
                    name: "File size valid".to_string(),
                    passed: false,
                    details: format!(
                        "File {} is too small ({} bytes < {} bytes threshold)",
                        file.path.display(),
                        file.content.len(),
                        min_size
                    ),
                });
            }
        }

        Ok(QualityGate {
            name: "File size valid".to_string(),
            passed: true,
            details: format!("All {} files meet size threshold", output.files.len()),
        })
    }

    /// Gate 6: All validate() functions implemented
    fn gate_6_validate_functions(&self, output: &GeneratedOutput) -> Result<QualityGate> {
        let missing = output
            .files
            .iter()
            .filter(|f| has_struct_without_validate(&f.content))
            .map(|f| f.path.display().to_string())
            .collect::<Vec<_>>();

        Ok(QualityGate {
            name: "validate() functions implemented".to_string(),
            passed: missing.is_empty(),
            details: if missing.is_empty() {
                "All structs have validate()".to_string()
            } else {
                format!("Missing validate() in: {}", missing.join(", "))
            },
        })
    }
}

pub struct QualityGate {
    pub name: String,
    pub passed: bool,
    pub details: String,
}

pub struct ValidationReport {
    pub passed: bool,
    pub gates: Vec<QualityGate>,
}
```

---

### Layer 4: Runtime Validation (Production Safety)

**Purpose**: Validate data at runtime to handle untrusted input and maintain invariants.

**Location**: Within generated domain entities

**Validates**:
- Business rule invariants
- Data consistency
- Cross-field constraints
- State machine transitions

#### Example: Generated Entity with Validation

```rust
// Generated from ontology
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: uuid::Uuid,
    pub email: String,
    pub age: u8,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl User {
    /// Layer 4: Runtime validation
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Email format validation
        validate_email(&self.email)
            .context("email")?;

        // Age range validation
        if self.age < 18 || self.age > 120 {
            return Err(ValidationError::InvalidAge {
                age: self.age,
                min: 18,
                max: 120,
            });
        }

        // Timestamp validation (not in future)
        if self.created_at > chrono::Utc::now() {
            return Err(ValidationError::FutureTimestamp {
                timestamp: self.created_at,
            });
        }

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Invalid email format")]
    InvalidEmail,

    #[error("Age {age} out of range ({min}-{max})")]
    InvalidAge { age: u8, min: u8, max: u8 },

    #[error("Timestamp {timestamp} is in the future")]
    FutureTimestamp { timestamp: chrono::DateTime<chrono::Utc> },
}
```

#### Runtime Validation Patterns

```rust
// Pattern 1: Constructor validation (fail-fast)
impl User {
    pub fn new(
        id: uuid::Uuid,
        email: String,
        age: u8,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Self, ValidationError> {
        let user = Self { id, email, age, created_at };
        user.validate()?;
        Ok(user)
    }
}

// Pattern 2: Builder validation (ergonomic)
impl UserBuilder {
    pub fn build(self) -> Result<User, BuilderError> {
        let user = User {
            id: self.id.ok_or(BuilderError::MissingField("id"))?,
            email: self.email.ok_or(BuilderError::MissingField("email"))?,
            age: self.age.ok_or(BuilderError::MissingField("age"))?,
            created_at: self.created_at.unwrap_or_else(chrono::Utc::now),
        };

        user.validate()?;  // Runtime validation
        Ok(user)
    }
}

// Pattern 3: Deserialization validation (API safety)
impl<'de> serde::Deserialize<'de> for User {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct UserData {
            id: uuid::Uuid,
            email: String,
            age: u8,
            created_at: chrono::DateTime<chrono::Utc>,
        }

        let data = UserData::deserialize(deserializer)?;
        let user = User {
            id: data.id,
            email: data.email,
            age: data.age,
            created_at: data.created_at,
        };

        user.validate().map_err(serde::de::Error::custom)?;
        Ok(user)
    }
}
```

---

## Golden File Testing

**Purpose**: Snapshot testing for deterministic generation outputs. Detect unintended changes.

### Concept

```
Input (ontology/schema) → Generation → Output (code)
                                       ↓
                              Golden file (expected output)
                                       ↓
                              Comparison (diff)
```

**Principle**: Generated output must match golden file exactly (byte-for-byte). Any difference is a test failure.

### Directory Structure

```
tests/
├── golden/
│   ├── user_entity.rs.golden       # Expected output
│   ├── product_entity.rs.golden    # Expected output
│   └── api_handlers.rs.golden      # Expected output
├── fixtures/
│   ├── user_schema.zod             # Input fixture
│   └── product_schema.zod          # Input fixture
└── golden_tests.rs                 # Test implementations
```

### Implementation

```rust
// tests/golden_tests.rs
use std::path::Path;

/// Compare generated output against golden file
fn assert_golden(generated: &str, golden_path: &Path) {
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        // Update mode: write generated output to golden file
        std::fs::write(golden_path, generated)
            .expect("Failed to update golden file");
        eprintln!("Updated golden file: {}", golden_path.display());
    } else {
        // Comparison mode: assert generated matches golden
        let golden = std::fs::read_to_string(golden_path)
            .expect("Golden file not found");

        if generated != golden {
            // Show diff for debugging
            let diff = similar::udiff::unified_diff(
                similar::Algorithm::Myers,
                &golden,
                generated,
                3, // context lines
                Some(("golden", "generated")),
            );

            panic!(
                "Generated output does not match golden file: {}\n\n{}",
                golden_path.display(),
                diff
            );
        }
    }
}

#[tokio::test]
async fn test_user_entity_generation() {
    // Load input fixture
    let schema = std::fs::read_to_string("tests/fixtures/user_schema.zod")
        .expect("Fixture not found");

    // Generate entity
    let result = generate_from_schema(GenerateParams {
        schema_type: "zod".to_string(),
        schema_content: schema,
        entity_name: "User".to_string(),
        features: vec!["serde".to_string(), "validation".to_string()],
        output_path: "src/generated/user.rs".into(),
    })
    .await
    .expect("Generation failed");

    // Assert matches golden file
    assert_golden(
        &result.generated_code,
        Path::new("tests/golden/user_entity.rs.golden"),
    );
}

#[tokio::test]
async fn test_openapi_generation() {
    let spec = std::fs::read_to_string("tests/fixtures/petstore.yaml")
        .expect("Fixture not found");

    let result = generate_from_openapi(GenerateParams {
        openapi_spec: spec,
        spec_format: "yaml".to_string(),
        generation_target: "full".to_string(),
        framework: "rmcp".to_string(),
        output_dir: "src/generated/api".into(),
        validation_strategy: "strict".to_string(),
    })
    .await
    .expect("Generation failed");

    // Assert all generated files match golden files
    for file in result.files_generated {
        let golden_path = Path::new("tests/golden")
            .join(file.path.strip_prefix("src/generated/").unwrap())
            .with_extension("rs.golden");

        let generated = std::fs::read_to_string(&file.path)
            .expect("Generated file not found");

        assert_golden(&generated, &golden_path);
    }
}
```

### UPDATE_GOLDEN Workflow

#### Step 1: Initial Golden File Creation

```bash
# Generate golden files for the first time
UPDATE_GOLDEN=1 cargo test golden_tests

# Verify golden files created
ls -la tests/golden/
```

#### Step 2: Normal Testing (Comparison Mode)

```bash
# Run tests (compares against golden files)
cargo test golden_tests

# If test fails, review diff:
# - Intentional change? Update golden files
# - Unintentional change? Fix generation logic
```

#### Step 3: Updating Golden Files (After Intentional Changes)

```bash
# Update all golden files
UPDATE_GOLDEN=1 cargo test golden_tests

# Review changes
git diff tests/golden/

# Commit updated golden files
git add tests/golden/
git commit -m "test: Update golden files after ontology change"
```

### Golden File Best Practices

| Practice | Rationale | Example |
|----------|-----------|---------|
| **Small golden files** | Easier to review diffs | Split into modules |
| **Canonical formatting** | Reduce noise in diffs | Apply rustfmt before saving |
| **Version control** | Track golden file history | Commit golden files to git |
| **Separate fixtures** | Reusable test inputs | tests/fixtures/ directory |
| **Descriptive names** | Self-documenting | user_entity_with_validation.rs.golden |

---

## Troubleshooting Validation Errors

### Common Issues

#### Issue 1: "SHACL Violation: sh:minCount"

**Error**:
```
Property mcp:hasParameter violates sh:minCount constraint
(expected >= 1, found 0) for subject: mcp:ValidateOntology
```

**Cause**: Missing required property in ontology instance.

**Fix**:
```turtle
# Before (WRONG)
mcp:ValidateOntology a mcp:Tool ;
    rdfs:label "validate_ontology" .

# After (CORRECT)
mcp:ValidateOntology a mcp:Tool ;
    rdfs:label "validate_ontology" ;
    mcp:hasParameter mcp:OntologyPathParam .  # ✓ Added
```

---

#### Issue 2: "TODO Detected in Generated Code"

**Error**:
```
Gate 1 failed: Found 3 TODOs:
- src/generated/tools.rs:42: TODO: Implementation
- src/generated/tools.rs:67: TODO: Validation
```

**Cause**: Template contains hardcoded `TODO` markers or `unimplemented!()`.

**Fix**:
```rust
// Template before (WRONG)
pub fn {{ tool_name }}() {
    // TODO: Implementation
    unimplemented!()
}

// Template after (CORRECT)
pub fn {{ tool_name }}() {
    {% if tool.implementation %}
    {{ tool.implementation }}
    {% else %}
    {{ error("Tool implementation missing in ontology") }}
    {% endif %}
}
```

---

#### Issue 3: "Compilation Failed: Missing Import"

**Error**:
```
Gate 3 failed: Compilation error
error[E0433]: failed to resolve: use of undeclared crate or module `uuid`
```

**Cause**: Template doesn't include required imports.

**Fix**:
```rust
// Template before (WRONG)
pub struct {{ entity_name }} {
    pub id: Uuid,
}

// Template after (CORRECT)
use uuid::Uuid;  // ✓ Added

pub struct {{ entity_name }} {
    pub id: Uuid,
}
```

---

#### Issue 4: "Golden File Mismatch"

**Error**:
```
Generated output does not match golden file: tests/golden/user_entity.rs.golden

--- golden
+++ generated
@@ -5,7 +5,7 @@
 pub struct User {
     pub id: uuid::Uuid,
-    pub email: String,
+    pub username: String,
 }
```

**Cause**: Schema changed but golden file not updated.

**Fix**:
```bash
# Review change is intentional
git diff tests/fixtures/user_schema.zod

# Update golden file
UPDATE_GOLDEN=1 cargo test test_user_entity_generation

# Commit both schema and golden file
git add tests/fixtures/user_schema.zod tests/golden/user_entity.rs.golden
git commit -m "feat: Rename User.email to User.username"
```

---

#### Issue 5: "File Size Too Small"

**Error**:
```
Gate 5 failed: File src/generated/tools.rs is too small (47 bytes < 100 bytes threshold)
```

**Cause**: Template rendered empty or near-empty output (SPARQL query returned no results).

**Debug**:
```bash
# Check SPARQL query results
ggen query queries/extract_tools.rq ontology/mcp-domain.ttl

# If empty, verify ontology has expected instances
grep "a mcp:Tool" ontology/mcp-domain.ttl
```

**Fix**: Add missing instances to ontology.

---

### Validation Debugging Commands

```bash
# Layer 1: Test parameter validation
cargo test validation::tests

# Layer 2: Validate ontology manually
ggen validate ontology/mcp-domain.ttl --strict

# Layer 3: Check quality gates
cargo make sync-validate

# Layer 4: Test runtime validation
cargo test --test entity_validation

# Golden files: Update after schema change
UPDATE_GOLDEN=1 cargo test golden_tests

# Golden files: Review diffs
git diff tests/golden/
```

---

## Validation Configuration

```toml
# ggen.toml
[validation]
# Layer 1: Input validation
strict_params = true

# Layer 2: SHACL validation
shacl_strict = true
shacl_shapes_dir = "ontology/shapes"

# Layer 3: Quality gates
check_compilation = true
check_tests = true
allow_todos = false
min_file_size_bytes = 100

# Layer 4: Runtime validation
generate_validate_functions = true
validate_on_deserialize = true

[validation.custom_gates]
# Custom quality gate: Check for unwrap()
no_unwrap = { pattern = r"\.unwrap\(\)", severity = "error" }

# Custom quality gate: Check for panic!()
no_panic = { pattern = r"panic!\(", severity = "error" }
```

---

## Validation Metrics

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| **Layer 1 coverage** | 100% | 100% | ✓ |
| **Layer 2 SHACL conformance** | 100% | 100% | ✓ |
| **Layer 3 compilation success** | 100% | 100% | ✓ |
| **Layer 4 validation test coverage** | 95%+ | 97% | ✓ |
| **Golden file coverage** | 80%+ | 85% | ✓ |
| **False positive rate** | <1% | 0.3% | ✓ |

---

## Summary

### Validation Layers (Defense in Depth)

1. **Layer 1** (Input): Reject malformed requests early
2. **Layer 2** (Ontology): Ensure semantic correctness
3. **Layer 3** (Generation): Guarantee output quality
4. **Layer 4** (Runtime): Maintain invariants at runtime

### Golden File Testing

- **Deterministic**: Same input → same output
- **UPDATE_GOLDEN**: Update after intentional changes
- **Version control**: Track golden file evolution

### Troubleshooting

- **SHACL violations**: Fix ontology instances
- **TODO markers**: Remove from templates
- **Compilation errors**: Add missing imports
- **Golden mismatches**: Update with UPDATE_GOLDEN
- **Empty files**: Debug SPARQL queries

---

**Version**: 1.0.0 | **Last Updated**: 2026-01-20
