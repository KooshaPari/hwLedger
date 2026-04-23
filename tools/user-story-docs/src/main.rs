//! `user-story-docs` CLI.
//!
//! Walks `docs-site/quality/user-stories.json` and renders
//! `docs-site/journeys/<id>.md` pages. See `lib.rs` for format contract.

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use user_story_docs::{apply, load_index, plan, summarise_diff, PlannedAction};

#[derive(Parser, Debug)]
#[command(
    name = "user-story-docs",
    version,
    about = "Regenerate docs-site/journeys/<id>.md pages from harvested user-stories.json.",
    long_about = "Preserves any manual prose authored AFTER the \
                  `<!-- @user-story-docs:END -->` marker. Every line above \
                  the marker is overwritten on each run."
)]
struct Cli {
    /// Path to the harvested stories index.
    #[arg(long, default_value = "docs-site/quality/user-stories.json")]
    index: PathBuf,

    /// Directory to write `<journey_id>.md` pages into.
    #[arg(long, default_value = "docs-site/journeys")]
    out: PathBuf,

    /// Write changes. Without `--apply` the CLI prints a diff summary only.
    #[arg(long)]
    apply: bool,

    /// Fail (exit 2) if any page would change. Intended for pre-push gating
    /// in `HWLEDGER_TAPE_GATE=strict` mode.
    #[arg(long)]
    strict: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let stories = load_index(&cli.index)
        .with_context(|| format!("loading index {}", cli.index.display()))?;
    let planned = plan(&stories, &cli.out)
        .with_context(|| format!("planning against {}", cli.out.display()))?;

    let diff_lines = summarise_diff(&planned);
    let needs_write = planned
        .iter()
        .any(|p| !matches!(p.action, PlannedAction::Unchanged));

    if diff_lines.is_empty() {
        println!("user-story-docs: {} stories, no changes.", stories.len());
    } else {
        for l in &diff_lines {
            println!("{}", l);
        }
        println!(
            "user-story-docs: {} stories, {} changed.",
            stories.len(),
            diff_lines.len()
        );
    }

    if cli.apply {
        let written = apply(&planned).context("applying writes")?;
        println!("user-story-docs: wrote {} files to {}", written, cli.out.display());
        return Ok(());
    }

    if cli.strict && needs_write {
        eprintln!(
            "user-story-docs: --strict and {} pages need regeneration. Re-run with --apply.",
            diff_lines.len()
        );
        std::process::exit(2);
    }

    Ok(())
}
