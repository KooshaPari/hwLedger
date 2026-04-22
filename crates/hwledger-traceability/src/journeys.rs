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

/// Blind-eval mode for an individual journey step.
///
/// `Skip` marks a step whose screenshot is an honest stub (real capture blocked
/// by macOS TCC) — the VLM/OCR judge MUST NOT score it. Default is `Honest`,
/// meaning the frame was really captured and is fair game for blind evaluation.
///
/// Traces to: FR-TRACE-003, FR-UX-VERIFY-002
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BlindEvalMode {
    #[default]
    Honest,
    Skip,
}

/// Intent ↔ blind agreement status baked into each verified manifest step
/// by `phenotype-journey verify`. Mirrors the Rust `Agreement` enum in
/// `phenotype-journey-core::agreement`.
///
/// Traces to: FR-UX-VERIFY-003
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AgreementStatus {
    #[default]
    Green,
    Yellow,
    Red,
}

/// Per-step agreement report — the overlap score + diff sets produced by
/// the Rust scorer at verify time. We decode only the fields the gate
/// cares about; the viewer reads the richer shape directly from JSON.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct StepAgreement {
    #[serde(default)]
    pub status: AgreementStatus,
    #[serde(default)]
    pub overlap: f64,
    #[serde(default)]
    pub missing_in_blind: Vec<String>,
    #[serde(default)]
    pub extras_in_blind: Vec<String>,
}

/// Per-step manifest fragment used by the journey gate. We only decode the
/// subset we need (blind-eval mode + slug for diagnostics); the real manifest
/// carries many more fields (annotations, descriptions, bboxes, …).
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ManifestStep {
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub blind_eval: BlindEvalMode,
    /// Intent ↔ blind agreement score. Missing => agreement was not
    /// computed (legacy manifest) and must not drive the gate.
    #[serde(default)]
    pub agreement: Option<StepAgreement>,
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
    /// Per-step blind-eval metadata. Missing => all steps default to `Honest`.
    #[serde(default)]
    pub steps: Vec<ManifestStep>,
    /// Inferred from the directory the manifest lives under
    /// (set post-deserialization by the scanner).
    #[serde(skip)]
    pub kind: Option<JourneyKind>,
    #[serde(skip)]
    pub manifest_path: PathBuf,
}

impl JourneyManifest {
    /// True when the manifest has at least one step marked `blind_eval: skip`.
    pub fn has_skipped_step(&self) -> bool {
        self.steps.iter().any(|s| s.blind_eval == BlindEvalMode::Skip)
    }

    /// True when any step carries an `agreement.status = red` report.
    ///
    /// Traces to: FR-UX-VERIFY-003
    pub fn has_red_agreement(&self) -> bool {
        self.steps.iter().any(|s| {
            s.agreement
                .as_ref()
                .map(|a| a.status == AgreementStatus::Red)
                .unwrap_or(false)
        })
    }
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
    /// True when the backing journey has at least one step whose
    /// intent↔blind agreement status is Red. Advisory by default;
    /// promoted to a hard failure by `--no-agreement-red`.
    ///
    /// Traces to: FR-UX-VERIFY-003
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub needs_agreement_review: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JourneyStatus {
    Ok,
    Missing,
    LowScore,
    NotPassed,
    /// GUI journey with ≥1 step flagged `blind_eval: skip` — real capture is
    /// pending (e.g. macOS TCC denied). Treated as a non-fatal warning by
    /// default under `--strict-journeys`, but upgradable to hard failure via
    /// `--no-skip-allowed`.
    NeedsCapture,
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
    /// True when any row or orphan requires a CI failure under the default
    /// `--strict-journeys` policy. `NeedsCapture` is a warning-class status
    /// (advisory) and does NOT count here — callers that want to promote it
    /// to a hard failure should check [`JourneyReport::has_needs_capture`]
    /// separately (see `--no-skip-allowed` in the CLI).
    pub fn has_failures(&self) -> bool {
        self.rows
            .iter()
            .any(|r| !matches!(r.status, JourneyStatus::Ok | JourneyStatus::NeedsCapture))
            || !self.orphan_journeys.is_empty()
    }

    /// True when any row is `NeedsCapture` — used by `--no-skip-allowed` to
    /// escalate the warning into a blocking failure.
    pub fn has_needs_capture(&self) -> bool {
        self.rows.iter().any(|r| r.status == JourneyStatus::NeedsCapture)
    }

    /// True when any row is flagged `needs_agreement_review` — i.e. the
    /// underlying journey has at least one step whose intent↔blind
    /// agreement is Red. Used by `--no-agreement-red` to escalate the
    /// advisory into a hard failure.
    ///
    /// Traces to: FR-UX-VERIFY-003
    pub fn has_agreement_red(&self) -> bool {
        self.rows.iter().any(|r| r.needs_agreement_review)
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
                    needs_agreement_review: false,
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
            let text_passed = best.passed
                && best.verification.as_ref().map(|v| v.all_intents_passed).unwrap_or(true);

            // Vision-judge authority (per FR-UX-VERIFY-002): the Sonnet judge
            // score on the authoritative signal for whether the journey
            // visually matches its intent. OCR-based text assertions
            // (`passed` / `all_intents_passed`) are fragile against glyph
            // misreads and are treated as an advisory signal only.
            //
            // Policy:
            //   score >= MIN_JOURNEY_SCORE AND text_passed       -> OK
            //   score >= MIN_JOURNEY_SCORE AND !text_passed      -> OK + warning
            //   score <  MIN_JOURNEY_SCORE                       -> LowScore
            //   score == 0 (no/pending capture)                  -> NotPassed
            // Blind-eval skip gate: a GUI journey with any step marked
            // `blind_eval: skip` has been explicitly admitted as
            // partially-captured. Regardless of vision-judge score (which
            // would be measuring honest-stub frames), surface this as
            // `NeedsCapture` so reviewers know real capture is still owed.
            let blind_skip = *kind == JourneyKind::Gui && best.has_skipped_step();

            let status = if blind_skip {
                JourneyStatus::NeedsCapture
            } else if score <= 0.0 {
                JourneyStatus::NotPassed
            } else if score < MIN_JOURNEY_SCORE {
                JourneyStatus::LowScore
            } else {
                if !text_passed {
                    report.warnings.push(format!(
                        "{} [{}]: journey {} vision-verified (score={:.2}) but text assertions failed \
                         (OCR advisory, non-blocking)",
                        fr.id,
                        kind.as_str(),
                        best.id,
                        score
                    ));
                }
                JourneyStatus::Ok
            };
            let passed = text_passed;

            let needs_agreement_review = best.has_red_agreement();
            if needs_agreement_review {
                report.warnings.push(format!(
                    "{} [{}]: journey {} has ≥1 step with intent↔blind agreement=red \
                     — surface as `needs_agreement_review` (advisory; \
                     `--no-agreement-red` promotes to FAIL)",
                    fr.id,
                    kind.as_str(),
                    best.id,
                ));
            }

            report.rows.push(JourneyCoverageRow {
                fr: fr.id.clone(),
                kind: kind.as_str().into(),
                journey_id: Some(best.id.clone()),
                score: Some(score),
                passed: Some(passed),
                status,
                needs_agreement_review,
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
        md.push_str("| FR | kind | journey id | score | passed | status | agreement |\n");
        md.push_str("|---|---|---|---|---|---|---|\n");
        for r in &report.rows {
            let jid = r.journey_id.clone().unwrap_or_else(|| "—".into());
            let score = r.score.map(|s| format!("{:.2}", s)).unwrap_or_else(|| "—".into());
            let passed = r.passed.map(|p| if p { "yes" } else { "no" }).unwrap_or("—").to_string();
            let status = match r.status {
                JourneyStatus::Ok => "OK",
                JourneyStatus::Missing => "MISSING",
                JourneyStatus::LowScore => "LOW_SCORE",
                JourneyStatus::NotPassed => "NOT_PASSED",
                JourneyStatus::NeedsCapture => "NEEDS_CAPTURE",
            };
            let agreement = if r.needs_agreement_review {
                "needs_agreement_review"
            } else {
                "—"
            };
            md.push_str(&format!(
                "| **{}** | {} | {} | {} | {} | {} | {} |\n",
                r.fr, r.kind, jid, score, passed, status, agreement
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
            steps: Vec::new(),
            kind: Some(kind),
            manifest_path: PathBuf::from(format!("fake/{}/manifest.verified.json", id)),
        }
    }

    fn manifest_with_skip(
        id: &str,
        kind: JourneyKind,
        traces: Vec<&str>,
        score: f64,
        passed: bool,
        skip_indices: &[usize],
    ) -> JourneyManifest {
        let mut m = manifest(id, kind, traces, score, passed);
        // Fabricate 6 steps, marking the listed indices as skip.
        m.steps = (0..6)
            .map(|i| ManifestStep {
                slug: format!("step-{}", i),
                blind_eval: if skip_indices.contains(&i) {
                    BlindEvalMode::Skip
                } else {
                    BlindEvalMode::Honest
                },
                agreement: None,
            })
            .collect();
        m
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

    fn manifest_with_agreement(
        id: &str,
        kind: JourneyKind,
        traces: Vec<&str>,
        score: f64,
        passed: bool,
        statuses: &[AgreementStatus],
    ) -> JourneyManifest {
        let mut m = manifest(id, kind, traces, score, passed);
        m.steps = statuses
            .iter()
            .enumerate()
            .map(|(i, s)| ManifestStep {
                slug: format!("step-{}", i),
                blind_eval: BlindEvalMode::Honest,
                agreement: Some(StepAgreement {
                    status: *s,
                    overlap: match s {
                        AgreementStatus::Green => 0.8,
                        AgreementStatus::Yellow => 0.45,
                        AgreementStatus::Red => 0.1,
                    },
                    missing_in_blind: vec!["plan".into()],
                    extras_in_blind: vec!["cat".into()],
                }),
            })
            .collect();
        m
    }

    /// A journey with any step whose `agreement.status == red` must surface
    /// `needs_agreement_review` on the row (advisory by default) and
    /// `has_agreement_red()` must return true so `--no-agreement-red`
    /// can escalate it.
    ///
    /// Traces to: FR-UX-VERIFY-003
    #[test]
    fn test_red_agreement_surfaces_needs_agreement_review() {
        let frs = vec![fr("FR-PLAN-003", vec![JourneyKind::Cli])];
        let scan = JourneyScan {
            manifests: vec![manifest_with_agreement(
                "cli-plan",
                JourneyKind::Cli,
                vec!["FR-PLAN-003"],
                0.92,
                true,
                &[AgreementStatus::Green, AgreementStatus::Red, AgreementStatus::Yellow],
            )],
            warnings: vec![],
        };
        let rep = evaluate(&frs, &scan);
        assert_eq!(rep.rows.len(), 1);
        // Status is still OK (score >= threshold, passed, no blind-eval skip).
        assert_eq!(rep.rows[0].status, JourneyStatus::Ok);
        // But the agreement flag must light up.
        assert!(rep.rows[0].needs_agreement_review);
        assert!(rep.has_agreement_red());
        // Advisory — does NOT count toward `has_failures()`.
        assert!(!rep.has_failures());
        // An advisory warning must surface with the journey id.
        assert!(
            rep.warnings
                .iter()
                .any(|w| w.contains("cli-plan") && w.contains("agreement=red"))
        );
    }

    /// Vision-judge authority: score >= threshold flips to OK even with
    /// OCR text-assertion violations. Emits an advisory warning.
    ///
    /// Traces to: FR-TRACE-003, FR-UX-VERIFY-002
    #[test]
    fn test_vision_score_overrides_text_assertion_failures() {
        let frs = vec![fr("FR-PLAN-003", vec![JourneyKind::Cli])];
        // passed=false (OCR text assertions failed) but Vision score = 0.92
        let scan = JourneyScan {
            manifests: vec![manifest(
                "cli-plan",
                JourneyKind::Cli,
                vec!["FR-PLAN-003"],
                0.92,
                false,
            )],
            warnings: vec![],
        };
        let rep = evaluate(&frs, &scan);
        assert_eq!(rep.rows[0].status, JourneyStatus::Ok);
        assert!(!rep.has_failures());
        // Advisory warning must surface.
        assert!(rep.warnings.iter().any(|w| w.contains("OCR advisory")));
    }

    /// GUI journey with ≥1 step marked `blind_eval: skip` surfaces as
    /// `NeedsCapture`, NOT `Ok`, regardless of vision-judge score. Under the
    /// default policy this is advisory (no CI failure) but
    /// `has_needs_capture()` returns true so `--no-skip-allowed` can escalate.
    ///
    /// Traces to: FR-TRACE-003, FR-UX-VERIFY-002
    #[test]
    fn test_gui_journey_with_blind_eval_skip_surfaces_needs_capture() {
        let frs = vec![fr("FR-UI-001", vec![JourneyKind::Gui])];
        let scan = JourneyScan {
            manifests: vec![manifest_with_skip(
                "planner-gui-launch",
                JourneyKind::Gui,
                vec!["FR-UI-001"],
                0.92, // even a "good" score must NOT mask a skipped step
                true,
                &[0, 4],
            )],
            warnings: vec![],
        };
        let rep = evaluate(&frs, &scan);
        assert_eq!(rep.rows.len(), 1);
        assert_eq!(rep.rows[0].status, JourneyStatus::NeedsCapture);
        // Advisory-class by default — does NOT trip has_failures().
        assert!(!rep.has_failures());
        // But the escalation hook must detect it.
        assert!(rep.has_needs_capture());
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
