//! ggen Project Initialization Tool
//!
//! MCP tool for bootstrapping new ggen projects with templates.
//!
//! Supports 3 templates:
//! - rust-mcp-server: MCP server with tools
//! - api-server: REST API with OpenAPI
//! - domain-model: DDD aggregates/entities/value objects

use crate::audit::integration::audit_tool;
use crate::state::AppState;
use anyhow::{Context, Result, bail};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// ============================================================================
// Constants
// ============================================================================

const MAX_PROJECT_NAME_LEN: usize = 64;
const MAX_TARGET_DIR_LEN: usize = 1024;
const MAX_ENTITY_NAME_LEN: usize = 128;
const MAX_ENTITIES_COUNT: usize = 20;

const VALID_TEMPLATES: &[&str] = &["rust-mcp-server", "api-server", "domain-model"];

// ============================================================================
// Embedded Templates
// ============================================================================

// --- Starter Ontologies ---

const ONTOLOGY_RUST_MCP_SERVER: &str = r#"@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix mcp: <http://example.org/mcp#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

# MCP Tool Definition
mcp:Tool a rdfs:Class ;
    rdfs:label "MCP Tool" ;
    rdfs:comment "Represents an MCP tool with name, description, and parameters" .

mcp:Parameter a rdfs:Class ;
    rdfs:label "Parameter" ;
    rdfs:comment "A tool parameter with name, type, and optional flag" .

mcp:Response a rdfs:Class ;
    rdfs:label "Response" ;
    rdfs:comment "Tool response structure" .

# Properties
mcp:toolName a rdf:Property ;
    rdfs:domain mcp:Tool ;
    rdfs:range xsd:string .

mcp:toolDescription a rdf:Property ;
    rdfs:domain mcp:Tool ;
    rdfs:range xsd:string .

mcp:hasParameter a rdf:Property ;
    rdfs:domain mcp:Tool ;
    rdfs:range mcp:Parameter .

mcp:parameterName a rdf:Property ;
    rdfs:domain mcp:Parameter ;
    rdfs:range xsd:string .

mcp:parameterType a rdf:Property ;
    rdfs:domain mcp:Parameter ;
    rdfs:range xsd:string .

mcp:isRequired a rdf:Property ;
    rdfs:domain mcp:Parameter ;
    rdfs:range xsd:boolean .
"#;

const ONTOLOGY_API_SERVER: &str = r#"@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix api: <http://example.org/api#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

# API Endpoint Definition
api:Endpoint a rdfs:Class ;
    rdfs:label "API Endpoint" ;
    rdfs:comment "Represents a REST API endpoint" .

api:Request a rdfs:Class ;
    rdfs:label "Request" ;
    rdfs:comment "HTTP request structure" .

api:Response a rdfs:Class ;
    rdfs:label "Response" ;
    rdfs:comment "HTTP response structure" .

api:Schema a rdfs:Class ;
    rdfs:label "Schema" ;
    rdfs:comment "JSON schema for request/response" .

# Properties
api:path a rdf:Property ;
    rdfs:domain api:Endpoint ;
    rdfs:range xsd:string .

api:method a rdf:Property ;
    rdfs:domain api:Endpoint ;
    rdfs:range xsd:string .

api:hasRequest a rdf:Property ;
    rdfs:domain api:Endpoint ;
    rdfs:range api:Request .

api:hasResponse a rdf:Property ;
    rdfs:domain api:Endpoint ;
    rdfs:range api:Response .

api:statusCode a rdf:Property ;
    rdfs:domain api:Response ;
    rdfs:range xsd:integer .
"#;

const ONTOLOGY_DOMAIN_MODEL: &str = r#"@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ddd: <http://example.org/ddd#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

# DDD Building Blocks
ddd:Entity a rdfs:Class ;
    rdfs:label "Entity" ;
    rdfs:comment "DDD Entity with unique identity" .

ddd:ValueObject a rdfs:Class ;
    rdfs:label "Value Object" ;
    rdfs:comment "Immutable value object defined by its attributes" .

ddd:Aggregate a rdfs:Class ;
    rdfs:label "Aggregate" ;
    rdfs:comment "Consistency boundary for entities" .

ddd:Event a rdfs:Class ;
    rdfs:label "Domain Event" ;
    rdfs:comment "Something that happened in the domain" .

# Properties
ddd:entityName a rdf:Property ;
    rdfs:domain ddd:Entity ;
    rdfs:range xsd:string .

ddd:hasAttribute a rdf:Property ;
    rdfs:domain ddd:Entity ;
    rdfs:range xsd:string .

ddd:aggregateRoot a rdf:Property ;
    rdfs:domain ddd:Aggregate ;
    rdfs:range ddd:Entity .

ddd:eventType a rdf:Property ;
    rdfs:domain ddd:Event ;
    rdfs:range xsd:string .
"#;

// --- SPARQL Queries ---

const QUERY_ENTITIES: &str = r#"PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>

SELECT ?entity ?name ?comment
WHERE {
    ?entity a rdfs:Class ;
            rdfs:label ?name ;
            rdfs:comment ?comment .
}
ORDER BY ?name
"#;

const QUERY_PROPERTIES: &str = r#"PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

SELECT ?property ?domain ?range
WHERE {
    ?property a rdf:Property ;
              rdfs:domain ?domain ;
              rdfs:range ?range .
}
ORDER BY ?property
"#;

// --- Tera Templates ---

const TEMPLATE_ENTITY: &str = r#"//! Generated entity from ontology
//!
//! DO NOT EDIT. Run: cargo make sync

use serde::{Deserialize, Serialize};

{% for entity in entities %}
/// {{ entity.comment }}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {{ entity.name }} {
    // TODO: Add fields from ontology properties
}

impl {{ entity.name }} {
    pub fn new() -> Self {
        Self {
            // TODO: Initialize fields
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        // TODO: Add validation logic
        Ok(())
    }
}

impl Default for {{ entity.name }} {
    fn default() -> Self {
        Self::new()
    }
}
{% endfor %}

#[cfg(test)]
mod tests {
    use super::*;

    {% for entity in entities %}
    #[test]
    fn {{ entity.name | lower }}_validates() {
        let entity = {{ entity.name }}::new();
        assert!(entity.validate().is_ok());
    }
    {% endfor %}
}
"#;

const TEMPLATE_API: &str = r#"//! Generated API endpoints from ontology
//!
//! DO NOT EDIT. Run: cargo make sync

use axum::{Router, Json};
use serde::{Deserialize, Serialize};

{% for endpoint in endpoints %}
/// {{ endpoint.description }}
#[derive(Debug, Serialize, Deserialize)]
pub struct {{ endpoint.name }}Request {
    // TODO: Add request fields from ontology
}

#[derive(Debug, Serialize, Deserialize)]
pub struct {{ endpoint.name }}Response {
    // TODO: Add response fields from ontology
}

pub async fn {{ endpoint.name | lower }}_handler(
    Json(req): Json<{{ endpoint.name }}Request>,
) -> Json<{{ endpoint.name }}Response> {
    // TODO: Implement handler logic
    Json({{ endpoint.name }}Response {})
}
{% endfor %}

pub fn routes() -> Router {
    Router::new()
    {% for endpoint in endpoints %}
        .route("/{{ endpoint.path }}", axum::routing::post({{ endpoint.name | lower }}_handler))
    {% endfor %}
}
"#;

// --- Configuration Files ---

const GGEN_TOML_TEMPLATE: &str = r#"# ggen Configuration
# Generated by: init_ggen_project

[project]
name = "{{ project_name }}"
version = "0.1.0"

[ontology]
base_path = "ontology"
files = ["domain.ttl"]

[generation]
output_dir = "src/generated"

[[rules]]
name = "entities"
query = "queries/entities.rq"
template = "templates/entity.rs.tera"
output = "src/generated/entities.rs"

[[rules]]
name = "properties"
query = "queries/properties.rq"
template = "templates/entity.rs.tera"
output = "src/generated/properties.rs"
"#;

const README_TEMPLATE: &str = r#"# {{ project_name }}

Generated ggen project using template: `{{ template }}`

## Structure

```
{{ project_name }}/
├── ggen.toml          # Generation configuration
├── ontology/          # RDF/Turtle ontology files
│   └── domain.ttl     # Domain definitions
├── queries/           # SPARQL queries
│   ├── entities.rq    # Extract entities
│   └── properties.rq  # Extract properties
├── templates/         # Tera templates
│   ├── entity.rs.tera # Rust entity generator
│   └── api.rs.tera    # API generator
└── src/
    └── generated/     # Generated code (DO NOT EDIT)
```

## Workflow

1. **Edit ontology**: Update `ontology/domain.ttl`
2. **Sync**: Run `ggen sync` to regenerate code
3. **Verify**: Check `src/generated/` for updated code
4. **Test**: Run tests to ensure correctness

## Commands

```bash
# Generate code from ontology
ggen sync

# Preview changes without writing
ggen sync --dry-run

# Force regeneration
ggen sync --force
```

## Adding Entities

Edit `ontology/domain.ttl` and add entity definitions:

```turtle
@prefix ddd: <http://example.org/ddd#> .

ddd:MyEntity a rdfs:Class ;
    rdfs:label "MyEntity" ;
    rdfs:comment "Description of MyEntity" .

ddd:myProperty a rdf:Property ;
    rdfs:domain ddd:MyEntity ;
    rdfs:range xsd:string .
```

Then run `ggen sync` to generate Rust code.

## Safety

- **Never edit** `src/generated/` directly
- Source of truth: `ontology/domain.ttl`
- All changes flow: Ontology → SPARQL → Tera → Generated Code
- Run tests after each sync

## References

- [ggen Documentation](https://github.com/seanchatmangpt/ggen)
- [RDF/Turtle Syntax](https://www.w3.org/TR/turtle/)
- [SPARQL Query Language](https://www.w3.org/TR/sparql11-query/)
- [Tera Template Engine](https://tera.netlify.app/)
"#;

const CARGO_TOML_TEMPLATE: &str = r#"[package]
name = "{{ project_name }}"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
"#;

// ============================================================================
// MCP Tool: init_ggen_project
// ============================================================================

/// Parameters for init_ggen_project tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct InitGgenProjectParams {
    /// Target directory for new project (must be empty or non-existent)
    pub target_dir: String,

    /// Project template: rust-mcp-server, api-server, domain-model
    pub template: String,

    /// Project name (alphanumeric, hyphens, underscores)
    pub project_name: String,

    /// Optional: starter ontology entities to create
    #[serde(default)]
    pub entities: Vec<String>,
}

/// Response from init_ggen_project tool
#[derive(Debug, Serialize, JsonSchema)]
pub struct InitGgenProjectResponse {
    /// Project root path
    pub project_path: String,

    /// Template used
    pub template: String,

    /// Files created (relative paths)
    pub files_created: Vec<String>,

    /// Total files created
    pub file_count: usize,

    /// Project instructions
    pub next_steps: Vec<String>,
}

/// Initialize a new ggen project with scaffolding
pub async fn init_ggen_project(
    _state: Arc<AppState>,
    params: InitGgenProjectParams,
) -> Result<InitGgenProjectResponse> {
    let _span = audit_tool("init_ggen_project", &params);

    // Validate parameters
    validate_init_params(&params)?;

    // Resolve target directory
    let target_path = resolve_target_path(&params.target_dir)?;

    // Validate target directory (must be empty or non-existent)
    validate_target_directory(&target_path)?;

    // Create project structure
    let files = create_project_structure(&target_path, &params)?;

    // Generate response with next steps
    let next_steps = generate_next_steps(&params.template);

    Ok(InitGgenProjectResponse {
        project_path: target_path.display().to_string(),
        template: params.template.clone(),
        files_created: files.clone(),
        file_count: files.len(),
        next_steps,
    })
}

// ============================================================================
// Validation Functions (Poka-Yoke)
// ============================================================================

fn validate_init_params(params: &InitGgenProjectParams) -> Result<()> {
    // Validate project name
    if params.project_name.is_empty() {
        bail!("Project name cannot be empty");
    }
    if params.project_name.len() > MAX_PROJECT_NAME_LEN {
        bail!(
            "Project name exceeds {} characters",
            MAX_PROJECT_NAME_LEN
        );
    }
    if !params
        .project_name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        bail!("Project name must be alphanumeric (hyphens and underscores allowed)");
    }

    // Validate target directory
    if params.target_dir.is_empty() {
        bail!("Target directory cannot be empty");
    }
    if params.target_dir.len() > MAX_TARGET_DIR_LEN {
        bail!("Target directory path exceeds {} characters", MAX_TARGET_DIR_LEN);
    }
    if params.target_dir.contains("../") || params.target_dir.contains("..\\") {
        bail!("Path traversal not allowed in target directory");
    }

    // Validate template
    if !VALID_TEMPLATES.contains(&params.template.as_str()) {
        bail!(
            "Invalid template '{}'. Valid templates: {}",
            params.template,
            VALID_TEMPLATES.join(", ")
        );
    }

    // Validate entities
    if params.entities.len() > MAX_ENTITIES_COUNT {
        bail!("Too many entities (max: {})", MAX_ENTITIES_COUNT);
    }
    for entity in &params.entities {
        if entity.is_empty() || entity.len() > MAX_ENTITY_NAME_LEN {
            bail!(
                "Entity name must be 1-{} characters",
                MAX_ENTITY_NAME_LEN
            );
        }
        if !entity
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_')
        {
            bail!("Entity names must be alphanumeric (underscores allowed)");
        }
    }

    Ok(())
}

fn resolve_target_path(target_dir: &str) -> Result<PathBuf> {
    let path = PathBuf::from(target_dir);

    // Convert to absolute path if relative
    let absolute = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .context("Failed to get current directory")?
            .join(path)
    };

    Ok(absolute)
}

fn validate_target_directory(path: &Path) -> Result<()> {
    if path.exists() {
        // Directory exists - must be empty
        if !path.is_dir() {
            bail!("Target path exists but is not a directory: {}", path.display());
        }

        let entries = fs::read_dir(path)
            .with_context(|| format!("Failed to read directory: {}", path.display()))?;

        if entries.count() > 0 {
            bail!("Target directory is not empty: {}", path.display());
        }
    }

    Ok(())
}

// ============================================================================
// Project Structure Creation
// ============================================================================

fn create_project_structure(
    target_path: &Path,
    params: &InitGgenProjectParams,
) -> Result<Vec<String>> {
    let mut files_created = Vec::new();

    // Create directory structure
    create_directories(target_path)?;

    // Select ontology based on template
    let ontology_content = select_ontology(&params.template);

    // Write ontology file
    write_file(
        target_path,
        "ontology/domain.ttl",
        ontology_content,
        &mut files_created,
    )?;

    // Write query files
    write_file(
        target_path,
        "queries/entities.rq",
        QUERY_ENTITIES,
        &mut files_created,
    )?;
    write_file(
        target_path,
        "queries/properties.rq",
        QUERY_PROPERTIES,
        &mut files_created,
    )?;

    // Write template files
    write_file(
        target_path,
        "templates/entity.rs.tera",
        TEMPLATE_ENTITY,
        &mut files_created,
    )?;
    if params.template == "api-server" {
        write_file(
            target_path,
            "templates/api.rs.tera",
            TEMPLATE_API,
            &mut files_created,
        )?;
    }

    // Write configuration files
    let ggen_toml = render_template(GGEN_TOML_TEMPLATE, params)?;
    write_file(
        target_path,
        "ggen.toml",
        &ggen_toml,
        &mut files_created,
    )?;

    let readme = render_template(README_TEMPLATE, params)?;
    write_file(
        target_path,
        "README.md",
        &readme,
        &mut files_created,
    )?;

    let cargo_toml = render_template(CARGO_TOML_TEMPLATE, params)?;
    write_file(
        target_path,
        "Cargo.toml",
        &cargo_toml,
        &mut files_created,
    )?;

    // Create empty generated directory
    fs::create_dir_all(target_path.join("src/generated"))
        .context("Failed to create src/generated directory")?;
    files_created.push("src/generated/".to_string());

    // Create .gitignore
    write_file(
        target_path,
        ".gitignore",
        "/target\n/src/generated/*\n",
        &mut files_created,
    )?;

    Ok(files_created)
}

fn create_directories(base: &Path) -> Result<()> {
    let dirs = vec![
        "ontology",
        "queries",
        "templates",
        "src/generated",
    ];

    for dir in dirs {
        let path = base.join(dir);
        fs::create_dir_all(&path)
            .with_context(|| format!("Failed to create directory: {}", path.display()))?;
    }

    Ok(())
}

fn select_ontology(template: &str) -> &'static str {
    match template {
        "rust-mcp-server" => ONTOLOGY_RUST_MCP_SERVER,
        "api-server" => ONTOLOGY_API_SERVER,
        "domain-model" => ONTOLOGY_DOMAIN_MODEL,
        _ => ONTOLOGY_DOMAIN_MODEL, // Default
    }
}

fn write_file(
    base: &Path,
    relative_path: &str,
    content: &str,
    files_created: &mut Vec<String>,
) -> Result<()> {
    let full_path = base.join(relative_path);

    // Ensure parent directory exists
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent directory: {}", parent.display()))?;
    }

    fs::write(&full_path, content)
        .with_context(|| format!("Failed to write file: {}", full_path.display()))?;

    files_created.push(relative_path.to_string());
    Ok(())
}

fn render_template(template: &str, params: &InitGgenProjectParams) -> Result<String> {
    // Simple string replacement for basic templating
    let mut result = template.to_string();
    result = result.replace("{{ project_name }}", &params.project_name);
    result = result.replace("{{ template }}", &params.template);
    Ok(result)
}

fn generate_next_steps(template: &str) -> Vec<String> {
    let mut steps = vec![
        "cd into the project directory".to_string(),
        "Review ontology/domain.ttl".to_string(),
        "Run 'ggen sync' to generate initial code".to_string(),
        "Review src/generated/ for generated code".to_string(),
    ];

    match template {
        "rust-mcp-server" => {
            steps.push("Implement MCP tool handlers in src/generated/".to_string());
            steps.push("Run 'cargo test' to verify implementation".to_string());
        }
        "api-server" => {
            steps.push("Implement API endpoint handlers".to_string());
            steps.push("Run 'cargo run' to start server".to_string());
        }
        "domain-model" => {
            steps.push("Define domain entities in ontology".to_string());
            steps.push("Add validation logic in generated code".to_string());
        }
        _ => {}
    }

    steps.push("Read README.md for detailed instructions".to_string());
    steps
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_project_name_valid() {
        let params = InitGgenProjectParams {
            target_dir: "/tmp/test-project".to_string(),
            template: "domain-model".to_string(),
            project_name: "my-project-123".to_string(),
            entities: vec![],
        };
        assert!(validate_init_params(&params).is_ok());
    }

    #[test]
    fn test_validate_project_name_empty() {
        let params = InitGgenProjectParams {
            target_dir: "/tmp/test".to_string(),
            template: "domain-model".to_string(),
            project_name: "".to_string(),
            entities: vec![],
        };
        assert!(validate_init_params(&params).is_err());
    }

    #[test]
    fn test_validate_project_name_invalid_chars() {
        let params = InitGgenProjectParams {
            target_dir: "/tmp/test".to_string(),
            template: "domain-model".to_string(),
            project_name: "my project!".to_string(),
            entities: vec![],
        };
        assert!(validate_init_params(&params).is_err());
    }

    #[test]
    fn test_validate_template_invalid() {
        let params = InitGgenProjectParams {
            target_dir: "/tmp/test".to_string(),
            template: "invalid-template".to_string(),
            project_name: "test".to_string(),
            entities: vec![],
        };
        assert!(validate_init_params(&params).is_err());
    }

    #[test]
    fn test_validate_template_valid() {
        for template in VALID_TEMPLATES {
            let params = InitGgenProjectParams {
                target_dir: "/tmp/test".to_string(),
                template: template.to_string(),
                project_name: "test".to_string(),
                entities: vec![],
            };
            assert!(validate_init_params(&params).is_ok());
        }
    }

    #[test]
    fn test_validate_path_traversal() {
        let params = InitGgenProjectParams {
            target_dir: "/tmp/../etc/passwd".to_string(),
            template: "domain-model".to_string(),
            project_name: "test".to_string(),
            entities: vec![],
        };
        assert!(validate_init_params(&params).is_err());
    }

    #[test]
    fn test_validate_entities_too_many() {
        let params = InitGgenProjectParams {
            target_dir: "/tmp/test".to_string(),
            template: "domain-model".to_string(),
            project_name: "test".to_string(),
            entities: (0..30).map(|i| format!("Entity{}", i)).collect(),
        };
        assert!(validate_init_params(&params).is_err());
    }

    #[test]
    fn test_select_ontology() {
        assert_eq!(
            select_ontology("rust-mcp-server"),
            ONTOLOGY_RUST_MCP_SERVER
        );
        assert_eq!(select_ontology("api-server"), ONTOLOGY_API_SERVER);
        assert_eq!(select_ontology("domain-model"), ONTOLOGY_DOMAIN_MODEL);
        assert_eq!(select_ontology("unknown"), ONTOLOGY_DOMAIN_MODEL); // Default
    }

    #[test]
    fn test_render_template_basic() {
        let params = InitGgenProjectParams {
            target_dir: "/tmp/test".to_string(),
            template: "domain-model".to_string(),
            project_name: "my-project".to_string(),
            entities: vec![],
        };

        let template = "Project: {{ project_name }}, Template: {{ template }}";
        let result = render_template(template, &params).unwrap();
        assert_eq!(result, "Project: my-project, Template: domain-model");
    }

    #[test]
    fn test_generate_next_steps() {
        let steps = generate_next_steps("rust-mcp-server");
        assert!(!steps.is_empty());
        assert!(steps.iter().any(|s| s.contains("MCP")));

        let steps = generate_next_steps("api-server");
        assert!(steps.iter().any(|s| s.contains("API")));

        let steps = generate_next_steps("domain-model");
        assert!(steps.iter().any(|s| s.contains("domain")));
    }
}
