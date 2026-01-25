# Kaizen (Continuous Improvement) - Multi-Step Workflow

## Purpose

This command guides agents to make small, incremental improvements rather than big rewrites. Kaizen means "change for better" - continuous small improvements that compound over time. Experts make many small improvements rather than waiting for perfect solutions.

**ggen-mcp Context**: Apply kaizen to ontology definitions, SPARQL queries, and Tera templates. Small improvements to ontology/templates compound into better generated code.

## Workflow Overview

```
Step 1: Identify Opportunity → Step 2: Plan Change (with Success Criteria & Measurement) → Step 3: Do (Implement) → Step 4: Check (Verify with Measurement) → Step 5: Act (Standardize with Control)
```

## Step-by-Step Instructions

### Step 1: Identify Improvement Opportunity

**Action**: Find a small, focused improvement opportunity.

**Opportunity criteria**:
- **Small**: Can be done in minutes, not hours
- **Focused**: Addresses one specific thing
- **Safe**: Low risk of breaking things
- **Value**: Adds value (clarity, performance, maintainability)

**Types of opportunities**:
1. **Code clarity** - Make code more readable
2. **Performance** - Small performance improvement
3. **Maintainability** - Easier to maintain
4. **Error prevention** - Prevent a class of errors
5. **Consistency** - Match existing patterns

**ggen-mcp specific opportunities**:
1. **Ontology improvements** - Add missing properties, improve SHACL shapes
2. **Template refinements** - Improve template clarity, add error guards
3. **SPARQL optimizations** - Simplify queries, add filters
4. **Type safety** - Add newtypes, improve validation

**Action**: List improvement opportunities

```markdown
## Kaizen Opportunities (ggen-mcp)

### Ontology Improvements
- [ ] Add missing property to entity definition
- [ ] Improve SHACL shape validation
- [ ] Add clarifying comment to ontology
- [ ] Rename property for clarity

### Template Refinements
- [ ] Add `{{ error() }}` guard for missing variable
- [ ] Extract repeated template pattern to macro
- [ ] Improve template variable naming
- [ ] Add template documentation comment

### SPARQL Optimizations
- [ ] Add LIMIT clause to prevent unbounded queries
- [ ] Simplify query structure
- [ ] Add filter to reduce result set
- [ ] Extract common query pattern

### Type Safety
- [ ] Add newtype for entity name (see [Poka-Yoke Design](./poka-yoke-design.md))
- [ ] Add validation for SPARQL query
- [ ] Add type bounds to template context

### Consistency
- [ ] Match ontology naming convention
- [ ] Match template style
- [ ] Match SPARQL query format
```

**Principle**: "Small improvements, continuously" - Don't wait for perfect. Make small improvements now.

---

### Step 2: Plan Change

**Action**: Design minimal change that improves code.

#### 2.1: Define Improvement

**Action**: Clearly define what will improve.

**Improvement statement**:
- **What**: What will change?
- **Why**: Why is this improvement valuable?
- **How**: How will it be implemented?
- **Risk**: What could go wrong?

**Example improvement statement - Ontology**:
```markdown
## Improvement Plan (Ontology)

**What**: Add `description` property to `mcp:Tool` entity in ontology
**Why**: Makes generated code more self-documenting, improves API clarity
**How**: 
1. Edit `ontology/mcp-domain.ttl`: Add `mcp:description` property to `mcp:Tool`
2. Update SPARQL query: Extract `description` in `queries/tools.rq`
3. Update template: Include `description` in `templates/tool.rs.tera`
4. Run `cargo make sync-dry-run` to preview changes
**Risk**: Low - adding property, not changing existing structure
```

**Example improvement statement - Template**:
```markdown
## Improvement Plan (Template)

**What**: Add `{{ error() }}` guard for missing `entity_name` variable
**Why**: Prevents template rendering with missing data, fails fast
**How**: 
1. Edit `templates/entity.rs.tera`: Add guard at top
2. Test with missing variable: Should fail with clear error
3. Verify guard works: Run template validation
**Risk**: Low - adding safety check, no logic change
```

#### 2.2: Define Success Criteria (DMAIC Measurement)

**Action**: Define measurable success criteria for the improvement.

**Success criteria format**:
- **Measurable**: Can be quantified
- **Achievable**: Realistic to achieve
- **Specific**: Clear what success looks like

**Example success criteria - Ontology**:
```markdown
## Success Criteria (Ontology Improvement)

**Primary**:
- Generated code includes description field
- API documentation improved
- Self-documenting code generated

**Measurable**:
- Properties in ontology: N → N+1 (added description)
- Generated code lines: M → M+5 (added description field)
- API clarity: Improved (subjective, but verifiable)

**Verification**:
- Ontology sync succeeds: `cargo make sync-dry-run`
- Generated code compiles: `cargo make check`
- Tests pass: `cargo make test`
```

#### 2.3: Collect Baseline Data (DMAIC Measurement)

**Action**: Measure current state before improvement.

**Data to collect**:
- **Current state**: What is the current state?
- **Metrics**: Quantify current state
- **Patterns**: What patterns exist?

**Action**: Collect baseline data

```bash
# Count properties in ontology
grep -c "mcp:Tool" ontology/mcp-domain.ttl
# Output: 1 tool definition found

# Check if description property exists
grep "mcp:description" ontology/mcp-domain.ttl
# Output: No description property found

# Count generated code properties
grep -c "pub " src/generated/tool.rs
# Output: 3 properties (name, inputSchema, outputSchema)
```

**Example baseline data**:
```markdown
## Baseline Data (Ontology Improvement)

**Tool Properties**: 3 (name, inputSchema, outputSchema)
**Description Property**: 0 (missing)
**Generated Code Lines**: 45 lines
**API Clarity**: Medium (no descriptions)
```

#### 2.4: Verify Safety

**Action**: Ensure change is safe.

**Safety checks**:
- ✅ No breaking changes to existing code
- ✅ Tests exist for affected code
- ✅ Change is isolated (doesn't affect other entities)
- ✅ Can be easily reverted if needed
- ✅ Preview before applying (ggen-mcp specific)

**Action**: Verify safety

```bash
# Preview changes before applying
cargo make sync-dry-run

# Check tests exist
cargo make test

# Verify current behavior
cargo make check
```

---

### Step 3: Do (Implement)

**Action**: Implement the improvement.

#### 3.1: Make Change

**Action**: Implement the planned change.

**Change principles**:
- **Minimal**: Change only what's necessary
- **Focused**: One improvement at a time
- **Clean**: Follow existing patterns

**Example implementation - Ontology**:
```turtle
# Before
mcp:Tool
    a owl:Class ;
    rdfs:label "Tool" ;
    mcp:name ?name ;
    mcp:inputSchema ?inputSchema ;
    mcp:outputSchema ?outputSchema .

# After (Kaizen improvement)
mcp:Tool
    a owl:Class ;
    rdfs:label "Tool" ;
    mcp:name ?name ;
    mcp:description ?description ;  # Added description property
    mcp:inputSchema ?inputSchema ;
    mcp:outputSchema ?outputSchema .
```

**Example implementation - Template**:
```tera
{# Before #}
pub struct {{ entity_name }} {
    // ...
}

{# After (Kaizen improvement) #}
{% if entity_name %}
pub struct {{ entity_name }} {
    // ...
}
{% else %}
{{ error("entity_name required") }}
{% endif %}
```

#### 3.2: Verify Compilation

**Action**: Ensure code compiles.

```bash
# Preview changes
cargo make sync-dry-run

# Verify compilation
cargo make check
```

**Expected**: Compiles successfully

---

### Step 4: Check (Verify)

**Action**: Verify improvement achieved its goal.

#### 4.1: Verify Functionality

**Action**: Ensure functionality preserved.

```bash
# Run tests
cargo make test

# Verify sync works
cargo make sync-dry-run
```

**Expected**: All tests pass, sync succeeds

#### 4.2: Verify Improvement

**Action**: Check that improvement achieved its goal.

**Verification**:
- **Code clarity**: Is code more readable?
- **Performance**: Is performance improved? (if applicable)
- **Maintainability**: Is code easier to maintain?
- **Error prevention**: Are errors prevented? (if applicable)
- **Consistency**: Does code match patterns? (if applicable)

**Example verification - Ontology**:
```rust
// Improvement: Added description property to Tool
// Verification:
// ✅ Generated code includes description: `pub description: String`
// ✅ API more self-documenting: Description explains tool purpose
// ✅ Functionality preserved: Tests pass
// ✅ No breaking changes: Existing code still works
```

#### 4.3: Measure Improvement (DMAIC Measurement)

**Action**: Measure improvement against baseline data and success criteria.

**Measurement**:
- Re-measure metrics after improvement
- Compare to baseline
- Calculate improvement percentage
- Verify success criteria met

**Action**: Measure improvement

```bash
# Re-count properties after improvement
grep -c "pub " src/generated/tool.rs
# Output: 4 properties (up from 3)

# Check if description property exists
grep "description" src/generated/tool.rs
# Output: pub description: String; (property added)

# Calculate improvement
# Baseline: 3 properties, 0 descriptions
# After improvement: 4 properties, 1 description
# Improvement: Added description property (33% more properties)
```

**Example improvement measurement**:
```markdown
## Improvement Measurement (Ontology)

**Baseline**: 3 properties, 0 descriptions, 45 lines
**After Improvement**: 4 properties, 1 description, 50 lines
**Improvement**: Added description property (33% more properties)

**Success Criteria Met**: ✅
- Properties in ontology: 3 → 4 (added description) ✅
- Generated code includes description ✅
- API clarity: Improved ✅
- Functionality preserved: Tests pass ✅
```

#### 4.4: Check for Regressions

**Action**: Ensure no regressions introduced.

**Checks**:
- ✅ All tests pass
- ✅ No performance degradation (if applicable)
- ✅ No new warnings
- ✅ Code still compiles
- ✅ Generated code still valid

**If regressions found**:
- Revert change (restore ontology/template)
- Re-plan improvement
- Return to Step 2

**If no regressions**:
- Proceed to Step 5

---

### Step 5: Act (Standardize)

**Action**: Standardize the improvement if successful.

#### 5.1: Apply Pattern Consistently

**Action**: Apply improvement pattern to similar code.

**Pattern application**:
- Find similar code that could benefit
- Apply same improvement
- Verify each application

**Example - Ontology**:
```turtle
# Applied improvement pattern to other entities
mcp:Tool
    mcp:description ?description ;  # Applied pattern

mcp:Resource
    mcp:description ?description ;  # Applied same pattern

mcp:Prompt
    mcp:description ?description ;  # Applied same pattern
```

**Example - Template**:
```tera
{# Applied error guard pattern to other templates #}
{% if entity_name %}
  // Template content
{% else %}
  {{ error("entity_name required") }}
{% endif %}

{% if property_name %}
  // Template content
{% else %}
  {{ error("property_name required") }}
{% endif %}
```

#### 5.2: Document Pattern

**Action**: Document improvement pattern for future use.

**Documentation**:
- What pattern was applied
- Why it's beneficial
- When to apply it
- How to apply it

**Example documentation**:
```turtle
# Tool entity definition.
# 
# Kaizen improvement: Added description property for self-documentation.
# Pattern: Add description property to all entities for:
# - API clarity
# - Self-documenting generated code
# - Better developer experience
mcp:Tool
    mcp:description ?description ;
```

#### 5.3: Establish Standard

**Action**: Make improvement part of coding standards.

**Standard establishment**:
- Add to code review checklist
- Add to coding standards
- Share with team

**Example standard**:
```markdown
## Coding Standard: Entity Descriptions

**Rule**: All entities in ontology must have description property
**Rationale**: Improves API clarity, self-documenting generated code
**Example**: `mcp:Tool mcp:description ?description`
**When**: For all new entities, add to existing entities incrementally
```

#### 5.4: Establish Controls (DMAIC Control)

**Action**: Set up controls to ensure improvement is sustained.

**Controls**:
- **Code review**: Check for descriptions in reviews
- **Automated checks**: Validate ontology has descriptions
- **Monitoring**: Track description coverage over time
- **Standards**: Document pattern in coding standards

**Action**: Create todo list for controls (10+ items)

```markdown
## Kaizen Control Todos (10+ items) - ggen-mcp

**Code Review Controls**:
- [ ] Add checklist item: All entities have description property
- [ ] Add checklist item: All templates have error guards
- [ ] Add checklist item: All SPARQL queries have LIMIT clause
- [ ] Update code review process to include checklist
- [ ] Verify checklist is used in reviews

**Automated Checks**:
- [ ] Add ontology validation: Check for description properties
- [ ] Add template validation: Check for error guards
- [ ] Add SPARQL validation: Check for LIMIT clauses
- [ ] Configure CI check: Fail if standards violated
- [ ] Review validation rules monthly

**Monitoring Controls**:
- [ ] Set up ontology coverage tracking dashboard
- [ ] Configure alerts if description coverage decreases
- [ ] Review coverage trends weekly
- [ ] Document improvement patterns

**Standards Controls**:
- [ ] Add standard to coding guidelines: "All entities have descriptions"
- [ ] Add standard: "All templates have error guards"
- [ ] Update team documentation with standards
- [ ] Verify standards are followed in code reviews
- [ ] Review standards quarterly
```

**Execution**:
1. Create todos using `todo_write` tool (10+ items minimum)
2. Execute todos one by one (implement controls)
3. Mark todos as completed as controls are implemented
4. Verify each control works before moving to next
5. Continue until all controls implemented

**Principle**: Implement controls to sustain improvement, don't just document them. Todos track progress, controls prevent regression.

#### 5.5: Monitor (DMAIC Control)

**Action**: Monitor to ensure improvement is sustained.

**Monitoring**:
- Track description coverage over time
- Set up alerts for regression
- Review trends periodically
- Adjust controls if needed

**Action**: Set up monitoring

```bash
# Monitor description coverage
# Run weekly: grep -c "mcp:description" ontology/mcp-domain.ttl
# Alert if coverage decreases

# Track trends
# Week 1: 0 descriptions (baseline)
# Week 2: 5 descriptions (after improvement)
# Week 3: 8 descriptions (pattern applied)
# Week 4: 10 descriptions (sustained)
```

---

## Complete Workflow Example (ggen-mcp)

```rust
// Step 1: Identify Opportunity
// Opportunity: Add description property to Tool entity

// Step 2: Plan Change
// Plan: Add mcp:description to ontology, update query/template, sync
// Risk: Low - adding property, not changing structure

// Step 3: Do (Implement)
// Edit ontology/mcp-domain.ttl: Add mcp:description property
// Update queries/tools.rq: Extract description
// Update templates/tool.rs.tera: Include description
// Preview: cargo make sync-dry-run

// Step 4: Check (Verify)
cargo make sync-dry-run  # Preview OK ✅
cargo make check         # Compiles ✅
cargo make test          # Tests pass ✅
// Improvement verified: Generated code includes description ✅

// Step 5: Act (Standardize)
// Apply pattern to other entities
// Document pattern
// Establish standard
```

## Kaizen Mindset

**Principles**:
1. **Small improvements** - Don't wait for perfect, improve now
2. **Continuous** - Make improvements regularly, not just once
3. **Everyone** - Anyone can suggest improvements
4. **No blame** - Focus on improvement, not blame
5. **Data-driven** - Use data to identify opportunities

**Benefits**:
- **Low risk** - Small changes are safer than big rewrites
- **Fast feedback** - See results quickly
- **Compound effect** - Small improvements add up over time
- **Sustainable** - Easier to maintain than big changes

**ggen-mcp specific**: 
- **Ontology improvements** compound into better generated code
- **Template refinements** improve all generated artifacts
- **SPARQL optimizations** improve query performance
- **Type safety** prevents entire classes of errors

## Integration with Other Commands

- **[Ontology Sync](./ontology-sync.md)** - Apply kaizen improvements before sync
- **[Poka-Yoke Design](./poka-yoke-design.md)** - Use kaizen to add type safety incrementally
- **[Eliminate Muda](./eliminate-muda.md)** - Use kaizen to eliminate waste incrementally
- **[Mura Elimination](./eliminate-mura.md)** - Use kaizen to standardize patterns
- **[DMAIC Problem Solving](./dmaic-problem-solving.md)** - Use DMAIC measurement and control steps integrated into this workflow

## Expert Insights

**Why this matters**: Big rewrites are risky and slow. Small improvements are safe and fast. Experts make many small improvements rather than waiting for perfect solutions.

**Key principle**: "Better is the enemy of good" - Don't wait for perfect. Make small improvements now.

**ggen-mcp specific**: 
- **Ontology-first**: Improve ontology, regenerate code - safer than editing generated code
- **Template-driven**: Improve templates, regenerate all artifacts - compound effect
- **Incremental**: Small ontology changes → small code changes → low risk

**Remember**: Kaizen is continuous. Don't stop after one improvement. Keep looking for opportunities. Small improvements compound over time.

**PDCA cycle**: Plan-Do-Check-Act is the Kaizen cycle. Plan small change, do it, check results, act to standardize. Repeat continuously.

**DfLSS alignment**: Kaizen (continuous improvement) aligns with DfLSS (Design for Lean Six Sigma) principles - improvements should address both efficiency (waste elimination) AND quality (defect prevention). Don't conflate DfLSS with DFSS (Design for Six Sigma) - DFSS only addresses quality, missing critical waste elimination. When making improvements, consider both efficiency gains and quality improvements.
