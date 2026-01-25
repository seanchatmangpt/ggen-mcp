# Ontology Sync Workflow - Multi-Step Command

## Purpose

This command guides agents through the complete ontology-driven code generation workflow using `ggen sync`. The workflow loads ontologies, executes SPARQL queries, renders Tera templates, validates generated code, and writes artifacts with cryptographic receipts.

**Core Principle**: Ontology is the single source of truth. All code generation flows from `ontology/mcp-domain.ttl` through SPARQL queries and Tera templates to Rust code.

## Workflow Overview

```
Step 1: Preview Changes (Default) → Step 2: Review Report → Step 3: Apply Changes → Step 4: Verify Receipt → Step 5: Validate Generated Code
```

## Step-by-Step Instructions

### Step 1: Preview Changes (Default Behavior)

**Action**: Run ggen sync in preview mode to see what will be generated without writing files.

```bash
cargo make sync-dry-run
```

**Alternative**: Use ggen directly
```bash
ggen sync --manifest ggen.toml --dry_run true
```

**What this does**:
- Loads ontology from `ontology/mcp-domain.ttl`
- Executes SPARQL queries from `queries/`
- Renders Tera templates from `templates/`
- Validates generated code syntax
- Shows proposed changes without writing files
- Generates preview report in `./ggen.out/reports/latest.md`

**Expected Output**:
```
✅ Loaded ontology: ontology/mcp-domain.ttl
✅ Executed 14 SPARQL queries
✅ Generated 21 files (preview)
✅ Validated: 0 TODO comments found
📄 Preview report: ./ggen.out/reports/latest.md
```

**If this step fails**: Fix ontology/query/template issues before proceeding

**If this step succeeds**: Proceed to Step 2

**CRITICAL**: Always preview first. Preview-by-default prevents accidental overwrites.

### Step 2: Review Preview Report

**Action**: Examine the preview report to verify changes are correct.

```bash
cat ./ggen.out/reports/latest.md
```

**What to check**:
- Files to be created/modified
- Guard verdicts (all should pass)
- Validation results
- No TODO comments in generated code
- Determinism verification (same inputs → same outputs)

**Report sections**:
- **Workspace**: Input ontology path and hash
- **Inputs**: SPARQL queries and templates used
- **Guards**: Validation results (PathSafety, TemplateCompile, etc.)
- **Changes**: Files to be created/modified
- **Validation**: Syntax validation results
- **Performance**: Generation timing

**If report shows issues**: Fix ontology/query/template, return to Step 1

**If report looks good**: Proceed to Step 3

### Step 3: Apply Changes (Explicit Opt-In)

**Action**: Apply the changes by running sync without dry-run flag.

```bash
cargo make sync
```

**Alternative**: Use ggen directly
```bash
ggen sync --manifest ggen.toml
```

**What this does**:
- Executes same workflow as preview
- Writes generated files to `src/generated/`
- Creates cryptographic receipt in `./ggen.out/receipts/sync-*.json`
- Records all hashes (ontology, queries, templates, outputs)

**Expected Output**:
```
✅ Loaded ontology: ontology/mcp-domain.ttl
✅ Executed 14 SPARQL queries
✅ Generated 21 files (2,847 total lines)
✅ Validated: 0 TODO comments found
✅ Created receipt: ./ggen.out/receipts/sync-2026-01-20T12:34:56.json

Generation complete in 0.8s
```

**CRITICAL**: Never edit generated code manually. Only edit:
- `ontology/mcp-domain.ttl` (source of truth)
- `queries/*.rq` (SPARQL queries)
- `templates/*.rs.tera` (Tera templates)

**If this step fails**: Review error messages, fix issues, return to Step 1

**If this step succeeds**: Proceed to Step 4

### Step 4: Verify Receipt

**Action**: Verify the cryptographic receipt to ensure generation integrity.

```bash
# Find latest receipt
ls -t ./ggen.out/receipts/sync-*.json | head -1

# Verify receipt (if verify_receipt tool available)
verify_receipt --receipt ./ggen.out/receipts/sync-*.json
```

**What to verify**:
- Receipt file exists
- All hashes present (ontologyHash, templateHash, artifactHash)
- Receipt signature valid
- No tampering detected

**Receipt structure**:
```json
{
  "receiptId": "sync-2026-01-20T12:34:56",
  "timestamp": "2026-01-20T12:34:56Z",
  "workspaceFingerprint": "abc123...",
  "inputs": {
    "ontology/mcp-domain.ttl": "def456...",
    "queries/aggregates.rq": "789ghi..."
  },
  "outputs": {
    "src/generated/entities.rs": "jkl012..."
  },
  "guards": [
    {"name": "PathSafetyGuard", "verdict": "pass"},
    {"name": "TemplateCompileGuard", "verdict": "pass"}
  ]
}
```

**If receipt invalid**: Regenerate and verify again

**If receipt valid**: Proceed to Step 5

### Step 5: Validate Generated Code

**Action**: Verify generated code compiles and tests pass.

```bash
# Check compilation
cargo make check

# Run tests
cargo make test

# Verify no TODOs in generated code
grep -r "TODO" src/generated/ || echo "✅ No TODOs found"
```

**What to verify**:
- Code compiles without errors
- All tests pass
- No TODO comments in generated code
- Generated code follows project standards
- Determinism: Re-running sync produces identical output

**If validation fails**: 
- Check error messages
- Review ontology/query/template
- Fix issues, return to Step 1

**If validation succeeds**: Workflow complete ✅

## Complete Workflow Example

```bash
# Step 1: Preview
cargo make sync-dry-run
# Output: Preview report generated

# Step 2: Review
cat ./ggen.out/reports/latest.md
# Review: All guards pass, changes look good

# Step 3: Apply
cargo make sync
# Output: 21 files generated, receipt created

# Step 4: Verify Receipt
ls -t ./ggen.out/receipts/sync-*.json | head -1
# Output: Receipt exists and is valid

# Step 5: Validate
cargo make check && cargo make test
# Output: Compilation OK, all tests pass ✅
```

## Integration with Makefile.toml

The workflow integrates with existing Makefile.toml tasks:

**Preview**:
```bash
cargo make sync-dry-run    # Preview without writing
```

**Apply**:
```bash
cargo make sync            # Generate code
```

**Validate Only**:
```bash
cargo make sync-validate   # Pre-flight validation
```

**Force Regenerate**:
```bash
cargo make sync-force      # Overwrite existing files
```

**Full Pipeline**:
```bash
cargo make gen             # Sync + test
```

## Error Handling

### If Ontology Invalid

**Symptoms**: SHACL validation errors, parsing failures

**Fix**:
1. Validate ontology syntax: `cargo make sync-validate`
2. Check SHACL shapes in ontology
3. Fix ontology file
4. Retry sync

### If SPARQL Query Fails

**Symptoms**: Query execution errors, no results

**Fix**:
1. Test query independently
2. Verify SPARQL syntax
3. Check ontology has required data
4. Fix query file
5. Retry sync

### If Template Rendering Fails

**Symptoms**: Template compilation errors, missing variables

**Fix**:
1. Check template syntax
2. Verify all template variables provided by SPARQL query
3. Check for `{{ error() }}` guards triggered
4. Fix template file
5. Retry sync

### If Generated Code Invalid

**Symptoms**: Compilation errors, syntax errors

**Fix**:
1. **DO NOT** edit generated code manually
2. Fix ontology/query/template (source of truth)
3. Regenerate code
4. Verify fix

## Best Practices

1. **Always Preview First**: Use `sync-dry-run` before applying changes
2. **Review Reports**: Check preview reports before applying
3. **Never Edit Generated Code**: Only edit ontology/queries/templates
4. **Verify Receipts**: Check cryptographic receipts for integrity
5. **Test After Sync**: Run tests after every sync
6. **Commit Receipts**: Include receipts in version control
7. **Small Incremental Changes**: Make small ontology changes, sync, verify

## Integration with Other Commands

- **[SPARQL Validation](./sparql-validation.md)** - Validate SPARQL queries before sync
- **[Template Rendering](./template-rendering.md)** - Test templates independently
- **[Code Generation](./code-generation.md)** - Full codegen pipeline workflow
- **[Poka-Yoke Design](./poka-yoke-design.md)** - Prevent errors in ontology design
- **[Kaizen Improvement](./kaizen-improvement.md)** - Incremental ontology improvements
- **[Verify Tests](./verify-tests.md)** - Ensure tests pass after sync

## Documentation References

- **[GGEN_SYNC_INSTRUCTIONS.md](../../GGEN_SYNC_INSTRUCTIONS.md)** - Detailed sync instructions
- **[ggen.toml](../../ggen.toml)** - Configuration file
- **[Makefile.toml](../../Makefile.toml)** - Build tasks
- **[CODE_GENERATION_WORKFLOWS.md](../../docs/CODE_GENERATION_WORKFLOWS.md)** - Workflow examples

## Quick Reference

```bash
# Full workflow
cargo make sync-dry-run              # Step 1: Preview
cat ./ggen.out/reports/latest.md     # Step 2: Review
cargo make sync                      # Step 3: Apply
verify_receipt ./ggen.out/receipts/ # Step 4: Verify
cargo make check && cargo make test  # Step 5: Validate
```
