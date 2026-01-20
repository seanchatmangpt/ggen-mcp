# Workflow Examples - Ontology Generation

**Version**: 1.0.0 | Real-world patterns | Production-ready workflows

---

## Quick Reference

| Example | Use Case | Complexity | Duration |
|---------|----------|------------|----------|
| [1. Simple Entity (Zod)](#example-1-simple-entity-generation-zod-schema) | Single domain entity | Low | 2-5s |
| [2. OpenAPI Spec](#example-2-openapi-spec-generation) | Complete REST API | Medium | 5-15s |
| [3. Full Workflow (13 Steps)](#example-3-full-workflow-13-steps) | Production ontology sync | High | 10-30s |
| [4. Preview Mode](#example-4-preview-mode-usage) | Safe change review | Low | 1-3s |
| [5. Error Recovery](#example-5-error-recovery-patterns) | Handle failures gracefully | Medium | Variable |

---

## Example 1: Simple Entity Generation (Zod Schema)

**Goal**: Generate a `User` entity from Zod schema with validation and builder pattern.

### Step 1: Define Schema

```typescript
// Input Zod schema
const UserSchema = z.object({
  id: z.string().uuid(),
  email: z.string().email(),
  username: z.string().min(3).max(20),
  age: z.number().int().min(18).max(120),
  created_at: z.string().datetime(),
  is_active: z.boolean().default(true),
});
```

### Step 2: Validate Schema First

```json
{
  "tool": "validate_ontology",
  "arguments": {
    "ontology_path": "schemas/user_schema.zod",
    "strict_mode": false,
    "resolve_imports": false
  }
}
```

**Response**:
```json
{
  "status": "valid",
  "errors": [],
  "statistics": {
    "triple_count": 0,
    "class_count": 0,
    "property_count": 6,
    "constraint_count": 8
  }
}
```

### Step 3: Generate Entity

```json
{
  "tool": "generate_from_schema",
  "arguments": {
    "schema_type": "zod",
    "schema_content": "z.object({ id: z.string().uuid(), email: z.string().email(), username: z.string().min(3).max(20), age: z.number().int().min(18).max(120), created_at: z.string().datetime(), is_active: z.boolean().default(true) })",
    "entity_name": "User",
    "features": ["serde", "validation", "builder", "debug"],
    "output_path": "src/domain/entities/user.rs"
  }
}
```

### Step 4: Review Generated Code

**Response**:
```json
{
  "entity_name": "User",
  "output_path": "src/domain/entities/user.rs",
  "features_applied": ["serde", "validation", "builder", "debug"],
  "validation_rules": [
    {"field": "id", "rule": "uuid", "constraint": "valid UUID v4"},
    {"field": "email", "rule": "email", "constraint": "RFC 5322 format"},
    {"field": "username", "rule": "length", "constraint": "3 <= len <= 20"},
    {"field": "age", "rule": "range", "constraint": "18 <= age <= 120"}
  ],
  "statistics": {
    "fields_generated": 6,
    "lines_of_code": 127,
    "validation_rules": 4
  }
}
```

### Generated Code Preview

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub age: u8,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
}

impl User {
    pub fn validate(&self) -> Result<(), ValidationError> {
        validate_email(&self.email)
            .map_err(|_| ValidationError::InvalidEmail)?;

        if self.username.len() < 3 || self.username.len() > 20 {
            return Err(ValidationError::InvalidUsernameLength);
        }

        if self.age < 18 || self.age > 120 {
            return Err(ValidationError::InvalidAge);
        }

        Ok(())
    }
}

#[derive(Default)]
pub struct UserBuilder {
    id: Option<Uuid>,
    email: Option<String>,
    username: Option<String>,
    age: Option<u8>,
    created_at: Option<DateTime<Utc>>,
    is_active: Option<bool>,
}

impl UserBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    pub fn age(mut self, age: u8) -> Self {
        self.age = Some(age);
        self
    }

    pub fn created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = Some(created_at);
        self
    }

    pub fn is_active(mut self, is_active: bool) -> Self {
        self.is_active = Some(is_active);
        self
    }

    pub fn build(self) -> Result<User, BuilderError> {
        let user = User {
            id: self.id.ok_or(BuilderError::MissingField("id"))?,
            email: self.email.ok_or(BuilderError::MissingField("email"))?,
            username: self.username.ok_or(BuilderError::MissingField("username"))?,
            age: self.age.ok_or(BuilderError::MissingField("age"))?,
            created_at: self.created_at.unwrap_or_else(Utc::now),
            is_active: self.is_active.unwrap_or(true),
        };

        user.validate()?;
        Ok(user)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Invalid email format")]
    InvalidEmail,
    #[error("Username must be between 3 and 20 characters")]
    InvalidUsernameLength,
    #[error("Age must be between 18 and 120")]
    InvalidAge,
}

#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    #[error("Missing required field: {0}")]
    MissingField(&'static str),
    #[error("Validation failed: {0}")]
    Validation(#[from] ValidationError),
}
```

### Step 5: Use Generated Code

```rust
use crate::domain::entities::user::{User, UserBuilder};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Using builder pattern
    let user = UserBuilder::new()
        .id(Uuid::new_v4())
        .email("alice@example.com")
        .username("alice123")
        .age(25)
        .is_active(true)
        .build()?;

    // Validation is automatic in builder
    assert!(user.validate().is_ok());

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&user)?;
    println!("{}", json);

    Ok(())
}
```

**Output**:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "email": "alice@example.com",
  "username": "alice123",
  "age": 25,
  "created_at": "2026-01-20T10:23:45Z",
  "is_active": true
}
```

---

## Example 2: OpenAPI Spec Generation

**Goal**: Generate complete REST API implementation from OpenAPI 3.x specification.

### Step 1: Prepare OpenAPI Spec

```yaml
# openapi/petstore.yaml
openapi: 3.0.3
info:
  title: Petstore API
  version: 1.0.0
paths:
  /pets:
    get:
      operationId: listPets
      summary: List all pets
      parameters:
        - name: limit
          in: query
          schema:
            type: integer
            minimum: 1
            maximum: 100
            default: 20
      responses:
        '200':
          description: Array of pets
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/Pet'
    post:
      operationId: createPet
      summary: Create a pet
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CreatePetRequest'
      responses:
        '201':
          description: Pet created
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Pet'
  /pets/{petId}:
    get:
      operationId: getPet
      summary: Get pet by ID
      parameters:
        - name: petId
          in: path
          required: true
          schema:
            type: string
            format: uuid
      responses:
        '200':
          description: Pet details
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Pet'
        '404':
          description: Pet not found
components:
  schemas:
    Pet:
      type: object
      required: [id, name]
      properties:
        id:
          type: string
          format: uuid
        name:
          type: string
          minLength: 1
          maxLength: 100
        category:
          $ref: '#/components/schemas/Category'
        tags:
          type: array
          items:
            type: string
    Category:
      type: object
      properties:
        id:
          type: integer
        name:
          type: string
    CreatePetRequest:
      type: object
      required: [name]
      properties:
        name:
          type: string
          minLength: 1
          maxLength: 100
        category_id:
          type: integer
        tags:
          type: array
          items:
            type: string
```

### Step 2: Preview Generation

```json
{
  "tool": "preview_generation",
  "arguments": {
    "generation_config": {
      "tool": "generate_from_openapi",
      "arguments": {
        "openapi_spec": "openapi/petstore.yaml",
        "spec_format": "yaml",
        "generation_target": "full",
        "framework": "rmcp",
        "output_dir": "src/generated/api",
        "validation_strategy": "strict"
      }
    },
    "show_diffs": false,
    "include_full_code": false
  }
}
```

**Response**:
```json
{
  "preview_mode": true,
  "files_to_generate": [
    {"path": "src/generated/api/models/pet.rs", "action": "create", "size_bytes": 1847},
    {"path": "src/generated/api/models/category.rs", "action": "create", "size_bytes": 634},
    {"path": "src/generated/api/models/create_pet_request.rs", "action": "create", "size_bytes": 789},
    {"path": "src/generated/api/handlers/pets.rs", "action": "create", "size_bytes": 3142},
    {"path": "src/generated/api/validators/pet_validator.rs", "action": "create", "size_bytes": 963},
    {"path": "src/generated/api/mod.rs", "action": "create", "size_bytes": 428},
    {"path": "tests/generated/api/pets_test.rs", "action": "create", "size_bytes": 1895}
  ],
  "statistics": {
    "files_to_create": 7,
    "files_to_update": 0,
    "files_to_skip": 0,
    "total_bytes": 9698
  },
  "warnings": []
}
```

### Step 3: Generate Full API

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

**Response**:
```json
{
  "api_name": "Petstore API",
  "api_version": "1.0.0",
  "output_dir": "src/generated/api",
  "files_generated": [
    {"path": "src/generated/api/models/pet.rs", "kind": "model", "lines": 87},
    {"path": "src/generated/api/models/category.rs", "kind": "model", "lines": 34},
    {"path": "src/generated/api/models/create_pet_request.rs", "kind": "model", "lines": 42},
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
    {"name": "Pet", "schema_ref": "#/components/schemas/Pet", "fields": 4},
    {"name": "Category", "schema_ref": "#/components/schemas/Category", "fields": 2},
    {"name": "CreatePetRequest", "schema_ref": "#/components/schemas/CreatePetRequest", "fields": 3}
  ],
  "statistics": {
    "total_lines": 491,
    "total_files": 7,
    "endpoints": 3,
    "models": 3,
    "validators": 1
  }
}
```

### Step 4: Generated Handler Example

```rust
// src/generated/api/handlers/pets.rs
use crate::generated::api::models::{Pet, CreatePetRequest};
use crate::generated::api::validators::PetValidator;
use anyhow::Result;
use rmcp::{ErrorData as McpError, Json, Parameters, tool};
use std::sync::Arc;
use uuid::Uuid;

#[tool(
    name = "list_pets",
    description = "List all pets with optional limit"
)]
pub async fn list_pets(
    state: Arc<AppState>,
    Parameters(params): Parameters<ListPetsParams>,
) -> Result<Json<Vec<Pet>>, McpError> {
    let limit = params.limit.unwrap_or(20);

    if limit < 1 || limit > 100 {
        return Err(McpError::invalid_params(
            "limit must be between 1 and 100",
            None,
        ));
    }

    let pets = state
        .db
        .list_pets(limit as usize)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    Ok(Json(pets))
}

#[derive(Debug, serde::Deserialize)]
pub struct ListPetsParams {
    #[serde(default)]
    pub limit: Option<i32>,
}

#[tool(
    name = "create_pet",
    description = "Create a new pet"
)]
pub async fn create_pet(
    state: Arc<AppState>,
    Parameters(params): Parameters<CreatePetRequest>,
) -> Result<Json<Pet>, McpError> {
    // Validate request
    PetValidator::validate_create_request(&params)
        .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

    let pet = Pet {
        id: Uuid::new_v4(),
        name: params.name,
        category: params.category_id.and_then(|id| {
            // Fetch category from DB
            None // Placeholder
        }),
        tags: params.tags.unwrap_or_default(),
    };

    state
        .db
        .insert_pet(&pet)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    Ok(Json(pet))
}

#[tool(
    name = "get_pet",
    description = "Get pet by ID"
)]
pub async fn get_pet(
    state: Arc<AppState>,
    Parameters(params): Parameters<GetPetParams>,
) -> Result<Json<Pet>, McpError> {
    let pet = state
        .db
        .get_pet(&params.pet_id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
        .ok_or_else(|| McpError::not_found("Pet not found", None))?;

    Ok(Json(pet))
}

#[derive(Debug, serde::Deserialize)]
pub struct GetPetParams {
    pub pet_id: Uuid,
}
```

### Step 5: Register Handlers in Server

```rust
// src/server.rs
use crate::generated::api::handlers;

#[tool_router]
impl PetstoreServer {
    // Handlers are automatically registered via #[tool] macro
    // No manual registration needed
}
```

---

## Example 3: Full Workflow (13 Steps)

**Goal**: Production-ready ontology sync with validation, generation, testing, and audit trail.

### Step 1: Ontology Structure

```turtle
# ontology/mcp-domain.ttl
@prefix mcp: <http://example.org/mcp#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .

# Tool class
mcp:Tool a rdfs:Class ;
    rdfs:label "MCP Tool" ;
    rdfs:comment "An MCP tool that can be invoked by clients" .

# Parameter class
mcp:Parameter a rdfs:Class ;
    rdfs:label "Tool Parameter" ;
    rdfs:comment "A parameter accepted by an MCP tool" .

# Properties
mcp:hasParameter a rdfs:Property ;
    rdfs:domain mcp:Tool ;
    rdfs:range mcp:Parameter .

mcp:parameterName a rdfs:Property ;
    rdfs:domain mcp:Parameter ;
    rdfs:range rdfs:Literal .

mcp:parameterType a rdfs:Property ;
    rdfs:domain mcp:Parameter ;
    rdfs:range rdfs:Literal .

# SHACL constraints
mcp:ToolShape a sh:NodeShape ;
    sh:targetClass mcp:Tool ;
    sh:property [
        sh:path rdfs:label ;
        sh:minCount 1 ;
        sh:datatype rdfs:Literal ;
    ] ;
    sh:property [
        sh:path mcp:hasParameter ;
        sh:minCount 1 ;
        sh:class mcp:Parameter ;
    ] .

# Tool instances
mcp:ValidateOntology a mcp:Tool ;
    rdfs:label "validate_ontology" ;
    rdfs:comment "Validate RDF/Turtle ontology files" ;
    mcp:hasParameter mcp:OntologyPathParam ;
    mcp:hasParameter mcp:StrictModeParam .

mcp:OntologyPathParam a mcp:Parameter ;
    mcp:parameterName "ontology_path" ;
    mcp:parameterType "string" ;
    mcp:required true .

mcp:StrictModeParam a mcp:Parameter ;
    mcp:parameterName "strict_mode" ;
    mcp:parameterType "boolean" ;
    mcp:required false .
```

### Step 2: SPARQL Query

```sparql
# queries/extract_tools.rq
PREFIX mcp: <http://example.org/mcp#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

SELECT ?tool ?label ?comment ?param_name ?param_type ?param_required
WHERE {
    ?tool a mcp:Tool ;
          rdfs:label ?label ;
          rdfs:comment ?comment ;
          mcp:hasParameter ?param .

    ?param mcp:parameterName ?param_name ;
           mcp:parameterType ?param_type .

    OPTIONAL { ?param mcp:required ?param_required }
}
ORDER BY ?tool ?param_name
```

### Step 3: Tera Template

```rust
// templates/tool_handlers.rs.tera
{% for tool in tools %}
#[tool(
    name = "{{ tool.label }}",
    description = "{{ tool.comment }}"
)]
pub async fn {{ tool.label | snake_case }}(
    state: Arc<AppState>,
    Parameters(params): Parameters<{{ tool.label | pascal_case }}Params>,
) -> Result<Json<{{ tool.label | pascal_case }}Response>, McpError> {
    // Validate parameters
    {% for param in tool.parameters %}
    {% if param.required %}
    if params.{{ param.name }}.is_none() {
        return Err(McpError::invalid_params(
            "Missing required parameter: {{ param.name }}",
            None,
        ));
    }
    {% endif %}
    {% endfor %}

    // TODO: Implementation
    unimplemented!("{{ tool.label }} not yet implemented")
}

#[derive(Debug, serde::Deserialize)]
pub struct {{ tool.label | pascal_case }}Params {
    {% for param in tool.parameters %}
    {% if param.required %}
    pub {{ param.name }}: {{ param.type | rust_type }},
    {% else %}
    #[serde(default)]
    pub {{ param.name }}: Option<{{ param.type | rust_type }}>,
    {% endif %}
    {% endfor %}
}
{% endfor %}
```

### Step 4: Generation Configuration

```toml
# ggen.toml
[manifest]
name = "mcp-tools"
version = "1.0.0"

[[generation]]
name = "tool_handlers"
sparql = "queries/extract_tools.rq"
template = "templates/tool_handlers.rs.tera"
output = "src/generated/tools.rs"

[validation]
check_compilation = true
check_tests = true
allow_todos = false

[audit]
enabled = true
output_dir = ".ggen/receipts"
```

### Step 5: Execute Full Sync

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

### Step 6: Monitor Pipeline Stages

**Response** (13 stages):
```json
{
  "sync_id": "sync-20260120-102345-a7b9c1d2",
  "timestamp": "2026-01-20T10:23:45Z",
  "status": "success",
  "pipeline_stages": [
    {
      "stage": "1. Load Ontology",
      "status": "completed",
      "duration_ms": 234,
      "details": "Loaded 347 triples from ontology/mcp-domain.ttl"
    },
    {
      "stage": "2. Validate SHACL",
      "status": "completed",
      "duration_ms": 89,
      "details": "All SHACL constraints satisfied (8 shapes validated)"
    },
    {
      "stage": "3. Resolve Dependencies",
      "status": "completed",
      "duration_ms": 12,
      "details": "Resolved 2 imports: rdfs, shacl"
    },
    {
      "stage": "4. Execute SPARQL Queries",
      "status": "completed",
      "duration_ms": 456,
      "details": "Executed queries/extract_tools.rq (1 query, 5 tools extracted)"
    },
    {
      "stage": "5. Validate Query Results",
      "status": "completed",
      "duration_ms": 23,
      "details": "All query results valid (5 tools, 12 parameters)"
    },
    {
      "stage": "6. Render Tera Templates",
      "status": "completed",
      "duration_ms": 789,
      "details": "Rendered templates/tool_handlers.rs.tera (1 template, 247 lines)"
    },
    {
      "stage": "7. Validate Generated Code",
      "status": "completed",
      "duration_ms": 145,
      "details": "Syntax validation passed (rustfmt dry-run)"
    },
    {
      "stage": "8. Format with rustfmt",
      "status": "completed",
      "duration_ms": 312,
      "details": "Formatted 1 file (247 lines)"
    },
    {
      "stage": "9. Check Compilation",
      "status": "completed",
      "duration_ms": 3421,
      "details": "cargo check passed (0 errors, 0 warnings)"
    },
    {
      "stage": "10. Detect TODOs",
      "status": "completed",
      "duration_ms": 67,
      "details": "Found 5 TODOs (expected for unimplemented functions)"
    },
    {
      "stage": "11. Run Tests",
      "status": "completed",
      "duration_ms": 2134,
      "details": "cargo test passed (347 tests, 0 failures)"
    },
    {
      "stage": "12. Generate Audit Receipt",
      "status": "completed",
      "duration_ms": 45,
      "details": "Generated receipt: .ggen/receipts/receipt-20260120-102345-a7b9c1d2.json"
    },
    {
      "stage": "13. Write Files",
      "status": "completed",
      "duration_ms": 189,
      "details": "Wrote 1 file to src/generated/tools.rs"
    }
  ],
  "files_generated": [
    {
      "path": "src/generated/tools.rs",
      "hash": "sha256:a7b9c1d2e3f4567890abcdef1234567890abcdef1234567890abcdef12345678",
      "size_bytes": 8742
    }
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
    "ontology_hash": "sha256:347triples-a7b9c1d2e3f4567890",
    "config_hash": "sha256:ggen-toml-1234567890abcdef",
    "output_hash": "sha256:1file-abcdef1234567890",
    "receipt_path": ".ggen/receipts/receipt-20260120-102345-a7b9c1d2.json"
  },
  "statistics": {
    "total_duration_ms": 7916,
    "files_generated": 1,
    "lines_of_code": 247,
    "sparql_queries_executed": 1,
    "templates_rendered": 1
  },
  "errors": []
}
```

### Step 7: Review Audit Receipt

```bash
cat .ggen/receipts/receipt-20260120-102345-a7b9c1d2.json
```

```json
{
  "receipt_id": "receipt-20260120-102345-a7b9c1d2",
  "timestamp": "2026-01-20T10:23:45Z",
  "inputs": {
    "ontology_path": "ontology/mcp-domain.ttl",
    "ontology_hash": "sha256:347triples-a7b9c1d2e3f4567890",
    "config_path": "ggen.toml",
    "config_hash": "sha256:ggen-toml-1234567890abcdef"
  },
  "outputs": {
    "files": [
      {
        "path": "src/generated/tools.rs",
        "hash": "sha256:a7b9c1d2e3f4567890abcdef1234567890abcdef1234567890abcdef12345678",
        "size_bytes": 8742
      }
    ],
    "output_hash": "sha256:1file-abcdef1234567890"
  },
  "execution": {
    "duration_ms": 7916,
    "stages": 13,
    "queries_executed": 1,
    "templates_rendered": 1
  },
  "validation": {
    "shacl_valid": true,
    "compilation_passed": true,
    "tests_passed": true,
    "todos_found": 5
  }
}
```

---

## Example 4: Preview Mode Usage

**Goal**: Review generation changes before applying them to avoid unintended overwrites.

### Scenario: Updating Existing Entity

```rust
// src/domain/user.rs (existing)
#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub email: String,
}
```

### Step 1: Preview Schema Update

```json
{
  "tool": "preview_generation",
  "arguments": {
    "generation_config": {
      "tool": "generate_from_schema",
      "arguments": {
        "schema_type": "zod",
        "schema_content": "z.object({ id: z.string().uuid(), email: z.string().email(), username: z.string() })",
        "entity_name": "User",
        "features": ["serde", "validation"],
        "output_path": "src/domain/user.rs"
      }
    },
    "show_diffs": true,
    "include_full_code": false
  }
}
```

### Step 2: Review Diff

**Response**:
```json
{
  "preview_mode": true,
  "files_to_generate": [
    {
      "path": "src/domain/user.rs",
      "action": "update",
      "size_bytes": 1847,
      "diff": "--- src/domain/user.rs\t2026-01-19 15:23:45\n+++ src/domain/user.rs\t2026-01-20 10:23:45\n@@ -1,6 +1,17 @@\n-#[derive(Debug, Clone)]\n+use serde::{Deserialize, Serialize};\n+use uuid::Uuid;\n+\n+#[derive(Debug, Clone, Serialize, Deserialize)]\n pub struct User {\n-    pub id: String,\n+    pub id: Uuid,\n     pub email: String,\n+    pub username: String,\n+}\n+\n+impl User {\n+    pub fn validate(&self) -> Result<(), ValidationError> {\n+        validate_email(&self.email)?;\n+        Ok(())\n+    }\n }",
      "code": null
    }
  ],
  "statistics": {
    "files_to_create": 0,
    "files_to_update": 1,
    "files_to_skip": 0,
    "total_bytes": 1847
  },
  "warnings": [
    "File 'src/domain/user.rs' exists and will be overwritten",
    "Type change: id field changed from String to Uuid (breaking change)"
  ]
}
```

### Step 3: Decision Point

**Option A**: Accept changes (apply generation)
```json
{
  "tool": "generate_from_schema",
  "arguments": {
    "schema_type": "zod",
    "schema_content": "z.object({ id: z.string().uuid(), email: z.string().email(), username: z.string() })",
    "entity_name": "User",
    "features": ["serde", "validation"],
    "output_path": "src/domain/user.rs"
  }
}
```

**Option B**: Reject changes (modify schema)
```typescript
// Adjust schema to preserve existing structure
const UserSchema = z.object({
  id: z.string(),  // Keep as String (not UUID)
  email: z.string().email(),
  username: z.string(),
});
```

### Step 4: Incremental Preview (Large Projects)

```json
{
  "tool": "preview_generation",
  "arguments": {
    "generation_config": {
      "tool": "sync_ontology",
      "arguments": {
        "ontology_path": "ontology/",
        "config_path": "ggen.toml"
      }
    },
    "show_diffs": false,
    "include_full_code": false
  }
}
```

**Response** (summary only):
```json
{
  "preview_mode": true,
  "files_to_generate": [
    {"path": "src/generated/tools.rs", "action": "update", "size_bytes": 8742},
    {"path": "src/generated/models.rs", "action": "create", "size_bytes": 3421},
    {"path": "src/generated/validators.rs", "action": "create", "size_bytes": 1247}
  ],
  "statistics": {
    "files_to_create": 2,
    "files_to_update": 1,
    "files_to_skip": 0,
    "total_bytes": 13410
  },
  "warnings": []
}
```

---

## Example 5: Error Recovery Patterns

**Goal**: Handle validation failures, compilation errors, and partial sync failures gracefully.

### Pattern 1: SHACL Validation Failure

**Scenario**: Ontology violates SHACL constraints.

#### Step 1: Attempt Sync

```json
{
  "tool": "sync_ontology",
  "arguments": {
    "ontology_path": "ontology/mcp-domain.ttl",
    "validation_level": "strict"
  }
}
```

#### Step 2: Receive Validation Error

**Response**:
```json
{
  "sync_id": "sync-20260120-103456-b8c0d2e3",
  "timestamp": "2026-01-20T10:34:56Z",
  "status": "failed",
  "pipeline_stages": [
    {
      "stage": "1. Load Ontology",
      "status": "completed",
      "duration_ms": 234,
      "details": "Loaded 347 triples"
    },
    {
      "stage": "2. Validate SHACL",
      "status": "failed",
      "duration_ms": 89,
      "details": "2 SHACL constraint violations detected"
    },
    {
      "stage": "3. Resolve Dependencies",
      "status": "skipped",
      "duration_ms": 0,
      "details": "Skipped due to validation failure"
    }
  ],
  "errors": [
    {
      "stage": "2. Validate SHACL",
      "severity": "error",
      "message": "Property mcp:hasParameter violates sh:minCount constraint (expected >= 1, found 0) for subject: mcp:ValidateOntology"
    },
    {
      "stage": "2. Validate SHACL",
      "severity": "error",
      "message": "Class mcp:Tool missing required rdfs:label annotation for subject: mcp:UnnamedTool"
    }
  ]
}
```

#### Step 3: Fix Ontology

```turtle
# ontology/mcp-domain.ttl (fixed)

# Fix 1: Add missing parameter
mcp:ValidateOntology a mcp:Tool ;
    rdfs:label "validate_ontology" ;
    rdfs:comment "Validate RDF/Turtle ontology files" ;
    mcp:hasParameter mcp:OntologyPathParam .  # ✓ Added

# Fix 2: Add missing label
mcp:UnnamedTool a mcp:Tool ;
    rdfs:label "unnamed_tool" .  # ✓ Added
```

#### Step 4: Retry Sync

```json
{
  "tool": "sync_ontology",
  "arguments": {
    "ontology_path": "ontology/mcp-domain.ttl",
    "validation_level": "strict"
  }
}
```

**Response**: ✓ Success

---

### Pattern 2: Compilation Failure

**Scenario**: Generated code doesn't compile.

#### Step 1: Sync Completes but Compilation Fails

**Response**:
```json
{
  "sync_id": "sync-20260120-104512-c9d1e2f3",
  "timestamp": "2026-01-20T10:45:12Z",
  "status": "partial",
  "pipeline_stages": [
    // ... stages 1-8 completed ...
    {
      "stage": "9. Check Compilation",
      "status": "failed",
      "duration_ms": 3421,
      "details": "cargo check failed with 1 error"
    }
  ],
  "errors": [
    {
      "stage": "9. Check Compilation",
      "severity": "error",
      "message": "error[E0433]: failed to resolve: use of undeclared crate or module `uuid`\n  --> src/generated/tools.rs:5:5\n   |\n 5 | use uuid::Uuid;\n   |     ^^^^ use of undeclared crate or module `uuid`"
    }
  ]
}
```

#### Step 2: Fix Template (Add Missing Import)

```rust
// templates/tool_handlers.rs.tera (fixed)
use uuid::Uuid;  // ✓ Added
use serde::{Deserialize, Serialize};
// ... rest of template ...
```

#### Step 3: Retry Generation

```json
{
  "tool": "sync_ontology",
  "arguments": {
    "ontology_path": "ontology/mcp-domain.ttl",
    "force": true  // Force regeneration
  }
}
```

**Response**: ✓ Success

---

### Pattern 3: Test Failure Recovery

**Scenario**: Generated code compiles but tests fail.

#### Step 1: Test Failure Detected

**Response**:
```json
{
  "pipeline_stages": [
    // ... stages 1-10 completed ...
    {
      "stage": "11. Run Tests",
      "status": "failed",
      "duration_ms": 2134,
      "details": "cargo test failed (2 failures, 345 passed)"
    }
  ],
  "errors": [
    {
      "stage": "11. Run Tests",
      "severity": "error",
      "message": "test validate_user_email ... FAILED\n\nExpected: Ok(())\nActual: Err(ValidationError::InvalidEmail)"
    }
  ]
}
```

#### Step 2: Fix Validation Logic in Template

```rust
// templates/validators.rs.tera (fixed)
pub fn validate_email(email: &str) -> Result<(), ValidationError> {
    // ✓ Fixed regex pattern
    let email_regex = regex::Regex::new(r"^[^@]+@[^@]+\.[^@]+$").unwrap();

    if email_regex.is_match(email) {
        Ok(())
    } else {
        Err(ValidationError::InvalidEmail)
    }
}
```

#### Step 3: Regenerate and Verify

```bash
# Regenerate
sync_ontology { force: true }

# Run tests manually to verify
cargo test
```

---

### Pattern 4: Rollback on Failure

**Scenario**: Need to rollback to previous working state.

#### Step 1: Backup Before Sync

```bash
# Create git commit before sync
git add .
git commit -m "Pre-sync checkpoint"
```

#### Step 2: Sync Fails

```json
{
  "status": "failed",
  "errors": [/* ... */]
}
```

#### Step 3: Rollback

```bash
# Revert to previous commit
git reset --hard HEAD~1

# Or use git stash if uncommitted
git stash
```

#### Step 4: Retry with Fixed Configuration

```bash
# Fix ontology/templates
# Retry sync
sync_ontology { ... }
```

---

### Pattern 5: Graceful Degradation

**Scenario**: Partial sync success (some files generated, some failed).

#### Step 1: Partial Success Response

**Response**:
```json
{
  "status": "partial",
  "files_generated": [
    {"path": "src/generated/models.rs", "hash": "...", "size_bytes": 1247}
  ],
  "errors": [
    {
      "stage": "6. Render Tera Templates",
      "severity": "error",
      "message": "Template rendering failed for templates/handlers.rs.tera: undefined variable 'param_type'"
    }
  ]
}
```

#### Step 2: Use Successfully Generated Files

```rust
// models.rs was generated successfully, use it
use crate::generated::models::User;

// handlers.rs failed, implement manually for now
pub async fn list_users() -> Result<Vec<User>> {
    // Manual implementation until template is fixed
    unimplemented!()
}
```

#### Step 3: Fix Failed Template

```rust
// templates/handlers.rs.tera (fixed)
{% for tool in tools %}
    {% for param in tool.parameters %}
        // ✓ Fixed: Use correct variable name
        pub {{ param.name }}: {{ param.parameter_type }},
    {% endfor %}
{% endfor %}
```

#### Step 4: Incremental Retry (Regenerate Only Failed Parts)

```json
{
  "tool": "sync_ontology",
  "arguments": {
    "config_path": "ggen.toml",
    "force": false  // Only regenerate changed/failed files
  }
}
```

---

## Best Practices Summary

| Practice | Benefit | Example |
|----------|---------|---------|
| **Always preview first** | Catch issues before writing files | Use `preview_generation` |
| **Use strict validation** | Catch errors early | `validation_level: "strict"` |
| **Enable audit trails** | Track provenance | `audit_trail: true` |
| **Git commits before sync** | Easy rollback | `git commit -m "Pre-sync"` |
| **Incremental regeneration** | Faster iteration | `force: false` |
| **Parallel generation** | 2-3x speed boost | `parallel_generation: true` |

---

**Version**: 1.0.0 | **Last Updated**: 2026-01-20
