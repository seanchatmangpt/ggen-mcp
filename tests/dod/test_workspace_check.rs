use anyhow::Result;
use ggen_mcp::dod::{CheckContext, CheckStatus, DodCheck};
use ggen_mcp::dod::checks::workspace::WorkspaceIntegrityCheck;
use std::path::PathBuf;

#[tokio::test]
async fn test_workspace_check_valid_workspace() -> Result<()> {
    // Use actual project root
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let context = CheckContext::new(workspace_root);

    let check = WorkspaceIntegrityCheck;
    let result = check.execute(&context).await?;

    // Should pass or warn (not fail) for valid workspace
    assert!(
        result.status == CheckStatus::Pass || result.status == CheckStatus::Warn,
        "Expected Pass or Warn, got {:?}: {}",
        result.status,
        result.message
    );

    // Should have evidence
    assert!(!result.evidence.is_empty(), "Should collect evidence");

    // Should have proper ID and category
    assert_eq!(result.id, "G0_WORKSPACE");

    Ok(())
}

#[tokio::test]
async fn test_workspace_check_nonexistent_root() -> Result<()> {
    let workspace_root = PathBuf::from("/nonexistent/path/does/not/exist");
    let context = CheckContext::new(workspace_root);

    let check = WorkspaceIntegrityCheck;
    let result = check.execute(&context).await?;

    // Should fail
    assert_eq!(result.status, CheckStatus::Fail);
    assert!(result.message.contains("does not exist"));
    assert!(!result.remediation.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_workspace_check_has_required_metadata() -> Result<()> {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let context = CheckContext::new(workspace_root);

    let check = WorkspaceIntegrityCheck;

    // Verify check metadata
    assert_eq!(check.id(), "G0_WORKSPACE");
    assert_eq!(
        check.description(),
        "Validates workspace structure, paths, and environment"
    );

    let result = check.execute(&context).await?;
    assert!(result.duration_ms > 0, "Should track execution time");

    Ok(())
}

#[tokio::test]
async fn test_workspace_check_detects_missing_cargo_toml() -> Result<()> {
    // Use /tmp as workspace (no Cargo.toml)
    let workspace_root = PathBuf::from("/tmp");
    let context = CheckContext::new(workspace_root);

    let check = WorkspaceIntegrityCheck;
    let result = check.execute(&context).await?;

    // Should fail
    assert_eq!(result.status, CheckStatus::Fail);
    assert!(
        result.message.contains("Cargo.toml"),
        "Message should mention Cargo.toml: {}",
        result.message
    );
    assert!(!result.remediation.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_workspace_check_evidence_includes_environment() -> Result<()> {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let context = CheckContext::new(workspace_root);

    let check = WorkspaceIntegrityCheck;
    let result = check.execute(&context).await?;

    // Check for environment evidence
    let has_env_evidence = result.evidence.iter().any(|ev| {
        ev.content.contains("rustc") || ev.content.contains("platform")
    });

    assert!(has_env_evidence, "Should capture environment information");

    Ok(())
}
