# Fork Transaction Guards - Test Suite

## Overview

Comprehensive test suite for fork operation transaction guards, rollback mechanisms, and automatic cleanup features.

## Test File

`/home/user/ggen-mcp/tests/fork_transaction_guards.rs`

## Running Tests

### Run All Fork Transaction Guard Tests
```bash
cargo test --test fork_transaction_guards --features recalc
```

### Run Specific Test
```bash
cargo test --test fork_transaction_guards test_temp_file_guard_cleanup --features recalc
```

### Run with Output
```bash
cargo test --test fork_transaction_guards --features recalc -- --nocapture
```

### Run with Logging
```bash
RUST_LOG=debug cargo test --test fork_transaction_guards --features recalc -- --nocapture
```

## Test Categories

### 1. RAII Guard Tests

#### `test_temp_file_guard_cleanup`
- **Purpose**: Verify TempFileGuard automatically cleans up files
- **What it tests**: File is deleted when guard goes out of scope
- **Expected**: Temp file does not exist after guard drop

#### `test_temp_file_guard_disarm`
- **Purpose**: Verify disarm prevents cleanup
- **What it tests**: File persists when guard is disarmed
- **Expected**: Temp file still exists after guard drop

#### `test_checkpoint_guard_cleanup_on_error`
- **Purpose**: Verify CheckpointGuard cleans up on error
- **What it tests**: Checkpoint file deleted without commit
- **Expected**: Checkpoint file removed on guard drop

### 2. Fork Creation Rollback Tests

#### `test_fork_creation_rollback_on_invalid_base`
- **Purpose**: Verify rollback when base file doesn't exist
- **What it tests**: No orphaned files when fork creation fails
- **Expected**: Fork directory empty after failed creation

#### `test_fork_creation_rollback_on_invalid_extension`
- **Purpose**: Verify rollback for unsupported file types
- **What it tests**: No orphaned files for wrong file extension
- **Expected**: Fork directory empty after validation failure

### 3. Checkpoint Validation Tests

#### `test_checkpoint_validation_before_restore`
- **Purpose**: Verify checkpoint validation before restoration
- **What it tests**: Corrupted checkpoints are rejected
- **Expected**: Error when checkpoint file is invalid

#### `test_checkpoint_restore_rollback_on_error`
- **Purpose**: Verify rollback when checkpoint restore fails
- **What it tests**: Original work file preserved on error
- **Expected**: Work file unchanged after failed restoration

### 4. Save Operation Tests

#### `test_save_fork_rollback_on_error`
- **Purpose**: Verify backup restoration on save error
- **What it tests**: Original file restored when save fails
- **Expected**: Original file unchanged after failed save

### 5. Concurrency Tests

#### `test_concurrent_fork_operations_lock_release`
- **Purpose**: Verify locks are released after concurrent operations
- **What it tests**: Multiple concurrent edits complete successfully
- **Expected**: Fork accessible after all operations complete

### 6. Cleanup Tests

#### `test_fork_context_drop_cleanup`
- **Purpose**: Verify ForkContext Drop implementation
- **What it tests**: Files cleaned up when fork is discarded
- **Expected**: Work file removed after fork discard

#### `test_checkpoint_limits_with_cleanup`
- **Purpose**: Verify checkpoint limits and cleanup
- **What it tests**: Old checkpoints automatically removed
- **Expected**: Checkpoint count stays within limits

## Test Utilities

### Helper Functions

```rust
fn recalc_enabled_config(workspace: &TestWorkspace) -> ServerConfig
fn app_state_with_recalc(workspace: &TestWorkspace) -> Arc<AppState>
async fn discover_workbook(state: Arc<AppState>) -> Result<WorkbookId>
```

## Common Test Patterns

### Pattern 1: Guard Cleanup Test
```rust
{
    let guard = TempFileGuard::new(path);
    assert!(path.exists());
}
assert!(!path.exists()); // Cleaned up
```

### Pattern 2: Rollback Test
```rust
let result = risky_operation();
assert!(result.is_err());
assert!(original_state_preserved);
```

### Pattern 3: Integration Test
```rust
let fork = create_fork(...).await?;
edit_batch(...).await?;
let result = save_fork(...).await;
assert!(result.is_ok());
```

## Test Data

Tests use temporary workbooks created with:
```rust
workspace.create_workbook("test.xlsx", |book| {
    let sheet = book.get_sheet_mut(&0).unwrap();
    sheet.get_cell_mut("A1").set_value_number(100);
});
```

## Debugging Failed Tests

### Enable Logging
```bash
RUST_LOG=spreadsheet_mcp::fork=debug cargo test --test fork_transaction_guards --features recalc -- --nocapture
```

### Check Temporary Files
Failed tests may leave files in `/tmp/mcp-forks` or `/tmp/mcp-checkpoints`. Clean up with:
```bash
rm -rf /tmp/mcp-forks/* /tmp/mcp-checkpoints/*
```

### Isolate Test
Run a single test to isolate issues:
```bash
cargo test --test fork_transaction_guards test_name --features recalc -- --exact
```

## Expected Output

Successful test run:
```
running 12 tests
test test_temp_file_guard_cleanup ... ok
test test_temp_file_guard_disarm ... ok
test test_fork_creation_rollback_on_invalid_base ... ok
test test_fork_creation_rollback_on_invalid_extension ... ok
test test_checkpoint_validation_before_restore ... ok
test test_checkpoint_restore_rollback_on_error ... ok
test test_save_fork_rollback_on_error ... ok
test test_checkpoint_guard_cleanup_on_error ... ok
test test_concurrent_fork_operations_lock_release ... ok
test test_fork_context_drop_cleanup ... ok
test test_checkpoint_limits_with_cleanup ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Coverage

### What's Tested
- ✅ TempFileGuard automatic cleanup
- ✅ TempFileGuard disarm mechanism
- ✅ ForkCreationGuard rollback
- ✅ CheckpointGuard rollback
- ✅ Checkpoint validation
- ✅ Checkpoint restoration with backup
- ✅ Save operation with backup
- ✅ Concurrent operations lock release
- ✅ ForkContext Drop cleanup
- ✅ Checkpoint limit enforcement

### What's Not Tested (Future Work)
- ⏳ Extremely large file handling
- ⏳ Disk full scenarios
- ⏳ Concurrent checkpoint restoration
- ⏳ Network file systems
- ⏳ Permission errors
- ⏳ Corrupted ZIP files (partial)

## Integration with Existing Tests

These tests complement the existing fork workflow tests in `tests/fork_workflow.rs`:
- Fork workflow tests verify business logic
- Transaction guard tests verify error handling and cleanup
- Together they provide comprehensive coverage

## Continuous Integration

To run in CI:
```yaml
- name: Test fork transaction guards
  run: cargo test --test fork_transaction_guards --features recalc
```

## Performance Notes

- Tests use temporary directories for isolation
- Each test creates fresh workbooks
- Cleanup is automatic via Drop implementations
- Tests run in parallel by default

## Troubleshooting

### Issue: Tests Hang
**Cause**: Deadlock in lock acquisition
**Solution**: Check for proper lock release, enable logging

### Issue: File Already Exists
**Cause**: Previous test didn't clean up
**Solution**: Manual cleanup of /tmp/mcp-* directories

### Issue: Permission Denied
**Cause**: File still open from previous operation
**Solution**: Ensure all file handles are closed

## Contributing

When adding new tests:
1. Follow existing naming conventions
2. Add docstring explaining test purpose
3. Use helper functions from `support` module
4. Clean up resources in test
5. Update this README with new test description

## See Also

- [Fork Transaction Guards Documentation](../docs/FORK_TRANSACTION_GUARDS.md)
- [Fork Enhancements Summary](../FORK_ENHANCEMENTS_SUMMARY.md)
- [Fork Workflow Tests](./fork_workflow.rs)
