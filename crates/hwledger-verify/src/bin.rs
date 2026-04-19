//! CLI binary for hwledger-verify: describe, judge, and verify journey manifests.

use clap::{Parser, Subcommand};
use hwledger_verify::{Verifier, VerifierConfig};
use owo_colors::OwoColorize;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "hwledger-verify",
    about = "Blackbox screenshot verification via Claude VLM + LLM-judge"
)]
struct Args {
    #[command(subcommand)]
    command: Command,

    /// Enable debug logging
    #[arg(global = true, long)]
    debug: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Describe a screenshot using Claude Opus 4.7
    Describe {
        /// Path to PNG screenshot
        #[arg(value_name = "FILE")]
        screenshot: PathBuf,

        /// Override describe model
        #[arg(long)]
        model: Option<String>,
    },

    /// Judge whether a description matches an intent
    Judge {
        /// User intent label
        #[arg(long)]
        intent: String,

        /// VLM-generated description
        #[arg(long)]
        description: String,

        /// Override judge model
        #[arg(long)]
        model: Option<String>,
    },

    /// Verify all steps in a journey manifest
    Manifest {
        /// Path to manifest JSON
        #[arg(value_name = "FILE")]
        manifest: PathBuf,

        /// Disable caching
        #[arg(long)]
        no_cache: bool,

        /// Output file for verified manifest
        #[arg(long, short = 'o')]
        out: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Setup logging
    let env_filter = if args.debug {
        EnvFilter::from_default_env()
            .add_directive("hwledger_verify=debug".parse().unwrap())
    } else {
        EnvFilter::from_default_env()
            .add_directive("hwledger_verify=info".parse().unwrap())
    };

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .init();

    match args.command {
        Command::Describe { screenshot, model } => {
            let config = match model {
                Some(m) => VerifierConfig::default().with_describe_model(m),
                None => VerifierConfig::default(),
            };
            let verifier = Verifier::new(config)?;

            let png_bytes = std::fs::read(&screenshot)?;
            println!("{} {}", "Reading".bold(), screenshot.display());

            let description = verifier.describe(&png_bytes).await?;

            println!("\n{}", "Description:".bold());
            println!("{}", description.text);

            if let Some(structured) = &description.structured {
                println!("\n{}", "Structured:".bold());
                println!("{}", serde_json::to_string_pretty(structured)?);
            }

            println!("\n{}", format!("Tokens used: {}", description.tokens_used).dimmed());
        }

        Command::Judge {
            intent,
            description,
            model,
        } => {
            let config = match model {
                Some(m) => VerifierConfig::default().with_judge_model(m),
                None => VerifierConfig::default(),
            };
            let verifier = Verifier::new(config)?;

            println!("{}\n{}", "Intent:".bold(), intent);
            println!("\n{}\n{}", "Description:".bold(), description);

            let verdict = verifier.judge(&intent, &description).await?;

            println!("\n{}", "Verdict:".bold());
            match verdict.score_1_to_5 {
                5 => println!("  Score: {}/5", verdict.score_1_to_5.to_string().green()),
                3..=4 => println!("  Score: {}/5", verdict.score_1_to_5.to_string().yellow()),
                _ => println!("  Score: {}/5", verdict.score_1_to_5.to_string().red()),
            };
            println!("  Rationale: {}", verdict.rationale);

            println!("\n{}", format!("Tokens used: {}", verdict.tokens_used).dimmed());
        }

        Command::Manifest {
            manifest,
            no_cache,
            out,
        } => {
            let config = if no_cache {
                VerifierConfig::default().with_cache_disabled()
            } else {
                VerifierConfig::default()
            };

            let verifier = Verifier::new(config)?;

            println!("{} {}", "Verifying manifest".bold(), manifest.display());

            let verification = verifier.verify_manifest(&manifest).await?;

            println!("\n{}", "Results:".bold());
            println!("  Journey ID: {}", verification.journey_id);
            println!("  Steps: {}", verification.steps.len());
            println!("  Overall score: {:.2}/5.0", verification.overall_score);
            println!("  Total tokens: {}", verification.total_tokens);

            // Print per-step summary
            println!("\n{}", "Per-step scores:".bold());
            for (i, step) in verification.steps.iter().enumerate() {
                let score_str = step.verdict.score_1_to_5.to_string();
                match step.verdict.score_1_to_5 {
                    5 => println!("  Step {}: {} — {}", i, score_str.green(), step.intent),
                    3..=4 => println!("  Step {}: {} — {}", i, score_str.yellow(), step.intent),
                    _ => println!("  Step {}: {} — {}", i, score_str.red(), step.intent),
                };
            }

            // Write output
            let output_json = serde_json::to_string_pretty(&verification)?;
            let output_path = match out {
                Some(p) => p,
                None => {
                    let manifest_dir = manifest.parent().unwrap_or_else(|| std::path::Path::new("."));
                    manifest_dir.join("manifest.verified.json")
                }
            };

            std::fs::write(&output_path, &output_json)?;
            println!(
                "\n{} {}",
                "Verification written to".green(),
                output_path.display()
            );
        }
    }

    Ok(())
}
