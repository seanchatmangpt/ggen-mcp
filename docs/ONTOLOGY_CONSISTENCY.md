# Ontology Consistency Checking and Validation

Comprehensive guide to ontology validation and consistency checking in the ggen-mcp system.

## Table of Contents

1. [Overview](#overview)
2. [Components](#components)
3. [Validation Rules](#validation-rules)
4. [Common Consistency Errors](#common-consistency-errors)
5. [Integration with SHACL Shapes](#integration-with-shacl-shapes)
6. [Performance Considerations](#performance-considerations)
7. [Troubleshooting Guide](#troubleshooting-guide)
8. [API Reference](#api-reference)

## Overview

The ontology consistency checking system provides comprehensive validation for RDF-based DDD ontologies used in code generation. It ensures that ontologies are structurally sound, semantically consistent, and conform to expected DDD patterns before code generation.

### Key Features

- **Consistency Checking**: Validates RDF graph integrity (cycles, domains, ranges, cardinality)
- **Schema Validation**: Ensures conformance to DDD patterns and required namespaces
- **Namespace Management**: Safe handling of namespace prefixes with collision detection
- **Ontology Merging**: Conflict detection and safe merging of multiple ontologies
- **Hash Verification**: Cryptographic integrity checking to detect tampering

### Why Consistency Checking Matters

1. **Prevents Code Generation Errors**: Invalid ontologies lead to broken generated code
2. **Catches Logic Errors Early**: Detects contradictions and structural issues before deployment
3. **Ensures DDD Compliance**: Validates that domain models follow DDD best practices
4. **Maintains Data Quality**: Prevents corruption and ensures ontology integrity over time

## Components

### 1. ConsistencyChecker

Validates RDF graph consistency through comprehensive structural analysis.

**Checks Performed:**
- Class hierarchy validation (detects cycles)
- Property domain/range type checking
- SHACL cardinality constraints
- Contradiction detection
- Required property presence

**Usage Example:**

```rust
use spreadsheet_mcp::ontology::{ConsistencyChecker, Store};
use oxigraph::store::Store;

// Load your ontology
let store = Store::new()?;
store.load_from_file("ontology/mcp-domain.ttl", GraphFormat::Turtle)?;

// Run consistency checks
let checker = ConsistencyChecker::new(store);
let report = checker.check_all();

if report.valid {
    println!("✓ Ontology is consistent");
    println!("  - {} classes", report.stats.total_classes);
    println!("  - {} properties", report.stats.total_properties);
    println!("  - Max hierarchy depth: {}", report.stats.max_hierarchy_depth);
} else {
    eprintln!("✗ Consistency errors found:");
    for error in &report.errors {
        eprintln!("  - {}", error);
    }
}

// Check for warnings
if !report.warnings.is_empty() {
    println!("⚠ Warnings:");
    for warning in &report.warnings {
        println!("  - {}", warning);
    }
}
```

### 2. SchemaValidator

Validates ontologies against expected schema patterns and DDD conventions.

**Checks Performed:**
- Required namespace presence (ddd, ggen, sh, rdfs, xsd)
- DDD aggregate structure validation
- Property type declarations
- Invariant definition correctness
- Orphaned node detection

**Usage Example:**

```rust
use spreadsheet_mcp::ontology::SchemaValidator;

let validator = SchemaValidator::new(store);
let report = validator.validate_all();

for error in &report.errors {
    match error {
        ValidationError::MissingNamespace { prefix, expected_uri } => {
            eprintln!("Missing namespace: {} ({})", prefix, expected_uri);
        }
        ValidationError::InvalidDddStructure { aggregate, reason } => {
            eprintln!("Invalid DDD structure in {}: {}", aggregate, reason);
        }
        _ => eprintln!("Validation error: {}", error),
    }
}
```

### 3. NamespaceManager

Manages namespace prefixes with collision detection and safe URI expansion.

**Features:**
- Register and validate namespace prefixes
- Prevent prefix collisions
- Expand QNames to full URIs
- Compact URIs to QNames
- Default namespace handling

**Usage Example:**

```rust
use spreadsheet_mcp::ontology::NamespaceManager;

let mut ns = NamespaceManager::new();

// Register custom namespace
ns.register("mcp", "http://ggen-mcp.dev/ontology/mcp#")?;

// Set default namespace
ns.set_default("http://ggen-mcp.dev/ontology/");

// Expand QName to full URI
let full_uri = ns.expand("mcp:Tool")?;
assert_eq!(full_uri, "http://ggen-mcp.dev/ontology/mcp#Tool");

// Compact URI to QName
let qname = ns.compact("http://ggen-mcp.dev/ontology/mcp#Resource");
assert_eq!(qname, "mcp:Resource");

// Detect collisions
match ns.register("mcp", "http://different-uri.dev/mcp#") {
    Err(ValidationError::NamespaceCollision { prefix, uri1, uri2 }) => {
        eprintln!("Collision detected for prefix '{}': {} vs {}", prefix, uri1, uri2);
    }
    _ => {}
}
```

### 4. OntologyMerger

Safely merges multiple ontologies with conflict detection and rollback.

**Features:**
- Pre-merge conflict detection
- Provenance tracking
- Duplicate definition handling
- Post-merge validation
- Automatic rollback on failure

**Usage Example:**

```rust
use spreadsheet_mcp::ontology::OntologyMerger;

let merger = OntologyMerger::new();

// Load source and target ontologies
let target = Store::new()?;
target.load_from_file("ontology/base.ttl", GraphFormat::Turtle)?;

let source = Store::new()?;
source.load_from_file("ontology/extension.ttl", GraphFormat::Turtle)?;

// Attempt merge
let result = merger.merge(&target, &source)?;

if result.success {
    println!("✓ Merged {} triples successfully", result.merged_triples);
} else {
    eprintln!("✗ Merge failed with conflicts:");
    for conflict in &result.conflicts {
        eprintln!("  - {}", conflict);
    }
}
```

### 5. HashVerifier

Verifies ontology integrity using SHA-256 cryptographic hashes.

**Features:**
- Compute consistent hashes for ontologies
- Detect tampering or corruption
- Verify against stored hash values
- Track version changes
- Automatic hash storage and retrieval

**Usage Example:**

```rust
use spreadsheet_mcp::ontology::HashVerifier;

let verifier = HashVerifier::new(store);

// Compute current hash
let hash = verifier.compute_hash()?;
println!("Ontology hash: {}", hash);

// Verify against expected hash
match verifier.verify_hash("a1b2c3...") {
    Ok(_) => println!("✓ Hash verification passed"),
    Err(ValidationError::HashMismatch { expected, actual }) => {
        eprintln!("✗ Hash mismatch! Possible tampering detected.");
        eprintln!("  Expected: {}", expected);
        eprintln!("  Actual:   {}", actual);
    }
    Err(e) => eprintln!("Error: {}", e),
}

// Store hash in ontology metadata
verifier.store_hash(&hash)?;

// Verify and auto-update
let is_valid = verifier.verify_and_update()?;
```

## Validation Rules

### Class Hierarchy Rules

#### Rule: No Cyclic Inheritance

**Rationale:** Cyclic class hierarchies are logically inconsistent and cause infinite loops in reasoners and code generators.

**Detection:** Depth-first search to detect back edges in the class hierarchy graph.

**Example of Invalid Ontology:**

```turtle
@prefix ex: <http://example.org/> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

ex:A rdfs:subClassOf ex:B .
ex:B rdfs:subClassOf ex:C .
ex:C rdfs:subClassOf ex:A .  # Creates cycle: A -> B -> C -> A
```

**Error Message:**
```
Cyclic class hierarchy detected: http://example.org/A -> http://example.org/B ->
http://example.org/C -> http://example.org/A
```

**Fix:** Remove one of the subclass relationships to break the cycle.

#### Rule: Finite Hierarchy Depth

**Rationale:** Excessively deep hierarchies indicate design problems and hurt code generation performance.

**Recommendation:** Keep hierarchy depth ≤ 5 levels for optimal code generation.

### Property Domain/Range Rules

#### Rule: Subject Must Match Property Domain

**Rationale:** Using a property on an instance that doesn't match the property's domain violates the ontology's type system.

**Example of Invalid Usage:**

```turtle
@prefix mcp: <http://ggen-mcp.dev/ontology/mcp#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

# Property definition
mcp:toolName a owl:DatatypeProperty ;
    rdfs:domain mcp:Tool ;
    rdfs:range xsd:string .

# Invalid usage - Resource is not a Tool
ex:MyResource a mcp:Resource ;
    mcp:toolName "invalid" .  # ERROR: domain violation
```

**Error Message:**
```
Invalid domain/range for property mcp:toolName: ex:MyResource -> xsd:string
(Subject not in property domain)
```

**Fix:** Use the property only on instances of the correct class.

#### Rule: Object Must Match Property Range

**Rationale:** Property values must conform to the declared range type.

**Example:**

```turtle
mcp:inputSchema a owl:ObjectProperty ;
    rdfs:domain mcp:Tool ;
    rdfs:range mcp:JsonSchema .

# Invalid - range should be JsonSchema, not string
ex:MyTool a mcp:Tool ;
    mcp:inputSchema "not a JsonSchema object" .  # ERROR: range violation
```

### Cardinality Rules

#### Rule: Minimum Cardinality Constraints

**Rationale:** Required properties must be present for instances to be valid.

**SHACL Definition:**

```turtle
mcp:ToolShape a sh:NodeShape ;
    sh:targetClass mcp:Tool ;
    sh:property [
        sh:path mcp:toolName ;
        sh:minCount 1 ;  # Exactly one required
        sh:maxCount 1 ;
        sh:message "Tool must have exactly one name"
    ] .
```

**Error Example:**

```turtle
ex:MyTool a mcp:Tool ;
    # Missing required toolName property
    mcp:toolDescription "A tool without a name" .
```

**Error Message:**
```
Cardinality violation at ex:MyTool: property mcp:toolName expected at least 1, found 0
```

#### Rule: Maximum Cardinality Constraints

**Rationale:** Prevents duplicate values where only one is expected.

**Error Example:**

```turtle
ex:MyTool a mcp:Tool ;
    mcp:toolName "first_name" ;
    mcp:toolName "second_name" .  # ERROR: maxCount = 1
```

**Error Message:**
```
Cardinality violation at ex:MyTool: property mcp:toolName expected at most 1, found 2
```

### DDD Structure Rules

#### Rule: Aggregates Must Have Properties

**Rationale:** An aggregate root without properties is not a valid domain model.

**Example:**

```turtle
ex:EmptyAggregate a owl:Class ;
    rdfs:subClassOf ddd:AggregateRoot ;
    rdfs:label "Empty Aggregate" .
    # ERROR: No ddd:hasProperty declarations
```

**Error Message:**
```
Invalid DDD structure for ex:EmptyAggregate: Aggregate has no properties defined
```

**Fix:** Add at least one property:

```turtle
ex:EmptyAggregate a owl:Class ;
    rdfs:subClassOf ddd:AggregateRoot ;
    rdfs:label "Empty Aggregate" ;
    ddd:hasProperty ex:someProperty .
```

#### Rule: Invariants Must Have Check Expressions

**Rationale:** Invariants without check expressions cannot be validated.

**Example:**

```turtle
mcp:Tool ddd:hasInvariant [
    rdfs:label "Tool name must be valid" ;
    # ERROR: Missing ddd:check expression
    ddd:message "Tool name must be a valid identifier"
] .
```

**Error Message:**
```
Invalid invariant at mcp:Tool: Invariant has no check expression
```

**Fix:**

```turtle
mcp:Tool ddd:hasInvariant [
    rdfs:label "Tool name must be valid" ;
    ddd:check "self.name.is_valid_identifier()" ;
    ddd:message "Tool name must be a valid identifier"
] .
```

### Namespace Rules

#### Rule: Required Namespaces Present

**Rationale:** Code generation requires specific vocabularies to be available.

**Required Namespaces:**

| Prefix | URI | Purpose |
|--------|-----|---------|
| `ddd` | `http://ggen-mcp.dev/ontology/ddd#` | DDD patterns |
| `ggen` | `http://ggen-mcp.dev/ontology/` | ggen metadata |
| `sh` | `http://www.w3.org/ns/shacl#` | Validation shapes |
| `rdfs` | `http://www.w3.org/2000/01/rdf-schema#` | RDF Schema |
| `xsd` | `http://www.w3.org/2001/XMLSchema#` | Datatypes |

**Warning (not error):** Missing recommended namespaces will generate warnings but won't fail validation.

## Common Consistency Errors

### 1. Cyclic Class Hierarchy

**Symptom:** Code generator enters infinite loop or crashes.

**Cause:** Classes form a circular inheritance chain.

**Detection:**
```rust
let checker = ConsistencyChecker::new(store);
let report = checker.check_all();

for error in &report.errors {
    if let ValidationError::CyclicHierarchy { cycle } = error {
        println!("Cycle detected: {}", cycle.join(" -> "));
    }
}
```

**Resolution:**
1. Identify the cycle in the error message
2. Review the class hierarchy design
3. Remove one inheritance link to break the cycle
4. Consider composition over inheritance

### 2. Property Type Mismatch

**Symptom:** Generated code has type errors, won't compile.

**Cause:** Using properties on instances of the wrong type.

**Resolution:**
1. Check the property's `rdfs:domain` declaration
2. Ensure subject instance has correct `rdf:type`
3. Use `rdf:type` with proper class hierarchy

### 3. Missing Required Properties

**Symptom:** Runtime panics due to missing fields in generated structs.

**Cause:** Instances lack properties required by SHACL shapes.

**Prevention:**
```turtle
# Define clear cardinality constraints
sh:property [
    sh:path mcp:serverId ;
    sh:minCount 1 ;
    sh:maxCount 1 ;
    sh:message "Server must have exactly one ID"
] .
```

**Resolution:**
1. Review SHACL shapes for the class
2. Add missing properties to instances
3. Or adjust cardinality constraints if requirements changed

### 4. Orphaned Nodes

**Symptom:** Entities appear in ontology but aren't connected to the domain model.

**Cause:** Nodes with no incoming edges and no `rdf:type`.

**Example:**

```turtle
# Orphaned - no type, no incoming edges
ex:OrphanedNode mcp:someProperty "value" .
```

**Resolution:**

```turtle
# Connect to type system
ex:OrphanedNode a mcp:Tool ;
    mcp:toolName "my_tool" ;
    mcp:someProperty "value" .
```

### 5. Namespace Collisions

**Symptom:** URI expansion produces incorrect results.

**Cause:** Same prefix mapped to different URIs in different files.

**Detection:**

```rust
let mut ns = NamespaceManager::new();
ns.register("mcp", "http://ggen-mcp.dev/mcp#")?;

// Later, in merged ontology:
match ns.register("mcp", "http://different-domain.org/mcp#") {
    Err(ValidationError::NamespaceCollision { .. }) => {
        // Handle collision
    }
    _ => {}
}
```

**Resolution:**
1. Standardize namespace URIs across all ontology files
2. Use unique prefixes for different namespaces
3. Document canonical namespace mappings

### 6. Hash Mismatch (Tampering)

**Symptom:** Hash verification fails unexpectedly.

**Cause:** Ontology file modified outside version control or corrupted.

**Detection:**

```rust
let verifier = HashVerifier::new(store);
match verifier.verify_hash(expected_hash) {
    Err(ValidationError::HashMismatch { expected, actual }) => {
        eprintln!("SECURITY: Ontology may have been tampered with!");
        eprintln!("Expected: {}", expected);
        eprintln!("Got:      {}", actual);
    }
    _ => {}
}
```

**Resolution:**
1. Review git history for unexpected changes
2. Restore from known-good backup
3. Recompute and store new hash if changes were intentional
4. Investigate potential security breach

## Integration with SHACL Shapes

SHACL (Shapes Constraint Language) provides a W3C standard for validating RDF graphs. Our consistency checker integrates with SHACL shapes defined in the ontology.

### SHACL Shape Structure

```turtle
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix mcp: <http://ggen-mcp.dev/ontology/mcp#> .

mcp:ToolShape a sh:NodeShape ;
    sh:targetClass mcp:Tool ;  # Apply to all instances of mcp:Tool
    sh:property [
        sh:path mcp:toolName ;
        sh:datatype xsd:string ;
        sh:minCount 1 ;
        sh:maxCount 1 ;
        sh:pattern "^[a-z][a-z0-9_]*$" ;
        sh:message "Tool name must be lowercase snake_case"
    ] .
```

### How Consistency Checker Uses SHACL

1. **Cardinality Validation**: `sh:minCount` and `sh:maxCount` are enforced
2. **Pattern Matching**: `sh:pattern` regex patterns are checked (future enhancement)
3. **Datatype Validation**: `sh:datatype` ensures correct XSD types
4. **Custom Messages**: `sh:message` provides user-friendly error messages

### Defining Custom SHACL Shapes

For custom domain classes:

```turtle
ex:CustomerShape a sh:NodeShape ;
    sh:targetClass ex:Customer ;
    sh:property [
        sh:path ex:customerId ;
        sh:minCount 1 ;
        sh:maxCount 1 ;
        sh:datatype xsd:string ;
        sh:minLength 1 ;
        sh:message "Customer must have a non-empty ID"
    ] ;
    sh:property [
        sh:path ex:email ;
        sh:maxCount 1 ;
        sh:datatype xsd:string ;
        sh:pattern "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$" ;
        sh:message "Email must be valid format"
    ] .
```

### SHACL Best Practices

1. **One Shape Per Class**: Keep shapes focused and maintainable
2. **Clear Messages**: Always include `sh:message` for better error reporting
3. **Reusable Constraints**: Extract common patterns into reusable property shapes
4. **Document Rationale**: Use `sh:description` to explain why constraints exist

## Performance Considerations

### Complexity Analysis

| Operation | Time Complexity | Space Complexity | Notes |
|-----------|----------------|------------------|-------|
| Cycle Detection | O(V + E) | O(V) | V = classes, E = subclass relations |
| Domain/Range Check | O(P × I) | O(1) | P = properties, I = instances |
| Cardinality Check | O(S × I × P) | O(1) | S = shapes, I = instances, P = properties per shape |
| Hash Computation | O(T log T) | O(T) | T = triples (sort required) |

### Optimization Strategies

#### 1. Incremental Validation

For large ontologies, validate only changed portions:

```rust
// Full validation (expensive)
let full_report = checker.check_all();

// Incremental validation (faster)
let mut report = ConsistencyReport::new();
checker.check_class_hierarchy(&mut report)?;
// Only run other checks if hierarchy is valid
if report.valid {
    checker.check_cardinality(&mut report)?;
}
```

#### 2. Parallel Validation

Independent checks can run in parallel:

```rust
use rayon::prelude::*;

let checks = vec![
    || checker.check_class_hierarchy(&mut report.clone()),
    || checker.check_property_domains(&mut report.clone()),
    || checker.check_property_ranges(&mut report.clone()),
];

checks.par_iter().for_each(|check| {
    check();
});
```

#### 3. Caching

Cache validation results for unchanged ontologies:

```rust
let hash = verifier.compute_hash()?;

if let Some(cached_report) = cache.get(&hash) {
    return cached_report;
}

let report = checker.check_all();
cache.insert(hash, report.clone());
```

#### 4. Query Optimization

Use indexed SPARQL queries:

```sparql
# Indexed by rdf:type (fast)
SELECT ?instance WHERE {
    ?instance rdf:type mcp:Tool .
}

# Full table scan (slow)
SELECT ?instance WHERE {
    ?instance ?p ?o .
    FILTER(?p = rdf:type && ?o = mcp:Tool)
}
```

### Scaling to Large Ontologies

For ontologies with >100k triples:

1. **Streaming Validation**: Process triples in batches
2. **Database Backend**: Use persistent storage (RocksDB) instead of in-memory
3. **Selective Validation**: Only validate modified subgraphs
4. **Distributed Validation**: Partition ontology across multiple validators

## Troubleshooting Guide

### Issue: Validation Takes Too Long

**Symptoms:**
- Validation takes >10 seconds for medium-sized ontologies
- High CPU usage during validation

**Diagnosis:**

```rust
use std::time::Instant;

let start = Instant::now();
let report = checker.check_all();
let duration = start.elapsed();

println!("Validation took: {:?}", duration);
println!("Triples: {}", report.stats.total_triples);
println!("Classes: {}", report.stats.total_classes);
```

**Solutions:**
1. Enable incremental validation (see Performance section)
2. Run only necessary checks for your use case
3. Optimize SPARQL queries (add indexes)
4. Consider caching results

### Issue: False Positive Errors

**Symptoms:**
- Validation reports errors in known-good ontologies
- Errors don't make sense in domain context

**Diagnosis:**

```rust
// Enable detailed logging
for error in &report.errors {
    println!("Error: {:?}", error);
    // Inspect the actual triples involved
}
```

**Solutions:**
1. Review SHACL shapes - may be overly restrictive
2. Check for namespace issues (wrong prefixes)
3. Verify ontology loaded correctly (no parse errors)
4. File a bug report with minimal reproducing example

### Issue: Missing Errors Not Detected

**Symptoms:**
- Known issues pass validation
- Generated code has bugs despite passing validation

**Diagnosis:**

1. Check which validations are enabled
2. Review SHACL shapes for coverage
3. Add custom validation rules

**Solutions:**

```rust
// Add custom validation
impl ConsistencyChecker {
    pub fn check_custom_rule(&self, report: &mut ConsistencyReport) -> Result<()> {
        // Your custom SPARQL query
        let query = r#"
            SELECT ?instance WHERE {
                ?instance a ex:MyClass .
                # Your condition
            }
        "#;

        // Check and add errors to report
        // ...
    }
}
```

### Issue: Hash Verification Keeps Failing

**Symptoms:**
- Hash changes on every validation
- Same ontology produces different hashes

**Cause:** Non-deterministic triple ordering or blank node identifiers

**Solutions:**

1. **Check for blank nodes:**
```sparql
SELECT ?subject WHERE {
    ?subject ?p ?o .
    FILTER(isBlank(?subject))
}
```

2. **Use named nodes instead:**
```turtle
# Bad - blank nodes not deterministic
ex:MyClass ddd:hasInvariant [
    ddd:check "some_check"
] .

# Good - named nodes are stable
ex:MyClass ddd:hasInvariant ex:MyClassInvariant1 .
ex:MyClassInvariant1 ddd:check "some_check" .
```

## API Reference

### ConsistencyChecker

```rust
impl ConsistencyChecker {
    pub fn new(store: Store) -> Self;
    pub fn check_all(&self) -> ConsistencyReport;
    pub fn check_class_hierarchy(&self, report: &mut ConsistencyReport) -> Result<()>;
    pub fn check_property_domains(&self, report: &mut ConsistencyReport) -> Result<()>;
    pub fn check_property_ranges(&self, report: &mut ConsistencyReport) -> Result<()>;
    pub fn check_cardinality(&self, report: &mut ConsistencyReport) -> Result<()>;
    pub fn check_required_properties(&self, report: &mut ConsistencyReport) -> Result<()>;
}
```

### SchemaValidator

```rust
impl SchemaValidator {
    pub fn new(store: Store) -> Self;
    pub fn validate_all(&self) -> ConsistencyReport;
    pub fn check_required_namespaces(&self, report: &mut ConsistencyReport) -> Result<()>;
    pub fn check_ddd_aggregate_structure(&self, report: &mut ConsistencyReport) -> Result<()>;
    pub fn check_property_types(&self, report: &mut ConsistencyReport) -> Result<()>;
    pub fn check_invariants(&self, report: &mut ConsistencyReport) -> Result<()>;
    pub fn check_orphaned_nodes(&self, report: &mut ConsistencyReport) -> Result<()>;
}
```

### NamespaceManager

```rust
impl NamespaceManager {
    pub fn new() -> Self;
    pub fn register(&mut self, prefix: &str, uri: &str) -> ValidationResult<()>;
    pub fn set_default(&mut self, uri: &str);
    pub fn get(&self, prefix: &str) -> Option<&String>;
    pub fn expand(&self, qname: &str) -> ValidationResult<String>;
    pub fn compact(&self, uri: &str) -> String;
    pub fn all(&self) -> &HashMap<String, String>;
}
```

### OntologyMerger

```rust
impl OntologyMerger {
    pub fn new() -> Self;
    pub fn merge(&self, target: &Store, source: &Store) -> Result<MergeResult>;
}
```

### HashVerifier

```rust
impl HashVerifier {
    pub fn new(store: Store) -> Self;
    pub fn compute_hash(&self) -> Result<String>;
    pub fn verify_hash(&self, expected_hash: &str) -> ValidationResult<()>;
    pub fn get_ontology_hash(&self) -> Result<Option<String>>;
    pub fn store_hash(&self, hash: &str) -> Result<()>;
    pub fn verify_and_update(&self) -> Result<bool>;
}
```

### ValidationError Enum

```rust
pub enum ValidationError {
    CyclicHierarchy { cycle: Vec<String> },
    InvalidDomainRange { property: String, subject: String, object: String, message: String },
    CardinalityViolation { node: String, property: String, expected: String, actual: usize },
    Contradiction { statement1: String, statement2: String, reason: String },
    MissingProperty { node: String, property: String },
    MissingNamespace { prefix: String, expected_uri: String },
    InvalidDddStructure { aggregate: String, reason: String },
    UntypedProperty { property: String },
    InvalidInvariant { node: String, reason: String },
    OrphanedNode { node: String },
    NamespaceCollision { prefix: String, uri1: String, uri2: String },
    MergeConflict { resource: String, reason: String },
    HashMismatch { expected: String, actual: String },
    Custom { message: String },
}
```

### ConsistencyReport Struct

```rust
pub struct ConsistencyReport {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub stats: ConsistencyStats,
}

pub struct ConsistencyStats {
    pub total_triples: usize,
    pub total_classes: usize,
    pub total_properties: usize,
    pub total_individuals: usize,
    pub max_hierarchy_depth: usize,
}
```

---

## See Also

- [SHACL Specification](https://www.w3.org/TR/shacl/)
- [RDF Schema](https://www.w3.org/TR/rdf-schema/)
- [OWL 2 Web Ontology Language](https://www.w3.org/TR/owl2-overview/)
- [Domain-Driven Design](https://martinfowler.com/tags/domain%20driven%20design.html)
- [Toyota Production System Documentation](docs/TPS_RESEARCH.md)

## Contributing

Found a bug or have a suggestion? Please open an issue or submit a pull request!
