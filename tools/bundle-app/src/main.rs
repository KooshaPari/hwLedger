//! `hwledger-bundle-app` — Rust port of `apps/macos/HwLedgerUITests/scripts/bundle-app.sh`.
//!
//! Control flow, arg parsing, and error handling are Rust. We still shell out
//! to platform tools (`swift build`, `codesign`, `install_name_tool`, `spctl`)
//! because those are Apple-provided binaries with no Rust bindings.
//!
//! Traces to: scripting policy (Rust-only glue, §5), FR-UI-JOURNEYS-001.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use clap::Parser;
use owo_colors::OwoColorize;

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
    name = "hwledger-bundle-app",
    about = "Create and codesign a macOS .app bundle for hwLedger."
)]
struct Cli {
    /// Build configuration.
    #[arg(value_enum, default_value = "release")]
    config: Config,
    /// Disable codesigning (otherwise signs with Developer ID Application).
    #[arg(long, default_value_t = false)]
    no_codesign: bool,
    /// Override CFBundleIdentifier.
    #[arg(long, env = "BUNDLE_ID", default_value = "com.kooshapari.hwLedger")]
    bundle_id: String,
    /// Override CFBundleShortVersionString (default: `git describe`).
    #[arg(long, env = "VERSION")]
    version: Option<String>,
    /// Sparkle EdDSA public key (SUPublicEDKey).
    #[arg(long, env = "SPARKLE_PUBLIC_KEY")]
    sparkle_public_key: Option<String>,
    /// Sparkle feed URL (SUFeedURL).
    #[arg(long, env = "SPARKLE_FEED_URL")]
    sparkle_feed_url: Option<String>,
    /// Codesigning identity.
    #[arg(
        long,
        env = "CODESIGN_IDENTITY",
        default_value = "Developer ID Application: Koosha Paridehpour (GCT2BN8WLL)"
    )]
    codesign_identity: String,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    run(cli)
}

fn run(cli: Cli) -> Result<()> {
    let script_dir = script_dir()?;
    let project_root = script_dir.parent().unwrap().to_path_buf(); // HwLedgerUITests
    let hwledger_src = project_root.parent().unwrap().join("HwLedger");
    let build_dir = project_root.parent().unwrap().parent().unwrap().join("build");
    let bundle_dir = build_dir.join("HwLedger.app");

    let repo_root =
        project_root.parent().unwrap().parent().unwrap().parent().unwrap().to_path_buf();

    let version = cli.version.clone().unwrap_or_else(|| git_describe(&repo_root));
    let version = version.trim_start_matches('v').to_string();
    let build_version = git_short_sha(&repo_root);

    let cfg = cli.config.as_str();
    eprintln!("{}", "=== hwLedger Bundle Script ===".bold());
    eprintln!("Configuration:  {cfg}");
    eprintln!("Bundle ID:      {}", cli.bundle_id);
    eprintln!("Version:        {version}");
    eprintln!("Build version:  {build_version}");
    eprintln!("Codesign:       {}", if cli.no_codesign { "disabled" } else { "enabled" });

    // 1. swift build
    eprintln!("{} swift build -c {cfg}", "build:".cyan().bold());
    run_cmd(Command::new("swift").args(["build", "-c", cfg]).current_dir(&hwledger_src))?;

    let exec_path = hwledger_src.join(format!(".build/{cfg}/HwLedgerApp"));
    if !exec_path.is_file() {
        bail!("executable not found at {}", exec_path.display());
    }

    // 2. Bundle structure
    eprintln!("{} creating bundle at {}", "bundle:".cyan().bold(), bundle_dir.display());
    if bundle_dir.exists() {
        std::fs::remove_dir_all(&bundle_dir).ok();
    }
    std::fs::create_dir_all(bundle_dir.join("Contents/MacOS"))?;
    std::fs::create_dir_all(bundle_dir.join("Contents/Resources"))?;

    let exec_dst = bundle_dir.join("Contents/MacOS/HwLedger");
    std::fs::copy(&exec_path, &exec_dst).context("copy executable")?;
    make_executable(&exec_dst)?;

    // 3. Embed Sparkle.framework
    let sparkle_src = hwledger_src.join(format!(".build/{cfg}/Sparkle.framework"));
    if sparkle_src.is_dir() {
        let sparkle_dst_dir = bundle_dir.join("Contents/Frameworks");
        std::fs::create_dir_all(&sparkle_dst_dir)?;
        let sparkle_dst = sparkle_dst_dir.join("Sparkle.framework");
        if sparkle_dst.exists() {
            std::fs::remove_dir_all(&sparkle_dst).ok();
        }
        run_cmd(Command::new("cp").args(["-R"]).arg(&sparkle_src).arg(&sparkle_dst))?;
        // install_name_tool may fail if rpath already present; ignore error.
        let _ = Command::new("install_name_tool")
            .args(["-add_rpath", "@executable_path/../Frameworks"])
            .arg(&exec_dst)
            .status();
        eprintln!("  embedded Sparkle.framework");
    } else {
        eprintln!(
            "{} Sparkle.framework not found at {} — app may crash at launch",
            "warn:".yellow().bold(),
            sparkle_src.display()
        );
    }

    // 4. Info.plist
    let plist = build_info_plist(
        &cli.bundle_id,
        &version,
        &build_version,
        cli.sparkle_feed_url.as_deref(),
        cli.sparkle_public_key.as_deref(),
    );
    std::fs::write(bundle_dir.join("Contents/Info.plist"), plist)?;
    eprintln!("  Info.plist written");

    // 5. Codesign
    if !cli.no_codesign {
        eprintln!("{} codesigning bundle", "sign:".cyan().bold());
        if which("codesign").is_none() {
            bail!("codesign not found — ensure Xcode command-line tools are installed");
        }
        let entitlements = project_root.parent().unwrap().join("HwLedger/entitlements.plist");
        run_cmd(
            Command::new("codesign")
                .args(["--sign", &cli.codesign_identity])
                .args(["--options", "runtime"])
                .arg("--timestamp")
                .args(["--entitlements"])
                .arg(&entitlements)
                .arg("--deep")
                .arg(&bundle_dir),
        )
        .context("codesign failed")?;
        run_cmd(
            Command::new("codesign").args(["--verify", "--strict", "--verbose=2"]).arg(&bundle_dir),
        )
        .context("signature verification failed")?;
        if which("spctl").is_some() {
            let _ =
                Command::new("spctl").args(["-a", "-t", "exec", "-vv"]).arg(&bundle_dir).status();
        }
        eprintln!("  signed + verified");
    } else {
        eprintln!("{} codesigning disabled", "sign:".yellow());
    }

    eprintln!();
    eprintln!("{}", "=== Bundle Complete ===".green().bold());
    eprintln!("Location:   {}", bundle_dir.display());
    eprintln!("Executable: {}", exec_dst.display());
    Ok(())
}

fn script_dir() -> Result<PathBuf> {
    // Hardcoded path relative to repo root; matches the old shell script semantics.
    // We expect to be run from anywhere; locate repo root by walking up.
    let mut cur = std::env::current_dir()?;
    loop {
        let candidate = cur.join("apps/macos/HwLedgerUITests/scripts");
        if candidate.is_dir() {
            return Ok(candidate);
        }
        if !cur.pop() {
            bail!("repo root (containing apps/macos/HwLedgerUITests/scripts) not found");
        }
    }
}

fn git_describe(repo_root: &Path) -> String {
    Command::new("git")
        .args(["describe", "--tags", "--always"])
        .current_dir(repo_root)
        .output()
        .ok()
        .and_then(|o| if o.status.success() { Some(o.stdout) } else { None })
        .and_then(|b| String::from_utf8(b).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "0.1.0".to_string())
}

fn git_short_sha(repo_root: &Path) -> String {
    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(repo_root)
        .output()
        .ok()
        .and_then(|o| if o.status.success() { Some(o.stdout) } else { None })
        .and_then(|b| String::from_utf8(b).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "1".to_string())
}

fn run_cmd(cmd: &mut Command) -> Result<()> {
    let status = cmd.status().with_context(|| format!("failed to spawn {:?}", cmd))?;
    if !status.success() {
        bail!("command failed: {:?} (exit {:?})", cmd, status.code());
    }
    Ok(())
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

fn which(tool: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(tool);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn build_info_plist(
    bundle_id: &str,
    version: &str,
    build_version: &str,
    feed_url: Option<&str>,
    public_key: Option<&str>,
) -> String {
    let mut extras = String::new();
    if let Some(url) = feed_url {
        extras.push_str(&format!(
            "    <key>SUFeedURL</key>\n    <string>{}</string>\n",
            xml_escape(url)
        ));
    }
    if let Some(key) = public_key {
        extras.push_str(&format!(
            "    <key>SUPublicEDKey</key>\n    <string>{}</string>\n",
            xml_escape(key)
        ));
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>HwLedger</string>
    <key>CFBundleIdentifier</key>
    <string>{bundle_id}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>HwLedger</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>{version}</string>
    <key>CFBundleVersion</key>
    <string>{build_version}</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.utilities</string>
    <key>LSMinimumSystemVersion</key>
    <string>14.0</string>
    <key>LSUIElement</key>
    <false/>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
    <key>NSRequiresIPhoneOS</key>
    <false/>
{extras}</dict>
</plist>
"#,
        bundle_id = xml_escape(bundle_id),
        version = xml_escape(version),
        build_version = xml_escape(build_version),
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_info_plist_no_sparkle() {
        let p = build_info_plist("com.test.app", "1.2.3", "abc", None, None);
        assert!(p.contains("<string>com.test.app</string>"));
        assert!(p.contains("<string>1.2.3</string>"));
        assert!(!p.contains("SUFeedURL"));
    }

    #[test]
    fn test_build_info_plist_with_sparkle() {
        let p = build_info_plist(
            "com.test.app",
            "1.2.3",
            "abc",
            Some("https://example.com/feed.xml"),
            Some("public-key"),
        );
        assert!(p.contains("SUFeedURL"));
        assert!(p.contains("SUPublicEDKey"));
    }

    #[test]
    fn test_xml_escape() {
        assert_eq!(xml_escape("a<b>c&d"), "a&lt;b&gt;c&amp;d");
    }
}
