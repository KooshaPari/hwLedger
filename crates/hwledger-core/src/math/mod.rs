//! Math core for capacity planning.
//!
//! See `PLAN.md` §5 and ADR-0004 for the architecture-keyed dispatch contract.
//! Formulas are keyed on [`AttentionKind`]; downstream code composes the total
//! VRAM equation via [`TotalMemory`] (TODO: WP06+).

pub mod attention;

pub use attention::{AttentionKind, KvFormula, LayerKind};
