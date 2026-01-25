# OpenAPI Tool Manifest Generation

**Version**: 1.0.0
**Schema Format**: JSON (draft-07)
**Purpose**: Breaking change detection for MCP tool schema

---

## Overview

Tool manifest generation creates `ggen.tools.json` - a JSON schema catalog of all MCP tools with:
- **Schema stability tracking** via SHA256 hash
- **Breaking change detection** for CI/CD pipelines
- **Tool metadata** (name, category, description, capabilities)
- **Parameter/Response schemas** for OpenAPI validation

---

## Schema Structure

```json
{
  "version": "1.0.0",           // Package version (Cargo.toml)
  "schema_hash": "abc123...",   // SHA256 of tools array
  "tools": [                     // Array of tool definitions
    {
      "name": "list_workbooks",
      "category": "core",
      "description": "List spreadsheet files in workspace",
      "params_schema": { ... },
      "response_schema": { ... },
      "capabilities": ["filter", "discovery"]
    }
  ]
}
```

### Tool Categories

- **core**: Read-only analysis tools (inspection, discovery)
- **authoring**: ggen authoring tools (ontology, template, config)
- **jira**: Jira integration tools (sync, query, dashboard)
- **vba**: VBA analysis tools
- **verification**: Receipt verification, audit tools

### Capabilities

Linked features:
- **core**: filter, discovery, inspection, navigation, reading, analysis, statistics, metadata, summary, regions, entry_points, data_extraction
- **authoring**: config, ontology, template, rendering, validation
- **jira**: query, create, sync, dashboard, mapping
- **verification**: hash_verification, guard_validation, audit, receipt

---

## Generation Workflow

### 1. Generate Manifest

```bash
# Compact JSON (stdout)
cargo run --bin generate_manifest

# Pretty-printed
cargo run --bin generate_manifest -- --pretty

# Via Makefile
cargo make generate-manifest
```

### 2. Verify Against Golden (CI/CD)

```bash
# Check for breaking changes
./scripts/verify_manifest.sh
# Exit 0: no changes
# Exit 1: breaking change detected
# Exit 2: script error

# Via Makefile
cargo make verify-manifest
```

### 3. Update Golden on Intent

```bash
# After intentional schema changes
cargo run --bin generate_manifest > tests/golden/ggen.tools.json
git add tests/golden/ggen.tools.json
git commit -m "feat: Update tool manifest schema"
```

---

## Implementation Details

### Hash Computation (SHA256)

1. Serialize tools array to JSON (consistent order)
2. Compute SHA256 of JSON string
3. Store hex digest in `schema_hash`

**Determinism**: Same tools → same hash (verified by tests)

### Simplified Schemas

Current implementation uses minimal schemas:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "title": "list_workbooksParams",
  "description": "Parameters for list_workbooks"
}
```

**Future Enhancement**: Use `schemars` crate to derive full schemas from:
- `ListWorkbooksParams` struct → `params_schema`
- `WorkbookListResponse` struct → `response_schema`

---

## Testing

### Unit Tests (manifest.rs)

```rust
#[test]
fn test_manifest_generation()      // Generates non-empty manifest
#[test]
fn test_manifest_consistency()      // Hash is deterministic
#[test]
fn test_tool_categories()          // Categories are valid
#[test]
fn test_schema_hash_length()       // SHA256 is 64 chars
```

### Integration Test (tests/integration_manifest.rs)

Ignored tests (require project compilation):
- `test_manifest_generation_deterministic()`
- `test_manifest_schema_structure()`
- `test_tool_categories_valid()`

Run with: `cargo test -- --ignored --test integration_manifest`

---

## Files

| File | Purpose | LOC |
|------|---------|-----|
| `src/tools/manifest.rs` | Manifest generator | 182 |
| `src/bin/generate_manifest.rs` | CLI binary | 20 |
| `scripts/verify_manifest.sh` | CI verification | 50 |
| `tests/golden/ggen.tools.json` | Golden manifest | 150 |
| `tests/integration_manifest.rs` | Integration tests | 25 |

---

## CI/CD Integration

### Pre-Commit Hook

```bash
cargo make verify-manifest
```

### CI Pipeline

```yaml
# Add to CI config
- name: Verify tool manifest
  run: cargo make verify-manifest
```

---

## Breaking Change Detection

Manifest hash changes when:
- Tool added/removed
- Tool name changed
- Tool category changed
- Capabilities modified
- Description changed

Schema hash **NOT** affected by:
- Internal implementation changes
- Parameter/response type changes (simplified schema mode)
- Non-functional refactoring

---

## Future Enhancements

1. **Full Schema Derivation**: Use `schemars::schema_for!()` to generate complete JSON schemas from Rust types

2. **Schema Validation**: Validate generated code against schemas at compile-time

3. **OpenAPI Export**: Generate OpenAPI 3.1.0 spec for tool discovery

4. **Capability Matching**: Match client capabilities against tool features

5. **Versioning**: Support multiple schema versions for backward compatibility

---

## Quick Reference

```bash
# Generate manifest
cargo make generate-manifest

# Verify (CI/CD)
cargo make verify-manifest

# Run unit tests
cargo test manifest::tests

# Run integration tests (require compilation)
cargo test integration_manifest -- --ignored
```

---

**SPR Summary**: Manifest = tool schemas + hash for breaking change detection. Generate once, verify in CI, update intentionally.
