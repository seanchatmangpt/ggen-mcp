# TRIZ Function Analysis: ggen sync MCP Tool

**Date**: 2026-01-20
**System**: ggen-mcp sync tool (ontology-driven code generation)
**Method**: TRIZ Function Analysis
**Notation**: SPR (Sparse Priming Representation)

---

## 1. Function Model (Main + 9 Auxiliary)

### Main Function
**"Ontology → Code transformation. Deterministic projection."**

Single responsibility: μ(O) → A where μ = five-stage pipeline, O = RDF ontology, A = generated artifacts.

### Auxiliary Functions (How main function achieved)

| # | Function | Purpose | Current Implementation |
|---|----------|---------|------------------------|
| 1 | **Read Config** | Load generation manifest | `ggen.toml` → TOML parser → `Config` struct |
| 2 | **Discover Ontology** | Find RDF sources | Glob `ontology/*.ttl` → Oxigraph TripleStore |
| 3 | **Discover Queries** | Find SPARQL extractors | Glob `queries/*.rq` → SPARQL AST |
| 4 | **Discover Templates** | Find Tera generators | Glob `templates/*.tera` → Tera Engine |
| 5 | **Execute Queries** | Extract domain facts | SPARQL → Oxigraph → JSON bindings |
| 6 | **Render Templates** | Transform bindings → code | Tera + bindings → raw output |
| 7 | **Write Files** | Persist artifacts | `output_file` paths → filesystem |
| 8 | **Validate Output** | Quality gates | Syntax check, no unsafe, doc comments |
| 9 | **Report Stats** | Observability | Files generated, timings, errors |

**Pattern**: Discovery → Extraction → Transformation → Persistence → Verification

---

## 2. Harmful Functions (Undesired Effects)

### Current System (ggen.toml configuration)

| Harmful Function | Root Cause | Impact | TRIZ Principle |
|------------------|------------|--------|----------------|
| **Manual orchestration** | Sequential pipeline (9 steps) | Slow, error-prone | **Principle 10**: Prior action (pre-compute dependencies) |
| **Error propagation** | No partial success handling | All-or-nothing failure | **Principle 11**: Beforehand cushioning (checkpoints) |
| **Partial execution** | No atomic transactions | Inconsistent state (orphaned files) | **Principle 15**: Dynamization (rollback state) |
| **State pollution** | Cache/temp files persist on failure | Corrupt regeneration | **Principle 34**: Discard/recover (clean temp on exit) |
| **Template injection** | No template sandboxing | Arbitrary code execution | **Principle 24**: Mediator (sandbox execution) |
| **SPARQL injection** | Templates embed user input | Malicious queries | **Principle 3**: Local quality (query builder) |
| **Path traversal** | `output_file` paths unchecked | Overwrite system files | **Principle 26**: Copying (validate paths) |
| **Dependency hell** | Inference rules lack topology sort | Circular dependencies deadlock | **Principle 28**: Replace mechanical system (DAG) |
| **Non-determinism** | Timestamp/random in generation | Different outputs from same input | **Principle 35**: Parameter change (deterministic IDs) |

### Elimination Strategy (SPR)

**Core Pattern**: Prevention > Detection > Recovery.

```
Template Injection    → Sandbox execution (Principle 24: Mediator)
SPARQL Injection      → Query builder, no string concat (Principle 3: Local quality)
Path Traversal        → Whitelist validation (Principle 26: Copying safe patterns)
Error Propagation     → Checkpoints, resume capability (Principle 11: Beforehand cushioning)
Partial Execution     → Atomic write (temp → rename) (Principle 15: Dynamization)
State Pollution       → Auto-cleanup on Drop (Principle 34: Discard/recover)
Dependency Hell       → Topological sort (Principle 28: DAG ordering)
Non-determinism       → Content-addressed IDs (Principle 35: SHA-256 hashing)
Manual Orchestration  → Dependency graph, parallel execution (Principle 10: Prior action)
```

**Implementation**: Six pre-flight checks (Poka-Yoke) + Five-stage pipeline (μ₁-μ₅) + Cryptographic receipts.

---

## 3. Insufficient Functions (Missing Capabilities)

### Critical Gaps

| Missing Function | Current Workaround | Ideal Solution | TRIZ Principle |
|------------------|-------------------|----------------|----------------|
| **Atomic transaction** | Partial writes possible | Temp dir → atomic rename | **Principle 1**: Segmentation (isolate workspace) |
| **Rollback on error** | Manual cleanup required | Checkpoint stack, auto-restore | **Principle 13**: Inversion (reverse operations) |
| **Incremental sync** | Regenerate all files | Content hash → skip unchanged | **Principle 16**: Partial/excessive action (generate only Δ) |
| **Dependency resolution** | Manual rule ordering | Topological sort (depends_on) | **Principle 28**: Replace mechanical (DAG scheduler) |
| **Conflict detection** | Overwrites silently | Detect overlapping rules, warn | **Principle 9**: Preliminary anti-action (pre-check) |
| **Dry-run preview** | `--dry_run` (basic) | Show diffs, impact analysis | **Principle 30**: Flexible shells (virtual filesystem) |
| **Cache invalidation** | Manual cache clear | Hash-based smart busting | **Principle 25**: Self-service (auto-detect stale) |
| **Parallel execution** | Sequential processing | Parallel independent rules | **Principle 10**: Prior action (parallel μ₃) |
| **Progress reporting** | Silent until completion | Real-time progress bar | **Principle 19**: Periodic action (emit updates) |
| **Checkpoint/resume** | Restart from scratch | Resume interrupted sync | **Principle 11**: Beforehand cushioning (save state) |

### Implementation Plan (80/20 Focus)

**20% effort → 80% value:**

1. **Atomic Transaction (Principle 1: Segmentation)**
   - Write to `.ggen/temp/{execution_id}/` → Validate → Atomic rename
   - On error: Auto-cleanup temp dir (Drop trait)
   - **Value**: Prevents inconsistent state, enables safe concurrent runs

2. **Incremental Sync (Principle 16: Partial action)**
   - Content-hash inputs (ontology + template + query)
   - Skip generation if output hash matches
   - **Value**: 10x faster for small changes (80% use case)

3. **Dependency Resolution (Principle 28: DAG)**
   - Topological sort `inference.rules` by `depends_on`
   - Detect cycles → fail-fast with clear error
   - **Value**: Prevents deadlock, enables parallelization

4. **Conflict Detection (Principle 9: Preliminary anti-action)**
   - Pre-check: Multiple rules → same `output_file` → error
   - Warn: Overlapping glob patterns
   - **Value**: Prevents silent overwrites (Poka-Yoke)

5. **Dry-Run Preview (Principle 30: Flexible shells)**
   - Virtual filesystem (HashMap<PathBuf, String>)
   - Diff against actual filesystem
   - **Value**: Safe exploration, user confidence

**Implementation Order** (dependency-aware):
1. Atomic Transaction (foundation for all others)
2. Incremental Sync (performance win)
3. Dependency Resolution (enables parallelization)
4. Conflict Detection (safety gate)
5. Dry-Run Preview (user experience)

---

## 4. Minimal Sync Tool Design (Maximizes Main, Minimizes Harmful)

### Core Architecture (Type-Safe)

```rust
/// Main Function: μ(O) → A
/// Five-stage deterministic pipeline
pub struct SyncPipeline {
    config: Config,                    // Read once, immutable
    workspace: AtomicWorkspace,        // Isolated temp directory
    dag: DependencyGraph,              // Topological ordering
    cache: IncrementalCache,           // Content-addressed
    reporter: ProgressReporter,        // Real-time feedback
}

impl SyncPipeline {
    /// μ₁ (Normalize): RDF validation, SHACL shapes, dependency resolution
    pub fn normalize(&self) -> Result<ValidatedOntology, Error> {
        // Pre-flight checks (Poka-Yoke)
        self.validate_manifest()?;
        self.validate_ontology()?;
        self.validate_queries()?;
        self.validate_templates()?;
        self.validate_permissions()?;
        self.validate_rules()?;

        // Load and validate ontology
        let store = TripleStore::load(&self.config.ontology.source)?;
        let shacl = ShaclValidator::new(&self.config.validation.shacl)?;
        shacl.validate(&store)?;

        Ok(ValidatedOntology { store })
    }

    /// μ₂ (Extract): SPARQL queries, OWL inference, rule execution
    pub fn extract(&self, ontology: &ValidatedOntology) -> Result<Vec<Binding>, Error> {
        // Topological sort (Principle 28: DAG)
        let ordered_rules = self.dag.topological_sort()?;

        // Execute queries in dependency order
        let mut bindings = Vec::new();
        for rule in ordered_rules {
            // Incremental cache check (Principle 16: Partial action)
            let cache_key = self.cache.hash_input(&ontology, &rule)?;
            if let Some(cached) = self.cache.get(&cache_key) {
                bindings.push(cached);
                continue;
            }

            // Execute SPARQL query
            let query = self.load_query_safe(&rule.query)?; // Injection prevention
            let result = ontology.store.execute(&query)?;

            // Cache result
            self.cache.insert(cache_key, result.clone());
            bindings.push(result);
        }

        Ok(bindings)
    }

    /// μ₃ (Emit): Tera template rendering, code generation
    pub fn emit(&self, bindings: &[Binding]) -> Result<Vec<Artifact>, Error> {
        // Parallel execution (Principle 10: Prior action)
        let generation_rules = self.config.generation.rules.clone();

        generation_rules.par_iter()
            .map(|rule| {
                // Template sandbox (Principle 24: Mediator)
                let template = self.load_template_sandboxed(&rule.template)?;

                // Render with bindings
                let output = template.render(&bindings)?;

                // Conflict detection (Principle 9: Preliminary anti-action)
                self.detect_conflict(&rule.output_file)?;

                Ok(Artifact {
                    path: rule.output_file.clone(),
                    content: output,
                    hash: sha256(&output), // Deterministic ID
                })
            })
            .collect()
    }

    /// μ₄ (Canonicalize): Deterministic formatting, content hashing
    pub fn canonicalize(&self, artifacts: &mut Vec<Artifact>) -> Result<(), Error> {
        for artifact in artifacts.iter_mut() {
            // Deterministic formatting (rustfmt, prettier, etc.)
            artifact.content = self.format_deterministic(&artifact.content)?;

            // Recompute hash after formatting
            artifact.hash = sha256(&artifact.content);
        }

        Ok(())
    }

    /// μ₅ (Receipt): Cryptographic proof generation, audit trail
    pub fn receipt(&self, artifacts: &[Artifact]) -> Result<Receipt, Error> {
        let receipt = Receipt {
            execution_id: Ulid::new(), // Deterministic, sortable
            timestamp: Utc::now(),
            manifest_hash: sha256(&self.config),
            ontology_hash: sha256(&self.ontology),
            artifacts: artifacts.iter()
                .map(|a| (a.path.clone(), a.hash.clone()))
                .collect(),
            inference_rules: self.dag.executed_rules(),
            generation_rules: self.config.generation.rules.len(),
        };

        // Write audit trail
        self.write_audit_log(&receipt)?;

        Ok(receipt)
    }

    /// Atomic write (Principle 1: Segmentation)
    pub fn commit(&self, artifacts: &[Artifact]) -> Result<(), Error> {
        // Write to temp workspace
        for artifact in artifacts {
            let temp_path = self.workspace.temp_path(&artifact.path);
            fs::write(&temp_path, &artifact.content)?;
        }

        // Validate all outputs
        self.validate_outputs(&artifacts)?;

        // Atomic rename (all-or-nothing)
        for artifact in artifacts {
            let temp_path = self.workspace.temp_path(&artifact.path);
            let final_path = self.workspace.final_path(&artifact.path);
            fs::rename(&temp_path, &final_path)?;
        }

        Ok(())
    }

    /// Rollback on error (Principle 13: Inversion)
    pub fn rollback(&self) -> Result<(), Error> {
        // Workspace auto-cleanup on Drop
        drop(&self.workspace);
        Ok(())
    }
}

/// Atomic workspace (Principle 1: Segmentation)
struct AtomicWorkspace {
    temp_dir: TempDir,      // Auto-cleanup on Drop (Principle 34)
    execution_id: Ulid,     // Deterministic ID
}

impl AtomicWorkspace {
    fn temp_path(&self, output_file: &Path) -> PathBuf {
        self.temp_dir.path()
            .join(self.execution_id.to_string())
            .join(output_file)
    }

    fn final_path(&self, output_file: &Path) -> PathBuf {
        // Path traversal validation (Principle 26: Copying)
        validate_path_safe(output_file)?;
        output_file.to_path_buf()
    }
}

/// Dependency graph (Principle 28: DAG)
struct DependencyGraph {
    rules: Vec<InferenceRule>,
    adjacency: HashMap<RuleId, Vec<RuleId>>,
}

impl DependencyGraph {
    fn topological_sort(&self) -> Result<Vec<&InferenceRule>, Error> {
        // Kahn's algorithm for DAG topological sort
        // Detect cycles → fail-fast
        // ...
    }
}

/// Incremental cache (Principle 16: Partial action)
struct IncrementalCache {
    store: HashMap<Hash, Binding>,
}

impl IncrementalCache {
    fn hash_input(&self, ontology: &ValidatedOntology, rule: &GenerationRule) -> Hash {
        // Content-addressed: hash(ontology + query + template)
        let mut hasher = Sha256::new();
        hasher.update(&ontology.hash);
        hasher.update(&rule.query.content);
        hasher.update(&rule.template.content);
        Hash(hasher.finalize().to_vec())
    }
}
```

### Usage (Minimal API)

```rust
// 1. Initialize pipeline
let pipeline = SyncPipeline::new("ggen.toml")?;

// 2. Execute five-stage pipeline
let ontology = pipeline.normalize()?;           // μ₁
let bindings = pipeline.extract(&ontology)?;    // μ₂
let mut artifacts = pipeline.emit(&bindings)?;  // μ₃
pipeline.canonicalize(&mut artifacts)?;         // μ₄
let receipt = pipeline.receipt(&artifacts)?;    // μ₅

// 3. Atomic commit (all-or-nothing)
pipeline.commit(&artifacts)?;

// On error: auto-rollback via Drop
```

### Key Features (Harmful Function Elimination)

| Harmful Function | Elimination Mechanism |
|------------------|----------------------|
| Manual orchestration | Five-stage pipeline (μ₁-μ₅), automatic |
| Error propagation | Atomic workspace, rollback on Drop |
| Partial execution | Temp → validate → atomic rename |
| State pollution | TempDir auto-cleanup (Drop trait) |
| Template injection | Sandboxed Tera environment |
| SPARQL injection | Query builder, no string concat |
| Path traversal | `validate_path_safe()` on all outputs |
| Dependency hell | Topological sort (Kahn's algorithm) |
| Non-determinism | Content-addressed hashing (SHA-256) |

---

## 5. TRIZ Principles Applied

| Principle | Application | Code Location |
|-----------|-------------|---------------|
| **1. Segmentation** | Atomic workspace isolation | `AtomicWorkspace` |
| **3. Local Quality** | SPARQL query builder | `load_query_safe()` |
| **9. Preliminary Anti-Action** | Conflict detection pre-check | `detect_conflict()` |
| **10. Prior Action** | Parallel template rendering | `emit()` with `par_iter()` |
| **11. Beforehand Cushioning** | Checkpoint/resume capability | `Receipt` + audit trail |
| **13. Inversion** | Rollback via Drop trait | `AtomicWorkspace::drop()` |
| **15. Dynamization** | Temp → atomic rename | `commit()` |
| **16. Partial/Excessive Action** | Incremental cache, skip unchanged | `IncrementalCache` |
| **19. Periodic Action** | Progress reporting | `ProgressReporter` |
| **24. Mediator** | Template sandbox | `load_template_sandboxed()` |
| **25. Self-Service** | Auto cache invalidation | `hash_input()` |
| **26. Copying** | Validate paths against whitelist | `validate_path_safe()` |
| **28. Replace Mechanical** | DAG scheduler | `DependencyGraph::topological_sort()` |
| **30. Flexible Shells** | Virtual filesystem for dry-run | `--dry_run` mode |
| **34. Discard/Recover** | Auto-cleanup temp files | `TempDir::drop()` |
| **35. Parameter Change** | Deterministic IDs (SHA-256) | `sha256()` everywhere |

---

## 6. Implementation Roadmap (80/20)

### Phase 1: Foundation (Week 1)
- [ ] Atomic workspace (`AtomicWorkspace` struct)
- [ ] Content-addressed cache (`IncrementalCache`)
- [ ] Dependency graph (`DependencyGraph` + topological sort)
- [ ] Pre-flight validation (six Poka-Yoke checks)

### Phase 2: Safety (Week 2)
- [ ] Template sandboxing (restrict filesystem access)
- [ ] SPARQL query builder (prevent injection)
- [ ] Path traversal validation (`validate_path_safe()`)
- [ ] Conflict detection (overlapping rules)

### Phase 3: Performance (Week 3)
- [ ] Parallel template rendering (`rayon` par_iter)
- [ ] Incremental sync (skip unchanged files)
- [ ] Progress reporting (real-time feedback)
- [ ] Benchmark suite (SLO verification)

### Phase 4: Observability (Week 4)
- [ ] Cryptographic receipts (SHA-256 audit trail)
- [ ] Dry-run preview (virtual filesystem diffs)
- [ ] Checkpoint/resume (interrupted sync recovery)
- [ ] Error context mapping (rich error messages)

---

## 7. Success Metrics

| Metric | Current | Target | TRIZ Principle |
|--------|---------|--------|----------------|
| **Safety**: Path traversal vulnerabilities | Unknown | 0 | Principle 26 (Copying) |
| **Safety**: SPARQL injection vectors | Unknown | 0 | Principle 3 (Local Quality) |
| **Safety**: Template injection vectors | Unknown | 0 | Principle 24 (Mediator) |
| **Performance**: First sync (1k triples) | ~10s | ≤5s | Principle 16 (Incremental) |
| **Performance**: Incremental sync (no changes) | ~10s | ≤1s | Principle 16 (Partial Action) |
| **Reliability**: Partial execution failures | Unknown | 0 | Principle 1 (Segmentation) |
| **Reliability**: State pollution on error | Unknown | 0 | Principle 34 (Discard/Recover) |
| **Usability**: Dry-run diff preview | No | Yes | Principle 30 (Flexible Shells) |
| **Observability**: Cryptographic audit trail | No | Yes | Principle 11 (Beforehand Cushioning) |

---

## 8. References

- **TRIZ 40 Principles**: Altshuller, G. (1969). "The Innovation Algorithm"
- **ggen Architecture**: `/home/user/ggen-mcp/ggen.toml`
- **Poka-Yoke Patterns**: `/home/user/ggen-mcp/POKA_YOKE_IMPLEMENTATION.md`
- **Chicago TDD**: `/home/user/ggen-mcp/CHICAGO_TDD_TEST_HARNESS_COMPLETE.md`
- **Five-Stage Pipeline**: ggen v6.0.0 (μ₁-μ₅ transformation)

---

**Conclusion**: Minimal sync tool = Five-stage pipeline + Atomic workspace + Dependency DAG + Incremental cache. Eliminates 9 harmful functions via 16 TRIZ principles. 80/20 focus: Safety > Performance > Observability.
