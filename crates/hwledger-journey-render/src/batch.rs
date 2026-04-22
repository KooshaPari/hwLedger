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

    // CLI (apps source-of-truth): cli-journeys/manifests/<id>/manifest.verified.json
    // (identical to the docs-site layout but rooted at `apps/`).
    // Handled by the first clause above when <root> is `apps/`.

    // Streamlit (apps source-of-truth):
    // streamlit/journeys/manifests/<id>/manifest.verified.json
    if parts.len() >= 5
        && parts[0] == "streamlit"
        && parts[1] == "journeys"
        && parts[2] == "manifests"
    {
        let id = parts[3].to_string();
        let base = root.join("streamlit").join("journeys");
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

/// Hash manifest content ignoring the enrichment fields we write back AND
/// ignoring the voiceover subtree, so that a voiceover-only tweak does not
/// invalidate the video skip-gate. The voiceover track has its own separate
/// hash (`voiceover_hash`) — see G-005.
fn canonical_manifest_hash(path: &Path) -> Result<String, std::io::Error> {
    let raw = std::fs::read(path)?;
    let mut v: serde_json::Value = serde_json::from_slice(&raw)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    if let Some(obj) = v.as_object_mut() {
        obj.remove("recording_rich");
        obj.remove("recording_rich_sha256");
        obj.remove("recording_rich_manifest_sha256");
        obj.remove("recording_rich_video_sha256");
        obj.remove("recording_audio_voiceover");
        obj.remove("voiceover_sha256");
        // Voiceover content is tracked separately by `voiceover_hash()`.
        obj.remove("voiceover");
        // The top-level `intent` feeds the spoken intro line but is otherwise
        // orthogonal to the video; keep it out of the video hash.
        obj.remove("intent");
        // Per-step voiceover source text — excluded so a Piper line tweak
        // doesn't force a video re-render (G-005). Frame-positioned visual
        // state lives in `annotations`, `screenshot_path`, and `assertions`,
        // which remain hashed.
        if let Some(serde_json::Value::Array(steps)) = obj.get_mut("steps") {
            for step in steps.iter_mut() {
                if let Some(so) = step.as_object_mut() {
                    so.remove("intent");
                    so.remove("description");
                    so.remove("blind_description");
                }
            }
        }
    }
    let canonical = serde_json::to_vec(&v)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(sha256_bytes(&canonical))
}

/// Hash only the voiceover-relevant content: the `voiceover` spec itself plus
/// the per-step text lines that Piper ingests (description > blind_description
/// > intent, matching `synthesise_voiceover_piper`). A change here implies the
/// audio track must be re-synthesised, even if the video can be reused.
fn voiceover_hash(path: &Path) -> Result<String, std::io::Error> {
    let raw = std::fs::read(path)?;
    let v: serde_json::Value = serde_json::from_slice(&raw)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let vo = v.get("voiceover").cloned().unwrap_or(serde_json::Value::Null);
    let mut lines: Vec<String> = Vec::new();
    if let Some(steps) = v.get("steps").and_then(|s| s.as_array()) {
        for step in steps {
            let line = step
                .get("description")
                .and_then(|x| x.as_str())
                .or_else(|| step.get("blind_description").and_then(|x| x.as_str()))
                .or_else(|| step.get("intent").and_then(|x| x.as_str()))
                .unwrap_or("");
            lines.push(line.to_string());
        }
    }
    let intent = v.get("intent").and_then(|x| x.as_str()).unwrap_or("");
    let payload = serde_json::json!({ "voiceover": vo, "intent": intent, "lines": lines });
    let canonical = serde_json::to_vec(&payload)
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

/// Patch rich-recording + hash enrichment fields into the manifest JSON at
/// `path`. Writes both the composite `recording_rich_manifest_sha256` (covers
/// non-audio state) AND the independent `voiceover_sha256` so audio-only
/// edits do not force a video re-render. The silent-video hash is optional
/// (only present when we actually produced a separate silent track).
#[allow(clippy::too_many_arguments)]
fn write_manifest_enrichment(
    path: &Path,
    recording_rich_rel: &str,
    rich_sha: &str,
    manifest_hash: &str,
    voiceover_sha: &str,
    voiceover_audio_rel: Option<&str>,
    video_sha: Option<&str>,
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
        obj.insert("voiceover_sha256".into(), serde_json::Value::String(voiceover_sha.into()));
        if let Some(audio) = voiceover_audio_rel {
            obj.insert("recording_audio_voiceover".into(), serde_json::Value::String(audio.into()));
        }
        if let Some(vsha) = video_sha {
            obj.insert(
                "recording_rich_video_sha256".into(),
                serde_json::Value::String(vsha.into()),
            );
        }
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
    // Canonicalise to absolute: subprocess cwd=remotion_root requires all
    // other paths to be absolute so they resolve correctly.
    let root = std::fs::canonicalize(root)
        .map_err(|e| anyhow::anyhow!("canonicalize root {}: {e}", root.display()))?;
    let remotion_root = std::fs::canonicalize(remotion_root).map_err(|e| {
        anyhow::anyhow!("canonicalize remotion_root {}: {e}", remotion_root.display())
    })?;
    let root = &root;
    let remotion_root = &remotion_root;
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
        let voice_hash = voiceover_hash(&resolved.manifest_path)?;

        // Skip gates. Two independent predicates per G-005:
        //   - manifest_hash  → covers video + non-audio state
        //   - voice_hash     → covers voiceover text + backend only
        // Both matching = full skip. Manifest match but voice drift = audio
        // re-mux only (video track reused via ffmpeg -c:v copy, silent
        // source preserved from prior render).
        let mut audio_only_remix = false;
        if !force {
            let raw = std::fs::read(&resolved.manifest_path)?;
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&raw) {
                let stored_manifest =
                    v.get("recording_rich_manifest_sha256").and_then(|x| x.as_str()).unwrap_or("");
                let stored_voice = v.get("voiceover_sha256").and_then(|x| x.as_str()).unwrap_or("");
                let manifest_match = stored_manifest == manifest_hash;
                let voice_match = stored_voice == voice_hash;
                let rich_exists = resolved.output_mp4.exists();
                if manifest_match && voice_match && rich_exists {
                    println!(
                        "[skip] {:9} {:<28} (video+audio hash match)",
                        format!("{:?}", resolved.family).to_lowercase(),
                        resolved.journey_id
                    );
                    skipped += 1;
                    continue;
                }
                if manifest_match && rich_exists && !voice_match {
                    audio_only_remix = true;
                }
            }
        }

        if audio_only_remix {
            // Re-synthesise voiceover + remux audio track onto the existing
            // rich MP4 without re-rendering the video timeline.
            let silent_cache = silent_cache_path(&resolved.output_mp4);
            match remix_audio_only(&resolved, remotion_root, &silent_cache) {
                Ok((new_rich_sha, voiceover_rel)) => {
                    let rel = recording_rich_relpath(&resolved);
                    if let Err(e) = write_manifest_enrichment(
                        &resolved.manifest_path,
                        &rel,
                        &new_rich_sha,
                        &manifest_hash,
                        &voice_hash,
                        Some(&voiceover_rel),
                        None,
                    ) {
                        eprintln!(
                            "[warn] manifest writeback failed for {} (audio-only): {}",
                            resolved.journey_id, e
                        );
                    }
                    println!(
                        "[audio] {:9} {:<28} (voiceover re-mixed, video reused)",
                        format!("{:?}", resolved.family).to_lowercase(),
                        resolved.journey_id
                    );
                    rendered += 1;
                    continue;
                }
                Err(e) => {
                    eprintln!(
                        "[warn] audio-only remix failed for {} ({}) — falling back to full render",
                        resolved.journey_id, e
                    );
                    // fall through to full render path below
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
                if !resolved.output_mp4.exists() {
                    let msg = format!(
                        "render returned Ok but output missing: {}",
                        resolved.output_mp4.display()
                    );
                    eprintln!("[FAIL] {:<28} {}", resolved.journey_id, msg);
                    failed.push((resolved.journey_id.clone(), msg));
                    continue;
                }
                let rich_sha = sha256_file(&resolved.output_mp4).unwrap_or_default();
                let size = std::fs::metadata(&resolved.output_mp4).map(|m| m.len()).unwrap_or(0);
                if size == 0 {
                    let msg = "output file is empty".to_string();
                    eprintln!("[FAIL] {:<28} {}", resolved.journey_id, msg);
                    failed.push((resolved.journey_id.clone(), msg));
                    continue;
                }
                let rel = recording_rich_relpath(&resolved);
                // Cache a silent copy of the render alongside the rich MP4 so
                // future voiceover-only edits can avoid re-rendering video
                // (see audio_only_remix path above).
                let silent_cache = silent_cache_path(&resolved.output_mp4);
                let _ = cache_silent_copy(&resolved.output_mp4, &silent_cache);
                let voiceover_rel = voiceover_audio_relpath(&resolved.journey_id);
                if let Err(e) = write_manifest_enrichment(
                    &resolved.manifest_path,
                    &rel,
                    &rich_sha,
                    &manifest_hash,
                    &voice_hash,
                    Some(&voiceover_rel),
                    None,
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

/// Location of the cached silent (video-only) render next to the rich MP4.
/// Pattern: `<id>.silent.mp4` so both files share a directory.
fn silent_cache_path(rich_mp4: &Path) -> PathBuf {
    let parent = rich_mp4.parent().map(Path::to_path_buf).unwrap_or_default();
    let stem = rich_mp4.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    // stem already contains `.rich` → strip it so we land on `<id>.silent.mp4`
    let base = stem.strip_suffix(".rich").unwrap_or(&stem);
    parent.join(format!("{base}.silent.mp4"))
}

/// On the first successful full render, cache a video-only (stream-copied)
/// copy so audio-only re-mux can reuse it. Best-effort: failures are logged
/// by callers via the `_ =` discard.
fn cache_silent_copy(rich: &Path, silent: &Path) -> Result<(), std::io::Error> {
    if silent.exists() {
        return Ok(());
    }
    let status = std::process::Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(rich)
        .args(["-an", "-c:v", "copy"])
        .arg(silent)
        .status()?;
    if !status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("ffmpeg silent cache failed for {}", rich.display()),
        ));
    }
    Ok(())
}

fn voiceover_audio_relpath(journey_id: &str) -> String {
    format!("audio/{journey_id}.voiceover.wav")
}

/// Audio-only re-mux: re-synthesise the voiceover via `render_voiceover_only`
/// and mux it onto the cached silent video track with `ffmpeg -c:v copy -c:a aac`.
/// Returns the new rich MP4's SHA-256 and the voiceover relpath on success.
fn remix_audio_only(
    resolved: &Resolved,
    remotion_root: &Path,
    silent_cache: &Path,
) -> Result<(String, String), anyhow::Error> {
    if !silent_cache.exists() {
        anyhow::bail!(
            "silent cache missing ({}); a prior full render must produce it first",
            silent_cache.display()
        );
    }
    // Build a render plan that mirrors the one used for full renders so
    // synthesise_voiceover_piper lays the WAV in the expected location.
    let mut plan = RenderPlan::new(
        resolved.journey_id.clone(),
        resolved.manifest_path.clone(),
        remotion_root.join("public").join("keyframes").join(&resolved.journey_id),
        remotion_root.to_path_buf(),
        resolved.output_mp4.clone(),
    );
    plan.voiceover = "piper".to_string();
    let voiceover_rel = crate::synthesise_voiceover_piper(&plan)
        .map_err(|e| anyhow::anyhow!("voiceover synth: {e}"))?;

    let voiceover_wav = remotion_root.join("public").join(&voiceover_rel);
    if !voiceover_wav.exists() {
        anyhow::bail!("voiceover wav missing after synth: {}", voiceover_wav.display());
    }

    // Mix: ffmpeg -i silent.mp4 -i voiceover.wav -c:v copy -c:a aac -shortest -> rich.mp4
    // Rationale: stream-copy the video, AAC-encode the fresh audio, -shortest
    // prevents a leftover tail when voiceover is shorter than the silent track.
    let tmp = resolved.output_mp4.with_extension("remix.mp4");
    let status = std::process::Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(silent_cache)
        .arg("-i")
        .arg(&voiceover_wav)
        .args(["-c:v", "copy", "-c:a", "aac", "-shortest"])
        .arg(&tmp)
        .status()?;
    if !status.success() {
        anyhow::bail!("ffmpeg remix failed for {}", resolved.journey_id);
    }
    std::fs::rename(&tmp, &resolved.output_mp4)?;
    let rich_sha = sha256_file(&resolved.output_mp4)?;
    Ok((rich_sha, voiceover_rel))
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

    /// G-005: flipping `step.intent` wording must leave the manifest hash
    /// stable (only the voiceover hash changes), so the video skip-gate
    /// holds and we remux audio only.
    #[test]
    fn voiceover_edit_leaves_manifest_hash_stable() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("manifest.verified.json");
        let base = serde_json::json!({
            "id": "test-journey",
            "intent": "original intent",
            "passed": true,
            "keyframe_count": 1,
            "steps": [
                { "index": 0, "slug": "s0", "intent": "say hello", "screenshot_path": "frame-001.png" }
            ],
            "voiceover": { "backend": "piper" }
        });
        std::fs::write(&path, serde_json::to_string_pretty(&base).unwrap()).unwrap();
        let mh1 = canonical_manifest_hash(&path).unwrap();
        let vh1 = voiceover_hash(&path).unwrap();

        // Edit step intent (voiceover source text) — manifest hash must stay put.
        let mut edited = base.clone();
        edited["steps"][0]["intent"] = serde_json::Value::String("say bonjour".into());
        std::fs::write(&path, serde_json::to_string_pretty(&edited).unwrap()).unwrap();
        let mh2 = canonical_manifest_hash(&path).unwrap();
        let vh2 = voiceover_hash(&path).unwrap();

        assert_eq!(mh1, mh2, "video/manifest hash must not change for voiceover-only edits");
        assert_ne!(vh1, vh2, "voiceover hash must change when step text changes");
    }

    /// Conversely, a non-voiceover edit (e.g. keyframe_count) must bump the
    /// manifest hash so a full video re-render is triggered.
    #[test]
    fn video_edit_bumps_manifest_hash() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("manifest.verified.json");
        let base = serde_json::json!({
            "id": "test-journey", "intent": "x", "passed": true,
            "keyframe_count": 1, "steps": [], "voiceover": { "backend": "piper" }
        });
        std::fs::write(&path, serde_json::to_string_pretty(&base).unwrap()).unwrap();
        let mh1 = canonical_manifest_hash(&path).unwrap();
        let mut edited = base.clone();
        edited["keyframe_count"] = serde_json::json!(7);
        std::fs::write(&path, serde_json::to_string_pretty(&edited).unwrap()).unwrap();
        let mh2 = canonical_manifest_hash(&path).unwrap();
        assert_ne!(mh1, mh2);
    }

    #[test]
    fn silent_cache_path_strips_rich_suffix() {
        let p = PathBuf::from("/out/dir/my-journey.rich.mp4");
        assert_eq!(silent_cache_path(&p), PathBuf::from("/out/dir/my-journey.silent.mp4"));
    }
}
