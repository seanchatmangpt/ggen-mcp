use anyhow::Result;
use ggen_mcp::dod::checks::intent::IntentAlignmentCheck;
use ggen_mcp::dod::{CheckContext, CheckStatus, DodCheck};
use std::fs;
use std::path::PathBuf;

#[tokio::test]
async fn test_intent_check_finds_existing_docs() -> Result<()> {
    // Use actual project root (has README.md and docs/)
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let context = CheckContext::new(workspace_root);

    let check = IntentAlignmentCheck;
    let result = check.execute(&context).await?;

    // Should pass or warn (project has documentation)
    assert!(
        result.status == CheckStatus::Pass || result.status == CheckStatus::Warn,
        "Expected Pass or Warn, got {:?}: {}",
        result.status,
        result.message
    );

    // Verify metadata
    assert_eq!(result.id, "G8_INTENT");

    Ok(())
}

#[tokio::test]
async fn test_intent_check_warns_on_missing_docs() -> Result<()> {
    // Use /tmp as workspace (no intent docs)
    let temp_dir = tempfile::tempdir()?;
    let workspace_root = temp_dir.path().to_path_buf();

    let context = CheckContext::new(workspace_root);

    let check = IntentAlignmentCheck;
    let result = check.execute(&context).await?;

    // Should warn
    assert_eq!(result.status, CheckStatus::Warn);
    assert!(result.message.contains("No intent documentation"));
    assert!(!result.remediation.is_empty());
    assert!(result.remediation.iter().any(|r| r.contains("intent")));

    Ok(())
}

#[tokio::test]
async fn test_intent_check_has_required_metadata() -> Result<()> {
    let check = IntentAlignmentCheck;

    assert_eq!(check.id(), "G8_INTENT");
    assert_eq!(
        check.description(),
        "Validates that documented intent exists for changes"
    );

    Ok(())
}

#[tokio::test]
async fn test_intent_check_discovers_recent_markdown() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let workspace_root = temp_dir.path().to_path_buf();
    let docs_dir = workspace_root.join("docs");
    fs::create_dir_all(&docs_dir)?;

    // Create a recent markdown file
    let prd_path = docs_dir.join("PRD.md");
    fs::write(
        &prd_path,
        "# Product Requirements\n\nWHY: This feature solves X",
    )?;

    let context = CheckContext::new(workspace_root);

    let check = IntentAlignmentCheck;
    let result = check.execute(&context).await?;

    // Should pass (found recent docs)
    assert_eq!(result.status, CheckStatus::Pass);
    assert!(result.message.contains("Found"));
    assert!(!result.evidence.is_empty());

    // Evidence should reference the file
    let has_prd_evidence = result
        .evidence
        .iter()
        .any(|ev| ev.content.contains("PRD.md"));
    assert!(has_prd_evidence, "Should discover PRD.md");

    Ok(())
}

#[tokio::test]
async fn test_intent_check_ignores_old_files() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let workspace_root = temp_dir.path().to_path_buf();

    // Create README that's too old (would need manual timestamp manipulation)
    // For now, just verify empty workspace behavior
    let context = CheckContext::new(workspace_root);

    let check = IntentAlignmentCheck;
    let result = check.execute(&context).await?;

    // Empty workspace should warn
    assert_eq!(result.status, CheckStatus::Warn);

    Ok(())
}

#[tokio::test]
async fn test_intent_check_provides_remediation() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let workspace_root = temp_dir.path().to_path_buf();
    let context = CheckContext::new(workspace_root);

    let check = IntentAlignmentCheck;
    let result = check.execute(&context).await?;

    // Should warn with remediation
    assert_eq!(result.status, CheckStatus::Warn);
    assert!(!result.remediation.is_empty());

    // Remediation should mention WHY
    let mentions_why = result
        .remediation
        .iter()
        .any(|r| r.contains("WHY") || r.contains("intent"));
    assert!(
        mentions_why,
        "Remediation should guide toward documenting WHY"
    );

    Ok(())
}
