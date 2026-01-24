//! Receipt Generator Integration Tests
//!
//! Tests cryptographic receipt generation, hash chaining, and verification.

use spreadsheet_mcp::dod::*;
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

// Test fixture helpers

fn create_evidence(kind: EvidenceKind, content: &str, hash: &str) -> Evidence {
    Evidence {
        kind,
        content: content.to_string(),
        file_path: None,
        line_number: None,
        hash: hash.to_string(),
    }
}

fn create_check_result(
    id: &str,
    category: CheckCategory,
    status: CheckStatus,
    severity: CheckSeverity,
    message: &str,
) -> DodCheckResult {
    DodCheckResult {
        id: id.to_string(),
        category,
        status,
        severity,
        message: message.to_string(),
        evidence: vec![],
        remediation: vec![],
        duration_ms: 100,
        check_hash: "test".to_string(),
    }
}

fn create_check_with_evidence(
    id: &str,
    status: CheckStatus,
    evidence: Vec<Evidence>,
) -> DodCheckResult {
    DodCheckResult {
        id: id.to_string(),
        category: CheckCategory::BuildCorrectness,
        status,
        severity: CheckSeverity::Fatal,
        message: "Check with evidence".to_string(),
        evidence,
        remediation: vec![],
        duration_ms: 150,
        check_hash: "test".to_string(),
    }
}

fn create_validation_result(
    checks: Vec<DodCheckResult>,
    verdict: OverallVerdict,
    profile: &str,
    mode: ValidationMode,
) -> DodValidationResult {
    let summary = ValidationSummary {
        checks_total: checks.len(),
        checks_passed: checks
            .iter()
            .filter(|c| matches!(c.status, CheckStatus::Pass))
            .count(),
        checks_failed: checks
            .iter()
            .filter(|c| matches!(c.status, CheckStatus::Fail))
            .count(),
        checks_warned: checks
            .iter()
            .filter(|c| matches!(c.status, CheckStatus::Warn))
            .count(),
        checks_skipped: checks
            .iter()
            .filter(|c| matches!(c.status, CheckStatus::Skip))
            .count(),
    };

    DodValidationResult {
        verdict,
        readiness_score: 85.5,
        profile: profile.to_string(),
        mode,
        summary,
        category_scores: HashMap::new(),
        check_results: checks,
        artifacts: ArtifactPaths {
            receipt_path: PathBuf::from("receipts/test.json"),
            report_path: PathBuf::from("reports/test.md"),
            bundle_path: None,
        },
        duration_ms: 2500,
    }
}

// Hash Determinism Tests

#[test]
fn hash_determinism_identical_checks_produce_same_hash() {
    let temp_dir = TempDir::new().unwrap();
    let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

    let check1 = create_check_result(
        "BUILD_CHECK",
        CheckCategory::BuildCorrectness,
        CheckStatus::Pass,
        CheckSeverity::Fatal,
        "Cargo build succeeded",
    );

    let check2 = create_check_result(
        "BUILD_CHECK",
        CheckCategory::BuildCorrectness,
        CheckStatus::Pass,
        CheckSeverity::Fatal,
        "Cargo build succeeded",
    );

    let checks1 = vec![check1];
    let checks2 = vec![check2];

    let result1 =
        create_validation_result(checks1, OverallVerdict::Ready, "dev", ValidationMode::Fast);
    let result2 =
        create_validation_result(checks2, OverallVerdict::Ready, "dev", ValidationMode::Fast);

    let receipt1 = generator.generate(&result1).unwrap();
    let receipt2 = generator.generate(&result2).unwrap();

    // Check hashes should match
    assert_eq!(receipt1.check_hashes[0].hash, receipt2.check_hashes[0].hash);

    // Final hashes should match (timestamps differ, but check hashes identical)
    // Note: timestamps will differ, so final_hash may differ slightly
    // But check_hashes should be deterministic
}

#[test]
fn hash_determinism_different_message_produces_different_hash() {
    let temp_dir = TempDir::new().unwrap();
    let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

    let check1 = create_check_result(
        "BUILD_CHECK",
        CheckCategory::BuildCorrectness,
        CheckStatus::Pass,
        CheckSeverity::Fatal,
        "Cargo build succeeded",
    );

    let check2 = create_check_result(
        "BUILD_CHECK",
        CheckCategory::BuildCorrectness,
        CheckStatus::Pass,
        CheckSeverity::Fatal,
        "Build completed successfully", // Different message
    );

    let result1 = create_validation_result(
        vec![check1],
        OverallVerdict::Ready,
        "dev",
        ValidationMode::Fast,
    );
    let result2 = create_validation_result(
        vec![check2],
        OverallVerdict::Ready,
        "dev",
        ValidationMode::Fast,
    );

    let receipt1 = generator.generate(&result1).unwrap();
    let receipt2 = generator.generate(&result2).unwrap();

    assert_ne!(
        receipt1.check_hashes[0].hash, receipt2.check_hashes[0].hash,
        "Different messages should produce different hashes"
    );
}

#[test]
fn hash_determinism_evidence_order_normalized() {
    let temp_dir = TempDir::new().unwrap();
    let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

    let evidence1 = vec![
        create_evidence(EvidenceKind::FileContent, "content1", "hash1"),
        create_evidence(EvidenceKind::FileContent, "content2", "hash2"),
    ];

    let evidence2 = vec![
        create_evidence(EvidenceKind::FileContent, "content2", "hash2"),
        create_evidence(EvidenceKind::FileContent, "content1", "hash1"),
    ];

    let check1 = create_check_with_evidence("CHECK1", CheckStatus::Pass, evidence1);
    let check2 = create_check_with_evidence("CHECK1", CheckStatus::Pass, evidence2);

    let result1 = create_validation_result(
        vec![check1],
        OverallVerdict::Ready,
        "dev",
        ValidationMode::Fast,
    );
    let result2 = create_validation_result(
        vec![check2],
        OverallVerdict::Ready,
        "dev",
        ValidationMode::Fast,
    );

    let receipt1 = generator.generate(&result1).unwrap();
    let receipt2 = generator.generate(&result2).unwrap();

    // Evidence hashes are sorted, so check hashes should match
    assert_eq!(
        receipt1.check_hashes[0].hash, receipt2.check_hashes[0].hash,
        "Evidence order should not affect hash (sorted internally)"
    );
}

// Chain Integrity Tests

#[test]
fn chain_integrity_single_check_produces_valid_chain() {
    let temp_dir = TempDir::new().unwrap();
    let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

    let check = create_check_result(
        "BUILD_CHECK",
        CheckCategory::BuildCorrectness,
        CheckStatus::Pass,
        CheckSeverity::Fatal,
        "Build succeeded",
    );

    let result = create_validation_result(
        vec![check],
        OverallVerdict::Ready,
        "dev",
        ValidationMode::Fast,
    );
    let receipt = generator.generate(&result).unwrap();

    assert_eq!(receipt.check_hashes.len(), 1);
    assert!(!receipt.final_hash.is_empty());

    let verified = generator.verify(&receipt).unwrap();
    assert!(verified, "Single check chain should verify");
}

#[test]
fn chain_integrity_multiple_checks_chain_correctly() {
    let temp_dir = TempDir::new().unwrap();
    let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

    let checks = vec![
        create_check_result(
            "WORKSPACE_CLEAN",
            CheckCategory::WorkspaceIntegrity,
            CheckStatus::Pass,
            CheckSeverity::Fatal,
            "Workspace clean",
        ),
        create_check_result(
            "BUILD_CHECK",
            CheckCategory::BuildCorrectness,
            CheckStatus::Pass,
            CheckSeverity::Fatal,
            "Build succeeded",
        ),
        create_check_result(
            "TEST_UNIT",
            CheckCategory::TestTruth,
            CheckStatus::Pass,
            CheckSeverity::Fatal,
            "Tests passed",
        ),
    ];

    let result =
        create_validation_result(checks, OverallVerdict::Ready, "dev", ValidationMode::Fast);
    let receipt = generator.generate(&result).unwrap();

    assert_eq!(receipt.check_hashes.len(), 3);

    let verified = generator.verify(&receipt).unwrap();
    assert!(verified, "Multi-check chain should verify");
}

#[test]
fn chain_integrity_order_matters() {
    let temp_dir = TempDir::new().unwrap();
    let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

    let checks1 = vec![
        create_check_result(
            "CHECK_A",
            CheckCategory::BuildCorrectness,
            CheckStatus::Pass,
            CheckSeverity::Fatal,
            "Check A",
        ),
        create_check_result(
            "CHECK_B",
            CheckCategory::TestTruth,
            CheckStatus::Pass,
            CheckSeverity::Fatal,
            "Check B",
        ),
    ];

    let checks2 = vec![
        create_check_result(
            "CHECK_B",
            CheckCategory::TestTruth,
            CheckStatus::Pass,
            CheckSeverity::Fatal,
            "Check B",
        ),
        create_check_result(
            "CHECK_A",
            CheckCategory::BuildCorrectness,
            CheckStatus::Pass,
            CheckSeverity::Fatal,
            "Check A",
        ),
    ];

    let result1 =
        create_validation_result(checks1, OverallVerdict::Ready, "dev", ValidationMode::Fast);
    let result2 =
        create_validation_result(checks2, OverallVerdict::Ready, "dev", ValidationMode::Fast);

    let receipt1 = generator.generate(&result1).unwrap();
    let receipt2 = generator.generate(&result2).unwrap();

    assert_ne!(
        receipt1.final_hash, receipt2.final_hash,
        "Check order should affect final hash"
    );
}

#[test]
fn chain_integrity_empty_checks_produces_valid_receipt() {
    let temp_dir = TempDir::new().unwrap();
    let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

    let result =
        create_validation_result(vec![], OverallVerdict::Ready, "dev", ValidationMode::Fast);
    let receipt = generator.generate(&result).unwrap();

    assert_eq!(receipt.check_hashes.len(), 0);
    assert!(!receipt.final_hash.is_empty());

    let verified = generator.verify(&receipt).unwrap();
    assert!(verified, "Empty receipt should verify");
}

// JSON Serialization Tests

#[test]
fn json_serialization_roundtrip_preserves_all_fields() {
    let temp_dir = TempDir::new().unwrap();
    let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

    let checks = vec![
        create_check_result(
            "BUILD_CHECK",
            CheckCategory::BuildCorrectness,
            CheckStatus::Pass,
            CheckSeverity::Fatal,
            "Build succeeded",
        ),
        create_check_result(
            "TEST_UNIT",
            CheckCategory::TestTruth,
            CheckStatus::Warn,
            CheckSeverity::Warning,
            "Some tests skipped",
        ),
    ];

    let result = create_validation_result(
        checks,
        OverallVerdict::Ready,
        "production",
        ValidationMode::Strict,
    );
    let original = generator.generate(&result).unwrap();
    let receipt_path = generator.save(&original).unwrap();

    let loaded = ReceiptGenerator::load(&receipt_path).unwrap();

    assert_eq!(original.version, loaded.version);
    assert_eq!(original.verdict, loaded.verdict);
    assert_eq!(original.score, loaded.score);
    assert_eq!(original.profile, loaded.profile);
    assert_eq!(original.mode, loaded.mode);
    assert_eq!(original.duration_ms, loaded.duration_ms);
    assert_eq!(original.final_hash, loaded.final_hash);
    assert_eq!(original.check_hashes.len(), loaded.check_hashes.len());

    // Verify check hashes match
    for (orig, load) in original.check_hashes.iter().zip(loaded.check_hashes.iter()) {
        assert_eq!(orig.check_id, load.check_id);
        assert_eq!(orig.category, load.category);
        assert_eq!(orig.status, load.status);
        assert_eq!(orig.severity, load.severity);
        assert_eq!(orig.hash, load.hash);
    }

    // Verify metadata
    assert_eq!(original.metadata.checks_total, loaded.metadata.checks_total);
    assert_eq!(
        original.metadata.checks_passed,
        loaded.metadata.checks_passed
    );
}

#[test]
fn json_serialization_filename_format_correct() {
    let temp_dir = TempDir::new().unwrap();
    let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

    let check = create_check_result(
        "CHECK1",
        CheckCategory::BuildCorrectness,
        CheckStatus::Pass,
        CheckSeverity::Fatal,
        "Test",
    );

    let result = create_validation_result(
        vec![check],
        OverallVerdict::Ready,
        "dev",
        ValidationMode::Fast,
    );
    let receipt = generator.generate(&result).unwrap();
    let path = generator.save(&receipt).unwrap();

    let filename = path.file_name().unwrap().to_str().unwrap();

    // Format: YYYY-MM-DD-HHMMSS.json
    assert!(filename.ends_with(".json"));
    assert!(filename.contains("-"));

    // Verify file exists
    assert!(path.exists());
}

#[test]
fn json_serialization_creates_valid_json() {
    let temp_dir = TempDir::new().unwrap();
    let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

    let check = create_check_result(
        "TEST_CHECK",
        CheckCategory::TestTruth,
        CheckStatus::Pass,
        CheckSeverity::Fatal,
        "All tests passed",
    );

    let result = create_validation_result(
        vec![check],
        OverallVerdict::Ready,
        "dev",
        ValidationMode::Fast,
    );
    let receipt = generator.generate(&result).unwrap();
    let path = generator.save(&receipt).unwrap();

    // Read and parse JSON
    let json_content = std::fs::read_to_string(&path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_content).unwrap();

    // Verify top-level fields exist
    assert!(parsed.get("version").is_some());
    assert!(parsed.get("timestamp").is_some());
    assert!(parsed.get("verdict").is_some());
    assert!(parsed.get("score").is_some());
    assert!(parsed.get("check_hashes").is_some());
    assert!(parsed.get("final_hash").is_some());
    assert!(parsed.get("metadata").is_some());
}

// Verification Tests

#[test]
fn verify_accepts_valid_receipt() {
    let temp_dir = TempDir::new().unwrap();
    let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

    let checks = vec![
        create_check_result(
            "CHECK1",
            CheckCategory::BuildCorrectness,
            CheckStatus::Pass,
            CheckSeverity::Fatal,
            "Check 1",
        ),
        create_check_result(
            "CHECK2",
            CheckCategory::TestTruth,
            CheckStatus::Pass,
            CheckSeverity::Fatal,
            "Check 2",
        ),
    ];

    let result =
        create_validation_result(checks, OverallVerdict::Ready, "dev", ValidationMode::Fast);
    let receipt = generator.generate(&result).unwrap();

    let verified = generator.verify(&receipt).unwrap();
    assert!(verified, "Valid receipt should verify successfully");
}

#[test]
fn verify_detects_tampered_check_hash() {
    let temp_dir = TempDir::new().unwrap();
    let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

    let checks = vec![
        create_check_result(
            "CHECK1",
            CheckCategory::BuildCorrectness,
            CheckStatus::Pass,
            CheckSeverity::Fatal,
            "Check 1",
        ),
        create_check_result(
            "CHECK2",
            CheckCategory::TestTruth,
            CheckStatus::Pass,
            CheckSeverity::Fatal,
            "Check 2",
        ),
    ];

    let result =
        create_validation_result(checks, OverallVerdict::Ready, "dev", ValidationMode::Fast);
    let mut receipt = generator.generate(&result).unwrap();

    // Tamper with check hash
    receipt.check_hashes[1].hash =
        "0000000000000000000000000000000000000000000000000000000000000000".to_string();

    let verified = generator.verify(&receipt).unwrap();
    assert!(!verified, "Tampered check hash should fail verification");
}

#[test]
fn verify_detects_tampered_final_hash() {
    let temp_dir = TempDir::new().unwrap();
    let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

    let check = create_check_result(
        "CHECK1",
        CheckCategory::BuildCorrectness,
        CheckStatus::Pass,
        CheckSeverity::Fatal,
        "Check 1",
    );

    let result = create_validation_result(
        vec![check],
        OverallVerdict::Ready,
        "dev",
        ValidationMode::Fast,
    );
    let mut receipt = generator.generate(&result).unwrap();

    // Tamper with final hash
    receipt.final_hash = "tampered_final_hash".to_string();

    let verified = generator.verify(&receipt).unwrap();
    assert!(!verified, "Tampered final hash should fail verification");
}

#[test]
fn verify_after_save_and_load() {
    let temp_dir = TempDir::new().unwrap();
    let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

    let checks = vec![
        create_check_result(
            "WORKSPACE_CLEAN",
            CheckCategory::WorkspaceIntegrity,
            CheckStatus::Pass,
            CheckSeverity::Fatal,
            "Workspace clean",
        ),
        create_check_result(
            "BUILD_CHECK",
            CheckCategory::BuildCorrectness,
            CheckStatus::Pass,
            CheckSeverity::Fatal,
            "Build succeeded",
        ),
    ];

    let result = create_validation_result(
        checks,
        OverallVerdict::Ready,
        "production",
        ValidationMode::Strict,
    );
    let receipt = generator.generate(&result).unwrap();
    let path = generator.save(&receipt).unwrap();

    // Load and verify
    let loaded = ReceiptGenerator::load(&path).unwrap();
    let verified = generator.verify(&loaded).unwrap();

    assert!(verified, "Loaded receipt should verify successfully");
}

// Integration Tests

#[test]
fn generate_and_save_creates_receipt_file() {
    let temp_dir = TempDir::new().unwrap();
    let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

    let check = create_check_result(
        "BUILD_CHECK",
        CheckCategory::BuildCorrectness,
        CheckStatus::Pass,
        CheckSeverity::Fatal,
        "Build succeeded",
    );

    let result = create_validation_result(
        vec![check],
        OverallVerdict::Ready,
        "dev",
        ValidationMode::Fast,
    );

    let receipt_path = generator.generate_and_save(&result).unwrap();

    assert!(receipt_path.exists());
    assert!(receipt_path.is_file());
    assert!(receipt_path.extension().unwrap() == "json");
}

#[test]
fn receipts_for_different_verdicts() {
    let temp_dir = TempDir::new().unwrap();
    let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

    let checks_ready = vec![create_check_result(
        "CHECK1",
        CheckCategory::BuildCorrectness,
        CheckStatus::Pass,
        CheckSeverity::Fatal,
        "All good",
    )];

    let checks_not_ready = vec![create_check_result(
        "CHECK1",
        CheckCategory::BuildCorrectness,
        CheckStatus::Fail,
        CheckSeverity::Fatal,
        "Build failed",
    )];

    let result_ready = create_validation_result(
        checks_ready,
        OverallVerdict::Ready,
        "dev",
        ValidationMode::Fast,
    );
    let result_not_ready = create_validation_result(
        checks_not_ready,
        OverallVerdict::NotReady,
        "dev",
        ValidationMode::Fast,
    );

    let receipt_ready = generator.generate(&result_ready).unwrap();
    let receipt_not_ready = generator.generate(&result_not_ready).unwrap();

    assert_eq!(receipt_ready.verdict, OverallVerdict::Ready);
    assert_eq!(receipt_not_ready.verdict, OverallVerdict::NotReady);

    // Different verdicts produce different final hashes (metadata differs)
    assert_ne!(receipt_ready.final_hash, receipt_not_ready.final_hash);
}

#[test]
fn receipts_for_different_profiles() {
    let temp_dir = TempDir::new().unwrap();
    let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

    let check = create_check_result(
        "CHECK1",
        CheckCategory::BuildCorrectness,
        CheckStatus::Pass,
        CheckSeverity::Fatal,
        "Test",
    );

    let result_dev = create_validation_result(
        vec![check.clone()],
        OverallVerdict::Ready,
        "dev",
        ValidationMode::Fast,
    );
    let result_prod = create_validation_result(
        vec![check],
        OverallVerdict::Ready,
        "production",
        ValidationMode::Strict,
    );

    let receipt_dev = generator.generate(&result_dev).unwrap();
    let receipt_prod = generator.generate(&result_prod).unwrap();

    assert_eq!(receipt_dev.profile, "dev");
    assert_eq!(receipt_prod.profile, "production");

    // Different profiles produce different final hashes
    assert_ne!(receipt_dev.final_hash, receipt_prod.final_hash);
}

#[test]
fn receipt_metadata_includes_summary() {
    let temp_dir = TempDir::new().unwrap();
    let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

    let checks = vec![
        create_check_result(
            "CHECK1",
            CheckCategory::BuildCorrectness,
            CheckStatus::Pass,
            CheckSeverity::Fatal,
            "Passed",
        ),
        create_check_result(
            "CHECK2",
            CheckCategory::TestTruth,
            CheckStatus::Fail,
            CheckSeverity::Warning,
            "Failed",
        ),
        create_check_result(
            "CHECK3",
            CheckCategory::GgenPipeline,
            CheckStatus::Warn,
            CheckSeverity::Warning,
            "Warned",
        ),
        create_check_result(
            "CHECK4",
            CheckCategory::SafetyInvariants,
            CheckStatus::Skip,
            CheckSeverity::Info,
            "Skipped",
        ),
    ];

    let result =
        create_validation_result(checks, OverallVerdict::Ready, "dev", ValidationMode::Fast);
    let receipt = generator.generate(&result).unwrap();

    assert_eq!(receipt.metadata.checks_total, 4);
    assert_eq!(receipt.metadata.checks_passed, 1);
    assert_eq!(receipt.metadata.checks_failed, 1);
    assert_eq!(receipt.metadata.checks_warned, 1);
    assert_eq!(receipt.metadata.checks_skipped, 1);
}
