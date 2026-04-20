#!/usr/bin/env python3
"""Generate placeholder GUI-journey artefacts.

Produces per-step PNG keyframes (gradient + narrated intent text), an H.264 mp4,
and an optimized preview GIF matching the real `run-journeys.sh` layout. Used
when the recorder cannot run in the current environment (missing Accessibility +
Screen Recording permission or non-macOS CI); the user re-runs run-journeys.sh
locally to overwrite these with real captures.

Usage:
    generate-placeholder-artefacts.py <slug> <width> <height> \\
        <color_a_hex> <color_b_hex> <title> <intent1> [intent2 ...]
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont


def hex_rgb(h: str) -> tuple[int, int, int]:
    h = h.lstrip("#")
    return tuple(int(h[i : i + 2], 16) for i in (0, 2, 4))  # type: ignore[return-value]


def load_font(size: int) -> ImageFont.FreeTypeFont:
    for path in (
        "/System/Library/Fonts/Supplemental/Menlo.ttc",
        "/System/Library/Fonts/Menlo.ttc",
        "/System/Library/Fonts/Helvetica.ttc",
    ):
        if Path(path).exists():
            try:
                return ImageFont.truetype(path, size)
            except OSError:
                continue
    return ImageFont.load_default()


def wrap(draw: ImageDraw.ImageDraw, text: str, font, max_width: int) -> list[str]:
    words = text.split()
    lines: list[str] = []
    cur = ""
    for w in words:
        trial = f"{cur} {w}".strip()
        if draw.textlength(trial, font=font) <= max_width:
            cur = trial
        else:
            if cur:
                lines.append(cur)
            cur = w
    if cur:
        lines.append(cur)
    return lines


def make_frame(
    out_path: Path,
    width: int,
    height: int,
    color_a: tuple[int, int, int],
    color_b: tuple[int, int, int],
    title: str,
    slug: str,
    frame_idx: int,
    total: int,
    intent: str,
) -> None:
    img = Image.new("RGB", (width, height), color_a)
    # Vertical gradient A -> B
    top = color_a
    bot = color_b
    for y in range(height):
        t = y / max(height - 1, 1)
        r = int(top[0] + (bot[0] - top[0]) * t)
        g = int(top[1] + (bot[1] - top[1]) * t)
        b = int(top[2] + (bot[2] - top[2]) * t)
        img.paste((r, g, b), (0, y, width, y + 1))

    draw = ImageDraw.Draw(img)
    title_font = load_font(44)
    sub_font = load_font(22)
    body_font = load_font(30)
    foot_font = load_font(18)

    draw.text((60, 60), title, fill=(255, 255, 255), font=title_font)
    draw.text(
        (60, 120),
        f"{slug} · frame {frame_idx} of {total}",
        fill=(255, 255, 255, 180),
        font=sub_font,
    )

    # Intent body, wrapped, centered vertically
    max_w = width - 120
    lines = wrap(draw, intent, body_font, max_w)
    line_h = 42
    total_h = line_h * len(lines)
    y0 = (height - total_h) // 2
    for i, line in enumerate(lines):
        draw.text((60, y0 + i * line_h), line, fill=(250, 250, 250), font=body_font)

    draw.text(
        (60, height - 60),
        "placeholder · real recording pending on user Mac (run-journeys.sh)",
        fill=(230, 230, 230, 170),
        font=foot_font,
    )

    img.save(out_path, format="PNG")


def main() -> int:
    if len(sys.argv) < 8:
        print(__doc__, file=sys.stderr)
        return 2

    slug = sys.argv[1]
    width = int(sys.argv[2])
    height = int(sys.argv[3])
    color_a = hex_rgb(sys.argv[4])
    color_b = hex_rgb(sys.argv[5])
    title = sys.argv[6]
    intents = sys.argv[7:]

    repo_root = Path(__file__).resolve().parents[4]
    out_dir = repo_root / "docs-site" / "public" / "gui-journeys" / slug
    kf_dir = out_dir / "keyframes"
    kf_dir.mkdir(parents=True, exist_ok=True)

    # Clean previous placeholder artefacts
    for p in kf_dir.glob("frame_*.png"):
        p.unlink()
    for name in ("recording.mp4", "preview.gif"):
        f = out_dir / name
        if f.exists():
            f.unlink()

    total = len(intents)
    for i, intent in enumerate(intents, start=1):
        make_frame(
            kf_dir / f"frame_{i:03d}.png",
            width,
            height,
            color_a,
            color_b,
            title,
            slug,
            i,
            total,
            intent,
        )

    # mp4 (1s per frame, 25 fps re-encode)
    subprocess.run(
        [
            "ffmpeg", "-y", "-loglevel", "error",
            "-framerate", "1",
            "-pattern_type", "glob",
            "-i", str(kf_dir / "frame_*.png"),
            "-c:v", "libx264",
            "-pix_fmt", "yuv420p",
            "-r", "25",
            "-vf", f"scale={width}:{height}",
            str(out_dir / "recording.mp4"),
        ],
        check=True,
    )

    # preview.gif (palette for quality)
    subprocess.run(
        [
            "ffmpeg", "-y", "-loglevel", "error",
            "-framerate", "1",
            "-pattern_type", "glob",
            "-i", str(kf_dir / "frame_*.png"),
            "-vf",
            f"scale={width}:{height}:flags=lanczos,fps=2,split[s0][s1];"
            "[s0]palettegen[p];[s1][p]paletteuse",
            str(out_dir / "preview.gif"),
        ],
        check=True,
    )

    print(f"Wrote {total} frames + mp4 + gif for {slug}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
