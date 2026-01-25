//! Integration tests for ggen_config tools
//!
//! Tests atomic file operations, validation, and TOML preservation.

mod support;

use anyhow::Result;
use spreadsheet_mcp::tools::ggen_config::*;
use support::TestWorkspace;
use tempfile::TempDir;
use tokio::fs;

/// Test helper: create temp ggen.toml
async fn create_test_config(dir: &TempDir) -> Result<String> {
    let config_path = dir.path().join("ggen.toml");
    let content = r#"
[project]
name = "test-project"
version = "0.1.0"

[ontology]
source = "ontology/test.ttl"

[generation]
output_dir = "."

[[generation.rules]]
name = "test-rule-1"
description = "Test rule 1"
query = { file = "queries/test1.rq" }
template = { file = "templates/test1.tera" }
output_file = "src/test1.rs"
mode = "Overwrite"

[[generation.rules]]
name = "test-rule-2"
description = "Test rule 2"
query = { file = "queries/test2.rq" }
template = { file = "templates/test2.tera" }
output_file = "src/test2.rs"
mode = "Overwrite"
"#;

    fs::write(&config_path, content).await?;
    Ok(config_path.to_string_lossy().to_string())
}

#[tokio::test]
async fn test_read_ggen_config() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = create_test_config(&temp_dir).await?;

    let workspace = TestWorkspace::new();
    let state = workspace.app_state();

    let params = ReadGgenConfigParams {
        config_path: Some(config_path),
    };

    let response = read_ggen_config(state, params).await?;

    assert_eq!(response.rule_count, 2);
    assert_eq!(response.rule_names, vec!["test-rule-1", "test-rule-2"]);
    assert!(response.file_size > 0);
    assert!(response.config.is_object());

    Ok(())
}

#[tokio::test]
async fn test_validate_ggen_config_valid() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = create_test_config(&temp_dir).await?;

    let workspace = TestWorkspace::new();
    let state = workspace.app_state();

    let params = ValidateGgenConfigParams {
        config_path: Some(config_path),
        check_file_refs: false, // Don't check files in test
        check_circular_deps: true,
        check_path_overlaps: true,
    };

    let response = validate_ggen_config(state, params).await?;

    assert!(response.valid);
    assert_eq!(response.rule_count, 2);
    assert_eq!(response.error_count, 0);

    Ok(())
}

#[tokio::test]
async fn test_validate_ggen_config_missing_section() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("ggen.toml");

    // Missing required sections
    fs::write(&config_path, "[project]\nname = \"test\"\n").await?;

    let workspace = TestWorkspace::new();
    let state = workspace.app_state();

    let params = ValidateGgenConfigParams {
        config_path: Some(config_path.to_string_lossy().to_string()),
        check_file_refs: false,
        check_circular_deps: true,
        check_path_overlaps: true,
    };

    let response = validate_ggen_config(state, params).await?;

    assert!(!response.valid);
    assert!(response.error_count > 0);

    Ok(())
}

#[tokio::test]
async fn test_add_generation_rule() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = create_test_config(&temp_dir).await?;

    let workspace = TestWorkspace::new();
    let state = workspace.app_state();

    let new_rule = GenerationRule {
        name: "new-rule".to_string(),
        description: "New test rule".to_string(),
        query_file: "queries/new.rq".to_string(),
        template_file: "templates/new.tera".to_string(),
        output_file: "src/new.rs".to_string(),
        mode: GenerationMode::Overwrite,
    };

    let params = AddGenerationRuleParams {
        config_path: Some(config_path.clone()),
        rule: new_rule,
        create_backup: true,
    };

    let response = add_generation_rule(state.clone(), params).await?;

    assert!(response.success);
    assert_eq!(response.rule_name, "new-rule");
    assert_eq!(response.rule_count, 3);
    assert!(response.backup_path.is_some());

    // Verify backup exists
    if let Some(backup) = &response.backup_path {
        assert!(std::path::Path::new(backup).exists());
    }

    // Verify rule was added
    let read_params = ReadGgenConfigParams {
        config_path: Some(config_path),
    };
    let read_response = read_ggen_config(state, read_params).await?;
    assert_eq!(read_response.rule_count, 3);
    assert!(read_response.rule_names.contains(&"new-rule".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_add_duplicate_rule_name() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = create_test_config(&temp_dir).await?;

    let workspace = TestWorkspace::new();
    let state = workspace.app_state();

    let duplicate_rule = GenerationRule {
        name: "test-rule-1".to_string(), // Already exists
        description: "Duplicate".to_string(),
        query_file: "queries/dup.rq".to_string(),
        template_file: "templates/dup.tera".to_string(),
        output_file: "src/dup.rs".to_string(),
        mode: GenerationMode::Overwrite,
    };

    let params = AddGenerationRuleParams {
        config_path: Some(config_path),
        rule: duplicate_rule,
        create_backup: true,
    };

    let result = add_generation_rule(state, params).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));

    Ok(())
}

#[tokio::test]
async fn test_update_generation_rule() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = create_test_config(&temp_dir).await?;

    let workspace = TestWorkspace::new();
    let state = workspace.app_state();

    let updated_rule = GenerationRule {
        name: "test-rule-1-updated".to_string(),
        description: "Updated description".to_string(),
        query_file: "queries/updated.rq".to_string(),
        template_file: "templates/updated.tera".to_string(),
        output_file: "src/updated.rs".to_string(),
        mode: GenerationMode::Append,
    };

    let params = UpdateGenerationRuleParams {
        config_path: Some(config_path.clone()),
        rule_name: "test-rule-1".to_string(),
        rule: updated_rule,
        create_backup: true,
    };

    let response = update_generation_rule(state.clone(), params).await?;

    assert!(response.success);
    assert_eq!(response.rule_name, "test-rule-1-updated");
    assert!(response.backup_path.is_some());

    // Verify rule was updated
    let read_params = ReadGgenConfigParams {
        config_path: Some(config_path),
    };
    let read_response = read_ggen_config(state, read_params).await?;
    assert!(
        read_response
            .rule_names
            .contains(&"test-rule-1-updated".to_string())
    );
    assert!(
        !read_response
            .rule_names
            .contains(&"test-rule-1".to_string())
    );

    Ok(())
}

#[tokio::test]
async fn test_update_nonexistent_rule() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = create_test_config(&temp_dir).await?;

    let workspace = TestWorkspace::new();
    let state = workspace.app_state();

    let updated_rule = GenerationRule {
        name: "updated".to_string(),
        description: "Test".to_string(),
        query_file: "queries/test.rq".to_string(),
        template_file: "templates/test.tera".to_string(),
        output_file: "src/test.rs".to_string(),
        mode: GenerationMode::Overwrite,
    };

    let params = UpdateGenerationRuleParams {
        config_path: Some(config_path),
        rule_name: "nonexistent-rule".to_string(),
        rule: updated_rule,
        create_backup: true,
    };

    let result = update_generation_rule(state, params).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));

    Ok(())
}

#[tokio::test]
async fn test_remove_generation_rule() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = create_test_config(&temp_dir).await?;

    let workspace = TestWorkspace::new();
    let state = workspace.app_state();

    let params = RemoveGenerationRuleParams {
        config_path: Some(config_path.clone()),
        rule_name: "test-rule-1".to_string(),
        create_backup: true,
    };

    let response = remove_generation_rule(state.clone(), params).await?;

    assert!(response.success);
    assert_eq!(response.rule_name, "test-rule-1");
    assert_eq!(response.rule_count, 1);
    assert!(response.backup_path.is_some());

    // Verify rule was removed
    let read_params = ReadGgenConfigParams {
        config_path: Some(config_path),
    };
    let read_response = read_ggen_config(state, read_params).await?;
    assert_eq!(read_response.rule_count, 1);
    assert!(
        !read_response
            .rule_names
            .contains(&"test-rule-1".to_string())
    );
    assert!(
        read_response
            .rule_names
            .contains(&"test-rule-2".to_string())
    );

    Ok(())
}

#[tokio::test]
async fn test_remove_nonexistent_rule() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = create_test_config(&temp_dir).await?;

    let workspace = TestWorkspace::new();
    let state = workspace.app_state();

    let params = RemoveGenerationRuleParams {
        config_path: Some(config_path),
        rule_name: "nonexistent-rule".to_string(),
        create_backup: true,
    };

    let result = remove_generation_rule(state, params).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));

    Ok(())
}

#[tokio::test]
async fn test_validate_path_safety() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = create_test_config(&temp_dir).await?;

    let workspace = TestWorkspace::new();
    let state = workspace.app_state();

    // Try to add rule with path traversal
    let dangerous_rule = GenerationRule {
        name: "dangerous".to_string(),
        description: "Path traversal attempt".to_string(),
        query_file: "../../../etc/passwd".to_string(),
        template_file: "templates/test.tera".to_string(),
        output_file: "src/test.rs".to_string(),
        mode: GenerationMode::Overwrite,
    };

    let params = AddGenerationRuleParams {
        config_path: Some(config_path),
        rule: dangerous_rule,
        create_backup: false,
    };

    let result = add_generation_rule(state, params).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_validate_output_overlaps() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("ggen.toml");

    let content = r#"
[ontology]
source = "test.ttl"

[generation]
output_dir = "."

[[generation.rules]]
name = "rule1"
description = "Rule 1"
query = { file = "q1.rq" }
template = { file = "t1.tera" }
output_file = "src/output.rs"
mode = "Overwrite"

[[generation.rules]]
name = "rule2"
description = "Rule 2"
query = { file = "q2.rq" }
template = { file = "t2.tera" }
output_file = "src/output.rs"
mode = "Overwrite"
"#;

    fs::write(&config_path, content).await?;

    let workspace = TestWorkspace::new();
    let state = workspace.app_state();

    let params = ValidateGgenConfigParams {
        config_path: Some(config_path.to_string_lossy().to_string()),
        check_file_refs: false,
        check_circular_deps: false,
        check_path_overlaps: true,
    };

    let response = validate_ggen_config(state, params).await?;

    assert!(!response.valid);
    assert!(response.error_count > 0);

    // Check for overlap error
    let has_overlap_error = response.issues.iter().any(|issue| {
        matches!(issue.severity, IssueSeverity::Error) && issue.message.contains("overlap")
    });
    assert!(has_overlap_error);

    Ok(())
}
