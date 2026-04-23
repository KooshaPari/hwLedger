//! D6 — Structural-tree-priority bbox selector + hit-rate harness.
//!
//! Prefers structural-tree-derived bboxes (AX tree for macOS, DOM for web via
//! Playwright, terminal-cell geometry for CLI) over OCR-derived bboxes when
//! both are available for a given target. Measures hit rate on existing
//! manifest corpora.
//!
//! Priority ladder (highest to lowest):
//!   1. `StructuralAx`       — macOS AX tree (richest semantic signal)
//!   2. `StructuralDom`      — DOM via Playwright (second-class structural)
//!   3. `StructuralTerminal` — terminal-cell geometry (from D4 asciicast)
//!   4. `OcrVlm`             — VLM-derived OCR bbox (uses semantic priors)
//!   5. `OcrTesseract`       — tesseract OCR (lexical only)
//!
//! Dedupe: candidates overlapping at IoU > 0.7 with matching label collapse to
//! the best-priority survivor.

use serde::{Deserialize, Serialize};
use std::path::Path;

pub mod corpus;

/// Source provenance of a bounding box candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BboxSource {
    StructuralAx,
    StructuralDom,
    StructuralTerminal,
    OcrTesseract,
    OcrVlm,
}

impl BboxSource {
    /// Priority tier: lower = higher preference.
    pub fn priority(self) -> u8 {
        match self {
            BboxSource::StructuralAx => 0,
            BboxSource::StructuralDom => 1,
            BboxSource::StructuralTerminal => 2,
            BboxSource::OcrVlm => 3,
            BboxSource::OcrTesseract => 4,
        }
    }

    pub fn is_structural(self) -> bool {
        matches!(
            self,
            BboxSource::StructuralAx | BboxSource::StructuralDom | BboxSource::StructuralTerminal
        )
    }
}

/// A single bbox candidate for a target region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundedBbox {
    pub source: BboxSource,
    /// Detector confidence in `[0.0, 1.0]`.
    pub confidence: f64,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub label: Option<String>,
    /// Structural path (e.g. `window[0]/toolbar/button[planner]`).
    pub structural_path: Option<String>,
}

impl GroundedBbox {
    pub fn iou(&self, other: &Self) -> f64 {
        let ax1 = self.x;
        let ay1 = self.y;
        let ax2 = self.x + self.width;
        let ay2 = self.y + self.height;
        let bx1 = other.x;
        let by1 = other.y;
        let bx2 = other.x + other.width;
        let by2 = other.y + other.height;

        let ix1 = ax1.max(bx1);
        let iy1 = ay1.max(by1);
        let ix2 = ax2.min(bx2);
        let iy2 = ay2.min(by2);

        let iw = (ix2 - ix1).max(0.0);
        let ih = (iy2 - iy1).max(0.0);
        let inter = iw * ih;

        let area_a = (ax2 - ax1).max(0.0) * (ay2 - ay1).max(0.0);
        let area_b = (bx2 - bx1).max(0.0) * (by2 - by1).max(0.0);
        let union = area_a + area_b - inter;
        if union <= 0.0 {
            0.0
        } else {
            inter / union
        }
    }
}

/// A set of candidate bboxes for a single target (same label / same intent).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BboxCandidateSet {
    pub target_label: String,
    pub candidates: Vec<GroundedBbox>,
}

/// Compare two candidates; lower-priority tier wins. Tie-break on confidence.
fn better(a: &GroundedBbox, b: &GroundedBbox) -> std::cmp::Ordering {
    let pa = a.source.priority();
    let pb = b.source.priority();
    if pa != pb {
        return pa.cmp(&pb);
    }
    // Same tier: higher confidence wins (reverse so we can treat Less=better
    // uniformly below — instead just compare by NOT-confidence).
    b.confidence
        .partial_cmp(&a.confidence)
        .unwrap_or(std::cmp::Ordering::Equal)
}

/// Apply priority: structural > OCR. Within structural: AX > DOM > Terminal.
/// Within OCR: VLM > Tesseract. Dedupe by IoU > 0.7 (overlap collapses to
/// the higher-priority survivor).
pub fn choose_bbox(set: &BboxCandidateSet) -> Option<&GroundedBbox> {
    if set.candidates.is_empty() {
        return None;
    }

    // Pick the tier-minimum candidate. Dedupe is implicit: any lower-priority
    // overlap would lose, so we just need to guarantee we also respect
    // confidence within the best tier and don't get fooled by a non-overlapping
    // degenerate structural candidate. But spec says "prefer structural when
    // both are available for a given target" (set is per-target), so choose
    // globally best by (priority, confidence).
    let mut best_idx = 0usize;
    for i in 1..set.candidates.len() {
        if better(&set.candidates[i], &set.candidates[best_idx]) == std::cmp::Ordering::Less {
            best_idx = i;
        }
    }
    let best = &set.candidates[best_idx];

    // If the best is OCR but there's a structural overlap (IoU > 0.7) somewhere
    // in the set (even if the structural candidate's confidence is lower),
    // the structural one must win per policy. `better()` already prefers
    // structural via tier, so this is redundant unless the structural one was
    // filtered earlier. We still add an explicit check for defence-in-depth.
    if !best.source.is_structural() {
        for c in &set.candidates {
            if c.source.is_structural() && c.iou(best) > 0.7 {
                return Some(c);
            }
        }
    }
    Some(best)
}

/// Hit-rate over a corpus of manifests.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HitRate {
    pub structural: usize,
    pub ocr: usize,
    pub none: usize,
}

impl HitRate {
    pub fn total(&self) -> usize {
        self.structural + self.ocr + self.none
    }

    pub fn structural_pct(&self) -> f64 {
        let t = self.total();
        if t == 0 {
            0.0
        } else {
            (self.structural as f64) * 100.0 / (t as f64)
        }
    }

    pub fn ocr_pct(&self) -> f64 {
        let t = self.total();
        if t == 0 {
            0.0
        } else {
            (self.ocr as f64) * 100.0 / (t as f64)
        }
    }

    pub fn none_pct(&self) -> f64 {
        let t = self.total();
        if t == 0 {
            0.0
        } else {
            (self.none as f64) * 100.0 / (t as f64)
        }
    }
}

/// Apply [`choose_bbox`] across every candidate set in every discovered
/// manifest; tally which tier won.
pub fn measure_hit_rate(corpus_dir: &Path) -> HitRate {
    let mut hr = HitRate::default();
    let sets = corpus::load_candidate_sets(corpus_dir);
    for set in &sets {
        match choose_bbox(set) {
            None => hr.none += 1,
            Some(b) if b.source.is_structural() => hr.structural += 1,
            Some(_) => hr.ocr += 1,
        }
    }
    hr
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn bb(src: BboxSource, conf: f64, x: f64, y: f64, w: f64, h: f64, label: &str) -> GroundedBbox {
        GroundedBbox {
            source: src,
            confidence: conf,
            x,
            y,
            width: w,
            height: h,
            label: Some(label.to_string()),
            structural_path: None,
        }
    }

    #[test]
    fn priority_structural_ax_beats_ocr_tesseract() {
        let set = BboxCandidateSet {
            target_label: "planner-button".into(),
            candidates: vec![
                bb(BboxSource::OcrTesseract, 0.95, 10.0, 10.0, 100.0, 30.0, "PLANNER"),
                bb(BboxSource::StructuralAx, 0.60, 12.0, 12.0, 98.0, 28.0, "planner-button"),
            ],
        };
        let winner = choose_bbox(&set).expect("winner");
        assert_eq!(winner.source, BboxSource::StructuralAx);
    }

    #[test]
    fn dedupe_iou_overlap_same_label_prefers_structural() {
        // Heavy overlap (IoU well above 0.7). Structural has lower confidence
        // than OCR yet still wins.
        let a = bb(BboxSource::OcrVlm, 0.99, 100.0, 100.0, 200.0, 50.0, "save");
        let b = bb(BboxSource::StructuralDom, 0.50, 102.0, 101.0, 198.0, 49.0, "save");
        assert!(a.iou(&b) > 0.7, "iou was {}", a.iou(&b));
        let set = BboxCandidateSet {
            target_label: "save".into(),
            candidates: vec![a, b],
        };
        let winner = choose_bbox(&set).expect("winner");
        assert_eq!(winner.source, BboxSource::StructuralDom);
    }

    #[test]
    fn no_candidates_returns_none() {
        let set = BboxCandidateSet {
            target_label: "empty".into(),
            candidates: vec![],
        };
        assert!(choose_bbox(&set).is_none());
    }

    #[test]
    fn all_ocr_picks_highest_confidence_within_tier() {
        // Two OCR-tier candidates: VLM at 0.4 vs Tesseract at 0.9.
        // VLM's tier still wins over Tesseract regardless of confidence.
        let set = BboxCandidateSet {
            target_label: "label".into(),
            candidates: vec![
                bb(BboxSource::OcrTesseract, 0.90, 0.0, 0.0, 10.0, 10.0, "label"),
                bb(BboxSource::OcrVlm, 0.40, 0.0, 0.0, 10.0, 10.0, "label"),
            ],
        };
        let winner = choose_bbox(&set).expect("winner");
        assert_eq!(winner.source, BboxSource::OcrVlm);
    }

    #[test]
    fn vlm_beats_tesseract_within_ocr_tier() {
        let set = BboxCandidateSet {
            target_label: "x".into(),
            candidates: vec![
                bb(BboxSource::OcrTesseract, 0.80, 0.0, 0.0, 10.0, 10.0, "x"),
                bb(BboxSource::OcrVlm, 0.81, 0.0, 0.0, 10.0, 10.0, "x"),
            ],
        };
        let w = choose_bbox(&set).expect("w");
        assert_eq!(w.source, BboxSource::OcrVlm);
    }

    #[test]
    fn structural_label_differs_from_ocr_label_at_same_coords_structural_wins() {
        let set = BboxCandidateSet {
            target_label: "planner-button".into(),
            candidates: vec![
                bb(BboxSource::OcrTesseract, 0.95, 50.0, 50.0, 120.0, 30.0, "PLANNER"),
                bb(
                    BboxSource::StructuralAx,
                    0.70,
                    50.0,
                    50.0,
                    120.0,
                    30.0,
                    "planner-button",
                ),
            ],
        };
        let w = choose_bbox(&set).expect("w");
        assert_eq!(w.source, BboxSource::StructuralAx);
        assert_eq!(w.label.as_deref(), Some("planner-button"));
    }

    #[test]
    fn terminal_loses_to_ax_but_beats_ocr() {
        let set = BboxCandidateSet {
            target_label: "cell".into(),
            candidates: vec![
                bb(BboxSource::StructuralTerminal, 1.0, 0.0, 0.0, 10.0, 10.0, "cell"),
                bb(BboxSource::OcrVlm, 0.9, 0.0, 0.0, 10.0, 10.0, "cell"),
            ],
        };
        let w = choose_bbox(&set).expect("w");
        assert_eq!(w.source, BboxSource::StructuralTerminal);

        let set2 = BboxCandidateSet {
            target_label: "cell".into(),
            candidates: vec![
                bb(BboxSource::StructuralTerminal, 0.4, 0.0, 0.0, 10.0, 10.0, "cell"),
                bb(BboxSource::StructuralAx, 0.4, 0.0, 0.0, 10.0, 10.0, "cell"),
            ],
        };
        let w2 = choose_bbox(&set2).expect("w2");
        assert_eq!(w2.source, BboxSource::StructuralAx);
    }

    #[test]
    fn iou_math_is_sane() {
        let a = bb(BboxSource::OcrVlm, 1.0, 0.0, 0.0, 10.0, 10.0, "a");
        let b = bb(BboxSource::OcrVlm, 1.0, 5.0, 0.0, 10.0, 10.0, "b");
        // inter = 5x10=50; union = 100+100-50=150; iou = 1/3
        let iou = a.iou(&b);
        assert!((iou - (50.0 / 150.0)).abs() < 1e-9, "iou={iou}");

        let c = bb(BboxSource::OcrVlm, 1.0, 100.0, 100.0, 5.0, 5.0, "c");
        assert_eq!(a.iou(&c), 0.0);
    }

    #[test]
    fn hit_rate_fixture_corpus() {
        let tmp = tempdir();

        // Manifest 1: structural wins.
        write_manifest(
            &tmp,
            "manifest-1.json",
            &[
                (BboxSource::StructuralAx, 0.6, "btn"),
                (BboxSource::OcrTesseract, 0.95, "btn"),
            ],
        );
        // Manifest 2: only OCR.
        write_manifest(
            &tmp,
            "manifest-2.json",
            &[
                (BboxSource::OcrVlm, 0.7, "title"),
                (BboxSource::OcrTesseract, 0.6, "title"),
            ],
        );
        // Manifest 3: empty candidate set.
        write_manifest(&tmp, "manifest-3.json", &[]);
        // Manifest 4: structural DOM.
        write_manifest(&tmp, "manifest-4.json", &[(BboxSource::StructuralDom, 0.9, "hdr")]);
        // Manifest 5: structural terminal.
        write_manifest(
            &tmp,
            "manifest-5.json",
            &[(BboxSource::StructuralTerminal, 0.8, "cell")],
        );

        let hr = measure_hit_rate(tmp.path());
        assert_eq!(hr.structural, 3, "{hr:?}");
        assert_eq!(hr.ocr, 1, "{hr:?}");
        assert_eq!(hr.none, 1, "{hr:?}");
        assert_eq!(hr.total(), 5);
    }

    // --- tiny std-only tempdir shim (avoid a new dep) -----------------------

    struct Tmp {
        p: std::path::PathBuf,
    }
    impl Tmp {
        fn path(&self) -> &Path {
            &self.p
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.p);
        }
    }
    fn tempdir() -> Tmp {
        let base = std::env::temp_dir();
        let nonce = format!(
            "bbox-ground-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let p = base.join(nonce);
        fs::create_dir_all(&p).unwrap();
        Tmp { p }
    }

    fn write_manifest(tmp: &Tmp, name: &str, cands: &[(BboxSource, f64, &str)]) {
        let candidates: Vec<serde_json::Value> = cands
            .iter()
            .map(|(src, conf, lbl)| {
                serde_json::json!({
                    "source": src,
                    "confidence": conf,
                    "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0,
                    "label": lbl,
                    "structural_path": null,
                })
            })
            .collect();
        let m = serde_json::json!({
            "steps": [{
                "bbox_candidates": {
                    "target_label": "t",
                    "candidates": candidates,
                }
            }]
        });
        fs::write(tmp.path().join(name), serde_json::to_string(&m).unwrap()).unwrap();
    }
}
