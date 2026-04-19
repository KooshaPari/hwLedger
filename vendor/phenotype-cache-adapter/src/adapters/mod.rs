//! Adapters layer - Infrastructure implementations that satisfy domain ports.
//!
//! This layer contains:
//! - Inbound Adapters: HTTP handlers, CLI commands, UI controllers
//! - Outbound Adapters: Database, cache, external service clients
//!
//! ## Dependency Rule
//! Adapters depend on Domain (implementing ports) but Domain
//! does NOT depend on Adapters.

pub mod outbound;
