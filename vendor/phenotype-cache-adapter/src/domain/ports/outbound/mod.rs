//! Outbound ports - Interfaces that the domain uses to communicate with external systems.
//!
//! These define what the domain needs from infrastructure:
//! - MetricsCollector: For recording cache statistics
//! - EntryStore: For persisting cache entries

/// Metrics collector interface for observability.
/// Allows plugging in different metrics backends (Prometheus, StatsD, etc.)
pub trait MetricsCollector: Send + Sync {
    fn record_l1_hit(&self);
    fn record_l2_hit(&self);
    fn record_miss(&self);
    fn record_promotion(&self);
    fn record_eviction(&self);
    fn record_expiration(&self);
}

/// Storage interface for cache entries.
/// Abstraction over in-memory or persistent storage.
pub trait EntryStore<K, V>: Send + Sync
where
    K: Send + Sync,
    V: Clone + Send + Sync,
{
    fn get(&self, key: &K) -> Option<CacheEntry<V>>;
    fn insert(&self, key: K, entry: CacheEntry<V>);
    fn remove(&self, key: &K);
    fn clear(&self);
    fn contains(&self, key: &K) -> Option<bool>;
}

// Re-export CacheEntry for use in the trait
use crate::domain::entities::CacheEntry;
