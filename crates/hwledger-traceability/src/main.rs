//! CLI for FR ↔ test traceability analysis.
//!
//! Traces to: NFR-006

use anyhow::{Context, Result};
use clap::Parser;
use hwledger_traceability::{
    evaluate_journeys, render_journey_markdown, scan_verified, AnnotationScanner, CoverageLevel,
    CoverageReport, JourneyStatus, PrdParser,
};
use std::path::PathBuf;

/// FR ↔ Test Traceability Checker
#[derive(Parser, Debug)]
#[command(
    name = "hwledger-traceability",
    about = "Verify functional requirement ↔ test coverage",
    long_about = None
)]
struct Args {
    /// Repository root path (contains PRD.md and crates/)
    #[arg(long, default_value = ".")]
    repo: PathBuf,

    /// Output as JSON
    #[arg(long)]
    json: bool,

    /// Write markdown summary to file
    #[arg(long)]
    markdown_out: Option<PathBuf>,

    /// Strict mode: fail if any FR has zero coverage or unknown cites exist
    #[arg(long)]
    strict: bool,

    /// Strict-journeys mode: fail only on journey-coverage violations (FR-TRACE-003).
    /// Useful as a narrow pre-push gate while the classic strict gate is still
    /// being stabilised.
    #[arg(long)]
    strict_journeys: bool,

    /// Treat `NotPassed` journey rows as warnings rather than failures. Missing
    /// journeys and orphan `traces_to` references still fail. Pair with
    /// `HWLEDGER_TAPE_GATE=warn` during the tape re-record retrofit.
    #[arg(long)]
    allow_not_passed: bool,

    /// Escalate `NeedsCapture` rows (GUI journeys with any `blind_eval: skip`
    /// step) from warning-class to hard failure. Default policy keeps them
    /// advisory so the pipeline can stay green while real macOS captures are
    /// pending TCC grant.
    #[arg(long)]
    no_skip_allowed: bool,

    /// Escalate `needs_agreement_review` rows (journeys with any step whose
    /// intent↔blind agreement is Red) from advisory to hard failure. Default
    /// policy keeps them advisory so noisy blind descriptions do not block
    /// CI while prompts are being tuned.
    ///
    /// Traces to: FR-UX-VERIFY-003
    #[arg(long)]
    no_agreement_red: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let repo_path = args.repo.to_string_lossy().to_string();

    // Parse PRD
    let prd_path = format!("{}/PRD.md", repo_path);
    if args.verbose {
        eprintln!("Parsing PRD from: {}", prd_path);
    }
    let frs = PrdParser::parse(&prd_path).context("Failed to parse PRD.md")?;

    if args.verbose {
        eprintln!("Found {} FR/NFR specifications", frs.len());
    }

    // Scan all annotations (cross-dimensional)
    if args.verbose {
        eprintln!("Scanning annotations across all dimensions...");
    }
    let annotations = AnnotationScanner::scan(&repo_path).context("Failed to scan annotations")?;

    if args.verbose {
        eprintln!(
            "Found {} annotations (Traces, Implements, Constrains, Documents, Exercises)",
            annotations.len()
        );
    }

    // Scan verified journey manifests (FR-TRACE-002).
    let journey_scan =
        scan_verified(&args.repo).context("Failed to scan verified journey manifests")?;
    if args.verbose {
        eprintln!(
            "Discovered {} verified journey manifests ({} warnings)",
            journey_scan.manifests.len(),
            journey_scan.warnings.len()
        );
        for w in &journey_scan.warnings {
            eprintln!("  warning: {}", w);
        }
    }

    // Generate cross-dim report and journey coverage report separately.
    let journey_report = evaluate_journeys(&frs, &journey_scan);
    let report = CoverageReport::generate_from_annotations(frs, annotations);

    // Output handling
    if args.json {
        // Merge the two reports under a single JSON envelope.
        let envelope = serde_json::json!({
            "coverage": &report,
            "journey_coverage": &journey_report,
        });
        let json = serde_json::to_string_pretty(&envelope)
            .context("Failed to serialize report to JSON")?;
        println!("{}", json);
    } else if let Some(out_path) = args.markdown_out {
        let mut md = report.to_markdown();
        md.push_str(&render_journey_markdown(&journey_report));
        std::fs::write(&out_path, md)
            .context(format!("Failed to write markdown to {}", out_path.display()))?;
        eprintln!("Wrote markdown report to: {}", out_path.display());
    } else {
        // Pretty print
        print_report(&report);
        println!("\n{}", render_journey_markdown(&journey_report));
    }

    // Strict mode checks
    if args.strict || args.strict_journeys {
        let mut fail = false;

        // Journey gate (FR-TRACE-003) — evaluated first so it reports even if
        // classic coverage is already green.
        if journey_report.has_failures()
            || journey_report.has_needs_capture()
            || journey_report.has_agreement_red()
        {
            let mut hard_fail = false;
            eprintln!("\nJourney coverage gate (--strict):");
            for row in &journey_report.rows {
                // Agreement-red gate (FR-UX-VERIFY-003) is orthogonal to the
                // status-based reasons below; emit it first so reviewers see
                // the provenance regardless of the row status.
                if row.needs_agreement_review {
                    let level = if args.no_agreement_red { "FAIL" } else { "WARN" };
                    eprintln!(
                        "  - {level} {} [{}] (needs_agreement_review: ≥1 step with \
                         intent↔blind agreement=red)",
                        row.fr, row.kind,
                    );
                    if args.no_agreement_red {
                        hard_fail = true;
                    }
                }
                let (reason, is_hard) = match row.status {
                    JourneyStatus::Ok => continue,
                    JourneyStatus::Missing => ("missing journey for tagged FR".to_string(), true),
                    JourneyStatus::LowScore => (
                        format!(
                            "score {:.2} < {:.2}",
                            row.score.unwrap_or(0.0),
                            hwledger_traceability::MIN_JOURNEY_SCORE
                        ),
                        true,
                    ),
                    JourneyStatus::NotPassed => {
                        ("journey not passed".to_string(), !args.allow_not_passed)
                    }
                    JourneyStatus::NeedsCapture => (
                        "real capture pending (blind_eval: skip; macOS TCC)".to_string(),
                        args.no_skip_allowed,
                    ),
                };
                let level = if is_hard { "FAIL" } else { "WARN" };
                eprintln!("  - {level} {} [{}] ({})", row.fr, row.kind, reason);
                if is_hard {
                    hard_fail = true;
                }
            }
            for orph in &journey_report.orphan_journeys {
                eprintln!(
                    "  - FAIL orphan journey {} cites unknown FR(s): {}",
                    orph.journey_id,
                    orph.unknown_frs.join(", ")
                );
                hard_fail = true;
            }
            if hard_fail {
                fail = true;
            }
        }

        let not_fully_traced: Vec<_> = if args.strict {
            report.frs.iter().filter(|f| f.coverage != CoverageLevel::FullyTraced).collect()
        } else {
            Vec::new()
        };

        if !not_fully_traced.is_empty() {
            eprintln!("\nFAIL: Not all FRs are FullyTraced (--strict):");
            for cov in not_fully_traced {
                let reason = match cov.coverage {
                    CoverageLevel::FullyTraced => "OK".to_string(),
                    CoverageLevel::Traced => {
                        if cov.implementations.is_empty() {
                            "missing impl".to_string()
                        } else {
                            "missing docs".to_string()
                        }
                    }
                    CoverageLevel::DocOnly => "no test".to_string(),
                    CoverageLevel::Zero => "no annotation".to_string(),
                    CoverageLevel::Orphaned => "ignored only".to_string(),
                };
                eprintln!("  - {} ({})", cov.fr, reason);
            }
            fail = true;
        }

        if fail {
            std::process::exit(1);
        }
    }

    Ok(())
}

fn print_report(report: &CoverageReport) {
    println!("\n=== Cross-Dimensional Traceability Report ===\n");
    println!("Total FRs/NFRs: {}", report.stats.total_frs);

    let fully_traced =
        report.frs.iter().filter(|f| f.coverage == CoverageLevel::FullyTraced).count();
    let traced = report.frs.iter().filter(|f| f.coverage == CoverageLevel::Traced).count();
    let doc_only = report.frs.iter().filter(|f| f.coverage == CoverageLevel::DocOnly).count();
    let zero = report.frs.iter().filter(|f| f.coverage == CoverageLevel::Zero).count();

    println!(
        "Fully Traced (test + impl + docs): {} ({:.1}%)",
        fully_traced,
        (fully_traced as f32 / report.stats.total_frs as f32) * 100.0
    );
    println!(
        "Traced (test + partial): {} ({:.1}%)",
        traced,
        (traced as f32 / report.stats.total_frs as f32) * 100.0
    );
    println!(
        "Doc-Only (docs but no test): {} ({:.1}%)",
        doc_only,
        (doc_only as f32 / report.stats.total_frs as f32) * 100.0
    );
    println!(
        "Zero Coverage: {} ({:.1}%)",
        zero,
        (zero as f32 / report.stats.total_frs as f32) * 100.0
    );
    println!("Total Tests: {}\n", report.stats.total_tests);

    if !report.stats.zero_coverage_frs.is_empty() {
        println!("ZERO COVERAGE (Blocker):");
        for fr in &report.stats.zero_coverage_frs {
            println!("  - {}", fr);
        }
        println!();
    }

    let top = report.top_covered(5);
    if !top.is_empty() {
        println!("Top 5 Best-Covered:");
        for cov in top {
            println!(
                "  {} (T:{} I:{} D:{})",
                cov.fr,
                cov.test_count,
                cov.implementations.len(),
                cov.documentation.len()
            );
        }
        println!();
    }

    let worst = report.worst_covered(5);
    if !worst.is_empty() {
        println!("Bottom 5 Worst-Covered:");
        for cov in worst {
            let status = match cov.coverage {
                CoverageLevel::Zero => "ZERO",
                CoverageLevel::DocOnly => "DOCS",
                CoverageLevel::Traced => "PART",
                CoverageLevel::FullyTraced => "FULL",
                CoverageLevel::Orphaned => "IGN",
            };
            println!(
                "  {} [{}] (T:{} I:{} D:{})",
                cov.fr,
                status,
                cov.test_count,
                cov.implementations.len(),
                cov.documentation.len()
            );
        }
        println!();
    }
}
