// MCP Performance Benchmarks
//
// This benchmark suite measures critical performance paths in ggen-mcp
// following Toyota Production System (TPS) principles:
// - Measure actual work (genchi genbutsu)
// - Identify waste (muda)
// - Standardize processes
//
// Run with: cargo bench
// View reports: target/criterion/report/index.html

use criterion::{
    BatchSize, BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main,
};
use lru::LruCache;
use parking_lot::RwLock;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Duration;

// =============================================================================
// 1. CACHE PERFORMANCE BENCHMARKS
// =============================================================================

fn bench_cache_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache");

    // Test different cache sizes (realistic for MCP server)
    for size in [10, 50, 100, 500] {
        // Cache Hit Performance
        group.bench_with_input(BenchmarkId::new("hit_rwlock", size), &size, |b, &size| {
            let cache = Arc::new(RwLock::new(LruCache::new(NonZeroUsize::new(size).unwrap())));

            // Pre-populate cache
            {
                let mut c = cache.write();
                for i in 0..size {
                    c.put(format!("key-{}", i), format!("value-{}", i));
                }
            }

            let key = format!("key-{}", size / 2);

            b.iter(|| {
                let c = cache.read();
                black_box(c.peek(&key).cloned())
            });
        });

        // Cache Miss + Insert Performance
        group.bench_with_input(
            BenchmarkId::new("miss_insert_rwlock", size),
            &size,
            |b, &size| {
                let cache = Arc::new(RwLock::new(LruCache::new(NonZeroUsize::new(size).unwrap())));
                let mut counter = 0;

                b.iter(|| {
                    counter += 1;
                    let key = format!("key-{}", counter);
                    let value = format!("value-{}", counter);

                    {
                        let c = cache.read();
                        if c.peek(&key).is_some() {
                            return;
                        }
                    }

                    {
                        let mut c = cache.write();
                        c.put(key, value);
                    }
                });
            },
        );

        // Concurrent Read Throughput (parking_lot::RwLock)
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("concurrent_reads", size),
            &size,
            |b, &size| {
                let cache = Arc::new(RwLock::new(LruCache::new(NonZeroUsize::new(size).unwrap())));

                {
                    let mut c = cache.write();
                    for i in 0..size {
                        c.put(format!("key-{}", i), Arc::new(format!("value-{}", i)));
                    }
                }

                let key = format!("key-{}", size / 2);

                b.iter(|| {
                    let c = cache.read();
                    black_box(c.peek(&key).cloned())
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// 2. ALLOCATION BENCHMARKS
// =============================================================================

fn bench_string_allocations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_alloc");

    // String::clone vs Arc<String>::clone
    group.bench_function("string_clone", |b| {
        let s = "workbook-12345-sheet-data".to_string();
        b.iter(|| black_box(s.clone()));
    });

    group.bench_function("arc_string_clone", |b| {
        let s = Arc::new("workbook-12345-sheet-data".to_string());
        b.iter(|| black_box(s.clone()));
    });

    // to_string() vs to_owned() vs String::from()
    let source = "example_workbook_identifier";

    group.bench_function("to_string", |b| {
        b.iter(|| black_box(source.to_string()));
    });

    group.bench_function("to_owned", |b| {
        b.iter(|| black_box(source.to_owned()));
    });

    group.bench_function("string_from", |b| {
        b.iter(|| black_box(String::from(source)));
    });

    // format! vs pre-allocated
    group.bench_function("format_macro", |b| {
        let id = "wb123";
        let sheet = "Sheet1";
        b.iter(|| black_box(format!("{}-{}", id, sheet)));
    });

    group.bench_function("preallocated_string", |b| {
        let id = "wb123";
        let sheet = "Sheet1";
        b.iter(|| {
            let mut s = String::with_capacity(id.len() + sheet.len() + 1);
            s.push_str(id);
            s.push('-');
            s.push_str(sheet);
            black_box(s)
        });
    });

    group.finish();
}

fn bench_vec_allocations(c: &mut Criterion) {
    let mut group = c.benchmark_group("vec_alloc");

    // Vec::new() with push vs with_capacity
    for size in [10, 100, 1000] {
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("push_reallocate", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    let mut v = Vec::new();
                    for i in 0..size {
                        v.push(i);
                    }
                    black_box(v)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("with_capacity", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    let mut v = Vec::with_capacity(size);
                    for i in 0..size {
                        v.push(i);
                    }
                    black_box(v)
                });
            },
        );

        group.bench_with_input(BenchmarkId::new("collect", size), &size, |b, &size| {
            b.iter(|| {
                let v: Vec<_> = (0..size).collect();
                black_box(v)
            });
        });
    }

    group.finish();
}

// =============================================================================
// 3. LOCK CONTENTION BENCHMARKS
// =============================================================================

fn bench_lock_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("locks");

    // parking_lot::RwLock vs std::sync::RwLock (read-heavy)
    group.bench_function("parking_lot_rwlock_read", |b| {
        let lock = parking_lot::RwLock::new(42u64);
        b.iter(|| {
            let guard = lock.read();
            black_box(*guard)
        });
    });

    group.bench_function("std_rwlock_read", |b| {
        let lock = std::sync::RwLock::new(42u64);
        b.iter(|| {
            let guard = lock.read().unwrap();
            black_box(*guard)
        });
    });

    // parking_lot::Mutex vs std::sync::Mutex
    group.bench_function("parking_lot_mutex", |b| {
        let lock = parking_lot::Mutex::new(42u64);
        b.iter(|| {
            let mut guard = lock.lock();
            *guard += 1;
            black_box(*guard)
        });
    });

    group.bench_function("std_mutex", |b| {
        let lock = std::sync::Mutex::new(42u64);
        b.iter(|| {
            let mut guard = lock.lock().unwrap();
            *guard += 1;
            black_box(*guard)
        });
    });

    // Atomic vs Mutex for counters
    use std::sync::atomic::{AtomicU64, Ordering};

    group.bench_function("atomic_counter", |b| {
        let counter = AtomicU64::new(0);
        b.iter(|| black_box(counter.fetch_add(1, Ordering::Relaxed)));
    });

    group.bench_function("mutex_counter", |b| {
        let counter = parking_lot::Mutex::new(0u64);
        b.iter(|| {
            let mut guard = counter.lock();
            *guard += 1;
            black_box(*guard)
        });
    });

    group.finish();
}

// =============================================================================
// 4. HASHING BENCHMARKS
// =============================================================================

fn bench_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashing");

    let data = b"SELECT ?s ?p ?o WHERE { ?s ?p ?o . ?s rdfs:label ?label }";

    // SHA256 (used in SPARQL cache fingerprinting)
    group.bench_function("sha256", |b| {
        b.iter(|| {
            let mut hasher = Sha256::new();
            hasher.update(black_box(data));
            black_box(format!("{:x}", hasher.finalize()))
        });
    });

    // ahash (faster, non-cryptographic)
    group.bench_function("ahash", |b| {
        use std::hash::{Hash, Hasher};
        b.iter(|| {
            let mut hasher = ahash::AHasher::default();
            black_box(data).hash(&mut hasher);
            black_box(hasher.finish())
        });
    });

    // std DefaultHasher
    group.bench_function("default_hasher", |b| {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        b.iter(|| {
            let mut hasher = DefaultHasher::new();
            black_box(data).hash(&mut hasher);
            black_box(hasher.finish())
        });
    });

    group.finish();
}

// =============================================================================
// 5. SPARQL QUERY CACHE SIMULATION (WITH OPTIMIZATION COMPARISON)
// =============================================================================

fn bench_sparql_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparql_cache");

    // Simulate SPARQL query fingerprinting + cache lookup
    let queries = vec![
        "SELECT ?s ?p ?o WHERE { ?s ?p ?o }",
        "SELECT ?name WHERE { ?person foaf:name ?name }",
        "SELECT ?label WHERE { ?concept rdfs:label ?label }",
        "CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }",
        "ASK { ?s a owl:Class }",
    ];

    // BEFORE: SHA256-based fingerprinting
    group.bench_function("sha256_fingerprint_and_lookup", |b| {
        let cache = Arc::new(RwLock::new(LruCache::<String, Vec<String>>::new(
            NonZeroUsize::new(100).unwrap(),
        )));

        // Pre-populate with some entries
        {
            let mut c = cache.write();
            for (i, query) in queries.iter().enumerate() {
                let mut hasher = Sha256::new();
                hasher.update(query.as_bytes());
                let fp = format!("{:x}", hasher.finalize());
                c.put(fp, vec![format!("result-{}", i)]);
            }
        }

        let mut query_idx = 0;

        b.iter(|| {
            let query = queries[query_idx % queries.len()];
            query_idx += 1;

            // Fingerprint
            let mut hasher = Sha256::new();
            hasher.update(query.as_bytes());
            let fp = format!("{:x}", hasher.finalize());

            // Cache lookup
            let c = cache.read();
            black_box(c.peek(&fp).cloned())
        });
    });

    // AFTER: ahash-based fingerprinting (optimized)
    group.bench_function("ahash_fingerprint_and_lookup", |b| {
        use std::hash::{Hash, Hasher};
        let cache = Arc::new(RwLock::new(LruCache::<String, Vec<String>>::new(
            NonZeroUsize::new(100).unwrap(),
        )));

        // Pre-populate with some entries
        {
            let mut c = cache.write();
            for (i, query) in queries.iter().enumerate() {
                let mut hasher = ahash::AHasher::default();
                query.hash(&mut hasher);
                let fp = format!("{:016x}", hasher.finish());
                c.put(fp, vec![format!("result-{}", i)]);
            }
        }

        let mut query_idx = 0;

        b.iter(|| {
            let query = queries[query_idx % queries.len()];
            query_idx += 1;

            // Fingerprint (optimized with ahash)
            let mut hasher = ahash::AHasher::default();
            query.hash(&mut hasher);
            let fp = format!("{:016x}", hasher.finish());

            // Cache lookup
            let c = cache.read();
            black_box(c.peek(&fp).cloned())
        });
    });

    group.finish();
}

// =============================================================================
// 6. WORKBOOK ID OPERATIONS
// =============================================================================

fn bench_workbook_id_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("workbook_id");

    // Case-insensitive ID lookup (common in ggen-mcp)
    let ids: HashMap<String, String> = (0..100)
        .map(|i| {
            let id = format!("workbook-{:04}", i);
            (id.to_lowercase(), id)
        })
        .collect();

    group.bench_function("case_insensitive_lookup", |b| {
        let lookup_id = "WORKBOOK-0042";
        b.iter(|| {
            let lower = lookup_id.to_lowercase();
            black_box(ids.get(&lower))
        });
    });

    // Cow optimization for already-lowercase
    use std::borrow::Cow;

    group.bench_function("cow_lowercase_needed", |b| {
        let id = "WORKBOOK-0042";
        b.iter(|| {
            let normalized: Cow<str> = if id.chars().all(|c| c.is_lowercase()) {
                Cow::Borrowed(id)
            } else {
                Cow::Owned(id.to_lowercase())
            };
            black_box(normalized)
        });
    });

    group.bench_function("cow_lowercase_not_needed", |b| {
        let id = "workbook-0042";
        b.iter(|| {
            let normalized: Cow<str> = if id.chars().all(|c| c.is_lowercase()) {
                Cow::Borrowed(id)
            } else {
                Cow::Owned(id.to_lowercase())
            };
            black_box(normalized)
        });
    });

    group.finish();
}

// =============================================================================
// 7. JSON SERIALIZATION
// =============================================================================

fn bench_json_serialization(c: &mut Criterion) {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct WorkbookDescriptor {
        workbook_id: String,
        short_id: String,
        slug: String,
        path: String,
        bytes: u64,
        last_modified: Option<String>,
    }

    let descriptor = WorkbookDescriptor {
        workbook_id: "abc123def456".to_string(),
        short_id: "wb42".to_string(),
        slug: "financial-report".to_string(),
        path: "/data/workbooks/financial-report.xlsx".to_string(),
        bytes: 1024000,
        last_modified: Some("2026-01-20T10:30:00Z".to_string()),
    };

    let mut group = c.benchmark_group("json");

    group.bench_function("serialize_to_string", |b| {
        b.iter(|| black_box(serde_json::to_string(&descriptor).unwrap()));
    });

    group.bench_function("serialize_to_vec", |b| {
        b.iter(|| black_box(serde_json::to_vec(&descriptor).unwrap()));
    });

    let json_str = serde_json::to_string(&descriptor).unwrap();

    group.bench_function("deserialize_from_str", |b| {
        b.iter(|| black_box(serde_json::from_str::<WorkbookDescriptor>(&json_str).unwrap()));
    });

    group.finish();
}

// =============================================================================
// 8. FORMULA PARSING SIMULATION (WITH CACHE BOUNDS)
// =============================================================================

fn bench_formula_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("formula");

    let formulas = vec![
        "SUM(A1:A10)",
        "IF(B2>100, B2*1.1, B2*0.9)",
        "VLOOKUP(D2, A1:B100, 2, FALSE)",
        "INDEX(MATCH(E2, A:A, 0), B:B)",
        "SUMIFS(Sales, Region, \"North\", Year, 2026)",
    ];

    // Simulate formula fingerprinting
    group.bench_function("fingerprint_formula", |b| {
        let mut idx = 0;
        b.iter(|| {
            let formula = formulas[idx % formulas.len()];
            idx += 1;

            // Simple fingerprint: hash the formula
            use std::hash::{Hash, Hasher};
            let mut hasher = ahash::AHasher::default();
            formula.hash(&mut hasher);
            black_box(hasher.finish())
        });
    });

    // BEFORE: Unbounded HashMap cache
    group.bench_function("unbounded_cache", |b| {
        use std::collections::HashMap;
        let cache = Arc::new(RwLock::new(HashMap::<String, String>::new()));

        b.iter(|| {
            let formula = formulas[black_box(0) % formulas.len()];

            // Check cache
            {
                let c = cache.read();
                if let Some(cached) = c.get(formula) {
                    return black_box(cached.clone());
                }
            }

            // Parse and insert
            let result = format!("parsed_{}", formula);
            {
                let mut c = cache.write();
                c.insert(formula.to_string(), result.clone());
            }

            black_box(result)
        });
    });

    // AFTER: LRU-bounded cache (prevents memory leak)
    group.bench_function("lru_bounded_cache", |b| {
        let cache = Arc::new(RwLock::new(LruCache::<String, String>::new(
            NonZeroUsize::new(100).unwrap(),
        )));

        b.iter(|| {
            let formula = formulas[black_box(0) % formulas.len()];

            // Check cache
            {
                let mut c = cache.write();
                if let Some(cached) = c.get(formula) {
                    return black_box(cached.clone());
                }
            }

            // Parse and insert
            let result = format!("parsed_{}", formula);
            {
                let mut c = cache.write();
                c.push(formula.to_string(), result.clone());
            }

            black_box(result)
        });
    });

    // Simulate dependency extraction (simple pattern matching)
    group.bench_function("extract_cell_refs", |b| {
        let formula = "SUM(A1:A10) + B5 * C3";
        b.iter(|| {
            let refs: Vec<&str> = formula
                .split(|c: char| !c.is_alphanumeric())
                .filter(|s| {
                    !s.is_empty()
                        && s.chars().next().unwrap().is_alphabetic()
                        && s.chars().any(|c| c.is_numeric())
                })
                .collect();
            black_box(refs)
        });
    });

    group.finish();
}

// =============================================================================
// 9. I/O PATTERNS
// =============================================================================

fn bench_io_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("io");
    group.sample_size(50); // Fewer samples for I/O benchmarks

    use std::io::Write;
    use tempfile::NamedTempFile;

    // Buffered vs unbuffered writes
    let data = vec![42u8; 4096]; // 4KB

    group.throughput(Throughput::Bytes(data.len() as u64));

    group.bench_function("unbuffered_write", |b| {
        b.iter_batched(
            || NamedTempFile::new().unwrap(),
            |mut file| {
                file.write_all(&data).unwrap();
                file.flush().unwrap();
            },
            BatchSize::PerIteration,
        );
    });

    group.bench_function("buffered_write", |b| {
        use std::io::BufWriter;
        b.iter_batched(
            || NamedTempFile::new().unwrap(),
            |file| {
                let mut writer = BufWriter::with_capacity(8192, file);
                writer.write_all(&data).unwrap();
                writer.flush().unwrap();
            },
            BatchSize::PerIteration,
        );
    });

    group.finish();
}

// =============================================================================
// 10. CACHE WARMING SIMULATION
// =============================================================================

fn bench_cache_warming(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_warming");
    group.sample_size(20);

    // Simulate cache warming for multiple workbooks
    group.bench_function("cold_start_no_warming", |b| {
        b.iter(|| {
            // Simulate loading workbook from disk (cold)
            let workbook_ids = vec!["wb1", "wb2", "wb3", "wb4", "wb5"];
            let mut results = Vec::new();

            for id in &workbook_ids {
                // Simulate disk I/O latency
                std::thread::sleep(Duration::from_micros(100));
                results.push(format!("loaded_{}", id));
            }

            black_box(results)
        });
    });

    group.bench_function("warm_start_with_cache", |b| {
        // Pre-warm cache
        let cache = Arc::new(RwLock::new(LruCache::<String, String>::new(
            NonZeroUsize::new(50).unwrap(),
        )));

        {
            let mut c = cache.write();
            for i in 1..=5 {
                let id = format!("wb{}", i);
                c.put(id.clone(), format!("loaded_{}", id));
            }
        }

        b.iter(|| {
            let workbook_ids = vec!["wb1", "wb2", "wb3", "wb4", "wb5"];
            let mut results = Vec::new();

            for id in &workbook_ids {
                let cache = cache.read();
                if let Some(cached) = cache.peek(*id) {
                    results.push(cached.clone());
                } else {
                    // Simulate disk I/O only for cache miss
                    std::thread::sleep(Duration::from_micros(100));
                    results.push(format!("loaded_{}", id));
                }
            }

            black_box(results)
        });
    });

    group.finish();
}

// =============================================================================
// 11. REALISTIC MCP REQUEST SIMULATION
// =============================================================================

fn bench_mcp_request_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("mcp_request");
    group.measurement_time(Duration::from_secs(10));

    // Simulate a typical MCP request flow
    struct MockAppState {
        cache: Arc<RwLock<LruCache<String, Arc<String>>>>,
        index: Arc<RwLock<HashMap<String, String>>>,
    }

    impl MockAppState {
        fn new() -> Self {
            let cache = Arc::new(RwLock::new(LruCache::new(NonZeroUsize::new(50).unwrap())));
            let index = Arc::new(RwLock::new(HashMap::new()));

            // Pre-populate
            {
                let mut idx = index.write();
                for i in 0..100 {
                    idx.insert(format!("wb-{}", i), format!("/path/to/wb-{}.xlsx", i));
                }
            }

            Self { cache, index }
        }

        fn get_workbook(&self, id: &str) -> Option<Arc<String>> {
            // 1. Try cache (read lock)
            {
                let cache = self.cache.read();
                if let Some(wb) = cache.peek(id) {
                    return Some(wb.clone());
                }
            }

            // 2. Cache miss - resolve path
            let path = {
                let index = self.index.read();
                index.get(id).cloned()?
            };

            // 3. Simulate loading (in real code: spawn_blocking)
            let workbook = Arc::new(format!("Workbook data from {}", path));

            // 4. Insert into cache
            {
                let mut cache = self.cache.write();
                cache.put(id.to_string(), workbook.clone());
            }

            Some(workbook)
        }
    }

    group.bench_function("cache_hit_request", |b| {
        let state = MockAppState::new();

        // Pre-populate cache
        for i in 0..10 {
            state.get_workbook(&format!("wb-{}", i));
        }

        b.iter(|| black_box(state.get_workbook("wb-5")));
    });

    group.bench_function("cache_miss_request", |b| {
        let state = MockAppState::new();
        let mut counter = 100;

        b.iter(|| {
            counter += 1;
            black_box(state.get_workbook(&format!("wb-{}", counter)))
        });
    });

    group.finish();
}

// =============================================================================
// CRITERION CONFIGURATION
// =============================================================================

criterion_group!(
    benches,
    bench_cache_operations,
    bench_string_allocations,
    bench_vec_allocations,
    bench_lock_types,
    bench_hashing,
    bench_sparql_cache,
    bench_workbook_id_operations,
    bench_json_serialization,
    bench_formula_patterns,
    bench_io_patterns,
    bench_cache_warming,
    bench_mcp_request_simulation,
);

criterion_main!(benches);
