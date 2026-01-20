//! Memory Safety Patterns for Rust MCP Servers
//!
//! This file demonstrates practical memory safety patterns found in ggen-mcp
//! and applicable to any Rust MCP server.
//!
//! Run with: cargo run --example memory_safety_patterns

use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

// =============================================================================
// Example 1: RAII Guards for Resource Cleanup
// =============================================================================

/// RAII guard ensures file cleanup even on early return or panic
pub struct TempFileGuard {
    path: PathBuf,
    cleanup_on_drop: bool,
}

impl TempFileGuard {
    pub fn new(path: PathBuf) -> Self {
        println!("Created temp file: {:?}", path);
        Self {
            path,
            cleanup_on_drop: true,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Disarm the guard - file will not be deleted on drop
    pub fn disarm(mut self) -> PathBuf {
        self.cleanup_on_drop = false;
        self.path.clone()
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.cleanup_on_drop {
            println!("Cleaning up temp file: {:?}", self.path);
            // In real code: std::fs::remove_file(&self.path)
        } else {
            println!("Temp file guard disarmed, not cleaning: {:?}", self.path);
        }
    }
}

fn example_raii_guard() {
    println!("\n=== Example 1: RAII Guards ===");

    let temp_path = PathBuf::from("/tmp/example.txt");
    let guard = TempFileGuard::new(temp_path.clone());

    println!("Using temp file: {:?}", guard.path());

    // File automatically cleaned up when guard drops
    // Even if we return early or panic!
}

fn example_raii_guard_disarm() {
    println!("\n=== Example 1b: RAII Guard Disarm ===");

    let temp_path = PathBuf::from("/tmp/keep_this.txt");
    let guard = TempFileGuard::new(temp_path.clone());

    println!("Using temp file: {:?}", guard.path());

    // Disarm guard - we want to keep this file
    let kept_path = guard.disarm();
    println!("Kept file: {:?}", kept_path);

    // Guard dropped but file NOT cleaned up
}

// =============================================================================
// Example 2: Shared State with Arc and RwLock
// =============================================================================

#[derive(Clone)]
pub struct CacheEntry {
    data: String,
    accessed_at: std::time::Instant,
}

pub struct SharedCache {
    // RwLock allows multiple concurrent readers, single writer
    entries: RwLock<HashMap<String, Arc<CacheEntry>>>,

    // Atomic counters (lock-free)
    hits: AtomicU64,
    misses: AtomicU64,
}

impl SharedCache {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            entries: RwLock::new(HashMap::new()),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        })
    }

    pub fn get(&self, key: &str) -> Option<Arc<CacheEntry>> {
        // Read lock (shared, many readers allowed)
        let entries = self.entries.read();

        if let Some(entry) = entries.get(key) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(entry.clone()) // Cheap Arc clone
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
        // Read lock automatically released
    }

    pub fn insert(&self, key: String, data: String) {
        let entry = Arc::new(CacheEntry {
            data,
            accessed_at: std::time::Instant::now(),
        });

        // Write lock (exclusive, blocks all readers and writers)
        let mut entries = self.entries.write();
        entries.insert(key, entry);
        // Write lock automatically released
    }

    pub fn stats(&self) -> (u64, u64) {
        (
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
        )
    }
}

fn example_shared_cache() {
    println!("\n=== Example 2: Shared Cache ===");

    let cache = SharedCache::new();

    // Insert some data
    cache.insert("user:123".to_string(), "Alice".to_string());
    cache.insert("user:456".to_string(), "Bob".to_string());

    // Multiple threads can read concurrently
    let cache_clone1 = Arc::clone(&cache);
    let handle1 = std::thread::spawn(move || {
        if let Some(entry) = cache_clone1.get("user:123") {
            println!("Thread 1: Got entry: {}", entry.data);
        }
    });

    let cache_clone2 = Arc::clone(&cache);
    let handle2 = std::thread::spawn(move || {
        if let Some(entry) = cache_clone2.get("user:456") {
            println!("Thread 2: Got entry: {}", entry.data);
        }
    });

    handle1.join().unwrap();
    handle2.join().unwrap();

    let (hits, misses) = cache.stats();
    println!("Cache stats: {} hits, {} misses", hits, misses);
}

// =============================================================================
// Example 3: String Capacity Pre-allocation
// =============================================================================

/// Format cell address (e.g., "A1", "XFD1048576")
fn format_cell_address(row: u32, col: u32) -> String {
    // Pre-allocate capacity (max: "XFD1048576" = 10 chars)
    let mut addr = String::with_capacity(10);

    // Convert column number to letters (A, B, ..., Z, AA, AB, ...)
    let mut col_num = col;
    let mut letters = Vec::new();

    while col_num > 0 {
        col_num -= 1;
        let letter = (b'A' + (col_num % 26) as u8) as char;
        letters.push(letter);
        col_num /= 26;
    }

    // Reverse and add to address
    for letter in letters.iter().rev() {
        addr.push(*letter);
    }

    // Add row number
    addr.push_str(&row.to_string());

    addr // No reallocation if capacity was sufficient
}

fn example_string_capacity() {
    println!("\n=== Example 3: String Capacity ===");

    let addresses = vec![
        format_cell_address(1, 1),       // A1
        format_cell_address(1, 26),      // Z1
        format_cell_address(1, 27),      // AA1
        format_cell_address(1048576, 16384), // XFD1048576 (max)
    ];

    for addr in addresses {
        println!("Cell address: {} (len={}, cap={})", addr, addr.len(), addr.capacity());
    }
}

// =============================================================================
// Example 4: Bounded Cache with LRU Eviction
// =============================================================================

/// Simple LRU cache implementation for demonstration
pub struct LruCache<K, V> {
    map: HashMap<K, V>,
    capacity: usize,
    access_order: Vec<K>,
}

impl<K: Clone + Eq + std::hash::Hash, V> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
            capacity,
            access_order: Vec::with_capacity(capacity),
        }
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.map.contains_key(key) {
            // Update access order (move to end)
            self.access_order.retain(|k| k != key);
            self.access_order.push(key.clone());
        }
        self.map.get(key)
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<(K, V)> {
        // If at capacity, evict least recently used
        let evicted = if self.map.len() >= self.capacity && !self.map.contains_key(&key) {
            if let Some(lru_key) = self.access_order.first().cloned() {
                self.access_order.remove(0);
                if let Some(evicted_value) = self.map.remove(&lru_key) {
                    Some((lru_key, evicted_value))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Insert new entry
        self.map.insert(key.clone(), value);

        // Update access order
        self.access_order.retain(|k| k != &key);
        self.access_order.push(key);

        evicted
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
}

fn example_bounded_cache() {
    println!("\n=== Example 4: Bounded LRU Cache ===");

    let mut cache = LruCache::new(3); // Capacity: 3

    println!("Inserting entries...");
    cache.insert("a", 1);
    cache.insert("b", 2);
    cache.insert("c", 3);
    println!("Cache size: {}", cache.len());

    // This should evict "a" (least recently used)
    println!("Inserting 'd' (should evict 'a')...");
    if let Some((evicted_key, evicted_value)) = cache.insert("d", 4) {
        println!("Evicted: {} -> {}", evicted_key, evicted_value);
    }

    println!("Cache size: {}", cache.len());

    // Access "b" to make it most recently used
    println!("Accessing 'b'...");
    cache.get(&"b");

    // This should evict "c" (least recently used, since "b" was accessed)
    println!("Inserting 'e' (should evict 'c')...");
    if let Some((evicted_key, evicted_value)) = cache.insert("e", 5) {
        println!("Evicted: {} -> {}", evicted_key, evicted_value);
    }

    println!("Final cache size: {}", cache.len());
}

// =============================================================================
// Example 5: Weak References to Break Cycles
// =============================================================================

use std::sync::Weak;

pub struct Parent {
    name: String,
    children: Mutex<Vec<Arc<Child>>>,
}

pub struct Child {
    name: String,
    parent: Weak<Parent>, // Weak prevents cycle
}

impl Parent {
    pub fn new(name: String) -> Arc<Self> {
        Arc::new(Self {
            name,
            children: Mutex::new(Vec::new()),
        })
    }

    pub fn add_child(self: &Arc<Self>, child_name: String) -> Arc<Child> {
        let child = Arc::new(Child {
            name: child_name,
            parent: Arc::downgrade(self), // Create weak reference
        });

        // Store child in parent
        let mut children = self.children.lock();
        children.push(Arc::clone(&child));

        child
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Child {
    pub fn parent_name(&self) -> Option<String> {
        // Upgrade weak to strong (returns None if parent dropped)
        self.parent.upgrade().map(|p| p.name.clone())
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

fn example_weak_references() {
    println!("\n=== Example 5: Weak References ===");

    let parent = Parent::new("Parent".to_string());
    println!("Created parent: {}", parent.name());

    let child1 = parent.add_child("Child 1".to_string());
    let child2 = parent.add_child("Child 2".to_string());

    println!("Added children: {}, {}", child1.name(), child2.name());

    // Children can access parent
    println!(
        "{}'s parent: {:?}",
        child1.name(),
        child1.parent_name()
    );
    println!(
        "{}'s parent: {:?}",
        child2.name(),
        child2.parent_name()
    );

    // Drop parent
    drop(parent);
    println!("Dropped parent");

    // Children's weak reference now returns None
    println!(
        "{}'s parent after drop: {:?}",
        child1.name(),
        child1.parent_name()
    );

    // No memory leak: children will be dropped when they go out of scope
}

// =============================================================================
// Example 6: Spawn Blocking for CPU-Bound Operations
// =============================================================================

#[tokio::main]
async fn example_spawn_blocking() {
    println!("\n=== Example 6: Spawn Blocking ===");

    // Simulate CPU-bound operation (parsing large file)
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    let result = tokio::task::spawn_blocking(move || {
        println!("Running CPU-bound operation in thread pool...");

        // Simulate expensive computation
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Process data
        let sum: i32 = data.iter().sum();
        println!("Computed sum: {}", sum);

        sum
    })
    .await
    .expect("spawn_blocking failed");

    println!("Result from blocking task: {}", result);
}

// =============================================================================
// Example 7: Input Validation for Buffer Allocation
// =============================================================================

const MAX_BUFFER_SIZE: usize = 10_000_000; // 10 MB

fn allocate_validated_buffer(size: usize) -> Result<Vec<u8>, String> {
    // Validate size before allocation
    if size == 0 {
        return Err("Buffer size must be > 0".to_string());
    }

    if size > MAX_BUFFER_SIZE {
        return Err(format!(
            "Buffer size {} exceeds maximum {}",
            size, MAX_BUFFER_SIZE
        ));
    }

    // Safe to allocate
    Ok(vec![0u8; size])
}

fn example_validated_allocation() {
    println!("\n=== Example 7: Validated Buffer Allocation ===");

    // Valid allocation
    match allocate_validated_buffer(1024) {
        Ok(buf) => println!("Allocated buffer of size: {}", buf.len()),
        Err(e) => println!("Error: {}", e),
    }

    // Invalid: Too large
    match allocate_validated_buffer(100_000_000) {
        Ok(buf) => println!("Allocated buffer of size: {}", buf.len()),
        Err(e) => println!("Error: {}", e),
    }

    // Invalid: Zero size
    match allocate_validated_buffer(0) {
        Ok(buf) => println!("Allocated buffer of size: {}", buf.len()),
        Err(e) => println!("Error: {}", e),
    }
}

// =============================================================================
// Example 8: Drop Implementation Without Panics
// =============================================================================

pub struct ResourceHandle {
    id: u64,
    released: bool,
}

impl ResourceHandle {
    pub fn new(id: u64) -> Self {
        println!("Acquired resource: {}", id);
        Self { id, released: false }
    }

    pub fn release(&mut self) -> Result<(), String> {
        if self.released {
            return Err("Resource already released".to_string());
        }

        println!("Releasing resource: {}", self.id);
        self.released = true;
        Ok(())
    }
}

impl Drop for ResourceHandle {
    fn drop(&mut self) {
        if !self.released {
            println!("Drop: Auto-releasing resource: {}", self.id);

            // ✅ Good: Handle errors without panicking
            if let Err(e) = self.release() {
                eprintln!("Warning: Failed to release resource {}: {}", self.id, e);
                // Log error but don't panic
            }
        } else {
            println!("Drop: Resource {} already released", self.id);
        }
    }
}

fn example_drop_without_panic() {
    println!("\n=== Example 8: Drop Without Panic ===");

    {
        let mut handle = ResourceHandle::new(1);
        handle.release().unwrap();
        // Drop called, but resource already released
    }

    {
        let _handle = ResourceHandle::new(2);
        // Drop called, auto-releases resource
    }
}

// =============================================================================
// Example 9: String Interning
// =============================================================================

pub struct StringInterner {
    strings: HashMap<String, Arc<str>>,
}

impl StringInterner {
    pub fn new() -> Self {
        Self {
            strings: HashMap::new(),
        }
    }

    pub fn intern(&mut self, s: &str) -> Arc<str> {
        // Return existing Arc if already interned
        if let Some(arc) = self.strings.get(s) {
            return Arc::clone(arc);
        }

        // Create new Arc and cache it
        let arc: Arc<str> = Arc::from(s);
        self.strings.insert(s.to_string(), Arc::clone(&arc));
        arc
    }

    pub fn stats(&self) -> usize {
        self.strings.len()
    }
}

fn example_string_interning() {
    println!("\n=== Example 9: String Interning ===");

    let mut interner = StringInterner::new();

    // Intern some strings
    let s1 = interner.intern("hello");
    let s2 = interner.intern("world");
    let s3 = interner.intern("hello"); // Same as s1

    println!(
        "Interned 3 strings, unique count: {}",
        interner.stats()
    );

    // s1 and s3 point to the same allocation
    println!(
        "s1 and s3 are same allocation: {}",
        Arc::ptr_eq(&s1, &s3)
    );
    println!(
        "s1 and s2 are same allocation: {}",
        Arc::ptr_eq(&s1, &s2)
    );

    println!("s1: {}", s1);
    println!("s2: {}", s2);
    println!("s3: {}", s3);
}

// =============================================================================
// Example 10: Atomic Operations for Lock-Free Counters
// =============================================================================

pub struct RequestMetrics {
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
}

impl RequestMetrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
        })
    }

    pub fn record_success(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.successful_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_failure(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn stats(&self) -> (u64, u64, u64) {
        (
            self.total_requests.load(Ordering::Relaxed),
            self.successful_requests.load(Ordering::Relaxed),
            self.failed_requests.load(Ordering::Relaxed),
        )
    }
}

fn example_atomic_counters() {
    println!("\n=== Example 10: Atomic Counters ===");

    let metrics = RequestMetrics::new();

    // Simulate concurrent requests
    let mut handles = vec![];

    for i in 0..10 {
        let metrics_clone = Arc::clone(&metrics);
        let handle = std::thread::spawn(move || {
            if i % 3 == 0 {
                metrics_clone.record_failure();
            } else {
                metrics_clone.record_success();
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let (total, success, failure) = metrics.stats();
    println!("Total: {}, Success: {}, Failure: {}", total, success, failure);
}

// =============================================================================
// Main Function
// =============================================================================

fn main() {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║      Memory Safety Patterns for Rust MCP Servers              ║");
    println!("║      Based on ggen-mcp Analysis (2026-01-20)                  ║");
    println!("╚════════════════════════════════════════════════════════════════╝");

    // Run all examples
    example_raii_guard();
    example_raii_guard_disarm();
    example_shared_cache();
    example_string_capacity();
    example_bounded_cache();
    example_weak_references();
    example_validated_allocation();
    example_drop_without_panic();
    example_string_interning();
    example_atomic_counters();

    // Async example
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(example_spawn_blocking());

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║      All examples completed successfully!                     ║");
    println!("║      No unsafe code, no memory leaks, no panics.              ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temp_file_guard() {
        let temp_path = PathBuf::from("/tmp/test.txt");
        let guard = TempFileGuard::new(temp_path.clone());
        assert_eq!(guard.path(), &temp_path);
        // Guard dropped, cleanup happens
    }

    #[test]
    fn test_shared_cache() {
        let cache = SharedCache::new();
        cache.insert("key".to_string(), "value".to_string());

        let entry = cache.get("key").expect("Entry should exist");
        assert_eq!(entry.data, "value");

        let (hits, misses) = cache.stats();
        assert_eq!(hits, 1);
        assert_eq!(misses, 0);
    }

    #[test]
    fn test_format_cell_address() {
        assert_eq!(format_cell_address(1, 1), "A1");
        assert_eq!(format_cell_address(1, 26), "Z1");
        assert_eq!(format_cell_address(1, 27), "AA1");
        assert_eq!(format_cell_address(100, 100), "CV100");
    }

    #[test]
    fn test_bounded_cache() {
        let mut cache = LruCache::new(2);

        cache.insert("a", 1);
        cache.insert("b", 2);

        // At capacity, next insert should evict "a"
        let evicted = cache.insert("c", 3);
        assert!(evicted.is_some());
        assert_eq!(evicted.unwrap().0, "a");

        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_validated_allocation() {
        // Valid
        assert!(allocate_validated_buffer(1024).is_ok());

        // Too large
        assert!(allocate_validated_buffer(100_000_000).is_err());

        // Zero
        assert!(allocate_validated_buffer(0).is_err());
    }

    #[test]
    fn test_string_interning() {
        let mut interner = StringInterner::new();

        let s1 = interner.intern("test");
        let s2 = interner.intern("test");

        assert!(Arc::ptr_eq(&s1, &s2));
        assert_eq!(interner.stats(), 1); // Only one unique string
    }

    #[test]
    fn test_atomic_counters() {
        let metrics = RequestMetrics::new();

        metrics.record_success();
        metrics.record_success();
        metrics.record_failure();

        let (total, success, failure) = metrics.stats();
        assert_eq!(total, 3);
        assert_eq!(success, 2);
        assert_eq!(failure, 1);
    }
}
