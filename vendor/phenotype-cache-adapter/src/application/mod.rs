//! Application layer - Coordinates domain objects and orchestrates use cases.
//!
//! This layer contains:
//! - Use Cases: Application-specific business rules
//! - DTOs: Data transfer objects for crossing layer boundaries
//! - Command/Query handlers
//!
//! ## Dependency Rule
//! Application depends on Domain but NOT on Adapters.

pub mod dto;
pub mod use_cases;
