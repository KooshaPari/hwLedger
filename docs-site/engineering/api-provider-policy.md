# API provider policy — MLX VLM priority chain

## Task routing (ADR-0015 v5, 2026-04-22)

The frame-describer chain is now **tiered by task family**, not a single
priority list. The runtime
(`tools/vlm-judge/src/providers.rs::describer_task_router`) infers the task
family from the keyframe's step context and walks the preferred tier list
until it finds an available backend. Florence-2-771M is the default tier-2
SLM — ~10× faster than UI-TARS-1.5-7B on the same frame at ~1.5 GB RAM vs.
~7 GB — and beats UI-TARS on generic captioning benchmarks. UI-TARS is
preserved as a tier-3 domain specialist for UI-action frames where its
screenshot→action training still wins.

**Tier semantics:**

| Tier                  | Purpose                                     | Canonical models                                        |
|-----------------------|---------------------------------------------|---------------------------------------------------------|
| `tier1_classical_cv`  | Deterministic OCR / layout                  | Apple Vision, Tesseract, PaddleOCR                      |
| `tier2_slm`           | Small task-specialist VLMs (caption/OCR)    | Florence-2-large/base, moondream2, SmolVLM2             |
| `tier3_domain`        | Domain specialists (UI-action describe)     | UI-TARS-1.5-7B (6-bit / 4-bit)                          |
| `tier4_omni`          | 2026 omni generalists (MLX)                 | Qwen3.6-35B-A3B, Qwen3.5-122B-A10B, Qwen3-VL-32B, …     |
| `tier5_cloud`         | Subscription / free cloud fallbacks         | Fireworks, OpenRouter `:free`, headless Claude CLI      |

**Task → tier mapping:**

| Task family          | Preferred tiers (first→last)                       | Rationale                                                                     |
|----------------------|----------------------------------------------------|-------------------------------------------------------------------------------|
| `caption_region`     | tier2_slm → tier3_domain → tier4_omni              | Florence-2 beats UI-TARS on generic captions; omni only when SLM unavailable. |
| `ui_action_describe` | tier3_domain → tier4_omni                          | UI-TARS wins on screenshot→action; skip tier2_slm entirely.                   |
| `ocr_only`           | tier1_classical_cv → tier2_slm                     | Classical OCR is deterministic; Florence-2 for layout-heavy frames.           |
| `novel_unusual`      | tier4_omni → tier5_cloud                           | Reach for the biggest generalist; fall to cloud if no local model loads.      |

**Task inference rules (runtime):**

- Terminal / CLI keyframes → `ocr_only`.
- SwiftUI / AppKit button or HUD interaction frames → `ui_action_describe`.
- Streamlit dashboard and docs-site gallery views → `caption_region`.
- Frames tagged `family: unknown` or outside the above → `novel_unusual`.

**Florence-2 footprint:**

- `microsoft/Florence-2-large` — 771M params, MIT licensed, ~1.5 GB RAM at
  float32 on MPS, ~50 ms/frame once the model is in memory. First-run
  download is ~1.6 GB from HuggingFace and a one-time `transformers` +
  `torch` install on the host (~2 min `pip install` on a warm pip cache).
- `microsoft/Florence-2-base` — 232M params, drop-in replacement for hosts
  with <4 GB free.

Florence-2 has no first-class MLX port yet, so the runtime shells to
`python -m` using `transformers` + `torch`. On Apple Silicon we select MPS
automatically; on other hosts the same code path uses CUDA or CPU.

---

The `mlx` provider in `docs/examples/api-providers.yaml` and the runtime
resolver in `tools/vlm-judge/src/main.rs` now treat `providers.mlx.models.vlm`
as a **priority chain**, not a single model id. The runtime walks the chain in
order and picks the first entry already present in the HuggingFace cache
(`~/.cache/huggingface/hub/models--<org>--<name>`). If nothing is cached, the
top entry is returned and `mlx_vlm.generate` triggers the download on first
use.

Override the chain with `--mlx-vlm-model <id>` (CLI flag, strongest),
`HWLEDGER_MLX_VLM_MODEL=<id>` (env var), or by editing the yaml directly.

## Natively-multimodal vs VLM-adapter architectural note

> **Qwen3.5 Highlights (Alibaba, official release spec):**
> *"Unified Vision-Language Foundation: Early fusion training on multimodal
> tokens achieves cross-generational parity with Qwen3 and outperforms
> Qwen3-VL models across reasoning, coding, agents, and visual understanding
> benchmarks."*

Two architectural families appear in this chain:

- **Natively multimodal (early-fusion):** Qwen3.5 and Qwen3.6 mix text and
  vision tokens during pre-training rather than bolting a vision encoder onto
  a frozen text LLM. Per Alibaba's own spec, this outperforms the adapter-style
  Qwen3-VL across every benchmark family they track (reasoning, coding,
  agents, visual understanding). The same pattern holds for GPT-4o-class
  proprietary models and is the emerging SOTA shape for open VLMs.
- **Adapter-style VLMs:** Qwen3-VL, Qwen2.5-VL, InternVL3, GLM-4.5V, Pixtral
  pair a pre-trained text LLM with a CLIP/SigLIP-family vision tower and a
  projector. Still useful (mature tooling, smaller footprints), but
  architecturally a generation behind for a given parameter budget.

Ranking rule: natively-multimodal Qwen3.5/3.6 MLX ports live in
`tier_mlx_native_multimodal` and are evaluated **before** any adapter-style
VLM. Qwen3-VL-32B was the previous top-of-chain entry; it is now demoted to
"adapter-VLM floor" and kept only for hosts that cannot run the MoE ports.

## Tiers

The yaml defines three tiers (ranked top to bottom within the chain):

- **tier_mlx_native_multimodal** — Qwen3.5 / Qwen3.6 natively-multimodal
  (early-fusion) MoE VLMs. Outperform adapter-style Qwen3-VL per Alibaba's
  Qwen3.5 Highlights spec. MLX ports verified on `mlx-community` 2026-04-22.
- **tier_mlx_moe_reap** — Cerebras REAP-pruned MoE VLMs (~40% experts removed,
  memory roughly halved so 4-bit MoE VLMs fit on Metal). Ranked above dense
  models of comparable quality when they ship. As of 2026-04-22 no
  `mlx-community` native 4-bit VLM REAP exists — only OpenMOSE BF16 + GGUF
  ports (`OpenMOSE/Qwen3-VL-REAP-24B-A3B`, `OpenMOSE/Qwen3-VL-REAP-145B-A22B`,
  both Apache-2.0). Re-check monthly; promote to the top of the chain when
  mlx-community publishes a native 4-bit port.
- **tier_mlx_dense** — dense 4-bit VLMs, ordered by 2025-Q4 quality.

## Capability matrix

"MLX port URL" is the exact `mlx-community/...` repo the runtime fetches.
Disk sizes come from HuggingFace model cards (verified 2026-04-22 for the
original six candidates; `InternVL3-38B`, `GLM-4.5V-9B`, `MiniCPM-V-4` are
listed from the reconciled yaml and marked availability-TBC where HF returned
`401` at probe time — the runtime will skip them and continue down the chain
if they are not downloadable).

| # | Tier | Model | Release | MLX port URL | Disk (4-bit) | VLM strengths |
|---|------|-------|---------|--------------|--------------|---------------|
| 0a | native-mm | Qwen3.6-35B-A3B       | 2026-Q2 | <https://huggingface.co/mlx-community/Qwen3.6-35B-A3B-4bit>        | ~18 GB     | **Natively multimodal (early-fusion).** 35B total / A3B active MoE. Image-Text-to-Text. Published ~2026-04-16. Outperforms Qwen3-VL per Alibaba's Qwen3.5 Highlights spec. Also available: `-5bit`, `-6bit`, `-8bit`, `-bf16`, `-nvfp4`, `-mxfp4`, `-mxfp8`, `-4bit-DWQ`, `-4.4bit-msq`. |
| 0b | native-mm | Qwen3.5-122B-A10B     | 2026-Q1 | <https://huggingface.co/mlx-community/Qwen3.5-122B-A10B-4bit>      | ~62 GB     | **Natively multimodal (early-fusion).** 122B total / A10B active MoE. Requires roughly >=96 GB unified memory; the cache-walk resolver skips it automatically on 16/32/64 GB hosts. Alibaba-official 2026-Q1 release. Also: `-5bit`, `-6bit`, `-8bit`, `-bf16`, `-heretic-v2-2.34bit-msq`. |
| 1 | adapter | Qwen3-VL-32B-Instruct     | 2025-Q4 | <https://huggingface.co/mlx-community/Qwen3-VL-32B-Instruct-4bit>  | 19.6 GB    | **Adapter-style VLM (not early-fusion).** Demoted from top slot 2026-04-22 per Alibaba Qwen3.5 Highlights; retained as adapter-VLM floor for hosts that cannot run the MoE ports above. Apache 2.0. Converted with mlx-vlm 0.3.4. |
| 2 | adapter | InternVL3-38B             | 2025-Q3 | <https://huggingface.co/mlx-community/InternVL3-38B-4bit>          | ~20 GB (TBC) | Best OCR at size on open benches. Availability in `mlx-community` TBC; skip on miss. |
| 3 | adapter | InternVL3-14B             | 2025-Q3 | <https://huggingface.co/mlx-community/InternVL3-14B-4bit>          | 8.94 GB    | 38B fallback; small footprint with strong OCR. Converted with mlx-vlm 0.1.25. |
| 4 | adapter | GLM-4.5V-9B               | 2025-Q3 | <https://huggingface.co/mlx-community/GLM-4.5V-9B-4bit>            | ~5 GB (TBC)  | Zhipu AI; underrated on UI / document VQA. Availability TBC; skip on miss. |
| 5 | adapter | MiniCPM-V-4               | 2025-Q2 | <https://huggingface.co/mlx-community/MiniCPM-V-4-4bit>            | ~5 GB (TBC)  | 8B fast OCR. HF returned 401 on 2026-04-22 probe; keep in chain and re-verify. |
| 6 | adapter | Gemma-3-27B-IT            | 2025-Q1 | <https://huggingface.co/mlx-community/gemma-3-27b-it-4bit>         | 16.8 GB    | 128K context window; native vision head. Converted with mlx-vlm 0.1.18. |
| 7 | adapter | Pixtral-12B               | 2024-Q3 | <https://huggingface.co/mlx-community/pixtral-12b-4bit>            | 7.14 GB    | Mistral's first VLM; small, fast, Apache 2.0. Converted with mlx-vlm 0.0.15. |
| 8 | adapter | Qwen2.5-VL-7B-Instruct    | 2024-Q1 | <https://huggingface.co/mlx-community/Qwen2.5-VL-7B-Instruct-4bit> | 5.64 GB    | Back-compat floor (see below). Converted with mlx-vlm 0.1.11. |

### Dropped from the original proposal

`mlx-community/Llama-4-Scout-17B-16E-Instruct-4bit` (61.1 GB, MoE 17B/109B,
converted with mlx-vlm 0.1.21) was removed on 2026-04-22 (agent ab6be8c9) as
obsolete vs. Qwen3-VL / InternVL3.5 / GLM-4.5V on every open bench we track.
It is not documented as a fallback and should not be re-added without
promoting a newer MoE VLM above it.

## REAP (Redundancy-Aware Expert Pruning) — 2026-04-22 catalog

REAP is Cerebras Research's 2025 router-weighted expert activation pruning
method for sparse-MoE models (see <https://www.cerebras.ai/blog/reap> and
<https://github.com/CerebrasResearch/reap>). Applied to an MoE VLM it drops
~40% of experts with marginal quality loss and halves on-disk / Metal-resident
memory — the reason 4-bit MoE VLMs like Qwen3-VL-30B-A3B suddenly fit on
Apple Silicon.

**Official Cerebras-published VLM REAPs (2026-04-22):** none. The cerebras/*
REAP catalog covers Qwen3-Coder, Step-3.5-Flash, MiniMax-M2.1/M2.5, and GLM-4.7
text-only MoEs.

**Community VLM REAPs found on HuggingFace (2026-04-22):**

| Model ID | Base | Pruning | Tensor | Notes |
|----------|------|---------|--------|-------|
| [`OpenMOSE/Qwen3-VL-REAP-24B-A3B`](https://huggingface.co/OpenMOSE/Qwen3-VL-REAP-24B-A3B) | Qwen3-VL-30B-A3B | ~40% experts, 30B→24B | BF16 | 2 quantized ports linked (GGUF). Apache-2.0. |
| [`OpenMOSE/Qwen3-VL-REAP-145B-A22B`](https://huggingface.co/OpenMOSE/Qwen3-VL-REAP-145B-A22B) | Qwen3-VL-235B-A22B | ~40% experts, 235B→145B | BF16 | GGUF port available. |
| [`atbender/Qwen3.6-VL-REAP-26B-A3B`](https://huggingface.co/atbender/Qwen3.6-VL-REAP-26B-A3B) | Qwen3.6-VL | REAP | BF16 + W4A16 variant | Most recent community VLM REAP found. |

No `mlx-community/*VL*REAP*-4bit` ports existed at probe time. The yaml
`tier_mlx_moe_reap` slot is therefore a reserved placeholder above the dense
chain, with a `TODO(vlm-judge): re-check REAP catalog monthly` marker in the
yaml. When (a) mlx-community publishes a native 4-bit VLM REAP, or (b) we
convert OpenMOSE/Qwen3-VL-REAP-24B-A3B ourselves via `mlx_vlm.convert`, the
resulting id slots in as the new top-of-chain entry.

No VLM REAPs exist in `cerebras/*` yet — re-check monthly at
<https://huggingface.co/cerebras>.

## Prompt design — zakelfassi borrow (2026-04-22)

The `BLIND_PROMPT` in `tools/vlm-judge/src/main.rs` was refreshed on
2026-04-22 against zakelfassi's "VLM visual-testing Chrome extension" post
(<https://zakelfassi.com/vlm-visual-testing-chrome-extension>). The post
describes a production deployment of VLM-as-visual-diff-oracle and its prompt
pattern translates cleanly to journey keyframes: (a) ask for concrete on-screen
elements in a tight sentence budget, (b) *explicitly* negate the common
failure modes ("This is NOT ..."). The old prompt asked for "2 sentences max"
and banned "placeholder"; the new prompt also bans "stub", "frame N",
"image N", "test image", and "no content" — the exact failure strings
observed in earlier vlm-judge runs against stub keyframes. See
`docs-site/research/imports-2026-04/zakelfassi-vlm-visual-testing.md` for the
full extraction and the two deferred follow-ups (structured JSON output with
confidence enum; between-frame delta pass).

## Why Qwen2.5-VL is the floor, not the default

Qwen2.5-VL-7B-Instruct was released in January 2025 (Qwen2-VL line; the 2.5
revision landed in Q1 2025). As of April 2026 it is approximately 13 months
old — old enough that three generations of open VLMs now beat it on every
public benchmark the team tracks (general VQA, OCR, document understanding,
grounded reasoning), and its base model lineage predates both the MoE wave
that Qwen3-VL is built on and the early-fusion natively-multimodal shift
that Qwen3.5 / Qwen3.6 represent. We keep it at the tail of the priority chain
because (a) every existing hwLedger dev box already has the ~5.6 GB weights
cached, so manifests regenerated against it stay reproducible; (b) it is the
smallest entry in the chain and therefore the safest fallback on 16 GB
Apple-Silicon hosts where the 2025 models will OOM; and (c) the blind journey
descriptions we cached during 2025-Q4 were produced by it, so it remains the
reference point when diffing old vs. new judge runs. New runs, however, should
resolve to `Qwen3.6-35B-A3B-4bit` (or, on >=96 GB hosts, `Qwen3.5-122B-A10B-4bit`)
whenever the host has the disk and RAM for them; Qwen3-VL-32B / InternVL3-38B /
InternVL3-14B remain as adapter-VLM fallbacks — hence "floor, not default".

## Fireworks provider — natively-multimodal routing

The `fireworks` provider's `models.vlm` field is read as an ordered sequence
(the current resolver pins the first entry; future resolver work will walk
the list end-to-end). As of 2026-04-22 the catalog contains, in priority
order:

| Rank | Model id | Architecture | Vision |
|------|----------|--------------|--------|
| 1 | `accounts/fireworks/models/qwen3p6-plus` | Qwen3.6 natively-multimodal (early-fusion) | Yes |
| 2 | `accounts/fireworks/models/qwen3p5-122b-a10b` | Qwen3.5 natively-multimodal MoE (122B / A10B) | Yes |
| 3 | `accounts/fireworks/models/kimi-k2p6` | Kimi K2.6 | Yes |
| 4 | `accounts/fireworks/models/kimi-k2p5` | Kimi K2.5 (back-compat) | Yes |

`qwen3p5-9b`, `qwen3p5-35b-a3b`, and `qwen3p5-27b` also appear in the Fireworks
catalog but are text-only LLM entries and are intentionally excluded from the
VLM chain. `kimi-k2-turbo` remains the text-only default (it is not listed
with a vision capability flag in the Fireworks catalog).

## Runtime contract

- Config: `docs/examples/api-providers.yaml`, `providers.mlx.models.vlm` is a
  YAML sequence. Drop a copy at `~/.hwledger/api-providers.yaml` to override
  per-host.
- Resolver: `tools/vlm-judge/src/main.rs::pick_mlx_vlm_model()`. The compiled
  `MLX_VLM_PRIORITY` constant is kept in sync with the yaml by convention.
  Unit-tested against a hermetic `HUGGINGFACE_HUB_CACHE` override.
- Override precedence (strongest first):
  1. `--mlx-vlm-model <id>` (CLI flag).
  2. `HWLEDGER_MLX_VLM_MODEL=<id>` (env var).
  3. First entry from `MLX_VLM_PRIORITY` that is already cached on disk.
  4. Top of `MLX_VLM_PRIORITY` (triggers download).
  5. Legacy `DEFAULT_MLX_MODEL` constant (Qwen2.5-VL-7B; only reached if the
     priority list is empty, which is a compile-time invariant).
