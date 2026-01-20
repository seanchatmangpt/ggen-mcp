# Ontology Module

Comprehensive RDF ontology validation and consistency checking for the ggen-mcp system.

## Module Structure

```
src/ontology/
├── mod.rs              - Module exports and documentation
├── consistency.rs      - Consistency checking and validation (NEW)
├── graph_integrity.rs  - Low-level graph integrity checking
└── shacl.rs           - SHACL validation support
```

## Components

### 1. consistency.rs (NEW)

High-level ontology validation with DDD-aware checking:

- **ConsistencyChecker**: Validates RDF graph consistency
  - Class hierarchy validation (no cycles)
  - Property domain/range checking
  - Cardinality constraints
  - Contradiction detection
  - Required property presence

- **SchemaValidator**: Validates against expected schema patterns
  - Required namespaces (ddd, ggen, sh, rdfs, xsd)
  - DDD aggregate structure
  - Property type declarations
  - Invariant definitions
  - Orphaned node detection

- **NamespaceManager**: Safe namespace handling
  - Register and validate prefixes
  - Prevent prefix collisions
  - Resolve QNames safely
  - URI expansion with validation
  - Default namespace handling

- **OntologyMerger**: Safe ontology merging
  - Conflict detection before merge
  - Preserve provenance
  - Handle duplicate definitions
  - Validate after merge
  - Rollback on failure

- **HashVerifier**: Verify ontology integrity
  - Compute consistent hashes (SHA-256)
  - Detect tampering or corruption
  - Verify hash matches expected value
  - Track version changes

### 2. graph_integrity.rs (Existing)

Low-level triple and reference validation.

### 3. shacl.rs (Existing)

W3C SHACL (Shapes Constraint Language) validation support.

## Usage

### Basic Validation

```rust
use spreadsheet_mcp::ontology::{ConsistencyChecker, SchemaValidator};
use oxigraph::store::Store;

let store = Store::new()?;
store.load_from_file("ontology/mcp-domain.ttl", GraphFormat::Turtle)?;

// Check RDF graph consistency
let checker = ConsistencyChecker::new(store.clone());
let report = checker.check_all();

if !report.valid {
    for error in &report.errors {
        eprintln!("Error: {}", error);
    }
}

// Validate DDD schema patterns
let validator = SchemaValidator::new(store);
let schema_report = validator.validate_all();
```

### Namespace Management

```rust
use spreadsheet_mcp::ontology::NamespaceManager;

let mut ns = NamespaceManager::new();
ns.register("mcp", "http://ggen-mcp.dev/ontology/mcp#")?;

let full_uri = ns.expand("mcp:Tool")?;
let qname = ns.compact("http://ggen-mcp.dev/ontology/mcp#Resource");
```

### Hash Verification

```rust
use spreadsheet_mcp::ontology::HashVerifier;

let verifier = HashVerifier::new(store);
let hash = verifier.compute_hash()?;

// Verify against expected hash
verifier.verify_hash("expected_sha256_hash")?;

// Store hash for future verification
verifier.store_hash(&hash)?;
```

### Merging Ontologies

```rust
use spreadsheet_mcp::ontology::OntologyMerger;

let merger = OntologyMerger::new();
let result = merger.merge(&target_store, &source_store)?;

if !result.success {
    for conflict in &result.conflicts {
        eprintln!("Conflict: {}", conflict);
    }
}
```

## Testing

Run the test suite:

```bash
cargo test --test ontology_consistency_tests
```

Run the validation example:

```bash
cargo run --example ontology_validation
```

## Documentation

See [ONTOLOGY_CONSISTENCY.md](../../docs/ONTOLOGY_CONSISTENCY.md) for:
- Detailed validation rules
- Common consistency errors
- Troubleshooting guide
- Performance considerations
- API reference

## Integration with Code Generation

The consistency checker integrates with the ggen code generation pipeline:

1. **Pre-generation validation**: Ontology is validated before code generation
2. **Error reporting**: Detailed error messages help fix ontology issues
3. **Hash tracking**: Detects when ontology changes require regeneration
4. **SHACL integration**: SHACL shapes are enforced during validation

## Common Patterns

### Validate Before Code Generation

```rust
fn generate_code(ontology_path: &str) -> Result<()> {
    let store = Store::new()?;
    store.load_from_file(ontology_path, GraphFormat::Turtle)?;

    // Validate first
    let checker = ConsistencyChecker::new(store.clone());
    let report = checker.check_all();

    if !report.valid {
        return Err(anyhow::anyhow!(
            "Ontology validation failed: {} errors",
            report.errors.len()
        ));
    }

    // Proceed with code generation
    // ...
}
```

### Incremental Validation

```rust
// For large ontologies, run only specific checks
let mut report = ConsistencyReport::new();
checker.check_class_hierarchy(&mut report)?;

if report.valid {
    // Only run expensive checks if basic structure is valid
    checker.check_cardinality(&mut report)?;
}
```

### Custom Validation Rules

```rust
impl ConsistencyChecker {
    pub fn check_custom_invariant(&self, report: &mut ConsistencyReport) -> Result<()> {
        let query = r#"
            SELECT ?aggregate WHERE {
                ?aggregate a ddd:AggregateRoot .
                # Your custom SPARQL constraint
            }
        "#;

        let results = self.store.query(query)?;
        // Add errors to report
        // ...
    }
}
```

## Performance

For ontologies with >100k triples:

- Use incremental validation (validate only changed subgraphs)
- Enable persistent storage (RocksDB backend)
- Run checks in parallel where possible
- Cache validation results keyed by hash

See [Performance Considerations](../../docs/ONTOLOGY_CONSISTENCY.md#performance-considerations) for details.

## Contributing

When adding new validation rules:

1. Add the check method to the appropriate validator
2. Define a new ValidationError variant if needed
3. Add tests in `tests/ontology_consistency_tests.rs`
4. Document the rule in `docs/ONTOLOGY_CONSISTENCY.md`
5. Update this README with usage examples

## See Also

- [MCP Domain Ontology](../../ontology/mcp-domain.ttl)
- [SHACL Shapes](../../ontology/shapes.ttl)
- [Graph Integrity Checker](./graph_integrity.rs)
- [Toyota Production System Research](../../docs/TPS_RESEARCH.md)
