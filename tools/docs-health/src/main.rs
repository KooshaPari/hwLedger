//! CLI entry point for `hwledger-docs-health`.
//!
//! Usage:
//!   hwledger-docs-health --root docs-site [--json] [--fail-on warning|error] [--only check]...

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use owo_colors::OwoColorize;

use hwledger_docs_health::{run_all, Finding, RunOptions, Severity};

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum FailOn {
    Warning,
    Error,
}

impl FailOn {
    fn threshold(self) -> Severity {
        match self {
            FailOn::Warning => Severity::Warn,
            FailOn::Error => Severity::Error,
        }
    }
}

#[derive(Parser, Debug)]
#[command(about = "Proactive docs-health gate (mermaid, LaTeX, video, links, placeholders).")]
struct Args {
    /// Root directory to scan (e.g. `docs-site`).
    #[arg(long, default_value = "docs-site")]
    root: PathBuf,

    /// Emit JSON instead of a human-readable report.
    #[arg(long, default_value_t = false)]
    json: bool,

    /// Exit non-zero when any finding at or above this severity is present.
    #[arg(long, value_enum, default_value_t = FailOn::Error)]
    fail_on: FailOn,

    /// Restrict to a subset of checks (repeatable). Empty = all.
    #[arg(long = "only")]
    only: Vec<String>,
}

fn main() -> ExitCode {
    match real_main() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("docs-health: fatal: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn real_main() -> Result<ExitCode> {
    let args = Args::parse();
    let opts = RunOptions {
        only: args.only.iter().cloned().collect::<HashSet<_>>(),
    };
    let findings = run_all(&args.root, &opts)?;

    if args.json {
        let body = serde_json::json!({ "findings": findings });
        println!("{}", serde_json::to_string_pretty(&body)?);
    } else {
        print_human(&findings);
    }

    let threshold = args.fail_on.threshold();
    let blocking = findings.iter().any(|f| f.severity.at_least(threshold));
    Ok(if blocking { ExitCode::from(1) } else { ExitCode::SUCCESS })
}

fn print_human(findings: &[Finding]) {
    if findings.is_empty() {
        println!("{} no findings", "ok".green().bold());
        return;
    }
    let mut by_check: std::collections::BTreeMap<&str, Vec<&Finding>> = Default::default();
    for f in findings {
        by_check.entry(f.check.as_str()).or_default().push(f);
    }
    for (check, items) in by_check {
        println!("\n{} {}", "==".blue(), check.bold());
        for f in items {
            let tag = match f.severity {
                Severity::Error => "ERR".red().bold().to_string(),
                Severity::Warn => "WARN".yellow().bold().to_string(),
            };
            let loc = match f.line {
                Some(n) => format!("{}:{}", f.path.display(), n),
                None => format!("{}", f.path.display()),
            };
            println!("  {tag} {loc} — {}", f.message);
        }
    }
    let errs = findings.iter().filter(|f| f.severity == Severity::Error).count();
    let warns = findings.iter().filter(|f| f.severity == Severity::Warn).count();
    println!(
        "\n{}: {} error(s), {} warning(s)",
        "summary".bold(),
        errs,
        warns
    );
}
