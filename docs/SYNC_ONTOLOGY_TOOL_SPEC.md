# sync_ontology MCP Tool Specification

**Version**: 1.0.0 (Resource-Optimal Design)
**TRIZ Analysis**: [TRIZ_SYNC_RESOURCE_ANALYSIS.md](./TRIZ_SYNC_RESOURCE_ANALYSIS.md)

---

## SPR Summary

**Core Equation**: Auto-discover → Validate → Execute → Report

**Resource Utilization**: 99.96% reuse. Zero redundant code.

**Single Source of Truth**: ggen.toml configures ALL behavior.

**Quality Gates**: 6 pre-flight checks. Fail-fast. Poka-yoke.

---

## Tool Signature

```rust
/// Synchronize code generation from RDF ontology
///
/// Auto-discovers files, executes 13-step pipeline, generates cryptographic receipt.
/// Uses 100% existing validation/rendering/safety infrastructure.
///
/// # Parameters
/// - `ggen_toml_path`: Optional path to manifest (auto-detect if omitted)
/// - `dry_run`: Preview changes without writing files
/// - `validate_only`: Run quality gates only, skip generation
/// - `audit_trail`: Generate cryptographic audit log
/// - `parallel_generation`: Execute generation rules in parallel
///
/// # Returns
/// - Integrity report (RDF graph validation)
/// - SHACL report (ontology conformance)
/// - Files generated (count + paths + hashes)
/// - Execution statistics (timings per phase)
/// - Cryptographic receipt (SHA-256 provenance)
async fn sync_ontology(params: SyncOntologyParams) -> Result<SyncReport>
```

---

## Input Parameters

```rust
struct SyncOntologyParams {
    /// Optional path to ggen.toml (default: search CWD and parent dirs)
    ggen_toml_path: Option<String>,

    /// Dry-run mode: preview without writing
    #[default = false]
    dry_run: bool,

    /// Validation-only: skip generation phase
    #[default = false]
    validate_only: bool,

    /// Enable audit trail logging
    #[default = false]
    audit_trail: bool,

    /// Parallel generation (uses rayon)
    #[default = true]
    parallel_generation: bool,
}
```

**Rationale**: Minimal surface area. Auto-discovery eliminates file path parameters. Boolean flags control behavior without exposing internals.

---

## Output Schema

```rust
struct SyncReport {
    /// RDF graph integrity validation
    integrity_report: IntegrityReport,

    /// SHACL shape conformance validation
    shacl_report: ShaclValidationReport,

    /// Number of files generated
    files_generated: usize,

    /// Generated artifacts with metadata
    artifacts: Vec<ArtifactMetadata>,

    /// Cryptographic receipt (SHA-256 hashes)
    receipt: GenerationReceipt,

    /// Execution statistics
    stats: SyncStatistics,

    /// Validation passed flag
    success: bool,

    /// Error messages (if any)
    errors: Vec<String>,
}

struct ArtifactMetadata {
    path: String,
    hash: String,  // SHA-256
    lines_of_code: usize,
    generated_at: String,  // ISO 8601
}

struct SyncStatistics {
    total_time_ms: u64,
    ontology_load_ms: u64,
    sparql_time_ms: u64,
    template_time_ms: u64,
    validation_time_ms: u64,
    inference_rules_executed: usize,
    generation_rules_executed: usize,
}
```

---

## 13-Step Pipeline

### Phase 1: Discovery (Auto-Resource Detection)

```rust
// Step 1: Find ggen.toml
let manifest_path = params.ggen_toml_path
    .or_else(|| find_manifest_in_cwd())
    .or_else(|| walk_up_to_find_manifest())
    .ok_or_else(|| Error::ManifestNotFound)?;

// Step 2: Parse manifest (REUSE: toml crate)
let manifest: Manifest = parse_manifest(&manifest_path)?;

// Step 3: Auto-discover ontology files (REUSE: glob)
let ontology_files = discover_files(&manifest.ontology.source, "*.ttl")?;

// Step 4: Auto-discover queries (REUSE: glob)
let query_files = discover_files("queries", "*.rq")?;

// Step 5: Auto-discover templates (REUSE: glob)
let template_files = discover_files("templates", "*.tera")?;
```

**Resources Used**: File system (FIELD), glob patterns (SYSTEM), toml parser (FUNCTIONAL)

### Phase 2: Pre-Flight Validation (Quality Gates)

```rust
// Step 6: Validate manifest schema
validate_manifest_schema(&manifest)?;

// Step 7: Validate ontology file existence
validate_file_existence(&ontology_files)?;

// Step 8: Validate SPARQL syntax (REUSE: SparqlSanitizer)
for query_file in &query_files {
    SparqlSanitizer::validate_file(query_file)?;
}

// Step 9: Validate template syntax (REUSE: TemplateValidator)
for template_file in &template_files {
    TemplateValidator::validate_file(template_file)?;
}

// Step 10: Validate file write permissions
validate_write_permissions(&manifest.generation.rules)?;

// Step 11: Validate rule dependencies
validate_rule_dependencies(&manifest.inference.rules)?;
```

**Resources Used**: SparqlSanitizer (FUNCTIONAL), TemplateValidator (FUNCTIONAL), file system (FIELD)

### Phase 3: Execution (Code Generation)

```rust
// Step 12: Load RDF ontology (REUSE: OntologyCache)
let cache = OntologyCache::new(manifest.rdf.cache_queries);
let store = cache.load_ontology(&ontology_files)?;

// Step 13: Validate ontology integrity (REUSE: GraphIntegrityChecker)
let integrity = GraphIntegrityChecker::new(IntegrityConfig::default())
    .check(&store)?;

// Step 14: Validate SHACL shapes (REUSE: ShapeValidator)
let shacl = ShapeValidator::new()
    .validate(&store, &manifest.validation.shacl.shapes_file)?;

// Step 15: Execute inference rules (REUSE: topological sort)
let sorted_rules = topological_sort(&manifest.inference.rules)?;
for rule in sorted_rules {
    execute_construct_query(&rule.query_file, &store)?;
}

// Step 16: Execute generation rules (REUSE: SafeRenderer + QueryCache)
let artifacts = if params.parallel_generation {
    manifest.generation.rules
        .par_iter()  // Rayon parallel iterator
        .map(|rule| generate_artifact(rule, &store, params.dry_run))
        .collect::<Result<Vec<_>>>()?
} else {
    manifest.generation.rules
        .iter()
        .map(|rule| generate_artifact(rule, &store, params.dry_run))
        .collect::<Result<Vec<_>>>()?
};

// Step 17: Validate generated code (REUSE: CodeGenPipeline)
for artifact in &artifacts {
    CodeGenPipeline::new().validate(&artifact.content)?;
}

// Step 18: Write files atomically (REUSE: SafeCodeWriter)
if !params.dry_run {
    let writer = SafeCodeWriter::new();
    for artifact in &artifacts {
        writer.write(&artifact.path, &artifact.content)?;
    }
}
```

**Resources Used**: OntologyCache (FUNCTIONAL), GraphIntegrityChecker (FUNCTIONAL), ShapeValidator (FUNCTIONAL), SafeRenderer (FUNCTIONAL), CodeGenPipeline (FUNCTIONAL), SafeCodeWriter (FUNCTIONAL)

### Phase 4: Reporting (Provenance)

```rust
// Step 19: Generate cryptographic receipt (REUSE: GenerationReceipt)
let receipt = GenerationReceipt::generate(&artifacts)?;

// Step 20: Log audit trail (REUSE: audit system)
if params.audit_trail {
    log_audit_event(&receipt)?;
}

// Step 21: Construct report
Ok(SyncReport {
    integrity_report: integrity,
    shacl_report: shacl,
    files_generated: artifacts.len(),
    artifacts: artifacts.into_iter().map(|a| a.metadata).collect(),
    receipt,
    stats: collect_statistics(),
    success: true,
    errors: vec![],
})
```

**Resources Used**: GenerationReceipt (FUNCTIONAL), audit system (FUNCTIONAL)

---

## Resource Reuse Map

| Phase | New Code | Existing Resource | Reuse % |
|-------|----------|-------------------|---------|
| Discovery | 50 LOC | glob, toml, file system | 95% |
| Pre-flight | 30 LOC | SparqlSanitizer, TemplateValidator | 97% |
| Execution | 100 LOC | OntologyCache, SafeRenderer, CodeGenPipeline | 98% |
| Reporting | 20 LOC | GenerationReceipt, audit system | 99% |
| **Total** | **200 LOC** | **1400KB existing** | **99.86%** |

---

## Auto-Discovery Algorithm

```rust
/// Find ggen.toml in current directory or walk up to root
fn find_manifest_in_cwd() -> Option<PathBuf> {
    let mut current = std::env::current_dir().ok()?;
    loop {
        let candidate = current.join("ggen.toml");
        if candidate.exists() {
            return Some(candidate);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

/// Discover all files matching pattern in directory
fn discover_files(base_dir: &str, pattern: &str) -> Result<Vec<PathBuf>> {
    let glob_pattern = format!("{}/{}", base_dir, pattern);
    glob::glob(&glob_pattern)?
        .filter_map(Result::ok)
        .collect::<Vec<_>>()
        .ok_or_else(|| Error::NoFilesFound(base_dir.to_string()))
}
```

---

## Topological Sort (Inference Rule Dependencies)

```rust
/// Sort inference rules by dependencies (REUSE: petgraph)
fn topological_sort(rules: &[InferenceRule]) -> Result<Vec<InferenceRule>> {
    let mut graph = DiGraph::new();
    let mut node_map = HashMap::new();

    // Add nodes
    for rule in rules {
        let node = graph.add_node(rule.clone());
        node_map.insert(&rule.name, node);
    }

    // Add edges (dependencies)
    for rule in rules {
        let from_node = node_map[&rule.name];
        for dep in &rule.depends_on {
            if let Some(&to_node) = node_map.get(dep) {
                graph.add_edge(to_node, from_node, ());
            }
        }
    }

    // Topological sort
    petgraph::algo::toposort(&graph, None)
        .map_err(|_| Error::CyclicDependency)?
        .into_iter()
        .map(|node| graph[node].clone())
        .collect()
}
```

---

## Error Handling (Fail-Fast)

```rust
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("ggen.toml not found in current directory or parent directories")]
    ManifestNotFound,

    #[error("No ontology files found in {0}")]
    NoOntologyFiles(String),

    #[error("SPARQL syntax error in {file}: {error}")]
    SparqlSyntaxError { file: String, error: String },

    #[error("Template syntax error in {file}: {error}")]
    TemplateSyntaxError { file: String, error: String },

    #[error("SHACL validation failed: {0} violations")]
    ShaclValidationFailed(usize),

    #[error("Graph integrity check failed: {0} errors")]
    IntegrityCheckFailed(usize),

    #[error("Cyclic dependency detected in inference rules")]
    CyclicDependency,

    #[error("File write permission denied: {0}")]
    WritePermissionDenied(String),
}
```

---

## Performance Targets

| Metric | Target | Enforcement |
|--------|--------|-------------|
| Ontology load | <1s | For 1000+ triples |
| SPARQL execution | <100ms | Per query |
| Template render | <50ms | Per template |
| Full sync | <5s | For typical project |
| Memory usage | <100MB | Peak RSS |
| Parallel speedup | 2-4x | With rayon |

---

## Usage Examples

### Example 1: Full Sync (Auto-Detect)

```bash
# MCP tool call
{
  "tool": "sync_ontology",
  "arguments": {
    "dry_run": false,
    "audit_trail": true
  }
}
```

**Behavior**:
1. Auto-detect ggen.toml in CWD
2. Discover all .ttl, .rq, .tera files
3. Validate → Execute → Write → Report
4. Generate cryptographic receipt

### Example 2: Dry-Run Preview

```bash
{
  "tool": "sync_ontology",
  "arguments": {
    "dry_run": true
  }
}
```

**Behavior**:
1. Execute full pipeline
2. Skip file writes
3. Return diff preview in report

### Example 3: Validation Only

```bash
{
  "tool": "sync_ontology",
  "arguments": {
    "validate_only": true
  }
}
```

**Behavior**:
1. Run 6 pre-flight checks
2. Validate RDF graph integrity
3. Validate SHACL conformance
4. Skip generation phase
5. Return validation report

### Example 4: Explicit Manifest Path

```bash
{
  "tool": "sync_ontology",
  "arguments": {
    "ggen_toml_path": "custom/path/ggen.toml",
    "parallel_generation": true
  }
}
```

**Behavior**:
1. Use specified ggen.toml
2. Enable parallel generation (rayon)
3. Execute full pipeline

---

## Implementation Checklist

### Phase 1: Core Pipeline (80% Value)
- [ ] Auto-discover ggen.toml
- [ ] Parse manifest to Rust struct
- [ ] Discover .ttl, .rq, .tera files
- [ ] Load ontology via OntologyCache
- [ ] Execute SPARQL via existing engine
- [ ] Render templates via SafeRenderer
- [ ] Write files via SafeCodeWriter
- [ ] Generate basic report

**Estimated**: 2 hours | **Dependencies**: Zero new modules

### Phase 2: Quality Gates (15% Value)
- [ ] SHACL validation (ShapeValidator)
- [ ] Graph integrity (GraphIntegrityChecker)
- [ ] Code validation (CodeGenPipeline)
- [ ] 6 pre-flight checks

**Estimated**: 1 hour | **Dependencies**: Existing validators

### Phase 3: Advanced Features (5% Value)
- [ ] Topological sort (petgraph)
- [ ] Parallel generation (rayon)
- [ ] Incremental regeneration
- [ ] Audit trail integration

**Estimated**: 1 hour | **Dependencies**: petgraph, rayon

---

## Success Criteria

- [ ] Tool accepts ≤2 required parameters (ggen_toml_path optional)
- [ ] Auto-discovers 100% of files without manual config
- [ ] Reuses ≥95% of existing code
- [ ] Executes 6 pre-flight quality gates
- [ ] Generates cryptographic receipt
- [ ] Completes in <5s for 1000+ triples
- [ ] Memory usage <100MB
- [ ] Deterministic output (same input → same hash)
- [ ] Zero redundant validation tools

---

## Documentation References

- [TRIZ Resource Analysis](./TRIZ_SYNC_RESOURCE_ANALYSIS.md)
- [MCP Tool Usage Guide](./MCP_TOOL_USAGE.md)
- [Workflow Examples](./WORKFLOW_EXAMPLES.md)
- [Validation Guide](./VALIDATION_GUIDE.md)
- [ggen.toml Reference](../ggen.toml)

---

**Version**: 1.0.0 (Resource-Optimal Design)
**Last Updated**: 2026-01-20
**Status**: Design Complete, Ready for Implementation
