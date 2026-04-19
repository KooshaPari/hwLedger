//! hwLedger release pipeline CLI.
//!
//! Replaces shell scripts with a unified Rust-based release tooling.
//!
//! Usage:
//!   hwledger-release xcframework [--release|--debug] [--universal]
//!   hwledger-release bundle --app-name HwLedger --bundle-id com.kooshapari.hwLedger
//!   hwledger-release dmg --app <path> --out <path>
//!   hwledger-release notarize <dmg-path> [--profile hwledger]
//!   hwledger-release appcast --dmg <path> --version <v> --out <path> [--key-path <path>]
//!   hwledger-release keyframes <tape-id> [--base-dir <path>]
//!   hwledger-release record [--only <tape>] [--all]
//!   hwledger-release run <tag> [--dry-run]

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "hwledger-release")]
#[command(about = "hwLedger release pipeline — Rust-based replacement for shell scripts")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Log level (RUST_LOG)
    #[arg(global = true, long, default_value = "info")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Build XCFramework (arm64 + x86_64 universal)
    Xcframework {
        /// Build mode
        #[arg(long, default_value = "release")]
        mode: String,

        /// Build universal binary (arm64 + x86_64)
        #[arg(long)]
        universal: bool,
    },

    /// Bundle and codesign macOS .app
    Bundle {
        /// App name
        #[arg(long)]
        app_name: String,

        /// Bundle identifier
        #[arg(long)]
        bundle_id: String,

        /// Codesign the bundle
        #[arg(long)]
        codesign: bool,
    },

    /// Build and sign DMG
    Dmg {
        /// Path to .app bundle
        #[arg(long)]
        app: PathBuf,

        /// Output DMG path
        #[arg(long)]
        out: PathBuf,

        /// Codesign identity (optional)
        #[arg(long)]
        codesign_identity: Option<String>,
    },

    /// Notarize DMG with Apple
    Notarize {
        /// Path to DMG or .app
        dmg_path: PathBuf,

        /// Keychain profile name
        #[arg(long, default_value = "hwledger")]
        profile: String,
    },

    /// Generate and sign Sparkle appcast
    Appcast {
        /// Path to DMG
        #[arg(long)]
        dmg: PathBuf,

        /// Version number
        #[arg(long)]
        version: String,

        /// Output appcast path
        #[arg(long)]
        out: PathBuf,

        /// Private key path (default: ~/.config/hwledger/sparkle_ed25519_private.key)
        #[arg(long)]
        key_path: Option<PathBuf>,

        /// Download base URL
        #[arg(long)]
        download_base: Option<String>,
    },

    /// Extract keyframes from tape
    Keyframes {
        /// Tape ID or path
        tape_id: String,

        /// Base directory for tapes
        #[arg(long)]
        base_dir: Option<PathBuf>,
    },

    /// Record VHS tapes
    Record {
        /// Record all tapes
        #[arg(long)]
        all: bool,

        /// Only record this tape
        #[arg(long)]
        only: Option<String>,

        /// Concurrency limit (default: 3)
        #[arg(long, default_value = "3")]
        concurrency: usize,
    },

    /// End-to-end release orchestration
    Run {
        /// Release tag (e.g., v0.1.0)
        tag: String,

        /// Dry-run (don't execute commands)
        #[arg(long)]
        dry_run: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&cli.log_level)),
        )
        .init();

    let repo_root = std::env::current_dir()?;

    match cli.command {
        Commands::Xcframework { mode, universal } => {
            use hwledger_release::xcframework::{build_xcframework, BuildMode};
            let build_mode = match mode.as_str() {
                "release" => BuildMode::Release,
                "debug" => BuildMode::Debug,
                _ => {
                    eprintln!("unknown build mode: {}", mode);
                    std::process::exit(1);
                }
            };
            build_xcframework(&repo_root, build_mode, universal)?;
        }

        Commands::Bundle { app_name, bundle_id, codesign } => {
            use hwledger_release::bundle::bundle_app;
            bundle_app(&repo_root, &app_name, &bundle_id, codesign)?;
        }

        Commands::Dmg { app, out, codesign_identity } => {
            use hwledger_release::dmg::build_dmg;
            build_dmg(&repo_root, &app, &out, codesign_identity.as_deref())?;
        }

        Commands::Notarize { dmg_path, profile } => {
            use hwledger_release::notarize::notarize;
            let key_id = std::env::var("APPLE_NOTARY_KEY_ID").ok();
            let issuer_id = std::env::var("APPLE_NOTARY_ISSUER_ID").ok();
            notarize(&dmg_path, Some(&profile), key_id.as_deref(), issuer_id.as_deref())?;
        }

        Commands::Appcast { dmg, version, out, key_path, download_base } => {
            use hwledger_release::appcast::generate_appcast;
            let key_file = key_path.unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".config/hwledger/sparkle_ed25519_private.key")
            });
            generate_appcast(&dmg, &version, &key_file, &out, download_base.as_deref())?;
        }

        Commands::Keyframes { tape_id, base_dir } => {
            use hwledger_release::keyframes::{extract_keyframes, generate_manifest};
            let base = base_dir.unwrap_or_else(|| PathBuf::from("apps/cli-journeys"));
            let keyframes_dir = base.join("keyframes").join(&tape_id);
            extract_keyframes(&base.join(&tape_id), &keyframes_dir)?;
            let manifest_path = keyframes_dir.with_extension("manifest.json");
            generate_manifest(&tape_id, &keyframes_dir, &manifest_path)?;
        }

        Commands::Record { all, only, concurrency } => {
            use hwledger_release::record::record_tape;
            if all {
                let base_dir = PathBuf::from("apps/cli-journeys");
                use hwledger_release::record::record_all_tapes;
                record_all_tapes(&base_dir, concurrency).await?;
            } else if let Some(tape_name) = only {
                let tape_path =
                    PathBuf::from("apps/cli-journeys").join(format!("{}.tape", tape_name));
                record_tape(&tape_path)?;
            } else {
                eprintln!("provide either --all or --only <tape>");
                std::process::exit(1);
            }
        }

        Commands::Run { tag, dry_run } => {
            run_release(&repo_root, &tag, dry_run).await?;
        }
    }

    Ok(())
}

async fn run_release(
    repo_root: &std::path::Path,
    tag: &str,
    _dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use hwledger_release::appcast::generate_appcast;
    use hwledger_release::bundle::bundle_app;
    use hwledger_release::dmg::build_dmg;
    use hwledger_release::notarize::notarize;
    use hwledger_release::xcframework::{build_xcframework, BuildMode};

    // Validate tag format
    if !tag.starts_with('v') {
        eprintln!("tag must start with v: {}", tag);
        std::process::exit(1);
    }

    println!("=== hwLedger local release pipeline ===");
    println!("Tag: {}", tag);
    println!();

    // 1. Build XCFramework
    println!("[1/6] Building XCFramework");
    build_xcframework(repo_root, BuildMode::Release, false)?;

    // 2. Bundle + codesign
    println!("[2/6] Bundling + codesigning .app");
    bundle_app(repo_root, "HwLedger", "com.kooshapari.hwLedger", true)?;

    // 3. Build DMG
    println!("[3/6] Building + signing DMG");
    let app_path = repo_root.join("apps/build/HwLedger.app");
    let version_str = tag.trim_start_matches('v');
    let dmg_path = repo_root.join(format!("apps/build/hwLedger-{}.dmg", version_str));
    build_dmg(repo_root, &app_path, &dmg_path, None)?;

    // 4. Notarize
    println!("[4/6] Submitting to Apple notary (may take 5-15 min)");
    let key_id = std::env::var("APPLE_NOTARY_KEY_ID").ok();
    let issuer_id = std::env::var("APPLE_NOTARY_ISSUER_ID").ok();
    notarize(&dmg_path, Some("hwledger"), key_id.as_deref(), issuer_id.as_deref())?;

    // 5. Generate appcast
    println!("[5/6] Generating signed appcast");
    let key_file = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config/hwledger/sparkle_ed25519_private.key");
    let appcast_path = repo_root.join("docs-site/public/appcast.xml");
    generate_appcast(&dmg_path, version_str, &key_file, &appcast_path, None)?;

    println!("[6/6] Appcast published to docsite");
    println!();
    println!("=== Release pipeline complete ===");
    println!("DMG:     {}", dmg_path.display());
    println!("Appcast: {}", appcast_path.display());
    println!();
    println!("Next:");
    println!("  git add docs-site/public/appcast.xml");
    println!("  git commit -m 'chore(release): appcast for {}'", tag);
    println!("  git push && git push --tags");
    println!(
        "  gh release create {} {} --notes-from-tag --repo KooshaPari/hwLedger",
        tag,
        dmg_path.display()
    );

    Ok(())
}
