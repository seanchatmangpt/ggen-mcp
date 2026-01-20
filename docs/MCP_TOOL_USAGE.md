# MCP Tool Usage - Ontology Generation

**Version**: 1.0.0 | Ontology-driven code generation | RDF → SPARQL → Tera → Rust

---

## Quick Reference

| Tool | Purpose | Latency | Idempotent |
|------|---------|---------|------------|
| `validate_ontology` | SHACL validation, dependency check | <1s | ✓ |
| `generate_from_schema` | Zod/JSON → Entity generation | 2-5s | ✓ |
| `generate_from_openapi` | OpenAPI → API implementation | 5-15s | ✓ |
| `preview_generation` | Dry-run preview (no file writes) | 1-3s | ✓ |
| `sync_ontology` | Full pipeline (13 steps) | 10-30s | ✓ |

**All tools return deterministic results** - same input → same output always.

---

## Tool 1: validate_ontology

**Purpose**: Validate RDF/Turtle ontology files for SHACL conformance, syntax errors, and dependency resolution.

### Parameters Schema

```json
{
  "ontology_path": {
    "type": "string",
    "description": "Path to RDF/Turtle ontology file (.ttl)",
    "required": true,
    "example": "ontology/mcp-domain.ttl",
    "validation": "File must exist and be readable"
  },
  "strict_mode": {
    "type": "boolean",
    "description": "Enable strict SHACL validation (warnings as errors)",
    "required": false,
    "default": false,
    "example": true
  },
  "resolve_imports": {
    "type": "boolean",
    "description": "Resolve and validate imported ontologies",
    "required": false,
    "default": true,
    "example": true
  }
}
```

### Response Schema

```json
{
  "status": {
    "type": "string",
    "enum": ["valid", "invalid", "warning"],
    "description": "Validation status"
  },
  "errors": {
    "type": "array",
    "items": {
      "type": "object",
      "properties": {
        "severity": {"type": "string", "enum": ["error", "warning", "info"]},
        "message": {"type": "string"},
        "line": {"type": "integer", "nullable": true},
        "subject": {"type": "string", "description": "RDF subject URI"}
      }
    }
  },
  "statistics": {
    "type": "object",
    "properties": {
      "triple_count": {"type": "integer"},
      "class_count": {"type": "integer"},
      "property_count": {"type": "integer"},
      "constraint_count": {"type": "integer"}
    }
  },
  "dependencies": {
    "type": "array",
    "items": {"type": "string"},
    "description": "Resolved import URIs"
  }
}
```

### Example Request

```json
{
  "tool": "validate_ontology",
  "arguments": {
    "ontology_path": "ontology/mcp-domain.ttl",
    "strict_mode": false,
    "resolve_imports": true
  }
}
```

### Example Response (Success)

```json
{
  "status": "valid",
  "errors": [],
  "statistics": {
    "triple_count": 347,
    "class_count": 12,
    "property_count": 45,
    "constraint_count": 8
  },
  "dependencies": [
    "http://www.w3.org/2000/01/rdf-schema#",
    "http://www.w3.org/ns/shacl#"
  ]
}
```

### Example Response (Validation Errors)

```json
{
  "status": "invalid",
  "errors": [
    {
      "severity": "error",
      "message": "Property mcp:hasParameter has no rdfs:range constraint",
      "line": 42,
      "subject": "http://example.org/mcp#hasParameter"
    },
    {
      "severity": "warning",
      "message": "Class mcp:Tool missing rdfs:label annotation",
      "line": 23,
      "subject": "http://example.org/mcp#Tool"
    }
  ],
  "statistics": {
    "triple_count": 347,
    "class_count": 12,
    "property_count": 45,
    "constraint_count": 8
  },
  "dependencies": []
}
```

### Error Codes

| Code | Meaning | Recovery |
|------|---------|----------|
| `ONTOLOGY_NOT_FOUND` | File path invalid | Check `ontology_path` parameter |
| `SYNTAX_ERROR` | Turtle parsing failed | Fix RDF syntax at reported line |
| `SHACL_VIOLATION` | Constraint violation | Fix ontology to satisfy SHACL shapes |
| `IMPORT_FAILED` | Dependency resolution error | Check network, verify import URIs |
| `INVALID_RDF` | Malformed RDF graph | Validate with external RDF tool |

---

## Tool 2: generate_from_schema

**Purpose**: Generate Rust entity code from Zod or JSON Schema definitions.

### Parameters Schema

```json
{
  "schema_type": {
    "type": "string",
    "enum": ["zod", "json_schema"],
    "description": "Input schema format",
    "required": true,
    "example": "zod"
  },
  "schema_content": {
    "type": "string",
    "description": "Schema definition (Zod DSL or JSON Schema)",
    "required": true,
    "example": "z.object({ id: z.string().uuid(), name: z.string().min(1) })"
  },
  "entity_name": {
    "type": "string",
    "description": "Generated entity name (PascalCase)",
    "required": true,
    "pattern": "^[A-Z][a-zA-Z0-9]*$",
    "example": "UserProfile"
  },
  "features": {
    "type": "array",
    "items": {"type": "string", "enum": ["serde", "validation", "builder", "debug"]},
    "description": "Code generation features",
    "required": false,
    "default": ["serde", "validation"],
    "example": ["serde", "validation", "builder"]
  },
  "output_path": {
    "type": "string",
    "description": "Output file path (relative to workspace root)",
    "required": false,
    "default": "src/generated/{entity_name}.rs",
    "example": "src/domain/entities/user_profile.rs"
  }
}
```

### Response Schema

```json
{
  "entity_name": {"type": "string"},
  "output_path": {"type": "string"},
  "generated_code": {
    "type": "string",
    "description": "Generated Rust code (formatted with rustfmt)"
  },
  "features_applied": {
    "type": "array",
    "items": {"type": "string"}
  },
  "validation_rules": {
    "type": "array",
    "items": {
      "type": "object",
      "properties": {
        "field": {"type": "string"},
        "rule": {"type": "string"},
        "constraint": {"type": "string"}
      }
    }
  },
  "statistics": {
    "type": "object",
    "properties": {
      "fields_generated": {"type": "integer"},
      "lines_of_code": {"type": "integer"},
      "validation_rules": {"type": "integer"}
    }
  }
}
```

### Example Request (Zod Schema)

```json
{
  "tool": "generate_from_schema",
  "arguments": {
    "schema_type": "zod",
    "schema_content": "z.object({ id: z.string().uuid(), email: z.string().email(), age: z.number().int().min(0).max(120) })",
    "entity_name": "User",
    "features": ["serde", "validation", "builder"],
    "output_path": "src/domain/user.rs"
  }
}
```

### Example Response

```json
{
  "entity_name": "User",
  "output_path": "src/domain/user.rs",
  "generated_code": "#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]\npub struct User {\n    pub id: uuid::Uuid,\n    pub email: String,\n    pub age: u8,\n}\n\nimpl User {\n    pub fn validate(&self) -> Result<(), ValidationError> {\n        validate_email(&self.email)?;\n        validate_range(self.age, 0, 120, \"age\")?;\n        Ok(())\n    }\n}\n\n#[derive(Default)]\npub struct UserBuilder {\n    id: Option<uuid::Uuid>,\n    email: Option<String>,\n    age: Option<u8>,\n}\n\nimpl UserBuilder {\n    pub fn id(mut self, id: uuid::Uuid) -> Self {\n        self.id = Some(id);\n        self\n    }\n    pub fn email(mut self, email: String) -> Self {\n        self.email = Some(email);\n        self\n    }\n    pub fn age(mut self, age: u8) -> Self {\n        self.age = Some(age);\n        self\n    }\n    pub fn build(self) -> Result<User, BuilderError> {\n        Ok(User {\n            id: self.id.ok_or(BuilderError::MissingField(\"id\"))?,\n            email: self.email.ok_or(BuilderError::MissingField(\"email\"))?,\n            age: self.age.ok_or(BuilderError::MissingField(\"age\"))?,\n        })\n    }\n}",
  "features_applied": ["serde", "validation", "builder"],
  "validation_rules": [
    {"field": "id", "rule": "uuid", "constraint": "valid UUID v4"},
    {"field": "email", "rule": "email", "constraint": "RFC 5322 format"},
    {"field": "age", "rule": "range", "constraint": "0 <= age <= 120"}
  ],
  "statistics": {
    "fields_generated": 3,
    "lines_of_code": 47,
    "validation_rules": 3
  }
}
```

### Error Codes

| Code | Meaning | Recovery |
|------|---------|----------|
| `INVALID_SCHEMA` | Schema syntax error | Fix schema syntax |
| `UNSUPPORTED_TYPE` | Schema type not supported | Use supported Zod/JSON Schema types |
| `INVALID_ENTITY_NAME` | Entity name violates naming rules | Use PascalCase identifier |
| `OUTPUT_PATH_INVALID` | Output path outside workspace | Use relative path within project |
| `FEATURE_CONFLICT` | Incompatible features selected | Remove conflicting feature |

---

## Tool 3: generate_from_openapi

**Purpose**: Generate complete API implementation (handlers, models, validation) from OpenAPI 3.x specification.

### Parameters Schema

```json
{
  "openapi_spec": {
    "type": "string",
    "description": "OpenAPI spec (YAML/JSON string or file path)",
    "required": true,
    "example": "openapi/petstore.yaml"
  },
  "spec_format": {
    "type": "string",
    "enum": ["yaml", "json", "auto"],
    "description": "Spec format (auto-detect from extension)",
    "required": false,
    "default": "auto",
    "example": "yaml"
  },
  "generation_target": {
    "type": "string",
    "enum": ["full", "models_only", "handlers_only", "validators_only"],
    "description": "What to generate",
    "required": false,
    "default": "full",
    "example": "full"
  },
  "framework": {
    "type": "string",
    "enum": ["rmcp", "axum", "actix_web", "warp"],
    "description": "Web framework for handlers",
    "required": false,
    "default": "rmcp",
    "example": "rmcp"
  },
  "output_dir": {
    "type": "string",
    "description": "Output directory for generated files",
    "required": false,
    "default": "src/generated",
    "example": "src/api"
  },
  "validation_strategy": {
    "type": "string",
    "enum": ["strict", "lenient", "none"],
    "description": "Request/response validation level",
    "required": false,
    "default": "strict",
    "example": "strict"
  }
}
```

### Response Schema

```json
{
  "api_name": {"type": "string"},
  "api_version": {"type": "string"},
  "output_dir": {"type": "string"},
  "files_generated": {
    "type": "array",
    "items": {
      "type": "object",
      "properties": {
        "path": {"type": "string"},
        "kind": {"type": "string", "enum": ["model", "handler", "validator", "test", "mod"]},
        "lines": {"type": "integer"}
      }
    }
  },
  "operations": {
    "type": "array",
    "items": {
      "type": "object",
      "properties": {
        "operation_id": {"type": "string"},
        "method": {"type": "string"},
        "path": {"type": "string"},
        "handler_name": {"type": "string"}
      }
    }
  },
  "models": {
    "type": "array",
    "items": {
      "type": "object",
      "properties": {
        "name": {"type": "string"},
        "schema_ref": {"type": "string"},
        "fields": {"type": "integer"}
      }
    }
  },
  "statistics": {
    "type": "object",
    "properties": {
      "total_lines": {"type": "integer"},
      "total_files": {"type": "integer"},
      "endpoints": {"type": "integer"},
      "models": {"type": "integer"},
      "validators": {"type": "integer"}
    }
  }
}
```

### Example Request

```json
{
  "tool": "generate_from_openapi",
  "arguments": {
    "openapi_spec": "openapi/petstore.yaml",
    "spec_format": "yaml",
    "generation_target": "full",
    "framework": "rmcp",
    "output_dir": "src/generated/api",
    "validation_strategy": "strict"
  }
}
```

### Example Response

```json
{
  "api_name": "Petstore API",
  "api_version": "1.0.0",
  "output_dir": "src/generated/api",
  "files_generated": [
    {"path": "src/generated/api/models/pet.rs", "kind": "model", "lines": 87},
    {"path": "src/generated/api/models/category.rs", "kind": "model", "lines": 34},
    {"path": "src/generated/api/handlers/pets.rs", "kind": "handler", "lines": 142},
    {"path": "src/generated/api/validators/pet_validator.rs", "kind": "validator", "lines": 63},
    {"path": "src/generated/api/mod.rs", "kind": "mod", "lines": 28},
    {"path": "tests/generated/api/pets_test.rs", "kind": "test", "lines": 95}
  ],
  "operations": [
    {"operation_id": "listPets", "method": "GET", "path": "/pets", "handler_name": "list_pets"},
    {"operation_id": "createPet", "method": "POST", "path": "/pets", "handler_name": "create_pet"},
    {"operation_id": "getPet", "method": "GET", "path": "/pets/{petId}", "handler_name": "get_pet"}
  ],
  "models": [
    {"name": "Pet", "schema_ref": "#/components/schemas/Pet", "fields": 5},
    {"name": "Category", "schema_ref": "#/components/schemas/Category", "fields": 2}
  ],
  "statistics": {
    "total_lines": 449,
    "total_files": 6,
    "endpoints": 3,
    "models": 2,
    "validators": 2
  }
}
```

### Error Codes

| Code | Meaning | Recovery |
|------|---------|----------|
| `INVALID_OPENAPI` | Spec validation failed | Fix OpenAPI spec syntax |
| `UNSUPPORTED_OPENAPI_VERSION` | Only OpenAPI 3.x supported | Upgrade spec to 3.0+ |
| `MISSING_OPERATION_ID` | Operation lacks operationId | Add operationId to all operations |
| `INVALID_SCHEMA_REF` | $ref resolution failed | Fix schema references |
| `GENERATION_CONFLICT` | File already exists | Use force flag or different output_dir |

---

## Tool 4: preview_generation

**Purpose**: Dry-run preview of generation output without writing files to disk.

### Parameters Schema

```json
{
  "generation_config": {
    "type": "object",
    "description": "Configuration for any generation tool (validate_ontology, generate_from_schema, generate_from_openapi, sync_ontology)",
    "required": true,
    "example": {
      "tool": "generate_from_schema",
      "arguments": {
        "schema_type": "zod",
        "schema_content": "z.object({ id: z.string() })",
        "entity_name": "Entity"
      }
    }
  },
  "show_diffs": {
    "type": "boolean",
    "description": "Show diffs if files already exist",
    "required": false,
    "default": false,
    "example": true
  },
  "include_full_code": {
    "type": "boolean",
    "description": "Include generated code in response",
    "required": false,
    "default": false,
    "example": true
  }
}
```

### Response Schema

```json
{
  "preview_mode": {"type": "boolean", "const": true},
  "files_to_generate": {
    "type": "array",
    "items": {
      "type": "object",
      "properties": {
        "path": {"type": "string"},
        "action": {"type": "string", "enum": ["create", "update", "skip"]},
        "size_bytes": {"type": "integer"},
        "diff": {"type": "string", "nullable": true, "description": "Unified diff if show_diffs=true"},
        "code": {"type": "string", "nullable": true, "description": "Full code if include_full_code=true"}
      }
    }
  },
  "statistics": {
    "type": "object",
    "properties": {
      "files_to_create": {"type": "integer"},
      "files_to_update": {"type": "integer"},
      "files_to_skip": {"type": "integer"},
      "total_bytes": {"type": "integer"}
    }
  },
  "warnings": {
    "type": "array",
    "items": {"type": "string"},
    "description": "Potential issues (conflicts, overwrites)"
  }
}
```

### Example Request

```json
{
  "tool": "preview_generation",
  "arguments": {
    "generation_config": {
      "tool": "generate_from_schema",
      "arguments": {
        "schema_type": "zod",
        "schema_content": "z.object({ id: z.string().uuid(), name: z.string() })",
        "entity_name": "Product",
        "output_path": "src/domain/product.rs"
      }
    },
    "show_diffs": true,
    "include_full_code": false
  }
}
```

### Example Response

```json
{
  "preview_mode": true,
  "files_to_generate": [
    {
      "path": "src/domain/product.rs",
      "action": "update",
      "size_bytes": 1247,
      "diff": "--- src/domain/product.rs\t2026-01-20 10:23:45\n+++ src/domain/product.rs\t2026-01-20 10:24:12\n@@ -5,6 +5,7 @@\n pub struct Product {\n     pub id: uuid::Uuid,\n+    pub name: String,\n }\n",
      "code": null
    }
  ],
  "statistics": {
    "files_to_create": 0,
    "files_to_update": 1,
    "files_to_skip": 0,
    "total_bytes": 1247
  },
  "warnings": [
    "File 'src/domain/product.rs' exists and will be overwritten"
  ]
}
```

### Error Codes

| Code | Meaning | Recovery |
|------|---------|----------|
| `INVALID_GENERATION_CONFIG` | Nested tool config invalid | Fix generation_config structure |
| `PREVIEW_FAILED` | Preview execution error | Check nested tool parameters |

---

## Tool 5: sync_ontology

**Purpose**: Full ontology synchronization pipeline (13 steps) - Load → Validate → Extract → Generate → Format → Verify.

### Parameters Schema

```json
{
  "ontology_path": {
    "type": "string",
    "description": "Path to ontology file or directory",
    "required": true,
    "example": "ontology/mcp-domain.ttl"
  },
  "config_path": {
    "type": "string",
    "description": "Path to ggen.toml configuration",
    "required": false,
    "default": "ggen.toml",
    "example": "meta/ggen.toml"
  },
  "force": {
    "type": "boolean",
    "description": "Force regeneration of all files",
    "required": false,
    "default": false,
    "example": false
  },
  "audit_trail": {
    "type": "boolean",
    "description": "Generate cryptographic audit receipt",
    "required": false,
    "default": true,
    "example": true
  },
  "validation_level": {
    "type": "string",
    "enum": ["minimal", "standard", "strict"],
    "description": "Validation strictness level",
    "required": false,
    "default": "standard",
    "example": "strict"
  },
  "parallel_generation": {
    "type": "boolean",
    "description": "Enable parallel file generation",
    "required": false,
    "default": true,
    "example": true
  }
}
```

### Response Schema

```json
{
  "sync_id": {"type": "string", "description": "Unique sync execution ID"},
  "timestamp": {"type": "string", "format": "date-time"},
  "status": {"type": "string", "enum": ["success", "partial", "failed"]},
  "pipeline_stages": {
    "type": "array",
    "items": {
      "type": "object",
      "properties": {
        "stage": {"type": "string"},
        "status": {"type": "string", "enum": ["completed", "failed", "skipped"]},
        "duration_ms": {"type": "integer"},
        "details": {"type": "string"}
      }
    },
    "description": "13-step pipeline execution details"
  },
  "files_generated": {
    "type": "array",
    "items": {
      "type": "object",
      "properties": {
        "path": {"type": "string"},
        "hash": {"type": "string", "description": "SHA-256 content hash"},
        "size_bytes": {"type": "integer"}
      }
    }
  },
  "validation_results": {
    "type": "object",
    "properties": {
      "ontology_valid": {"type": "boolean"},
      "queries_valid": {"type": "boolean"},
      "templates_valid": {"type": "boolean"},
      "generated_code_compiles": {"type": "boolean"},
      "tests_pass": {"type": "boolean"}
    }
  },
  "audit_receipt": {
    "type": "object",
    "nullable": true,
    "properties": {
      "receipt_id": {"type": "string"},
      "ontology_hash": {"type": "string"},
      "config_hash": {"type": "string"},
      "output_hash": {"type": "string"},
      "receipt_path": {"type": "string"}
    }
  },
  "statistics": {
    "type": "object",
    "properties": {
      "total_duration_ms": {"type": "integer"},
      "files_generated": {"type": "integer"},
      "lines_of_code": {"type": "integer"},
      "sparql_queries_executed": {"type": "integer"},
      "templates_rendered": {"type": "integer"}
    }
  },
  "errors": {
    "type": "array",
    "items": {
      "type": "object",
      "properties": {
        "stage": {"type": "string"},
        "severity": {"type": "string"},
        "message": {"type": "string"}
      }
    }
  }
}
```

### Example Request

```json
{
  "tool": "sync_ontology",
  "arguments": {
    "ontology_path": "ontology/mcp-domain.ttl",
    "config_path": "ggen.toml",
    "force": false,
    "audit_trail": true,
    "validation_level": "strict",
    "parallel_generation": true
  }
}
```

### Example Response (Success)

```json
{
  "sync_id": "sync-20260120-102345-a7b9c1d2",
  "timestamp": "2026-01-20T10:23:45Z",
  "status": "success",
  "pipeline_stages": [
    {"stage": "1. Load Ontology", "status": "completed", "duration_ms": 234, "details": "Loaded 347 triples"},
    {"stage": "2. Validate SHACL", "status": "completed", "duration_ms": 89, "details": "All constraints satisfied"},
    {"stage": "3. Resolve Dependencies", "status": "completed", "duration_ms": 12, "details": "2 imports resolved"},
    {"stage": "4. Execute SPARQL Queries", "status": "completed", "duration_ms": 456, "details": "14 queries executed"},
    {"stage": "5. Validate Query Results", "status": "completed", "duration_ms": 23, "details": "All results valid"},
    {"stage": "6. Render Tera Templates", "status": "completed", "duration_ms": 789, "details": "21 templates rendered"},
    {"stage": "7. Validate Generated Code", "status": "completed", "duration_ms": 145, "details": "Syntax valid"},
    {"stage": "8. Format with rustfmt", "status": "completed", "duration_ms": 312, "details": "21 files formatted"},
    {"stage": "9. Check Compilation", "status": "completed", "duration_ms": 3421, "details": "cargo check passed"},
    {"stage": "10. Detect TODOs", "status": "completed", "duration_ms": 67, "details": "0 TODOs found"},
    {"stage": "11. Run Tests", "status": "completed", "duration_ms": 2134, "details": "347 tests passed"},
    {"stage": "12. Generate Audit Receipt", "status": "completed", "duration_ms": 45, "details": "Receipt generated"},
    {"stage": "13. Write Files", "status": "completed", "duration_ms": 189, "details": "21 files written"}
  ],
  "files_generated": [
    {"path": "src/generated/domain/entities/user.rs", "hash": "a7b9c1d2e3f4567890abcdef12345678", "size_bytes": 1247},
    {"path": "src/generated/domain/entities/product.rs", "hash": "1234567890abcdefa7b9c1d2e3f45678", "size_bytes": 983}
  ],
  "validation_results": {
    "ontology_valid": true,
    "queries_valid": true,
    "templates_valid": true,
    "generated_code_compiles": true,
    "tests_pass": true
  },
  "audit_receipt": {
    "receipt_id": "receipt-20260120-102345-a7b9c1d2",
    "ontology_hash": "sha256:347triples-a7b9c1d2",
    "config_hash": "sha256:ggen-toml-1234567890",
    "output_hash": "sha256:21files-abcdef123456",
    "receipt_path": ".ggen/receipts/receipt-20260120-102345-a7b9c1d2.json"
  },
  "statistics": {
    "total_duration_ms": 7916,
    "files_generated": 21,
    "lines_of_code": 4892,
    "sparql_queries_executed": 14,
    "templates_rendered": 21
  },
  "errors": []
}
```

### Example Response (Partial Failure)

```json
{
  "sync_id": "sync-20260120-103456-b8c0d2e3",
  "timestamp": "2026-01-20T10:34:56Z",
  "status": "failed",
  "pipeline_stages": [
    {"stage": "1. Load Ontology", "status": "completed", "duration_ms": 234, "details": "Loaded 347 triples"},
    {"stage": "2. Validate SHACL", "status": "failed", "duration_ms": 89, "details": "2 constraint violations"},
    {"stage": "3. Resolve Dependencies", "status": "skipped", "duration_ms": 0, "details": "Skipped due to validation failure"}
  ],
  "files_generated": [],
  "validation_results": {
    "ontology_valid": false,
    "queries_valid": false,
    "templates_valid": false,
    "generated_code_compiles": false,
    "tests_pass": false
  },
  "audit_receipt": null,
  "statistics": {
    "total_duration_ms": 323,
    "files_generated": 0,
    "lines_of_code": 0,
    "sparql_queries_executed": 0,
    "templates_rendered": 0
  },
  "errors": [
    {
      "stage": "2. Validate SHACL",
      "severity": "error",
      "message": "Property mcp:hasParameter violates sh:minCount constraint (expected >= 1, found 0)"
    },
    {
      "stage": "2. Validate SHACL",
      "severity": "error",
      "message": "Class mcp:Tool missing required rdfs:label annotation"
    }
  ]
}
```

### Error Codes

| Code | Meaning | Recovery |
|------|---------|----------|
| `ONTOLOGY_INVALID` | Ontology validation failed | Fix SHACL violations reported in errors |
| `SPARQL_EXECUTION_FAILED` | Query execution error | Check SPARQL syntax in queries/*.rq |
| `TEMPLATE_RENDERING_FAILED` | Tera template error | Fix template syntax in templates/*.tera |
| `COMPILATION_FAILED` | Generated code doesn't compile | Fix ontology/template producing invalid Rust |
| `TEST_FAILURE` | Generated code tests failed | Review test failures, fix ontology |
| `TODO_DETECTED` | Generated code contains TODO markers | Remove hardcoded TODOs from templates |
| `AUDIT_WRITE_FAILED` | Cannot write receipt | Check filesystem permissions |

---

## Best Practices

### 1. Always Validate First
```bash
# Before sync, validate ontology
validate_ontology → sync_ontology
```

### 2. Use Preview Mode for Safety
```bash
# Preview changes before applying
preview_generation → (review) → generate_*
```

### 3. Enable Audit Trails for Production
```json
{
  "audit_trail": true  // Always in production
}
```

### 4. Strict Validation in CI/CD
```json
{
  "validation_level": "strict",  // Catch issues early
  "strict_mode": true
}
```

### 5. Parallel Generation for Speed
```json
{
  "parallel_generation": true  // 2-3x faster for large ontologies
}
```

---

## Error Handling Patterns

### Pattern 1: Retry with Backoff
```rust
let mut retries = 3;
while retries > 0 {
    match sync_ontology(params) {
        Ok(result) => return Ok(result),
        Err(e) if e.is_transient() => {
            retries -= 1;
            tokio::time::sleep(Duration::from_secs(2u64.pow(3 - retries))).await;
        }
        Err(e) => return Err(e),
    }
}
```

### Pattern 2: Fallback to Preview
```rust
match sync_ontology(params) {
    Err(e) if e.code() == "COMPILATION_FAILED" => {
        // Fallback to preview to see what would be generated
        preview_generation(preview_params)?
    }
    result => result,
}
```

### Pattern 3: Incremental Recovery
```rust
// If full sync fails, try minimal validation
if sync_ontology(params).is_err() {
    validate_ontology(validate_params)?; // Identify root cause
}
```

---

## Performance Characteristics

| Tool | Typical Latency | Bottleneck | Optimization |
|------|----------------|------------|--------------|
| `validate_ontology` | 100ms-1s | SHACL validation | Cache validation results |
| `generate_from_schema` | 1-5s | Template rendering | Use simpler templates |
| `generate_from_openapi` | 5-15s | Large specs | Split into smaller specs |
| `preview_generation` | 500ms-3s | Diff computation | Disable diffs if not needed |
| `sync_ontology` | 10-30s | Compilation check | Use `cargo make check` (faster) |

**Scaling**:
- Ontologies <1000 triples: <10s
- Ontologies 1000-10000 triples: 10-30s
- Ontologies >10000 triples: 30-60s

---

## Security Considerations

### 1. Path Traversal Prevention
All file paths validated against workspace root. Paths outside workspace rejected.

### 2. SPARQL Injection Prevention
SPARQL queries parameterized, not concatenated. No user input in query strings.

### 3. Template Injection Prevention
Tera templates sandboxed, no filesystem access beyond workspace root.

### 4. Audit Trail Integrity
All receipts cryptographically signed with SHA-256 hashes. Tamper detection included.

---

## Tool Comparison

| Feature | validate_ontology | generate_from_schema | generate_from_openapi | preview_generation | sync_ontology |
|---------|-------------------|----------------------|-----------------------|--------------------|---------------|
| Validates ontology | ✓ | - | - | ✓ (nested) | ✓ |
| Generates code | - | ✓ | ✓ | - (preview only) | ✓ |
| File writes | - | ✓ | ✓ | - | ✓ |
| Compilation check | - | - | - | - | ✓ |
| Test execution | - | - | - | - | ✓ |
| Audit trail | - | - | - | - | ✓ |
| Dry-run mode | - | - | - | ✓ (always) | - |

**Recommendation**:
- **Quick validation**: `validate_ontology`
- **Single entity**: `generate_from_schema`
- **Full API**: `generate_from_openapi`
- **Safety check**: `preview_generation` before any generation
- **Production sync**: `sync_ontology` with `audit_trail=true`

---

**Version**: 1.0.0 | **Last Updated**: 2026-01-20
