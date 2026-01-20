# Unified ggen Resource Management Tool

**Version**: 1.0.0
**Status**: Production Ready
**Token Savings**: ~700 tokens on tool discovery

---

## Overview

Single MCP tool consolidating 15 separate authoring operations:
- **ggen.toml** (5 ops): read, validate, add_rule, update_rule, remove_rule
- **Turtle ontology** (5 ops): read, add_entity, add_property, validate, query
- **Tera templates** (5 ops): read, validate, test, create, list_vars

### Benefits

1. **Token Efficiency**: 700 token savings vs 15 separate tool registrations
2. **Unified Error Handling**: Single error path, consistent context
3. **Simpler Mental Model**: One tool to learn vs 15
4. **Reduced Registration**: 1 tool registration vs 15
5. **Consistent Response Structure**: All operations return same envelope

---

## Tool Signature

```rust
pub async fn manage_ggen_resource(
    state: Arc<AppState>,
    params: ManageGgenResourceParams,
) -> Result<ManageGgenResourceResponse>

pub struct ManageGgenResourceParams {
    pub operation: ResourceOperation,
}

pub enum ResourceOperation {
    // ggen.toml (5)
    ReadConfig { config_path: Option<String> },
    ValidateConfig { config_path, check_file_refs, check_circular_deps, check_path_overlaps },
    AddRule { config_path, rule, create_backup },
    UpdateRule { config_path, rule_name, rule, create_backup },
    RemoveRule { config_path, rule_name, create_backup },

    // Turtle (5)
    ReadOntology { path, include_entities, include_prefixes },
    AddEntity { path, entity_name, entity_type, properties, label, comment, create_backup, validate_syntax },
    AddProperty { path, entity_name, property, create_backup, validate_syntax },
    ValidateOntology { path, shacl_validation, strict_mode },
    QueryEntities { path, entity_type_filter, include_properties },

    // Tera (5)
    ReadTemplate { template, analyze_variables, analyze_filters, analyze_structures },
    ValidateTemplate { template, check_variables, check_filters, check_blocks },
    TestTemplate { template, context, timeout_ms, show_metrics },
    CreateTemplate { pattern, variables, output_name },
    ListTemplateVars { template, include_filters, include_type_hints },
}

pub struct ManageGgenResourceResponse {
    pub operation: String,
    pub result: JsonValue,
    pub metadata: ResponseMetadata,
}
```

---

## Usage Examples

### 1. ggen.toml Operations

#### Read Configuration
```json
{
  "operation": {
    "type": "read_config",
    "config_path": "ggen.toml"
  }
}
```

**Response:**
```json
{
  "operation": "read_config",
  "result": {
    "config": { ... },
    "rule_count": 5,
    "file_size": 2048,
    "rule_names": ["rule1", "rule2", ...]
  },
  "metadata": {
    "success": true,
    "duration_ms": 12,
    "category": "config"
  }
}
```

#### Add Generation Rule
```json
{
  "operation": {
    "type": "add_rule",
    "config_path": "ggen.toml",
    "rule": {
      "name": "user-entity",
      "description": "Generate User entity from ontology",
      "query_file": "queries/user.rq",
      "template_file": "templates/entity.rs.tera",
      "output_file": "src/generated/user.rs",
      "mode": "Overwrite"
    },
    "create_backup": true
  }
}
```

**Response:**
```json
{
  "operation": "add_rule",
  "result": {
    "success": true,
    "rule_name": "user-entity",
    "backup_path": "ggen.toml.backup",
    "rule_count": 6
  },
  "metadata": {
    "success": true,
    "duration_ms": 18,
    "category": "config"
  }
}
```

#### Validate Configuration
```json
{
  "operation": {
    "type": "validate_config",
    "config_path": "ggen.toml",
    "check_file_refs": true,
    "check_circular_deps": true,
    "check_path_overlaps": true
  }
}
```

**Response:**
```json
{
  "operation": "validate_config",
  "result": {
    "valid": true,
    "issues": [],
    "rule_count": 5,
    "error_count": 0,
    "warning_count": 0
  },
  "metadata": {
    "success": true,
    "duration_ms": 25,
    "category": "config"
  }
}
```

### 2. Turtle Ontology Operations

#### Read Ontology
```json
{
  "operation": {
    "type": "read_ontology",
    "path": "ontology/mcp-domain.ttl",
    "include_entities": true,
    "include_prefixes": true
  }
}
```

**Response:**
```json
{
  "operation": "read_ontology",
  "result": {
    "path": "ontology/mcp-domain.ttl",
    "content": "@prefix mcp: ...",
    "triple_count": 142,
    "entities": [
      {
        "name": "User",
        "iri": "http://ggen-mcp.dev/ontology/mcp#User",
        "entity_type": "http://ggen-mcp.dev/ontology/ddd#Entity",
        "properties": [...]
      }
    ],
    "prefixes": {
      "mcp": "http://ggen-mcp.dev/ontology/mcp#",
      "ddd": "http://ggen-mcp.dev/ontology/ddd#"
    },
    "parse_time_ms": 8
  },
  "metadata": {
    "success": true,
    "duration_ms": 15,
    "category": "ontology"
  }
}
```

#### Add Entity
```json
{
  "operation": {
    "type": "add_entity",
    "path": "ontology/mcp-domain.ttl",
    "entity_name": "Order",
    "entity_type": "aggregate_root",
    "properties": [
      {
        "name": "orderId",
        "rust_type": "String",
        "required": true,
        "description": "Unique order identifier"
      },
      {
        "name": "customerId",
        "rust_type": "String",
        "required": true,
        "description": "Customer who placed order"
      },
      {
        "name": "totalAmount",
        "rust_type": "f64",
        "required": true,
        "description": "Total order amount"
      }
    ],
    "label": "Order Aggregate",
    "comment": "Aggregate root for order management",
    "create_backup": true,
    "validate_syntax": true
  }
}
```

**Response:**
```json
{
  "operation": "add_entity",
  "result": {
    "path": "ontology/mcp-domain.ttl",
    "entity_iri": "http://ggen-mcp.dev/ontology/mcp#Order",
    "triples_added": 12,
    "backup_path": "ontology/mcp-domain.ttl.backup",
    "validation": {
      "syntax_valid": true,
      "parse_errors": [],
      "warnings": []
    },
    "duration_ms": 22
  },
  "metadata": {
    "success": true,
    "duration_ms": 28,
    "category": "ontology"
  }
}
```

#### Query Entities
```json
{
  "operation": {
    "type": "query_entities",
    "path": "ontology/mcp-domain.ttl",
    "entity_type_filter": "entity",
    "include_properties": true
  }
}
```

**Response:**
```json
{
  "operation": "query_entities",
  "result": {
    "path": "ontology/mcp-domain.ttl",
    "entities": [
      {
        "name": "User",
        "iri": "http://ggen-mcp.dev/ontology/mcp#User",
        "entity_type": "http://ggen-mcp.dev/ontology/ddd#Entity",
        "properties": [
          {
            "name": "userId",
            "iri": "http://ggen-mcp.dev/ontology/mcp#userId",
            "rust_type": "String",
            "required": true,
            "label": "User ID"
          }
        ],
        "label": "User Entity",
        "comment": "Represents a user in the system"
      }
    ],
    "duration_ms": 14
  },
  "metadata": {
    "success": true,
    "duration_ms": 18,
    "category": "ontology"
  }
}
```

### 3. Tera Template Operations

#### Read Template
```json
{
  "operation": {
    "type": "read_template",
    "template": "inline:pub struct {{ struct_name }} { {{ fields }} }",
    "analyze_variables": true,
    "analyze_filters": true,
    "analyze_structures": true
  }
}
```

**Response:**
```json
{
  "operation": "read_template",
  "result": {
    "content": "pub struct {{ struct_name }} { {{ fields }} }",
    "size": 45,
    "variables": ["struct_name", "fields"],
    "filters": [],
    "structures": [],
    "blocks": [],
    "includes": [],
    "macros": []
  },
  "metadata": {
    "success": true,
    "duration_ms": 5,
    "category": "template"
  }
}
```

#### Test Template
```json
{
  "operation": {
    "type": "test_template",
    "template": "inline:Hello {{ name | upper }}!",
    "context": {
      "name": "world"
    },
    "timeout_ms": 5000,
    "show_metrics": true
  }
}
```

**Response:**
```json
{
  "operation": "test_template",
  "result": {
    "output": "Hello WORLD!",
    "success": true,
    "errors": [],
    "duration_ms": 3,
    "output_size": 12,
    "variables_used": ["name"]
  },
  "metadata": {
    "success": true,
    "duration_ms": 8,
    "category": "template"
  }
}
```

#### Create Template from Pattern
```json
{
  "operation": {
    "type": "create_template",
    "pattern": "struct",
    "variables": {},
    "output_name": "user.rs.tera"
  }
}
```

**Response:**
```json
{
  "operation": "create_template",
  "result": {
    "template": "/// {{ description }}\n#[derive(Debug, Clone...)]\npub struct {{ struct_name }} {...}",
    "pattern": "struct",
    "size": 512,
    "suggested_name": "user.rs.tera"
  },
  "metadata": {
    "success": true,
    "duration_ms": 2,
    "category": "template"
  }
}
```

---

## Migration Guide

### Before (15 separate tools)
```rust
// Register 15 tools
server.register("read_ggen_config", read_ggen_config);
server.register("validate_ggen_config", validate_ggen_config);
server.register("add_generation_rule", add_generation_rule);
server.register("update_generation_rule", update_generation_rule);
server.register("remove_generation_rule", remove_generation_rule);

server.register("read_turtle_ontology", read_turtle_ontology);
server.register("add_entity_to_ontology", add_entity_to_ontology);
server.register("add_property_to_entity", add_property_to_entity);
server.register("validate_turtle_syntax", validate_turtle_syntax);
server.register("query_ontology_entities", query_ontology_entities);

server.register("read_tera_template", read_tera_template);
server.register("validate_tera_template", validate_tera_template);
server.register("test_tera_template", test_tera_template);
server.register("create_tera_template", create_tera_template);
server.register("list_template_variables", list_template_variables);
```

### After (1 unified tool)
```rust
// Register single unified tool
server.register("manage_ggen_resource", manage_ggen_resource);
```

### Client Code Migration

**Before:**
```json
// Call specific tool
{
  "tool": "read_ggen_config",
  "params": {
    "config_path": "ggen.toml"
  }
}
```

**After:**
```json
// Call unified tool with operation
{
  "tool": "manage_ggen_resource",
  "params": {
    "operation": {
      "type": "read_config",
      "config_path": "ggen.toml"
    }
  }
}
```

---

## Error Handling

All operations use consistent error handling:

```json
// Error response (HTTP 500)
{
  "error": {
    "code": "operation_failed",
    "message": "read_config operation failed: Failed to read ggen.toml",
    "context": {
      "operation": "read_config",
      "category": "config",
      "path": "ggen.toml"
    }
  }
}
```

Error categories:
- `read_config operation failed`: ggen.toml read/parse errors
- `add_rule operation failed`: Rule addition errors (duplicate, invalid)
- `read_ontology operation failed`: Turtle parse/read errors
- `add_entity operation failed`: Entity creation errors (duplicate, invalid)
- `read_template operation failed`: Template load/parse errors
- `test_template operation failed`: Template rendering errors

---

## Performance Characteristics

| Operation | Typical Duration | Max Size |
|-----------|-----------------|----------|
| `read_config` | 5-20ms | 10MB |
| `validate_config` | 10-50ms | 10MB |
| `add_rule` | 10-30ms | - |
| `read_ontology` | 10-100ms | 10MB |
| `add_entity` | 20-80ms | - |
| `validate_ontology` | 20-200ms | 10MB |
| `read_template` | 5-15ms | 1MB |
| `test_template` | 5-50ms | 1MB |
| `create_template` | 2-5ms | - |

All operations include `duration_ms` in response metadata.

---

## Testing

Run unified tool tests:
```bash
cargo test --test ggen_unified_test
```

Test categories:
- **Config operations** (5 tests): read, validate, add, update, remove
- **Ontology operations** (5 tests): read, add_entity, add_property, validate, query
- **Template operations** (5 tests): read, validate, test, create, list_vars
- **Error handling** (2 tests): missing file, invalid operation
- **Performance** (1 test): duration tracking

Expected output: 18 tests passing.

---

## Implementation Details

### Delegation Pattern
```rust
match params.operation {
    ResourceOperation::ReadConfig { config_path } => {
        // Delegate to existing ggen_config::read_ggen_config
        let resp = ggen_config::read_ggen_config(
            state,
            ggen_config::ReadGgenConfigParams { config_path },
        ).await?;

        ("read_config", "config", serde_json::to_value(resp)?)
    }
    // ... 14 more operations
}
```

### Zero Code Duplication
- No logic rewritten
- All operations delegate to existing modules
- Type conversions handled at boundary
- Error context added at dispatch level

### Type Safety
- Tagged enum prevents invalid operations
- Compile-time exhaustiveness checks
- NewType wrappers prevent confusion (EntityName, PropertyName)
- JSON schema generation for MCP clients

---

## Future Enhancements

1. **Batch Operations**: Execute multiple operations in single call
2. **Transaction Support**: Rollback on failure across operations
3. **Dry Run Mode**: Preview changes without writing
4. **Diff Output**: Show before/after for modifications
5. **Workspace Awareness**: Auto-detect workspace root

---

## Related Documentation

- `RUST_MCP_BEST_PRACTICES.md` - Tool design patterns
- `POKA_YOKE_IMPLEMENTATION.md` - Error-proofing strategies
- `CHICAGO_TDD_TEST_HARNESS_COMPLETE.md` - Testing infrastructure

---

**Remember**: 1 tool to rule them all. 15 operations. 700 token savings. Zero duplication.
