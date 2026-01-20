// =============================================================================
// Comprehensive Caching Infrastructure Tests
// =============================================================================
// Tests for ontology cache, SPARQL query cache, and integrated caching

use oxigraph::store::Store;
use spreadsheet_mcp::ontology::{OntologyCache, OntologyCacheConfig, OntologyId};
use spreadsheet_mcp::sparql::cache::{CacheConfig, CacheInvalidationStrategy, QueryResultCache};
use std::sync::Arc;
use std::time::Duration;

// =============================================================================
// Helper Functions
// =============================================================================

fn create_test_store() -> Store {
    Store::new().expect("create test store")
}

fn populate_test_store(store: &Store, triple_count: usize) {
    use oxigraph::model::*;

    let ex = NamedNodeRef::new("http://example.org/").unwrap();
    for i in 0..triple_count {
        let subject = NamedNode::new(format!("http://example.org/s{}", i)).unwrap();
        let predicate = NamedNode::new(format!("http://example.org/p{}", i)).unwrap();
        let object = Literal::new_simple_literal(format!("value{}", i));
        store
            .insert(&Quad::new(subject, predicate, object, None))
            .expect("insert triple");
    }
}

// =============================================================================
// Ontology Cache Tests
// =============================================================================

#[test]
fn test_ontology_cache_basic_operations() {
    let cache = OntologyCache::with_defaults();

    // Test put and get
    let id = OntologyId::new("test-ontology");
    let store = create_test_store();
    cache.put(id.clone(), store, None, None);

    let retrieved = cache.get(&id);
    assert!(retrieved.is_some(), "should retrieve cached ontology");

    // Test contains
    assert!(
        cache.contains(&id),
        "cache should contain the ontology"
    );
}

#[test]
fn test_ontology_cache_miss() {
    let cache = OntologyCache::with_defaults();
    let id = OntologyId::new("nonexistent");

    let result = cache.get(&id);
    assert!(result.is_none(), "should return None for cache miss");

    let stats = cache.stats();
    assert_eq!(stats.misses, 1, "miss counter should increment");
    assert_eq!(stats.hits, 0, "hit counter should be zero");
}

#[test]
fn test_ontology_cache_hit_tracking() {
    let cache = OntologyCache::with_defaults();
    let id = OntologyId::new("tracked");

    cache.put(id.clone(), create_test_store(), None, None);

    // Multiple accesses
    cache.get(&id);
    cache.get(&id);
    cache.get(&id);

    let stats = cache.stats();
    assert_eq!(stats.hits, 3, "should track all hits");
}

#[test]
fn test_ontology_cache_lru_eviction() {
    let config = OntologyCacheConfig {
        max_entries: 3,
        default_ttl_secs: 3600,
        auto_evict_expired: true,
    };
    let cache = OntologyCache::new(config);

    // Fill cache to capacity
    cache.put(OntologyId::new("ont1"), create_test_store(), None, None);
    cache.put(OntologyId::new("ont2"), create_test_store(), None, None);
    cache.put(OntologyId::new("ont3"), create_test_store(), None, None);

    // Access ont1 to make it recently used
    cache.get(&OntologyId::new("ont1"));

    // Add ont4, should evict ont2 (least recently used)
    cache.put(OntologyId::new("ont4"), create_test_store(), None, None);

    assert!(
        cache.contains(&OntologyId::new("ont1")),
        "ont1 should remain (recently accessed)"
    );
    assert!(
        !cache.contains(&OntologyId::new("ont2")),
        "ont2 should be evicted (LRU)"
    );
    assert!(
        cache.contains(&OntologyId::new("ont3")),
        "ont3 should remain"
    );
    assert!(
        cache.contains(&OntologyId::new("ont4")),
        "ont4 should be cached"
    );
}

#[test]
fn test_ontology_cache_ttl_expiration() {
    let config = OntologyCacheConfig {
        max_entries: 10,
        default_ttl_secs: 1,
        auto_evict_expired: true,
    };
    let cache = OntologyCache::new(config);
    let id = OntologyId::new("expire-test");

    // Cache with very short TTL
    cache.put(
        id.clone(),
        create_test_store(),
        Some(Duration::from_millis(100)),
        None,
    );

    // Should be cached initially
    assert!(cache.contains(&id), "should be cached initially");

    // Wait for expiration
    std::thread::sleep(Duration::from_millis(150));

    // Should be expired and evicted on access
    let result = cache.get(&id);
    assert!(
        result.is_none(),
        "should return None for expired entry"
    );
}

#[test]
fn test_ontology_cache_invalidation() {
    let cache = OntologyCache::with_defaults();
    let id = OntologyId::new("invalidate-me");

    cache.put(id.clone(), create_test_store(), None, None);
    assert!(cache.contains(&id), "should be cached");

    let invalidated = cache.invalidate(&id);
    assert!(invalidated, "invalidate should return true");
    assert!(!cache.contains(&id), "should be removed after invalidation");
}

#[test]
fn test_ontology_cache_clear() {
    let cache = OntologyCache::with_defaults();

    cache.put(OntologyId::new("ont1"), create_test_store(), None, None);
    cache.put(OntologyId::new("ont2"), create_test_store(), None, None);
    cache.put(OntologyId::new("ont3"), create_test_store(), None, None);

    let stats = cache.stats();
    assert_eq!(stats.entries, 3, "should have 3 entries");

    cache.clear();

    let stats = cache.stats();
    assert_eq!(stats.entries, 0, "should be empty after clear");
}

#[test]
fn test_ontology_cache_refresh_ttl() {
    let cache = OntologyCache::with_defaults();
    let id = OntologyId::new("refresh-test");

    cache.put(id.clone(), create_test_store(), None, None);

    std::thread::sleep(Duration::from_millis(10));

    let result = cache.refresh(&id, Some(Duration::from_secs(7200)));
    assert!(result.is_ok(), "refresh should succeed");

    let info = cache.get_info(&id).expect("info should exist");
    assert_eq!(info.ttl, Duration::from_secs(7200), "TTL should be updated");
}

#[test]
fn test_ontology_cache_hit_rate_calculation() {
    let cache = OntologyCache::with_defaults();
    let id = OntologyId::new("hit-rate-test");

    cache.put(id.clone(), create_test_store(), None, None);

    // 3 hits
    cache.get(&id);
    cache.get(&id);
    cache.get(&id);

    // 2 misses
    cache.get(&OntologyId::new("miss1"));
    cache.get(&OntologyId::new("miss2"));

    let stats = cache.stats();
    assert_eq!(stats.hits, 3, "should have 3 hits");
    assert_eq!(stats.misses, 2, "should have 2 misses");
    assert!(
        (stats.hit_rate - 0.6).abs() < 0.01,
        "hit rate should be ~60%"
    );
}

#[test]
fn test_ontology_cache_size_estimation() {
    let cache = OntologyCache::with_defaults();
    let id = OntologyId::new("size-test");

    let store = create_test_store();
    populate_test_store(&store, 100);

    cache.put(id.clone(), store, None, Some(20000));

    let stats = cache.stats();
    assert!(
        stats.total_size_bytes > 0,
        "total size should be tracked"
    );
}

#[test]
fn test_ontology_cache_concurrent_access() {
    use std::sync::Arc;
    use std::thread;

    let cache = Arc::new(OntologyCache::with_defaults());
    let id = OntologyId::new("concurrent-test");

    // Pre-populate
    cache.put(id.clone(), create_test_store(), None, None);

    let mut handles = vec![];

    // Spawn multiple threads reading from cache
    for i in 0..10 {
        let cache_clone = Arc::clone(&cache);
        let id_clone = id.clone();

        let handle = thread::spawn(move || {
            for _ in 0..100 {
                let result = cache_clone.get(&id_clone);
                assert!(result.is_some(), "thread {} should get cached value", i);
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().expect("thread should complete successfully");
    }

    let stats = cache.stats();
    assert_eq!(stats.hits, 1000, "should track all concurrent hits");
}

#[test]
fn test_ontology_cache_hit_rate_above_80_percent() {
    let cache = OntologyCache::with_defaults();

    // Cache multiple ontologies
    for i in 0..5 {
        let id = OntologyId::new(format!("ont{}", i));
        cache.put(id, create_test_store(), None, None);
    }

    // Access patterns: 9 hits, 1 miss
    for _ in 0..9 {
        for i in 0..5 {
            cache.get(&OntologyId::new(format!("ont{}", i)));
        }
    }
    cache.get(&OntologyId::new("nonexistent")); // 1 miss

    let stats = cache.stats();
    assert!(
        stats.hit_rate > 0.80,
        "hit rate should be >80% (actual: {:.2}%)",
        stats.hit_rate * 100.0
    );
}

// =============================================================================
// SPARQL Query Cache Tests
// =============================================================================

#[test]
fn test_query_cache_basic_operations() {
    let cache = QueryResultCache::default();
    let query = "SELECT ?s WHERE { ?s ?p ?o }";

    cache.put(query, vec![], None, vec![]);

    let result = cache.get(query);
    assert!(result.is_some(), "should retrieve cached query result");
}

#[test]
fn test_query_cache_fingerprinting_consistency() {
    let query = "SELECT ?s ?p ?o WHERE { ?s ?p ?o }";
    let fp1 = QueryResultCache::fingerprint(query);
    let fp2 = QueryResultCache::fingerprint(query);

    assert_eq!(fp1, fp2, "fingerprints should be consistent");
}

#[test]
fn test_query_cache_different_queries_different_fingerprints() {
    let query1 = "SELECT ?s WHERE { ?s ?p ?o }";
    let query2 = "SELECT ?p WHERE { ?s ?p ?o }";

    let fp1 = QueryResultCache::fingerprint(query1);
    let fp2 = QueryResultCache::fingerprint(query2);

    assert_ne!(fp1, fp2, "different queries should have different fingerprints");
}

#[test]
fn test_query_cache_ttl_expiration() {
    let config = CacheConfig {
        max_entries: 100,
        default_ttl: 1, // 1 second
        auto_evict: true,
        max_memory_bytes: 10_000_000,
    };
    let cache = QueryResultCache::new(config);
    let query = "SELECT ?x WHERE { ?x ?y ?z }";

    // Cache with very short TTL (override default)
    cache.put(query, vec![], Some(1), vec![]);

    // Should be cached initially
    assert!(cache.contains(query), "should be cached initially");

    // Wait for expiration
    std::thread::sleep(Duration::from_secs(2));

    // Should be expired
    let result = cache.get(query);
    assert!(result.is_none(), "should be None for expired entry");
}

#[test]
fn test_query_cache_invalidation_all() {
    let cache = QueryResultCache::default();

    cache.put("query1", vec![], None, vec![]);
    cache.put("query2", vec![], None, vec![]);
    cache.put("query3", vec![], None, vec![]);

    cache.invalidate(CacheInvalidationStrategy::All);

    assert!(!cache.contains("query1"), "query1 should be invalidated");
    assert!(!cache.contains("query2"), "query2 should be invalidated");
    assert!(!cache.contains("query3"), "query3 should be invalidated");
}

#[test]
fn test_query_cache_invalidation_by_query() {
    let cache = QueryResultCache::default();

    cache.put("query1", vec![], None, vec![]);
    cache.put("query2", vec![], None, vec![]);

    cache.invalidate(CacheInvalidationStrategy::ByQuery("query1".to_string()));

    assert!(!cache.contains("query1"), "query1 should be invalidated");
    assert!(cache.contains("query2"), "query2 should remain");
}

#[test]
fn test_query_cache_invalidation_by_tag() {
    let cache = QueryResultCache::default();

    cache.put("query1", vec![], None, vec!["tag1".to_string()]);
    cache.put("query2", vec![], None, vec!["tag1".to_string()]);
    cache.put("query3", vec![], None, vec!["tag2".to_string()]);

    cache.invalidate(CacheInvalidationStrategy::ByTag("tag1".to_string()));

    assert!(!cache.contains("query1"), "query1 should be invalidated");
    assert!(!cache.contains("query2"), "query2 should be invalidated");
    assert!(cache.contains("query3"), "query3 should remain (different tag)");
}

#[test]
fn test_query_cache_hit_rate_calculation() {
    let cache = QueryResultCache::default();
    let query = "SELECT ?s WHERE { ?s ?p ?o }";

    cache.put(query, vec![], None, vec![]);

    // 4 hits
    cache.get(query);
    cache.get(query);
    cache.get(query);
    cache.get(query);

    // 1 miss
    cache.get("nonexistent query");

    let stats = cache.stats();
    assert_eq!(stats.hits, 4, "should have 4 hits");
    assert_eq!(stats.misses, 1, "should have 1 miss");
    assert!(
        (stats.hit_rate - 0.8).abs() < 0.01,
        "hit rate should be ~80%"
    );
}

#[test]
fn test_query_cache_lru_eviction() {
    let config = CacheConfig {
        max_entries: 3,
        default_ttl: 300,
        auto_evict: true,
        max_memory_bytes: 10_000_000,
    };
    let cache = QueryResultCache::new(config);

    cache.put("query1", vec![], None, vec![]);
    cache.put("query2", vec![], None, vec![]);
    cache.put("query3", vec![], None, vec![]);

    // Access query1 to make it recently used
    cache.get("query1");

    // Add query4, should evict query2 (LRU)
    cache.put("query4", vec![], None, vec![]);

    assert!(cache.contains("query1"), "query1 should remain");
    assert!(!cache.contains("query2"), "query2 should be evicted");
    assert!(cache.contains("query3"), "query3 should remain");
    assert!(cache.contains("query4"), "query4 should be cached");
}

#[test]
fn test_query_cache_hit_rate_above_80_percent() {
    let cache = QueryResultCache::default();

    // Cache 5 queries
    for i in 0..5 {
        let query = format!("SELECT ?x WHERE {{ ?x ?y {} }}", i);
        cache.put(&query, vec![], None, vec![]);
    }

    // Access patterns: 9 rounds of hits per query (45 hits), then 5 misses
    for _ in 0..9 {
        for i in 0..5 {
            let query = format!("SELECT ?x WHERE {{ ?x ?y {} }}", i);
            cache.get(&query);
        }
    }

    // 5 misses
    for i in 10..15 {
        let query = format!("SELECT ?x WHERE {{ ?x ?y {} }}", i);
        cache.get(&query);
    }

    let stats = cache.stats();
    assert_eq!(stats.hits, 45, "should have 45 hits");
    assert_eq!(stats.misses, 5, "should have 5 misses");
    assert!(
        stats.hit_rate > 0.80,
        "hit rate should be >80% (actual: {:.2}%)",
        stats.hit_rate * 100.0
    );
}

// =============================================================================
// Integration Tests
// =============================================================================

#[test]
fn test_combined_cache_statistics() {
    let ontology_cache = OntologyCache::with_defaults();
    let query_cache = QueryResultCache::default();

    // Populate ontology cache
    for i in 0..3 {
        let id = OntologyId::new(format!("ont{}", i));
        ontology_cache.put(id, create_test_store(), None, None);
    }

    // Populate query cache
    for i in 0..5 {
        let query = format!("SELECT ?x WHERE {{ ?x ?y {} }}", i);
        query_cache.put(&query, vec![], None, vec![]);
    }

    // Access patterns
    for i in 0..3 {
        ontology_cache.get(&OntologyId::new(format!("ont{}", i)));
    }

    for i in 0..5 {
        let query = format!("SELECT ?x WHERE {{ ?x ?y {} }}", i);
        query_cache.get(&query);
    }

    let ont_stats = ontology_cache.stats();
    let query_stats = query_cache.stats();

    assert_eq!(ont_stats.entries, 3, "ontology cache should have 3 entries");
    assert_eq!(query_stats.entries, 5, "query cache should have 5 entries");
    assert_eq!(ont_stats.hits, 3, "ontology cache should have 3 hits");
    assert_eq!(query_stats.hits, 5, "query cache should have 5 hits");
}

#[test]
fn test_cache_performance_stress() {
    let cache = QueryResultCache::default();

    // Stress test: cache 100 queries
    for i in 0..100 {
        let query = format!("SELECT ?x WHERE {{ ?x ?y {} }}", i);
        cache.put(&query, vec![], None, vec![]);
    }

    // Access each query 10 times
    for _ in 0..10 {
        for i in 0..100 {
            let query = format!("SELECT ?x WHERE {{ ?x ?y {} }}", i);
            let result = cache.get(&query);
            assert!(result.is_some(), "query {} should be cached", i);
        }
    }

    let stats = cache.stats();
    assert_eq!(stats.hits, 1000, "should have 1000 hits");
    assert_eq!(stats.hit_rate, 1.0, "hit rate should be 100%");
}

#[test]
fn test_cache_maintenance() {
    let config = OntologyCacheConfig {
        max_entries: 10,
        default_ttl_secs: 1,
        auto_evict_expired: true,
    };
    let cache = OntologyCache::new(config);

    // Add entries with short TTL
    for i in 0..5 {
        let id = OntologyId::new(format!("ont{}", i));
        cache.put(
            id,
            create_test_store(),
            Some(Duration::from_millis(100)),
            None,
        );
    }

    let stats = cache.stats();
    assert_eq!(stats.entries, 5, "should have 5 entries");

    // Wait for expiration
    std::thread::sleep(Duration::from_millis(150));

    // Perform maintenance
    let evicted = cache.maintain();
    assert_eq!(evicted, 5, "should evict all 5 expired entries");

    let stats = cache.stats();
    assert_eq!(stats.entries, 0, "cache should be empty after maintenance");
}
