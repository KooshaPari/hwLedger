# Python host required: KittenTTS is a Python-only package (KittenML repo)
# that wraps an ONNX text-to-speech model. No native binding exists.
"""Render KittenTTS sample."""
from __future__ import annotations

import sys
import time
from pathlib import Path

import re

import numpy as np
import soundfile as sf
from kittentts import KittenTTS


def _chunks(text: str) -> list[str]:
    # KittenTTS nano ONNX trips an Expand-node shape bug on long inputs.
    # Split on sentence punctuation to keep each chunk short.
    parts = [p.strip() for p in re.split(r"(?<=[.!?])\s+", text) if p.strip()]
    return parts or [text]


def main() -> int:
    script_path = Path(sys.argv[1]).resolve()
    out_path = Path(sys.argv[2]).resolve()
    voice = sys.argv[3] if len(sys.argv) > 3 else "expr-voice-5-f"

    text = script_path.read_text(encoding="utf-8").strip()
    model = KittenTTS()  # default: downloads KittenML/kitten-tts-nano-0.1 from HF

    sample_rate = 24000
    silence = np.zeros(int(sample_rate * 0.18), dtype=np.float32)
    pieces: list[np.ndarray] = []
    start = time.perf_counter()
    for i, chunk in enumerate(_chunks(text)):
        audio = model.generate(chunk, voice=voice)
        pieces.append(audio.astype(np.float32))
        if i < 100:
            pieces.append(silence)
    elapsed = time.perf_counter() - start
    full = np.concatenate(pieces)
    sf.write(str(out_path), full, sample_rate)
    print(f">> render_seconds={elapsed:.3f} output={out_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
