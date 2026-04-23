//! hwledger-docs-health — proactive documentation health gate.
//!
//! Re-implements the checks originally drafted by agent `af76ed69` after a
//! mid-write ENOSPC truncated the source files. Eight lightweight checks
//! scan a docs root for common regressions that otherwise silently ship:
//!
//! * `check_mermaid`    — unbalanced fences, unknown diagram kinds, bracket/quote parity
//! * `check_latex`      — double-escape `\\\\` inside `$…$` / `$$…$$` math
//! * `check_video`      — MP4 assets under `docs-site/public/**` that are suspiciously tiny
//! * `check_assets`     — relative `<img src>` / `<video src>` pointing at missing files
//! * `check_journey`    — warns if `phenotype-journey` is not on PATH
//! * `check_agreement`  — red-zone / placeholder phrases in agreement docs
//! * `check_links`      — broken markdown links `[x](./rel.md)` resolving to missing files
//! * `check_placeholders` — lingering `TODO` / `TBD` / `PLACEHOLDER` / `REDACTED` markers
//!
//! Every check returns a `Vec<Finding>`; callers fan the results through a
//! single collector. Checks are intentionally small and independent so new
//! rules can be slotted in without perturbing existing ones.

use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Finding severity — `Error` blocks by default, `Warn` is advisory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warn,
}

impl Severity {
    /// `true` when `self` is at least as severe as `threshold`.
    pub fn at_least(self, threshold: Severity) -> bool {
        matches!(
            (self, threshold),
            (Severity::Error, _) | (Severity::Warn, Severity::Warn)
        )
    }
}

/// A single surfaced issue. `line` is 1-indexed when known.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub check: String,
    pub severity: Severity,
    pub path: PathBuf,
    pub line: Option<u32>,
    pub message: String,
}

impl Finding {
    pub fn error(check: &str, path: impl Into<PathBuf>, line: Option<u32>, msg: impl Into<String>) -> Self {
        Self {
            check: check.into(),
            severity: Severity::Error,
            path: path.into(),
            line,
            message: msg.into(),
        }
    }
    pub fn warn(check: &str, path: impl Into<PathBuf>, line: Option<u32>, msg: impl Into<String>) -> Self {
        Self {
            check: check.into(),
            severity: Severity::Warn,
            path: path.into(),
            line,
            message: msg.into(),
        }
    }
}

/// Mermaid diagram kinds this gate recognises. Keep in alphabetical order.
pub const MERMAID_KINDS: &[&str] = &[
    "classDiagram",
    "erDiagram",
    "flowchart",
    "gantt",
    "graph",
    "journey",
    "mindmap",
    "pie",
    "sequenceDiagram",
    "stateDiagram",
    "timeline",
];

// -------- walker helpers --------

const SKIP_DIRS: &[&str] = &["node_modules", "dist", "target", ".git", ".claude", ".worktrees"];

fn should_skip_dir(name: &str) -> bool {
    SKIP_DIRS.contains(&name)
}

/// Walk `root`, yielding `*.md` paths, skipping dependency/build dirs and
/// refusing to follow symlinks (avoids cycles on synthetic trees).
pub fn md_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if !root.exists() {
        return out;
    }
    let walker = WalkDir::new(root).follow_links(false).into_iter();
    for entry in walker.filter_entry(|e| {
        let name = e.file_name().to_string_lossy();
        !(e.depth() > 0 && e.file_type().is_dir() && should_skip_dir(&name))
    }) {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            out.push(path.to_path_buf());
        }
    }
    out
}

/// Walk `root` for rendered HTML under a `dist/` tree. Used as a hint for
/// broken-asset detection in published docs.
pub fn dist_html_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if !root.exists() {
        return out;
    }
    for entry in WalkDir::new(root).follow_links(false).into_iter().flatten() {
        if entry.file_type().is_file()
            && entry.path().extension().and_then(|s| s.to_str()) == Some("html")
        {
            out.push(entry.path().to_path_buf());
        }
    }
    out
}

/// Best-effort PATH lookup. Returns `true` iff an executable named `cmd`
/// exists on any `PATH` entry.
pub fn which(cmd: &str) -> bool {
    let Ok(path_var) = std::env::var("PATH") else {
        return false;
    };
    for p in std::env::split_paths(&path_var) {
        let candidate = p.join(cmd);
        if candidate.is_file() {
            return true;
        }
    }
    false
}

// -------- mermaid --------

/// Extracts mermaid code blocks as `(start_line, body)` pairs.
/// A block is a run of lines between ```` ```mermaid ```` and a closing ```` ``` ````.
/// If the file has an unmatched opening fence, returns it with body through EOF
/// so callers can flag unbalanced fences.
pub fn mermaid_fences(text: &str) -> Vec<(u32, String, bool /* closed */)> {
    let mut out = Vec::new();
    let mut in_fence = false;
    let mut start_line: u32 = 0;
    let mut body = String::new();
    for (i, line) in text.lines().enumerate() {
        let lineno = (i + 1) as u32;
        let trimmed = line.trim_start();
        if !in_fence {
            if trimmed.starts_with("```mermaid") {
                in_fence = true;
                start_line = lineno;
                body.clear();
            }
        } else if trimmed.starts_with("```") {
            out.push((start_line, std::mem::take(&mut body), true));
            in_fence = false;
        } else {
            body.push_str(line);
            body.push('\n');
        }
    }
    if in_fence {
        out.push((start_line, body, false));
    }
    out
}

/// Heuristic validator for a single mermaid block. Returns `Err(message)` on
/// the first detected problem; `Ok(())` otherwise. Deliberately conservative:
/// we only flag things that would clearly fail to render.
pub fn mermaid_heuristic(body: &str) -> std::result::Result<(), String> {
    let stripped = body.trim();
    if stripped.is_empty() {
        return Err("empty mermaid block".into());
    }
    // First non-comment, non-directive line must start with a known kind.
    let first = stripped
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty() && !l.starts_with("%%") && !l.starts_with("---"))
        .unwrap_or("");
    let head = first.split_whitespace().next().unwrap_or("");
    // `flowchart LR` / `graph TD` style: match on the head word.
    let kind = head.trim_end_matches(':');
    if !MERMAID_KINDS.contains(&kind) {
        return Err(format!("unknown mermaid kind: {kind:?}"));
    }
    // Bracket / paren / brace balance across the whole body.
    let mut paren = 0i32;
    let mut brack = 0i32;
    let mut brace = 0i32;
    let mut dquote = 0i32;
    for ch in body.chars() {
        match ch {
            '(' => paren += 1,
            ')' => paren -= 1,
            '[' => brack += 1,
            ']' => brack -= 1,
            '{' => brace += 1,
            '}' => brace -= 1,
            '"' => dquote += 1,
            _ => {}
        }
        if paren < 0 || brack < 0 || brace < 0 {
            return Err("unbalanced brackets inside mermaid block".into());
        }
    }
    if paren != 0 || brack != 0 || brace != 0 {
        return Err("unbalanced brackets inside mermaid block".into());
    }
    if dquote % 2 != 0 {
        return Err("unbalanced double-quotes inside mermaid block".into());
    }
    Ok(())
}

pub fn check_mermaid(root: &Path) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for file in md_files(root) {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        for (start, body, closed) in mermaid_fences(&text) {
            if !closed {
                findings.push(Finding::error(
                    "mermaid",
                    &file,
                    Some(start),
                    "unbalanced ```mermaid fence (no closing ```)",
                ));
                continue;
            }
            if let Err(msg) = mermaid_heuristic(&body) {
                findings.push(Finding::error("mermaid", &file, Some(start), msg));
            }
        }
    }
    Ok(findings)
}

// -------- latex --------

/// Matches inline `$…$` or block `$$…$$` math. Non-greedy; no nesting.
fn latex_regex() -> Regex {
    Regex::new(r"\$\$(?s:.+?)\$\$|\$(?s:.+?)\$").expect("valid regex")
}

pub fn check_latex(root: &Path) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    let re = latex_regex();
    for file in md_files(root) {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        for m in re.find_iter(&text) {
            // Four consecutive backslashes ("\\\\") inside math is always a
            // double-escape bug — authors meant `\\` (single newline command).
            if m.as_str().contains("\\\\\\\\") {
                let line = 1 + text[..m.start()].bytes().filter(|b| *b == b'\n').count() as u32;
                findings.push(Finding::error(
                    "latex",
                    &file,
                    Some(line),
                    "double-escaped backslash (\\\\\\\\) inside $…$ math — use \\\\ instead",
                ));
            }
        }
    }
    Ok(findings)
}

// -------- video --------

/// Warn on MP4 files smaller than this threshold. Real journey recordings
/// are comfortably > 100 KB; anything smaller is almost always a stub or
/// truncated write.
pub const TINY_MP4_BYTES: u64 = 100 * 1024;

pub fn check_video(root: &Path) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    let public = root.join("public");
    let scan_root = if public.exists() { public } else { root.to_path_buf() };
    if !scan_root.exists() {
        return Ok(findings);
    }
    for entry in WalkDir::new(&scan_root).follow_links(false).into_iter().flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("mp4") {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if meta.len() < TINY_MP4_BYTES {
            findings.push(Finding::warn(
                "video",
                path,
                None,
                format!("mp4 is only {} bytes (< {})", meta.len(), TINY_MP4_BYTES),
            ));
        }
    }
    Ok(findings)
}

// -------- assets --------

fn asset_regex() -> Regex {
    // Matches <img src="..."> / <video src="..."> with single or double quotes.
    Regex::new(r#"<(?:img|video|source)[^>]*\ssrc=["']([^"']+)["']"#).expect("valid regex")
}

pub fn check_assets(root: &Path) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    let re = asset_regex();
    for file in md_files(root) {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let dir = file.parent().unwrap_or(root).to_path_buf();
        for cap in re.captures_iter(&text) {
            let src = &cap[1];
            // Skip remote URLs and data URIs.
            if src.starts_with("http://")
                || src.starts_with("https://")
                || src.starts_with("data:")
            {
                continue;
            }
            let resolved = if let Some(rest) = src.strip_prefix('/') {
                root.join(rest)
            } else {
                dir.join(src)
            };
            if !resolved.exists() {
                let line = 1 + text[..cap.get(0).unwrap().start()]
                    .bytes()
                    .filter(|b| *b == b'\n')
                    .count() as u32;
                findings.push(Finding::error(
                    "assets",
                    &file,
                    Some(line),
                    format!("missing asset: {src}"),
                ));
            }
        }
    }
    Ok(findings)
}

// -------- journey --------

pub fn check_journey(_root: &Path) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    if !which("phenotype-journey") {
        findings.push(Finding::warn(
            "journey",
            PathBuf::from("<env>"),
            None,
            "phenotype-journey not on PATH — journey gates will fall back to warn mode",
        ));
    }
    Ok(findings)
}

// -------- agreement --------

/// Red-zone phrases that must never ship in agreement docs. The phrases are
/// matched case-insensitively as whole-word-ish substrings.
const AGREEMENT_RED_ZONE: &[&str] = &[
    "lorem ipsum",
    "placeholder",
    "redacted",
    "tbd",
    "to be determined",
    "xxx",
    "fixme",
];

pub fn check_agreement(root: &Path) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for file in md_files(root) {
        // Only inspect files under an `agreement[s]` or `quality/agreement*`
        // subtree — everything else uses `check_placeholders`.
        let p = file.to_string_lossy().to_lowercase();
        if !(p.contains("/agreement") || p.contains("agreements/")) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let lower = text.to_lowercase();
        for phrase in AGREEMENT_RED_ZONE {
            if let Some(idx) = lower.find(phrase) {
                let line = 1 + lower[..idx].bytes().filter(|b| *b == b'\n').count() as u32;
                findings.push(Finding::error(
                    "agreement",
                    &file,
                    Some(line),
                    format!("red-zone phrase in agreement doc: {phrase:?}"),
                ));
            }
        }
    }
    Ok(findings)
}

// -------- links --------

fn md_link_regex() -> Regex {
    // [text](target) — target is any run of non-whitespace without ')'.
    Regex::new(r"\[[^\]]+\]\(([^)\s]+)\)").expect("valid regex")
}

pub fn check_links(root: &Path) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    let re = md_link_regex();
    for file in md_files(root) {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let dir = file.parent().unwrap_or(root).to_path_buf();
        for cap in re.captures_iter(&text) {
            let target = cap.get(1).unwrap().as_str();
            // Skip external, in-page, mailto.
            if target.starts_with("http://")
                || target.starts_with("https://")
                || target.starts_with("#")
                || target.starts_with("mailto:")
            {
                continue;
            }
            // Strip fragment.
            let (path_part, _frag) = match target.split_once('#') {
                Some((p, f)) => (p, Some(f)),
                None => (target, None),
            };
            if path_part.is_empty() {
                continue;
            }
            let base = if let Some(rest) = path_part.strip_prefix('/') {
                root.join(rest)
            } else {
                dir.join(path_part)
            };
            // VitePress rewrites `/foo/bar` → `foo/bar.md` or `foo/bar/index.md`.
            // Accept the link if any of these forms exists on disk.
            let candidates = [
                base.clone(),
                base.with_extension("md"),
                base.join("index.md"),
                {
                    let mut p = base.clone();
                    let name = p.file_name().map(|s| s.to_owned());
                    if let Some(name) = name {
                        p.set_file_name(format!("{}.html", name.to_string_lossy()));
                    }
                    p
                },
            ];
            if !candidates.iter().any(|c| c.exists()) {
                let line = 1 + text[..cap.get(0).unwrap().start()]
                    .bytes()
                    .filter(|b| *b == b'\n')
                    .count() as u32;
                findings.push(Finding::error(
                    "links",
                    &file,
                    Some(line),
                    format!("broken link: {target}"),
                ));
            }
        }
    }
    Ok(findings)
}

// -------- placeholders --------

const PLACEHOLDER_MARKERS: &[&str] = &["TODO", "PLACEHOLDER", "REDACTED", "TBD", "FIXME"];

pub fn check_placeholders(root: &Path) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for file in md_files(root) {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        for (i, line) in text.lines().enumerate() {
            for marker in PLACEHOLDER_MARKERS {
                if line.contains(marker) {
                    findings.push(Finding::warn(
                        "placeholders",
                        &file,
                        Some((i + 1) as u32),
                        format!("placeholder marker: {marker}"),
                    ));
                    break;
                }
            }
        }
    }
    Ok(findings)
}

// -------- runner --------

/// Which checks to run. Empty = all.
#[derive(Debug, Default, Clone)]
pub struct RunOptions {
    pub only: HashSet<String>,
}

impl RunOptions {
    fn enabled(&self, name: &str) -> bool {
        self.only.is_empty() || self.only.contains(name)
    }
}

/// Execute every enabled check and return a flat list of findings.
pub fn run_all(root: &Path, opts: &RunOptions) -> Result<Vec<Finding>> {
    let mut out = Vec::new();
    if opts.enabled("mermaid") {
        out.extend(check_mermaid(root)?);
    }
    if opts.enabled("latex") {
        out.extend(check_latex(root)?);
    }
    if opts.enabled("video") {
        out.extend(check_video(root)?);
    }
    if opts.enabled("assets") {
        out.extend(check_assets(root)?);
    }
    if opts.enabled("journey") {
        out.extend(check_journey(root)?);
    }
    if opts.enabled("agreement") {
        out.extend(check_agreement(root)?);
    }
    if opts.enabled("links") {
        out.extend(check_links(root)?);
    }
    if opts.enabled("placeholders") {
        out.extend(check_placeholders(root)?);
    }
    Ok(out)
}

// -------- tests --------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&p, body).unwrap();
        p
    }

    #[test]
    fn heuristic_accepts_known_kind() {
        assert!(mermaid_heuristic("graph TD\n  A --> B\n").is_ok());
        assert!(mermaid_heuristic("flowchart LR\n  A --> B\n").is_ok());
        assert!(mermaid_heuristic("sequenceDiagram\n  A->>B: x\n").is_ok());
    }

    #[test]
    fn heuristic_rejects_unknown_kind() {
        let err = mermaid_heuristic("unknownDiagram\n  a --> b\n").unwrap_err();
        assert!(err.contains("unknown mermaid kind"));
    }

    #[test]
    fn heuristic_flags_unbalanced_brackets() {
        let err = mermaid_heuristic("graph TD\n  A[one --> B\n").unwrap_err();
        assert!(err.contains("bracket"));
    }

    #[test]
    fn heuristic_flags_unbalanced_quotes() {
        let err = mermaid_heuristic("graph TD\n  A[\"open]\n").unwrap_err();
        assert!(err.contains("quote"));
    }

    #[test]
    fn heuristic_accepts_balanced_quotes() {
        assert!(mermaid_heuristic("graph TD\n  A[\"ok\"] --> B\n").is_ok());
    }

    #[test]
    fn fences_extract_and_unbalanced_is_flagged() {
        let text = "intro\n```mermaid\ngraph TD\n  A --> B\n```\nmiddle\n```mermaid\nflowchart LR\n";
        let fences = mermaid_fences(text);
        assert_eq!(fences.len(), 2);
        assert!(fences[0].2, "first fence closed");
        assert!(!fences[1].2, "second fence unbalanced");
    }

    #[test]
    fn latex_double_escape_is_flagged() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "math.md",
            "intro\n\n$a = b \\\\\\\\ c$\n\nmore\n",
        );
        let f = check_latex(dir.path()).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Error);
    }

    #[test]
    fn latex_single_escape_is_fine() {
        let dir = tempdir().unwrap();
        write(dir.path(), "math.md", "$a \\\\ b$\n");
        let f = check_latex(dir.path()).unwrap();
        assert!(f.is_empty(), "single \\\\ must not fire: {f:?}");
    }

    #[test]
    fn tiny_mp4_triggers_warn() {
        let dir = tempdir().unwrap();
        let pub_dir = dir.path().join("public").join("vids");
        fs::create_dir_all(&pub_dir).unwrap();
        fs::write(pub_dir.join("tiny.mp4"), b"not really a video").unwrap();
        let f = check_video(dir.path()).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Warn);
    }

    #[test]
    fn assets_present_vs_missing() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("img")).unwrap();
        fs::write(dir.path().join("img/ok.png"), b"\x89PNG\r\n\x1a\n").unwrap();
        write(
            dir.path(),
            "page.md",
            "<img src=\"img/ok.png\" />\n<img src=\"img/missing.png\" />\n",
        );
        let f = check_assets(dir.path()).unwrap();
        assert_eq!(f.len(), 1);
        assert!(f[0].message.contains("missing.png"));
    }

    #[test]
    fn agreement_red_zone_vs_clean() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "quality/agreements/deal.md",
            "# Deal\n\nAll terms are **TBD** until signed.\n",
        );
        write(
            dir.path(),
            "quality/agreements/clean.md",
            "# Clean\n\nAll terms are bound by Exhibit A.\n",
        );
        let f = check_agreement(dir.path()).unwrap();
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].severity, Severity::Error);
    }

    #[test]
    fn node_modules_is_skipped() {
        let dir = tempdir().unwrap();
        write(dir.path(), "node_modules/pkg/README.md", "TODO leak\n");
        write(dir.path(), "docs/page.md", "clean page\n");
        let files = md_files(dir.path());
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("docs/page.md"));
    }

    #[test]
    fn empty_root_yields_no_findings() {
        let dir = tempdir().unwrap();
        let f = run_all(dir.path(), &RunOptions::default()).unwrap();
        // `journey` may still fire if phenotype-journey is missing on PATH;
        // only assert the non-journey checks are quiet.
        assert!(f.iter().all(|x| x.check == "journey"));
    }

    #[test]
    fn symlink_cycles_do_not_panic() {
        let dir = tempdir().unwrap();
        write(dir.path(), "a.md", "# a\n");
        // Best-effort symlink; on platforms where we can't create one, just
        // confirm the walker tolerates the absence.
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let target = dir.path().join("a.md");
            let link = dir.path().join("loop.md");
            let _ = symlink(target, link);
        }
        let files = md_files(dir.path());
        assert!(!files.is_empty());
    }

    #[test]
    fn links_broken_vs_present() {
        let dir = tempdir().unwrap();
        write(dir.path(), "target.md", "# t\n");
        write(
            dir.path(),
            "page.md",
            "see [ok](./target.md) and [bad](./missing.md)\n",
        );
        let f = check_links(dir.path()).unwrap();
        let broken: Vec<_> = f.iter().filter(|x| x.check == "links").collect();
        assert_eq!(broken.len(), 1);
        assert!(broken[0].message.contains("missing.md"));
    }

    #[test]
    fn placeholders_are_warn() {
        let dir = tempdir().unwrap();
        write(dir.path(), "p.md", "line ok\nline TODO finish\n");
        let f = check_placeholders(dir.path()).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Warn);
    }

    #[test]
    fn run_all_honours_only_filter() {
        let dir = tempdir().unwrap();
        write(dir.path(), "p.md", "TODO\n");
        let mut opts = RunOptions::default();
        opts.only.insert("placeholders".into());
        let f = run_all(dir.path(), &opts).unwrap();
        assert!(f.iter().all(|x| x.check == "placeholders"));
    }

    #[test]
    fn fixture_broken_mermaid_is_flagged() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures");
        if !fixture.exists() {
            return; // fixture dir may not be present in all sandboxes
        }
        let findings = check_mermaid(&fixture).unwrap();
        assert!(
            findings.iter().any(|f| f.severity == Severity::Error),
            "expected at least one Error finding in fixtures, got {:?}",
            findings
        );
    }
}
