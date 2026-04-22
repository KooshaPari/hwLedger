# Python is the correct host here: IndexTTS 2.0 ships as a Python-only
# inference package (torch + modelscope + custom CUDA kernels). There is no
# Rust/Go/Zig binding; this driver is the minimum Python glue to call
# IndexTTS2.infer() from our repo with no extra dependencies.
"""Render IndexTTS 2.0 sample for voice A/B page.

Usage (run under the IndexTTS venv from ~/.cache/hwledger/tts/index-tts/.venv):

    python render_indextts.py <script.txt> <ref.wav> <out.wav>
"""
from __future__ import annotations

import os
import sys
import time
from pathlib import Path


def main() -> int:
    script_path = Path(sys.argv[1]).resolve()
    ref_path = Path(sys.argv[2]).resolve()
    out_path = Path(sys.argv[3]).resolve()

    text = script_path.read_text(encoding="utf-8").strip()
    repo_root = Path(os.environ["INDEXTTS_ROOT"]).resolve()
    os.chdir(repo_root)  # IndexTTS hard-codes ./checkpoints paths

    from indextts.infer_v2 import IndexTTS2  # noqa: E402

    tts = IndexTTS2(
        cfg_path="checkpoints/config.yaml",
        model_dir="checkpoints",
        use_fp16=False,
    )

    print(f">> device={tts.device}")
    start = time.perf_counter()
    tts.infer(
        spk_audio_prompt=str(ref_path),
        text=text,
        output_path=str(out_path),
        verbose=False,
    )
    elapsed = time.perf_counter() - start
    print(f">> render_seconds={elapsed:.3f} output={out_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
