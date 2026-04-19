//! Domain layer - Pure business logic with no external dependencies.
//!
//! This layer contains:
//! - Entities: Core domain objects (CacheEntry, TieredCache)
//! - Value Objects: Immutable types (CacheMetrics)
//! - Ports: Interfaces for inbound and outbound communication
//! - Domain Services: Pure domain logic
//!
//! ## Dependency Rule
//! Domain MUST NOT depend on application or adapters layers.
//! Only external traits (std library) are allowed.

pub mod entities;
pub mod ports;
pub mod services;
