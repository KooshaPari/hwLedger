//! Data Transfer Objects for the cache adapter.
//!
//! DTOs are used to transfer data between layers and external systems.
//! They should be simple, serializable structures.

/// Cache metrics snapshot for observability.
#[derive(Debug, Clone, Default)]
pub struct CacheMetricsDto {
    pub l1_hits: u64,
    pub l2_hits: u64,
    pub misses: u64,
    pub promotions: u64,
    pub evictions: u64,
    pub expirations: u64,
}

impl CacheMetricsDto {
    /// Total number of hits (L1 + L2).
    pub fn total_hits(&self) -> u64 {
        self.l1_hits + self.l2_hits
    }

    /// Total number of requests.
    pub fn total_requests(&self) -> u64 {
        self.total_hits() + self.misses
    }

    /// Hit rate as a percentage (0.0 - 100.0).
    pub fn hit_rate_percent(&self) -> f64 {
        let total = self.total_requests();
        if total == 0 {
            0.0
        } else {
            (self.total_hits() as f64 / total as f64) * 100.0
        }
    }
}

/// Request DTO for cache operations.
#[derive(Debug, Clone)]
pub struct CacheRequest<K, V> {
    pub key: K,
    pub value: Option<V>,
    pub ttl_secs: Option<u64>,
}

/// Response DTO for cache get operations.
#[derive(Debug, Clone)]
pub struct CacheResponse<V> {
    pub hit: bool,
    pub value: Option<V>,
    pub from_l1: bool,
}

/// Batch operation result.
#[derive(Debug, Clone)]
pub struct BatchResult {
    pub successful: usize,
    pub failed: usize,
}
