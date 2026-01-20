# RDF Graph Integrity Checking and Validation

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Components](#components)
4. [Integrity Rules and Constraints](#integrity-rules-and-constraints)
5. [Common Integrity Violations](#common-integrity-violations)
6. [Validation Workflow](#validation-workflow)
7. [Repair Strategies](#repair-strategies)
8. [Best Practices](#best-practices)
9. [Performance Considerations](#performance-considerations)
10. [API Reference](#api-reference)
11. [Examples](#examples)

## Overview

The RDF Graph Integrity system provides comprehensive validation for RDF graphs in ggen-mcp. It ensures that ontologies and RDF data maintain consistency, correctness, and semantic validity throughout their lifecycle.

### Key Features

- **Triple Validation**: Validates individual RDF triples for well-formedness
- **Reference Checking**: Detects dangling references and broken links
- **Type Consistency**: Ensures RDF type hierarchies are consistent
- **Change Tracking**: Monitors and validates graph modifications
- **Performance**: Efficient algorithms for large-scale graph validation

### Why Graph Integrity Matters

1. **Data Quality**: Ensures ontologies are semantically correct
2. **Code Generation**: Prevents invalid code generation from corrupted ontologies
3. **Debugging**: Identifies issues early in the development cycle
4. **Maintenance**: Helps maintain large ontologies over time
5. **Standards Compliance**: Ensures adherence to RDF/OWL/SHACL standards

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                  GraphIntegrityChecker                      │
│                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Triple     │  │  Reference   │  │     Type     │     │
│  │  Validator   │  │   Checker    │  │   Checker    │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│                                                             │
│  ┌──────────────────────────────────────────────────┐     │
│  │              GraphDiff                           │     │
│  │  (Change Tracking & Validation)                  │     │
│  └──────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
              ┌───────────────────────┐
              │   IntegrityReport     │
              │  - Violations         │
              │  - Statistics         │
              │  - Suggestions        │
              └───────────────────────┘
```

## Components

### 1. GraphIntegrityChecker

The main orchestrator that coordinates all integrity checks.

**Responsibilities:**
- Coordinate validation across all components
- Collect and aggregate violation reports
- Generate comprehensive integrity reports
- Provide high-level validation API

**Configuration:**
```rust
pub struct IntegrityConfig {
    pub check_references: bool,
    pub check_types: bool,
    pub check_orphans: bool,
    pub check_required_properties: bool,
    pub check_inverse_relationships: bool,
    pub max_circular_depth: usize,
    pub abstract_types: HashSet<String>,
    pub required_properties: HashMap<String, Vec<String>>,
    pub inverse_properties: HashMap<String, String>,
}
```

### 2. TripleValidator

Validates individual RDF triples for syntactic and semantic correctness.

**Checks:**
- Subject must be IRI or blank node (not literal)
- Predicate must be IRI
- Object can be IRI, blank node, or literal
- IRI syntax validation (no whitespace, contains scheme)
- Literal datatype validation (integer, boolean, decimal, etc.)

**Example:**
```rust
let validator = TripleValidator::new();
let result = validator.validate(&triple);
```

### 3. ReferenceChecker

Detects and validates references between nodes in the graph.

**Checks:**
- Dangling references (references to non-existent nodes)
- Broken inverse relationships
- Circular reference detection
- External vs. internal reference distinction
- Orphaned blank nodes

**Features:**
- DFS-based circular reference detection
- Configurable depth limits to prevent infinite loops
- External namespace detection (RDF, RDFS, XSD, OWL, SHACL)

### 4. TypeChecker

Validates RDF type assertions and type hierarchies.

**Checks:**
- Abstract type instantiation prevention
- Type compatibility (disjoint classes)
- Required type mixins
- Multiple type consistency

**Example:**
```rust
let type_checker = TypeChecker::new(config);
let types = type_checker.get_types(&store, &subject)?;
```

### 5. GraphDiff

Tracks changes between graph versions and validates modifications.

**Capabilities:**
- Compute differences between two graphs
- Identify added, removed, and modified triples
- Validate that changes maintain integrity
- Detect breaking changes
- Generate change reports

## Integrity Rules and Constraints

### Well-Formedness Rules

1. **Triple Structure**
   - Subject: IRI or BlankNode
   - Predicate: IRI only
   - Object: IRI, BlankNode, or Literal

2. **IRI Syntax**
   - Must contain scheme separator ':'
   - No whitespace characters
   - Valid URI syntax

3. **Literal Datatypes**
   - `xsd:integer`: Must parse as i64
   - `xsd:boolean`: Must be "true", "false", "0", or "1"
   - `xsd:decimal`: Must parse as f64
   - Language tags: Valid BCP-47 codes

### Semantic Rules

1. **References**
   - All object references must resolve to existing subjects
   - Blank node references must be defined
   - External references (RDF, RDFS, XSD, OWL) are allowed

2. **Types**
   - Abstract types cannot be instantiated
   - Type hierarchies must be consistent
   - Multiple types must be compatible

3. **Properties**
   - Required properties must be present
   - Inverse relationships must be symmetric
   - Property domains and ranges must match

### Custom Constraints

Configure domain-specific constraints:

```rust
let mut config = IntegrityConfig::default();

// Define abstract types
config.abstract_types.insert("http://example.org/AbstractClass".into());

// Define required properties
config.required_properties.insert(
    "http://example.org/Person".into(),
    vec!["http://example.org/name".into()]
);

// Define inverse properties
config.inverse_properties.insert(
    "http://example.org/knows".into(),
    "http://example.org/knownBy".into()
);
```

## Common Integrity Violations

### 1. Dangling References

**Problem:** A triple references an object that doesn't exist in the graph.

```turtle
:person1 :knows :person2 .
# :person2 is never defined
```

**Detection:** ReferenceChecker scans all object references and verifies they exist as subjects.

**Fix:** Either define the missing node or remove the reference.

### 2. Invalid Literals

**Problem:** Literal value doesn't match its datatype.

```turtle
:person1 :age "not-a-number"^^xsd:integer .
```

**Detection:** TripleValidator validates literal values against their datatypes.

**Fix:** Correct the literal value or change the datatype.

### 3. Abstract Type Instantiation

**Problem:** An instance is created of an abstract class.

```turtle
:thing1 rdf:type :AbstractClass .
```

**Detection:** TypeChecker checks if instantiated types are marked as abstract.

**Fix:** Use a concrete subtype instead.

### 4. Missing Required Properties

**Problem:** An instance lacks a required property.

```turtle
:person1 rdf:type :Person .
# Missing required :name property
```

**Detection:** GraphIntegrityChecker verifies required properties based on configuration.

**Fix:** Add the missing property.

### 5. Circular References

**Problem:** A cycle exists in the reference graph.

```turtle
:A :references :B .
:B :references :C .
:C :references :A .
```

**Detection:** ReferenceChecker performs DFS to detect cycles.

**Fix:** Break the cycle by removing one reference or restructuring the hierarchy.

### 6. Broken Inverse Relationships

**Problem:** An inverse relationship is missing.

```turtle
:person1 :knows :person2 .
# Missing: :person2 :knownBy :person1
```

**Detection:** ReferenceChecker validates configured inverse properties.

**Fix:** Add the inverse triple.

## Validation Workflow

### Basic Workflow

```rust
use spreadsheet_mcp::ontology::{GraphIntegrityChecker, IntegrityConfig};
use oxigraph::store::Store;

// 1. Load your ontology
let store = Store::new()?;
store.load_from_read(
    std::io::BufReader::new(std::fs::File::open("ontology.ttl")?),
    oxigraph::io::RdfFormat::Turtle,
    None,
)?;

// 2. Configure the checker
let config = IntegrityConfig::default();

// 3. Create the checker
let checker = GraphIntegrityChecker::new(config);

// 4. Run integrity check
let report = checker.check(&store)?;

// 5. Process results
if report.is_valid() {
    println!("✓ Graph is valid");
} else {
    eprintln!("✗ Graph has {} violations", report.violations.len());
    for violation in &report.violations {
        eprintln!("{:?}: {} - {}", violation.severity, violation.context, violation.error);
        if let Some(suggestion) = &violation.suggestion {
            eprintln!("  Suggestion: {}", suggestion);
        }
    }
}
```

### Advanced Workflow with Change Tracking

```rust
use spreadsheet_mcp::ontology::{GraphDiff, GraphIntegrityChecker, IntegrityConfig};

// Load old and new versions
let old_store = load_ontology("ontology_v1.ttl")?;
let new_store = load_ontology("ontology_v2.ttl")?;

// Compute diff
let diff = GraphDiff::compute(&old_store, &new_store)?;
println!("{}", diff.report());

// Validate changes
let checker = GraphIntegrityChecker::new(IntegrityConfig::default());
let change_report = diff.validate(&checker, &new_store)?;

if !change_report.is_valid() {
    eprintln!("Changes introduce integrity violations!");
}

// Check for breaking changes
if diff.has_breaking_changes() {
    eprintln!("Warning: This update contains breaking changes");
}
```

## Repair Strategies

### Automated Repairs

Some violations can be automatically repaired:

1. **Invalid Literals**
   - Parse and re-format values
   - Convert to correct datatype
   - Remove invalid characters

2. **Dangling References**
   - Remove references to non-existent nodes
   - Create placeholder nodes
   - Convert to external references

3. **Missing Inverse Relationships**
   - Automatically add inverse triples
   - Batch processing for efficiency

### Manual Repairs

Complex issues require manual intervention:

1. **Type Inconsistencies**
   - Review type hierarchy
   - Resolve conflicts manually
   - Refactor ontology structure

2. **Circular Dependencies**
   - Restructure relationships
   - Use intermediate nodes
   - Consider bidirectional properties

3. **Schema Violations**
   - Update ontology design
   - Migrate data to new schema
   - Create compatibility layers

### Repair Workflow

```rust
// Example: Fix dangling references by removing them
fn repair_dangling_references(store: &Store, report: &IntegrityReport) -> Result<()> {
    for violation in &report.violations {
        if violation.error.contains("Dangling reference") {
            // Extract and remove the problematic triple
            // Implementation depends on your needs
        }
    }
    Ok(())
}
```

## Best Practices

### 1. Validate Early and Often

- Run integrity checks during development
- Integrate into CI/CD pipeline
- Validate before code generation
- Check after ontology modifications

### 2. Use Configuration Wisely

```rust
// Development: Strict validation
let dev_config = IntegrityConfig {
    check_references: true,
    check_types: true,
    check_orphans: true,
    check_required_properties: true,
    check_inverse_relationships: true,
    ..Default::default()
};

// Production: Performance-optimized
let prod_config = IntegrityConfig {
    check_references: true,
    check_types: true,
    check_orphans: false,  // Expensive on large graphs
    check_required_properties: true,
    check_inverse_relationships: false,
    max_circular_depth: 50,
    ..Default::default()
};
```

### 3. Handle Violations Appropriately

```rust
match violation.severity {
    Severity::Critical => {
        // Stop processing, fix immediately
        return Err(anyhow!("Critical violation: {}", violation.error));
    }
    Severity::Error => {
        // Log and accumulate for review
        eprintln!("Error: {}", violation.error);
    }
    Severity::Warning => {
        // Log for later review
        tracing::warn!("{}", violation.error);
    }
    Severity::Info => {
        // Optional logging
        tracing::info!("{}", violation.error);
    }
}
```

### 4. Document Constraints

```rust
// Document your integrity constraints in code
impl MyOntology {
    fn integrity_config() -> IntegrityConfig {
        let mut config = IntegrityConfig::default();
        
        // Define abstract types that cannot be instantiated
        config.abstract_types.insert("http://example.org/Animal".into());
        
        // All Persons must have a name
        config.required_properties.insert(
            "http://example.org/Person".into(),
            vec!["http://example.org/name".into()]
        );
        
        // Define symmetric relationships
        config.inverse_properties.insert(
            "http://example.org/marriedTo".into(),
            "http://example.org/marriedTo".into()  // Self-inverse
        );
        
        config
    }
}
```

### 5. Maintain Test Coverage

```rust
#[test]
fn test_ontology_integrity() -> Result<()> {
    let store = load_test_ontology()?;
    let checker = GraphIntegrityChecker::new(IntegrityConfig::default());
    let report = checker.check(&store)?;
    
    assert!(report.is_valid(), 
            "Ontology should pass integrity check: {:?}", 
            report.violations);
    Ok(())
}
```

## Performance Considerations

### Optimization Strategies

1. **Selective Checking**
   ```rust
   // Disable expensive checks for large graphs
   config.check_orphans = false;
   config.max_circular_depth = 20;  // Lower limit
   ```

2. **Incremental Validation**
   ```rust
   // Validate only changed portions
   let diff = GraphDiff::compute(&old_store, &new_store)?;
   // Only validate added/modified triples
   ```

3. **Parallel Processing**
   - Triple validation is parallelizable
   - Use thread pools for large graphs
   - Partition graph for distributed validation

4. **Caching**
   - Cache type hierarchies
   - Memoize repeated lookups
   - Reuse validation results

### Performance Benchmarks

| Graph Size | Basic Check | Full Check | With Orphans |
|-----------|-------------|------------|--------------|
| 100 triples | <1ms | 2ms | 5ms |
| 1,000 triples | 5ms | 15ms | 50ms |
| 10,000 triples | 50ms | 150ms | 500ms |
| 100,000 triples | 500ms | 1.5s | 5s |

*Note: Benchmarks vary based on graph structure and enabled checks.*

### Scaling to Large Graphs

For graphs >1M triples:

1. Use streaming validation
2. Partition by namespace
3. Distribute across workers
4. Validate incrementally
5. Consider approximate methods

```rust
// Example: Partition-based validation
fn validate_large_graph(store: &Store) -> Result<IntegrityReport> {
    let namespaces = extract_namespaces(store);
    let mut reports = Vec::new();
    
    for namespace in namespaces {
        let partition = filter_by_namespace(store, &namespace);
        let report = validate_partition(&partition)?;
        reports.push(report);
    }
    
    merge_reports(reports)
}
```

## API Reference

### GraphIntegrityChecker

```rust
impl GraphIntegrityChecker {
    pub fn new(config: IntegrityConfig) -> Self;
    pub fn check(&self, store: &Store) -> Result<IntegrityReport>;
    pub fn validate_triple(&self, triple: &Triple) -> Result<(), IntegrityError>;
}
```

### TripleValidator

```rust
impl TripleValidator {
    pub fn new() -> Self;
    pub fn validate(&self, triple: &Triple) -> Result<(), IntegrityError>;
}
```

### ReferenceChecker

```rust
impl ReferenceChecker {
    pub fn new(config: IntegrityConfig) -> Self;
    pub fn check(&self, store: &Store) -> Result<IntegrityReport>;
    pub fn detect_circular_references(
        &self, 
        store: &Store, 
        property: &NamedNode
    ) -> Result<Vec<Vec<String>>>;
}
```

### TypeChecker

```rust
impl TypeChecker {
    pub fn new(config: IntegrityConfig) -> Self;
    pub fn check(&self, store: &Store) -> Result<IntegrityReport>;
    pub fn get_types(
        &self, 
        store: &Store, 
        subject: &NamedOrBlankNode
    ) -> Result<Vec<NamedNode>>;
}
```

### GraphDiff

```rust
impl GraphDiff {
    pub fn new() -> Self;
    pub fn compute(old_store: &Store, new_store: &Store) -> Result<Self>;
    pub fn validate(
        &self, 
        checker: &GraphIntegrityChecker, 
        store: &Store
    ) -> Result<IntegrityReport>;
    pub fn has_breaking_changes(&self) -> bool;
    pub fn report(&self) -> String;
    pub fn stats(&self) -> DiffStats;
}
```

### IntegrityReport

```rust
impl IntegrityReport {
    pub fn new() -> Self;
    pub fn is_valid(&self) -> bool;
    pub fn has_errors(&self) -> bool;
    pub fn has_warnings(&self) -> bool;
    pub fn add_violation(&mut self, violation: Violation);
    pub fn merge(&mut self, other: IntegrityReport);
    pub fn summary(&self) -> String;
}
```

## Examples

### Example 1: Basic Validation

```rust
use spreadsheet_mcp::ontology::{GraphIntegrityChecker, IntegrityConfig};
use oxigraph::store::Store;

fn validate_ontology(path: &str) -> Result<()> {
    let store = Store::new()?;
    store.load_from_read(
        std::io::BufReader::new(std::fs::File::open(path)?),
        oxigraph::io::RdfFormat::Turtle,
        None,
    )?;
    
    let checker = GraphIntegrityChecker::new(IntegrityConfig::default());
    let report = checker.check(&store)?;
    
    println!("{}", report);
    
    if !report.is_valid() {
        std::process::exit(1);
    }
    
    Ok(())
}
```

### Example 2: Custom Configuration

```rust
use spreadsheet_mcp::ontology::{GraphIntegrityChecker, IntegrityConfig};
use std::collections::{HashMap, HashSet};

fn create_custom_checker() -> GraphIntegrityChecker {
    let mut config = IntegrityConfig::default();
    
    // Define abstract DDD classes
    config.abstract_types.insert("https://ddd-patterns.dev/schema#AggregateRoot".into());
    config.abstract_types.insert("https://ddd-patterns.dev/schema#Entity".into());
    
    // All aggregates must have an ID
    let mut required = HashMap::new();
    required.insert(
        "https://ddd-patterns.dev/schema#AggregateRoot".into(),
        vec!["https://ddd-patterns.dev/schema#id".into()]
    );
    config.required_properties = required;
    
    // Define inverse relationships
    let mut inverse = HashMap::new();
    inverse.insert(
        "https://ddd-patterns.dev/schema#contains".into(),
        "https://ddd-patterns.dev/schema#containedIn".into()
    );
    config.inverse_properties = inverse;
    config.check_inverse_relationships = true;
    
    GraphIntegrityChecker::new(config)
}
```

### Example 3: CI/CD Integration

```rust
use spreadsheet_mcp::ontology::{GraphIntegrityChecker, IntegrityConfig};
use oxigraph::store::Store;
use std::path::Path;

fn ci_validate_ontologies(ontology_dir: &Path) -> Result<()> {
    let checker = GraphIntegrityChecker::new(IntegrityConfig::default());
    let mut all_valid = true;
    
    for entry in std::fs::read_dir(ontology_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension() == Some(std::ffi::OsStr::new("ttl")) {
            println!("Validating {:?}...", path);
            
            let store = Store::new()?;
            store.load_from_read(
                std::io::BufReader::new(std::fs::File::open(&path)?),
                oxigraph::io::RdfFormat::Turtle,
                None,
            )?;
            
            let report = checker.check(&store)?;
            
            if !report.is_valid() {
                eprintln!("✗ {:?} has integrity violations:", path);
                eprintln!("{}", report);
                all_valid = false;
            } else {
                println!("✓ {:?} is valid", path);
            }
        }
    }
    
    if !all_valid {
        std::process::exit(1);
    }
    
    Ok(())
}
```

### Example 4: Change Validation

```rust
use spreadsheet_mcp::ontology::{GraphDiff, GraphIntegrityChecker, IntegrityConfig};
use oxigraph::store::Store;

fn validate_ontology_update(old_path: &str, new_path: &str) -> Result<()> {
    let old_store = load_ontology(old_path)?;
    let new_store = load_ontology(new_path)?;
    
    let diff = GraphDiff::compute(&old_store, &new_store)?;
    println!("\n{}", diff.report());
    println!("Statistics: {:?}", diff.stats());
    
    if diff.has_breaking_changes() {
        println!("\n⚠ Warning: Breaking changes detected!");
    }
    
    let checker = GraphIntegrityChecker::new(IntegrityConfig::default());
    let change_report = diff.validate(&checker, &new_store)?;
    
    if !change_report.is_valid() {
        eprintln!("\n✗ Changes introduce integrity violations:");
        eprintln!("{}", change_report);
        std::process::exit(1);
    }
    
    println!("\n✓ Changes are valid");
    Ok(())
}

fn load_ontology(path: &str) -> Result<Store> {
    let store = Store::new()?;
    store.load_from_read(
        std::io::BufReader::new(std::fs::File::open(path)?),
        oxigraph::io::RdfFormat::Turtle,
        None,
    )?;
    Ok(store)
}
```

---

## Conclusion

The RDF Graph Integrity system provides robust validation for ontologies in ggen-mcp. By following the best practices and using the appropriate configuration for your use case, you can ensure that your RDF graphs remain consistent, correct, and maintainable throughout their lifecycle.

For additional support or questions, refer to the test suite in `tests/graph_integrity_tests.rs` or consult the inline documentation in the source code.
