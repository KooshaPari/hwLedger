//! Corpus loader for hit-rate measurement.
//!
//! Walks a directory, reads `manifest*.json` files, and extracts
//! [`BboxCandidateSet`]s. Two shapes are supported:
//!
//!   1. **Native D6 shape** — a step carries
//!      `"bbox_candidates": { "target_label": "...", "candidates": [...] }`
//!      where each candidate has a `source` tag.
//!   2. **Legacy annotations shape** — a step carries
//!      `"annotations": [{"bbox": [x,y,w,h], "label": "..."}, ...]` with no
//!      `source` field. Legacy rows are treated as untagged and excluded from
//!      the hit-rate count (they surface as `none` only if no native shape is
//!      present). This keeps the harness honest: an empty native-shape corpus
//!      will produce an empty table, as the spec demands.

use crate::{BboxCandidateSet, GroundedBbox};
use serde_json::Value;
use std::path::Path;
use walkdir::WalkDir;

/// Load every [`BboxCandidateSet`] found under `root`.
///
/// Quietly skips unreadable files and malformed JSON — this is an analysis
/// harness, not a validator.
pub fn load_candidate_sets(root: &Path) -> Vec<BboxCandidateSet> {
    let mut out = Vec::new();
    if !root.exists() {
        return out;
    }
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = match path.file_name().and_then(|s| s.to_str()) {
            Some(n) => n,
            None => continue,
        };
        let is_candidate_file = name.ends_with(".json")
            && (name.starts_with("manifest") || name == "bbox-candidates.json");
        if !is_candidate_file {
            continue;
        }
        let body = match std::fs::read_to_string(path) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let json: Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(_) => continue,
        };
        collect_from_value(&json, &mut out);
    }
    out
}

fn collect_from_value(v: &Value, out: &mut Vec<BboxCandidateSet>) {
    // Top-level `bbox_candidates`.
    if let Some(bc) = v.get("bbox_candidates") {
        if let Some(set) = parse_candidate_set(bc) {
            out.push(set);
        }
    }
    // Per-step `bbox_candidates`.
    if let Some(steps) = v.get("steps").and_then(|s| s.as_array()) {
        for step in steps {
            if let Some(bc) = step.get("bbox_candidates") {
                if let Some(set) = parse_candidate_set(bc) {
                    out.push(set);
                } else if bc.is_array() {
                    // Alternative shape: a bare array of candidates. Wrap it.
                    if let Some(cands) = parse_candidate_array(bc) {
                        let label = step
                            .get("slug")
                            .and_then(|s| s.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        out.push(BboxCandidateSet {
                            target_label: label,
                            candidates: cands,
                        });
                    }
                }
            }
        }
    }
}

fn parse_candidate_set(v: &Value) -> Option<BboxCandidateSet> {
    let obj = v.as_object()?;
    let label = obj
        .get("target_label")
        .and_then(|x| x.as_str())
        .unwrap_or("unknown")
        .to_string();
    let cands = obj.get("candidates").and_then(parse_candidate_array)?;
    Some(BboxCandidateSet {
        target_label: label,
        candidates: cands,
    })
}

fn parse_candidate_array(v: &Value) -> Option<Vec<GroundedBbox>> {
    let arr = v.as_array()?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        if let Ok(gb) = serde_json::from_value::<GroundedBbox>(item.clone()) {
            out.push(gb);
        }
    }
    Some(out)
}
