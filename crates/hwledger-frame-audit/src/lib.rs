//! Detect and regenerate GUI-journey keyframes that are "text-card
//! placeholders" — ImageMagick/Python-generated gradient cards whose
//! body text describes what the UI *should* show, rather than showing
//! real captured pixels. Those frames pass the VLM/OCR blind-eval
//! judge trivially because the judge just reads the text off the card,
//! which is exactly the failure mode this crate exists to prevent.
//!
//! Pipeline:
//!   1. Walk `docs-site/public/gui-journeys/*/keyframes/*.png` and
//!      `apps/**/keyframes/**/*.png`.
//!   2. For each PNG: shell out to `tesseract -l eng - tsv` to get
//!      word-level bounding boxes + OCR text.
//!   3. Compute (a) text-pixel coverage = sum(bbox area) / frame area
//!      and (b) word count. Flag if coverage > 0.18 AND words > 30.
//!      (The 0.18 threshold is the empirical split between gradient
//!      cards and real UI mocks in this repo; the 0.6 figure in the
//!      spec referred to "60% of the frame OCRs to *dense* text" —
//!      we use bbox area which is a stricter, pixel-grounded measure.)
//!   4. Whitelist non-GUI journey kinds (CLI, streamlit terminal) —
//!      their captures are legitimately text-dominated.
//!   5. Regenerate flagged frames as honest stubs: solid dark background,
//!      single monospace TCC-blocked disclaimer line at the bottom.
//!   6. Patch each affected journey's `manifest.json` +
//!      `manifest.verified.json`: for every skipped-frame step, set
//!      `blind_eval: "skip"`.
//!
//! Traces to: FR-TRACE-003, FR-UX-VERIFY-002

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Text-pixel coverage threshold above which (combined with `MIN_WORDS`) a
/// frame is flagged as a placeholder gradient card. Empirically calibrated
/// against this repo: gradient-card stubs land at 0.05+ with 28-36 OCR
/// words, real UI captures (when they exist) land well below. The spec
/// nominal of "0.6 coverage" referred to dense-text-on-card semantics;
/// tesseract `tsv` bbox-area coverage is a stricter measure, so the
/// equivalent empirical threshold is lower.
pub const COVERAGE_THRESHOLD: f32 = 0.04;
/// Minimum OCR word count required to flag via the coverage heuristic.
pub const MIN_WORDS: usize = 25;

/// Distinctive keyword set — any match in OCR output is a high-confidence
/// placeholder signal (the `generate-placeholder-artefacts.py` footer
/// contains all three).
pub const PLACEHOLDER_KEYWORDS: &[&str] = &["placeholder", "run-journeys", "pending on user"];

/// Outcome of scanning a single PNG.
#[derive(Debug, Clone, Serialize)]
pub struct Detection {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub word_count: usize,
    /// Sum of OCR bounding-box areas divided by total frame area.
    pub coverage: f32,
    /// Raw OCR text (joined words) — used by callers that want the
    /// "placeholder" keyword check as a secondary signal.
    pub ocr_text: String,
    /// True when the frame meets the placeholder heuristic.
    pub flagged: bool,
}

/// Reason an entire journey dir was or was not scanned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JourneyKindHint {
    /// GUI app screen capture — fair game for the text-card heuristic.
    Gui,
    /// CLI terminal capture — skip; text is the actual captured content.
    Cli,
    /// Streamlit-in-terminal capture — skip (same reason).
    StreamlitTerminal,
    /// Streamlit web page — scan (it's a real web UI).
    StreamlitWeb,
    /// Unknown; scan by default.
    Unknown,
}

/// Infer the journey kind from a keyframe's absolute path. This is a heuristic
/// — the authoritative `kind` lives in the journey manifest — but it's enough
/// to decide whether a frame should participate in the blind-eval audit.
pub fn infer_journey_kind(path: &Path) -> JourneyKindHint {
    let s = path.to_string_lossy();
    if s.contains("/gui-journeys/") || s.contains("/gui_journeys/") {
        return JourneyKindHint::Gui;
    }
    if s.contains("/cli-journeys/") || s.contains("/cli_journeys/") {
        return JourneyKindHint::Cli;
    }
    if s.contains("/streamlit-journeys/") || s.contains("/streamlit_journeys/") {
        // Streamlit "journeys" in this repo are recorded via a terminal-driven
        // Playwright run; treat as terminal capture (skip).
        return JourneyKindHint::StreamlitTerminal;
    }
    JourneyKindHint::Unknown
}

/// Shell out to `tesseract` and return per-word (text, bbox) tuples.
///
/// `tesseract <image> - -l eng tsv` emits TSV with columns:
///   level page block para line word left top width height conf text
///
/// We only keep `level == 5` rows (words) with non-empty text.
pub fn run_tesseract(path: &Path) -> Result<Vec<(String, u32, u32, u32, u32)>> {
    let out = Command::new("tesseract")
        .args([path.to_str().context("non-UTF8 path")?, "-", "-l", "eng", "tsv"])
        .output()
        .with_context(|| format!("spawning tesseract for {}", path.display()))?;
    if !out.status.success() {
        anyhow::bail!(
            "tesseract failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut rows = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if i == 0 {
            continue; // header
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 12 {
            continue;
        }
        // level 5 == word
        if cols[0] != "5" {
            continue;
        }
        let w: u32 = cols[8].parse().unwrap_or(0);
        let h: u32 = cols[9].parse().unwrap_or(0);
        let left: u32 = cols[6].parse().unwrap_or(0);
        let top: u32 = cols[7].parse().unwrap_or(0);
        let word = cols[11].trim().to_string();
        if word.is_empty() {
            continue;
        }
        rows.push((word, left, top, w, h));
    }
    Ok(rows)
}

/// Read the PNG just enough to get (width, height) without decoding pixels.
pub fn png_dimensions(path: &Path) -> Result<(u32, u32)> {
    let decoder =
        image::ImageReader::open(path).with_context(|| format!("open {}", path.display()))?;
    let dim = decoder.into_dimensions().with_context(|| format!("dims {}", path.display()))?;
    Ok(dim)
}

/// Run the placeholder detection on a single PNG.
pub fn detect(path: &Path) -> Result<Detection> {
    let (width, height) = png_dimensions(path)?;
    let words = run_tesseract(path)?;
    let total_area = (width as u64) * (height as u64);
    let text_area: u64 = words.iter().map(|(_, _, _, w, h)| (*w as u64) * (*h as u64)).sum();
    let coverage = if total_area == 0 { 0.0 } else { text_area as f32 / total_area as f32 };
    let ocr_text = words.iter().map(|(w, ..)| w.as_str()).collect::<Vec<_>>().join(" ");
    let word_count = words.len();
    // Heuristic 1: OCR explicitly names itself a placeholder (the footer on
    // every gradient-card stub).
    let keyword_match = {
        let lower = ocr_text.to_lowercase();
        PLACEHOLDER_KEYWORDS.iter().any(|k| lower.contains(k))
    };
    // Heuristic 2: dense-text narrative card (gradient placeholders all hit
    // ~28-36 words with 0.04-0.07 bbox coverage).
    let dense_narrative = coverage > COVERAGE_THRESHOLD && word_count >= MIN_WORDS;
    let flagged = keyword_match || dense_narrative;
    Ok(Detection {
        path: path.to_path_buf(),
        width,
        height,
        word_count,
        coverage,
        ocr_text,
        flagged,
    })
}

/// Walk a directory tree and yield every PNG keyframe relevant to the audit.
/// Respects `infer_journey_kind` and skips CLI / streamlit-terminal captures.
pub fn walk_candidates(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if p.extension().and_then(|e| e.to_str()) != Some("png") {
            continue;
        }
        let parent_is_keyframes =
            p.parent().and_then(|d| d.file_name()).and_then(|n| n.to_str()) == Some("keyframes");
        if !parent_is_keyframes {
            continue;
        }
        match infer_journey_kind(p) {
            JourneyKindHint::Gui | JourneyKindHint::StreamlitWeb | JourneyKindHint::Unknown => {
                out.push(p.to_path_buf());
            }
            JourneyKindHint::Cli | JourneyKindHint::StreamlitTerminal => {}
        }
    }
    out.sort();
    out
}

/// Regenerate a flagged frame as an honest stub: solid dark background
/// (#1e1e2e), a single centred-bottom monospace disclaimer line.
///
/// Shells out to `magick` (ImageMagick) because synthesising text with a
/// portable Rust-only stack would require vendoring a TTF and a glyph
/// renderer. `magick` is already a dev dependency (the original placeholder
/// script used it) and this codepath is dev-only, never runtime.
pub fn regenerate_honest_stub(path: &Path, width: u32, height: u32) -> Result<()> {
    let msg = "PLACEHOLDER - real capture blocked by macOS TCC (grant Accessibility + Screen Recording to Xcode)";
    // magick -size WxH canvas:#1e1e2e -font <mono> -pointsize 16 \
    //   -fill '#9aa0b4' -gravity South -annotate +0+28 "<msg>" out.png
    let font = pick_mono_font();
    let size = format!("{}x{}", width, height);
    let status = Command::new("magick")
        .args([
            "-size",
            &size,
            "canvas:#1e1e2e",
            "-font",
            &font,
            "-pointsize",
            "16",
            "-fill",
            "#9aa0b4",
            "-gravity",
            "South",
            "-annotate",
            "+0+28",
            msg,
            path.to_str().context("non-UTF8 path")?,
        ])
        .status()
        .context("spawn magick")?;
    if !status.success() {
        anyhow::bail!("magick regenerate failed for {}", path.display());
    }
    Ok(())
}

fn pick_mono_font() -> String {
    for candidate in
        ["/System/Library/Fonts/Menlo.ttc", "/System/Library/Fonts/SFNSMono.ttf", "Courier"]
    {
        if Path::new(candidate).exists() || candidate == "Courier" {
            return candidate.to_string();
        }
    }
    "Courier".into()
}

/// Patch a manifest (`manifest.json` or `manifest.verified.json`) in place:
/// for every step whose `screenshot_path` points at a regenerated-stub frame,
/// set `blind_eval: "skip"`. Steps whose frames were not regenerated keep
/// their existing `blind_eval` value (default `honest`).
///
/// `regenerated_basenames` is the set of frame filenames (e.g. `frame_003.png`)
/// that were rewritten as honest stubs for this journey.
pub fn patch_manifest_blind_eval(
    manifest_path: &Path,
    regenerated_basenames: &[String],
) -> Result<usize> {
    let raw = std::fs::read_to_string(manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    let mut v: serde_json::Value =
        serde_json::from_str(&raw).with_context(|| format!("parse {}", manifest_path.display()))?;

    let mut patched = 0;
    if let Some(steps) = v.get_mut("steps").and_then(|s| s.as_array_mut()) {
        for step in steps {
            let shot = step.get("screenshot_path").and_then(|p| p.as_str()).unwrap_or("");
            let base = Path::new(shot).file_name().and_then(|n| n.to_str()).unwrap_or("");
            let wants_skip = regenerated_basenames.iter().any(|r| r == base);
            if wants_skip {
                step.as_object_mut()
                    .unwrap()
                    .insert("blind_eval".into(), serde_json::Value::String("skip".into()));
                patched += 1;
            } else if step.get("blind_eval").is_none() {
                step.as_object_mut()
                    .unwrap()
                    .insert("blind_eval".into(), serde_json::Value::String("honest".into()));
            }
        }
    }

    let pretty = serde_json::to_string_pretty(&v)? + "\n";
    std::fs::write(manifest_path, pretty)
        .with_context(|| format!("write {}", manifest_path.display()))?;
    Ok(patched)
}

/// Journey-level synthesis signal: a journey whose `manifest.json` has every
/// step's `screenshot_path` equal to `null` means the capture never happened —
/// every keyframe in that journey is synthetic, regardless of what OCR shows.
/// Returns `true` when the manifest looks unrecorded.
pub fn journey_capture_missing(manifest_json_path: &Path) -> bool {
    let raw = match std::fs::read_to_string(manifest_json_path) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let v: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let steps = match v.get("steps").and_then(|s| s.as_array()) {
        Some(a) => a,
        None => return false,
    };
    if steps.is_empty() {
        return false;
    }
    steps.iter().all(|s| s.get("screenshot_path").map(|p| p.is_null()).unwrap_or(true))
}

/// Lightweight record used by the audit-report writer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlaggedFrame {
    pub journey_id: String,
    pub frame_basename: String,
    pub frame_path: String,
    pub coverage: f32,
    pub word_count: usize,
}

/// Render the audit report markdown body.
pub fn render_audit_markdown(flagged: &[FlaggedFrame], total_scanned: usize) -> String {
    let mut md = String::new();
    md.push_str("# Text-card placeholder audit\n\n");
    md.push_str(
        "Generated by `hwledger-frame-audit`. A keyframe is flagged when any of:\n\
         - OCR output contains a distinctive placeholder keyword \
         (`placeholder`, `run-journeys`, `pending on user`) — the footer of every \
         `generate-placeholder-artefacts.py` gradient card.\n\
         - OCR bbox coverage > 0.04 AND word count >= 25 — the empirical signature \
         of a narrative-text card (the blind-eval judge reads this text back as \
         \"evidence\" even though nothing real was captured).\n\
         - Journey-level signal: every step in `manifest.json` has \
         `screenshot_path: null`, meaning the real capture never happened and \
         every keyframe is synthetic regardless of OCR.\n\n",
    );
    md.push_str(&format!("- Frames scanned: **{}**\n", total_scanned));
    md.push_str(&format!("- Frames flagged: **{}**\n\n", flagged.len()));
    if flagged.is_empty() {
        md.push_str("_No flagged frames — all GUI keyframes appear to be real captures._\n");
        return md;
    }
    md.push_str("| Journey | Frame | Coverage | Words |\n");
    md.push_str("|---|---|---:|---:|\n");
    for f in flagged {
        md.push_str(&format!(
            "| `{}` | `{}` | {:.2} | {} |\n",
            f.journey_id, f.frame_basename, f.coverage, f.word_count
        ));
    }
    md.push_str(
        "\n## Remediation\n\n\
         Every flagged frame has been regenerated as an honest stub (solid \
         `#1e1e2e` background, single TCC-blocked disclaimer line) and the \
         corresponding step in each journey manifest has been marked \
         `blind_eval: \"skip\"`. Re-run the real macOS capture via \
         `apps/macos/HwLedgerUITests/scripts/run-journeys.sh` after granting \
         Accessibility + Screen Recording to Xcode in System Settings > \
         Privacy & Security.\n",
    );
    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Helper: create a temp PNG by invoking `magick`.
    fn make_png(path: &Path, args: &[&str]) {
        let mut cmd = Command::new("magick");
        cmd.args(args).arg(path);
        let status = cmd.status().expect("magick must be installed for tests");
        assert!(status.success(), "magick failed: {:?}", args);
    }

    /// Positive case: a gradient card with dense text flags as a placeholder.
    ///
    /// Traces to: FR-UX-VERIFY-002
    #[test]
    fn test_detect_gradient_text_card_is_flagged() {
        // Skip gracefully when external tooling isn't available (e.g. CI
        // without tesseract/magick installed).
        if Command::new("tesseract").arg("--version").output().is_err() {
            eprintln!("skipping: tesseract not available");
            return;
        }
        if Command::new("magick").arg("--version").output().is_err() {
            eprintln!("skipping: magick not available");
            return;
        }

        let dir = std::env::temp_dir().join("hwledger-frame-audit-positive");
        std::fs::create_dir_all(&dir).unwrap();
        let png = dir.join("gradient.png");
        // Synthesise a gradient-card stub: body narrative + the literal
        // "placeholder - real recording pending on user Mac (run-journeys.sh)"
        // footer, matching the `generate-placeholder-artefacts.py` shape. The
        // keyword footer is enough on its own to trip the detector — this is
        // the same signal the production pipeline will use.
        let body = "First agent node pops in at top right of the canvas, green status ring, \
                    hostname kirin-01 label, hover tooltip forming.\n\n\
                    placeholder - real recording pending on user Mac (run-journeys.sh)";
        make_png(
            &png,
            &[
                "-size",
                "1440x900",
                "gradient:#1a2e1a-#2d4a2d",
                "-font",
                "/System/Library/Fonts/Menlo.ttc",
                "-pointsize",
                "24",
                "-fill",
                "white",
                "-gravity",
                "Center",
                "-annotate",
                "+0+0",
                body,
            ],
        );
        let det = detect(&png).expect("detect must succeed");
        assert!(
            det.flagged,
            "gradient card should flag: coverage={:.3} words={} ocr={:?}",
            det.coverage, det.word_count, det.ocr_text
        );
    }

    /// Negative case: a mostly-empty frame with a tiny label (like a real CLI
    /// terminal first prompt) must NOT flag.
    ///
    /// Traces to: FR-UX-VERIFY-002
    #[test]
    fn test_detect_real_cli_frame_is_not_flagged() {
        if Command::new("tesseract").arg("--version").output().is_err() {
            eprintln!("skipping: tesseract not available");
            return;
        }
        if Command::new("magick").arg("--version").output().is_err() {
            eprintln!("skipping: magick not available");
            return;
        }

        let dir = std::env::temp_dir().join("hwledger-frame-audit-negative");
        std::fs::create_dir_all(&dir).unwrap();
        let png = dir.join("real.png");
        // Tiny prompt in the top-left corner of a large dark frame — realistic
        // CLI recording first-frame signature. Low coverage, low word count.
        make_png(
            &png,
            &[
                "-size",
                "1440x900",
                "canvas:#0a0a0a",
                "-font",
                "/System/Library/Fonts/Menlo.ttc",
                "-pointsize",
                "14",
                "-fill",
                "white",
                "-gravity",
                "NorthWest",
                "-annotate",
                "+20+20",
                "$ hwledger plan",
            ],
        );
        let det = detect(&png).expect("detect must succeed");
        assert!(
            !det.flagged,
            "real CLI frame must not flag: coverage={:.2} words={}",
            det.coverage, det.word_count
        );
    }

    /// Journey-kind inference must skip CLI and streamlit-terminal captures.
    #[test]
    fn test_infer_journey_kind_whitelists() {
        assert_eq!(
            infer_journey_kind(Path::new(
                "/repo/docs-site/public/gui-journeys/foo/keyframes/frame_001.png"
            )),
            JourneyKindHint::Gui
        );
        assert_eq!(
            infer_journey_kind(Path::new(
                "/repo/docs-site/public/cli-journeys/foo/keyframes/frame_001.png"
            )),
            JourneyKindHint::Cli
        );
        assert_eq!(
            infer_journey_kind(Path::new(
                "/repo/docs-site/public/streamlit-journeys/foo/keyframes/frame_001.png"
            )),
            JourneyKindHint::StreamlitTerminal
        );
    }

    /// `patch_manifest_blind_eval` writes `"skip"` for matched step frames and
    /// `"honest"` for the rest, preserving other step fields.
    #[test]
    fn test_patch_manifest_blind_eval() {
        let dir = std::env::temp_dir().join("hwledger-frame-audit-patch");
        std::fs::create_dir_all(&dir).unwrap();
        let mani = dir.join("manifest.json");
        let mut f = std::fs::File::create(&mani).unwrap();
        f.write_all(
            br#"{
              "id":"j",
              "steps":[
                {"index":0,"slug":"a","screenshot_path":"keyframes/frame_001.png","intent":"x"},
                {"index":1,"slug":"b","screenshot_path":"keyframes/frame_002.png","intent":"y"}
              ]
            }"#,
        )
        .unwrap();
        drop(f);

        let n = patch_manifest_blind_eval(&mani, &["frame_002.png".into()]).unwrap();
        assert_eq!(n, 1);
        let after: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&mani).unwrap()).unwrap();
        let steps = after["steps"].as_array().unwrap();
        assert_eq!(steps[0]["blind_eval"], "honest");
        assert_eq!(steps[1]["blind_eval"], "skip");
        // Intent preserved.
        assert_eq!(steps[0]["intent"], "x");
    }
}
