# Concurrency Guards and Race Condition Prevention

This document describes the comprehensive concurrency enhancements added to prevent race conditions and data corruption in the ggen-mcp project.

## Overview

The enhancements focus on five key areas:
1. Fork operation protection from concurrent modification
2. Read/write locks for workbook cache access
3. Version checking for optimistic locking
4. Per-fork recalc locks to prevent concurrent recalculations
5. Atomic operations for state transitions

## Changes to `src/fork.rs`

### 1. Version Tracking for Optimistic Locking

**Added to `ForkContext`:**
```rust
version: AtomicU64,
```

**New Methods:**
- `version()` - Get current version
- `increment_version()` - Atomically increment version after modification
- `validate_version(expected: u64)` - Validate version for optimistic locking

**Benefits:**
- Detect concurrent modifications
- Prevent lost updates
- Enable safe concurrent access patterns

### 2. RwLock for Better Read Concurrency

**Changed from:**
```rust
forks: Mutex<HashMap<String, ForkContext>>,
```

**To:**
```rust
forks: RwLock<HashMap<String, ForkContext>>,
```

**Benefits:**
- Multiple concurrent readers
- Exclusive write access when needed
- Better throughput for read-heavy workloads

### 3. Per-Fork Recalc Locks

**Added to `ForkRegistry`:**
```rust
recalc_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
```

**New Methods:**
- `acquire_recalc_lock(fork_id: &str)` - Get per-fork recalc lock
- `release_recalc_lock(fork_id: &str)` - Clean up recalc lock

**Benefits:**
- Prevent concurrent recalc operations on the same fork
- Allow concurrent recalc on different forks
- Automatic cleanup when fork is discarded

### 4. Enhanced RAII Guards

**Added Guards:**
- `ForkCreationGuard` - Automatic rollback on fork creation failure
- `CheckpointGuard` - Automatic cleanup on checkpoint creation failure
- `TempFileGuard` - Automatic file cleanup on drop

**Benefits:**
- Exception-safe resource management
- Automatic cleanup on errors
- No resource leaks

### 5. Version-Aware Mutation Methods

**New Methods:**
- `with_fork_mut()` - Execute function with automatic version increment
- `with_fork_mut_versioned()` - Execute with version validation (optimistic locking)

**Benefits:**
- Consistent version management
- Prevent race conditions on modifications
- Enable optimistic concurrency control

## Changes to `src/state.rs`

### 1. Cache Statistics with Atomic Counters

**Added Fields:**
```rust
cache_ops: AtomicU64,     // Total cache operations
cache_hits: AtomicU64,    // Cache hits
cache_misses: AtomicU64,  // Cache misses
```

**New Methods:**
- `cache_stats()` - Get cache statistics
- `CacheStats::hit_rate()` - Calculate cache hit rate

**Benefits:**
- Monitor cache performance
- No locking overhead for statistics
- Thread-safe counters

### 2. Optimized Lock Granularity

**Improvements:**
- Check cache with read lock first before acquiring write lock
- Release locks before expensive operations (filesystem I/O)
- Minimize lock hold times
- Use separate locks for cache, index, and aliases

**Benefits:**
- Reduced lock contention
- Better concurrent performance
- Fewer blocking operations

### 3. Enhanced Logging

**Added:**
- Debug logging for all cache operations
- Path resolution logging
- Workbook registration logging

**Benefits:**
- Better observability
- Easier debugging
- Performance monitoring

## Usage Examples

### Fork Operations with Optimistic Locking

```rust
// Get fork and version
let fork = registry.get_fork("fork-123")?;
let version = fork.version();

// Perform operation with version check
registry.with_fork_mut_versioned("fork-123", version, |ctx| {
    // Modify fork context
    ctx.edits.push(edit);
    Ok(())
})?;
```

### Per-Fork Recalc Protection

```rust
// Acquire per-fork recalc lock
let lock = registry.acquire_recalc_lock(&fork_id);
let _guard = lock.lock();

// Perform recalc - no other recalc can run on this fork
recalc_backend.recalculate(&fork_path).await?;

// Lock automatically released when _guard is dropped
// Clean up lock entry when done
registry.release_recalc_lock(&fork_id);
```

### Cache Statistics Monitoring

```rust
let stats = state.cache_stats();
println!("Cache hit rate: {:.2}%", stats.hit_rate() * 100.0);
println!("Cache size: {}/{}", stats.size, stats.capacity);
println!("Total operations: {}", stats.operations);
```

## Concurrency Guarantees

### Fork Operations
- ✅ **Fork creation** - Protected by RwLock, atomic file operations
- ✅ **Fork modification** - Protected by version checking
- ✅ **Fork deletion** - Protected by write lock, cleanup guaranteed
- ✅ **Concurrent reads** - Multiple readers allowed via RwLock
- ✅ **Recalc operations** - Per-fork locks prevent concurrent recalc

### Workbook Cache
- ✅ **Cache reads** - Lock-free after initial read lock
- ✅ **Cache writes** - Exclusive write lock
- ✅ **Eviction** - Atomic operation with write lock
- ✅ **Statistics** - Lock-free atomic counters

### State Transitions
- ✅ **Version increments** - Atomic operations
- ✅ **Statistics updates** - Atomic operations
- ✅ **Index updates** - Protected by RwLock

## Testing Recommendations

### Concurrency Tests
1. **Parallel fork creation** - Verify max_forks limit enforcement
2. **Concurrent modifications** - Test optimistic locking with version checks
3. **Concurrent recalc** - Verify per-fork serialization
4. **Cache stress test** - Multiple threads reading/writing
5. **Eviction race** - Concurrent eviction and access

### Performance Tests
1. **Read scalability** - Measure throughput with multiple readers
2. **Write contention** - Measure latency under write load
3. **Cache hit rate** - Monitor cache effectiveness
4. **Lock contention** - Profile lock wait times

## Migration Guide

### For Existing Code

**Before:**
```rust
let fork = registry.get_fork("fork-123")?;
// fork might be stale
```

**After:**
```rust
// With version checking
let fork = registry.get_fork("fork-123")?;
let version = fork.version();
// Use version for optimistic locking
registry.with_fork_mut_versioned("fork-123", version, |ctx| {
    // Safe modification
    Ok(())
})?;
```

### For Recalc Operations

**Before:**
```rust
// No protection against concurrent recalc
recalc_backend.recalculate(&fork_path).await?;
```

**After:**
```rust
// Protected with per-fork lock
let lock = registry.acquire_recalc_lock(&fork_id);
let _guard = lock.lock();
recalc_backend.recalculate(&fork_path).await?;
drop(_guard);
registry.release_recalc_lock(&fork_id);
```

## Performance Impact

### Expected Improvements
- **Read throughput**: 2-5x improvement for read-heavy workloads (RwLock)
- **Write latency**: Minimal increase due to version increment (atomic operation)
- **Recalc**: Better concurrency for different forks, proper serialization per fork
- **Cache**: Reduced contention, better hit rates with monitoring

### Overhead
- **Memory**: +8 bytes per fork (AtomicU64), +24 bytes per state (3x AtomicU64)
- **CPU**: Minimal (atomic operations, no syscalls)
- **Lock contention**: Reduced overall due to RwLock

## Future Enhancements

1. **Lock-free cache** - Use concurrent hash map for better scalability
2. **Async locks** - Use tokio::sync::RwLock for better async integration
3. **Distributed locking** - For multi-process deployments
4. **Deadlock detection** - Runtime deadlock monitoring
5. **Lock metrics** - Prometheus/OpenTelemetry integration

## References

- parking_lot documentation: https://docs.rs/parking_lot/
- Rust atomics: https://doc.rust-lang.org/std/sync/atomic/
- Optimistic locking patterns: https://en.wikipedia.org/wiki/Optimistic_concurrency_control
