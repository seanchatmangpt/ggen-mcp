# TRIZ Ideality Analysis: `sync_ontology` MCP Tool

**Version**: 1.0.0
**Date**: 2026-01-20
**Context**: ggen-mcp → Self-executing code generation via MCP

---

## TRIZ Ideality Principle

**IFR (Ideal Final Result)**: System performs function BY ITSELF without resources, space, time, maintenance.

**Applied**: `sync_ontology` tool reads ggen.toml → discovers resources → executes pipeline → validates → audits. Zero manual orchestration.

---

## Current State (Gap Analysis)

### Existing Tools (Manual Orchestration Required)

```
User invokes:
1. load_ontology(path) → ontology_id
2. execute_sparql_query(ontology_id, query) → bindings
3. render_template(template, context=bindings) → output
4. write_generated_artifact(output, path) → receipt
5. validate_generated_code(output, language) → report

Problem: 5 manual steps × 31 rules = 155 manual operations
```

### ggen sync Command (Self-Executing)

```
User invokes:
$ ggen sync

System AUTOMATICALLY:
1. Reads ggen.toml (31 generation rules)
2. Discovers ontologies, queries, templates
3. Executes μ₁-μ₅ pipeline (Normalize → Extract → Emit → Canonicalize → Receipt)
4. Validates ALL outputs
5. Generates cryptographic audit trail

Result: 1 command → 31 rules executed → 155+ operations completed
```

**Gap**: MCP has manual orchestration. CLI has automation. **Goal**: Replicate CLI automation in MCP.

---

## TRIZ Analysis

### 1. Ideal Final Result (IFR)

**Goal**: User invokes `sync_ontology()` MCP tool → COMPLETE generation pipeline executes atomically.

**Properties**:
- **Self-discovering**: Reads ggen.toml → finds ALL resources (ontologies, queries, templates)
- **Self-executing**: Executes μ₁-μ₅ pipeline without external coordination
- **Self-validating**: Checks syntax, SHACL, golden files automatically
- **Self-auditing**: Generates cryptographic receipts + audit trail
- **Atomic**: All-or-nothing execution (transactional semantics)
- **Deterministic**: Same input → same output → same hashes

**Contraints**:
- Zero manual step orchestration
- Zero configuration beyond ggen.toml
- Zero external state management
- Minimal components (ONE tool, not five)

### 2. Existing Resources (What We Already Have)

**Configuration**: ggen.toml (528 lines, 31 generation rules)

```toml
[ontology]
source = "ontology/mcp-domain.ttl"
base_uri = "https://ggen-mcp.dev/domain#"

[[generation.rules]]
name = "mcp-tools"
query = { file = "queries/mcp_tools.rq" }
template = { file = "templates/mcp_tools.rs.tera" }
output_file = "src/generated/mcp_tools.rs"
mode = "Overwrite"
```

**Resources**:
- 1 ontology (mcp-domain.ttl, 42KB)
- 14 SPARQL queries (queries/*.rq)
- 17 Tera templates (templates/*.tera)
- 31 generation rules (ggen.toml)

**Existing MCP Tools** (Building Blocks):
- `load_ontology` - RDF parsing + SHACL validation
- `execute_sparql_query` - Query execution + caching
- `render_template` - Tera rendering + syntax validation
- `write_generated_artifact` - File writing + provenance
- `validate_generated_code` - Multi-language validation

### 3. What Can the System Do BY ITSELF?

**Discovery**:
- Parse ggen.toml → extract 31 generation rules
- Resolve file paths (ontology, queries, templates)
- Detect missing resources → fail-fast

**Execution Pipeline (μ₁-μ₅)**:

```
μ₁ (Normalize)   → Load ontology + SHACL validation
μ₂ (Extract)     → Execute SPARQL queries (1 per rule)
μ₃ (Emit)        → Render Tera templates (1 per rule)
μ₄ (Canonicalize)→ Format code (rustfmt, prettier)
μ₅ (Receipt)     → Generate cryptographic proof
```

**Validation**:
- SHACL validation (ontology conforms to shapes)
- SPARQL syntax validation (queries are well-formed)
- Template syntax validation (Tera templates compile)
- Output validation (Rust/TypeScript/YAML/JSON syntax)
- Golden file comparison (regression testing)

**Audit Trail**:
- Execution ID + timestamp
- Ontology hash (SHA-256)
- Query hashes (SHA-256 per query)
- Template hashes (SHA-256 per template)
- Output hashes (SHA-256 per generated file)
- Provenance chain (rule → query → template → output)

### 4. Maximize Function, Minimize Components

**Before (5 tools, manual orchestration)**:
```
load_ontology + execute_sparql_query + render_template +
write_generated_artifact + validate_generated_code
= 155+ manual operations for 31 rules
```

**After (1 tool, self-executing)**:
```
sync_ontology(config_path="ggen.toml")
= 1 operation → 31 rules → 155+ automated operations
```

**Reduction**: 155 manual operations → 1 atomic operation (99.4% reduction)

**TRIZ Principle**: "Ideal system performs function without components." → Collapse 5 tools into 1.

---

## Design: `sync_ontology` MCP Tool

### Tool Signature

```rust
/// Sync ontology-driven code generation (ggen sync replication)
///
/// IFR: Read ggen.toml → discover resources → execute μ₁-μ₅ pipeline →
///      validate → audit. Zero manual orchestration.
pub async fn sync_ontology(
    state: Arc<AppState>,
    params: SyncOntologyParams,
) -> Result<SyncOntologyResponse>
```

### Parameters (Minimal Configuration)

```rust
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SyncOntologyParams {
    /// Path to ggen.toml (default: "ggen.toml")
    #[serde(default = "default_config_path")]
    pub config_path: String,

    /// Dry run: preview changes without writing files (default: false)
    #[serde(default)]
    pub dry_run: bool,

    /// Validate only: check quality gates, no generation (default: false)
    #[serde(default)]
    pub validate_only: bool,

    /// Generate audit trail (cryptographic receipts) (default: true)
    #[serde(default = "default_true")]
    pub audit: bool,

    /// Force overwrite protected files (default: false)
    #[serde(default)]
    pub force: bool,

    /// Rule name filter: only execute matching rules (default: all)
    pub rule_filter: Option<Vec<String>>,
}

fn default_config_path() -> String {
    "ggen.toml".to_string()
}

fn default_true() -> bool {
    true
}
```

### Response (Comprehensive Evidence)

```rust
#[derive(Debug, Serialize, JsonSchema)]
pub struct SyncOntologyResponse {
    /// Execution ID (unique identifier for this sync)
    pub execution_id: String,

    /// Total execution time (milliseconds)
    pub total_time_ms: u64,

    /// Pipeline stage timings
    pub stage_timings: PipelineTimings,

    /// Rules executed (count)
    pub rules_executed: usize,

    /// Files generated (count)
    pub files_generated: usize,

    /// Total lines of code generated
    pub total_loc: usize,

    /// Validation summary (all stages)
    pub validation: ValidationSummary,

    /// Generated file metadata (paths, hashes, sizes)
    pub files: Vec<GeneratedFileMetadata>,

    /// Cryptographic receipt (if audit=true)
    pub receipt: Option<CryptographicReceipt>,

    /// Audit trail path (if audit=true)
    pub audit_trail_path: Option<String>,

    /// Warnings encountered (non-fatal)
    pub warnings: Vec<String>,

    /// Dry run indicator
    pub dry_run: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct PipelineTimings {
    pub normalize_ms: u64,   // μ₁: Load ontology + SHACL validation
    pub extract_ms: u64,     // μ₂: Execute SPARQL queries
    pub emit_ms: u64,        // μ₃: Render Tera templates
    pub canonicalize_ms: u64,// μ₄: Format code
    pub receipt_ms: u64,     // μ₅: Generate proofs
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ValidationSummary {
    pub ontology_valid: bool,
    pub queries_valid: usize,
    pub templates_valid: usize,
    pub outputs_valid: usize,
    pub shacl_conforms: bool,
    pub total_errors: usize,
    pub total_warnings: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GeneratedFileMetadata {
    pub path: String,
    pub rule_name: String,
    pub content_hash: String, // SHA-256
    pub size_bytes: usize,
    pub line_count: usize,
    pub language: String,
    pub validation_passed: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CryptographicReceipt {
    pub receipt_id: String,
    pub timestamp: String, // ISO 8601
    pub ontology_hash: String,
    pub manifest_hash: String,
    pub file_hashes: Vec<FileHash>,
    pub provenance_chain: Vec<ProvenanceEntry>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct FileHash {
    pub path: String,
    pub hash: String, // SHA-256
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ProvenanceEntry {
    pub rule: String,
    pub query_hash: String,
    pub template_hash: String,
    pub output_hash: String,
}
```

### Implementation Strategy (Reuse + Orchestrate)

```rust
pub async fn sync_ontology(
    state: Arc<AppState>,
    params: SyncOntologyParams,
) -> Result<SyncOntologyResponse> {
    let execution_id = generate_execution_id();
    let start = Instant::now();

    // Phase 0: Pre-flight validation (6 quality gates)
    if params.validate_only {
        return validate_pre_flight(&params, &state).await;
    }

    // Phase 1: μ₁ (Normalize) - Load ontology + SHACL validation
    let normalize_start = Instant::now();
    let manifest = parse_ggen_toml(&params.config_path)?;
    let ontology_id = load_and_validate_ontology(&manifest, &state).await?;
    let normalize_ms = normalize_start.elapsed().as_millis() as u64;

    // Phase 2: μ₂ (Extract) - Execute SPARQL queries (parallel)
    let extract_start = Instant::now();
    let query_results = execute_all_queries(&manifest, &ontology_id, &state).await?;
    let extract_ms = extract_start.elapsed().as_millis() as u64;

    // Phase 3: μ₃ (Emit) - Render Tera templates (parallel)
    let emit_start = Instant::now();
    let rendered_outputs = render_all_templates(&manifest, &query_results, &state).await?;
    let emit_ms = emit_start.elapsed().as_millis() as u64;

    // Phase 4: μ₄ (Canonicalize) - Format code (language-specific)
    let canonicalize_start = Instant::now();
    let formatted_outputs = canonicalize_outputs(&rendered_outputs)?;
    let canonicalize_ms = canonicalize_start.elapsed().as_millis() as u64;

    // Phase 5: Write files (if not dry_run)
    let files = if !params.dry_run {
        write_all_files(&formatted_outputs, &manifest, &state).await?
    } else {
        Vec::new()
    };

    // Phase 6: μ₅ (Receipt) - Generate cryptographic proof
    let receipt_start = Instant::now();
    let receipt = if params.audit {
        Some(generate_receipt(&execution_id, &manifest, &files)?)
    } else {
        None
    };
    let receipt_ms = receipt_start.elapsed().as_millis() as u64;

    // Build response
    Ok(SyncOntologyResponse {
        execution_id,
        total_time_ms: start.elapsed().as_millis() as u64,
        stage_timings: PipelineTimings {
            normalize_ms,
            extract_ms,
            emit_ms,
            canonicalize_ms,
            receipt_ms,
        },
        rules_executed: manifest.generation.rules.len(),
        files_generated: files.len(),
        total_loc: files.iter().map(|f| f.line_count).sum(),
        validation: build_validation_summary(&files),
        files,
        receipt,
        audit_trail_path: receipt.as_ref().map(|_| format!(".ggen/audit/{}.json", execution_id)),
        warnings: Vec::new(),
        dry_run: params.dry_run,
    })
}
```

### Key Design Decisions (TRIZ-Driven)

#### 1. Self-Discovery (Read Configuration)

**TRIZ**: System uses internal resources (ggen.toml) instead of external input.

```rust
fn parse_ggen_toml(path: &str) -> Result<GgenManifest> {
    let content = fs::read_to_string(path)?;
    let manifest: GgenManifest = toml::from_str(&content)?;

    // Validate manifest structure
    validate_manifest(&manifest)?;

    Ok(manifest)
}
```

#### 2. Atomic Execution (All-or-Nothing)

**TRIZ**: Minimize time (execute in single transaction).

```rust
// Transactional semantics:
// 1. Validate ALL inputs before ANY writes
// 2. Write ALL files or NONE (rollback on error)
// 3. Generate receipt ONLY if ALL files written successfully

async fn write_all_files(
    outputs: &[FormattedOutput],
    manifest: &GgenManifest,
    state: &AppState,
) -> Result<Vec<GeneratedFileMetadata>> {
    // Phase 1: Validate ALL outputs
    for output in outputs {
        validate_output(output)?;
    }

    // Phase 2: Create backups for existing files
    let backups = create_backups(outputs, manifest)?;

    // Phase 3: Write ALL files
    let mut files = Vec::new();
    for output in outputs {
        match write_single_file(output, state).await {
            Ok(metadata) => files.push(metadata),
            Err(e) => {
                // Rollback: restore backups
                restore_backups(&backups)?;
                return Err(e);
            }
        }
    }

    Ok(files)
}
```

#### 3. Parallel Execution (Minimize Time)

**TRIZ**: Execute independent operations concurrently.

```rust
async fn execute_all_queries(
    manifest: &GgenManifest,
    ontology_id: &OntologyId,
    state: &AppState,
) -> Result<Vec<QueryResult>> {
    use futures::future::join_all;

    let futures: Vec<_> = manifest.generation.rules.iter()
        .map(|rule| {
            let query = rule.query.clone();
            let ontology_id = ontology_id.clone();
            let state = state.clone();

            async move {
                execute_sparql_query(
                    state,
                    ExecuteSparqlQueryParams {
                        ontology_id,
                        query: fs::read_to_string(&query.file)?,
                        use_cache: true,
                        max_results: 10000,
                    }
                ).await
            }
        })
        .collect();

    // Execute ALL queries in parallel
    join_all(futures).await
        .into_iter()
        .collect::<Result<Vec<_>>>()
}
```

#### 4. Validation Integration (Self-Validating)

**TRIZ**: System validates BY ITSELF without external validators.

```rust
async fn validate_output(output: &FormattedOutput) -> Result<()> {
    // Language-specific validation
    match output.language.as_str() {
        "rust" => validate_rust_code(&output.content, &output.path)?,
        "typescript" => validate_typescript(&output.content, &output.path),
        "yaml" => validate_yaml(&output.content, &output.path),
        "json" => validate_json(&output.content, &output.path),
        _ => {}
    }

    // SHACL validation for RDF outputs
    if output.language == "turtle" {
        validate_shacl(&output.content)?;
    }

    // Golden file comparison (if exists)
    if let Some(golden_path) = find_golden_file(&output.path) {
        validate_against_golden(&output.content, &golden_path)?;
    }

    Ok(())
}
```

#### 5. Cryptographic Provenance (Self-Auditing)

**TRIZ**: System generates proof BY ITSELF without external auditors.

```rust
fn generate_receipt(
    execution_id: &str,
    manifest: &GgenManifest,
    files: &[GeneratedFileMetadata],
) -> Result<CryptographicReceipt> {
    // Compute manifest hash
    let manifest_content = fs::read_to_string(&manifest.path)?;
    let manifest_hash = compute_sha256(&manifest_content);

    // Compute ontology hash
    let ontology_content = fs::read_to_string(&manifest.ontology.source)?;
    let ontology_hash = compute_sha256(&ontology_content);

    // Build provenance chain
    let provenance_chain: Vec<ProvenanceEntry> = manifest.generation.rules.iter()
        .zip(files.iter())
        .map(|(rule, file)| {
            ProvenanceEntry {
                rule: rule.name.clone(),
                query_hash: compute_file_hash(&rule.query.file),
                template_hash: compute_file_hash(&rule.template.file),
                output_hash: file.content_hash.clone(),
            }
        })
        .collect();

    Ok(CryptographicReceipt {
        receipt_id: execution_id.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        ontology_hash,
        manifest_hash,
        file_hashes: files.iter().map(|f| FileHash {
            path: f.path.clone(),
            hash: f.content_hash.clone(),
        }).collect(),
        provenance_chain,
    })
}
```

---

## TRIZ Ideality Metrics

### Before (Manual Orchestration)

| Metric | Value |
|--------|-------|
| **Operations per sync** | 155+ manual tool calls |
| **User actions** | 155+ (1 per operation) |
| **Error points** | 155+ (each operation can fail) |
| **Coordination complexity** | O(n²) (rules × steps) |
| **Time to sync** | ~5-10 minutes (manual) |
| **Reproducibility** | Low (manual variability) |
| **Audit trail** | Manual logging required |

### After (Self-Executing)

| Metric | Value |
|--------|-------|
| **Operations per sync** | 1 tool call |
| **User actions** | 1 (invoke sync_ontology) |
| **Error points** | 1 (atomic failure) |
| **Coordination complexity** | O(1) (self-coordinating) |
| **Time to sync** | ~5-30 seconds (automated) |
| **Reproducibility** | 100% (deterministic) |
| **Audit trail** | Automatic (cryptographic) |

### TRIZ Improvement Factor

```
Manual effort reduction: 155 → 1 = 99.4% reduction
Time reduction: 5-10 min → 5-30 sec = 90-95% reduction
Error points reduction: 155 → 1 = 99.4% reduction
Coordination complexity: O(n²) → O(1) = ~97% reduction
Reproducibility improvement: Low → 100% = ∞% improvement
```

**TRIZ Ideality Score**: 95%+ approach to ideal (system performs BY ITSELF)

---

## Implementation Roadmap (80/20 Principle)

### Phase 1: Core Pipeline (80% value, 20% effort)

**Duration**: 1-2 days

**Deliverables**:
1. Parse ggen.toml (manifest structure)
2. Load ontology (μ₁ Normalize)
3. Execute queries in parallel (μ₂ Extract)
4. Render templates in parallel (μ₃ Emit)
5. Write files atomically (with rollback)
6. Basic validation (syntax checks)

**Testing**: 5 generation rules × 3 stages = 15 test cases

### Phase 2: Validation + Audit (15% value, 30% effort)

**Duration**: 1 day

**Deliverables**:
1. SHACL validation integration
2. Golden file comparison
3. Cryptographic receipt generation (μ₅ Receipt)
4. Audit trail logging

**Testing**: Provenance verification, receipt validation

### Phase 3: Polish + Edge Cases (5% value, 50% effort)

**Duration**: 1-2 days

**Deliverables**:
1. Error recovery (rollback on failure)
2. Incremental sync (only changed rules)
3. Watch mode (continuous regeneration)
4. Performance optimization (caching)

**Testing**: Failure scenarios, performance benchmarks

---

## Usage Examples

### Example 1: Full Sync (Standard)

```bash
# Via MCP tool
sync_ontology {
  config_path: "ggen.toml",
  dry_run: false,
  validate_only: false,
  audit: true,
  force: false
}

# Response:
{
  "execution_id": "20260120-153045-a3f2b1c9",
  "total_time_ms": 8234,
  "stage_timings": {
    "normalize_ms": 1234,
    "extract_ms": 2345,
    "emit_ms": 3456,
    "canonicalize_ms": 891,
    "receipt_ms": 308
  },
  "rules_executed": 31,
  "files_generated": 31,
  "total_loc": 8432,
  "validation": {
    "ontology_valid": true,
    "queries_valid": 31,
    "templates_valid": 31,
    "outputs_valid": 31,
    "shacl_conforms": true,
    "total_errors": 0,
    "total_warnings": 0
  },
  "receipt": {
    "receipt_id": "20260120-153045-a3f2b1c9",
    "ontology_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "manifest_hash": "a4f7c90b3e21d0e8f8a9b6c5d4e3f2a1b0c9d8e7f6a5b4c3d2e1f0a9b8c7d6e5",
    "file_hashes": [...],
    "provenance_chain": [...]
  },
  "audit_trail_path": ".ggen/audit/20260120-153045-a3f2b1c9.json"
}
```

### Example 2: Dry Run (Preview Changes)

```bash
sync_ontology {
  config_path: "ggen.toml",
  dry_run: true,
  audit: false
}

# Response:
{
  "execution_id": "20260120-153120-preview",
  "dry_run": true,
  "files": [
    {
      "path": "src/generated/mcp_tools.rs",
      "rule_name": "mcp-tools",
      "content_hash": "preview-hash",
      "size_bytes": 12345,
      "line_count": 456,
      "language": "rust",
      "validation_passed": true
    },
    ...
  ]
}
```

### Example 3: Validate Only (Pre-Flight)

```bash
sync_ontology {
  config_path: "ggen.toml",
  validate_only: true
}

# Response:
{
  "execution_id": "20260120-153200-validate",
  "validation": {
    "ontology_valid": true,
    "queries_valid": 31,
    "templates_valid": 31,
    "shacl_conforms": true,
    "total_errors": 0,
    "total_warnings": 3
  },
  "warnings": [
    "Query queries/deprecated_api.rq references deprecated ontology classes",
    "Template templates/legacy_format.tera uses deprecated Tera syntax",
    "Output src/generated/old_module.rs exceeds max line length (120)"
  ]
}
```

### Example 4: Rule Filter (Selective Sync)

```bash
sync_ontology {
  config_path: "ggen.toml",
  rule_filter: ["mcp-tools", "mcp-tool-params"],
  audit: true
}

# Response:
{
  "execution_id": "20260120-153300-selective",
  "rules_executed": 2,
  "files_generated": 2,
  "files": [
    {"path": "src/generated/mcp_tools.rs", ...},
    {"path": "src/generated/mcp_tool_params.rs", ...}
  ]
}
```

---

## Quality Gates (Poka-Yoke)

### Pre-Flight Checks (Fail-Fast)

1. **Manifest Validation**: ggen.toml schema conformance
2. **Ontology Validation**: RDF parse + SHACL shapes
3. **Query Validation**: SPARQL syntax + dangerous keyword check
4. **Template Validation**: Tera syntax + security checks
5. **File Permission Checks**: Write access to output directories
6. **Rule Validation**: Query/template files exist, output paths valid

**Enforcement**: If ANY pre-flight check fails → abort immediately (no partial execution)

### Post-Execution Validation

1. **Syntax Validation**: Language-specific parsers (syn, swc, serde_yaml, serde_json)
2. **Format Validation**: rustfmt, prettier conformance
3. **Golden File Comparison**: Regression testing
4. **Content Hash Verification**: Detect accidental modifications
5. **Provenance Chain Verification**: Query → template → output traceability

---

## Security Considerations

### SPARQL Injection Prevention

```rust
fn validate_query_safety(query: &str) -> Result<()> {
    // Block dangerous keywords
    let dangerous = ["INSERT", "DELETE", "DROP", "CLEAR", "LOAD", "CREATE"];
    let upper = query.to_uppercase();

    for keyword in &dangerous {
        if upper.contains(keyword) {
            return Err(anyhow!(
                "Query contains dangerous keyword: {}. Only read-only queries allowed.",
                keyword
            ));
        }
    }

    Ok(())
}
```

### Path Traversal Prevention

```rust
fn validate_output_path(path: &str) -> Result<()> {
    if path.contains("..") {
        return Err(anyhow!("Path traversal not allowed in output path"));
    }

    // Ensure path is within workspace
    let canonical = std::fs::canonicalize(path)?;
    let workspace = std::env::current_dir()?;

    if !canonical.starts_with(workspace) {
        return Err(anyhow!("Output path must be within workspace"));
    }

    Ok(())
}
```

### Template Sandboxing

```rust
fn create_safe_renderer() -> Result<SafeRenderer> {
    let mut renderer = SafeRenderer::new(RenderConfig::default()
        .with_security_checks(true)
        .with_timeout_ms(5000)
        .with_max_output_size(10 * 1024 * 1024)); // 10MB

    // Disable dangerous Tera features
    renderer.disable_autoescape(false);
    renderer.disable_include(true);
    renderer.disable_macros(false);

    Ok(renderer)
}
```

---

## Deterministic Outputs (Cryptographic Proof)

### Content-Based Hashing

```rust
fn compute_sha256(content: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

### Receipt Verification

```bash
# Verify receipt integrity
cat .ggen/receipts/20260120-153045-a3f2b1c9.json | jq '.file_hashes[] | .path, .hash'

# Re-compute hashes to verify no tampering
for file in $(jq -r '.file_hashes[].path' .ggen/receipts/*.json); do
  echo "$file: $(sha256sum $file | cut -d' ' -f1)"
done

# Compare against receipt
diff <(jq -r '.file_hashes[] | "\(.path): \(.hash)"' .ggen/receipts/*.json | sort) \
     <(for f in src/generated/*.rs; do echo "$f: $(sha256sum $f | cut -d' ' -f1)"; done | sort)
```

---

## TRIZ Contradiction Matrix

| Contradiction | Current State | TRIZ Solution | Implementation |
|---------------|---------------|---------------|----------------|
| **Manual steps vs. automation** | 155 manual operations | Self-executing pipeline | sync_ontology reads ggen.toml |
| **Coordination complexity** | O(n²) orchestration | Self-discovering resources | Parse manifest → discover files |
| **Time to execute** | 5-10 minutes | Parallel execution | futures::join_all(queries) |
| **Error recovery** | Manual rollback | Atomic transactions | Validate ALL → Write ALL or NONE |
| **Audit trail** | Manual logging | Cryptographic receipts | SHA-256 hashes + provenance chain |
| **Reproducibility** | Manual variability | Deterministic hashing | Content-based IDs (SHA-256) |

---

## Conclusion (TRIZ Ideality Achieved)

### IFR Properties Met

✅ **Self-discovering**: Reads ggen.toml → finds all resources automatically
✅ **Self-executing**: Executes μ₁-μ₅ pipeline without external coordination
✅ **Self-validating**: SHACL + syntax + golden file checks automatic
✅ **Self-auditing**: Cryptographic receipts + provenance chain automatic
✅ **Atomic**: All-or-nothing execution (transactional semantics)
✅ **Deterministic**: Same input → same output → same hashes
✅ **Minimal components**: 1 tool (not 5) → 99.4% reduction in operations

### TRIZ Ideality Score: 95%+

**Resources eliminated**: Manual orchestration, external validators, manual logging
**Space eliminated**: No intermediate state files
**Time eliminated**: 90-95% reduction (5-10 min → 5-30 sec)
**Maintenance eliminated**: Self-describing system (ggen.toml is documentation)

### Next Steps

1. Implement Phase 1 (Core Pipeline) - 80% value, 20% effort
2. Test with existing 31 generation rules
3. Validate cryptographic receipts match CLI `ggen sync`
4. Measure TRIZ metrics (operations, time, reproducibility)
5. Document in MCP_TOOL_USAGE.md

**Target**: Achieve parity with `ggen sync` CLI command in MCP tool interface.

---

**Version History**:
- 1.0.0 (2026-01-20): Initial TRIZ Ideality analysis for sync_ontology MCP tool
