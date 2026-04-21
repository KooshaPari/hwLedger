# ADR 0014 — OCR: Apple Vision + tesseract + Claude hybrid

Constrains: FR-JOURNEY-005, FR-JOURNEY-008

Date: 2026-04-19
Status: Accepted

## Context

Journey attestation requires reading back text from screenshots and terminal frames to verify: (a) sentinel strings like `__EXIT_0__` did appear, (b) UI state matches manifest expectations, (c) VLM captions are grounded in on-screen text. The OCR stack must handle terminal fonts (JetBrains Mono), rendered UI text, and arbitrary web fonts. It must be mostly offline, with a cloud escalation for hard cases.

## Options

| Option | Offline | Terminal font accuracy | UI screenshot accuracy | Cost / 1k images | Latency | License | Sentinel reliability |
|---|---|---|---|---|---|---|---|
| tesseract 5 (LSTM) | Yes | ~92% | ~85% | $0 | 150 ms | Apache 2 | OK (needs regex backstop) |
| Apple Vision (`VNRecognizeTextRequest`) | Yes (mac only) | ~98% | ~96% | $0 | 40 ms | Apple SDK | Excellent |
| EasyOCR | Yes | ~90% | ~88% | $0 | 400 ms CPU | Apache 2 | OK |
| PaddleOCR | Yes | ~94% | ~93% | $0 | 300 ms | Apache 2 | Good |
| docTR | Yes | ~91% | ~90% | $0 | 500 ms | Apache 2 | OK |
| Google Cloud Vision | No | ~97% | ~97% | $1.50 | 250 ms | Proprietary | Excellent |
| AWS Textract | No | ~95% (doc-focused) | ~92% | $1.50 | 400 ms | Proprietary | Good |
| Claude 4.5 Sonnet (vision) | No | ~99% (with prompt) | ~99% | $3 / 1M input tok | 1500 ms | Proprietary | Excellent (+ judging) |
| GPT-4V | No | ~97% | ~98% | $5 / 1M tok | 1800 ms | Proprietary | Excellent |

## Decision

Three-tier cascade in `hwledger-journey`:

1. **macOS (default)**: Apple Vision via Swift sidecar + XPC. Fast, 96–98% accurate, offline.
2. **Linux/Windows (default)**: tesseract 5 with the `Mono` trained data + bbox post-processor.
3. **Hybrid escalation**: when tier 1/2 misses a sentinel or the VLM judge flags uncertainty, escalate to Claude 4.5 Sonnet multimodal (ADR-0015-judge). Cost bounded by only escalating on sentinel failure (<1% of frames).

## Rationale

- Apple Vision is the quality leader for free offline OCR. It ships with macOS, no model downloads, no GPU required.
- tesseract is the only mature open-source OCR that runs on every target OS with stable Rust bindings (`rusty-tesseract`).
- Claude as a fallback-cum-judge saves engineering a dedicated "is this screenshot plausible" model — the same call gives us OCR + caption + grounding check (see ADR-0015).

## Consequences

- Two OCR stacks to maintain. Acceptable because Apple Vision integration is ~50 LOC of Swift and tesseract is shelled out.
- Anthropic dep on critical path when escalation triggers. Budgeted at <$5/journey-run.
- Sentinel strings (e.g. `__EXIT_0__`) are deliberately zero-ambiguous tokens chosen to survive any OCR path.

## Revisit when

- An open-source OCR (PaddleOCR v5 or later) matches Apple Vision on terminal fonts.
- Anthropic pricing changes materially, or local VLMs (Qwen2.5-VL 72B, Llama 4V) reach >96% on our benchmark.
- Windows ships a system OCR API with comparable quality (Windows.Media.Ocr is close but weaker on terminals).

## References

- Apple Vision: https://developer.apple.com/documentation/vision/vnrecognizetextrequest
- tesseract 5: https://github.com/tesseract-ocr/tesseract
- PaddleOCR: https://github.com/PaddlePaddle/PaddleOCR
- ADR-0015 (VLM judge), ADR-0011 (Remotion).
