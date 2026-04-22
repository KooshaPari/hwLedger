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

pub mod journeys;
pub mod prd;
pub mod report;
pub mod scan;

pub use journeys::{
    evaluate as evaluate_journeys, render_markdown as render_journey_markdown, scan_verified,
    BlindEvalMode, JourneyCoverageRow, JourneyManifest, JourneyReport, JourneyScan, JourneyStatus,
    ManifestStep, ManifestVerification, OrphanJourney, MIN_JOURNEY_SCORE,
};
pub use prd::{FrKind, FrSpec, JourneyKind, PrdParser};
pub use report::{CoverageLevel, CoverageReport, FrCoverage, Stats};
pub use scan::{
    AnnotationScanner, AnnotationVerb, Citer, ScanResult, TestScanner, TestTrace, TraceAnnotation,
};
