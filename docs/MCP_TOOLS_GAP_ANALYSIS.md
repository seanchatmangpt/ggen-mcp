# MCP Tools - Documentation vs. Implementation Gap Analysis

**Version**: 1.0.0 | **Date**: 2026-01-20 | **Status**: Findings

---

## Executive Summary

**Gap Identified**: Existing documentation (MCP_TOOL_USAGE.md, WORKFLOW_EXAMPLES.md) describes 5 high-level ontology tools that don't exist in codebase. Actual implementation provides 8 lower-level tools.

**Impact**: Documentation misleads users. Examples reference non-existent tools.

**Recommendation**: Update documentation to reflect actual implementation. Create higher-level tools if needed.

---

## Documented Tools (Not Implemented)

These tools are documented in `docs/MCP_TOOL_USAGE.md` and `docs/WORKFLOW_EXAMPLES.md` but don't exist in source code:

### 1. validate_ontology
**Documented**: SHACL validation with dependency resolution
**Status**: ❌ Not implemented
**Closest Match**: `load_ontology` (partial functionality)

### 2. generate_from_schema
**Documented**: Generate Rust entities from Zod/JSON Schema
**Status**: ❌ Not implemented
**No direct equivalent**

### 3. generate_from_openapi
**Documented**: Generate full API from OpenAPI 3.x spec
**Status**: ❌ Not implemented
**No direct equivalent**

### 4. preview_generation
**Documented**: Dry-run preview of generation
**Status**: ❌ Not implemented
**No direct equivalent**

### 5. sync_ontology
**Documented**: 13-step pipeline (validate → extract → generate → audit)
**Status**: ❌ Not implemented
**Partial match**: Combination of existing tools could achieve this

---

## Actually Implemented Tools

### A. Core Spreadsheet Tools (42 tools)

**Location**: `src/tools/mod.rs`, `src/server.rs`
**Status**: ✅ Fully implemented
**Documentation**: Inline in server.rs BASE_INSTRUCTIONS

<details>
<summary>Complete list (click to expand)</summary>

1. `list_workbooks` - List spreadsheet files
2. `describe_workbook` - Workbook metadata
3. `list_sheets` - Sheet summaries
4. `workbook_summary` - Regions and entry points
5. `sheet_overview` - Narrative overview
6. `sheet_page` - Page through cells
7. `find_value` - Search cell values/labels
8. `read_table` - Structured data extraction
9. `table_profile` - Column/type summary
10. `range_values` - Fetch raw values
11. `sheet_statistics` - Aggregated stats
12. `sheet_formula_map` - Formula group summary
13. `formula_trace` - Precedents/dependents
14. `named_ranges` - List named ranges
15. `scan_volatiles` - Volatile formula scan
16. `find_formula` - Search formulas
17. `workbook_style_summary` - Style summary
18. `sheet_styles` - Sheet styles
19. `get_manifest_stub` - ManifestStub generation
20. `close_workbook` - Evict from cache

</details>

### B. Fork/Write Tools (20 tools)

**Location**: `src/tools/fork.rs`
**Status**: ✅ Fully implemented (feature-gated: `recalc`)
**Documentation**: Inline in server.rs WRITE_INSTRUCTIONS

<details>
<summary>Complete list (click to expand)</summary>

1. `create_fork` - Create editable copy
2. `edit_batch` - Batch value/formula edits
3. `transform_batch` - Range transforms (clear/fill/replace)
4. `style_batch` - Batch style edits
5. `apply_formula_pattern` - Autofill-like formula pattern
6. `structure_batch` - Structural edits (rows/cols/sheets)
7. `get_edits` - List applied edits
8. `get_changeset` - Diff fork vs. original
9. `recalculate` - Trigger LibreOffice recalc
10. `list_forks` - List active forks
11. `discard_fork` - Delete fork without saving
12. `save_fork` - Write fork to file
13. `checkpoint_fork` - Snapshot fork state
14. `list_checkpoints` - List fork checkpoints
15. `restore_checkpoint` - Restore to checkpoint
16. `delete_checkpoint` - Remove checkpoint
17. `list_staged_changes` - List previewed changes
18. `apply_staged_change` - Apply staged change
19. `discard_staged_change` - Discard staged change
20. `screenshot_sheet` - Render sheet range to PNG

</details>

### C. VBA Tools (2 tools)

**Location**: `src/tools/vba.rs`
**Status**: ✅ Fully implemented (feature-gated: VBA_ENABLED)
**Documentation**: Inline in server.rs VBA_INSTRUCTIONS

1. `vba_project_summary` - Parse vbaProject.bin, list modules
2. `vba_module_source` - Return paged module code

### D. Ontology Generation Tools (3 tools)

**Location**: `src/tools/ontology_generation.rs`
**Status**: ✅ Fully implemented
**Documentation**: ❌ Missing comprehensive docs

1. **render_template**
   - Renders Tera template with context data
   - Parameters: template (name or inline), context (JSON), validation flags
   - Response: rendered output, warnings, hash
   - Use case: Code generation from templates

2. **write_generated_artifact**
   - Writes generated code with validation and audit trail
   - Parameters: content, output_path, backup flag, hashes, metadata
   - Response: written status, hash, receipt_id, backup_path
   - Use case: Safe file writing with provenance tracking

3. **validate_generated_code**
   - Multi-language validation (Rust, TypeScript, JSON, YAML)
   - Parameters: code, language, file_name, golden_file_path, strict_mode
   - Response: validation status, errors, warnings, diff
   - Use case: Ensure generated code quality

### E. Ontology SPARQL Tools (2 tools)

**Location**: `src/tools/ontology_sparql.rs`
**Status**: ✅ Fully implemented
**Documentation**: ❌ Missing comprehensive docs

1. **load_ontology**
   - Load Turtle ontology → validate → cache → return stats
   - Parameters: path, validate flag, base_iri
   - Response: ontology_id, triple count, validation results
   - Use case: Load RDF ontology for SPARQL queries

2. **execute_sparql_query**
   - Execute SPARQL query against loaded ontology
   - Parameters: ontology_id, query, cache options, timeout
   - Response: results (JSON), query analysis, cache status
   - Use case: Extract data from ontology

---

## Gap Analysis

### Critical Gaps

| Documented Tool | Actual Tooling | Gap Severity |
|----------------|----------------|--------------|
| `validate_ontology` | `load_ontology` (partial) | ⚠️ **Medium** - Missing SHACL validation |
| `generate_from_schema` | None | 🔴 **High** - No schema → entity generation |
| `generate_from_openapi` | None | 🔴 **High** - No OpenAPI generation |
| `preview_generation` | None | 🟡 **Low** - Could use dry-run flags |
| `sync_ontology` | Combination of tools | ⚠️ **Medium** - No orchestrator |

### Functional Gaps

1. **Schema-to-Entity Generation**
   - Missing: Zod/JSON Schema parsing
   - Missing: Entity code generation from schema
   - Missing: Validation rule derivation

2. **OpenAPI Generation**
   - Missing: OpenAPI 3.x parsing
   - Missing: Handler generation
   - Missing: Model generation
   - Missing: Validator generation

3. **Orchestration Pipeline**
   - Missing: 13-step sync pipeline
   - Missing: Dependency resolution
   - Missing: Compilation checking
   - Missing: Test execution
   - Missing: Audit receipt generation

4. **Preview/Dry-Run**
   - Missing: Unified preview mode across tools
   - Missing: Diff computation for existing files

---

## Recommendations

### Short-Term (Correct Documentation)

1. **Update MCP_TOOL_USAGE.md**
   - Remove non-existent tool documentation
   - Add documentation for actual tools (render_template, write_generated_artifact, validate_generated_code, load_ontology, execute_sparql_query)
   - Add comprehensive parameter schemas and examples

2. **Update WORKFLOW_EXAMPLES.md**
   - Replace examples using non-existent tools
   - Create real workflows using actual tools
   - Example: "Ontology → SPARQL → Template → Generated Code" workflow

3. **Update CLAUDE.md**
   - Remove references to non-existent tools
   - Add correct tool invocation examples

4. **Create New Docs**
   - `ONTOLOGY_TOOLS_REFERENCE.md` - Detailed reference for 5 ontology tools
   - `CODE_GENERATION_WORKFLOWS.md` - Real workflows end-to-end

### Medium-Term (Implement Missing Tools)

1. **Implement validate_ontology**
   - Wrap `load_ontology` with SHACL validation
   - Add dependency resolution
   - Add error reporting

2. **Implement preview_generation**
   - Add dry-run mode to render_template
   - Add diff computation
   - Add file existence checking

3. **Create Orchestration Tool (sync_ontology)**
   - Pipeline coordinator
   - Uses: load_ontology → execute_sparql_query → render_template → validate_generated_code → write_generated_artifact
   - Add compilation checking (cargo check)
   - Add audit receipt generation

### Long-Term (Schema/OpenAPI Generation)

1. **Implement generate_from_schema**
   - Zod parser
   - JSON Schema parser
   - Entity template library
   - Validation rule generator

2. **Implement generate_from_openapi**
   - OpenAPI 3.x parser
   - Handler generator
   - Model generator
   - Test generator

---

## Migration Path

### Phase 1: Documentation Accuracy (1-2 days)
- ✅ Gap analysis (this document)
- ⏳ Update MCP_TOOL_USAGE.md
- ⏳ Update WORKFLOW_EXAMPLES.md
- ⏳ Update CLAUDE.md
- ⏳ Create ONTOLOGY_TOOLS_REFERENCE.md

### Phase 2: Missing Tool Implementations (1-2 weeks)
- Implement validate_ontology wrapper
- Implement preview_generation
- Implement sync_ontology orchestrator
- Add comprehensive tests

### Phase 3: Advanced Features (4-6 weeks)
- Implement generate_from_schema
- Implement generate_from_openapi
- Integration testing
- Performance optimization

---

## Tool Mapping

### How to Achieve Documented Workflows with Actual Tools

#### Documented: validate_ontology
```json
{
  "tool": "validate_ontology",
  "arguments": {
    "ontology_path": "ontology/mcp-domain.ttl",
    "strict_mode": true
  }
}
```

**Actual Equivalent**:
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

#### Documented: sync_ontology (13 steps)
**Actual Equivalent** (manual orchestration):
```bash
# Step 1: Load ontology
load_ontology { path: "ontology/mcp-domain.ttl", validate: true }

# Step 2: Execute SPARQL query
execute_sparql_query {
  ontology_id: "<id from step 1>",
  query: "SELECT * WHERE { ?s ?p ?o }"
}

# Step 3: Render template with query results
render_template {
  template: "templates/entities.rs.tera",
  context: { "results": "<results from step 2>" },
  validate_syntax: true
}

# Step 4: Validate generated code
validate_generated_code {
  code: "<output from step 3>",
  language: "rust",
  file_name: "entities.rs"
}

# Step 5: Write artifact
write_generated_artifact {
  content: "<validated code from step 4>",
  output_path: "src/generated/entities.rs",
  create_backup: true
}
```

---

## Conclusion

**Current State**: Documentation describes aspirational high-level API. Implementation provides low-level building blocks.

**Desired State**: Either:
1. Update documentation to match implementation (short-term, recommended)
2. Implement missing high-level tools (medium-term)
3. Both (ideal)

**Next Steps**:
1. Create accurate documentation (this PR)
2. File issues for missing tool implementations
3. Prioritize orchestration tools (validate_ontology, sync_ontology)
4. Defer schema/OpenAPI generation to future milestone

---

**Version**: 1.0.0 | **Author**: Gap Analysis | **Status**: ✅ Complete
