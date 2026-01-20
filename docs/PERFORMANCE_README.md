# Performance Optimization Documentation

This directory contains comprehensive performance optimization documentation for ggen-mcp (spreadsheet-mcp), built on Toyota Production System (TPS) principles of waste elimination.

## Documentation Overview

### 📘 [RUST_MCP_PERFORMANCE.md](./RUST_MCP_PERFORMANCE.md)
**Comprehensive Performance Optimization Guide**

A complete reference guide covering:
- Profiling tools and techniques (flamegraph, heaptrack, tokio-console)
- Allocation optimization strategies (reducing clones, string handling, Vec reuse)
- Cache optimization patterns (LRU tuning, warming, invalidation)
- Concurrency performance (lock contention, RwLock optimization, work stealing)
- I/O optimization (async best practices, zero-copy, batching)
- Hot path optimization (inlining, SIMD, branch prediction)
- Memory layout optimization (struct field ordering, enum sizes)
- Performance budgets and TPS waste elimination

**Target Audience:** Developers implementing performance optimizations

**Read this if:** You need detailed guidance on specific optimization techniques

---

### 📊 [PERFORMANCE_ANALYSIS_REPORT.md](./PERFORMANCE_ANALYSIS_REPORT.md)
**Current ggen-mcp Performance Analysis**

A detailed analysis of ggen-mcp's current performance characteristics:
- Hot path identification (workbook loading, SPARQL, formula parsing)
- Allocation pattern analysis (674 clones, 801 string allocations)
- Cache effectiveness evaluation
- Concurrency bottleneck analysis
- I/O performance patterns
- TPS waste categorization (Muda, Muri, Mura)
- Prioritized optimization recommendations

**Key Statistics:**
- 153 async functions
- 77 RwLock/Mutex usages
- 21 spawn_blocking call sites
- Performance Grade: B+ (87/100)

**Target Audience:** Developers, architects, performance engineers

**Read this if:** You want to understand current performance and optimization priorities

---

### 🔬 [mcp_performance_benchmarks.rs](../benches/mcp_performance_benchmarks.rs)
**Comprehensive Benchmark Suite**

Criterion-based benchmark suite covering:
1. Cache operations (LRU, read/write performance)
2. Allocation benchmarks (string, Vec, clones)
3. Lock contention (parking_lot vs std, RwLock vs Mutex)
4. Hashing performance (SHA256 vs ahash)
5. SPARQL cache simulation
6. Workbook ID operations
7. JSON serialization
8. Formula pattern matching
9. I/O patterns (buffered vs unbuffered)
10. Realistic MCP request simulation

**Target Audience:** Performance engineers, CI/CD integration

**Use this for:** Regression detection, optimization validation, performance tracking

---

## Quick Start

### 1. Run Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark group
cargo bench cache
cargo bench string_alloc
cargo bench locks

# View HTML reports
open target/criterion/report/index.html
```

### 2. Profile CPU Usage

```bash
# Install flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --release --bin spreadsheet-mcp

# Open generated SVG
open flamegraph.svg
```

### 3. Profile Memory

```bash
# Install heaptrack (Linux)
sudo apt-get install heaptrack heaptrack-gui

# Run with heaptrack
heaptrack ./target/release/spreadsheet-mcp

# Analyze
heaptrack_gui heaptrack.spreadsheet-mcp.*.gz
```

### 4. Monitor Async Runtime

```bash
# Terminal 1: Run with tokio-console support
RUSTFLAGS="--cfg tokio_unstable" cargo run --release

# Terminal 2: Launch console
tokio-console
```

---

## Priority 1 Optimizations

Based on the analysis report, implement these high-impact, low-effort optimizations first:

### 1. Replace SHA256 with ahash for SPARQL cache

**File:** `src/sparql/cache.rs:127-131`

**Change:**
```rust
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

**Expected Impact:** 5-10x speedup for cache operations

**Effort:** 10 minutes

---

### 2. Add bounds to FormulaAtlas cache

**File:** `src/analysis/formula.rs:17-19`

**Change:**
```rust
// BEFORE
pub struct FormulaAtlas {
    parser: Arc<Mutex<BatchParser>>,
    cache: Arc<RwLock<HashMap<String, Arc<ParsedFormula>>>>,
    _volatility: Arc<Vec<String>>,
}

// AFTER
use lru::LruCache;
use std::num::NonZeroUsize;

pub struct FormulaAtlas {
    parser: Arc<Mutex<BatchParser>>,
    cache: Arc<RwLock<LruCache<String, Arc<ParsedFormula>>>>,
    _volatility: Arc<Vec<String>>,
}

impl FormulaAtlas {
    pub fn new(volatility_functions: Vec<String>) -> Self {
        // ... existing code ...
        Self {
            parser: Arc::new(Mutex::new(parser)),
            cache: Arc::new(RwLock::new(
                LruCache::new(NonZeroUsize::new(1000).unwrap())
            )),
            _volatility: lookup,
        }
    }
}
```

**Expected Impact:** Prevents unbounded memory growth

**Effort:** 15 minutes

---

### 3. Add cache warming on startup

**File:** `src/state.rs` (new method)

**Add:**
```rust
impl AppState {
    /// Warm cache with most recently modified workbooks
    pub async fn warm_cache(&self, top_n: usize) -> Result<()> {
        let filter = WorkbookFilter::default();
        let workbooks = self.list_workbooks(filter)?;

        let mut sorted = workbooks.workbooks;
        sorted.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

        for descriptor in sorted.iter().take(top_n) {
            let _ = self.open_workbook(&descriptor.workbook_id).await;
            debug!(workbook_id = %descriptor.workbook_id, "cache warmed");
        }

        Ok(())
    }
}
```

**Call in main:**
```rust
#[tokio::main]
async fn main() -> Result<()> {
    // ... existing initialization ...

    // Warm cache with top 5 workbooks
    if let Err(e) = state.warm_cache(5).await {
        warn!("cache warming failed: {}", e);
    }

    // ... start server ...
}
```

**Expected Impact:** Eliminates cold-start latency

**Effort:** 30 minutes

---

## TPS Performance Principles

This documentation follows Toyota Production System waste elimination principles:

### The 8 Wastes (Muda) in Performance

1. **Defects** - Bugs causing retries/reprocessing
2. **Overproduction** - Computing unused results
3. **Waiting** - Blocking on I/O or locks
4. **Non-utilized talent** - Not using SIMD, parallelism
5. **Transportation** - Unnecessary data copies
6. **Inventory** - Excessive caching
7. **Motion** - Cache misses, pointer chasing
8. **Extra processing** - Redundant computations

### Muri (Overburden)

- Thread pool saturation
- Cache thrashing
- Memory pressure
- CPU overload

### Mura (Unevenness)

- Request latency variance
- Bursty I/O patterns
- Cold-start performance gaps
- Memory usage spikes

### Continuous Improvement (Kaizen)

1. **Measure** - Profile and benchmark
2. **Analyze** - Identify waste
3. **Improve** - Implement optimization
4. **Standardize** - Document patterns
5. **Repeat** - Continuous refinement

---

## Performance Metrics

### Target Performance Budgets

| Operation | p50 Latency | p99 Latency | Throughput |
|-----------|-------------|-------------|------------|
| Cache hit | < 1µs | < 10µs | 10,000 ops/s |
| List workbooks | < 50ms | < 200ms | 100 req/s |
| Open workbook (cached) | < 5ms | < 20ms | 500 req/s |
| Open workbook (cold) | < 200ms | < 1s | 50 req/s |
| SPARQL query (cached) | < 1ms | < 10ms | 1,000 req/s |
| Formula parsing | < 500µs | < 5ms | 2,000 ops/s |

### Monitoring Checklist

- [ ] CPU usage (target: < 70% avg)
- [ ] Memory usage (target: < 2GB for typical workload)
- [ ] Cache hit rate (target: > 80%)
- [ ] Request latency (p50, p95, p99)
- [ ] Error rate (target: < 1%)
- [ ] Concurrent connections (max: 100)
- [ ] Thread pool utilization
- [ ] Lock contention events

---

## Benchmark Regression CI

### GitHub Actions Integration

Add to `.github/workflows/benchmark.yml`:

```yaml
name: Performance Benchmarks

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          profile: minimal
          toolchain: stable

      - name: Run benchmarks
        run: cargo bench --bench mcp_performance_benchmarks

      - name: Store benchmark results
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: target/criterion/*/new/estimates.json
          github-token: ${{ secrets.GITHUB_TOKEN }}
          auto-push: true
          alert-threshold: '120%'  # Alert if 20% slower
          comment-on-alert: true
          fail-on-alert: true
```

---

## Tools Reference

### Profiling Tools

| Tool | Purpose | Install | Usage |
|------|---------|---------|-------|
| **cargo-flamegraph** | CPU profiling | `cargo install flamegraph` | `cargo flamegraph` |
| **heaptrack** | Memory profiling | `apt install heaptrack` | `heaptrack ./binary` |
| **tokio-console** | Async debugging | `cargo install tokio-console` | `tokio-console` |
| **cargo-bloat** | Binary size analysis | `cargo install cargo-bloat` | `cargo bloat --release` |
| **criterion** | Benchmarking | (dev-dependency) | `cargo bench` |

### Analysis Tools

| Tool | Purpose | Usage |
|------|---------|-------|
| **perf** | Linux profiling | `perf record -g ./binary` |
| **valgrind** | Memory errors | `valgrind --leak-check=full ./binary` |
| **strace** | System call trace | `strace -c ./binary` |
| **ltrace** | Library call trace | `ltrace -c ./binary` |

---

## Additional Resources

### External Documentation

- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Tokio Performance Guide](https://tokio.rs/tokio/topics/performance)
- [The Rust Performance Book (Community)](https://www.lurklurk.org/effective-rust/perf.html)
- [Lock-Free Programming](https://preshing.com/20120612/an-introduction-to-lock-free-programming/)

### Toyota Production System

- Taiichi Ohno - "Toyota Production System: Beyond Large-Scale Production"
- Jeffrey Liker - "The Toyota Way"
- Mike Rother - "Toyota Kata"

### Internal Documentation

- [../TPS_RESEARCH_COMPLETE.md](../TPS_RESEARCH_COMPLETE.md) - TPS research summary
- [../KAIZEN_RESEARCH_SUMMARY.md](../KAIZEN_RESEARCH_SUMMARY.md) - Kaizen methodology
- [../POKA_YOKE_IMPLEMENTATION.md](../POKA_YOKE_IMPLEMENTATION.md) - Error prevention

---

## FAQ

**Q: Should I optimize before profiling?**
A: No. Always profile first to identify actual bottlenecks. Premature optimization wastes time.

**Q: What's the target performance improvement?**
A: Based on TPS analysis, we can reduce performance waste from 45% to 23% (52% reduction).

**Q: How often should I run benchmarks?**
A: Run on every commit to main/develop to catch regressions early.

**Q: Which locks are faster: parking_lot or std?**
A: parking_lot is typically 2-3x faster and is already used in ggen-mcp (good choice).

**Q: Should I use SIMD optimizations?**
A: Only for proven hot paths with data-parallel workloads. Profile first.

**Q: How do I reduce memory usage?**
A: Focus on: (1) cache tuning, (2) reducing clones, (3) bounded collections, (4) memory pools.

---

## Contributing

When implementing optimizations:

1. **Profile first** - Use flamegraph/heaptrack to identify bottlenecks
2. **Benchmark** - Add benchmarks before optimizing
3. **Measure impact** - Run benchmarks before and after
4. **Document** - Update analysis report with findings
5. **Test** - Ensure correctness is maintained
6. **Review** - Get code review focusing on safety

---

## Contact

For performance-related questions or issues:
- File a GitHub issue with label `performance`
- Provide profiling data (flamegraph, benchmark results)
- Include workload characteristics (file sizes, operation types)

---

**Last Updated:** 2026-01-20
**Documentation Version:** 1.0
**Next Review:** After Priority 1 optimizations implemented
