//! Coverage report generation and analysis.
//!
//! Traces to: NFR-006

use crate::prd::FrSpec;
use crate::scan::{AnnotationVerb, Citer, TestTrace, TraceAnnotation};
use serde::Serialize;
use std::collections::HashMap;

/// Coverage level for a single FR across all dimensions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageLevel {
    /// Has >=1 test + >=1 implementation + >=1 documentation (all dimensions)
    FullyTraced,
    /// Has >=1 test but missing implementation or documentation
    Traced,
    /// Has documentation/ADR citations but no test (doc-only)
    DocOnly,
    /// No annotations in any dimension
    Zero,
    /// Legacy: only ignored tests (deprecated with new dimensional model)
    Orphaned,
}

/// Coverage information for a single FR across all dimensions.
#[derive(Debug, Clone, Serialize)]
pub struct FrCoverage {
    pub fr: String,
    pub section: String,
    pub description: String,
    pub coverage: CoverageLevel,
    // Tests (Traces verb)
    pub tests: Vec<String>,
    pub test_count: usize,
    pub ignored_count: usize,
    // Implementations (Implements verb)
    pub implementations: Vec<TraceAnnotation>,
    // ADR constraints (Constrains verb)
    pub constraints: Vec<TraceAnnotation>,
    // Documentation (Documents verb)
    pub documentation: Vec<TraceAnnotation>,
    // Journeys (Exercises verb)
    pub journeys: Vec<TraceAnnotation>,
}

/// Summary statistics for the report.
#[derive(Debug, Clone, Serialize)]
pub struct Stats {
    pub total_frs: usize,
    pub covered_count: usize,
    pub zero_coverage_count: usize,
    pub orphaned_count: usize,
    pub coverage_percent: f32,
    pub total_tests: usize,
    pub zero_coverage_frs: Vec<String>,
    pub orphaned_frs: Vec<String>,
    pub nonexistent_cites: Vec<(String, String)>, // (test, unknown_fr)
}

/// Complete coverage report.
#[derive(Debug, Clone, Serialize)]
pub struct CoverageReport {
    pub frs: Vec<FrCoverage>,
    pub stats: Stats,
    pub orphans: Vec<TestTrace>,
}

impl CoverageReport {
    /// Generates a coverage report from FRs and cross-dimensional annotations.
    ///
    /// Traces to: NFR-006
    pub fn generate(frs: Vec<FrSpec>, traces: Vec<TestTrace>) -> Self {
        // Convert legacy TestTrace to TraceAnnotation for compatibility
        let annotations: Vec<TraceAnnotation> = traces
            .into_iter()
            .map(|t| TraceAnnotation {
                citer: Citer::RustTest,
                verb: AnnotationVerb::Traces,
                file: t.file,
                line: t.line,
                cited_frs: t.cited_frs,
                context: t.test_name,
                is_ignored: t.is_ignored,
            })
            .collect();

        Self::generate_from_annotations(frs, annotations)
    }

    /// Generates a coverage report from FRs and cross-dimensional annotations.
    ///
    /// Evaluates coverage_level per FR:
    /// - FullyTraced: >=1 test + >=1 implementation + >=1 documentation
    /// - Traced: >=1 test but missing impl or docs
    /// - DocOnly: has docs/ADRs but no test
    /// - Zero: nothing
    ///
    /// Traces to: NFR-006
    pub fn generate_from_annotations(frs: Vec<FrSpec>, annotations: Vec<TraceAnnotation>) -> Self {
        let fr_map: HashMap<String, &FrSpec> = frs.iter().map(|f| (f.id.clone(), f)).collect();

        // Organize annotations by (FR, verb)
        let mut anno_by_fr_verb: HashMap<(String, AnnotationVerb), Vec<TraceAnnotation>> =
            HashMap::new();

        for anno in annotations {
            for fr in &anno.cited_frs {
                if fr_map.contains_key(fr) {
                    anno_by_fr_verb.entry((fr.clone(), anno.verb)).or_default().push(anno.clone());
                }
            }
        }

        // Build coverage info for each FR
        let mut coverage_list = Vec::new();
        let mut zero_coverage_frs = Vec::new();
        let mut fully_traced_count = 0;
        let mut traced_count = 0;
        let mut _doc_only_count = 0;

        for fr in &frs {
            let tests = anno_by_fr_verb
                .get(&(fr.id.clone(), AnnotationVerb::Traces))
                .cloned()
                .unwrap_or_default();
            let implementations = anno_by_fr_verb
                .get(&(fr.id.clone(), AnnotationVerb::Implements))
                .cloned()
                .unwrap_or_default();
            let constraints = anno_by_fr_verb
                .get(&(fr.id.clone(), AnnotationVerb::Constrains))
                .cloned()
                .unwrap_or_default();
            let documentation = anno_by_fr_verb
                .get(&(fr.id.clone(), AnnotationVerb::Documents))
                .cloned()
                .unwrap_or_default();
            let journeys = anno_by_fr_verb
                .get(&(fr.id.clone(), AnnotationVerb::Exercises))
                .cloned()
                .unwrap_or_default();

            let ignored_count = tests.iter().filter(|t| t.is_ignored).count();
            let active_tests = tests.iter().filter(|t| !t.is_ignored).count();

            let has_test = active_tests > 0;
            let has_impl = !implementations.is_empty();
            let has_doc = !documentation.is_empty() || !constraints.is_empty();

            let coverage = if has_test && has_impl && has_doc {
                fully_traced_count += 1;
                CoverageLevel::FullyTraced
            } else if has_test {
                // Has test but missing impl or docs
                traced_count += 1;
                CoverageLevel::Traced
            } else if has_doc {
                _doc_only_count += 1;
                CoverageLevel::DocOnly
            } else {
                zero_coverage_frs.push(fr.id.clone());
                CoverageLevel::Zero
            };

            coverage_list.push(FrCoverage {
                fr: fr.id.clone(),
                section: fr.section.clone(),
                description: fr.description.clone(),
                coverage,
                tests: tests.iter().map(|t| t.context.clone()).collect(),
                test_count: active_tests,
                ignored_count,
                implementations,
                constraints,
                documentation,
                journeys,
            });
        }

        let total_frs = frs.len();
        let total_tests = anno_by_fr_verb.values().flatten().filter(|t| !t.is_ignored).count();
        let covered_count = fully_traced_count + traced_count;
        let coverage_percent =
            if total_frs > 0 { (covered_count as f32 / total_frs as f32) * 100.0 } else { 0.0 };

        let stats = Stats {
            total_frs,
            covered_count,
            zero_coverage_count: zero_coverage_frs.len(),
            orphaned_count: 0, // Legacy field
            coverage_percent,
            total_tests,
            zero_coverage_frs,
            orphaned_frs: Vec::new(), // Legacy field
            nonexistent_cites: Vec::new(),
        };

        CoverageReport { frs: coverage_list, stats, orphans: Vec::new() }
    }

    /// Returns the top N best-covered FRs (by test count).
    ///
    /// Traces to: NFR-006
    pub fn top_covered(&self, n: usize) -> Vec<&FrCoverage> {
        let mut sorted = self.frs.iter().collect::<Vec<_>>();
        sorted.sort_by_key(|c| std::cmp::Reverse(c.test_count));
        sorted.into_iter().take(n).collect()
    }

    /// Returns the bottom N worst-covered FRs (by test count, zeros first).
    ///
    /// Traces to: NFR-006
    pub fn worst_covered(&self, n: usize) -> Vec<&FrCoverage> {
        let mut sorted = self.frs.iter().collect::<Vec<_>>();
        sorted.sort_by(|a, b| {
            let a_is_zero = a.coverage == CoverageLevel::Zero;
            let b_is_zero = b.coverage == CoverageLevel::Zero;
            if a_is_zero != b_is_zero {
                return b_is_zero.cmp(&a_is_zero); // zeros first
            }
            a.test_count.cmp(&b.test_count) // then by count
        });
        sorted.into_iter().take(n).collect()
    }

    /// Generates a markdown summary of the report with cross-dimensional matrix.
    ///
    /// Traces to: NFR-006
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# Functional Requirement Traceability Report\n\n");
        md.push_str("> Auto-generated by `hwledger-traceability`. To refresh, run:\n");
        md.push_str("> ```bash\n");
        md.push_str("> cargo run -p hwledger-traceability -- --markdown-out docs-site/quality/traceability.md\n");
        md.push_str("> ```\n\n");

        md.push_str("## Summary\n\n");
        md.push_str(&format!("- **Total FRs/NFRs:** {}\n", self.stats.total_frs));

        let fully_traced =
            self.frs.iter().filter(|f| f.coverage == CoverageLevel::FullyTraced).count();
        let traced = self.frs.iter().filter(|f| f.coverage == CoverageLevel::Traced).count();
        let doc_only = self.frs.iter().filter(|f| f.coverage == CoverageLevel::DocOnly).count();
        let zero = self.frs.iter().filter(|f| f.coverage == CoverageLevel::Zero).count();

        md.push_str(&format!(
            "- **Fully Traced** (test + impl + docs): {} ({:.1}%)\n",
            fully_traced,
            (fully_traced as f32 / self.stats.total_frs as f32) * 100.0
        ));
        md.push_str(&format!(
            "- **Traced** (test + partial): {} ({:.1}%)\n",
            traced,
            (traced as f32 / self.stats.total_frs as f32) * 100.0
        ));
        md.push_str(&format!(
            "- **Doc-Only** (docs but no test): {} ({:.1}%)\n",
            doc_only,
            (doc_only as f32 / self.stats.total_frs as f32) * 100.0
        ));
        md.push_str(&format!(
            "- **Zero Coverage:** {} ({:.1}%)\n",
            zero,
            (zero as f32 / self.stats.total_frs as f32) * 100.0
        ));
        md.push_str(&format!("- **Total Tests:** {}\n\n", self.stats.total_tests));

        // Cross-dimensional matrix table
        md.push_str("## Cross-Dimensional Traceability Matrix\n\n");
        md.push_str("| FR | Tests | Source | ADRs | Docs | Journeys | Level |\n");
        md.push_str("|---|---|---|---|---|---|---|\n");

        for cov in &self.frs {
            let test_count = cov.test_count;
            let impl_count = cov.implementations.len();
            let constraint_count = cov.constraints.len();
            let doc_count = cov.documentation.len();
            let journey_count = cov.journeys.len();

            let level_icon = match cov.coverage {
                CoverageLevel::FullyTraced => "OK",
                CoverageLevel::Traced => "PART",
                CoverageLevel::DocOnly => "DOCS",
                CoverageLevel::Zero => "NONE",
                CoverageLevel::Orphaned => "IGN",
            };

            md.push_str(&format!(
                "| **{}** | {} | {} | {} | {} | {} | {} |\n",
                cov.fr,
                test_count,
                impl_count,
                constraint_count,
                doc_count,
                journey_count,
                level_icon
            ));
        }
        md.push('\n');

        if !self.stats.zero_coverage_frs.is_empty() {
            md.push_str("## Zero Coverage (Requires Documentation)\n\n");
            for fr in &self.stats.zero_coverage_frs {
                md.push_str(&format!("- **{}**\n", fr));
            }
            md.push('\n');
        }

        let top = self.top_covered(5);
        if !top.is_empty() {
            md.push_str("## Best Covered (Top 5)\n\n");
            for cov in top {
                md.push_str(&format!(
                    "- **{}** (tests: {}, impl: {}, docs: {}): {}\n",
                    cov.fr,
                    cov.test_count,
                    cov.implementations.len(),
                    cov.documentation.len(),
                    cov.description
                ));
            }
            md.push('\n');
        }

        let worst = self.worst_covered(5);
        if !worst.is_empty() {
            md.push_str("## Worst Covered (Bottom 5)\n\n");
            for cov in worst {
                let status = match cov.coverage {
                    CoverageLevel::Zero => "ZERO",
                    CoverageLevel::DocOnly => "DOCS",
                    CoverageLevel::Traced => "PART",
                    CoverageLevel::FullyTraced => "FULL",
                    CoverageLevel::Orphaned => "IGN",
                };
                md.push_str(&format!(
                    "- **{}** [{}] (tests: {}, impl: {}, docs: {}): {}\n",
                    cov.fr,
                    status,
                    cov.test_count,
                    cov.implementations.len(),
                    cov.documentation.len(),
                    cov.description
                ));
            }
            md.push('\n');
        }

        md.push_str("## Coverage by Section\n\n");
        let mut sections: HashMap<String, Vec<&FrCoverage>> = HashMap::new();
        for cov in &self.frs {
            sections.entry(cov.section.clone()).or_default().push(cov);
        }

        for (section, covs) in sections {
            let fully_traced_in_section =
                covs.iter().filter(|c| c.coverage == CoverageLevel::FullyTraced).count();
            md.push_str(&format!(
                "### {} ({}/{})\n\n",
                section,
                fully_traced_in_section,
                covs.len()
            ));
            for cov in covs {
                let icon = match cov.coverage {
                    CoverageLevel::FullyTraced => "OK",
                    CoverageLevel::Traced => "PART",
                    CoverageLevel::DocOnly => "DOCS",
                    CoverageLevel::Zero => "NONE",
                    CoverageLevel::Orphaned => "IGN",
                };
                md.push_str(&format!(
                    "- [{}] **{}** (T:{}, I:{}, D:{})\n",
                    icon,
                    cov.fr,
                    cov.test_count,
                    cov.implementations.len(),
                    cov.documentation.len()
                ));
            }
            md.push('\n');
        }

        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Traces to: NFR-006
    #[test]
    fn test_coverage_report_empty() {
        let report = CoverageReport::generate(vec![], vec![]);
        assert_eq!(report.stats.total_frs, 0);
        assert_eq!(report.stats.covered_count, 0);
    }

    /// Traces to: NFR-006
    #[test]
    fn test_coverage_level_detection() {
        let fr = FrSpec {
            id: "FR-PLAN-001".to_string(),
            kind: crate::prd::FrKind::Fr,
            description: "Test".to_string(),
            section: "Test Section".to_string(),
        };
        let trace = TestTrace {
            file: "test.rs".to_string(),
            line: 10,
            test_name: "test_example".to_string(),
            cited_frs: vec!["FR-PLAN-001".to_string()],
            is_ignored: false,
        };

        let report = CoverageReport::generate(vec![fr], vec![trace]);
        // With legacy convert, only test (Traces) is present, so it's Traced not FullyTraced
        assert_eq!(report.frs[0].coverage, CoverageLevel::Traced);
        assert_eq!(report.stats.zero_coverage_count, 0);
    }

    /// Traces to: NFR-006
    #[test]
    fn test_fully_traced_detection() {
        let fr = FrSpec {
            id: "FR-PLAN-001".to_string(),
            kind: crate::prd::FrKind::Fr,
            description: "Test".to_string(),
            section: "Test Section".to_string(),
        };

        let annotations = vec![
            // Test
            TraceAnnotation {
                citer: Citer::RustTest,
                verb: AnnotationVerb::Traces,
                file: "test.rs".to_string(),
                line: 10,
                cited_frs: vec!["FR-PLAN-001".to_string()],
                context: "test_example".to_string(),
                is_ignored: false,
            },
            // Implementation
            TraceAnnotation {
                citer: Citer::RustSource,
                verb: AnnotationVerb::Implements,
                file: "src/lib.rs".to_string(),
                line: 5,
                cited_frs: vec!["FR-PLAN-001".to_string()],
                context: "compute".to_string(),
                is_ignored: false,
            },
            // Documentation
            TraceAnnotation {
                citer: Citer::DocPage,
                verb: AnnotationVerb::Documents,
                file: "PLAN.md".to_string(),
                line: 20,
                cited_frs: vec!["FR-PLAN-001".to_string()],
                context: "Overview".to_string(),
                is_ignored: false,
            },
        ];

        let report = CoverageReport::generate_from_annotations(vec![fr], annotations);
        assert_eq!(report.frs.len(), 1);
        assert_eq!(report.frs[0].coverage, CoverageLevel::FullyTraced);
    }

    /// Traces to: NFR-006
    #[test]
    fn test_doc_only_detection() {
        let fr = FrSpec {
            id: "FR-PLAN-002".to_string(),
            kind: crate::prd::FrKind::Fr,
            description: "Test".to_string(),
            section: "Test Section".to_string(),
        };

        let annotations = vec![TraceAnnotation {
            citer: Citer::DocPage,
            verb: AnnotationVerb::Documents,
            file: "PLAN.md".to_string(),
            line: 20,
            cited_frs: vec!["FR-PLAN-002".to_string()],
            context: "Overview".to_string(),
            is_ignored: false,
        }];

        let report = CoverageReport::generate_from_annotations(vec![fr], annotations);
        assert_eq!(report.frs[0].coverage, CoverageLevel::DocOnly);
    }
}
