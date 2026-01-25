//! Integration tests for Jira stage in ggen sync pipeline
//!
//! Tests the optional stage 14 (Jira integration) that runs after code generation.

use spreadsheet_mcp::tools::ggen_sync::jira_stage::{
    ColumnMapping, GeneratedFileInfo, JiraConfig, JiraMode, JiraStage, SyncContext,
};

#[test]
fn test_jira_config_from_toml_disabled() {
    let toml_str = r#"
        [jira]
        enabled = false
    "#;
    let toml: toml::Value = toml::from_str(toml_str).unwrap();
    let config = JiraConfig::from_toml(&toml).unwrap();
    assert!(config.is_none(), "Disabled Jira config should return None");
}

#[test]
fn test_jira_config_from_toml_dry_run() {
    std::env::set_var("JIRA_TOKEN_TEST", "test-token-123");

    let toml_str = r#"
        [jira]
        enabled = true
        mode = "dry_run"
        project_key = "TEST"
        base_url = "https://test.atlassian.net"
        auth_token_env = "JIRA_TOKEN_TEST"

        [jira.mapping]
        summary_column = "B"
        status_column = "C"
        assignee_column = "D"
    "#;
    let toml: toml::Value = toml::from_str(toml_str).unwrap();
    let config = JiraConfig::from_toml(&toml).unwrap().unwrap();

    assert_eq!(config.mode, JiraMode::DryRun);
    assert_eq!(config.project_key, "TEST");
    assert_eq!(config.base_url, "https://test.atlassian.net");
    assert_eq!(config.auth_token_env, "JIRA_TOKEN_TEST");
    assert_eq!(config.mapping.summary_column, "B");
    assert_eq!(config.mapping.status_column, "C");
    assert_eq!(config.mapping.assignee_column, "D");
}

#[test]
fn test_jira_config_from_toml_create_mode() {
    std::env::set_var("JIRA_API_KEY", "api-key-456");

    let toml_str = r#"
        [jira]
        enabled = true
        mode = "create"
        project_key = "DEMO"
        base_url = "https://demo.atlassian.net"
        auth_token_env = "JIRA_API_KEY"
    "#;
    let toml: toml::Value = toml::from_str(toml_str).unwrap();
    let config = JiraConfig::from_toml(&toml).unwrap().unwrap();

    assert_eq!(config.mode, JiraMode::Create);
    assert_eq!(config.project_key, "DEMO");
}

#[test]
fn test_jira_config_from_toml_sync_mode() {
    std::env::set_var("JIRA_SECRET", "secret-789");

    let toml_str = r#"
        [jira]
        enabled = true
        mode = "sync"
        project_key = "SYNC"
        base_url = "https://sync.atlassian.net"
        auth_token_env = "JIRA_SECRET"
    "#;
    let toml: toml::Value = toml::from_str(toml_str).unwrap();
    let config = JiraConfig::from_toml(&toml).unwrap().unwrap();

    assert_eq!(config.mode, JiraMode::Sync);
}

#[test]
fn test_jira_config_missing_token_env() {
    std::env::remove_var("MISSING_TOKEN_VAR");

    let toml_str = r#"
        [jira]
        enabled = true
        mode = "dry_run"
        project_key = "FAIL"
        base_url = "https://fail.atlassian.net"
        auth_token_env = "MISSING_TOKEN_VAR"
    "#;
    let toml: toml::Value = toml::from_str(toml_str).unwrap();
    let result = JiraConfig::from_toml(&toml);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("MISSING_TOKEN_VAR"));
}

#[test]
fn test_jira_config_invalid_mode() {
    std::env::set_var("JIRA_TOKEN", "token");

    let toml_str = r#"
        [jira]
        enabled = true
        mode = "invalid_mode"
        project_key = "TEST"
        base_url = "https://test.atlassian.net"
        auth_token_env = "JIRA_TOKEN"
    "#;
    let toml: toml::Value = toml::from_str(toml_str).unwrap();
    let result = JiraConfig::from_toml(&toml);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Invalid jira.mode"));
}

#[test]
fn test_jira_config_default_mapping() {
    std::env::set_var("JIRA_TOKEN", "token");

    let toml_str = r#"
        [jira]
        enabled = true
        mode = "dry_run"
        project_key = "DEFAULT"
        base_url = "https://default.atlassian.net"
        auth_token_env = "JIRA_TOKEN"
    "#;
    let toml: toml::Value = toml::from_str(toml_str).unwrap();
    let config = JiraConfig::from_toml(&toml).unwrap().unwrap();

    assert_eq!(config.mapping.summary_column, "B");
    assert_eq!(config.mapping.status_column, "C");
    assert_eq!(config.mapping.assignee_column, "D");
    assert!(config.mapping.description_column.is_none());
}

#[test]
fn test_generate_plan_empty_files() {
    std::env::set_var("JIRA_TOKEN", "token");

    let config = JiraConfig {
        enabled: true,
        mode: JiraMode::DryRun,
        project_key: "PLAN".to_string(),
        base_url: "https://plan.atlassian.net".to_string(),
        auth_token_env: "JIRA_TOKEN".to_string(),
        mapping: ColumnMapping {
            summary_column: "B".to_string(),
            status_column: "C".to_string(),
            assignee_column: "D".to_string(),
            description_column: None,
        },
    };

    let ctx = SyncContext {
        workbook_id: "test.xlsx".to_string(),
        fork_id: None,
        generated_files: vec![],
    };

    let plan = JiraStage::generate_plan(&ctx, &config).unwrap();

    assert_eq!(plan.project_key, "PLAN");
    assert_eq!(plan.tickets.len(), 0);
    assert!(plan.dry_run);
}

#[test]
fn test_generate_plan_multiple_files() {
    std::env::set_var("JIRA_TOKEN", "token");

    let config = JiraConfig {
        enabled: true,
        mode: JiraMode::DryRun,
        project_key: "MULTI".to_string(),
        base_url: "https://multi.atlassian.net".to_string(),
        auth_token_env: "JIRA_TOKEN".to_string(),
        mapping: ColumnMapping {
            summary_column: "B".to_string(),
            status_column: "C".to_string(),
            assignee_column: "D".to_string(),
            description_column: Some("E".to_string()),
        },
    };

    let ctx = SyncContext {
        workbook_id: "project.xlsx".to_string(),
        fork_id: Some("fork-123".to_string()),
        generated_files: vec![
            GeneratedFileInfo {
                path: "src/generated/tool.rs".to_string(),
                module_name: "tool".to_string(),
                source_query: "tool.rq".to_string(),
                source_template: "tool.rs.tera".to_string(),
            },
            GeneratedFileInfo {
                path: "src/generated/handler.rs".to_string(),
                module_name: "handler".to_string(),
                source_query: "handler.rq".to_string(),
                source_template: "handler.rs.tera".to_string(),
            },
            GeneratedFileInfo {
                path: "src/generated/validator.rs".to_string(),
                module_name: "validator".to_string(),
                source_query: "validator.rq".to_string(),
                source_template: "validator.rs.tera".to_string(),
            },
        ],
    };

    let plan = JiraStage::generate_plan(&ctx, &config).unwrap();

    assert_eq!(plan.project_key, "MULTI");
    assert_eq!(plan.tickets.len(), 3);
    assert!(plan.dry_run);

    assert_eq!(plan.tickets[0].summary, "Implement tool");
    assert!(plan.tickets[0].description.contains("tool.rq"));
    assert!(plan.tickets[0].description.contains("tool.rs.tera"));
    assert!(plan.tickets[0].labels.contains(&"generated".to_string()));
    assert!(plan.tickets[0].labels.contains(&"ggen".to_string()));

    assert_eq!(plan.tickets[1].summary, "Implement handler");
    assert_eq!(plan.tickets[2].summary, "Implement validator");
}

#[test]
fn test_column_mapping_defaults() {
    let mapping = ColumnMapping {
        summary_column: "B".to_string(),
        status_column: "C".to_string(),
        assignee_column: "D".to_string(),
        description_column: None,
    };

    assert_eq!(mapping.summary_column, "B");
    assert_eq!(mapping.status_column, "C");
    assert_eq!(mapping.assignee_column, "D");
    assert!(mapping.description_column.is_none());
}

#[test]
fn test_jira_mode_serialization() {
    let dry_run = JiraMode::DryRun;
    let create = JiraMode::Create;
    let sync = JiraMode::Sync;

    let dry_run_json = serde_json::to_string(&dry_run).unwrap();
    let create_json = serde_json::to_string(&create).unwrap();
    let sync_json = serde_json::to_string(&sync).unwrap();

    assert_eq!(dry_run_json, r#""dry_run""#);
    assert_eq!(create_json, r#""create""#);
    assert_eq!(sync_json, r#""sync""#);
}

#[test]
fn test_get_auth_token_success() {
    std::env::set_var("AUTH_TOKEN_SUCCESS", "success-token");

    let config = JiraConfig {
        enabled: true,
        mode: JiraMode::DryRun,
        project_key: "TEST".to_string(),
        base_url: "https://test.atlassian.net".to_string(),
        auth_token_env: "AUTH_TOKEN_SUCCESS".to_string(),
        mapping: ColumnMapping {
            summary_column: "B".to_string(),
            status_column: "C".to_string(),
            assignee_column: "D".to_string(),
            description_column: None,
        },
    };

    let token = config.get_auth_token().unwrap();
    assert_eq!(token, "success-token");
}

#[test]
fn test_get_auth_token_failure() {
    std::env::remove_var("AUTH_TOKEN_MISSING");

    let config = JiraConfig {
        enabled: true,
        mode: JiraMode::DryRun,
        project_key: "TEST".to_string(),
        base_url: "https://test.atlassian.net".to_string(),
        auth_token_env: "AUTH_TOKEN_MISSING".to_string(),
        mapping: ColumnMapping {
            summary_column: "B".to_string(),
            status_column: "C".to_string(),
            assignee_column: "D".to_string(),
            description_column: None,
        },
    };

    let result = config.get_auth_token();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("AUTH_TOKEN_MISSING")
    );
}
