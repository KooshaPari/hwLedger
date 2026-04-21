# ADR 0015 — VLM judge: Claude default, Ollama/MLX local as fallback

Constrains: FR-JOURNEY-006, FR-JOURNEY-008

Date: 2026-04-19
Status: Accepted

## Context

Journey attestation requires a vision-language model (VLM) to:
- Caption frames for docs-site `<Shot>` alt text.
- Judge whether rendered UI state matches manifest expectations ("the sidebar now shows three items").
- Detect bounding boxes for callouts (ADR-0011 composition).
- Act as OCR tier-3 fallback (ADR-0014).

The judge is on the hot path for every journey render, so cost and latency matter; but quality trumps cost (a mis-judged journey silently publishes false docs).

## Options

| Option | Host | Cost / frame (~1000 tok) | Latency | Caption quality | Bbox detection | Local-capable |
|---|---|---|---|---|---|---|
| Claude 4.5 Sonnet vision | Anthropic API | ~$0.003 | 1.2 s | Excellent | Good | No |
| Claude 4.5 Opus vision | Anthropic API | ~$0.015 | 2.0 s | State of the art | Excellent | No |
| GPT-4.1 vision | OpenAI API | ~$0.005 | 1.8 s | Excellent | Good | No |
| Gemini 2.5 Pro vision | Google API | ~$0.002 | 1.0 s | Very good | Very good | No |
| Qwen2.5-VL 72B | Ollama / vLLM | $0 (compute only) | 3.0 s (H100) / 10 s (M3 Max MLX) | Very good | OK | Yes |
| Llama 4V 90B | Ollama | $0 | 4.0 s H100 | Good | Weak | Yes |
| InternVL 2.5 78B | vLLM | $0 | 3.5 s H100 | Very good | Good | Yes |
| LLaVA-NeXT 34B | Ollama | $0 | 2.5 s | OK | Weak | Yes |

## Decision

**Default**: Claude 4.5 Sonnet via Anthropic API for both caption + judge. Opus used only for attestation-critical frames (sentinel failure escalations).

**Local fallback**: `HWLEDGER_VLM=ollama:qwen2.5-vl:72b` swaps in a local model via the `mlx-omni-server` (ADR-0002 shared infra) or Ollama on the self-hosted runner. Used when offline or for air-gapped journeys.

## Rationale

- Claude 4.5 Sonnet hits the best quality/cost point as of 2026-04. Its instruction-following on structured caption JSON is materially better than GPT-4.1 on our internal benchmark (n=200 frames).
- Qwen2.5-VL 72B is the leading open VLM for bbox + caption; MLX quantized it fits on an M3 Max 128 GB.
- Keeping two backends behind one `judge::Judge` trait means journey code is backend-agnostic.

## Consequences

- Anthropic dep and API spend on CI. Budgeted at ≤$10/full-journey-suite-run, monitored.
- Local fallback is ~3× slower and ~1 quality point lower. Acceptable for staging.
- Prompt caching (Anthropic) used aggressively: journey caption prompt is shared across all frames, saving 80% on input tokens.

## Revisit when

- A local VLM (Qwen3-VL, Llama 5V) matches Claude 4.5 Sonnet on our benchmark.
- Anthropic pricing changes or rate-limits break CI.
- Structured vision output (constrained JSON) ships in Ollama with stable grammar constraints.

## References

- Anthropic vision docs: https://docs.anthropic.com/en/docs/build-with-claude/vision
- Qwen2.5-VL: https://github.com/QwenLM/Qwen2.5-VL
- ADR-0014 (OCR), ADR-0016 (manifest).
