//! Functional Requirement ↔ Test Traceability Analysis
//!
//! Parses PRD.md to extract FRs/NFRs, scans test files for `Traces to:` annotations,
//! and produces coverage reports to ensure all FRs are tested and all test citations are valid.
//!
//! Traces to: NFR-006

pub mod prd;
pub mod report;
pub mod scan;

pub use prd::{FrKind, FrSpec, PrdParser};
pub use report::{CoverageLevel, CoverageReport, FrCoverage, Stats};
pub use scan::{ScanResult, TestTrace, TestScanner};
