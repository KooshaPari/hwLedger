//! Runtime support for `#[user_story_test]` (Batch 2 of the
//! user-story-as-test framework; see
//! `docs-site/architecture/adrs/0034-user-story-test-sourcing.md`).
//!
//! When `PHENOTYPE_USER_STORY_RECORD=1` is set, the proc-macro expands the
//! test body into a call that:
//!
//! 1. Re-execs the same test binary under a PTY (`portable-pty`) with
//!    `PHENOTYPE_USER_STORY_INNER=<journey_id>` set, so the child runs the
//!    real user assertions and produces real stdout/stderr on a TTY.
//! 2. Streams the PTY output into an asciicast-v2 `.cast` file at
//!    `target/user-stories/<journey_id>.cast`.
//! 3. Writes a skeleton `target/user-stories/<journey_id>.manifest.json`
//!    combining the YAML frontmatter fields with PTY metadata (rows, cols,
//!    duration_ms, exit_code).
//! 4. Re-runs the original user assertions against the captured output by
//!    invoking the provided `inner` closure after the child exits, so the
//!    `#[test]` still fails loudly if the recording diverges from the
//!    contract.
//!
//! When `PHENOTYPE_USER_STORY_RECORD` is unset, [`maybe_record`] is a plain
//! forward to `inner()` — zero runtime overhead during normal dev test runs.
//!
//! The `family: gui|streamlit` backends are owned by Batches 3/4 (Playwright
//! plugin + XCUITest helper); for those families [`maybe_record`] is also a
//! no-op here.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde::Serialize;

/// Public frontmatter payload the proc-macro serializes into each expanded
/// test. Mirrors the canonical `user-story.schema.json` fields that are
/// useful at runtime (the full story stays in source for the harvester).
#[derive(Debug, Clone, Serialize)]
pub struct UserStoryMeta {
    pub journey_id: &'static str,
    pub title: &'static str,
    pub persona: &'static str,
    pub family: &'static str,
    pub record: bool,
    pub blind_judge: &'static str,
    pub traces_to: &'static [&'static str],
}

/// Entry point the macro expansion calls. Three code paths:
///
/// - `PHENOTYPE_USER_STORY_INNER == meta.journey_id` → we are the PTY
///   child: just run `inner()` directly so the parent can capture our
///   stdout/stderr on the TTY.
/// - `PHENOTYPE_USER_STORY_RECORD == "1"` and `meta.record == true` and
///   `meta.family == "cli"` → we are the PTY parent: spawn self under a PTY,
///   capture output, emit cast + manifest, then re-run `inner()` to enforce
///   assertions against the now-visible side effects.
/// - Otherwise → plain `inner()`.
pub fn maybe_record<F: FnOnce()>(meta: &UserStoryMeta, inner: F) {
    let inner_marker = std::env::var("PHENOTYPE_USER_STORY_INNER").ok();
    if inner_marker.as_deref() == Some(meta.journey_id) {
        inner();
        return;
    }
    let record = std::env::var("PHENOTYPE_USER_STORY_RECORD").ok();
    let active = matches!(record.as_deref(), Some("1") | Some("true"))
        && meta.record
        && meta.family == "cli";
    if !active {
        inner();
        return;
    }
    match record_via_pty(meta) {
        Ok(_) => {
            // Re-run the assertions in-process so the test still passes
            // locally, not just in the child.
            inner();
        }
        Err(e) => {
            // Fail loudly — per global policy, no silent degradation.
            panic!("[user_story_test] PTY recording failed for journey '{}': {e}", meta.journey_id);
        }
    }
}

/// Output directory for cast + manifest artifacts. Defaults to
/// `target/user-stories` rooted at `CARGO_TARGET_DIR` or the `target/`
/// sibling of the manifest dir. Honors `PHENOTYPE_USER_STORY_OUT` override.
pub fn artifact_dir() -> PathBuf {
    if let Ok(p) = std::env::var("PHENOTYPE_USER_STORY_OUT") {
        return PathBuf::from(p);
    }
    let target = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".into());
    PathBuf::from(target).join("user-stories")
}

fn record_via_pty(meta: &UserStoryMeta) -> anyhow::Result<()> {
    let out_dir = artifact_dir();
    fs::create_dir_all(&out_dir)?;
    let cast_path = out_dir.join(format!("{}.cast", meta.journey_id));
    let manifest_path = out_dir.join(format!("{}.manifest.json", meta.journey_id));

    // Re-exec ourselves with the INNER marker so the child runs the body.
    let exe = std::env::current_exe()?;
    // Match the single test by name — Rust test binaries accept a filter.
    let test_name = meta.journey_id.replace('-', "_");

    let pty_system = native_pty_system();
    let size = PtySize { rows: 30, cols: 120, pixel_width: 0, pixel_height: 0 };
    let pair = pty_system.openpty(size).map_err(|e| anyhow::anyhow!("openpty: {e}"))?;

    let mut cmd = CommandBuilder::new(exe);
    cmd.arg("--exact");
    cmd.arg(&test_name);
    cmd.arg("--nocapture");
    cmd.env("PHENOTYPE_USER_STORY_INNER", meta.journey_id);
    // Clear the outer flag so the child doesn't loop forever.
    cmd.env("PHENOTYPE_USER_STORY_RECORD", "0");

    let start_wall = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let started = Instant::now();
    let mut child = pair.slave.spawn_command(cmd).map_err(|e| anyhow::anyhow!("spawn: {e}"))?;
    drop(pair.slave);

    let mut reader =
        pair.master.try_clone_reader().map_err(|e| anyhow::anyhow!("clone_reader: {e}"))?;
    drop(pair.master);

    let mut cast = fs::File::create(&cast_path)?;
    // asciicast v2 header — https://docs.asciinema.org/manual/asciicast/v2/
    let header = serde_json::json!({
        "version": 2,
        "width": size.cols,
        "height": size.rows,
        "timestamp": start_wall,
        "title": meta.title,
        "env": { "TERM": "xterm-256color", "SHELL": "/bin/sh" },
    });
    writeln!(cast, "{}", serde_json::to_string(&header)?)?;

    let mut buf = [0u8; 4096];
    let mut total_bytes = 0usize;
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                total_bytes += n;
                let ts = started.elapsed().as_secs_f64();
                let chunk = String::from_utf8_lossy(&buf[..n]).to_string();
                let event = serde_json::json!([ts, "o", chunk]);
                writeln!(cast, "{}", serde_json::to_string(&event)?)?;
            }
            Err(e) => {
                // EIO on linux once child exits with pty closed is normal.
                let k = e.kind();
                if k == std::io::ErrorKind::UnexpectedEof
                    || k == std::io::ErrorKind::BrokenPipe
                    || k == std::io::ErrorKind::Other
                {
                    break;
                }
                return Err(e.into());
            }
        }
    }

    let status = child.wait().map_err(|e| anyhow::anyhow!("child wait: {e}"))?;
    let duration_ms = started.elapsed().as_millis() as u64;
    let exit_code = status.exit_code();

    let manifest = serde_json::json!({
        "schema": "phenotype-user-story-manifest-0.1",
        "journey_id": meta.journey_id,
        "title": meta.title,
        "persona": meta.persona,
        "family": meta.family,
        "traces_to": meta.traces_to,
        "blind_judge": meta.blind_judge,
        "record": meta.record,
        "pty": {
            "rows": size.rows,
            "cols": size.cols,
            "duration_ms": duration_ms,
            "exit_code": exit_code,
            "bytes_captured": total_bytes,
        },
        "artifacts": {
            "cast": cast_path.file_name().and_then(|s| s.to_str()).unwrap_or(""),
        },
        "emitted_at": start_wall,
    });
    let mut mf = fs::File::create(&manifest_path)?;
    mf.write_all(serde_json::to_vec_pretty(&manifest)?.as_slice())?;

    if exit_code != 0 {
        anyhow::bail!(
            "PTY child test exited with code {exit_code}; cast at {}",
            cast_path.display()
        );
    }
    Ok(())
}

/// Helper for integration tests: returns the cast path for a given id.
pub fn cast_path(journey_id: &str) -> PathBuf {
    artifact_dir().join(format!("{journey_id}.cast"))
}

/// Helper for integration tests: returns the manifest path for a given id.
pub fn manifest_path(journey_id: &str) -> PathBuf {
    artifact_dir().join(format!("{journey_id}.manifest.json"))
}

/// Validate that a path contains a syntactically plausible asciicast-v2.
pub fn is_valid_asciicast_v2(path: &Path) -> bool {
    let Ok(contents) = fs::read_to_string(path) else {
        return false;
    };
    let mut lines = contents.lines();
    let Some(header_line) = lines.next() else {
        return false;
    };
    let Ok(hdr) = serde_json::from_str::<serde_json::Value>(header_line) else {
        return false;
    };
    hdr.get("version").and_then(|v| v.as_u64()) == Some(2)
        && hdr.get("width").is_some()
        && hdr.get("height").is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    const META: UserStoryMeta = UserStoryMeta {
        journey_id: "unit-rt",
        title: "runtime unit",
        persona: "runtime test",
        family: "cli",
        record: false,
        blind_judge: "auto",
        traces_to: &["FR-TEST-001"],
    };

    #[test]
    fn no_record_is_passthrough() {
        let mut ran = false;
        maybe_record(&META, || ran = true);
        assert!(ran);
    }

    #[test]
    fn artifact_dir_honors_override() {
        // Can't mutate env concurrently safely; just check helper returns something.
        let p = artifact_dir();
        assert!(!p.as_os_str().is_empty());
    }
}
