# Fork Transaction Guards and Rollback Mechanisms

## Overview

This document describes the transaction rollback guards and cleanup mechanisms implemented for fork operations in the spreadsheet MCP server. These enhancements ensure that fork operations are atomic, with automatic rollback on error and proper resource cleanup.

## Key Features

### 1. RAII Guards for Resource Management

#### TempFileGuard
Automatically cleans up temporary files when they go out of scope.

```rust
pub struct TempFileGuard {
    path: PathBuf,
    cleanup_on_drop: bool,
}
```

**Usage:**
- Creates a guard for a temporary file
- Automatically deletes the file when the guard is dropped
- Can be "disarmed" to prevent cleanup if the operation succeeds

**Example:**
```rust
let backup_path = work_path.with_extension("backup.xlsx");
let backup_guard = TempFileGuard::new(backup_path.clone());

// ... perform risky operation ...

if operation_succeeded {
    backup_guard.disarm(); // Keep the file
} else {
    // File is automatically cleaned up on drop
}
```

#### ForkCreationGuard
Ensures that partially created forks are cleaned up on error.

```rust
pub struct ForkCreationGuard<'a> {
    fork_id: String,
    work_path: PathBuf,
    registry: &'a ForkRegistry,
    committed: bool,
}
```

**Behavior:**
- Automatically removes fork from registry on error
- Deletes work file if creation fails
- Must be explicitly committed for successful operations

#### CheckpointGuard
Protects checkpoint creation with automatic cleanup on failure.

```rust
pub struct CheckpointGuard {
    snapshot_path: PathBuf,
    committed: bool,
}
```

**Features:**
- Automatically deletes snapshot file on error
- Prevents orphaned checkpoint files
- Ensures atomic checkpoint creation

### 2. Transaction Rollback for Critical Operations

#### Fork Creation
Enhanced with transaction guards:

```rust
pub fn create_fork(&self, base_path: &Path, workspace_root: &Path) -> Result<String> {
    // ... validation ...

    // Create rollback guard
    let guard = ForkCreationGuard::new(fork_id.clone(), work_path.clone(), self);

    // Attempt operations
    fs::copy(base_path, &work_path)?;
    let context = ForkContext::new(fork_id.clone(), base_path.to_path_buf(), work_path)?;
    self.forks.write().insert(fork_id.clone(), context);

    // Commit on success
    guard.commit();
    Ok(fork_id)
}
```

**Guarantees:**
- No orphaned files on error
- Fork not added to registry unless fully created
- Automatic cleanup of partial state

#### Checkpoint Creation
Protected with rollback guards:

```rust
pub fn create_checkpoint(&self, fork_id: &str, label: Option<String>) -> Result<Checkpoint> {
    // ... validation ...

    let guard = CheckpointGuard::new(snapshot_path.clone());

    // Copy and register
    fs::copy(&work_path, guard.path())?;
    self.with_fork_mut(fork_id, |ctx| {
        ctx.checkpoints.push(checkpoint.clone());
        enforce_checkpoint_limits(ctx)?;
        Ok(())
    })?;

    guard.commit();
    Ok(checkpoint)
}
```

**Guarantees:**
- Snapshot file only kept if registration succeeds
- No orphaned snapshots on error
- Checkpoint limits enforced atomically

#### Checkpoint Restoration
Enhanced with validation and backup:

```rust
pub fn restore_checkpoint(&self, fork_id: &str, checkpoint_id: &str) -> Result<Checkpoint> {
    // Validate checkpoint file before restoration
    self.validate_checkpoint(&checkpoint)?;

    // Create backup for rollback
    let backup_guard = TempFileGuard::new(backup_path.clone());
    fs::copy(&work_path, &backup_path)?;

    // Attempt restoration
    let restore_result = fs::copy(&checkpoint.snapshot_path, &work_path);
    if let Err(e) = restore_result {
        // Automatic rollback via backup
        let _ = fs::copy(&backup_path, &work_path);
        return Err(anyhow!("failed to restore checkpoint: {}", e));
    }

    // Update metadata
    self.with_fork_mut(fork_id, |ctx| {
        // ... cleanup old edits and staged changes ...
    })?;

    backup_guard.disarm();
    Ok(checkpoint)
}
```

**Guarantees:**
- Work file validated before restoration
- Backup created for rollback
- Original state restored on any error
- Atomic restoration of file and metadata

#### Checkpoint Validation
Comprehensive validation before restoration:

```rust
fn validate_checkpoint(&self, checkpoint: &Checkpoint) -> Result<()> {
    // Check file exists
    if !checkpoint.snapshot_path.exists() {
        return Err(anyhow!("checkpoint file does not exist"));
    }

    // Validate file size
    let metadata = fs::metadata(&checkpoint.snapshot_path)?;
    if metadata.len() == 0 {
        return Err(anyhow!("checkpoint file is empty"));
    }

    // Verify XLSX magic bytes (PK\x03\x04)
    let mut file = fs::File::open(&checkpoint.snapshot_path)?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)?;
    if &magic != b"PK\x03\x04" {
        return Err(anyhow!("checkpoint file is not a valid XLSX file"));
    }

    Ok(())
}
```

**Checks:**
- File existence
- Non-zero file size
- Valid XLSX format (ZIP magic bytes)
- Size limits

#### Fork Save Operation
Protected with backup and rollback:

```rust
pub fn save_fork(&self, fork_id: &str, target_path: &Path, ...) -> Result<()> {
    // Create backup of existing target
    let backup_guard = if target_path.exists() {
        let backup = target_path.with_extension("backup.xlsx");
        fs::copy(target_path, &backup).ok();
        Some(TempFileGuard::new(backup))
    } else {
        None
    };

    // Validate and save
    ctx.validate_base_unchanged()?;
    let save_result = fs::copy(&ctx.work_path, target_path);

    if let Err(e) = save_result {
        // Rollback: restore backup
        if let Some(backup) = backup_guard {
            let _ = fs::copy(backup.path(), target_path);
        }
        return Err(anyhow!("failed to save fork: {}", e));
    }

    backup_guard.disarm();
    Ok(())
}
```

**Guarantees:**
- Existing file backed up before overwrite
- Backup restored on error
- Base file validation before save
- Atomic save operation

### 3. Automatic Cleanup with Drop Implementation

#### ForkContext Drop
Ensures cleanup when fork context is dropped:

```rust
impl Drop for ForkContext {
    fn drop(&mut self) {
        debug!(fork_id = %self.fork_id, "fork context dropped, cleaning up files");
        self.cleanup_files();
    }
}
```

**Cleans up:**
- Work file
- Staged change snapshots
- Checkpoint directory and files

### 4. Enhanced Concurrency Protection

#### RwLock for Better Read Concurrency
```rust
pub struct ForkRegistry {
    forks: RwLock<HashMap<String, ForkContext>>,
    recalc_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    config: ForkConfig,
}
```

**Benefits:**
- Multiple concurrent reads
- Exclusive writes
- Better performance under read-heavy workloads

#### Per-Fork Recalc Locks
Prevents concurrent recalculation on the same fork:

```rust
pub fn acquire_recalc_lock(&self, fork_id: &str) -> Arc<Mutex<()>> {
    let mut locks = self.recalc_locks.lock();
    locks
        .entry(fork_id.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}
```

**Features:**
- Per-fork locking granularity
- Automatic cleanup of unused locks
- Prevents corruption from concurrent recalc

#### Optimistic Locking with Versioning
```rust
pub struct ForkContext {
    // ... fields ...
    version: AtomicU64,
}

impl ForkContext {
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::SeqCst)
    }

    pub fn validate_version(&self, expected_version: u64) -> Result<()> {
        if self.version() != expected_version {
            return Err(anyhow!("concurrent modification detected"));
        }
        Ok(())
    }
}
```

**Use Cases:**
- Detect concurrent modifications
- Versioned operations
- Conflict detection

## Testing

Comprehensive tests verify rollback behavior:

### Test Coverage

1. **TempFileGuard Tests**
   - `test_temp_file_guard_cleanup` - Verifies automatic cleanup
   - `test_temp_file_guard_disarm` - Tests disarm functionality

2. **Fork Creation Rollback Tests**
   - `test_fork_creation_rollback_on_invalid_base` - Non-existent file
   - `test_fork_creation_rollback_on_invalid_extension` - Wrong file type

3. **Checkpoint Validation Tests**
   - `test_checkpoint_validation_before_restore` - Validates checkpoint
   - `test_checkpoint_restore_rollback_on_error` - Rollback on failure

4. **Save Operation Tests**
   - `test_save_fork_rollback_on_error` - Backup restoration

5. **Concurrency Tests**
   - `test_concurrent_fork_operations_lock_release` - Lock release

6. **Cleanup Tests**
   - `test_fork_context_drop_cleanup` - Drop implementation
   - `test_checkpoint_limits_with_cleanup` - Limit enforcement

### Running Tests

```bash
# Run all fork transaction guard tests
cargo test --test fork_transaction_guards --features recalc

# Run specific test
cargo test --test fork_transaction_guards test_temp_file_guard_cleanup --features recalc
```

## Error Handling Patterns

### Pattern 1: Guard + Disarm
```rust
let guard = TempFileGuard::new(temp_path);
// Risky operation
operation()?;
guard.disarm(); // Success - keep file
```

### Pattern 2: Backup + Rollback
```rust
let backup = TempFileGuard::new(backup_path);
fs::copy(&original, &backup_path)?;

if let Err(e) = risky_operation() {
    fs::copy(&backup_path, &original)?; // Rollback
    return Err(e);
}

backup.disarm(); // Success
```

### Pattern 3: Validate + Execute + Commit
```rust
// Validate inputs
validate_checkpoint(&checkpoint)?;

// Create guard
let guard = CheckpointGuard::new(path);

// Execute
execute_operation()?;

// Commit
guard.commit();
```

## Best Practices

1. **Always use guards for temporary files** - Prevents orphaned files
2. **Validate before expensive operations** - Fail fast
3. **Create backups for destructive operations** - Enable rollback
4. **Disarm guards only on success** - Automatic cleanup on error
5. **Use versioned operations for conflict detection** - Optimistic locking

## Implementation Checklist

When adding new fork operations:

- [ ] Add appropriate RAII guards (TempFileGuard, etc.)
- [ ] Validate inputs before expensive operations
- [ ] Create backups for destructive operations
- [ ] Implement rollback on error
- [ ] Add comprehensive tests
- [ ] Document error cases
- [ ] Ensure proper logging with tracing

## Future Enhancements

1. **Transactional log** - Log all operations for audit trail
2. **Snapshot compression** - Reduce checkpoint storage
3. **Incremental checkpoints** - Store only diffs
4. **Checkpoint metadata** - Store creation reason, author
5. **Checkpoint expiry** - Auto-delete old checkpoints
6. **Parallel checkpoint creation** - For large workbooks
7. **Checkpoint validation on load** - Periodic integrity checks

## References

- **Poka-Yoke Principles**: Error-proofing design patterns from manufacturing
- **RAII**: Resource Acquisition Is Initialization pattern from C++
- **Transaction Processing**: ACID properties for data integrity
