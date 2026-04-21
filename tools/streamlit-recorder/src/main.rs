//! `hwledger-streamlit-recorder` — Rust port of
//! `apps/streamlit/journeys/scripts/record-all.sh`.
//!
//! Boots Streamlit on a free port, waits for `/_stcore/health`, runs Playwright,
//! cleans up the Streamlit child on exit, then converts Playwright videos to
//! mp4 + gif (via `ffmpeg`) and mirrors each manifest into `journeys/manifests/<slug>/manifest.json`.
//!
//! Traces to: scripting policy (Rust-only glue), FR-STREAMLIT-JOURNEYS-001.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::Parser;
use owo_colors::OwoColorize;

#[derive(Debug, Parser)]
#[command(
    name = "hwledger-streamlit-recorder",
    about = "Record all Streamlit Playwright journeys."
)]
struct Cli {
    /// Streamlit port (overrides STREAMLIT_PORT env).
    #[arg(long, env = "STREAMLIT_PORT", default_value_t = 8599)]
    port: u16,
    /// Playwright headless override.
    #[arg(long, env = "HEADLESS", default_value = "0")]
    headless: String,
    /// Max seconds to wait for Streamlit health.
    #[arg(long, default_value_t = 60)]
    health_timeout_s: u64,
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
    let script_dir = find_script_dir()?;
    let journeys_root = script_dir.parent().unwrap().to_path_buf();
    let app_root = journeys_root.parent().unwrap().to_path_buf();
    let recordings_dir = journeys_root.join("recordings");
    let manifests_dir = journeys_root.join("manifests");
    let pw_output_dir = journeys_root.join("playwright-output");
    let streamlit_log = journeys_root.join(".streamlit.log");

    std::fs::create_dir_all(&recordings_dir)?;
    std::fs::create_dir_all(&manifests_dir)?;
    std::fs::create_dir_all(&pw_output_dir)?;

    for tool in ["ffmpeg", "npx"] {
        if which(tool).is_none() {
            bail!("required tool '{tool}' not on PATH");
        }
    }

    let streamlit_url = format!("http://127.0.0.1:{}", cli.port);
    let health_url = format!("{streamlit_url}/_stcore/health");

    eprintln!("{} booting streamlit on {streamlit_url}", "record-all".cyan().bold());
    let mut streamlit = spawn_streamlit(&app_root, cli.port, &streamlit_log)?;

    // RAII guard kills streamlit on any error path.
    let guard = ChildGuard { child: Some(&mut streamlit) };

    eprintln!("{} waiting for {health_url}", "record-all".cyan().bold());
    let healthy = wait_for_health(&health_url, cli.health_timeout_s, guard.pid().unwrap_or(0))?;
    if !healthy {
        tail_log(&streamlit_log, 40);
        bail!("streamlit failed to become healthy within {}s", cli.health_timeout_s);
    }

    eprintln!("{} installing playwright chromium (if needed)", "record-all".cyan().bold());
    install_playwright(&journeys_root)?;

    eprintln!("{} running playwright", "record-all".cyan().bold());
    let mut pw = Command::new("npx");
    pw.arg("--yes")
        .arg("playwright")
        .args(["test", "--config=playwright.config.ts"])
        .env("STREAMLIT_URL", &streamlit_url)
        .env("HEADLESS", &cli.headless)
        .current_dir(&journeys_root);
    run_cmd(&mut pw).context("playwright run failed")?;

    drop(guard); // kills streamlit

    eprintln!("{} converting videos", "record-all".cyan().bold());
    convert_videos(&recordings_dir, &manifests_dir, &pw_output_dir)?;

    eprintln!("{} done", "record-all".green().bold());
    Ok(())
}

struct ChildGuard<'a> {
    child: Option<&'a mut Child>,
}

impl<'a> ChildGuard<'a> {
    fn pid(&self) -> Option<u32> {
        self.child.as_ref().map(|c| c.id())
    }
}

impl<'a> Drop for ChildGuard<'a> {
    fn drop(&mut self) {
        if let Some(child) = self.child.take() {
            if let Some(pid) = Some(child.id()) {
                eprintln!("[record-all] stopping streamlit pid={pid}");
            }
            #[cfg(unix)]
            {
                // SIGTERM for graceful shutdown.
                unsafe {
                    libc::kill(child.id() as i32, libc::SIGTERM);
                }
                // Best-effort wait.
                let _ = child.wait();
            }
            #[cfg(not(unix))]
            {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

fn spawn_streamlit(app_root: &Path, port: u16, log: &Path) -> Result<Child> {
    let log_file = std::fs::File::create(log)?;
    let log_err = log_file.try_clone()?;
    let (cmd, args): (&str, Vec<String>) = if which("uv").is_some() {
        (
            "uv",
            vec![
                "run".into(),
                "streamlit".into(),
                "run".into(),
                "app.py".into(),
                "--server.port".into(),
                port.to_string(),
                "--server.headless".into(),
                "true".into(),
                "--browser.gatherUsageStats".into(),
                "false".into(),
                "--server.runOnSave".into(),
                "false".into(),
            ],
        )
    } else {
        (
            "streamlit",
            vec![
                "run".into(),
                "app.py".into(),
                "--server.port".into(),
                port.to_string(),
                "--server.headless".into(),
                "true".into(),
                "--browser.gatherUsageStats".into(),
                "false".into(),
                "--server.runOnSave".into(),
                "false".into(),
            ],
        )
    };
    let child = Command::new(cmd)
        .args(&args)
        .current_dir(app_root)
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_err))
        .spawn()
        .with_context(|| format!("failed to spawn `{cmd}`"))?;
    Ok(child)
}

fn wait_for_health(url: &str, timeout_s: u64, _pid: u32) -> Result<bool> {
    for i in 1..=timeout_s {
        if ureq::get(url).timeout(Duration::from_secs(2)).call().is_ok() {
            eprintln!("[record-all] streamlit healthy after {i}s");
            return Ok(true);
        }
        thread::sleep(Duration::from_secs(1));
    }
    Ok(false)
}

fn tail_log(path: &Path, n: usize) {
    if let Ok(file) = std::fs::File::open(path) {
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().map_while(|l| l.ok()).collect();
        let start = lines.len().saturating_sub(n);
        for line in &lines[start..] {
            eprintln!("{line}");
        }
    }
}

fn install_playwright(journeys_root: &Path) -> Result<()> {
    let node_modules = journeys_root.join("node_modules");
    if !node_modules.is_dir() {
        if which("bun").is_some() {
            run_cmd(Command::new("bun").arg("install").current_dir(journeys_root))?;
        } else {
            run_cmd(Command::new("npm").arg("install").current_dir(journeys_root))?;
        }
    }
    run_cmd(
        Command::new("npx")
            .args(["--yes", "playwright", "install", "chromium"])
            .current_dir(journeys_root),
    )?;
    Ok(())
}

fn convert_videos(recordings_dir: &Path, manifests_dir: &Path, pw_output_dir: &Path) -> Result<()> {
    let mut webms: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(pw_output_dir) {
        for e in entries.flatten() {
            let candidate = e.path().join("video.webm");
            if candidate.is_file() {
                webms.push(candidate);
            }
        }
    }
    // sort newest-first by mtime
    webms.sort_by_key(|p| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .ok()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });
    webms.reverse();

    if let Ok(entries) = std::fs::read_dir(recordings_dir) {
        for e in entries.flatten() {
            let manifest = e.path().join("manifest.json");
            if !manifest.is_file() {
                continue;
            }
            let slug = e.file_name().to_string_lossy().to_string();
            let keyword = slug.strip_prefix("streamlit-").unwrap_or(&slug).to_string();

            let video = webms.iter().find(|p| {
                p.parent()
                    .and_then(|d| d.file_name())
                    .and_then(|n| n.to_str())
                    .map(|s| s.contains(&keyword))
                    .unwrap_or(false)
            });

            let target_mp4 = e.path().join(format!("{slug}.mp4"));
            let target_gif = e.path().join(format!("{slug}.gif"));

            if let Some(v) = video {
                eprintln!(
                    "[record-all] {slug}: {} -> {} + .gif",
                    v.display(),
                    target_mp4.display()
                );
                run_cmd(
                    Command::new("ffmpeg")
                        .args(["-y", "-loglevel", "error", "-i"])
                        .arg(v)
                        .args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-movflags", "+faststart"])
                        .arg(&target_mp4),
                )?;
                run_cmd(
                    Command::new("ffmpeg")
                        .args(["-y", "-loglevel", "error", "-i"])
                        .arg(v)
                        .args(["-vf", "fps=10,scale=800:-1:flags=lanczos"])
                        .arg(&target_gif),
                )?;
            } else {
                eprintln!(
                    "[record-all] {slug}: no playwright video found; skipping mp4/gif conversion"
                );
            }

            let target_manifest_dir = manifests_dir.join(&slug);
            std::fs::create_dir_all(&target_manifest_dir)?;
            std::fs::copy(&manifest, target_manifest_dir.join("manifest.json"))?;
        }
    }
    Ok(())
}

fn find_script_dir() -> Result<PathBuf> {
    let mut cur = std::env::current_dir()?;
    loop {
        let candidate = cur.join("apps/streamlit/journeys/scripts");
        if candidate.is_dir() {
            return Ok(candidate);
        }
        if !cur.pop() {
            bail!("apps/streamlit/journeys/scripts not found from cwd");
        }
    }
}

fn which(tool: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let c = dir.join(tool);
        if c.is_file() {
            return Some(c);
        }
    }
    None
}

fn run_cmd(cmd: &mut Command) -> Result<()> {
    let status = cmd.status().with_context(|| format!("spawn {:?}", cmd))?;
    if !status.success() {
        bail!("command failed: {:?}", cmd);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_which_cargo() {
        // cargo is guaranteed to be on PATH in CI.
        assert!(which("cargo").is_some());
    }

    #[test]
    fn test_tail_log_missing_file_no_panic() {
        tail_log(Path::new("/nonexistent/path/log.txt"), 10);
    }
}
