//! Inbound ports - Interfaces that the domain exposes to callers.
//!
//! These define what operations are available on the cache from
//! the perspective of the application layer or external clients.

use crate::application::dto::CacheMetricsDto;

/// Cache service interface - the primary port for cache operations.
/// This is the main interface that clients (application layer, adapters) use.
pub trait CacheService<K, V>: Send + Sync
where
    K: Send + Sync,
    V: Clone + Send + Sync,
{
    /// Get a value from the cache by key.
    fn get(&self, key: &K) -> Option<V>;

    /// Insert a value with default TTL.
    fn insert(&self, key: K, value: V);

    /// Insert a value with custom TTL in seconds.
    fn insert_with_ttl(&self, key: K, value: V, ttl_secs: u64);

    /// Remove a key from the cache.
    fn remove(&self, key: &K);

    /// Clear all entries from the cache.
    fn clear(&self);

    /// Check if the cache contains a key.
    fn contains(&self, key: &K) -> bool;

    /// Get current cache metrics.
    fn metrics(&self) -> CacheMetricsDto;

    /// Get the number of entries in L1 (hot cache).
    fn l1_len(&self) -> usize;

    /// Get the number of entries in L2 (warm cache).
    fn l2_len(&self) -> usize;
}
