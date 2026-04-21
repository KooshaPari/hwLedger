//! Batch driver: walks a root looking for `manifest.verified.json`, derives the
//! journey layout (CLI / Streamlit / GUI) by path convention, stages keyframes
//! into the Remotion `public/` tree, renders each journey, and writes
//! `recording_rich` + `recording_rich_sha256` + `recording_rich_manifest_sha256`
//! back into the manifest JSON.
//!
//! Idempotent: if the current manifest SHA-256 matches the stored
//! `recording_rich_manifest_sha256` and the rich MP4 exists, the journey is
//! skipped.

use std::path::{Path, PathBuf};
use std::time::Instant;

use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::{run, RenderError, RenderPlan};

/// Abstract classification of a journey by its on-disk layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Cli,
    Streamlit,
    Gui,
}

#[derive(Debug)]
pub struct Resolved {
    pub family: Family,
    pub journey_id: String,
    pub manifest_path: PathBuf,
    pub keyframes_src: PathBuf,
    pub output_mp4: PathBuf,
}

/// Classify a `manifest.verified.json` path into a Resolved layout.
/// Returns None if the path isn't under a recognised layout.
pub fn classify(manifest_path: &Path, root: &Path) -> Option<Resolved> {
    let rel = manifest_path.strip_prefix(root).ok()?;
    let parts: Vec<&str> = rel.iter().filter_map(|s| s.to_str()).collect();

    // CLI: cli-journeys/manifests/<id>/manifest.verified.json
    if parts.len() >= 4 && parts[0] == "cli-journeys" && parts[1] == "manifests" {
        let id = parts[2].to_string();
        let base = root.join("cli-journeys");
        return Some(Resolved {
            family: Family::Cli,
            journey_id: id.clone(),
            manifest_path: manifest_path.to_path_buf(),
            keyframes_src: base.join("keyframes").join(&id),
            output_mp4: base.join("recordings").join(&id).join(format!("{id}.rich.mp4")),
        });
    }

    // Streamlit: streamlit-journeys/manifests/<id>/manifest.verified.json
    if parts.len() >= 4 && parts[0] == "streamlit-journeys" && parts[1] == "manifests" {
        let id = parts[2].to_string();
        let base = root.join("streamlit-journeys");
        // Streamlit keyframes live in the recording dir (frame-001.png etc).
        return Some(Resolved {
            family: Family::Streamlit,
            journey_id: id.clone(),
            manifest_path: manifest_path.to_path_buf(),
            keyframes_src: base.join("recordings").join(&id),
            output_mp4: base.join("recordings").join(&id).join(format!("{id}.rich.mp4")),
        });
    }

    // GUI: gui-journeys/<id>/manifest.verified.json
    if parts.len() >= 3 && parts[0] == "gui-journeys" {
        let id = parts[1].to_string();
        let base = root.join("gui-journeys").join(&id);
        return Some(Resolved {
            family: Family::Gui,
            journey_id: id.clone(),
            manifest_path: manifest_path.to_path_buf(),
            keyframes_src: base.join("keyframes"),
            output_mp4: base.join(format!("{id}.rich.mp4")),
        });
    }

    None
}

fn sha256_file(p: &Path) -> Result<String, std::io::Error> {
    let data = std::fs::read(p)?;
    let mut h = Sha256::new();
    h.update(&data);
    Ok(hex::encode(h.finalize()))
}

fn sha256_bytes(b: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b);
    hex::encode(h.finalize())
}

/// Hash manifest content ignoring the enrichment fields we write back, so that
/// a just-written manifest still matches on the next run.
fn canonical_manifest_hash(path: &Path) -> Result<String, std::io::Error> {
    let raw = std::fs::read(path)?;
    let mut v: serde_json::Value = serde_json::from_slice(&raw)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    if let Some(obj) = v.as_object_mut() {
        obj.remove("recording_rich");
        obj.remove("recording_rich_sha256");
        obj.remove("recording_rich_manifest_sha256");
    }
    // Serialise canonicalised (sorted via serde_json::Value's BTreeMap-like
    // behaviour is not guaranteed — use to_vec which preserves insertion).
    // For idempotency we only need determinism across runs of this same code.
    let canonical = serde_json::to_vec(&v)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(sha256_bytes(&canonical))
}

/// Stage keyframes from `src_dir` into `remotion_root/public/keyframes/<id>/`
/// so that `staticFile("keyframes/<id>/frame-001.png")` resolves.
fn stage_keyframes(
    src_dir: &Path,
    remotion_root: &Path,
    journey_id: &str,
) -> Result<PathBuf, std::io::Error> {
    let dst = remotion_root.join("public").join("keyframes").join(journey_id);
    std::fs::create_dir_all(&dst)?;
    if src_dir.exists() {
        for entry in std::fs::read_dir(src_dir)? {
            let e = entry?;
            let name = e.file_name();
            let name_s = name.to_string_lossy();
            if name_s.ends_with(".png") {
                let to = dst.join(&name);
                // best-effort copy (overwrite existing)
                let _ = std::fs::remove_file(&to);
                std::fs::copy(e.path(), &to)?;
            }
        }
    }
    Ok(dst)
}

/// Patch `recording_rich`, `recording_rich_sha256`, and
/// `recording_rich_manifest_sha256` into the manifest JSON at `path`.
fn write_manifest_enrichment(
    path: &Path,
    recording_rich_rel: &str,
    rich_sha: &str,
    manifest_hash: &str,
) -> Result<(), std::io::Error> {
    let raw = std::fs::read(path)?;
    let mut v: serde_json::Value = serde_json::from_slice(&raw)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    if let Some(obj) = v.as_object_mut() {
        obj.insert("recording_rich".into(), serde_json::Value::String(recording_rich_rel.into()));
        obj.insert("recording_rich_sha256".into(), serde_json::Value::String(rich_sha.into()));
        obj.insert(
            "recording_rich_manifest_sha256".into(),
            serde_json::Value::String(manifest_hash.into()),
        );
    }
    let pretty = serde_json::to_string_pretty(&v)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, format!("{pretty}\n"))?;
    Ok(())
}

fn recording_rich_relpath(resolved: &Resolved) -> String {
    match resolved.family {
        Family::Cli => {
            format!("recordings/{id}/{id}.rich.mp4", id = resolved.journey_id)
        }
        Family::Streamlit => {
            format!("recordings/{id}/{id}.rich.mp4", id = resolved.journey_id)
        }
        Family::Gui => format!("{id}.rich.mp4", id = resolved.journey_id),
    }
}

pub fn render_all(
    root: &Path,
    remotion_root: &Path,
    force: bool,
    voiceover: &str,
) -> Result<(), anyhow::Error> {
    let mut manifests: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if entry.file_name() == "manifest.verified.json" {
            manifests.push(entry.path().to_path_buf());
        }
    }
    manifests.sort();

    let mut rendered = 0usize;
    let mut skipped = 0usize;
    let mut failed: Vec<(String, String)> = Vec::new();
    let batch_start = Instant::now();

    for m in &manifests {
        let resolved = match classify(m, root) {
            Some(r) => r,
            None => {
                eprintln!("[skip:unknown-layout] {}", m.display());
                continue;
            }
        };

        let manifest_hash = canonical_manifest_hash(&resolved.manifest_path)?;

        // Skip check
        if !force {
            let raw = std::fs::read(&resolved.manifest_path)?;
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&raw) {
                let stored =
                    v.get("recording_rich_manifest_sha256").and_then(|x| x.as_str()).unwrap_or("");
                if stored == manifest_hash && resolved.output_mp4.exists() {
                    println!(
                        "[skip] {:9} {:<28} (hash match, rich exists)",
                        format!("{:?}", resolved.family).to_lowercase(),
                        resolved.journey_id
                    );
                    skipped += 1;
                    continue;
                }
            }
        }

        // Stage keyframes (best-effort — GUI journeys without steps render fine with no frames).
        let staged =
            match stage_keyframes(&resolved.keyframes_src, remotion_root, &resolved.journey_id) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("[warn] stage keyframes failed for {}: {}", resolved.journey_id, e);
                    remotion_root.join("public").join("keyframes").join(&resolved.journey_id)
                }
            };

        // Ensure output dir exists.
        if let Some(parent) = resolved.output_mp4.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut plan = RenderPlan::new(
            resolved.journey_id.clone(),
            resolved.manifest_path.clone(),
            staged,
            remotion_root.to_path_buf(),
            resolved.output_mp4.clone(),
        );
        plan.voiceover = voiceover.to_string();

        let t0 = Instant::now();
        match run(&plan) {
            Ok(_) => {
                let dt = t0.elapsed();
                let rich_sha = sha256_file(&resolved.output_mp4).unwrap_or_default();
                let size = std::fs::metadata(&resolved.output_mp4).map(|m| m.len()).unwrap_or(0);
                let rel = recording_rich_relpath(&resolved);
                if let Err(e) = write_manifest_enrichment(
                    &resolved.manifest_path,
                    &rel,
                    &rich_sha,
                    &manifest_hash,
                ) {
                    eprintln!(
                        "[warn] manifest writeback failed for {}: {}",
                        resolved.journey_id, e
                    );
                }
                println!(
                    "[ok]   {:9} {:<28} {:>7.2}s  {:>7} KB",
                    format!("{:?}", resolved.family).to_lowercase(),
                    resolved.journey_id,
                    dt.as_secs_f64(),
                    size / 1024
                );
                rendered += 1;
            }
            Err(e) => {
                let msg = match e {
                    RenderError::RenderFailed { code, stderr } => {
                        format!(
                            "render fail code={code} stderr={}",
                            stderr.lines().last().unwrap_or("")
                        )
                    }
                    other => format!("{other}"),
                };
                eprintln!("[FAIL] {:<28} {}", resolved.journey_id, msg);
                failed.push((resolved.journey_id.clone(), msg));
            }
        }
    }

    println!(
        "\n--- summary: rendered={rendered} skipped={skipped} failed={} total={} wall={:.1}s ---",
        failed.len(),
        manifests.len(),
        batch_start.elapsed().as_secs_f64()
    );
    for (id, msg) in &failed {
        eprintln!("  FAIL {id}: {msg}");
    }
    if !failed.is_empty() {
        anyhow::bail!("{} journey(s) failed", failed.len());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_cli() {
        let root = PathBuf::from("/r/docs-site/public");
        let m = root.join("cli-journeys/manifests/plan-deepseek/manifest.verified.json");
        let r = classify(&m, &root).unwrap();
        assert_eq!(r.family, Family::Cli);
        assert_eq!(r.journey_id, "plan-deepseek");
        assert!(r.output_mp4.ends_with("recordings/plan-deepseek/plan-deepseek.rich.mp4"));
    }

    #[test]
    fn classify_streamlit() {
        let root = PathBuf::from("/r/docs-site/public");
        let m = root.join("streamlit-journeys/manifests/streamlit-planner/manifest.verified.json");
        let r = classify(&m, &root).unwrap();
        assert_eq!(r.family, Family::Streamlit);
        assert_eq!(r.journey_id, "streamlit-planner");
    }

    #[test]
    fn classify_gui() {
        let root = PathBuf::from("/r/docs-site/public");
        let m = root.join("gui-journeys/planner-gui-launch/manifest.verified.json");
        let r = classify(&m, &root).unwrap();
        assert_eq!(r.family, Family::Gui);
        assert_eq!(r.journey_id, "planner-gui-launch");
        assert!(r.output_mp4.ends_with("planner-gui-launch.rich.mp4"));
    }
}
