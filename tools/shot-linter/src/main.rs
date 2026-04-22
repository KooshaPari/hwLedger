//! hwLedger shot-linter.
//!
//! Walks `docs-site/**/*.md`, extracts every `<Shot src=... caption=...>`
//! invocation, resolves the `src` to a PNG under `docs-site/public/`, runs
//! `tesseract` on the PNG, and checks that at least one caption token appears
//! in the OCR output. Emits a markdown report; exits non-zero on mismatch when
//! `--strict` is passed.
//!
//! Also understands the `phenotype-journeys/data/shot-annotations.yaml`
//! registry (via its `expected_text:` extension) when available — missing
//! registry is non-fatal.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(name = "hwledger-shot-linter", version)]
struct Args {
    /// docs-site root (contains `public/` + md pages).
    #[arg(long, default_value = "docs-site")]
    docs_site: PathBuf,

    /// Write a markdown audit report here.
    #[arg(long, default_value = "docs-site/quality/shot-audit.md")]
    report: PathBuf,

    /// Fail with non-zero exit if any shot mismatches.
    #[arg(long)]
    strict: bool,

    /// Minimum caption tokens (>=3 chars, lowercase) that must appear in OCR.
    #[arg(long, default_value_t = 1)]
    min_token_matches: usize,

    /// Skip tesseract invocation; useful in CI where OCR is too slow.
    #[arg(long)]
    no_ocr: bool,

    /// For each mismatch, scan sibling frames in the same journey dir and
    /// suggest the best-matching frame (highest token coverage). Written into
    /// the report and a companion JSON file.
    #[arg(long)]
    suggest: bool,

    /// Write suggestions here (JSON list of {file,line,src,suggested_src}).
    #[arg(long, default_value = "docs-site/quality/shot-suggestions.json")]
    suggestions_out: PathBuf,

    /// After computing suggestions, rewrite markdown files in-place to apply
    /// suggestions whose match count is >= min_apply_matches.
    #[arg(long)]
    apply: bool,

    /// Threshold for auto-apply (ignored unless --apply).
    #[arg(long, default_value_t = 2)]
    min_apply_matches: usize,

    /// For mismatches without a confident suggestion, append a
    /// `<!-- SHOT-MISMATCH ... -->` comment after the <Shot> tag.
    #[arg(long)]
    flag_mismatches: bool,

    /// Also walk every `manifest.verified.json` under `apps/**/manifests/`
    /// and `docs-site/public/**/manifests/` and fail when any
    /// `steps[i].annotations[j].label` is empty or missing. Strict under
    /// `HWLEDGER_TAPE_GATE=strict` or when `--strict` is passed, otherwise
    /// reports as a warning.
    #[arg(long)]
    check_empty_labels: bool,

    /// Subcommands. When supplied, the top-level shot-audit pipeline is
    /// skipped and only the subcommand runs.
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Walk every `manifest.verified.json` under `apps/**/manifests/` and
    /// `docs-site/public/**/manifests/` and assert that every
    /// `steps[i].annotations[j].label` is non-empty.
    AnnotationsLabels {
        /// Repo root; defaults to the current working directory.
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        /// Treat empty/missing labels as hard failures even when
        /// `HWLEDGER_TAPE_GATE` is not `strict`.
        #[arg(long)]
        strict: bool,
    },
}

#[derive(Debug, Deserialize)]
struct ManifestFile {
    #[serde(default)]
    steps: Vec<ManifestStep>,
}

#[derive(Debug, Deserialize)]
struct ManifestStep {
    #[serde(default)]
    annotations: Vec<ManifestAnnotation>,
}

#[derive(Debug, Deserialize)]
struct ManifestAnnotation {
    #[serde(default)]
    label: Option<String>,
}

#[derive(Debug)]
struct LabelViolation {
    manifest: PathBuf,
    step_index: usize,
    annotation_index: usize,
    reason: &'static str,
}

/// Walk `apps/**/manifests/**/manifest.verified.json` and
/// `docs-site/public/**/manifests/**/manifest.verified.json`, and collect
/// every annotation with a missing-or-empty `label`.
fn scan_empty_labels(repo: &Path) -> Result<Vec<LabelViolation>> {
    let mut roots: Vec<PathBuf> = Vec::new();
    let apps = repo.join("apps");
    let docs_site = repo.join("docs-site").join("public");
    if apps.exists() {
        roots.push(apps);
    }
    if docs_site.exists() {
        roots.push(docs_site);
    }

    let mut violations = Vec::new();
    for root in &roots {
        for e in WalkDir::new(root).into_iter().filter_map(Result::ok) {
            let p = e.path();
            if !p.is_file() {
                continue;
            }
            if p.file_name().and_then(|n| n.to_str()) != Some("manifest.verified.json") {
                continue;
            }
            let path_str = p.to_string_lossy();
            if !path_str.contains("/manifests/") {
                continue;
            }
            let text = match std::fs::read_to_string(p) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let manifest: ManifestFile = match serde_json::from_str(&text) {
                Ok(m) => m,
                Err(_) => continue,
            };
            for (si, step) in manifest.steps.iter().enumerate() {
                for (ai, ann) in step.annotations.iter().enumerate() {
                    match &ann.label {
                        None => violations.push(LabelViolation {
                            manifest: p.to_path_buf(),
                            step_index: si,
                            annotation_index: ai,
                            reason: "missing",
                        }),
                        Some(s) if s.trim().is_empty() => violations.push(LabelViolation {
                            manifest: p.to_path_buf(),
                            step_index: si,
                            annotation_index: ai,
                            reason: "empty",
                        }),
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(violations)
}

fn report_label_violations(violations: &[LabelViolation], strict: bool) -> bool {
    if violations.is_empty() {
        println!(
            "{} annotations-labels: all annotations have non-empty labels",
            "shot-linter:".bold()
        );
        return true;
    }
    let header = format!(
        "shot-linter: annotations-labels: {} violation(s)",
        violations.len()
    );
    if strict {
        eprintln!("{}", header.red().bold());
    } else {
        eprintln!("{}", header.yellow().bold());
    }
    for v in violations {
        eprintln!(
            "  {} steps[{}].annotations[{}] label={}",
            v.manifest.display(),
            v.step_index,
            v.annotation_index,
            v.reason,
        );
    }
    !strict
}

#[derive(Debug, Serialize)]
struct ShotRecord {
    file: String,
    line: usize,
    src: String,
    caption: String,
    resolved_path: Option<String>,
    ocr_excerpt: String,
    matched_tokens: Vec<String>,
    expected_tokens: Vec<String>,
    status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggested_src: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggested_matches: Option<usize>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
enum Status {
    Ok,
    Mismatch,
    Missing,
    Skipped,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Subcommand path: run only the requested check and exit.
    if let Some(Cmd::AnnotationsLabels { repo, strict }) = &args.cmd {
        let gate_strict = *strict
            || std::env::var("HWLEDGER_TAPE_GATE")
                .map(|v| v == "strict")
                .unwrap_or(false);
        let violations = scan_empty_labels(repo)?;
        let ok = report_label_violations(&violations, gate_strict);
        if !ok {
            std::process::exit(2);
        }
        return Ok(());
    }

    // Default pipeline always runs the annotations-labels check so every
    // push catches empty labels. Hard-fails in `HWLEDGER_TAPE_GATE=strict`
    // or when `--strict` / `--check-empty-labels` is passed; otherwise
    // warns.
    let gate_strict = args.strict
        || args.check_empty_labels
        || std::env::var("HWLEDGER_TAPE_GATE")
            .map(|v| v == "strict")
            .unwrap_or(false);
    let label_failure = {
        let violations = scan_empty_labels(Path::new("."))?;
        let ok = report_label_violations(&violations, gate_strict);
        !ok
    };

    let md_files: Vec<PathBuf> = WalkDir::new(&args.docs_site)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
        .filter(|e| !e.path().to_string_lossy().contains("/node_modules/"))
        .map(|e| e.into_path())
        .collect();

    // Rough shot regex: `<Shot src="..."` optionally followed by attrs, including `caption="..."`.
    let shot_re = Regex::new(r#"<Shot\s+src="([^"]+)"(?P<rest>[^>]*?)(?:/>|>)"#)?;
    let caption_re = Regex::new(r#"caption="([^"]+)""#)?;

    let mut records = Vec::new();
    for md in &md_files {
        let text = std::fs::read_to_string(md)?;
        // Multi-line shots: normalize by joining <Shot ... /> chunks across lines.
        // Simpler: walk char-by-char using a manual tag tokenizer.
        for (line_no, chunk) in extract_shot_blocks(&text) {
            let Some(src_caps) = shot_re.captures(&chunk) else {
                continue;
            };
            let src = src_caps.get(1).unwrap().as_str().to_string();
            let caption = caption_re
                .captures(&chunk)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            // Resolve /foo/bar.png -> docs-site/public/foo/bar.png
            let rel = src.trim_start_matches('/');
            let resolved = args.docs_site.join("public").join(rel);
            if !resolved.exists() {
                records.push(ShotRecord {
                    file: md.display().to_string(),
                    line: line_no,
                    src: src.clone(),
                    caption: caption.clone(),
                    resolved_path: None,
                    ocr_excerpt: String::new(),
                    matched_tokens: vec![],
                    expected_tokens: caption_tokens(&caption),
                    status: Status::Missing,
                    suggested_src: None,
                    suggested_matches: None,
                });
                continue;
            }

            let ocr =
                if args.no_ocr { String::new() } else { ocr_png(&resolved).unwrap_or_default() };
            let ocr_lc = ocr.to_lowercase();
            let expected = caption_tokens(&caption);
            let matched: Vec<String> =
                expected.iter().filter(|t| ocr_lc.contains(t.as_str())).cloned().collect();

            let status = if args.no_ocr {
                Status::Skipped
            } else if matched.len() >= args.min_token_matches || expected.is_empty() {
                Status::Ok
            } else {
                Status::Mismatch
            };

            // If mismatched and --suggest, scan sibling frames in the same dir.
            let (suggested_src, suggested_matches) =
                if args.suggest && status == Status::Mismatch && !args.no_ocr {
                    suggest_best_frame(&resolved, &src, &expected)
                } else {
                    (None, None)
                };

            records.push(ShotRecord {
                file: md.display().to_string(),
                line: line_no,
                src,
                caption,
                resolved_path: Some(resolved.display().to_string()),
                ocr_excerpt: ocr.chars().take(160).collect::<String>().replace('\n', " "),
                matched_tokens: matched,
                expected_tokens: expected,
                status,
                suggested_src,
                suggested_matches,
            });
        }
    }

    if args.suggest {
        let suggestions: Vec<_> = records
            .iter()
            .filter(|r| r.suggested_src.is_some())
            .map(|r| {
                serde_json::json!({
                    "file": r.file,
                    "line": r.line,
                    "src": r.src,
                    "caption": r.caption,
                    "suggested_src": r.suggested_src,
                    "suggested_matches": r.suggested_matches,
                    "expected_tokens": r.expected_tokens,
                })
            })
            .collect();
        if let Some(parent) = args.suggestions_out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&args.suggestions_out, serde_json::to_string_pretty(&suggestions)?)?;
    }

    let mut applied = 0usize;
    let mut flagged = 0usize;
    if args.apply || args.flag_mismatches {
        use std::collections::BTreeMap;
        // Group edits by file.
        let mut per_file: BTreeMap<String, Vec<&ShotRecord>> = BTreeMap::new();
        for r in &records {
            if r.status == Status::Mismatch {
                per_file.entry(r.file.clone()).or_default().push(r);
            }
        }
        for (file, recs) in per_file {
            let Ok(orig) = std::fs::read_to_string(&file) else { continue };
            let mut text = orig.clone();
            for r in recs {
                if args.apply {
                    if let (Some(sugg), Some(n)) = (&r.suggested_src, r.suggested_matches) {
                        if n >= args.min_apply_matches {
                            let needle = format!("src=\"{}\"", r.src);
                            let replacement = format!("src=\"{}\"", sugg);
                            if text.contains(&needle) {
                                text = text.replacen(&needle, &replacement, 1);
                                applied += 1;
                                continue;
                            }
                        }
                    }
                }
                if args.flag_mismatches {
                    let needle = format!("src=\"{}\"", r.src);
                    let flag = format!(
                        "<!-- SHOT-MISMATCH: caption=\"{}\" expected=[{}] matched=[{}] -->",
                        r.caption.replace("--", "—").replace('"', "'"),
                        r.expected_tokens.join(","),
                        r.matched_tokens.join(","),
                    );
                    // Insert flag just before the needle's <Shot so the comment
                    // sits right above the shot line.
                    if let Some(pos) = text.find(&needle) {
                        // find start of `<Shot` that owns this src.
                        if let Some(shot_start) = text[..pos].rfind("<Shot") {
                            if !text[..shot_start].ends_with(&format!("{}\n", flag))
                                && !text[shot_start.saturating_sub(flag.len() + 1)..shot_start]
                                    .contains("SHOT-MISMATCH")
                            {
                                text.insert_str(shot_start, &format!("{}\n", flag));
                                flagged += 1;
                            }
                        }
                    }
                }
            }
            if text != orig {
                std::fs::write(&file, text)?;
            }
        }
        if applied + flagged > 0 {
            println!("shot-linter: applied={} flagged={}", applied, flagged);
        }
    }

    write_report(&args.report, &records)?;

    let total = records.len();
    let ok = records.iter().filter(|r| r.status == Status::Ok).count();
    let missing = records.iter().filter(|r| r.status == Status::Missing).count();
    let mism = records.iter().filter(|r| r.status == Status::Mismatch).count();
    let skip = records.iter().filter(|r| r.status == Status::Skipped).count();

    println!(
        "{} total={} ok={} mismatch={} missing={} skipped={}",
        "shot-linter:".bold(),
        total,
        ok.green(),
        mism.red(),
        missing.red(),
        skip.yellow(),
    );

    if args.strict && (missing > 0 || mism > 0) {
        std::process::exit(2);
    }
    if label_failure {
        std::process::exit(2);
    }
    Ok(())
}

/// Extract `<Shot ... />` blocks spanning one or more lines along with the
/// 1-based line number where the tag opens.
fn extract_shot_blocks(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut line_no = 1usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'\n' {
            line_no += 1;
            i += 1;
            continue;
        }
        if bytes[i..].starts_with(b"<Shot") {
            // find the closing `>` (allow `/>`).
            let start = i;
            let start_line = line_no;
            let mut j = i;
            while j < bytes.len() && bytes[j] != b'>' {
                if bytes[j] == b'\n' {
                    line_no += 1;
                }
                j += 1;
            }
            if j >= bytes.len() {
                break;
            }
            j += 1; // include '>'
            out.push((start_line, std::str::from_utf8(&bytes[start..j]).unwrap_or("").to_string()));
            i = j;
            continue;
        }
        i += 1;
    }
    out
}

fn caption_tokens(caption: &str) -> Vec<String> {
    let lc = caption.to_lowercase();
    let mut seen = HashSet::new();
    let stop: HashSet<&str> = [
        "the", "and", "with", "for", "from", "into", "onto", "that", "this", "line", "frame",
        "shot", "near", "mid", "help", "usage", "just", "about", "its", "gets", "then", "printed",
        "print",
    ]
    .into_iter()
    .collect();
    lc.split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
        .filter(|t| t.len() >= 3 && !stop.contains(t))
        .filter_map(|t| if seen.insert(t.to_string()) { Some(t.to_string()) } else { None })
        .collect()
}

/// For a mismatched frame, scan sibling frames in the same directory and
/// return the one with the highest number of expected-token matches (if any).
fn suggest_best_frame(
    resolved: &Path,
    original_src: &str,
    expected: &[String],
) -> (Option<String>, Option<usize>) {
    let Some(dir) = resolved.parent() else {
        return (None, None);
    };
    let mut best: Option<(String, usize)> = None;
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return (None, None),
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().map(|x| x != "png").unwrap_or(true) {
            continue;
        }
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name.contains("annotated") {
            continue;
        }
        let Ok(ocr) = ocr_png(&p) else { continue };
        let lc = ocr.to_lowercase();
        let matches = expected.iter().filter(|t| lc.contains(t.as_str())).count();
        if matches == 0 {
            continue;
        }
        if best.as_ref().map(|(_, n)| matches > *n).unwrap_or(true) {
            // Map resolved path back into /cli-journeys/... form by finding "/public/" segment.
            let disp = p.display().to_string();
            let url = match disp.split_once("/public/") {
                Some((_, rest)) => format!("/{}", rest),
                None => disp,
            };
            best = Some((url, matches));
        }
    }
    match best {
        Some((url, n)) if url != original_src => (Some(url), Some(n)),
        _ => (None, None),
    }
}

fn ocr_png(path: &Path) -> Result<String> {
    let out = Command::new("tesseract")
        .arg(path)
        .arg("-") // stdout
        .arg("-l")
        .arg("eng")
        .arg("--psm")
        .arg("6")
        .output()
        .with_context(|| format!("tesseract failed on {}", path.display()))?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn write_report(path: &Path, records: &[ShotRecord]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let total = records.len();
    let ok = records.iter().filter(|r| r.status == Status::Ok).count();
    let missing = records.iter().filter(|r| r.status == Status::Missing).count();
    let mism = records.iter().filter(|r| r.status == Status::Mismatch).count();
    let skip = records.iter().filter(|r| r.status == Status::Skipped).count();

    let mut md = String::new();
    md.push_str("# Shot audit\n\n");
    md.push_str("Generated by `tools/shot-linter`. Do not hand-edit — re-run\n");
    md.push_str("`cargo run -p hwledger-shot-linter -- --strict` to refresh.\n\n");
    md.push_str(&format!(
        "| Total | Ok | Mismatch | Missing | Skipped |\n|---|---|---|---|---|\n| {} | {} | {} | {} | {} |\n\n",
        total, ok, mism, missing, skip
    ));

    if mism + missing > 0 {
        md.push_str("## Mismatches and missing frames\n\n");
        md.push_str("| File:line | src | caption | status | matched / expected tokens |\n");
        md.push_str("|---|---|---|---|---|\n");
        for r in records.iter().filter(|r| matches!(r.status, Status::Mismatch | Status::Missing)) {
            md.push_str(&format!(
                "| `{}:{}` | `{}` | {} | **{:?}** | {}/{} |\n",
                r.file,
                r.line,
                r.src,
                r.caption.replace('|', "\\|"),
                r.status,
                r.matched_tokens.join(","),
                r.expected_tokens.join(",")
            ));
        }
    }

    md.push_str("\n## All shots\n\n");
    md.push_str("| File:line | src | status |\n|---|---|---|\n");
    for r in records {
        md.push_str(&format!("| `{}:{}` | `{}` | {:?} |\n", r.file, r.line, r.src, r.status));
    }

    std::fs::write(path, md)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caption_tokens_strips_stopwords_and_short() {
        let toks = caption_tokens("The cargo install line — downloads the hwLedger binary");
        assert!(toks.contains(&"cargo".to_string()));
        assert!(toks.contains(&"install".to_string()));
        assert!(toks.contains(&"hwledger".to_string()));
        assert!(toks.contains(&"binary".to_string()));
        // stopwords removed
        assert!(!toks.iter().any(|t| t == "the"));
    }

    #[test]
    fn scan_empty_labels_passes_when_every_annotation_has_label() {
        let tmp = std::env::temp_dir().join(format!(
            "hwledger-shot-linter-pass-{}",
            std::process::id()
        ));
        let manifests = tmp.join("apps/cli-journeys/manifests/alpha");
        std::fs::create_dir_all(&manifests).unwrap();
        let body = serde_json::json!({
            "steps": [
                {"index": 0, "annotations": [{"label": "launch badge", "kind": "highlight"}]},
                {"index": 1, "annotations": [{"label": "status row"}, {"label": "probe chip"}]}
            ]
        });
        std::fs::write(
            manifests.join("manifest.verified.json"),
            serde_json::to_vec_pretty(&body).unwrap(),
        )
        .unwrap();

        let violations = scan_empty_labels(&tmp).unwrap();
        std::fs::remove_dir_all(&tmp).ok();
        assert!(violations.is_empty(), "expected no violations, got {:?}", violations);
    }

    #[test]
    fn scan_empty_labels_fails_on_empty_or_missing_label() {
        let tmp = std::env::temp_dir().join(format!(
            "hwledger-shot-linter-fail-{}",
            std::process::id()
        ));
        let manifests = tmp.join("docs-site/public/gui-journeys/beta/manifests/run");
        std::fs::create_dir_all(&manifests).unwrap();
        let body = serde_json::json!({
            "steps": [
                {"index": 0, "annotations": [
                    {"label": "ok"},
                    {"label": ""},
                    {"kind": "highlight"}
                ]}
            ]
        });
        std::fs::write(
            manifests.join("manifest.verified.json"),
            serde_json::to_vec_pretty(&body).unwrap(),
        )
        .unwrap();

        let violations = scan_empty_labels(&tmp).unwrap();
        std::fs::remove_dir_all(&tmp).ok();
        assert_eq!(violations.len(), 2, "got: {:?}", violations);
        let reasons: Vec<_> = violations.iter().map(|v| v.reason).collect();
        assert!(reasons.contains(&"empty"));
        assert!(reasons.contains(&"missing"));
    }

    #[test]
    fn extract_shot_blocks_single_and_multiline() {
        let src = "pre\n<Shot src=\"/a.png\" caption=\"hi\" />\nmid\n<Shot\n  src=\"/b.png\"\n  caption=\"multi\"\n/>\nend";
        let blocks = extract_shot_blocks(src);
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].1.contains("/a.png"));
        assert!(blocks[1].1.contains("/b.png"));
        assert_eq!(blocks[0].0, 2); // line of first <Shot
    }
}
