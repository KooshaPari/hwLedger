//! Outbound adapters - Infrastructure implementations.
//!
//! These adapters implement the outbound ports defined in the domain:
//! - EntryStore: In-memory tier implementations
//! - MetricsCollector: Metrics collection implementations

pub mod memory;
pub mod metrics;

pub use memory::in_memory::{InMemoryEntryStore, InMemoryTier};
pub use metrics::default::{NoopMetricsCollector, AtomicMetricsCollector};
