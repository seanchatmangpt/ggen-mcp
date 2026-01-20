//! Chicago-style TDD Master Test Fixture Library
//!
//! This module provides a comprehensive fixture management system following Chicago-style TDD
//! principles, where fixtures represent real, integrated domain objects with actual implementations.
//!
//! # Architecture
//!
//! - **FixtureLibrary**: Master registry for all fixtures with caching and lazy loading
//! - **Builders**: Fluent APIs for constructing domain objects (Aggregate, Ontology, Config, Context)
//! - **Fixtures**: Pre-configured test data organized by category
//! - **Composers**: Combine multiple fixtures into complex test scenarios
//!
//! # Usage
//!
//! ```rust,ignore
//! use fixture_library::Fixtures;
//!
//! // Load pre-configured fixtures
//! let user = Fixtures::user().minimal();
//! let order = Fixtures::order().with_items(3);
//! let config = Fixtures::config().production();
//!
//! // Build custom fixtures
//! let aggregate = AggregateBuilder::new("Product")
//!     .with_id("prod_123")
//!     .with_field("name", "String", true)
//!     .with_field("price", "Money", true)
//!     .with_command("UpdatePrice")
//!     .with_event("PriceUpdated")
//!     .build();
//!
//! // Compose complex scenarios
//! let domain = FixtureComposer::new()
//!     .add(user)
//!     .add(order)
//!     .build_ontology();
//! ```

#![allow(dead_code)]

use anyhow::{Context, Result};
use oxigraph::io::GraphFormat;
use oxigraph::model::{GraphNameRef, NamedNode, Subject, Term, Triple};
use oxigraph::store::Store;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tempfile::{tempdir, TempDir};

// =============================================================================
// Core Types and Traits
// =============================================================================

/// Version for fixture compatibility tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FixtureVersion(pub u32);

impl FixtureVersion {
    pub const V1: Self = Self(1);
    pub const CURRENT: Self = Self::V1;
}

/// Fixture metadata for tracking and documentation
#[derive(Debug, Clone)]
pub struct FixtureMetadata {
    pub name: String,
    pub category: FixtureCategory,
    pub version: FixtureVersion,
    pub description: String,
    pub valid: bool,
    pub tags: HashSet<String>,
}

/// Categories for organizing fixtures (80/20 principle)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FixtureCategory {
    Domain,
    Configuration,
    Ontology,
    TemplateContext,
    SparqlQuery,
}

/// Main fixture trait that all fixtures must implement
pub trait Fixture: Send + Sync {
    fn metadata(&self) -> &FixtureMetadata;
    fn clone_box(&self) -> Box<dyn Fixture>;
}

// =============================================================================
// Fixture Library - Master Registry
// =============================================================================

/// Master fixture registry with lazy loading and caching
pub struct FixtureLibrary {
    fixtures: Arc<Mutex<HashMap<String, Box<dyn Fixture>>>>,
    cache: Arc<Mutex<HashMap<String, CachedFixture>>>,
    base_path: PathBuf,
}

#[derive(Clone)]
struct CachedFixture {
    fixture: Arc<Box<dyn Fixture>>,
    loaded_at: std::time::Instant,
}

impl FixtureLibrary {
    /// Create a new fixture library with default fixtures directory
    pub fn new() -> Self {
        let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures");

        Self {
            fixtures: Arc::new(Mutex::new(HashMap::new())),
            cache: Arc::new(Mutex::new(HashMap::new())),
            base_path,
        }
    }

    /// Create with custom fixtures directory
    pub fn with_path<P: Into<PathBuf>>(path: P) -> Self {
        Self {
            fixtures: Arc::new(Mutex::new(HashMap::new())),
            cache: Arc::new(Mutex::new(HashMap::new())),
            base_path: path.into(),
        }
    }

    /// Register a fixture
    pub fn register(&self, name: impl Into<String>, fixture: Box<dyn Fixture>) {
        let mut fixtures = self.fixtures.lock().unwrap();
        fixtures.insert(name.into(), fixture);
    }

    /// Get a fixture by name (with caching)
    pub fn get(&self, name: &str) -> Option<Arc<Box<dyn Fixture>>> {
        // Check cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some(cached) = cache.get(name) {
                return Some(Arc::clone(&cached.fixture));
            }
        }

        // Load from registry
        let fixtures = self.fixtures.lock().unwrap();
        if let Some(fixture) = fixtures.get(name) {
            let fixture_arc = Arc::new(fixture.clone_box());

            // Cache it
            let mut cache = self.cache.lock().unwrap();
            cache.insert(
                name.to_string(),
                CachedFixture {
                    fixture: Arc::clone(&fixture_arc),
                    loaded_at: std::time::Instant::now(),
                },
            );

            return Some(fixture_arc);
        }

        None
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }

    /// List all registered fixtures
    pub fn list(&self) -> Vec<String> {
        let fixtures = self.fixtures.lock().unwrap();
        fixtures.keys().cloned().collect()
    }

    /// Get fixtures by category
    pub fn by_category(&self, category: FixtureCategory) -> Vec<String> {
        let fixtures = self.fixtures.lock().unwrap();
        fixtures
            .iter()
            .filter(|(_, f)| f.metadata().category == category)
            .map(|(name, _)| name.clone())
            .collect()
    }
}

impl Default for FixtureLibrary {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Domain Fixtures
// =============================================================================

/// Domain aggregate fixture
#[derive(Debug, Clone)]
pub struct AggregateFixture {
    pub metadata: FixtureMetadata,
    pub id: String,
    pub name: String,
    pub fields: Vec<AggregateField>,
    pub commands: Vec<String>,
    pub events: Vec<String>,
    pub invariants: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AggregateField {
    pub name: String,
    pub type_name: String,
    pub required: bool,
    pub description: Option<String>,
}

impl Fixture for AggregateFixture {
    fn metadata(&self) -> &FixtureMetadata {
        &self.metadata
    }

    fn clone_box(&self) -> Box<dyn Fixture> {
        Box::new(self.clone())
    }
}

impl AggregateFixture {
    /// Convert to ontology TTL
    pub fn to_ttl(&self) -> String {
        let mut ttl = String::new();
        ttl.push_str(&format!("@prefix ex: <http://example.org/> .\n"));
        ttl.push_str(&format!(
            "@prefix ddd: <https://ddd-patterns.dev/schema#> .\n"
        ));
        ttl.push_str(&format!(
            "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\n"
        ));

        ttl.push_str(&format!("ex:{} a ddd:Aggregate ;\n", self.name));
        ttl.push_str(&format!("    rdfs:label \"{}\" ;\n", self.name));

        // Add fields
        for field in &self.fields {
            ttl.push_str(&format!(
                "    ddd:hasField ex:{}_{} ;\n",
                self.name, field.name
            ));
        }

        // Add commands
        for cmd in &self.commands {
            ttl.push_str(&format!("    ddd:handlesCommand ex:{} ;\n", cmd));
        }

        // Add events
        for event in &self.events {
            ttl.push_str(&format!("    ddd:emitsEvent ex:{} ;\n", event));
        }

        ttl.push_str("    .\n");
        ttl
    }

    /// Convert to template context
    pub fn to_context(&self) -> TemplateContextFixture {
        let mut fields_map = HashMap::new();
        for field in &self.fields {
            fields_map.insert(field.name.clone(), field.type_name.clone());
        }

        TemplateContextFixture {
            metadata: FixtureMetadata {
                name: format!("{}_context", self.name),
                category: FixtureCategory::TemplateContext,
                version: FixtureVersion::CURRENT,
                description: format!("Template context for {}", self.name),
                valid: true,
                tags: HashSet::new(),
            },
            entity_name: self.name.clone(),
            fields: fields_map,
            imports: HashMap::new(),
            custom: HashMap::new(),
        }
    }
}

// =============================================================================
// Aggregate Builder
// =============================================================================

/// Fluent builder for domain aggregates
pub struct AggregateBuilder {
    id: Option<String>,
    name: String,
    fields: Vec<AggregateField>,
    commands: Vec<String>,
    events: Vec<String>,
    invariants: Vec<String>,
    description: Option<String>,
    valid: bool,
    tags: HashSet<String>,
}

impl AggregateBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: None,
            name: name.into(),
            fields: Vec::new(),
            commands: Vec::new(),
            events: Vec::new(),
            invariants: Vec::new(),
            description: None,
            valid: true,
            tags: HashSet::new(),
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn with_field(
        mut self,
        name: impl Into<String>,
        type_name: impl Into<String>,
        required: bool,
    ) -> Self {
        self.fields.push(AggregateField {
            name: name.into(),
            type_name: type_name.into(),
            required,
            description: None,
        });
        self
    }

    pub fn with_field_desc(
        mut self,
        name: impl Into<String>,
        type_name: impl Into<String>,
        required: bool,
        description: impl Into<String>,
    ) -> Self {
        self.fields.push(AggregateField {
            name: name.into(),
            type_name: type_name.into(),
            required,
            description: Some(description.into()),
        });
        self
    }

    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.commands.push(command.into());
        self
    }

    pub fn with_event(mut self, event: impl Into<String>) -> Self {
        self.events.push(event.into());
        self
    }

    pub fn with_invariant(mut self, invariant: impl Into<String>) -> Self {
        self.invariants.push(invariant.into());
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn invalid(mut self) -> Self {
        self.valid = false;
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.insert(tag.into());
        self
    }

    pub fn build(self) -> AggregateFixture {
        let id = self
            .id
            .unwrap_or_else(|| format!("{}_001", self.name.to_lowercase()));
        let description = self
            .description
            .unwrap_or_else(|| format!("{} aggregate", self.name));

        AggregateFixture {
            metadata: FixtureMetadata {
                name: self.name.clone(),
                category: FixtureCategory::Domain,
                version: FixtureVersion::CURRENT,
                description,
                valid: self.valid,
                tags: self.tags,
            },
            id,
            name: self.name,
            fields: self.fields,
            commands: self.commands,
            events: self.events,
            invariants: self.invariants,
        }
    }
}

// =============================================================================
// Configuration Fixtures
// =============================================================================

/// Configuration fixture
#[derive(Debug, Clone)]
pub struct ConfigFixture {
    pub metadata: FixtureMetadata,
    pub workspace_root: PathBuf,
    pub cache_capacity: usize,
    pub recalc_enabled: bool,
    pub vba_enabled: bool,
    pub max_concurrent_recalcs: usize,
    pub tool_timeout_ms: Option<u64>,
    pub max_response_bytes: Option<u64>,
}

impl Fixture for ConfigFixture {
    fn metadata(&self) -> &FixtureMetadata {
        &self.metadata
    }

    fn clone_box(&self) -> Box<dyn Fixture> {
        Box::new(self.clone())
    }
}

// =============================================================================
// Config Builder
// =============================================================================

/// Fluent builder for server configurations
pub struct ConfigBuilder {
    workspace_root: Option<PathBuf>,
    cache_capacity: usize,
    recalc_enabled: bool,
    vba_enabled: bool,
    max_concurrent_recalcs: usize,
    tool_timeout_ms: Option<u64>,
    max_response_bytes: Option<u64>,
    description: Option<String>,
    valid: bool,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            workspace_root: None,
            cache_capacity: 5,
            recalc_enabled: false,
            vba_enabled: false,
            max_concurrent_recalcs: 2,
            tool_timeout_ms: Some(30_000),
            max_response_bytes: Some(1_000_000),
            description: None,
            valid: true,
        }
    }

    pub fn workspace_root<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.workspace_root = Some(path.into());
        self
    }

    pub fn cache_capacity(mut self, capacity: usize) -> Self {
        self.cache_capacity = capacity;
        self
    }

    pub fn with_recalc(mut self) -> Self {
        self.recalc_enabled = true;
        self
    }

    pub fn with_vba(mut self) -> Self {
        self.vba_enabled = true;
        self
    }

    pub fn max_concurrent_recalcs(mut self, max: usize) -> Self {
        self.max_concurrent_recalcs = max;
        self
    }

    pub fn tool_timeout_ms(mut self, timeout: u64) -> Self {
        self.tool_timeout_ms = Some(timeout);
        self
    }

    pub fn no_timeout(mut self) -> Self {
        self.tool_timeout_ms = None;
        self
    }

    pub fn max_response_bytes(mut self, max: u64) -> Self {
        self.max_response_bytes = Some(max);
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn invalid(mut self) -> Self {
        self.valid = false;
        self
    }

    pub fn build(self) -> ConfigFixture {
        let workspace_root = self
            .workspace_root
            .unwrap_or_else(|| PathBuf::from("/tmp/test-workspace"));
        let description = self
            .description
            .unwrap_or_else(|| "Test configuration".to_string());

        ConfigFixture {
            metadata: FixtureMetadata {
                name: "config".to_string(),
                category: FixtureCategory::Configuration,
                version: FixtureVersion::CURRENT,
                description,
                valid: self.valid,
                tags: HashSet::new(),
            },
            workspace_root,
            cache_capacity: self.cache_capacity,
            recalc_enabled: self.recalc_enabled,
            vba_enabled: self.vba_enabled,
            max_concurrent_recalcs: self.max_concurrent_recalcs,
            tool_timeout_ms: self.tool_timeout_ms,
            max_response_bytes: self.max_response_bytes,
        }
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Ontology Fixtures
// =============================================================================

/// Ontology fixture with RDF store
#[derive(Clone)]
pub struct OntologyFixture {
    pub metadata: FixtureMetadata,
    pub ttl: String,
    pub prefixes: HashMap<String, String>,
    store: Arc<Mutex<Option<Store>>>,
}

impl std::fmt::Debug for OntologyFixture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OntologyFixture")
            .field("metadata", &self.metadata)
            .field(
                "ttl",
                &format!("{}...", &self.ttl.chars().take(50).collect::<String>()),
            )
            .field("prefixes", &self.prefixes)
            .finish()
    }
}

impl Fixture for OntologyFixture {
    fn metadata(&self) -> &FixtureMetadata {
        &self.metadata
    }

    fn clone_box(&self) -> Box<dyn Fixture> {
        Box::new(self.clone())
    }
}

impl OntologyFixture {
    /// Get or create the RDF store
    pub fn store(&self) -> Result<Store> {
        let mut store_opt = self.store.lock().unwrap();

        if let Some(store) = store_opt.as_ref() {
            return Ok(store.clone());
        }

        // Create new store
        let store = Store::new()?;
        store.load_from_reader(
            GraphFormat::Turtle,
            self.ttl.as_bytes(),
            GraphNameRef::DefaultGraph,
            None,
        )?;

        *store_opt = Some(store.clone());
        Ok(store)
    }

    /// Validate the ontology
    pub fn validate(&self) -> Result<()> {
        let _store = self.store()?;
        // TODO: Add SHACL validation
        Ok(())
    }
}

// =============================================================================
// Ontology Builder
// =============================================================================

/// Fluent builder for RDF ontologies
pub struct OntologyBuilder {
    prefixes: HashMap<String, String>,
    aggregates: Vec<String>,
    value_objects: Vec<String>,
    commands: Vec<String>,
    events: Vec<String>,
    custom_triples: Vec<(String, String, String)>,
    description: Option<String>,
    valid: bool,
}

impl OntologyBuilder {
    pub fn new() -> Self {
        let mut prefixes = HashMap::new();
        prefixes.insert(
            "ddd".to_string(),
            "https://ddd-patterns.dev/schema#".to_string(),
        );
        prefixes.insert("ex".to_string(), "http://example.org/".to_string());
        prefixes.insert(
            "rdf".to_string(),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string(),
        );
        prefixes.insert(
            "rdfs".to_string(),
            "http://www.w3.org/2000/01/rdf-schema#".to_string(),
        );

        Self {
            prefixes,
            aggregates: Vec::new(),
            value_objects: Vec::new(),
            commands: Vec::new(),
            events: Vec::new(),
            custom_triples: Vec::new(),
            description: None,
            valid: true,
        }
    }

    pub fn prefix(mut self, prefix: impl Into<String>, namespace: impl Into<String>) -> Self {
        self.prefixes.insert(prefix.into(), namespace.into());
        self
    }

    pub fn add_aggregate(mut self, name: impl Into<String>) -> Self {
        self.aggregates.push(name.into());
        self
    }

    pub fn add_value_object(mut self, name: impl Into<String>) -> Self {
        self.value_objects.push(name.into());
        self
    }

    pub fn add_command(mut self, name: impl Into<String>) -> Self {
        self.commands.push(name.into());
        self
    }

    pub fn add_event(mut self, name: impl Into<String>) -> Self {
        self.events.push(name.into());
        self
    }

    pub fn add_triple(
        mut self,
        subject: impl Into<String>,
        predicate: impl Into<String>,
        object: impl Into<String>,
    ) -> Self {
        self.custom_triples
            .push((subject.into(), predicate.into(), object.into()));
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn invalid(mut self) -> Self {
        self.valid = false;
        self
    }

    pub fn build_ttl(self) -> String {
        let mut ttl = String::new();

        // Add prefixes
        for (prefix, namespace) in &self.prefixes {
            ttl.push_str(&format!("@prefix {}: <{}> .\n", prefix, namespace));
        }
        ttl.push('\n');

        // Add aggregates
        for aggregate in &self.aggregates {
            ttl.push_str(&format!("ex:{} a ddd:Aggregate ;\n", aggregate));
            ttl.push_str(&format!("    rdfs:label \"{}\" .\n\n", aggregate));
        }

        // Add value objects
        for vo in &self.value_objects {
            ttl.push_str(&format!("ex:{} a ddd:ValueObject ;\n", vo));
            ttl.push_str(&format!("    rdfs:label \"{}\" .\n\n", vo));
        }

        // Add commands
        for cmd in &self.commands {
            ttl.push_str(&format!("ex:{} a ddd:Command ;\n", cmd));
            ttl.push_str(&format!("    rdfs:label \"{}\" .\n\n", cmd));
        }

        // Add events
        for event in &self.events {
            ttl.push_str(&format!("ex:{} a ddd:Event ;\n", event));
            ttl.push_str(&format!("    rdfs:label \"{}\" .\n\n", event));
        }

        // Add custom triples
        for (s, p, o) in &self.custom_triples {
            ttl.push_str(&format!("{} {} {} .\n", s, p, o));
        }

        ttl
    }

    pub fn build(self) -> OntologyFixture {
        let ttl = self.build_ttl();
        let description = self
            .description
            .unwrap_or_else(|| "Test ontology".to_string());

        OntologyFixture {
            metadata: FixtureMetadata {
                name: "ontology".to_string(),
                category: FixtureCategory::Ontology,
                version: FixtureVersion::CURRENT,
                description,
                valid: self.valid,
                tags: HashSet::new(),
            },
            ttl,
            prefixes: self.prefixes,
            store: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for OntologyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Template Context Fixtures
// =============================================================================

/// Template context fixture for code generation
#[derive(Debug, Clone)]
pub struct TemplateContextFixture {
    pub metadata: FixtureMetadata,
    pub entity_name: String,
    pub fields: HashMap<String, String>,
    pub imports: HashMap<String, Vec<String>>,
    pub custom: HashMap<String, JsonValue>,
}

impl Fixture for TemplateContextFixture {
    fn metadata(&self) -> &FixtureMetadata {
        &self.metadata
    }

    fn clone_box(&self) -> Box<dyn Fixture> {
        Box::new(self.clone())
    }
}

impl TemplateContextFixture {
    /// Convert to JSON for template rendering
    pub fn to_json(&self) -> JsonValue {
        let mut map = serde_json::Map::new();
        map.insert(
            "entity_name".to_string(),
            JsonValue::String(self.entity_name.clone()),
        );
        map.insert(
            "fields".to_string(),
            serde_json::to_value(&self.fields).unwrap(),
        );
        map.insert(
            "imports".to_string(),
            serde_json::to_value(&self.imports).unwrap(),
        );

        for (k, v) in &self.custom {
            map.insert(k.clone(), v.clone());
        }

        JsonValue::Object(map)
    }
}

// =============================================================================
// Template Context Builder
// =============================================================================

/// Fluent builder for template contexts
pub struct TemplateContextBuilder {
    entity_name: String,
    fields: HashMap<String, String>,
    imports: HashMap<String, Vec<String>>,
    custom: HashMap<String, JsonValue>,
    description: Option<String>,
    valid: bool,
}

impl TemplateContextBuilder {
    pub fn new() -> Self {
        Self {
            entity_name: String::new(),
            fields: HashMap::new(),
            imports: HashMap::new(),
            custom: HashMap::new(),
            description: None,
            valid: true,
        }
    }

    pub fn entity_name(mut self, name: impl Into<String>) -> Self {
        self.entity_name = name.into();
        self
    }

    pub fn add_field(mut self, name: impl Into<String>, type_name: impl Into<String>) -> Self {
        self.fields.insert(name.into(), type_name.into());
        self
    }

    pub fn add_import(mut self, module: impl Into<String>, items: Vec<impl Into<String>>) -> Self {
        self.imports
            .insert(module.into(), items.into_iter().map(|s| s.into()).collect());
        self
    }

    pub fn add_custom(mut self, key: impl Into<String>, value: JsonValue) -> Self {
        self.custom.insert(key.into(), value);
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn invalid(mut self) -> Self {
        self.valid = false;
        self
    }

    pub fn build(self) -> TemplateContextFixture {
        let description = self
            .description
            .unwrap_or_else(|| format!("Template context for {}", self.entity_name));

        TemplateContextFixture {
            metadata: FixtureMetadata {
                name: format!("{}_context", self.entity_name),
                category: FixtureCategory::TemplateContext,
                version: FixtureVersion::CURRENT,
                description,
                valid: self.valid,
                tags: HashSet::new(),
            },
            entity_name: self.entity_name,
            fields: self.fields,
            imports: self.imports,
            custom: self.custom,
        }
    }
}

impl Default for TemplateContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// SPARQL Query Fixtures
// =============================================================================

/// SPARQL query fixture
#[derive(Debug, Clone)]
pub struct SparqlQueryFixture {
    pub metadata: FixtureMetadata,
    pub query: String,
    pub query_type: QueryType,
    pub expected_results: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryType {
    Select,
    Construct,
    Ask,
    Describe,
}

impl Fixture for SparqlQueryFixture {
    fn metadata(&self) -> &FixtureMetadata {
        &self.metadata
    }

    fn clone_box(&self) -> Box<dyn Fixture> {
        Box::new(self.clone())
    }
}

// =============================================================================
// Fixture Composer - Complex Scenarios
// =============================================================================

/// Compose multiple fixtures into complex test scenarios
pub struct FixtureComposer {
    fixtures: Vec<Box<dyn Fixture>>,
}

impl FixtureComposer {
    pub fn new() -> Self {
        Self {
            fixtures: Vec::new(),
        }
    }

    pub fn add(mut self, fixture: impl Fixture + 'static) -> Self {
        self.fixtures.push(Box::new(fixture));
        self
    }

    pub fn add_boxed(mut self, fixture: Box<dyn Fixture>) -> Self {
        self.fixtures.push(fixture);
        self
    }

    /// Build a combined ontology from all aggregate fixtures
    pub fn build_ontology(self) -> Result<OntologyFixture> {
        let mut builder = OntologyBuilder::new();
        builder = builder.description("Composed ontology from multiple fixtures");

        for fixture in &self.fixtures {
            // Try to downcast to AggregateFixture
            if let Some(agg_fixture) =
                (fixture as &dyn std::any::Any).downcast_ref::<AggregateFixture>()
            {
                builder = builder.add_aggregate(&agg_fixture.name);
                for cmd in &agg_fixture.commands {
                    builder = builder.add_command(cmd);
                }
                for event in &agg_fixture.events {
                    builder = builder.add_event(event);
                }
            }
        }

        Ok(builder.build())
    }

    pub fn fixtures(&self) -> &[Box<dyn Fixture>] {
        &self.fixtures
    }
}

impl Default for FixtureComposer {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Pre-configured Fixture Collections
// =============================================================================

/// Main fixture accessor with pre-configured fixtures
pub struct Fixtures;

impl Fixtures {
    /// User aggregate fixtures
    pub fn user() -> UserFixtures {
        UserFixtures
    }

    /// Order aggregate fixtures
    pub fn order() -> OrderFixtures {
        OrderFixtures
    }

    /// Product aggregate fixtures
    pub fn product() -> ProductFixtures {
        ProductFixtures
    }

    /// Payment aggregate fixtures
    pub fn payment() -> PaymentFixtures {
        PaymentFixtures
    }

    /// Configuration fixtures
    pub fn config() -> ConfigFixtures {
        ConfigFixtures
    }

    /// Ontology fixtures
    pub fn ontology() -> OntologyFixtures {
        OntologyFixtures
    }
}

/// User aggregate fixtures
pub struct UserFixtures;

impl UserFixtures {
    pub fn minimal() -> AggregateFixture {
        AggregateBuilder::new("User")
            .with_id("user_001")
            .with_field("id", "UserId", true)
            .with_field("name", "String", true)
            .with_field("email", "Email", true)
            .with_command("CreateUser")
            .with_event("UserCreated")
            .description("Minimal valid user aggregate")
            .tag("minimal")
            .build()
    }

    pub fn complete() -> AggregateFixture {
        AggregateBuilder::new("User")
            .with_id("user_002")
            .with_field("id", "UserId", true)
            .with_field("name", "String", true)
            .with_field("email", "Email", true)
            .with_field("phone", "PhoneNumber", false)
            .with_field("address", "Address", false)
            .with_field("created_at", "DateTime", true)
            .with_field("updated_at", "DateTime", true)
            .with_command("CreateUser")
            .with_command("UpdateUserProfile")
            .with_command("ChangeUserEmail")
            .with_command("DeleteUser")
            .with_event("UserCreated")
            .with_event("UserProfileUpdated")
            .with_event("UserEmailChanged")
            .with_event("UserDeleted")
            .with_invariant("email must be unique")
            .with_invariant("name must not be empty")
            .description("Complete user aggregate with all fields")
            .tag("complete")
            .build()
    }

    pub fn invalid() -> AggregateFixture {
        AggregateBuilder::new("User")
            .with_id("user_invalid")
            .invalid()
            .description("Invalid user aggregate for testing error handling")
            .tag("invalid")
            .build()
    }
}

/// Order aggregate fixtures
pub struct OrderFixtures;

impl OrderFixtures {
    pub fn empty() -> AggregateFixture {
        AggregateBuilder::new("Order")
            .with_id("order_001")
            .with_field("id", "OrderId", true)
            .with_field("user_id", "UserId", true)
            .with_field("status", "OrderStatus", true)
            .with_field("total", "Money", true)
            .with_command("CreateOrder")
            .with_event("OrderCreated")
            .description("Empty order without items")
            .tag("empty")
            .build()
    }

    pub fn with_items(count: usize) -> AggregateFixture {
        let mut builder = AggregateBuilder::new("Order")
            .with_id(format!("order_{:03}", count))
            .with_field("id", "OrderId", true)
            .with_field("user_id", "UserId", true)
            .with_field("status", "OrderStatus", true)
            .with_field("total", "Money", true)
            .with_field("items", "Vec<OrderItem>", true)
            .with_command("CreateOrder")
            .with_command("AddOrderItem")
            .with_command("RemoveOrderItem")
            .with_event("OrderCreated")
            .with_event("OrderItemAdded")
            .with_event("OrderItemRemoved")
            .description(format!("Order with {} items", count))
            .tag("with_items");

        for i in 0..count {
            builder = builder.with_invariant(format!("item_{} must have positive quantity", i));
        }

        builder.build()
    }

    pub fn cancelled() -> AggregateFixture {
        AggregateBuilder::new("Order")
            .with_id("order_cancelled")
            .with_field("id", "OrderId", true)
            .with_field("user_id", "UserId", true)
            .with_field("status", "OrderStatus", true)
            .with_field("total", "Money", true)
            .with_field("cancelled_at", "DateTime", true)
            .with_field("cancellation_reason", "String", true)
            .with_command("CancelOrder")
            .with_event("OrderCancelled")
            .description("Cancelled order")
            .tag("cancelled")
            .build()
    }
}

/// Product aggregate fixtures
pub struct ProductFixtures;

impl ProductFixtures {
    pub fn in_stock() -> AggregateFixture {
        AggregateBuilder::new("Product")
            .with_id("product_in_stock")
            .with_field("id", "ProductId", true)
            .with_field("name", "String", true)
            .with_field("price", "Money", true)
            .with_field("quantity", "u32", true)
            .with_command("CreateProduct")
            .with_command("UpdateStock")
            .with_event("ProductCreated")
            .with_event("StockUpdated")
            .with_invariant("quantity > 0")
            .description("Product in stock")
            .tag("in_stock")
            .build()
    }

    pub fn out_of_stock() -> AggregateFixture {
        AggregateBuilder::new("Product")
            .with_id("product_out_of_stock")
            .with_field("id", "ProductId", true)
            .with_field("name", "String", true)
            .with_field("price", "Money", true)
            .with_field("quantity", "u32", true)
            .with_command("CreateProduct")
            .with_event("ProductCreated")
            .with_event("ProductOutOfStock")
            .with_invariant("quantity == 0")
            .description("Product out of stock")
            .tag("out_of_stock")
            .build()
    }
}

/// Payment aggregate fixtures
pub struct PaymentFixtures;

impl PaymentFixtures {
    pub fn pending() -> AggregateFixture {
        AggregateBuilder::new("Payment")
            .with_id("payment_pending")
            .with_field("id", "PaymentId", true)
            .with_field("order_id", "OrderId", true)
            .with_field("amount", "Money", true)
            .with_field("status", "PaymentStatus", true)
            .with_command("InitiatePayment")
            .with_event("PaymentInitiated")
            .description("Pending payment")
            .tag("pending")
            .build()
    }

    pub fn completed() -> AggregateFixture {
        AggregateBuilder::new("Payment")
            .with_id("payment_completed")
            .with_field("id", "PaymentId", true)
            .with_field("order_id", "OrderId", true)
            .with_field("amount", "Money", true)
            .with_field("status", "PaymentStatus", true)
            .with_field("completed_at", "DateTime", true)
            .with_command("CompletePayment")
            .with_event("PaymentCompleted")
            .description("Completed payment")
            .tag("completed")
            .build()
    }

    pub fn failed() -> AggregateFixture {
        AggregateBuilder::new("Payment")
            .with_id("payment_failed")
            .with_field("id", "PaymentId", true)
            .with_field("order_id", "OrderId", true)
            .with_field("amount", "Money", true)
            .with_field("status", "PaymentStatus", true)
            .with_field("failed_at", "DateTime", true)
            .with_field("failure_reason", "String", true)
            .with_command("FailPayment")
            .with_event("PaymentFailed")
            .description("Failed payment")
            .tag("failed")
            .build()
    }
}

/// Configuration fixtures
pub struct ConfigFixtures;

impl ConfigFixtures {
    pub fn minimal() -> ConfigFixture {
        ConfigBuilder::new()
            .workspace_root("/tmp/minimal")
            .description("Minimal configuration")
            .build()
    }

    pub fn complete() -> ConfigFixture {
        ConfigBuilder::new()
            .workspace_root("/tmp/complete")
            .cache_capacity(10)
            .with_recalc()
            .with_vba()
            .max_concurrent_recalcs(4)
            .tool_timeout_ms(60_000)
            .max_response_bytes(5_000_000)
            .description("Complete configuration with all features")
            .build()
    }

    pub fn development() -> ConfigFixture {
        ConfigBuilder::new()
            .workspace_root("/tmp/dev")
            .cache_capacity(3)
            .with_recalc()
            .max_concurrent_recalcs(2)
            .tool_timeout_ms(120_000)
            .description("Development configuration")
            .build()
    }

    pub fn production() -> ConfigFixture {
        ConfigBuilder::new()
            .workspace_root("/tmp/prod")
            .cache_capacity(20)
            .with_recalc()
            .max_concurrent_recalcs(8)
            .tool_timeout_ms(30_000)
            .max_response_bytes(1_000_000)
            .description("Production configuration")
            .build()
    }

    pub fn invalid_cache_too_small() -> ConfigFixture {
        ConfigBuilder::new()
            .workspace_root("/tmp/invalid")
            .cache_capacity(0)
            .description("Invalid config: cache too small")
            .invalid()
            .build()
    }

    pub fn invalid_timeout_too_low() -> ConfigFixture {
        ConfigBuilder::new()
            .workspace_root("/tmp/invalid")
            .tool_timeout_ms(50)
            .description("Invalid config: timeout too low")
            .invalid()
            .build()
    }
}

/// Ontology fixtures
pub struct OntologyFixtures;

impl OntologyFixtures {
    pub fn single_aggregate() -> OntologyFixture {
        OntologyBuilder::new()
            .add_aggregate("User")
            .add_command("CreateUser")
            .add_event("UserCreated")
            .description("Single aggregate ontology")
            .build()
    }

    pub fn complete_domain() -> OntologyFixture {
        OntologyBuilder::new()
            .add_aggregate("User")
            .add_aggregate("Order")
            .add_aggregate("Product")
            .add_aggregate("Payment")
            .add_value_object("Email")
            .add_value_object("Money")
            .add_value_object("Address")
            .add_command("CreateUser")
            .add_command("CreateOrder")
            .add_command("CreateProduct")
            .add_event("UserCreated")
            .add_event("OrderCreated")
            .add_event("ProductCreated")
            .description("Complete domain ontology")
            .build()
    }

    pub fn mcp_tools() -> OntologyFixture {
        OntologyBuilder::new()
            .prefix("mcp", "https://modelcontextprotocol.io/schema#")
            .add_aggregate("Tool")
            .add_aggregate("Resource")
            .add_aggregate("Prompt")
            .add_command("RegisterTool")
            .add_command("InvokeTool")
            .add_event("ToolRegistered")
            .add_event("ToolInvoked")
            .description("MCP tools ontology")
            .build()
    }

    pub fn ddd_patterns() -> OntologyFixture {
        OntologyBuilder::new()
            .add_aggregate("Aggregate")
            .add_value_object("ValueObject")
            .add_aggregate("Entity")
            .add_command("Command")
            .add_event("DomainEvent")
            .description("DDD patterns ontology")
            .build()
    }

    pub fn invalid_missing_type() -> OntologyFixture {
        OntologyBuilder::new()
            .add_triple("ex:InvalidEntity", "rdfs:label", "\"Invalid\"")
            .description("Invalid ontology: missing rdf:type")
            .invalid()
            .build()
    }

    pub fn invalid_cyclic_hierarchy() -> OntologyFixture {
        OntologyBuilder::new()
            .add_triple("ex:A", "rdfs:subClassOf", "ex:B")
            .add_triple("ex:B", "rdfs:subClassOf", "ex:C")
            .add_triple("ex:C", "rdfs:subClassOf", "ex:A")
            .description("Invalid ontology: cyclic class hierarchy")
            .invalid()
            .build()
    }
}

// =============================================================================
// Test Data Management
// =============================================================================

/// Isolated test workspace with automatic cleanup
pub struct TestWorkspace {
    _tempdir: TempDir,
    root: PathBuf,
}

impl TestWorkspace {
    pub fn new() -> Result<Self> {
        let tempdir = tempdir()?;
        let root = tempdir.path().to_path_buf();
        Ok(Self {
            _tempdir: tempdir,
            root,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    pub fn create_file(&self, name: &str, content: &str) -> Result<PathBuf> {
        let path = self.path(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, content)?;
        Ok(path)
    }

    pub fn copy_fixture(&self, fixture_path: &Path, name: &str) -> Result<PathBuf> {
        let dest = self.path(name);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(fixture_path, &dest)?;
        Ok(dest)
    }
}

impl Default for TestWorkspace {
    fn default() -> Self {
        Self::new().expect("Failed to create test workspace")
    }
}

// =============================================================================
// Fixture Utilities
// =============================================================================

/// Utility functions for working with fixtures
pub mod utils {
    use super::*;

    /// Load a fixture file from the fixtures directory
    pub fn load_fixture_file(category: &str, name: &str) -> Result<String> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(category)
            .join(name);

        std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to load fixture file: {:?}", path))
    }

    /// Validate fixture structure
    pub fn validate_fixture(fixture: &dyn Fixture) -> Result<()> {
        let metadata = fixture.metadata();

        anyhow::ensure!(!metadata.name.is_empty(), "Fixture name cannot be empty");
        anyhow::ensure!(
            !metadata.description.is_empty(),
            "Fixture description cannot be empty"
        );

        Ok(())
    }

    /// Compare two fixtures for equality (metadata only)
    pub fn fixtures_match(a: &dyn Fixture, b: &dyn Fixture) -> bool {
        let meta_a = a.metadata();
        let meta_b = b.metadata();

        meta_a.name == meta_b.name
            && meta_a.category == meta_b.category
            && meta_a.version == meta_b.version
    }
}

// =============================================================================
// Common Test Patterns
// =============================================================================

/// AAA (Arrange-Act-Assert) pattern helper
pub struct AAAPattern<T> {
    arranged: Option<T>,
    acted: Option<T>,
}

impl<T> AAAPattern<T> {
    pub fn new() -> Self {
        Self {
            arranged: None,
            acted: None,
        }
    }

    pub fn arrange(mut self, data: T) -> Self {
        self.arranged = Some(data);
        self
    }

    pub fn act<F>(mut self, f: F) -> Self
    where
        F: FnOnce(T) -> T,
    {
        if let Some(data) = self.arranged.take() {
            self.acted = Some(f(data));
        }
        self
    }

    pub fn assert<F>(self, f: F)
    where
        F: FnOnce(Option<T>),
    {
        f(self.acted);
    }
}

impl<T> Default for AAAPattern<T> {
    fn default() -> Self {
        Self::new()
    }
}
