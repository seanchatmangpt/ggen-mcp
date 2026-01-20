# sync_ontology Implementation Guide

**Version**: 1.0.0 | Developer guide | 80/20 implementation

---

## Quick Start (For Developers)

### 1. Read Design Documents
- `SYNC_ONTOLOGY_TRIZ_DESIGN.md` (comprehensive, 22K tokens)
- `SYNC_ONTOLOGY_EXECUTIVE_SUMMARY.md` (SPR, 2.5K tokens) ← Start here

### 2. Review Scaffolding
Code skeleton ready in `src/tools/sync/`:
```
sync/
├── mod.rs           ✓ API defined
├── discovery.rs     ✓ Auto-discovery complete
├── pipeline.rs      ⏳ Stages 5-13 TODO
├── transaction.rs   ✓ Atomic writes complete
└── cache.rs         ✓ Query cache complete
```

### 3. Implementation Tasks (Priority Order)

#### Task 1: Stage 5 - Execute SPARQL (HIGH PRIORITY)
**File**: `src/tools/sync/stages/execute_sparql.rs`

**What**: Execute SPARQL queries against Oxigraph store, with caching.

**Dependencies**: Reuse `src/tools/ontology_sparql.rs`

**Code Skeleton**:
```rust
use crate::tools::ontology_sparql::execute_sparql_query;
use crate::tools::sync::cache::QueryCache;
use oxigraph::store::Store;

pub async fn execute_queries_parallel(
    store: &Store,
    resources: &ResourceDiscovery,
    params: &SyncOntologyParams,
) -> Result<HashMap<String, serde_json::Value>> {
    use rayon::prelude::*;

    let cache = QueryCache::new(&resources.cache_dir);

    resources.pairs()
        .par_iter()  // Rayon parallel
        .map(|(name, query_path, _)| {
            // Check cache
            let query_content = std::fs::read_to_string(query_path)?;
            let ontology_content = get_ontology_content(store)?;
            let cache_key = QueryCache::compute_key(&ontology_content, &query_content);

            if !params.force {
                if let Some(cached) = cache.get(&cache_key) {
                    return Ok((name.clone(), serde_json::from_str(&cached)?));
                }
            }

            // Execute query
            let result = execute_sparql_query(store, &query_content)?;

            // Cache result
            cache.set(&cache_key, &serde_json::to_string(&result)?)?;

            Ok((name.clone(), result))
        })
        .collect()
}
```

**Test**:
```bash
cargo test --test sync_execute_sparql
```

---

#### Task 2: Stage 7 - Render Templates (HIGH PRIORITY)
**File**: `src/tools/sync/stages/render_templates.rs`

**What**: Render Tera templates with SPARQL results.

**Dependencies**: Reuse `src/template/SafeRenderer`

**Code Skeleton**:
```rust
use crate::template::{RenderConfig, SafeRenderer, RenderContext};
use tera::Context;

pub async fn render_templates_parallel(
    resources: &ResourceDiscovery,
    query_results: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<PathBuf, String>> {
    use rayon::prelude::*;

    // Create Tera engine with all templates
    let config = RenderConfig::default()
        .with_security_checks(true)
        .with_syntax_validation(true);

    let renderer = SafeRenderer::new(config)?;

    // Load all templates
    for (name, template_path) in &resources.templates {
        let content = std::fs::read_to_string(template_path)?;
        renderer.add_template(name, &content)?;
    }

    // Render in parallel
    resources.pairs()
        .par_iter()
        .map(|(name, _, template_path)| {
            // Get query result
            let query_result = query_results.get(name)
                .ok_or_else(|| anyhow!("Missing query result for {}", name))?;

            // Build Tera context
            let mut context = RenderContext::new();
            context.insert("data", query_result)?;

            // Render
            let output = renderer.render_safe(name, &context)?;

            // Determine output path
            let output_path = PathBuf::from(format!("src/generated/{}.rs", name));

            Ok((output_path, output))
        })
        .collect()
}
```

**Test**:
```bash
cargo test --test sync_render_templates
```

---

#### Task 3: Stage 8 - Validate Syntax (MEDIUM PRIORITY)
**File**: `src/tools/sync/stages/validate_syntax.rs`

**What**: Validate generated code syntax (Rust, JSON, YAML).

**Dependencies**: Reuse `src/codegen/validation.rs`

**Code Skeleton**:
```rust
use crate::codegen::validation::validate_rust_code;
use syn::parse_file;

pub async fn validate_syntax_all(
    files: &HashMap<PathBuf, String>,
) -> Result<Vec<ValidationError>> {
    let mut errors = Vec::new();

    for (path, content) in files {
        let extension = path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        match extension {
            "rs" => {
                if let Err(e) = parse_file(content) {
                    errors.push(ValidationError {
                        file: path.clone(),
                        message: format!("Rust syntax error: {}", e),
                    });
                }
            }
            "json" => {
                if let Err(e) = serde_json::from_str::<serde_json::Value>(content) {
                    errors.push(ValidationError {
                        file: path.clone(),
                        message: format!("JSON syntax error: {}", e),
                    });
                }
            }
            "yaml" => {
                if let Err(e) = serde_yaml::from_str::<serde_yaml::Value>(content) {
                    errors.push(ValidationError {
                        file: path.clone(),
                        message: format!("YAML syntax error: {}", e),
                    });
                }
            }
            _ => {}
        }
    }

    ensure!(errors.is_empty(), "Syntax validation failed: {} errors", errors.len());
    Ok(errors)
}
```

---

#### Task 4: Stage 9 - Format Output (MEDIUM PRIORITY)
**File**: `src/tools/sync/stages/format_output.rs`

**What**: Format Rust code with rustfmt, JSON with serde_json::to_string_pretty.

**Code Skeleton**:
```rust
use std::process::Command;

pub async fn format_all(
    files: &HashMap<PathBuf, String>,
) -> Result<HashMap<PathBuf, String>> {
    let mut formatted = HashMap::new();

    for (path, content) in files {
        let extension = path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        let formatted_content = match extension {
            "rs" => format_rust(content)?,
            "json" => format_json(content)?,
            _ => content.clone(),
        };

        formatted.insert(path.clone(), formatted_content);
    }

    Ok(formatted)
}

fn format_rust(code: &str) -> Result<String> {
    // Write to temp file
    let temp_file = tempfile::NamedTempFile::new()?;
    std::fs::write(temp_file.path(), code)?;

    // Run rustfmt
    let output = Command::new("rustfmt")
        .arg("--emit=stdout")
        .arg(temp_file.path())
        .output()?;

    ensure!(output.status.success(), "rustfmt failed");

    Ok(String::from_utf8(output.stdout)?)
}

fn format_json(json: &str) -> Result<String> {
    let value: serde_json::Value = serde_json::from_str(json)?;
    Ok(serde_json::to_string_pretty(&value)?)
}
```

---

#### Task 5: Stage 10 - Check Compilation (LOW PRIORITY, STRICT MODE ONLY)
**File**: `src/tools/sync/stages/check_compilation.rs`

**What**: Run `cargo check` on generated code (only in strict mode).

**Code Skeleton**:
```rust
use std::process::Command;

pub async fn check_compilation(
    files: &HashMap<PathBuf, String>,
) -> Result<()> {
    // Write all files to temp workspace
    let temp_dir = tempfile::tempdir()?;
    let temp_src = temp_dir.path().join("src");
    std::fs::create_dir_all(&temp_src)?;

    for (path, content) in files {
        let dest = temp_src.join(path.file_name().unwrap());
        std::fs::write(&dest, content)?;
    }

    // Create minimal Cargo.toml
    let cargo_toml = r#"
[package]
name = "generated-check"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
    "#;
    std::fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml)?;

    // Run cargo check
    let output = Command::new("cargo")
        .arg("check")
        .arg("--quiet")
        .current_dir(temp_dir.path())
        .output()?;

    ensure!(output.status.success(),
        "Compilation check failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}
```

---

#### Task 6: Stage 11 - Detect TODOs (LOW PRIORITY)
**File**: `src/tools/sync/stages/detect_todos.rs`

**What**: Scan generated code for TODO markers (fail-fast).

**Code Skeleton**:
```rust
use regex::Regex;

pub async fn detect_todos(
    files: &HashMap<PathBuf, String>,
) -> Result<()> {
    let todo_regex = Regex::new(r"(?i)TODO|FIXME|XXX")?;
    let mut todos = Vec::new();

    for (path, content) in files {
        for (line_num, line) in content.lines().enumerate() {
            if todo_regex.is_match(line) {
                todos.push(TodoOccurrence {
                    file: path.clone(),
                    line: line_num + 1,
                    text: line.to_string(),
                });
            }
        }
    }

    ensure!(todos.is_empty(),
        "Generated code contains {} TODOs (not allowed)",
        todos.len()
    );

    Ok(())
}

struct TodoOccurrence {
    file: PathBuf,
    line: usize,
    text: String,
}
```

---

#### Task 7: Stages 1-4 Completion (MEDIUM PRIORITY)
**Current Status**: Skeleton implemented in `pipeline.rs`

**TODO**:
1. Stage 1: Load ontology with Oxigraph
2. Stage 2: SHACL validation
3. Stage 3: RDF import resolution
4. Stage 4: Already implemented ✓

**Stage 1 Implementation**:
```rust
async fn stage_load_ontology(&self) -> Result<(Ontology, PipelineStage)> {
    let start = Instant::now();

    use oxigraph::store::Store;
    use oxigraph::io::GraphFormat;

    let mut store = Store::new()?;

    // Load all ontology files
    let ontology_path = Path::new(&self.params.ontology_path);
    if ontology_path.is_dir() {
        for entry in glob::glob(&format!("{}/*.ttl", ontology_path.display()))? {
            let path = entry?;
            let content = std::fs::read_to_string(&path)?;
            store.load_from_reader(GraphFormat::Turtle, content.as_bytes())?;
        }
    } else {
        let content = std::fs::read_to_string(ontology_path)?;
        store.load_from_reader(GraphFormat::Turtle, content.as_bytes())?;
    }

    Ok((Ontology { store }, PipelineStage {
        stage_number: 1,
        stage_name: "Load Ontology".to_string(),
        status: StageStatus::Completed,
        duration_ms: start.elapsed().as_millis() as u64,
        details: format!("Loaded {} triples", store.len()?),
    }))
}

struct Ontology {
    store: oxigraph::store::Store,
}
```

**Stage 2 Implementation** (SHACL):
```rust
// SHACL validation is complex - consider using external library
// or defer to future version
async fn stage_validate_shacl(&self, ontology: &Ontology) -> Result<PipelineStage> {
    let start = Instant::now();

    if self.params.validation_level == ValidationLevel::Minimal {
        return Ok(PipelineStage {
            stage_number: 2,
            stage_name: "Validate SHACL".to_string(),
            status: StageStatus::Skipped,
            duration_ms: 0,
            details: "Skipped in minimal mode".to_string(),
        });
    }

    // TODO: Implement SHACL validation
    // For MVP, just check that ontology loaded successfully
    ensure!(ontology.store.len()? > 0, "Empty ontology");

    Ok(PipelineStage {
        stage_number: 2,
        stage_name: "Validate SHACL".to_string(),
        status: StageStatus::Completed,
        duration_ms: start.elapsed().as_millis() as u64,
        details: "Basic validation passed".to_string(),
    })
}
```

---

### 4. Integration (Wire Everything Together)

**File**: `src/tools/sync/pipeline.rs` (complete execute method)

```rust
pub async fn execute(self) -> Result<SyncOntologyResponse> {
    let sync_id = Self::generate_sync_id();
    let start_time = Instant::now();
    let mut stages = Vec::with_capacity(13);
    let mut transaction = FileTransaction::new();

    // Stages 1-4 (existing)
    let (ontology, stage1) = self.stage_load_ontology().await?;
    stages.push(stage1);

    let stage2 = self.stage_validate_shacl(&ontology).await?;
    stages.push(stage2);

    let (store, stage3) = self.stage_resolve_dependencies(&ontology).await?;
    stages.push(stage3);

    let (resources, stage4) = self.stage_discover_resources().await?;
    stages.push(stage4);

    // Stage 5: Execute SPARQL (NEW)
    let (query_results, stage5) = self.stage_execute_sparql(&store, &resources).await?;
    stages.push(stage5);

    // Stage 6: Validate results (NEW)
    let stage6 = self.stage_validate_results(&query_results).await?;
    stages.push(stage6);

    // Stage 7: Render templates (NEW)
    let (rendered, stage7) = self.stage_render_templates(&resources, &query_results).await?;
    stages.push(stage7);

    // Stage 8: Validate syntax (NEW)
    let stage8 = self.stage_validate_syntax(&rendered).await?;
    stages.push(stage8);

    // Stage 9: Format output (NEW)
    let (formatted, stage9) = self.stage_format_output(&rendered).await?;
    stages.push(stage9);

    // Stage 10: Check compilation (NEW, strict mode only)
    if self.params.validation_level == ValidationLevel::Strict {
        let stage10 = self.stage_check_compilation(&formatted).await?;
        stages.push(stage10);
    }

    // Stage 11: Detect TODOs (NEW)
    let stage11 = self.stage_detect_todos(&formatted).await?;
    stages.push(stage11);

    // Stage 12: Write files (NEW)
    if !self.params.preview {
        for (path, content) in &formatted {
            transaction.stage_write(path, content)?;
        }
        transaction.commit()?;
    }
    let stage12 = PipelineStage {
        stage_number: 12,
        stage_name: "Write Files".to_string(),
        status: if self.params.preview { StageStatus::Skipped } else { StageStatus::Completed },
        duration_ms: 0,
        details: if self.params.preview {
            "Preview mode: no files written".to_string()
        } else {
            format!("{} files written", formatted.len())
        },
    };
    stages.push(stage12);

    // Stage 13: Generate receipt (NEW)
    let receipt = if self.params.audit_trail {
        Some(self.stage_generate_receipt(&sync_id, &ontology, &formatted).await?)
    } else {
        None
    };
    let stage13 = PipelineStage {
        stage_number: 13,
        stage_name: "Generate Receipt".to_string(),
        status: if receipt.is_some() { StageStatus::Completed } else { StageStatus::Skipped },
        duration_ms: 0,
        details: receipt.as_ref()
            .map(|r| format!("Receipt: {}", r.receipt_id))
            .unwrap_or_else(|| "Skipped".to_string()),
    };
    stages.push(stage13);

    // Build response
    Ok(SyncOntologyResponse {
        sync_id,
        timestamp: chrono::Utc::now().to_rfc3339(),
        status: SyncStatus::Success,
        pipeline_stages: stages,
        files_generated: build_file_list(&formatted, &resources),
        validation_results: ValidationResults::all_passed(),
        audit_receipt: receipt,
        statistics: compute_stats(start_time, &formatted, &query_results),
        errors: vec![],
        preview: self.params.preview,
    })
}
```

---

### 5. Testing Strategy

#### Unit Tests (Per Stage)
```bash
# Test each stage independently
cargo test --test sync_stage_1_load_ontology
cargo test --test sync_stage_5_execute_sparql
cargo test --test sync_stage_7_render_templates
# etc.
```

#### Integration Tests (Full Pipeline)
```bash
# Test end-to-end pipeline
cargo test --test sync_integration_basic
cargo test --test sync_integration_preview_mode
cargo test --test sync_integration_strict_mode
cargo test --test sync_integration_rollback
```

**Test Fixture**:
```
tests/fixtures/sync/
├── ontology/
│   └── test-domain.ttl       # Minimal test ontology
├── queries/
│   └── test_entity.rq        # Minimal SPARQL query
├── templates/
│   └── test_entity.rs.tera   # Minimal template
└── expected/
    └── test_entity.rs        # Expected output
```

**Example Integration Test**:
```rust
#[tokio::test]
async fn test_sync_basic_pipeline() {
    let params = SyncOntologyParams {
        ontology_path: "tests/fixtures/sync/ontology/".to_string(),
        preview: false,
        validation_level: ValidationLevel::Standard,
        parallel: false,
        force: false,
        audit_trail: true,
        config_path: None,
    };

    let state = Arc::new(AppState::new());
    let response = sync_ontology(state, params).await.unwrap();

    assert_eq!(response.status, SyncStatus::Success);
    assert_eq!(response.pipeline_stages.len(), 13);
    assert!(response.files_generated.len() > 0);
    assert!(response.validation_results.ontology_valid);

    // Verify output matches expected
    let generated = std::fs::read_to_string("src/generated/test_entity.rs").unwrap();
    let expected = std::fs::read_to_string("tests/fixtures/sync/expected/test_entity.rs").unwrap();
    assert_eq!(generated, expected);
}
```

---

### 6. Performance Optimization

#### Parallel Execution (Rayon)
```toml
# Cargo.toml
[dependencies]
rayon = "1.8"
```

```rust
use rayon::prelude::*;

// Parallel query execution
resources.pairs()
    .par_iter()  // Rayon parallel iterator
    .map(|(name, query_path, _)| {
        execute_query(store, query_path)
    })
    .collect()
```

#### Caching Strategy
```
Cache Key: SHA-256(ontology_content + query_content)
Cache Dir: .ggen/cache/
Cache Invalidation: File mtime check
Cache TTL: 1 hour (configurable)
```

#### Benchmarks
```bash
cargo bench --bench sync_performance
```

**Benchmark Template**:
```rust
#[bench]
fn bench_sync_small_ontology(b: &mut Bencher) {
    let params = SyncOntologyParams { /* ... */ };
    b.iter(|| {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(sync_ontology(state.clone(), params.clone()))
    });
}
```

---

### 7. Error Handling

#### Error Categories
1. **Ontology Errors**: Invalid RDF, SHACL violations
2. **Query Errors**: Invalid SPARQL, execution failures
3. **Template Errors**: Invalid Tera syntax, rendering failures
4. **Validation Errors**: Invalid Rust syntax, compilation failures
5. **IO Errors**: File read/write failures

#### Error Recovery
```rust
match stage_result {
    Ok(_) => continue,
    Err(e) => {
        // Log error
        tracing::error!("Stage {} failed: {}", stage_num, e);

        // Rollback transaction
        transaction.rollback()?;

        // Return detailed error
        return Err(SyncError {
            stage: stage_name,
            severity: ErrorSeverity::Error,
            message: e.to_string(),
            suggestion: Some(suggest_fix(&e)),
        });
    }
}
```

---

### 8. Documentation Updates

#### MCP_TOOL_USAGE.md
```markdown
## Tool 5: sync_ontology (Recommended)

**Purpose**: Complete ontology synchronization pipeline. Replaces all 5 previous tools.

### Parameters
- `ontology_path` (required): Path to ontology directory
- `preview` (optional): Dry-run mode
- `validation_level` (optional): minimal | standard | strict
- `parallel` (optional): Enable parallelization (default: true)

### Example
```json
{
  "tool": "sync_ontology",
  "arguments": {
    "ontology_path": "ontology/"
  }
}
```
```

#### CLAUDE.md
```markdown
## Code Generation Workflow (Updated)

### Simplified (Recommended)
```bash
cargo make sync   # Calls sync_ontology internally
```

### Advanced
```bash
# Preview changes
sync_ontology(ontology_path="ontology/", preview=true)

# Strict validation
sync_ontology(ontology_path="ontology/", validation_level="strict")
```
```

---

## Completion Checklist

### Core Implementation
- [ ] Stage 5: Execute SPARQL (HIGH)
- [ ] Stage 6: Validate results (MEDIUM)
- [ ] Stage 7: Render templates (HIGH)
- [ ] Stage 8: Validate syntax (MEDIUM)
- [ ] Stage 9: Format output (MEDIUM)
- [ ] Stage 10: Check compilation (LOW, strict mode)
- [ ] Stage 11: Detect TODOs (LOW)
- [ ] Complete stages 1-3 (MEDIUM)
- [ ] Wire all stages in pipeline.rs (HIGH)

### Testing
- [ ] Unit tests for each stage (13 tests)
- [ ] Integration test: basic pipeline
- [ ] Integration test: preview mode
- [ ] Integration test: strict mode
- [ ] Integration test: rollback on failure
- [ ] Performance benchmarks

### Documentation
- [ ] Update MCP_TOOL_USAGE.md
- [ ] Update CLAUDE.md
- [ ] Create migration guide
- [ ] Add inline code documentation

### Deployment
- [ ] Add compatibility shims for old tools
- [ ] Mark old tools as deprecated
- [ ] Update examples in README.md
- [ ] Update CI/CD pipeline

---

## Timeline Estimate

**80/20 Implementation** (Core 80% in 20% time):
- Week 1: Stages 5, 7 (execute SPARQL, render templates) → 40% complete
- Week 2: Stages 8, 9 (validate syntax, format output) → 70% complete
- Week 3: Integration, testing, documentation → 80% complete
- Week 4: Polish, edge cases, deployment → 100% complete

**Total**: 4 weeks for production-ready implementation.

---

**Quick Reference**: See `SYNC_ONTOLOGY_TRIZ_DESIGN.md` for comprehensive design.
**Version**: 1.0.0 | **Status**: Ready for implementation
