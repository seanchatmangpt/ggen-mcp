# Ontology Tools Reference

**Version**: 1.0.0 | **Accurate Implementation Docs** | 5 Tools

---

## Overview

Ontology tools enable RDF/SPARQL-driven code generation. Load Turtle ontologies → Query with SPARQL → Render Tera templates → Validate → Write with audit trail.

**Tool Chain**:
```
load_ontology → execute_sparql_query → render_template → validate_generated_code → write_generated_artifact
```

**All tools idempotent**. Same input → same output.

---

## Quick Reference

| Tool | Purpose | Latency | Key Parameters |
|------|---------|---------|----------------|
| `load_ontology` | Load RDF/Turtle ontology | <1s | path, validate, base_iri |
| `execute_sparql_query` | Query loaded ontology | <500ms | ontology_id, query, cache_ttl |
| `render_template` | Render Tera template | 100-500ms | template, context, validate_syntax |
| `validate_generated_code` | Multi-language validation | 200ms-2s | code, language, golden_file_path |
| `write_generated_artifact` | Write with audit trail | <100ms | content, output_path, create_backup |

---

## Tool 1: load_ontology

**Purpose**: Load RDF/Turtle ontology file into Oxigraph store. Validate syntax. Cache for SPARQL queries.

### Parameters

```typescript
{
  path: string              // Path to .ttl file (relative to workspace)
  validate?: boolean        // Enable SHACL validation (default: true)
  base_iri?: string        // Base IRI for relative IRIs (optional)
}
```

### Response

```typescript
{
  ontology_id: string       // SHA-256 hash (use for execute_sparql_query)
  path: string              // Ontology file path
  triple_count: number      // RDF triples loaded
  class_count: number       // rdfs:Class count
  property_count: number    // rdfs:Property count
  validation_passed: boolean// SHACL validation status
  validation_errors: Array<{ // Validation errors (if any)
    severity: "error" | "warning"
    message: string
    subject: string          // RDF subject URI
  }>
  load_duration_ms: number
  cached: boolean           // Whether result from cache
}
```

### Example

```json
{
  "tool": "load_ontology",
  "arguments": {
    "path": "ontology/mcp-domain.ttl",
    "validate": true,
    "base_iri": "http://example.org/mcp#"
  }
}
```

**Response**:
```json
{
  "ontology_id": "sha256:7f83b1657ff1fc53b92dc18148a1d65dfc2d4b1fa3d677284addd200126d9069",
  "path": "ontology/mcp-domain.ttl",
  "triple_count": 347,
  "class_count": 12,
  "property_count": 45,
  "validation_passed": true,
  "validation_errors": [],
  "load_duration_ms": 234,
  "cached": false
}
```

### Error Codes

| Code | Meaning | Recovery |
|------|---------|----------|
| `ONTOLOGY_NOT_FOUND` | File doesn't exist | Check path parameter |
| `INVALID_TURTLE_SYNTAX` | RDF parsing failed | Fix Turtle syntax |
| `VALIDATION_FAILED` | SHACL constraints violated | Fix ontology constraints |
| `PATH_TRAVERSAL_DETECTED` | Unsafe path | Use workspace-relative path |

### Use Cases

1. **Load ontology for queries**
   ```bash
   load_ontology { path: "ontology/domain.ttl" }
   # Use returned ontology_id for SPARQL queries
   ```

2. **Validate ontology without caching**
   ```bash
   load_ontology { path: "ontology/test.ttl", validate: true }
   # Check validation_passed before proceeding
   ```

3. **Pre-load multiple ontologies**
   ```bash
   load_ontology { path: "ontology/core.ttl" }
   load_ontology { path: "ontology/extensions.ttl" }
   # Cache both for parallel queries
   ```

---

## Tool 2: execute_sparql_query

**Purpose**: Execute SPARQL SELECT/CONSTRUCT query against loaded ontology. Return typed results as JSON.

### Parameters

```typescript
{
  ontology_id: string       // From load_ontology response
  query: string             // SPARQL query text
  cache_ttl?: number        // Cache TTL in seconds (default: 300)
  timeout_ms?: number       // Query timeout (default: 5000, max: 30000)
  explain?: boolean         // Include query analysis (default: false)
}
```

### Response

```typescript
{
  results: Array<Record<string, any>>  // Query results (JSON objects)
  result_count: number                 // Number of results
  execution_time_ms: number
  from_cache: boolean
  query_analysis?: {                   // If explain=true
    complexity: "simple" | "medium" | "complex"
    estimated_cost: number
    uses_reasoning: boolean
    triple_pattern_count: number
  }
}
```

### Example

```json
{
  "tool": "execute_sparql_query",
  "arguments": {
    "ontology_id": "sha256:7f83b1657ff1fc53b92dc18148a1d65dfc2d4b1fa3d677284addd200126d9069",
    "query": "PREFIX mcp: <http://example.org/mcp#>\nPREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>\n\nSELECT ?tool ?label ?comment\nWHERE {\n  ?tool a mcp:Tool ;\n        rdfs:label ?label ;\n        rdfs:comment ?comment .\n}\nORDER BY ?label",
    "cache_ttl": 600,
    "explain": true
  }
}
```

**Response**:
```json
{
  "results": [
    {
      "tool": "http://example.org/mcp#ValidateTool",
      "label": "validate_ontology",
      "comment": "Validate RDF/Turtle ontology files for SHACL conformance"
    },
    {
      "tool": "http://example.org/mcp#GenerateTool",
      "label": "generate_from_schema",
      "comment": "Generate Rust entities from Zod or JSON Schema"
    }
  ],
  "result_count": 2,
  "execution_time_ms": 23,
  "from_cache": false,
  "query_analysis": {
    "complexity": "simple",
    "estimated_cost": 150,
    "uses_reasoning": false,
    "triple_pattern_count": 3
  }
}
```

### Error Codes

| Code | Meaning | Recovery |
|------|---------|----------|
| `INVALID_ONTOLOGY_ID` | Ontology not loaded or expired | Load ontology first |
| `SPARQL_SYNTAX_ERROR` | Query syntax invalid | Fix SPARQL syntax |
| `QUERY_TIMEOUT` | Exceeded timeout_ms | Simplify query or increase timeout |
| `EMPTY_RESULT_SET` | Query returned no results | Check query logic |

### SPARQL Injection Prevention

**Built-in safety**: Query text sanitized. No user input concatenation. Parameterized bindings only.

**Safe**:
```json
{
  "query": "SELECT ?s WHERE { ?s a mcp:Tool }"
}
```

**Unsafe** (not possible with this tool):
```json
{
  "query": "SELECT ?s WHERE { ?s a " + user_input + " }"  // ❌ Not supported
}
```

### Use Cases

1. **Extract tool definitions**
   ```sparql
   PREFIX mcp: <http://example.org/mcp#>
   SELECT ?tool ?label WHERE {
     ?tool a mcp:Tool ; rdfs:label ?label .
   }
   ```

2. **Get parameters for tool**
   ```sparql
   PREFIX mcp: <http://example.org/mcp#>
   SELECT ?param ?name ?type WHERE {
     mcp:ValidateTool mcp:hasParameter ?param .
     ?param mcp:parameterName ?name ;
            mcp:parameterType ?type .
   }
   ```

3. **Count entities by type**
   ```sparql
   SELECT ?class (COUNT(?instance) AS ?count) WHERE {
     ?instance a ?class .
   } GROUP BY ?class
   ```

---

## Tool 3: render_template

**Purpose**: Render Tera template with context data. Output validated code/config.

### Parameters

```typescript
{
  template: string          // Template name OR "inline:<template_text>"
  context: Record<string, any>  // Context variables (JSON object)
  timeout_ms?: number       // Render timeout (default: 5000, max: 30000)
  max_output_size?: number  // Max output bytes (default: 1MB, max: 10MB)
  validate_syntax?: boolean // Enable syntax checks (default: true)
  security_checks?: boolean // Enable security checks (default: true)
  preview?: boolean         // Preview mode (don't write files) (default: false)
  output_format?: string    // Target format: rust, typescript, yaml, json, toml
}
```

### Response

```typescript
{
  output: string            // Rendered template output
  output_size: number       // Output size in bytes
  duration_ms: number
  warnings: string[]        // Validation warnings
  preview: boolean
  content_hash: string      // SHA-256 of output
}
```

### Example (Template File)

```json
{
  "tool": "render_template",
  "arguments": {
    "template": "entity.rs.tera",
    "context": {
      "entity_name": "User",
      "fields": [
        {"name": "id", "type": "Uuid"},
        {"name": "email", "type": "String"}
      ]
    },
    "output_format": "rust",
    "validate_syntax": true
  }
}
```

**Response**:
```json
{
  "output": "#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]\npub struct User {\n    pub id: Uuid,\n    pub email: String,\n}\n",
  "output_size": 127,
  "duration_ms": 45,
  "warnings": [],
  "preview": false,
  "content_hash": "sha256:abc123def456..."
}
```

### Example (Inline Template)

```json
{
  "tool": "render_template",
  "arguments": {
    "template": "inline:pub fn {{ fn_name }}() -> Result<()> { Ok(()) }",
    "context": {
      "fn_name": "validate_user"
    },
    "output_format": "rust"
  }
}
```

**Response**:
```json
{
  "output": "pub fn validate_user() -> Result<()> { Ok(()) }",
  "output_size": 47,
  "duration_ms": 12,
  "warnings": [],
  "preview": false,
  "content_hash": "sha256:xyz789abc123..."
}
```

### Error Codes

| Code | Meaning | Recovery |
|------|---------|----------|
| `TEMPLATE_NOT_FOUND` | Template file doesn't exist | Check template name/path |
| `TEMPLATE_SYNTAX_ERROR` | Tera syntax error | Fix template syntax |
| `CONTEXT_INVALID` | Context not JSON object | Pass valid JSON object |
| `OUTPUT_TOO_LARGE` | Output exceeds max_output_size | Increase limit or simplify template |
| `RENDER_TIMEOUT` | Exceeded timeout_ms | Simplify template or increase timeout |

### Tera Template Features

**Supported filters**:
- `upper`, `lower`, `capitalize`
- `snake_case`, `pascal_case`, `camel_case`
- `truncate`, `wordcount`, `replace`
- `json_encode`

**Example template** (`entity.rs.tera`):
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct {{ entity_name }} {
    {% for field in fields %}
    pub {{ field.name }}: {{ field.type }},
    {% endfor %}
}

impl {{ entity_name }} {
    pub fn new(
        {% for field in fields %}
        {{ field.name }}: {{ field.type }},
        {% endfor %}
    ) -> Self {
        Self {
            {% for field in fields %}
            {{ field.name }},
            {% endfor %}
        }
    }
}
```

### Use Cases

1. **Generate Rust struct from SPARQL results**
   ```bash
   # Step 1: Query ontology
   execute_sparql_query { ontology_id: "...", query: "SELECT ..." }

   # Step 2: Render template with results
   render_template {
     template: "struct.rs.tera",
     context: { "results": "<from step 1>" }
   }
   ```

2. **Inline template for quick generation**
   ```bash
   render_template {
     template: "inline:{{ prefix }}::{{ name }}",
     context: { "prefix": "crate::domain", "name": "User" }
   }
   ```

3. **Preview mode for testing**
   ```bash
   render_template {
     template: "complex.rs.tera",
     context: { ... },
     preview: true  # Don't write files
   }
   ```

---

## Tool 4: validate_generated_code

**Purpose**: Validate generated code syntax and semantics. Compare against golden files. Multi-language support.

### Parameters

```typescript
{
  code: string              // Generated code to validate
  language: string          // Language: rust, typescript, yaml, json
  file_name: string         // File name for error context
  golden_file_path?: string // Path to golden file for comparison
  strict_mode?: boolean     // Treat warnings as errors (default: false)
  allow_golden_update?: boolean  // Allow updating golden file (default: false)
}
```

### Response

```typescript
{
  valid: boolean
  errors: Array<{
    message: string
    location?: string       // file:line:col
    suggestion?: string
  }>
  warnings: string[]
  suggestions: string[]
  golden_file_diff?: {
    golden_file: string
    additions: number
    deletions: number
    changes: number
    is_identical: boolean
    diff_sample: string[]   // First 20 diff lines
  }
  language: string
  summary: string
}
```

### Example (Rust Validation)

```json
{
  "tool": "validate_generated_code",
  "arguments": {
    "code": "pub struct User {\n    pub id: Uuid,\n    pub email: String,\n}\n",
    "language": "rust",
    "file_name": "user.rs",
    "strict_mode": false
  }
}
```

**Response**:
```json
{
  "valid": true,
  "errors": [],
  "warnings": [],
  "suggestions": [
    "Consider adding #[derive(Debug)] for better error messages"
  ],
  "language": "rust",
  "summary": "✓ Validation passed for rust code."
}
```

### Example (Golden File Comparison)

```json
{
  "tool": "validate_generated_code",
  "arguments": {
    "code": "pub struct User { pub id: Uuid }",
    "language": "rust",
    "file_name": "user.rs",
    "golden_file_path": "tests/golden/user.rs",
    "allow_golden_update": false
  }
}
```

**Response**:
```json
{
  "valid": true,
  "errors": [],
  "warnings": [
    "Generated code differs from golden file: 0 additions, 1 deletions, 1 changes"
  ],
  "suggestions": [],
  "golden_file_diff": {
    "golden_file": "tests/golden/user.rs",
    "additions": 0,
    "deletions": 1,
    "changes": 1,
    "is_identical": false,
    "diff_sample": [
      "   1 pub struct User {",
      "   2     pub id: Uuid,",
      "-  3     pub email: String,",
      "   4 }"
    ]
  },
  "language": "rust",
  "summary": "✓ Validation passed. Code differs from golden file (2 changes)."
}
```

### Supported Languages

| Language | Validation Method | Golden File Support |
|----------|-------------------|---------------------|
| `rust` | syn parser (AST validation) | ✅ |
| `typescript` | Basic syntax checks | ✅ |
| `javascript` | Basic syntax checks | ✅ |
| `json` | serde_json parser | ✅ |
| `yaml` | serde_yaml parser | ✅ |

### Error Codes

| Code | Meaning | Recovery |
|------|---------|----------|
| `SYNTAX_ERROR` | Code syntax invalid | Fix generated code or template |
| `UNSUPPORTED_LANGUAGE` | Language not supported | Use supported language |
| `GOLDEN_FILE_NOT_FOUND` | Golden file missing | Create golden file or remove param |

### Golden File Workflow

1. **Initial generation** (no golden file):
   ```bash
   validate_generated_code {
     code: "<generated>",
     language: "rust",
     file_name: "entity.rs",
     golden_file_path: "tests/golden/entity.rs",
     allow_golden_update: true
   }
   # ✅ Golden file created
   ```

2. **Subsequent validations**:
   ```bash
   validate_generated_code {
     code: "<new version>",
     language: "rust",
     file_name: "entity.rs",
     golden_file_path: "tests/golden/entity.rs"
   }
   # ⚠️ Warns if diff detected
   ```

3. **Update golden file** (when intentional change):
   ```bash
   export UPDATE_GOLDEN=1
   validate_generated_code {
     code: "<new version>",
     language: "rust",
     file_name: "entity.rs",
     golden_file_path: "tests/golden/entity.rs",
     allow_golden_update: true
   }
   # ✅ Golden file updated
   ```

### Use Cases

1. **Validate before writing**
   ```bash
   render_template { ... } → validate_generated_code { ... } → write_generated_artifact { ... }
   ```

2. **Regression testing**
   ```bash
   validate_generated_code {
     code: "<newly generated>",
     golden_file_path: "tests/golden/baseline.rs"
   }
   # Ensure generation hasn't regressed
   ```

3. **Multi-language validation**
   ```bash
   # Rust
   validate_generated_code { code: "...", language: "rust" }
   # TypeScript
   validate_generated_code { code: "...", language: "typescript" }
   # JSON
   validate_generated_code { code: "...", language: "json" }
   ```

---

## Tool 5: write_generated_artifact

**Purpose**: Write generated code to file with validation, backup, and cryptographic audit trail.

### Parameters

```typescript
{
  content: string           // Content to write
  output_path: string       // Output file path (relative to workspace)
  create_backup?: boolean   // Backup existing file (default: true)
  ontology_hash?: string    // SHA-256 of source ontology (for provenance)
  template_hash?: string    // SHA-256 of template (for provenance)
  metadata?: Record<string, string>  // Additional metadata for receipt
  preview?: boolean         // Preview mode - don't write (default: false)
}
```

### Response

```typescript
{
  output_path: string
  written: boolean          // False in preview mode
  content_hash: string      // SHA-256 of content
  receipt_id: string        // Generation receipt ID
  backup_path?: string      // Backup file path (if created)
  size: number              // Content size in bytes
  preview: boolean
}
```

### Example

```json
{
  "tool": "write_generated_artifact",
  "arguments": {
    "content": "pub struct User {\n    pub id: Uuid,\n    pub email: String,\n}\n",
    "output_path": "src/generated/user.rs",
    "create_backup": true,
    "ontology_hash": "sha256:7f83b165...",
    "template_hash": "sha256:abc123de...",
    "metadata": {
      "generator": "render_template",
      "version": "1.0.0"
    }
  }
}
```

**Response**:
```json
{
  "output_path": "src/generated/user.rs",
  "written": true,
  "content_hash": "sha256:def456ab...",
  "receipt_id": "receipt-20260120-102345-a7b9c1d2",
  "backup_path": "src/generated/user.rs.bak",
  "size": 78,
  "preview": false
}
```

### Generation Receipt

**Saved to**: `<output_path>.receipt.json`

**Example** (`src/generated/user.rs.receipt.json`):
```json
{
  "receipt_id": "receipt-20260120-102345-a7b9c1d2",
  "timestamp": "2026-01-20T10:23:45Z",
  "output_file": "src/generated/user.rs",
  "output_hash": "sha256:def456ab...",
  "provenance": {
    "ontology_hash": "sha256:7f83b165...",
    "template_hash": "sha256:abc123de...",
    "generator": "render_template",
    "version": "1.0.0"
  },
  "metadata": {
    "size_bytes": 78,
    "backup_created": true
  }
}
```

### Error Codes

| Code | Meaning | Recovery |
|------|---------|----------|
| `INVALID_PATH` | Path outside workspace or unsafe | Use workspace-relative path |
| `WRITE_FAILED` | File write error | Check permissions |
| `CONTENT_EMPTY` | Content parameter empty | Provide non-empty content |

### Safety Features

1. **Path traversal prevention**: Rejects `../` in paths
2. **Backup on overwrite**: Preserves existing files as `.bak`
3. **Atomic writes**: Uses temp file + rename (no partial writes)
4. **Receipt generation**: Cryptographic provenance tracking

### Use Cases

1. **Write after validation**
   ```bash
   validate_generated_code { code: "..." } → write_generated_artifact { content: "..." }
   ```

2. **Preview before writing**
   ```bash
   write_generated_artifact {
     content: "...",
     output_path: "src/generated/new.rs",
     preview: true  # Check if write would succeed
   }
   ```

3. **Audit trail for compliance**
   ```bash
   write_generated_artifact {
     content: "...",
     output_path: "src/generated/entity.rs",
     ontology_hash: "sha256:...",  # Track source
     template_hash: "sha256:...",  # Track generator
     metadata: { "ticket": "JIRA-123" }
   }
   # Receipt saved for audit
   ```

---

## Complete Workflow Example

**Goal**: Generate Rust entity from ontology definition.

### Step 1: Load Ontology

```json
{
  "tool": "load_ontology",
  "arguments": {
    "path": "ontology/entities.ttl",
    "validate": true
  }
}
```

**Response**: `ontology_id: "sha256:7f83b165..."`

### Step 2: Extract Entity Definition (SPARQL)

```json
{
  "tool": "execute_sparql_query",
  "arguments": {
    "ontology_id": "sha256:7f83b165...",
    "query": "PREFIX entity: <http://example.org/entity#>\nSELECT ?name ?field_name ?field_type WHERE {\n  ?entity a entity:Entity ;\n    entity:name ?name ;\n    entity:hasField ?field .\n  ?field entity:fieldName ?field_name ;\n    entity:fieldType ?field_type .\n}"
  }
}
```

**Response**:
```json
{
  "results": [
    {"name": "User", "field_name": "id", "field_type": "Uuid"},
    {"name": "User", "field_name": "email", "field_type": "String"}
  ],
  "result_count": 2
}
```

### Step 3: Render Template

```json
{
  "tool": "render_template",
  "arguments": {
    "template": "entity.rs.tera",
    "context": {
      "entity_name": "User",
      "fields": [
        {"name": "id", "type": "Uuid"},
        {"name": "email", "type": "String"}
      ]
    },
    "output_format": "rust"
  }
}
```

**Response**: `output: "pub struct User { ... }"`

### Step 4: Validate Generated Code

```json
{
  "tool": "validate_generated_code",
  "arguments": {
    "code": "pub struct User { pub id: Uuid, pub email: String }",
    "language": "rust",
    "file_name": "user.rs",
    "golden_file_path": "tests/golden/user.rs"
  }
}
```

**Response**: `valid: true`

### Step 5: Write Artifact

```json
{
  "tool": "write_generated_artifact",
  "arguments": {
    "content": "pub struct User { pub id: Uuid, pub email: String }",
    "output_path": "src/generated/user.rs",
    "create_backup": true,
    "ontology_hash": "sha256:7f83b165...",
    "template_hash": "sha256:abc123de..."
  }
}
```

**Response**: `written: true, receipt_id: "receipt-..."`

---

## Performance Characteristics

| Tool | Typical Latency | Bottleneck | Caching |
|------|----------------|------------|---------|
| `load_ontology` | 100ms-1s | RDF parsing | ✅ 5min TTL |
| `execute_sparql_query` | 10-500ms | Query complexity | ✅ Configurable |
| `render_template` | 50-500ms | Template size | ❌ No cache |
| `validate_generated_code` | 100ms-2s | Syntax parsing | ❌ No cache |
| `write_generated_artifact` | 10-100ms | Disk I/O | ❌ No cache |

**Total pipeline**: 300ms-4s (typical)

---

## Security Considerations

1. **Path Traversal Prevention**
   - All paths validated against workspace root
   - `../` rejected
   - Symbolic links followed with caution

2. **SPARQL Injection Prevention**
   - Query text sanitized
   - No user input concatenation
   - Parameterized bindings only

3. **Template Injection Prevention**
   - Tera sandbox enabled
   - No filesystem access beyond workspace
   - No arbitrary code execution

4. **Audit Trail Integrity**
   - SHA-256 hashes for all artifacts
   - Receipt files tamper-evident
   - Timestamps in ISO-8601

---

## Troubleshooting

### Issue: load_ontology fails with INVALID_TURTLE_SYNTAX

**Cause**: RDF/Turtle syntax error in ontology file

**Fix**:
1. Validate Turtle syntax with external tool (rapper, rdflib)
2. Check for missing prefixes or malformed URIs
3. Ensure all triples end with `.`

### Issue: execute_sparql_query returns empty results

**Cause**: Query doesn't match ontology structure

**Fix**:
1. Check prefix definitions match ontology
2. Verify triple patterns exist in ontology
3. Use `execute_sparql_query` with `explain: true` to analyze query

### Issue: render_template fails with TEMPLATE_SYNTAX_ERROR

**Cause**: Tera template syntax error

**Fix**:
1. Check for unmatched `{% %}` tags
2. Verify filter names are correct (use `snake_case`, not `snakeCase`)
3. Test template with minimal context first

### Issue: validate_generated_code reports SYNTAX_ERROR

**Cause**: Generated code has syntax errors

**Fix**:
1. Review template logic
2. Ensure context variables match template expectations
3. Run `rustfmt` or language formatter on generated code

### Issue: write_generated_artifact fails with INVALID_PATH

**Cause**: Path outside workspace or contains unsafe characters

**Fix**:
1. Use relative path from workspace root
2. Remove `../` from path
3. Check workspace_root configuration

---

## Best Practices

1. **Always validate ontology before querying**
   ```bash
   load_ontology { validate: true } → execute_sparql_query { ... }
   ```

2. **Use golden files for regression testing**
   ```bash
   validate_generated_code { golden_file_path: "tests/golden/..." }
   ```

3. **Enable audit trails for production**
   ```bash
   write_generated_artifact {
     ontology_hash: "...",
     template_hash: "...",
     metadata: { "env": "production" }
   }
   ```

4. **Preview before writing in CI/CD**
   ```bash
   write_generated_artifact { preview: true } → review → write_generated_artifact { preview: false }
   ```

5. **Cache SPARQL queries for performance**
   ```bash
   execute_sparql_query { cache_ttl: 3600 }  # 1 hour cache
   ```

---

**Version**: 1.0.0 | **Last Updated**: 2026-01-20 | **Status**: ✅ Accurate Implementation Docs
