//! Journey manifest scanner — walks verified manifests under
//! `docs-site/public/{cli,gui,streamlit}-journeys/**/manifest.verified.json`
//! and parses their `id`, `traces_to`, and `verification` shape.
//!
//! Missing directories (e.g. streamlit manifests not yet generated) are
//! treated as a warning, not a hard failure.
//!
//! Traces to: FR-TRACE-002, FR-TRACE-003

use crate::prd::{FrSpec, JourneyKind};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Default minimum score a verified journey must meet to pass the gate.
pub const MIN_JOURNEY_SCORE: f64 = 0.7;

/// Errors raised while scanning journey manifests.
#[derive(Debug, Error)]
pub enum JourneyError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error in {path}: {source}")]
    Json { path: String, source: serde_json::Error },
}

/// Verification block (subset of fields relevant to the gate).
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ManifestVerification {
    #[serde(default)]
    pub overall_score: f64,
    #[serde(default)]
    pub all_intents_passed: bool,
}

/// Parsed shape of a verified journey manifest.
///
/// We allow additional fields (the real manifests contain a lot more).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JourneyManifest {
    pub id: String,
    #[serde(default)]
    pub passed: bool,
    #[serde(default)]
    pub verification: Option<ManifestVerification>,
    /// Backing FR IDs this journey exercises.
    ///
    /// Extended field added as part of the FR-TRACE-002 rollout — existing
    /// manifests are backfilled.
    #[serde(default)]
    pub traces_to: Vec<String>,
    /// Inferred from the directory the manifest lives under
    /// (set post-deserialization by the scanner).
    #[serde(skip)]
    pub kind: Option<JourneyKind>,
    #[serde(skip)]
    pub manifest_path: PathBuf,
}

/// Result of scanning all journey roots.
#[derive(Debug, Default)]
pub struct JourneyScan {
    pub manifests: Vec<JourneyManifest>,
    /// Warnings for missing roots (e.g. streamlit dir not yet created).
    pub warnings: Vec<String>,
}

/// Scan verified manifests under the conventional journey roots.
///
/// Traces to: FR-TRACE-002
pub fn scan_verified(repo: &Path) -> Result<JourneyScan, JourneyError> {
    let roots = [
        ("docs-site/public/cli-journeys/manifests", JourneyKind::Cli),
        ("docs-site/public/gui-journeys", JourneyKind::Gui),
        ("docs-site/public/streamlit-journeys/manifests", JourneyKind::Web),
    ];

    let mut out = JourneyScan::default();

    for (rel, kind) in roots {
        let root = repo.join(rel);
        if !root.exists() {
            out.warnings.push(format!("journey root missing: {} (skipping)", rel));
            continue;
        }
        collect_dir(&root, kind, &mut out)?;
    }

    Ok(out)
}

fn collect_dir(dir: &Path, kind: JourneyKind, out: &mut JourneyScan) -> Result<(), JourneyError> {
    // Each journey lives in a subdirectory; verified manifest is manifest.verified.json.
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            let mani = p.join("manifest.verified.json");
            if mani.exists() {
                match load_manifest(&mani, kind) {
                    Ok(m) => out.manifests.push(m),
                    Err(e) => {
                        out.warnings.push(format!("failed to parse {}: {}", mani.display(), e))
                    }
                }
            }
        }
    }
    Ok(())
}

fn load_manifest(path: &Path, kind: JourneyKind) -> Result<JourneyManifest, JourneyError> {
    let raw = std::fs::read_to_string(path)?;
    let mut m: JourneyManifest = serde_json::from_str(&raw)
        .map_err(|e| JourneyError::Json { path: path.display().to_string(), source: e })?;
    m.kind = Some(kind);
    m.manifest_path = path.to_path_buf();
    Ok(m)
}

/// Coverage outcome for a single (FR, kind) pairing.
#[derive(Debug, Clone, Serialize)]
pub struct JourneyCoverageRow {
    pub fr: String,
    pub kind: String,
    pub journey_id: Option<String>,
    pub score: Option<f64>,
    pub passed: Option<bool>,
    /// Human-readable reason when the row fails the gate.
    pub status: JourneyStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JourneyStatus {
    Ok,
    Missing,
    LowScore,
    NotPassed,
}

/// Report produced by [`evaluate`].
#[derive(Debug, Default, Serialize)]
pub struct JourneyReport {
    pub rows: Vec<JourneyCoverageRow>,
    pub orphan_journeys: Vec<OrphanJourney>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrphanJourney {
    pub journey_id: String,
    pub manifest: String,
    pub unknown_frs: Vec<String>,
}

impl JourneyReport {
    /// True when any row or orphan requires a CI failure.
    pub fn has_failures(&self) -> bool {
        self.rows.iter().any(|r| r.status != JourneyStatus::Ok) || !self.orphan_journeys.is_empty()
    }
}

/// Evaluate whether every FR tagged with a `journey_kind` has a verified
/// manifest tracing back to it, with an acceptable score.
///
/// Traces to: FR-TRACE-003
pub fn evaluate(frs: &[FrSpec], scan: &JourneyScan) -> JourneyReport {
    use std::collections::HashSet;

    let fr_ids: HashSet<&str> = frs.iter().map(|f| f.id.as_str()).collect();
    let mut report = JourneyReport { warnings: scan.warnings.clone(), ..Default::default() };

    // Orphan detection — any manifest tracing to an FR that does not exist.
    for m in &scan.manifests {
        let unknown: Vec<String> =
            m.traces_to.iter().filter(|fr| !fr_ids.contains(fr.as_str())).cloned().collect();
        if !unknown.is_empty() {
            report.orphan_journeys.push(OrphanJourney {
                journey_id: m.id.clone(),
                manifest: m.manifest_path.display().to_string(),
                unknown_frs: unknown,
            });
        }
    }

    // FR coverage — per (FR, kind) tagged, find a matching verified manifest.
    for fr in frs {
        for kind in &fr.journey_kinds {
            // `[journey_kind: none]` is an explicit-no-journey declaration
            // for server-internal or spec-only primitives (NFRs, parser
            // internals). Skip it — it should not drive a manifest lookup
            // and it should NOT fail the gate.
            if matches!(kind, JourneyKind::None) {
                continue;
            }
            // Find manifests for this kind that trace to this FR.
            let mut matches: Vec<&JourneyManifest> = scan
                .manifests
                .iter()
                .filter(|m| m.kind == Some(*kind) && m.traces_to.iter().any(|t| t == &fr.id))
                .collect();

            if matches.is_empty() {
                report.rows.push(JourneyCoverageRow {
                    fr: fr.id.clone(),
                    kind: kind.as_str().into(),
                    journey_id: None,
                    score: None,
                    passed: None,
                    status: JourneyStatus::Missing,
                });
                continue;
            }

            // Highest-scoring match wins for the reported row.
            matches.sort_by(|a, b| {
                let sa = a.verification.as_ref().map(|v| v.overall_score).unwrap_or(0.0);
                let sb = b.verification.as_ref().map(|v| v.overall_score).unwrap_or(0.0);
                sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
            });
            let best = matches[0];
            let score = best.verification.as_ref().map(|v| v.overall_score).unwrap_or(0.0);
            let passed = best.passed
                && best.verification.as_ref().map(|v| v.all_intents_passed).unwrap_or(true);

            let status = if !passed {
                JourneyStatus::NotPassed
            } else if score < MIN_JOURNEY_SCORE {
                JourneyStatus::LowScore
            } else {
                JourneyStatus::Ok
            };

            report.rows.push(JourneyCoverageRow {
                fr: fr.id.clone(),
                kind: kind.as_str().into(),
                journey_id: Some(best.id.clone()),
                score: Some(score),
                passed: Some(passed),
                status,
            });
        }
    }

    report
}

/// Render the journey coverage section as markdown.
///
/// Traces to: FR-TRACE-004
pub fn render_markdown(report: &JourneyReport) -> String {
    let mut md = String::new();
    md.push_str("## Journey coverage\n\n");

    if report.rows.is_empty() {
        md.push_str("_No FRs tagged with `journey_kind` yet._\n\n");
    } else {
        md.push_str("| FR | kind | journey id | score | passed | status |\n");
        md.push_str("|---|---|---|---|---|---|\n");
        for r in &report.rows {
            let jid = r.journey_id.clone().unwrap_or_else(|| "—".into());
            let score = r.score.map(|s| format!("{:.2}", s)).unwrap_or_else(|| "—".into());
            let passed = r.passed.map(|p| if p { "yes" } else { "no" }).unwrap_or("—").to_string();
            let status = match r.status {
                JourneyStatus::Ok => "OK",
                JourneyStatus::Missing => "MISSING",
                JourneyStatus::LowScore => "LOW_SCORE",
                JourneyStatus::NotPassed => "NOT_PASSED",
            };
            md.push_str(&format!(
                "| **{}** | {} | {} | {} | {} | {} |\n",
                r.fr, r.kind, jid, score, passed, status
            ));
        }
        md.push('\n');
    }

    if !report.orphan_journeys.is_empty() {
        md.push_str("### Orphan journeys (cite unknown FRs)\n\n");
        for o in &report.orphan_journeys {
            md.push_str(&format!(
                "- **{}** → unknown: {} ({})\n",
                o.journey_id,
                o.unknown_frs.join(", "),
                o.manifest
            ));
        }
        md.push('\n');
    }

    if !report.warnings.is_empty() {
        md.push_str("### Warnings\n\n");
        for w in &report.warnings {
            md.push_str(&format!("- {}\n", w));
        }
        md.push('\n');
    }

    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prd::{FrKind, FrSpec};

    fn fr(id: &str, kinds: Vec<JourneyKind>) -> FrSpec {
        FrSpec {
            id: id.into(),
            kind: FrKind::Fr,
            description: String::new(),
            section: String::new(),
            journey_kinds: kinds,
        }
    }

    fn manifest(
        id: &str,
        kind: JourneyKind,
        traces: Vec<&str>,
        score: f64,
        passed: bool,
    ) -> JourneyManifest {
        JourneyManifest {
            id: id.into(),
            passed,
            verification: Some(ManifestVerification {
                overall_score: score,
                all_intents_passed: passed,
            }),
            traces_to: traces.into_iter().map(String::from).collect(),
            kind: Some(kind),
            manifest_path: PathBuf::from(format!("fake/{}/manifest.verified.json", id)),
        }
    }

    /// Traces to: FR-TRACE-001
    #[test]
    fn test_journey_kind_parse_multi() {
        let parsed: Vec<_> = "cli, web , GUI".split(',').filter_map(JourneyKind::parse).collect();
        assert_eq!(parsed, vec![JourneyKind::Cli, JourneyKind::Web, JourneyKind::Gui]);
    }

    /// Traces to: FR-TRACE-001
    #[test]
    fn test_prd_line_with_journey_kind_tag() {
        let content = r#"### Section
- **FR-PLAN-003** [journey_kind: cli,web]: Memory planner
- **FR-GUI-001** [journey_kind: gui]: macOS planner UI
- **FR-PLAN-002**: No tag
"#;
        let frs = crate::prd::PrdParser::parse_content(content).unwrap();
        assert_eq!(frs.len(), 3);
        assert_eq!(frs[0].journey_kinds, vec![JourneyKind::Cli, JourneyKind::Web]);
        assert_eq!(frs[1].journey_kinds, vec![JourneyKind::Gui]);
        assert!(frs[2].journey_kinds.is_empty());
        // Description must not leak the tag.
        assert_eq!(frs[0].description, "Memory planner");
    }

    /// Traces to: FR-TRACE-002
    #[test]
    fn test_scan_verified_handles_missing_root() {
        // Scanning a non-existent repo path should succeed with warnings, not panic.
        let tmp = std::env::temp_dir().join("hwledger-journey-scan-test");
        let _ = std::fs::create_dir_all(&tmp);
        let scan = scan_verified(&tmp).expect("scan must not panic on missing roots");
        assert!(scan.manifests.is_empty());
        assert!(!scan.warnings.is_empty());
    }

    /// Traces to: FR-TRACE-003
    #[test]
    fn test_missing_journey_for_tagged_fr() {
        let frs = vec![fr("FR-PLAN-003", vec![JourneyKind::Cli])];
        let scan = JourneyScan { manifests: vec![], warnings: vec![] };
        let rep = evaluate(&frs, &scan);
        assert_eq!(rep.rows.len(), 1);
        assert_eq!(rep.rows[0].status, JourneyStatus::Missing);
        assert!(rep.has_failures());
    }

    /// Traces to: FR-TRACE-003
    #[test]
    fn test_orphan_journey_detection() {
        let frs = vec![fr("FR-PLAN-003", vec![JourneyKind::Cli])];
        let scan = JourneyScan {
            manifests: vec![manifest(
                "cli-bogus",
                JourneyKind::Cli,
                vec!["FR-DOES-NOT-EXIST-999"],
                0.9,
                true,
            )],
            warnings: vec![],
        };
        let rep = evaluate(&frs, &scan);
        assert_eq!(rep.orphan_journeys.len(), 1);
        assert!(rep.has_failures());
    }

    /// Traces to: FR-TRACE-003
    #[test]
    fn test_low_score_journey_fails() {
        let frs = vec![fr("FR-PLAN-003", vec![JourneyKind::Cli])];
        let scan = JourneyScan {
            manifests: vec![manifest("cli-plan", JourneyKind::Cli, vec!["FR-PLAN-003"], 0.5, true)],
            warnings: vec![],
        };
        let rep = evaluate(&frs, &scan);
        assert_eq!(rep.rows[0].status, JourneyStatus::LowScore);
        assert!(rep.has_failures());
    }

    /// Traces to: FR-TRACE-003
    #[test]
    fn test_happy_path_journey_ok() {
        let frs = vec![fr("FR-PLAN-003", vec![JourneyKind::Cli])];
        let scan = JourneyScan {
            manifests: vec![manifest(
                "cli-plan",
                JourneyKind::Cli,
                vec!["FR-PLAN-003"],
                0.92,
                true,
            )],
            warnings: vec![],
        };
        let rep = evaluate(&frs, &scan);
        assert_eq!(rep.rows[0].status, JourneyStatus::Ok);
        assert!(!rep.has_failures());
    }

    /// Traces to: FR-TRACE-004
    #[test]
    fn test_render_markdown_includes_header_and_row() {
        let frs = vec![fr("FR-PLAN-003", vec![JourneyKind::Cli])];
        let scan = JourneyScan {
            manifests: vec![manifest(
                "cli-plan",
                JourneyKind::Cli,
                vec!["FR-PLAN-003"],
                0.92,
                true,
            )],
            warnings: vec![],
        };
        let rep = evaluate(&frs, &scan);
        let md = render_markdown(&rep);
        assert!(md.contains("## Journey coverage"));
        assert!(md.contains("FR-PLAN-003"));
        assert!(md.contains("cli-plan"));
    }
}
