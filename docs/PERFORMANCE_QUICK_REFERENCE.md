# Performance Optimization Quick Reference Card

## 🎯 Top 3 Optimizations (High Impact, Low Effort)

### 1. Replace SHA256 with ahash (10 minutes)
**Expected Impact:** 5-10x speedup for SPARQL cache operations

```rust
// File: src/sparql/cache.rs:127-131
// BEFORE
use sha2::{Digest, Sha256};
pub fn fingerprint(query: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(query.as_bytes());
    format!("{:x}", hasher.finalize())
}

// AFTER
use ahash::AHasher;
use std::hash::{Hash, Hasher};
pub fn fingerprint(query: &str) -> u64 {
    let mut hasher = AHasher::default();
    query.hash(&mut hasher);
    hasher.finish()
}
```

### 2. Add Formula Cache Bounds (15 minutes)
**Expected Impact:** Prevents unbounded memory growth

```rust
// File: src/analysis/formula.rs:17-19
// Change HashMap to LruCache
use lru::LruCache;
use std::num::NonZeroUsize;

pub struct FormulaAtlas {
    parser: Arc<Mutex<BatchParser>>,
    cache: Arc<RwLock<LruCache<String, Arc<ParsedFormula>>>>, // Changed
    _volatility: Arc<Vec<String>>,
}

impl FormulaAtlas {
    pub fn new(volatility_functions: Vec<String>) -> Self {
        Self {
            cache: Arc::new(RwLock::new(
                LruCache::new(NonZeroUsize::new(1000).unwrap())
            )),
            // ... rest
        }
    }
}
```

### 3. Add Cache Warming (30 minutes)
**Expected Impact:** Eliminates cold-start latency

```rust
// File: src/state.rs (add new method)
impl AppState {
    pub async fn warm_cache(&self, top_n: usize) -> Result<()> {
        let filter = WorkbookFilter::default();
        let workbooks = self.list_workbooks(filter)?;

        let mut sorted = workbooks.workbooks;
        sorted.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

        for descriptor in sorted.iter().take(top_n) {
            let _ = self.open_workbook(&descriptor.workbook_id).await;
        }
        Ok(())
    }
}

// File: src/main.rs (call on startup)
if let Err(e) = state.warm_cache(5).await {
    warn!("cache warming failed: {}", e);
}
```

---

## 📊 Quick Commands

### Profiling
```bash
# CPU profiling
cargo install flamegraph
cargo flamegraph --release

# Memory profiling
sudo apt-get install heaptrack
heaptrack ./target/release/spreadsheet-mcp

# Async debugging
RUSTFLAGS="--cfg tokio_unstable" cargo run --release
tokio-console  # in another terminal
```

### Benchmarking
```bash
# Run all benchmarks
cargo bench

# Run specific group
cargo bench cache
cargo bench string_alloc

# View reports
open target/criterion/report/index.html
```

### Analysis
```bash
# Binary size
cargo install cargo-bloat
cargo bloat --release -n 20

# Check struct sizes
cargo expand --lib | grep "struct "
```

---

## 🔍 Hot Paths (by CPU %)

| Path | Estimated CPU | File |
|------|---------------|------|
| Workbook Loading | 15-25% | `src/workbook.rs:142-191` |
| SPARQL Execution | 20-30% | `src/sparql/` |
| Formula Analysis | 10-15% | `src/analysis/formula.rs` |
| JSON Serialization | 8-12% | `src/server.rs` |
| Cache Operations | 5-10% | `src/state.rs:165-210` |

---

## 📈 Current Metrics

| Metric | Value | Status |
|--------|-------|--------|
| `.clone()` calls | 674 | ⚠️ Some optimizable |
| String allocations | 801 | ⚠️ Use Cow |
| RwLock/Mutex | 77 | ✅ Good choice |
| Async functions | 153 | ✅ Proper usage |
| `spawn_blocking` | 21 | ✅ Correct |

**Performance Grade:** B+ (87/100)

---

## 🎓 Pattern Library

### Pattern: Avoid Unnecessary Clones
```rust
// ❌ Bad
pub fn get_config(&self) -> ServerConfig {
    self.config.clone()
}

// ✅ Good
pub fn get_config(&self) -> &ServerConfig {
    &self.config
}

// ✅ Also Good (Arc)
pub fn get_config(&self) -> Arc<ServerConfig> {
    self.config.clone()  // Cheap Arc clone
}
```

### Pattern: String Optimization with Cow
```rust
use std::borrow::Cow;

// ✅ Good: Only allocate if needed
pub fn normalize_id(id: &str) -> Cow<str> {
    if id.chars().all(|c| c.is_lowercase()) {
        Cow::Borrowed(id)  // No allocation
    } else {
        Cow::Owned(id.to_lowercase())
    }
}
```

### Pattern: Vec with Capacity
```rust
// ❌ Bad: Multiple reallocations
let mut results = Vec::new();
for item in items {
    results.push(item);
}

// ✅ Good: Single allocation
let mut results = Vec::with_capacity(items.len());
for item in items {
    results.push(item);
}
```

### Pattern: Minimize Lock Hold Time
```rust
// ❌ Bad: Hold lock during I/O
let mut cache = self.cache.write();
let data = expensive_io_operation();
cache.put(key, data);

// ✅ Good: Release lock, then acquire
{
    let cache = self.cache.read();
    if cache.contains(&key) { return; }
}
let data = expensive_io_operation();
{
    let mut cache = self.cache.write();
    cache.put(key, data);
}
```

---

## 🚨 Anti-Patterns to Avoid

### ❌ Holding Lock During I/O
```rust
let mut cache = self.cache.write();
let data = fs::read(path)?;  // Blocking!
cache.put(key, data);
```

### ❌ Blocking in Async Context
```rust
async fn load_workbook() {
    // ❌ Bad: Blocks async runtime
    let data = fs::read(path)?;

    // ✅ Good: Use spawn_blocking
    let data = tokio::task::spawn_blocking(move || {
        fs::read(path)
    }).await??;
}
```

### ❌ Unnecessary format! in Loops
```rust
for i in 0..1000 {
    let key = format!("key-{}", i);  // ❌ 1000 allocations
}

// ✅ Better
let mut key = String::with_capacity(10);
for i in 0..1000 {
    key.clear();
    write!(&mut key, "key-{}", i).unwrap();
}
```

---

## 🎯 Performance Budgets

| Operation | Target p50 | Target p99 |
|-----------|------------|------------|
| Cache hit | < 1µs | < 10µs |
| List workbooks | < 50ms | < 200ms |
| Open (cached) | < 5ms | < 20ms |
| Open (cold) | < 200ms | < 1s |
| SPARQL (cached) | < 1ms | < 10ms |
| Formula parse | < 500µs | < 5ms |

---

## 📚 Documentation Links

- **Start Here:** [PERFORMANCE_README.md](PERFORMANCE_README.md)
- **Complete Guide:** [RUST_MCP_PERFORMANCE.md](RUST_MCP_PERFORMANCE.md)
- **Current Analysis:** [PERFORMANCE_ANALYSIS_REPORT.md](PERFORMANCE_ANALYSIS_REPORT.md)
- **Summary:** [../PERFORMANCE_OPTIMIZATION_IMPLEMENTATION_SUMMARY.md](../PERFORMANCE_OPTIMIZATION_IMPLEMENTATION_SUMMARY.md)

---

## 🔧 Tool Cheat Sheet

| Need | Tool | Command |
|------|------|---------|
| CPU profile | flamegraph | `cargo flamegraph` |
| Memory profile | heaptrack | `heaptrack ./binary` |
| Async debug | tokio-console | `tokio-console` |
| Benchmark | criterion | `cargo bench` |
| Binary size | cargo-bloat | `cargo bloat --release` |

---

## 🏆 TPS Waste Categories

| Waste | Example | Fix |
|-------|---------|-----|
| **Muda** (Waste) | SHA256 overhead | Use ahash |
| **Muri** (Overburden) | Unbounded cache | Add LRU bounds |
| **Mura** (Unevenness) | Cold start | Cache warming |

---

**Last Updated:** 2026-01-20
**Version:** 1.0
