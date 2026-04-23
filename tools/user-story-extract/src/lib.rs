//! `user-story-extract` — library surface.
//!
//! Harvests `@phenotype/user-story` YAML frontmatter blocks from test source files
//! (Rust, Swift, Playwright TypeScript, k6 JavaScript) and validates them against
//! the canonical `user-story.schema.json`.
//!
//! This is Batch 1 of the user-story-as-test framework (see
//! `docs-site/architecture/adrs/0034-user-story-test-sourcing.md`). Downstream
//! consumers (manifest generator, auto-doc, traceability matrix) read the JSON
//! index produced by [`extract_paths`].

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

/// Embedded copy of `phenotype-journeys/crates/phenotype-journey-core/schema/user-story.schema.json`.
pub const USER_STORY_SCHEMA: &str = include_str!("../schema/user-story.schema.json");

/// A parsed, schema-validated user-story extracted from a source file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserStory {
    pub journey_id: String,
    pub title: String,
    pub persona: String,
    pub given: String,
    pub when: Vec<String>,
    pub then: Vec<String>,
    pub traces_to: Vec<String>,
    #[serde(default = "default_record")]
    pub record: bool,
    #[serde(default = "default_blind_judge")]
    pub blind_judge: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub backend: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub blind_eval: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub family: Option<String>,

    /// Provenance: absolute or repo-relative path to the source file (added by extractor).
    #[serde(default)]
    pub source_file: String,
    /// Line number (1-based) of the start of the frontmatter block.
    #[serde(default)]
    pub source_line: usize,
}

fn default_record() -> bool { true }
fn default_blind_judge() -> String { "auto".to_string() }

/// Detected source language for a given file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Swift,
    TypeScript,
    JavaScript,
}

impl Language {
    pub fn from_path(p: &Path) -> Option<Self> {
        let ext = p.extension()?.to_str()?;
        match ext {
            "rs" => Some(Self::Rust),
            "swift" => Some(Self::Swift),
            "ts" | "tsx" => Some(Self::TypeScript),
            "js" | "mjs" | "cjs" => Some(Self::JavaScript),
            _ => None,
        }
    }
}

#[derive(Debug, Error)]
pub enum ExtractError {
    #[error("io error reading {path}: {source}")]
    Io { path: PathBuf, #[source] source: std::io::Error },
    #[error("yaml parse error in {path} at line {line}: {source}")]
    Yaml { path: PathBuf, line: usize, #[source] source: serde_yaml::Error },
    #[error("schema validation failed in {path} at line {line}: {errors:?}")]
    Schema { path: PathBuf, line: usize, errors: Vec<String> },
    #[error("duplicate journey_id `{id}`: first at {first}, then at {second}")]
    DuplicateId { id: String, first: String, second: String },
    #[error("unknown FR `{fr}` referenced by {journey_id} at {path}")]
    UnknownFr { fr: String, journey_id: String, path: String },
}

/// Find frontmatter regions per language.
///
/// Returns a vec of `(line_number_1based, yaml_body)` tuples.
pub fn find_frontmatter_blocks(content: &str, lang: Language) -> Vec<(usize, String)> {
    match lang {
        Language::Rust => find_rust(content),
        Language::Swift => find_swift(content),
        Language::TypeScript => find_jsdoc(content),
        Language::JavaScript => find_js_block(content),
    }
}

/// Rust — attribute-style `#[user_story(r#"...yaml..."#)]` or doc-comment fallback
/// `//! @user-story` block above a `#[test]` / `#[tokio::test]`.
///
/// For Batch 1 we support the simpler line-comment form:
///
/// ```text
/// // @user-story
/// // journey_id: foo
/// // ...
/// // @end
/// ```
fn find_rust(content: &str) -> Vec<(usize, String)> {
    find_line_comment_block(content, "//")
}

/// Swift — `// MARK: @user-story` ... `// MARK: @end` line-comment block.
fn find_swift(content: &str) -> Vec<(usize, String)> {
    static START: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s*//\s*MARK:\s*@user-story\s*$").unwrap());
    static END: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s*//\s*MARK:\s*@end\s*$").unwrap());
    find_delimited(content, &START, &END, "//")
}

/// Playwright / TS — JSDoc block containing `@user-story` tag.
///
/// ```text
/// /**
///  * @user-story
///  * journey_id: foo
///  * ...
///  */
/// ```
fn find_jsdoc(content: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim_start().starts_with("/**") {
            // Search for closing */ and @user-story tag.
            let start = i;
            let mut j = i + 1;
            let mut has_tag = false;
            while j < lines.len() {
                let l = lines[j].trim_start();
                if l.contains("@user-story") { has_tag = true; }
                if l.contains("*/") { break; }
                j += 1;
            }
            if has_tag && j < lines.len() {
                // Body = lines[start+1 .. j], strip " * " prefix, drop the @user-story marker line.
                let mut body = String::new();
                for raw in lines.iter().take(j).skip(start + 1).copied() {
                    let stripped = strip_leading(raw, "*");
                    if stripped.trim() == "@user-story" { continue; }
                    body.push_str(&stripped);
                    body.push('\n');
                }
                out.push((start + 1, body));
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// k6 / JS — `/* @user-story ... */` plain block comment.
fn find_js_block(content: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim_start();
        if trimmed.starts_with("/*") && lines[i].contains("@user-story") {
            let start = i;
            // Check for same-line close.
            let rest = &trimmed["/*".len()..];
            if rest.contains("*/") {
                // unusual; ignore
                i += 1;
                continue;
            }
            let mut j = i + 1;
            while j < lines.len() && !lines[j].contains("*/") { j += 1; }
            if j >= lines.len() { break; }
            let mut body = String::new();
            for raw in lines.iter().take(j).skip(start + 1).copied() {
                let stripped = strip_leading(raw, "*");
                body.push_str(&stripped);
                body.push('\n');
            }
            out.push((start + 1, body));
            i = j + 1;
            continue;
        }
        i += 1;
    }
    out
}

/// Generic line-comment block: looks for `// @user-story` ... `// @end` regions.
fn find_line_comment_block(content: &str, prefix: &str) -> Vec<(usize, String)> {
    static START: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s*//\s*@user-story\s*$").unwrap());
    static END: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s*//\s*@end\s*$").unwrap());
    find_delimited(content, &START, &END, prefix)
}

fn find_delimited(
    content: &str,
    start_re: &Regex,
    end_re: &Regex,
    line_prefix: &str,
) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        if start_re.is_match(lines[i]) {
            let start = i + 1;
            let mut j = i + 1;
            while j < lines.len() && !end_re.is_match(lines[j]) { j += 1; }
            if j >= lines.len() { break; }
            let mut body = String::new();
            for raw in lines.iter().take(j).skip(start).copied() {
                let stripped = strip_leading(raw, line_prefix);
                body.push_str(&stripped);
                body.push('\n');
            }
            out.push((start, body));
            i = j + 1;
            continue;
        }
        i += 1;
    }
    out
}

/// Strip a single comment prefix if present, preserving inner indentation.
///
/// For line-comment languages (`//`, `*` inside JSDoc) this looks at the first
/// non-whitespace run: if it is exactly the prefix, it is removed along with
/// one following space. If no prefix is found, the line is returned unchanged
/// (important for block-style comments where YAML scalar indentation is
/// semantically significant).
fn strip_leading(line: &str, prefix: &str) -> String {
    let leading_ws_len = line.len() - line.trim_start().len();
    let (ws, rest) = line.split_at(leading_ws_len);
    if let Some(after) = rest.strip_prefix(prefix) {
        let after = after.strip_prefix(' ').unwrap_or(after);
        // Drop the leading whitespace that sat *before* the prefix — it was
        // comment indentation, not YAML indentation.
        let _ = ws;
        after.to_string()
    } else {
        line.to_string()
    }
}

/// Parse + schema-validate a YAML body. Returns a `UserStory` or a list of
/// schema errors.
pub fn parse_and_validate(
    yaml_body: &str,
    path: &Path,
    line: usize,
) -> Result<UserStory, ExtractError> {
    let raw: serde_json::Value = {
        let yaml_val: serde_yaml::Value = serde_yaml::from_str(yaml_body)
            .map_err(|source| ExtractError::Yaml { path: path.to_path_buf(), line, source })?;
        serde_json::to_value(&yaml_val)
            .map_err(|e| ExtractError::Schema {
                path: path.to_path_buf(),
                line,
                errors: vec![format!("yaml->json conversion: {e}")],
            })?
    };

    let schema: serde_json::Value = serde_json::from_str(USER_STORY_SCHEMA)
        .expect("embedded user-story schema must be valid JSON");
    let compiled = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema)
        .expect("embedded user-story schema must compile");
    if let Err(errors) = compiled.validate(&raw) {
        let msgs: Vec<String> = errors.map(|e| format!("{e}")).collect();
        return Err(ExtractError::Schema {
            path: path.to_path_buf(),
            line,
            errors: msgs,
        });
    }

    let mut story: UserStory = serde_json::from_value(raw).map_err(|e| ExtractError::Schema {
        path: path.to_path_buf(),
        line,
        errors: vec![format!("deserialize: {e}")],
    })?;
    story.source_file = path.display().to_string();
    story.source_line = line;
    // Auto-infer family from path if absent.
    if story.family.is_none() {
        story.family = Some(infer_family(path));
    }
    Ok(story)
}

/// Infer family from a path heuristically.
pub fn infer_family(path: &Path) -> String {
    let s = path.to_string_lossy();
    if s.contains("apps/streamlit/journeys") || s.contains("playwright") { "streamlit".into() }
    else if s.contains("apps/macos") || s.contains("UITests") || s.contains("apps/windows") || s.contains("apps/linux") { "gui".into() }
    else if s.contains("load/") || s.ends_with("k6.js") || s.contains("/k6/") { "k6".into() }
    else if s.contains("tests/fixtures/user-story") {
        // Fixtures: infer by extension.
        match Language::from_path(path) {
            Some(Language::TypeScript) => "streamlit".into(),
            Some(Language::Swift) => "gui".into(),
            Some(Language::JavaScript) => "k6".into(),
            Some(Language::Rust) => "cli".into(),
            _ => "other".into(),
        }
    }
    else if s.contains("cli-journeys") || s.contains("crates/") { "cli".into() }
    else { "other".into() }
}

/// Walk a set of roots, harvest stories from every candidate file.
///
/// Reports errors as it goes but returns the union of successful stories + any
/// failures that were captured.
pub fn extract_paths(roots: &[PathBuf]) -> (Vec<UserStory>, Vec<ExtractError>) {
    let mut stories = Vec::new();
    let mut errors = Vec::new();
    for root in roots {
        if !root.exists() { continue; }
        for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_file() { continue; }
            let Some(lang) = Language::from_path(path) else { continue; };
            let content = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(source) => {
                    errors.push(ExtractError::Io { path: path.to_path_buf(), source });
                    continue;
                }
            };
            for (line, body) in find_frontmatter_blocks(&content, lang) {
                match parse_and_validate(&body, path, line) {
                    Ok(story) => stories.push(story),
                    Err(e) => errors.push(e),
                }
            }
        }
    }
    (stories, errors)
}

/// Detect duplicate `journey_id` across stories.
pub fn check_duplicate_ids(stories: &[UserStory]) -> Vec<ExtractError> {
    use std::collections::HashMap;
    let mut seen: HashMap<&str, &UserStory> = HashMap::new();
    let mut errors = Vec::new();
    for s in stories {
        if let Some(prev) = seen.get(s.journey_id.as_str()) {
            errors.push(ExtractError::DuplicateId {
                id: s.journey_id.clone(),
                first: format!("{}:{}", prev.source_file, prev.source_line),
                second: format!("{}:{}", s.source_file, s.source_line),
            });
        } else {
            seen.insert(&s.journey_id, s);
        }
    }
    errors
}

/// Cross-reference `traces_to` entries against a set of known FR IDs. Any
/// story citing an FR not in `known_frs` becomes an `UnknownFr` error.
pub fn check_coverage(
    stories: &[UserStory],
    known_frs: &BTreeSet<String>,
) -> Vec<ExtractError> {
    let mut errors = Vec::new();
    for s in stories {
        for fr in &s.traces_to {
            if !known_frs.contains(fr) {
                errors.push(ExtractError::UnknownFr {
                    fr: fr.clone(),
                    journey_id: s.journey_id.clone(),
                    path: format!("{}:{}", s.source_file, s.source_line),
                });
            }
        }
    }
    errors
}

/// Parse FR IDs out of a Markdown document (e.g. `PRD.md`). A line that
/// contains an `FR-XXX` token (matching `FR-[A-Z0-9_-]+`) contributes its IDs.
pub fn parse_fr_list(markdown: &str) -> BTreeSet<String> {
    static FR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"FR-[A-Z0-9][A-Z0-9_-]*").unwrap());
    FR_RE.find_iter(markdown).map(|m| m.as_str().to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf { PathBuf::from(s) }

    #[test]
    fn rust_parser_happy_path() {
        let src = "\
fn boilerplate() {}

// @user-story
// journey_id: rust-demo
// title: Rust demo
// persona: developer
// given: A workspace
// when:
//   - open editor
// then:
//   - sees code
// traces_to: [FR-DEV-001]
// @end
#[test]
fn t() {}
";
        let blocks = find_frontmatter_blocks(src, Language::Rust);
        assert_eq!(blocks.len(), 1);
        let (line, body) = &blocks[0];
        assert!(*line >= 3, "line number should point into the block");
        let story = parse_and_validate(body, &p("demo.rs"), *line).unwrap();
        assert_eq!(story.journey_id, "rust-demo");
        assert_eq!(story.traces_to, vec!["FR-DEV-001"]);
        assert!(story.record, "record defaults to true");
        assert_eq!(story.blind_judge, "auto");
    }

    #[test]
    fn rust_parser_malformed_yaml_errors() {
        let src = "\
// @user-story
// journey_id: broken
//   invalid yaml
// @end
";
        let blocks = find_frontmatter_blocks(src, Language::Rust);
        assert_eq!(blocks.len(), 1);
        let err = parse_and_validate(&blocks[0].1, &p("x.rs"), 1).unwrap_err();
        match err {
            ExtractError::Yaml { .. } | ExtractError::Schema { .. } => {}
            other => panic!("expected Yaml/Schema error, got {other:?}"),
        }
    }

    #[test]
    fn swift_parser_mark_block() {
        let src = "\
import XCTest
// MARK: @user-story
// journey_id: swift-demo
// title: Swift demo
// persona: macOS user
// given: The app is launched
// when:
//   - tap button
// then:
//   - sees sheet
// traces_to: [FR-UI-001]
// MARK: @end
class MyTest: XCTestCase {}
";
        let blocks = find_frontmatter_blocks(src, Language::Swift);
        assert_eq!(blocks.len(), 1);
        let story = parse_and_validate(&blocks[0].1, &p("x.swift"), blocks[0].0).unwrap();
        assert_eq!(story.journey_id, "swift-demo");
    }

    #[test]
    fn swift_parser_missing_required_field_fails_schema() {
        let src = "\
// MARK: @user-story
// journey_id: broken
// title: Missing stuff
// persona: x
// given: y
// when: [a]
// then: [b]
// MARK: @end
";
        let blocks = find_frontmatter_blocks(src, Language::Swift);
        let err = parse_and_validate(&blocks[0].1, &p("x.swift"), 1).unwrap_err();
        assert!(matches!(err, ExtractError::Schema { .. }));
    }

    #[test]
    fn playwright_jsdoc_block() {
        let src = "\
import { test } from '@playwright/test';

/**
 * @user-story
 * journey_id: playwright-demo
 * title: Playwright demo
 * persona: qa engineer
 * given: The browser is open
 * when:
 *   - click login
 * then:
 *   - sees dashboard
 * traces_to: [FR-UI-002]
 */
test('demo', async () => {});
";
        let blocks = find_frontmatter_blocks(src, Language::TypeScript);
        assert_eq!(blocks.len(), 1);
        let story = parse_and_validate(&blocks[0].1, &p("x.spec.ts"), blocks[0].0).unwrap();
        assert_eq!(story.journey_id, "playwright-demo");
    }

    #[test]
    fn playwright_malformed_fails() {
        // Not a jsdoc but contains @user-story — should be ignored (no block found).
        let src = "// @user-story inline not a jsdoc\n";
        let blocks = find_frontmatter_blocks(src, Language::TypeScript);
        assert!(blocks.is_empty());
    }

    #[test]
    fn k6_block_comment() {
        let src = "\
import http from 'k6/http';
/* @user-story
journey_id: k6-demo
title: k6 demo
persona: load runner
given: fleet under load
when:
  - fire requests
then:
  - p95 under 200ms
traces_to: [FR-TEL-001]
*/
export default function () {}
";
        let blocks = find_frontmatter_blocks(src, Language::JavaScript);
        assert_eq!(blocks.len(), 1);
        let story = parse_and_validate(&blocks[0].1, &p("load/k6.js"), blocks[0].0).unwrap();
        assert_eq!(story.journey_id, "k6-demo");
        assert_eq!(story.family.as_deref(), Some("k6"));
    }

    #[test]
    fn k6_malformed_schema() {
        // traces_to missing — schema violation.
        let src = "\
/* @user-story
journey_id: bad
title: bad
persona: x
given: y
when: [a]
then: [b]
*/
";
        let blocks = find_frontmatter_blocks(src, Language::JavaScript);
        let err = parse_and_validate(&blocks[0].1, &p("x.js"), 1).unwrap_err();
        assert!(matches!(err, ExtractError::Schema { .. }));
    }

    #[test]
    fn duplicate_ids_detected() {
        let a = UserStory {
            journey_id: "same".into(),
            title: "A".into(), persona: "p".into(), given: "g".into(),
            when: vec!["x".into()], then: vec!["y".into()],
            traces_to: vec!["FR-001".into()],
            record: true, blind_judge: "auto".into(),
            backend: None, blind_eval: None, family: None,
            source_file: "a.rs".into(), source_line: 1,
        };
        let mut b = a.clone(); b.source_file = "b.rs".into();
        let errs = check_duplicate_ids(&[a, b]);
        assert_eq!(errs.len(), 1);
        assert!(matches!(&errs[0], ExtractError::DuplicateId { .. }));
    }

    #[test]
    fn unknown_fr_detected() {
        let s = UserStory {
            journey_id: "j".into(), title: "T".into(), persona: "p".into(),
            given: "g".into(), when: vec!["x".into()], then: vec!["y".into()],
            traces_to: vec!["FR-GHOST".into()],
            record: true, blind_judge: "auto".into(),
            backend: None, blind_eval: None, family: None,
            source_file: "a.rs".into(), source_line: 1,
        };
        let mut known = BTreeSet::new();
        known.insert("FR-001".to_string());
        let errs = check_coverage(&[s], &known);
        assert_eq!(errs.len(), 1);
        assert!(matches!(&errs[0], ExtractError::UnknownFr { .. }));
    }

    #[test]
    fn parse_fr_list_picks_up_markdown_tokens() {
        let md = "- FR-001 description\n- FR-FLEET-002: more";
        let got = parse_fr_list(md);
        assert!(got.contains("FR-001"));
        assert!(got.contains("FR-FLEET-002"));
    }

    #[test]
    fn infer_family_paths() {
        assert_eq!(infer_family(&p("apps/streamlit/journeys/specs/x.spec.ts")), "streamlit");
        assert_eq!(infer_family(&p("apps/macos/HwLedgerUITests/x.swift")), "gui");
        assert_eq!(infer_family(&p("load/perf.js")), "k6");
        assert_eq!(infer_family(&p("crates/foo/tests/bar.rs")), "cli");
    }
}
