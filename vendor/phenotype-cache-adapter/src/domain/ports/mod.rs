//! Domain ports - Interfaces that define boundaries of the domain.
//!
//! Ports are the way the domain defines contracts without knowing
//! about the implementation details (adapters).
//!
//! ## Port Types
//! - **Inbound Ports**: What the domain offers to the outside world
//! - **Outbound Ports**: What the domain needs from external systems

pub mod inbound;
pub mod outbound;

pub use inbound::CacheService;
pub use outbound::{MetricsCollector, EntryStore};
