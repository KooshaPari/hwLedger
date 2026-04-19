//! CLI for FR ↔ test traceability analysis.
//!
//! Traces to: NFR-006

use anyhow::{Context, Result};
use clap::Parser;
use hwledger_traceability::{AnnotationScanner, CoverageReport, CoverageLevel, PrdParser};
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
    let annotations = AnnotationScanner::scan(&repo_path)
        .context("Failed to scan annotations")?;

    if args.verbose {
        eprintln!("Found {} annotations (Traces, Implements, Constrains, Documents, Exercises)", annotations.len());
    }

    // Generate report
    let report = CoverageReport::generate_from_annotations(frs, annotations);

    // Output handling
    if args.json {
        let json =
            serde_json::to_string_pretty(&report).context("Failed to serialize report to JSON")?;
        println!("{}", json);
    } else if let Some(out_path) = args.markdown_out {
        let md = report.to_markdown();
        std::fs::write(&out_path, md)
            .context(format!("Failed to write markdown to {}", out_path.display()))?;
        eprintln!("Wrote markdown report to: {}", out_path.display());
    } else {
        // Pretty print
        print_report(&report);
    }

    // Strict mode checks
    if args.strict {
        let not_fully_traced: Vec<_> = report.frs
            .iter()
            .filter(|f| f.coverage != CoverageLevel::FullyTraced)
            .collect();

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
                    },
                    CoverageLevel::DocOnly => "no test".to_string(),
                    CoverageLevel::Zero => "no annotation".to_string(),
                    CoverageLevel::Orphaned => "ignored only".to_string(),
                };
                eprintln!("  - {} ({})", cov.fr, reason);
            }
            std::process::exit(1);
        }
    }

    Ok(())
}

fn print_report(report: &CoverageReport) {
    println!("\n=== Cross-Dimensional Traceability Report ===\n");
    println!("Total FRs/NFRs: {}", report.stats.total_frs);

    let fully_traced = report.frs.iter().filter(|f| f.coverage == CoverageLevel::FullyTraced).count();
    let traced = report.frs.iter().filter(|f| f.coverage == CoverageLevel::Traced).count();
    let doc_only = report.frs.iter().filter(|f| f.coverage == CoverageLevel::DocOnly).count();
    let zero = report.frs.iter().filter(|f| f.coverage == CoverageLevel::Zero).count();

    println!("Fully Traced (test + impl + docs): {} ({:.1}%)",
        fully_traced, (fully_traced as f32 / report.stats.total_frs as f32) * 100.0);
    println!("Traced (test + partial): {} ({:.1}%)",
        traced, (traced as f32 / report.stats.total_frs as f32) * 100.0);
    println!("Doc-Only (docs but no test): {} ({:.1}%)",
        doc_only, (doc_only as f32 / report.stats.total_frs as f32) * 100.0);
    println!("Zero Coverage: {} ({:.1}%)",
        zero, (zero as f32 / report.stats.total_frs as f32) * 100.0);
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
            println!("  {} (T:{} I:{} D:{})",
                cov.fr, cov.test_count, cov.implementations.len(), cov.documentation.len());
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
                cov.fr, status, cov.test_count, cov.implementations.len(), cov.documentation.len()
            );
        }
        println!();
    }
}
