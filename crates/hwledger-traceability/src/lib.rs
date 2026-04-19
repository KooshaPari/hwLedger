//! Functional Requirement ↔ Cross-Dimension Traceability Analysis
//!
//! Parses PRD.md to extract FRs/NFRs, scans across all dimensions:
//! - Tests (Traces verb)
//! - Source code (Implements verb)
//! - ADRs (Constrains verb)
//! - Documentation (Documents verb)
//! - Journey manifests (Exercises verb)
//!
//! Produces coverage reports ensuring FRs are fully traced across all dimensions.
//!
//! Traces to: NFR-006

pub mod prd;
pub mod report;
pub mod scan;

pub use prd::{FrKind, FrSpec, PrdParser};
pub use report::{CoverageLevel, CoverageReport, FrCoverage, Stats};
pub use scan::{
    AnnotationScanner, AnnotationVerb, Citer, ScanResult, TestScanner, TestTrace, TraceAnnotation,
};
