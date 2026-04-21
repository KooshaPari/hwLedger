//! Benchmark corpus: published numbers keyed by (family, params_b, hardware).
//!
//! Loaded from `data/benchmarks.yaml` at compile time via `include_str!`.
//! Each row carries a `source` field — arxiv id or vendor whitepaper URL — and
//! every prediction that consults a row emits that row's [`Citation`].
//!
//! Traces to: FR-PREDICT-006

use crate::Citation;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Benchmark {
    pub model: String,
    pub family: String,
    pub params_b: f64,
    pub hardware: String,
    pub batch: u32,
    pub seq: u32,
    pub weight_quant: String,
    pub kv_quant: String,
    pub decode_tps: f64,
    pub ttft_ms: Option<f64>,
    pub runtime: String,
    pub source: String,
    pub url: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct BenchmarkCorpus {
    rows: Vec<Benchmark>,
}

impl BenchmarkCorpus {
    pub fn from_yaml(s: &str) -> Result<Self, serde_yaml::Error> {
        let rows: Vec<Benchmark> = serde_yaml::from_str(s)?;
        Ok(Self { rows })
    }

    pub fn rows(&self) -> &[Benchmark] {
        &self.rows
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Exact-ish match: family matches and params_b within ±10%.
    pub fn find_match(
        &self,
        family: &str,
        params_b: f64,
        hardware: &str,
    ) -> Option<(&Benchmark, Citation)> {
        let fam = family.to_lowercase();
        let hw = hardware.to_lowercase();
        self.rows
            .iter()
            .find(|r| {
                r.family.to_lowercase() == fam
                    && r.hardware.to_lowercase() == hw
                    && (r.params_b - params_b).abs() / params_b.max(0.1) <= 0.10
            })
            .map(|r| {
                (
                    r,
                    Citation {
                        label: format!("Benchmark: {} on {}", r.model, r.hardware),
                        source: r.source.clone(),
                        url: r.url.clone(),
                    },
                )
            })
    }

    /// Nearest-family row on the given hardware.
    pub fn find_nearest_same_family(
        &self,
        family: &str,
        params_b: f64,
        hardware: &str,
    ) -> Option<(&Benchmark, Citation)> {
        let fam = family.to_lowercase();
        let hw = hardware.to_lowercase();
        self.rows
            .iter()
            .filter(|r| r.family.to_lowercase() == fam && r.hardware.to_lowercase() == hw)
            .min_by(|a, b| {
                (a.params_b - params_b)
                    .abs()
                    .partial_cmp(&(b.params_b - params_b).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|r| {
                (
                    r,
                    Citation {
                        label: format!("Extrapolated from: {} on {}", r.model, r.hardware),
                        source: r.source.clone(),
                        url: r.url.clone(),
                    },
                )
            })
    }
}

const EMBEDDED_YAML: &str = include_str!("../data/benchmarks.yaml");

static DEFAULT_CORPUS: Lazy<BenchmarkCorpus> =
    Lazy::new(|| BenchmarkCorpus::from_yaml(EMBEDDED_YAML).expect("benchmarks.yaml parse error"));

pub fn default_corpus() -> &'static BenchmarkCorpus {
    &DEFAULT_CORPUS
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-PREDICT-006
    #[test]
    fn corpus_loads_at_least_50_rows() {
        assert!(default_corpus().len() >= 50, "corpus has {} rows", default_corpus().len());
    }

    // Traces to: FR-PREDICT-006
    #[test]
    fn every_row_cites_a_source() {
        for r in default_corpus().rows() {
            assert!(
                r.source.starts_with("arxiv:")
                    || r.source.starts_with("vendor:")
                    || r.source.starts_with("hf:"),
                "row {:?} has no canonical source",
                r.model
            );
        }
    }

    // Traces to: FR-PREDICT-006
    #[test]
    fn llama_a100_lookup_works() {
        let c = default_corpus();
        let hit = c.find_match("llama", 70.0, "A100-80G");
        assert!(hit.is_some(), "expected llama-70B A100 benchmark");
    }
}
