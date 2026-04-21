# Rich journey renders (Remotion pipeline)

Every CLI or Streamlit journey has a **raw** recording — the unedited tape from the recorder — and an optional **rich** recording: the same frames composited by [Remotion](https://www.remotion.dev/) with a title card, per-step callouts, captions, and subtle Ken-Burns motion on each keyframe.

The `<JourneyViewer>` component surfaces both via a **Rich / Raw** toggle at the top of the video. When a manifest has no `recording_rich` field the toggle disappears and the raw tape plays as before.

## What the pipeline produces

| Field | Source | Purpose |
|---|---|---|
| `recording` | the recorder | raw terminal/Streamlit tape |
| `recording_gif` | ffmpeg | low-bandwidth preview |
| `recording_rich` | Remotion | annotated, narrated render |

A rich MP4 is roughly **3–5 MB for 10–15 s at 1280×800**, rendered in **~7–8 s** per journey on an M-series Mac.

## How to render a journey

```bash
# 1. One-time deps (node_modules + chromium).
cd tools/journey-remotion
bun install
cd -

# 2a. Batch-render every manifest under a root (idempotent — skips
#     journeys whose manifest hash matches the stored
#     recording_rich_manifest_sha256).
cargo run -p hwledger-journey-render --release -- \
  all docs-site/public

# 2b. Or render a single journey.
cargo run -p hwledger-journey-render --release -- \
  one \
  --journey plan-deepseek \
  --manifest "$(pwd)/docs-site/public/cli-journeys/manifests/plan-deepseek/manifest.verified.json" \
  --keyframes "$(pwd)/docs-site/public/cli-journeys/keyframes/plan-deepseek" \
  --remotion-root "$(pwd)/tools/journey-remotion" \
  --output "$(pwd)/docs-site/public/cli-journeys/recordings/plan-deepseek/plan-deepseek.rich.mp4"
```

> **Always pass absolute paths** to `one`. The Remotion subprocess runs
> with `current_dir = remotion-root`, so relative paths resolve against
> the wrong directory and the MP4 lands nested under the Remotion tree.
> The `all` subcommand canonicalises paths automatically.

### How to add a new journey

1. **Tape file** — author the driver in `apps/*-journeys/` (a `.tape`
   for VHS, a Playwright script for Streamlit, or a SwiftUI record for
   GUI). It must emit a raw MP4 and a `keyframes/<id>/frame-NNN.png`
   directory.
2. **record-all** — run the family's `record-all.sh` (CLI or Streamlit)
   or the GUI recorder. Output lands under
   `docs-site/public/<family>-journeys/...`.
3. **Verify** — `phenotype-journey check-verified` emits
   `manifest.verified.json` alongside `manifest.json`.
4. **Enrich** — `cargo run -p hwledger-journey-render --release --
   all docs-site/public`. The new journey is picked up automatically,
   rendered, and patched with `recording_rich` +
   `recording_rich_sha256` + `recording_rich_manifest_sha256`.
5. **Docs embed** — the journey-viewer page finds the rich MP4 via
   `manifest.recording_rich`; no MDX changes are needed. If you want
   the rich render inline in a narrative page, use
   `<Shot manifest="…" variant="rich"/>` (see
   `visual-walkthrough-plan-deepseek.md`).

### Authoring in the Remotion Studio

```bash
cd tools/journey-remotion
bun run studio   # opens http://localhost:3000
```

Drag scene boundaries, tweak callouts, re-render from the UI.

## Annotated keyframes

If a manifest has `steps[].annotations` (bbox + label — see [the manifest schema](https://github.com/KooshaPari/phenotype-journeys/blob/main/schema/manifest.schema.json)), the pipeline runs `src/annotate.ts` under [sharp](https://sharp.pixelplumbing.com/) before rendering, composites SVG overlays on each PNG, and writes `<frame>.annotated.png` next to the source. The rich render automatically picks up the annotated PNG when `annotated_keyframes` lists it.

## Voiceover (optional, opt-in)

The pipeline supports two backends:

- **`silent`** (default) — no audio.
- **`piper`** — local neural TTS. Install via:

  ```bash
  # macOS
  brew install piper

  # Linux
  curl -L https://github.com/rhasspy/piper/releases/latest/download/piper_linux_x86_64.tar.gz | tar xz
  ```

  Then pass `--voiceover piper` to the Rust CLI; the pipeline will invoke `piper --model en_US-ryan-high.onnx` per scene line and route the resulting WAV into the Remotion `<Audio>` track.

If Piper is not on PATH the pipeline stays silent — it does **not** fail — and the enriched manifest records `voiceover: "silent"` so this is visible in the artefact trail.

## Self-hosted CI

`.github/workflows/journey-rich-render.yml` runs on a **self-hosted Linux runner only** (the hosted macOS/Windows runners are skipped per the Phenotype-wide Actions billing policy). It:

1. Installs Bun and caches `tools/journey-remotion/node_modules`.
2. Renders every journey that has a `scenes:` section in its sidecar.
3. Uploads the MP4s as a workflow artefact for preview.
4. **Does not** commit the rendered MP4s back to the repo — treat it as an acceptance gate, not a publish step.

Run locally with `act` or `gh workflow run journey-rich-render.yml`.

## Borrowed components

The Remotion components in `tools/journey-remotion/src/components/` (CalloutBox, CaptionBar, TitleCard, FrameStill) are inlined from the [dino scripts/video pattern](https://github.com/KooshaPari/dino/tree/main/scripts/video); see `/Users/kooshapari/CodeProjects/Phenotype/repos/phenotype-journeys/remotion/borrowed/PROVENANCE.md` for license and attribution details.
