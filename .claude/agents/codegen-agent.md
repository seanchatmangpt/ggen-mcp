# Code Generation Agent

**Purpose**: Orchestrate ontology-driven code generation with quality gates

**Trigger**: Manual invocation or ontology file changes

## Workflow

### 1. Update Ontology
```
Input: User intent (new feature, schema change, validation)
Action: Update ontology/mcp-domain.ttl (RDF/Turtle)
Verify: Turtle syntax valid, no duplicate URIs
```

### 2. Create/Update SPARQL Query
```
Input: Extract requirements from ontology change
Action: Create queries/*.rq file (SPARQL)
Verify: Syntax valid, returns expected triples
```

### 3. Create/Update Tera Template
```
Input: Output format needed (Rust, test, doc)
Action: Create templates/*.rs.tera file
Verify: Template syntax valid, no undefined variables
```

### 4. Update Generation Config
```
Input: Input/output mappings
Action: Add rule to ggen.toml
Format: [[generation]]
        sparql_query = "queries/my_query.rq"
        template = "templates/output.rs.tera"
        output = "src/generated/my_generated.rs"
```

### 5. Run Generation (Andon Cord)
```bash
cargo make sync-dry-run   # Preview without writing
# Verify output before committing
cargo make sync           # Generate with validation
```

### 6. Quality Gates (MUST PASS)
```
✓ Zero TODOs in src/generated/
✓ cargo check compiles clean
✓ All validate() functions implemented
✓ File size > 100 bytes (detect empty generation)
✓ cargo test passes
```

## Checks

### Ontology Health
```bash
# Check for syntax errors
grep -E "^\s*$|^\s*#" ontology/mcp-domain.ttl

# Verify SPARQL queries reference valid URIs
# Verify no orphaned generation rules in ggen.toml
```

### Generation Verification
```bash
# Must be empty
grep -r "TODO" src/generated/

# Must compile
cargo check

# Must pass tests
cargo test

# Validate file sizes
find src/generated -type f -exec wc -l {} \; | awk '$1 < 2 {print}'
```

## Failure Cases (Andon Cord)

### Generation Produces TODOs
```
Action: FIX template or SPARQL query
Don't: Commit generated code with TODOs
Result: Re-run sync and verify
```

### Compilation Fails
```
Action: Fix ontology → fix SPARQL → fix template
Don't: Edit generated code manually
Result: cargo make sync && cargo check
```

### Test Failures
```
Action: Verify generated code is correct
Action: Update tests if spec changed
Don't: Skip tests, commit broken code
Result: cargo test && cargo make pre-commit
```

### File Size Too Small
```
Action: Verify template generates content
Don't: Ignore empty files (symptom of broken generation)
Result: Debug SPARQL query, rerun sync
```

## Agent Commands
```bash
# Validate ontology
claude-code codegen-agent validate-ontology

# Generate with preview
claude-code codegen-agent preview

# Sync and verify
claude-code codegen-agent sync

# Check quality gates
claude-code codegen-agent check-gates
```

## Output Template
```
## Ontology Update
- Updated: ontology/mcp-domain.ttl (X lines changed)
- Reason: [feature intent]
- Syntax: ✓ Valid Turtle

## SPARQL Query
- Created: queries/my_query.rq
- Triple patterns: [count]
- Syntax: ✓ Valid SPARQL

## Tera Template
- Created: templates/output.rs.tera
- Variables: [list]
- Syntax: ✓ Valid template

## Generation Config
- Added rule to ggen.toml
- Input: queries/my_query.rq
- Output: src/generated/my_output.rs

## Generation Run
- Command: cargo make sync
- Result: ✓ Generated X lines
- File: src/generated/my_output.rs

## Quality Gates
- [x] Zero TODOs
- [x] Compiles clean
- [x] validate() implemented
- [x] File size: 250 bytes (> 100)
- [x] Tests pass

**VERDICT**: ✓ Code generation complete. Ready to test.
```

---

**Philosophy**: Ontology is truth. Everything flows from there. Generation must be deterministic, repeatable, and high-quality.**
