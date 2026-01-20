//! # Chicago-Style TDD Snapshot Testing Harness
//!
//! This module provides a comprehensive snapshot testing framework for validating
//! code generation, template rendering, SPARQL queries, and configuration outputs.
//!
//! ## Features
//!
//! - **Golden File Testing**: Compare outputs against versioned reference files
//! - **Multi-Format Support**: Rust, JSON, TOML, TTL, Debug output, Binary
//! - **Diff Visualization**: Colorized, line-by-line comparison
//! - **Update Workflow**: Interactive and batch snapshot updates
//! - **CI Integration**: Fail on unexpected changes
//! - **Chicago-Style TDD**: State-based verification with comprehensive assertions
//!
//! ## Usage
//!
//! ```rust
//! use snapshot_testing_harness::*;
//!
//! #[test]
//! fn test_user_aggregate_generation() {
//!     let harness = SnapshotTestHarness::new();
//!     let code = generate_user_aggregate();
//!
//!     assert_snapshot!(harness, "user_aggregate", code);
//! }
//! ```

use std::collections::{HashMap, HashSet};
use std::env;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};

// ============================================================================
// Core Types
// ============================================================================

/// Main snapshot testing harness
#[derive(Debug)]
pub struct SnapshotTestHarness {
    /// Root directory for snapshots
    snapshot_root: PathBuf,
    /// Whether to update snapshots automatically
    update_mode: UpdateMode,
    /// Snapshot metadata cache
    metadata_cache: HashMap<String, SnapshotMetadata>,
    /// Statistics for the test run
    stats: SnapshotStats,
}

/// Update mode for snapshots
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateMode {
    /// Never update, always fail on mismatch
    Never,
    /// Update all mismatched snapshots
    Always,
    /// Update interactively (prompt user)
    Interactive,
    /// Update only new snapshots (missing files)
    New,
}

/// Snapshot format types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotFormat {
    /// Plain text
    Text,
    /// Rust source code
    Rust,
    /// JSON data
    Json,
    /// TOML configuration
    Toml,
    /// Turtle/TTL ontology
    Turtle,
    /// Debug output (prettified Debug trait)
    Debug,
    /// Binary data
    Binary,
}

/// Snapshot metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Snapshot name
    pub name: String,
    /// Category (codegen, templates, sparql, config)
    pub category: String,
    /// Format
    pub format: SnapshotFormat,
    /// Creation timestamp
    pub created_at: String,
    /// Last updated timestamp
    pub updated_at: String,
    /// Content hash (SHA-256)
    pub hash: String,
    /// Size in bytes
    pub size: usize,
    /// Test module path
    pub test_module: String,
}

/// Statistics for snapshot testing
#[derive(Debug, Default, Clone)]
pub struct SnapshotStats {
    /// Total snapshots checked
    pub total: usize,
    /// Snapshots that matched
    pub matched: usize,
    /// Snapshots that were created
    pub created: usize,
    /// Snapshots that were updated
    pub updated: usize,
    /// Snapshots that failed
    pub failed: usize,
}

/// Snapshot comparison result
#[derive(Debug, Clone)]
pub struct SnapshotComparison {
    /// Whether the snapshot matches
    pub matches: bool,
    /// Snapshot name
    pub name: String,
    /// Expected content (from file)
    pub expected: Option<String>,
    /// Actual content (from test)
    pub actual: String,
    /// Diff if not matching
    pub diff: Option<Diff>,
}

/// Diff between expected and actual
#[derive(Debug, Clone)]
pub struct Diff {
    /// Line-by-line differences
    pub lines: Vec<DiffLine>,
    /// Number of additions
    pub additions: usize,
    /// Number of deletions
    pub deletions: usize,
    /// Number of unchanged lines
    pub unchanged: usize,
}

/// Individual diff line
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLine {
    /// Unchanged line
    Context(String),
    /// Added line
    Addition(String),
    /// Deleted line
    Deletion(String),
}

/// Snapshot assertion error
#[derive(Debug)]
pub struct SnapshotError {
    pub message: String,
    pub snapshot_name: String,
    pub diff: Option<Diff>,
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Snapshot mismatch for '{}': {}", self.snapshot_name, self.message)
    }
}

impl std::error::Error for SnapshotError {}

// ============================================================================
// SnapshotTestHarness Implementation
// ============================================================================

impl SnapshotTestHarness {
    /// Create a new snapshot test harness
    pub fn new() -> Self {
        let snapshot_root = Self::get_snapshot_root();
        let update_mode = Self::get_update_mode();

        Self {
            snapshot_root,
            update_mode,
            metadata_cache: HashMap::new(),
            stats: SnapshotStats::default(),
        }
    }

    /// Create harness with custom snapshot root
    pub fn with_root<P: AsRef<Path>>(root: P) -> Self {
        let mut harness = Self::new();
        harness.snapshot_root = root.as_ref().to_path_buf();
        harness
    }

    /// Get snapshot root from environment or default
    fn get_snapshot_root() -> PathBuf {
        env::var("SNAPSHOT_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let manifest_dir = env::var("CARGO_MANIFEST_DIR")
                    .unwrap_or_else(|_| ".".to_string());
                PathBuf::from(manifest_dir).join("snapshots")
            })
    }

    /// Get update mode from environment
    fn get_update_mode() -> UpdateMode {
        match env::var("UPDATE_SNAPSHOTS").as_deref() {
            Ok("1") | Ok("true") | Ok("always") => UpdateMode::Always,
            Ok("interactive") => UpdateMode::Interactive,
            Ok("new") => UpdateMode::New,
            _ => UpdateMode::Never,
        }
    }

    /// Assert snapshot matches
    pub fn assert_snapshot<S: AsRef<str>>(
        &mut self,
        category: &str,
        name: &str,
        actual: S,
        format: SnapshotFormat,
    ) -> Result<(), SnapshotError> {
        let comparison = self.compare_snapshot(category, name, actual.as_ref(), format)?;

        self.stats.total += 1;

        if comparison.matches {
            self.stats.matched += 1;
            Ok(())
        } else {
            self.stats.failed += 1;
            Err(SnapshotError {
                message: "Snapshot does not match".to_string(),
                snapshot_name: name.to_string(),
                diff: comparison.diff,
            })
        }
    }

    /// Compare snapshot and handle update mode
    fn compare_snapshot(
        &mut self,
        category: &str,
        name: &str,
        actual: &str,
        format: SnapshotFormat,
    ) -> Result<SnapshotComparison, SnapshotError> {
        let snapshot_path = self.get_snapshot_path(category, name, format);

        // Format the actual content
        let formatted_actual = self.format_content(actual, format)?;

        // Check if snapshot exists
        let expected = if snapshot_path.exists() {
            Some(fs::read_to_string(&snapshot_path).map_err(|e| SnapshotError {
                message: format!("Failed to read snapshot: {}", e),
                snapshot_name: name.to_string(),
                diff: None,
            })?)
        } else {
            None
        };

        // Compare or create
        match expected {
            Some(expected_content) => {
                let matches = expected_content == formatted_actual;

                if !matches {
                    let diff = self.compute_diff(&expected_content, &formatted_actual);

                    match self.update_mode {
                        UpdateMode::Always => {
                            self.write_snapshot(&snapshot_path, &formatted_actual, category, name, format)?;
                            self.stats.updated += 1;
                            Ok(SnapshotComparison {
                                matches: true,
                                name: name.to_string(),
                                expected: Some(expected_content),
                                actual: formatted_actual,
                                diff: Some(diff),
                            })
                        }
                        UpdateMode::Interactive => {
                            if self.prompt_update(name, &diff)? {
                                self.write_snapshot(&snapshot_path, &formatted_actual, category, name, format)?;
                                self.stats.updated += 1;
                                Ok(SnapshotComparison {
                                    matches: true,
                                    name: name.to_string(),
                                    expected: Some(expected_content),
                                    actual: formatted_actual,
                                    diff: Some(diff),
                                })
                            } else {
                                Ok(SnapshotComparison {
                                    matches: false,
                                    name: name.to_string(),
                                    expected: Some(expected_content),
                                    actual: formatted_actual,
                                    diff: Some(diff),
                                })
                            }
                        }
                        UpdateMode::Never | UpdateMode::New => {
                            Ok(SnapshotComparison {
                                matches: false,
                                name: name.to_string(),
                                expected: Some(expected_content),
                                actual: formatted_actual,
                                diff: Some(diff),
                            })
                        }
                    }
                } else {
                    Ok(SnapshotComparison {
                        matches: true,
                        name: name.to_string(),
                        expected: Some(expected_content),
                        actual: formatted_actual,
                        diff: None,
                    })
                }
            }
            None => {
                // Snapshot doesn't exist - create it
                if self.update_mode == UpdateMode::Never {
                    Err(SnapshotError {
                        message: "Snapshot file does not exist".to_string(),
                        snapshot_name: name.to_string(),
                        diff: None,
                    })
                } else {
                    self.write_snapshot(&snapshot_path, &formatted_actual, category, name, format)?;
                    self.stats.created += 1;
                    Ok(SnapshotComparison {
                        matches: true,
                        name: name.to_string(),
                        expected: None,
                        actual: formatted_actual,
                        diff: None,
                    })
                }
            }
        }
    }

    /// Format content according to format type
    fn format_content(&self, content: &str, format: SnapshotFormat) -> Result<String, SnapshotError> {
        match format {
            SnapshotFormat::Json => {
                // Pretty-print JSON
                serde_json::from_str::<JsonValue>(content)
                    .and_then(|v| serde_json::to_string_pretty(&v))
                    .map_err(|e| SnapshotError {
                        message: format!("Failed to format JSON: {}", e),
                        snapshot_name: String::new(),
                        diff: None,
                    })
            }
            SnapshotFormat::Rust => {
                // Format Rust code using syn (basic)
                Ok(content.to_string())
            }
            SnapshotFormat::Toml => {
                // TOML is already formatted
                Ok(content.to_string())
            }
            SnapshotFormat::Debug => {
                // Debug output is already formatted
                Ok(content.to_string())
            }
            SnapshotFormat::Text | SnapshotFormat::Turtle | SnapshotFormat::Binary => {
                Ok(content.to_string())
            }
        }
    }

    /// Get snapshot file path
    fn get_snapshot_path(&self, category: &str, name: &str, format: SnapshotFormat) -> PathBuf {
        let extension = match format {
            SnapshotFormat::Rust => "rs.snap",
            SnapshotFormat::Json => "json.snap",
            SnapshotFormat::Toml => "toml.snap",
            SnapshotFormat::Turtle => "ttl.snap",
            SnapshotFormat::Debug => "debug.snap",
            SnapshotFormat::Binary => "bin.snap",
            SnapshotFormat::Text => "txt.snap",
        };

        self.snapshot_root
            .join(category)
            .join(format!("{}.{}", name, extension))
    }

    /// Write snapshot to file
    fn write_snapshot(
        &mut self,
        path: &Path,
        content: &str,
        category: &str,
        name: &str,
        format: SnapshotFormat,
    ) -> Result<(), SnapshotError> {
        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| SnapshotError {
                message: format!("Failed to create snapshot directory: {}", e),
                snapshot_name: name.to_string(),
                diff: None,
            })?;
        }

        // Write content
        fs::write(path, content).map_err(|e| SnapshotError {
            message: format!("Failed to write snapshot: {}", e),
            snapshot_name: name.to_string(),
            diff: None,
        })?;

        // Update metadata
        let metadata = SnapshotMetadata {
            name: name.to_string(),
            category: category.to_string(),
            format,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            hash: self.compute_hash(content),
            size: content.len(),
            test_module: self.get_test_module(),
        };

        self.metadata_cache.insert(name.to_string(), metadata.clone());
        self.write_metadata(&metadata)?;

        Ok(())
    }

    /// Compute SHA-256 hash of content
    fn compute_hash(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Get current test module path
    fn get_test_module(&self) -> String {
        // This would ideally use std::panic::Location but that requires nightly
        // For now, we'll use a placeholder
        env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "unknown".to_string())
    }

    /// Write metadata file
    fn write_metadata(&self, metadata: &SnapshotMetadata) -> Result<(), SnapshotError> {
        let metadata_path = self.snapshot_root
            .join(&metadata.category)
            .join(format!("{}.meta.json", metadata.name));

        let json = serde_json::to_string_pretty(metadata).map_err(|e| SnapshotError {
            message: format!("Failed to serialize metadata: {}", e),
            snapshot_name: metadata.name.clone(),
            diff: None,
        })?;

        fs::write(metadata_path, json).map_err(|e| SnapshotError {
            message: format!("Failed to write metadata: {}", e),
            snapshot_name: metadata.name.clone(),
            diff: None,
        })?;

        Ok(())
    }

    /// Compute diff between expected and actual
    fn compute_diff(&self, expected: &str, actual: &str) -> Diff {
        let expected_lines: Vec<&str> = expected.lines().collect();
        let actual_lines: Vec<&str> = actual.lines().collect();

        let mut diff_lines = Vec::new();
        let mut additions = 0;
        let mut deletions = 0;
        let mut unchanged = 0;

        // Simple line-by-line diff (Myers algorithm would be better but more complex)
        let max_len = expected_lines.len().max(actual_lines.len());

        for i in 0..max_len {
            match (expected_lines.get(i), actual_lines.get(i)) {
                (Some(&exp), Some(&act)) => {
                    if exp == act {
                        diff_lines.push(DiffLine::Context(exp.to_string()));
                        unchanged += 1;
                    } else {
                        diff_lines.push(DiffLine::Deletion(exp.to_string()));
                        diff_lines.push(DiffLine::Addition(act.to_string()));
                        deletions += 1;
                        additions += 1;
                    }
                }
                (Some(&exp), None) => {
                    diff_lines.push(DiffLine::Deletion(exp.to_string()));
                    deletions += 1;
                }
                (None, Some(&act)) => {
                    diff_lines.push(DiffLine::Addition(act.to_string()));
                    additions += 1;
                }
                (None, None) => unreachable!(),
            }
        }

        Diff {
            lines: diff_lines,
            additions,
            deletions,
            unchanged,
        }
    }

    /// Prompt user to update snapshot (interactive mode)
    fn prompt_update(&self, name: &str, diff: &Diff) -> Result<bool, SnapshotError> {
        println!("\n{}", "=".repeat(80));
        println!("Snapshot '{}' differs:", name);
        println!("{}", "=".repeat(80));
        self.print_diff(diff);
        println!("{}", "=".repeat(80));
        print!("Update snapshot? [y/N]: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(|e| SnapshotError {
            message: format!("Failed to read input: {}", e),
            snapshot_name: name.to_string(),
            diff: None,
        })?;

        Ok(input.trim().to_lowercase() == "y")
    }

    /// Print diff to stdout with colors (when available)
    pub fn print_diff(&self, diff: &Diff) {
        println!("Changes: +{} -{} ~{}", diff.additions, diff.deletions, diff.unchanged);
        println!();

        for (i, line) in diff.lines.iter().enumerate() {
            match line {
                DiffLine::Context(content) => {
                    println!("  {}", content);
                }
                DiffLine::Addition(content) => {
                    println!("+ {}", content);
                }
                DiffLine::Deletion(content) => {
                    println!("- {}", content);
                }
            }

            // Limit output for very large diffs
            if i > 100 {
                println!("... ({} more lines)", diff.lines.len() - i);
                break;
            }
        }
    }

    /// Get statistics
    pub fn stats(&self) -> &SnapshotStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = SnapshotStats::default();
    }

    /// Find orphaned snapshots (no corresponding test)
    pub fn find_orphaned_snapshots(&self) -> Result<Vec<PathBuf>, io::Error> {
        let mut orphaned = Vec::new();

        for category in &["codegen", "templates", "sparql", "config", "misc"] {
            let category_path = self.snapshot_root.join(category);
            if !category_path.exists() {
                continue;
            }

            for entry in fs::read_dir(category_path)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("snap") {
                    // In a real implementation, we'd check if the test exists
                    // For now, we'll just collect all snapshots
                }
            }
        }

        Ok(orphaned)
    }

    /// Get snapshot statistics report
    pub fn generate_report(&self) -> SnapshotReport {
        let mut report = SnapshotReport {
            total_snapshots: 0,
            by_category: HashMap::new(),
            by_format: HashMap::new(),
            total_size: 0,
            average_size: 0,
        };

        for (_, metadata) in &self.metadata_cache {
            report.total_snapshots += 1;
            *report.by_category.entry(metadata.category.clone()).or_insert(0) += 1;
            *report.by_format.entry(format!("{:?}", metadata.format)).or_insert(0) += 1;
            report.total_size += metadata.size;
        }

        if report.total_snapshots > 0 {
            report.average_size = report.total_size / report.total_snapshots;
        }

        report
    }
}

impl Default for SnapshotTestHarness {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot report
#[derive(Debug, Clone)]
pub struct SnapshotReport {
    pub total_snapshots: usize,
    pub by_category: HashMap<String, usize>,
    pub by_format: HashMap<String, usize>,
    pub total_size: usize,
    pub average_size: usize,
}

impl fmt::Display for SnapshotReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Snapshot Report")?;
        writeln!(f, "===============")?;
        writeln!(f, "Total snapshots: {}", self.total_snapshots)?;
        writeln!(f, "Total size: {} bytes", self.total_size)?;
        writeln!(f, "Average size: {} bytes", self.average_size)?;
        writeln!(f)?;
        writeln!(f, "By category:")?;
        for (category, count) in &self.by_category {
            writeln!(f, "  {}: {}", category, count)?;
        }
        writeln!(f)?;
        writeln!(f, "By format:")?;
        for (format, count) in &self.by_format {
            writeln!(f, "  {}: {}", format, count)?;
        }
        Ok(())
    }
}

// ============================================================================
// Convenience Macros
// ============================================================================

/// Assert that a snapshot matches
#[macro_export]
macro_rules! assert_snapshot {
    ($harness:expr, $name:expr, $actual:expr) => {
        $harness.assert_snapshot("misc", $name, $actual, $crate::SnapshotFormat::Text)
            .expect(&format!("Snapshot assertion failed for '{}'", $name))
    };
    ($harness:expr, $category:expr, $name:expr, $actual:expr) => {
        $harness.assert_snapshot($category, $name, $actual, $crate::SnapshotFormat::Text)
            .expect(&format!("Snapshot assertion failed for '{}'", $name))
    };
}

/// Assert JSON snapshot matches
#[macro_export]
macro_rules! assert_json_snapshot {
    ($harness:expr, $name:expr, $actual:expr) => {
        {
            let json_str = serde_json::to_string_pretty(&$actual)
                .expect("Failed to serialize to JSON");
            $harness.assert_snapshot("misc", $name, json_str, $crate::SnapshotFormat::Json)
                .expect(&format!("JSON snapshot assertion failed for '{}'", $name))
        }
    };
    ($harness:expr, $category:expr, $name:expr, $actual:expr) => {
        {
            let json_str = serde_json::to_string_pretty(&$actual)
                .expect("Failed to serialize to JSON");
            $harness.assert_snapshot($category, $name, json_str, $crate::SnapshotFormat::Json)
                .expect(&format!("JSON snapshot assertion failed for '{}'", $name))
        }
    };
}

/// Assert debug snapshot matches
#[macro_export]
macro_rules! assert_debug_snapshot {
    ($harness:expr, $name:expr, $actual:expr) => {
        {
            let debug_str = format!("{:#?}", $actual);
            $harness.assert_snapshot("misc", $name, debug_str, $crate::SnapshotFormat::Debug)
                .expect(&format!("Debug snapshot assertion failed for '{}'", $name))
        }
    };
    ($harness:expr, $category:expr, $name:expr, $actual:expr) => {
        {
            let debug_str = format!("{:#?}", $actual);
            $harness.assert_snapshot($category, $name, debug_str, $crate::SnapshotFormat::Debug)
                .expect(&format!("Debug snapshot assertion failed for '{}'", $name))
        }
    };
}

// ============================================================================
// Snapshot Utilities
// ============================================================================

/// Cleanup old snapshots
pub fn cleanup_snapshots(snapshot_root: &Path, older_than_days: u32) -> Result<Vec<PathBuf>, io::Error> {
    let mut removed = Vec::new();
    let cutoff = chrono::Utc::now() - chrono::Duration::days(older_than_days as i64);

    for entry in walkdir::WalkDir::new(snapshot_root) {
        let entry = entry?;
        if entry.file_type().is_file() && entry.path().extension().and_then(|s| s.to_str()) == Some("snap") {
            let metadata = entry.metadata()?;
            if let Ok(modified) = metadata.modified() {
                let modified: chrono::DateTime<chrono::Utc> = modified.into();
                if modified < cutoff {
                    fs::remove_file(entry.path())?;
                    removed.push(entry.path().to_path_buf());
                }
            }
        }
    }

    Ok(removed)
}

/// Get all snapshots in directory
pub fn list_snapshots(snapshot_root: &Path) -> Result<Vec<PathBuf>, io::Error> {
    let mut snapshots = Vec::new();

    for entry in walkdir::WalkDir::new(snapshot_root) {
        let entry = entry?;
        if entry.file_type().is_file() && entry.path().extension().and_then(|s| s.to_str()) == Some("snap") {
            snapshots.push(entry.path().to_path_buf());
        }
    }

    snapshots.sort();
    Ok(snapshots)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_snapshot_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mut harness = SnapshotTestHarness::with_root(temp_dir.path());

        // Enable update mode for this test
        harness.update_mode = UpdateMode::Always;

        // Create a new snapshot
        let content = "Hello, World!";
        let result = harness.assert_snapshot("test", "hello", content, SnapshotFormat::Text);

        assert!(result.is_ok());
        assert_eq!(harness.stats().created, 1);

        // Verify snapshot file was created
        let snapshot_path = harness.get_snapshot_path("test", "hello", SnapshotFormat::Text);
        assert!(snapshot_path.exists());

        let saved_content = fs::read_to_string(snapshot_path).unwrap();
        assert_eq!(saved_content, content);
    }

    #[test]
    fn test_snapshot_matching() {
        let temp_dir = TempDir::new().unwrap();
        let mut harness = SnapshotTestHarness::with_root(temp_dir.path());
        harness.update_mode = UpdateMode::Always;

        // Create initial snapshot
        let content = "Hello, World!";
        harness.assert_snapshot("test", "match_test", content, SnapshotFormat::Text).unwrap();

        // Reset stats
        harness.reset_stats();

        // Verify it matches on second run
        harness.update_mode = UpdateMode::Never;
        let result = harness.assert_snapshot("test", "match_test", content, SnapshotFormat::Text);

        assert!(result.is_ok());
        assert_eq!(harness.stats().matched, 1);
    }

    #[test]
    fn test_snapshot_mismatch() {
        let temp_dir = TempDir::new().unwrap();
        let mut harness = SnapshotTestHarness::with_root(temp_dir.path());
        harness.update_mode = UpdateMode::Always;

        // Create initial snapshot
        harness.assert_snapshot("test", "mismatch_test", "Hello!", SnapshotFormat::Text).unwrap();

        // Reset and try different content
        harness.reset_stats();
        harness.update_mode = UpdateMode::Never;

        let result = harness.assert_snapshot("test", "mismatch_test", "Goodbye!", SnapshotFormat::Text);

        assert!(result.is_err());
        assert_eq!(harness.stats().failed, 1);

        if let Err(err) = result {
            assert!(err.diff.is_some());
            let diff = err.diff.unwrap();
            assert!(diff.additions > 0 || diff.deletions > 0);
        }
    }

    #[test]
    fn test_json_formatting() {
        let temp_dir = TempDir::new().unwrap();
        let mut harness = SnapshotTestHarness::with_root(temp_dir.path());
        harness.update_mode = UpdateMode::Always;

        let json = r#"{"name":"test","value":42}"#;
        harness.assert_snapshot("test", "json_test", json, SnapshotFormat::Json).unwrap();

        let snapshot_path = harness.get_snapshot_path("test", "json_test", SnapshotFormat::Json);
        let saved = fs::read_to_string(snapshot_path).unwrap();

        // Should be pretty-printed
        assert!(saved.contains("  "));
        assert!(saved.contains("\"name\": \"test\""));
    }

    #[test]
    fn test_diff_computation() {
        let harness = SnapshotTestHarness::new();

        let expected = "line1\nline2\nline3";
        let actual = "line1\nline2_modified\nline3";

        let diff = harness.compute_diff(expected, actual);

        assert_eq!(diff.unchanged, 2);
        assert_eq!(diff.additions, 1);
        assert_eq!(diff.deletions, 1);
    }

    #[test]
    fn test_hash_computation() {
        let harness = SnapshotTestHarness::new();

        let content = "test content";
        let hash1 = harness.compute_hash(content);
        let hash2 = harness.compute_hash(content);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 produces 64 hex characters
    }

    #[test]
    fn test_snapshot_report() {
        let temp_dir = TempDir::new().unwrap();
        let mut harness = SnapshotTestHarness::with_root(temp_dir.path());
        harness.update_mode = UpdateMode::Always;

        // Create multiple snapshots
        harness.assert_snapshot("codegen", "test1", "content1", SnapshotFormat::Rust).unwrap();
        harness.assert_snapshot("templates", "test2", "content2", SnapshotFormat::Json).unwrap();
        harness.assert_snapshot("codegen", "test3", "content3", SnapshotFormat::Rust).unwrap();

        let report = harness.generate_report();

        assert_eq!(report.total_snapshots, 3);
        assert_eq!(report.by_category.get("codegen"), Some(&2));
        assert_eq!(report.by_category.get("templates"), Some(&1));
    }
}
