//! Coverage report generation and analysis.
//!
//! Traces to: NFR-006

use crate::prd::FrSpec;
use crate::scan::TestTrace;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// Coverage level for a single FR.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageLevel {
    Covered,
    Zero,
    Orphaned,
}

/// Coverage information for a single FR.
#[derive(Debug, Clone, Serialize)]
pub struct FrCoverage {
    pub fr: String,
    pub section: String,
    pub description: String,
    pub coverage: CoverageLevel,
    pub test_count: usize,
    pub tests: Vec<String>,
    pub ignored_count: usize,
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
    /// Generates a coverage report from FRs and test traces.
    ///
    /// Traces to: NFR-006
    pub fn generate(frs: Vec<FrSpec>, traces: Vec<TestTrace>) -> Self {
        let fr_map: HashMap<String, &FrSpec> = frs.iter().map(|f| (f.id.clone(), f)).collect();
        let mut coverage_map: HashMap<String, Vec<TestTrace>> = HashMap::new();
        let mut all_cited_frs = HashSet::new();
        let mut nonexistent_cites = Vec::new();
        let mut orphans = Vec::new();

        // Group traces by cited FRs and validate
        for trace in traces {
            if trace.cited_frs.is_empty() {
                continue;
            }

            let mut found_any = false;
            for fr in &trace.cited_frs {
                all_cited_frs.insert(fr.clone());
                if fr_map.contains_key(fr) {
                    found_any = true;
                    coverage_map
                        .entry(fr.clone())
                        .or_default()
                        .push(trace.clone());
                } else {
                    nonexistent_cites.push((trace.test_name.clone(), fr.clone()));
                }
            }

            if !found_any {
                orphans.push(trace);
            }
        }

        // Build coverage info for each FR
        let mut coverage_list = Vec::new();
        let mut zero_coverage_frs = Vec::new();
        let mut orphaned_frs = Vec::new();
        let mut covered_count = 0;
        let mut orphaned_count = 0;

        for fr in &frs {
            let tests = coverage_map.get(&fr.id).cloned().unwrap_or_default();
            let ignored_count = tests.iter().filter(|t| t.is_ignored).count();
            let active_tests = tests.iter().filter(|t| !t.is_ignored).count();

            let coverage = if active_tests > 0 {
                covered_count += 1;
                CoverageLevel::Covered
            } else if !tests.is_empty() {
                orphaned_count += 1;
                orphaned_frs.push(fr.id.clone());
                CoverageLevel::Orphaned
            } else {
                zero_coverage_frs.push(fr.id.clone());
                CoverageLevel::Zero
            };

            coverage_list.push(FrCoverage {
                fr: fr.id.clone(),
                section: fr.section.clone(),
                description: fr.description.clone(),
                coverage,
                test_count: active_tests,
                tests: tests.iter().map(|t| t.test_name.clone()).collect(),
                ignored_count,
            });
        }

        let total_frs = frs.len();
        let total_tests = coverage_map
            .values()
            .flat_map(|v| v.iter())
            .filter(|t| !t.is_ignored)
            .count();
        let coverage_percent = if total_frs > 0 {
            (covered_count as f32 / total_frs as f32) * 100.0
        } else {
            0.0
        };

        let stats = Stats {
            total_frs,
            covered_count,
            zero_coverage_count: zero_coverage_frs.len(),
            orphaned_count,
            coverage_percent,
            total_tests,
            zero_coverage_frs,
            orphaned_frs,
            nonexistent_cites,
        };

        CoverageReport {
            frs: coverage_list,
            stats,
            orphans,
        }
    }

    /// Returns the top N best-covered FRs (by test count).
    ///
    /// Traces to: NFR-006
    pub fn top_covered(&self, n: usize) -> Vec<&FrCoverage> {
        let mut sorted = self.frs.iter().collect::<Vec<_>>();
        sorted.sort_by(|a, b| b.test_count.cmp(&a.test_count));
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

    /// Generates a markdown summary of the report.
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
        md.push_str(&format!(
            "- **Total FRs/NFRs:** {}\n",
            self.stats.total_frs
        ));
        md.push_str(&format!(
            "- **Covered:** {} ({:.1}%)\n",
            self.stats.covered_count, self.stats.coverage_percent
        ));
        md.push_str(&format!(
            "- **Zero Coverage:** {}\n",
            self.stats.zero_coverage_count
        ));
        md.push_str(&format!(
            "- **Orphaned (ignored only):** {}\n",
            self.stats.orphaned_count
        ));
        md.push_str(&format!("- **Total Tests:** {}\n\n", self.stats.total_tests));

        if !self.stats.zero_coverage_frs.is_empty() {
            md.push_str("## Zero Coverage (Blocker)\n\n");
            for fr in &self.stats.zero_coverage_frs {
                md.push_str(&format!("- **{}**\n", fr));
            }
            md.push('\n');
        }

        if !self.stats.nonexistent_cites.is_empty() {
            md.push_str("## Unknown FR Citations (Typos)\n\n");
            for (test, fr) in &self.stats.nonexistent_cites {
                md.push_str(&format!("- Test `{}` cites unknown FR `{}`\n", test, fr));
            }
            md.push('\n');
        }

        let top = self.top_covered(5);
        if !top.is_empty() {
            md.push_str("## Best Covered (Top 5)\n\n");
            for cov in top {
                md.push_str(&format!(
                    "- **{}** ({} tests): {}\n",
                    cov.fr, cov.test_count, cov.description
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
                    CoverageLevel::Orphaned => "ORPHANED",
                    CoverageLevel::Covered => "COVERED",
                };
                md.push_str(&format!(
                    "- **{}** [{}] ({} tests): {}\n",
                    cov.fr, status, cov.test_count, cov.description
                ));
            }
            md.push('\n');
        }

        md.push_str("## Coverage by Section\n\n");
        let mut sections: HashMap<String, Vec<&FrCoverage>> = HashMap::new();
        for cov in &self.frs {
            sections
                .entry(cov.section.clone())
                .or_default()
                .push(cov);
        }

        for (section, covs) in sections {
            let covered_in_section = covs.iter().filter(|c| c.coverage == CoverageLevel::Covered).count();
            md.push_str(&format!("### {} ({}/{})\n\n", section, covered_in_section, covs.len()));
            for cov in covs {
                let icon = match cov.coverage {
                    CoverageLevel::Covered => "✓",
                    CoverageLevel::Zero => "✗",
                    CoverageLevel::Orphaned => "~",
                };
                md.push_str(&format!(
                    "- {} **{}** ({} tests)\n",
                    icon, cov.fr, cov.test_count
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
        assert_eq!(report.stats.covered_count, 1);
        assert_eq!(report.stats.zero_coverage_count, 0);
    }
}
