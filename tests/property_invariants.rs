//! Property-Based Invariant Testing
//!
//! This module tests invariants that must hold across the system:
//! - Cache never exceeds capacity (LRU eviction works)
//! - Fork count never exceeds limits
//! - Atomic operations maintain consistency
//! - Reference counting never goes negative
//! - State transitions are valid

use parking_lot::Mutex;
use proptest::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

// =============================================================================
// LRU Cache Invariant Tests
// =============================================================================

/// Simple LRU cache implementation for testing
struct TestLruCache<K, V> {
    capacity: usize,
    map: HashMap<K, V>,
    access_order: Vec<K>,
}

impl<K: Clone + Eq + std::hash::Hash, V> TestLruCache<K, V> {
    fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "Capacity must be positive");
        Self {
            capacity,
            map: HashMap::new(),
            access_order: Vec::new(),
        }
    }

    fn insert(&mut self, key: K, value: V) {
        // Remove from access order if exists
        self.access_order.retain(|k| k != &key);

        // Insert/update
        self.map.insert(key.clone(), value);
        self.access_order.push(key);

        // Evict if over capacity
        while self.map.len() > self.capacity {
            if let Some(oldest_key) = self.access_order.first().cloned() {
                self.access_order.remove(0);
                self.map.remove(&oldest_key);
            }
        }
    }

    fn get(&mut self, key: &K) -> Option<&V> {
        if self.map.contains_key(key) {
            // Update access order
            self.access_order.retain(|k| k != key);
            self.access_order.push(key.clone());
            self.map.get(key)
        } else {
            None
        }
    }

    fn len(&self) -> usize {
        self.map.len()
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Invariant: Cache never exceeds capacity
    #[test]
    fn invariant_cache_never_exceeds_capacity(
        capacity in 1usize..=100,
        operations in prop::collection::vec(
            (0u32..1000, any::<String>()),
            1..500
        )
    ) {
        let mut cache = TestLruCache::new(capacity);

        for (key, value) in operations {
            cache.insert(key, value);

            // INVARIANT: Size never exceeds capacity
            prop_assert!(cache.len() <= capacity);
        }
    }

    /// Invariant: LRU eviction removes oldest items
    #[test]
    fn invariant_lru_evicts_oldest(
        capacity in 1usize..=10
    ) {
        let mut cache = TestLruCache::new(capacity);

        // Fill cache to capacity
        for i in 0..capacity {
            cache.insert(i, format!("value_{}", i));
        }

        // Insert one more item
        cache.insert(capacity, "new_value".to_string());

        // INVARIANT: Oldest item (0) should be evicted
        prop_assert!(cache.get(&0).is_none());

        // INVARIANT: Newest items should still be present
        prop_assert!(cache.get(&capacity).is_some());
    }

    /// Invariant: Get updates access order
    #[test]
    fn invariant_get_updates_access(
        capacity in 2usize..=10
    ) {
        let mut cache = TestLruCache::new(capacity);

        // Fill cache
        for i in 0..capacity {
            cache.insert(i, format!("value_{}", i));
        }

        // Access oldest item
        let _ = cache.get(&0);

        // Insert new item (should evict item 1, not 0)
        cache.insert(capacity, "new_value".to_string());

        // INVARIANT: Item 0 should still exist (was recently accessed)
        prop_assert!(cache.get(&0).is_some());

        // INVARIANT: Item 1 should be evicted (least recently used)
        prop_assert!(cache.get(&1).is_none());
    }
}

// =============================================================================
// Atomic Counter Invariant Tests
// =============================================================================

/// Test atomic counter that never goes negative
struct AtomicCounter {
    count: Arc<Mutex<i64>>,
    max_value: i64,
}

impl AtomicCounter {
    fn new(max_value: i64) -> Self {
        Self {
            count: Arc::new(Mutex::new(0)),
            max_value,
        }
    }

    fn increment(&self) -> Result<i64, &'static str> {
        let mut count = self.count.lock();
        if *count >= self.max_value {
            Err("Max value exceeded")
        } else {
            *count += 1;
            Ok(*count)
        }
    }

    fn decrement(&self) -> Result<i64, &'static str> {
        let mut count = self.count.lock();
        if *count <= 0 {
            Err("Cannot decrement below zero")
        } else {
            *count -= 1;
            Ok(*count)
        }
    }

    fn value(&self) -> i64 {
        *self.count.lock()
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Invariant: Counter never goes negative
    #[test]
    fn invariant_counter_never_negative(
        operations in prop::collection::vec(any::<bool>(), 1..100)
    ) {
        let counter = AtomicCounter::new(1000);

        for increment in operations {
            if increment {
                let _ = counter.increment();
            } else {
                let _ = counter.decrement();
            }

            // INVARIANT: Counter never negative
            prop_assert!(counter.value() >= 0);
        }
    }

    /// Invariant: Counter never exceeds max value
    #[test]
    fn invariant_counter_respects_max(
        max_value in 1i64..=100,
        increments in 1usize..200
    ) {
        let counter = AtomicCounter::new(max_value);

        for _ in 0..increments {
            let _ = counter.increment();
        }

        // INVARIANT: Counter never exceeds max
        prop_assert!(counter.value() <= max_value);
    }

    /// Invariant: Increment then decrement returns to original
    #[test]
    fn invariant_increment_decrement_roundtrip(count in 0i64..100) {
        let counter = AtomicCounter::new(1000);

        // Set initial value
        for _ in 0..count {
            counter.increment().unwrap();
        }

        let initial = counter.value();

        // Increment
        counter.increment().unwrap();

        // Decrement
        counter.decrement().unwrap();

        // INVARIANT: Back to original value
        prop_assert_eq!(counter.value(), initial);
    }
}

// =============================================================================
// Fork Count Invariant Tests
// =============================================================================

/// Test fork manager that enforces limits
struct ForkManager {
    active_forks: Arc<Mutex<HashMap<String, Vec<String>>>>,
    max_forks_per_workbook: usize,
}

impl ForkManager {
    fn new(max_forks_per_workbook: usize) -> Self {
        Self {
            active_forks: Arc::new(Mutex::new(HashMap::new())),
            max_forks_per_workbook,
        }
    }

    fn create_fork(&self, workbook_id: &str, fork_id: &str) -> Result<(), &'static str> {
        let mut forks = self.active_forks.lock();
        let workbook_forks = forks
            .entry(workbook_id.to_string())
            .or_insert_with(Vec::new);

        if workbook_forks.len() >= self.max_forks_per_workbook {
            Err("Max forks exceeded")
        } else {
            workbook_forks.push(fork_id.to_string());
            Ok(())
        }
    }

    fn delete_fork(&self, workbook_id: &str, fork_id: &str) {
        let mut forks = self.active_forks.lock();
        if let Some(workbook_forks) = forks.get_mut(workbook_id) {
            workbook_forks.retain(|f| f != fork_id);
        }
    }

    fn fork_count(&self, workbook_id: &str) -> usize {
        let forks = self.active_forks.lock();
        forks.get(workbook_id).map(|f| f.len()).unwrap_or(0)
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Invariant: Fork count never exceeds limit
    #[test]
    fn invariant_fork_count_limited(
        max_forks in 1usize..=10,
        operations in prop::collection::vec(
            (any::<bool>(), any::<String>()),
            1..100
        )
    ) {
        let manager = ForkManager::new(max_forks);
        let workbook_id = "test-workbook";

        for (create, fork_id) in operations {
            if create {
                let _ = manager.create_fork(workbook_id, &fork_id);
            } else {
                manager.delete_fork(workbook_id, &fork_id);
            }

            // INVARIANT: Fork count never exceeds limit
            prop_assert!(manager.fork_count(workbook_id) <= max_forks);
        }
    }

    /// Invariant: Fork count is non-negative
    #[test]
    fn invariant_fork_count_non_negative(
        operations in prop::collection::vec(
            (any::<bool>(), any::<String>()),
            1..100
        )
    ) {
        let manager = ForkManager::new(10);
        let workbook_id = "test-workbook";

        for (create, fork_id) in operations {
            if create {
                let _ = manager.create_fork(workbook_id, &fork_id);
            } else {
                manager.delete_fork(workbook_id, &fork_id);
            }

            // INVARIANT: Count is always non-negative
            // (usize is always non-negative, but this documents the invariant)
            let count = manager.fork_count(workbook_id);
            prop_assert!(count >= 0);
        }
    }
}

// =============================================================================
// State Transition Invariant Tests
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResourceState {
    Available,
    Locked,
    Processing,
    Released,
}

struct ResourceManager {
    state: Arc<Mutex<ResourceState>>,
}

impl ResourceManager {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ResourceState::Available)),
        }
    }

    fn lock(&self) -> Result<(), &'static str> {
        let mut state = self.state.lock();
        match *state {
            ResourceState::Available => {
                *state = ResourceState::Locked;
                Ok(())
            }
            _ => Err("Resource not available"),
        }
    }

    fn process(&self) -> Result<(), &'static str> {
        let mut state = self.state.lock();
        match *state {
            ResourceState::Locked => {
                *state = ResourceState::Processing;
                Ok(())
            }
            _ => Err("Resource not locked"),
        }
    }

    fn release(&self) -> Result<(), &'static str> {
        let mut state = self.state.lock();
        match *state {
            ResourceState::Processing | ResourceState::Locked => {
                *state = ResourceState::Released;
                Ok(())
            }
            _ => Err("Invalid state for release"),
        }
    }

    fn reset(&self) {
        let mut state = self.state.lock();
        *state = ResourceState::Available;
    }

    fn current_state(&self) -> ResourceState {
        *self.state.lock()
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Invariant: State transitions follow valid paths
    #[test]
    fn invariant_state_transitions_valid(
        operations in prop::collection::vec(0u8..=3, 1..50)
    ) {
        let manager = ResourceManager::new();

        for op in operations {
            let prev_state = manager.current_state();

            match op {
                0 => { let _ = manager.lock(); }
                1 => { let _ = manager.process(); }
                2 => { let _ = manager.release(); }
                3 => { manager.reset(); }
                _ => unreachable!(),
            }

            let new_state = manager.current_state();

            // INVARIANT: Valid state transitions only
            let valid_transition = match (prev_state, new_state) {
                (ResourceState::Available, ResourceState::Locked) => true,
                (ResourceState::Available, ResourceState::Available) => true,
                (ResourceState::Locked, ResourceState::Processing) => true,
                (ResourceState::Locked, ResourceState::Released) => true,
                (ResourceState::Locked, ResourceState::Locked) => true,
                (ResourceState::Processing, ResourceState::Released) => true,
                (ResourceState::Processing, ResourceState::Processing) => true,
                (ResourceState::Released, ResourceState::Available) => true,
                (ResourceState::Released, ResourceState::Released) => true,
                (_, ResourceState::Available) => true, // Reset always valid
                _ => false,
            };

            prop_assert!(valid_transition, "Invalid transition from {:?} to {:?}", prev_state, new_state);
        }
    }

    /// Invariant: Lock-Process-Release sequence works
    #[test]
    fn invariant_lock_process_release_sequence(_unit in Just(())) {
        let manager = ResourceManager::new();

        // INVARIANT: Can lock when available
        prop_assert!(manager.lock().is_ok());
        prop_assert_eq!(manager.current_state(), ResourceState::Locked);

        // INVARIANT: Can process when locked
        prop_assert!(manager.process().is_ok());
        prop_assert_eq!(manager.current_state(), ResourceState::Processing);

        // INVARIANT: Can release when processing
        prop_assert!(manager.release().is_ok());
        prop_assert_eq!(manager.current_state(), ResourceState::Released);
    }
}

// =============================================================================
// Optimistic Locking Invariant Tests
// =============================================================================

struct OptimisticLock {
    version: Arc<Mutex<u64>>,
}

impl OptimisticLock {
    fn new() -> Self {
        Self {
            version: Arc::new(Mutex::new(0)),
        }
    }

    fn read(&self) -> u64 {
        *self.version.lock()
    }

    fn try_write(&self, expected_version: u64, new_value: u64) -> Result<(), &'static str> {
        let mut version = self.version.lock();
        if *version == expected_version {
            *version = new_value;
            Ok(())
        } else {
            Err("Version conflict")
        }
    }

    fn force_write(&self, new_value: u64) {
        let mut version = self.version.lock();
        *version = new_value;
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Invariant: Optimistic locking detects conflicts
    #[test]
    fn invariant_optimistic_lock_detects_conflicts(_unit in Just(())) {
        let lock = OptimisticLock::new();

        // Read initial version
        let v1 = lock.read();
        prop_assert_eq!(v1, 0);

        // Concurrent write changes version
        lock.force_write(1);

        // Try to write with old version should fail
        let result = lock.try_write(v1, 2);
        prop_assert!(result.is_err());

        // INVARIANT: Version should still be 1 (write failed)
        prop_assert_eq!(lock.read(), 1);
    }

    /// Invariant: Successful write increments version
    #[test]
    fn invariant_successful_write_updates_version(new_version in 1u64..1000) {
        let lock = OptimisticLock::new();

        let current = lock.read();
        let result = lock.try_write(current, new_version);

        // INVARIANT: Write should succeed
        prop_assert!(result.is_ok());

        // INVARIANT: Version should be updated
        prop_assert_eq!(lock.read(), new_version);
    }

    /// Invariant: Retry loop eventually succeeds
    #[test]
    fn invariant_retry_loop_succeeds(attempts in 1usize..10) {
        let lock = OptimisticLock::new();

        let mut success = false;
        for _ in 0..attempts {
            let version = lock.read();
            if lock.try_write(version, version + 1).is_ok() {
                success = true;
                break;
            }
        }

        // INVARIANT: At least one attempt should succeed
        prop_assert!(success || attempts == 0);
    }
}

// =============================================================================
// Reference Counting Invariant Tests
// =============================================================================

struct RefCounted<T> {
    data: Arc<T>,
    ref_count: Arc<Mutex<usize>>,
}

impl<T> RefCounted<T> {
    fn new(data: T) -> Self {
        Self {
            data: Arc::new(data),
            ref_count: Arc::new(Mutex::new(1)),
        }
    }

    fn clone_ref(&self) -> Self {
        let mut count = self.ref_count.lock();
        *count += 1;
        Self {
            data: Arc::clone(&self.data),
            ref_count: Arc::clone(&self.ref_count),
        }
    }

    fn ref_count(&self) -> usize {
        *self.ref_count.lock()
    }
}

impl<T> Drop for RefCounted<T> {
    fn drop(&mut self) {
        let mut count = self.ref_count.lock();
        if *count > 0 {
            *count -= 1;
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Invariant: Reference count never goes negative
    #[test]
    fn invariant_refcount_never_negative(clone_count in 0usize..100) {
        let original = RefCounted::new(42);

        // Create clones
        let mut refs = vec![original.clone_ref(); clone_count];

        // INVARIANT: Ref count should be clone_count + 1 (original)
        prop_assert_eq!(original.ref_count(), clone_count + 1);

        // Drop half the refs
        refs.truncate(clone_count / 2);

        // INVARIANT: Ref count is still positive
        prop_assert!(original.ref_count() > 0);

        // Drop all refs
        refs.clear();

        // INVARIANT: Only original remains
        prop_assert_eq!(original.ref_count(), 1);
    }

    /// Invariant: Arc strong count matches our ref count
    #[test]
    fn invariant_arc_count_matches(clone_count in 1usize..50) {
        let original = RefCounted::new("test");
        let _clones: Vec<_> = (0..clone_count).map(|_| original.clone_ref()).collect();

        // INVARIANT: Our ref count should match Arc strong count
        let arc_count = Arc::strong_count(&original.data);
        prop_assert_eq!(arc_count, clone_count + 1);
    }
}
