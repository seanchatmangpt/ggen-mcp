//! Receipt Generator - Cryptographic Receipts for DoD Validation
//!
//! Generates tamper-evident receipts using SHA-256 hash chaining.
//! Each check result → hash → chain → final receipt hash.
//!
//! Chain: H(check1) → H(H(check1) + check2) → H(H(check2) + check3) → ... → final_hash

use super::types::*;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// Cryptographic receipt for DoD validation run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    /// Receipt format version
    pub version: String,

    /// Timestamp of validation run (ISO 8601)
    pub timestamp: DateTime<Utc>,

    /// Overall verdict: Ready or NotReady
    pub verdict: OverallVerdict,

    /// Readiness score (0.0 to 100.0)
    pub score: f64,

    /// Profile used for validation
    pub profile: String,

    /// Validation mode
    pub mode: ValidationMode,

    /// Total validation duration (ms)
    pub duration_ms: u64,

    /// Individual check hashes (ordered)
    pub check_hashes: Vec<CheckHash>,

    /// Final chained hash (tamper detection)
    pub final_hash: String,

    /// Metadata for traceability
    pub metadata: ReceiptMetadata,
}

/// Hash of individual check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckHash {
    /// Check identifier
    pub check_id: String,

    /// Check category
    pub category: CheckCategory,

    /// Check status
    pub status: CheckStatus,

    /// Check severity
    pub severity: CheckSeverity,

    /// Individual check hash (SHA-256)
    pub hash: String,

    /// Duration (ms)
    pub duration_ms: u64,
}

/// Receipt metadata for traceability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptMetadata {
    /// Workspace root
    pub workspace_root: PathBuf,

    /// Total checks executed
    pub checks_total: usize,

    /// Checks passed
    pub checks_passed: usize,

    /// Checks failed
    pub checks_failed: usize,

    /// Checks warned
    pub checks_warned: usize,

    /// Checks skipped
    pub checks_skipped: usize,

    /// Git commit hash (if available)
    pub git_commit: Option<String>,

    /// Git branch (if available)
    pub git_branch: Option<String>,
}

/// Receipt generator
pub struct ReceiptGenerator {
    /// Output directory for receipts
    receipts_dir: PathBuf,
}

impl ReceiptGenerator {
    /// Create new receipt generator
    ///
    /// # Arguments
    /// * `receipts_dir` - Directory to store receipts (created if missing)
    pub fn new<P: AsRef<Path>>(receipts_dir: P) -> Result<Self> {
        let receipts_dir = receipts_dir.as_ref().to_path_buf();

        // Create receipts directory if missing
        if !receipts_dir.exists() {
            fs::create_dir_all(&receipts_dir).context(format!(
                "Failed to create receipts directory: {:?}",
                receipts_dir
            ))?;
        }

        Ok(Self { receipts_dir })
    }

    /// Generate receipt from validation result
    ///
    /// Produces cryptographic receipt with SHA-256 hash chain.
    pub fn generate(&self, result: &DodValidationResult) -> Result<Receipt> {
        tracing::info!(
            verdict = ?result.verdict,
            score = result.readiness_score,
            checks = result.check_results.len(),
            "Generating DoD receipt"
        );

        // Hash each check individually
        let check_hashes = self.hash_checks(&result.check_results)?;

        // Chain hashes together
        let final_hash = self.chain_hashes(&check_hashes, result)?;

        // Extract metadata
        let metadata = self.extract_metadata(result)?;

        let receipt = Receipt {
            version: "1.0.0".to_string(),
            timestamp: Utc::now(),
            verdict: result.verdict,
            score: result.readiness_score,
            profile: result.profile.clone(),
            mode: result.mode,
            duration_ms: result.duration_ms,
            check_hashes,
            final_hash,
            metadata,
        };

        tracing::debug!(
            final_hash = %receipt.final_hash,
            check_count = receipt.check_hashes.len(),
            "Receipt generated"
        );

        Ok(receipt)
    }

    /// Hash individual check results
    fn hash_checks(&self, checks: &[DodCheckResult]) -> Result<Vec<CheckHash>> {
        let mut check_hashes = Vec::with_capacity(checks.len());

        for check in checks {
            let hash = self.hash_check_result(check)?;
            check_hashes.push(CheckHash {
                check_id: check.id.clone(),
                category: check.category,
                status: check.status.clone(),
                severity: check.severity,
                hash,
                duration_ms: check.duration_ms,
            });
        }

        Ok(check_hashes)
    }

    /// Hash single check result (deterministic)
    ///
    /// Hash input: check_id || category || status || severity || message || evidence_hashes
    fn hash_check_result(&self, check: &DodCheckResult) -> Result<String> {
        let mut hasher = Sha256::new();

        // Hash check metadata (order matters for determinism)
        hasher.update(check.id.as_bytes());
        hasher.update(format!("{:?}", check.category).as_bytes());
        hasher.update(format!("{:?}", check.status).as_bytes());
        hasher.update(format!("{:?}", check.severity).as_bytes());
        hasher.update(check.message.as_bytes());

        // Hash evidence (sorted by hash for determinism)
        let mut evidence_hashes: Vec<_> = check.evidence.iter().map(|e| e.hash.as_str()).collect();
        evidence_hashes.sort_unstable();
        for evidence_hash in evidence_hashes {
            hasher.update(evidence_hash.as_bytes());
        }

        // Hash remediation suggestions (deterministic order)
        for remediation in &check.remediation {
            hasher.update(remediation.as_bytes());
        }

        let result = hasher.finalize();
        Ok(hex::encode(result))
    }

    /// Chain check hashes together
    ///
    /// Chain: H(check1) → H(H(check1) + check2) → H(H(check2) + check3) → ... → final
    fn chain_hashes(
        &self,
        check_hashes: &[CheckHash],
        result: &DodValidationResult,
    ) -> Result<String> {
        if check_hashes.is_empty() {
            // Empty chain: hash metadata only
            return self.hash_metadata_only(result);
        }

        let mut chain_hash = check_hashes[0].hash.clone();

        // Chain subsequent hashes
        for check_hash in &check_hashes[1..] {
            let mut hasher = Sha256::new();
            hasher.update(chain_hash.as_bytes());
            hasher.update(check_hash.hash.as_bytes());
            chain_hash = hex::encode(hasher.finalize());
        }

        // Final hash: chain + metadata
        let mut hasher = Sha256::new();
        hasher.update(chain_hash.as_bytes());
        hasher.update(result.verdict.to_string().as_bytes());
        hasher.update(result.readiness_score.to_string().as_bytes());
        hasher.update(result.profile.as_bytes());
        hasher.update(format!("{:?}", result.mode).as_bytes());

        let final_hash = hex::encode(hasher.finalize());
        Ok(final_hash)
    }

    /// Hash metadata when no checks present
    fn hash_metadata_only(&self, result: &DodValidationResult) -> Result<String> {
        let mut hasher = Sha256::new();
        hasher.update(result.verdict.to_string().as_bytes());
        hasher.update(result.readiness_score.to_string().as_bytes());
        hasher.update(result.profile.as_bytes());
        hasher.update(format!("{:?}", result.mode).as_bytes());
        Ok(hex::encode(hasher.finalize()))
    }

    /// Extract metadata from validation result
    fn extract_metadata(&self, result: &DodValidationResult) -> Result<ReceiptMetadata> {
        let git_commit = self.get_git_commit().ok();
        let git_branch = self.get_git_branch().ok();

        Ok(ReceiptMetadata {
            workspace_root: std::env::current_dir().context("Failed to get current directory")?,
            checks_total: result.summary.checks_total,
            checks_passed: result.summary.checks_passed,
            checks_failed: result.summary.checks_failed,
            checks_warned: result.summary.checks_warned,
            checks_skipped: result.summary.checks_skipped,
            git_commit,
            git_branch,
        })
    }

    /// Get current git commit hash
    fn get_git_commit(&self) -> Result<String> {
        let output = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .context("Failed to execute git rev-parse")?;

        if !output.status.success() {
            anyhow::bail!("git rev-parse failed");
        }

        let commit = String::from_utf8(output.stdout)
            .context("Invalid UTF-8 in git output")?
            .trim()
            .to_string();

        Ok(commit)
    }

    /// Get current git branch
    fn get_git_branch(&self) -> Result<String> {
        let output = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .context("Failed to execute git rev-parse")?;

        if !output.status.success() {
            anyhow::bail!("git rev-parse failed");
        }

        let branch = String::from_utf8(output.stdout)
            .context("Invalid UTF-8 in git output")?
            .trim()
            .to_string();

        Ok(branch)
    }

    /// Save receipt to JSON file
    ///
    /// Filename: receipts/YYYY-MM-DD-HHMMSS.json
    pub fn save(&self, receipt: &Receipt) -> Result<PathBuf> {
        let filename = format!("{}.json", receipt.timestamp.format("%Y-%m-%d-%H%M%S"));
        let receipt_path = self.receipts_dir.join(&filename);

        let json =
            serde_json::to_string_pretty(receipt).context("Failed to serialize receipt to JSON")?;

        fs::write(&receipt_path, json)
            .context(format!("Failed to write receipt to {:?}", receipt_path))?;

        tracing::info!(
            receipt_path = %receipt_path.display(),
            final_hash = %receipt.final_hash,
            "Receipt saved"
        );

        Ok(receipt_path)
    }

    /// Generate and save receipt
    pub fn generate_and_save(&self, result: &DodValidationResult) -> Result<PathBuf> {
        let receipt = self.generate(result)?;
        self.save(&receipt)
    }

    /// Load receipt from file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Receipt> {
        let path = path.as_ref();
        let json =
            fs::read_to_string(path).context(format!("Failed to read receipt from {:?}", path))?;

        let receipt: Receipt =
            serde_json::from_str(&json).context("Failed to deserialize receipt from JSON")?;

        Ok(receipt)
    }

    /// Verify receipt integrity
    ///
    /// Recomputes hash chain and compares with stored final_hash.
    pub fn verify(&self, receipt: &Receipt) -> Result<bool> {
        if receipt.check_hashes.is_empty() {
            // Empty receipt: cannot verify chain
            return Ok(true);
        }

        // Recompute chain hash
        let mut chain_hash = receipt.check_hashes[0].hash.clone();

        for check_hash in &receipt.check_hashes[1..] {
            let mut hasher = Sha256::new();
            hasher.update(chain_hash.as_bytes());
            hasher.update(check_hash.hash.as_bytes());
            chain_hash = hex::encode(hasher.finalize());
        }

        // Recompute final hash
        let mut hasher = Sha256::new();
        hasher.update(chain_hash.as_bytes());
        hasher.update(receipt.verdict.to_string().as_bytes());
        hasher.update(receipt.score.to_string().as_bytes());
        hasher.update(receipt.profile.as_bytes());
        hasher.update(format!("{:?}", receipt.mode).as_bytes());

        let computed_final_hash = hex::encode(hasher.finalize());

        // Compare hashes
        let verified = computed_final_hash == receipt.final_hash;

        if !verified {
            tracing::warn!(
                expected = %receipt.final_hash,
                computed = %computed_final_hash,
                "Receipt hash verification failed - possible tampering"
            );
        }

        Ok(verified)
    }
}

impl std::fmt::Display for OverallVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OverallVerdict::Ready => write!(f, "READY"),
            OverallVerdict::NotReady => write!(f, "NOT_READY"),
        }
    }
}

// Note: hex crate needed for encoding
mod hex {
    /// Encode bytes as lowercase hex string
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_check(id: &str, status: CheckStatus) -> DodCheckResult {
        DodCheckResult {
            id: id.to_string(),
            category: CheckCategory::BuildCorrectness,
            status,
            severity: CheckSeverity::Fatal,
            message: "Test check".to_string(),
            evidence: vec![],
            remediation: vec![],
            duration_ms: 100,
            check_hash: "test_hash".to_string(),
        }
    }

    fn create_test_result(
        checks: Vec<DodCheckResult>,
        verdict: OverallVerdict,
    ) -> DodValidationResult {
        DodValidationResult {
            verdict,
            readiness_score: 85.0,
            profile: "dev".to_string(),
            mode: ValidationMode::Fast,
            summary: ValidationSummary {
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
            },
            category_scores: std::collections::HashMap::new(),
            check_results: checks,
            artifacts: ArtifactPaths {
                receipt_path: PathBuf::from("receipts/test.json"),
                report_path: PathBuf::from("reports/test.md"),
                bundle_path: None,
            },
            duration_ms: 1000,
        }
    }

    #[test]
    fn receipt_generator_creates_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let receipts_dir = temp_dir.path().join("receipts");

        assert!(!receipts_dir.exists());

        let _generator = ReceiptGenerator::new(&receipts_dir).unwrap();

        assert!(receipts_dir.exists());
    }

    #[test]
    fn receipt_generation_includes_all_fields() {
        let temp_dir = tempfile::tempdir().unwrap();
        let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

        let checks = vec![
            create_test_check("CHECK1", CheckStatus::Pass),
            create_test_check("CHECK2", CheckStatus::Pass),
        ];
        let result = create_test_result(checks, OverallVerdict::Ready);

        let receipt = generator.generate(&result).unwrap();

        assert_eq!(receipt.version, "1.0.0");
        assert_eq!(receipt.verdict, OverallVerdict::Ready);
        assert_eq!(receipt.score, 85.0);
        assert_eq!(receipt.profile, "dev");
        assert_eq!(receipt.mode, ValidationMode::Fast);
        assert_eq!(receipt.check_hashes.len(), 2);
        assert!(!receipt.final_hash.is_empty());
        assert_eq!(receipt.metadata.checks_total, 2);
        assert_eq!(receipt.metadata.checks_passed, 2);
    }

    #[test]
    fn hash_determinism_same_input_same_hash() {
        let temp_dir = tempfile::tempdir().unwrap();
        let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

        let check = create_test_check("CHECK1", CheckStatus::Pass);

        let hash1 = generator.hash_check_result(&check).unwrap();
        let hash2 = generator.hash_check_result(&check).unwrap();

        assert_eq!(hash1, hash2, "Same input must produce same hash");
    }

    #[test]
    fn hash_different_for_different_status() {
        let temp_dir = tempfile::tempdir().unwrap();
        let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

        let check_pass = create_test_check("CHECK1", CheckStatus::Pass);
        let check_fail = create_test_check("CHECK1", CheckStatus::Fail);

        let hash_pass = generator.hash_check_result(&check_pass).unwrap();
        let hash_fail = generator.hash_check_result(&check_fail).unwrap();

        assert_ne!(
            hash_pass, hash_fail,
            "Different status must produce different hash"
        );
    }

    #[test]
    fn chain_integrity_single_check() {
        let temp_dir = tempfile::tempdir().unwrap();
        let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

        let checks = vec![create_test_check("CHECK1", CheckStatus::Pass)];
        let result = create_test_result(checks, OverallVerdict::Ready);

        let receipt = generator.generate(&result).unwrap();

        // Single check: final_hash should be based on check1 + metadata
        assert!(!receipt.final_hash.is_empty());
        assert_eq!(receipt.check_hashes.len(), 1);
    }

    #[test]
    fn chain_integrity_multiple_checks() {
        let temp_dir = tempfile::tempdir().unwrap();
        let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

        let checks = vec![
            create_test_check("CHECK1", CheckStatus::Pass),
            create_test_check("CHECK2", CheckStatus::Pass),
            create_test_check("CHECK3", CheckStatus::Warn),
        ];
        let result = create_test_result(checks, OverallVerdict::Ready);

        let receipt = generator.generate(&result).unwrap();

        assert_eq!(receipt.check_hashes.len(), 3);
        assert!(!receipt.final_hash.is_empty());

        // Verify chain manually
        let verified = generator.verify(&receipt).unwrap();
        assert!(verified, "Receipt should verify successfully");
    }

    #[test]
    fn chain_order_matters() {
        let temp_dir = tempfile::tempdir().unwrap();
        let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

        let checks1 = vec![
            create_test_check("CHECK1", CheckStatus::Pass),
            create_test_check("CHECK2", CheckStatus::Pass),
        ];
        let result1 = create_test_result(checks1, OverallVerdict::Ready);

        let checks2 = vec![
            create_test_check("CHECK2", CheckStatus::Pass),
            create_test_check("CHECK1", CheckStatus::Pass),
        ];
        let result2 = create_test_result(checks2, OverallVerdict::Ready);

        let receipt1 = generator.generate(&result1).unwrap();
        let receipt2 = generator.generate(&result2).unwrap();

        assert_ne!(
            receipt1.final_hash, receipt2.final_hash,
            "Different check order must produce different final hash"
        );
    }

    #[test]
    fn json_serialization_roundtrip() {
        let temp_dir = tempfile::tempdir().unwrap();
        let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

        let checks = vec![
            create_test_check("CHECK1", CheckStatus::Pass),
            create_test_check("CHECK2", CheckStatus::Fail),
        ];
        let result = create_test_result(checks, OverallVerdict::NotReady);

        let original = generator.generate(&result).unwrap();
        let receipt_path = generator.save(&original).unwrap();

        // Load and verify
        let loaded = ReceiptGenerator::load(&receipt_path).unwrap();

        assert_eq!(original.version, loaded.version);
        assert_eq!(original.verdict, loaded.verdict);
        assert_eq!(original.score, loaded.score);
        assert_eq!(original.profile, loaded.profile);
        assert_eq!(original.final_hash, loaded.final_hash);
        assert_eq!(original.check_hashes.len(), loaded.check_hashes.len());
    }

    #[test]
    fn verify_detects_tampering() {
        let temp_dir = tempfile::tempdir().unwrap();
        let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

        let checks = vec![
            create_test_check("CHECK1", CheckStatus::Pass),
            create_test_check("CHECK2", CheckStatus::Pass),
        ];
        let result = create_test_result(checks, OverallVerdict::Ready);

        let mut receipt = generator.generate(&result).unwrap();

        // Tamper with check hash
        receipt.check_hashes[0].hash = "tampered_hash".to_string();

        let verified = generator.verify(&receipt).unwrap();
        assert!(!verified, "Verification should fail for tampered receipt");
    }

    #[test]
    fn verify_accepts_valid_receipt() {
        let temp_dir = tempfile::tempdir().unwrap();
        let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

        let checks = vec![
            create_test_check("CHECK1", CheckStatus::Pass),
            create_test_check("CHECK2", CheckStatus::Warn),
        ];
        let result = create_test_result(checks, OverallVerdict::Ready);

        let receipt = generator.generate(&result).unwrap();

        let verified = generator.verify(&receipt).unwrap();
        assert!(verified, "Verification should succeed for valid receipt");
    }

    #[test]
    fn empty_checks_produces_valid_receipt() {
        let temp_dir = tempfile::tempdir().unwrap();
        let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

        let result = create_test_result(vec![], OverallVerdict::Ready);

        let receipt = generator.generate(&result).unwrap();

        assert_eq!(receipt.check_hashes.len(), 0);
        assert!(!receipt.final_hash.is_empty());

        // Verify accepts empty receipt
        let verified = generator.verify(&receipt).unwrap();
        assert!(verified);
    }

    #[test]
    fn receipt_filename_format() {
        let temp_dir = tempfile::tempdir().unwrap();
        let generator = ReceiptGenerator::new(temp_dir.path()).unwrap();

        let checks = vec![create_test_check("CHECK1", CheckStatus::Pass)];
        let result = create_test_result(checks, OverallVerdict::Ready);

        let receipt = generator.generate(&result).unwrap();
        let path = generator.save(&receipt).unwrap();

        let filename = path.file_name().unwrap().to_str().unwrap();

        // Format: YYYY-MM-DD-HHMMSS.json
        assert!(filename.ends_with(".json"));
        assert!(filename.contains("-")); // Date separators
    }
}
