//! Unified model-source resolver for the hwLedger Planner combobox.
//!
//! Traces to: FR-HF-001, FR-PLAN-003
//!
//! Accepts four input styles and normalises them into a single
//! [`ModelSource`] enum:
//!
//! 1. **Free text** — e.g. `"deepseek"`: returns [`ResolveError::AmbiguousQuery`]
//!    so the caller can pivot to an HF search + golden-fixture match.
//! 2. **Model id** — e.g. `deepseek-ai/DeepSeek-V3`: [`ModelSource::HfRepo`].
//! 3. **HF URL** — e.g. `https://huggingface.co/<org>/<name>[/tree/<rev>]`:
//!    [`ModelSource::HfRepo`] with optional revision.
//! 4. **Golden shortcut** — e.g. `gold:deepseek-v3`: [`ModelSource::GoldenFixture`]
//!    resolved against the workspace `tests/golden/<name>.json` file.
//!
//! Absolute `.json` paths are treated as [`ModelSource::LocalConfig`].

use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use thiserror::Error;

/// A resolved model source ready to be ingested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelSource {
    /// Local golden fixture JSON (`tests/golden/<name>.json`).
    GoldenFixture(PathBuf),
    /// Hugging Face Hub repo (optionally pinned to a revision / branch / tag).
    HfRepo { repo_id: String, revision: Option<String> },
    /// Absolute path to a user-supplied `config.json` on disk.
    LocalConfig(PathBuf),
}

/// Errors returned by [`resolve`].
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ResolveError {
    /// Input was empty or whitespace-only.
    #[error("input is empty")]
    Empty,

    /// Input did not match any of the structured patterns. The caller should
    /// fall back to an HF search (and/or golden-fixture fuzzy match) and
    /// surface the contained `hint` to the user as the search text.
    #[error("ambiguous query: {hint}")]
    AmbiguousQuery { hint: String },

    /// A `gold:<name>` shortcut pointed at a fixture that does not exist.
    #[error("golden fixture not found: {0}")]
    GoldenNotFound(String),

    /// Input looked like a path but was not absolute / not a `.json` file.
    #[error("invalid local config path: {0}")]
    InvalidLocalPath(String),
}

/// Regex for a bare HF `org/name` repo id (no URL scheme).
///
/// HF accepts `A-Za-z0-9_.-` in both segments, separated by exactly one `/`.
fn repo_id_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^[A-Za-z0-9][A-Za-z0-9._-]*/[A-Za-z0-9][A-Za-z0-9._-]*$")
            .expect("repo-id regex must compile")
    })
}

/// Regex for an HF Hub URL. Extracts `org`, `name`, and optional `revision`.
fn hf_url_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?x)
            ^https?://(?:www\.)?huggingface\.co/
            (?P<org>[A-Za-z0-9][A-Za-z0-9._-]*)/
            (?P<name>[A-Za-z0-9][A-Za-z0-9._-]*)
            (?:/(?:tree|blob|resolve|commit)/(?P<rev>[A-Za-z0-9._/-]+?))?
            /?$
            ",
        )
        .expect("hf-url regex must compile")
    })
}

/// Workspace-relative golden-fixture directory. Computed from `CARGO_MANIFEST_DIR`
/// at build time; falls back to the CWD-relative `tests/golden/` used by the
/// published crate.
fn golden_dir() -> PathBuf {
    // crates/hwledger-ingest/ -> workspace root -> tests/golden
    let manifest = env!("CARGO_MANIFEST_DIR");
    let mut p = PathBuf::from(manifest);
    p.pop(); // hwledger-ingest -> crates
    p.pop(); // crates -> workspace root
    p.push("tests");
    p.push("golden");
    p
}

/// Resolve a user-supplied string into a [`ModelSource`].
///
/// Rule matrix:
///
/// | Input form | Dispatch |
/// |---|---|
/// | empty / whitespace | [`ResolveError::Empty`] |
/// | `gold:<name>` | [`ModelSource::GoldenFixture`] (or `GoldenNotFound`) |
/// | absolute `.json` path | [`ModelSource::LocalConfig`] |
/// | `http(s)://huggingface.co/<org>/<name>[/tree/<rev>]` | [`ModelSource::HfRepo`] |
/// | `<org>/<name>` matching the repo-id regex | [`ModelSource::HfRepo`] |
/// | anything else | [`ResolveError::AmbiguousQuery`] |
pub fn resolve(input: &str) -> Result<ModelSource, ResolveError> {
    resolve_with_golden_dir(input, &golden_dir())
}

/// Same as [`resolve`] but with an injectable golden-fixture root. Useful for
/// unit tests and for the Streamlit/Swift callers that ship their own fixture
/// directory.
pub fn resolve_with_golden_dir(
    input: &str,
    golden_dir: &Path,
) -> Result<ModelSource, ResolveError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ResolveError::Empty);
    }

    // 1. Golden shortcut: `gold:<name>`
    if let Some(rest) = trimmed.strip_prefix("gold:") {
        let name = rest.trim();
        if name.is_empty() {
            return Err(ResolveError::GoldenNotFound(String::new()));
        }
        let mut path = golden_dir.to_path_buf();
        // Normalise: allow `.json` suffix or bare stem.
        let with_ext =
            if name.ends_with(".json") { name.to_string() } else { format!("{}.json", name) };
        path.push(&with_ext);
        if !path.exists() {
            return Err(ResolveError::GoldenNotFound(name.to_string()));
        }
        return Ok(ModelSource::GoldenFixture(path));
    }

    // 2. Absolute JSON path.
    if trimmed.starts_with('/') || trimmed.starts_with("~/") {
        let expanded = if let Some(rest) = trimmed.strip_prefix("~/") {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join(rest)
        } else {
            PathBuf::from(trimmed)
        };
        if expanded.extension().and_then(|s| s.to_str()) != Some("json") {
            return Err(ResolveError::InvalidLocalPath(trimmed.to_string()));
        }
        return Ok(ModelSource::LocalConfig(expanded));
    }

    // 3. HF URL.
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        if let Some(caps) = hf_url_regex().captures(trimmed) {
            let org = caps.name("org").unwrap().as_str();
            let name = caps.name("name").unwrap().as_str();
            let rev = caps.name("rev").map(|m| m.as_str().trim_end_matches('/').to_string());
            // HF "main" is the default; surface explicitly only when not `main`.
            let revision = rev.filter(|r| !r.is_empty() && r != "main");
            return Ok(ModelSource::HfRepo { repo_id: format!("{}/{}", org, name), revision });
        }
        return Err(ResolveError::AmbiguousQuery { hint: trimmed.to_string() });
    }

    // 4. Bare `org/name` repo id.
    if repo_id_regex().is_match(trimmed) {
        return Ok(ModelSource::HfRepo { repo_id: trimmed.to_string(), revision: None });
    }

    // 5. Fall-through — free text. Strip any leading/trailing quotes.
    let hint = trimmed.trim_matches('"').trim_matches('\'').to_string();
    Err(ResolveError::AmbiguousQuery { hint })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // Traces to: FR-HF-001
    #[test]
    fn resolve_bare_repo_id() {
        let r = resolve("deepseek-ai/DeepSeek-V3").unwrap();
        assert_eq!(
            r,
            ModelSource::HfRepo { repo_id: "deepseek-ai/DeepSeek-V3".into(), revision: None }
        );
    }

    // Traces to: FR-HF-001
    #[test]
    fn resolve_repo_id_with_dots_and_dashes() {
        assert!(matches!(
            resolve("meta-llama/Llama-3.1-8B-Instruct").unwrap(),
            ModelSource::HfRepo { .. }
        ));
    }

    // Traces to: FR-HF-001
    #[test]
    fn resolve_hf_url_with_tree_revision() {
        let r = resolve("https://huggingface.co/deepseek-ai/DeepSeek-V3/tree/main").unwrap();
        // `main` is the default and should be dropped.
        assert_eq!(
            r,
            ModelSource::HfRepo { repo_id: "deepseek-ai/DeepSeek-V3".into(), revision: None }
        );
    }

    // Traces to: FR-HF-001
    #[test]
    fn resolve_hf_url_with_non_main_revision() {
        let r = resolve("https://huggingface.co/org/name/tree/v0.2").unwrap();
        assert_eq!(
            r,
            ModelSource::HfRepo { repo_id: "org/name".into(), revision: Some("v0.2".into()) }
        );
    }

    // Traces to: FR-HF-001
    #[test]
    fn resolve_hf_url_root_only() {
        let r = resolve("https://huggingface.co/meta-llama/Llama-3-70B").unwrap();
        assert_eq!(
            r,
            ModelSource::HfRepo { repo_id: "meta-llama/Llama-3-70B".into(), revision: None }
        );
    }

    // Traces to: FR-HF-001
    #[test]
    fn resolve_non_hf_url_is_ambiguous() {
        let err = resolve("https://example.com/foo/bar").unwrap_err();
        assert!(matches!(err, ResolveError::AmbiguousQuery { .. }));
    }

    // Traces to: FR-HF-001
    #[test]
    fn resolve_free_text_is_ambiguous() {
        let err = resolve("deepseek v3").unwrap_err();
        match err {
            ResolveError::AmbiguousQuery { hint } => assert_eq!(hint, "deepseek v3"),
            other => panic!("expected AmbiguousQuery, got {:?}", other),
        }
    }

    // Traces to: FR-HF-001
    #[test]
    fn resolve_empty_is_empty_error() {
        assert_eq!(resolve("   ").unwrap_err(), ResolveError::Empty);
    }

    // Traces to: FR-HF-001
    #[test]
    fn resolve_golden_shortcut_hits_fixture() {
        let dir = tempdir().unwrap();
        let fixture = dir.path().join("deepseek-v3.json");
        fs::write(&fixture, "{}").unwrap();

        let r = resolve_with_golden_dir("gold:deepseek-v3", dir.path()).unwrap();
        assert_eq!(r, ModelSource::GoldenFixture(fixture));
    }

    // Traces to: FR-HF-001
    #[test]
    fn resolve_golden_shortcut_missing_fixture() {
        let dir = tempdir().unwrap();
        let err = resolve_with_golden_dir("gold:does-not-exist", dir.path()).unwrap_err();
        assert!(matches!(err, ResolveError::GoldenNotFound(_)));
    }

    // Traces to: FR-HF-001
    #[test]
    fn resolve_absolute_json_path_is_local_config() {
        let dir = tempdir().unwrap();
        let abs = dir.path().join("my-config.json");
        fs::write(&abs, "{}").unwrap();
        let r = resolve(abs.to_str().unwrap()).unwrap();
        assert_eq!(r, ModelSource::LocalConfig(abs));
    }

    // Traces to: FR-HF-001
    #[test]
    fn resolve_absolute_non_json_path_rejected() {
        let err = resolve("/etc/hosts").unwrap_err();
        assert!(matches!(err, ResolveError::InvalidLocalPath(_)));
    }

    // Traces to: FR-HF-001
    #[test]
    fn resolve_single_token_is_ambiguous() {
        // Free text with no slash, no prefix — must route to search.
        let err = resolve("deepseek").unwrap_err();
        assert!(matches!(err, ResolveError::AmbiguousQuery { .. }));
    }

    // Traces to: FR-HF-001
    #[test]
    fn resolve_strips_surrounding_quotes_in_hint() {
        let err = resolve("\"deepseek v3\"").unwrap_err();
        match err {
            ResolveError::AmbiguousQuery { hint } => assert_eq!(hint, "deepseek v3"),
            other => panic!("unexpected: {:?}", other),
        }
    }
}
