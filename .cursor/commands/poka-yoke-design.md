# Poka-Yoke Design (Error Prevention) - Multi-Step Workflow

## Purpose

This command guides agents to design code that prevents errors at compile time through type safety and invariants. Poka-yoke means "mistake-proofing" - making errors impossible through design. Experts use the type system to prevent entire classes of errors.

**ggen-mcp Context**: Apply poka-yoke to ontology entities, SPARQL queries, and template rendering to prevent errors in the code generation pipeline.

## Workflow Overview

```
Step 1: Identify Error Modes → Step 2: Design Type-Level Prevention → Step 3: Add Compile-Time Checks → Step 4: Verify Prevention (with Measurement) → Step 5: Document Invariants (with Control)
```

## Step-by-Step Instructions

### Step 1: Identify Error Modes

**Action**: List all ways code can fail at runtime.

**Error mode categories**:

1. **Invalid state** - States that shouldn't exist
   - Example: Negative count, empty required field, invalid enum variant
   - **ggen-mcp**: Ontology entity missing required properties, invalid SPARQL query state

2. **Invalid input** - Inputs that cause errors
   - Example: Empty string when non-empty required, null when non-null required
   - **ggen-mcp**: Empty entity name, invalid IRI, SPARQL injection patterns

3. **Invalid operations** - Operations that fail in certain states
   - Example: Reading from closed file, modifying immutable data
   - **ggen-mcp**: Editing generated code manually, executing invalid SPARQL query

4. **Resource errors** - Resource-related failures
   - Example: Out of memory, file not found, network error
   - **ggen-mcp**: Ontology file not found, template file missing

5. **Logic errors** - Errors in program logic
   - Example: Division by zero, index out of bounds, overflow
   - **ggen-mcp**: SPARQL query complexity too high, template variable missing

**Action**: Create error mode inventory

```markdown
## Error Modes Inventory (ggen-mcp)

### Invalid State
- [ ] Ontology entity missing required properties (SHACL violation)
- [ ] SPARQL query in invalid state after error
- [ ] Template missing required variables

### Invalid Input
- [ ] Empty entity name passed to code generation (should be non-empty)
- [ ] Invalid IRI in SPARQL query
- [ ] SPARQL injection patterns in user input

### Invalid Operations
- [ ] Editing generated code manually (should only edit ontology)
- [ ] Executing SPARQL query without validation
- [ ] Rendering template without all variables

### Resource Errors
- [ ] Ontology file not found
- [ ] Template file missing
- [ ] SPARQL query file not found

### Logic Errors
- [ ] SPARQL query complexity exceeds budget
- [ ] Template variable not provided by SPARQL query
- [ ] Generated code has syntax errors
```

---

### Step 2: Design Type-Level Prevention

**Action**: Use Rust's type system to make errors impossible.

#### 2.1: Use Newtypes for Validation (ggen-mcp: Ontology Entities)

**Action**: Create newtypes that enforce invariants.

**Example - Ontology Entity Names**:
```rust
// ❌ BAD: Can have invalid state
struct Entity {
    name: String, // Can be empty!
}

// ✅ GOOD: Type prevents invalid state
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntityName(String);

impl EntityName {
    pub fn new(name: String) -> Result<Self, ValidationError> {
        if name.is_empty() {
            return Err(ValidationError::EmptyEntityName);
        }
        if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(ValidationError::InvalidEntityName);
        }
        Ok(Self(name))
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// Cannot create EntityName with empty string - type prevents it
```

**Example - SPARQL Query Safety**:
```rust
// ❌ BAD: Can have injection vulnerabilities
fn execute_query(query: String) -> Result<QueryResults> {
    // String concatenation allows injection!
    store.query(&query)
}

// ✅ GOOD: Type-safe query construction
use spreadsheet_mcp::sparql::injection_prevention::{QueryBuilder, SafeLiteralBuilder};

pub struct SafeSparqlQuery(String);

impl SafeSparqlQuery {
    pub fn new() -> QueryBuilder {
        QueryBuilder::select()
    }
    
    pub fn build(self) -> String {
        self.0
    }
}

// Cannot inject - QueryBuilder prevents it
```

#### 2.2: Use Enums for State Machines (ggen-mcp: Generation Pipeline)

**Action**: Use enums to represent valid states only.

**Example - Code Generation State**:
```rust
// ❌ BAD: Can be in invalid state
struct CodeGenerator {
    is_preview: bool,
    is_applied: bool,
    // Can have both true - invalid state!
}

// ✅ GOOD: Enum prevents invalid states
enum GenerationState {
    Preview(PreviewReport),
    Applied(GenerationReceipt),
    Error(GenerationError),
}

struct CodeGenerator {
    state: GenerationState, // Only valid states possible
}

impl CodeGenerator {
    fn apply(self) -> Result<GenerationReceipt, GenerationError> {
        match self.state {
            GenerationState::Preview(report) => {
                // Can only apply from Preview state
                Ok(GenerationReceipt::from_preview(report))
            }
            _ => Err(GenerationError::InvalidState),
        }
    }
}
```

#### 2.3: Use Option/Result for Nullable Values (ggen-mcp: Template Variables)

**Action**: Use `Option<T>` instead of nullable types.

**Example - Template Context**:
```rust
// ❌ BAD: Can pass null, causes runtime error
fn render_template(template: &str, entity_name: &str) -> String {
    template.replace("{{ entity_name }}", entity_name) // Panics if empty!
}

// ✅ GOOD: Type forces handling of None
use spreadsheet_mcp::template::TemplateContext;

fn render_template(
    template: &str,
    context: TemplateContext
) -> Result<String, RenderingError> {
    match context.get("entity_name") {
        Some(name) if !name.is_empty() => {
            // Render with name
            Ok(template.replace("{{ entity_name }}", name))
        }
        Some(_) => Err(RenderingError::EmptyEntityName),
        None => Err(RenderingError::MissingEntityName),
    }
}
```

#### 2.4: Use PhantomData for Type-Level Invariants (ggen-mcp: Ontology Validation)

**Action**: Use PhantomData to encode invariants in types.

**Example - Validated Ontology**:
```rust
use std::marker::PhantomData;

// Type-level invariant: ValidatedOntology<Validated> vs ValidatedOntology<Unvalidated>
struct Validated;
struct Unvalidated;

struct ValidatedOntology<State> {
    store: oxigraph::Store,
    _state: PhantomData<State>,
}

impl ValidatedOntology<Unvalidated> {
    fn validate(self) -> Result<ValidatedOntology<Validated>, ValidationError> {
        // Validate SHACL shapes
        let validator = ShapeValidator::from_store(&self.store)?;
        let report = validator.validate(&self.store)?;
        
        if !report.is_valid() {
            return Err(ValidationError::ShaclViolations(report.violations));
        }
        
        Ok(ValidatedOntology {
            store: self.store,
            _state: PhantomData,
        })
    }
}

impl ValidatedOntology<Validated> {
    fn execute_sparql(&self, query: &SafeSparqlQuery) -> Result<QueryResults> {
        // Can only execute SPARQL on validated ontology
        self.store.query(&query.build())
    }
}

// Cannot execute SPARQL on unvalidated ontology - compiler error!
```

---

### Step 3: Add Compile-Time Checks

**Action**: Leverage Rust's compiler to catch errors.

#### 3.1: Use Type Bounds (ggen-mcp: SPARQL Query Builder)

**Action**: Add trait bounds to restrict valid types.

**Example**:
```rust
use spreadsheet_mcp::sparql::injection_prevention::QueryBuilder;

// QueryBuilder only accepts SafeLiteralBuilder for values
fn build_query<T: Into<SafeLiteralBuilder>>(value: T) -> QueryBuilder {
    QueryBuilder::select()
        .variable("?s")
        .where_clause(&format!("?s :value {}", value.into().build()))
}

// Compiler error if type doesn't implement Into<SafeLiteralBuilder>
```

#### 3.2: Use Const Generics for Sizes (ggen-mcp: Template Variable Arrays)

**Action**: Use const generics to prevent size errors.

**Example**:
```rust
// Array size encoded in type - prevents index errors
fn process_template_variables<const N: usize>(
    vars: [TemplateVariable; N]
) -> TemplateContext {
    // N is known at compile time
    // Cannot index out of bounds - compiler knows size
    let mut ctx = TemplateContext::new();
    for var in vars {
        ctx.insert(var.name, var.value)?;
    }
    ctx
}
```

#### 3.3: Use Lifetimes to Prevent Use-After-Free (ggen-mcp: Ontology References)

**Action**: Use lifetimes to prevent memory errors.

**Example**:
```rust
// Lifetime ensures reference doesn't outlive ontology
fn extract_entity<'a>(
    ontology: &'a ValidatedOntology<Validated>,
    entity_name: &str
) -> Result<&'a Entity, ExtractionError> {
    // Returned reference tied to ontology lifetime
    // Compiler prevents use-after-free
    ontology.get_entity(entity_name)
}
```

#### 3.4: SPARQL Injection Prevention (ggen-mcp Specific)

**Action**: Use QueryBuilder to prevent injection.

**Example**:
```rust
use spreadsheet_mcp::sparql::injection_prevention::{QueryBuilder, SafeLiteralBuilder};

// ✅ SAFE: Type-safe construction prevents injection
fn build_safe_query(entity_name: &str) -> Result<String, SparqlError> {
    let query = QueryBuilder::select()
        .variable("?entity")
        .variable("?name")
        .where_clause("?entity a mcp:Entity")
        .where_clause(&format!("?entity mcp:name {}",
            SafeLiteralBuilder::string(entity_name).build()))
        .build()?;
    
    Ok(query)
}

// ❌ UNSAFE: String concatenation allows injection
fn build_unsafe_query(entity_name: &str) -> String {
    format!("SELECT ?entity WHERE {{ ?entity mcp:name \"{}\" }}", entity_name)
    // If entity_name contains ' } UNION { ?s ?p ?o }, injection occurs!
}
```

---

### Step 4: Verify Prevention

**Action**: Ensure type system prevents errors.

#### 4.1: Attempt Invalid Operations

**Action**: Try to write code that should fail to compile.

**Example - Ontology Entities**:
```rust
// Try to create invalid entity name - should fail to compile
let entity = EntityName::new("".to_string()); // Compile error - returns Result

// Try to use unvalidated ontology - should fail to compile
let ontology = ValidatedOntology::<Unvalidated>::new()?;
ontology.execute_sparql(&query); // Compile error - can't execute on unvalidated
```

**Verification**: Code that should be invalid doesn't compile

```bash
cargo make check
# Should show compile errors for invalid operations
```

#### 4.2: Verify Valid Operations Compile

**Action**: Ensure valid code compiles successfully.

**Example**:
```rust
// Valid operations should compile
let entity_name = EntityName::new("User".to_string())?; // Valid
let ontology = ValidatedOntology::<Unvalidated>::new()?.validate()?; // Valid
ontology.execute_sparql(&query); // Valid - ontology is validated
```

**Verification**: Valid code compiles

```bash
cargo make check
# Should compile successfully
```

#### 4.3: Test Runtime Behavior

**Action**: Verify type-level prevention works at runtime.

```bash
cargo make test
# Tests should pass - type system prevents errors
```

#### 4.4: Measure Error Prevention (DMAIC Measurement)

**Action**: Measure error prevention effectiveness against baseline.

**Measurement**:
- Count errors prevented by type system
- Compare to baseline (errors that would occur without types)
- Calculate prevention percentage
- Verify success criteria met

**Action**: Measure error prevention

```bash
# Count compile-time errors caught (prevented runtime errors)
cargo make check 2>&1 | grep -c "error\["
# Output: 8 compile-time errors (prevented 8 runtime errors)

# Count runtime errors (should be 0 with type prevention)
cargo make test 2>&1 | grep -c "panicked"
# Output: 0 panics (type system prevented errors)

# Count SPARQL injection attempts prevented
cargo make test 2>&1 | grep -c "injection"
# Output: 0 injections (QueryBuilder prevented all)

# Calculate prevention
# Baseline: 8 potential runtime errors (without type prevention)
# After type prevention: 0 runtime errors (caught at compile time)
# Prevention: 100% (8/8 errors prevented)
```

**Example error prevention measurement**:
```markdown
## Error Prevention Measurement (ggen-mcp)

**Baseline**: 8 potential runtime errors (without type prevention)
**After Type Prevention**: 0 runtime errors (caught at compile time)
**Prevention**: 100% (8/8 errors prevented)

**By Error Type**:
- Invalid state errors: 3 → 0 (100% prevented)
  - Empty entity names: 1 → 0
  - Invalid ontology states: 1 → 0
  - Missing template variables: 1 → 0
- Invalid input errors: 2 → 0 (100% prevented)
  - SPARQL injection: 1 → 0
  - Invalid IRIs: 1 → 0
- Invalid operation errors: 3 → 0 (100% prevented)
  - Editing generated code: 1 → 0
  - Unvalidated ontology queries: 1 → 0
  - Missing template variables: 1 → 0

**Success Criteria Met**: ✅
- All errors caught at compile time ✅
- No runtime errors ✅
- Type system prevents invalid states ✅
- SPARQL injection prevented ✅
```

---

### Step 5: Document Invariants

**Action**: Explain why design prevents errors.

#### 5.1: Document Type Invariants (ggen-mcp: Ontology Entities)

**Action**: Document invariants enforced by types.

**Example**:
```rust
/// Entity name that cannot be empty or contain invalid characters.
/// 
/// **Poka-yoke**: Uses `EntityName` newtype instead of `String` to prevent
/// invalid entity names at compile time. The type system makes invalid states
/// impossible.
/// 
/// **ggen-mcp**: Used in ontology-driven code generation to ensure all
/// generated entities have valid names.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntityName(String); // Invariant: Always non-empty, alphanumeric+underscore (enforced by type)
```

#### 5.2: Document State Machine Invariants (ggen-mcp: Generation Pipeline)

**Action**: Document valid state transitions.

**Example**:
```rust
/// Code generation state machine.
/// 
/// **Poka-yoke**: Enum prevents invalid states. Cannot be both preview and applied.
/// Valid transitions:
/// - Preview -> Applied (after review)
/// - Preview -> Error (if validation fails)
/// - Cannot transition from Applied back to Preview (type prevents it)
/// 
/// **ggen-mcp**: Ensures preview-by-default pattern - always preview before applying.
enum GenerationState {
    Preview(PreviewReport),
    Applied(GenerationReceipt),
    Error(GenerationError),
}
```

#### 5.3: Document Usage Patterns (ggen-mcp: SPARQL Safety)

**Action**: Document how to use types safely.

**Example**:
```rust
/// SPARQL query builder that prevents injection attacks.
/// 
/// **Poka-yoke**: Type-safe query construction prevents SPARQL injection.
/// 
/// **ggen-mcp**: Used in ontology sync workflow to safely execute SPARQL queries.
/// 
/// # Example
/// 
/// ```rust
/// use spreadsheet_mcp::sparql::injection_prevention::{QueryBuilder, SafeLiteralBuilder};
/// 
/// let query = QueryBuilder::select()
///     .variable("?entity")
///     .where_clause(&format!("?entity mcp:name {}",
///         SafeLiteralBuilder::string("User").build()))
///     .build()?;
/// 
/// // Injection attempts are prevented at compile time
/// // query.execute() // Safe - no injection possible
/// ```
```

#### 5.4: Establish Controls (DMAIC Control)

**Action**: Set up controls to ensure error prevention is sustained.

**Controls**:
- **Code review**: Check for type safety in reviews
- **Automated checks**: Lint rules to flag unsafe patterns
- **Monitoring**: Track error prevention effectiveness over time
- **Standards**: Document type safety patterns in coding standards

**Action**: Create todo list for controls (10+ items)

```markdown
## Poka-Yoke Control Todos (10+ items) - ggen-mcp

**Code Review Controls**:
- [ ] Add checklist item: Use type system to prevent errors
- [ ] Add checklist item: No runtime error handling for invalid states
- [ ] Add checklist item: Use QueryBuilder for all SPARQL queries
- [ ] Add checklist item: Use EntityName newtype, not String
- [ ] Update code review process to include type safety checks
- [ ] Verify checklist is used in reviews

**Automated Checks**:
- [ ] Add lint rule: Flag unsafe SPARQL patterns (string concatenation)
- [ ] Add lint rule: Flag missing type safety (bare String IDs)
- [ ] Add lint rule: Flag manual edits to generated code
- [ ] Configure CI check: Verify type safety
- [ ] Review lint rules monthly

**Monitoring Controls**:
- [ ] Set up error prevention tracking dashboard
- [ ] Configure alerts if runtime errors increase
- [ ] Track SPARQL injection attempts (should be 0)
- [ ] Review error prevention trends weekly
- [ ] Document error prevention patterns

**Standards Controls**:
- [ ] Add standard: Use type system to prevent errors
- [ ] Add standard: Make invalid states unrepresentable
- [ ] Add standard: Use QueryBuilder for SPARQL (no string concatenation)
- [ ] Add standard: Use newtypes for ontology entities
- [ ] Update team documentation with standards
- [ ] Verify standards are followed in code reviews
```

**Execution**:
1. Create todos using `todo_write` tool (10+ items minimum)
2. Execute todos one by one (implement controls)
3. Mark todos as completed as controls are implemented
4. Verify each control works before moving to next
5. Continue until all controls implemented

**Principle**: Implement controls to sustain error prevention, don't just document them. Todos track progress, controls prevent regression.

#### 5.5: Monitor (DMAIC Control)

**Action**: Monitor to ensure error prevention is sustained.

**Monitoring**:
- Track runtime error count over time
- Set up alerts for regression
- Review trends periodically
- Adjust controls if needed

**Action**: Set up monitoring

```bash
# Monitor runtime errors
# Run weekly: cargo make test 2>&1 | grep -c "panicked"
# Alert if error count > 0

# Monitor SPARQL injection attempts
# Run weekly: cargo make test 2>&1 | grep -c "injection"
# Alert if injection attempts > 0

# Track trends
# Week 1: 8 potential errors (baseline - without type prevention)
# Week 2: 0 errors (after type prevention)
# Week 3: 0 errors (controls working)
# Week 4: 0 errors (sustained)
```

---

## Complete Workflow Example (ggen-mcp)

```rust
// Step 1: Identify Error Modes
// Error: Entity name can be empty
// Error: SPARQL injection possible
// Error: Can execute query on unvalidated ontology

// Step 2: Design Type-Level Prevention
// EntityName: Use newtype instead of String
// SafeSparqlQuery: Use QueryBuilder instead of String
// ValidatedOntology: Use PhantomData for validation state

// Step 3: Add Compile-Time Checks
struct EntityName(String); // Prevents empty names
struct SafeSparqlQuery(String); // Prevents injection
struct ValidatedOntology<State> { /* ... */ } // Prevents unvalidated queries

// Step 4: Verify Prevention
cargo make check
// Attempt invalid operations - should fail to compile
// let entity = EntityName::new("".to_string()); // Compile error!
// let query = format!("SELECT * WHERE {{ ?s ?p \"{}\" }}", user_input); // Compile error!

// Step 5: Document Invariants
/// EntityName that cannot be empty (Poka-yoke: newtype prevents it)
/// SafeSparqlQuery that prevents injection (Poka-yoke: QueryBuilder prevents it)
```

## Integration with Other Commands

- **[Ontology Sync](./ontology-sync.md)** - Apply poka-yoke to ontology entities before sync
- **[SPARQL Validation](./sparql-validation.md)** - Use QueryBuilder to prevent injection
- **[Template Rendering](./template-rendering.md)** - Use TemplateContext to prevent missing variables
- **[Code Generation](./code-generation.md)** - Apply 4-layer validation (poka-yoke at each layer)
- **[Gemba Walk](./gemba-walk.md)** - Verify actual type behavior matches design
- **[Root Cause Analysis](./root-cause-analysis.md)** - Understand why errors occur, then prevent with types
- **[DMAIC Problem Solving](./dmaic-problem-solving.md)** - Use DMAIC measurement and control steps integrated into this workflow
- **[Eliminate Muda](./eliminate-muda.md)** - Remove error-prone patterns, replace with type-safe designs

## Expert Insights

**Why this matters**: Runtime errors are expensive. Type-level prevention catches errors at compile time, before they reach production.

**Key principle**: "Make invalid states unrepresentable" - Use types to make errors impossible.

**ggen-mcp specific**: 
- **Ontology entities**: Use newtypes (`EntityName`, `PropertyName`) instead of `String`
- **SPARQL queries**: Use `QueryBuilder` instead of string concatenation
- **Template variables**: Use `TemplateContext` instead of raw maps
- **Generation state**: Use enums instead of boolean flags

**Remember**: The type system is your friend. Use it to prevent entire classes of errors. If you can't represent an invalid state in your type system, you've prevented that error.

**Poka-yoke principle**: "Prevention is better than detection" - Prevent errors at compile time rather than catching them at runtime.

**DfLSS alignment**: Poka-yoke (defect prevention) is the Six Sigma component of DfLSS (Design for Lean Six Sigma). However, defect prevention alone is incomplete - DfLSS addresses both efficiency (waste elimination) AND quality (defect prevention). Don't conflate DfLSS with DFSS (Design for Six Sigma) - DFSS only addresses quality, missing critical waste elimination. When preventing defects with types, also consider eliminating waste (unnecessary complexity, redundant checks).
