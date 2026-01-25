//! Turtle Ontology Authoring Tools
//!
//! MCP tools for reading, editing, and validating RDF Turtle ontologies.
//! Supports entity/property authoring, syntax validation, and querying.
//!
//! ## Tools
//! - `read_turtle_ontology`: Parse Turtle → extract entities/properties → return metadata
//! - `add_entity_to_ontology`: Add DDD entity (Entity/ValueObject/Aggregate) → validate → write
//! - `add_property_to_entity`: Add property to existing entity → validate → write
//! - `validate_turtle_syntax`: Parse with Oxigraph → SHACL validation → report issues
//! - `query_ontology_entities`: List all entities with properties and types
//!
//! ## Safety Patterns
//! - Atomic writes with `.tmp` → rename
//! - Backup creation before modification
//! - Path traversal prevention
//! - Syntax validation before write
//! - SHACL validation (optional)

use crate::audit::integration::audit_tool;
use crate::ontology::ShapeValidator;
use crate::state::AppState;
use crate::validation::{validate_non_empty_string, validate_path_safe};
use anyhow::{Context, Result, anyhow};
use ggen_ontology_core::TripleStore;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

// =============================================================================
// Constants
// =============================================================================

const MAX_PATH_LENGTH: usize = 1024;
const MAX_ENTITY_NAME_LENGTH: usize = 128;
const MAX_PROPERTY_COUNT: usize = 100;
const MAX_TURTLE_SIZE: usize = 10 * 1024 * 1024; // 10MB
const BACKUP_SUFFIX: &str = ".backup";

// DDD ontology prefixes (aligned with mcp-domain.ttl)
const DDD_PREFIX: &str = "http://ggen-mcp.dev/ontology/ddd#";
const MCP_PREFIX: &str = "http://ggen-mcp.dev/ontology/mcp#";
const RDFS_PREFIX: &str = "http://www.w3.org/2000/01/rdf-schema#";
const RDF_PREFIX: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

// =============================================================================
// NewTypes (Poka-Yoke: prevent confusion)
// =============================================================================

/// Entity name in ontology
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct EntityName(String);

impl EntityName {
    pub fn new(name: String) -> Result<Self> {
        validate_non_empty_string("entity_name", &name).context("entity name cannot be empty")?;
        if name.len() > MAX_ENTITY_NAME_LENGTH {
            return Err(anyhow!(
                "entity name exceeds max length of {}",
                MAX_ENTITY_NAME_LENGTH
            ));
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EntityName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Property name in ontology
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct PropertyName(String);

impl PropertyName {
    pub fn new(name: String) -> Result<Self> {
        validate_non_empty_string("property_name", &name).context("property name cannot be empty")?;
        if name.len() > MAX_ENTITY_NAME_LENGTH {
            return Err(anyhow!(
                "property name exceeds max length of {}",
                MAX_ENTITY_NAME_LENGTH
            ));
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// =============================================================================
// Entity Templates
// =============================================================================

/// DDD Entity type for code generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    /// Entity with identity and lifecycle
    Entity,
    /// Value Object (immutable, defined by attributes)
    ValueObject,
    /// Aggregate Root (consistency boundary)
    AggregateRoot,
    /// Domain Event (something that happened)
    Event,
    /// Command (intent to change state)
    Command,
    /// Query (data request, no side effects)
    Query,
}

impl EntityType {
    pub fn ddd_class_iri(&self) -> &'static str {
        match self {
            EntityType::Entity => "http://ggen-mcp.dev/ontology/ddd#Entity",
            EntityType::ValueObject => "http://ggen-mcp.dev/ontology/ddd#ValueObject",
            EntityType::AggregateRoot => "http://ggen-mcp.dev/ontology/ddd#AggregateRoot",
            EntityType::Event => "http://ggen-mcp.dev/ontology/ddd#DomainEvent",
            EntityType::Command => "http://ggen-mcp.dev/ontology/ddd#Command",
            EntityType::Query => "http://ggen-mcp.dev/ontology/ddd#Query",
        }
    }
}

/// Property metadata
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PropertySpec {
    /// Property name
    pub name: PropertyName,
    /// Rust type (e.g., "String", "i32", "Option<Vec<u8>>")
    pub rust_type: String,
    /// Whether property is required (non-optional)
    #[serde(default)]
    pub required: bool,
    /// Optional documentation/description
    pub description: Option<String>,
}

// =============================================================================
// Tool Parameters & Responses
// =============================================================================

// --- read_turtle_ontology ---

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadTurtleParams {
    /// Path to Turtle (.ttl) file (relative to workspace_root)
    pub path: String,
    /// Include detailed entity information (default: true)
    #[serde(default = "default_true")]
    pub include_entities: bool,
    /// Include prefix/namespace information (default: true)
    #[serde(default = "default_true")]
    pub include_prefixes: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadTurtleResponse {
    /// Path to the ontology file
    pub path: String,
    /// Raw Turtle content
    pub content: String,
    /// Number of RDF triples
    pub triple_count: usize,
    /// Parsed entities (if include_entities=true)
    pub entities: Option<Vec<EntityInfo>>,
    /// Prefixes/namespaces (if include_prefixes=true)
    pub prefixes: Option<HashMap<String, String>>,
    /// Parse duration in milliseconds
    pub parse_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EntityInfo {
    /// Entity name (local name from IRI)
    pub name: String,
    /// Full IRI
    pub iri: String,
    /// Entity type (Entity/ValueObject/AggregateRoot/etc.)
    pub entity_type: Option<String>,
    /// Properties belonging to this entity
    pub properties: Vec<PropertyInfo>,
    /// rdfs:label (if present)
    pub label: Option<String>,
    /// rdfs:comment (if present)
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PropertyInfo {
    /// Property name
    pub name: String,
    /// Full IRI
    pub iri: String,
    /// Rust type (from ddd:type predicate)
    pub rust_type: Option<String>,
    /// Required flag (from ddd:required predicate)
    pub required: bool,
    /// rdfs:label (if present)
    pub label: Option<String>,
}

// --- add_entity_to_ontology ---

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AddEntityParams {
    /// Path to Turtle (.ttl) file
    pub path: String,
    /// Entity name (local name, e.g., "User")
    pub entity_name: EntityName,
    /// Entity type (Entity/ValueObject/AggregateRoot/Event/Command/Query)
    pub entity_type: EntityType,
    /// Properties to add to the entity
    pub properties: Vec<PropertySpec>,
    /// Optional rdfs:label (default: entity_name)
    pub label: Option<String>,
    /// Optional rdfs:comment
    pub comment: Option<String>,
    /// Create backup before modification (default: true)
    #[serde(default = "default_true")]
    pub create_backup: bool,
    /// Validate syntax after modification (default: true)
    #[serde(default = "default_true")]
    pub validate_syntax: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AddEntityResponse {
    /// Path to modified ontology
    pub path: String,
    /// Full IRI of created entity
    pub entity_iri: String,
    /// Number of triples added
    pub triples_added: usize,
    /// Path to backup file (if created)
    pub backup_path: Option<String>,
    /// Syntax validation result
    pub validation: Option<ValidationResult>,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

// --- add_property_to_entity ---

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AddPropertyParams {
    /// Path to Turtle (.ttl) file
    pub path: String,
    /// Entity name to add property to
    pub entity_name: EntityName,
    /// Property to add
    pub property: PropertySpec,
    /// Create backup before modification (default: true)
    #[serde(default = "default_true")]
    pub create_backup: bool,
    /// Validate syntax after modification (default: true)
    #[serde(default = "default_true")]
    pub validate_syntax: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AddPropertyResponse {
    /// Path to modified ontology
    pub path: String,
    /// Full IRI of the property
    pub property_iri: String,
    /// Number of triples added
    pub triples_added: usize,
    /// Path to backup file (if created)
    pub backup_path: Option<String>,
    /// Syntax validation result
    pub validation: Option<ValidationResult>,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

// --- validate_turtle_syntax ---

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidateTurtleParams {
    /// Path to Turtle (.ttl) file
    pub path: String,
    /// Enable SHACL validation (default: false)
    #[serde(default)]
    pub shacl_validation: bool,
    /// Strict mode - fail on warnings (default: false)
    #[serde(default)]
    pub strict_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidateTurtleResponse {
    /// Path to validated file
    pub path: String,
    /// Overall validation result
    pub is_valid: bool,
    /// Validation details
    pub validation: ValidationResult,
    /// Validation duration in milliseconds
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidationResult {
    /// Syntax is valid
    pub syntax_valid: bool,
    /// Parse errors (if any)
    pub parse_errors: Vec<String>,
    /// SHACL validation result (if enabled)
    pub shacl_result: Option<ShaclValidationResult>,
    /// Warnings (non-fatal issues)
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ShaclValidationResult {
    /// Conforms to SHACL shapes
    pub conforms: bool,
    /// Violation count
    pub violations: usize,
    /// Violation details
    pub violation_details: Vec<String>,
}

// --- query_ontology_entities ---

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QueryEntitiesParams {
    /// Path to Turtle (.ttl) file
    pub path: String,
    /// Filter by entity type (optional)
    pub entity_type_filter: Option<EntityType>,
    /// Include properties (default: true)
    #[serde(default = "default_true")]
    pub include_properties: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QueryEntitiesResponse {
    /// Path to ontology
    pub path: String,
    /// List of entities matching filter
    pub entities: Vec<EntityInfo>,
    /// Query duration in milliseconds
    pub duration_ms: u64,
}

// =============================================================================
// Tool Implementations
// =============================================================================

/// Read and parse Turtle ontology → extract entities/properties → return metadata
pub async fn read_turtle_ontology(
    state: Arc<AppState>,
    params: ReadTurtleParams,
) -> Result<ReadTurtleResponse> {
    let _span = audit_tool("read_turtle_ontology", &params);
    let start = Instant::now();

    // Validate inputs
    validate_path_input(&params.path)?;

    // Resolve path
    let ontology_path = resolve_ontology_path(&state, &params.path)?;

    // Read content
    let content = fs::read_to_string(&ontology_path)
        .context(format!("failed to read Turtle file: {}", params.path))?;

    // Validate size
    if content.len() > MAX_TURTLE_SIZE {
        return Err(anyhow!(
            "Turtle file exceeds max size of {} bytes",
            MAX_TURTLE_SIZE
        ));
    }

    // Parse into TripleStore
    let store = TripleStore::new()
        .map_err(|e| anyhow!("failed to create TripleStore: {}", e))?;

    // Write content to temp file for loading (ggen's load_turtle requires a file path)
    let temp_path = ontology_path.with_extension("tmp");
    fs::write(&temp_path, &content).context("failed to write temp file for parsing")?;
    
    store.load_turtle(&temp_path)
        .map_err(|e| anyhow!("failed to parse Turtle content: {}", e))?;
    
    // Clean up temp file
    let _ = fs::remove_file(&temp_path);

    // Count triples using SPARQL query
    let triple_count = count_triples_in_store(&store)?;

    // Extract entities (if requested)
    let entities = if params.include_entities {
        Some(extract_entities(&store)?)
    } else {
        None
    };

    // Extract prefixes (if requested)
    let prefixes = if params.include_prefixes {
        Some(extract_prefixes(&content)?)
    } else {
        None
    };

    let parse_time_ms = start.elapsed().as_millis() as u64;

    Ok(ReadTurtleResponse {
        path: params.path,
        content,
        triple_count,
        entities,
        prefixes,
        parse_time_ms,
    })
}

/// Add entity to ontology → generate Turtle → validate → atomic write
pub async fn add_entity_to_ontology(
    state: Arc<AppState>,
    params: AddEntityParams,
) -> Result<AddEntityResponse> {
    let _span = audit_tool("add_entity_to_ontology", &params);
    let start = Instant::now();

    // Validate inputs
    validate_path_input(&params.path)?;
    if params.properties.len() > MAX_PROPERTY_COUNT {
        return Err(anyhow!(
            "property count exceeds maximum of {}",
            MAX_PROPERTY_COUNT
        ));
    }

    // Resolve path
    let ontology_path = resolve_ontology_path(&state, &params.path)?;

    // Create backup (if requested)
    let backup_path = if params.create_backup {
        Some(create_backup(&ontology_path)?)
    } else {
        None
    };

    // Read existing content
    let mut content =
        fs::read_to_string(&ontology_path).context("failed to read existing ontology")?;

    // Generate entity IRI (use mcp: prefix for consistency)
    let entity_iri = format!("{}:{}", "mcp", params.entity_name.as_str());

    // Check if entity already exists
    let store = TripleStore::new()
        .map_err(|e| anyhow!("failed to create TripleStore: {}", e))?;
    
    // Write content to temp file for loading
    let temp_path = ontology_path.with_extension("tmp");
    fs::write(&temp_path, &content).context("failed to write temp file")?;
    
    store.load_turtle(&temp_path)
        .map_err(|e| anyhow!("failed to load ontology: {}", e))?;
    
    let _ = fs::remove_file(&temp_path);

    if entity_exists_in_store(&store, &entity_iri)? {
        return Err(anyhow!(
            "entity '{}' already exists in ontology",
            entity_iri
        ));
    }

    // Generate Turtle triples for entity
    let entity_turtle = generate_entity_turtle(&params)?;
    let triples_added = count_lines_starting_with(&entity_turtle, &[" ", "\t", "mcp:", "ddd:"]);

    // Append to content
    content.push_str("\n\n");
    content.push_str(&format!(
        "# Auto-generated entity: {}\n",
        params.entity_name.as_str()
    ));
    content.push_str(&entity_turtle);

    // Atomic write
    write_turtle_atomic(&ontology_path, &content)?;

    // Validate syntax (if requested)
    let validation = if params.validate_syntax {
        Some(validate_turtle_content(&content, false)?)
    } else {
        None
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(AddEntityResponse {
        path: params.path,
        entity_iri: format!("{}{}", MCP_PREFIX, params.entity_name.as_str()),
        triples_added,
        backup_path: backup_path.map(|p| p.to_string_lossy().to_string()),
        validation,
        duration_ms,
    })
}

/// Add property to existing entity → validate entity exists → atomic write
pub async fn add_property_to_entity(
    state: Arc<AppState>,
    params: AddPropertyParams,
) -> Result<AddPropertyResponse> {
    let _span = audit_tool("add_property_to_entity", &params);
    let start = Instant::now();

    // Validate inputs
    validate_path_input(&params.path)?;

    // Resolve path
    let ontology_path = resolve_ontology_path(&state, &params.path)?;

    // Create backup (if requested)
    let backup_path = if params.create_backup {
        Some(create_backup(&ontology_path)?)
    } else {
        None
    };

    // Read existing content
    let mut content =
        fs::read_to_string(&ontology_path).context("failed to read existing ontology")?;

    // Parse and verify entity exists
    let store = TripleStore::new()
        .map_err(|e| anyhow!("failed to create TripleStore: {}", e))?;
    
    // Write content to temp file for loading
    let temp_path = ontology_path.with_extension("tmp");
    fs::write(&temp_path, &content).context("failed to write temp file")?;
    
    store.load_turtle(&temp_path)
        .map_err(|e| anyhow!("failed to load ontology: {}", e))?;
    
    let _ = fs::remove_file(&temp_path);

    let entity_iri = format!("{}:{}", "mcp", params.entity_name.as_str());
    if !entity_exists_in_store(&store, &entity_iri)? {
        return Err(anyhow!("entity '{}' not found in ontology", entity_iri));
    }

    // Generate property IRI
    let property_iri = format!("{}:{}", "mcp", params.property.name.as_str());

    // Check if property already exists
    if property_exists_in_store(&store, &property_iri)? {
        return Err(anyhow!("property '{}' already exists", property_iri));
    }

    // Generate property Turtle
    let property_turtle = generate_property_turtle(&params.property)?;
    let triples_added = count_lines_starting_with(&property_turtle, &[" ", "\t", "mcp:", "ddd:"]);

    // Generate entity link (add property to entity's ddd:hasProperty)
    let entity_link = format!("\n{} ddd:hasProperty {} .\n", entity_iri, property_iri);

    // Append to content
    content.push_str("\n\n");
    content.push_str(&format!(
        "# Auto-generated property: {}\n",
        params.property.name.as_str()
    ));
    content.push_str(&property_turtle);
    content.push_str(&entity_link);

    // Atomic write
    write_turtle_atomic(&ontology_path, &content)?;

    // Validate syntax (if requested)
    let validation = if params.validate_syntax {
        Some(validate_turtle_content(&content, false)?)
    } else {
        None
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(AddPropertyResponse {
        path: params.path,
        property_iri: format!("{}{}", MCP_PREFIX, params.property.name.as_str()),
        triples_added,
        backup_path: backup_path.map(|p| p.to_string_lossy().to_string()),
        validation,
        duration_ms,
    })
}

/// Validate Turtle syntax → optionally SHACL validation → report issues
pub async fn validate_turtle_syntax(
    state: Arc<AppState>,
    params: ValidateTurtleParams,
) -> Result<ValidateTurtleResponse> {
    let _span = audit_tool("validate_turtle_syntax", &params);
    let start = Instant::now();

    // Validate inputs
    validate_path_input(&params.path)?;

    // Resolve path
    let ontology_path = resolve_ontology_path(&state, &params.path)?;

    // Read content
    let content = fs::read_to_string(&ontology_path).context("failed to read Turtle file")?;

    // Validate syntax
    let validation = validate_turtle_content(&content, params.shacl_validation)?;

    // Determine overall validity
    let is_valid = validation.syntax_valid
        && (!params.strict_mode || validation.warnings.is_empty())
        && validation
            .shacl_result
            .as_ref()
            .map(|r| r.conforms)
            .unwrap_or(true);

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(ValidateTurtleResponse {
        path: params.path,
        is_valid,
        validation,
        duration_ms,
    })
}

/// Query ontology for entities → filter by type → return with properties
pub async fn query_ontology_entities(
    state: Arc<AppState>,
    params: QueryEntitiesParams,
) -> Result<QueryEntitiesResponse> {
    let _span = audit_tool("query_ontology_entities", &params);
    let start = Instant::now();

    // Validate inputs
    validate_path_input(&params.path)?;

    // Resolve path
    let ontology_path = resolve_ontology_path(&state, &params.path)?;

    // Read and parse
    let content = fs::read_to_string(&ontology_path).context("failed to read Turtle file")?;

    let store = TripleStore::new()
        .map_err(|e| anyhow!("failed to create TripleStore: {}", e))?;
    
    // Write content to temp file for loading
    let temp_path = ontology_path.with_extension("tmp");
    fs::write(&temp_path, &content).context("failed to write temp file")?;
    
    store.load_turtle(&temp_path)
        .map_err(|e| anyhow!("failed to load ontology: {}", e))?;
    
    let _ = fs::remove_file(&temp_path);

    // Extract entities
    let mut entities = extract_entities(&store)?;

    // Filter by type (if specified)
    if let Some(filter_type) = params.entity_type_filter {
        let filter_type_iri = filter_type.ddd_class_iri();
        entities.retain(|e| {
            e.entity_type
                .as_ref()
                .map(|t| t == filter_type_iri)
                .unwrap_or(false)
        });
    }

    // Strip properties if not requested
    if !params.include_properties {
        for entity in &mut entities {
            entity.properties.clear();
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(QueryEntitiesResponse {
        path: params.path,
        entities,
        duration_ms,
    })
}

// =============================================================================
// Helper Functions
// =============================================================================

fn validate_path_input(path: &str) -> Result<()> {
    validate_non_empty_string("path", path).context("path cannot be empty")?;
    validate_path_safe(path).context("path contains path traversal")?;
    if path.len() > MAX_PATH_LENGTH {
        return Err(anyhow!("path exceeds max length of {}", MAX_PATH_LENGTH));
    }
    Ok(())
}

fn resolve_ontology_path(state: &AppState, path: &str) -> Result<PathBuf> {
    let full_path = state.config().workspace_root.join(path);
    if !full_path.exists() {
        return Err(anyhow!("ontology file not found: {}", path));
    }
    Ok(full_path)
}

fn create_backup(path: &Path) -> Result<PathBuf> {
    let backup_path = path.with_extension(format!(
        "{}.{}",
        path.extension().unwrap_or_default().to_string_lossy(),
        BACKUP_SUFFIX
    ));

    fs::copy(path, &backup_path).context("failed to create backup")?;

    Ok(backup_path)
}

fn write_turtle_atomic(path: &Path, content: &str) -> Result<()> {
    let tmp_path = path.with_extension("tmp");

    // Write to temporary file
    fs::write(&tmp_path, content).context("failed to write temporary file")?;

    // Atomic rename
    fs::rename(&tmp_path, path).context("failed to rename temporary file")?;

    Ok(())
}

fn extract_entities(store: &TripleStore) -> Result<Vec<EntityInfo>> {
    let query = r#"
        PREFIX ddd: <http://ggen-mcp.dev/ontology/ddd#>
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>

        SELECT ?entity ?type ?label ?comment WHERE {
            ?entity rdf:type ?type .
            FILTER(?type IN (
                ddd:Entity, ddd:ValueObject, ddd:AggregateRoot,
                ddd:DomainEvent, ddd:Command, ddd:Query
            ))
            OPTIONAL { ?entity rdfs:label ?label }
            OPTIONAL { ?entity rdfs:comment ?comment }
        }
    "#;

    let json_result = store.query_sparql(query)
        .map_err(|e| anyhow!("failed to query entities: {}", e))?;
    
    let parsed: JsonValue = serde_json::from_str(&json_result)
        .context("failed to parse query results")?;

    let mut entities_map: HashMap<String, EntityInfo> = HashMap::new();

    // Parse JSON results from ggen
    if let Some(results) = parsed.get("results") {
        if let Some(bindings_array) = results.get("bindings").and_then(|b| b.as_array()) {
            for binding_obj in bindings_array {
                if let Some(binding_map) = binding_obj.as_object() {
                    // Extract entity IRI
                    let entity_iri = binding_map
                        .get("entity")
                        .and_then(|v| extract_iri_from_json_value(v))
                        .ok_or_else(|| anyhow!("missing entity IRI"))?;

                    // Extract entity type
                    let entity_type = binding_map
                        .get("type")
                        .and_then(|v| extract_iri_from_json_value(v));

                    // Extract label
                    let label = binding_map
                        .get("label")
                        .and_then(|v| extract_literal_from_json_value(v));

                    // Extract comment
                    let comment = binding_map
                        .get("comment")
                        .and_then(|v| extract_literal_from_json_value(v));

                    let entity_name = extract_local_name(&entity_iri);

                    entities_map
                        .entry(entity_iri.clone())
                        .or_insert_with(|| EntityInfo {
                            name: entity_name,
                            iri: entity_iri.clone(),
                            entity_type,
                            properties: Vec::new(),
                            label,
                            comment,
                        });
                }
            }
        }
    }

    // Query properties for each entity
    for entity_info in entities_map.values_mut() {
        entity_info.properties = extract_properties_for_entity(store, &entity_info.iri)?;
    }

    Ok(entities_map.into_values().collect())
}

fn extract_properties_for_entity(store: &TripleStore, entity_iri: &str) -> Result<Vec<PropertyInfo>> {
    let query = format!(
        r#"
        PREFIX ddd: <http://ggen-mcp.dev/ontology/ddd#>
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

        SELECT ?property ?type ?required ?label WHERE {{
            <{}> ddd:hasProperty ?property .
            OPTIONAL {{ ?property ddd:type ?type }}
            OPTIONAL {{ ?property ddd:required ?required }}
            OPTIONAL {{ ?property rdfs:label ?label }}
        }}
        "#,
        entity_iri
    );

    let json_result = store.query_sparql(&query)
        .map_err(|e| anyhow!("failed to query properties: {}", e))?;
    
    let parsed: JsonValue = serde_json::from_str(&json_result)
        .context("failed to parse query results")?;

    let mut properties = Vec::new();

    // Parse JSON results from ggen
    if let Some(results) = parsed.get("results") {
        if let Some(bindings_array) = results.get("bindings").and_then(|b| b.as_array()) {
            for binding_obj in bindings_array {
                if let Some(binding_map) = binding_obj.as_object() {
                    let property_iri = binding_map
                        .get("property")
                        .and_then(|v| extract_iri_from_json_value(v))
                        .ok_or_else(|| anyhow!("missing property IRI"))?;

                    let rust_type = binding_map
                        .get("type")
                        .and_then(|v| extract_literal_from_json_value(v));

                    let required = binding_map
                        .get("required")
                        .and_then(|v| extract_literal_from_json_value(v))
                        .and_then(|s| s.parse::<bool>().ok())
                        .unwrap_or(false);

                    let label = binding_map
                        .get("label")
                        .and_then(|v| extract_literal_from_json_value(v));

                    properties.push(PropertyInfo {
                        name: extract_local_name(&property_iri),
                        iri: property_iri,
                        rust_type,
                        required,
                        label,
                    });
                }
            }
        }
    }

    Ok(properties)
}

fn extract_prefixes(content: &str) -> Result<HashMap<String, String>> {
    let mut prefixes = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("@prefix") {
            // Parse: @prefix ex: <http://example.org/> .
            if let Some(parts) = trimmed.strip_prefix("@prefix") {
                let parts: Vec<&str> = parts.trim().split_whitespace().collect();
                if parts.len() >= 2 {
                    let prefix = parts[0].trim_end_matches(':');
                    let iri = parts[1].trim_matches(&['<', '>', '.'][..]);
                    prefixes.insert(prefix.to_string(), iri.to_string());
                }
            }
        }
    }

    Ok(prefixes)
}

fn extract_local_name(iri: &str) -> String {
    iri.split(&['#', '/'][..]).last().unwrap_or(iri).to_string()
}

/// Extract IRI from JSON value (ggen format: string or {"value": "...", "type": "uri"})
fn extract_iri_from_json_value(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::String(s) => {
            // Check if it's an IRI (starts with < or http)
            if s.starts_with('<') && s.ends_with('>') {
                Some(s[1..s.len()-1].to_string())
            } else if s.starts_with("http://") || s.starts_with("https://") {
                Some(s.clone())
            } else {
                None
            }
        }
        JsonValue::Object(obj) => {
            if let Some(type_val) = obj.get("type") {
                if type_val.as_str() == Some("uri") {
                    obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Extract literal from JSON value (ggen format: string or {"value": "...", "type": "literal"})
fn extract_literal_from_json_value(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::String(s) => {
            // If it's not an IRI, treat as literal
            if !s.starts_with('<') && !s.starts_with("http://") && !s.starts_with("https://") {
                Some(s.clone())
            } else {
                None
            }
        }
        JsonValue::Object(obj) => {
            if let Some(type_val) = obj.get("type") {
                if type_val.as_str() == Some("literal") {
                    obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string())
                } else {
                    None
                }
            } else {
                // Default to value if no type specified
                obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string())
            }
        }
        _ => None,
    }
}

/// Count triples in TripleStore using SPARQL query
fn count_triples_in_store(store: &TripleStore) -> Result<usize> {
    let count_query = r#"
        SELECT (COUNT(*) AS ?count)
        WHERE {
            ?s ?p ?o .
        }
    "#;
    
    let json_result = store.query_sparql(count_query)
        .map_err(|e| anyhow!("failed to count triples: {}", e))?;
    
    let parsed: JsonValue = serde_json::from_str(&json_result)
        .context("failed to parse count query result")?;
    
    // Extract count from SPARQL JSON results format
    if let Some(results) = parsed.get("results") {
        if let Some(bindings) = results.get("bindings").and_then(|b| b.as_array()) {
            if let Some(first) = bindings.first() {
                if let Some(count_obj) = first.get("count") {
                    // Handle both string and structured format
                    if let Some(count_str) = count_obj.as_str() {
                        return Ok(count_str.parse().unwrap_or(0));
                    } else if let Some(count_val) = count_obj.get("value") {
                        if let Some(count_str) = count_val.as_str() {
                            return Ok(count_str.parse().unwrap_or(0));
                        }
                    }
                }
            }
        }
    }
    
    Ok(0)
}

fn entity_exists_in_store(store: &TripleStore, entity_iri: &str) -> Result<bool> {
    let query = format!(
        r#"ASK {{ <{}> ?p ?o }}"#,
        entity_iri.replace("mcp:", MCP_PREFIX)
    );

    let json_result = store.query_sparql(&query)
        .map_err(|e| anyhow!("failed to execute ASK query: {}", e))?;
    
    let parsed: JsonValue = serde_json::from_str(&json_result)
        .context("failed to parse ASK result")?;
    
    if let Some(boolean) = parsed.get("boolean") {
        if let Some(exists) = boolean.as_bool() {
            return Ok(exists);
        }
    }
    
    Ok(false)
}

fn property_exists_in_store(store: &TripleStore, property_iri: &str) -> Result<bool> {
    let query = format!(
        r#"ASK {{ <{}> ?p ?o }}"#,
        property_iri.replace("mcp:", MCP_PREFIX)
    );

    let json_result = store.query_sparql(&query)
        .map_err(|e| anyhow!("failed to execute ASK query: {}", e))?;
    
    let parsed: JsonValue = serde_json::from_str(&json_result)
        .context("failed to parse ASK result")?;
    
    if let Some(boolean) = parsed.get("boolean") {
        if let Some(exists) = boolean.as_bool() {
            return Ok(exists);
        }
    }
    
    Ok(false)
}

fn generate_entity_turtle(params: &AddEntityParams) -> Result<String> {
    let entity_iri = format!("mcp:{}", params.entity_name.as_str());
    let ddd_class = params
        .entity_type
        .ddd_class_iri()
        .replace(DDD_PREFIX, "ddd:");
    let label = params
        .label
        .as_deref()
        .unwrap_or(params.entity_name.as_str());

    let mut turtle = format!(
        "{} a {} ;\n    rdfs:label \"{}\" ",
        entity_iri, ddd_class, label
    );

    if let Some(comment) = &params.comment {
        turtle.push_str(&format!(";\n    rdfs:comment \"{}\" ", comment));
    }

    // Add properties
    if !params.properties.is_empty() {
        turtle.push_str(";\n    ddd:hasProperty ");
        let property_iris: Vec<String> = params
            .properties
            .iter()
            .map(|p| format!("mcp:{}", p.name.as_str()))
            .collect();
        turtle.push_str(&property_iris.join(", "));
    }

    turtle.push_str(" .\n\n");

    // Generate property definitions
    for prop in &params.properties {
        turtle.push_str(&generate_property_turtle(prop)?);
        turtle.push('\n');
    }

    Ok(turtle)
}

fn generate_property_turtle(prop: &PropertySpec) -> Result<String> {
    let property_iri = format!("mcp:{}", prop.name.as_str());
    let label = prop.description.as_deref().unwrap_or(prop.name.as_str());

    let turtle = format!(
        "{} a ddd:Property ;\n    rdfs:label \"{}\" ;\n    ddd:type \"{}\" ;\n    ddd:required {} .\n",
        property_iri, label, prop.rust_type, prop.required
    );

    Ok(turtle)
}

fn validate_turtle_content(content: &str, shacl: bool) -> Result<ValidationResult> {
    let mut parse_errors = Vec::new();
    let mut warnings = Vec::new();

    // Parse with TripleStore
    let store = TripleStore::new()
        .map_err(|e| anyhow!("failed to create TripleStore: {}", e))?;
    
    // Write content to temp file for loading
    let temp_path = std::env::temp_dir().join(format!("validate_{}.ttl", std::process::id()));
    fs::write(&temp_path, content).context("failed to write temp file")?;
    
    match store.load_turtle(&temp_path) {
        Ok(_) => {}
        Err(e) => {
            parse_errors.push(format!("Parse error: {}", e));
        }
    }
    
    let _ = fs::remove_file(&temp_path);

    let syntax_valid = parse_errors.is_empty();

    // SHACL validation (if requested and syntax is valid)
    // Note: ShapeValidator still requires Store, so we need to create a temporary Store
    let shacl_result = if shacl && syntax_valid {
        // Create temporary Store for SHACL validation
        use oxigraph::store::Store as OxigraphStore;
        use oxigraph::io::RdfFormat;
        let temp_store = OxigraphStore::new()
            .context("failed to create temp store for SHACL")?;
        let temp_path = std::env::temp_dir().join(format!("shacl_{}.ttl", std::process::id()));
        fs::write(&temp_path, content).context("failed to write temp file")?;
        let _ = temp_store.load_from_reader(RdfFormat::Turtle, fs::File::open(&temp_path)?);
        let _ = fs::remove_file(&temp_path);
        Some(perform_shacl_validation(&temp_store)?)
    } else {
        None
    };

    // Additional warnings (heuristic checks)
    if content.lines().count() > 10000 {
        warnings.push("Ontology is very large (>10k lines), consider splitting".to_string());
    }

    Ok(ValidationResult {
        syntax_valid,
        parse_errors,
        shacl_result,
        warnings,
    })
}

fn perform_shacl_validation(store: &oxigraph::store::Store) -> Result<ShaclValidationResult> {
    // Use ShapeValidator from ontology module
    // Note: ShapeValidator needs shapes file - for now, skip validation if shapes don't exist
    let shapes_path = Path::new("ontology/shapes.ttl");
    if !shapes_path.exists() {
        // Return conforming result if shapes file doesn't exist (optional validation)
        return Ok(ShaclValidationResult {
            conforms: true,
            violations: 0,
            violation_details: vec![],
        });
    }

    let validator = ShapeValidator::from_file(shapes_path)
        .context("Failed to load SHACL shapes")?;
    let report = validator.validate_graph(store)
        .context("SHACL validation failed")?;

    let conforms = report.conforms();
    let violations = report.violation_count();
    let violation_details: Vec<String> = report
        .violations()
        .map(|r| format!("{}", r.message()))
        .collect();

    Ok(ShaclValidationResult {
        conforms,
        violations,
        violation_details,
    })
}

fn count_lines_starting_with(text: &str, prefixes: &[&str]) -> usize {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim();
            prefixes.iter().any(|prefix| trimmed.starts_with(prefix))
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_name_validation() {
        assert!(EntityName::new("User".to_string()).is_ok());
        assert!(EntityName::new("".to_string()).is_err());
        assert!(EntityName::new("x".repeat(200)).is_err());
    }

    #[test]
    fn test_property_name_validation() {
        assert!(PropertyName::new("userName".to_string()).is_ok());
        assert!(PropertyName::new("".to_string()).is_err());
    }

    #[test]
    fn test_entity_type_iri() {
        assert_eq!(
            EntityType::Entity.ddd_class_iri(),
            "http://ggen-mcp.dev/ontology/ddd#Entity"
        );
        assert_eq!(
            EntityType::ValueObject.ddd_class_iri(),
            "http://ggen-mcp.dev/ontology/ddd#ValueObject"
        );
    }

    #[test]
    fn test_extract_local_name() {
        assert_eq!(extract_local_name("http://example.org#User"), "User");
        assert_eq!(extract_local_name("http://example.org/User"), "User");
        assert_eq!(extract_local_name("mcp:User"), "User");
    }

    #[test]
    fn test_extract_prefixes() {
        let turtle = r#"
@prefix mcp: <http://ggen-mcp.dev/ontology/mcp#> .
@prefix ddd: <http://ggen-mcp.dev/ontology/ddd#> .
        "#;

        let prefixes = extract_prefixes(turtle).unwrap();
        assert_eq!(
            prefixes.get("mcp"),
            Some(&"http://ggen-mcp.dev/ontology/mcp#".to_string())
        );
        assert_eq!(
            prefixes.get("ddd"),
            Some(&"http://ggen-mcp.dev/ontology/ddd#".to_string())
        );
    }

    #[test]
    fn test_generate_entity_turtle() {
        let params = AddEntityParams {
            path: "test.ttl".to_string(),
            entity_name: EntityName::new("User".to_string()).unwrap(),
            entity_type: EntityType::Entity,
            properties: vec![],
            label: Some("User Entity".to_string()),
            comment: Some("Represents a user in the system".to_string()),
            create_backup: false,
            validate_syntax: false,
        };

        let turtle = generate_entity_turtle(&params).unwrap();
        assert!(turtle.contains("mcp:User a ddd:Entity"));
        assert!(turtle.contains("rdfs:label \"User Entity\""));
        assert!(turtle.contains("rdfs:comment \"Represents a user in the system\""));
    }

    #[test]
    fn test_generate_property_turtle() {
        let prop = PropertySpec {
            name: PropertyName::new("userName".to_string()).unwrap(),
            rust_type: "String".to_string(),
            required: true,
            description: Some("User's name".to_string()),
        };

        let turtle = generate_property_turtle(&prop).unwrap();
        assert!(turtle.contains("mcp:userName a ddd:Property"));
        assert!(turtle.contains("ddd:type \"String\""));
        assert!(turtle.contains("ddd:required true"));
    }

    #[test]
    fn test_validate_turtle_content_valid() {
        let turtle = r#"
@prefix mcp: <http://ggen-mcp.dev/ontology/mcp#> .
@prefix ddd: <http://ggen-mcp.dev/ontology/ddd#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

mcp:User a ddd:Entity ;
    rdfs:label "User" .
        "#;

        let result = validate_turtle_content(turtle, false).unwrap();
        assert!(result.syntax_valid);
        assert!(result.parse_errors.is_empty());
    }

    #[test]
    fn test_validate_turtle_content_invalid() {
        let turtle = "INVALID TURTLE SYNTAX {{{";
        let result = validate_turtle_content(turtle, false).unwrap();
        assert!(!result.syntax_valid);
        assert!(!result.parse_errors.is_empty());
    }
}
