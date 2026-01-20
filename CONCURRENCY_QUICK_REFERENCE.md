# Concurrency Guards - Quick Reference

## TL;DR

This guide provides quick code examples for using the new concurrency protection features.

## Fork Operations

### Basic Fork Access (Read)

```rust
// Get fork information (read-only)
let fork = registry.get_fork("fork-123")?;
println!("Version: {}", fork.version());
println!("Edits: {}", fork.edits.len());
```

### Fork Modification (Basic)

```rust
// Modify fork with automatic version increment
registry.with_fork_mut("fork-123", |ctx| {
    ctx.edits.push(edit);
    Ok(())
})?;
```

### Fork Modification (With Version Check)

```rust
// Get current version
let fork = registry.get_fork("fork-123")?;
let version = fork.version();

// Later... modify with version validation
match registry.with_fork_mut_versioned("fork-123", version, |ctx| {
    ctx.edits.push(edit);
    Ok(())
}) {
    Ok(result) => {
        println!("Modification succeeded");
    }
    Err(e) if e.to_string().contains("version mismatch") => {
        println!("Concurrent modification detected, retry needed");
        // Retry logic here
    }
    Err(e) => return Err(e),
}
```

### Safe Recalc Operations

```rust
async fn recalc_fork(
    registry: &ForkRegistry,
    backend: &dyn RecalcBackend,
    fork_id: &str,
) -> Result<()> {
    // Acquire per-fork recalc lock
    let lock = registry.acquire_recalc_lock(fork_id);

    // Lock will block if another recalc is running on this fork
    let _guard = lock.lock();

    // Get fork path
    let fork_path = registry
        .get_fork_path(fork_id)
        .ok_or_else(|| anyhow!("fork not found"))?;

    // Perform recalc (exclusive access guaranteed)
    backend.recalculate(&fork_path).await?;

    // Release guard explicitly (or let it drop)
    drop(_guard);

    // Clean up lock entry
    registry.release_recalc_lock(fork_id);

    Ok(())
}
```

### Fork Creation with Error Handling

```rust
// Fork creation is automatically protected with RAII guards
match registry.create_fork(&base_path, &workspace_root) {
    Ok(fork_id) => {
        println!("Fork created: {}", fork_id);
        // Fork successfully created and registered
    }
    Err(e) => {
        // Automatic rollback: registry entry and work file cleaned up
        eprintln!("Fork creation failed: {}", e);
    }
}
```

### Checkpoint Operations

```rust
// Create checkpoint (with automatic cleanup on error)
match registry.create_checkpoint("fork-123", Some("Before changes".to_string())) {
    Ok(checkpoint) => {
        println!("Checkpoint created: {}", checkpoint.checkpoint_id);
    }
    Err(e) => {
        // Snapshot file automatically cleaned up
        eprintln!("Checkpoint failed: {}", e);
    }
}

// Restore checkpoint
registry.restore_checkpoint("fork-123", "cp-abc123")?;
```

## Workbook Cache Operations

### Open Workbook

```rust
// Optimized cache access with RwLock
let workbook = state.open_workbook(&workbook_id).await?;

// Access workbook data
println!("Sheets: {}", workbook.sheet_names.len());
```

### Monitor Cache Performance

```rust
// Get cache statistics (lock-free atomic access)
let stats = state.cache_stats();

println!("Cache Statistics:");
println!("  Operations: {}", stats.operations);
println!("  Hits: {}", stats.hits);
println!("  Misses: {}", stats.misses);
println!("  Hit Rate: {:.2}%", stats.hit_rate() * 100.0);
println!("  Size: {}/{}", stats.size, stats.capacity);

// Check if cache needs tuning
if stats.hit_rate() < 0.5 {
    println!("Warning: Low cache hit rate, consider increasing cache capacity");
}
```

### Evict Workbooks

```rust
// Evict by ID
state.close_workbook(&workbook_id)?;

// Evict by path (useful for file watchers)
state.evict_by_path(Path::new("/path/to/workbook.xlsx"));
```

## Common Patterns

### Retry on Version Conflict

```rust
use std::cmp::min;
use std::time::Duration;
use tokio::time::sleep;

async fn modify_fork_with_retry(
    registry: &ForkRegistry,
    fork_id: &str,
    max_retries: u32,
) -> Result<()> {
    let mut attempt = 0;

    loop {
        // Get current version
        let fork = registry.get_fork(fork_id)?;
        let version = fork.version();

        // Try to modify
        match registry.with_fork_mut_versioned(fork_id, version, |ctx| {
            // Your modification here
            ctx.edits.push(/* ... */);
            Ok(())
        }) {
            Ok(_) => return Ok(()),
            Err(e) if e.to_string().contains("version mismatch") => {
                attempt += 1;
                if attempt >= max_retries {
                    return Err(anyhow!("Max retries exceeded"));
                }

                // Exponential backoff
                let delay = Duration::from_millis(10 * 2_u64.pow(min(attempt, 5)));
                sleep(delay).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### Batch Operations with Version Check

```rust
fn batch_edit_fork(
    registry: &ForkRegistry,
    fork_id: &str,
    edits: Vec<EditOp>,
) -> Result<u64> {
    // Get initial version
    let fork = registry.get_fork(fork_id)?;
    let version = fork.version();

    // Apply all edits in single transaction
    registry.with_fork_mut_versioned(fork_id, version, |ctx| {
        for edit in edits {
            ctx.edits.push(edit);
        }
        Ok(ctx.version())
    })
}
```

### Concurrent Fork Operations

```rust
use tokio::task::JoinSet;

async fn process_forks_concurrently(
    registry: Arc<ForkRegistry>,
    fork_ids: Vec<String>,
) -> Result<Vec<Result<()>>> {
    let mut set = JoinSet::new();

    for fork_id in fork_ids {
        let registry = registry.clone();
        set.spawn(async move {
            // Each fork can be processed concurrently
            // Per-fork locks ensure safety
            process_single_fork(&registry, &fork_id).await
        });
    }

    let mut results = Vec::new();
    while let Some(res) = set.join_next().await {
        results.push(res?);
    }

    Ok(results)
}

async fn process_single_fork(
    registry: &ForkRegistry,
    fork_id: &str,
) -> Result<()> {
    // Acquire recalc lock (may wait if another task is processing this fork)
    let lock = registry.acquire_recalc_lock(fork_id);
    let _guard = lock.lock();

    // Process fork
    // ... your logic here ...

    Ok(())
}
```

### Safe Resource Cleanup

```rust
use scopeguard::defer;

fn complex_fork_operation(registry: &ForkRegistry) -> Result<()> {
    let fork_id = registry.create_fork(&base_path, &workspace_root)?;

    // Ensure cleanup even on panic
    defer! {
        let _ = registry.discard_fork(&fork_id);
    }

    // Do complex operations
    // ...

    Ok(())
}
```

## Debug Logging

Enable debug logging to see concurrency details:

```bash
RUST_LOG=ggen_mcp=debug cargo run
```

Look for:
- `cache hit` / `cache miss` - Cache performance
- `created fork` / `discarded fork` - Fork lifecycle
- `created checkpoint` / `restored checkpoint` - Checkpoint operations
- `resolved to fork path` / `resolved from index` - Path resolution
- `registered workbook location` - Index updates

## Performance Tips

### 1. Minimize Lock Hold Time

**Bad:**
```rust
let mut forks = registry.forks.write();
// Expensive I/O while holding lock
let data = expensive_io_operation()?;
forks.get_mut(fork_id).unwrap().data = data;
```

**Good:**
```rust
// Do I/O without lock
let data = expensive_io_operation()?;

// Hold lock only for mutation
registry.with_fork_mut(fork_id, |ctx| {
    ctx.data = data;
    Ok(())
})?;
```

### 2. Use Read Locks When Possible

**Bad:**
```rust
let forks = registry.forks.write(); // Exclusive access
let count = forks.len();
```

**Good:**
```rust
let forks = registry.forks.read(); // Shared access
let count = forks.len();
```

### 3. Monitor Cache Hit Rate

```rust
// Periodically check cache performance
let stats = state.cache_stats();
if stats.operations > 1000 && stats.hit_rate() < 0.8 {
    tracing::warn!(
        "Low cache hit rate: {:.2}% ({}/{})",
        stats.hit_rate() * 100.0,
        stats.hits,
        stats.operations
    );
}
```

### 4. Batch Version Checks

**Bad:**
```rust
for edit in edits {
    let version = fork.version();
    registry.with_fork_mut_versioned(fork_id, version, |ctx| {
        ctx.edits.push(edit);
        Ok(())
    })?;
}
```

**Good:**
```rust
let version = fork.version();
registry.with_fork_mut_versioned(fork_id, version, |ctx| {
    for edit in edits {
        ctx.edits.push(edit);
    }
    Ok(())
})?;
```

## Troubleshooting

### Version Mismatch Errors

**Symptom:** `version mismatch: expected X, got Y (concurrent modification detected)`

**Cause:** Another thread modified the fork between version check and mutation

**Solution:**
1. Implement retry logic with exponential backoff
2. Use `with_fork_mut()` without version check if optimistic locking not needed
3. Reduce concurrent modifications to same fork

### Deadlocks

**Symptom:** Application hangs, no progress

**Cause:** Circular lock dependencies or holding multiple locks

**Solution:**
1. Always acquire locks in same order
2. Release locks before acquiring new ones
3. Use `with_fork_mut()` instead of manual lock management

### Performance Issues

**Symptom:** Slow response times, high CPU usage

**Diagnosis:**
```rust
// Add instrumentation
let start = Instant::now();
registry.with_fork_mut(fork_id, |ctx| {
    // Your operation
    Ok(())
})?;
println!("Operation took: {:?}", start.elapsed());

// Check cache stats
let stats = state.cache_stats();
println!("Cache hit rate: {:.2}%", stats.hit_rate() * 100.0);
```

**Solutions:**
1. Increase cache capacity if hit rate < 80%
2. Reduce lock hold times
3. Use read locks instead of write locks where possible
4. Batch operations to reduce lock acquisitions

## See Also

- [CONCURRENCY_ENHANCEMENTS.md](CONCURRENCY_ENHANCEMENTS.md) - Detailed documentation
- [CHANGES_SUMMARY.md](CHANGES_SUMMARY.md) - Summary of all changes
- [parking_lot documentation](https://docs.rs/parking_lot/) - Lock primitives
- [std::sync::atomic](https://doc.rust-lang.org/std/sync/atomic/) - Atomic operations
