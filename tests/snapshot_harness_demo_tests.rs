//! Comprehensive demonstration of snapshot testing harness
//!
//! This test file showcases all capabilities of the Chicago-style TDD
//! snapshot testing harness, including:
//! - Code generation snapshots
//! - Template rendering snapshots
//! - SPARQL query result snapshots
//! - Configuration snapshots
//! - Multi-format support
//! - Update workflows

mod harness;

use harness::{SnapshotFormat, SnapshotTestHarness, UpdateMode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Test Setup and Utilities
// ============================================================================

/// Sample domain entity for code generation
#[derive(Debug, Serialize, Deserialize)]
struct UserAggregate {
    id: String,
    name: String,
    email: String,
    roles: Vec<String>,
}

/// Sample template context
#[derive(Debug, Serialize, Deserialize)]
struct TemplateContext {
    entity_name: String,
    fields: Vec<FieldDefinition>,
    has_validation: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct FieldDefinition {
    name: String,
    field_type: String,
    required: bool,
}

/// Sample SPARQL query result
#[derive(Debug, Serialize, Deserialize)]
struct SparqlResult {
    head: SparqlHead,
    results: SparqlBindings,
}

#[derive(Debug, Serialize, Deserialize)]
struct SparqlHead {
    vars: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SparqlBindings {
    bindings: Vec<HashMap<String, SparqlValue>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SparqlValue {
    #[serde(rename = "type")]
    value_type: String,
    value: String,
}

// ============================================================================
// Code Generation Snapshot Tests
// ============================================================================

#[test]
fn test_user_aggregate_code_generation() {
    let mut harness = SnapshotTestHarness::new();

    // Generate user aggregate code
    let generated_code = generate_user_aggregate_code();

    // Assert snapshot matches
    let result = harness.assert_snapshot(
        "codegen",
        "UserAggregate",
        generated_code,
        SnapshotFormat::Rust,
    );

    match result {
        Ok(_) => println!("✓ UserAggregate snapshot matches"),
        Err(e) => {
            if let Some(diff) = e.diff {
                println!("✗ UserAggregate snapshot differs:");
                harness.print_diff(&diff);
            }
            panic!("Snapshot mismatch: {}", e);
        }
    }
}

#[test]
fn test_mcp_tool_handler_generation() {
    let mut harness = SnapshotTestHarness::new();

    let generated_code = generate_mcp_tool_handler();

    harness
        .assert_snapshot(
            "codegen",
            "MCPToolHandler",
            generated_code,
            SnapshotFormat::Rust,
        )
        .expect("MCP tool handler snapshot should match");
}

#[test]
fn test_command_handler_generation() {
    let mut harness = SnapshotTestHarness::new();

    let generated_code = generate_command_handler();

    harness
        .assert_snapshot(
            "codegen",
            "CommandHandler",
            generated_code,
            SnapshotFormat::Rust,
        )
        .expect("Command handler snapshot should match");
}

#[test]
fn test_value_object_generation() {
    let mut harness = SnapshotTestHarness::new();

    let generated_code = generate_value_object_code();

    harness
        .assert_snapshot(
            "codegen",
            "EmailValueObject",
            generated_code,
            SnapshotFormat::Rust,
        )
        .expect("Value object snapshot should match");
}

#[test]
fn test_repository_implementation() {
    let mut harness = SnapshotTestHarness::new();

    let generated_code = generate_repository_code();

    harness
        .assert_snapshot(
            "codegen",
            "UserRepository",
            generated_code,
            SnapshotFormat::Rust,
        )
        .expect("Repository snapshot should match");
}

#[test]
fn test_service_implementation() {
    let mut harness = SnapshotTestHarness::new();

    let generated_code = generate_service_code();

    harness
        .assert_snapshot(
            "codegen",
            "UserService",
            generated_code,
            SnapshotFormat::Rust,
        )
        .expect("Service snapshot should match");
}

// ============================================================================
// Template Rendering Snapshot Tests
// ============================================================================

#[test]
fn test_domain_entity_template() {
    let mut harness = SnapshotTestHarness::new();

    let context = TemplateContext {
        entity_name: "Product".to_string(),
        fields: vec![
            FieldDefinition {
                name: "id".to_string(),
                field_type: "Uuid".to_string(),
                required: true,
            },
            FieldDefinition {
                name: "name".to_string(),
                field_type: "String".to_string(),
                required: true,
            },
            FieldDefinition {
                name: "price".to_string(),
                field_type: "Decimal".to_string(),
                required: true,
            },
        ],
        has_validation: true,
    };

    let rendered = render_domain_entity_template(&context);

    harness
        .assert_snapshot(
            "templates",
            "domain_entity_product",
            rendered,
            SnapshotFormat::Rust,
        )
        .expect("Domain entity template snapshot should match");
}

#[test]
fn test_template_with_various_contexts() {
    let mut harness = SnapshotTestHarness::new();

    // Test with minimal context
    let minimal_context = TemplateContext {
        entity_name: "Simple".to_string(),
        fields: vec![],
        has_validation: false,
    };

    let rendered = render_domain_entity_template(&minimal_context);

    harness
        .assert_snapshot(
            "templates",
            "domain_entity_minimal",
            rendered,
            SnapshotFormat::Rust,
        )
        .expect("Minimal template snapshot should match");

    // Test with edge case: many fields
    let many_fields_context = TemplateContext {
        entity_name: "Complex".to_string(),
        fields: (0..20)
            .map(|i| FieldDefinition {
                name: format!("field_{}", i),
                field_type: "String".to_string(),
                required: i % 2 == 0,
            })
            .collect(),
        has_validation: true,
    };

    let rendered = render_domain_entity_template(&many_fields_context);

    harness
        .assert_snapshot(
            "templates",
            "domain_entity_complex",
            rendered,
            SnapshotFormat::Rust,
        )
        .expect("Complex template snapshot should match");
}

// ============================================================================
// SPARQL Query Result Snapshot Tests
// ============================================================================

#[test]
fn test_sparql_aggregate_query_results() {
    let mut harness = SnapshotTestHarness::new();

    let results = create_sample_sparql_results();
    let json_str = serde_json::to_string_pretty(&results).unwrap();

    harness
        .assert_snapshot("sparql", "aggregates_query", json_str, SnapshotFormat::Json)
        .expect("SPARQL aggregates query snapshot should match");
}

#[test]
fn test_sparql_binding_structures() {
    let mut harness = SnapshotTestHarness::new();

    let bindings = create_complex_bindings();
    let json_str = serde_json::to_string_pretty(&bindings).unwrap();

    harness
        .assert_snapshot("sparql", "complex_bindings", json_str, SnapshotFormat::Json)
        .expect("SPARQL bindings snapshot should match");
}

#[test]
fn test_sparql_graph_patterns() {
    let mut harness = SnapshotTestHarness::new();

    let graph_pattern = create_graph_pattern();

    harness
        .assert_snapshot(
            "sparql",
            "graph_pattern",
            graph_pattern,
            SnapshotFormat::Text,
        )
        .expect("Graph pattern snapshot should match");
}

// ============================================================================
// Configuration Snapshot Tests
// ============================================================================

#[test]
fn test_complete_config_serialization() {
    let mut harness = SnapshotTestHarness::new();

    let config = create_complete_config();

    harness
        .assert_snapshot("config", "complete_config", config, SnapshotFormat::Toml)
        .expect("Complete config snapshot should match");
}

#[test]
fn test_validation_report_snapshot() {
    let mut harness = SnapshotTestHarness::new();

    let validation_report = create_validation_report();
    let json_str = serde_json::to_string_pretty(&validation_report).unwrap();

    harness
        .assert_snapshot(
            "config",
            "validation_report",
            json_str,
            SnapshotFormat::Json,
        )
        .expect("Validation report snapshot should match");
}

#[test]
fn test_error_messages_snapshot() {
    let mut harness = SnapshotTestHarness::new();

    let error_messages = create_error_messages();

    harness
        .assert_snapshot(
            "config",
            "error_messages",
            error_messages,
            SnapshotFormat::Text,
        )
        .expect("Error messages snapshot should match");
}

// ============================================================================
// Debug Output Snapshot Tests
// ============================================================================

#[test]
fn test_domain_model_debug_output() {
    let mut harness = SnapshotTestHarness::new();

    let user = UserAggregate {
        id: "user-123".to_string(),
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        roles: vec!["admin".to_string(), "user".to_string()],
    };

    let debug_output = format!("{:#?}", user);

    harness
        .assert_snapshot(
            "misc",
            "user_aggregate_debug",
            debug_output,
            SnapshotFormat::Debug,
        )
        .expect("User aggregate debug snapshot should match");
}

// ============================================================================
// Snapshot Statistics and Reporting
// ============================================================================

#[test]
fn test_snapshot_statistics() {
    let mut harness = SnapshotTestHarness::new();

    // Run multiple snapshot assertions
    let _ = harness.assert_snapshot("misc", "stat_test_1", "content1", SnapshotFormat::Text);
    let _ = harness.assert_snapshot("misc", "stat_test_2", "content2", SnapshotFormat::Text);

    let stats = harness.stats();
    println!("Snapshot Statistics:");
    println!("  Total: {}", stats.total);
    println!("  Matched: {}", stats.matched);
    println!("  Created: {}", stats.created);
    println!("  Updated: {}", stats.updated);
    println!("  Failed: {}", stats.failed);

    assert!(stats.total >= 2);
}

#[test]
fn test_snapshot_report_generation() {
    let mut harness = SnapshotTestHarness::new();

    // Create some snapshots
    let _ = harness.assert_snapshot(
        "codegen",
        "report_test_1",
        "code content",
        SnapshotFormat::Rust,
    );
    let _ = harness.assert_snapshot(
        "templates",
        "report_test_2",
        r#"{"template": "data"}"#,
        SnapshotFormat::Json,
    );

    let report = harness.generate_report();
    println!("{}", report);

    assert!(report.total_snapshots > 0);
}

// ============================================================================
// Update Mode Tests
// ============================================================================

#[test]
#[ignore] // Run with UPDATE_SNAPSHOTS=1 to enable
fn test_update_mode_always() {
    let mut harness = SnapshotTestHarness::new();
    harness.update_mode = UpdateMode::Always;

    // This will update the snapshot
    harness
        .assert_snapshot("misc", "update_test", "new content", SnapshotFormat::Text)
        .expect("Should succeed in update mode");

    let stats = harness.stats();
    assert!(stats.created > 0 || stats.updated > 0);
}

// ============================================================================
// Helper Functions (Code Generation Stubs)
// ============================================================================

fn generate_user_aggregate_code() -> String {
    r#"use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User aggregate root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAggregate {
    pub id: Uuid,
    pub name: String,
    pub email: Email,
    pub roles: Vec<Role>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl UserAggregate {
    pub fn new(name: String, email: Email) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            email,
            roles: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_role(&mut self, role: Role) {
        if !self.roles.contains(&role) {
            self.roles.push(role);
            self.updated_at = chrono::Utc::now();
        }
    }

    pub fn remove_role(&mut self, role: &Role) {
        if let Some(pos) = self.roles.iter().position(|r| r == role) {
            self.roles.remove(pos);
            self.updated_at = chrono::Utc::now();
        }
    }
}
"#
    .to_string()
}

fn generate_mcp_tool_handler() -> String {
    r#"use rmcp::protocol::Tool;
use serde_json::Value;

/// MCP tool handler for user operations
pub struct UserToolHandler {
    repository: Arc<dyn UserRepository>,
}

impl UserToolHandler {
    pub fn new(repository: Arc<dyn UserRepository>) -> Self {
        Self { repository }
    }

    pub async fn handle_create_user(&self, params: Value) -> Result<Value, ToolError> {
        let request: CreateUserRequest = serde_json::from_value(params)?;

        let email = Email::try_from(request.email)?;
        let user = UserAggregate::new(request.name, email);

        self.repository.save(user).await?;

        Ok(serde_json::to_value(&user)?)
    }

    pub async fn handle_get_user(&self, params: Value) -> Result<Value, ToolError> {
        let request: GetUserRequest = serde_json::from_value(params)?;

        let user = self.repository.find_by_id(&request.id).await?
            .ok_or(ToolError::NotFound)?;

        Ok(serde_json::to_value(&user)?)
    }
}
"#
    .to_string()
}

fn generate_command_handler() -> String {
    r#"use async_trait::async_trait;

/// Command for creating a user
pub struct CreateUserCommand {
    pub name: String,
    pub email: String,
    pub initial_roles: Vec<String>,
}

/// Handler for CreateUserCommand
pub struct CreateUserCommandHandler {
    repository: Arc<dyn UserRepository>,
    event_bus: Arc<dyn EventBus>,
}

#[async_trait]
impl CommandHandler<CreateUserCommand> for CreateUserCommandHandler {
    type Result = UserAggregate;
    type Error = DomainError;

    async fn handle(&self, command: CreateUserCommand) -> Result<Self::Result, Self::Error> {
        // Validate email
        let email = Email::try_from(command.email)?;

        // Create aggregate
        let mut user = UserAggregate::new(command.name, email);

        // Add initial roles
        for role_name in command.initial_roles {
            let role = Role::try_from(role_name)?;
            user.add_role(role);
        }

        // Save
        self.repository.save(user.clone()).await?;

        // Publish event
        self.event_bus.publish(UserCreatedEvent {
            user_id: user.id,
            email: user.email.clone(),
        }).await?;

        Ok(user)
    }
}
"#
    .to_string()
}

fn generate_value_object_code() -> String {
    r#"use std::fmt;
use serde::{Deserialize, Serialize};

/// Email value object with validation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Email(String);

impl Email {
    const MAX_LENGTH: usize = 254;

    pub fn new(value: impl Into<String>) -> Result<Self, EmailError> {
        let value = value.into();
        Self::validate(&value)?;
        Ok(Self(value))
    }

    fn validate(value: &str) -> Result<(), EmailError> {
        if value.is_empty() {
            return Err(EmailError::Empty);
        }

        if value.len() > Self::MAX_LENGTH {
            return Err(EmailError::TooLong);
        }

        if !value.contains('@') {
            return Err(EmailError::InvalidFormat);
        }

        let parts: Vec<&str> = value.split('@').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(EmailError::InvalidFormat);
        }

        Ok(())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Email {
    type Error = EmailError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
"#
    .to_string()
}

fn generate_repository_code() -> String {
    r#"use async_trait::async_trait;
use uuid::Uuid;

/// User repository trait
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn save(&self, user: UserAggregate) -> Result<(), RepositoryError>;
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<UserAggregate>, RepositoryError>;
    async fn find_by_email(&self, email: &Email) -> Result<Option<UserAggregate>, RepositoryError>;
    async fn delete(&self, id: &Uuid) -> Result<(), RepositoryError>;
}

/// In-memory implementation for testing
pub struct InMemoryUserRepository {
    users: Arc<RwLock<HashMap<Uuid, UserAggregate>>>,
}

#[async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn save(&self, user: UserAggregate) -> Result<(), RepositoryError> {
        let mut users = self.users.write().await;
        users.insert(user.id, user);
        Ok(())
    }

    async fn find_by_id(&self, id: &Uuid) -> Result<Option<UserAggregate>, RepositoryError> {
        let users = self.users.read().await;
        Ok(users.get(id).cloned())
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<UserAggregate>, RepositoryError> {
        let users = self.users.read().await;
        Ok(users.values().find(|u| &u.email == email).cloned())
    }

    async fn delete(&self, id: &Uuid) -> Result<(), RepositoryError> {
        let mut users = self.users.write().await;
        users.remove(id);
        Ok(())
    }
}
"#
    .to_string()
}

fn generate_service_code() -> String {
    r#"use std::sync::Arc;

/// User domain service
pub struct UserService {
    repository: Arc<dyn UserRepository>,
    email_service: Arc<dyn EmailService>,
}

impl UserService {
    pub fn new(
        repository: Arc<dyn UserRepository>,
        email_service: Arc<dyn EmailService>,
    ) -> Self {
        Self {
            repository,
            email_service,
        }
    }

    pub async fn register_user(
        &self,
        name: String,
        email: String,
    ) -> Result<UserAggregate, ServiceError> {
        // Validate email format
        let email = Email::try_from(email)?;

        // Check if email already exists
        if self.repository.find_by_email(&email).await?.is_some() {
            return Err(ServiceError::EmailAlreadyExists);
        }

        // Create user
        let user = UserAggregate::new(name, email.clone());

        // Save to repository
        self.repository.save(user.clone()).await?;

        // Send welcome email
        self.email_service.send_welcome_email(&email).await?;

        Ok(user)
    }

    pub async fn update_user_roles(
        &self,
        user_id: &Uuid,
        roles: Vec<Role>,
    ) -> Result<UserAggregate, ServiceError> {
        let mut user = self.repository.find_by_id(user_id).await?
            .ok_or(ServiceError::UserNotFound)?;

        // Update roles
        user.roles = roles;
        user.updated_at = chrono::Utc::now();

        // Save
        self.repository.save(user.clone()).await?;

        Ok(user)
    }
}
"#
    .to_string()
}

fn render_domain_entity_template(context: &TemplateContext) -> String {
    let mut output = format!(
        r#"// Generated domain entity: {}

use serde::{{Deserialize, Serialize}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {} {{
"#,
        context.entity_name, context.entity_name
    );

    for field in &context.fields {
        let required_marker = if field.required { "" } else { "Option<" };
        let required_end = if field.required { "" } else { ">" };
        output.push_str(&format!(
            "    pub {}: {}{}{},\n",
            field.name, required_marker, field.field_type, required_end
        ));
    }

    output.push_str("}\n");

    if context.has_validation {
        output.push_str(&format!(
            r#"
impl {} {{
    pub fn validate(&self) -> Result<(), ValidationError> {{
        // Validation logic here
        Ok(())
    }}
}}
"#,
            context.entity_name
        ));
    }

    output
}

fn create_sample_sparql_results() -> SparqlResult {
    let mut binding1 = HashMap::new();
    binding1.insert(
        "aggregate".to_string(),
        SparqlValue {
            value_type: "uri".to_string(),
            value: "http://example.org/User".to_string(),
        },
    );
    binding1.insert(
        "name".to_string(),
        SparqlValue {
            value_type: "literal".to_string(),
            value: "User".to_string(),
        },
    );

    let mut binding2 = HashMap::new();
    binding2.insert(
        "aggregate".to_string(),
        SparqlValue {
            value_type: "uri".to_string(),
            value: "http://example.org/Product".to_string(),
        },
    );
    binding2.insert(
        "name".to_string(),
        SparqlValue {
            value_type: "literal".to_string(),
            value: "Product".to_string(),
        },
    );

    SparqlResult {
        head: SparqlHead {
            vars: vec!["aggregate".to_string(), "name".to_string()],
        },
        results: SparqlBindings {
            bindings: vec![binding1, binding2],
        },
    }
}

fn create_complex_bindings() -> SparqlBindings {
    let mut bindings = Vec::new();

    for i in 0..5 {
        let mut binding = HashMap::new();
        binding.insert(
            "id".to_string(),
            SparqlValue {
                value_type: "literal".to_string(),
                value: format!("id-{}", i),
            },
        );
        binding.insert(
            "value".to_string(),
            SparqlValue {
                value_type: "literal".to_string(),
                value: format!("value-{}", i),
            },
        );
        bindings.push(binding);
    }

    SparqlBindings { bindings }
}

fn create_graph_pattern() -> String {
    r#"PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX ggen: <http://example.org/ggen#>

SELECT ?aggregate ?name ?description
WHERE {
    ?aggregate rdf:type ggen:Aggregate .
    ?aggregate ggen:name ?name .
    OPTIONAL { ?aggregate ggen:description ?description }
}
ORDER BY ?name
LIMIT 100"#
        .to_string()
}

fn create_complete_config() -> String {
    r#"[server]
host = "127.0.0.1"
port = 3000
workers = 4

[database]
url = "postgresql://localhost/ggen"
pool_size = 10
timeout_seconds = 30

[ontology]
file = "ggen-mcp.ttl"
namespace = "http://example.org/ggen#"

[codegen]
output_dir = "generated"
template_dir = "templates"
overwrite = false

[validation]
enabled = true
strict_mode = true
max_errors = 100
"#
    .to_string()
}

#[derive(Debug, Serialize)]
struct ValidationReport {
    status: String,
    errors: Vec<ValidationIssue>,
    warnings: Vec<ValidationIssue>,
    timestamp: String,
}

#[derive(Debug, Serialize)]
struct ValidationIssue {
    severity: String,
    message: String,
    location: String,
}

fn create_validation_report() -> ValidationReport {
    ValidationReport {
        status: "failed".to_string(),
        errors: vec![
            ValidationIssue {
                severity: "error".to_string(),
                message: "Missing required field 'name'".to_string(),
                location: "User.name".to_string(),
            },
            ValidationIssue {
                severity: "error".to_string(),
                message: "Invalid email format".to_string(),
                location: "User.email".to_string(),
            },
        ],
        warnings: vec![ValidationIssue {
            severity: "warning".to_string(),
            message: "Field 'description' is recommended".to_string(),
            location: "User.description".to_string(),
        }],
        timestamp: "2024-01-20T10:30:00Z".to_string(),
    }
}

fn create_error_messages() -> String {
    r#"Error Messages Report
=====================

Template Rendering Errors:
- Template 'user_entity' not found
- Undefined variable 'entity_name' in template 'domain'
- Syntax error in template 'repository': unexpected token '}}}'

SPARQL Query Errors:
- Query timeout after 30 seconds
- Invalid SPARQL syntax: missing WHERE clause
- Unknown prefix 'custom' in query

Code Generation Errors:
- Failed to parse Rust code: expected `;` at line 42
- Duplicate entity name 'User'
- Invalid field type 'UnknownType'

Configuration Errors:
- Missing required configuration key 'server.port'
- Invalid TOML syntax at line 15
- Configuration file not found: 'config.toml'
"#
    .to_string()
}
