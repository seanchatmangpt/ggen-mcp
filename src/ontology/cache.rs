//! Ontology and Query Result Caching
//!
//! LRU caching for loaded ontologies and SPARQL query results.
//! Thread-safe with parking_lot RwLock. Atomic counters for metrics.

use crate::tools::ontology_sparql::{ExecuteSparqlQueryResponse, OntologyId, QueryCacheKey};
use anyhow::{Context, Result};
use ggen_ontology_core::TripleStore;
use lru::LruCache;
use parking_lot::RwLock;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Ontology cache with LRU eviction
pub struct OntologyCache {
    /// Cache storage (RwLock for concurrent reads)
    cache: RwLock<LruCache<OntologyId, Arc<TripleStore>>>,
    /// Cache hits
    hits: AtomicU64,
    /// Cache misses
    misses: AtomicU64,
}

impl OntologyCache {
    /// Create new ontology cache with capacity
    pub fn new(capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity.max(1)).unwrap();
        Self {
            cache: RwLock::new(LruCache::new(capacity)),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Insert ontology into cache
    pub fn insert(&self, id: OntologyId, store: TripleStore) {
        let mut cache = self.cache.write();
        cache.put(id, Arc::new(store));
    }

    /// Get ontology from cache
    pub fn get(&self, id: &OntologyId) -> Option<Arc<TripleStore>> {
        let mut cache = self.cache.write();
        if let Some(store) = cache.get(id) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(store.clone())
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.read();
        CacheStats {
            size: cache.len(),
            capacity: cache.cap().get(),
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
        }
    }

    /// Clear cache
    pub fn clear(&self) {
        let mut cache = self.cache.write();
        cache.clear();
    }
}

/// Query result cache with LRU eviction
pub struct QueryCache {
    /// Cache storage (RwLock for concurrent reads)
    cache: RwLock<LruCache<QueryCacheKey, ExecuteSparqlQueryResponse>>,
    /// Cache hits
    hits: AtomicU64,
    /// Cache misses
    misses: AtomicU64,
}

impl QueryCache {
    /// Create new query cache with capacity
    pub fn new(capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity.max(1)).unwrap();
        Self {
            cache: RwLock::new(LruCache::new(capacity)),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Insert query result into cache
    pub fn insert(&self, key: QueryCacheKey, response: ExecuteSparqlQueryResponse) {
        let mut cache = self.cache.write();
        cache.put(key, response);
    }

    /// Get query result from cache
    pub fn get(&self, key: &QueryCacheKey) -> Option<ExecuteSparqlQueryResponse> {
        let mut cache = self.cache.write();
        if let Some(response) = cache.get(key) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            // Mark as cached
            let mut cached_response = response.clone();
            cached_response.from_cache = true;
            Some(cached_response)
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.read();
        CacheStats {
            size: cache.len(),
            capacity: cache.cap().get(),
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
        }
    }

    /// Clear cache
    pub fn clear(&self) {
        let mut cache = self.cache.write();
        cache.clear();
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
    pub hits: u64,
    pub misses: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ontology_sparql::{QueryPerformance, QueryResult};

    #[test]
    fn test_ontology_cache() {
        let cache = OntologyCache::new(2);
        let id1 = OntologyId::new("test1");
        let id2 = OntologyId::new("test2");
        let id3 = OntologyId::new("test3");

        let store1 = Store::new().unwrap();
        let store2 = Store::new().unwrap();
        let store3 = Store::new().unwrap();

        cache.insert(id1.clone(), store1);
        cache.insert(id2.clone(), store2);

        assert!(cache.get(&id1).is_some());
        assert!(cache.get(&id2).is_some());

        // Insert third item, should evict first
        cache.insert(id3.clone(), store3);

        let stats = cache.stats();
        assert_eq!(stats.size, 2);
        assert_eq!(stats.capacity, 2);
    }

    #[test]
    fn test_query_cache() {
        let cache = QueryCache::new(2);
        let key1 = QueryCacheKey::new("SELECT * WHERE { ?s ?p ?o }");
        let key2 = QueryCacheKey::new("SELECT * WHERE { ?x ?y ?z }");

        let response1 = ExecuteSparqlQueryResponse {
            cache_key: key1.clone(),
            result: QueryResult::Ask { result: true },
            performance: QueryPerformance {
                execution_time_ms: 100,
                result_count: 1,
                complexity_score: 10.0,
            },
            from_cache: false,
        };

        cache.insert(key1.clone(), response1.clone());

        let cached = cache.get(&key1).unwrap();
        assert!(cached.from_cache);

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
    }
}
