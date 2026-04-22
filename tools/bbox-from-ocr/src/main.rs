//! hwledger-bbox-from-ocr
//!
//! Rewrites `steps[].annotations[].bbox` in cli-journey manifests using
//! word-level bounding boxes from `tesseract ... -c tessedit_create_tsv=1`.
//!
//! Strategy for each annotation with a non-empty `label`:
//!   1. OCR the step's keyframe PNG with tesseract TSV output.
//!   2. Tokenize the label (whitespace + punctuation).
//!   3. Find the longest consecutive word-run in the TSV whose
//!      concatenation fuzzy-matches the label under a Levenshtein
//!      distance budget (<=15% of label char length).
//!   4. If matched, replace the annotation bbox with the union of those
//!      TSV word boxes + 4px padding.
//!   5. If not matched, leave the bbox alone and emit a warning that
//!      lists the five closest TSV candidate runs.
//!
//! For annotations whose `label` is empty, `--generate-labels` will OCR
//! the current bbox region and set `label` to the concatenated OCR
//! tokens in that region.
//!
//! The Apple Vision backend is gated on the environment variable
//! `PHENOTYPE_JOURNEY_OCR_BACKEND=vision`. When set, the tool shells
//! out to `phenotype-journey-ocr` (if available on PATH) for TSV rows;
//! otherwise it uses tesseract. Missing the vision backend is a hard
//! error ONLY when explicitly selected.
//!
//! By default this tool is a dry-run and prints a unified-ish diff of
//! bbox changes per manifest; pass `--apply` to write updated manifests
//! back to disk.

use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use owo_colors::OwoColorize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser, Debug)]
#[command(name = "hwledger-bbox-from-ocr", version)]
struct Args {
    /// Path to a manifest.verified.json (or any manifest with the same
    /// schema). Required.
    #[arg(long)]
    manifest: PathBuf,

    /// Directory containing the keyframes referenced by
    /// `steps[].screenshot_path`. Typically
    /// `apps/cli-journeys/keyframes/<journey>/`.
    #[arg(long)]
    keyframes_dir: PathBuf,

    /// Write changes back to the manifest file. Without this flag the
    /// tool is a dry-run and only prints what would change.
    #[arg(long)]
    apply: bool,

    /// For annotations with an empty `label`, OCR the current bbox
    /// region and populate the label with the tokens found there.
    #[arg(long)]
    generate_labels: bool,

    /// Padding (in pixels) applied on all sides of the OCR-derived
    /// bbox union.
    #[arg(long, default_value_t = 4)]
    pad: i64,

    /// Maximum allowed Levenshtein edit distance as a fraction of the
    /// label char length. 0.15 = up to 15% of characters may differ.
    #[arg(long, default_value_t = 0.15f64)]
    fuzzy_budget: f64,
}

#[derive(Debug, Clone)]
struct TsvWord {
    text: String,
    left: i64,
    top: i64,
    width: i64,
    height: i64,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let summary = run(&args)?;
    eprintln!(
        "{} manifest={} updated={} warned={} unmatched={} labels_generated={}",
        if args.apply { "APPLIED".green().to_string() } else { "DRY-RUN".yellow().to_string() },
        args.manifest.display(),
        summary.updated,
        summary.warnings,
        summary.unmatched,
        summary.labels_generated,
    );
    Ok(())
}

#[derive(Default, Debug)]
struct Summary {
    updated: usize,
    warnings: usize,
    unmatched: usize,
    labels_generated: usize,
}

fn run(args: &Args) -> Result<Summary> {
    let raw = std::fs::read_to_string(&args.manifest)
        .with_context(|| format!("reading manifest {}", args.manifest.display()))?;
    let mut doc: Value = serde_json::from_str(&raw)
        .with_context(|| format!("parsing manifest {}", args.manifest.display()))?;

    let mut summary = Summary::default();
    let journey_id = doc
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("<unknown>")
        .to_string();

    let steps = doc
        .get_mut("steps")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| anyhow!("manifest has no `steps` array"))?;

    for step in steps.iter_mut() {
        let screenshot = step
            .get("screenshot_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let Some(screenshot) = screenshot else { continue };
        let frame_path = args.keyframes_dir.join(&screenshot);
        if !frame_path.exists() {
            eprintln!(
                "{} step screenshot missing: {} (skipping)",
                "warn".yellow(),
                frame_path.display()
            );
            continue;
        }
        let Some(annots) = step.get_mut("annotations").and_then(|v| v.as_array_mut()) else {
            continue;
        };
        if annots.is_empty() {
            continue;
        }

        // OCR once per frame.
        let words = ocr_words(&frame_path)
            .with_context(|| format!("ocr failed for {}", frame_path.display()))?;

        for annot in annots.iter_mut() {
            let label = annot
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if label.trim().is_empty() {
                if args.generate_labels {
                    if let Some(generated) = generate_label_from_bbox(annot, &words) {
                        println!(
                            "  [{}] generate label: \"{}\"",
                            journey_id.cyan(),
                            generated
                        );
                        annot["label"] = Value::String(generated);
                        summary.labels_generated += 1;
                    }
                }
                continue;
            }

            match match_label(&label, &words, args.fuzzy_budget) {
                Some(span) => {
                    let bbox = union_bbox(&words[span.start..=span.end], args.pad);
                    let old = annot.get("bbox").cloned();
                    annot["bbox"] = bbox_to_json(bbox);
                    println!(
                        "  [{}] \"{}\"  {:?} -> {:?}",
                        journey_id.cyan(),
                        label,
                        old.unwrap_or(Value::Null),
                        annot["bbox"],
                    );
                    summary.updated += 1;
                }
                None => {
                    summary.unmatched += 1;
                    summary.warnings += 1;
                    let candidates = closest_candidates(&label, &words, 5);
                    eprintln!(
                        "{} [{}] no fuzzy match for label \"{}\" in {}",
                        "warn".yellow(),
                        journey_id,
                        label,
                        frame_path.display()
                    );
                    for (score, cand) in candidates {
                        eprintln!("       candidate (dist={score}): \"{cand}\"");
                    }
                }
            }
        }
    }

    // Only rewrite when we actually changed something — avoids spurious
    // field-order / unicode-escape churn from the JSON round-trip.
    if args.apply && (summary.updated + summary.labels_generated) > 0 {
        let serialized = serde_json::to_string_pretty(&doc)?;
        let mut out = serialized;
        if !out.ends_with('\n') {
            out.push('\n');
        }
        std::fs::write(&args.manifest, out)
            .with_context(|| format!("writing {}", args.manifest.display()))?;
    }
    Ok(summary)
}

fn ocr_words(path: &Path) -> Result<Vec<TsvWord>> {
    let backend = std::env::var("PHENOTYPE_JOURNEY_OCR_BACKEND").unwrap_or_default();
    let tsv = if backend == "vision" {
        // Delegate to the Apple-Vision-backed sidecar binary. Expected to
        // emit tesseract-compatible TSV on stdout. Missing binary is a
        // hard error when the backend is explicitly selected.
        let out = Command::new("phenotype-journey-ocr")
            .arg("--tsv")
            .arg(path)
            .output()
            .with_context(|| "spawning phenotype-journey-ocr (vision backend)")?;
        if !out.status.success() {
            bail!(
                "phenotype-journey-ocr failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        String::from_utf8_lossy(&out.stdout).into_owned()
    } else {
        let out = Command::new("tesseract")
            .arg(path)
            .arg("-")
            .arg("-c")
            .arg("tessedit_create_tsv=1")
            .output()
            .with_context(|| "spawning tesseract")?;
        if !out.status.success() {
            bail!(
                "tesseract failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    Ok(parse_tsv(&tsv))
}

fn parse_tsv(tsv: &str) -> Vec<TsvWord> {
    let mut words = Vec::new();
    for (i, line) in tsv.lines().enumerate() {
        if i == 0 {
            // Header row: level page_num block_num par_num line_num
            // word_num left top width height conf text
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 12 {
            continue;
        }
        // Level 5 = word-level row.
        if cols[0] != "5" {
            continue;
        }
        let text = cols[11].trim();
        if text.is_empty() {
            continue;
        }
        let left: i64 = cols[6].parse().unwrap_or(0);
        let top: i64 = cols[7].parse().unwrap_or(0);
        let width: i64 = cols[8].parse().unwrap_or(0);
        let height: i64 = cols[9].parse().unwrap_or(0);
        words.push(TsvWord {
            text: text.to_string(),
            left,
            top,
            width,
            height,
        });
    }
    words
}

/// Tokenize a label for matching. Split on whitespace and punctuation;
/// keep alphanumeric + underscore runs; lowercase.
fn tokenize(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() || c == '_' {
            cur.push(c.to_ascii_lowercase());
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn normalize_run(words: &[TsvWord]) -> String {
    let mut s = String::new();
    for w in words {
        for c in w.text.chars() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c.to_ascii_lowercase());
            }
        }
    }
    s
}

fn normalize_label(label: &str) -> String {
    tokenize(label).join("")
}

#[derive(Debug, Clone, Copy)]
struct Span {
    start: usize,
    end: usize,
}

/// Find the longest consecutive word-run whose concatenation matches
/// the label within the fuzzy budget. Prefers shorter runs on ties to
/// avoid capturing extra adjacent words.
fn match_label(label: &str, words: &[TsvWord], fuzzy_budget: f64) -> Option<Span> {
    let label_norm = normalize_label(label);
    if label_norm.is_empty() || words.is_empty() {
        return None;
    }
    let budget = ((label_norm.chars().count() as f64) * fuzzy_budget).ceil() as usize;
    let label_tokens = tokenize(label);
    let max_run = label_tokens.len().max(1);
    // Allow runs from 1 word up to label_tokens.len() + small slack
    // (extra words get folded in when tesseract splits differently).
    let max_run_with_slack = max_run + 2;

    let mut best: Option<(Span, usize)> = None; // (span, dist)
    for start in 0..words.len() {
        for len in 1..=max_run_with_slack.min(words.len() - start) {
            let end = start + len - 1;
            let run = normalize_run(&words[start..=end]);
            if run.is_empty() {
                continue;
            }
            let dist = levenshtein(&run, &label_norm);
            if dist <= budget {
                // Prefer a run whose length (tokens) is closest to the
                // label's token count, then minimum distance.
                let current = best.map(|(_, d)| d).unwrap_or(usize::MAX);
                if dist < current {
                    best = Some((Span { start, end }, dist));
                }
            }
        }
    }
    best.map(|(span, _)| span)
}

fn union_bbox(words: &[TsvWord], pad: i64) -> (i64, i64, i64, i64) {
    let mut x0 = i64::MAX;
    let mut y0 = i64::MAX;
    let mut x1 = i64::MIN;
    let mut y1 = i64::MIN;
    for w in words {
        x0 = x0.min(w.left);
        y0 = y0.min(w.top);
        x1 = x1.max(w.left + w.width);
        y1 = y1.max(w.top + w.height);
    }
    let x = (x0 - pad).max(0);
    let y = (y0 - pad).max(0);
    let w = (x1 - x0) + pad * 2;
    let h = (y1 - y0) + pad * 2;
    (x, y, w.max(1), h.max(1))
}

fn bbox_to_json((x, y, w, h): (i64, i64, i64, i64)) -> Value {
    Value::Array(vec![
        Value::from(x),
        Value::from(y),
        Value::from(w),
        Value::from(h),
    ])
}

fn closest_candidates(label: &str, words: &[TsvWord], n: usize) -> Vec<(usize, String)> {
    let label_norm = normalize_label(label);
    if label_norm.is_empty() {
        return Vec::new();
    }
    let label_tokens = tokenize(label);
    let max_run = (label_tokens.len() + 2).max(2);
    let mut scored: Vec<(usize, String)> = Vec::new();
    for start in 0..words.len() {
        for len in 1..=max_run.min(words.len() - start) {
            let end = start + len - 1;
            let run_text: Vec<String> = words[start..=end].iter().map(|w| w.text.clone()).collect();
            let run_norm = normalize_run(&words[start..=end]);
            if run_norm.is_empty() {
                continue;
            }
            let d = levenshtein(&run_norm, &label_norm);
            scored.push((d, run_text.join(" ")));
        }
    }
    scored.sort_by_key(|(d, _)| *d);
    scored.dedup_by(|a, b| a.1 == b.1);
    scored.into_iter().take(n).collect()
}

/// Populate an annotation's `label` from the OCR tokens that fall
/// inside its current bbox. Returns the generated label string if any
/// words overlap, else None.
fn generate_label_from_bbox(annot: &mut Value, words: &[TsvWord]) -> Option<String> {
    let bbox = annot.get("bbox")?.as_array()?;
    if bbox.len() < 4 {
        return None;
    }
    let x = bbox[0].as_i64()?;
    let y = bbox[1].as_i64()?;
    let w = bbox[2].as_i64()?;
    let h = bbox[3].as_i64()?;
    let (bx0, by0, bx1, by1) = (x, y, x + w, y + h);
    let mut hits: Vec<&TsvWord> = Vec::new();
    for word in words {
        let wx0 = word.left;
        let wy0 = word.top;
        let wx1 = word.left + word.width;
        let wy1 = word.top + word.height;
        // Center-in-bbox test tolerant of padding.
        let cx = (wx0 + wx1) / 2;
        let cy = (wy0 + wy1) / 2;
        if cx >= bx0 && cx <= bx1 && cy >= by0 && cy <= by1 {
            hits.push(word);
        }
    }
    if hits.is_empty() {
        return None;
    }
    let joined = hits
        .iter()
        .map(|w| w.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    Some(joined)
}

/// Iterative, row-based Levenshtein distance. O(m*n) time, O(n) space.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr: Vec<usize> = vec![0; b.len() + 1];
    for i in 1..=a.len() {
        curr[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1)
                .min(curr[j - 1] + 1)
                .min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(text: &str, left: i64, top: i64, width: i64, height: i64) -> TsvWord {
        TsvWord {
            text: text.into(),
            left,
            top,
            width,
            height,
        }
    }

    #[test]
    fn matches_exact_token_run() {
        // "Model: DeepSeek-V2"
        let words = vec![
            w("Terminal", 10, 10, 80, 16),
            w("Model:", 100, 100, 60, 16),
            w("DeepSeek-V2", 170, 100, 120, 16),
            w("elsewhere", 400, 200, 90, 16),
        ];
        let span = match_label("Model: DeepSeek-V2", &words, 0.15)
            .expect("expected exact token match");
        assert_eq!(span.start, 1);
        assert_eq!(span.end, 2);
        let (x, y, bw, bh) = union_bbox(&words[span.start..=span.end], 4);
        assert_eq!(x, 96);
        assert_eq!(y, 96);
        // Union width: 170+120 - 100 = 190; +2*pad = 198.
        assert_eq!(bw, 198);
        assert_eq!(bh, 16 + 8);
    }

    #[test]
    fn matches_label_with_ocr_typos() {
        // Label: "MLA (kv_lora_rank=512)"; OCR mangles punctuation
        // and inserts typos.
        let words = vec![
            w("leading", 0, 0, 40, 10),
            // OCR output: "MIA" (I for L), "kv_Iora_rank=5l2" (l vs 1)
            w("MIA", 50, 50, 40, 16),
            w("(kv_Iora_rank=5l2)", 95, 50, 220, 16),
            w("trailing", 400, 200, 40, 10),
        ];
        let span = match_label("MLA (kv_lora_rank=512)", &words, 0.15)
            .expect("expected fuzzy match under 15% budget");
        assert_eq!(span.start, 1);
        assert_eq!(span.end, 2);
    }

    #[test]
    fn tokenize_drops_punctuation() {
        assert_eq!(
            tokenize("MLA (kv_lora_rank=512)"),
            vec!["mla", "kv_lora_rank", "512"]
        );
    }

    #[test]
    fn levenshtein_sanity() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", "abc"), 0);
    }
}
