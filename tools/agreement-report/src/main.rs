//! Build-time agreement report.
//!
//! Walks every `manifest.verified.json` under the conventional journey roots
//! (`docs-site/public/{cli,gui,streamlit}-journeys/**`, plus `apps/cli-journeys`
//! + `apps/streamlit/journeys` for pre-sync development) and emits a Markdown
//! table keyed on (journey, frame) listing the overlap percent + color for
//! the intent↔blind agreement.
//!
//! Traces to: FR-UX-VERIFY-003
//!
//! Emits to `docs-site/quality/agreement-report.md` by default; override with
//! `--out <path>`. Exits 0 on any outcome (non-blocking); use
//! `hwledger-traceability --no-agreement-red` for the gate.

use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(name = "hwledger-agreement-report", version, about)]
struct Args {
    /// Repository root (defaults to cwd).
    #[arg(long, default_value = ".")]
    repo: PathBuf,
    /// Output markdown path.
    #[arg(long, default_value = "docs-site/quality/agreement-report.md")]
    out: PathBuf,
}

#[derive(Debug, Deserialize)]
struct Agreement {
    #[serde(default)]
    status: String,
    #[serde(default)]
    overlap: f64,
    #[serde(default)]
    missing_in_blind: Vec<String>,
    #[serde(default)]
    extras_in_blind: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Step {
    #[serde(default)]
    index: u32,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    intent: String,
    #[serde(default)]
    blind_description: Option<String>,
    #[serde(default)]
    agreement: Option<Agreement>,
}

mod score {
    //! Mirror of phenotype-journey-core::agreement — kept inline so the
    //! build-time report can fall back to on-the-fly scoring for legacy
    //! manifests (those written before the `agreement` field was baked).
    use rust_stemmers::{Algorithm, Stemmer};
    use std::collections::BTreeSet;

    pub(crate) struct Report {
        pub status: String,
        pub overlap: f64,
        pub missing_in_blind: Vec<String>,
        pub extras_in_blind: Vec<String>,
    }

    const STOPWORDS: &[&str] = &[
        "a","an","the","and","or","but","if","then","else","of","for","to",
        "in","on","at","by","with","from","as","is","are","was","were","be",
        "been","being","it","its","this","that","these","those","do","does",
        "did","have","has","had","will","would","should","could","can","may",
        "might","must","so","than","when","while","where","who","what","which",
        "some","any","all","no","not","out","up","down","into","over","under",
        "again","about","after","before","just","also","only","very","too",
        "there","here","s","t",
    ];

    fn stop(w: &str) -> bool { w.len() <= 1 || STOPWORDS.contains(&w) }

    fn tokenise(text: &str) -> Vec<String> {
        let stemmer = Stemmer::create(Algorithm::English);
        let mut out: BTreeSet<String> = BTreeSet::new();
        let mut cur = String::new();
        for ch in text.chars() {
            if ch.is_alphanumeric() {
                for c in ch.to_lowercase() { cur.push(c); }
            } else {
                push(&stemmer, &cur, &mut out);
                cur.clear();
            }
        }
        push(&stemmer, &cur, &mut out);
        out.into_iter().collect()
    }

    fn push(stemmer: &Stemmer, w: &str, out: &mut BTreeSet<String>) {
        if w.is_empty() || stop(w) { return; }
        let s = stemmer.stem(w).to_string();
        if s.is_empty() || stop(&s) { return; }
        out.insert(s);
    }

    pub(crate) fn score(intent: &str, blind: &str) -> Report {
        let it = tokenise(intent);
        let bt = tokenise(blind);
        let i: BTreeSet<&String> = it.iter().collect();
        let b: BTreeSet<&String> = bt.iter().collect();
        let overlap = if i.is_empty() && b.is_empty() {
            1.0
        } else if i.is_empty() || b.is_empty() {
            0.0
        } else {
            i.intersection(&b).count() as f64 / i.union(&b).count() as f64
        };
        let status = if overlap >= 0.6 { "green" } else if overlap >= 0.3 { "yellow" } else { "red" };
        Report {
            status: status.to_string(),
            overlap,
            missing_in_blind: i.difference(&b).map(|x| (*x).clone()).collect(),
            extras_in_blind: b.difference(&i).map(|x| (*x).clone()).collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Manifest {
    #[serde(default)]
    id: String,
    #[serde(default)]
    steps: Vec<Step>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let roots = [
        "docs-site/public/cli-journeys/manifests",
        "docs-site/public/gui-journeys",
        "docs-site/public/streamlit-journeys/manifests",
        "apps/cli-journeys/manifests",
        "apps/streamlit/journeys/manifests",
    ];

    let mut manifests: Vec<(PathBuf, Manifest)> = Vec::new();
    for rel in roots {
        let root = args.repo.join(rel);
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(&root).max_depth(4) {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            if entry.file_name() != "manifest.verified.json" {
                continue;
            }
            let path = entry.path();
            let raw = match std::fs::read_to_string(path) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let m: Manifest = match serde_json::from_str(&raw) {
                Ok(m) => m,
                Err(_) => continue,
            };
            manifests.push((path.to_path_buf(), m));
        }
    }

    manifests.sort_by(|(_, a), (_, b)| a.id.cmp(&b.id));
    // Dedupe: the same verified manifest often exists under both
    // `apps/*/manifests/<id>/` and its synced twin under `docs-site/public/`.
    // Keep the first occurrence per id.
    let mut seen = std::collections::BTreeSet::new();
    manifests.retain(|(_, m)| seen.insert(m.id.clone()));

    let mut green = 0usize;
    let mut yellow = 0usize;
    let mut red = 0usize;
    let mut missing = 0usize;
    let mut total_steps = 0usize;

    let mut md = String::new();
    md.push_str("# Journey agreement report\n\n");
    md.push_str(
        "Per-step intent↔blind agreement across every verified journey \
         manifest. Green ≥60% overlap, Yellow 30–60%, Red <30%. Generated \
         by `hwledger-agreement-report` (traces to FR-UX-VERIFY-003).\n\n",
    );
    md.push_str("| Journey | Frame | Slug | Status | Overlap | Missing | Extras |\n");
    md.push_str("|---|---|---|---|---|---|---|\n");

    for (path, m) in &manifests {
        let _ = path;
        for step in &m.steps {
            total_steps += 1;
            // Prefer the baked agreement payload; fall back to live scoring
            // for legacy manifests that were verified before the agreement
            // module existed. This keeps the report usable during rollout.
            let (status_str, overlap, missing_v, extras_v, is_live) = match &step.agreement {
                Some(a) => (
                    a.status.clone(),
                    a.overlap,
                    a.missing_in_blind.clone(),
                    a.extras_in_blind.clone(),
                    false,
                ),
                None => {
                    let blind = step.blind_description.clone().unwrap_or_default();
                    if step.intent.is_empty() && blind.is_empty() {
                        missing += 1;
                        (
                            "unknown".to_string(),
                            0.0,
                            vec![],
                            vec![],
                            true,
                        )
                    } else {
                        let r = score::score(&step.intent, &blind);
                        (r.status, r.overlap, r.missing_in_blind, r.extras_in_blind, true)
                    }
                }
            };
            match status_str.as_str() {
                "green" => green += 1,
                "yellow" => yellow += 1,
                "red" => red += 1,
                _ => {}
            }
            let glyph = match status_str.as_str() {
                "green" => "🟢 green",
                "yellow" => "🟡 yellow",
                "red" => "🔴 red",
                _ => "— unknown",
            };
            let status_cell = if is_live && status_str != "unknown" {
                format!("{} *", glyph)
            } else {
                glyph.to_string()
            };
            let overlap_cell = if status_str == "unknown" {
                "—".to_string()
            } else {
                format!("{:>3.0}%", overlap * 100.0)
            };
            let missing_cell = truncate_tokens(&missing_v);
            let extras_cell = truncate_tokens(&extras_v);
            let slug = if step.slug.is_empty() {
                format!("frame-{}", step.index)
            } else {
                step.slug.clone()
            };
            md.push_str(&format!(
                "| `{}` | {} | `{}` | {} | {} | {} | {} |\n",
                m.id,
                step.index + 1,
                slug,
                status_cell,
                overlap_cell,
                missing_cell,
                extras_cell,
            ));
        }
    }

    md.push_str("\n_Rows marked `*` were scored live by `hwledger-agreement-report` because the manifest pre-dates the `agreement` field._\n");
    md.push_str("\n## Distribution\n\n");
    md.push_str(&format!(
        "- Manifests scanned: **{}**\n\
         - Total steps: **{}**\n\
         - 🟢 Green: **{}**\n\
         - 🟡 Yellow: **{}**\n\
         - 🔴 Red: **{}**\n\
         - Missing agreement report: **{}**\n",
        manifests.len(),
        total_steps,
        green,
        yellow,
        red,
        missing,
    ));

    let out = if args.out.is_absolute() {
        args.out.clone()
    } else {
        args.repo.join(&args.out)
    };
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create dir {}", parent.display()))?;
    }
    std::fs::write(&out, md).with_context(|| format!("write {}", out.display()))?;
    eprintln!("wrote {}", out.display());
    eprintln!(
        "distribution: green={} yellow={} red={} missing={} (manifests={}, steps={})",
        green, yellow, red, missing, manifests.len(), total_steps
    );

    Ok(())
}

fn truncate_tokens(tokens: &[String]) -> String {
    if tokens.is_empty() {
        return "—".to_string();
    }
    let max = 6;
    let shown: Vec<String> = tokens.iter().take(max).cloned().collect();
    let extra = tokens.len().saturating_sub(max);
    let body = shown.join(", ");
    if extra > 0 {
        format!("{body}, …+{extra}")
    } else {
        body
    }
}

