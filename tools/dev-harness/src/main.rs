//! `hwledger-dev-harness` CLI entry point.
//!
//! Subcommands:
//! - `up`   — verify toolchain, build workspace, spawn server + docs-site + streamlit.
//! - `down` — terminate services recorded in `~/.hwledger/dev-harness.pid`.
//! - `status` — print recorded service records + liveness.
//!
//! Brief §5 mandates Rust-only glue; bash `scripts/dev.sh` should not exist.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use hwledger_dev_harness::{
    build_workspace, check_optional_tools, check_toolchain, log_dir, pid_file, repo_root,
    spawn_service, teardown, HarnessState, Palette, UpConfig, DEFAULT_CLIENTS,
};
use owo_colors::OwoColorize;

#[derive(Debug, Parser)]
#[command(name = "hwledger-dev-harness", about = "hwLedger dev orchestrator")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Verify toolchain, build artifacts, launch services, tail combined log.
    Up {
        /// Clients to launch. Comma-separated: cli, streamlit, swift, web.
        #[arg(long, value_delimiter = ',', default_values_t = DEFAULT_CLIENTS.iter().map(|s| s.to_string()).collect::<Vec<_>>())]
        clients: Vec<String>,
        /// Port base; server=base+80, streamlit=base+511, web=5173.
        #[arg(long, default_value_t = 8000)]
        port_base: u16,
        /// Skip `cargo build --release` (assume artifacts are current).
        #[arg(long)]
        skip_build: bool,
    },
    /// Kill services recorded in the PID file.
    Down,
    /// Print current service state.
    Status,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Up {
            clients,
            port_base,
            skip_build,
        } => cmd_up(clients, port_base, skip_build),
        Cmd::Down => cmd_down(),
        Cmd::Status => cmd_status(),
    }
}

fn cmd_up(clients: Vec<String>, port_base: u16, skip_build: bool) -> Result<()> {
    let root = repo_root().context("hwLedger repo root not found")?;
    let cfg = UpConfig {
        clients,
        port_base,
        repo_root: root.clone(),
        release: true,
    };

    // 1. Toolchain.
    let missing = check_toolchain();
    if !missing.is_empty() {
        eprintln!("{}", "required tools missing:".red().bold());
        for (tool, hint) in &missing {
            eprintln!("  - {}: {}", tool.yellow(), hint);
        }
        bail!("install the tools above and retry");
    }
    for (tool, hint) in check_optional_tools() {
        eprintln!(
            "{} {} missing ({}); some demos will be disabled.",
            "warn:".yellow().bold(),
            tool,
            hint
        );
    }

    // 2. Build.
    if !skip_build {
        eprintln!("{} cargo build --release (FFI + CLI + server)…", "build:".cyan().bold());
        build_workspace(&cfg)?;
    }

    // 3. Spawn services.
    let mut state = HarnessState::default();
    let mut palette = Palette::default();

    // a. server (HTTP, dev-mode plain)
    {
        let port = cfg.port_base + 80;
        let bin = target_bin(&root, "hwledger-server");
        let args = vec![
            "--port".to_string(),
            port.to_string(),
            "--dev".to_string(),
        ];
        let env: HashMap<String, String> = HashMap::new();
        let rec = spawn_service(&cfg, "server", &bin, &args, &env, &root, Some(port), &mut palette)?;
        state.services.push(rec);
    }

    // b. docs-site (VitePress / bun dev)
    if cfg.clients.iter().any(|c| c == "web") {
        let port = 5173;
        let env: HashMap<String, String> = HashMap::new();
        let rec = spawn_service(
            &cfg,
            "docs-site",
            "bun",
            &[
                "run".into(),
                "dev".into(),
                "--port".into(),
                port.to_string(),
            ],
            &env,
            &root.join("docs-site"),
            Some(port),
            &mut palette,
        )?;
        state.services.push(rec);
    }

    // c. streamlit (with FFI dylib path hint)
    if cfg.clients.iter().any(|c| c == "streamlit") {
        let port = cfg.port_base + 511;
        let dylib = target_dylib(&root);
        let mut env = HashMap::new();
        env.insert("HWLEDGER_FFI_PATH".into(), dylib.display().to_string());
        let rec = spawn_service(
            &cfg,
            "streamlit",
            "uv",
            &[
                "run".into(),
                "--project".into(),
                root.join("apps/streamlit").display().to_string(),
                "streamlit".into(),
                "run".into(),
                root.join("apps/streamlit/app.py").display().to_string(),
                "--server.port".into(),
                port.to_string(),
                "--server.headless".into(),
                "true".into(),
            ],
            &env,
            &root,
            Some(port),
            &mut palette,
        )?;
        state.services.push(rec);
    }

    // d. swift client: informational only; the native macOS app is built via Xcode.
    if cfg.clients.iter().any(|c| c == "swift") {
        eprintln!(
            "{} `swift` client: build via Xcode at apps/macos/HwLedger.xcworkspace. Linking libhwledger_ffi.dylib from {}.",
            "note:".cyan().bold(),
            target_dylib(&root).display()
        );
    }

    // 4. Persist state and tail.
    let pid_path = pid_file()?;
    state.save(&pid_path)?;
    eprintln!(
        "{} {} services running; PID file: {}",
        "ok:".green().bold(),
        state.services.len(),
        pid_path.display()
    );
    for svc in &state.services {
        eprintln!(
            "  {} pid={} port={:?} log={}",
            svc.name.bold(),
            svc.pid,
            svc.port,
            svc.log_path.display()
        );
    }
    eprintln!(
        "{} combined log tail begins below (Ctrl-C to detach; `hwledger-dev-harness down` to stop).",
        "tail:".cyan().bold()
    );
    // The pump threads are already writing to stdout; block forever until the
    // user interrupts. We use a park so the main thread does not spin.
    loop {
        std::thread::park();
    }
}

fn cmd_down() -> Result<()> {
    let pid_path = pid_file()?;
    if !pid_path.exists() {
        eprintln!("{} no PID file at {}", "info:".cyan(), pid_path.display());
        return Ok(());
    }
    let killed = teardown(&pid_path)?;
    eprintln!(
        "{} terminated {} service(s). Logs preserved at {}.",
        "ok:".green().bold(),
        killed.len(),
        log_dir()?.display()
    );
    Ok(())
}

fn cmd_status() -> Result<()> {
    let pid_path = pid_file()?;
    let state = HarnessState::load(&pid_path)?;
    if state.services.is_empty() {
        println!("no services recorded");
        return Ok(());
    }
    for svc in &state.services {
        println!(
            "{:12} pid={:<7} port={:?} log={}",
            svc.name, svc.pid, svc.port, svc.log_path.display()
        );
    }
    Ok(())
}

fn target_bin(root: &std::path::Path, name: &str) -> String {
    root.join("target/release").join(name).display().to_string()
}

fn target_dylib(root: &std::path::Path) -> PathBuf {
    let name = if cfg!(target_os = "macos") {
        "libhwledger_ffi.dylib"
    } else if cfg!(target_os = "windows") {
        "hwledger_ffi.dll"
    } else {
        "libhwledger_ffi.so"
    };
    root.join("target/release").join(name)
}
