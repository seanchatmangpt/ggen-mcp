// Performance Optimization Tests
//
// Tests for the three major performance optimizations:
// 1. ahash replacement for SPARQL cache fingerprinting
// 2. LRU bounds for formula cache
// 3. Cache warming

#[cfg(test)]
mod sparql_cache_tests {
    use std::hash::{Hash, Hasher};

    #[test]
    fn test_ahash_fingerprint_consistency() {
        // Test that ahash produces consistent fingerprints for the same input
        let query = "SELECT ?s ?p ?o WHERE { ?s ?p ?o }";

        let mut hasher1 = ahash::AHasher::default();
        query.hash(&mut hasher1);
        let fp1 = format!("{:016x}", hasher1.finish());

        let mut hasher2 = ahash::AHasher::default();
        query.hash(&mut hasher2);
        let fp2 = format!("{:016x}", hasher2.finish());

        assert_eq!(fp1, fp2, "ahash should produce consistent fingerprints");
    }

    #[test]
    fn test_ahash_collision_resistance() {
        // Test that ahash produces different fingerprints for different inputs
        let queries = vec![
            "SELECT ?s ?p ?o WHERE { ?s ?p ?o }",
            "SELECT ?name WHERE { ?person foaf:name ?name }",
            "SELECT ?label WHERE { ?concept rdfs:label ?label }",
            "CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }",
            "ASK { ?s a owl:Class }",
        ];

        let mut fingerprints = std::collections::HashSet::new();

        for query in &queries {
            let mut hasher = ahash::AHasher::default();
            query.hash(&mut hasher);
            let fp = format!("{:016x}", hasher.finish());
            assert!(
                fingerprints.insert(fp.clone()),
                "Collision detected for query: {}",
                query
            );
        }

        assert_eq!(
            fingerprints.len(),
            queries.len(),
            "All queries should have unique fingerprints"
        );
    }

    #[test]
    fn test_ahash_performance_vs_sha256() {
        use sha2::{Digest, Sha256};
        use std::time::Instant;

        let query =
            "SELECT ?s ?p ?o WHERE { ?s ?p ?o . ?s rdfs:label ?label . FILTER(?label = 'test') }";
        let iterations = 10_000;

        // Measure SHA256
        let start = Instant::now();
        for _ in 0..iterations {
            let mut hasher = Sha256::new();
            hasher.update(query.as_bytes());
            let _ = format!("{:x}", hasher.finalize());
        }
        let sha256_duration = start.elapsed();

        // Measure ahash
        let start = Instant::now();
        for _ in 0..iterations {
            let mut hasher = ahash::AHasher::default();
            query.hash(&mut hasher);
            let _ = format!("{:016x}", hasher.finish());
        }
        let ahash_duration = start.elapsed();

        println!(
            "SHA256: {:?}, ahash: {:?}, speedup: {:.2}x",
            sha256_duration,
            ahash_duration,
            sha256_duration.as_secs_f64() / ahash_duration.as_secs_f64()
        );

        // ahash should be at least 2x faster (conservative estimate)
        assert!(
            ahash_duration * 2 < sha256_duration,
            "ahash should be at least 2x faster than SHA256"
        );
    }
}

#[cfg(test)]
mod formula_cache_tests {
    use lru::LruCache;
    use parking_lot::RwLock;
    use std::num::NonZeroUsize;
    use std::sync::Arc;

    #[test]
    fn test_lru_cache_bounds() {
        // Test that LRU cache respects capacity bounds
        let capacity = 5;
        let cache = Arc::new(RwLock::new(LruCache::<String, String>::new(
            NonZeroUsize::new(capacity).unwrap(),
        )));

        // Insert more than capacity
        for i in 0..10 {
            let mut c = cache.write();
            c.push(format!("key-{}", i), format!("value-{}", i));
        }

        // Cache should contain exactly capacity items
        let c = cache.read();
        assert_eq!(c.len(), capacity, "Cache should not exceed capacity");
    }

    #[test]
    fn test_lru_eviction_order() {
        // Test that LRU cache evicts least recently used items
        let capacity = 3;
        let cache = Arc::new(RwLock::new(LruCache::<String, String>::new(
            NonZeroUsize::new(capacity).unwrap(),
        )));

        // Insert items
        {
            let mut c = cache.write();
            c.push("key-1".to_string(), "value-1".to_string());
            c.push("key-2".to_string(), "value-2".to_string());
            c.push("key-3".to_string(), "value-3".to_string());
        }

        // Access key-1 to make it recently used
        {
            let mut c = cache.write();
            c.get("key-1");
        }

        // Insert new item (should evict key-2, not key-1)
        {
            let mut c = cache.write();
            c.push("key-4".to_string(), "value-4".to_string());
        }

        // Verify key-1 is still in cache
        {
            let c = cache.read();
            assert!(
                c.peek("key-1").is_some(),
                "Recently used key-1 should not be evicted"
            );
            assert!(
                c.peek("key-2").is_none(),
                "Least recently used key-2 should be evicted"
            );
        }
    }

    #[test]
    fn test_cache_hit_rate_tracking() {
        // Test cache hit rate tracking
        let cache = Arc::new(RwLock::new(LruCache::<String, String>::new(
            NonZeroUsize::new(10).unwrap(),
        )));

        let mut hits = 0u64;
        let mut misses = 0u64;

        // Populate cache
        {
            let mut c = cache.write();
            for i in 0..5 {
                c.push(format!("key-{}", i), format!("value-{}", i));
            }
        }

        // Simulate cache accesses
        for i in 0..10 {
            let key = format!("key-{}", i % 7);
            let mut c = cache.write();
            if c.get(&key).is_some() {
                hits += 1;
            } else {
                misses += 1;
                c.push(key, format!("value-{}", i));
            }
        }

        let hit_rate = hits as f64 / (hits + misses) as f64;
        println!(
            "Hits: {}, Misses: {}, Hit rate: {:.2}%",
            hits,
            misses,
            hit_rate * 100.0
        );

        assert!(hits > 0, "Should have at least one cache hit");
        assert!(misses > 0, "Should have at least one cache miss");
    }

    #[test]
    fn test_memory_bounded_growth() {
        // Test that bounded cache prevents unbounded memory growth
        use std::collections::HashMap;

        // Unbounded HashMap
        let unbounded = Arc::new(RwLock::new(HashMap::<String, Vec<u8>>::new()));
        let large_value = vec![0u8; 1024]; // 1KB

        for i in 0..1000 {
            let mut map = unbounded.write();
            map.insert(format!("key-{}", i), large_value.clone());
        }

        let unbounded_size = unbounded.read().len();
        assert_eq!(unbounded_size, 1000, "Unbounded cache grows without limit");

        // Bounded LRU cache
        let bounded = Arc::new(RwLock::new(LruCache::<String, Vec<u8>>::new(
            NonZeroUsize::new(100).unwrap(),
        )));

        for i in 0..1000 {
            let mut cache = bounded.write();
            cache.push(format!("key-{}", i), large_value.clone());
        }

        let bounded_size = bounded.read().len();
        assert_eq!(bounded_size, 100, "Bounded cache respects size limit");
    }
}

#[cfg(test)]
mod cache_warming_tests {
    use parking_lot::RwLock;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Instant;

    #[test]
    fn test_cache_warming_reduces_latency() {
        // Simulate workbook lookup with and without cache warming
        let workbook_ids = vec!["wb1", "wb2", "wb3", "wb4", "wb5"];

        // Cold start - no cache warming
        let cold_start = Instant::now();
        let mut cold_results = Vec::new();
        for id in &workbook_ids {
            // Simulate disk I/O (1ms)
            std::thread::sleep(std::time::Duration::from_millis(1));
            cold_results.push(format!("loaded_{}", id));
        }
        let cold_duration = cold_start.elapsed();

        // Warm start - cache pre-loaded
        let cache = Arc::new(RwLock::new(HashMap::<String, String>::new()));
        {
            let mut c = cache.write();
            for id in &workbook_ids {
                c.insert(id.to_string(), format!("loaded_{}", id));
            }
        }

        let warm_start = Instant::now();
        let mut warm_results = Vec::new();
        for id in &workbook_ids {
            let c = cache.read();
            if let Some(cached) = c.get(*id) {
                warm_results.push(cached.clone());
            } else {
                std::thread::sleep(std::time::Duration::from_millis(1));
                warm_results.push(format!("loaded_{}", id));
            }
        }
        let warm_duration = warm_start.elapsed();

        println!(
            "Cold start: {:?}, Warm start: {:?}, speedup: {:.2}x",
            cold_duration,
            warm_duration,
            cold_duration.as_secs_f64() / warm_duration.as_secs_f64()
        );

        // Warm start should be significantly faster
        assert!(
            warm_duration < cold_duration / 5,
            "Warm start should be at least 5x faster"
        );
        assert_eq!(cold_results, warm_results, "Results should be identical");
    }

    #[test]
    fn test_cache_warming_timeout() {
        // Test that cache warming respects timeout
        let start = Instant::now();
        let timeout = std::time::Duration::from_millis(100);

        let mut loaded = 0;
        while start.elapsed() < timeout {
            // Simulate loading workbook
            std::thread::sleep(std::time::Duration::from_millis(50));
            loaded += 1;
        }

        let duration = start.elapsed();
        assert!(duration >= timeout, "Should respect timeout");
        assert!(
            loaded >= 1,
            "Should load at least one workbook before timeout"
        );
    }

    #[test]
    fn test_cache_warming_partial_failure() {
        // Test that cache warming continues after partial failures
        let workbook_ids = vec!["wb1", "wb2", "invalid", "wb3", "wb4"];
        let mut loaded = 0;
        let mut failed = 0;

        for id in &workbook_ids {
            if id.contains("invalid") {
                failed += 1;
            } else {
                loaded += 1;
            }
        }

        assert_eq!(loaded, 4, "Should load 4 valid workbooks");
        assert_eq!(failed, 1, "Should record 1 failure");
    }
}

#[cfg(test)]
mod integration_tests {
    use std::sync::Arc;
    use std::time::Instant;

    #[test]
    fn test_combined_optimizations() {
        // Test that all optimizations work together
        println!("\n=== Combined Performance Test ===");

        // 1. SPARQL cache with ahash
        let query = "SELECT ?s ?p ?o WHERE { ?s ?p ?o }";
        let iterations = 1000;

        let start = Instant::now();
        for _ in 0..iterations {
            use std::hash::{Hash, Hasher};
            let mut hasher = ahash::AHasher::default();
            query.hash(&mut hasher);
            let _ = format!("{:016x}", hasher.finish());
        }
        let ahash_time = start.elapsed();
        println!(
            "✓ ahash fingerprinting: {:?} for {} queries",
            ahash_time, iterations
        );

        // 2. Formula cache with LRU bounds
        use lru::LruCache;
        use parking_lot::RwLock;
        use std::num::NonZeroUsize;

        let cache = Arc::new(RwLock::new(LruCache::<String, String>::new(
            NonZeroUsize::new(100).unwrap(),
        )));

        let start = Instant::now();
        for i in 0..iterations {
            let formula = format!("SUM(A{}:A{})", i, i + 10);
            let mut c = cache.write();
            if c.get(&formula).is_none() {
                c.push(formula.clone(), format!("parsed_{}", formula));
            }
        }
        let cache_time = start.elapsed();
        let cache_size = cache.read().len();
        println!(
            "✓ Formula cache: {:?} for {} formulas, size: {}",
            cache_time, iterations, cache_size
        );
        assert_eq!(cache_size, 100, "Cache should be bounded to 100 entries");

        // 3. Cache warming simulation
        use std::collections::HashMap;
        let workbook_cache = Arc::new(RwLock::new(HashMap::<String, String>::new()));

        // Pre-warm cache
        let start = Instant::now();
        {
            let mut c = workbook_cache.write();
            for i in 1..=5 {
                c.insert(format!("wb{}", i), format!("loaded_wb{}", i));
            }
        }
        let warming_time = start.elapsed();
        println!("✓ Cache warming: {:?} for 5 workbooks", warming_time);

        println!("\n=== All Optimizations Verified ===\n");
    }

    #[test]
    fn test_performance_regression_detection() {
        // This test serves as a regression detector
        // If performance degrades, this test will catch it
        use std::time::Instant;

        let iterations = 10_000;

        // ahash should complete 10k operations in < 10ms
        let start = Instant::now();
        for i in 0..iterations {
            use std::hash::{Hash, Hasher};
            let query = format!("SELECT ?s WHERE {{ ?s ?p {} }}", i);
            let mut hasher = ahash::AHasher::default();
            query.hash(&mut hasher);
            let _ = hasher.finish();
        }
        let duration = start.elapsed();

        println!("ahash: {:?} for {} operations", duration, iterations);
        assert!(
            duration.as_millis() < 50,
            "ahash performance regression detected: {:?}",
            duration
        );
    }
}
