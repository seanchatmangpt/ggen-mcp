//! Evidence Bundle Generator tests
//!
//! Validates evidence bundle generation, artifact collection, and manifest creation.

use anyhow::Result;
use spreadsheet_mcp::dod::{
    CheckCategory, CheckSeverity, CheckStatus, DodCheckResult, DodValidationResult,
    EvidenceBundleGenerator, EvidenceKind, Evidence, OverallVerdict, ValidationMode,
    ValidationSummary, CategoryScore, ArtifactPaths, FileType,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// Test utilities
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn create_test_result(receipt_path: PathBuf, report_path: PathBuf) -> DodValidationResult {
    let check_results = vec![
        DodCheckResult {
            id: "BUILD_CHECK".to_string(),
            category: CheckCategory::BuildCorrectness,
            status: CheckStatus::Pass,
            severity: CheckSeverity::Fatal,
            message: "Build successful".to_string(),
            evidence: vec![],
            remediation: vec![],
            duration_ms: 1000,
            check_hash: "abc123".to_string(),
        },
        DodCheckResult {
            id: "TEST_UNIT".to_string(),
            category: CheckCategory::TestTruth,
            status: CheckStatus::Fail,
            severity: CheckSeverity::Fatal,
            message: "2 tests failed".to_string(),
            evidence: vec![Evidence {
                kind: EvidenceKind::CommandOutput,
                content: "test output".to_string(),
                file_path: None,
                line_number: None,
                hash: "def456".to_string(),
            }],
            remediation: vec!["Fix failing tests".to_string()],
            duration_ms: 2000,
            check_hash: "def456".to_string(),
        },
        DodCheckResult {
            id: "GGEN_DRY_RUN".to_string(),
            category: CheckCategory::GgenPipeline,
            status: CheckStatus::Pass,
            severity: CheckSeverity::Fatal,
            message: "Dry run successful".to_string(),
            evidence: vec![
                Evidence {
                    kind: EvidenceKind::FileContent,
                    content: "Generated code preview".to_string(),
                    file_path: Some(PathBuf::from("src/generated/mod.rs")),
                    line_number: None,
                    hash: "ghi789".to_string(),
                },
            ],
            remediation: vec![],
            duration_ms: 500,
            check_hash: "ghi789".to_string(),
        },
    ];

    let mut category_scores = HashMap::new();
    category_scores.insert(
        CheckCategory::BuildCorrectness,
        CategoryScore {
            category: CheckCategory::BuildCorrectness,
            score: 100.0,
            weight: 0.3,
            checks_passed: 1,
            checks_failed: 0,
            checks_warned: 0,
            checks_skipped: 0,
        },
    );
    category_scores.insert(
        CheckCategory::TestTruth,
        CategoryScore {
            category: CheckCategory::TestTruth,
            score: 0.0,
            weight: 0.3,
            checks_passed: 0,
            checks_failed: 1,
            checks_warned: 0,
            checks_skipped: 0,
        },
    );

    DodValidationResult {
        verdict: OverallVerdict::NotReady,
        readiness_score: 66.7,
        profile: "dev".to_string(),
        mode: ValidationMode::Fast,
        summary: ValidationSummary {
            checks_total: 3,
            checks_passed: 2,
            checks_failed: 1,
            checks_warned: 0,
            checks_skipped: 0,
        },
        category_scores,
        check_results,
        artifacts: ArtifactPaths {
            receipt_path,
            report_path,
            bundle_path: None,
        },
        duration_ms: 3500,
    }
}

#[test]
fn test_bundle_generator_creates_directory_structure() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = temp_dir.path().join("dod-evidence");

    // Create test receipt and report
    let receipt_path = temp_dir.path().join("receipt.json");
    let report_path = temp_dir.path().join("report.md");
    fs::write(&receipt_path, r#"{"receipt_id": "test-123"}"#)?;
    fs::write(&report_path, "# Test Report\nAll checks passed.")?;

    let result = create_test_result(receipt_path, report_path);
    let generator = EvidenceBundleGenerator::new(output_dir.clone());

    let bundle_path = generator.generate(&result, &workspace_root())?;

    // Verify bundle directory exists
    assert!(bundle_path.exists());
    assert!(bundle_path.is_dir());

    // Verify subdirectories exist
    assert!(bundle_path.join("logs").exists());
    assert!(bundle_path.join("artifacts").exists());

    // Verify manifest exists
    assert!(bundle_path.join("manifest.json").exists());

    Ok(())
}

#[test]
fn test_bundle_copies_receipt_and_report() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = temp_dir.path().join("dod-evidence");

    let receipt_path = temp_dir.path().join("receipt.json");
    let report_path = temp_dir.path().join("report.md");
    fs::write(&receipt_path, r#"{"receipt_id": "test-456"}"#)?;
    fs::write(&report_path, "# Report\nResults here.")?;

    let result = create_test_result(receipt_path.clone(), report_path.clone());
    let generator = EvidenceBundleGenerator::new(output_dir);

    let bundle_path = generator.generate(&result, &workspace_root())?;

    // Verify files copied
    let bundle_receipt = bundle_path.join("receipt.json");
    let bundle_report = bundle_path.join("report.md");

    assert!(bundle_receipt.exists());
    assert!(bundle_report.exists());

    // Verify content
    let receipt_content = fs::read_to_string(&bundle_receipt)?;
    assert!(receipt_content.contains("test-456"));

    let report_content = fs::read_to_string(&bundle_report)?;
    assert!(report_content.contains("Report"));

    Ok(())
}

#[test]
fn test_bundle_creates_log_files_for_each_check() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = temp_dir.path().join("dod-evidence");

    let receipt_path = temp_dir.path().join("receipt.json");
    let report_path = temp_dir.path().join("report.md");
    fs::write(&receipt_path, "{}")?;
    fs::write(&report_path, "# Report")?;

    let result = create_test_result(receipt_path, report_path);
    let generator = EvidenceBundleGenerator::new(output_dir);

    let bundle_path = generator.generate(&result, &workspace_root())?;

    let logs_dir = bundle_path.join("logs");
    assert!(logs_dir.exists());

    // Verify log files created for each check
    assert!(logs_dir.join("build-check.log").exists());
    assert!(logs_dir.join("test-unit.log").exists());
    assert!(logs_dir.join("ggen-dry-run.log").exists());

    // Verify log content
    let build_log = fs::read_to_string(logs_dir.join("build-check.log"))?;
    assert!(build_log.contains("BUILD_CHECK"));
    assert!(build_log.contains("Build successful"));
    assert!(build_log.contains("Pass"));

    let test_log = fs::read_to_string(logs_dir.join("test-unit.log"))?;
    assert!(test_log.contains("TEST_UNIT"));
    assert!(test_log.contains("2 tests failed"));
    assert!(test_log.contains("Fail"));
    assert!(test_log.contains("Evidence:"));
    assert!(test_log.contains("Remediation:"));

    Ok(())
}

#[test]
fn test_bundle_collects_artifact_snapshots() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = temp_dir.path().join("dod-evidence");

    let receipt_path = temp_dir.path().join("receipt.json");
    let report_path = temp_dir.path().join("report.md");
    fs::write(&receipt_path, "{}")?;
    fs::write(&report_path, "# Report")?;

    let result = create_test_result(receipt_path, report_path);
    let generator = EvidenceBundleGenerator::new(output_dir);

    let bundle_path = generator.generate(&result, &workspace_root())?;

    let artifacts_dir = bundle_path.join("artifacts");
    assert!(artifacts_dir.exists());

    // Verify key artifacts collected (if they exist in workspace)
    // Note: Cargo.lock and Cargo.toml should exist in workspace_root
    let cargo_lock = artifacts_dir.join("Cargo.lock");
    let cargo_toml = artifacts_dir.join("Cargo.toml");

    // These should exist in the actual workspace
    assert!(cargo_lock.exists(), "Cargo.lock should be collected");
    assert!(cargo_toml.exists(), "Cargo.toml should be collected");

    Ok(())
}

#[test]
fn test_manifest_contains_all_files() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = temp_dir.path().join("dod-evidence");

    let receipt_path = temp_dir.path().join("receipt.json");
    let report_path = temp_dir.path().join("report.md");
    fs::write(&receipt_path, r#"{"id": "test"}"#)?;
    fs::write(&report_path, "# Report")?;

    let result = create_test_result(receipt_path, report_path);
    let generator = EvidenceBundleGenerator::new(output_dir);

    let bundle_path = generator.generate(&result, &workspace_root())?;

    // Read manifest
    let manifest_path = bundle_path.join("manifest.json");
    let manifest_content = fs::read_to_string(&manifest_path)?;
    let manifest: serde_json::Value = serde_json::from_str(&manifest_content)?;

    // Verify manifest structure
    assert!(manifest["created_at"].is_string());
    assert_eq!(manifest["profile"], "dev");
    assert_eq!(manifest["verdict"], "NotReady");
    assert_eq!(manifest["readiness_score"], 66.7);

    // Verify files section
    let files = manifest["files"].as_object().unwrap();
    assert!(files.contains_key("receipt.json"));
    assert!(files.contains_key("report.md"));

    // Log files
    assert!(files.contains_key("logs/build-check.log"));
    assert!(files.contains_key("logs/test-unit.log"));

    // Verify file entries have required fields
    let receipt_entry = &files["receipt.json"];
    assert!(receipt_entry["path"].is_string());
    assert!(receipt_entry["size_bytes"].is_number());
    assert!(receipt_entry["hash"].is_string());
    assert_eq!(receipt_entry["file_type"], "Receipt");

    Ok(())
}

#[test]
fn test_manifest_includes_file_hashes() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = temp_dir.path().join("dod-evidence");

    let receipt_path = temp_dir.path().join("receipt.json");
    let report_path = temp_dir.path().join("report.md");
    let test_content = "test content for hashing";
    fs::write(&receipt_path, test_content)?;
    fs::write(&report_path, "# Report")?;

    let result = create_test_result(receipt_path, report_path);
    let generator = EvidenceBundleGenerator::new(output_dir);

    let bundle_path = generator.generate(&result, &workspace_root())?;

    // Read manifest
    let manifest_path = bundle_path.join("manifest.json");
    let manifest_content = fs::read_to_string(&manifest_path)?;
    let manifest: serde_json::Value = serde_json::from_str(&manifest_content)?;

    let files = manifest["files"].as_object().unwrap();
    let receipt_entry = &files["receipt.json"];

    // Verify hash is present and is a valid SHA-256 (64 hex chars)
    let hash = receipt_entry["hash"].as_str().unwrap();
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));

    Ok(())
}

#[test]
fn test_compression_creates_tar_gz() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = temp_dir.path().join("dod-evidence");

    let receipt_path = temp_dir.path().join("receipt.json");
    let report_path = temp_dir.path().join("report.md");
    fs::write(&receipt_path, "{}")?;
    fs::write(&report_path, "# Report")?;

    let result = create_test_result(receipt_path, report_path);
    let generator = EvidenceBundleGenerator::new(output_dir.clone())
        .with_compression();

    let bundle_path = generator.generate(&result, &workspace_root())?;

    // Should return path to .tar.gz file
    assert!(bundle_path.extension().map(|e| e == "gz").unwrap_or(false));
    assert!(bundle_path.exists());
    assert!(bundle_path.is_file());

    // Original directory should be removed
    let bundle_name = bundle_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap()
        .trim_end_matches(".tar");
    let original_dir = output_dir.join(bundle_name);
    assert!(!original_dir.exists(), "Original directory should be removed after compression");

    Ok(())
}

#[test]
fn test_bundle_handles_missing_receipt() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = temp_dir.path().join("dod-evidence");

    // Only create report, not receipt
    let receipt_path = temp_dir.path().join("nonexistent_receipt.json");
    let report_path = temp_dir.path().join("report.md");
    fs::write(&report_path, "# Report")?;

    let result = create_test_result(receipt_path, report_path);
    let generator = EvidenceBundleGenerator::new(output_dir);

    // Should not fail
    let bundle_path = generator.generate(&result, &workspace_root())?;

    // Bundle should exist
    assert!(bundle_path.exists());

    // Report should be present
    assert!(bundle_path.join("report.md").exists());

    // Receipt should not be present (was missing)
    assert!(!bundle_path.join("receipt.json").exists());

    Ok(())
}

#[test]
fn test_validate_inputs_rejects_nonexistent_workspace() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("dod-evidence");

    let receipt_path = temp_dir.path().join("receipt.json");
    let report_path = temp_dir.path().join("report.md");

    let result = create_test_result(receipt_path, report_path);
    let generator = EvidenceBundleGenerator::new(output_dir);

    let nonexistent = PathBuf::from("/nonexistent/workspace/path");
    let validation = generator.generate(&result, &nonexistent);

    assert!(validation.is_err());
    let error_msg = validation.unwrap_err().to_string();
    assert!(error_msg.contains("does not exist"));
}

#[test]
fn test_bundle_preserves_directory_structure_for_artifacts() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = temp_dir.path().join("dod-evidence");

    let receipt_path = temp_dir.path().join("receipt.json");
    let report_path = temp_dir.path().join("report.md");
    fs::write(&receipt_path, "{}")?;
    fs::write(&report_path, "# Report")?;

    let result = create_test_result(receipt_path, report_path);
    let generator = EvidenceBundleGenerator::new(output_dir);

    let bundle_path = generator.generate(&result, &workspace_root())?;

    // Check if ontology directory structure is preserved (if it exists)
    let ontology_artifact = bundle_path.join("artifacts/ontology/mcp-domain.ttl");
    if workspace_root().join("ontology/mcp-domain.ttl").exists() {
        assert!(ontology_artifact.exists(), "Directory structure should be preserved");

        // Parent directory should exist
        assert!(ontology_artifact.parent().unwrap().exists());
    }

    Ok(())
}

#[test]
fn test_manifest_total_size_matches_sum() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = temp_dir.path().join("dod-evidence");

    let receipt_path = temp_dir.path().join("receipt.json");
    let report_path = temp_dir.path().join("report.md");
    fs::write(&receipt_path, "{}")?;
    fs::write(&report_path, "# Report")?;

    let result = create_test_result(receipt_path, report_path);
    let generator = EvidenceBundleGenerator::new(output_dir);

    let bundle_path = generator.generate(&result, &workspace_root())?;

    // Read manifest
    let manifest_path = bundle_path.join("manifest.json");
    let manifest_content = fs::read_to_string(&manifest_path)?;
    let manifest: serde_json::Value = serde_json::from_str(&manifest_content)?;

    let files = manifest["files"].as_object().unwrap();
    let total_from_files: u64 = files
        .values()
        .map(|f| f["size_bytes"].as_u64().unwrap())
        .sum();

    let total_in_manifest = manifest["total_size_bytes"].as_u64().unwrap();

    assert_eq!(total_from_files, total_in_manifest, "Total size should match sum of individual files");

    Ok(())
}

#[test]
fn test_file_types_are_correctly_categorized() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_dir = temp_dir.path().join("dod-evidence");

    let receipt_path = temp_dir.path().join("receipt.json");
    let report_path = temp_dir.path().join("report.md");
    fs::write(&receipt_path, "{}")?;
    fs::write(&report_path, "# Report")?;

    let result = create_test_result(receipt_path, report_path);
    let generator = EvidenceBundleGenerator::new(output_dir);

    let bundle_path = generator.generate(&result, &workspace_root())?;

    // Read manifest
    let manifest_path = bundle_path.join("manifest.json");
    let manifest_content = fs::read_to_string(&manifest_path)?;
    let manifest: serde_json::Value = serde_json::from_str(&manifest_content)?;

    let files = manifest["files"].as_object().unwrap();

    // Verify file types
    assert_eq!(files["receipt.json"]["file_type"], "Receipt");
    assert_eq!(files["report.md"]["file_type"], "Report");

    // Log files should be categorized as Log
    for (path, entry) in files {
        if path.starts_with("logs/") {
            assert_eq!(entry["file_type"], "Log", "Log files should have Log type");
        }
        if path.starts_with("artifacts/") {
            assert_eq!(entry["file_type"], "Artifact", "Artifact files should have Artifact type");
        }
    }

    Ok(())
}
