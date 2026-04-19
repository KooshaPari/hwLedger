//! Default metrics collection implementations.

use crate::domain::ports::outbound::MetricsCollector;
use std::sync::atomic::{AtomicU64, Ordering};

/// No-op metrics collector that does nothing.
/// Default for production when metrics are not needed.
pub struct NoopMetricsCollector;

impl MetricsCollector for NoopMetricsCollector {
    #[inline]
    fn record_l1_hit(&self) {}

    #[inline]
    fn record_l2_hit(&self) {}

    #[inline]
    fn record_miss(&self) {}

    #[inline]
    fn record_promotion(&self) {}

    #[inline]
    fn record_eviction(&self) {}

    #[inline]
    fn record_expiration(&self) {}
}

/// Atomic metrics collector for testing and observability.
pub struct AtomicMetricsCollector {
    l1_hits: AtomicU64,
    l2_hits: AtomicU64,
    misses: AtomicU64,
    promotions: AtomicU64,
    evictions: AtomicU64,
    expirations: AtomicU64,
}

impl AtomicMetricsCollector {
    pub fn new() -> Self {
        Self {
            l1_hits: AtomicU64::new(0),
            l2_hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            promotions: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            expirations: AtomicU64::new(0),
        }
    }

    pub fn l1_hits(&self) -> u64 {
        self.l1_hits.load(Ordering::Relaxed)
    }

    pub fn l2_hits(&self) -> u64 {
        self.l2_hits.load(Ordering::Relaxed)
    }

    pub fn misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    pub fn promotions(&self) -> u64 {
        self.promotions.load(Ordering::Relaxed)
    }

    pub fn evictions(&self) -> u64 {
        self.evictions.load(Ordering::Relaxed)
    }

    pub fn expirations(&self) -> u64 {
        self.expirations.load(Ordering::Relaxed)
    }

    pub fn reset(&self) {
        self.l1_hits.store(0, Ordering::Relaxed);
        self.l2_hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.promotions.store(0, Ordering::Relaxed);
        self.evictions.store(0, Ordering::Relaxed);
        self.expirations.store(0, Ordering::Relaxed);
    }
}

impl Default for AtomicMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector for AtomicMetricsCollector {
    #[inline]
    fn record_l1_hit(&self) {
        self.l1_hits.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    fn record_l2_hit(&self) {
        self.l2_hits.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    fn record_promotion(&self) {
        self.promotions.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    fn record_eviction(&self) {
        self.evictions.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    fn record_expiration(&self) {
        self.expirations.fetch_add(1, Ordering::Relaxed);
    }
}
