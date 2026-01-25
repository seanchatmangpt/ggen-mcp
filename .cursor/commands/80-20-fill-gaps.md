# 80/20 Fill the Gaps - Capability Completion Workflow

## Purpose

This command enables agents to autonomously scan the codebase using 80/20 thinking, identify incomplete capabilities, finish them, validate them, and determine next steps. The agent uses the full context window to make strategic decisions and implements without asking for confirmation.

**ggen-mcp Context**: Scan ontology, SPARQL queries, templates, and generated code to identify incomplete capabilities in the code generation pipeline. Focus on ontology-driven improvements that compound into better generated code.

## Core Principle: 80/20 Thinking

**The 80/20 rule**: 20% of capabilities deliver 80% of value. Value includes quality, consistency, and maintainability - these are not optional. Focus on high-impact capabilities that provide maximum value while maintaining quality standards.

**Quality-First Principle (DfLSS Alignment)**:
- **Quality is HIGH VALUE**, not optional - design for quality from the start
- **Consistency is HIGH VALUE** - using project language (Rust in Rust project) is high value, not "extra effort"
- **Maintainability is HIGH VALUE** - code that's easy to maintain prevents defects and reduces technical debt
- **Prevent defects AND waste** - don't fix them later (DfLSS principle - addresses both efficiency and quality)
- Quality work may require more effort, but it's still high value because it prevents defects, maintains consistency, and improves maintainability

**Above-AGI thinking**:
- Use **full context window** to scan entire codebase (ontology, queries, templates, generated code)
- Identify **incomplete capabilities** (not just bugs) - missing ontology properties, incomplete SPARQL queries, template gaps
- Prioritize by **impact and value** (80/20 matrix) - where value includes quality, consistency, maintainability
- **Finish capabilities** completely with quality standards
- **Validate** implementations thoroughly (sync, test, verify)
- **Determine next steps** strategically

## Workflow Overview

```
Step 1: 80/20 Scan → Step 2: Identify Incomplete Capabilities → Step 3: Finish Capabilities → Step 4: Validate → Step 5: Next Steps
```

## Step-by-Step Instructions

### Step 1: 80/20 Scan

**Action**: Rapidly scan the codebase to identify incomplete capabilities using 80/20 thinking.

#### 1.1: Quick Context Scan

**Action**: Use full context window to scan codebase efficiently.

**Scan targets** (ggen-mcp specific):
- **Ontology** (`ontology/mcp-domain.ttl`) - Look for incomplete entity definitions, missing properties, incomplete SHACL shapes
- **SPARQL queries** (`queries/*.rq`) - Look for incomplete queries, missing variables, unbounded queries
- **Templates** (`templates/*.rs.tera`) - Look for incomplete templates, missing error guards, incomplete variable handling
- **Generated code** (`src/generated/*.rs`) - Look for incomplete implementations (but fix via ontology, not directly)
- **Source files** (`src/**/*.rs`) - Look for incomplete implementations
- **Test files** (`tests/**/*.rs`) - Look for missing test coverage
- **Configuration** (`ggen.toml`, `Makefile.toml`) - Look for incomplete features

**Action**: Scan systematically

```bash
# Quick scan for incomplete capabilities
grep -r "TODO\|FIXME\|unimplemented\|incomplete\|partial" ontology/ --include="*.ttl"
grep -r "TODO\|FIXME\|unimplemented\|incomplete\|partial" queries/ --include="*.rq"
grep -r "TODO\|FIXME\|unimplemented\|incomplete\|partial" templates/ --include="*.tera"
grep -r "TODO\|FIXME\|unimplemented\|incomplete\|partial" src/ --include="*.rs"

# Check ontology completeness
grep -c "mcp:description" ontology/mcp-domain.ttl  # Count entities with descriptions
grep -c "sh:property" ontology/mcp-domain.ttl      # Count SHACL properties

# Check SPARQL query completeness
grep -c "LIMIT" queries/*.rq                       # Count queries with LIMIT
grep -c "SELECT" queries/*.rq                        # Count SELECT queries

# Check template completeness
grep -c "{{ error() }}" templates/*.tera            # Count templates with error guards
```

**Tool usage**: Use `grep`, `codebase_search`, `read_file` to quickly identify incomplete capabilities.

#### 1.2: Identify Capability Patterns

**Action**: Look for patterns that indicate incomplete capabilities.

**Capability indicators** (ggen-mcp specific):
1. **Incomplete ontology** - Entities missing properties, incomplete SHACL shapes
2. **Incomplete SPARQL queries** - Missing LIMIT clauses, unbounded queries, missing variables
3. **Incomplete templates** - Missing error guards, incomplete variable handling
4. **Incomplete error handling** - Error paths not fully handled
5. **Incomplete type safety** - Types that could be more type-safe (newtypes)
6. **Incomplete tests** - Code without tests, missing integration tests
7. **Incomplete validation** - Missing 4-layer validation, incomplete guards

**Action**: Create capability inventory

```rust
// Example: Identify incomplete capabilities (ggen-mcp)
// 1. Entity missing description property - incomplete ontology
// 2. SPARQL query missing LIMIT clause - incomplete query safety
// 3. Template missing {{ error() }} guard - incomplete template safety
// 4. Generated code uses String instead of EntityName newtype - incomplete type safety
// 5. Missing ontology sync integration test - incomplete testing
```

---

### Step 2: Identify Incomplete Capabilities

**Action**: Identify capabilities that are incomplete and prioritize by 80/20.

#### 2.1: Capability Categories

**Action**: Categorize incomplete capabilities.

**Categories** (ggen-mcp specific):
1. **Ontology completeness** - Missing properties, incomplete SHACL shapes
2. **SPARQL safety** - Missing LIMIT clauses, unbounded queries, injection risks
3. **Template safety** - Missing error guards, incomplete variable handling
4. **Type safety** - Incomplete type safety (e.g., `String` instead of newtype)
5. **Validation** - Incomplete validation (e.g., missing 4-layer validation)
6. **Testing** - Incomplete test coverage (e.g., missing integration tests)
7. **Code generation** - Incomplete codegen pipeline (e.g., missing receipts)

**Action**: List incomplete capabilities

```markdown
## Incomplete Capabilities (ggen-mcp)

### Ontology Completeness
- Entity missing description property (should have description)
- Missing SHACL shape for entity validation
- Incomplete property definitions

### SPARQL Safety
- Query missing LIMIT clause (should have LIMIT)
- Unbounded query (should be bounded)
- Missing variable extraction

### Template Safety
- Template missing {{ error() }} guard (should have guard)
- Incomplete variable handling
- Missing template documentation

### Type Safety
- String used instead of EntityName newtype (should use newtype)
- Bare String IDs (should use newtypes)
- Runtime validation where compile-time possible

### Validation
- Missing 4-layer validation (should have all layers)
- Incomplete input guards
- Missing receipt verification

### Testing
- Missing ontology sync integration test
- Missing SPARQL query tests
- Missing template rendering tests

### Code Generation
- Missing receipt generation
- Incomplete audit trail
- Missing golden file comparison
```

#### 2.2: 80/20 Prioritization

**Action**: Prioritize capabilities by impact and value (where value includes quality, consistency, maintainability).

**80/20 Matrix** (Quality-First):
- **High Impact, High Value** (Quality Work) - Finish first - Quality, consistency, and maintainability are high value
- **High Impact, Medium Value** (Good Work) - Plan carefully - May require more effort but maintains quality
- **Low Impact, High Value** (Foundation Work) - Do when convenient - Quality foundations prevent future problems
- **Low Impact, Low Value** (Avoid) - Don't do - Not worth the effort

**Value Includes**:
- **Quality**: Code that works correctly, handles errors, follows patterns
- **Consistency**: Uses project language, follows project conventions, maintains patterns
- **Maintainability**: Easy to understand, modify, and extend
- **Prevention**: Prevents defects and waste rather than fixing them later (DfLSS)

**Action**: Prioritize capabilities

```markdown
## Top 20% Capabilities (80% of Value - Quality First)

### High Impact, High Value (Quality Work - Do First)
1. Add LIMIT clauses to all SPARQL queries - Prevents unbounded queries, maintains query safety
2. Add {{ error() }} guards to all templates - Prevents rendering errors, maintains template safety
3. Add description properties to all entities - Improves API clarity, maintains consistency
4. Add EntityName newtype - Prevents type errors, maintains type safety

### High Impact, Medium Value (Good Work - Plan)
5. Add 4-layer validation to all codegen paths - Complete validation, maintains quality
6. Add integration tests for ontology sync - Complete test coverage, maintains quality standards

### Foundation Work (High Value, Lower Impact)
7. Add SHACL shapes to all entities - Incremental improvement, maintains consistency
8. Add receipt verification to all syncs - Incremental improvement, maintains quality
```

---

### Step 3: Finish Capabilities

**Action**: Complete incomplete capabilities without asking for confirmation.

#### 3.1: Implementation Strategy

**Action**: Finish capabilities systematically.

**Implementation order** (ggen-mcp specific):
1. **Ontology improvements first** - Fix ontology, regenerate code (compounds into better generated code)
2. **SPARQL safety** - Add LIMIT clauses, validate queries
3. **Template safety** - Add error guards, validate variables
4. **Type safety** - Add newtypes, improve validation
5. **Testing** - Add integration tests, verify behavior

**Action**: Implement fixes

```turtle
# Example: Finish ontology capability
# BEFORE: Incomplete (missing description)
mcp:Tool
    a owl:Class ;
    mcp:name ?name ;
    mcp:inputSchema ?inputSchema .

# AFTER: Complete (added description)
mcp:Tool
    a owl:Class ;
    mcp:name ?name ;
    mcp:description ?description ;  # Added description property
    mcp:inputSchema ?inputSchema .
```

```sparql
# Example: Finish SPARQL query capability
# BEFORE: Incomplete (missing LIMIT)
SELECT ?entity ?name
WHERE {
    ?entity a mcp:Entity .
    ?entity mcp:name ?name .
}

# AFTER: Complete (added LIMIT)
SELECT ?entity ?name
WHERE {
    ?entity a mcp:Entity .
    ?entity mcp:name ?name .
}
LIMIT 1000  # Added LIMIT clause
```

```tera
{# Example: Finish template capability #}
{# BEFORE: Incomplete (missing error guard) #}
pub struct {{ entity_name }} {
    // ...
}

{# AFTER: Complete (added error guard) #}
{% if entity_name %}
pub struct {{ entity_name }} {
    // ...
}
{% else %}
{{ error("entity_name required") }}
{% endif %}
```

#### 3.2: Capability Completion Checklist

**Action**: Ensure capabilities are fully complete.

**Checklist** (ggen-mcp specific):
- [ ] Ontology complete (all entities have required properties)
- [ ] SPARQL queries safe (all have LIMIT clauses)
- [ ] Templates safe (all have error guards)
- [ ] Type safety complete (newtypes used where appropriate)
- [ ] Validation complete (4-layer validation in place)
- [ ] Tests complete (integration tests added)
- [ ] All tests pass: `cargo make test`
- [ ] Code compiles: `cargo make check`
- [ ] Ontology sync works: `cargo make sync`
- [ ] Generated code compiles: `cargo make check`

#### 3.3: Batch Completion

**Action**: Complete multiple capabilities in parallel when possible.

**Batching strategy** (ggen-mcp specific):
- **Related capabilities** - Group related completions together (e.g., all ontology improvements)
- **Independent capabilities** - Can be done in parallel (e.g., add LIMIT to all queries)
- **Dependent capabilities** - Complete in order (e.g., ontology → SPARQL → template → code)

**Example batch**:
```bash
# Batch 1: SPARQL safety completions (all independent)
# - Add LIMIT to queries/aggregates.rq
# - Add LIMIT to queries/commands.rq
# - Add LIMIT to queries/tools.rq
# All can be completed together

# Batch 2: Template safety completions (all independent)
# - Add {{ error() }} guard to templates/entity.rs.tera
# - Add {{ error() }} guard to templates/tool.rs.tera
# All can be completed together
```

---

### Step 4: Validate

**Action**: Validate that capabilities are complete and working correctly.

#### 4.1: Functional Validation

**Action**: Ensure capabilities work as intended.

**Validation steps** (ggen-mcp specific):
1. **Preview sync** - `cargo make sync-dry-run` (preview changes)
2. **Apply sync** - `cargo make sync` (apply changes)
3. **Compile** - `cargo make check`
4. **Test** - `cargo make test`
5. **Lint** - `cargo make lint`
6. **Format** - `cargo make fmt`
7. **Integration** - Run integration tests

**Action**: Run validation

```bash
# Full validation (ggen-mcp)
cargo make sync-dry-run          # Preview ontology sync
cargo make sync                  # Apply sync
cargo make check                 # Verify compilation
cargo make test                  # Run tests
cargo make lint                  # Lint code
cargo make fmt                   # Format code

# Verify specific capabilities
cargo make test test_ontology_sync
cargo make test test_sparql_query
cargo make test test_template_rendering
```

#### 4.2: Capability Validation

**Action**: Verify each capability is complete.

**Validation criteria** (ggen-mcp specific):
- ✅ **Ontology** - All entities have required properties
- ✅ **SPARQL** - All queries have LIMIT clauses
- ✅ **Templates** - All templates have error guards
- ✅ **Type safety** - Newtypes used where appropriate
- ✅ **Validation** - 4-layer validation in place
- ✅ **Testing** - Integration tests verify behavior
- ✅ **Usage** - Capability is usable

**Action**: Validate each capability

```markdown
## Capability Validation (ggen-mcp)

### SPARQL LIMIT clauses - ✅ COMPLETE
- ✅ All queries have LIMIT clauses
- ✅ Queries validated
- ✅ Tests pass
- ✅ Usage verified

### Template error guards - ✅ COMPLETE
- ✅ All templates have {{ error() }} guards
- ✅ Guards tested
- ✅ Validation complete

### Entity descriptions - ✅ COMPLETE
- ✅ All entities have description properties
- ✅ Generated code includes descriptions
- ✅ API clarity improved
```

---

### Step 5: Next Steps

**Action**: Determine what to do next based on completed capabilities.

#### 5.1: Assess Completion Status

**Action**: Evaluate what's been completed and what remains.

**Assessment** (ggen-mcp specific):
- **Completed capabilities** - What was finished
- **Remaining capabilities** - What's left
- **Blocked capabilities** - What's blocked
- **Future capabilities** - What could be added

**Action**: Create next steps plan

```markdown
## Next Steps (ggen-mcp)

### Immediate (High Priority)
1. ✅ SPARQL LIMIT clauses - COMPLETE
2. ✅ Template error guards - COMPLETE
3. ✅ Entity descriptions - COMPLETE
4. ✅ EntityName newtype - COMPLETE

### Next (Medium Priority)
5. 4-layer validation - In progress
6. Integration tests - In progress

### Future (Lower Priority)
7. SHACL shapes for all entities - Plan for later
8. Receipt verification - Incremental
```

#### 5.2: Strategic Next Steps

**Action**: Determine strategic next steps using 80/20 thinking (quality-first).

**Next steps criteria**:
1. **Impact** - How much value does this provide? (Value includes quality, consistency, maintainability)
2. **Value** - Does this maintain quality standards? Does it maintain consistency? Does it improve maintainability?
3. **Dependencies** - What does this unblock?
4. **Risk** - What's the risk of not doing this? (Quality risks, consistency risks, maintainability risks)

**Action**: Prioritize next steps

```markdown
## Strategic Next Steps (80/20 - Quality First)

### High Impact, High Value (Do Next - Quality Work)
1. Complete 4-layer validation for all codegen paths
   - Impact: HIGH (prevents errors)
   - Value: HIGH (quality, consistency)
   - Quality: Maintains validation standards

2. Add integration tests for ontology sync
   - Impact: HIGH (test coverage)
   - Value: HIGH (quality, prevents defects)
   - Quality: Maintains test quality standards

3. Add SHACL shapes to all entities
   - Impact: HIGH (validation)
   - Value: HIGH (quality, consistency)
   - Quality: Maintains ontology quality standards

### High Impact, Medium Value (Plan - Good Work)
4. Add receipt verification to all syncs
   - Impact: HIGH (provenance)
   - Value: MEDIUM (quality, consistency)
   - Plan: Incremental with quality checks

### Foundation Work (High Value, Lower Impact)
5. Additional template documentation
   - Impact: MEDIUM
   - Value: HIGH (maintainability, quality)
   - Do when convenient
```

#### 5.3: Capability Roadmap

**Action**: Create roadmap for remaining capabilities.

**Roadmap structure**:
- **Completed** - What's done
- **In Progress** - What's being worked on
- **Planned** - What's planned
- **Future** - What could be done

**Action**: Create roadmap

```markdown
## Capability Roadmap (ggen-mcp)

### Completed ✅
- SPARQL LIMIT clauses
- Template error guards
- Entity descriptions
- EntityName newtype

### In Progress 🚧
- 4-layer validation
- Integration tests

### Planned 📋
- SHACL shapes for all entities
- Receipt verification

### Future 🔮
- Additional type safety improvements
- Performance optimizations
- Template macro extraction
```

---

## Complete Workflow Example (ggen-mcp)

```bash
# Step 1: 80/20 Scan
# - Scanned ontology (42KB)
# - Scanned 14 SPARQL queries
# - Scanned 21 templates
# - Found 8 incomplete capabilities

# Step 2: Identify Incomplete Capabilities
# - Categorized 8 capabilities
# - Prioritized by 80/20
# - Selected top 4 (80% of value)

# Step 3: Finish Capabilities
# - Added LIMIT clauses to all SPARQL queries
# - Added {{ error() }} guards to all templates
# - Added description properties to all entities
# - Added EntityName newtype

# Step 4: Validate
# - Preview sync: ✅
# - Apply sync: ✅
# - All tests pass: ✅
# - Code compiles: ✅
# - Capabilities verified: ✅

# Step 5: Next Steps
# - Completed: 4 capabilities
# - In progress: 2 capabilities
# - Planned: 2 capabilities
# - Next: Complete 4-layer validation
```

## Integration with Other Commands

- **[Ontology Sync](./ontology-sync.md)** - Verify sync before completing capabilities
- **[SPARQL Validation](./sparql-validation.md)** - Validate queries before completing
- **[Template Rendering](./template-rendering.md)** - Validate templates before completing
- **[Code Generation](./code-generation.md)** - Verify codegen pipeline before completing
- **[Poka-Yoke Design](./poka-yoke-design.md)** - Use to complete type safety capabilities
- **[Kaizen Improvement](./kaizen-improvement.md)** - Use for incremental improvements
- **[Verify Tests](./verify-tests.md)** - Verify tests before completing capabilities

## Expert Insights

**Why this matters**: Incomplete capabilities accumulate technical debt. Finishing capabilities completely prevents bugs and improves code quality.

**ggen-mcp specific**: 
- **Ontology-first**: Fix ontology, regenerate code - compounds into better generated code
- **Template-driven**: Fix templates, regenerate all artifacts - compound effect
- **Incremental**: Small ontology changes → small code changes → low risk

**Key principle**: "80/20 thinking" - Focus on completing the 20% of capabilities that deliver 80% of value. Value includes quality, consistency, and maintainability - these are not optional. Quality work may require more effort, but it's still high value.

**Above-AGI thinking**: Use the full context window to make comprehensive decisions. Think strategically about impact and value (where value includes quality, consistency, maintainability). Finish capabilities completely with quality standards without asking for confirmation.

**Remember**: 
- **Quality first** - Quality, consistency, and maintainability are high value, not optional
- **Finish completely** - Don't leave capabilities half-done - complete with quality standards
- **Validate thoroughly** - Ensure capabilities work correctly and maintain quality
- **Strategic next steps** - Plan what to do next based on 80/20 value (including quality)
- **Ontology-driven** - Fix ontology/templates, not generated code

**80/20 principle**: 20% of capabilities deliver 80% of value. Value includes quality, consistency, and maintainability. Complete those first while maintaining quality standards.

**DfLSS Alignment**: Design for Lean Six Sigma - addresses both efficiency (Lean waste elimination) AND quality (Six Sigma defect prevention) from the start. Prevent defects AND waste rather than fixing them later. Maintain consistency (e.g., Rust in Rust project). Quality and efficiency are foundational value, not optional.

**Autonomous execution**: Once capabilities are identified and prioritized, finish them without asking. The agent has full context and can make informed decisions. Always prioritize quality, consistency, and maintainability.
