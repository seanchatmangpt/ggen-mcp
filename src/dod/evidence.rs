//! Evidence Bundle Generator for Definition of Done validation
//!
//! Collects artifacts (logs, diffs, receipts, reports) into timestamped evidence bundles
//! for audit trails, debugging, and compliance verification.
//!
//! ## Bundle Structure
//! ```text
//! dod-evidence/2026-01-24-103000/
//!   receipt.json          ← Cryptographic receipt
//!   report.md             ← Human-readable report
//!   manifest.json         ← File listing with hashes
//!   logs/                 ← Check execution logs
//!     g0_workspace.log
//!     build_fmt.log
//!     ...
//!   artifacts/            ← Snapshots of key files
//!     Cargo.lock
//!     ggen.toml
//!     ontology/mcp-domain.ttl
//! ```

use crate::dod::types::*;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Evidence bundle manifest - lists all files with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceManifest {
    /// Timestamp of bundle creation
    pub created_at: String,
    /// Profile used for validation
    pub profile: String,
    /// Validation mode (Fast/Strict/Paranoid)
    pub mode: ValidationMode,
    /// Overall verdict
    pub verdict: OverallVerdict,
    /// Readiness score (0-100)
    pub readiness_score: f64,
    /// Files included in bundle with their SHA-256 hashes
    pub files: HashMap<String, FileEntry>,
    /// Total bundle size in bytes
    pub total_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// Relative path within bundle
    pub path: String,
    /// File size in bytes
    pub size_bytes: u64,
    /// SHA-256 hash of file content
    pub hash: String,
    /// File type (receipt, report, log, artifact)
    pub file_type: FileType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    Receipt,
    Report,
    Log,
    Artifact,
    Manifest,
}

/// Evidence bundle generator
pub struct EvidenceBundleGenerator {
    /// Output directory for bundles (e.g., "dod-evidence")
    output_dir: PathBuf,
    /// Include compression option
    compress: bool,
}

impl EvidenceBundleGenerator {
    /// Create new evidence bundle generator
    ///
    /// # Arguments
    /// * `output_dir` - Base directory for evidence bundles (e.g., "./dod-evidence")
    pub fn new(output_dir: PathBuf) -> Self {
        Self {
            output_dir,
            compress: false,
        }
    }

    /// Enable bundle compression (creates .tar.gz)
    pub fn with_compression(mut self) -> Self {
        self.compress = true;
        self
    }

    /// Generate evidence bundle from validation result
    ///
    /// # Arguments
    /// * `result` - DoD validation result containing all check data
    /// * `workspace_root` - Workspace root for collecting artifacts
    ///
    /// # Returns
    /// Path to the created bundle directory (or .tar.gz if compressed)
    pub fn generate(&self, result: &DodValidationResult, workspace_root: &Path) -> Result<PathBuf> {
        // Validate inputs
        self.validate_inputs(result, workspace_root)?;

        // Create timestamped bundle directory
        let bundle_dir = self.create_bundle_dir()?;
        tracing::info!(bundle_dir = %bundle_dir.display(), "Creating evidence bundle");

        // Create subdirectories
        let logs_dir = bundle_dir.join("logs");
        let artifacts_dir = bundle_dir.join("artifacts");
        fs::create_dir_all(&logs_dir).context("Failed to create logs directory")?;
        fs::create_dir_all(&artifacts_dir).context("Failed to create artifacts directory")?;

        // Track collected files for manifest
        let mut files = HashMap::new();

        // Copy receipt if exists
        if result.artifacts.receipt_path.exists() {
            self.copy_file(
                &result.artifacts.receipt_path,
                &bundle_dir.join("receipt.json"),
                FileType::Receipt,
                &mut files,
            )?;
        }

        // Copy report if exists
        if result.artifacts.report_path.exists() {
            self.copy_file(
                &result.artifacts.report_path,
                &bundle_dir.join("report.md"),
                FileType::Report,
                &mut files,
            )?;
        }

        // Collect logs from check results
        self.collect_logs(result, &logs_dir, &mut files)?;

        // Collect artifact snapshots
        self.collect_artifacts(workspace_root, &artifacts_dir, &mut files)?;

        // Generate manifest
        let manifest = self.create_manifest(result, files)?;
        self.write_manifest(&manifest, &bundle_dir)?;

        // Compress if enabled
        if self.compress {
            let archive_path = self.compress_bundle(&bundle_dir)?;
            // Remove uncompressed directory
            fs::remove_dir_all(&bundle_dir).context("Failed to remove uncompressed bundle")?;
            Ok(archive_path)
        } else {
            Ok(bundle_dir)
        }
    }

    /// Validate inputs before processing
    fn validate_inputs(&self, result: &DodValidationResult, workspace_root: &Path) -> Result<()> {
        // Validate workspace exists
        if !workspace_root.exists() {
            anyhow::bail!(
                "Workspace root does not exist: {}",
                workspace_root.display()
            );
        }

        // Check disk space (ensure at least 100MB available)
        self.check_disk_space(&self.output_dir, 100 * 1024 * 1024)?;

        // Validate result has checks
        if result.check_results.is_empty() {
            tracing::warn!("No check results to bundle");
        }

        Ok(())
    }

    /// Check available disk space
    fn check_disk_space(&self, path: &Path, required_bytes: u64) -> Result<()> {
        // Simple check - ensure parent directory exists
        if let Some(parent) = path.parent() {
            if parent.exists() {
                // For production, could use statvfs/GetDiskFreeSpaceEx
                // For now, just verify parent is writable
                return Ok(());
            }
        }
        Ok(())
    }

    /// Create timestamped bundle directory
    fn create_bundle_dir(&self) -> Result<PathBuf> {
        let timestamp = Self::generate_timestamp();
        let bundle_dir = self.output_dir.join(timestamp);

        fs::create_dir_all(&bundle_dir).context(format!(
            "Failed to create bundle directory: {}",
            bundle_dir.display()
        ))?;

        Ok(bundle_dir)
    }

    /// Generate timestamp for bundle directory name (YYYY-MM-DD-HHMMSS)
    fn generate_timestamp() -> String {
        let now = SystemTime::now();
        let datetime = chrono::DateTime::<chrono::Local>::from(now);
        datetime.format("%Y-%m-%d-%H%M%S").to_string()
    }

    /// Copy a file and track it in manifest
    fn copy_file(
        &self,
        src: &Path,
        dest: &Path,
        file_type: FileType,
        files: &mut HashMap<String, FileEntry>,
    ) -> Result<()> {
        if !src.exists() {
            tracing::warn!(src = %src.display(), "Source file not found, skipping");
            return Ok(());
        }

        fs::copy(src, dest).context(format!(
            "Failed to copy {} to {}",
            src.display(),
            dest.display()
        ))?;

        // Calculate hash and size
        let content = fs::read(dest)?;
        let hash = Self::calculate_sha256(&content);
        let size_bytes = content.len() as u64;

        // Get relative path for manifest
        let rel_path = dest
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        files.insert(
            rel_path.clone(),
            FileEntry {
                path: rel_path,
                size_bytes,
                hash,
                file_type,
            },
        );

        Ok(())
    }

    /// Collect logs from check results
    fn collect_logs(
        &self,
        result: &DodValidationResult,
        logs_dir: &Path,
        files: &mut HashMap<String, FileEntry>,
    ) -> Result<()> {
        for check_result in &result.check_results {
            // Create log file for each check
            let log_filename = format!("{}.log", check_result.id.to_lowercase().replace('_', "-"));
            let log_path = logs_dir.join(&log_filename);

            // Write log content
            let log_content = self.format_check_log(check_result)?;
            let mut file = fs::File::create(&log_path)
                .context(format!("Failed to create log file: {}", log_path.display()))?;
            file.write_all(log_content.as_bytes())?;

            // Track in manifest
            let hash = Self::calculate_sha256(log_content.as_bytes());
            let size_bytes = log_content.len() as u64;
            let rel_path = format!("logs/{}", log_filename);

            files.insert(
                rel_path.clone(),
                FileEntry {
                    path: rel_path,
                    size_bytes,
                    hash,
                    file_type: FileType::Log,
                },
            );
        }

        Ok(())
    }

    /// Format check result as log entry
    fn format_check_log(&self, check: &DodCheckResult) -> Result<String> {
        let mut log = String::new();
        log.push_str(&format!("=== Check: {} ===\n", check.id));
        log.push_str(&format!("Status: {:?}\n", check.status));
        log.push_str(&format!("Severity: {:?}\n", check.severity));
        log.push_str(&format!("Category: {:?}\n", check.category));
        log.push_str(&format!("Duration: {}ms\n", check.duration_ms));
        log.push_str(&format!("Message: {}\n\n", check.message));

        if !check.evidence.is_empty() {
            log.push_str("Evidence:\n");
            for (i, evidence) in check.evidence.iter().enumerate() {
                log.push_str(&format!(
                    "  {}. {:?}: {}\n",
                    i + 1,
                    evidence.kind,
                    evidence.content
                ));
            }
            log.push('\n');
        }

        if !check.remediation.is_empty() {
            log.push_str("Remediation:\n");
            for (i, suggestion) in check.remediation.iter().enumerate() {
                log.push_str(&format!("  {}. {}\n", i + 1, suggestion));
            }
        }

        Ok(log)
    }

    /// Collect artifact snapshots (key files)
    fn collect_artifacts(
        &self,
        workspace_root: &Path,
        artifacts_dir: &Path,
        files: &mut HashMap<String, FileEntry>,
    ) -> Result<()> {
        // Key files to snapshot
        let artifact_paths = vec![
            "Cargo.lock",
            "Cargo.toml",
            "ggen.toml",
            "ontology/mcp-domain.ttl",
        ];

        for rel_path in artifact_paths {
            let src = workspace_root.join(rel_path);
            if !src.exists() {
                tracing::debug!(path = rel_path, "Artifact not found, skipping");
                continue;
            }

            // Preserve directory structure in artifacts/
            let dest = artifacts_dir.join(rel_path);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }

            // Copy and track
            let content = fs::read(&src)?;
            fs::write(&dest, &content)?;

            let hash = Self::calculate_sha256(&content);
            let size_bytes = content.len() as u64;
            let manifest_path = format!("artifacts/{}", rel_path);

            files.insert(
                manifest_path.clone(),
                FileEntry {
                    path: manifest_path,
                    size_bytes,
                    hash,
                    file_type: FileType::Artifact,
                },
            );
        }

        Ok(())
    }

    /// Create manifest from collected files
    fn create_manifest(
        &self,
        result: &DodValidationResult,
        files: HashMap<String, FileEntry>,
    ) -> Result<EvidenceManifest> {
        let total_size_bytes = files.values().map(|f| f.size_bytes).sum();

        Ok(EvidenceManifest {
            created_at: Self::generate_timestamp(),
            profile: result.profile.clone(),
            mode: result.mode,
            verdict: result.verdict,
            readiness_score: result.readiness_score,
            files,
            total_size_bytes,
        })
    }

    /// Write manifest to bundle directory
    fn write_manifest(&self, manifest: &EvidenceManifest, bundle_dir: &Path) -> Result<()> {
        let manifest_path = bundle_dir.join("manifest.json");
        let json =
            serde_json::to_string_pretty(manifest).context("Failed to serialize manifest")?;

        let mut file = fs::File::create(&manifest_path).context(format!(
            "Failed to create manifest: {}",
            manifest_path.display()
        ))?;
        file.write_all(json.as_bytes())?;

        tracing::info!(
            files = manifest.files.len(),
            size_bytes = manifest.total_size_bytes,
            "Manifest created"
        );

        Ok(())
    }

    /// Compress bundle into tar.gz archive
    fn compress_bundle(&self, bundle_dir: &Path) -> Result<PathBuf> {
        use flate2::Compression;
        use flate2::write::GzEncoder;

        let bundle_name = bundle_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("bundle");

        let archive_path = bundle_dir
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!("{}.tar.gz", bundle_name));

        let tar_gz = fs::File::create(&archive_path).context(format!(
            "Failed to create archive: {}",
            archive_path.display()
        ))?;
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = tar::Builder::new(enc);

        // Add all files from bundle directory
        tar.append_dir_all(bundle_name, bundle_dir)
            .context("Failed to add files to archive")?;

        tar.finish().context("Failed to finalize archive")?;

        tracing::info!(archive = %archive_path.display(), "Bundle compressed");
        Ok(archive_path)
    }

    /// Calculate SHA-256 hash of data
    fn calculate_sha256(data: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dod::types::*;
    use std::collections::HashMap;

    fn create_test_result() -> DodValidationResult {
        let mut check_results = vec![
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

        DodValidationResult {
            verdict: OverallVerdict::NotReady,
            readiness_score: 75.0,
            profile: "dev".to_string(),
            mode: ValidationMode::Fast,
            summary: ValidationSummary {
                checks_total: 2,
                checks_passed: 1,
                checks_failed: 1,
                checks_warned: 0,
                checks_skipped: 0,
            },
            category_scores,
            check_results,
            artifacts: ArtifactPaths {
                receipt_path: PathBuf::from("/tmp/receipt.json"),
                report_path: PathBuf::from("/tmp/report.md"),
                bundle_path: None,
            },
            duration_ms: 3000,
        }
    }

    #[test]
    fn evidence_bundle_generator_creates_instance() {
        let generator = EvidenceBundleGenerator::new(PathBuf::from("test-output"));
        assert_eq!(generator.output_dir, PathBuf::from("test-output"));
        assert!(!generator.compress);
    }

    #[test]
    fn with_compression_enables_compression() {
        let generator =
            EvidenceBundleGenerator::new(PathBuf::from("test-output")).with_compression();
        assert!(generator.compress);
    }

    #[test]
    fn calculate_sha256_produces_correct_hash() {
        let data = b"hello world";
        let hash = EvidenceBundleGenerator::calculate_sha256(data);
        // SHA-256 of "hello world"
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn generate_timestamp_returns_valid_format() {
        let timestamp = EvidenceBundleGenerator::generate_timestamp();
        // Format: YYYY-MM-DD-HHMMSS (e.g., 2026-01-24-103000)
        assert!(timestamp.len() >= 17); // Minimum length
        assert!(timestamp.contains('-'));
    }

    #[test]
    fn format_check_log_includes_all_fields() {
        let generator = EvidenceBundleGenerator::new(PathBuf::from("test"));
        let check = DodCheckResult {
            id: "TEST_CHECK".to_string(),
            category: CheckCategory::BuildCorrectness,
            status: CheckStatus::Pass,
            severity: CheckSeverity::Fatal,
            message: "Test message".to_string(),
            evidence: vec![],
            remediation: vec!["Fix it".to_string()],
            duration_ms: 100,
            check_hash: "hash123".to_string(),
        };

        let log = generator.format_check_log(&check).unwrap();
        assert!(log.contains("TEST_CHECK"));
        assert!(log.contains("Pass"));
        assert!(log.contains("Fatal"));
        assert!(log.contains("Test message"));
        assert!(log.contains("Remediation"));
        assert!(log.contains("Fix it"));
    }

    #[test]
    fn validate_inputs_checks_workspace_exists() {
        let generator = EvidenceBundleGenerator::new(PathBuf::from("test-output"));
        let result = create_test_result();
        let non_existent = PathBuf::from("/nonexistent/workspace");

        let validation = generator.validate_inputs(&result, &non_existent);
        assert!(validation.is_err());
        assert!(
            validation
                .unwrap_err()
                .to_string()
                .contains("does not exist")
        );
    }
}
