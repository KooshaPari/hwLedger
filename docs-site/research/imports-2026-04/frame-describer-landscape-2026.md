# Frame-Describer Landscape 2026 — Omni + UI-Specialist Two-Stage Architecture

## Provenance

- **Authored:** 2026-04-22
- **Scope:** Re-scope of `tools/vlm-judge` from "VLM judge" framing (2024-era) to
  a **two-stage frame-describer** (2026 framing) composed of a UI-grounded
  parser and an omni-modal describer.
- **Trigger:** User note that the "VLM" category is 2024 framing; the 2026
  norm is **omni-modal** (vision + audio + text natively fused), and that for
  hwLedger's workload — keyframe → described-action — **domain-specific UI
  models beat generalist omni-models**.
- **Sibling docs:** [`zakelfassi-vlm-visual-testing.md`](./zakelfassi-vlm-visual-testing.md)
  (prompt design), [`ar-vs-diffusion.md`](./ar-vs-diffusion.md),
  [ADR-0015](../../architecture/adrs/0015-vlm-judge-claude.md),
  [ADR-0038 — Frame-Describer Two-Stage](../../architecture/adrs/0038-frame-describer-two-stage.md).
- **Live-evidence sources consulted 2026-04-22:** HuggingFace search (`bytedance-research`,
  `ByteDance-Seed`, `mlx-community`, `microsoft`, `apple`, `Qwen`, `openbmb`,
  `zai-org`), `ai.google.dev/gemma`, `machinelearning.apple.com/research`,
  `blog.google`, `arxiv.org`, `infoq.com`. All claims in this doc either link
  to a concrete source or are explicitly labelled as inference from public
  release patterns.

## 1. Why rename: `vlm-judge` → `frame-describer`

The crate name `vlm-judge` bakes two obsolete assumptions into our
architecture:

1. **"VLM" is a 2024 category.** By 2026, the frontier model release pattern
   is *natively multimodal* — vision, audio, and text are fused at pretraining
   time rather than stitched through an adapter. Alibaba, Google (Gemma 4),
   OpenBMB (MiniCPM-o 4.5), ByteDance (Seed 1.6), and Qwen3-Omni all ship
   early-fusion omni models as the default public endpoint. "VLM" now
   describes a *legacy adapter-style architecture* (Qwen2.5-VL, Pixtral,
   InternVL3), not a capability tier. Continuing to call our crate a "VLM
   judge" implies we are fishing for the adapter architecture, which is the
   opposite of what we want.

2. **"Judge" implies a single generative call.** Our actual workload is a
   **pipeline** — keyframe in, grounded structured description out — and the
   2026 SOTA for screenshots is a two-stage parser + describer, not a
   one-shot caption. "Frame-describer" describes the pipeline's actual job.

The rename to `tools/frame-describer/` (plus crate rename in
`Cargo.toml` members) is **tracked as a follow-up commit**, not done here;
this doc + ADR 0038 + the `providers.frame_describer:` YAML block record the
decision and the chain so the rename is mechanical.

## 2. Two-stage architecture

```
                 ┌──────────────────────────────────────────┐
                 │  Stage 1: PARSER (domain-specific)       │
                 │  screenshot → bbox-grounded element list │
                 │  structural, non-generative, low halluc. │
                 └──────────────────────────────────────────┘
                                    │
                                    ▼ JSON {elements:[{bbox,text,role,...}]}
                 ┌──────────────────────────────────────────┐
                 │  Stage 2: DESCRIBER (omni-modal + UI-LM) │
                 │  elements + raw frame + intent →         │
                 │  grounded blind description + score      │
                 └──────────────────────────────────────────┘
                                    │
                                    ▼
                        {description, confidence, agreement, bboxes}
```

**Why this beats a single omni call:**
- The parser's output is **addressable** (each element has a `bbox`), so the
  describer can say *"clicks the `Record` button at bbox(842,112,920,140)"*
  instead of the 2024-era flowery *"the user appears to be interacting with
  a recording control in the toolbar region."*
- The parser's output is **deterministic** enough to hash and attest — we
  can cache Stage 1 by keyframe content-hash and re-run Stage 2 whenever
  the describer chain changes.
- When the describer hallucinates a UI element that isn't in the parser
  output, we have a **programmatic contradiction check**, not a vibe check.
- Omni models are excellent at fluent description but systematically weak at
  pixel-coordinate OCR on dense UIs (see
  [`zakelfassi-vlm-visual-testing.md`](./zakelfassi-vlm-visual-testing.md)
  for prior evidence on this class of failure). UI-specialist parsers are
  trained on exactly that distribution.

## 3. Full landscape table (25+ models)

| Model | Release | Org | Category | Params | License | OCRBench | ScreenSpot | MLX | Fireworks | OpenRouter free | Notes |
|-------|---------|-----|----------|--------|---------|----------|------------|-----|-----------|-----------------|-------|
| **UI-TARS-2 (DPO)** | 2025-09 | ByteDance | ui-specialist | 2B / 7B / 72B | model-terms | n/a | 94.2 (V2) | **unofficial** (TARS-1.5 has MLX; TARS-2 MLX not yet on mlx-community as of 2026-04-22) | no | no | SOTA GUI agent; OSWorld 47.5 |
| **UI-TARS-1.5** | 2025-04 | ByteDance | ui-specialist | 7B | model-terms | n/a | 89 | **yes** (`mlx-community/UI-TARS-1.5-7B-4bit`, `-6bit`, `portalAI/…-bf16`) | no | no | native MLX port via mlx-vlm 0.1.25 |
| **UI-TARS (original)** | 2024-12 | ByteDance | ui-specialist | 2B / 7B / 72B | model-terms | n/a | 82 | partial | no | no | SFT + DPO checkpoints public |
| **OmniParser-v2.0** | 2025-02 | Microsoft | ui-parser | 0.3B (YOLOv8 + Florence-2) | MIT | n/a | n/a (pure parser) | **no dedicated MLX port** — CPU/CUDA only. Can run on Apple Silicon via PyTorch MPS fallback (Codersera guide) | no | no | **best parser 2026**; 60% faster than v1 |
| **Ferret-UI 2** | 2024-10 paper / 2025-Q1 models | Apple | ui-specialist | 2B (Gemma-2B) / 8B (Llama-3-8B) | Apple research | n/a | strong cross-platform | likely (Apple authorship; MLX port not yet published under `mlx-community`) | no | no | iPhone/Android/iPad/Web/AppleTV |
| **Ferret-UI Lite** | 2025-09 | Apple | ui-specialist | 0.5B-1B est. | Apple research | n/a | n/a | likely | no | no | on-device focus; smaller-is-smarter follow-up |
| **SeeClick** | 2024-01 | Tsinghua (NJU) | ui-specialist | 9.6B (Qwen-VL base) | ACL 2024 | n/a | 53-71 by region | no | no | no | Historical anchor; superseded by UI-TARS |
| **ShowUI-2B** | 2024-11 | Show Lab (NTU) | ui-specialist | 2B | MIT-ish | n/a | 75 zero-shot | community MLX effort exists (unverified) | no | no | Smallest serious UI model |
| **CogAgent-9b-20241220** | 2024-12 | Zhipu AI (THU) | ui-specialist | 9B | Apache-2.0 (variant) | n/a | SOTA late-2024 | no | no | no | |
| **Aguvis** | 2024-12 → 2025-Q1 | UIUC / Salesforce | ui-specialist | 7B / 72B | research | n/a | 59-84 | no | no | no | Unified pure-vision GUI agent |
| **OS-Atlas** | 2024-10 | UNSW / Shanghai AI Lab | ui-specialist | 4B / 7B | research | n/a | SOTA on multi-OS | no | no | no | Cross-platform GUI grounding |
| **Claude Computer Use** | 2024-10 → 2025 | Anthropic | ui-specialist (closed) | n/a | proprietary | n/a | n/a | n/a | no | no | Policy-blocked per `api-providers.yaml` |
| **Gemma 4 31B Dense** | 2026-04-02 | Google DeepMind | omni | 31B | **Apache-2.0** | very high (claimed pareto frontier) | n/a | pending (Gemma 3 already on mlx-community; Gemma 4 MLX port likely within days) | no (not yet in FW catalog) | likely (Gemma 3 already free-tiered) | **Text + image + video + OCR; E2B/E4B also do audio** |
| **Gemma 4 26B MoE** | 2026-04-02 | Google DeepMind | omni | 26B total | Apache-2.0 | high | n/a | pending | no | likely | context up to 256K |
| **Gemma 4 E4B** | 2026-04-02 | Google DeepMind | omni (+audio) | 4B | Apache-2.0 | high-for-size | n/a | pending | no | likely | on-device E-tier; native audio input |
| **Gemma 4 E2B** | 2026-04-02 | Google DeepMind | omni (+audio) | 2B | Apache-2.0 | strong-for-size | n/a | pending | no | likely | phone/edge |
| **Qwen3.6-plus / A3B MoE** | 2026-Q1 | Alibaba | omni (native early-fusion) | 35B/A3B | Apache-2.0 | high | strong | **yes** (`mlx-community/Qwen3.6-35B-A3B-4bit`) | **yes** (`qwen3p6-plus`) | varies | already in our chain |
| **Qwen3.5-122B-A10B** | 2026-Q1 | Alibaba | omni (native early-fusion) | 122B/A10B | Apache-2.0 | very high | strong | **yes** (`mlx-community/Qwen3.5-122B-A10B-4bit`) | yes | no | needs ≥96 GB RAM |
| **Qwen3-Omni** | 2025-Q4 | Alibaba | omni | 7B / 30B | Apache-2.0 | high | strong | partial | yes | yes | natively end-to-end omni (text/audio/image/video + speech out) |
| **Qwen2.5-Omni-7B** | 2025-Q1 | Alibaba | omni | 7B | Apache-2.0 | strong | ok | yes | yes | yes | Thinker-Talker; streaming speech |
| **Qwen3-VL-32B** | 2025-Q4 | Alibaba | vlm (adapter) | 32B | Apache-2.0 | high | strong | yes (`mlx-community/Qwen3-VL-32B-Instruct-4bit`) | varies | no | **adapter-VLM floor** per our chain |
| **Qwen2.5-VL-7B** | 2024-Q1 | Alibaba | vlm (adapter) | 7B | Apache-2.0 | ok | ok | yes | yes | yes (`qwen2.5-vl-72b-instruct:free` at 72B) | back-compat anchor |
| **MiniCPM-o 4.5** | 2025-08 | OpenBMB (Tsinghua) | omni | 9B (SigLip2+Whisper+CosyVoice2+Qwen3-8B) | MIT-ish | high | ok | likely soon | no | no | Gemini 2.5 Flash-level claim; 77.6 OpenCompass |
| **MiniCPM-o 2.6** | 2025-02 | OpenBMB | omni | 8B | MIT-ish | strong | ok | yes (mlx-vlm) | no | no | vision + audio + speech out |
| **MiniCPM-V-4** | 2025-Q2 | OpenBMB | vlm | 8B | MIT-ish | strong for size | ok | yes (`mlx-community/MiniCPM-V-4-4bit`) | no | no | ~5 GB, fast OCR — already in our chain |
| **GLM-4.5V-9B** | 2025-Q3 | Zhipu AI | vlm | 9B | permissive | high | strong on UI/doc | yes (availability TBC) | no | no | Underrated on UI VQA |
| **GLM-4.5-Omni** | 2025-Q4 (inferred) | Zhipu AI | omni | 9B-ish | permissive | — | — | tbd | no | no | Successor to GLM-4.5V; omni variant |
| **Baichuan-Omni-2** | 2025-Q3 | Baichuan | omni | 8B-ish | research | — | — | no | no | no | audio+vision+text; underrepresented on mlx-community |
| **Aria-3.9B-A** | 2024-10 | Rhymes AI | omni MoE | 3.9B active / 24.9B total | Apache-2.0 | ok | — | limited | no | no | long context; native multimodal |
| **Unified-IO 3** | 2025-late (inferred) | AI2 | true-unified | — | Apache-2.0 | — | — | no | no | no | True unified modality (text/image/audio/video encode & decode) |
| **InternVL3-38B** | 2025-Q3 | OpenGVLab | vlm | 38B | Apache-2.0 | **SOTA OCR at size** | strong | yes (`mlx-community/InternVL3-38B-4bit`) | no | no | already in our chain |
| **InternVL3-14B** | 2025-Q3 | OpenGVLab | vlm | 14B | Apache-2.0 | high | strong | yes (`mlx-community/InternVL3-14B-4bit`) | no | no | mid-tier fallback |
| **Pixtral-12B** | 2024-Q3 | Mistral | vlm | 12B | Apache-2.0 | ok | ok | yes (`mlx-community/pixtral-12b-4bit`) | no | no | Apache floor |
| **Kimi K2.6** | 2026-Q1 | Moonshot | vision (under investigation) | 32B+MoE | proprietary | — | — | no | **yes** (`kimi-k2p6`) | no | Vision capability per FW catalog entry; verify on actual journey frame in follow-up |
| **Kimi K2.5** | 2025-Q4 | Moonshot | vision | — | proprietary | — | — | no | **yes** (`kimi-k2p5`) | no | K1.5 had vision; K2.0 was text-only; K2.5/K2.6 restored vision |

Columns marked `—` are unverified at the time of writing; treat as gaps to
close in the next research pass rather than as claims of absence.

## 4. Gemma 4 deep-dive (2026-04-02 release — CONFIRMED)

Gemma 4 shipped on 2026-04-02 under **Apache-2.0** (a meaningful upgrade
from the Gemma-specific "Gemma Prohibited Use Policy" license that shipped
with Gemma 1-3; Gemma 4 is the first Gemma release under an unmodified OSI
licence per the InfoQ coverage and the HF blog). Four variants:

| Variant | Params | Modality | Target |
|---------|--------|----------|--------|
| Gemma 4 E2B | 2B | text + image + video + OCR + **audio** | phone, edge |
| Gemma 4 E4B | 4B | text + image + video + OCR + **audio** | laptop, consumer GPU |
| Gemma 4 26B MoE | 26B | text + image + video + OCR | workstation |
| Gemma 4 31B Dense | 31B | text + image + video + OCR | server-class |

Key facts for our purposes:

- **Natively multimodal across all sizes** — text + image + video + OCR.
  E-tier variants additionally accept native audio.
- **Context 256K tokens.**
- **Claimed pareto-frontier arena scores** (Google positions the 31B Dense
  against 400B-class proprietary models; take with Google-marketing-salt and
  validate on our own keyframes).
- **Apache-2.0** — this is the first Gemma under a true OSI license and
  removes the friction that kept us from redistributing Gemma-bundled
  derivatives (see ADR-0015 prior constraint discussion).
- **MLX port status (2026-04-22):** Gemma 3-27B is already on
  `mlx-community/gemma-3-27b-it-4bit` (in our chain). A Gemma 4 port is
  pending; historically mlx-community turns Gemma releases around inside
  2-5 days. **Action: monitor `mlx-community/gemma-4-*-4bit`.**

**Decision for hwLedger:** once `mlx-community/gemma-4-31b-4bit` (or
`gemma-4-26b-moe-4bit`) publishes, insert it as a **describer** tier above
Qwen3-VL-32B but below Qwen3.5/3.6 and UI-TARS-1.5. Do **not** promote Gemma 4
above the UI-specialists — generalist omni still loses to UI-grounded
specialists on our exact workload (screenshot → action description).

## 5. UI-specialist section (primary for our workload)

### 5.1 UI-TARS family (ByteDance)

**The most important family for hwLedger.** UI-TARS is trained
end-to-end on screenshot→action pairs, which is — exactly — our workload.

| Checkpoint | Date | Sizes | MLX | Notes |
|------------|------|-------|-----|-------|
| UI-TARS | 2024-12 | 2B/7B/72B SFT + DPO | partial | v1 baseline |
| UI-TARS-1.5 | 2025-04 | 7B | **yes** (mlx-community 4bit + 6bit; portalAI bf16) | mlx-vlm 0.1.25 conversion; RL-tuned reasoning |
| **UI-TARS-2** | **2025-09** | 2B/7B/72B SFT + DPO | **not on mlx-community yet** (as of 2026-04-22; bytedance/UI-TARS#193 tracks MLX support request) | OSWorld 47.5, WindowsAgentArena 50.6, AndroidWorld 73.3 |

**Critical for our recommendation:** UI-TARS-2-7B-DPO exists but is NOT yet on
`mlx-community`. Therefore **UI-TARS-1.5-7B-4bit is our current MLX
describer of choice**, with UI-TARS-2 pinned as the upgrade target.

### 5.2 OmniParser v2 (Microsoft)

**The parser, not a describer.** OmniParser v2 is:
- A fine-tuned **YOLOv8** for UI element detection (bbox + class).
- A fine-tuned **Florence-2** for per-region semantic labelling.

It is **not a VLM** — it's a detection + labelling pipeline. Output is
strictly structural: `{elements: [{bbox, label, text?, interactable?}]}`.

**MLX status:** no dedicated MLX port as of 2026-04-22. Runs on Apple
Silicon via PyTorch MPS fallback (see the Codersera walkthrough), which is
slow but viable. **Workaround for Stage 1:** either (a) run OmniParser v2
under MPS and accept the 2-4x latency penalty, or (b) drop to
`mlx-community/paddleocr-vl.swift` (native Swift + MLX port of PaddleOCR-VL,
0.9B, which gives us OCR + table/chart + layout on MLX, losing the
element-interactability labels).

**Recommendation:** use OmniParser-v2 under MPS as the canonical Stage 1,
and document paddleocr-vl.swift as the "fast path" for CI where latency
matters more than interactability labels.

### 5.3 Ferret-UI 2 (Apple)

Cross-platform (iPhone / Android / iPad / Web / AppleTV) UI grounding with
referring expressions — *"tap the second-from-top blue button in the
toolbar"* parses to a bbox. Because Apple authored it, an MLX port is
likely eventually, but as of 2026-04-22 nothing is published under
`mlx-community/ferret-ui-2-*`. **Ferret-UI Lite** (2025-09) is the
on-device-focused follow-up and is probably the MLX-relevant variant when
the port lands.

### 5.4 SeeClick / ShowUI / CogAgent / Aguvis / OS-Atlas

These are research-grade checkpoints that predate UI-TARS-2. Use only as
fallbacks if UI-TARS-2 and UI-TARS-1.5 are both unavailable:

- **ShowUI-2B** — smallest useful model; 75% zero-shot ScreenSpot.
- **OS-Atlas-7B** — strongest cross-OS grounding pre-UI-TARS.
- **CogAgent-9b-20241220** — Zhipu's 2024 flagship; superseded by GLM-4.5V.
- **Aguvis** — Salesforce/UIUC, 7B/72B; research licence.
- **SeeClick** — historical anchor.

## 6. Omni-modal section (secondary)

Omni-modal models are our **describer fallback** when UI-specialists are
unavailable or the keyframe content leaves the pure-UI distribution (e.g.,
terminal recordings, video embeds, media playback frames).

| Model | Modalities | MLX | Notes |
|-------|------------|-----|-------|
| Qwen3.6 / Qwen3.5 (native early-fusion) | text + image + video | yes | already in our chain; Alibaba claims cross-generational parity with Qwen3 and outperforms Qwen3-VL |
| Qwen3-Omni | text + image + video + audio (+speech out) | partial | natively end-to-end omni |
| Qwen2.5-Omni-7B | text + image + audio + video + speech out | yes | Thinker-Talker arch; streaming speech |
| MiniCPM-o 4.5 | text + image + audio + video + speech | likely soon | 77.6 OpenCompass; Gemini 2.5 Flash-level |
| GLM-4.5V / GLM-4.5-Omni | text + image (+audio in Omni variant) | unknown | underrated on UI/doc VQA |
| Baichuan-Omni-2 | text + image + audio | no | good but mlx-coverage weak |
| Aria (Rhymes) | text + image + audio + video | limited | MoE 3.9B active / 24.9B total |
| Unified-IO 3 (AI2) | true unified (generate any modality) | no | research showcase |

## 7. Prompt templates for the two-stage pipeline

### 7.1 Stage 1 (parser — OmniParser-v2)

No prompt — OmniParser returns structured JSON directly:

```json
{
  "elements": [
    { "id": 0, "bbox": [842,112,920,140], "label": "button", "text": "Record", "interactable": true },
    { "id": 1, "bbox": [  8, 48,  56,  80], "label": "logo",   "text": "",       "interactable": false },
    { "id": 2, "bbox": [ 96,200, 480, 232], "label": "text",   "text": "Scene 1 — first plan demo", "interactable": false }
  ]
}
```

### 7.2 Stage 2 (describer — UI-TARS-1.5 / Qwen3.5-omni)

```
You are describing a single keyframe from a recorded journey.

The frame contains these parsed UI elements (authoritative — trust these
over your own visual reading when in doubt):

{stage1_elements_json}

Rules:
- Describe ONE user-visible action in ≤2 sentences.
- Reference elements by their text or role when possible; include bbox id
  in parentheses the first time you name an element, e.g. "the Record
  button (#0)".
- Do NOT mention elements that are not in the list above.
- Do NOT speculate about state changes that happened before this frame.
- If the frame is mid-animation or partial, return confidence:"low" and
  describe what is unambiguously visible.

Return ONLY this JSON (no prose):
{
  "action": "<one sentence>",
  "referenced_element_ids": [<int>, ...],
  "confidence": "high" | "medium" | "low",
  "notes": "<optional, ≤1 sentence>"
}
```

The `confidence` enum is borrowed from zakelfassi (see sibling research doc).
The `referenced_element_ids` field gives us a programmatic check: if the
describer references an id not present in Stage 1 output, we can flag the
frame for re-render **without** reading the prose.

## 8. Recommended chain for hwLedger

Two chains — one for the parser, one for the describer — both priority-walked
top-down, first cached-or-installable wins:

### 8.1 Parser chain (`providers.frame_describer.parser:`)

1. **`microsoft/OmniParser-v2.0` (PyTorch MPS)** — canonical parser, dedicated
   to the task. Pay the MPS-fallback latency on Apple Silicon until a
   native MLX port ships.
2. **`mlx-community/paddleocr-vl.swift`** — native Swift + MLX; faster on
   Apple Silicon, loses `interactable` labels. Use when latency dominates.
3. **describer-only mode** — if neither parser is available, skip Stage 1,
   hand the raw frame directly to the describer, and accept that the
   output loses bbox grounding.

### 8.2 Describer chain (`providers.frame_describer.describer:`)

1. **`mlx-community/UI-TARS-1.5-7B-6bit`** — UI-grounded specialist on MLX,
   trained on exactly our workload (screenshot→action pairs). *Preferred.*
2. **`mlx-community/UI-TARS-1.5-7B-4bit`** — smaller-quant fallback when RAM
   is tight.
3. **`mlx-community/UI-TARS-2-7B-DPO-4bit`** — *insert when mlx-community
   publishes the UI-TARS-2 MLX port.* Will displace UI-TARS-1.5 immediately.
4. **`mlx-community/Qwen3.6-35B-A3B-4bit`** — natively-multimodal omni
   fallback when UI-TARS can't be loaded.
5. **`mlx-community/Qwen3-VL-32B-Instruct-4bit`** — adapter-VLM floor
   (already in our current chain).
6. **`mlx-community/MiniCPM-o-4_5` (when ported)** / **`MiniCPM-V-4-4bit`** —
   cheap on-device fallback with strong OCR.
7. **`mlx-community/Qwen2.5-VL-7B-Instruct-4bit`** — back-compat anchor only.

### 8.3 Cloud / subscription chain (no API key spend)

1. `fireworks: accounts/fireworks/models/qwen3p6-plus` (unlimited plan).
2. `fireworks: accounts/fireworks/models/kimi-k2p6` (if vision confirmed).
3. `openrouter: qwen/qwen2.5-vl-72b-instruct:free`.
4. `claude_code_headless` (subscription CLI, text-only image handling;
   reasoning-tier fallback after a cheaper describer already produced the
   blind description).

## 9. Open questions + next steps

1. **Live bench on our own keyframes.** Every claim in the table is either
   sourced or inferred from release patterns; we do not yet have head-to-head
   numbers on hwLedger's own journey corpus. Next commit: add
   `tools/frame-describer/benches/keyframe_corpus.rs` that runs the parser +
   describer chain over our `public/cli-journeys/**/*.png` fixtures and
   records per-model agreement scores.
2. **UI-TARS-2 MLX port.** Track bytedance/UI-TARS#193. When
   `mlx-community/UI-TARS-2-7B-DPO-4bit` publishes, promote it to position 1
   of the describer chain and demote UI-TARS-1.5 to position 2.
3. **OmniParser MLX port.** Either Microsoft publishes one or we fund a
   community effort. In the interim, benchmark `paddleocr-vl.swift` against
   OmniParser-v2-MPS on our corpus to decide whether the speed tradeoff is
   worth the label-quality loss.
4. **Gemma 4 MLX port.** Monitor `mlx-community/gemma-4-*-4bit` weekly; when
   it lands, benchmark Gemma 4 31B Dense as describer and decide placement
   relative to Qwen3.5/3.6.
5. **Kimi K2.6 vision confirmation.** Our Fireworks catalog comment
   asserts K2.6 has vision; confirm with a live call returning a
   `content=[{type:image,...}]` payload before we rely on it in production.
6. **Confidence enum propagation.** Stage 2's `confidence` field needs a
   matching manifest schema bump (`FR-JOURNEY-006` / `FR-JOURNEY-008`) plus
   an agreement-scorer update — deferred per sibling research doc.
7. **Between-frame delta pass.** zakelfassi's *visual diff oracle* pattern
   (two frames → delta summary) is a natural Stage 3 extension but requires
   a manifest schema change; track as a follow-up ADR.

## 10. Sources

- Gemma 4 release (2026-04-02): https://huggingface.co/blog/gemma4 ;
  https://deepmind.google/models/gemma/gemma-4/ ; https://blog.google/innovation-and-ai/technology/developers-tools/gemma-4/ ;
  https://www.infoq.com/news/2026/04/google-gemm4/ ; https://ai.google.dev/gemma/docs/core/model_card_4 .
- UI-TARS-1.5 MLX: https://huggingface.co/mlx-community/UI-TARS-1.5-7B-4bit ;
  https://huggingface.co/mlx-community/UI-TARS-1.5-7B-6bit ;
  https://huggingface.co/portalAI/UI-TARS-1.5-7B-mlx-bf16 .
- UI-TARS source: https://huggingface.co/ByteDance-Seed/UI-TARS-1.5-7B ;
  https://github.com/bytedance/UI-TARS ; MLX-support request: https://github.com/bytedance/UI-TARS/issues/193 .
- UI-TARS-2 (2025-09): https://huggingface.co/papers/2509.02544 ;
  https://huggingface.co/ByteDance-Seed/UI-TARS-7B-SFT ; https://huggingface.co/ByteDance-Seed/UI-TARS-72B-DPO .
- OmniParser v2: https://github.com/microsoft/OmniParser ;
  https://huggingface.co/microsoft/OmniParser-v2.0 ;
  https://www.microsoft.com/en-us/research/articles/omniparser-v2-turning-any-llm-into-a-computer-use-agent/ ;
  macOS guide: https://codersera.com/blog/run-microsoft-omniparser-v2-on-macos-step-by-step-installation-guide .
- paddleocr-vl.swift: https://github.com/mlx-community/paddleocr-vl.swift .
- Ferret-UI 2: https://machinelearning.apple.com/research/ferret-ui-2 ;
  https://huggingface.co/papers/2410.18967 ; Ferret-UI Lite: https://huggingface.co/papers/2509.26539 .
- ShowUI / OS-Atlas / Aguvis / CogAgent: https://github.com/showlab/Awesome-GUI-Agent ;
  https://arxiv.org/html/2410.23218v1 (OS-Atlas) ; https://github.com/zai-org/CogAgent ;
  https://arxiv.org/html/2504.07981v1 (ScreenSpot-Pro).
- SeeClick: https://huggingface.co/papers/2401.10935 ; https://github.com/njucckevin/SeeClick .
- Qwen2.5-Omni: https://github.com/QwenLM/Qwen2.5-Omni ; https://huggingface.co/Qwen/Qwen2.5-Omni-7B .
- Qwen3-Omni: https://github.com/QwenLM/Qwen3-Omni .
- MiniCPM-o 4.5: https://github.com/OpenBMB/MiniCPM-o ; https://huggingface.co/openbmb/MiniCPM-o-4_5 .
- Sibling hwLedger docs: [`zakelfassi-vlm-visual-testing.md`](./zakelfassi-vlm-visual-testing.md) ;
  [`ADR-0015`](../../architecture/adrs/0015-vlm-judge-claude.md) ;
  [`ADR-0038`](../../architecture/adrs/0038-frame-describer-two-stage.md).
