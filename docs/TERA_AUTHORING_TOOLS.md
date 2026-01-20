# Tera Template Authoring Tools

**Version**: 1.0.0
**Module**: `src/tools/tera_authoring.rs`
**Tools**: 5 MCP tools for Tera template development

---

## Overview

Tera template authoring tools → Read, validate, test, create, analyze templates. Safe rendering. Pattern scaffolding. Built-in library (struct/endpoint/schema/interface).

### Core Capabilities

- **Analysis**: Extract variables, filters, control structures, blocks, macros
- **Validation**: Syntax checking, balanced blocks, filter existence, common issues
- **Testing**: Render with sample context, performance metrics, error reporting
- **Scaffolding**: Create from patterns (Rust struct, API endpoint, OpenAPI schema, TypeScript interface)
- **Extraction**: List all variables with usage count, filters, type hints

---

## MCP Tools (5)

### 1. read_tera_template

**Purpose**: Parse and analyze Tera template structure.

**Parameters**:
```json
{
  "template": "struct.rs | inline:{{ content }} | path/to/template.tera",
  "analyze_variables": true,
  "analyze_filters": true,
  "analyze_structures": true
}
```

**Response**:
```json
{
  "content": "template source",
  "size": 1234,
  "variables": ["name", "fields", "description"],
  "filters": ["upper", "snake_case", "default"],
  "structures": [
    {"kind": "for", "line": 5, "details": "field in fields"},
    {"kind": "if", "line": 10, "details": "has_validation"}
  ],
  "blocks": ["header", "body"],
  "includes": ["common/header.tera"],
  "macros": ["render_field"]
}
```

**Use Cases**:
- Understand template before editing
- Audit variable usage
- Check control flow complexity

---

### 2. validate_tera_template

**Purpose**: Validate syntax, references, balanced blocks.

**Parameters**:
```json
{
  "template": "struct.rs | inline:...",
  "check_variables": true,
  "check_filters": true,
  "check_blocks": true
}
```

**Response**:
```json
{
  "valid": false,
  "errors": [
    {
      "error_type": "unbalanced_blocks",
      "message": "Unclosed blocks: if",
      "line": 15,
      "suggestion": "Ensure all {% if %}, {% for %}, {% block %} tags are closed"
    }
  ],
  "warnings": ["Unknown filter 'my_custom_filter' - may not exist in Tera"],
  "syntax_valid": true,
  "blocks_balanced": false
}
```

**Validation Checks**:
1. Tera syntax compilation
2. Balanced blocks (if/endif, for/endfor, block/endblock)
3. Filter existence (warns on unknown)
4. Common issues (escaping, malformed tags)

---

### 3. test_tera_template

**Purpose**: Render template with sample context → verify output.

**Parameters**:
```json
{
  "template": "struct.rs | inline:...",
  "context": {
    "struct_name": "User",
    "description": "User entity",
    "serde": true,
    "fields": [
      {"name": "id", "type_name": "u64", "description": "User ID"},
      {"name": "name", "type_name": "String", "description": "User name"}
    ]
  },
  "timeout_ms": 5000,
  "show_metrics": true
}
```

**Response**:
```json
{
  "output": "/// User entity\n#[derive(Debug, Clone, Serialize, Deserialize)]...",
  "success": true,
  "errors": [],
  "duration_ms": 12,
  "output_size": 456,
  "variables_used": ["struct_name", "description", "serde", "fields"]
}
```

**Use Cases**:
- Verify template produces correct output
- Test with edge cases (empty arrays, missing optionals)
- Performance profiling

---

### 4. create_tera_template

**Purpose**: Scaffold template from common patterns.

**Parameters**:
```json
{
  "pattern": "struct | endpoint | schema | interface",
  "variables": {},
  "output_name": "custom_struct.rs.tera"
}
```

**Response**:
```json
{
  "template": "/// {{ description }}\n#[derive(Debug, Clone{% if serde %}, Serialize, Deserialize{% endif %})]...",
  "pattern": "struct",
  "size": 789,
  "suggested_name": "custom_struct.rs.tera"
}
```

**Built-in Patterns**:

#### struct.rs
```tera
/// {{ description }}
#[derive(Debug, Clone{% if serde %}, Serialize, Deserialize{% endif %})]
pub struct {{ struct_name }} {
    {% for field in fields %}
    pub {{ field.name }}: {{ field.type_name }},
    {% endfor %}
}
```

#### endpoint.rs
```tera
pub async fn {{ handler_name }}(
    state: Arc<AppState>,
    params: {{ params_type }},
) -> Result<{{ response_type }}, McpError> {
    // Validation + implementation
}
```

#### schema.yaml
```tera
{{ schema_name }}:
  type: object
  properties:
    {% for field in fields %}
    {{ field.name }}:
      type: {{ field.type }}
    {% endfor %}
```

#### interface.ts
```tera
export interface {{ interface_name }} {
  {% for field in fields %}
  {{ field.name }}: {{ field.type_name }};
  {% endfor %}
}
```

---

### 5. list_template_variables

**Purpose**: Extract all variables → usage analysis.

**Parameters**:
```json
{
  "template": "struct.rs | inline:...",
  "include_filters": true,
  "include_type_hints": false
}
```

**Response**:
```json
{
  "variables": [
    {
      "name": "struct_name",
      "usage_count": 3,
      "filters": ["pascal_case"],
      "has_default": false,
      "type_hint": "string"
    },
    {
      "name": "fields",
      "usage_count": 5,
      "filters": [],
      "has_default": false,
      "type_hint": "array"
    }
  ],
  "count": 7,
  "required": ["struct_name", "fields", "description"],
  "optional": ["serde", "validation"]
}
```

**Type Inference**:
- Filter-based: `date` → DateTime, `plus` → number
- Name-based: `*_id` → number, `*_at` → DateTime, plural → array
- Default: string

---

## Template Library (Embedded)

Built-in templates accessible by name:

| Name | Pattern | Use Case |
|------|---------|----------|
| `struct` / `struct.rs` | Rust struct | Domain entities, value objects |
| `endpoint` / `endpoint.rs` | API handler | MCP tools, REST endpoints |
| `schema` / `schema.yaml` | OpenAPI schema | API documentation |
| `interface` / `interface.ts` | TypeScript interface | Frontend types |

**Access**:
```rust
// In template parameter
"template": "struct.rs"

// Returns built-in template content
TemplateLibrary::get("struct") // Some(&str)
TemplateLibrary::list()        // ["struct.rs", "endpoint.rs", ...]
```

---

## Validation Rules

### Syntax Validation
- Compiles template with Tera engine
- Detects unclosed tags, invalid syntax
- Reports line numbers (where possible)

### Balanced Blocks
- Tracks open/close pairs: if/endif, for/endfor, block/endblock, macro/endmacro
- Detects mismatches: `{% if %}...{% endfor %}`
- Reports unclosed blocks: `{% for item in items %}` (missing endfor)

### Filter Checking
- Known Tera filters (40+ built-in):
  - String: upper, lower, capitalize, trim, slugify, safe, escape
  - Array: length, first, last, join, split, sort, unique
  - Math: round, abs, plus, minus, times, divided_by
  - Custom: snake_case, pascal_case, camel_case, kebab_case
- Warns on unknown filters (may be custom)

### Common Issues
- Escaping: `{{{{` detected → possible escaping problem
- Empty variables: Missing required context
- Large templates: 1MB limit enforced

---

## Safety Features

### Input Validation
```rust
validate_template_param(template)?;   // Length, non-empty
validate_template_size(content)?;     // 1MB max
```

### Rendering Safety (via SafeRenderer)
- Timeout: 5s default (configurable)
- Max output: 1MB
- Syntax validation: enabled
- Security checks: enabled

### Error Context
```rust
McpError::builder(ErrorCode::TemplateError)
    .message("Failed to load template")
    .param("template", template_name)
    .suggestion("Use built-in template or inline content")
    .build()
```

---

## Usage Examples

### Example 1: Validate before rendering
```json
// Step 1: Validate
{
  "tool": "validate_tera_template",
  "arguments": {
    "template": "inline:{% for field in fields %}{{ field.name }}{% endfor %}",
    "check_blocks": true
  }
}
// → { "valid": true, "blocks_balanced": true }

// Step 2: Test render
{
  "tool": "test_tera_template",
  "arguments": {
    "template": "inline:...",
    "context": {"fields": [{"name": "id"}, {"name": "name"}]}
  }
}
// → { "output": "idname", "success": true }
```

### Example 2: Scaffold from pattern
```json
// Step 1: Create from pattern
{
  "tool": "create_tera_template",
  "arguments": {
    "pattern": "struct",
    "output_name": "my_entity.rs.tera"
  }
}
// → Returns struct template

// Step 2: Customize template (manually edit)

// Step 3: Test with data
{
  "tool": "test_tera_template",
  "arguments": {
    "template": "my_entity.rs.tera",
    "context": {
      "struct_name": "Product",
      "description": "Product entity",
      "serde": true,
      "fields": [...]
    }
  }
}
```

### Example 3: Audit template complexity
```json
{
  "tool": "read_tera_template",
  "arguments": {
    "template": "templates/complex_domain.rs.tera",
    "analyze_structures": true
  }
}
// → Response shows 15 if blocks, 8 for loops, 3 macros
// → Suggests refactoring if too complex
```

---

## Integration with ggen Workflow

### Before (manual Tera editing)
1. Edit `.tera` file
2. Run `cargo make sync`
3. Check compile errors
4. Repeat

### After (MCP-assisted)
1. **read_tera_template** → understand structure
2. **validate_tera_template** → catch errors early
3. **test_tera_template** → verify output
4. **list_template_variables** → document required context
5. Commit with confidence

---

## Error Handling

### ErrorCode::TemplateError (-32011)
Used for:
- Failed to load template file
- Tera compilation errors
- Rendering failures

**Example**:
```rust
McpError::builder(ErrorCode::TemplateError)
    .message("Tera syntax error: unexpected end of input")
    .param("template", "struct.rs.tera")
    .suggestion("Check for unclosed tags")
    .build()
```

### ErrorCode::ValidationError (-32004)
Used for:
- Invalid parameters (empty template name)
- Template size exceeds limit
- Context not a JSON object

---

## Testing Strategy

### Unit Tests (8 tests)
```rust
#[test]
fn test_extract_variables() { ... }        // {{ var }} detection
#[test]
fn test_extract_filters() { ... }         // {{ var | filter }}
#[test]
fn test_balanced_blocks_valid() { ... }   // {% if %}...{% endif %}
#[test]
fn test_balanced_blocks_invalid() { ... } // Mismatched
#[test]
fn test_template_library_get() { ... }    // Built-in templates
#[test]
fn test_validate_tera_syntax_valid() { ... }
#[test]
fn test_validate_tera_syntax_invalid() { ... }
#[test]
fn test_extract_blocks() { ... }          // {% block name %}
```

### Integration Tests
```bash
cargo test --test tera_authoring_integration
```

### Property Tests (Chicago-style)
```rust
#[test]
fn all_builtin_templates_compile() {
    for name in TemplateLibrary::list() {
        let content = TemplateLibrary::get(name).unwrap();
        assert!(validate_tera_syntax(content, name).is_ok());
    }
}
```

---

## Performance Characteristics

| Operation | Typical Time | Max Size |
|-----------|--------------|----------|
| read_tera_template | <10ms | 1MB |
| validate_tera_template | <20ms | 1MB |
| test_tera_template | <50ms | 1MB output |
| create_tera_template | <1ms | N/A (built-in) |
| list_template_variables | <15ms | 1MB |

**Timeout**: 5s default for rendering (configurable per-tool)

---

## Limitations & Future Work

### Current Limitations
1. **Type inference**: Heuristic-based (not precise)
2. **Custom filters**: No validation (warns only)
3. **Macro analysis**: Basic detection (no parameter extraction)
4. **Include resolution**: Detects directives but doesn't follow

### Future Enhancements
1. **Template linting**: Style guide enforcement
2. **Macro parameter extraction**: Full signature analysis
3. **Include resolution**: Follow and analyze included templates
4. **Diff preview**: Compare before/after template changes
5. **Autocomplete**: Variable/filter suggestions based on context

---

## Dependencies

- `tera = "1"` - Template engine
- `regex = "1.10"` - Pattern matching
- `serde_json = "1.0"` - JSON context handling

**No new dependencies added** (all exist in project)

---

## File Structure

```
src/tools/tera_authoring.rs (1100 lines)
├── Constants (MAX_TEMPLATE_SIZE, timeouts)
├── TemplateLibrary (4 built-in templates)
├── MCP Tools (5 handlers)
│   ├── read_tera_template
│   ├── validate_tera_template
│   ├── test_tera_template
│   ├── create_tera_template
│   └── list_template_variables
├── Helper Functions (15 functions)
│   ├── extract_variables
│   ├── extract_filters
│   ├── extract_structures
│   ├── check_balanced_blocks
│   └── validate_tera_syntax
└── Tests (8 unit tests)
```

---

## Quick Reference

### Read template
```
read_tera_template { template: "struct.rs" }
→ variables, filters, structures
```

### Validate
```
validate_tera_template { template: "inline:..." }
→ errors, warnings, blocks_balanced
```

### Test render
```
test_tera_template { template: "...", context: {...} }
→ output, success, duration_ms
```

### Create from pattern
```
create_tera_template { pattern: "struct" }
→ template content
```

### List variables
```
list_template_variables { template: "..." }
→ variables with usage_count, filters
```

---

**Status**: Production-ready
**Coverage**: 8 unit tests, integration pending
**Documentation**: Complete
**Poka-yoke**: Input validation, size limits, error context, safe rendering
