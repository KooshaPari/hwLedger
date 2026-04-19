---
title: Research Provenance & Source Verification
brief_id: provenance
status: reference
date: 2026-04-19
---

# Research Provenance — Source Verification Log

This page tracks the **verification date and status** of every external source cited across the hwLedger research briefs. All sources were verified on **April 19, 2026**.

## 2026 Models & Architecture (April 2026)

| Source | Type | Status | Last Verified | Notes |
|--------|------|--------|---------------|-------|
| [Meta Llama 4 Multimodal Intelligence](https://ai.meta.com/blog/llama-4-multimodal-intelligence/) | Blog | ✅ Active | 2026-04-19 | Llama 4 Maverick: 17B active, 400B total, iRoPE, sparse MoE routing |
| [Gemma 3 Technical Report](https://arxiv.org/abs/2503.19786) | arXiv | ✅ Active | 2026-04-19 | 5:1 interleaved local (1024-token window) + global attention, 128K context |
| [Mamba-3: Improved Sequence Modeling](https://arxiv.org/abs/2603.15569) | arXiv | ✅ Active | 2026-04-19 | MIMO variant: state_size=64 (2× reduction vs Mamba-2), Mar 2026 |
| [DeepSeek-V3 Model Documentation](https://huggingface.co/docs/transformers/en/model_doc/deepseek_v3) | HF Docs | ✅ Active | 2026-04-19 | MLA parameters: kv_lora_rank=512, qk_rope_head_dim=64 |
| [Qwen 3.6 GitHub Repository](https://github.com/QwenLM/Qwen3.6) | GitHub | ✅ Active | 2026-04-19 | Hybrid GDN + sparse MoE, 256 experts, 1M context |
| [Jamba-1.5: Hybrid Transformer-Mamba at Scale](https://arxiv.org/abs/2408.12570) | arXiv | ✅ Active | 2026-04-19 | 94B active (Large), 12B active (Mini), 256K context, hybrid layers |

## Inference Engines (April 2026)

| Source | Type | Status | Last Verified | Version |
|--------|------|--------|---------------|---------|
| [vLLM v0.19.0 Release Notes](https://github.com/vllm-project/vllm/releases) | GitHub | ✅ Active | 2026-04-19 | v0.19.0 (Apr 2026): PagedAttention v2, MLA support, Hugging Face integration |
| [oMLX Repository](https://github.com/jundot/omlx) | GitHub | ✅ Active | 2026-04-19 | v0.3.6 (Apr 2026): SSD-paged KV cache, MLX wrapper, MLA support |
| [Apple MLX Framework](https://github.com/ml-explore/mlx) | GitHub | ✅ Active | 2026-04-19 | Actively maintained, MLA kernels, unified memory optimizations |
| [mistral.rs Releases](https://github.com/EricLBuehler/mistral.rs/releases) | GitHub | ✅ Active | 2026-04-19 | MLA support (native), CUDA+Metal, MoE-aware routing |
| [llama.cpp](https://github.com/ggml-org/llama.cpp) | GitHub | ✅ Active | 2026-04-19 | Universal GGUF format, partial MLA support, ROCm/CUDA/Metal backends |

## Attention Research Papers

| Source | Type | Status | Last Verified | Year | Notes |
|--------|------|--------|---------------|------|-------|
| [Llama 2: Open Foundation and Fine-Tuned Chat Models](https://arxiv.org/abs/2307.09288) | arXiv | ✅ Active | 2026-04-19 | 2023 | Foundation for GQA adoption |
| [Mistral 7B](https://arxiv.org/abs/2310.06825) | arXiv | ✅ Active | 2026-04-19 | 2023 | Sliding window attention pioneer |
| [Mixtral of Experts](https://arxiv.org/abs/2401.04088) | arXiv | ✅ Active | 2026-04-19 | 2024 | Sparse MoE standard |
| [GQA: Training Generalized Multi-Query Transformers](https://arxiv.org/abs/2305.13245) | arXiv | ✅ Active | 2026-04-19 | 2023 | Grouped-query attention formalization |
| [Mamba: Linear-Time Sequence Modeling with Selective State Spaces](https://arxiv.org/abs/2312.00752) | arXiv | ✅ Active | 2026-04-19 | 2023 | SSM foundation |
| [Efficient Streaming Language Models with Attention Sinks](https://arxiv.org/abs/2309.17453) | arXiv | ✅ Active | 2026-04-19 | 2023 | Attention sink mechanism |

## FFI & Platform Tools (April 2026)

| Source | Type | Status | Last Verified | Version |
|--------|------|--------|---------------|---------|
| [UniFFI v0.31.0](https://github.com/mozilla/uniffi-rs) | GitHub | ✅ Active | 2026-04-19 | v0.31.0 (Jan 2026) |
| [CXX-Qt v0.7](https://github.com/kdab/cxx-qt) | GitHub | ✅ Active | 2026-04-19 | v0.7 (KDAB, 2026) |
| [Slint 1.15.1](https://github.com/slint-ui/slint) | GitHub | ✅ Active | 2026-04-19 | v1.15.1 (Apr 2026) |

## Competitive Tools & References

| Source | Type | Status | Last Verified | Notes |
|--------|------|--------|---------------|-------|
| [HF Model Memory Usage Calculator](https://huggingface.co/spaces/hf-accelerate/model-memory-usage) | Web | ✅ Active | 2026-04-19 | Streamlit-based; no MLA/Mamba support |
| [can-it-run-llm Space](https://huggingface.co/spaces/Vokturz/can-it-run-llm) | Web | ✅ Active | 2026-04-19 | Community-maintained; GPU selector |
| [LM Studio](https://lmstudio.ai/) | Desktop | ✅ Active | 2026-04-19 | Cross-platform GUI; GGUF models |
| [Claude Opus 4.7 Announcement](https://www.anthropic.com/news/claude-opus-4-7) | Blog | ✅ Active | 2026-04-19 | Released Apr 2026; 2576px (3.75MP) image support, extended thinking |

## Verification Notes

- **All links verified as resolvable** via HTTP HEAD requests on 2026-04-19.
- **arXiv papers** confirmed via direct abstract pages.
- **GitHub repositories** confirmed as actively maintained (commits within last 30 days).
- **Blog posts** from official vendor sources (Meta, Anthropic, Google, etc.).
- **Withdrawn or archived papers**: None detected in citation set.
- **Dead links**: None. All sources remain accessible as of 2026-04-19.

## Next Refresh Cycle

Expected: **June 15, 2026** (8-week interval)

Tasks:
1. Re-verify all URLs for accessibility.
2. Check for new model releases (Llama 5, DeepSeek-V4, Gemma 4, Qwen 4.x).
3. Update vLLM/mistral.rs/llama.cpp version pins.
4. Audit new papers on arXiv for attention mechanisms, quantization, MoE scaling.

---

**Generated**: 2026-04-19 UTC  
**Last Verified**: 2026-04-19 UTC  
**Brief Coverage**: All 12 research briefs + KV-cache math page
