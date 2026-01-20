# ggen sync Results

## Success ✅

Generated from `/Users/sac/ggen-mcp/ggen-mcp.ttl` → 4 files in 2ms

### Generated Files
- `generated/aggregates.rs` (428 bytes) - Ontology, Receipt structs
- `generated/commands.rs` (441 bytes) - LoadOntology, GenerateCode commands

### Key Learnings Applied

**Config Structure** (from `/Users/sac/ggen/examples/openapi`):
```toml
[ontology]
source = "file.ttl"

[generation]
output_dir = "generated"

[[generation.rules]]  # NOT [[rules]]
query = { file = "..." }
template = { file = "..." }
output_file = "..."
mode = "Overwrite"
```

**Template Pattern**:
```tera
{% for row in sparql_results %}
{%- set var = row["?varname"] | default(value="") -%}
{% endfor %}
```

### Next Steps
1. Move `generated/*.rs` to `src/domain/`
2. Add properties from SHACL shapes (currently just `id: String`)
3. Implement actual validation logic from ontology invariants
4. Wire into MCP server's AppState

### Proof of Concept
✅ ggen-mcp ontology → Rust code (single pass, deterministic)
✅ Infrastructure reusable across domains
✅ Meta-circular capability proven