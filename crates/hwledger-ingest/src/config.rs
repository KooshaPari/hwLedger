//! Model config max-context resolution + human token count parsing.
//!
//! Traces to: FR-PLAN-003 (Planner sequence-length bounds)
//!
//! # Overview
//!
//! Reads a HuggingFace-style `config.json` payload and returns the effective
//! maximum context window length the model supports. Supports:
//!
//! - `max_position_embeddings` (primary)
//! - `rope_scaling.original_max_position_embeddings` (for RoPE-scaled configs)
//! - `rope_scaling.factor` as a multiplier over `max_position_embeddings`
//! - `sliding_window` as an additional cap (e.g. Mistral, Gemma-2)
//! - `model_max_length` (tokenizer-level cap; often identical)
//!
//! Returns `None` for state-space architectures with no positional cap
//! (Mamba, Mamba-2, pure SSM) — those are effectively unbounded.
//!
//! Also exposes `parse_token_count` for `"128K"` / `"1M"` / `"10M"` CLI inputs.

use serde_json::Value;

/// Parse an effective max-context bound from raw `config.json` JSON.
///
/// Returns:
/// - `Some(n)` when the config specifies a finite positional / window bound.
/// - `None` when the model is effectively unbounded (pure SSM / Mamba with no
///   positional encoding) OR the JSON is unparseable / contains no recognised
///   field. Callers should treat `None` as "unknown → allow full slider range".
///
/// # Priority
///
/// 1. Pure SSM (model_type in {mamba, mamba2}) with no attention fields → `None`.
/// 2. `rope_scaling.factor × max_position_embeddings` (RoPE-scaled Llama / Qwen).
/// 3. `max_position_embeddings` as the baseline.
/// 4. `sliding_window` as an additional cap (applied as `min`).
/// 5. `model_max_length` fallback if `max_position_embeddings` is missing.
pub fn parse_max_context(config_json: &str) -> Option<u32> {
    let value: Value = serde_json::from_str(config_json).ok()?;

    // 1. Pure SSM / Mamba: no positional bound.
    let model_type = value.get("model_type").and_then(Value::as_str).map(str::to_ascii_lowercase);
    if let Some(ref mt) = model_type {
        let is_pure_ssm = (mt == "mamba" || mt == "mamba2")
            && value.get("num_attention_heads").is_none()
            && value.get("layer_types").is_none();
        if is_pure_ssm {
            return None;
        }
    }

    // 2. Primary: max_position_embeddings.
    let base_mpe = value.get("max_position_embeddings").and_then(Value::as_u64);

    // 3. RoPE scaling can extend the base window.
    let rope_scaling = value.get("rope_scaling").and_then(Value::as_object);
    let rope_extended = rope_scaling.and_then(|rs| {
        let original =
            rs.get("original_max_position_embeddings").and_then(Value::as_u64).or(base_mpe)?;
        let factor = rs.get("factor").and_then(Value::as_f64).unwrap_or(1.0);
        let scaled = (original as f64) * factor;
        Some(scaled.round().max(0.0).min(u32::MAX as f64) as u32)
    });

    // 4. sliding_window caps the above (Mistral, Gemma-2 local attention).
    let sliding = value.get("sliding_window").and_then(Value::as_u64).map(|v| v as u32);

    // 5. model_max_length as secondary fallback.
    let model_max_length = value.get("model_max_length").and_then(Value::as_u64).map(|v| v as u32);

    let base = rope_extended.or_else(|| base_mpe.map(|v| v as u32)).or(model_max_length)?;

    // Apply sliding-window as an upper cap when present AND smaller.
    // Rationale: local attention layers bound effective range to `sliding_window`
    // tokens even when positional embeddings stretch further.
    let effective = match sliding {
        Some(sw) if sw > 0 && sw < base => sw,
        _ => base,
    };

    Some(effective)
}

/// Parse a human token-count string like `"128K"`, `"1M"`, `"10M"`, `"4096"`.
///
/// Accepted suffixes (case-insensitive): `K` = 1_024, `M` = 1_048_576,
/// `G` = 1_073_741_824. Underscores and commas are allowed as digit separators.
///
/// Returns `Err(String)` with a user-readable message on failure.
///
/// # Examples
///
/// ```
/// use hwledger_ingest::config::parse_token_count;
/// assert_eq!(parse_token_count("128K").unwrap(), 128 * 1024);
/// assert_eq!(parse_token_count("1M").unwrap(), 1024 * 1024);
/// assert_eq!(parse_token_count("4096").unwrap(), 4096);
/// ```
pub fn parse_token_count(s: &str) -> Result<u64, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err("empty token count".to_string());
    }
    let cleaned: String = trimmed.chars().filter(|c| *c != '_' && *c != ',').collect();

    let last = cleaned.chars().last().unwrap();
    let (digits, multiplier) = match last {
        'k' | 'K' => (&cleaned[..cleaned.len() - 1], 1024_u64),
        'm' | 'M' => (&cleaned[..cleaned.len() - 1], 1024 * 1024),
        'g' | 'G' => (&cleaned[..cleaned.len() - 1], 1024 * 1024 * 1024),
        c if c.is_ascii_digit() => (cleaned.as_str(), 1),
        _ => return Err(format!("invalid token-count suffix in '{}'", s)),
    };

    let base: f64 = digits.parse().map_err(|_| format!("invalid token-count digits in '{}'", s))?;
    if base < 0.0 || !base.is_finite() {
        return Err(format!("token count must be non-negative: '{}'", s));
    }
    let value = base * (multiplier as f64);
    if value > (u64::MAX as f64) {
        return Err(format!("token count overflows u64: '{}'", s));
    }
    Ok(value.round() as u64)
}

/// Format a token count using the same `K`/`M`/`G` suffixes, used for error
/// messages so CLI output matches what the user entered.
pub fn fmt_token_count(n: u64) -> String {
    const G: u64 = 1024 * 1024 * 1024;
    const M: u64 = 1024 * 1024;
    const K: u64 = 1024;
    if n >= G && n % G == 0 {
        format!("{}G", n / G)
    } else if n >= M && n % M == 0 {
        format!("{}M", n / M)
    } else if n >= K && n % K == 0 {
        format!("{}K", n / K)
    } else {
        n.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_golden(name: &str) -> String {
        let root = env!("CARGO_MANIFEST_DIR");
        // Fixtures live at <repo>/tests/golden/ — two levels up from the crate.
        let path = std::path::Path::new(root)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests/golden")
            .join(format!("{}.json", name));
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn parse_max_context_deepseek_v3() {
        // DeepSeek-V3 config declares 131_072 positional embeddings.
        let cfg = load_golden("deepseek-v3");
        assert_eq!(parse_max_context(&cfg), Some(131_072));
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn parse_max_context_llama3_1_8b() {
        // No rope_scaling override in the fixture → uses max_position_embeddings.
        let cfg = load_golden("llama3.1-8b");
        assert_eq!(parse_max_context(&cfg), Some(8192));
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn parse_max_context_llama3_70b() {
        let cfg = load_golden("llama3-70b");
        assert_eq!(parse_max_context(&cfg), Some(8192));
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn parse_max_context_llama2_70b() {
        let cfg = load_golden("llama2-70b");
        assert_eq!(parse_max_context(&cfg), Some(4096));
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn parse_max_context_qwen2_7b() {
        let cfg = load_golden("qwen2-7b");
        assert_eq!(parse_max_context(&cfg), Some(32768));
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn parse_max_context_mistral_7b_sliding_window() {
        // max_position_embeddings = 32768, sliding_window = 4096 → effective 4096.
        let cfg = load_golden("mistral-7b");
        assert_eq!(parse_max_context(&cfg), Some(4096));
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn parse_max_context_mixtral_sliding_window() {
        let cfg = load_golden("mixtral-8x7b");
        assert_eq!(parse_max_context(&cfg), Some(4096));
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn parse_max_context_gemma2_12b_sliding_window() {
        // max_position_embeddings = 8192, sliding_window = 4096.
        let cfg = load_golden("gemma3-12b");
        assert_eq!(parse_max_context(&cfg), Some(4096));
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn parse_max_context_jamba_hybrid() {
        // Jamba hybrid SSM+attention: NOT pure SSM, has max_position_embeddings.
        let cfg = load_golden("jamba-v0.1");
        assert_eq!(parse_max_context(&cfg), Some(262_144));
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn parse_max_context_mamba2_pure_ssm_unbounded() {
        // Pure Mamba with no attention → None (unbounded).
        let cfg = load_golden("mamba2-2.7b");
        assert_eq!(parse_max_context(&cfg), None);
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn parse_max_context_rope_scaled_virtual_extended() {
        let cfg = r#"{
            "model_type": "llama",
            "max_position_embeddings": 8192,
            "rope_scaling": { "type": "linear", "factor": 4.0 }
        }"#;
        assert_eq!(parse_max_context(cfg), Some(32_768));
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn parse_max_context_rope_original_fallback() {
        // original_max_position_embeddings preferred over current mpe as the base.
        let cfg = r#"{
            "model_type": "llama",
            "max_position_embeddings": 131072,
            "rope_scaling": {
                "factor": 8.0,
                "original_max_position_embeddings": 8192
            }
        }"#;
        assert_eq!(parse_max_context(cfg), Some(65_536));
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn parse_max_context_model_max_length_fallback() {
        let cfg = r#"{"model_type":"foo","model_max_length":16384}"#;
        assert_eq!(parse_max_context(cfg), Some(16_384));
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn parse_max_context_invalid_json_returns_none() {
        assert_eq!(parse_max_context("{not json"), None);
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn parse_max_context_empty_config_returns_none() {
        assert_eq!(parse_max_context("{}"), None);
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn parse_token_count_plain_integer() {
        assert_eq!(parse_token_count("4096").unwrap(), 4096);
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn parse_token_count_k_suffix() {
        assert_eq!(parse_token_count("128K").unwrap(), 128 * 1024);
        assert_eq!(parse_token_count("4k").unwrap(), 4 * 1024);
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn parse_token_count_m_suffix() {
        assert_eq!(parse_token_count("1M").unwrap(), 1024 * 1024);
        assert_eq!(parse_token_count("10M").unwrap(), 10 * 1024 * 1024);
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn parse_token_count_g_suffix() {
        assert_eq!(parse_token_count("2G").unwrap(), 2 * 1024 * 1024 * 1024);
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn parse_token_count_separators() {
        assert_eq!(parse_token_count("1_024").unwrap(), 1024);
        assert_eq!(parse_token_count("1,024").unwrap(), 1024);
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn parse_token_count_rejects_garbage() {
        assert!(parse_token_count("").is_err());
        assert!(parse_token_count("abc").is_err());
        assert!(parse_token_count("5T").is_err());
        assert!(parse_token_count("-1K").is_err());
    }

    // Traces to: FR-PLAN-003
    #[test]
    fn fmt_token_count_roundtrip() {
        assert_eq!(fmt_token_count(4096), "4K");
        assert_eq!(fmt_token_count(128 * 1024), "128K");
        assert_eq!(fmt_token_count(1024 * 1024), "1M");
        assert_eq!(fmt_token_count(4097), "4097");
    }
}
