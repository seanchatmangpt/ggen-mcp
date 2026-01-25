//! Tests for build correctness checks

use ggen_mcp::dod::check::{CheckContext, DodCheck};
use ggen_mcp::dod::checks::build::{BuildCheckCheck, BuildClippyCheck, BuildFmtCheck};
use ggen_mcp::dod::types::{CheckCategory, CheckSeverity, CheckStatus};
use std::path::PathBuf;

// BUILD_FMT tests
#[tokio::test]
async fn test_build_fmt_check_metadata() {
    let check = BuildFmtCheck;

    assert_eq!(check.id(), "BUILD_FMT");
    assert_eq!(check.category(), CheckCategory::BuildCorrectness);
    assert_eq!(check.severity(), CheckSeverity::Fatal);
    assert!(check.description().contains("fmt"));
}

#[tokio::test]
async fn test_build_fmt_check_executes() {
    let check = BuildFmtCheck;
    let context = CheckContext::new(PathBuf::from("/home/user/ggen-mcp")).with_timeout(60_000);

    let result = check.execute(&context).await;
    assert!(result.is_ok(), "Check should execute without errors");

    let result = result.unwrap();
    assert_eq!(result.id, "BUILD_FMT");
    assert!(
        matches!(result.status, CheckStatus::Pass | CheckStatus::Fail),
        "Status should be Pass or Fail"
    );
}

#[tokio::test]
async fn test_build_fmt_check_provides_remediation_on_failure() {
    let check = BuildFmtCheck;
    let context = CheckContext::new(PathBuf::from("/home/user/ggen-mcp")).with_timeout(60_000);

    let result = check.execute(&context).await.unwrap();

    if result.status == CheckStatus::Fail {
        assert!(
            !result.remediation.is_empty(),
            "Failed check should provide remediation"
        );
        assert!(
            result.remediation[0].contains("cargo fmt"),
            "Remediation should mention cargo fmt"
        );
    }
}

// BUILD_CLIPPY tests
#[tokio::test]
async fn test_build_clippy_check_metadata() {
    let check = BuildClippyCheck;

    assert_eq!(check.id(), "BUILD_CLIPPY");
    assert_eq!(check.category(), CheckCategory::BuildCorrectness);
    assert_eq!(check.severity(), CheckSeverity::Warning);
    assert!(check.description().contains("clippy"));
}

#[tokio::test]
async fn test_build_clippy_check_executes() {
    let check = BuildClippyCheck;
    let context = CheckContext::new(PathBuf::from("/home/user/ggen-mcp")).with_timeout(120_000);

    let result = check.execute(&context).await;
    assert!(result.is_ok(), "Check should execute without errors");

    let result = result.unwrap();
    assert_eq!(result.id, "BUILD_CLIPPY");
    assert!(
        matches!(
            result.status,
            CheckStatus::Pass | CheckStatus::Fail | CheckStatus::Warn
        ),
        "Status should be Pass, Fail, or Warn"
    );
}

#[tokio::test]
async fn test_build_clippy_check_counts_warnings() {
    let check = BuildClippyCheck;
    let context = CheckContext::new(PathBuf::from("/home/user/ggen-mcp")).with_timeout(120_000);

    let result = check.execute(&context).await.unwrap();

    if result.status == CheckStatus::Warn {
        assert!(
            result.message.contains("warning"),
            "Warning status should mention warnings in message"
        );
    }
}

// BUILD_CHECK tests
#[tokio::test]
async fn test_build_check_check_metadata() {
    let check = BuildCheckCheck;

    assert_eq!(check.id(), "BUILD_CHECK");
    assert_eq!(check.category(), CheckCategory::BuildCorrectness);
    assert_eq!(check.severity(), CheckSeverity::Fatal);
    assert!(check.description().contains("compiles"));
}

#[tokio::test]
async fn test_build_check_check_executes() {
    let check = BuildCheckCheck;
    let context = CheckContext::new(PathBuf::from("/home/user/ggen-mcp")).with_timeout(120_000);

    let result = check.execute(&context).await;
    assert!(result.is_ok(), "Check should execute without errors");

    let result = result.unwrap();
    assert_eq!(result.id, "BUILD_CHECK");
    assert!(
        matches!(result.status, CheckStatus::Pass | CheckStatus::Fail),
        "Status should be Pass or Fail"
    );
}

#[tokio::test]
async fn test_build_check_provides_evidence_on_failure() {
    let check = BuildCheckCheck;
    let context = CheckContext::new(PathBuf::from("/home/user/ggen-mcp")).with_timeout(120_000);

    let result = check.execute(&context).await.unwrap();

    if result.status == CheckStatus::Fail {
        assert!(
            !result.evidence.is_empty(),
            "Failed check should provide evidence (compilation errors)"
        );
    }
}

// Integration tests
#[tokio::test]
async fn test_all_build_checks_have_unique_ids() {
    let checks: Vec<Box<dyn DodCheck>> = vec![
        Box::new(BuildFmtCheck),
        Box::new(BuildClippyCheck),
        Box::new(BuildCheckCheck),
    ];

    let ids: std::collections::HashSet<_> = checks.iter().map(|c| c.id()).collect();
    assert_eq!(ids.len(), 3, "All build checks should have unique IDs");
}

#[tokio::test]
async fn test_all_build_checks_same_category() {
    let checks: Vec<Box<dyn DodCheck>> = vec![
        Box::new(BuildFmtCheck),
        Box::new(BuildClippyCheck),
        Box::new(BuildCheckCheck),
    ];

    for check in checks {
        assert_eq!(
            check.category(),
            CheckCategory::BuildCorrectness,
            "All build checks should be in BuildCorrectness category"
        );
    }
}
