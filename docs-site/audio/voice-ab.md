---
title: Narration Voice A/B — Taste Test
---

# Narration Voice A/B — Taste Test

We're picking the voice that narrates every hwLedger walkthrough. Four candidates
below. Each reads the **same ~40-second script** (pulled from the
`plan-deepseek` walkthrough prose) so you can compare them on identical material.

Don't read the spec table first — press play on all four, then glance at the
table to confirm what you just heard. Your ear picks.

## The script (identical for all four)

> You start with a fresh shell and install hwLedger via cargo. The install prints
> the resolved version plus the path to the binary, which you verify with
> dash-dash-version. Next, you open the planner help to see the full flag list.
> Two flags matter most for DeepSeek: dash-dash-context and dash-dash-batch. With
> those set, the planner runs, detects the MLA attention kind automatically, and
> breaks down memory across weights, KV cache, and prefill activations. On an
> H100, DeepSeek-V3 at 32K context needs roughly three hundred and sixty
> gigabytes in total.

---

## A — edge-tts / Aria (News voice, female)

<audio controls preload="metadata" src="/audio/voice-ab/a-edge-aria.wav"></audio>

*Clear female newscaster, bright confident delivery. Microsoft's flagship News
voice — this is the closest match to the Clipchamp baseline you've already
approved by ear.*

```
codec_name=pcm_s16le
sample_rate=22050
channels=1
bits_per_sample=16
duration=40.416009
bit_rate=352800
```

---

## B — edge-tts / Ava Multilingual (Conversational, female)

<audio controls preload="metadata" src="/audio/voice-ab/b-edge-ava.wav"></audio>

*Warmer, more conversational female voice. Microsoft's Copilot voice — closer
to a podcast host than a newsreader. Less authoritative, more approachable.*

> **Note on SSML styles.** The plan asked for `<mstts:express-as
> style="newscast">` around this voice. The `edge-tts` Python CLI does not pipe
> SSML through to the Edge speech endpoint (it reads the tags as literal text).
> To get styled speech you'd either call Azure Speech SDK directly with your own
> key, or patch `edge-tts` to emit the SSML envelope. Sample B is Ava's
> default expressive delivery, which is what most Clipchamp users hear when
> they pick her.

```
codec_name=pcm_s16le
sample_rate=22050
channels=1
bits_per_sample=16
duration=38.351973
bit_rate=352800
```

---

## C — edge-tts / Christopher (News voice, male)

<audio controls preload="metadata" src="/audio/voice-ab/c-edge-christopher.wav"></audio>

*Authoritative male newsreader. Reliable, grounded delivery with clear
cadence. The spec asked for **Davis** (Clipchamp's authoritative male), but
Davis is not exposed by the `edge-tts` voice list in this version (v7.2.8) —
Microsoft appears to have pulled him from the public endpoint. **Christopher is
the tagged-closest substitute** ("News, Novel" category, "Reliable, Authority"
traits). If you approve this direction we can re-render with Davis the moment
he reappears, or switch to Azure Speech (paid) to get him back today.*

```
codec_name=pcm_s16le
sample_rate=22050
channels=1
bits_per_sample=16
duration=40.440000
bit_rate=352800
```

---

## D — Piper / Lessac (offline, male US)

<audio controls preload="metadata" src="/audio/voice-ab/d-piper-lessac.wav"></audio>

*Fully offline neural TTS via [Piper](https://github.com/rhasspy/piper).
Runs on CPU, no cloud, no API key. Lessac is the medium-quality US English
male voice from the Rhasspy piper-voices collection. **Stand-in for
IndexTTS 2.0** — see next section for why.*

```
codec_name=pcm_s16le
sample_rate=22050
channels=1
bits_per_sample=16
duration=34.713832
bit_rate=352800
```

### Why Piper instead of IndexTTS 2.0

IndexTTS 2.0 was the headline fourth candidate. Install status on this host
(`~/.cache/hwledger/tts/index-tts`) at render time:

| Step | Status | Notes |
| --- | --- | --- |
| `git clone github.com/index-tts/index-tts` | ✅ | LFS examples failed (repo over quota) — not needed for inference |
| Python venv (needs `>=3.10,<3.12`) | ✅ | Python 3.11.14 venv at `.venv/` — miniforge 3.13 was rejected by `numba==0.58.1` |
| `pip install -e .` (torch 2.8, transformers, numba, keras, opencv, modelscope) | ✅ | ~1.3 GB of deps resolved cleanly |
| `hf download IndexTeam/IndexTTS-2 --local-dir=checkpoints` | ⏳ in progress at publish time | Multi-GB model weights; finishes asynchronously |
| First inference run on Metal MPS | ⏳ pending weights | No reference clip needed (zero-shot mode per spec) |

Piper is a legitimate offline substitute for the comparison (same "offline
neural, no cloud" category IndexTTS occupies) but **it is not the same model
class**. IndexTTS 2.0 is a zero-shot, emotion-expressive, duration-controlled
autoregressive model — it should sound markedly more human than Piper's
fast-but-robotic VITS output. Treat sample D as "the floor of the offline
class, not the ceiling." If offline is the direction you like, we re-render D
with IndexTTS 2.0 the moment weights finish syncing and you get to hear the
real thing before committing.

---

## Spec summary (read *after* listening)

| Slug | Engine | Voice | License | Offline-capable | Sample rate |
| --- | --- | --- | --- | --- | --- |
| `a-edge-aria` | edge-tts 7.2.8 | `en-US-AriaNeural` | Microsoft Edge TTS ToS (free, undocumented) | ❌ cloud | 22050 mono s16 |
| `b-edge-ava` | edge-tts 7.2.8 | `en-US-AvaMultilingualNeural` | Microsoft Edge TTS ToS | ❌ cloud | 22050 mono s16 |
| `c-edge-christopher` | edge-tts 7.2.8 | `en-US-ChristopherNeural` (Davis substitute) | Microsoft Edge TTS ToS | ❌ cloud | 22050 mono s16 |
| `d-piper-lessac` | piper-tts 1.3.0 | `en_US-lessac-medium` (IndexTTS stand-in) | MIT (piper) + MIT (voice) | ✅ fully local | 22050 mono s16 |

All four were converted to WAV 22050 Hz mono 16-bit PCM with ffmpeg. The
`duration=` line in each `ffprobe` block confirms no time-stretch happened
during conversion.

## Your take

Which of these would you ship?

- **If it's `a-edge-aria` or `c-edge-christopher`** and you're OK with "cloud
  in the hot path for the hero journeys," we flip the render pipeline to a
  hybrid that talks to Edge TTS for narration and keeps the rest local.
- **If it's `b-edge-ava`** same as above, just with a warmer voice.
- **If it's `d-piper-lessac`** we stay fully offline — no API, no egress, no
  availability risk — at the cost of robotism.
- **If you'd rather judge IndexTTS 2.0 first**, say so — we wait for weights
  to finish and re-render sample D, then pick.

No pipeline changes have been made yet. The Rust render wrapper stays on its
current backend until you call it. No render invalidation, no re-render of the
26 journeys. This page is strictly a listening test.
