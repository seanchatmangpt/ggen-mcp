# Migration Guide: 15 Tools → 1 Unified Tool

**Version**: 1.0.0
**Target**: Existing users of separate ggen authoring tools

---

## Overview

This guide helps migrate from 15 separate tools to the unified `manage_ggen_resource` tool.

### What's Changing
- **Before**: 15 separate tool calls (`read_ggen_config`, `add_entity_to_ontology`, etc.)
- **After**: 1 unified tool call with `operation` parameter
- **Breaking**: Tool names change, parameter structure changes
- **Compatible**: All functionality preserved, responses identical

---

## Quick Reference Table

| Old Tool | New Operation | Category |
|----------|---------------|----------|
| `read_ggen_config` | `read_config` | config |
| `validate_ggen_config` | `validate_config` | config |
| `add_generation_rule` | `add_rule` | config |
| `update_generation_rule` | `update_rule` | config |
| `remove_generation_rule` | `remove_rule` | config |
| `read_turtle_ontology` | `read_ontology` | ontology |
| `add_entity_to_ontology` | `add_entity` | ontology |
| `add_property_to_entity` | `add_property` | ontology |
| `validate_turtle_syntax` | `validate_ontology` | ontology |
| `query_ontology_entities` | `query_entities` | ontology |
| `read_tera_template` | `read_template` | template |
| `validate_tera_template` | `validate_template` | template |
| `test_tera_template` | `test_template` | template |
| `create_tera_template` | `create_template` | template |
| `list_template_variables` | `list_template_vars` | template |

---

## Migration Examples

### 1. ggen.toml Operations

#### Read Config

**Before:**
```json
{
  "tool": "read_ggen_config",
  "params": {
    "config_path": "ggen.toml"
  }
}
```

**After:**
```json
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

**Response Structure:** Unchanged ✅

---

#### Validate Config

**Before:**
```json
{
  "tool": "validate_ggen_config",
  "params": {
    "config_path": "ggen.toml",
    "check_file_refs": true,
    "check_circular_deps": true,
    "check_path_overlaps": true
  }
}
```

**After:**
```json
{
  "tool": "manage_ggen_resource",
  "params": {
    "operation": {
      "type": "validate_config",
      "config_path": "ggen.toml",
      "check_file_refs": true,
      "check_circular_deps": true,
      "check_path_overlaps": true
    }
  }
}
```

**Response Structure:** Wrapped in envelope (see below) ⚠️

---

#### Add Rule

**Before:**
```json
{
  "tool": "add_generation_rule",
  "params": {
    "config_path": "ggen.toml",
    "rule": {
      "name": "user-entity",
      "description": "Generate User entity",
      "query_file": "queries/user.rq",
      "template_file": "templates/entity.rs.tera",
      "output_file": "src/generated/user.rs",
      "mode": "Overwrite"
    },
    "create_backup": true
  }
}
```

**After:**
```json
{
  "tool": "manage_ggen_resource",
  "params": {
    "operation": {
      "type": "add_rule",
      "config_path": "ggen.toml",
      "rule": {
        "name": "user-entity",
        "description": "Generate User entity",
        "query_file": "queries/user.rq",
        "template_file": "templates/entity.rs.tera",
        "output_file": "src/generated/user.rs",
        "mode": "Overwrite"
      },
      "create_backup": true
    }
  }
}
```

**Response Structure:** Wrapped in envelope ⚠️

---

### 2. Turtle Ontology Operations

#### Read Ontology

**Before:**
```json
{
  "tool": "read_turtle_ontology",
  "params": {
    "path": "ontology/mcp-domain.ttl",
    "include_entities": true,
    "include_prefixes": true
  }
}
```

**After:**
```json
{
  "tool": "manage_ggen_resource",
  "params": {
    "operation": {
      "type": "read_ontology",
      "path": "ontology/mcp-domain.ttl",
      "include_entities": true,
      "include_prefixes": true
    }
  }
}
```

---

#### Add Entity

**Before:**
```json
{
  "tool": "add_entity_to_ontology",
  "params": {
    "path": "ontology/mcp-domain.ttl",
    "entity_name": "Order",
    "entity_type": "aggregate_root",
    "properties": [
      {
        "name": "orderId",
        "rust_type": "String",
        "required": true,
        "description": "Order ID"
      }
    ],
    "label": "Order Aggregate",
    "comment": "Order management aggregate",
    "create_backup": true,
    "validate_syntax": true
  }
}
```

**After:**
```json
{
  "tool": "manage_ggen_resource",
  "params": {
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
          "description": "Order ID"
        }
      ],
      "label": "Order Aggregate",
      "comment": "Order management aggregate",
      "create_backup": true,
      "validate_syntax": true
    }
  }
}
```

---

#### Query Entities

**Before:**
```json
{
  "tool": "query_ontology_entities",
  "params": {
    "path": "ontology/mcp-domain.ttl",
    "entity_type_filter": "entity",
    "include_properties": true
  }
}
```

**After:**
```json
{
  "tool": "manage_ggen_resource",
  "params": {
    "operation": {
      "type": "query_entities",
      "path": "ontology/mcp-domain.ttl",
      "entity_type_filter": "entity",
      "include_properties": true
    }
  }
}
```

---

### 3. Tera Template Operations

#### Read Template

**Before:**
```json
{
  "tool": "read_tera_template",
  "params": {
    "template": "inline:{{ name }}",
    "analyze_variables": true,
    "analyze_filters": true,
    "analyze_structures": true
  }
}
```

**After:**
```json
{
  "tool": "manage_ggen_resource",
  "params": {
    "operation": {
      "type": "read_template",
      "template": "inline:{{ name }}",
      "analyze_variables": true,
      "analyze_filters": true,
      "analyze_structures": true
    }
  }
}
```

---

#### Test Template

**Before:**
```json
{
  "tool": "test_tera_template",
  "params": {
    "template": "inline:Hello {{ name }}!",
    "context": { "name": "World" },
    "timeout_ms": 5000,
    "show_metrics": true
  }
}
```

**After:**
```json
{
  "tool": "manage_ggen_resource",
  "params": {
    "operation": {
      "type": "test_template",
      "template": "inline:Hello {{ name }}!",
      "context": { "name": "World" },
      "timeout_ms": 5000,
      "show_metrics": true
    }
  }
}
```

---

## Response Structure Changes

### Response Envelope

All operations now return a consistent envelope:

**Old Response (direct):**
```json
{
  "config": { ... },
  "rule_count": 5,
  "file_size": 2048
}
```

**New Response (wrapped):**
```json
{
  "operation": "read_config",
  "result": {
    "config": { ... },
    "rule_count": 5,
    "file_size": 2048
  },
  "metadata": {
    "success": true,
    "duration_ms": 12,
    "category": "config"
  }
}
```

### Accessing Results

**Before:**
```python
response = client.call("read_ggen_config", {...})
rule_count = response["rule_count"]
```

**After:**
```python
response = client.call("manage_ggen_resource", {
    "operation": {"type": "read_config", ...}
})
rule_count = response["result"]["rule_count"]
operation_name = response["operation"]
duration = response["metadata"]["duration_ms"]
```

---

## Automated Migration Script

### Python Client

```python
# migration_helper.py
def migrate_call(old_tool: str, old_params: dict) -> dict:
    """Convert old tool call to new unified format"""

    # Map old tool names to new operation types
    operation_map = {
        "read_ggen_config": "read_config",
        "validate_ggen_config": "validate_config",
        "add_generation_rule": "add_rule",
        "update_generation_rule": "update_rule",
        "remove_generation_rule": "remove_rule",
        "read_turtle_ontology": "read_ontology",
        "add_entity_to_ontology": "add_entity",
        "add_property_to_entity": "add_property",
        "validate_turtle_syntax": "validate_ontology",
        "query_ontology_entities": "query_entities",
        "read_tera_template": "read_template",
        "validate_tera_template": "validate_template",
        "test_tera_template": "test_template",
        "create_tera_template": "create_template",
        "list_template_variables": "list_template_vars",
    }

    operation_type = operation_map.get(old_tool)
    if not operation_type:
        raise ValueError(f"Unknown tool: {old_tool}")

    # Wrap params in operation envelope
    return {
        "tool": "manage_ggen_resource",
        "params": {
            "operation": {
                "type": operation_type,
                **old_params
            }
        }
    }

# Usage
old_call = {
    "tool": "read_ggen_config",
    "params": {"config_path": "ggen.toml"}
}

new_call = migrate_call("read_ggen_config", {"config_path": "ggen.toml"})
# Returns: {"tool": "manage_ggen_resource", "params": {"operation": {...}}}
```

### JavaScript/TypeScript Client

```typescript
// migrationHelper.ts
interface OldCall {
  tool: string;
  params: Record<string, any>;
}

interface NewCall {
  tool: 'manage_ggen_resource';
  params: {
    operation: {
      type: string;
      [key: string]: any;
    };
  };
}

const OPERATION_MAP: Record<string, string> = {
  read_ggen_config: 'read_config',
  validate_ggen_config: 'validate_config',
  add_generation_rule: 'add_rule',
  update_generation_rule: 'update_rule',
  remove_generation_rule: 'remove_rule',
  read_turtle_ontology: 'read_ontology',
  add_entity_to_ontology: 'add_entity',
  add_property_to_entity: 'add_property',
  validate_turtle_syntax: 'validate_ontology',
  query_ontology_entities: 'query_entities',
  read_tera_template: 'read_template',
  validate_tera_template: 'validate_template',
  test_tera_template: 'test_template',
  create_tera_template: 'create_template',
  list_template_variables: 'list_template_vars',
};

export function migrateCall(oldTool: string, oldParams: Record<string, any>): NewCall {
  const operationType = OPERATION_MAP[oldTool];
  if (!operationType) {
    throw new Error(`Unknown tool: ${oldTool}`);
  }

  return {
    tool: 'manage_ggen_resource',
    params: {
      operation: {
        type: operationType,
        ...oldParams,
      },
    },
  };
}

// Usage
const oldCall = {
  tool: 'read_ggen_config',
  params: { config_path: 'ggen.toml' },
};

const newCall = migrateCall('read_ggen_config', { config_path: 'ggen.toml' });
```

---

## Testing Migration

### Verify Equivalence

Run side-by-side comparison:

```bash
# 1. Call old tool
curl -X POST http://localhost:8080/mcp/read_ggen_config \
  -H "Content-Type: application/json" \
  -d '{"config_path": "ggen.toml"}' \
  > old_response.json

# 2. Call new unified tool
curl -X POST http://localhost:8080/mcp/manage_ggen_resource \
  -H "Content-Type: application/json" \
  -d '{
    "operation": {
      "type": "read_config",
      "config_path": "ggen.toml"
    }
  }' \
  > new_response.json

# 3. Compare results (unwrap new response)
jq '.result' new_response.json > new_unwrapped.json
diff old_response.json new_unwrapped.json
# Should be identical ✅
```

### Run Test Suite

```bash
# Run migration tests
cargo test --test ggen_unified_test

# Expected: 18 tests passing
# - 5 config operations
# - 5 ontology operations
# - 5 template operations
# - 2 error handling
# - 1 performance
```

---

## Rollback Plan

If migration causes issues:

1. **Keep old tools available** (deprecated but functional)
2. **Gradual migration** (migrate one operation type at a time)
3. **Dual support period** (6 months recommended)
4. **Version flag** in response to indicate which tool was used

```rust
// Support both old and new tools during transition
server.register("read_ggen_config", read_ggen_config); // OLD (deprecated)
server.register("manage_ggen_resource", manage_ggen_resource); // NEW
```

---

## Benefits Summary

✅ **Token Savings**: 700 tokens on tool discovery
✅ **Unified Errors**: Single error handling path
✅ **Simpler Mental Model**: 1 tool vs 15
✅ **Consistent Responses**: Uniform metadata structure
✅ **Performance Tracking**: Duration in all responses
✅ **Type Safety**: Tagged enum operations

---

## Deprecation Timeline

- **v1.0.0**: Unified tool introduced, old tools marked deprecated
- **v1.1.0** (3 months): Warning logs when using old tools
- **v2.0.0** (6 months): Old tools removed

---

## Support

Questions? Issues? Feedback?
- GitHub Issues: https://github.com/seanchatmangpt/ggen-mcp/issues
- Docs: `/docs/UNIFIED_GGEN_TOOL.md`
- Tests: `/tests/ggen_unified_test.rs`

---

**Remember**: Same functionality. Different interface. Better organization. 700 token savings.
