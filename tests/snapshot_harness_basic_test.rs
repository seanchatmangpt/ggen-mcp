//! Basic validation test for snapshot harness
//!
//! This test verifies the snapshot harness compiles and basic functionality works.

mod harness;

use harness::{SnapshotTestHarness, SnapshotFormat, UpdateMode};
use tempfile::TempDir;

#[test]
fn test_harness_initialization() {
    let harness = SnapshotTestHarness::new();
    assert!(harness.stats().total == 0);
}

#[test]
fn test_snapshot_creation_in_temp_dir() {
    let temp_dir = TempDir::new().unwrap();
    let mut harness = SnapshotTestHarness::with_root(temp_dir.path());
    harness.update_mode = UpdateMode::Always;

    let content = "Hello, snapshot testing!";
    let result = harness.assert_snapshot(
        "test",
        "basic_test",
        content,
        SnapshotFormat::Text,
    );

    assert!(result.is_ok(), "Snapshot creation should succeed");
    assert_eq!(harness.stats().created, 1);
}

#[test]
fn test_snapshot_matching() {
    let temp_dir = TempDir::new().unwrap();
    let mut harness = SnapshotTestHarness::with_root(temp_dir.path());
    harness.update_mode = UpdateMode::Always;

    // Create snapshot
    let content = "Test content";
    harness
        .assert_snapshot("test", "match_test", content, SnapshotFormat::Text)
        .unwrap();

    // Reset stats and switch to never update mode
    harness.reset_stats();
    harness.update_mode = UpdateMode::Never;

    // Should match
    let result = harness.assert_snapshot("test", "match_test", content, SnapshotFormat::Text);
    assert!(result.is_ok(), "Snapshot should match");
    assert_eq!(harness.stats().matched, 1);
}

#[test]
fn test_snapshot_mismatch() {
    let temp_dir = TempDir::new().unwrap();
    let mut harness = SnapshotTestHarness::with_root(temp_dir.path());
    harness.update_mode = UpdateMode::Always;

    // Create original snapshot
    harness
        .assert_snapshot("test", "mismatch_test", "original", SnapshotFormat::Text)
        .unwrap();

    // Reset and try different content
    harness.reset_stats();
    harness.update_mode = UpdateMode::Never;

    let result = harness.assert_snapshot(
        "test",
        "mismatch_test",
        "modified",
        SnapshotFormat::Text,
    );

    assert!(result.is_err(), "Snapshot should not match");
    assert_eq!(harness.stats().failed, 1);
}

#[test]
fn test_json_snapshot() {
    use serde_json::json;

    let temp_dir = TempDir::new().unwrap();
    let mut harness = SnapshotTestHarness::with_root(temp_dir.path());
    harness.update_mode = UpdateMode::Always;

    let json_data = json!({
        "name": "test",
        "value": 42,
        "nested": {
            "key": "value"
        }
    });

    let json_str = serde_json::to_string_pretty(&json_data).unwrap();

    let result = harness.assert_snapshot(
        "test",
        "json_test",
        json_str,
        SnapshotFormat::Json,
    );

    assert!(result.is_ok(), "JSON snapshot should be created");
}

#[test]
fn test_snapshot_stats() {
    let temp_dir = TempDir::new().unwrap();
    let mut harness = SnapshotTestHarness::with_root(temp_dir.path());
    harness.update_mode = UpdateMode::Always;

    // Create multiple snapshots
    for i in 0..5 {
        let _ = harness.assert_snapshot(
            "test",
            &format!("stat_test_{}", i),
            &format!("content {}", i),
            SnapshotFormat::Text,
        );
    }

    let stats = harness.stats();
    assert_eq!(stats.total, 5);
    assert_eq!(stats.created, 5);
}

#[test]
fn test_diff_computation() {
    let temp_dir = TempDir::new().unwrap();
    let mut harness = SnapshotTestHarness::with_root(temp_dir.path());
    harness.update_mode = UpdateMode::Always;

    // Create original
    let original = "line1\nline2\nline3";
    harness
        .assert_snapshot("test", "diff_test", original, SnapshotFormat::Text)
        .unwrap();

    // Reset and compare with modified
    harness.reset_stats();
    harness.update_mode = UpdateMode::Never;

    let modified = "line1\nmodified_line2\nline3\nline4";
    let result = harness.assert_snapshot("test", "diff_test", modified, SnapshotFormat::Text);

    assert!(result.is_err());
    if let Err(err) = result {
        assert!(err.diff.is_some(), "Should have diff");
        let diff = err.diff.unwrap();
        assert!(diff.additions > 0 || diff.deletions > 0);
    }
}

#[test]
fn test_update_modes() {
    let temp_dir = TempDir::new().unwrap();

    // Test Never mode
    let mut harness = SnapshotTestHarness::with_root(temp_dir.path());
    harness.update_mode = UpdateMode::Never;

    let result = harness.assert_snapshot(
        "test",
        "never_test",
        "content",
        SnapshotFormat::Text,
    );
    assert!(result.is_err(), "Should fail when snapshot doesn't exist in Never mode");

    // Test Always mode
    let mut harness = SnapshotTestHarness::with_root(temp_dir.path());
    harness.update_mode = UpdateMode::Always;

    let result = harness.assert_snapshot(
        "test",
        "always_test",
        "content",
        SnapshotFormat::Text,
    );
    assert!(result.is_ok(), "Should create snapshot in Always mode");

    // Test New mode
    let mut harness = SnapshotTestHarness::with_root(temp_dir.path());
    harness.update_mode = UpdateMode::New;

    let result = harness.assert_snapshot(
        "test",
        "new_test",
        "content",
        SnapshotFormat::Text,
    );
    assert!(result.is_ok(), "Should create new snapshot in New mode");

    // Should not update existing in New mode
    harness.reset_stats();
    let result = harness.assert_snapshot(
        "test",
        "new_test",
        "different",
        SnapshotFormat::Text,
    );
    assert!(result.is_err(), "Should not update existing snapshot in New mode");
}
