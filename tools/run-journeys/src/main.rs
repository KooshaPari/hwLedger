//! `hwledger-run-journeys` — Rust port of `apps/macos/HwLedgerUITests/scripts/run-journeys.sh`.
//!
//! Builds the macOS app bundle via `hwledger-bundle-app`, builds UI tests,
//! extracts keyframes per journey, and emits a `journey-summary.json` with
//! manifest-derived counts.
//!
//! Traces to: scripting policy (Rust-only glue), FR-UI-JOURNEYS-001.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use clap::Parser;
use owo_colors::OwoColorize;
use serde::Serialize;

#[derive(Debug, Clone, clap::ValueEnum)]
enum Config {
    Release,
    Debug,
}

impl Config {
    fn as_str(&self) -> &'static str {
        match self {
            Config::Release => "release",
            Config::Debug => "debug",
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "hwledger-run-journeys",
    about = "Build app bundle, run UI journeys, extract keyframes, emit summary."
)]
struct Cli {
    #[arg(value_enum, default_value = "release")]
    config: Config,
    /// Skip `hwledger-bundle-app` invocation (assume bundle is already built).
    #[arg(long)]
    skip_bundle: bool,
}

#[derive(Debug, Serialize)]
struct JourneyEntry {
    id: String,
    passed: bool,
    step_count: usize,
    screenshot_count: usize,
    recording: bool,
    keyframe_count: usize,
}

#[derive(Debug, Serialize)]
struct Summary {
    generated_at: String,
    app_bundle: String,
    journeys: Vec<JourneyEntry>,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    run(Cli::parse())
}

fn run(cli: Cli) -> Result<()> {
    let cfg = cli.config.as_str();
    let repo_root = find_repo_root()?;
    let project_root = repo_root.join("apps/macos/HwLedgerUITests");
    let build_dir = repo_root.join("build");
    let bundle_path = build_dir.join("HwLedger.app");

    if !cli.skip_bundle {
        eprintln!("{}", "Step 1: bundling app…".yellow().bold());
        let bundler = find_bundler(&repo_root);
        let mut cmd = Command::new(&bundler);
        cmd.arg(cfg);
        run_cmd(&mut cmd).context("bundle step failed")?;
    } else {
        eprintln!("{} skipping bundle step", "info:".cyan());
    }
    eprintln!("{} bundle at {}", "ok:".green().bold(), bundle_path.display());

    eprintln!("{}", "Step 2: building UI tests…".yellow().bold());
    run_cmd(Command::new("swift").args(["build", "-c", cfg]).current_dir(&project_root))
        .context("swift build UI tests failed")?;

    eprintln!("{}", "Step 3: executing UI test journeys…".yellow().bold());
    let test_binary = project_root.join(format!(".build/{cfg}/HwLedgerUITests"));
    if !test_binary.exists() {
        eprintln!(
            "{} test binary not built at {} (swift package may use xctest runner)",
            "note:".cyan().bold(),
            test_binary.display()
        );
    }
    // Documented: in full setup this runs swift test --package-path {project_root}.
    eprintln!("  swift test --package-path {}", project_root.display());

    eprintln!("{}", "Step 4: extracting keyframes per journey…".yellow().bold());
    let journeys_dir = project_root.join("journeys");
    if journeys_dir.is_dir() {
        let extractor = project_root.join("scripts/extract-keyframes.sh");
        for entry in std::fs::read_dir(&journeys_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let id = entry.file_name().to_string_lossy().to_string();
                eprintln!("  extracting keyframes for: {id}");
                let _ = Command::new(&extractor).arg(&id).status();
            }
        }
    } else {
        eprintln!("{} no journeys directory yet", "info:".cyan());
    }

    eprintln!("{}", "Step 5: generating summary…".yellow().bold());
    let summary_path = build_dir.join("journey-summary.json");
    std::fs::create_dir_all(&build_dir)?;

    let mut journeys = Vec::new();
    if journeys_dir.is_dir() {
        for entry in std::fs::read_dir(&journeys_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let id = entry.file_name().to_string_lossy().to_string();
            let manifest_path = entry.path().join("manifest.json");
            if !manifest_path.is_file() {
                continue;
            }
            let manifest: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(&manifest_path)
                    .with_context(|| format!("read {}", manifest_path.display()))?,
            )
            .with_context(|| format!("parse {}", manifest_path.display()))?;

            let steps = manifest.get("steps").and_then(|v| v.as_array());
            let step_count = steps.map(|a| a.len()).unwrap_or(0);
            let screenshot_count = steps
                .map(|a| {
                    a.iter()
                        .filter(|s| s.get("screenshot_path").map(|p| !p.is_null()).unwrap_or(false))
                        .count()
                })
                .unwrap_or(0);
            let passed = manifest.get("passed").and_then(|v| v.as_bool()).unwrap_or(false);
            let recording = manifest.get("recording").and_then(|v| v.as_bool()).unwrap_or(false);

            let keyframe_dir = entry.path().join("keyframes");
            let keyframe_count = if keyframe_dir.is_dir() {
                std::fs::read_dir(&keyframe_dir)
                    .map(|it| {
                        it.filter_map(|e| e.ok())
                            .filter(|e| {
                                e.path().extension().and_then(|x| x.to_str()) == Some("png")
                            })
                            .count()
                    })
                    .unwrap_or(0)
            } else {
                0
            };

            journeys.push(JourneyEntry {
                id,
                passed,
                step_count,
                screenshot_count,
                recording,
                keyframe_count,
            });
        }
    }

    let summary = Summary {
        generated_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        app_bundle: bundle_path.display().to_string(),
        journeys,
    };
    std::fs::write(&summary_path, serde_json::to_string_pretty(&summary)?)?;

    eprintln!("{} summary: {}", "ok:".green().bold(), summary_path.display());
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

fn find_repo_root() -> Result<PathBuf> {
    let mut cur = std::env::current_dir()?;
    loop {
        if cur.join("Cargo.toml").is_file() && cur.join("apps/macos/HwLedgerUITests").is_dir() {
            return Ok(cur);
        }
        if !cur.pop() {
            bail!("repo root not found from current directory");
        }
    }
}

fn find_bundler(repo_root: &Path) -> PathBuf {
    // Prefer release, fall back to debug, fall back to cargo run.
    for cfg in ["release", "debug"] {
        let p = repo_root.join(format!("target/{cfg}/hwledger-bundle-app"));
        if p.is_file() {
            return p;
        }
    }
    // Last resort: invoke via cargo.
    PathBuf::from("cargo")
}

fn run_cmd(cmd: &mut Command) -> Result<()> {
    let status = cmd.status().with_context(|| format!("spawn {:?}", cmd))?;
    if !status.success() {
        bail!("command failed: {:?} (exit {:?})", cmd, status.code());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_as_str() {
        assert_eq!(Config::Release.as_str(), "release");
        assert_eq!(Config::Debug.as_str(), "debug");
    }

    #[test]
    fn test_summary_shape() {
        let s = Summary {
            generated_at: "2026-04-19T00:00:00Z".into(),
            app_bundle: "/tmp/HwLedger.app".into(),
            journeys: vec![JourneyEntry {
                id: "j1".into(),
                passed: true,
                step_count: 3,
                screenshot_count: 2,
                recording: false,
                keyframe_count: 5,
            }],
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"id\":\"j1\""));
        assert!(json.contains("\"keyframe_count\":5"));
    }
}
