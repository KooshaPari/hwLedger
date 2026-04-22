# ADR 0010 — TTS backend: five-tier chain (IndexTTS 2.0 default)

Constrains: FR-JOURNEY-001, FR-DOCS-002

Date: 2026-04-19 (v1, Piper default)
Revised: 2026-04-22 (v2, five-tier chain — IndexTTS 2.0 default; Piper demoted to tier-5)
Status: Accepted (v2 supersedes v1)

## Context

hwLedger journey captures pair terminal recordings and screenshots with a
narrated voiceover. The voiceover pipeline must work offline on the
self-hosted macOS runner, be reasonably deterministic, and free for CI.
Paid cloud TTS is acceptable for marketing renders but must not be on the
critical path.

v1 (2026-04-19) chose Piper as the default because it was the only offline
MIT-licensed option with acceptable quality at the time. Three things
changed between v1 and v2:

1. **IndexTTS 2.0** (IndexTeam/IndexTTS-2, Apache-2.0, 2026-02) shipped
   zero-shot expressive synthesis that runs on Apple MPS and CUDA. See
   `docs-site/audio/voice-ab.md` slot D — it is the clear quality winner
   on the 40-second `plan-deepseek` script.
2. **Kokoro-82M** (Apache-2.0, kokoro-onnx wrapper) shipped pure-ONNX
   StyleTTS-2 derivative that runs CPU-only at roughly Piper speed with
   materially better prosody. See slot E.
3. **KittenTTS nano** (MIT, KittenML) shipped a tiny (~25 MB) ONNX model
   usable on resource-constrained CI that previously only had Piper.

All three of these are **offline** and **local**. Piper is not the only
MIT-ish offline option any more; it is the *worst* one.

Key constraints (unchanged):

- Offline + air-gapped capable (self-hosted runner policy, ADR-0022).
- Permissive license for redistribution inside rendered videos (Apache-2.0,
  MIT, or equivalent).
- Manifest-driven voice selection (ADR-0016).
- Latency budget on the Remotion render loop (ADR-0011): ideally <200 ms /
  200-char sentence on the host that runs the pipeline; batch cold-start
  cost is tolerated once per journey batch.

## Options

| Option | Offline | License | Quality (rough MOS) | Latency | Cost / 1M chars | Install | Voice variety | Cloning |
|---|---|---|---|---|---|---|---|---|
| **IndexTTS 2.0** (IndexTeam) | Yes | Apache-2.0 | **4.5** | 13.5x RTF on MPS; near-realtime on CUDA | $0 | `uv` venv + 5.5 GB weights | inf (zero-shot clone) | Yes |
| **Kokoro-82M** (kokoro-onnx) | Yes | Apache-2.0 | 4.2 | ~0.3x RTF CPU | $0 | `pip install kokoro-onnx` + 330 MB | 50+ voices | No |
| **KittenTTS nano** (KittenML) | Yes | MIT | 3.7 | ~0.4x RTF CPU | $0 | `pip install kittentts` + 25 MB | 8 voices | No |
| **AVSpeechSynthesizer** (macOS `say`) | Yes | Apple EULA (no redistribution) | 3.3 | 10 ms | $0 | built-in macOS | ~20 voices | No |
| **NeuTTS Air** (fluxions-ai) | Yes | Apache-2.0 | 4.1 (est.) | ~0.6x RTF CPU | $0 | `pip` + 400 MB | zero-shot | Yes |
| **F5-TTS** (SWivid) | Yes | CC-BY-NC-4.0 | 4.4 | ~0.4x RTF on CUDA | $0 (non-commercial only) | `pip` + 1.4 GB | zero-shot | Yes |
| **Chatterbox** (ResembleAI) | Yes | MIT | 4.3 | ~0.5x RTF CPU | $0 | `pip` + 1.6 GB | zero-shot, emotion dials | Yes |
| **StyleTTS 2** (yl4579) | Yes | MIT | 4.3 | ~0.4x RTF on CUDA | $0 | `pip` + 1.2 GB | zero-shot | Yes |
| **Piper** (rhasspy) | Yes | MIT | 3.9 | ~30-60 ms (fastest) | $0 | `brew`/`cargo` + 60 MB | 60+ voices | Opt-in (LJS-style) |
| **edge-tts** (Microsoft Edge ReadAloud) | No | undocumented ToS | 4.4 | 120 ms | $0 (ToS-gray) | `pip install edge-tts` | 400+ voices | No |
| **Coqui XTTS v2** | Yes | CPML (non-commercial) | 4.4 | 300 ms CPU / 90 ms GPU | $0 (non-commercial only) | `pip` + pytorch | inf | Yes |
| **ElevenLabs v3** | No | Proprietary | 4.7 | 250 ms | $99/mo (~$0.30/1M) | API key | 1000+ | Yes |
| **OpenAI TTS-1-hd** | No | Proprietary | 4.3 | 400 ms | $30/1M | API key | 6 | No |
| **AWS Polly Neural** | No | Proprietary | 4.1 | 200 ms | $16/1M | AWS creds | 100+ | No |
| **Azure Neural TTS** | No | Proprietary | 4.2 | 200 ms | $16/1M | Azure creds | 400+ | Custom (paid) |
| **macOS `say`** (bare) | Yes | Apple EULA | 3.2 | 10 ms | $0 | built-in | ~20 | No |

## Decision (v2)

**Five-tier auto chain**, implemented in
`crates/hwledger-journey-render/src/lib.rs::select_voice_backend`:

1. `HWLEDGER_VOICE` explicit override (always wins).
2. **IndexTTS 2.0** — if a GPU (MPS or CUDA) *and* the IndexTTS venv +
   checkpoints are present.
3. **Kokoro-82M** — if its driver venv is present.
4. **KittenTTS nano** — if its driver venv is present.
5. **AVSpeechSynthesizer** (`say`) — on macOS hosts.
6. **Piper** — tier-5 CI fallback for headless Linux runners.
7. **Silent** — last resort.

**edge-tts is never selected by `auto`.** It is cloud and is reserved for
explicit opt-in only (`HWLEDGER_VOICE=edge-tts` with optional
`HWLEDGER_EDGE_VOICE`). Keeps the offline/air-gapped posture honest.

**Paid tier**: ElevenLabs remains wired for marketing renders behind
`HWLEDGER_TTS_BACKEND=elevenlabs` + `ELEVENLABS_API_KEY`. Never on CI.

**Rejected for default**:

- F5-TTS and Coqui XTTS — non-commercial licence.
- Chatterbox, StyleTTS 2, NeuTTS Air — promising, but install size /
  maturity worse than Kokoro at similar quality; revisit when any of them
  beats IndexTTS on the A/B page.
- macOS `say` bare — redistribution terms. (AVSpeechSynthesizer is
  invoked for *local preview* only; generated audio is not redistributed
  with releases; we fall through to a permissively-licensed engine for
  publish-path renders.)
- edge-tts / OpenAI / Polly / Azure — cloud + ToS/billable.

## Rationale

- IndexTTS 2.0 matches cloud neural quality on the A/B script while
  staying fully offline and Apache-2.0.
- Kokoro-82M replaces Piper as the CPU-only default because it sounds
  better at a comparable install size.
- Piper stays in the chain because some CI runners can't install torch or
  onnxruntime — a 60 MB single-binary fallback is still worth having.
- Keeping edge-tts opt-in (rather than auto-fallback) removes the ToS and
  network-egress risk from automatic execution paths.

## Consequences

- First-time IndexTTS setup is ~6.8 GB (weights + deps). Bundled script:
  `tools/tts-ab/render_indextts.py`.
- Per-journey render is dominated by IndexTTS cold-start (~30 s on MPS)
  plus RTF ~13.5x for the autoregressive pass. Batch mode amortises the
  cold-start over N journeys; single-shot re-renders are slow.
- Determinism: IndexTTS is *not* byte-deterministic across torch / MPS
  versions. Journey attestation (ADR-0015) now hashes the final WAV
  rather than requiring bitwise reproducibility.

## Revisit when

- A new offline model beats IndexTTS 2.0 on the A/B page (leaderboard
  candidates: NeuTTS Air, F5-TTS, Chatterbox, StyleTTS 2 — any of these
  promoted above IndexTTS triggers an ADR 0010 v3).
- IndexTTS licence changes (Apache-2.0 regression).
- MPS or CUDA RTF on the self-hosted runner exceeds 20x (indicates
  pipeline regression).
- Piper is abandoned upstream — if that happens, drop tier-5 entirely in
  favour of `Silent`.
- Apple redistribution terms for AVSpeechSynthesizer output change — if
  they allow redistribution, promote AVSpeech above KittenTTS on macOS.

## References

- IndexTTS 2.0: https://github.com/index-tts/index-tts (Apache-2.0)
- Kokoro-82M: https://huggingface.co/hexgrad/Kokoro-82M (Apache-2.0)
- KittenTTS: https://huggingface.co/KittenML/kitten-tts-nano-0.1 (MIT)
- NeuTTS Air: https://github.com/fluxions-ai/neutts-air
- F5-TTS: https://github.com/SWivid/F5-TTS (CC-BY-NC-4.0)
- Chatterbox: https://github.com/resemble-ai/chatterbox
- StyleTTS 2: https://github.com/yl4579/StyleTTS2
- Piper: https://github.com/rhasspy/piper
- A/B taste test: `docs-site/audio/voice-ab.md`
- Selector code: `crates/hwledger-journey-render/src/lib.rs::select_voice_backend`
- ADR-0011 (Remotion), ADR-0015 (attestation), ADR-0016 (manifest voices),
  ADR-0022 (self-hosted runner).
