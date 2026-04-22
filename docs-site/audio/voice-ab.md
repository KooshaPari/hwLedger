---
title: Narration Voice A/B — Taste Test
---

# Narration Voice A/B — Taste Test

Press play on the eight clips below, then glance at the spec tables to
confirm what you heard. Each clip reads the **same ~40-second script** from
the `plan-deepseek` walkthrough so the comparison is on identical material.

> **Piper is tier-5 fallback only — IndexTTS and Kokoro both sound better
> and are just as offline.** Piper is kept in the chain strictly for
> headless Linux CI runners that can't install `torch`/`onnxruntime`.
> On any workstation with GPU (MPS or CUDA) the auto-selector now picks
> IndexTTS 2.0 first, then Kokoro-82M, then KittenTTS, then AVSpeech
> (macOS) — and only falls through to Piper when nothing else is
> installable. See ADR-0010 v2.

## The script (identical for all eight)

> You start with a fresh shell and install hwLedger via cargo. The install
> prints the resolved version plus the path to the binary, which you verify
> with dash-dash-version. Next, you open the planner help to see the full
> flag list. Two flags matter most for DeepSeek: dash-dash-context and
> dash-dash-batch. With those set, the planner runs, detects the MLA
> attention kind automatically, and breaks down memory across weights, KV
> cache, and prefill activations. On an H100, DeepSeek-V3 at 32K context
> needs roughly three hundred and sixty gigabytes in total.

---

## Offline local voices

Run fully on the self-hosted macOS runner (or any dev laptop with a GPU).
No cloud, no API key, no network egress.

### D — IndexTTS 2.0 (zero-shot, default speaker prompt, MPS)

<audio controls preload="metadata" src="/audio/voice-ab/d-indextts.wav"></audio>

*IndexTeam/IndexTTS-2 autoregressive zero-shot model. Rendered on Apple
MPS with the default speaker prompt extracted from the upstream demo
video (no user-supplied reference clip). Markedly more natural than
Piper — matches the offline cloud class on expressiveness and pacing.*

### E — Kokoro-82M (kokoro-onnx, af_heart voice)

<audio controls preload="metadata" src="/audio/voice-ab/e-kokoro.wav"></audio>

*82M-param ONNX StyleTTS-2 derivative. Pure CPU ONNX runtime, no torch.
Tier-2 offline: close to IndexTTS quality at a fraction of the install
footprint and realtime factor.*

### F — KittenTTS nano (expr-voice-5-f)

<audio controls preload="metadata" src="/audio/voice-ab/f-kittentts.wav"></audio>

*KittenML 0.1 nano ONNX model. Smallest offline engine we tested (~25 MB
weights). Usable on devices where Kokoro and IndexTTS don't fit.*

### H — Piper / Lessac (tier-5 CI fallback)

<audio controls preload="metadata" src="/audio/voice-ab/h-piper-lessac.wav"></audio>

*Piper VITS medium quality voice. **Kept strictly for headless Linux CI
runners** that can't install torch/onnxruntime. Fast-but-robotic; outranked
by every other offline option above for non-CI renders.*

### Offline spec table

| Voice | License | Offline | GPU? | Install size | Realtime factor | One-line note |
| --- | --- | --- | --- | --- | --- | --- |
| **IndexTTS 2.0** (`d-indextts`) | Apache-2.0 | ✅ | MPS/CUDA (CPU slow) | ~5.5 GB weights + 1.3 GB deps | **13.5× on MPS** (506 s for 37 s audio) | Best offline quality; zero-shot voice cloning, expressive, slow without GPU |
| **Kokoro-82M** (`e-kokoro`) | Apache-2.0 | ✅ | CPU-ok, optional GPU | ~330 MB weights + onnxruntime | **~0.3×** on M-series CPU | Best install-size/quality ratio; pure ONNX, no torch |
| **KittenTTS nano** (`f-kittentts`) | MIT | ✅ | CPU-only | ~25 MB weights | **~0.4×** on CPU | Tiny; good for resource-constrained devices, below Kokoro on prosody |
| **Piper Lessac** (`h-piper-lessac`) | MIT | ✅ | CPU-only | ~60 MB weights | **~0.05×** on CPU | Fast, robotic; tier-5 CI fallback only |

---

## Online free-tier voices

Microsoft edge-tts — Clipchamp's underlying endpoint. Free, undocumented,
ToS-bound. Explicit opt-in only (`HWLEDGER_VOICE=edge-tts`); the auto
selector never picks these. All four are direct reference renders of the
Clipchamp default voice family.

### A — edge-tts / Aria (News voice, female)

<audio controls preload="metadata" src="/audio/voice-ab/a-edge-aria.wav"></audio>

*Clear female newscaster, bright confident delivery. Microsoft's flagship
News voice.*

### B — edge-tts / Ava Multilingual (Conversational, female)

<audio controls preload="metadata" src="/audio/voice-ab/b-edge-ava.wav"></audio>

*Warmer, more conversational female voice. Microsoft's Copilot voice —
closer to a podcast host than a newsreader.*

### C — edge-tts / Christopher (News voice, male)

<audio controls preload="metadata" src="/audio/voice-ab/c-edge-christopher.wav"></audio>

*Authoritative male newsreader. News/Novel category — tagged-closest
substitute for the (withdrawn) Davis voice.*

### I — edge-tts / Andrew Multilingual (Copilot authoritative, male)

<audio controls preload="metadata" src="/audio/voice-ab/i-edge-andrew.wav"></audio>

*The Clipchamp authoritative male. Warm, Confident, Authentic, Honest —
Conversation/Copilot cadence rather than News cadence.*

### Online spec table

| Voice | License | Offline | GPU? | Install size | Realtime factor | One-line note |
| --- | --- | --- | --- | --- | --- | --- |
| **edge-tts Aria** (`a-edge-aria`) | Microsoft Edge TTS ToS | ❌ cloud | n/a | `pip install edge-tts` (~5 MB) | ~0.2× wall-clock | Flagship News female; closest Clipchamp baseline |
| **edge-tts Ava** (`b-edge-ava`) | Microsoft Edge TTS ToS | ❌ cloud | n/a | `pip install edge-tts` | ~0.2× | Conversational female; warmer than Aria |
| **edge-tts Christopher** (`c-edge-christopher`) | Microsoft Edge TTS ToS | ❌ cloud | n/a | `pip install edge-tts` | ~0.2× | Authoritative News male; Davis substitute |
| **edge-tts Andrew** (`i-edge-andrew`) | Microsoft Edge TTS ToS | ❌ cloud | n/a | `pip install edge-tts` | ~0.2× | Clipchamp authoritative male (Copilot cadence) |

---

## Sample metadata (for the record)

All eight clips are WAV 22050 Hz mono s16 PCM (bit rate 352800).

| Slug | Duration (s) | Engine version |
| --- | --- | --- |
| `a-edge-aria` | 40.42 | edge-tts 7.2.8 |
| `b-edge-ava` | 38.35 | edge-tts 7.2.8 |
| `c-edge-christopher` | 40.44 | edge-tts 7.2.8 |
| `d-indextts` | 37.36 | IndexTTS-2 @ main, torch 2.8, MPS |
| `e-kokoro` | 35.29 | kokoro-onnx v1.0 |
| `f-kittentts` | 41.43 | kittentts nano 0.1 |
| `h-piper-lessac` | 34.71 | piper-tts 1.3.0, en_US-lessac-medium |
| `i-edge-andrew` | 38.81 | edge-tts 7.2.8 |

## Your take

No pipeline changes land until you pick. The Rust selector (ADR-0010 v2)
already favours IndexTTS over Piper on GPU hosts, but the 26 journeys are
still rendered with whatever they were when captured. Once you pick a
winner:

- **IndexTTS (D)** — we re-render the 26 journeys locally on MPS; longest
  wall-clock but best quality.
- **Kokoro (E)** — we re-render locally on CPU; fastest offline path.
- **KittenTTS (F)** — pick only if install size matters more than quality.
- **Any edge-tts (A/B/C/I)** — we wire `HWLEDGER_VOICE=edge-tts` +
  `HWLEDGER_EDGE_VOICE=<name>` into the render-wrapper and keep cloud on
  the hot path for hero journeys.
- **Piper (H)** — we leave the chain as-is; tier-5 only.
