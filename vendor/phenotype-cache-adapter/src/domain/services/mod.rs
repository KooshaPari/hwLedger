//! Domain services - Pure domain logic without side effects.
//!
//! These services contain business logic that doesn't belong to a single entity
//! or that coordinates multiple entities/ports.

use crate::domain::entities::CacheConfig;

/// Cache configuration builder for creating valid configurations.
pub struct CacheConfigBuilder {
    l1_capacity: usize,
    default_ttl_secs: u64,
}

impl CacheConfigBuilder {
    pub fn new() -> Self {
        Self {
            l1_capacity: 1000,
            default_ttl_secs: 3600,
        }
    }

    pub fn l1_capacity(mut self, capacity: usize) -> Self {
        self.l1_capacity = capacity.max(1);
        self
    }

    pub fn default_ttl_secs(mut self, ttl: u64) -> Self {
        self.default_ttl_secs = ttl;
        self
    }

    pub fn build(self) -> CacheConfig {
        CacheConfig::new(self.l1_capacity, self.default_ttl_secs)
    }
}

impl Default for CacheConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Domain service for calculating cache statistics.
pub struct CacheStatsCalculator;

impl CacheStatsCalculator {
    /// Calculate hit rate from metrics.
    pub fn hit_rate(hits: u64, misses: u64) -> f64 {
        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    /// Calculate promotion rate from L2 hits and promotions.
    pub fn promotion_rate(l2_hits: u64, promotions: u64) -> f64 {
        if l2_hits == 0 {
            0.0
        } else {
            promotions as f64 / l2_hits as f64
        }
    }
}

/// Domain service for TTL validation.
pub struct TtlValidator;

impl TtlValidator {
    /// Validate that TTL is within acceptable bounds.
    pub fn is_valid(ttl_secs: u64) -> bool {
        // Reasonable bounds: 1 second to 30 days
        ttl_secs >= 1 && ttl_secs <= 2_592_000
    }

    /// Normalize TTL to acceptable bounds.
    pub fn normalize(ttl_secs: u64) -> u64 {
        ttl_secs.clamp(1, 2_592_000)
    }
}
