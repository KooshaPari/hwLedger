//! CLI for FR ↔ test traceability analysis.
//!
//! Traces to: NFR-006

use anyhow::{Context, Result};
use clap::Parser;
use hwledger_traceability::{CoverageReport, PrdParser, TestScanner};
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

    // Scan tests
    let crates_path = format!("{}/crates", repo_path);
    if args.verbose {
        eprintln!("Scanning tests in: {}", crates_path);
    }
    let traces = TestScanner::scan(&crates_path).context("Failed to scan test files")?;

    if args.verbose {
        eprintln!("Found {} test traces", traces.len());
    }

    // Generate report
    let report = CoverageReport::generate(frs, traces);

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
        if !report.stats.zero_coverage_frs.is_empty() {
            eprintln!("\nFAIL: Zero coverage FRs detected (--strict):");
            for fr in &report.stats.zero_coverage_frs {
                eprintln!("  - {}", fr);
            }
            std::process::exit(1);
        }

        if !report.stats.nonexistent_cites.is_empty() {
            eprintln!("\nFAIL: Unknown FR citations detected (--strict):");
            for (test, fr) in &report.stats.nonexistent_cites {
                eprintln!("  - {} cites unknown {}", test, fr);
            }
            std::process::exit(1);
        }
    }

    Ok(())
}

fn print_report(report: &CoverageReport) {
    println!("\n=== Traceability Report ===\n");
    println!("Total FRs/NFRs: {}", report.stats.total_frs);
    println!("Covered: {} ({:.1}%)", report.stats.covered_count, report.stats.coverage_percent);
    println!("Zero Coverage: {}", report.stats.zero_coverage_count);
    println!("Orphaned (ignored-only): {}", report.stats.orphaned_count);
    println!("Total Tests: {}\n", report.stats.total_tests);

    if !report.stats.zero_coverage_frs.is_empty() {
        println!("ZERO COVERAGE (Blocker):");
        for fr in &report.stats.zero_coverage_frs {
            println!("  - {}", fr);
        }
        println!();
    }

    if !report.stats.nonexistent_cites.is_empty() {
        println!("UNKNOWN FR CITATIONS (Typos):");
        for (test, fr) in &report.stats.nonexistent_cites {
            println!("  - {} cites unknown {}", test, fr);
        }
        println!();
    }

    let top = report.top_covered(5);
    if !top.is_empty() {
        println!("Top 5 Best-Covered:");
        for cov in top {
            println!("  {} ({} tests)", cov.fr, cov.test_count);
        }
        println!();
    }

    let worst = report.worst_covered(5);
    if !worst.is_empty() {
        println!("Bottom 5 Worst-Covered:");
        for cov in worst {
            println!(
                "  {} ({} tests) [{}]",
                cov.fr,
                cov.test_count,
                match cov.coverage {
                    hwledger_traceability::CoverageLevel::Zero => "ZERO",
                    hwledger_traceability::CoverageLevel::Orphaned => "ORPHANED",
                    hwledger_traceability::CoverageLevel::Covered => "OK",
                }
            );
        }
        println!();
    }
}
