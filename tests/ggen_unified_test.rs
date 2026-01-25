//! Integration tests for unified ggen resource management tool
//!
//! Tests all 15 operations through single dispatch point.
//! Verifies: delegation correctness, error handling, response structure.

use ggen_mcp::state::AppState;
use ggen_mcp::tools::ggen_config::{GenerationMode, GenerationRule};
use ggen_mcp::tools::ggen_unified::{
    ManageGgenResourceParams, ResourceOperation, manage_ggen_resource,
};
use ggen_mcp::tools::tera_authoring;
use ggen_mcp::tools::turtle_authoring::{EntityName, EntityType, PropertyName, PropertySpec};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

// ============================================================================
// Test Fixtures
// ============================================================================

fn create_test_state(workspace: &TempDir) -> Arc<AppState> {
    let config = ggen_mcp::config::Config {
        workspace_root: workspace.path().to_path_buf(),
        ..Default::default()
    };
    Arc::new(AppState::new(config))
}

fn create_sample_ggen_toml(path: &PathBuf) {
    let content = r#"
[ontology]
file = "ontology/mcp-domain.ttl"

[[generation.rules]]
name = "test-rule"
description = "Test rule"
query = { file = "queries/test.rq" }
template = { file = "templates/test.tera" }
output_file = "src/generated/test.rs"
mode = "Overwrite"
"#;
    fs::write(path, content).expect("Failed to write test ggen.toml");
}

fn create_sample_turtle(path: &PathBuf) {
    let content = r#"
@prefix mcp: <http://ggen-mcp.dev/ontology/mcp#> .
@prefix ddd: <http://ggen-mcp.dev/ontology/ddd#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

mcp:TestEntity a ddd:Entity ;
    rdfs:label "Test Entity" .
"#;
    fs::create_dir_all(path.parent().unwrap()).expect("Failed to create ontology dir");
    fs::write(path, content).expect("Failed to write test Turtle");
}

// ============================================================================
// Config Operations Tests (5)
// ============================================================================

#[tokio::test]
async fn test_read_config_operation() {
    let temp_dir = TempDir::new().unwrap();
    let state = create_test_state(&temp_dir);
    let config_path = temp_dir.path().join("ggen.toml");
    create_sample_ggen_toml(&config_path);

    let params = ManageGgenResourceParams {
        operation: ResourceOperation::ReadConfig {
            config_path: Some("ggen.toml".to_string()),
        },
    };

    let result = manage_ggen_resource(state, params).await;
    assert!(result.is_ok(), "read_config should succeed");

    let response = result.unwrap();
    assert_eq!(response.operation, "read_config");
    assert_eq!(response.metadata.category, "config");
    assert!(response.metadata.success);

    // Verify result structure
    let config = &response.result;
    assert!(config["config"].is_object());
    assert!(config["rule_count"].is_number());
    assert_eq!(config["rule_count"], 1);
}

#[tokio::test]
async fn test_validate_config_operation() {
    let temp_dir = TempDir::new().unwrap();
    let state = create_test_state(&temp_dir);
    let config_path = temp_dir.path().join("ggen.toml");
    create_sample_ggen_toml(&config_path);

    let params = ManageGgenResourceParams {
        operation: ResourceOperation::ValidateConfig {
            config_path: Some("ggen.toml".to_string()),
            check_file_refs: false, // Skip file existence checks
            check_circular_deps: true,
            check_path_overlaps: true,
        },
    };

    let result = manage_ggen_resource(state, params).await;
    assert!(result.is_ok(), "validate_config should succeed");

    let response = result.unwrap();
    assert_eq!(response.operation, "validate_config");
    assert_eq!(response.metadata.category, "config");

    let validation = &response.result;
    assert!(validation["rule_count"].is_number());
}

#[tokio::test]
async fn test_add_rule_operation() {
    let temp_dir = TempDir::new().unwrap();
    let state = create_test_state(&temp_dir);
    let config_path = temp_dir.path().join("ggen.toml");
    create_sample_ggen_toml(&config_path);

    let new_rule = GenerationRule {
        name: "new-rule".to_string(),
        description: "New test rule".to_string(),
        query_file: "queries/new.rq".to_string(),
        template_file: "templates/new.tera".to_string(),
        output_file: "src/generated/new.rs".to_string(),
        mode: GenerationMode::Overwrite,
    };

    let params = ManageGgenResourceParams {
        operation: ResourceOperation::AddRule {
            config_path: Some("ggen.toml".to_string()),
            rule: new_rule,
            create_backup: true,
        },
    };

    let result = manage_ggen_resource(state, params).await;
    assert!(result.is_ok(), "add_rule should succeed");

    let response = result.unwrap();
    assert_eq!(response.operation, "add_rule");
    assert_eq!(response.metadata.category, "config");
    assert!(response.result["success"].as_bool().unwrap());
    assert_eq!(response.result["rule_count"], 2);
}

#[tokio::test]
async fn test_update_rule_operation() {
    let temp_dir = TempDir::new().unwrap();
    let state = create_test_state(&temp_dir);
    let config_path = temp_dir.path().join("ggen.toml");
    create_sample_ggen_toml(&config_path);

    let updated_rule = GenerationRule {
        name: "test-rule".to_string(),
        description: "Updated description".to_string(),
        query_file: "queries/test_updated.rq".to_string(),
        template_file: "templates/test_updated.tera".to_string(),
        output_file: "src/generated/test_updated.rs".to_string(),
        mode: GenerationMode::Append,
    };

    let params = ManageGgenResourceParams {
        operation: ResourceOperation::UpdateRule {
            config_path: Some("ggen.toml".to_string()),
            rule_name: "test-rule".to_string(),
            rule: updated_rule,
            create_backup: true,
        },
    };

    let result = manage_ggen_resource(state, params).await;
    assert!(result.is_ok(), "update_rule should succeed");

    let response = result.unwrap();
    assert_eq!(response.operation, "update_rule");
    assert_eq!(response.metadata.category, "config");
    assert!(response.result["success"].as_bool().unwrap());
}

#[tokio::test]
async fn test_remove_rule_operation() {
    let temp_dir = TempDir::new().unwrap();
    let state = create_test_state(&temp_dir);
    let config_path = temp_dir.path().join("ggen.toml");
    create_sample_ggen_toml(&config_path);

    let params = ManageGgenResourceParams {
        operation: ResourceOperation::RemoveRule {
            config_path: Some("ggen.toml".to_string()),
            rule_name: "test-rule".to_string(),
            create_backup: true,
        },
    };

    let result = manage_ggen_resource(state, params).await;
    assert!(result.is_ok(), "remove_rule should succeed");

    let response = result.unwrap();
    assert_eq!(response.operation, "remove_rule");
    assert_eq!(response.metadata.category, "config");
    assert!(response.result["success"].as_bool().unwrap());
    assert_eq!(response.result["rule_count"], 0);
}

// ============================================================================
// Ontology Operations Tests (5)
// ============================================================================

#[tokio::test]
async fn test_read_ontology_operation() {
    let temp_dir = TempDir::new().unwrap();
    let state = create_test_state(&temp_dir);
    let ontology_path = temp_dir.path().join("ontology/test.ttl");
    create_sample_turtle(&ontology_path);

    let params = ManageGgenResourceParams {
        operation: ResourceOperation::ReadOntology {
            path: "ontology/test.ttl".to_string(),
            include_entities: true,
            include_prefixes: true,
        },
    };

    let result = manage_ggen_resource(state, params).await;
    assert!(result.is_ok(), "read_ontology should succeed");

    let response = result.unwrap();
    assert_eq!(response.operation, "read_ontology");
    assert_eq!(response.metadata.category, "ontology");
    assert!(response.result["triple_count"].is_number());
}

#[tokio::test]
async fn test_add_entity_operation() {
    let temp_dir = TempDir::new().unwrap();
    let state = create_test_state(&temp_dir);
    let ontology_path = temp_dir.path().join("ontology/test.ttl");
    create_sample_turtle(&ontology_path);

    let params = ManageGgenResourceParams {
        operation: ResourceOperation::AddEntity {
            path: "ontology/test.ttl".to_string(),
            entity_name: EntityName::new("NewEntity".to_string()).unwrap(),
            entity_type: EntityType::Entity,
            properties: vec![PropertySpec {
                name: PropertyName::new("testProp".to_string()).unwrap(),
                rust_type: "String".to_string(),
                required: true,
                description: Some("Test property".to_string()),
            }],
            label: Some("New Entity".to_string()),
            comment: Some("A test entity".to_string()),
            create_backup: true,
            validate_syntax: true,
        },
    };

    let result = manage_ggen_resource(state, params).await;
    assert!(result.is_ok(), "add_entity should succeed");

    let response = result.unwrap();
    assert_eq!(response.operation, "add_entity");
    assert_eq!(response.metadata.category, "ontology");
    assert!(response.result["entity_iri"].is_string());
}

#[tokio::test]
async fn test_add_property_operation() {
    let temp_dir = TempDir::new().unwrap();
    let state = create_test_state(&temp_dir);
    let ontology_path = temp_dir.path().join("ontology/test.ttl");
    create_sample_turtle(&ontology_path);

    let params = ManageGgenResourceParams {
        operation: ResourceOperation::AddProperty {
            path: "ontology/test.ttl".to_string(),
            entity_name: EntityName::new("TestEntity".to_string()).unwrap(),
            property: PropertySpec {
                name: PropertyName::new("newProp".to_string()).unwrap(),
                rust_type: "i32".to_string(),
                required: false,
                description: Some("New property".to_string()),
            },
            create_backup: true,
            validate_syntax: true,
        },
    };

    let result = manage_ggen_resource(state, params).await;
    assert!(result.is_ok(), "add_property should succeed");

    let response = result.unwrap();
    assert_eq!(response.operation, "add_property");
    assert_eq!(response.metadata.category, "ontology");
}

#[tokio::test]
async fn test_validate_ontology_operation() {
    let temp_dir = TempDir::new().unwrap();
    let state = create_test_state(&temp_dir);
    let ontology_path = temp_dir.path().join("ontology/test.ttl");
    create_sample_turtle(&ontology_path);

    let params = ManageGgenResourceParams {
        operation: ResourceOperation::ValidateOntology {
            path: "ontology/test.ttl".to_string(),
            shacl_validation: false,
            strict_mode: false,
        },
    };

    let result = manage_ggen_resource(state, params).await;
    assert!(result.is_ok(), "validate_ontology should succeed");

    let response = result.unwrap();
    assert_eq!(response.operation, "validate_ontology");
    assert_eq!(response.metadata.category, "ontology");
    assert!(response.result["is_valid"].is_boolean());
}

#[tokio::test]
async fn test_query_entities_operation() {
    let temp_dir = TempDir::new().unwrap();
    let state = create_test_state(&temp_dir);
    let ontology_path = temp_dir.path().join("ontology/test.ttl");
    create_sample_turtle(&ontology_path);

    let params = ManageGgenResourceParams {
        operation: ResourceOperation::QueryEntities {
            path: "ontology/test.ttl".to_string(),
            entity_type_filter: Some(EntityType::Entity),
            include_properties: true,
        },
    };

    let result = manage_ggen_resource(state, params).await;
    assert!(result.is_ok(), "query_entities should succeed");

    let response = result.unwrap();
    assert_eq!(response.operation, "query_entities");
    assert_eq!(response.metadata.category, "ontology");
    assert!(response.result["entities"].is_array());
}

// ============================================================================
// Template Operations Tests (5)
// ============================================================================

#[tokio::test]
async fn test_read_template_operation() {
    let temp_dir = TempDir::new().unwrap();
    let state = create_test_state(&temp_dir);

    let params = ManageGgenResourceParams {
        operation: ResourceOperation::ReadTemplate {
            template: "inline:{{ name }} is {{ age }}".to_string(),
            analyze_variables: true,
            analyze_filters: true,
            analyze_structures: true,
        },
    };

    let result = manage_ggen_resource(state, params).await;
    assert!(result.is_ok(), "read_template should succeed");

    let response = result.unwrap();
    assert_eq!(response.operation, "read_template");
    assert_eq!(response.metadata.category, "template");
    assert!(response.result["variables"].is_array());
    assert!(response.result["content"].is_string());
}

#[tokio::test]
async fn test_validate_template_operation() {
    let temp_dir = TempDir::new().unwrap();
    let state = create_test_state(&temp_dir);

    let params = ManageGgenResourceParams {
        operation: ResourceOperation::ValidateTemplate {
            template: "inline:{% if condition %}valid{% endif %}".to_string(),
            check_variables: true,
            check_filters: true,
            check_blocks: true,
        },
    };

    let result = manage_ggen_resource(state, params).await;
    assert!(result.is_ok(), "validate_template should succeed");

    let response = result.unwrap();
    assert_eq!(response.operation, "validate_template");
    assert_eq!(response.metadata.category, "template");
    assert!(response.result["valid"].is_boolean());
}

#[tokio::test]
async fn test_test_template_operation() {
    let temp_dir = TempDir::new().unwrap();
    let state = create_test_state(&temp_dir);

    let params = ManageGgenResourceParams {
        operation: ResourceOperation::TestTemplate {
            template: "inline:Hello {{ name }}!".to_string(),
            context: json!({"name": "World"}),
            timeout_ms: Some(5000),
            show_metrics: true,
        },
    };

    let result = manage_ggen_resource(state, params).await;
    assert!(result.is_ok(), "test_template should succeed");

    let response = result.unwrap();
    assert_eq!(response.operation, "test_template");
    assert_eq!(response.metadata.category, "template");
    assert_eq!(response.result["output"], "Hello World!");
    assert!(response.result["success"].as_bool().unwrap());
}

#[tokio::test]
async fn test_create_template_operation() {
    let temp_dir = TempDir::new().unwrap();
    let state = create_test_state(&temp_dir);

    let params = ManageGgenResourceParams {
        operation: ResourceOperation::CreateTemplate {
            pattern: "struct".to_string(),
            variables: json!({}),
            output_name: Some("my_struct.rs.tera".to_string()),
        },
    };

    let result = manage_ggen_resource(state, params).await;
    assert!(result.is_ok(), "create_template should succeed");

    let response = result.unwrap();
    assert_eq!(response.operation, "create_template");
    assert_eq!(response.metadata.category, "template");
    assert!(response.result["template"].is_string());
    assert!(
        response.result["template"]
            .as_str()
            .unwrap()
            .contains("struct")
    );
}

#[tokio::test]
async fn test_list_template_vars_operation() {
    let temp_dir = TempDir::new().unwrap();
    let state = create_test_state(&temp_dir);

    let params = ManageGgenResourceParams {
        operation: ResourceOperation::ListTemplateVars {
            template: "inline:{{ name | upper }} {{ age }}".to_string(),
            include_filters: true,
            include_type_hints: false,
        },
    };

    let result = manage_ggen_resource(state, params).await;
    assert!(result.is_ok(), "list_template_vars should succeed");

    let response = result.unwrap();
    assert_eq!(response.operation, "list_template_vars");
    assert_eq!(response.metadata.category, "template");
    assert!(response.result["variables"].is_array());
    assert_eq!(response.result["count"], 2);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_error_handling_missing_file() {
    let temp_dir = TempDir::new().unwrap();
    let state = create_test_state(&temp_dir);

    let params = ManageGgenResourceParams {
        operation: ResourceOperation::ReadConfig {
            config_path: Some("nonexistent.toml".to_string()),
        },
    };

    let result = manage_ggen_resource(state, params).await;
    assert!(result.is_err(), "should fail with missing file");

    let error = result.unwrap_err();
    assert!(error.to_string().contains("read_config operation failed"));
}

#[tokio::test]
async fn test_error_handling_invalid_operation() {
    let temp_dir = TempDir::new().unwrap();
    let state = create_test_state(&temp_dir);
    let config_path = temp_dir.path().join("ggen.toml");
    create_sample_ggen_toml(&config_path);

    // Try to remove non-existent rule
    let params = ManageGgenResourceParams {
        operation: ResourceOperation::RemoveRule {
            config_path: Some("ggen.toml".to_string()),
            rule_name: "nonexistent-rule".to_string(),
            create_backup: false,
        },
    };

    let result = manage_ggen_resource(state, params).await;
    assert!(result.is_err(), "should fail with nonexistent rule");
}

// ============================================================================
// Performance Tests
// ============================================================================

#[tokio::test]
async fn test_response_includes_duration() {
    let temp_dir = TempDir::new().unwrap();
    let state = create_test_state(&temp_dir);
    let config_path = temp_dir.path().join("ggen.toml");
    create_sample_ggen_toml(&config_path);

    let params = ManageGgenResourceParams {
        operation: ResourceOperation::ReadConfig {
            config_path: Some("ggen.toml".to_string()),
        },
    };

    let result = manage_ggen_resource(state, params).await.unwrap();
    assert!(result.metadata.duration_ms > 0);
    assert!(result.metadata.duration_ms < 10000); // Should complete in <10s
}
