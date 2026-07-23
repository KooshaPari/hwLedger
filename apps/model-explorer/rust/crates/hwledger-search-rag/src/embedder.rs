//! Deterministic stub embedder + the [`Embedder`] trait real backends implement.
//!
//! [`StubEmbedder`] is intentionally dependency-free: it hashes the input
//! text with FNV-1a to derive a seed, walks a linear congruential generator
//! to fill a buffer of `dim` floats in `[-1, 1]`, then L2-normalizes the
//! resulting vector. The output is fully deterministic per input — same
//! text → same vector — and stable across runs and platforms, which makes
//! it ideal for golden tests and offline fixtures.

use serde::{Deserialize, Serialize};

use crate::error::RagError;

/// Knobs for any concrete [`Embedder`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbedderConfig {
    /// Output dimensionality.
    pub dim: usize,
    /// Whether the embedder must be deterministic (i.e. same input →
    /// same vector). [`StubEmbedder`] always honors this.
    pub deterministic: bool,
}

impl Default for EmbedderConfig {
    fn default() -> Self {
        Self {
            dim: 384,
            deterministic: true,
        }
    }
}

/// Backend-agnostic embedding interface.
pub trait Embedder: Send + Sync {
    /// Embed `text` into a `dim`-dimensional float vector.
    fn embed(&self, text: &str) -> Result<Vec<f32>, RagError>;

    /// Output dimensionality.
    fn dim(&self) -> usize;

    /// Stable human-readable name (e.g. `"stub"`, `"bge-small"`).
    fn name(&self) -> &str;
}

/// Zero-dependency, deterministic stub embedder.
#[derive(Debug, Clone, Default)]
pub struct StubEmbedder {
    cfg: EmbedderConfig,
}

impl StubEmbedder {
    /// Build a stub embedder with the default config (`dim=384`,
    /// `deterministic=true`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a stub embedder with a custom output dimensionality.
    pub fn with_dim(dim: usize) -> Self {
        Self {
            cfg: EmbedderConfig {
                dim,
                deterministic: true,
            },
        }
    }
}

impl Embedder for StubEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>, RagError> {
        if self.cfg.dim == 0 {
            return Err(RagError::Embedder("dim must be > 0".into()));
        }
        let seed = fnv1a_64(text.as_bytes());
        let mut lcg = Lcg::new(seed);
        let mut vec: Vec<f32> = Vec::with_capacity(self.cfg.dim);
        for _ in 0..self.cfg.dim {
            // Map LCG output to [-1, 1].
            let u = lcg.next_unit();
            vec.push(u * 2.0 - 1.0);
        }
        // L2-normalize so cosine sim is just a dot product.
        let norm = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in vec.iter_mut() {
                *v /= norm;
            }
        } else {
            // Degenerate: all zeros → uniform unit vector to keep dim stable.
            let uniform = 1.0 / (self.cfg.dim as f32).sqrt();
            for v in vec.iter_mut() {
                *v = uniform;
            }
        }
        Ok(vec)
    }

    fn dim(&self) -> usize {
        self.cfg.dim
    }

    fn name(&self) -> &str {
        "stub"
    }
}

/// FNV-1a 64-bit hash. Stable, no-dependency, branch-light.
fn fnv1a_64(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut h = FNV_OFFSET;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// Tiny LCG used purely to fill an embedding buffer deterministically.
/// Constants are from Numerical Recipes' L'Ecuyer-style combined
/// generator — any LCG with a full period would do here, since the only
/// requirement is reproducibility.
struct Lcg {
    state: u64,
}

impl Lcg {
    const MULT: u64 = 6_364_136_223_846_793_005;
    const INC: u64 = 1_442_695_040_888_963_407;

    fn new(seed: u64) -> Self {
        // Avoid the zero state (degenerate period).
        Self {
            state: seed.wrapping_add(1),
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(Self::MULT).wrapping_add(Self::INC);
        self.state
    }

    /// Uniform [0, 1).
    fn next_unit(&mut self) -> f32 {
        // Take the top 24 bits so the result fits in f32 mantissa.
        let u = self.next_u64() >> 40;
        (u as f32) / ((1u32 << 24) as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dim_is_384() {
        let e = StubEmbedder::default();
        assert_eq!(e.dim(), 384);
        assert_eq!(e.name(), "stub");
    }

    #[test]
    fn embed_returns_configured_dim() {
        let v = StubEmbedder::with_dim(64).embed("hello").unwrap();
        assert_eq!(v.len(), 64);
    }

    #[test]
    fn embedding_is_l2_normalized() {
        let v = StubEmbedder::default().embed("the quick brown fox").unwrap();
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5, "norm was {norm}");
    }

    #[test]
    fn fnv1a_is_stable() {
        assert_eq!(fnv1a_64(b""), 0xcbf2_9ce4_8422_2325);
    }
}