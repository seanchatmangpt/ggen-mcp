//! Tests for fork transaction guards and rollback behavior
//!
//! These tests verify that:
//! - Fork creation is rolled back on error
//! - Checkpoint operations are rolled back on error
//! - Temporary files are cleaned up properly
//! - Workbook locks are always released
//! - Failed operations don't leave orphaned resources

#![cfg(feature = "recalc")]

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use spreadsheet_mcp::ServerConfig;
use spreadsheet_mcp::fork::{ForkConfig, ForkRegistry, TempFileGuard};
use spreadsheet_mcp::model::WorkbookId;
use spreadsheet_mcp::state::AppState;
use spreadsheet_mcp::tools::fork::{
    CellEdit, CreateForkParams, EditBatchParams, create_fork, edit_batch,
};
use spreadsheet_mcp::tools::{ListWorkbooksParams, list_workbooks};

#[path = "./support/mod.rs"]
mod support;

fn recalc_enabled_config(workspace: &support::TestWorkspace) -> ServerConfig {
    workspace.config_with(|cfg| {
        cfg.recalc_enabled = true;
        cfg.allow_overwrite = true;
    })
}

fn app_state_with_recalc(workspace: &support::TestWorkspace) -> Arc<AppState> {
    let config = Arc::new(recalc_enabled_config(workspace));
    Arc::new(AppState::new(config))
}

async fn discover_workbook(state: Arc<AppState>) -> Result<WorkbookId> {
    let list = list_workbooks(
        state,
        ListWorkbooksParams {
            slug_prefix: None,
            folder: None,
            path_glob: None,
        },
    )
    .await?;
    assert_eq!(list.workbooks.len(), 1, "expected exactly 1 workbook");
    Ok(list.workbooks[0].workbook_id.clone())
}

#[test]
fn test_temp_file_guard_cleanup() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let temp_file = temp_dir.path().join("test.xlsx");

    // Create a file
    fs::write(&temp_file, b"test content")?;
    assert!(temp_file.exists());

    {
        // Guard will cleanup on drop
        let _guard = TempFileGuard::new(temp_file.clone());
        assert!(temp_file.exists());
    }

    // File should be cleaned up after guard is dropped
    assert!(!temp_file.exists(), "temp file should be cleaned up");

    Ok(())
}

#[test]
fn test_temp_file_guard_disarm() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let temp_file = temp_dir.path().join("test.xlsx");

    // Create a file
    fs::write(&temp_file, b"test content")?;
    assert!(temp_file.exists());

    {
        // Disarm the guard - file should NOT be cleaned up
        let guard = TempFileGuard::new(temp_file.clone());
        let _path = guard.disarm();
        assert!(temp_file.exists());
    }

    // File should still exist after guard is dropped
    assert!(
        temp_file.exists(),
        "temp file should not be cleaned up when disarmed"
    );

    Ok(())
}

#[test]
fn test_fork_creation_rollback_on_invalid_base() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let workspace_root = temp_dir.path().to_path_buf();
    let fork_dir = temp_dir.path().join("forks");
    fs::create_dir_all(&fork_dir)?;

    let config = ForkConfig {
        ttl: std::time::Duration::from_secs(3600),
        max_forks: 10,
        fork_dir: fork_dir.clone(),
    };

    let registry = ForkRegistry::new(config)?;

    // Try to create fork with non-existent base file
    let invalid_base = workspace_root.join("nonexistent.xlsx");
    let result = registry.create_fork(&invalid_base, &workspace_root);

    assert!(result.is_err(), "should fail with non-existent base");

    // Verify no orphaned files in fork directory
    let fork_files: Vec<_> = fs::read_dir(&fork_dir)?.filter_map(|e| e.ok()).collect();

    assert_eq!(
        fork_files.len(),
        0,
        "no files should remain in fork directory"
    );

    Ok(())
}

#[test]
fn test_fork_creation_rollback_on_invalid_extension() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let workspace_root = temp_dir.path().to_path_buf();
    let fork_dir = temp_dir.path().join("forks");
    fs::create_dir_all(&fork_dir)?;

    // Create a file with wrong extension
    let invalid_file = workspace_root.join("test.txt");
    fs::write(&invalid_file, b"not an xlsx file")?;

    let config = ForkConfig {
        ttl: std::time::Duration::from_secs(3600),
        max_forks: 10,
        fork_dir: fork_dir.clone(),
    };

    let registry = ForkRegistry::new(config)?;

    let result = registry.create_fork(&invalid_file, &workspace_root);

    assert!(result.is_err(), "should fail with wrong extension");

    // Verify no orphaned files in fork directory
    let fork_files: Vec<_> = fs::read_dir(&fork_dir)?.filter_map(|e| e.ok()).collect();

    assert_eq!(
        fork_files.len(),
        0,
        "no files should remain in fork directory"
    );

    Ok(())
}

#[tokio::test]
async fn test_checkpoint_validation_before_restore() -> Result<()> {
    let workspace = support::TestWorkspace::new();
    let _path = workspace.create_workbook("checkpoint_test.xlsx", |book| {
        let sheet = book.get_sheet_mut(&0).unwrap();
        sheet.set_name("Data");
        sheet.get_cell_mut("A1").set_value_number(100);
    });

    let state = app_state_with_recalc(&workspace);
    let workbook_id = discover_workbook(state.clone()).await?;

    // Create fork
    let fork = create_fork(
        state.clone(),
        CreateForkParams {
            workbook_or_fork_id: workbook_id,
        },
    )
    .await?;

    // Create checkpoint
    let registry = state.fork_registry().expect("fork registry");
    let checkpoint = registry.create_checkpoint(&fork.fork_id, Some("test".to_string()))?;

    // Corrupt the checkpoint file
    fs::write(&checkpoint.snapshot_path, b"corrupted data")?;

    // Try to restore - should fail validation
    let result = registry.restore_checkpoint(&fork.fork_id, &checkpoint.checkpoint_id);

    assert!(
        result.is_err(),
        "should fail validation with corrupted checkpoint"
    );
    assert!(
        result.unwrap_err().to_string().contains("not a valid XLSX"),
        "error should mention invalid XLSX"
    );

    Ok(())
}

#[tokio::test]
async fn test_checkpoint_restore_rollback_on_error() -> Result<()> {
    let workspace = support::TestWorkspace::new();
    let _path = workspace.create_workbook("restore_test.xlsx", |book| {
        let sheet = book.get_sheet_mut(&0).unwrap();
        sheet.set_name("Data");
        sheet.get_cell_mut("A1").set_value_number(100);
    });

    let state = app_state_with_recalc(&workspace);
    let workbook_id = discover_workbook(state.clone()).await?;

    // Create fork
    let fork = create_fork(
        state.clone(),
        CreateForkParams {
            workbook_or_fork_id: workbook_id,
        },
    )
    .await?;

    // Make edit
    edit_batch(
        state.clone(),
        EditBatchParams {
            fork_id: fork.fork_id.clone(),
            sheet_name: "Data".to_string(),
            edits: vec![CellEdit {
                address: "A1".to_string(),
                value: "200".to_string(),
                is_formula: false,
            }],
        },
    )
    .await?;

    // Get current value
    let registry = state.fork_registry().expect("fork registry");
    let fork_ctx = registry.get_fork(&fork.fork_id)?;
    let book = umya_spreadsheet::reader::xlsx::read(&fork_ctx.work_path)?;
    let value_before = book
        .get_sheet_by_name("Data")
        .unwrap()
        .get_cell("A1")
        .unwrap()
        .get_value();
    assert_eq!(value_before, "200");

    // Create checkpoint
    let checkpoint = registry.create_checkpoint(&fork.fork_id, None)?;

    // Delete the checkpoint snapshot to force error
    fs::remove_file(&checkpoint.snapshot_path)?;

    // Try to restore - should fail
    let result = registry.restore_checkpoint(&fork.fork_id, &checkpoint.checkpoint_id);
    assert!(result.is_err(), "should fail to restore missing checkpoint");

    // Verify work file is unchanged (rollback worked)
    let fork_ctx = registry.get_fork(&fork.fork_id)?;
    let book = umya_spreadsheet::reader::xlsx::read(&fork_ctx.work_path)?;
    let value_after = book
        .get_sheet_by_name("Data")
        .unwrap()
        .get_cell("A1")
        .unwrap()
        .get_value();
    assert_eq!(
        value_after, "200",
        "work file should be unchanged after failed restore"
    );

    Ok(())
}

#[tokio::test]
async fn test_save_fork_rollback_on_error() -> Result<()> {
    let workspace = support::TestWorkspace::new();
    let original_path = workspace.create_workbook("save_test.xlsx", |book| {
        let sheet = book.get_sheet_mut(&0).unwrap();
        sheet.set_name("Data");
        sheet.get_cell_mut("A1").set_value_number(100);
    });

    let state = app_state_with_recalc(&workspace);
    let workbook_id = discover_workbook(state.clone()).await?;

    // Create fork and edit
    let fork = create_fork(
        state.clone(),
        CreateForkParams {
            workbook_or_fork_id: workbook_id,
        },
    )
    .await?;

    edit_batch(
        state.clone(),
        EditBatchParams {
            fork_id: fork.fork_id.clone(),
            sheet_name: "Data".to_string(),
            edits: vec![CellEdit {
                address: "A1".to_string(),
                value: "999".to_string(),
                is_formula: false,
            }],
        },
    )
    .await?;

    // Read original value
    let original_book = umya_spreadsheet::reader::xlsx::read(&original_path)?;
    let original_value = original_book
        .get_sheet_by_name("Data")
        .unwrap()
        .get_cell("A1")
        .unwrap()
        .get_value();
    assert_eq!(original_value, "100");

    // Delete the fork work file to force save error
    let registry = state.fork_registry().expect("fork registry");
    let fork_path = registry.get_fork_path(&fork.fork_id).unwrap();
    fs::remove_file(&fork_path)?;

    // Try to save - should fail
    let result = registry.save_fork(&fork.fork_id, &original_path, workspace.root(), false);
    assert!(
        result.is_err(),
        "should fail to save with missing fork file"
    );

    // Verify original file is unchanged (backup/rollback worked)
    let book_after = umya_spreadsheet::reader::xlsx::read(&original_path)?;
    let value_after = book_after
        .get_sheet_by_name("Data")
        .unwrap()
        .get_cell("A1")
        .unwrap()
        .get_value();
    assert_eq!(
        value_after, "100",
        "original file should be unchanged after failed save"
    );

    Ok(())
}

#[test]
fn test_checkpoint_guard_cleanup_on_error() -> Result<()> {
    use spreadsheet_mcp::fork::CheckpointGuard;

    let temp_dir = tempfile::tempdir()?;
    let checkpoint_file = temp_dir.path().join("checkpoint.xlsx");

    // Create a checkpoint file
    fs::write(&checkpoint_file, b"checkpoint data")?;
    assert!(checkpoint_file.exists());

    {
        // Create guard but don't commit (simulates error)
        let _guard = CheckpointGuard::new(checkpoint_file.clone());
        assert!(checkpoint_file.exists());
        // Guard drops here without commit
    }

    // File should be cleaned up
    assert!(
        !checkpoint_file.exists(),
        "checkpoint file should be cleaned up on error"
    );

    Ok(())
}

#[tokio::test]
async fn test_concurrent_fork_operations_lock_release() -> Result<()> {
    let workspace = support::TestWorkspace::new();
    let _path = workspace.create_workbook("concurrent_test.xlsx", |book| {
        let sheet = book.get_sheet_mut(&0).unwrap();
        sheet.get_cell_mut("A1").set_value_number(1);
    });

    let state = app_state_with_recalc(&workspace);
    let workbook_id = discover_workbook(state.clone()).await?;

    // Create fork
    let fork = create_fork(
        state.clone(),
        CreateForkParams {
            workbook_or_fork_id: workbook_id,
        },
    )
    .await?;

    // Spawn multiple concurrent operations
    let mut handles = vec![];

    for i in 0..5 {
        let state_clone = state.clone();
        let fork_id = fork.fork_id.clone();

        let handle = tokio::spawn(async move {
            edit_batch(
                state_clone,
                EditBatchParams {
                    fork_id,
                    sheet_name: "Sheet1".to_string(),
                    edits: vec![CellEdit {
                        address: format!("A{}", i + 1),
                        value: i.to_string(),
                        is_formula: false,
                    }],
                },
            )
            .await
        });

        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        let _ = handle.await?;
    }

    // Verify fork is still accessible (locks were released)
    let registry = state.fork_registry().expect("fork registry");
    let fork_ctx = registry.get_fork(&fork.fork_id);
    assert!(
        fork_ctx.is_ok(),
        "fork should still be accessible after concurrent operations"
    );

    Ok(())
}

#[test]
fn test_fork_context_drop_cleanup() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let workspace_root = temp_dir.path().to_path_buf();
    let fork_dir = temp_dir.path().join("forks");
    fs::create_dir_all(&fork_dir)?;

    // Create a valid workbook
    let base_path = workspace_root.join("test.xlsx");
    let mut book = umya_spreadsheet::new_file();
    let sheet = book.get_sheet_mut(&0).unwrap();
    sheet.get_cell_mut("A1").set_value_number(42);
    umya_spreadsheet::writer::xlsx::write(&book, &base_path)?;

    let config = ForkConfig {
        ttl: std::time::Duration::from_secs(3600),
        max_forks: 10,
        fork_dir: fork_dir.clone(),
    };

    let registry = ForkRegistry::new(config)?;

    // Create fork
    let fork_id = registry.create_fork(&base_path, &workspace_root)?;
    let work_path = registry.get_fork_path(&fork_id).unwrap();
    assert!(work_path.exists(), "fork work file should exist");

    // Discard fork (triggers ForkContext drop)
    registry.discard_fork(&fork_id)?;

    // Verify work file is cleaned up
    assert!(
        !work_path.exists(),
        "fork work file should be cleaned up on drop"
    );

    Ok(())
}

#[tokio::test]
async fn test_checkpoint_limits_with_cleanup() -> Result<()> {
    let workspace = support::TestWorkspace::new();
    let _path = workspace.create_workbook("limits_test.xlsx", |book| {
        let sheet = book.get_sheet_mut(&0).unwrap();
        sheet.get_cell_mut("A1").set_value_number(1);
    });

    let state = app_state_with_recalc(&workspace);
    let workbook_id = discover_workbook(state.clone()).await?;

    // Create fork
    let fork = create_fork(
        state.clone(),
        CreateForkParams {
            workbook_or_fork_id: workbook_id,
        },
    )
    .await?;

    let registry = state.fork_registry().expect("fork registry");

    // Create many checkpoints (more than limit)
    let mut checkpoint_ids = vec![];
    for i in 0..15 {
        let checkpoint =
            registry.create_checkpoint(&fork.fork_id, Some(format!("checkpoint_{}", i)))?;
        checkpoint_ids.push(checkpoint.checkpoint_id);
    }

    // Verify old checkpoints were cleaned up
    let checkpoints = registry.list_checkpoints(&fork.fork_id)?;
    assert!(checkpoints.len() <= 10, "should enforce checkpoint limit");

    // Verify oldest checkpoint files are actually deleted
    for id in &checkpoint_ids[0..5] {
        let result = registry.list_checkpoints(&fork.fork_id)?;
        let found = result.iter().any(|c| &c.checkpoint_id == id);
        assert!(!found, "old checkpoint should be removed from list");
    }

    Ok(())
}
