//! Application use cases for cache operations.
//!
//! Use cases encapsulate application-specific business rules
//! and coordinate between ports.

use crate::domain::ports::CacheService;
use crate::application::dto::{CacheMetricsDto, CacheResponse};

/// Use case: Get value from cache.
pub struct GetFromCache<K, V, S>
where
    K: Clone + std::hash::Hash + Eq + Send + Sync,
    V: Clone + Send + Sync,
    S: CacheService<K, V>,
    K: Clone + std::hash::Hash + Eq + Send + Sync,
    V: Clone + Send + Sync,
{
    cache: S,
    _phantom: std::marker::PhantomData<(K, V)>,
}

impl<K, V, S> GetFromCache<K, V, S>
where
    K: Clone + std::hash::Hash + Eq + Send + Sync,
    V: Clone + Send + Sync,
    S: CacheService<K, V>,
{
    pub fn new(cache: S) -> Self {
        Self {
            cache,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn execute(&self, key: &K) -> CacheResponse<V> {
        let value = self.cache.get(key);
        CacheResponse {
            hit: value.is_some(),
            value,
            from_l1: true, // Simplified; actual implementation checks L1 first
        }
    }
}

/// Use case: Insert value into cache.
pub struct InsertIntoCache<K, V, S>
where
    K: Clone + std::hash::Hash + Eq + Send + Sync,
    V: Clone + Send + Sync,
    S: CacheService<K, V>,
    K: Clone + std::hash::Hash + Eq + Send + Sync,
    V: Clone + Send + Sync,
{
    cache: S,
    _phantom: std::marker::PhantomData<(K, V)>,
}

impl<K, V, S> InsertIntoCache<K, V, S>
where
    K: Clone + std::hash::Hash + Eq + Send + Sync,
    V: Clone + Send + Sync,
    S: CacheService<K, V>,
{
    pub fn new(cache: S) -> Self {
        Self {
            cache,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn execute(&self, key: K, value: V) {
        self.cache.insert(key, value);
    }

    pub fn execute_with_ttl(&self, key: K, value: V, ttl_secs: u64) {
        self.cache.insert_with_ttl(key, value, ttl_secs);
    }
}

/// Use case: Remove value from cache.
pub struct RemoveFromCache<K, V, S>
where
    K: Clone + std::hash::Hash + Eq + Send + Sync,
    V: Clone + Send + Sync,
    S: CacheService<K, V>,
    K: Clone + std::hash::Hash + Eq + Send + Sync,
    V: Clone + Send + Sync,
{
    cache: S,
    _phantom: std::marker::PhantomData<(K, V)>,
}

impl<K, V, S> RemoveFromCache<K, V, S>
where
    K: Clone + std::hash::Hash + Eq + Send + Sync,
    V: Clone + Send + Sync,
    S: CacheService<K, V>,
{
    pub fn new(cache: S) -> Self {
        Self {
            cache,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn execute(&self, key: &K) {
        self.cache.remove(key);
    }
}

/// Use case: Get cache metrics.
pub struct GetCacheMetrics<K, V, S>
where
    K: Clone + std::hash::Hash + Eq + Send + Sync,
    V: Clone + Send + Sync,
    S: CacheService<K, V>,
    K: Clone + std::hash::Hash + Eq + Send + Sync,
    V: Clone + Send + Sync,
{
    cache: S,
    _phantom: std::marker::PhantomData<(K, V)>,
}

impl<K, V, S> GetCacheMetrics<K, V, S>
where
    K: Clone + std::hash::Hash + Eq + Send + Sync,
    V: Clone + Send + Sync,
    S: CacheService<K, V>,
{
    pub fn new(cache: S) -> Self {
        Self {
            cache,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn execute(&self) -> CacheMetricsDto {
        self.cache.metrics()
    }
}
