//! Core data types and math for hwLedger capacity planning.
//!
//! Implements: FR-PLAN-002, FR-PLAN-003
//!
//! See `PLAN.md` §5 and ADR-0004 for the architecture-keyed dispatch contract.

pub mod math;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
