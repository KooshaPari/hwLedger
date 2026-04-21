# ADR 0010 — TTS backend: Piper default, ElevenLabs as paid tier

Constrains: FR-JOURNEY-001, FR-DOCS-002

Date: 2026-04-19
Status: Accepted

## Context

hwLedger journey captures pair terminal recordings and screenshots with a narrated voiceover. The voiceover pipeline must work offline on the self-hosted macOS runner, be deterministic (same text → same waveform byte-for-byte, modulo sampling), and free for CI. Paid cloud TTS is acceptable for marketing-grade renders but must not be on the critical path.

Key constraints:

- Offline + air-gapped capable (self-hosted runner policy, see ADR-0022).
- MIT/Apache-compatible license for redistribution inside rendered videos.
- <200 ms latency per ~200-char sentence so the Remotion render loop does not stall (ADR-0011).
- Programmatic voice selection via manifest (ADR-0016).

## Options

| Option | Offline | License | Quality (rough MOS) | Latency | Cost / 1M chars | Install | Voice variety | Cloning |
|---|---|---|---|---|---|---|---|---|
| Piper (onnx, rhasspy) | Yes | MIT | 3.9 | 30–60 ms | $0 | `brew/cargo` | 60+ voices | Opt-in (LJS-style) |
| edge-tts (Azure ReadAloud) | No | undocumented, effectively TOS-bound | 4.2 | 120 ms | $0 (ToS-gray) | pip | 400+ voices | No |
| Coqui XTTS v2 | Yes | CPML (non-commercial research) | 4.4 | 300 ms CPU / 90 ms GPU | $0 (non-commercial only) | pip + pytorch | ∞ (zero-shot clone) | Yes |
| ElevenLabs v3 | No | Proprietary | 4.7 | 250 ms | $99/mo ≈ $0.30/1M | API key | 1000+ voices | Yes |
| OpenAI TTS-1-hd | No | Proprietary | 4.3 | 400 ms | $30/1M | API key | 6 voices | No |
| AWS Polly Neural | No | Proprietary | 4.1 | 200 ms | $16/1M | AWS creds | 100+ voices | No |
| Azure Neural TTS | No | Proprietary | 4.2 | 200 ms | $16/1M | Azure creds | 400+ voices | Custom neural voice (paid) |
| macOS `say` | Yes | Apple EULA (no redistribution) | 3.2 | 10 ms | $0 | built-in | ~20 voices | No |
| Apple Speech (AVSpeechSynthesizer) | Yes | Apple EULA | 3.3 | 10 ms | $0 | Swift SDK | ~20 voices | No |

## Decision

**Default**: Piper (`rhasspy/piper`) invoked as a sidecar binary fed UTF-8 on stdin, emitting WAV on stdout. Voice model `en_US-lessac-medium.onnx` shipped in `sidecars/piper/models/`.

**Paid tier**: ElevenLabs is wired through `hwledger-journey` when `HWLEDGER_TTS_BACKEND=elevenlabs` and `ELEVENLABS_API_KEY` are set. Used for marketing cuts; never on CI.

**Rejected for default**: Coqui XTTS (license), edge-tts (ToS/net), macOS `say` (redistribution terms). OpenAI/Polly/Azure all rejected as defaults because they are net-only and billable.

## Rationale

- Piper is the only offline, MIT-licensed, Rust-friendly (pure onnxruntime) option with acceptable quality. 3.9 MOS is below cloud neural TTS but above every other free offline engine we tested.
- Determinism: Piper + fixed onnxruntime version + fixed random seed yields byte-equal output; important for journey attestation (ADR-0015).
- Self-hosting cost is zero. Air-gapped CI works.

## Consequences

- Voice expressiveness ceiling is lower than ElevenLabs. Accepted: we ship a non-marketing "technical narrator" persona as the default voice.
- Model file ~60 MB per voice; shipped as a sidecar artifact not a dep. Release tarballs include it.
- If we ever need zero-shot cloning on-prem, revisit Coqui (if license changes) or XTTS-derived MIT forks.

## Revisit when

- An MIT-licensed neural TTS matches ElevenLabs quality on the expressiveness axis (track `coqui-ai/TTS` forks).
- Piper switches licenses or is abandoned.
- Offline per-sentence latency on the runner exceeds 200 ms (indicates model drift or runtime regression).

## References

- Piper: https://github.com/rhasspy/piper
- Coqui XTTS license: https://coqui.ai/cpml
- ADR-0011 (Remotion), ADR-0015 (attestation), ADR-0022 (self-hosted runner).
