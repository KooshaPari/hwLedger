//! `hwledger-cli-journey-record` — Rust pre-flight + dispatcher for the
//! CLI journey recorder. Replaces the 36-line shell `record-all.sh` so the
//! surviving stub is ≤5 lines (see `docs/engineering/scripting-policy.md`).
//!
//! Responsibilities:
//! 1. Resolve repo root, journeys root, tapes/recordings dirs.
//! 2. Pin the `target/release/hwledger-cli -> hwledger` symlink so tape scripts
//!    that call `hwledger` resolve to the freshly-built binary.
//! 3. `exec` into `phenotype-journey record` with the canonical flags.
//!
//! Traces to: scripting-policy (Rust-first glue), G-004.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};

/// Target platform for the journey-record pipeline. `auto` detects from the
/// host OS (desktop) and probes for `adb` / `xcrun simctl` (mobile).
/// Mobile + WearOS backends are stubbed — see ADR-0036
/// (`docs-site/architecture/adrs/0036-mobile-recording-backends.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Platform {
    Auto,
    Linux,
    Macos,
    Windows,
    Android,
    Ios,
    Wearos,
}

#[derive(Debug, Parser)]
#[command(
    name = "hwledger-cli-journey-record",
    about = "Pre-flight + dispatch to `phenotype-journey record` for CLI journeys.",
    trailing_var_arg = true,
    allow_hyphen_values = true
)]
struct Cli {
    /// Override repo root (auto-detected from CARGO_MANIFEST_DIR by default).
    #[arg(long)]
    repo_root: Option<PathBuf>,
    /// Target platform. `auto` auto-detects (default). Android/iOS/WearOS
    /// are stubbed — see ADR-0036.
    #[arg(long, value_enum, default_value_t = Platform::Auto)]
    platform: Platform,
    /// Extra args forwarded verbatim to `phenotype-journey record`.
    #[arg(trailing_var_arg = true)]
    forward: Vec<OsString>,
}

/// Resolve `Platform::Auto` to a concrete platform by probing the host.
/// TODO(ADR-0036): replace with live probes of `adb devices` and
/// `xcrun simctl list devices booted` once mobile sidecars ship.
fn resolve_platform(p: Platform) -> Platform {
    if p != Platform::Auto {
        return p;
    }
    match std::env::consts::OS {
        "linux" => Platform::Linux,
        "macos" => Platform::Macos,
        "windows" => Platform::Windows,
        // Unknown host — fall back to linux semantics; the shell stub can
        // override via explicit --platform.
        _ => Platform::Linux,
    }
}

/// Dispatch guard for mobile + WearOS. Returns `Ok(())` for shipping
/// desktop targets; returns `Err` with a clear ADR-0036 reference for
/// not-yet-implemented backends (per "fail clearly, not silently" policy).
fn guard_platform(p: Platform) -> Result<()> {
    match p {
        Platform::Auto => unreachable!("resolve_platform should have eliminated Auto"),
        Platform::Linux | Platform::Macos | Platform::Windows => Ok(()),
        Platform::Android => bail!(
            "android backend stubbed — Kotlin APK sidecar ships in a follow-up. See \
             docs-site/architecture/adrs/0036-mobile-recording-backends.md"
        ),
        Platform::Ios => bail!(
            "ios backend stubbed — Swift + WebDriverAgent sidecar ships in a follow-up. \
             See docs-site/architecture/adrs/0036-mobile-recording-backends.md"
        ),
        Platform::Wearos => bail!(
            "wearos backend deferred — revisit when Wear 5+ exposes a desktop-adjacent \
             screen API. See docs-site/architecture/adrs/0036-mobile-recording-backends.md"
        ),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let platform = resolve_platform(cli.platform);
    guard_platform(platform)?;
    let repo_root = cli
        .repo_root
        .or_else(resolve_repo_root_from_env)
        .context("could not resolve hwLedger repo root")?;

    let journeys_root = repo_root.join("apps/cli-journeys");
    let tapes_dir = journeys_root.join("tapes");
    let recordings_dir = journeys_root.join("recordings");
    let summary_path = journeys_root.join("record-summary.json");
    let release_dir = repo_root.join("target/release");

    pin_cli_symlink(&release_dir).context("pin hwledger-cli -> hwledger symlink")?;

    let mut cmd = locate_phenotype_journey()?;
    cmd.arg("record")
        .arg("--tapes-dir")
        .arg(&tapes_dir)
        .arg("--recordings-dir")
        .arg(&recordings_dir)
        .arg("--cwd")
        .arg(&repo_root)
        .arg("--path-prepend")
        .arg(&release_dir)
        .arg("--summary-path")
        .arg(&summary_path)
        .args(&cli.forward);

    // execvp — hand the process off so signal handling + exit code are clean.
    let err = cmd.exec();
    bail!("failed to exec phenotype-journey: {err}")
}

fn resolve_repo_root_from_env() -> Option<PathBuf> {
    // When invoked via the shell stub we sit under <repo>/tools/cli-journey-record,
    // so CARGO_MANIFEST_DIR/../../ is the workspace root.
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from)?;
    let candidate = manifest_dir.join("../..").canonicalize().ok()?;
    if candidate.join("Cargo.toml").is_file() && candidate.join("apps/cli-journeys").is_dir() {
        Some(candidate)
    } else {
        // Fallback: walk up from cwd until we hit an hwLedger workspace marker.
        let mut cur = env::current_dir().ok()?;
        loop {
            if cur.join("apps/cli-journeys").is_dir() && cur.join("Cargo.toml").is_file() {
                return Some(cur);
            }
            if !cur.pop() {
                return None;
            }
        }
    }
}

/// Ensure `target/release/hwledger` points at `hwledger-cli`. Tapes call
/// `hwledger` (short name) but cargo builds the binary as `hwledger-cli`.
fn pin_cli_symlink(release_dir: &Path) -> Result<()> {
    let src = release_dir.join("hwledger-cli");
    let dst = release_dir.join("hwledger");
    if !src.is_file() {
        bail!(
            "hwledger-cli binary not found at {}\nRun: cargo build --release -p hwledger-cli",
            src.display()
        );
    }
    let needs_refresh = match fs::symlink_metadata(&dst) {
        Ok(meta) => {
            let src_mtime = fs::metadata(&src)?.modified()?;
            let dst_mtime = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
            src_mtime > dst_mtime
        }
        Err(_) => true,
    };
    if needs_refresh {
        let _ = fs::remove_file(&dst);
        std::os::unix::fs::symlink("hwledger-cli", &dst)
            .with_context(|| format!("symlink {} -> hwledger-cli", dst.display()))?;
    }
    Ok(())
}

fn locate_phenotype_journey() -> Result<Command> {
    if which("phenotype-journey").is_some() {
        return Ok(Command::new("phenotype-journey"));
    }
    let phenotype_root =
        env::var_os("PHENOTYPE_JOURNEYS_ROOT").map(PathBuf::from).unwrap_or_else(|| {
            PathBuf::from("/Users/kooshapari/CodeProjects/Phenotype/repos/phenotype-journeys")
        });
    let manifest = phenotype_root.join("Cargo.toml");
    if !manifest.is_file() {
        bail!(
            "phenotype-journey not on PATH and {} missing; set PHENOTYPE_JOURNEYS_ROOT",
            manifest.display()
        );
    }
    let mut c = Command::new("cargo");
    c.args(["run", "--quiet", "--manifest-path"]).arg(manifest).args([
        "--bin",
        "phenotype-journey",
        "--",
    ]);
    Ok(c)
}

fn which(bin: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    for dir in env::split_paths(&path) {
        let candidate = dir.join(bin);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    //! Traces to: FR-JOURNEY-00x platform flag (ADR-0036).
    use super::*;
    use clap::Parser;

    #[test]
    fn platform_auto_is_default() {
        let cli = Cli::try_parse_from(["hwledger-cli-journey-record"]).expect("parse");
        assert_eq!(cli.platform, Platform::Auto);
    }

    #[test]
    fn platform_flag_parses_each_variant() {
        for (arg, want) in [
            ("linux", Platform::Linux),
            ("macos", Platform::Macos),
            ("windows", Platform::Windows),
            ("android", Platform::Android),
            ("ios", Platform::Ios),
            ("wearos", Platform::Wearos),
            ("auto", Platform::Auto),
        ] {
            let cli = Cli::try_parse_from(["hwledger-cli-journey-record", "--platform", arg])
                .unwrap_or_else(|e| panic!("parse --platform {arg}: {e}"));
            assert_eq!(cli.platform, want, "--platform {arg}");
        }
    }

    #[test]
    fn platform_flag_rejects_unknown() {
        let err = Cli::try_parse_from(["hwledger-cli-journey-record", "--platform", "symbian"]);
        assert!(err.is_err(), "symbian should be rejected");
    }

    #[test]
    fn resolve_platform_replaces_auto_with_host_os() {
        let resolved = resolve_platform(Platform::Auto);
        assert_ne!(resolved, Platform::Auto);
        // Host must be one of the desktop platforms.
        assert!(matches!(
            resolved,
            Platform::Linux | Platform::Macos | Platform::Windows
        ));
    }

    #[test]
    fn resolve_platform_preserves_explicit() {
        assert_eq!(resolve_platform(Platform::Android), Platform::Android);
        assert_eq!(resolve_platform(Platform::Ios), Platform::Ios);
        assert_eq!(resolve_platform(Platform::Wearos), Platform::Wearos);
    }

    #[test]
    fn guard_platform_allows_desktop() {
        assert!(guard_platform(Platform::Linux).is_ok());
        assert!(guard_platform(Platform::Macos).is_ok());
        assert!(guard_platform(Platform::Windows).is_ok());
    }

    #[test]
    fn guard_platform_rejects_mobile_with_adr_reference() {
        for p in [Platform::Android, Platform::Ios, Platform::Wearos] {
            let err = guard_platform(p).expect_err("must bail");
            let msg = format!("{err}");
            assert!(
                msg.contains("0036"),
                "error for {p:?} should reference ADR-0036: got {msg}"
            );
        }
    }

    #[test]
    fn forward_args_captured_after_platform_flag() {
        let cli = Cli::try_parse_from([
            "hwledger-cli-journey-record",
            "--platform",
            "linux",
            "--dry-run",
            "--verbose",
        ])
        .expect("parse");
        assert_eq!(cli.platform, Platform::Linux);
        assert_eq!(
            cli.forward,
            vec![OsString::from("--dry-run"), OsString::from("--verbose")]
        );
    }
}
