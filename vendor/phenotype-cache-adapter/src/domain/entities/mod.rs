//! Domain entities for the cache adapter.
//!
//! These are pure domain objects with no external framework dependencies.

use std::time::Instant;
use std::hash::{Hash, Hasher};

/// Cache entry with TTL support.
/// Value object - immutable after creation.
#[derive(Debug, Clone)]
pub struct CacheEntry<V> {
    pub value: V,
    pub expires_at: Instant,
}

impl<V: PartialEq> PartialEq for CacheEntry<V> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<V: Eq> Eq for CacheEntry<V> {}

impl<V: Hash> Hash for CacheEntry<V> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<V> CacheEntry<V> {
    /// Create a new cache entry with TTL.
    pub fn new(value: V, ttl: std::time::Duration) -> Self {
        Self {
            value,
            expires_at: Instant::now() + ttl,
        }
    }

    /// Check if the entry has expired.
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    /// Get remaining TTL in seconds.
    pub fn remaining_ttl_secs(&self) -> Option<u64> {
        let remaining = self.expires_at.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            None
        } else {
            Some(remaining.as_secs())
        }
    }
}

/// Cache configuration for creating cache instances.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub l1_capacity: usize,
    pub default_ttl_secs: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            l1_capacity: 1000,
            default_ttl_secs: 3600,
        }
    }
}

impl CacheConfig {
    pub fn new(l1_capacity: usize, default_ttl_secs: u64) -> Self {
        Self {
            l1_capacity: l1_capacity.max(1),
            default_ttl_secs,
        }
    }
}
