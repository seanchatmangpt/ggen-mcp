# Guard Kernel

**Version**: 2.1.0 | 7 Safety Checks | Fail-Fast Code Generation

---

## Overview

**Guard Kernel**: 7 safety checks executed before code generation. **Fail-fast by default** → unsafe compilation blocked.

**Principle**: Prevention > Detection. Guards catch errors **before** code generation, not after.

**Execution**: Stage 2 of compilation pipeline (after discovery, before SPARQL).

**Guards**:
1. **G1: Path Safety** — No path traversal
2. **G2: Output Overlap** — No duplicate outputs
3. **G3: Template Compilation** — Valid Tera syntax
4. **G4: Turtle Parse** — Valid RDF syntax
5. **G5: SPARQL Execution** — Valid queries
6. **G6: Determinism** — Same inputs → same outputs
7. **G7: Bounds** — Size/time limits enforced

---

## Guard G1: Path Safety

### Purpose
Prevent path traversal attacks. Ensure all output paths stay within workspace root.

### Check Logic
```rust
fn check_path_safety(path: &Path, workspace_root: &Path) -> Result<()> {
    // 1. No path traversal (..)
    if path.components().any(|c| c == Component::ParentDir) {
        return Err("Path traversal detected");
    }

    // 2. Must be relative (no absolute paths)
    if path.is_absolute() {
        return Err("Absolute paths not allowed");
    }

    // 3. Canonicalize and verify within workspace
    let canonical = workspace_root.join(path).canonicalize()?;
    if !canonical.starts_with(workspace_root) {
        return Err("Path escapes workspace");
    }

    Ok(())
}
```

### Examples

#### PASS
```toml
[[generate]]
output_path = "src/generated/entity.rs"  # ✅ Relative, within workspace
```

#### FAIL
```toml
[[generate]]
output_path = "../../../etc/passwd"  # ❌ Path traversal detected
```

```toml
[[generate]]
output_path = "/etc/passwd"  # ❌ Absolute path not allowed
```

### Remediation
```markdown
**Issue**: Output path '../../../etc/passwd' attempts path traversal.

**Fix**:
1. Use paths relative to workspace root: 'src/generated/entity.rs'
2. Verify ggen.toml output paths do not contain '..'
3. Check template variables for user-supplied paths
```

---

## Guard G2: Output Overlap

### Purpose
Prevent multiple generation rules from writing to the same file (data loss risk).

### Check Logic
```rust
fn check_output_overlap(rules: &[GenerationRule]) -> Result<()> {
    let mut seen = HashMap::new();

    for rule in rules {
        let output = &rule.output_path;

        if let Some(existing_rule) = seen.get(output) {
            return Err(format!(
                "Output overlap: {} writes to {:?} (conflicts with {})",
                rule.name, output, existing_rule
            ));
        }

        seen.insert(output, &rule.name);
    }

    Ok(())
}
```

### Examples

#### PASS
```toml
[[generate]]
name = "generate_entities"
template = "templates/entity.rs.tera"
output_path = "src/generated/entity.rs"  # ✅ Unique

[[generate]]
name = "generate_models"
template = "templates/model.rs.tera"
output_path = "src/generated/model.rs"  # ✅ Unique (different file)
```

#### FAIL
```toml
[[generate]]
name = "generate_entities"
template = "templates/entity.rs.tera"
output_path = "src/generated/entity.rs"  # ❌ Duplicate

[[generate]]
name = "generate_models"
template = "templates/model.rs.tera"
output_path = "src/generated/entity.rs"  # ❌ Same output as above
```

### Remediation
```markdown
**Issue**: Multiple rules write to 'src/generated/entity.rs':
- Rule 'generate_entities' (templates/entity.rs.tera)
- Rule 'generate_models' (templates/model.rs.tera)

**Fix**:
1. Rename one output file: 'src/generated/entity_model.rs'
2. Or consolidate rules into single template
3. Check ggen.toml [[generate]] sections for duplicate output_path
```

---

## Guard G3: Template Compilation

### Purpose
Catch Tera template syntax errors before rendering.

### Check Logic
```rust
fn check_template_compilation(templates: &[PathBuf]) -> Result<()> {
    let mut tera = Tera::default();

    for template_path in templates {
        let content = fs::read_to_string(template_path)?;

        tera.add_raw_template(
            template_path.to_str().unwrap(),
            &content
        ).map_err(|e| format!(
            "Template '{}' failed to compile: {}",
            template_path.display(), e
        ))?;
    }

    Ok(())
}
```

### Examples

#### PASS
```jinja2
{# templates/entity.rs.tera #}
pub struct {{ entity_name }} {
    {% for field in fields %}
    pub {{ field.name }}: {{ field.type }},
    {% endfor %}
}
```

#### FAIL
```jinja2
{# templates/entity.rs.tera #}
pub struct {{ entity_name }} {
    {% for field in fields %}
    pub {{ field.name }}: {{ field.type }},
    {% endfor }}  {# ❌ Extra closing brace #}
}
```

**Error**:
```
Template 'templates/entity.rs.tera' failed to compile:
  Line 5: Unexpected token '}}', expected filter or tag end.
```

### Remediation
```markdown
**Issue**: Template 'templates/entity.rs.tera' failed to compile:
  Line 5: Unexpected token '}}', expected filter or tag end.

**Fix**:
1. Check template syntax at line 5
2. Common error: Unmatched braces '{{ }}' vs control tags '{% %}'
3. Run: validate_tera_template { template: "templates/entity.rs.tera" }
```

---

## Guard G4: Turtle Parse

### Purpose
Validate RDF ontologies parse as valid Turtle syntax.

### Check Logic
```rust
fn check_turtle_parse(ontologies: &[PathBuf]) -> Result<()> {
    let store = Store::new()?;

    for ontology_path in ontologies {
        let content = fs::read_to_string(ontology_path)?;

        store.load_from_reader(
            RdfFormat::Turtle,
            content.as_bytes()
        ).map_err(|e| format!(
            "Ontology '{}' failed to parse: {}",
            ontology_path.display(), e
        ))?;
    }

    Ok(())
}
```

### Examples

#### PASS
```turtle
# ontology/mcp-domain.ttl
@prefix mcp: <http://example.com/mcp#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

mcp:Tool a rdf:Class ;
    mcp:name "read_table" ;
    mcp:description "Read table from spreadsheet" .
```

#### FAIL
```turtle
# ontology/mcp-domain.ttl
@prefix mcp: <http://example.com/mcp#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

mcp:Tool a rdf:Class
    mcp:name "read_table"  # ❌ Missing semicolon
    mcp:description "Read table from spreadsheet" .
```

**Error**:
```
Ontology 'ontology/mcp-domain.ttl' failed to parse:
  Line 5: Expected '.' or ';' at end of triple, found 'mcp:name'.
```

### Remediation
```markdown
**Issue**: Ontology 'ontology/mcp-domain.ttl' failed to parse:
  Line 5: Expected '.' or ';' at end of triple, found 'mcp:name'.

**Fix**:
1. Check Turtle syntax at line 5
2. Common error: Missing semicolon ';' between predicates
3. Run: validate_ontology { ontology_path: "ontology/mcp-domain.ttl" }
```

---

## Guard G5: SPARQL Execution

### Purpose
Validate SPARQL queries execute without errors.

### Check Logic
```rust
fn check_sparql_execution(queries: &[PathBuf], store: &Store) -> Result<()> {
    for query_path in queries {
        let query_str = fs::read_to_string(query_path)?;

        let query = Query::parse(&query_str, None)
            .map_err(|e| format!(
                "Query '{}' failed to parse: {}",
                query_path.display(), e
            ))?;

        store.query(query)
            .map_err(|e| format!(
                "Query '{}' failed to execute: {}",
                query_path.display(), e
            ))?;
    }

    Ok(())
}
```

### Examples

#### PASS
```sparql
# queries/entities.rq
PREFIX mcp: <http://example.com/mcp#>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>

SELECT ?name ?description
WHERE {
  ?entity a mcp:Tool ;
          mcp:name ?name ;
          mcp:description ?description .
}
```

#### FAIL
```sparql
# queries/entities.rq
PREFIX mcp: <http://example.com/mcp#>

SELECT ?name ?description
WHERE {
  ?entity a mcp:Tool ;
          mcp:name ?name .
  # ❌ Variable ?description used but not bound
}
```

**Error**:
```
Query 'queries/entities.rq' failed to execute:
  Variable ?description used in SELECT but not bound in WHERE clause.
```

### Remediation
```markdown
**Issue**: Query 'queries/entities.rq' failed to execute:
  Variable ?description used in SELECT but not bound in WHERE clause.

**Fix**:
1. Add binding for ?description in WHERE clause
2. Or remove ?description from SELECT
3. Test query: sparql_query { query: "queries/entities.rq" }
```

---

## Guard G6: Determinism

### Purpose
Ensure same inputs → same outputs (reproducible builds).

### Check Logic
```rust
fn check_determinism(
    inputs: &[PathBuf],
    cached_hash: Option<&str>
) -> Result<()> {
    // Compute SHA-256 of all inputs
    let mut hasher = Sha256::new();

    for input in inputs {
        let content = fs::read(input)?;
        hasher.update(&content);
    }

    let current_hash = format!("{:x}", hasher.finalize());

    // Compare with cached hash (if exists)
    if let Some(cached) = cached_hash {
        if current_hash != cached {
            return Err(format!(
                "Determinism check failed: hash mismatch\n\
                 Expected: {}\n\
                 Got:      {}",
                cached, current_hash
            ));
        }
    }

    Ok(())
}
```

### Examples

#### PASS (Cache Hit)
```markdown
**Run 1**: SHA-256 = 7f83b165a3d9f7e2...
**Run 2**: SHA-256 = 7f83b165a3d9f7e2... ✅ Match (cache hit)
```

#### FAIL (Non-Deterministic)
```markdown
**Run 1**: SHA-256 = 7f83b165a3d9f7e2...
**Run 2**: SHA-256 = 9a2c1d4f8b6e3a7c... ❌ Mismatch
```

### Common Causes of Non-Determinism

#### 1. Random Values
```jinja2
{# ❌ BAD: Random UUID #}
pub const ID: Uuid = Uuid::parse_str("{{ uuid_v4() }}").unwrap();

{# ✅ GOOD: Deterministic seed #}
pub const ID: Uuid = Uuid::parse_str("{{ entity_name | hash | slice(end=36) }}").unwrap();
```

#### 2. Timestamps
```jinja2
{# ❌ BAD: Current timestamp #}
// Generated at: {{ now() }}

{# ✅ GOOD: Placeholder #}
// Generated from ontology (see ggen.toml)
```

#### 3. HashMap Iteration
```rust
// ❌ BAD: HashMap (unstable order)
let mut map = HashMap::new();
for (k, v) in map.iter() {
    println!("{}: {}", k, v);
}

// ✅ GOOD: BTreeMap (stable order)
let mut map = BTreeMap::new();
for (k, v) in map.iter() {
    println!("{}: {}", k, v);
}
```

### Remediation
```markdown
**Issue**: Output hash mismatch for 'src/generated/entity.rs':
  Expected: 7f83b165a3d9f7e2...
  Got:      9a2c1d4f8b6e3a7c...

**Possible Causes**:
1. Template uses random values (UUID, timestamps)
2. Template iterates HashMap (unstable order)
3. System clock or environment variables in output

**Fix**:
1. Use stable iteration (BTreeMap, sorted Vec)
2. Replace random values with deterministic seeds
3. Avoid timestamps in generated code
```

---

## Guard G7: Bounds

### Purpose
Enforce size/time limits to prevent resource exhaustion.

### Check Logic
```rust
fn check_bounds(
    files: &[PathBuf],
    max_files: usize,
    max_bytes: usize
) -> Result<()> {
    // Check file count
    if files.len() > max_files {
        return Err(format!(
            "File count ({}) exceeds limit ({})",
            files.len(), max_files
        ));
    }

    // Check total size
    let total_size: usize = files.iter()
        .map(|f| fs::metadata(f).unwrap().len() as usize)
        .sum();

    if total_size > max_bytes {
        return Err(format!(
            "Total size ({} bytes) exceeds limit ({} bytes)",
            total_size, max_bytes
        ));
    }

    Ok(())
}
```

### Limits (Default)

```toml
[limits]
max_output_files = 1000           # Max files generated
max_output_bytes = 104857600      # 100 MB total
max_template_bytes = 1048576      # 1 MB per template
max_ontology_triples = 100000     # 100K triples per ontology
max_sparql_duration_ms = 5000     # 5s per query
```

### Examples

#### PASS
```markdown
**Files**: 24 (under 1000 limit)
**Total Size**: 47.2 KB (under 100 MB limit)
**Result**: ✅ PASS
```

#### FAIL (File Count)
```markdown
**Files**: 1,247 (exceeds 1000 limit)
**Total Size**: 23.4 MB (under 100 MB limit)
**Result**: ❌ FAIL (file count)
```

#### FAIL (Total Size)
```markdown
**Files**: 347 (under 1000 limit)
**Total Size**: 127 MB (exceeds 100 MB limit)
**Result**: ❌ FAIL (total size)
```

### Remediation
```markdown
**Issue**: Total output size (127 MB) exceeds limit (100 MB).

**Fix**:
1. Reduce number of generated files (current: 547)
2. Increase limit: ggen.toml [limits] max_output_bytes = 200000000
3. Split generation rules across multiple syncs
4. Optimize template output (remove comments, compact formatting)
```

---

## Guard Configuration

### Enable/Disable Guards

```toml
[guards]
enabled = ["G1", "G2", "G3", "G4", "G5", "G6", "G7"]  # All enabled
# enabled = ["G1", "G2", "G3"]  # Only first 3 enabled
# enabled = []  # All disabled (⚠️ unsafe)
```

### Fail-Fast vs Continue

```toml
[guards]
fail_fast = true  # Stop on first failure (default)
# fail_fast = false  # Run all guards, report all failures
```

**Fail-Fast Example**:
```markdown
## Guard Verdicts (fail_fast = true)

| Guard | Verdict | Diagnostic |
|-------|---------|------------|
| G1: Path Safety | ✅ PASS | All paths validated |
| G2: Output Overlap | ❌ FAIL | Duplicate output detected |
| G3: Template Compilation | ⏸️ SKIPPED | Blocked by G2 failure |
| G4: Turtle Parse | ⏸️ SKIPPED | Blocked by G2 failure |

**Result**: Compilation stopped at G2.
```

**Continue Example**:
```markdown
## Guard Verdicts (fail_fast = false)

| Guard | Verdict | Diagnostic |
|-------|---------|------------|
| G1: Path Safety | ✅ PASS | All paths validated |
| G2: Output Overlap | ❌ FAIL | Duplicate output detected |
| G3: Template Compilation | ❌ FAIL | 2 templates invalid |
| G4: Turtle Parse | ✅ PASS | All ontologies parsed |

**Result**: 2 guards failed. Fix all before applying.
```

---

## Force Mode (Bypass Guards)

**Usage**: `force: true` bypasses guard failures.

**API**:
```json
{
  "tool": "sync_ggen",
  "params": {
    "workspace_root": ".",
    "force": true  // ⚠️ Bypasses guards
  }
}
```

**Behavior**:
- Guards still execute (for reporting)
- Failures logged but don't block compilation
- **Use with extreme caution** (security risk)

**Example**:
```markdown
## Guard Verdicts (force = true)

| Guard | Verdict | Diagnostic |
|-------|---------|------------|
| G1: Path Safety | ✅ PASS | All paths validated |
| G2: Output Overlap | ❌ FAIL | Duplicate output detected |

**⚠️ WARNING**: Guard failure bypassed (force mode enabled).
**Result**: Compilation proceeded despite G2 failure.
```

**When to Use**:
- **NEVER** in production
- Emergency fixes (manual review required)
- Testing/debugging guard logic

---

## Custom Guards (Future: v2.2)

**Trait**:
```rust
pub trait Guard {
    fn name(&self) -> &str;
    fn check(&self, ctx: &CompilationContext) -> GuardResult;
    fn remediation(&self, failure: &GuardFailure) -> String;
}
```

**Example**:
```rust
struct LicenseHeaderGuard;

impl Guard for LicenseHeaderGuard {
    fn name(&self) -> &str {
        "G8_LicenseHeader"
    }

    fn check(&self, ctx: &CompilationContext) -> GuardResult {
        for file in &ctx.outputs {
            let content = fs::read_to_string(file)?;
            if !content.starts_with("// Copyright") {
                return GuardResult::Fail(format!(
                    "File {} missing license header",
                    file.display()
                ));
            }
        }
        GuardResult::Pass
    }

    fn remediation(&self, failure: &GuardFailure) -> String {
        format!(
            "Add license header to top of file:\n\
             // Copyright 2026 Your Company\n\
             // Licensed under Apache-2.0"
        )
    }
}
```

**Configuration**:
```toml
[guards]
enabled = ["G1", "G2", "G3", "G4", "G5", "G6", "G7", "G8_LicenseHeader"]
custom = ["path/to/custom_guards.rs"]
```

---

## Best Practices

### 1. Never Disable Guards in Production
```toml
# ❌ DON'T
[guards]
enabled = []  # Disables all guards (unsafe)

# ✅ DO
[guards]
enabled = ["G1", "G2", "G3", "G4", "G5", "G6", "G7"]  # All guards
```

### 2. Fix Guard Failures, Don't Bypass
```bash
# ❌ DON'T
ggen sync --force  # Bypasses guards

# ✅ DO
ggen sync          # Review guard failures
# Fix issues
ggen sync          # Re-run until all guards pass
```

### 3. Use Fail-Fast in Development
```toml
[guards]
fail_fast = true  # Immediate feedback
```

### 4. Review Guard Verdicts in Reports
```bash
cat ./ggen.out/reports/latest.md | grep -A 10 "Guard Verdicts"
```

### 5. Test Non-Determinism Early
```bash
# Run twice, verify same output
ggen sync
OUTPUT1=$(sha256sum src/generated/entity.rs)

ggen sync
OUTPUT2=$(sha256sum src/generated/entity.rs)

# Should match
[ "$OUTPUT1" = "$OUTPUT2" ] || echo "Non-deterministic generation detected"
```

---

## Troubleshooting

### Issue: All Guards Fail

**Symptom**: Every guard reports FAIL.

**Causes**:
1. Workspace corruption (config/ontologies invalid)
2. Missing dependencies (Tera/Oxigraph not installed)

**Solutions**:
1. Verify workspace: `ls -lh ggen.toml ontology/ queries/ templates/`
2. Re-install: `cargo install ggen-mcp --force`

---

### Issue: G6 Always Fails

**Symptom**: Determinism check fails every run.

**Causes**:
1. Template uses timestamps/random values
2. HashMap iteration (unstable order)

**Solutions**:
1. Search templates for `now()`, `uuid_v4()`, `random()`
2. Replace HashMap with BTreeMap
3. Sort all collections before iteration

---

### Issue: G7 Fails (Bounds)

**Symptom**: Output exceeds limits.

**Causes**:
1. Too many files generated
2. Large templates (verbose output)

**Solutions**:
1. Increase limits (ggen.toml `[limits]`)
2. Split generation rules (multiple syncs)
3. Optimize templates (remove debug output)

---

## References

- **Proof-First Compiler**: [docs/PROOF_FIRST_COMPILER.md](./PROOF_FIRST_COMPILER.md)
- **First Light Report**: [docs/FIRST_LIGHT_REPORT.md](./FIRST_LIGHT_REPORT.md)
- **Receipt Verification**: [docs/RECEIPT_VERIFICATION.md](./RECEIPT_VERIFICATION.md)
- **Migration Guide**: [MIGRATION_GUIDE_V2.1.md](../MIGRATION_GUIDE_V2.1.md)

---

**End of GUARD_KERNEL.md**
