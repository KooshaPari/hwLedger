//! Inlined intent ↔ blind-description agreement scoring.
//!
//! Mirrors `phenotype_journey_core::agreement` — trait + three backends
//! (Jaccard / Sentence-Transformer / SigLIP) selected by
//! `AgreementBackend` — so vlm-judge can compute Green/Yellow/Red buckets
//! without pulling phenotype-journey-core into the hwLedger workspace
//! graph. When the upstream crate lands as a vendored dependency, swap
//! this file for a re-export.
//!
//! Thresholds are backend-specific:
//!
//! | Backend              | Green | Yellow | Red |
//! |----------------------|-------|--------|-----|
//! | Jaccard              | ≥0.60 | 0.30   | <0.30 |
//! | SentenceTransformer  | ≥0.75 | 0.50   | <0.50 |
//! | SigLip               | ≥0.30 | 0.10   | <0.10 |

use rust_stemmers::{Algorithm, Stemmer};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agreement {
    Green,
    Yellow,
    Red,
}

impl Agreement {
    pub fn as_str(&self) -> &'static str {
        match self {
            Agreement::Green => "green",
            Agreement::Yellow => "yellow",
            Agreement::Red => "red",
        }
    }
    /// Green/Yellow -> passed, Red -> not passed.
    pub fn is_passed(&self) -> bool {
        matches!(self, Agreement::Green | Agreement::Yellow)
    }
}

#[derive(Debug, Clone)]
pub struct AgreementReport {
    pub status: Agreement,
    /// Display-normalised score in [0.0, 1.0].
    pub overlap: f64,
    /// Backend-native raw score (Jaccard / cosine / SigLIP prob).
    pub raw_score: f64,
    pub backend: String,
    pub backend_model: Option<String>,
    pub missing_in_blind: Vec<String>,
    pub extras_in_blind: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum AgreementBackend {
    Jaccard,
    SentenceTransformer { model: String },
    SigLip { model: String },
    Auto,
}

impl Default for AgreementBackend {
    fn default() -> Self {
        AgreementBackend::Auto
    }
}

impl AgreementBackend {
    pub fn parse_flag(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "jaccard" | "token" => Ok(AgreementBackend::Jaccard),
            "sentence" | "sentence-transformer" | "st" => {
                Ok(AgreementBackend::SentenceTransformer {
                    model: "all-mpnet-base-v2".into(),
                })
            }
            "siglip" => Ok(AgreementBackend::SigLip {
                model: "google/siglip-so400m-patch14-384".into(),
            }),
            "auto" => Ok(AgreementBackend::Auto),
            other => Err(format!(
                "unknown agreement backend '{other}' (expected one of: jaccard, sentence, siglip, auto)"
            )),
        }
    }

    pub fn resolve_concrete(&self, image_available: bool) -> AgreementBackend {
        match self {
            AgreementBackend::Auto => {
                if image_available
                    && python_module_importable("transformers")
                    && python_module_importable("torch")
                {
                    AgreementBackend::SigLip {
                        model: "google/siglip-so400m-patch14-384".into(),
                    }
                } else if python_module_importable("sentence_transformers") {
                    AgreementBackend::SentenceTransformer {
                        model: "all-mpnet-base-v2".into(),
                    }
                } else {
                    AgreementBackend::Jaccard
                }
            }
            other => other.clone(),
        }
    }

    pub fn build(&self, image_available: bool) -> Box<dyn AgreementScorer> {
        match self.resolve_concrete(image_available) {
            AgreementBackend::Jaccard => Box::new(JaccardScorer),
            AgreementBackend::SentenceTransformer { model } => {
                Box::new(SentenceTransformerScorer { model })
            }
            AgreementBackend::SigLip { model } => Box::new(SigLipScorer { model }),
            AgreementBackend::Auto => Box::new(JaccardScorer),
        }
    }
}

pub trait AgreementScorer: Send + Sync {
    fn score(&self, intent: &str, blind: &str, image: Option<&Path>) -> AgreementReport;
    fn name(&self) -> &'static str;
}

fn python_cmd() -> Option<String> {
    if let Ok(p) = std::env::var("HWLEDGER_AGREEMENT_PYTHON") {
        if !p.is_empty() {
            return Some(p);
        }
    }
    for cand in ["python3", "python"] {
        if Command::new(cand).arg("--version").output().is_ok() {
            return Some(cand.to_string());
        }
    }
    None
}

fn python_module_importable(module: &str) -> bool {
    static CACHE: OnceLock<std::sync::Mutex<std::collections::HashMap<String, bool>>> =
        OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    if let Some(v) = cache.lock().ok().and_then(|g| g.get(module).copied()) {
        return v;
    }
    let ok = python_cmd()
        .and_then(|py| {
            Command::new(py)
                .args(["-c", &format!("import {module}")])
                .output()
                .ok()
        })
        .map(|o| o.status.success())
        .unwrap_or(false);
    if let Ok(mut g) = cache.lock() {
        g.insert(module.to_string(), ok);
    }
    ok
}

// ---------------------------------------------------------------------------
// Jaccard
// ---------------------------------------------------------------------------

const STOPWORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "but", "if", "then", "else", "of", "for", "to", "in", "on",
    "at", "by", "with", "from", "as", "is", "are", "was", "were", "be", "been", "being", "it",
    "its", "this", "that", "these", "those", "i", "you", "he", "she", "we", "they", "them", "his",
    "her", "their", "our", "your", "my", "me", "him", "us", "do", "does", "did", "have", "has",
    "had", "will", "would", "should", "could", "can", "may", "might", "must", "so", "than", "when",
    "while", "where", "who", "what", "which", "some", "any", "all", "no", "not", "out", "up",
    "down", "into", "over", "under", "again", "about", "after", "before", "just", "also", "only",
    "very", "too", "there", "here", "s", "t",
];

fn is_stopword(w: &str) -> bool {
    w.len() <= 1 || STOPWORDS.contains(&w)
}

pub fn tokenise(text: &str) -> Vec<String> {
    let stemmer = Stemmer::create(Algorithm::English);
    let mut out: BTreeSet<String> = BTreeSet::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            for c in ch.to_lowercase() {
                current.push(c);
            }
        } else if !current.is_empty() {
            push_token(&stemmer, &current, &mut out);
            current.clear();
        }
    }
    if !current.is_empty() {
        push_token(&stemmer, &current, &mut out);
    }
    out.into_iter().collect()
}

fn push_token(stemmer: &Stemmer, word: &str, out: &mut BTreeSet<String>) {
    if is_stopword(word) {
        return;
    }
    let stem = stemmer.stem(word).to_string();
    if stem.is_empty() || is_stopword(&stem) {
        return;
    }
    out.insert(stem);
}

pub struct JaccardScorer;

impl AgreementScorer for JaccardScorer {
    fn name(&self) -> &'static str {
        "jaccard"
    }
    fn score(&self, intent: &str, blind: &str, _image: Option<&Path>) -> AgreementReport {
        let intent_tokens = tokenise(intent);
        let blind_tokens = tokenise(blind);
        let intent_set: BTreeSet<&String> = intent_tokens.iter().collect();
        let blind_set: BTreeSet<&String> = blind_tokens.iter().collect();
        let overlap = if intent_set.is_empty() && blind_set.is_empty() {
            1.0
        } else if intent_set.is_empty() || blind_set.is_empty() {
            0.0
        } else {
            let inter = intent_set.intersection(&blind_set).count() as f64;
            let union = intent_set.union(&blind_set).count() as f64;
            inter / union
        };
        let status = if overlap >= 0.6 {
            Agreement::Green
        } else if overlap >= 0.3 {
            Agreement::Yellow
        } else {
            Agreement::Red
        };
        let missing_in_blind: Vec<String> =
            intent_set.difference(&blind_set).map(|s| (*s).clone()).collect();
        let extras_in_blind: Vec<String> =
            blind_set.difference(&intent_set).map(|s| (*s).clone()).collect();
        AgreementReport {
            status,
            overlap,
            raw_score: overlap,
            backend: "jaccard".into(),
            backend_model: None,
            missing_in_blind,
            extras_in_blind,
        }
    }
}

// ---------------------------------------------------------------------------
// Sentence-Transformer
// ---------------------------------------------------------------------------

pub struct SentenceTransformerScorer {
    pub model: String,
}

impl AgreementScorer for SentenceTransformerScorer {
    fn name(&self) -> &'static str {
        "sentence-transformer"
    }
    fn score(&self, intent: &str, blind: &str, _image: Option<&Path>) -> AgreementReport {
        let cosine = match run_sentence_transformer(&self.model, intent, blind) {
            Ok(c) => c,
            Err(_) => {
                let mut fb = JaccardScorer.score(intent, blind, None);
                fb.backend = "jaccard-fallback:sentence-transformer".into();
                return fb;
            }
        };
        let c = cosine.clamp(-1.0, 1.0);
        let overlap_display = ((c + 1.0) / 2.0).clamp(0.0, 1.0);
        let status = if c >= 0.75 {
            Agreement::Green
        } else if c >= 0.5 {
            Agreement::Yellow
        } else {
            Agreement::Red
        };
        AgreementReport {
            status,
            overlap: overlap_display,
            raw_score: c,
            backend: "sentence-transformer".into(),
            backend_model: Some(self.model.clone()),
            missing_in_blind: vec![],
            extras_in_blind: vec![],
        }
    }
}

fn run_sentence_transformer(model: &str, intent: &str, blind: &str) -> Result<f64, String> {
    let py = python_cmd().ok_or("python not on PATH")?;
    let script = r#"
import json, sys
from sentence_transformers import SentenceTransformer, util
payload = json.load(sys.stdin)
m = SentenceTransformer(payload["model"])
a, b = m.encode([payload["intent"], payload["blind"]])
print(util.cos_sim(a, b).item())
"#;
    let mut child = Command::new(&py)
        .args(["-c", script])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn: {e}"))?;
    let payload = serde_json::json!({
        "model": model,
        "intent": intent,
        "blind": blind,
    });
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(payload.to_string().as_bytes())
            .map_err(|e| format!("write: {e}"))?;
    }
    let out = child.wait_with_output().map_err(|e| format!("wait: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "sentence-transformer failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .lines()
        .last()
        .ok_or("empty stdout".to_string())?
        .parse::<f64>()
        .map_err(|e| format!("parse: {e}"))
}

// ---------------------------------------------------------------------------
// SigLIP
// ---------------------------------------------------------------------------

pub struct SigLipScorer {
    pub model: String,
}

impl AgreementScorer for SigLipScorer {
    fn name(&self) -> &'static str {
        "siglip"
    }
    fn score(&self, intent: &str, blind: &str, image: Option<&Path>) -> AgreementReport {
        let img = match image {
            Some(p) if p.exists() => p.to_path_buf(),
            _ => {
                let mut fb = JaccardScorer.score(intent, blind, None);
                fb.backend = "jaccard-fallback:siglip-no-image".into();
                return fb;
            }
        };
        let raw = match run_siglip(&self.model, intent, &img) {
            Ok(r) => r,
            Err(_) => {
                let mut fb = JaccardScorer.score(intent, blind, None);
                fb.backend = "jaccard-fallback:siglip".into();
                return fb;
            }
        };
        let status = if raw >= 0.3 {
            Agreement::Green
        } else if raw >= 0.1 {
            Agreement::Yellow
        } else {
            Agreement::Red
        };
        AgreementReport {
            status,
            overlap: raw.clamp(0.0, 1.0),
            raw_score: raw,
            backend: "siglip".into(),
            backend_model: Some(self.model.clone()),
            missing_in_blind: vec![],
            extras_in_blind: vec![],
        }
    }
}

fn run_siglip(model: &str, intent: &str, image: &Path) -> Result<f64, String> {
    let py = python_cmd().ok_or("python not on PATH")?;
    let script = r#"
import json, sys, torch
from transformers import AutoProcessor, AutoModel
from PIL import Image
payload = json.load(sys.stdin)
processor = AutoProcessor.from_pretrained(payload["model"])
model = AutoModel.from_pretrained(payload["model"])
img = Image.open(payload["image"]).convert("RGB")
inputs = processor(text=[payload["intent"]], images=[img], return_tensors="pt", padding=True)
with torch.no_grad():
    out = model(**inputs)
    prob = torch.sigmoid(out.logits_per_image).item()
print(prob)
"#;
    let mut child = Command::new(&py)
        .args(["-c", script])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn: {e}"))?;
    let payload = serde_json::json!({
        "model": model,
        "intent": intent,
        "image": image.display().to_string(),
    });
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(payload.to_string().as_bytes())
            .map_err(|e| format!("write: {e}"))?;
    }
    let out = child.wait_with_output().map_err(|e| format!("wait: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "siglip failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .lines()
        .last()
        .ok_or("empty stdout".to_string())?
        .parse::<f64>()
        .map_err(|e| format!("parse: {e}"))
}

// ---------------------------------------------------------------------------
// Backward-compat free fn + helpers
// ---------------------------------------------------------------------------

/// Legacy Jaccard-only API. New callers should plumb a concrete
/// [`AgreementBackend`] instead.
pub fn score(intent: &str, blind: &str) -> AgreementReport {
    JaccardScorer.score(intent, blind, None)
}

/// Resolve a keyframe absolute path from a manifest entry.
pub fn resolve_keyframe(manifest_dir: &Path, screenshot_path: &str) -> Option<PathBuf> {
    let candidate = manifest_dir.join(screenshot_path);
    if candidate.exists() {
        Some(candidate)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_intent_and_blind_scores_green() {
        let r = score(
            "Show the plan command help options available",
            "Terminal shows plan command help options available",
        );
        assert!(r.overlap >= 0.6);
        assert_eq!(r.status, Agreement::Green);
        assert_eq!(r.backend, "jaccard");
    }

    #[test]
    fn divergent_intent_and_blind_scores_red() {
        let r = score(
            "Show the plan command help text with all available options",
            "A photograph of a cat sitting on a windowsill bathed in sunlight.",
        );
        assert!(r.overlap < 0.3);
        assert_eq!(r.status, Agreement::Red);
    }

    #[test]
    fn test_jaccard_still_works_as_fallback() {
        let backend = AgreementBackend::Jaccard;
        let scorer = backend.build(false);
        let r = scorer.score("hello world", "hello world", None);
        assert_eq!(r.status, Agreement::Green);
        assert_eq!(r.backend, "jaccard");
    }

    #[test]
    fn backend_parse_flag() {
        assert!(matches!(AgreementBackend::parse_flag("jaccard"), Ok(AgreementBackend::Jaccard)));
        assert!(matches!(AgreementBackend::parse_flag("auto"), Ok(AgreementBackend::Auto)));
        assert!(AgreementBackend::parse_flag("nope").is_err());
    }

    #[test]
    fn test_siglip_falls_back_when_no_image() {
        let scorer = SigLipScorer { model: "google/siglip-so400m-patch14-384".into() };
        let r = scorer.score("user clicks plan", "the plan action triggers", None);
        assert_eq!(r.backend, "jaccard-fallback:siglip-no-image");
    }

    #[test]
    fn test_auto_backend_picks_concrete() {
        let concrete = AgreementBackend::Auto.resolve_concrete(false);
        assert!(!matches!(concrete, AgreementBackend::Auto));
    }
}
