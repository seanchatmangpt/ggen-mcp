# Rust MCP Server Memory Safety Guide

**Project:** ggen-mcp (Spreadsheet MCP Server)
**Purpose:** Comprehensive guide to memory safety and ownership patterns
**Audience:** Rust MCP server developers
**Last Updated:** 2026-01-20

## Table of Contents

1. [Overview](#overview)
2. [Ownership Patterns](#ownership-patterns)
3. [Safe Unsafe Code](#safe-unsafe-code)
4. [Buffer Management](#buffer-management)
5. [String Handling](#string-handling)
6. [Reference Counting](#reference-counting)
7. [Memory Leak Prevention](#memory-leak-prevention)
8. [FFI Safety](#ffi-safety)
9. [TPS Principles](#tps-principles)
10. [Best Practices Summary](#best-practices-summary)

---

## Overview

The ggen-mcp server demonstrates **zero-unsafe** Rust architecture for MCP servers. This guide documents patterns that achieve memory safety without sacrificing performance.

### Memory Safety Audit Results

**Analysis Date:** 2026-01-20

- **Unsafe Blocks:** 0 (100% safe code)
- **Memory Leaks Detected:** 0
- **Reference Cycles:** 0 (potential improvement areas documented)
- **Buffer Overflows:** 0 (prevented by bounds checking)
- **Use-After-Free:** 0 (prevented by ownership system)

### Key Dependencies for Memory Safety

```toml
# Concurrency primitives (faster than std)
parking_lot = "0.12"           # RwLock, Mutex with better performance

# Bounded caching
lru = "0.12"                   # LRU cache with automatic eviction

# Small-vector optimization
smallvec = "1.13"              # Stack-allocated vectors for small sizes

# Fast hashing
ahash = "0.8"                  # DDoS-resistant hash function

# Async runtime
tokio = { version = "1.37", features = ["macros", "rt-multi-thread", "sync"] }
```

---

## Ownership Patterns

### Pattern 1: Ownership Transfer in MCP Handlers

MCP handlers typically receive owned values and return owned results. This prevents accidental sharing and makes lifetimes explicit.

```rust
// ✅ Good: Clear ownership transfer
pub async fn open_workbook(
    &self,
    workbook_id: WorkbookId,  // Owned, not borrowed
) -> Result<WorkbookContext> {
    // Load workbook - ownership transferred to caller
    let workbook = self.load_workbook(&workbook_id).await?;
    Ok(workbook)
}

// ❌ Avoid: Returning borrowed data from async contexts
pub async fn get_workbook_ref(&self) -> Result<&WorkbookContext> {
    // This creates lifetime complexity in async code
}
```

### Pattern 2: Arc for Shared State

Use `Arc<T>` for shared, immutable data across async tasks and threads.

```rust
use std::sync::Arc;
use parking_lot::RwLock;

pub struct AppState {
    config: Arc<ServerConfig>,  // Shared config, cheap to clone
    cache: RwLock<LruCache<WorkbookId, Arc<WorkbookContext>>>,  // Shared cache entries
}

impl AppState {
    pub fn config(&self) -> Arc<ServerConfig> {
        self.config.clone()  // Arc::clone is cheap (atomic increment)
    }

    pub async fn open_workbook(&self, id: &WorkbookId) -> Result<Arc<WorkbookContext>> {
        // Return Arc for shared access without copying entire workbook
        let workbook = self.load_workbook(id).await?;
        Ok(Arc::new(workbook))
    }
}
```

**Key Insight:** `Arc::clone()` only increments a reference counter (atomic operation). It's much cheaper than cloning the underlying data.

### Pattern 3: Interior Mutability with RwLock

Use `RwLock<T>` for shared mutable state with concurrent reads.

```rust
use parking_lot::RwLock;
use std::collections::HashMap;

pub struct WorkbookContext {
    // Allow multiple concurrent readers, single writer
    sheet_cache: RwLock<HashMap<String, Arc<SheetCacheEntry>>>,
}

impl WorkbookContext {
    pub fn get_sheet(&self, name: &str) -> Option<Arc<SheetCacheEntry>> {
        let cache = self.sheet_cache.read();  // Read lock (shared)
        cache.get(name).cloned()  // Clone Arc, not data
    }

    pub fn cache_sheet(&self, name: String, entry: Arc<SheetCacheEntry>) {
        let mut cache = self.sheet_cache.write();  // Write lock (exclusive)
        cache.insert(name, entry);
    }  // Lock automatically released (RAII)
}
```

**Why parking_lot?**
- No poisoning (simpler error handling)
- Smaller memory footprint
- Better performance under contention
- Works seamlessly with Tokio

### Pattern 4: Borrowing vs Cloning Decisions

**Decision Matrix:**

| Scenario | Use | Reason |
|----------|-----|--------|
| Function argument (sync, short-lived) | `&T` or `&mut T` | Zero cost, no allocation |
| Function argument (async, complex lifetime) | `Arc<T>` | Avoids lifetime parameters in futures |
| Return value from cache | `Arc<T>` | Allows shared ownership |
| Return computed value | `T` (owned) | Caller owns result |
| Small copyable types (`u32`, `bool`) | `T` (by value) | Copy is cheaper than reference |
| Large types in hot paths | `&T` | Avoid unnecessary copies |

```rust
// ✅ Good: Borrow for synchronous, short-lived access
pub fn validate_cell_address(address: &str) -> Result<(u32, u32)> {
    // address lifetime ends with function, no async
    parse_address(address)
}

// ✅ Good: Clone Arc for async, shared access
pub async fn process_workbook(workbook: Arc<WorkbookContext>) -> Result<Report> {
    // workbook.clone() creates new Arc, not new data
    let wb = workbook.clone();
    tokio::spawn(async move {
        analyze_workbook(wb).await
    });
    // Original workbook still valid
    Ok(generate_report(&workbook))
}

// ✅ Good: Return owned for computed results
pub fn compute_statistics(data: &[f64]) -> Statistics {
    Statistics {
        mean: calculate_mean(data),
        median: calculate_median(data),
        // ... owned result
    }
}
```

### Pattern 5: Ownership and Async

Async functions require `'static` lifetimes for spawned tasks. Use owned values or `Arc`.

```rust
// ✅ Good: Arc for shared state in async
pub async fn spawn_background_task(
    state: Arc<AppState>,
    workbook_id: WorkbookId,  // Owned, not borrowed
) {
    tokio::spawn(async move {
        // state and workbook_id are moved into task
        process_workbook(&state, &workbook_id).await
    });
}

// ❌ Bad: Borrowed data doesn't work with spawn
pub async fn spawn_background_task_bad<'a>(
    state: &'a AppState,  // Lifetime parameter
    workbook_id: &'a WorkbookId,
) {
    // Won't compile: task might outlive borrowed data
    tokio::spawn(async move {
        process_workbook(state, workbook_id).await
    });
}
```

### Pattern 6: Interior Mutability Patterns

**When to use each:**

| Type | Thread-Safe | Use Case | Cost |
|------|-------------|----------|------|
| `Mutex<T>` | Yes | Exclusive access | Lock overhead |
| `RwLock<T>` | Yes | Multiple readers, single writer | Lock overhead |
| `Arc<RwLock<T>>` | Yes | Shared mutable state | Arc + Lock overhead |
| `Cell<T>` | No | Single-threaded mutation | Zero overhead |
| `RefCell<T>` | No | Single-threaded dynamic borrowing | Runtime check |
| `AtomicU64`, `AtomicBool` | Yes | Simple counters/flags | Atomic operation |

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use parking_lot::{RwLock, Mutex};

pub struct AppState {
    // ✅ Atomic for simple counters (lock-free)
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,

    // ✅ RwLock for read-heavy access
    cache: RwLock<LruCache<WorkbookId, Arc<WorkbookContext>>>,

    // ✅ Mutex for exclusive access
    active_tasks: Mutex<HashMap<String, TaskHandle>>,
}

impl AppState {
    pub fn record_cache_hit(&self) {
        // Atomic increment (lock-free, fast)
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            hits: self.cache_hits.load(Ordering::Relaxed),
            misses: self.cache_misses.load(Ordering::Relaxed),
            size: self.cache.read().len(),
        }
    }
}
```

**Ordering Guidelines:**

- `Ordering::Relaxed`: For statistics, counters (no ordering guarantees)
- `Ordering::SeqCst`: For synchronization, critical correctness (strongest guarantees)
- See `std::sync::atomic::Ordering` for full details

---

## Safe Unsafe Code

### Zero-Unsafe Architecture

**ggen-mcp uses ZERO unsafe code.** This is achieved through:

1. **Safe abstractions:** Use libraries that encapsulate unsafe code safely
2. **Performance without unsafe:** `parking_lot`, `smallvec`, `ahash`
3. **Validation instead of trust:** Bounds checking, input validation

### When Unsafe is Justified (General Guidelines)

If you absolutely need unsafe code in an MCP server:

```rust
/// ✅ Good: Well-documented invariants
///
/// # Safety
///
/// This function is safe IFF:
/// 1. `ptr` points to valid, initialized `T`
/// 2. `ptr` is aligned for `T`
/// 3. No other references to `*ptr` exist during this call
/// 4. Caller has exclusive access to `*ptr`
///
/// # Rationale
///
/// Used in hot path for zero-copy deserialization from trusted source.
/// Benchmarked: 3x faster than safe alternative.
/// Called only from `deserialize_trusted()` which validates invariants.
unsafe fn read_unaligned<T>(ptr: *const T) -> T {
    ptr.read_unaligned()
}

// ❌ Bad: No safety documentation
unsafe fn do_something(ptr: *const u8) -> u8 {
    *ptr  // What are the invariants? When is this safe?
}
```

### FFI Safety Considerations

ggen-mcp interacts with LibreOffice via process spawning, NOT FFI. This avoids entire classes of unsafe code:

```rust
// ✅ Safe: Process-based interaction (no FFI)
pub async fn recalculate_workbook(path: &Path) -> Result<()> {
    let output = tokio::process::Command::new("soffice")
        .arg("--headless")
        .arg("--convert-to").arg("xlsx")
        .arg(path)
        .output()
        .await?;

    if !output.status.success() {
        bail!("LibreOffice recalculation failed");
    }
    Ok(())
}
```

If you MUST use FFI:

1. **Isolate unsafe blocks**
2. **Document invariants exhaustively**
3. **Use repr(C) for structs crossing FFI boundary**
4. **Validate all data from C**
5. **Use CString for C strings, never raw pointers**

```rust
use std::ffi::CString;
use std::os::raw::c_char;

// External C function
extern "C" {
    fn c_process_string(s: *const c_char) -> i32;
}

// ✅ Safe wrapper
pub fn process_string(s: &str) -> Result<i32> {
    // Validate input
    if s.len() > 1024 {
        bail!("String too long for C function");
    }

    // Convert to CString (handles NUL termination)
    let c_str = CString::new(s)?;

    // SAFETY: c_str is valid, NUL-terminated, and lives for call duration
    let result = unsafe { c_process_string(c_str.as_ptr()) };

    if result < 0 {
        bail!("C function returned error: {}", result);
    }
    Ok(result)
}
```

---

## Buffer Management

### Pattern 1: Fixed-Size Buffers

Use fixed-size arrays for bounded data.

```rust
const MAX_CELL_ADDRESS_LEN: usize = 32;  // e.g., "Sheet1!XFD1048576"

pub struct CellAddress {
    // Stack-allocated, no heap allocation
    buffer: [u8; MAX_CELL_ADDRESS_LEN],
    len: usize,
}

impl CellAddress {
    pub fn new(address: &str) -> Result<Self> {
        if address.len() > MAX_CELL_ADDRESS_LEN {
            bail!("Address too long: {}", address.len());
        }

        let mut buffer = [0u8; MAX_CELL_ADDRESS_LEN];
        buffer[..address.len()].copy_from_slice(address.as_bytes());

        Ok(Self {
            buffer,
            len: address.len(),
        })
    }

    pub fn as_str(&self) -> &str {
        // SAFETY: buffer[..len] contains valid UTF-8 (validated in new())
        std::str::from_utf8(&self.buffer[..self.len]).unwrap()
    }
}
```

### Pattern 2: Growable Buffers

Use `Vec<T>` and `String` with capacity hints.

```rust
// ✅ Good: Pre-allocate capacity
pub fn escape_sparql_string(input: &str) -> String {
    // Worst case: every char needs escaping (2x original)
    let mut escaped = String::with_capacity(input.len() * 2);

    for ch in input.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\'' => escaped.push_str("\\'"),
            _ => escaped.push(ch),
        }
    }

    escaped  // No reallocation if capacity was sufficient
}

// ❌ Bad: No capacity hint (many reallocations)
pub fn escape_sparql_string_bad(input: &str) -> String {
    let mut escaped = String::new();  // Starts with capacity 0
    // Will reallocate multiple times as it grows
    for ch in input.chars() {
        // ...
    }
    escaped
}
```

### Pattern 3: SmallVec for Stack Optimization

Use `SmallVec` for collections that are usually small.

```rust
use smallvec::{SmallVec, smallvec};

// Most cells have 0-4 dependencies
pub type DependencyList = SmallVec<[CellAddress; 4]>;

pub struct FormulaCell {
    formula: String,
    // Stored on stack if <= 4 dependencies, heap otherwise
    dependencies: DependencyList,
}

impl FormulaCell {
    pub fn new(formula: String) -> Self {
        Self {
            formula,
            dependencies: smallvec![],  // Stack-allocated
        }
    }

    pub fn add_dependency(&mut self, addr: CellAddress) {
        self.dependencies.push(addr);  // Spills to heap if > 4
    }
}
```

### Pattern 4: Buffer Pooling (Advanced)

For high-throughput scenarios, reuse buffers.

```rust
use std::sync::Arc;
use parking_lot::Mutex;

pub struct BufferPool {
    buffers: Mutex<Vec<Vec<u8>>>,
    buffer_size: usize,
}

impl BufferPool {
    pub fn new(buffer_size: usize, initial_count: usize) -> Arc<Self> {
        let buffers = (0..initial_count)
            .map(|_| Vec::with_capacity(buffer_size))
            .collect();

        Arc::new(Self {
            buffers: Mutex::new(buffers),
            buffer_size,
        })
    }

    pub fn acquire(&self) -> Vec<u8> {
        self.buffers.lock().pop()
            .unwrap_or_else(|| Vec::with_capacity(self.buffer_size))
    }

    pub fn release(&self, mut buffer: Vec<u8>) {
        buffer.clear();  // Clear but keep capacity
        if buffer.capacity() == self.buffer_size {
            let mut buffers = self.buffers.lock();
            if buffers.len() < 100 {  // Don't pool too many
                buffers.push(buffer);
            }
        }
    }
}
```

### Pattern 5: Bounded Buffers

Always validate buffer sizes against maximum limits.

```rust
// From ggen-mcp/src/validation/bounds.rs

pub const EXCEL_MAX_ROWS: u32 = 1_048_576;
pub const EXCEL_MAX_COLUMNS: u32 = 16_384;

pub fn validate_range_size(rows: u32, cols: u32) -> Result<()> {
    validate_row_1based(rows, "range")?;
    validate_column_1based(cols, "range")?;

    // Prevent allocation of excessively large buffers
    let total_cells = rows as u64 * cols as u64;
    const MAX_CELLS: u64 = 5_000_000;  // Reasonable limit

    if total_cells > MAX_CELLS {
        bail!(
            "Range too large: {}x{} = {} cells exceeds limit of {}",
            rows, cols, total_cells, MAX_CELLS
        );
    }

    Ok(())
}

pub fn allocate_cell_buffer(rows: u32, cols: u32) -> Result<Vec<Cell>> {
    validate_range_size(rows, cols)?;

    let total_cells = rows as usize * cols as usize;
    // Safe: validated above
    Ok(Vec::with_capacity(total_cells))
}
```

---

## String Handling

### Pattern 1: String vs &str Decisions

| Use Case | Type | Reason |
|----------|------|--------|
| Function parameter (read-only) | `&str` | Accepts `String`, `&str`, `&String` |
| Function return (computed) | `String` | Caller owns result |
| Struct field (owned) | `String` | Struct owns data |
| Struct field (borrowed, short-lived) | `&'a str` | Avoids allocation (rare) |
| Dictionary key | `String` | HashMap needs owned keys |
| Cache entry | `Arc<str>` | Shared, immutable string |

```rust
// ✅ Good: &str parameter accepts any string type
pub fn validate_sheet_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Sheet name cannot be empty");
    }
    if name.len() > 31 {
        bail!("Sheet name too long: {}", name.len());
    }
    Ok(())
}

// ✅ Good: String return for computed result
pub fn format_cell_value(value: &CellValue) -> String {
    match value {
        CellValue::Number(n) => n.to_string(),
        CellValue::Text(s) => s.clone(),
        CellValue::Bool(b) => b.to_string(),
        CellValue::Empty => String::from(""),
    }
}

// ✅ Good: Arc<str> for shared, immutable strings
use std::sync::Arc;

pub struct StringCache {
    cache: HashMap<String, Arc<str>>,
}

impl StringCache {
    pub fn intern(&mut self, s: &str) -> Arc<str> {
        if let Some(cached) = self.cache.get(s) {
            return cached.clone();  // Cheap Arc clone
        }

        let arc: Arc<str> = Arc::from(s);
        self.cache.insert(s.to_string(), arc.clone());
        arc
    }
}
```

### Pattern 2: OsString for Paths

Always use `Path`, `PathBuf`, `OsStr`, `OsString` for filesystem paths.

```rust
use std::path::{Path, PathBuf};
use std::ffi::OsStr;

// ✅ Good: Path for file paths (handles non-UTF8 paths)
pub fn load_workbook(path: &Path) -> Result<Workbook> {
    if !path.exists() {
        bail!("File not found: {}", path.display());
    }

    // Use display() for error messages only
    let file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open: {}", path.display()))?;

    Ok(parse_workbook(file)?)
}

// ❌ Bad: String for paths (fails on non-UTF8 paths)
pub fn load_workbook_bad(path: &str) -> Result<Workbook> {
    let path = Path::new(path);  // Extra conversion
    // ...
}
```

### Pattern 3: String Capacity Hints

Pre-allocate when final size is known or predictable.

```rust
// ✅ Good: Capacity hint for known size
pub fn format_cell_address(row: u32, col: u32) -> String {
    // "XFD1048576" is max 10 chars
    let mut addr = String::with_capacity(10);

    // Column letter (e.g., "A", "AB", "XFD")
    let mut col_num = col;
    while col_num > 0 {
        col_num -= 1;
        let letter = (b'A' + (col_num % 26) as u8) as char;
        addr.insert(0, letter);
        col_num /= 26;
    }

    // Row number
    addr.push_str(&row.to_string());

    addr  // No reallocation
}

// ❌ Bad: No capacity hint
pub fn format_cell_address_bad(row: u32, col: u32) -> String {
    let mut addr = String::new();  // Starts at capacity 0
    // Will reallocate as it grows
    // ...
}
```

### Pattern 4: String Interning

For repeated strings, use string interning to save memory.

```rust
use std::sync::Arc;
use std::collections::HashMap;

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
            return arc.clone();
        }

        // Create new Arc and cache it
        let arc: Arc<str> = Arc::from(s);
        self.strings.insert(s.to_string(), arc.clone());
        arc
    }

    pub fn get(&self, s: &str) -> Option<Arc<str>> {
        self.strings.get(s).cloned()
    }
}

// Usage example: intern sheet names (usually repeated)
pub struct WorkbookContext {
    interner: RwLock<StringInterner>,
}

impl WorkbookContext {
    pub fn intern_sheet_name(&self, name: &str) -> Arc<str> {
        self.interner.write().intern(name)
    }
}
```

### Pattern 5: UTF-8 Validation

Rust strings are always valid UTF-8. When working with bytes:

```rust
// ✅ Good: Validate UTF-8 explicitly
pub fn parse_cell_content(bytes: &[u8]) -> Result<String> {
    // Validate UTF-8
    let s = std::str::from_utf8(bytes)
        .context("Invalid UTF-8 in cell content")?;

    Ok(s.to_string())
}

// ✅ Good: Lossy conversion when exact UTF-8 not required
pub fn parse_cell_content_lossy(bytes: &[u8]) -> String {
    // Replace invalid UTF-8 with �
    String::from_utf8_lossy(bytes).to_string()
}

// ⚠️ Unsafe: Only use if you KNOW bytes are valid UTF-8
pub fn parse_cell_content_unchecked(bytes: &[u8]) -> String {
    // SAFETY: Caller MUST guarantee bytes are valid UTF-8
    // Used when parsing Excel XML that we've already validated
    unsafe {
        String::from_utf8_unchecked(bytes.to_vec())
    }
}
```

---

## Reference Counting

### Pattern 1: Arc Usage Patterns

`Arc<T>` provides shared ownership with atomic reference counting.

```rust
use std::sync::Arc;
use parking_lot::RwLock;

pub struct AppState {
    config: Arc<ServerConfig>,  // Shared, read-only config
    cache: Arc<RwLock<Cache>>,  // Shared, mutable cache
}

impl AppState {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config: Arc::new(config),
            cache: Arc::new(RwLock::new(Cache::new())),
        }
    }

    // ✅ Return Arc for shared access
    pub fn config(&self) -> Arc<ServerConfig> {
        Arc::clone(&self.config)  // Explicit clone
        // or: self.config.clone()  // Also works
    }

    // ✅ Pass Arc by value for async tasks
    pub async fn spawn_task(state: Arc<AppState>) {
        tokio::spawn(async move {
            // state is moved, Arc keeps it alive
            process_data(&state).await;
        });
    }
}
```

**Arc Clone Cost:** Atomic increment only (~5-10 CPU cycles). Much cheaper than copying data.

### Pattern 2: When to Use Weak References

Use `Weak<T>` to break reference cycles.

```rust
use std::sync::{Arc, Weak};
use parking_lot::RwLock;

// Example: Parent-child relationship with cycle breaking

pub struct Parent {
    name: String,
    children: Vec<Arc<Child>>,
}

pub struct Child {
    name: String,
    parent: Weak<Parent>,  // Weak to break cycle
}

impl Parent {
    pub fn new(name: String) -> Arc<Self> {
        Arc::new(Self {
            name,
            children: Vec::new(),
        })
    }

    pub fn add_child(self: &Arc<Self>, name: String) -> Arc<Child> {
        let child = Arc::new(Child {
            name,
            parent: Arc::downgrade(self),  // Create weak reference
        });

        // Store in parent (would create cycle if child had Arc<Parent>)
        // Note: This is simplified; in real code, use interior mutability
        // self.children.push(child.clone());

        child
    }
}

impl Child {
    pub fn parent_name(&self) -> Option<String> {
        // Upgrade weak to strong (returns None if parent dropped)
        self.parent.upgrade().map(|p| p.name.clone())
    }
}
```

**ggen-mcp Status:** Currently uses NO `Weak` references. This is safe because:
- No parent-child cycles in data structures
- All references are hierarchical (state → cache → entries)
- Entries don't reference back to cache or state

**Potential Improvement Area:** If adding bidirectional navigation (e.g., cell → formula → cell), use `Weak` to prevent cycles.

### Pattern 3: Rc vs Arc

| Type | Thread-Safe | Cost | Use Case |
|------|-------------|------|----------|
| `Rc<T>` | No | Faster (non-atomic) | Single-threaded reference counting |
| `Arc<T>` | Yes | Slower (atomic) | Multi-threaded reference counting |

```rust
use std::rc::Rc;
use std::sync::Arc;

// ✅ Single-threaded: Use Rc (faster)
fn single_threaded_example() {
    let data = Rc::new(vec![1, 2, 3]);
    let clone1 = Rc::clone(&data);
    let clone2 = Rc::clone(&data);
    // Non-atomic increment, faster
}

// ✅ Multi-threaded: Use Arc (thread-safe)
fn multi_threaded_example() {
    let data = Arc::new(vec![1, 2, 3]);

    let clone = Arc::clone(&data);
    std::thread::spawn(move || {
        // clone is moved into thread
        println!("{:?}", clone);
    });

    // Original data still valid
    println!("{:?}", data);
}
```

**ggen-mcp:** Always uses `Arc` because MCP servers are multi-threaded (async runtime).

### Pattern 4: Reference Count Tracking

Monitor reference counts for debugging.

```rust
use std::sync::Arc;

pub fn debug_arc_count<T>(arc: &Arc<T>, name: &str) {
    let strong_count = Arc::strong_count(arc);
    let weak_count = Arc::weak_count(arc);

    tracing::debug!(
        name = name,
        strong_count = strong_count,
        weak_count = weak_count,
        "Arc reference counts"
    );
}

// Usage
pub fn cache_workbook(cache: &mut Cache, wb: Workbook) {
    let arc = Arc::new(wb);
    debug_arc_count(&arc, "workbook");  // strong_count = 1

    cache.insert(arc.clone());
    debug_arc_count(&arc, "workbook");  // strong_count = 2 (cache + local)
}
```

### Pattern 5: Avoiding Reference Cycles

**Common cycle patterns to avoid:**

```rust
// ❌ Bad: Reference cycle (memory leak)
pub struct Node {
    next: Option<Arc<Node>>,
    prev: Option<Arc<Node>>,  // Creates cycle!
}

// ✅ Good: Break cycle with Weak
pub struct Node {
    next: Option<Arc<Node>>,
    prev: Option<Weak<Node>>,  // Weak breaks cycle
}

// ❌ Bad: Event handler cycle
pub struct EventEmitter {
    handlers: Vec<Arc<dyn Fn() -> ()>>,
}

pub struct Handler {
    emitter: Arc<EventEmitter>,  // Cycle if emitter stores this handler
}

// ✅ Good: Use Weak or callback IDs
pub struct EventEmitter {
    handlers: HashMap<usize, Arc<dyn Fn() -> ()>>,
}

pub struct Handler {
    emitter: Weak<EventEmitter>,  // Weak breaks cycle
    handler_id: usize,
}
```

---

## Memory Leak Prevention

### Pattern 1: RAII Guards

Use RAII (Resource Acquisition Is Initialization) for automatic cleanup.

```rust
// From ggen-mcp: TempFileGuard
pub struct TempFileGuard {
    path: PathBuf,
    cleanup_on_drop: bool,
}

impl TempFileGuard {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            cleanup_on_drop: true,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Disarm guard - file will not be deleted
    pub fn disarm(mut self) -> PathBuf {
        self.cleanup_on_drop = false;
        self.path.clone()
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.cleanup_on_drop {
            let _ = std::fs::remove_file(&self.path);
            tracing::debug!(path = ?self.path, "cleaned up temp file");
        }
    }
}

// Usage: Automatic cleanup even on early return or panic
pub fn process_with_temp_file() -> Result<()> {
    let temp_path = PathBuf::from("/tmp/temp.xlsx");
    let _guard = TempFileGuard::new(temp_path.clone());

    // Do work with temp file
    process_file(&temp_path)?;

    // File automatically deleted when _guard drops
    // Even if process_file() returns early!
    Ok(())
}
```

### Pattern 2: Bounded Caches

Use LRU cache to prevent unbounded memory growth.

```rust
use lru::LruCache;
use std::num::NonZeroUsize;

pub struct WorkbookCache {
    cache: LruCache<WorkbookId, Arc<Workbook>>,
}

impl WorkbookCache {
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).expect("capacity must be > 0");
        Self {
            cache: LruCache::new(cap),
        }
    }

    pub fn get(&mut self, id: &WorkbookId) -> Option<Arc<Workbook>> {
        self.cache.get(id).cloned()
    }

    pub fn insert(&mut self, id: WorkbookId, wb: Arc<Workbook>) {
        // Automatically evicts least recently used if at capacity
        if let Some((evicted_id, _evicted_wb)) = self.cache.push(id, wb) {
            tracing::debug!(evicted_id = ?evicted_id, "evicted from cache");
        }
    }
}
```

### Pattern 3: Leak Detection Tools

**Valgrind (for unsafe FFI code):**

```bash
# Run with valgrind to detect memory leaks
valgrind --leak-check=full --show-leak-kinds=all \
    ./target/debug/spreadsheet-mcp

# Look for:
# - "definitely lost" (direct leaks)
# - "indirectly lost" (leaked via other leaks)
# - "still reachable" (not freed but referenced - usually OK)
```

**Miri (Rust interpreter for detecting UB):**

```bash
# Install miri
rustup component add miri

# Run tests under miri
cargo miri test

# Miri detects:
# - Use-after-free
# - Out-of-bounds access
# - Data races
# - Invalid pointer arithmetic
```

**AddressSanitizer (for FFI code):**

```bash
# Compile with AddressSanitizer
RUSTFLAGS="-Z sanitizer=address" cargo build --target x86_64-unknown-linux-gnu

# Run
./target/x86_64-unknown-linux-gnu/debug/spreadsheet-mcp

# Detects:
# - Use-after-free
# - Heap buffer overflow
# - Stack buffer overflow
# - Memory leaks
```

### Pattern 4: Common Leak Patterns

```rust
// ❌ Leak: Reference cycle
use std::sync::Arc;
use parking_lot::Mutex;

pub struct Node {
    value: i32,
    next: Mutex<Option<Arc<Node>>>,
}

// This creates a cycle: A → B → A
let a = Arc::new(Node { value: 1, next: Mutex::new(None) });
let b = Arc::new(Node { value: 2, next: Mutex::new(Some(a.clone())) });
*a.next.lock() = Some(b.clone());
// a and b will never be freed!

// ❌ Leak: Forgetting to release resources
pub fn leak_file_handle() {
    let file = std::fs::File::open("data.txt").unwrap();
    std::mem::forget(file);  // File handle leaked!
}

// ❌ Leak: Long-lived allocation in loop
pub fn leak_in_loop() {
    let mut cache = Vec::new();
    loop {
        let data = vec![0u8; 1024 * 1024];  // 1 MB
        cache.push(data);  // Never removed, grows forever!
    }
}

// ✅ Fix: Use Weak to break cycle
pub struct Node {
    value: i32,
    next: Mutex<Option<Weak<Node>>>,  // Weak breaks cycle
}

// ✅ Fix: Use RAII guard
pub fn with_file_guard() {
    let _file = std::fs::File::open("data.txt").unwrap();
    // File automatically closed when _file drops
}

// ✅ Fix: Use bounded cache
pub fn bounded_loop() {
    let mut cache = LruCache::new(NonZeroUsize::new(100).unwrap());
    loop {
        let data = vec![0u8; 1024 * 1024];
        cache.push(key, data);  // Automatically evicts old entries
    }
}
```

### Pattern 5: Drop Implementation Correctness

Ensure `Drop` implementations don't leak or panic.

```rust
// ✅ Good: Drop implementation that handles errors
impl Drop for ForkContext {
    fn drop(&mut self) {
        tracing::debug!(fork_id = %self.fork_id, "dropping fork context");

        // Clean up work file
        if let Err(e) = std::fs::remove_file(&self.work_path) {
            // Log error but don't panic in Drop
            tracing::warn!(
                path = ?self.work_path,
                error = %e,
                "failed to cleanup work file"
            );
        }

        // Clean up staged changes
        for staged in &self.staged_changes {
            if let Some(path) = &staged.fork_path_snapshot {
                let _ = std::fs::remove_file(path);  // Ignore errors
            }
        }

        // Clean up checkpoint directory
        let checkpoint_dir = self.checkpoint_dir();
        let _ = std::fs::remove_dir_all(&checkpoint_dir);
    }
}

// ❌ Bad: Drop that panics
impl Drop for BadType {
    fn drop(&mut self) {
        std::fs::remove_file(&self.path)
            .expect("failed to remove file");  // PANIC in drop = abort!
    }
}
```

**Drop Guidelines:**
1. Never panic in `Drop` (causes abort if already panicking)
2. Log errors instead of propagating
3. Clean up even on partial failure
4. Keep `Drop` simple and fast

---

## FFI Safety

### Process-Based vs FFI-Based Integration

**ggen-mcp uses process-based LibreOffice interaction (safer):**

```rust
// ✅ Safe: Process-based interaction
use tokio::process::Command;

pub async fn recalculate_workbook(path: &Path) -> Result<RecalcResult> {
    let output = Command::new("soffice")
        .arg("--headless")
        .arg("--calc")
        .arg("--convert-to").arg("xlsx")
        .arg(path)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("LibreOffice failed: {}", stderr);
    }

    Ok(RecalcResult { success: true })
}
```

**Benefits of process-based:**
- No unsafe code required
- Process crashes don't crash MCP server
- Clear ownership boundaries
- Easy to timeout or kill

### FFI Safety Principles (If Needed)

If you must use FFI:

#### 1. C String Handling

```rust
use std::ffi::{CString, CStr};
use std::os::raw::c_char;

extern "C" {
    fn c_process_string(input: *const c_char) -> *mut c_char;
}

// ✅ Safe wrapper
pub fn process_string(input: &str) -> Result<String> {
    // Convert Rust string to C string
    let c_input = CString::new(input)
        .context("String contains NUL byte")?;

    // Call C function
    // SAFETY: c_input is valid, NUL-terminated C string
    let result_ptr = unsafe { c_process_string(c_input.as_ptr()) };

    if result_ptr.is_null() {
        bail!("C function returned NULL");
    }

    // Convert C string back to Rust
    // SAFETY: C function contract guarantees valid, NUL-terminated string
    let result = unsafe {
        let c_str = CStr::from_ptr(result_ptr);
        c_str.to_string_lossy().to_string()
    };

    // Free C-allocated string (depends on C library contract)
    // SAFETY: C function contract says caller must free
    unsafe {
        libc::free(result_ptr as *mut libc::c_void);
    }

    Ok(result)
}
```

#### 2. Memory Ownership Across FFI

```rust
// ✅ Rust owns, C borrows (safe)
pub fn rust_owns_c_borrows(data: &[u8]) {
    extern "C" {
        fn c_process_bytes(data: *const u8, len: usize);
    }

    // SAFETY: data is valid for the duration of this call
    unsafe {
        c_process_bytes(data.as_ptr(), data.len());
    }
    // data remains owned by Rust
}

// ⚠️ C owns, Rust borrows (complex)
pub fn c_owns_rust_borrows() -> &'static str {
    extern "C" {
        fn c_get_static_string() -> *const c_char;
    }

    // SAFETY: C contract guarantees this points to static data
    unsafe {
        let c_str = CStr::from_ptr(c_get_static_string());
        c_str.to_str().unwrap()
    }
}

// ⚠️ Ownership transfer (very complex)
pub fn transfer_ownership_to_c(data: Vec<u8>) {
    extern "C" {
        fn c_takes_ownership(data: *mut u8, len: usize);
    }

    // Transfer ownership to C (C must free)
    let mut data = data;
    let ptr = data.as_mut_ptr();
    let len = data.len();
    std::mem::forget(data);  // Prevent Rust from freeing

    // SAFETY: C function contract says it will free this memory
    unsafe {
        c_takes_ownership(ptr, len);
    }
}
```

#### 3. Callback Safety

```rust
use std::ffi::c_void;

type Callback = extern "C" fn(*const c_void);

extern "C" {
    fn c_register_callback(callback: Callback, user_data: *const c_void);
}

// ✅ Safe: Use raw pointer for user data
pub struct CallbackData {
    count: usize,
}

extern "C" fn my_callback(user_data: *const c_void) {
    // SAFETY: user_data was created by us, still valid
    unsafe {
        let data = &*(user_data as *const CallbackData);
        println!("Callback called, count = {}", data.count);
    }
}

pub fn register_callback_safe() {
    let data = Box::new(CallbackData { count: 0 });
    let data_ptr = Box::into_raw(data);

    // SAFETY: callback and data_ptr are valid
    unsafe {
        c_register_callback(my_callback, data_ptr as *const c_void);
    }

    // Later: unregister and free
    // let data = unsafe { Box::from_raw(data_ptr) };
    // drop(data);
}
```

#### 4. Signal Handling (Unix)

```rust
// ⚠️ Signal handlers must be async-signal-safe
extern "C" fn signal_handler(_sig: libc::c_int) {
    // ✅ OK: Writing to stderr
    let msg = b"Signal received\n";
    unsafe {
        libc::write(2, msg.as_ptr() as *const _, msg.len());
    }

    // ❌ NOT OK: Allocating, locking, or calling most functions
    // println!("Signal");  // NOT async-signal-safe
    // mutex.lock();        // NOT async-signal-safe
}
```

---

## TPS Principles

### Respect for People: Safe Code is Maintainable Code

Memory safety directly relates to TPS's **respect for people** principle:

1. **Safe code respects future maintainers**
   - No hidden invariants in unsafe code
   - Compiler catches most bugs
   - Clear ownership makes code understandable

2. **Memory safety prevents production incidents**
   - No use-after-free crashes in production
   - No buffer overflows leading to security issues
   - Predictable resource usage

3. **Type safety enables confident refactoring**
   - Compiler validates changes
   - Tests remain valid across refactors
   - Less fear of breaking production

### Jidoka: Built-In Quality

Memory safety is **built-in quality** (jidoka):

```rust
// ❌ Manual memory management (error-prone)
// In C:
// char* buffer = malloc(1024);
// // ... use buffer ...
// free(buffer);  // Easy to forget, double-free, or use-after-free

// ✅ Automatic memory management (jidoka)
pub fn process_data() -> Result<()> {
    let buffer = vec![0u8; 1024];
    // ... use buffer ...
    Ok(())
    // buffer automatically freed, impossible to use-after-free
}
```

### Kaizen: Continuous Improvement

**Memory safety improvements for ggen-mcp:**

1. **Add Weak references for future bidirectional navigation**
   - Currently: No cycles, so Arc is fine
   - Future: If adding cell ↔ formula links, use Weak

2. **Add memory profiling in CI**
   ```bash
   # Add to CI pipeline
   cargo bench --bench memory_usage
   ```

3. **Monitor cache hit rates**
   ```rust
   // Already implemented in ggen-mcp
   pub fn cache_stats(&self) -> CacheStats {
       CacheStats {
           hits: self.cache_hits.load(Ordering::Relaxed),
           misses: self.cache_misses.load(Ordering::Relaxed),
       }
   }
   ```

4. **Add memory limits to config**
   ```rust
   pub struct ServerConfig {
       pub cache_capacity: usize,
       pub max_workbook_size_mb: u64,  // ← Add this
       pub max_memory_mb: u64,          // ← Add this
   }
   ```

---

## Best Practices Summary

### Ownership
- ✅ Use `Arc<T>` for shared state across threads
- ✅ Use `&T` for function parameters (accept any string type)
- ✅ Return `T` (owned) for computed results
- ✅ Clone `Arc` freely (cheap atomic increment)
- ❌ Avoid `'static` lifetimes unless truly static

### Interior Mutability
- ✅ Use `RwLock` for read-heavy access
- ✅ Use `Mutex` for exclusive access
- ✅ Use `AtomicU64` for counters
- ✅ Use parking_lot for better performance
- ❌ Avoid `RefCell` in multi-threaded code

### Buffers
- ✅ Pre-allocate with `String::with_capacity()`
- ✅ Use `SmallVec` for usually-small collections
- ✅ Validate buffer sizes against limits
- ✅ Use LRU cache for bounded growth
- ❌ Don't trust user input for allocation sizes

### Strings
- ✅ Use `&str` for parameters
- ✅ Use `String` for owned strings
- ✅ Use `PathBuf`/`Path` for file paths
- ✅ Intern repeated strings
- ❌ Don't use `String` for paths

### Reference Counting
- ✅ Use `Arc` in async/multi-threaded code
- ✅ Use `Weak` to break reference cycles
- ✅ Monitor reference counts for debugging
- ❌ Don't create reference cycles

### Memory Leaks
- ✅ Use RAII guards for cleanup
- ✅ Implement `Drop` without panicking
- ✅ Use bounded caches (LRU)
- ✅ Run miri in CI for leak detection
- ❌ Don't `std::mem::forget()` unless necessary

### FFI
- ✅ Prefer process-based over FFI
- ✅ Document safety invariants exhaustively
- ✅ Use `CString`/`CStr` for C strings
- ✅ Validate all data from C
- ❌ Never trust C code implicitly

---

## References

- [Rust Book: Ownership](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [Rust Nomicon: Advanced](https://doc.rust-lang.org/nomicon/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [parking_lot Documentation](https://docs.rs/parking_lot/)
- [LRU Cache Documentation](https://docs.rs/lru/)
- [ggen-mcp Source Code](https://github.com/PSU3D0/spreadsheet-mcp)

---

## Changelog

- **2026-01-20:** Initial creation based on ggen-mcp audit
- Memory safety audit: 0 unsafe blocks, 0 memory leaks detected
- Documented all ownership patterns found in codebase
- Added TPS principles integration
- Created comprehensive examples

---

**Remember:** Memory safety is not just about correctness—it's about **respect for people**. Safe code is maintainable code. Safe code prevents production incidents. Safe code enables confident refactoring.

Build memory safety into your MCP servers from day one.
