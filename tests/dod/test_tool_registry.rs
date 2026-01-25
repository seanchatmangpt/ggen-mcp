//! Tests for tool registry consistency check

use ggen_mcp::dod::check::{CheckContext, DodCheck};
use ggen_mcp::dod::checks::tool_registry::ToolRegistryCheck;
use ggen_mcp::dod::types::{CheckCategory, CheckSeverity, CheckStatus};
use std::path::PathBuf;

#[tokio::test]
async fn test_tool_registry_check_metadata() {
    let check = ToolRegistryCheck;

    assert_eq!(check.id(), "WHAT_TOOL_REGISTRY");
    assert_eq!(check.category(), CheckCategory::ToolRegistry);
    assert_eq!(check.severity(), CheckSeverity::Fatal);
    assert!(!check.description().is_empty());
}

#[tokio::test]
async fn test_tool_registry_check_executes() {
    let check = ToolRegistryCheck;
    let context = CheckContext::new(PathBuf::from("/home/user/ggen-mcp")).with_timeout(30_000);

    let result = check.execute(&context).await;
    assert!(result.is_ok(), "Check should execute without errors");

    let result = result.unwrap();
    assert_eq!(result.id, "WHAT_TOOL_REGISTRY");
    assert_eq!(result.category, CheckCategory::ToolRegistry);
    assert_eq!(result.severity, CheckSeverity::Fatal);

    // Status can be Pass or Fail depending on actual tool registry state
    assert!(
        matches!(result.status, CheckStatus::Pass | CheckStatus::Fail),
        "Status should be Pass or Fail, got {:?}",
        result.status
    );
}

#[tokio::test]
async fn test_tool_registry_check_timeout() {
    let check = ToolRegistryCheck;
    // Very short timeout to test timeout handling
    let context = CheckContext::new(PathBuf::from("/home/user/ggen-mcp")).with_timeout(1);

    // This might timeout or succeed depending on system speed
    // We just want to ensure it doesn't panic
    let _ = check.execute(&context).await;
}

#[tokio::test]
async fn test_tool_registry_check_provides_evidence() {
    let check = ToolRegistryCheck;
    let context = CheckContext::new(PathBuf::from("/home/user/ggen-mcp")).with_timeout(30_000);

    let result = check.execute(&context).await.unwrap();

    // Should provide evidence about tool counts
    assert!(!result.evidence.is_empty(), "Should provide evidence");

    let evidence = &result.evidence[0];
    assert!(
        evidence.content.contains("Registered")
            || evidence.content.contains("Implemented")
            || evidence.content.contains("Documented"),
        "Evidence should mention tool counts"
    );
}

#[tokio::test]
async fn test_tool_registry_check_provides_remediation_on_failure() {
    let check = ToolRegistryCheck;
    let context = CheckContext::new(PathBuf::from("/home/user/ggen-mcp")).with_timeout(30_000);

    let result = check.execute(&context).await.unwrap();

    if result.status == CheckStatus::Fail {
        assert!(
            !result.remediation.is_empty(),
            "Failed check should provide remediation steps"
        );
    }
}
