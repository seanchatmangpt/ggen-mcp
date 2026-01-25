# Template Rendering Workflow - Multi-Step Command

## Purpose

This command guides agents through validating and rendering Tera templates safely. It covers template syntax validation, variable extraction, context building, multi-language syntax checking, and golden file comparison.

**Core Principle**: Templates must be validated before rendering. All variables must be provided by SPARQL queries. Output must be validated for syntax and security.

## Workflow Overview

```
Step 1: Validate Template Syntax → Step 2: Extract Variables → Step 3: Build Context → Step 4: Render Template → Step 5: Validate Output (with Measurement)
```

## Step-by-Step Instructions

### Step 1: Validate Template Syntax

**Action**: Verify Tera template has valid syntax before rendering.

```bash
# Using MCP tool (if available)
validate_tera_template {
  "template": "templates/aggregate.rs.tera"
}

# Or using cargo make
cargo make validate-template templates/aggregate.rs.tera
```

**What to check**:
- Valid Tera syntax (`{% %}`, `{{ }}`, `{# #}`)
- Proper control flow (`if`, `for`, `macro`)
- Valid filter syntax
- Balanced tags
- Valid variable references

**Common Syntax Errors**:
- Unclosed `{% %}` tags
- Invalid filter syntax
- Undefined macros
- Syntax errors in expressions

**Using TemplateValidator**:
```rust
use spreadsheet_mcp::template::TemplateValidator;

let validator = TemplateValidator::new();
validator.validate_syntax(&template_content)
    .context("Template syntax validation failed")?;
```

**If syntax invalid**: Fix template syntax, retry validation

**If syntax valid**: Proceed to Step 2

### Step 2: Extract Variables

**Action**: Identify all variables used in template to ensure SPARQL query provides them.

**Variable extraction**:
- Variables in `{{ variable }}` expressions
- Variables in `{% if variable %}` conditions
- Variables in `{% for item in items %}` loops
- Variables in filter expressions

**Using extract_variables**:
```rust
use spreadsheet_mcp::template::extract_variables;

let variables_used = extract_variables(&template_content);
// Returns: ["entity_name", "fields", "invariants", ...]
```

**Verify SPARQL provides variables**:
```rust
// Check SPARQL query results contain all template variables
let query_variables: HashSet<String> = sparql_results.variables().collect();
let template_variables: HashSet<String> = extract_variables(&template).collect();

let missing = template_variables.difference(&query_variables);
if !missing.is_empty() {
    return Err(Error::MissingVariables {
        variables: missing.collect(),
    });
}
```

**CRITICAL**: Template `{{ error() }}` guards prevent rendering if variables missing:
```tera
{% if entity_name %}
  // Template content
{% else %}
  {{ error("entity_name required") }}
{% endif %}
```

**If variables missing**: Update SPARQL query to provide variables, or add defaults

**If all variables provided**: Proceed to Step 3

### Step 3: Build Context

**Action**: Build validated template context from SPARQL query results.

**Using TemplateContext**:
```rust
use spreadsheet_mcp::template::TemplateContext;

let mut ctx = TemplateContext::new("aggregate.rs.tera");

// Insert variables from SPARQL results
for solution in sparql_results {
    ctx.insert_string("entity_name", solution.get("entity_name")?)?;
    ctx.insert_array("fields", solution.get("fields")?)?;
    ctx.insert_bool("has_id", solution.get("has_id")?)?;
}

// Validate context before rendering
ctx.validate()?;
```

**Type-safe insertion**:
- `insert_string()` - String values
- `insert_number()` - Numeric values
- `insert_bool()` - Boolean values
- `insert_array()` - Array values
- `insert_object()` - Object values

**Parameter schema validation** (if schema defined):
```rust
use spreadsheet_mcp::template::{ParameterSchema, ParameterDefinition, ParameterType};

let schema = ParameterSchema::new("aggregate.rs.tera")
    .parameter(
        ParameterDefinition::new("entity_name", ParameterType::String)
            .required()
            .description("Entity name")
    )
    .parameter(
        ParameterDefinition::new("fields", ParameterType::Array)
            .required()
            .description("Entity fields")
    );

schema.validate_context(ctx.inner())?;
```

**If context invalid**: Fix SPARQL query or template, retry

**If context valid**: Proceed to Step 4

### Step 4: Render Template

**Action**: Render template with validated context and safety guards.

**Using SafeRenderer**:
```rust
use spreadsheet_mcp::template::SafeRenderer;

let config = RenderConfig::default()
    .with_timeout_ms(5000)
    .with_syntax_validation(true);

let renderer = SafeRenderer::new(config)?;

// Add template
renderer.add_template("aggregate.rs.tera", &template_content)?;

// Render with context
let output = renderer.render_safe("aggregate.rs.tera", &render_context)?;
```

**Safety guards**:
- Timeout protection (default: 5 seconds)
- Recursion limit (prevents infinite loops)
- Output size limit (prevents memory exhaustion)
- Syntax validation before rendering
- Security pattern detection

**If rendering fails**: Review error, fix template/context, retry

**If rendering succeeds**: Proceed to Step 5

### Step 5: Validate Output (with Measurement)

**Action**: Validate rendered output for syntax, security, and correctness.

**Multi-language syntax validation**:
```rust
use spreadsheet_mcp::template::OutputValidator;

let validator = OutputValidator::new();

let validation_report = match output_format {
    "rust" => validator.validate_rust_syntax(&output)?,
    "typescript" => validator.validate_typescript_syntax(&output)?,
    "yaml" => validator.validate_yaml_syntax(&output)?,
    "json" => validator.validate_json_syntax(&output)?,
    _ => validator.validate_generic(&output)?,
};
```

**Security checks**:
```rust
let security_report = validator.check_security_patterns(&output)?;
// Checks for: injection patterns, unsafe code, etc.
```

**Golden file comparison** (if golden file exists):
```rust
use spreadsheet_mcp::codegen::validation::GoldenFileValidator;

let validator = GoldenFileValidator::new();
let comparison = validator.compare_with_golden(
    &output,
    "expected/aggregate.rs"
)?;

if comparison.has_mismatches() {
    // Review differences
    for mismatch in comparison.mismatches {
        println!("Mismatch: {}", mismatch);
    }
}
```

**Measurement**:
- Rendering time
- Output size
- Validation errors
- Security warnings

**If validation fails**: Fix template, regenerate, retry

**If validation succeeds**: Template rendering complete ✅

## Complete Workflow Example

```rust
// Step 1: Validate Syntax
let validator = TemplateValidator::new();
validator.validate_syntax(&template)?;
// Syntax OK ✅

// Step 2: Extract Variables
let variables = extract_variables(&template);
// Variables: ["entity_name", "fields"] ✅

// Step 3: Build Context
let mut ctx = TemplateContext::new("aggregate.rs.tera");
ctx.insert_string("entity_name", "User")?;
ctx.insert_array("fields", vec![...])?;
ctx.validate()?;
// Context valid ✅

// Step 4: Render Template
let renderer = SafeRenderer::new(config)?;
renderer.add_template("aggregate.rs.tera", &template)?;
let output = renderer.render_safe("aggregate.rs.tera", &ctx)?;
// Rendered ✅

// Step 5: Validate Output
let validator = OutputValidator::new();
let report = validator.validate_rust_syntax(&output)?;
// Output valid ✅
```

## Integration with Ontology Sync

Template rendering integrates with ontology sync workflow:

**Before Sync**:
```bash
# Validate all templates before sync
for template in templates/*.tera; do
    cargo make validate-template "$template"
done
```

**During Sync**:
- Templates validated automatically
- Variables extracted from SPARQL results
- Context built from query results
- Templates rendered with safety guards
- Output validated for syntax

**After Sync**:
- Golden files compared
- Validation reports generated
- Metrics recorded

## Error Handling

### If Template Syntax Invalid

**Symptoms**: Syntax validation errors

**Fix**:
1. Check Tera syntax documentation
2. Verify balanced tags
3. Fix filter syntax
4. Retry validation

### If Variables Missing

**Symptoms**: `{{ error() }}` triggered, undefined variable errors

**Fix**:
1. Check SPARQL query provides all variables
2. Add default values in template
3. Update query to extract missing variables
4. Retry rendering

### If Context Invalid

**Symptoms**: Parameter validation errors, type mismatches

**Fix**:
1. Verify SPARQL result types match template expectations
2. Update template parameter schema
3. Fix context building code
4. Retry rendering

### If Rendering Fails

**Symptoms**: Timeout, recursion limit, output size limit

**Fix**:
1. Simplify template logic
2. Reduce context size
3. Increase timeout (if legitimate)
4. Optimize template structure
5. Retry rendering

### If Output Invalid

**Symptoms**: Syntax errors, security warnings

**Fix**:
1. Review template logic
2. Fix syntax errors
3. Remove security issues
4. Regenerate output
5. Retry validation

## Best Practices

1. **Always Validate Syntax**: Check template syntax before rendering
2. **Extract Variables First**: Ensure SPARQL provides all template variables
3. **Use Type-Safe Context**: Use `TemplateContext` for type safety
4. **Validate Output**: Check rendered output for syntax and security
5. **Use Golden Files**: Compare output against expected results
6. **Test Templates**: Test templates with sample data before sync
7. **Document Variables**: Document required template variables

## Integration with Other Commands

- **[Ontology Sync](./ontology-sync.md)** - Render templates during sync
- **[SPARQL Validation](./sparql-validation.md)** - Ensure queries provide template variables
- **[Code Generation](./code-generation.md)** - Validate templates in codegen pipeline
- **[Poka-Yoke Design](./poka-yoke-design.md)** - Prevent errors through template design

## Documentation References

- **[TEMPLATE_PARAMETER_VALIDATION.md](../../docs/TEMPLATE_PARAMETER_VALIDATION.md)** - Parameter validation guide
- **[TERA_AUTHORING_TOOLS.md](../../docs/TERA_AUTHORING_TOOLS.md)** - Template authoring tools
- **[src/template/parameter_validation.rs](../../src/template/parameter_validation.rs)** - Source code
- **[src/tools/tera_authoring.rs](../../src/tools/tera_authoring.rs)** - Tera authoring tools

## Quick Reference

```rust
// Full template rendering workflow
let validator = TemplateValidator::new();
validator.validate_syntax(&template)?;                    // Step 1: Syntax

let variables = extract_variables(&template);             // Step 2: Variables

let mut ctx = TemplateContext::new("template.tera");     // Step 3: Context
ctx.insert_string("var", "value")?;
ctx.validate()?;

let renderer = SafeRenderer::new(config)?;                // Step 4: Render
let output = renderer.render_safe("template.tera", &ctx)?;

let validator = OutputValidator::new();                   // Step 5: Validate
validator.validate_rust_syntax(&output)?;
```
