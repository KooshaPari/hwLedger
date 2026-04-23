---
title: Multi-Head Latent Attention (MLA)
description: Low-rank KV projection
---

# Multi-Head Latent Attention (MLA)

<Shot src="/cli-journeys/keyframes/plan-deepseek/frame-003.png"
      caption="Planner auto-detects MLA with kv_lora_rank=512 for DeepSeek-V3"
      size="small" align="right" />

<RecordingEmbed tape="planner-gui-launch" kind="gui" caption="Planner GUI: MLA auto-detected, per-layer latent-KV breakdown rendered natively on macOS" />

<RecordingEmbed tape="streamlit-planner" kind="streamlit" caption="Streamlit Planner: same MLA breakdown in the browser — drag context, watch latent band stay flat" />

<RecordingEmbed tape="plan-mla-deepseek" kind="cli" caption="CLI plan: MLA planner run — per-layer KV breakdown in latent space (scriptable)" />

Compresses KV cache by projecting to a low-rank latent space before multi-head operation.

<!-- SHOT-MISMATCH: caption="Latent projection step highlighted in the plan trace" expected=[latent,projection,step,highlighted,plan,trace] matched=[] -->
<Shot src="/cli-journeys/keyframes/plan-mla-deepseek/frame-001.png"
      caption="Latent projection step highlighted in the plan trace"
      size="small" align="left"
      :annotations='[{"bbox":[120,180,320,28],"label":"latent_dim","color":"#89b4fa","position":"center-top"}]' />

## Formula

Project keys and values to latent dimension d_latent << d_model:

$$K_{\text{latent}} = KW^K \in \mathbb{R}^{\text{batch} \times \text{context} \times d_\text{latent}}$$
$$V_{\text{latent}} = VW^V \in \mathbb{R}^{\text{batch} \times \text{context} \times d_\text{latent}}$$

Then apply standard multi-head attention on latent space:

$$\text{MLA}(Q, K, V) = \text{Concat}(\text{head}_1, \ldots, \text{head}_h) W^O$$

Benefit: KV cache is d_latent-sized instead of d_model-sized.

## Why this variant

MLA was the DeepSeek team's answer to the specific problem that even GQA's 8× compression left long-context (>100K) inference infeasible on commodity hardware for models in the 200B+ parameter class. By projecting into a latent space *before* splitting into heads, MLA stores a single `kv_lora_rank`-sized tensor per token instead of per-head K and V tensors — a 10–16× reduction over GQA at equivalent quality. It was introduced in [DeepSeek-V2 (2024)](https://arxiv.org/abs/2405.04434) and productionized in [DeepSeek-V3 (2024–2025)](https://arxiv.org/abs/2412.19437) and DeepSeek-R1 (2025). The technique is also the basis for Qwen's latent variants.

<Shot src="/cli-journeys/keyframes/plan-mla-deepseek/frame-002.png"
      caption="Per-layer KV cache breakdown for DeepSeek-V2 MLA sweep"
      size="small" align="left" />

**hwLedger accounting gotcha.** MLA's KV footprint is `2 * kv_lora_rank * bytes` per token per layer — NOT `2 * num_kv_heads * head_dim * bytes`. A naive reuse of the GQA formula overstates memory by ~10× for DeepSeek-V3. `AttentionKind::MLA { latent_dim }` carries the latent dim explicitly; the planner will refuse to produce a result if `latent_dim` is missing rather than silently fall back to GQA math.

<!-- SHOT-MISMATCH: caption="Refuse-to-plan path: missing latent_dim surfaces as a hard error, not a silent fallback" expected=[refuse-to-plan,path,missing,latent_dim,surfaces,hard,error,not,silent,fallback] matched=[] -->
<Shot src="/cli-journeys/keyframes/plan-mla-deepseek/frame-004.png"
      caption="Refuse-to-plan path: missing latent_dim surfaces as a hard error, not a silent fallback"
      size="small" align="right"
      :annotations='[{"bbox":[80,240,480,32],"label":"E-PLAN-MLA-MISSING","color":"#f38ba8","style":"dashed","position":"bottom-left"}]' />

<RecordingEmbed tape="plan-deepseek" kind="cli" caption="CLI plan-deepseek: MLA classification inside the full plan run (supplementary, CLI-only)" />

<!-- SHOT-PENDING: inline <Shot> showing the refusal-to-plan error when latent_dim is missing -->

## Memory footprint (32K context, 7B model)

<Shot src="/cli-journeys/keyframes/plan-mla-deepseek/frame-003.png"
      caption="KV cache footprint (32K context) at FP16"
      size="medium" align="right" />

DeepSeek-V2 with MLA (d_latent = 256 vs d_model = 4096):
- KV cache per layer: 32K × 256 × 2 = **16.4 MB/layer**
- Full cache: **524 MB for 32 layers**
- Savings: **16x vs standard MHA**

## Which models use it

- **DeepSeek-V2** (128K context, latent-only KV)
- **Qwen2.5-32B** (rotating latent projections)

MLA is production-proven for ultra-long context models (100K+).

## hwLedger variant

`AttentionKind::MLA { latent_dim }` — stores latent dimension for dynamic planning. Enables longest context windows on memory-constrained hardware.

## Worked example: 32K context

DeepSeek-V2 (176B mixture-of-experts, d_latent=256):
- KV cache all layers: 32K × 256 × 2 × 60 (layers) = **983 MB**
- Decode batch size: 64 tokens simultaneously
- Total memory with model weights: ~50 GB (vs 100+ for standard attention)

### MLA vs MHA baseline (DeepSeek-V3, 32K context, FP16)

| Model | kv_lora_rank | layers | KV/layer | Full cache | vs MHA baseline |
|-------|--------------|--------|----------|------------|-----------------|
| DeepSeek-V2 MLA | 256 | 60 | 16 MiB | 960 MiB | ~16× smaller |
| DeepSeek-V3 MLA | 512 | 61 | 32 MiB | ~1.9 GiB | ~8× smaller |
| DeepSeek-V3 as-if-MHA (hypothetical) | — | 61 | ~256 MiB | ~15 GiB | 1× |

## 2026 citations

- [DeepSeek-V2 Technical Report](https://arxiv.org/abs/2405.04434) — production MLA with mixture-of-experts

## Related

- [Sliding Window Attention: Context management](/math/sliding-window)
- [GQA: Grouped compression](/math/gqa)
- [KV Cache Deep Dive](/math/kv-cache)
