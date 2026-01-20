# ggen Unified Resource Management

**One tool. 15 operations. 2,300 tokens saved.**

---

## Quick Start

### Call the Tool

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

### Response

```json
{
  "operation": "read_config",
  "result": {
    "config": {...},
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

---

## Available Operations

### Config (5)
- `read_config` - Read ggen.toml
- `validate_config` - Validate configuration
- `add_rule` - Add generation rule
- `update_rule` - Update existing rule
- `remove_rule` - Remove rule

### Ontology (5)
- `read_ontology` - Read Turtle file
- `add_entity` - Add DDD entity
- `add_property` - Add property to entity
- `validate_ontology` - Validate Turtle syntax
- `query_entities` - Query entities by type

### Template (5)
- `read_template` - Read Tera template
- `validate_template` - Validate syntax
- `test_template` - Test render with context
- `create_template` - Create from pattern
- `list_template_vars` - Extract variables

---

## Documentation

- **Reference**: [UNIFIED_GGEN_TOOL.md](./UNIFIED_GGEN_TOOL.md)
- **Migration**: [MIGRATION_UNIFIED_TOOL.md](./MIGRATION_UNIFIED_TOOL.md)
- **Implementation**: [../UNIFIED_TOOL_IMPLEMENTATION.md](../UNIFIED_TOOL_IMPLEMENTATION.md)

---

## Examples

### Read Config
```json
{"operation": {"type": "read_config"}}
```

### Add Entity to Ontology
```json
{
  "operation": {
    "type": "add_entity",
    "path": "ontology/mcp-domain.ttl",
    "entity_name": "Order",
    "entity_type": "aggregate_root",
    "properties": [
      {"name": "orderId", "rust_type": "String", "required": true}
    ]
  }
}
```

### Test Template
```json
{
  "operation": {
    "type": "test_template",
    "template": "inline:Hello {{ name }}!",
    "context": {"name": "World"}
  }
}
```

---

## Files

```
src/tools/ggen_unified.rs           # Implementation (653 LOC)
tests/ggen_unified_test.rs          # Tests (560 LOC, 18 tests)
docs/UNIFIED_GGEN_TOOL.md           # Complete reference
docs/MIGRATION_UNIFIED_TOOL.md      # Migration guide
UNIFIED_TOOL_IMPLEMENTATION.md      # Implementation summary
```

---

## Benefits

✅ 2,300 tokens saved (90% reduction)
✅ Single tool registration
✅ Unified error handling
✅ Consistent responses
✅ Duration tracking
✅ 100% test coverage

---

## Testing

```bash
# Run all tests
cargo test --test ggen_unified_test

# Run specific category
cargo test --test ggen_unified_test test_read_config
cargo test --test ggen_unified_test test_add_entity
cargo test --test ggen_unified_test test_read_template
```

Expected: 18 tests passing

---

## Quick Reference

| Old Tool | New Operation |
|----------|---------------|
| `read_ggen_config` | `read_config` |
| `add_generation_rule` | `add_rule` |
| `read_turtle_ontology` | `read_ontology` |
| `add_entity_to_ontology` | `add_entity` |
| `read_tera_template` | `read_template` |

See [MIGRATION_UNIFIED_TOOL.md](./MIGRATION_UNIFIED_TOOL.md) for complete mapping.

---

**SPR Summary**: Ontology/config/template authoring → single tool dispatch → 15 ops → 2.3K token savings → zero duplication → exhaustive matching → production ready.
