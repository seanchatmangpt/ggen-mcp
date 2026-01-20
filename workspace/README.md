# Workspace Directory - OpenAPI Example for MCP Server

This workspace contains the ported ggen OpenAPI example for end-to-end testing with the MCP server.

## Directory Structure

```
workspace/
├── ontology/              # RDF ontology files
│   ├── blog-api.ttl      # Blog API instance data (Users, Posts, Comments, Tags)
│   └── api-schema.ttl    # API schema vocabulary (Entity, Property, Endpoint, etc.)
├── templates/openapi/     # Tera templates (frontmatter stripped for SafeRenderer)
│   ├── openapi-info.tera
│   ├── openapi-schemas.tera
│   ├── openapi-paths.tera
│   ├── openapi-combined.tera
│   ├── typescript-interfaces.tera
│   ├── typescript-api-types.tera
│   ├── zod-schemas.tera
│   ├── zod-request-schemas.tera
│   ├── type-guards.tera
│   ├── index.tera
│   ├── schemas-index.tera
│   ├── types-index.tera
│   └── guards-index.tera
└── workflows/             # Workflow JSON definitions
    └── openapi_generation.json  # 24-step workflow (load → query → render → validate)
```

## Workflow Structure

The `openapi_generation.json` workflow contains 24 steps:

1. **Load Ontology** - Load blog-api.ttl and api-schema.ttl
2-22. **Query & Render Steps** - Execute SPARQL queries and render templates for:
   - OpenAPI specification (info, schemas, paths, combined)
   - TypeScript JSDoc type definitions (entities, requests)
   - Zod validation schemas (entities, requests)
   - Runtime type guards
   - Barrel export index files
23. **Validate Outputs** - Validate all generated files

## SPARQL Queries

The workflow uses several SPARQL queries to extract data from the ontology:

- **API Info**: Extract specification metadata (title, version, description, server)
- **Entity Schemas**: Extract entities and their properties with validation constraints
- **API Paths**: Extract endpoint definitions (path, method, operationId, request/response)
- **Request Types**: Extract request type properties for create/update operations
- **Type Guards**: Extract entity properties for runtime type checking

## Templates

All templates have been adapted for MCP server use:
- Frontmatter removed (ggen-specific metadata)
- Compatible with SafeRenderer
- Use `sparql_results` array with `?`-prefixed keys
- Support default values and optional properties

## Testing

End-to-end tests are located in `tests/openapi_example_test.rs`:

- `test_load_ontology_and_schema` - Verify ontology loads correctly
- `test_workflow_json_structure` - Validate workflow JSON structure
- `test_query_api_info` - Test API info extraction
- `test_query_entity_schemas` - Test entity schema extraction
- `test_render_openapi_info` - Test OpenAPI info rendering
- `test_render_zod_schemas` - Test Zod schema generation
- `test_full_workflow_execution` - Execute complete 13-step workflow
- `test_compare_with_golden_files` - Compare outputs with golden files

## Running Tests

```bash
# Compile the test
cargo test --test openapi_example_test --no-run

# Run all openapi tests
cargo test --test openapi_example_test

# Run specific test
cargo test --test openapi_example_test test_load_ontology_and_schema
```

## Generating Golden Files

Golden files can be generated using the original ggen tool:

```bash
cd ggen/examples/openapi
ggen sync
cp -r lib/* ../../tests/golden/openapi/lib/
```

## Expected Outputs

The workflow generates 13 files:

1. `openapi/api-info.yaml` - OpenAPI info section
2. `openapi/schemas.yaml` - Component schemas
3. `openapi/paths.yaml` - API endpoints
4. `openapi/openapi.yaml` - Combined OpenAPI spec
5. `types/entities.mjs` - JSDoc type definitions
6. `types/requests.mjs` - Request type definitions
7. `schemas/entities.mjs` - Zod entity schemas
8. `schemas/requests.mjs` - Zod request schemas
9. `guards/entities.mjs` - Runtime type guards
10. `index.mjs` - Main barrel export
11. `schemas/index.mjs` - Schemas barrel export
12. `types/index.mjs` - Types barrel export
13. `guards/index.mjs` - Guards barrel export

## Integration with MCP Server

This workspace demonstrates how the MCP server can replicate ggen's code generation workflow:

1. **Load Ontology** - Use `OntologyEngine::load_ontology_file()`
2. **Execute Queries** - Use `engine.execute_sparql_query()`
3. **Render Templates** - Use `SafeRenderer::render_safe()`
4. **Validate Output** - Use output validators and golden files

## 80/20 Principles Applied

- **Essential Files Only** - 2 ontology files, 13 templates, 1 workflow JSON
- **Reusable Patterns** - SPARQL queries are declarative and reusable
- **Deterministic** - Same ontology + templates = same output
- **Validated** - Quality gates at every step (SPARQL, rendering, output)
- **Documented** - Self-describing workflow JSON with step descriptions

## References

- Original Example: `ggen/examples/openapi/`
- Strategy Doc: `MCP_SERVER_OPENAPI_REPLICATION_STRATEGY.md`
- MCP Server Docs: `docs/` directory
