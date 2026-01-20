# MCP Server Replication Strategy: ggen OpenAPI Example

**Date**: 2026-01-20
**Version**: 1.0.0
**Research**: 10-Agent Parallel Exploration
**Validation**: 80/20 Principles (SPR, TPS, Chicago-TDD)

---

## Executive Summary (SPR)

**Pattern**: RDF Ontology → SPARQL SELECT → Tera Templates → Multi-format outputs (OpenAPI YAML + Zod schemas + TypeScript types).

**Core Architecture**: MCP server exposes generation pipeline as atomic tools with validation gates, caching, and deterministic receipts. LLM orchestrates: `load_ontology` → `execute_query` → `render_template` → `validate_output` → `write_artifact`.

**Key Insight**: ggen's 13-rule generation workflow replicable as MCP tools providing granular control, preview mode, selective generation, and error recovery—advantages over single atomic commands.

---

## 1. Research Methodology

### 10-Agent Parallel Exploration

1. **MCP Server Architecture** - Tool registration, routing, state management
2. **RDF/Ontology Integration** - Oxigraph, Turtle parsing, validation
3. **SPARQL Query Execution** - Safety, performance, caching
4. **Template Rendering** - Tera integration, context building, output validation
5. **Code Generation Tools** - Multi-language output, quality gates
6. **Validation & Testing** - 4-layer Poka-Yoke, golden files, Chicago-TDD
7. **File System & Resources** - Path safety, caching, resource exposure
8. **Configuration Management** - ggen.toml structure, environment overrides
9. **Multi-Step Workflows** - Fork/checkpoint patterns, recovery strategies
10. **BFF Integration** - Next.js/Express patterns, type safety, API boundaries

**Total Research**: 97 test files, 34.8K LOC test infrastructure, 226+ test cases, 23 SPARQL queries analyzed, 11 SPARQL modules (6,631 LOC), 13 Tera templates, 528-line ggen.toml configuration.

---

## 2. The ggen OpenAPI Pattern (Current Implementation)

### Input: RDF Ontology (blog-api.ttl)

```turtle
@prefix blog: <https://ggen.io/examples/blog#> .
@prefix api: <https://ggen.io/ontology/api#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

blog:User a api:Entity ;
    api:name "User" ;
    rdfs:comment "Blog user account" ;
    api:hasProperty blog:User_id, blog:User_email, blog:User_username .

blog:User_email a api:Property ;
    api:name "email" ;
    api:type "string" ;
    api:format "email" ;
    api:required "true" ;
    api:minLength "5" ;
    api:maxLength "255" .
```

### Transformation: 13 Generation Rules

| Rule | Output | Purpose |
|------|--------|---------|
| 1 | `lib/openapi/api-info.yaml` | OpenAPI spec header |
| 2 | `lib/openapi/schemas.yaml` | Component schemas |
| 3 | `lib/openapi/paths.yaml` | Endpoint definitions |
| 4 | `lib/openapi/openapi.yaml` | Combined spec |
| 5 | `lib/types/entities.mjs` | JSDoc type definitions |
| 6 | `lib/types/requests.mjs` | Request type definitions |
| 7 | `lib/schemas/entities.mjs` | Zod entity schemas |
| 8 | `lib/schemas/requests.mjs` | Zod request schemas |
| 9 | `lib/guards/entities.mjs` | Runtime type guards |
| 10-13 | `lib/**/index.mjs` | Barrel exports |

### Output: Multi-Format Code

**Zod Schema** (lib/schemas/entities.mjs):
```javascript
import { z } from 'zod';

export const userSchema = z.object({
  id: z.string().min(1),
  email: z.string().email("Must be a valid email address"),
  username: z.string().min(1).max(255),
});
```

**TypeScript Types** (lib/types/entities.mjs):
```javascript
/**
 * User - Blog user account
 * @typedef {Object} User
 * @property {string} id
 * @property {string} email
 * @property {string} username
 */
```

**OpenAPI Spec** (lib/openapi/openapi.yaml):
```yaml
/users:
  get:
    summary: List all users
    responses:
      '200':
        content:
          application/json:
            schema:
              type: array
              items:
                $ref: '#/components/schemas/User'
```

---

## 3. MCP Server Architecture for Replication

### Tool Category: "Ontology Code Generation"

#### Tool 1: `load_ontology`

**Purpose**: Load RDF/Turtle ontology into memory with validation

**Input Schema**:
```typescript
{
  ontology_path: string,          // Path to .ttl file
  validate_shacl: boolean,        // SHACL shape validation (default: true)
  cache_ttl: number,              // Cache duration in seconds (default: 3600)
  imports: string[]               // Additional ontology imports (optional)
}
```

**Output Schema**:
```typescript
{
  ontology_id: string,            // Unique handle for subsequent queries
  entity_count: number,           // Number of api:Entity instances
  property_count: number,         // Number of api:Property instances
  validation_report: {
    valid: boolean,
    errors: string[],
    warnings: string[]
  }
}
```

**Implementation Pattern** (src/tools/ontology_generation.rs):
```rust
pub async fn load_ontology(
    state: Arc<AppState>,
    params: LoadOntologyParams,
) -> Result<LoadOntologyResponse> {
    // 1. Path safety validation
    validate_path_safe(&params.ontology_path)?;
    validate_path_within_workspace(state.config(), &params.ontology_path)?;

    // 2. Load into Oxigraph store
    let store = Store::new()?;
    let content = tokio::fs::read_to_string(&params.ontology_path).await
        .context("Failed to read ontology file")?;

    store.load_from_reader(
        oxigraph::io::RdfFormat::Turtle,
        content.as_bytes()
    )?;

    // 3. Load imports if specified
    for import in params.imports.unwrap_or_default() {
        let import_content = tokio::fs::read_to_string(&import).await?;
        store.load_from_reader(RdfFormat::Turtle, import_content.as_bytes())?;
    }

    // 4. Run consistency checks
    let checker = ConsistencyChecker::new(store.clone());
    let report = checker.check_all();

    if !report.valid {
        return Err(Error::ConsistencyCheckFailed { errors: report.errors });
    }

    // 5. SHACL validation
    if params.validate_shacl.unwrap_or(true) {
        let validator = ShapeValidator::from_file("ontology/shapes.ttl")?;
        let shacl_report = validator.validate_graph(&store)?;

        if !shacl_report.conforms() {
            return Err(Error::ShapeValidationFailed {
                violations: shacl_report.violations()
            });
        }
    }

    // 6. Cache ontology with TTL
    let ontology_id = generate_ontology_id();
    let ttl = Duration::from_secs(params.cache_ttl.unwrap_or(3600));
    state.cache_ontology(ontology_id.clone(), store.clone(), ttl);

    // 7. Count entities and properties
    let entity_count = count_entities(&store)?;
    let property_count = count_properties(&store)?;

    Ok(LoadOntologyResponse {
        ontology_id,
        entity_count,
        property_count,
        validation_report: ValidationReport {
            valid: true,
            errors: vec![],
            warnings: report.warnings,
        },
    })
}

fn count_entities(store: &Store) -> Result<usize> {
    let query = "SELECT (COUNT(?entity) AS ?count) WHERE { ?entity a api:Entity }";
    let results = store.query(query)?;
    // Extract count from results
    Ok(count)
}
```

#### Tool 2: `execute_sparql_query`

**Purpose**: Execute SPARQL SELECT query with safety checks and performance budgets

**Input Schema**:
```typescript
{
  ontology_id: string,            // From load_ontology response
  query: string,                  // SPARQL SELECT query
  limit: number,                  // Max results (default: 1000)
  cache: boolean,                 // Cache results (default: true)
  timeout_ms: number              // Query timeout (default: 30000)
}
```

**Output Schema**:
```typescript
{
  variable_names: string[],       // SPARQL variables selected
  results: object[],              // Array of variable bindings
  execution_time_ms: number,      // Query execution duration
  result_count: number,           // Number of results returned
  cache_hit: boolean              // Whether result from cache
}
```

**Implementation with Security**:
```rust
pub async fn execute_sparql_query(
    state: Arc<AppState>,
    params: ExecuteSparqlQueryParams,
) -> Result<SparqlQueryResponse> {
    // 1. Retrieve ontology from cache
    let store = state.get_ontology(&params.ontology_id)
        .ok_or_else(|| Error::OntologyNotFound { id: params.ontology_id.clone() })?;

    // 2. SPARQL injection prevention
    let sanitizer = SparqlSanitizer::new();
    sanitizer.validate_query(&params.query)
        .context("Query failed security validation")?;

    // 3. Query complexity analysis
    let analyzer = QueryAnalyzer::new();
    let complexity = analyzer.analyze(&params.query)?;

    if complexity.level == PerformanceLevel::Critical {
        return Err(Error::QueryTooComplex {
            complexity: complexity.score,
            threshold: 20.0,
        });
    }

    // 4. Performance budget validation
    let budget = PerformanceBudget::default();
    budget.validate_query(&complexity)?;

    // 5. Check cache first
    let cache_key = compute_query_cache_key(&params.ontology_id, &params.query);
    if params.cache.unwrap_or(true) {
        if let Some(cached) = state.query_cache.get(&cache_key) {
            if !cached.is_expired() {
                return Ok(cached.response.clone());
            }
        }
    }

    // 6. Execute with profiling and timeout
    let mut profiler = QueryProfiler::new(&cache_key);
    profiler.start();

    let timeout_duration = Duration::from_millis(
        params.timeout_ms.unwrap_or(30000)
    );

    let solutions = tokio::time::timeout(
        timeout_duration,
        tokio::task::spawn_blocking(move || {
            store.query(&params.query)
        })
    ).await??;

    profiler.record_result_size(solutions.len());
    let metrics = profiler.finish();

    // 7. Validate execution against budget
    budget.validate_execution(&metrics)?;

    // 8. Map to JSON-serializable format
    let limit = params.limit.unwrap_or(1000);
    let results: Vec<HashMap<String, serde_json::Value>> = solutions
        .into_iter()
        .take(limit)
        .map(|sol| {
            let binding = TypedBinding::new(&sol);
            binding.to_map()
        })
        .collect();

    let response = SparqlQueryResponse {
        variable_names: extract_variable_names(&params.query),
        results,
        execution_time_ms: metrics.duration_ms,
        result_count: results.len(),
        cache_hit: false,
    };

    // 9. Cache result
    if params.cache.unwrap_or(true) {
        state.query_cache.insert(cache_key, response.clone(), ttl);
    }

    Ok(response)
}
```

#### Tool 3: `render_template`

**Purpose**: Render Tera template with SPARQL results and validate output

**Input Schema**:
```typescript
{
  template_path: string,          // Path to .tera template
  context: object,                // Template variables (includes sparql_results)
  output_format: string,          // rust | typescript | yaml | json
  preview: boolean                // If true, don't write file (default: false)
}
```

**Output Schema**:
```typescript
{
  rendered_code: string,          // Generated code
  byte_size: number,              // Output size in bytes
  line_count: number,             // Number of lines
  validation_report: {
    valid: boolean,
    syntax_errors: string[],
    warnings: string[],
    suggestions: string[]
  }
}
```

**Implementation**:
```rust
pub async fn render_template(
    state: Arc<AppState>,
    params: RenderTemplateParams,
) -> Result<RenderTemplateResponse> {
    // 1. Path safety validation
    validate_path_safe(&params.template_path)?;

    // 2. Load template file
    let template_content = tokio::fs::read_to_string(&params.template_path).await
        .context("Failed to read template file")?;

    // 3. Validate template syntax
    let validator = TemplateValidator::new();
    validator.validate_syntax(&template_content)
        .context("Template syntax validation failed")?;

    // 4. Build template context
    let mut context = TemplateContext::new(&params.template_path);

    // Insert all context variables
    for (key, value) in params.context {
        context.insert(&key, value)?;
    }

    // Validate context against schema
    context.validate()?;

    // 5. Render with safety guards
    let renderer = SafeRenderer::new();
    let output = renderer.render_safe(
        &params.template_path,
        context.to_tera_context(),
    )?;

    // 6. Validate output based on format
    let output_validator = OutputValidator::new();
    let validation_report = match params.output_format.as_str() {
        "rust" => output_validator.validate_rust_syntax(&output)?,
        "typescript" => output_validator.validate_typescript_syntax(&output)?,
        "yaml" => output_validator.validate_yaml_syntax(&output)?,
        "json" => output_validator.validate_json_syntax(&output)?,
        _ => output_validator.validate_generic(&output)?,
    };

    // 7. Security checks
    let security_report = output_validator.check_security_patterns(&output)?;

    Ok(RenderTemplateResponse {
        rendered_code: output.clone(),
        byte_size: output.len(),
        line_count: output.lines().count(),
        validation_report: ValidationReport {
            valid: validation_report.is_valid(),
            syntax_errors: validation_report.errors,
            warnings: validation_report.warnings
                .into_iter()
                .chain(security_report.warnings)
                .collect(),
            suggestions: validation_report.suggestions,
        },
    })
}
```

#### Tool 4: `write_generated_artifact`

**Purpose**: Write generated code with backup, rollback, and provenance tracking

**Input Schema**:
```typescript
{
  output_path: string,            // Target file path
  content: string,                // Generated code to write
  create_backup: boolean,         // Backup existing file (default: true)
  validate_syntax: boolean        // Validate before writing (default: true)
}
```

**Output Schema**:
```typescript
{
  written_to: string,             // Absolute path of written file
  backup_path: string | null,     // Backup file path if created
  content_hash: string,           // SHA-256 hash of content
  receipt_id: string              // Provenance receipt ID
}
```

**Implementation with RAII Guards**:
```rust
pub async fn write_generated_artifact(
    state: Arc<AppState>,
    params: WriteGeneratedArtifactParams,
) -> Result<WriteGeneratedArtifactResponse> {
    // 1. Path safety validation
    validate_path_safe(&params.output_path)?;
    validate_path_within_workspace(state.config(), &params.output_path)?;

    // 2. Syntax validation before writing
    if params.validate_syntax.unwrap_or(true) {
        let format = infer_format_from_extension(&params.output_path);
        let validator = OutputValidator::new();
        validator.validate(&params.content, &format)?;
    }

    // 3. Create backup if file exists
    let backup_path = if params.output_path.exists() && params.create_backup.unwrap_or(true) {
        let backup = create_backup(&params.output_path)?;
        Some(backup)
    } else {
        None
    };

    // 4. RAII guard for rollback on error
    let _rollback_guard = BackupGuard::new(backup_path.clone());

    // 5. Atomic write operation
    let writer = SafeCodeWriter::new();
    writer.atomic_write(&params.output_path, &params.content)?;

    // 6. Compute content hash
    let content_hash = compute_string_hash(&params.content);

    // 7. Generate provenance receipt
    let receipt = GenerationReceipt {
        receipt_id: generate_receipt_id(&content_hash),
        timestamp: Utc::now().timestamp() as u64,
        artifact_hash: content_hash.clone(),
        output_path: params.output_path.to_string_lossy().to_string(),
        generation_metadata: HashMap::new(),
    };

    // 8. Store receipt for audit trail
    state.artifact_tracker.record_artifact(&receipt)?;

    // Commit (prevent rollback)
    _rollback_guard.commit();

    Ok(WriteGeneratedArtifactResponse {
        written_to: params.output_path.canonicalize()?.to_string_lossy().to_string(),
        backup_path: backup_path.map(|p| p.to_string_lossy().to_string()),
        content_hash,
        receipt_id: receipt.receipt_id,
    })
}
```

#### Tool 5: `validate_generated_code`

**Purpose**: Comprehensive validation of generated code

**Input Schema**:
```typescript
{
  file_path: string,              // Path to generated file
  language: string,               // rust | typescript | yaml | json
  strict: boolean,                // Fail on warnings (default: false)
  golden_file: string | null      // Compare against golden file (optional)
}
```

**Output Schema**:
```typescript
{
  valid: boolean,
  errors: {
    type: string,
    message: string,
    line: number | null
  }[],
  warnings: string[],
  suggestions: string[],
  golden_file_diff: {
    has_changes: boolean,
    additions: number,
    deletions: number,
    diff_summary: string
  } | null
}
```

---

## 4. Workflow Orchestration Pattern (LLM-Driven)

### Current ggen: Single Atomic Command

```bash
cd ggen/examples/openapi
ggen sync  # Runs all 13 rules in sequence
```

**Characteristics**:
- All-or-nothing execution
- No preview capability
- No selective generation
- Single error stops entire pipeline
- No incremental updates

### MCP Server: Multi-Step Orchestration

```typescript
// Claude orchestrates this workflow with full control:

// Step 1: Load ontology with validation
const { ontology_id, validation_report } = await mcp.load_ontology({
  ontology_path: "ontology/blog-api.ttl",
  validate_shacl: true,
  cache_ttl: 3600,
});

if (!validation_report.valid) {
  console.error("Ontology validation failed:", validation_report.errors);
  return;
}

// Step 2: Execute SPARQL query for entities
const entityQuery = `
  PREFIX api: <https://ggen.io/ontology/api#>
  PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

  SELECT ?entityName ?entityDescription ?propertyName ?propertyType ?required
  WHERE {
    ?entity a api:Entity ;
            api:name ?entityName .
    OPTIONAL { ?entity rdfs:comment ?entityDescription }
    ?entity api:hasProperty ?property .
    ?property api:name ?propertyName ;
              api:type ?propertyType .
    OPTIONAL { ?property api:required ?required }
  }
  ORDER BY ?entityName ?propertyName
`;

const entities = await mcp.execute_sparql_query({
  ontology_id,
  query: entityQuery,
  limit: 5000,
  cache: true,
});

console.log(`Found ${entities.result_count} entity-property pairs`);

// Step 3: Render Zod schemas (PREVIEW MODE)
const zodPreview = await mcp.render_template({
  template_path: "templates/zod-schemas.tera",
  context: {
    sparql_results: entities.results,
  },
  output_format: "typescript",
  preview: true,  // Don't write yet
});

console.log("Preview of Zod schemas:");
console.log(zodPreview.rendered_code.substring(0, 500));

if (!zodPreview.validation_report.valid) {
  console.error("Validation errors:", zodPreview.validation_report.syntax_errors);
  return;
}

// Step 4: Write artifact (only if preview looks good)
const written = await mcp.write_generated_artifact({
  output_path: "lib/schemas/entities.mjs",
  content: zodPreview.rendered_code,
  create_backup: true,
  validate_syntax: true,
});

console.log(`Written to: ${written.written_to}`);
console.log(`Content hash: ${written.content_hash}`);

// Step 5: Validate generated code
const validation = await mcp.validate_generated_code({
  file_path: written.written_to,
  language: "typescript",
  strict: false,
  golden_file: "tests/golden/lib/schemas/entities.mjs",
});

if (!validation.valid) {
  console.error("Generated code validation failed:");
  validation.errors.forEach(err => console.error(`  - ${err.message}`));
}

if (validation.golden_file_diff && validation.golden_file_diff.has_changes) {
  console.warn(`Golden file diff: +${validation.golden_file_diff.additions} -${validation.golden_file_diff.deletions}`);
}

// Step 6: Repeat for other templates (OpenAPI, TypeScript types, etc.)
// Each step can be previewed, validated, and selectively executed
```

### Advantages Over Single Atomic Command

| Feature | ggen sync | MCP Server Tools |
|---------|-----------|------------------|
| **Preview Mode** | ❌ No | ✅ Yes (preview before writing) |
| **Selective Generation** | ❌ All or nothing | ✅ Choose specific artifacts |
| **Error Recovery** | ❌ Restart from scratch | ✅ Retry individual steps |
| **Incremental Updates** | ❌ Regenerate all | ✅ Update only changed templates |
| **Cross-Language** | ✅ Yes | ✅ Yes (same flexibility) |
| **Caching** | ⚠️ Limited | ✅ Multi-layer (ontology, query, results) |
| **Validation Control** | ⚠️ Fixed | ✅ Configurable (strict/permissive) |
| **Golden File Comparison** | ❌ Manual | ✅ Built-in tool |
| **LLM Visibility** | ❌ Black box | ✅ Full transparency |

---

## 5. Validation Strategy (4-Layer Poka-Yoke)

### Layer 1: Input Validation (Pre-Execution)

**Goal**: Prevent invalid inputs from reaching execution stage

```rust
// Path safety
validate_path_safe(ontology_path)?;
validate_path_within_workspace(config, path)?;

// Template syntax
validate_template_syntax(template)?;

// SPARQL query security
SparqlSanitizer::validate_query(query)?;
SparqlSanitizer::prevent_comment_injection(query)?;
SparqlSanitizer::prevent_union_injection(query)?;

// Parameter validation
validate_non_empty_string("ontology_path", path)?;
validate_numeric_range("cache_ttl", ttl, 1, 86400)?;
```

### Layer 2: Execution Validation (During Generation)

**Goal**: Enforce performance budgets and resource limits

```rust
// Query complexity analysis
let complexity = QueryAnalyzer::analyze(query)?;
if complexity.level == PerformanceLevel::Critical {
    return Err(Error::QueryTooComplex);
}

// Performance budget enforcement
let budget = PerformanceBudget::default();
budget.validate_query(&complexity)?;

// Timeout wrapper
let result = tokio::time::timeout(
    Duration::from_secs(30),
    render_template(context)
).await?;

// Memory limits
SafeRenderer::validate_output_size(output, max_bytes)?;
```

### Layer 3: Output Validation (Post-Generation)

**Goal**: Ensure generated code is syntactically correct and follows conventions

```rust
// Syntax checking
match language {
    "rust" => {
        syn::parse_file(&code)?;  // Rust AST parsing
    },
    "typescript" => {
        validate_typescript_syntax(&code)?;
    },
    "yaml" => {
        serde_yaml::from_str::<Value>(&code)?;
    },
}

// Naming conventions
validate_naming_conventions(&code, language)?;

// Security patterns
check_for_unsafe_blocks(&code)?;
check_for_sql_injection_patterns(&code)?;

// Structural validation
validate_balanced_delimiters(&code)?;
validate_no_empty_structs(&code)?;
```

### Layer 4: Regression Testing (Golden File Comparison)

**Goal**: Detect unintended changes in generated output

```rust
// Compare against expected output
let golden_path = format!("tests/golden/{}", output_path);
if golden_path.exists() {
    let golden_content = fs::read_to_string(&golden_path)?;
    let diff = compute_diff(&golden_content, &generated_content);

    if diff.has_changes() {
        if env::var("UPDATE_GOLDEN").is_ok() {
            // Update golden file
            fs::write(&golden_path, &generated_content)?;
        } else {
            // Return diff for review
            return Err(Error::GoldenFileMismatch {
                diff_summary: diff.summary(),
                additions: diff.additions,
                deletions: diff.deletions,
            });
        }
    }
}
```

---

## 6. Implementation Roadmap

### Phase 1: Core Tools (Week 1)

**Deliverables**:
- [ ] Create `src/tools/ontology_generation.rs` module
- [ ] Implement `load_ontology` tool with Oxigraph integration
- [ ] Implement `execute_sparql_query` with safety checks
- [ ] Implement `render_template` with Tera + validation
- [ ] Implement `write_generated_artifact` with backup/rollback
- [ ] Register 5 tools in MCP server router

**Acceptance Criteria**:
- All tools compile without warnings
- Unit tests pass (80%+ coverage)
- Integration test: Load ontology → Query → Render → Write

### Phase 2: Validation Layer (Week 2)

**Deliverables**:
- [ ] Extend `GeneratedCodeValidator` for TypeScript/YAML/JSON
- [ ] Implement `validate_generated_code` tool
- [ ] Create golden file comparison functionality
- [ ] Add SHACL shape validation to `load_ontology`
- [ ] Implement pre-flight validation checks

**Acceptance Criteria**:
- Validator detects syntax errors in all supported languages
- Golden file diffs computed accurately
- SHACL violations prevent ontology loading

### Phase 3: Caching & Performance (Week 3)

**Deliverables**:
- [ ] Implement ontology caching (LRU with TTL)
- [ ] Implement SPARQL query result caching
- [ ] Add performance profiling metrics
- [ ] Optimize template rendering (parallel execution)
- [ ] Add cache statistics tool

**Acceptance Criteria**:
- Cache hit ratio > 80% for repeated queries
- Query execution time < 100ms for cached results
- Memory usage stays within bounds (configurable limits)

### Phase 4: Example Integration (Week 4)

**Deliverables**:
- [ ] Port ggen/examples/openapi ontology to MCP server workspace
- [ ] Convert 13 SPARQL queries to MCP tool calls
- [ ] Adapt Tera templates for MCP server usage
- [ ] Create end-to-end test: Ontology → 13 outputs
- [ ] Document workflow examples

**Acceptance Criteria**:
- Generated outputs match ggen sync outputs (byte-for-byte)
- All 13 templates render successfully
- Golden file tests pass

### Phase 5: Documentation & Testing (Week 5)

**Deliverables**:
- [ ] Document MCP tool usage (README)
- [ ] Create workflow examples (OpenAPI, Zod, TypeScript)
- [ ] Write integration tests (Chicago-TDD style)
- [ ] Create golden file test suite (snapshots)
- [ ] Performance benchmarks

**Acceptance Criteria**:
- Documentation covers all 5 tools
- 3+ workflow examples provided
- Test coverage > 80%
- All benchmarks meet performance targets

---

## 7. Key Files to Create

### Source Code
```
src/tools/ontology_generation.rs          (NEW) - MCP tools implementation
src/ontology/cache.rs                     (NEW) - Ontology caching layer
src/template/multi_format_validator.rs    (NEW) - Multi-language validation
src/sparql/query_cache.rs                 (NEW) - Query result caching
```

### Configuration & Queries
```
queries/examples/blog_api_entities.rq     (NEW) - Entity extraction query
queries/examples/blog_api_endpoints.rq    (NEW) - Endpoint extraction query
queries/examples/blog_api_schemas.rq      (NEW) - Schema extraction query
```

### Templates
```
templates/examples/zod-schemas.tera       (NEW) - Zod schema generation
templates/examples/typescript-types.tera  (NEW) - TypeScript type definitions
templates/examples/openapi-spec.tera      (NEW) - OpenAPI YAML generation
```

### Test Fixtures
```
tests/golden/lib/schemas/entities.mjs     (NEW) - Expected Zod output
tests/golden/lib/types/entities.mjs       (NEW) - Expected TypeScript output
tests/golden/lib/openapi/openapi.yaml     (NEW) - Expected OpenAPI output
tests/ontology/blog-api.ttl               (NEW) - Test ontology
```

### Documentation
```
docs/MCP_TOOL_USAGE.md                    (NEW) - Tool documentation
docs/WORKFLOW_EXAMPLES.md                 (NEW) - End-to-end examples
docs/VALIDATION_GUIDE.md                  (NEW) - Validation patterns
```

---

## 8. Validation Against ggen-mcp 80/20 Principles

### TPS (Toyota Production System) Alignment

| Principle | Implementation | Validation |
|-----------|----------------|------------|
| **Jidoka** | Compile-time prevention (NewTypes), fail-fast validation | ✅ Type-safe parameters, SPARQL injection prevention |
| **Andon Cord** | Tests pass or stop, validation gates block progress | ✅ 4-layer validation, golden file checks |
| **Poka-Yoke** | Error-proofing via input guards, type safety | ✅ Path safety, SPARQL sanitization, template validation |
| **Kaizen** | Metrics tracked, decisions documented, incremental improvement | ✅ Performance profiling, audit trail, caching metrics |
| **Single Piece Flow** | One artifact per tool call, fast feedback | ✅ Atomic tools, preview mode, selective generation |

### SPR (Sparse Priming Representation) Compliance

- ✅ **Distilled**: Essential concepts only (load → query → render → write)
- ✅ **Associated**: Linked patterns (ontology → SPARQL → Tera → code)
- ✅ **Compressed**: Maximum concept density (SPR used throughout)
- ✅ **Activated**: Latent space efficiently primed
- ✅ **Verified**: Self-check mandatory before response

### Chicago-Style TDD Patterns

- ✅ **State-Based Testing**: Test object state changes, not call sequences
- ✅ **Real Implementations**: Use actual Oxigraph, Tera, validators (no mocks)
- ✅ **Integration-Focused**: Test component interactions end-to-end
- ✅ **Minimal Mocking**: Only mock external services (network, LibreOffice)

### Deterministic Code Generation

- ✅ **Same Input → Same Output**: SHA-256 receipts verify reproducibility
- ✅ **Provenance Tracking**: Generation receipts with timestamps + hashes
- ✅ **Ordered Collections**: BTreeMap ensures deterministic serialization
- ✅ **Content Hashing**: Every artifact tracked with SHA-256

---

## 9. Research Agent Findings Summary

### Agent 1: MCP Server Architecture
- **Key Finding**: Decorator composition pattern (multiple tool routers merged at runtime)
- **Pattern**: Feature-gating (VBA, recalc tools conditional)
- **Insight**: Tool timeout enforcement via global wrapper

### Agent 2: RDF/Ontology Integration
- **Key Finding**: Three-layer validation (integrity → consistency → SHACL)
- **Pattern**: Oxigraph in-memory store with SPARQL 1.1 support
- **Insight**: Validation prevents generation of code with defects

### Agent 3: SPARQL Query Execution
- **Key Finding**: 11 SPARQL modules (6,631 LOC) with 5-layer injection prevention
- **Pattern**: QueryAnalyzer + PerformanceBudget + SlowQueryDetector
- **Insight**: 90 tests cover attack scenarios

### Agent 4: Template Rendering
- **Key Finding**: SPARQL results → Tera context → Multi-format code
- **Pattern**: Type-safe parameter validation (3-tier: types + rules + schema)
- **Insight**: Rate-limited filters prevent DOS attacks

### Agent 5: Code Generation Tools
- **Key Finding**: Five-stage pipeline (Normalize → Extract → Emit → Canonicalize → Receipt)
- **Pattern**: Quality gates at each stage (Poka-Yoke)
- **Insight**: GenerationReceipt provides cryptographic proof

### Agent 6: Validation & Testing
- **Key Finding**: 97 test files, 34.8K LOC test infrastructure, 11 harnesses
- **Pattern**: Chicago-TDD (state-based, real implementations)
- **Insight**: Snapshot testing with golden files + SHA-256 hashes

### Agent 7: File System & Resources
- **Key Finding**: LRU cache with RwLock, RAII guards for cleanup
- **Pattern**: Path containment validation prevents traversal attacks
- **Insight**: Resource lifecycle managed via guards (TempFileGuard, ForkCreationGuard)

### Agent 8: Configuration Management
- **Key Finding**: Three-layer config (CLI > File > Env > Defaults)
- **Pattern**: ggen.toml with 13+ generation rules, BTreeMap for determinism
- **Insight**: Environment profiles (dev/ci/prod) in single file

### Agent 9: Multi-Step Workflows
- **Key Finding**: Fork → Checkpoint → Edit → Recalc → Verify → Save
- **Pattern**: Optimistic locking (version tracking), circuit breaker, exponential backoff
- **Insight**: Audit trail with event correlation via span IDs

### Agent 10: BFF Integration
- **Key Finding**: Ontology → OpenAPI + Zod + TypeScript (single source of truth)
- **Pattern**: 4-quadrant validation (Input → Business → Output → Cross-boundary)
- **Insight**: Type safety across Rust/TypeScript/Python from same ontology

---

## 10. Conclusion

An MCP server can successfully replicate ggen's OpenAPI example workflow by exposing the generation pipeline as **5 atomic tools**:

1. `load_ontology` - Load and validate RDF/Turtle ontologies
2. `execute_sparql_query` - Execute queries with safety + performance budgets
3. `render_template` - Render Tera templates with validation
4. `write_generated_artifact` - Write code with backup/rollback
5. `validate_generated_code` - Comprehensive validation + golden file comparison

### Key Advantages

**Granular Control**: LLM orchestrates multi-step workflows with preview mode, selective generation, and error recovery.

**Validation at Every Layer**: Input guards → execution budgets → output validation → golden file comparison.

**Deterministic & Auditable**: SHA-256 receipts, provenance tracking, reproducible builds.

**Type-Safe & Secure**: SPARQL injection prevention, path safety, resource limits.

### Next Steps

1. **Create implementation branch**: `claude/mcp-ontology-generation-tools-{session-id}`
2. **Phase 1 implementation**: Core 5 tools (Week 1)
3. **Integration testing**: Replicate ggen/examples/openapi (Week 4)
4. **Documentation**: Tool usage guide + workflow examples (Week 5)

---

**Research Complete**: 2026-01-20
**Agents Deployed**: 10 parallel explorers
**Total Research**: 226+ tests, 6,631 LOC SPARQL, 528-line ggen.toml, 13 templates
**Validation**: ✅ 80/20 Principles (SPR + TPS + Chicago-TDD)
