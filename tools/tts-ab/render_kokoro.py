# Python host required: kokoro-onnx is a Python-only wrapper around the Kokoro
# ONNX model. Consuming it via any other language would mean re-implementing
# the tokenizer + phonemizer the package bundles.
"""Render Kokoro-82M sample via kokoro-onnx."""
from __future__ import annotations

import sys
import time
import urllib.request
from pathlib import Path

import soundfile as sf
from kokoro_onnx import Kokoro


MODEL_URL = "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/kokoro-v1.0.onnx"
VOICES_URL = "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/voices-v1.0.bin"


def _ensure(path: Path, url: str) -> Path:
    if not path.exists():
        print(f">> fetching {url} -> {path}")
        path.parent.mkdir(parents=True, exist_ok=True)
        urllib.request.urlretrieve(url, path)
    return path


def main() -> int:
    script_path = Path(sys.argv[1]).resolve()
    out_path = Path(sys.argv[2]).resolve()
    voice = sys.argv[3] if len(sys.argv) > 3 else "af_heart"

    cache = Path.home() / ".cache" / "hwledger" / "tts" / "kokoro"
    model = _ensure(cache / "kokoro-v1.0.onnx", MODEL_URL)
    voices = _ensure(cache / "voices-v1.0.bin", VOICES_URL)

    text = script_path.read_text(encoding="utf-8").strip()
    kokoro = Kokoro(str(model), str(voices))

    start = time.perf_counter()
    samples, sample_rate = kokoro.create(text, voice=voice, speed=1.0, lang="en-us")
    elapsed = time.perf_counter() - start

    sf.write(str(out_path), samples, sample_rate)
    print(f">> render_seconds={elapsed:.3f} sample_rate={sample_rate} output={out_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
