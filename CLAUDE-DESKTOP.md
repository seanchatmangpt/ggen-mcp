# CLAUDE-DESKTOP.md - ggen-mcp Operations Standard (TPS × CDCS v9.0)

## System Architecture

**Project Type**: Ontology-Driven Meta-Circular (CDCS v9.0 Pattern)
**Source of Truth**: `/Users/sac/ggen-mcp/ggen-mcp.ttl` (DDD ontology)
**Generation**: `ggen sync` → writes directly to `src/`
**Quality**: TPS integrated (Andon, Jidoka, Single Piece Flow)

## Standardized Work Sequence

**Takt Time**: 30 seconds (ontology modification → sync → verification)

### Station 1: Context Regeneration (v9.0 Source-Based)
```bash
# v9.0: Generate context from ontology, not cached session
desktop-commander:read_file /Users/sac/ggen-mcp/ggen-mcp.ttl
desktop-commander:read_file /Users/sac/ggen-mcp/ggen.toml
desktop-commander:list_directory /Users/sac/ggen-mcp/src/domain
desktop-commander:list_directory /Users/sac/ggen-mcp/src/application
```

**Regenerative Understanding**:
1. Query ontology for defined components (aggregates, commands, events, etc.)
2. Check src/ for generated implementation
3. Calculate completion: N/10 files present
4. Identify next component from priority sequence
5. Present: "ggen-mcp: N/10 complete. Ontology defines [missing]. Next: [component]"

**NOT session cache replay** - reconstruct from source of truth

### Station 2: Ontology Enhancement (Value-Add)
```bash
desktop-commander:write_file ggen-mcp.ttl
```

**Standard**: Add ONE complete DDD component
- All properties with `ddd:type`
- All invariants with `ddd:check` (executable Rust code)
- All relationships (hasProperty, forAggregate, handles, etc.)

**Andon Trigger**: Pull if component incomplete

### Station 3: Query/Template Pair (Synchronized Work)
```bash
desktop-commander:write_file queries/[component].rq
desktop-commander:write_file templates/[component].rs.tera
```

**Standard**: Query MUST extract ALL variables template uses
**Jidoka**: Template MUST have `{{ error() }}` guard for missing data

### Station 4: Configuration (Kanban Signal)
```bash
desktop-commander:edit_block ggen.toml
```

**Standard**: Add [[generation.rules]] entry
**Pull System**: Config authorizes generation

### Station 5: Code Generation (Automation)
```bash
mac-shell:execute_command ggen sync --manifest /Users/sac/ggen-mcp/ggen.toml
```

**Andon**: Stop immediately on failure, never continue with partial
**Single Piece Flow**: One component per cycle

### Station 6: Quality Verification (Poka-Yoke)
```bash
mac-shell:execute_command grep -r "TODO" /Users/sac/ggen-mcp/src/
mac-shell:execute_command cargo check --manifest-path /Users/sac/ggen-mcp/Cargo.toml
desktop-commander:read_file /Users/sac/ggen-mcp/src/domain/[component].rs
```

**Go/No-Go**:
- Zero TODO (hard requirement)
- Zero compile errors (hard requirement)
- validate() has actual code (hard requirement)
- File size > 100 bytes (detect empty generation)

### Station 7: Session Documentation (Kaizen)
```bash
desktop-commander:write_file .claude/desktop/sessions/SESSION_{{utc_timestamp}}.md
```

**Record**: Decision rationale, next priority, blockers

## TPS Quality Gates

### Andon Cord (Stop the Line)

**Pull immediately if**:
- ggen sync fails → Fix ontology before proceeding
- Template error → Fix query/template mismatch
- TODO in generated code → Ontology incomplete
- cargo check fails → Type definition missing
- File count ≠ 10 at completion → Generation incomplete

**Resolution**: Fix root cause in ontology, NEVER work around in generated code

### Jidoka (Built-in Quality)

**Templates enforce completeness**:
```tera
{% if properties | length == 0 %}
{{ error("Aggregate " ~ aggregate_name ~ " has no properties. Add ddd:hasProperty to ontology.") }}
{% endif %}
```

**Automatic verification**:
- Rust compiler validates generated code
- SPARQL validates query syntax
- Turtle parser validates ontology

**Human judgment reserved for**: Ontology design decisions, architecture trade-offs

### Kanban Board

```
TODO              IN PROGRESS       DONE
────────────────────────────────────────
ValueObject       —                 ✓ Aggregate
Repository                          ✓ Command
Service
Handler
Policy
Event
Domain Mod
Application Mod
```

**WIP Limit**: 1 (Single Piece Flow)

## CDCS v9.0 Integration

### Meta-Circular Verification (Required Every 5 Files)

**Test**:
1. Add property to `ggen:Ontology` in ggen-mcp.ttl
2. Run `ggen sync`
3. Verify `src/domain/aggregates.rs` regenerated
4. Confirm system can use new property

**Pass Criteria**: System regenerates itself from own ontology

### Generative Recovery Protocol

**On /continue**:
1. Read ontology (source of truth)
2. Query current state (SPARQL if needed)
3. Verify src/ matches ontology generation state
4. If mismatch: Report desync, suggest sync
5. Present: "N/10 complete. Next: [component from priority]"

**NOT**: Read cached session → replay conversation

### Compound Intelligence (26x Performance)

**Pattern Recognition**: Reuse established patterns
- Value Object: String wrapper with validation
- Repository: trait with find_by_id, save, delete
- Service: operations with business logic
- Handler: command → events transformation

**SPR Compression**: Use established patterns, don't reinvent
**Predictive Loading**: Next component known from sequence

## Component Priority Sequence

**Phase 1** (Foundation):
1. ValueObject (OntologyId, ReceiptId, SPARQLQuery, Template)
2. Event (complete OntologyLoaded, CodeGenerated)

**Phase 2** (Persistence):
3. Repository (OntologyRepository, ReceiptRepository traits)

**Phase 3** (Operations):
4. Service (OntologyService, GenerationService)
5. Handler (LoadOntologyHandler, GenerateCodeHandler)

**Phase 4** (Policy):
6. Policy (CompletenessPolicy, DeterminismPolicy)

**Phase 5** (Integration):
7. Domain Mod (exports all domain types)
8. Application Mod (exports all application types)

**Phase 6** (MCP):
9. Tools (list_ontologies, query_ontology, sync)
10. Server Integration (wire AppState, register tools)

## MCP Tool Selection

**File Operations** → desktop-commander (reliable, efficient)
**Command Execution** → mac-shell (ggen, cargo not in container)
**Directory Operations** → desktop-commander (native)

## 5S Workspace

**Seiri**: No `generated/` folder - writes to src/ directly
**Seiton**: Standard paths (queries/, templates/, src/)
**Seiso**: Remove SESSION_*.md from root (use .claude/desktop/sessions/)
**Seiketsu**: This document
**Shitsuke**: Update session after each component

## Success Criteria (v1.0)

- [ ] 10 files in src/ (6 domain, 3 application, 1 lib)
- [ ] grep -r "TODO" returns empty
- [ ] cargo build succeeds
- [ ] Meta-circular test passes
- [ ] All validate() have actual code
- [ ] MCP tools registered (v2.0)

## Operational Commands

- `O:[change]` - Modify ontology
- `G` - ggen sync
- `S` - Status (ontology query + src/ verification)
- `M:[tool]` - MCP tool implementation
- `F` - Finalize (commit)

## Session Continuity

**Input**: "Read CLAUDE-DESKTOP.md and continue ggen-mcp"

**Response**:
1. Execute Station 1 (context regeneration from ontology)
2. Present current state (N/10, next component)
3. Ask: "Continue with [next component]?"
4. On confirmation: Execute Stations 2-7

**Output**: Updated ontology, generated code, session doc

---

**System regenerates context from source of truth. Ontology IS the code. TPS ensures quality. CDCS v9.0 ensures continuity.**