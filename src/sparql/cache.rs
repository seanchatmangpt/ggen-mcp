// =============================================================================
// SPARQL Query Result Cache
// =============================================================================
// Cache validated query results with TTL, memory bounds, and invalidation strategies

use anyhow::{Result, anyhow};
use chrono::{DateTime, Duration, Utc};
use lru::LruCache;
use oxigraph::sparql::QuerySolution;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::Arc;

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of cached query results
    pub max_entries: usize,
    /// Default TTL for cached results (in seconds)
    pub default_ttl: i64,
    /// Enable automatic eviction based on TTL
    pub auto_evict: bool,
    /// Maximum memory size in bytes (approximate)
    pub max_memory_bytes: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            default_ttl: 300, // 5 minutes
            auto_evict: true,
            max_memory_bytes: 100 * 1024 * 1024, // 100 MB
        }
    }
}

/// Cache invalidation strategy
#[derive(Debug, Clone, PartialEq)]
pub enum CacheInvalidationStrategy {
    /// Invalidate all cached results
    All,
    /// Invalidate by query fingerprint
    ByQuery(String),
    /// Invalidate by prefix (for related queries)
    ByPrefix(String),
    /// Invalidate by tag
    ByTag(String),
    /// Invalidate expired entries only
    Expired,
    /// Custom predicate
    Custom,
}

/// Cached query result entry
#[derive(Debug, Clone)]
struct CacheEntry {
    /// Query fingerprint (hash)
    fingerprint: String,
    /// Cached solutions (stored as serialized JSON since QuerySolution doesn't implement Clone)
    solutions_json: String,
    /// Timestamp when cached
    cached_at: DateTime<Utc>,
    /// Time-to-live in seconds
    ttl: i64,
    /// Optional tags for invalidation
    tags: Vec<String>,
    /// Approximate size in bytes
    size_bytes: usize,
}

impl CacheEntry {
    /// Check if entry is expired
    fn is_expired(&self) -> bool {
        let expiry = self.cached_at + Duration::seconds(self.ttl);
        Utc::now() > expiry
    }

    /// Get remaining TTL in seconds
    fn remaining_ttl(&self) -> i64 {
        let expiry = self.cached_at + Duration::seconds(self.ttl);
        let remaining = expiry - Utc::now();
        remaining.num_seconds().max(0)
    }
}

/// Query result cache with type information
///
/// Features:
/// - LRU eviction policy
/// - TTL-based expiration
/// - Query fingerprinting
/// - Memory bounds
/// - Tag-based invalidation
pub struct QueryResultCache {
    config: CacheConfig,
    cache: Arc<RwLock<LruCache<String, CacheEntry>>>,
    total_size: Arc<RwLock<usize>>,
    /// Statistics
    hits: Arc<RwLock<u64>>,
    misses: Arc<RwLock<u64>>,
    evictions: Arc<RwLock<u64>>,
}

impl QueryResultCache {
    /// Create a new query result cache
    ///
    /// # Errors
    /// Returns `Err` if `config.max_entries` is 0 (invalid configuration)
    pub fn new(config: CacheConfig) -> Result<Self> {
        // TPS: Fail fast on invalid config - no fallback to default capacity
        let capacity = NonZeroUsize::new(config.max_entries)
            .ok_or_else(|| anyhow!("Cache max_entries must be > 0, got {}", config.max_entries))?;

        Ok(Self {
            config,
            cache: Arc::new(RwLock::new(LruCache::new(capacity))),
            total_size: Arc::new(RwLock::new(0)),
            hits: Arc::new(RwLock::new(0)),
            misses: Arc::new(RwLock::new(0)),
            evictions: Arc::new(RwLock::new(0)),
        })
    }

    /// Create cache with default configuration
    ///
    /// # Panics
    /// Panics if default configuration is invalid (should never happen - programming error)
    pub fn default() -> Self {
        // Default config has max_entries: 1000, so this should never fail
        // If it does, it's a programming error in CacheConfig::default()
        Self::new(CacheConfig::default())
            .expect("Default cache configuration is invalid - this is a programming error")
    }

    /// Generate fingerprint for a query using ahash (5-10x faster than SHA256)
    /// This is safe for cache keys as we don't need cryptographic security
    #[inline]
    pub fn fingerprint(query: &str) -> String {
        let mut hasher = ahash::AHasher::default();
        query.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Store query results in cache
    pub fn put(
        &self,
        query: &str,
        solutions: Vec<QuerySolution>,
        ttl: Option<i64>,
        tags: Vec<String>,
    ) {
        let fingerprint = Self::fingerprint(query);
        let size_bytes = self.estimate_size(&solutions);

        // Check if adding this would exceed memory limit
        let current_size = *self.total_size.read();
        if current_size + size_bytes > self.config.max_memory_bytes {
            // Evict entries to make room
            self.evict_to_fit(size_bytes);
        }

        let entry = CacheEntry {
            fingerprint: fingerprint.clone(),
            solutions_json: serde_json::to_string(&solutions.iter().map(|s| {
                let mut row = serde_json::Map::new();
                for (var, term) in s.iter() {
                    row.insert(var.as_str().to_string(), serde_json::json!(term.to_string()));
                }
                serde_json::Value::Object(row)
            }).collect::<Vec<_>>()).unwrap_or_default(),
            cached_at: Utc::now(),
            ttl: ttl.unwrap_or(self.config.default_ttl),
            tags,
            size_bytes,
        };

        let mut cache = self.cache.write();

        // If evicting an old entry, update size
        if let Some(old_entry) = cache.put(fingerprint, entry) {
            let mut total_size = self.total_size.write();
            *total_size = total_size.saturating_sub(old_entry.size_bytes);
            *self.evictions.write() += 1;
        }

        // Update total size
        *self.total_size.write() += size_bytes;
    }

    /// Get cached query results
    pub fn get(&self, query: &str) -> Option<Vec<QuerySolution>> {
        let fingerprint = Self::fingerprint(query);
        let mut cache = self.cache.write();

        if let Some(entry) = cache.get(&fingerprint) {
            // Check if expired
            if self.config.auto_evict && entry.is_expired() {
                cache.pop(&fingerprint);
                *self.misses.write() += 1;
                return None;
            }

            *self.hits.write() += 1;
            // Deserialize solutions from JSON
            serde_json::from_str(&entry.solutions_json).ok()
        } else {
            *self.misses.write() += 1;
            None
        }
    }

    /// Check if query is cached and valid
    pub fn contains(&self, query: &str) -> bool {
        let fingerprint = Self::fingerprint(query);
        let cache = self.cache.read();

        if let Some(entry) = cache.peek(&fingerprint) {
            !self.config.auto_evict || !entry.is_expired()
        } else {
            false
        }
    }

    /// Invalidate cache entries based on strategy
    pub fn invalidate(&self, strategy: CacheInvalidationStrategy) {
        let mut cache = self.cache.write();
        let mut total_size = self.total_size.write();

        match strategy {
            CacheInvalidationStrategy::All => {
                cache.clear();
                *total_size = 0;
            }
            CacheInvalidationStrategy::ByQuery(query) => {
                let fingerprint = Self::fingerprint(&query);
                if let Some(entry) = cache.pop(&fingerprint) {
                    *total_size = total_size.saturating_sub(entry.size_bytes);
                }
            }
            CacheInvalidationStrategy::ByPrefix(prefix) => {
                let to_remove: Vec<String> = cache
                    .iter()
                    .filter(|(fp, _)| fp.starts_with(&prefix))
                    .map(|(fp, _)| fp.clone())
                    .collect();

                for fp in to_remove {
                    if let Some(entry) = cache.pop(&fp) {
                        *total_size = total_size.saturating_sub(entry.size_bytes);
                    }
                }
            }
            CacheInvalidationStrategy::ByTag(tag) => {
                let to_remove: Vec<String> = cache
                    .iter()
                    .filter(|(_, entry)| entry.tags.contains(&tag))
                    .map(|(fp, _)| fp.clone())
                    .collect();

                for fp in to_remove {
                    if let Some(entry) = cache.pop(&fp) {
                        *total_size = total_size.saturating_sub(entry.size_bytes);
                    }
                }
            }
            CacheInvalidationStrategy::Expired => {
                let to_remove: Vec<String> = cache
                    .iter()
                    .filter(|(_, entry)| entry.is_expired())
                    .map(|(fp, _)| fp.clone())
                    .collect();

                for fp in to_remove {
                    if let Some(entry) = cache.pop(&fp) {
                        *total_size = total_size.saturating_sub(entry.size_bytes);
                    }
                }
            }
            CacheInvalidationStrategy::Custom => {
                // For custom strategies, caller should use invalidate_if
            }
        }
    }

    /// Invalidate entries matching a predicate
    pub fn invalidate_if<F>(&self, predicate: F)
    where
        F: Fn(&str, &CacheEntry) -> bool,
    {
        let mut cache = self.cache.write();
        let mut total_size = self.total_size.write();

        let to_remove: Vec<String> = cache
            .iter()
            .filter(|(fp, entry)| predicate(fp, entry))
            .map(|(fp, _)| fp.clone())
            .collect();

        for fp in to_remove {
            if let Some(entry) = cache.pop(&fp) {
                *total_size = total_size.saturating_sub(entry.size_bytes);
            }
        }
    }

    /// Evict entries to fit new entry of given size
    fn evict_to_fit(&self, new_size: usize) {
        let mut cache = self.cache.write();
        let mut total_size = self.total_size.write();

        while *total_size + new_size > self.config.max_memory_bytes {
            // LRU will evict the least recently used
            if let Some((_, entry)) = cache.pop_lru() {
                *total_size = total_size.saturating_sub(entry.size_bytes);
                *self.evictions.write() += 1;
            } else {
                break; // Cache is empty
            }
        }
    }

    /// Estimate size of solutions in bytes (rough approximation)
    fn estimate_size(&self, solutions: &[QuerySolution]) -> usize {
        // Rough estimate: 100 bytes per solution + variable data
        let base_size = solutions.len() * 100;

        let variable_size: usize = solutions
            .iter()
            .map(|sol| {
                sol.variables()
                    .iter()
                    .map(|var| var.as_str().len())
                    .sum::<usize>()
                    + 50 // estimate for term data
            })
            .sum();

        base_size + variable_size
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.read();
        let hits = *self.hits.read();
        let misses = *self.misses.read();
        let total_requests = hits + misses;
        let hit_rate = if total_requests > 0 {
            hits as f64 / total_requests as f64
        } else {
            0.0
        };

        CacheStats {
            entries: cache.len(),
            total_size_bytes: *self.total_size.read(),
            hits,
            misses,
            hit_rate,
            evictions: *self.evictions.read(),
        }
    }

    /// Clear all statistics
    pub fn clear_stats(&self) {
        *self.hits.write() = 0;
        *self.misses.write() = 0;
        *self.evictions.write() = 0;
    }

    /// Get information about a specific cached query
    pub fn get_info(&self, query: &str) -> Option<CachedQueryInfo> {
        let fingerprint = Self::fingerprint(query);
        let cache = self.cache.read();

        cache.peek(&fingerprint).map(|entry| CachedQueryInfo {
            fingerprint: entry.fingerprint.clone(),
            cached_at: entry.cached_at,
            ttl: entry.ttl,
            remaining_ttl: entry.remaining_ttl(),
            size_bytes: entry.size_bytes,
            result_count: entry.solutions.len(),
            tags: entry.tags.clone(),
            is_expired: entry.is_expired(),
        })
    }

    /// Refresh TTL for a cached query
    pub fn refresh(&self, query: &str, new_ttl: Option<i64>) {
        let fingerprint = Self::fingerprint(query);
        let mut cache = self.cache.write();

        if let Some(entry) = cache.get_mut(&fingerprint) {
            entry.cached_at = Utc::now();
            if let Some(ttl) = new_ttl {
                entry.ttl = ttl;
            }
        }
    }

    /// Perform cache maintenance (remove expired entries)
    pub fn maintain(&self) {
        if self.config.auto_evict {
            self.invalidate(CacheInvalidationStrategy::Expired);
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub total_size_bytes: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub evictions: u64,
}

/// Information about a cached query
#[derive(Debug, Clone)]
pub struct CachedQueryInfo {
    pub fingerprint: String,
    pub cached_at: DateTime<Utc>,
    pub ttl: i64,
    pub remaining_ttl: i64,
    pub size_bytes: usize,
    pub result_count: usize,
    pub tags: Vec<String>,
    pub is_expired: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_consistency() {
        let query = "SELECT ?s ?p ?o WHERE { ?s ?p ?o }";
        let fp1 = QueryResultCache::fingerprint(query);
        let fp2 = QueryResultCache::fingerprint(query);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_cache_put_get() {
        let cache = QueryResultCache::default();
        let query = "SELECT ?name WHERE { ?s ?p ?name }";
        let solutions = vec![]; // Empty for test

        cache.put(query, solutions.clone(), None, vec![]);

        let cached = cache.get(query);
        assert!(cached.is_some());
    }

    #[test]
    fn test_cache_invalidation_all() {
        let cache = QueryResultCache::default();
        cache.put("query1", vec![], None, vec![]);
        cache.put("query2", vec![], None, vec![]);

        cache.invalidate(CacheInvalidationStrategy::All);

        assert!(!cache.contains("query1"));
        assert!(!cache.contains("query2"));
    }

    #[test]
    fn test_cache_stats() {
        let cache = QueryResultCache::default();
        cache.put("query", vec![], None, vec![]);

        cache.get("query"); // hit
        cache.get("missing"); // miss

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert!(stats.hit_rate > 0.0);
    }
}
