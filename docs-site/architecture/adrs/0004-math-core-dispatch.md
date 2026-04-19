# ADR 0004 — Math core: architecture-keyed `AttentionKind` dispatch

Date: 2026-04-19
Status: Accepted

## Context

hwLedger's differentiator is correctly handling MoE, MLA, hybrid attention, and SSM — every public VRAM calculator gets these wrong. The math core must dispatch on architecture shape, not on a one-size-fits-all dense-model formula with fudge factors.

Research brief 04 (`docs/research/04-kv-formulas.md` — pending archive) enumerates eight architecture families with distinct per-token state formulas:

| Kind | Formula (bytes / token) |
|---|---|
| `Mha` | `2 · L · H · d · b` |
| `Gqa` | `2 · L · H_kv · d · b` |
| `Mqa` | `2 · L · 1 · d · b` |
| `Mla` | `(kv_lora_rank + qk_rope_head_dim) · b` — **layer-invariant** in absorb mode |
| `SlidingWindow { window }` | `2 · L · H_kv · d · min(seq_len, window) · b` |
| `Ssm { state_size }` | `state_size · L · b` — **seq-invariant** |
| `Hybrid(Vec<LayerKind>)` | Σ over layers by kind |
| `AttentionSink { sinks, window }` | `2 · L · H_kv · d · (sinks + window) · b` |

## Decision

The math core crate `hwledger-core::math` exposes an `AttentionKind` enum with one variant per architecture family, and a `KvFormula` trait with a single method:

```text
trait KvFormula {
    fn bytes_per_token(&self, seq_len: u64, bytes_per_element: f64) -> f64;
}
```

Each `AttentionKind` variant carries its architecture-specific parameters (e.g. `Mla { kv_lora_rank, qk_rope_head_dim }`, `Hybrid(Vec<LayerKind>)`). The `hwledger-arch` crate owns the `classify(&Config) -> AttentionKind` mapping from HF `config.json` + GGUF headers + MLX configs.

Total memory is composed, not baked into one formula:

```text
VRAM  =  W_weights(quant)
      +  O_runtime
      +  KV_seq · live_sequences
      +  A_prefill(batch, seq)
```

Where `W_weights` distinguishes **resident vs active** parameters for MoE (full model loaded, only a fraction active per token), and `O_runtime` is calibrated per backend (MLX, mistral.rs, vLLM).

### Open-enum policy

`AttentionKind` is non-exhaustive. Adding a new variant for (e.g.) future MLA revisions or new hybrid layouts is an additive change guarded by:
- A new `LayerKind` or top-level variant.
- Property tests proving the new formula matches a vendored reference model within ±200 MB.
- An ADR addendum describing provenance of the formula.

## Consequences

- **Correctness over convenience.** Downstream UI code must always render a breakdown (weights / KV / runtime / prefill / free); it cannot collapse to a single "VRAM" number without losing the MoE-vs-total and KV-vs-weights distinctions users need.
- **Classification is fallible.** `classify` returns `Result<AttentionKind, ClassifyError>`; unknown configs surface as explicit errors, not silent dense-fallbacks (fail-loudly policy).
- **Test matrix grows per new family.** Accepted: each new variant must land with its golden-test fixture.

## Rejected alternatives

- **Single dense formula + per-model correction factor.** This is what HF Accelerate and can-it-run-llm do. Fails for MoE and MLA at >50 % error margins.
- **Closed-enum with string-based fallback.** Makes classification drift silent; contradicts fail-loudly.
- **Trait objects all the way down (no enum).** Loses pattern-match exhaustiveness that catches missing implementations at compile time.

## References

- `PLAN.md` §5 Math core.
- Research brief: KV / state formulas per architecture (to be archived at `docs/research/04-kv-formulas.md`).
- DeepSeek-V2 MLA paper, Qwen3.6 `config.json` (`layer_types`), Jamba paper.
- vLLM `kv_cache_dtype` flag; llama.cpp KV-quant options.
