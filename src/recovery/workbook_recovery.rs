//! Recovery strategies for corrupted workbook state

use anyhow::{Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, error, warn};

/// Corruption detection and recovery
pub struct CorruptionDetector {
    /// Minimum file size to be considered valid (in bytes)
    pub min_file_size: u64,
    /// Maximum file size to process (in bytes)
    pub max_file_size: u64,
}

impl Default for CorruptionDetector {
    fn default() -> Self {
        Self {
            min_file_size: 100,               // 100 bytes minimum
            max_file_size: 500 * 1024 * 1024, // 500 MB maximum
        }
    }
}

impl CorruptionDetector {
    /// Check if a workbook file appears corrupted
    pub fn detect_corruption(&self, path: &Path) -> Result<CorruptionStatus> {
        if !path.exists() {
            return Ok(CorruptionStatus::Missing);
        }

        let metadata = fs::metadata(path)
            .map_err(|e| anyhow!("failed to read file metadata for {:?}: {}", path, e))?;

        if !metadata.is_file() {
            return Ok(CorruptionStatus::NotAFile);
        }

        let file_size = metadata.len();

        if file_size < self.min_file_size {
            warn!(
                path = ?path,
                size = file_size,
                min_size = self.min_file_size,
                "file too small, likely corrupted"
            );
            return Ok(CorruptionStatus::TooSmall { actual: file_size });
        }

        if file_size > self.max_file_size {
            warn!(
                path = ?path,
                size = file_size,
                max_size = self.max_file_size,
                "file too large"
            );
            return Ok(CorruptionStatus::TooLarge { actual: file_size });
        }

        // Check file extension
        if !self.is_supported_extension(path) {
            return Ok(CorruptionStatus::UnsupportedFormat);
        }

        // Try to read first few bytes to check magic numbers
        match self.check_file_signature(path) {
            Ok(true) => Ok(CorruptionStatus::Healthy),
            Ok(false) => Ok(CorruptionStatus::InvalidSignature),
            Err(e) => {
                warn!(path = ?path, error = %e, "failed to check file signature");
                Ok(CorruptionStatus::Unknown)
            }
        }
    }

    fn is_supported_extension(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                let lower = ext.to_ascii_lowercase();
                lower == "xlsx" || lower == "xlsm" || lower == "xls"
            })
            .unwrap_or(false)
    }

    fn check_file_signature(&self, path: &Path) -> Result<bool> {
        let file = fs::File::open(path)?;
        let mut reader = std::io::BufReader::new(file);

        // Check for ZIP signature (XLSX/XLSM are ZIP files)
        let mut header = [0u8; 4];
        use std::io::Read;
        reader.read_exact(&mut header)?;

        // ZIP file signature: 0x504B0304 or 0x504B0506
        if header == [0x50, 0x4B, 0x03, 0x04] || header == [0x50, 0x4B, 0x05, 0x06] {
            return Ok(true);
        }

        // Legacy XLS file signature: 0xD0CF11E0A1B11AE1
        if header[0..2] == [0xD0, 0xCF] {
            return Ok(true);
        }

        Ok(false)
    }
}

/// Status of workbook corruption check
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorruptionStatus {
    /// File is healthy
    Healthy,
    /// File does not exist
    Missing,
    /// Path is not a file
    NotAFile,
    /// File is too small to be valid
    TooSmall { actual: u64 },
    /// File is too large to process
    TooLarge { actual: u64 },
    /// File has invalid signature
    InvalidSignature,
    /// File format is not supported
    UnsupportedFormat,
    /// Unable to determine status
    Unknown,
}

impl CorruptionStatus {
    pub fn is_healthy(&self) -> bool {
        matches!(self, CorruptionStatus::Healthy)
    }

    pub fn is_recoverable(&self) -> bool {
        matches!(self, CorruptionStatus::Healthy | CorruptionStatus::Unknown)
    }
}

/// Recovery action to take for corrupted workbooks
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryAction {
    /// No recovery needed
    None,
    /// Restore from backup
    RestoreFromBackup { backup_path: PathBuf },
    /// Recreate the file
    Recreate,
    /// Evict from cache and reload
    EvictAndReload,
    /// Mark as permanently corrupted
    MarkCorrupted,
    /// Skip and use fallback
    UseFallback,
}

/// Workbook recovery strategy
pub struct WorkbookRecoveryStrategy {
    detector: CorruptionDetector,
    backup_enabled: bool,
}

impl Default for WorkbookRecoveryStrategy {
    fn default() -> Self {
        Self {
            detector: CorruptionDetector::default(),
            backup_enabled: false,
        }
    }
}

impl WorkbookRecoveryStrategy {
    pub fn new(backup_enabled: bool) -> Self {
        Self {
            detector: CorruptionDetector::default(),
            backup_enabled,
        }
    }

    /// Determine recovery action for a workbook
    pub fn determine_action(&self, path: &Path) -> Result<RecoveryAction> {
        let status = self.detector.detect_corruption(path)?;

        debug!(path = ?path, status = ?status, "detected corruption status");

        match status {
            CorruptionStatus::Healthy => Ok(RecoveryAction::None),

            CorruptionStatus::Missing => {
                if self.backup_enabled {
                    if let Some(backup) = self.find_backup(path) {
                        return Ok(RecoveryAction::RestoreFromBackup {
                            backup_path: backup,
                        });
                    }
                }
                Ok(RecoveryAction::MarkCorrupted)
            }

            CorruptionStatus::InvalidSignature | CorruptionStatus::TooSmall { .. } => {
                if self.backup_enabled {
                    if let Some(backup) = self.find_backup(path) {
                        return Ok(RecoveryAction::RestoreFromBackup {
                            backup_path: backup,
                        });
                    }
                }
                Ok(RecoveryAction::UseFallback)
            }

            CorruptionStatus::TooLarge { .. } => Ok(RecoveryAction::MarkCorrupted),

            CorruptionStatus::NotAFile | CorruptionStatus::UnsupportedFormat => {
                Ok(RecoveryAction::MarkCorrupted)
            }

            CorruptionStatus::Unknown => Ok(RecoveryAction::EvictAndReload),
        }
    }

    /// Execute a recovery action
    pub fn execute_recovery(&self, path: &Path, action: RecoveryAction) -> Result<RecoveryResult> {
        debug!(path = ?path, action = ?action, "executing recovery action");

        match action {
            RecoveryAction::None => Ok(RecoveryResult::NoActionNeeded),

            RecoveryAction::RestoreFromBackup { backup_path } => {
                self.restore_from_backup(path, &backup_path)?;
                Ok(RecoveryResult::Restored {
                    from: backup_path.to_string_lossy().to_string(),
                })
            }

            RecoveryAction::EvictAndReload => {
                // This would be handled by the caller (evict from cache)
                Ok(RecoveryResult::ShouldReload)
            }

            RecoveryAction::MarkCorrupted => {
                error!(path = ?path, "marking workbook as permanently corrupted");
                Ok(RecoveryResult::Corrupted)
            }

            RecoveryAction::UseFallback => {
                warn!(path = ?path, "using fallback for corrupted workbook");
                Ok(RecoveryResult::UsingFallback)
            }

            RecoveryAction::Recreate => {
                // This would require external implementation
                Ok(RecoveryResult::RecreateNeeded)
            }
        }
    }

    fn find_backup(&self, _original_path: &Path) -> Option<PathBuf> {
        // In a real implementation, this would search for backup files
        // For now, return None
        None
    }

    fn restore_from_backup(&self, target: &Path, backup: &Path) -> Result<()> {
        debug!(target = ?target, backup = ?backup, "restoring from backup");

        if !backup.exists() {
            return Err(anyhow!("backup file not found: {:?}", backup));
        }

        fs::copy(backup, target)?;
        Ok(())
    }

    /// Create a backup of a workbook
    pub fn create_backup(&self, path: &Path) -> Result<PathBuf> {
        if !path.exists() {
            return Err(anyhow!("cannot backup non-existent file: {:?}", path));
        }

        let backup_path = self.get_backup_path(path);

        // Ensure backup directory exists
        if let Some(parent) = backup_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::copy(path, &backup_path)?;
        debug!(original = ?path, backup = ?backup_path, "created backup");

        Ok(backup_path)
    }

    fn get_backup_path(&self, original: &Path) -> PathBuf {
        let mut backup = original.to_path_buf();
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");

        if let Some(file_name) = original.file_name() {
            let backup_name = format!("{}.{}.backup", file_name.to_string_lossy(), timestamp);
            backup.set_file_name(backup_name);
        }

        backup
    }
}

/// Result of a recovery operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryResult {
    /// No action was needed
    NoActionNeeded,
    /// File was restored from backup
    Restored { from: String },
    /// File should be reloaded
    ShouldReload,
    /// File is corrupted and cannot be recovered
    Corrupted,
    /// Using fallback instead of real file
    UsingFallback,
    /// File needs to be recreated
    RecreateNeeded,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_corruption_detector_file_size() {
        let detector = CorruptionDetector::default();

        // Create a temporary small file
        let temp_dir = tempfile::tempdir().unwrap();
        let small_file = temp_dir.path().join("small.xlsx");
        fs::write(&small_file, b"too small").unwrap();

        let status = detector.detect_corruption(&small_file).unwrap();
        assert!(matches!(status, CorruptionStatus::TooSmall { .. }));
    }

    #[test]
    fn test_corruption_detector_missing_file() {
        let detector = CorruptionDetector::default();
        let missing_file = PathBuf::from("/nonexistent/file.xlsx");

        let status = detector.detect_corruption(&missing_file).unwrap();
        assert_eq!(status, CorruptionStatus::Missing);
    }

    #[test]
    fn test_corruption_detector_valid_zip() {
        let detector = CorruptionDetector::default();

        let temp_dir = tempfile::tempdir().unwrap();
        let valid_file = temp_dir.path().join("valid.xlsx");

        // Write ZIP file signature
        let mut file = fs::File::create(&valid_file).unwrap();
        file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
        file.write_all(&[0u8; 200]).unwrap(); // Padding to meet min size
        drop(file);

        let status = detector.detect_corruption(&valid_file).unwrap();
        assert_eq!(status, CorruptionStatus::Healthy);
    }

    #[test]
    fn test_recovery_strategy_healthy_file() {
        let strategy = WorkbookRecoveryStrategy::default();

        let temp_dir = tempfile::tempdir().unwrap();
        let file = temp_dir.path().join("test.xlsx");

        let mut f = fs::File::create(&file).unwrap();
        f.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
        f.write_all(&[0u8; 200]).unwrap();
        drop(f);

        let action = strategy.determine_action(&file).unwrap();
        assert_eq!(action, RecoveryAction::None);
    }

    #[test]
    fn test_create_backup() {
        let strategy = WorkbookRecoveryStrategy::new(true);

        let temp_dir = tempfile::tempdir().unwrap();
        let file = temp_dir.path().join("test.xlsx");
        fs::write(&file, b"test content").unwrap();

        let backup = strategy.create_backup(&file).unwrap();
        assert!(backup.exists());

        let backup_content = fs::read_to_string(&backup).unwrap();
        assert_eq!(backup_content, "test content");
    }
}
