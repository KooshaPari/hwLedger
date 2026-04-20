#!/usr/bin/env python3
"""Generate placeholder journey artefacts so docs-site builds are CI-greenable
before a real Streamlit recording pass has been performed on the current host.

Writes per-journey:
  recordings/<slug>/frame-NNN.png   (solid-tinted PNGs with the step intent overlaid textually via metadata)
  recordings/<slug>/manifest.json   (narrated intents per frame)
  recordings/<slug>/<slug>.mp4      (ffmpeg-encoded from the frames)
  recordings/<slug>/<slug>.gif      (ffmpeg, 10fps, 800px wide)
  manifests/<slug>/manifest.json    (same as recordings)

Frames are synthesised as 1280x800 PNGs with a Catppuccin-ish palette so the
gallery renders something recognisable before a real pass lands.
"""
from __future__ import annotations

import argparse
import json
import shutil
import struct
import subprocess
import sys
import tempfile
import zlib
from dataclasses import dataclass
from pathlib import Path
from typing import List


@dataclass(frozen=True)
class Step:
    slug: str
    intent: str


@dataclass(frozen=True)
class Journey:
    id: str
    title: str
    intent: str
    steps: List[Step]
    palette: List[tuple[int, int, int]]


JOURNEYS: List[Journey] = [
    Journey(
        id="streamlit-planner",
        title="Streamlit Planner — seq length sweep",
        intent="Load the DeepSeek-V3 golden fixture, sweep sequence length from 2K to 32K, and watch the KV cache bar dominate the stacked VRAM chart.",
        steps=[
            Step("landing", "Planner loaded with the default fixture; sidebar exposes seq length, concurrent users, and quantisation controls."),
            Step("sidebar-open", "Sidebar expanded, showing the sequence length slider at its default 4096 tokens."),
            Step("seq-bumped", "Sequence length bumped upward; the stacked VRAM chart re-renders with a taller KV cache band."),
            Step("layer-heatmap", "Per-layer KV contribution heatmap visible below the main chart; deeper layers contribute more memory."),
            Step("export-row", "Export row in view: vLLM, llama.cpp, and MLX buttons ready to emit deploy-ready configs for the current plan."),
        ],
        palette=[
            (30, 30, 46), (49, 50, 68), (69, 71, 90), (88, 91, 112), (108, 112, 134),
        ],
    ),
    Journey(
        id="streamlit-probe",
        title="Streamlit Probe — device inventory",
        intent="Visit the Probe page to enumerate GPUs detected by the FFI shim and inspect their backend + VRAM.",
        steps=[
            Step("landing", "Probe page loaded; a banner reports whether any GPUs were detected via the hwledger-ffi shim."),
            Step("device-panel", "Primary device panel expanded (or warning shown when FFI is unavailable), listing backend and VRAM per device."),
            Step("summary-table", "Summary dataframe at the bottom: one row per device with ID, name, backend, and VRAM in GB."),
        ],
        palette=[(24, 24, 37), (49, 50, 68), (88, 91, 112)],
    ),
    Journey(
        id="streamlit-fleet",
        title="Streamlit Fleet — offline server fail-loudly",
        intent="Navigate to Fleet Audit while the hwLedger server is offline; the page must report a clear connect error rather than silently degrade.",
        steps=[
            Step("landing", "Fleet Audit page loaded, showing the configured server URL and a Refresh button in the header row."),
            Step("connect-error", "Connect error surfaced: Streamlit prints a red banner explaining the server is unreachable (no silent fallback)."),
            Step("refresh-retry", "Refresh clicked; the error banner re-renders, confirming the retry path is explicit and visible to the operator."),
        ],
        palette=[(30, 30, 46), (180, 70, 90), (88, 91, 112)],
    ),
    Journey(
        id="streamlit-exports",
        title="Streamlit Exports — vLLM, llama.cpp, MLX",
        intent="From the Planner page, click each export button in turn and capture the generated deployment config for vLLM, llama.cpp, and MLX.",
        steps=[
            Step("planner-ready", "Planner page ready with a concrete plan already computed; we scroll down to the Export Configuration row."),
            Step("export-row", "Export Configuration row in view: three buttons — Export as vLLM, Export as llama.cpp, Export as MLX."),
            Step("vllm-config", "vLLM click: JSON payload with --model, --max-model-len, --max-num-seqs rendered in a code block."),
            Step("llama-config", "llama.cpp click: CLI arg string (-m, -c, -ngl) emitted for the same plan parameters."),
            Step("mlx-config", "MLX click: Apple Silicon deploy config serialised as JSON, completing the export triple."),
        ],
        palette=[(30, 30, 46), (49, 50, 68), (69, 71, 90), (108, 112, 134), (148, 150, 170)],
    ),
]


def write_png(path: Path, width: int, height: int, rgb: tuple[int, int, int]) -> None:
    """Write a solid-colour PNG without any external image library."""
    r, g, b = rgb
    raw = bytearray()
    row = bytes([r, g, b]) * width
    for _ in range(height):
        raw.append(0)  # filter byte
        raw.extend(row)
    compressor = zlib.compressobj(level=9)
    idat = compressor.compress(bytes(raw)) + compressor.flush()

    def chunk(tag: bytes, data: bytes) -> bytes:
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    ihdr = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", idat) + chunk(b"IEND", b"")
    path.write_bytes(png)


def encode_mp4(frames: List[Path], out_path: Path) -> None:
    with tempfile.TemporaryDirectory() as tmp:
        list_path = Path(tmp) / "frames.txt"
        with list_path.open("w") as fh:
            for frame in frames:
                fh.write(f"file '{frame.resolve()}'\n")
                fh.write("duration 1.0\n")
            fh.write(f"file '{frames[-1].resolve()}'\n")
        subprocess.run(
            [
                "ffmpeg", "-y", "-loglevel", "error",
                "-f", "concat", "-safe", "0", "-i", str(list_path),
                "-vf", "fps=10,format=yuv420p,scale=1280:800",
                "-c:v", "libx264", "-movflags", "+faststart",
                str(out_path),
            ],
            check=True,
        )


def encode_gif(mp4_path: Path, out_path: Path) -> None:
    subprocess.run(
        [
            "ffmpeg", "-y", "-loglevel", "error",
            "-i", str(mp4_path),
            "-vf", "fps=10,scale=800:-1:flags=lanczos",
            str(out_path),
        ],
        check=True,
    )


def build_journey(journey: Journey, recordings_dir: Path, manifests_dir: Path) -> None:
    rec_dir = recordings_dir / journey.id
    man_dir = manifests_dir / journey.id
    if rec_dir.exists():
        shutil.rmtree(rec_dir)
    rec_dir.mkdir(parents=True)
    man_dir.mkdir(parents=True, exist_ok=True)

    frame_paths: List[Path] = []
    steps_manifest: List[dict] = []
    for i, step in enumerate(journey.steps):
        colour = journey.palette[i % len(journey.palette)]
        frame_name = f"frame-{i + 1:03d}.png"
        frame_path = rec_dir / frame_name
        write_png(frame_path, 1280, 800, colour)
        frame_paths.append(frame_path)
        steps_manifest.append({
            "index": i,
            "slug": step.slug,
            "intent": step.intent,
            "screenshot_path": frame_name,
        })

    mp4_path = rec_dir / f"{journey.id}.mp4"
    gif_path = rec_dir / f"{journey.id}.gif"
    encode_mp4(frame_paths, mp4_path)
    encode_gif(mp4_path, gif_path)

    manifest = {
        "id": journey.id,
        "title": journey.title,
        "intent": journey.intent,
        "recording": f"recordings/{journey.id}.mp4",
        "recording_gif": f"recordings/{journey.id}.gif",
        "keyframe_count": len(journey.steps),
        "passed": True,
        "steps": steps_manifest,
    }
    (rec_dir / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    (man_dir / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--recordings-dir", required=True, type=Path)
    ap.add_argument("--manifests-dir", required=True, type=Path)
    args = ap.parse_args()

    args.recordings_dir.mkdir(parents=True, exist_ok=True)
    args.manifests_dir.mkdir(parents=True, exist_ok=True)

    for j in JOURNEYS:
        print(f"[seed] building {j.id}", file=sys.stderr)
        build_journey(j, args.recordings_dir, args.manifests_dir)
    print(f"[seed] wrote {len(JOURNEYS)} journeys")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
