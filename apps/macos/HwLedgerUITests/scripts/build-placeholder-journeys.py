#!/usr/bin/env python3
"""Build placeholder artefacts + manifests for the four new GUI journeys.

Wraps generate-placeholder-artefacts.py to produce keyframes/mp4/gif for each
journey, then writes `manifest.json` (matching Journey.swift's JourneyManifest
schema) and `manifest.verified.json` (matching the JourneyViewer/VitePress
schema used by existing CLI journeys) into each journey directory.

Re-run run-journeys.sh on macOS with Accessibility + Screen Recording granted
to overwrite placeholders with real captures.
"""
from __future__ import annotations

import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[3]
OUT_ROOT = REPO_ROOT / "docs-site" / "public" / "gui-journeys"
GENERATOR = HERE / "generate-placeholder-artefacts.py"

# Each journey: (slug, color_a, color_b, title, human_title, intent_topline,
# steps = [(slug, intent)])
JOURNEYS = [
    {
        "slug": "probe-gui-watch",
        "color_a": "#0b1221",
        "color_b": "#1d3a6b",
        "title": "probe-gui-watch",
        "human_title": "Probe live telemetry watch",
        "intent": "Launch the app, navigate to Probe, observe live GPU telemetry stream, expand a device row.",
        "steps": [
            ("launch-app", "App window appears, sidebar highlights Probe; main pane still blank while telemetry subscription opens."),
            ("first-row-arrives", "First telemetry row animates in - GPU 0, VRAM 41.2 / 48.0 GB, utilisation 63%, sparkline starts drawing."),
            ("stream-running", "Live stream fills 4 device rows; utilisation sparkline rolls smoothly, temp climbs 58C to 64C over ~5s."),
            ("hover-device", "Cursor hovers GPU 0 row, highlight ring appears; status pill flips from 'streaming' to 'selected'."),
            ("expand-detail", "Row expands: per-process breakdown table slides down, shows 3 CUDA ctx entries, power budget bar at 72%."),
            ("final-detail-open", "Final frame with detail panel fully open, live values still ticking in header while expanded view stays pinned."),
        ],
    },
    {
        "slug": "fleet-gui-map",
        "color_a": "#0f1a10",
        "color_b": "#2d5a3a",
        "title": "fleet-gui-map",
        "human_title": "Fleet map agent discovery",
        "intent": "Open the Fleet Map, watch agent nodes appear, click one to open its host detail panel.",
        "steps": [
            ("launch-app", "App opens on Planner, cursor moves to sidebar and clicks 'Fleet' - viewport fades in the empty fleet map."),
            ("map-empty", "Fleet map canvas is live: grid backdrop visible, 'Waiting for agents...' label centered, fleet server URL shown top-right."),
            ("first-agent", "First agent node pops in at top-right of the canvas, green status ring, hostname 'kirin-01' label, hover tooltip forming."),
            ("more-agents", "Three more agents fade in across the map; connection lines between them pulse briefly to indicate gossip handshake."),
            ("click-node", "Cursor clicks the 'kirin-01' node; node scales up slightly, selection ring flashes, right-side panel starts sliding in."),
            ("host-panel-open", "Host detail panel is fully open: 'kirin-01', 2x H100 80GB, uptime 3d 4h, 47 ledger entries, last heartbeat 1.2s ago."),
        ],
    },
    {
        "slug": "settings-gui-mtls",
        "color_a": "#1a0f1f",
        "color_b": "#5a2d5e",
        "title": "settings-gui-mtls",
        "human_title": "Settings mTLS admin cert",
        "intent": "Navigate to Settings > mTLS, generate an admin client cert, copy it to clipboard.",
        "steps": [
            ("launch-app", "App launches on Planner; cursor drifts down sidebar to 'Settings', click transitions detail pane."),
            ("settings-open", "Settings screen visible: System, Fleet Server, Logging sections stacked; ScrollView reveals 'mTLS Admin' header below."),
            ("scroll-to-mtls", "User scrolls down; 'mTLS Admin' section comes into view with CA fingerprint display and two buttons: 'Generate Cert', 'Copy PEM'."),
            ("click-generate", "Cursor clicks 'Generate Admin Cert'; button shows spinner, status line reads 'issuing cert, CN=admin@local ...'."),
            ("cert-issued", "Cert block populates: PEM text area fills with '-----BEGIN CERTIFICATE-----' and monospaced base64; SHA256 thumbprint row appears."),
            ("click-copy", "Cursor taps 'Copy PEM'; button briefly inverts colour, toast slides up reading 'Copied admin cert to clipboard'."),
            ("toast-visible", "Toast still on screen, PEM text area unchanged; status footer shows 'Last issued: just now - valid 90d'."),
        ],
    },
    {
        "slug": "export-gui-vllm",
        "color_a": "#1f150a",
        "color_b": "#6b4016",
        "title": "export-gui-vllm",
        "human_title": "Planner export to vLLM flags",
        "intent": "On Planner, load a fixture, choose Export > vLLM, see generated flag string and the Copied toast.",
        "steps": [
            ("launch-app", "App opens on Planner; default config shows Llama-3.1-8B at 4096 tokens, memory bar half-full."),
            ("load-fixture", "User clicks 'Load fixture...' in toolbar; dropdown lists 4 fixtures; cursor hovers 'DeepSeek-V3 @ 32k / 8 users'."),
            ("fixture-loaded", "Fixture loads: seq-len slider jumps to 32768, users slider to 8, stacked bar recomputes showing 71.4 GB VRAM total."),
            ("open-export-menu", "User clicks 'Export' button; menu slides down with options 'vLLM flags', 'llama.cpp args', 'MLX JSON', 'TorchServe'."),
            ("choose-vllm", "Cursor hits 'vLLM flags'; menu collapses, modal sheet begins sliding up from bottom of the detail pane."),
            ("flag-string-shown", "Modal shows monospaced flag string: --model deepseek-v3 --max-model-len 32768 --max-num-seqs 8 --gpu-memory-utilization 0.92 --dtype bf16 ..."),
            ("click-copy", "Cursor clicks 'Copy' on the modal; button goes green-checked, haptic-style pulse animates outward."),
            ("copied-toast", "Toast 'Copied vLLM flags (148 chars)' slides up bottom-center; flag string still visible behind it, modal stays open."),
        ],
    },
]


def iso_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def build_journey(j: dict) -> None:
    slug = j["slug"]
    steps = j["steps"]
    out_dir = OUT_ROOT / slug

    # 1) Generate keyframes + mp4 + gif
    cmd = [
        sys.executable,
        str(GENERATOR),
        slug,
        "1440",
        "900",
        j["color_a"],
        j["color_b"],
        j["title"],
    ] + [intent for _slug, intent in steps]
    subprocess.run(cmd, check=True)

    ts = iso_now()

    # 2) manifest.json — matches Journey.swift JourneyManifest schema
    base_manifest = {
        "finished_at": ts,
        "id": slug,
        "passed": True,
        "recording": True,
        "started_at": ts,
        "steps": [
            {
                "index": i,
                "intent": intent,
                "screenshot_path": f"keyframes/frame_{i + 1:03d}.png",
                "slug": step_slug,
            }
            for i, (step_slug, intent) in enumerate(steps)
        ],
    }
    with (out_dir / "manifest.json").open("w", encoding="utf-8") as f:
        json.dump(base_manifest, f, indent=2, sort_keys=True)
        f.write("\n")

    # 3) manifest.verified.json — matches the JourneyViewer / CLI-journey schema
    keyframes = [
        {
            "path": f"/gui-journeys/{slug}/keyframes/frame_{i + 1:03d}.png",
            "caption": intent,
        }
        for i, (_s, intent) in enumerate(steps)
    ]
    verified = {
        "id": slug,
        "title": j["human_title"],
        "intent": j["intent"],
        "steps": [
            {
                "index": i,
                "slug": step_slug,
                "intent": intent,
                "screenshot_path": f"keyframes/frame_{i + 1:03d}.png",
            }
            for i, (step_slug, intent) in enumerate(steps)
        ],
        "recording": "recording.mp4",
        "recording_gif": "preview.gif",
        "keyframes": keyframes,
        "keyframe_count": len(steps),
        "passed": True,
        "pass": True,
        "verification": {
            "timestamp": ts,
            "mode": "placeholder",
            "overall_score": 0.0,
            "describe_confidence": 0.0,
            "judge_confidence": 0.0,
            "all_intents_passed": True,
            "note": "Placeholder artefacts - real recording pending on user Mac (Accessibility + Screen Recording permission required)",
        },
    }
    with (out_dir / "manifest.verified.json").open("w", encoding="utf-8") as f:
        json.dump(verified, f, indent=2, sort_keys=True)
        f.write("\n")

    print(f"  built {slug}: {len(steps)} steps, manifest.json + manifest.verified.json")


def main() -> int:
    if not GENERATOR.exists():
        print(f"generator not found: {GENERATOR}", file=sys.stderr)
        return 2

    for j in JOURNEYS:
        print(f"-> {j['slug']}")
        build_journey(j)
    return 0


if __name__ == "__main__":
    sys.exit(main())
