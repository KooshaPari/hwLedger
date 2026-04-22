//! `user-story-extract` CLI.
//!
//! See `src/lib.rs` for architectural notes and the canonical
//! `user-story.schema.json` embedded at build time.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use user_story_extract::{
    check_coverage, check_duplicate_ids, extract_paths, parse_fr_list, ExtractError,
};

#[derive(Parser, Debug)]
#[command(
    name = "user-story-extract",
    version,
    about = "Harvest @phenotype/user-story YAML blocks from test sources.",
    long_about = "Walks one or more roots, finds language-native frontmatter blocks (Rust //, \
                  Swift // MARK:, Playwright JSDoc, k6 /* ... */), validates them against the \
                  canonical user-story.schema.json, and emits a single JSON index."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Harvest and write JSON index.
    Extract(ExtractArgs),
    /// Harvest + schema-validate. Exits 1 on any malformed story.
    Validate(ExtractArgs),
    /// Harvest and cross-reference `traces_to` against a Markdown FR list.
    CheckCoverage(CoverageArgs),
    /// Harvest and fail on duplicate `journey_id`.
    CheckDuplicateIds(ExtractArgs),
}

#[derive(Parser, Debug)]
struct ExtractArgs {
    /// Roots to walk. Defaults applied when empty.
    #[arg(value_name = "PATH")]
    roots: Vec<PathBuf>,

    /// Output path for the JSON index.
    #[arg(long, short = 'o', default_value = "docs-site/quality/user-stories.json")]
    out: PathBuf,

    /// Write to stdout instead of `--out`.
    #[arg(long)]
    stdout: bool,
}

#[derive(Parser, Debug)]
struct CoverageArgs {
    #[command(flatten)]
    extract: ExtractArgs,

    /// FR source (Markdown file with `FR-XXX` tokens). Defaults to PRD.md.
    #[arg(long, default_value = "PRD.md")]
    fr_source: PathBuf,
}

fn default_roots() -> Vec<PathBuf> {
    [
        "tests",
        "crates",
        "apps/macos/HwLedgerUITests",
        "apps/windows",
        "apps/linux-slint",
        "apps/linux-qt",
        "apps/streamlit/journeys/specs",
        "load",
    ]
    .iter()
    .map(PathBuf::from)
    .collect()
}

fn resolve_roots(args_roots: &[PathBuf]) -> Vec<PathBuf> {
    if args_roots.is_empty() { default_roots() } else { args_roots.to_vec() }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Extract(args) => run_extract(args, /*fail_on_errors=*/ false),
        Cmd::Validate(args) => run_extract(args, /*fail_on_errors=*/ true),
        Cmd::CheckDuplicateIds(args) => run_dup(args),
        Cmd::CheckCoverage(args) => run_coverage(args),
    }
}

fn run_extract(args: ExtractArgs, fail_on_errors: bool) -> Result<()> {
    let roots = resolve_roots(&args.roots);
    let (stories, errors) = extract_paths(&roots);

    report_errors(&errors);
    let json = serde_json::to_string_pretty(&stories)
        .context("serialise user-stories JSON index")?;
    if args.stdout {
        println!("{json}");
    } else {
        if let Some(parent) = args.out.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create parent of {}", args.out.display()))?;
        }
        fs::write(&args.out, json)
            .with_context(|| format!("write {}", args.out.display()))?;
        eprintln!("wrote {} stories to {}", stories.len(), args.out.display());
    }

    if fail_on_errors && !errors.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}

fn run_dup(args: ExtractArgs) -> Result<()> {
    let roots = resolve_roots(&args.roots);
    let (stories, errors) = extract_paths(&roots);
    report_errors(&errors);
    let dups = check_duplicate_ids(&stories);
    if !dups.is_empty() {
        for e in &dups { eprintln!("{e}"); }
        std::process::exit(1);
    }
    eprintln!("ok: {} unique journey_ids", stories.len());
    Ok(())
}

fn run_coverage(args: CoverageArgs) -> Result<()> {
    let roots = resolve_roots(&args.extract.roots);
    let (stories, errors) = extract_paths(&roots);
    report_errors(&errors);
    let fr_md = fs::read_to_string(&args.fr_source)
        .with_context(|| format!("read FR source {}", args.fr_source.display()))?;
    let known: BTreeSet<String> = parse_fr_list(&fr_md);
    let missing = check_coverage(&stories, &known);
    if !missing.is_empty() {
        for e in &missing { eprintln!("{e}"); }
        std::process::exit(1);
    }
    eprintln!("ok: {} stories cite only known FRs ({} known)", stories.len(), known.len());
    Ok(())
}

fn report_errors(errors: &[ExtractError]) {
    for e in errors {
        eprintln!("error: {e}");
    }
}
