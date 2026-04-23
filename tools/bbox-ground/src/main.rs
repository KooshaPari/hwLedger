//! `bbox-ground` — validation harness for D6 structural-tree-priority.
//!
//! Subcommand: `measure --corpus <dir>` prints a hit-rate table.

use clap::{Parser, Subcommand};
use hwledger_bbox_ground::measure_hit_rate;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "bbox-ground",
    about = "Structural-tree-priority bbox selector + hit-rate harness (D6)."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Walk a corpus and print a structural/ocr/none hit-rate table.
    Measure {
        /// Directory containing `manifest*.json` files (recursive).
        #[arg(long)]
        corpus: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Measure { corpus } => {
            let hr = measure_hit_rate(&corpus);
            println!("bbox-ground hit-rate report");
            println!("corpus: {}", corpus.display());
            println!("---------------------------------------------");
            println!(
                "{:<14} {:>8} {:>8}",
                "tier", "count", "percent"
            );
            println!("---------------------------------------------");
            println!(
                "{:<14} {:>8} {:>7.2}%",
                "structural",
                hr.structural,
                hr.structural_pct()
            );
            println!("{:<14} {:>8} {:>7.2}%", "ocr", hr.ocr, hr.ocr_pct());
            println!("{:<14} {:>8} {:>7.2}%", "none", hr.none, hr.none_pct());
            println!("---------------------------------------------");
            println!("{:<14} {:>8}", "total", hr.total());
            ExitCode::SUCCESS
        }
    }
}
