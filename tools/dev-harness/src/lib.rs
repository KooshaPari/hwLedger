//! Library surface for `hwledger-dev-harness`.
//!
//! Exposes the process orchestration primitives so the binary and integration
//! tests share one code path. See `main.rs` for the CLI entry point.
//!
//! Design constraints (brief §5 scripting policy):
//! - Rust only, no bash glue.
//! - Child processes are tracked in a PID file under `~/.hwledger/dev-harness.pid`.
//! - Each service writes to its own log under `~/.hwledger/logs/<service>.log`.
//! - Combined stdout tail is colorized per-service via `owo-colors`.

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{bail, Context, Result};
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

/// Default clients enabled by `hwledger-dev up`.
pub const DEFAULT_CLIENTS: &[&str] = &["cli", "streamlit", "web"];

/// Canonical PID file path (`~/.hwledger/dev-harness.pid`).
pub fn pid_file() -> Result<PathBuf> {
    let base = state_dir()?;
    Ok(base.join("dev-harness.pid"))
}

/// `~/.hwledger/logs/`.
pub fn log_dir() -> Result<PathBuf> {
    let base = state_dir()?;
    let dir = base.join("logs");
    fs::create_dir_all(&dir).ok();
    Ok(dir)
}

fn state_dir() -> Result<PathBuf> {
    if let Ok(overridden) = std::env::var("HWLEDGER_HOME") {
        let p = PathBuf::from(overridden);
        fs::create_dir_all(&p).ok();
        return Ok(p);
    }
    let home = dirs::home_dir().context("could not determine $HOME")?;
    let p = home.join(".hwledger");
    fs::create_dir_all(&p).ok();
    Ok(p)
}

/// Record of a launched service, persisted to the PID file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRecord {
    pub name: String,
    pub pid: u32,
    pub port: Option<u16>,
    pub log_path: PathBuf,
}

/// PID-file payload.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct HarnessState {
    pub services: Vec<ServiceRecord>,
}

impl HarnessState {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Ok(serde_json::from_str(&raw).unwrap_or_default())
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        let raw = serde_json::to_string_pretty(self)?;
        fs::write(path, raw)?;
        Ok(())
    }
}

/// Tools required before launching services.
pub const REQUIRED_TOOLS: &[(&str, &str)] = &[
    ("cargo", "rustup (https://rustup.rs)"),
    ("bun", "https://bun.sh"),
    ("uv", "https://docs.astral.sh/uv/"),
];

/// Optional tools. Their absence prints a warning but does not fail.
pub const OPTIONAL_TOOLS: &[(&str, &str)] = &[
    ("vhs", "charmbracelet/vhs — tape recording"),
    ("ffmpeg", "telemetry/demo capture"),
    ("tesseract", "OCR parity checks"),
];

/// Returns missing required tools, as `(tool, install_hint)` pairs.
pub fn check_toolchain() -> Vec<(String, String)> {
    let mut missing = Vec::new();
    for (tool, hint) in REQUIRED_TOOLS {
        if which(tool).is_none() {
            missing.push(((*tool).to_string(), (*hint).to_string()));
        }
    }
    missing
}

/// Returns absent optional tools for warnings only.
pub fn check_optional_tools() -> Vec<(String, String)> {
    let mut missing = Vec::new();
    for (tool, hint) in OPTIONAL_TOOLS {
        if which(tool).is_none() {
            missing.push(((*tool).to_string(), (*hint).to_string()));
        }
    }
    missing
}

fn which(tool: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(tool);
        if candidate.is_file() {
            return Some(candidate);
        }
        // Windows fallback for .exe
        let with_exe = dir.join(format!("{tool}.exe"));
        if with_exe.is_file() {
            return Some(with_exe);
        }
    }
    None
}

/// Locate the hwLedger repo root. Walks up from CWD looking for `Cargo.toml`
/// adjacent to `crates/hwledger-ffi`.
pub fn repo_root() -> Result<PathBuf> {
    let mut cur = std::env::current_dir()?;
    loop {
        if cur.join("Cargo.toml").is_file() && cur.join("crates/hwledger-ffi").is_dir() {
            return Ok(cur);
        }
        if !cur.pop() {
            bail!("hwLedger repo root not found from {:?}", std::env::current_dir()?);
        }
    }
}

/// Config produced by `hwledger-dev up`.
#[derive(Debug, Clone)]
pub struct UpConfig {
    pub clients: Vec<String>,
    pub port_base: u16,
    pub repo_root: PathBuf,
    pub release: bool,
}

impl UpConfig {
    /// Port for a given service, derived from `port_base`.
    /// - server: `port_base + 80` (e.g. 8080 when port_base=8000)
    /// - streamlit: `port_base + 511` (e.g. 8511 when port_base=8000)
    /// - web: always 5173 (VitePress default)
    pub fn port_for(&self, service: &str) -> u16 {
        match service {
            "server" => self.port_base + 80,
            "streamlit" => self.port_base + 511,
            "web" => 5173,
            _ => self.port_base,
        }
    }
}

/// Build the workspace artifacts needed by enabled clients. Returns build logs
/// directory. Uses `cargo build --release -p ...` for each required crate,
/// in a single cargo invocation (cargo parallelizes internally).
pub fn build_workspace(cfg: &UpConfig) -> Result<()> {
    let mut crates = vec!["hwledger-ffi"]; // always
    if cfg.clients.iter().any(|c| c == "cli") {
        crates.push("hwledger-cli");
    }
    // server is required for streamlit + web demos
    crates.push("hwledger-server");

    let mut args = vec!["build".to_string()];
    if cfg.release {
        args.push("--release".to_string());
    }
    for c in &crates {
        args.push("-p".into());
        args.push((*c).to_string());
    }

    let log_path = log_dir()?.join("build.log");
    let mut log = fs::File::create(&log_path)?;
    writeln!(log, "$ cargo {}", args.join(" "))?;

    let status = Command::new("cargo")
        .args(&args)
        .current_dir(&cfg.repo_root)
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log))
        .status()
        .context("failed to invoke cargo build")?;

    if !status.success() {
        bail!("cargo build failed (see {}). exit code: {:?}", log_path.display(), status.code());
    }
    Ok(())
}

/// Spawn a child process, wiring its stdout/stderr into a per-service log file
/// and a combined colorized tail on the current stdout.
///
/// When the `mock-spawn` feature is enabled, this function spawns a long-running
/// no-op (`sleep 3600`) so integration tests can exercise PID-file and teardown
/// logic without needing real binaries or network ports.
pub fn spawn_service(
    _cfg: &UpConfig,
    name: &str,
    cmd: &str,
    args: &[String],
    env: &HashMap<String, String>,
    cwd: &Path,
    port: Option<u16>,
    palette: &mut Palette,
) -> Result<ServiceRecord> {
    let log_path = log_dir()?.join(format!("{name}.log"));
    let log_file = fs::File::create(&log_path)?;

    #[cfg(feature = "mock-spawn")]
    let (real_cmd, real_args): (String, Vec<String>) = {
        let _ = (cmd, args, cwd, env);
        ("sleep".into(), vec!["3600".into()])
    };
    #[cfg(not(feature = "mock-spawn"))]
    let (real_cmd, real_args): (String, Vec<String>) = (cmd.to_string(), args.to_vec());

    let mut command = Command::new(&real_cmd);
    command.args(&real_args).current_dir(cwd).stdout(Stdio::piped()).stderr(Stdio::piped());
    for (k, v) in env {
        command.env(k, v);
    }

    let mut child: Child = command
        .spawn()
        .with_context(|| format!("failed to spawn `{real_cmd}` for service `{name}`"))?;
    let pid = child.id();
    let color = palette.assign(name);

    // Stream stdout + stderr to both the per-service log and the combined tail.
    if let Some(out) = child.stdout.take() {
        let tag = name.to_string();
        let lf = log_file.try_clone()?;
        let color = color.clone();
        thread::spawn(move || pump(out, lf, &tag, &color));
    }
    if let Some(err) = child.stderr.take() {
        let tag = format!("{name}!err");
        let lf = log_file;
        thread::spawn(move || pump(err, lf, &tag, &color));
    }

    // Detach: we only track the PID; actual process supervision is via SIGTERM
    // from `down` reading the PID file. We intentionally do not call `wait()`.
    std::mem::forget(child);

    Ok(ServiceRecord { name: name.to_string(), pid, port, log_path })
}

fn pump<R: std::io::Read + Send + 'static>(
    source: R,
    mut log_file: fs::File,
    tag: &str,
    color: &ServiceColor,
) {
    let reader = BufReader::new(source);
    for line in reader.lines().map_while(|l| l.ok()) {
        // Log file: no color codes.
        let _ = writeln!(log_file, "{line}");
        // Combined stdout: colorized prefix.
        let prefix = format!("[{tag:>9}]");
        let painted = match color {
            ServiceColor::Cyan => prefix.cyan().to_string(),
            ServiceColor::Green => prefix.green().to_string(),
            ServiceColor::Magenta => prefix.magenta().to_string(),
            ServiceColor::Yellow => prefix.yellow().to_string(),
            ServiceColor::Blue => prefix.blue().to_string(),
            ServiceColor::Red => prefix.red().to_string(),
        };
        println!("{painted} {line}");
    }
}

/// Per-service log prefix color.
#[derive(Debug, Clone)]
pub enum ServiceColor {
    Cyan,
    Green,
    Magenta,
    Yellow,
    Blue,
    Red,
}

/// Round-robin palette assigner so each service gets a distinct color.
#[derive(Debug, Default)]
pub struct Palette {
    assigned: HashMap<String, ServiceColor>,
    cursor: usize,
}

impl Palette {
    pub fn assign(&mut self, name: &str) -> ServiceColor {
        if let Some(c) = self.assigned.get(name) {
            return c.clone();
        }
        let palette = [
            ServiceColor::Cyan,
            ServiceColor::Green,
            ServiceColor::Magenta,
            ServiceColor::Yellow,
            ServiceColor::Blue,
            ServiceColor::Red,
        ];
        let color = palette[self.cursor % palette.len()].clone();
        self.cursor += 1;
        self.assigned.insert(name.to_string(), color.clone());
        color
    }
}

/// Gracefully kill each PID recorded in the PID file, then remove it.
pub fn teardown(pid_path: &Path) -> Result<Vec<u32>> {
    let state = HarnessState::load(pid_path)?;
    let mut killed = Vec::new();
    for svc in &state.services {
        if kill_pid(svc.pid).is_ok() {
            killed.push(svc.pid);
        }
    }
    if pid_path.exists() {
        fs::remove_file(pid_path).ok();
    }
    Ok(killed)
}

#[cfg(unix)]
fn kill_pid(pid: u32) -> Result<()> {
    // SAFETY: `kill(pid, SIGTERM)` is safe with any i32 pid.
    let ret = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
    if ret != 0 {
        let err = std::io::Error::last_os_error();
        // ESRCH means the process already died; treat as success.
        if err.raw_os_error() == Some(libc::ESRCH) {
            return Ok(());
        }
        return Err(anyhow::anyhow!("kill({pid}): {err}"));
    }
    Ok(())
}

#[cfg(not(unix))]
fn kill_pid(pid: u32) -> Result<()> {
    // Windows: use taskkill.
    let status = Command::new("taskkill").args(["/PID", &pid.to_string(), "/F"]).status()?;
    if !status.success() {
        bail!("taskkill /PID {pid} failed: {status}");
    }
    Ok(())
}

/// Shared state used by the binary to coordinate cleanup across services.
pub type HarnessHandle = Arc<Mutex<HarnessState>>;
