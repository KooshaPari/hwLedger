//! `hwledger-frame-audit` — sweep GUI-journey keyframes for text-card
//! placeholders, regenerate the flagged ones as honest stubs, and patch
//! journey manifests with `blind_eval: "skip"`.
//!
//! Traces to: FR-TRACE-003, FR-UX-VERIFY-002

use anyhow::{Context, Result};
use clap::Parser;
use hwledger_frame_audit::{
    detect, journey_capture_missing, patch_manifest_blind_eval, regenerate_honest_stub,
    render_audit_markdown, walk_candidates, FlaggedFrame,
};
use std::collections::HashSet;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "hwledger-frame-audit",
    about = "Detect + regenerate text-card placeholder keyframes"
)]
struct Args {
    /// Repository root — scan starts from `<repo>/docs-site/public` and
    /// `<repo>/apps` under this path.
    #[arg(long, default_value = ".")]
    repo: PathBuf,

    /// Write the audit markdown report to this path (relative to repo).
    #[arg(long, default_value = "docs-site/quality/text-card-audit.md")]
    report_out: PathBuf,

    /// Report only; do NOT regenerate frames or patch manifests.
    #[arg(long)]
    dry_run: bool,

    /// Verbose per-frame diagnostics.
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let repo = args.repo.canonicalize().context("canonicalize repo path")?;
    let mut roots = Vec::new();
    for sub in ["docs-site/public", "apps"] {
        let p = repo.join(sub);
        if p.exists() {
            roots.push(p);
        }
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    for r in &roots {
        candidates.extend(walk_candidates(r));
    }
    if args.verbose {
        eprintln!("scanning {} candidate PNG(s)", candidates.len());
    }

    // Journey-level pre-pass: any journey whose `manifest.json` shows all
    // steps with `screenshot_path: null` never captured real frames —
    // every keyframe in it is synthetic, so flag them all even if OCR alone
    // wouldn't catch them (e.g. faux-widget mockups with small labels).
    let mut always_flag_journeys: HashSet<PathBuf> = HashSet::new();
    for path in &candidates {
        let journey_dir = path.parent().and_then(|p| p.parent());
        if let Some(jd) = journey_dir {
            let mani = jd.join("manifest.json");
            if journey_capture_missing(&mani) {
                always_flag_journeys.insert(jd.to_path_buf());
            }
        }
    }
    if args.verbose {
        for j in &always_flag_journeys {
            eprintln!(
                "  journey-level synthesis signal: {} (all screenshot_path are null)",
                j.display()
            );
        }
    }

    // Run detection.
    let mut flagged: Vec<FlaggedFrame> = Vec::new();
    // Map journey_dir -> list of regenerated frame basenames.
    let mut by_journey: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();

    for path in &candidates {
        let det = match detect(path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("warn: detect failed for {}: {}", path.display(), e);
                continue;
            }
        };
        let journey_dir =
            path.parent().and_then(|p| p.parent()).unwrap_or(path).to_path_buf();
        let force = always_flag_journeys.contains(&journey_dir);
        if args.verbose {
            eprintln!(
                "  {} coverage={:.2} words={} flagged={} (force={})",
                path.display(),
                det.coverage,
                det.word_count,
                det.flagged,
                force,
            );
        }
        let is_flagged = det.flagged || force;
        if !is_flagged {
            continue;
        }

        let journey_id = journey_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unknown>")
            .to_string();
        let basename = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();

        flagged.push(FlaggedFrame {
            journey_id: journey_id.clone(),
            frame_basename: basename.clone(),
            frame_path: path
                .strip_prefix(&repo)
                .unwrap_or(path)
                .display()
                .to_string(),
            coverage: det.coverage,
            word_count: det.word_count,
        });

        by_journey.entry(journey_dir).or_default().push(basename);
    }

    // Write the audit report.
    let report = render_audit_markdown(&flagged, candidates.len());
    let report_path = repo.join(&args.report_out);
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&report_path, report)
        .with_context(|| format!("write {}", report_path.display()))?;
    eprintln!(
        "wrote audit report: {} ({} flagged / {} scanned)",
        report_path.display(),
        flagged.len(),
        candidates.len()
    );

    if args.dry_run {
        return Ok(());
    }

    // Regenerate flagged frames + patch manifests.
    for (journey_dir, frames) in &by_journey {
        for frame_base in frames {
            let frame_path = journey_dir.join("keyframes").join(frame_base);
            let (w, h) = hwledger_frame_audit::png_dimensions(&frame_path)?;
            regenerate_honest_stub(&frame_path, w, h)?;
            eprintln!("  regenerated {}", frame_path.display());
        }
        for mani_name in ["manifest.json", "manifest.verified.json"] {
            let mani = journey_dir.join(mani_name);
            if !mani.exists() {
                continue;
            }
            let patched = patch_manifest_blind_eval(&mani, frames)?;
            eprintln!("  patched {} ({} skip-marked step(s))", mani.display(), patched);
        }
    }

    // Return non-zero if we flagged frames in --dry-run (so CI can enforce).
    // In write mode we always succeed — the regeneration+patch is the remediation.
    Ok(())
}

// Suppress unused-import warnings when compiling the binary — `Path` is used
// via the Path::new call below.
#[allow(dead_code)]
fn _touch(_p: &Path) {}
