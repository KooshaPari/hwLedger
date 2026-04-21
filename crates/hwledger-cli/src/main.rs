//! Command-line interface for hwLedger operations and diagnostics.
//!
//! Provides commands for:
//! - plan: memory capacity planning with hardware profiling
//! - ingest: model metadata ingestion (HF, GGUF, safetensors, Ollama, LM Studio, MLX)
//! - probe: GPU device discovery and telemetry
//! - fleet: remote server management and audit
//! - version: print version
//! - completions: shell completion generation
//!
//! All commands respect `--log-level` (trace|debug|info|warn|error) and `--no-color`.

mod cmd;
mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::filter::EnvFilter;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// hwLedger CLI — GPU capacity planning, hardware probing, and fleet management.
#[derive(Parser)]
#[command(version = VERSION, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Log level (trace, debug, info, warn, error).
    #[arg(global = true, long, default_value = "warn", env = "HWLEDGER_LOG_LEVEL")]
    log_level: String,

    /// Disable colored output; respects NO_COLOR env.
    #[arg(global = true, long, env = "NO_COLOR")]
    no_color: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Plan GPU memory allocation for model inference.
    Plan(cmd::plan::PlanArgs),

    /// Ingest and analyze model metadata from various sources.
    Ingest(cmd::ingest::IngestArgs),

    /// Probe and monitor GPU devices.
    #[command(subcommand)]
    Probe(cmd::probe::ProbeSubcommand),

    /// Manage fleet servers and agents.
    #[command(subcommand)]
    Fleet(cmd::fleet::FleetSubcommand),

    /// Search Hugging Face Hub for models (anonymous by default).
    #[command(subcommand)]
    Search(cmd::search::SearchSubcommand),

    /// Predict the impact of swapping from a baseline config to a candidate (what-if).
    Predict(cmd::predict::PredictArgs),

    /// Resolve a Planner input string (file path, HF repo-id, HF URL,
    /// `gold:<name>`, or free text) into a structured model source.
    Resolve(cmd::resolve::ResolveArgs),

    /// Local-CI attestation: build / verify / walk the hash-chained manifest log.
    #[command(subcommand)]
    Attest(cmd::attest::AttestSubcommand),

    /// Display version information.
    Version,

    /// Generate shell completions.
    Completions(cmd::completions::CompletionsArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing with the specified log level.
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&cli.log_level));

    tracing_subscriber::fmt().with_env_filter(env_filter).with_writer(std::io::stderr).init();

    // Determine color output (respect --no-color and NO_COLOR env var).
    let use_color = !cli.no_color && atty::is(atty::Stream::Stdout);
    output::set_use_color(use_color);

    match cli.command {
        Commands::Plan(args) => cmd::plan::run(args),
        Commands::Ingest(args) => cmd::ingest::run(args),
        Commands::Probe(subcommand) => cmd::probe::run(subcommand),
        Commands::Fleet(subcommand) => cmd::fleet::run(subcommand),
        Commands::Search(subcommand) => cmd::search::run(subcommand),
        Commands::Predict(args) => cmd::predict::run(args),
        Commands::Resolve(args) => cmd::resolve::run(args),
        Commands::Attest(sub) => cmd::attest::run(sub),
        Commands::Version => {
            println!("hwledger-cli v{}", VERSION);
            Ok(())
        }
        Commands::Completions(args) => cmd::completions::run(args),
    }
}
