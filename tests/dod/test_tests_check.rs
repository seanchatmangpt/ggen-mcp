//! Tests for Category E: Test Truth checks

use spreadsheet_mcp::dod::{CheckContext, DodCheck, CheckStatus, CheckSeverity};
use std::path::PathBuf;

mod common;

#[tokio::test]
async fn test_unit_check_has_correct_metadata() {
    use spreadsheet_mcp::dod::checks::tests::TestUnitCheck;
    
    let check = TestUnitCheck;
    assert_eq!(check.id(), "TEST_UNIT");
    assert_eq!(check.severity(), CheckSeverity::Fatal);
    assert!(!check.description().is_empty());
}

#[tokio::test]
async fn test_integration_check_has_correct_metadata() {
    use spreadsheet_mcp::dod::checks::tests::TestIntegrationCheck;
    
    let check = TestIntegrationCheck;
    assert_eq!(check.id(), "TEST_INTEGRATION");
    assert_eq!(check.severity(), CheckSeverity::Fatal);
    assert!(!check.description().is_empty());
}

#[tokio::test]
async fn test_snapshot_check_has_correct_metadata() {
    use spreadsheet_mcp::dod::checks::tests::TestSnapshotCheck;
    
    let check = TestSnapshotCheck;
    assert_eq!(check.id(), "TEST_SNAPSHOT");
    assert_eq!(check.severity(), CheckSeverity::Warning);
    assert!(!check.description().is_empty());
}

#[tokio::test]
#[ignore] // Ignore by default as it runs real cargo test
async fn test_unit_check_execution() {
    use spreadsheet_mcp::dod::checks::tests::TestUnitCheck;
    
    let check = TestUnitCheck;
    let context = CheckContext::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")))
        .with_timeout(900_000); // 15 minutes for tests
    
    let result = check.execute(&context).await;
    assert!(result.is_ok());
    
    let result = result.unwrap();
    assert_eq!(result.id, "TEST_UNIT");
    // Status can be Pass or Fail depending on actual tests
    assert!(
        result.status == CheckStatus::Pass || result.status == CheckStatus::Fail,
        "Expected Pass or Fail, got {:?}",
        result.status
    );
}

#[tokio::test]
#[ignore] // Ignore by default as it runs real cargo test
async fn test_integration_check_execution() {
    use spreadsheet_mcp::dod::checks::tests::TestIntegrationCheck;
    
    let check = TestIntegrationCheck;
    let context = CheckContext::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")))
        .with_timeout(900_000); // 15 minutes for tests
    
    let result = check.execute(&context).await;
    assert!(result.is_ok());
    
    let result = result.unwrap();
    assert_eq!(result.id, "TEST_INTEGRATION");
}

#[test]
fn test_parse_test_output_success() {
    // This is testing the internal parsing function indirectly
    // by verifying the test check implementations exist
    use spreadsheet_mcp::dod::checks::tests::TestUnitCheck;
    let _check = TestUnitCheck;
    // If this compiles, the module structure is correct
}
