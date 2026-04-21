# Rich video pipeline — reference

End-to-end reference for the `hwledger-journey-render` crate + the
Remotion project under `tools/journey-remotion/`. For the user-facing
walkthrough see the [rich-journey-renders guide](../guides/rich-journey-renders.md).

## Binary — `hwledger-journey-render`

```
hwledger-journey-render <command>

Commands:
  all <root>      Batch-render every manifest.verified.json under <root>
  one [flags]     Render a single journey (absolute paths)
```

### `all` — batch mode

```
hwledger-journey-render all <root> [--remotion-root <path>] [--force]
                                   [--voiceover silent|piper]
```

- **Walk:** every `manifest.verified.json` under `<root>`.
- **Classify** by path convention:
  | Family      | Manifest path (under `<root>`)                          | Keyframe source                          | Output                                  |
  |-------------|---------------------------------------------------------|------------------------------------------|-----------------------------------------|
  | CLI         | `cli-journeys/manifests/<id>/manifest.verified.json`    | `cli-journeys/keyframes/<id>/`           | `cli-journeys/recordings/<id>/<id>.rich.mp4` |
  | Streamlit   | `streamlit-journeys/manifests/<id>/manifest.verified.json` | `streamlit-journeys/recordings/<id>/`    | `streamlit-journeys/recordings/<id>/<id>.rich.mp4` |
  | GUI         | `gui-journeys/<id>/manifest.verified.json`              | `gui-journeys/<id>/keyframes/`           | `gui-journeys/<id>/<id>.rich.mp4`       |
- **Idempotent:** on each invocation the driver computes the SHA-256 of
  the manifest with the three enrichment fields redacted. If that hash
  matches the stored `recording_rich_manifest_sha256` and the rich MP4
  still exists, the journey is skipped. `--force` overrides.
- **Writeback:** on a successful render, the driver patches:

  ```jsonc
  {
    "recording_rich": "recordings/<id>/<id>.rich.mp4",
    "recording_rich_sha256": "<sha256 of the MP4 bytes>",
    "recording_rich_manifest_sha256": "<sha256 of the canonicalised manifest>"
  }
  ```

- **Exit code:** 0 if every journey rendered or was skipped, non-zero
  if any render failed. Failures are logged per-journey; the batch
  continues past missing/corrupt raw recordings without blocking.

### `one` — single-journey mode

Same semantics as the legacy flag-only invocation (`--journey ... --manifest ...`), kept for direct scripting. Pass absolute paths.

## Remotion composition

`tools/journey-remotion/src/index.tsx` registers one composition,
`JourneyRich`, 1280×800 @ 30 fps. Scene layout:

```
[0 .. 2s)      TitleCard (journey intent)
[2s .. 2s+Nd)  Per-step FrameStill + CalloutBox + CaptionBar (N = steps, d = 3s default)
[+1s]          Outro TitleCard (pass/fail)
```

The per-step duration can be overridden via a `scenes[]` sidecar in the
enriched manifest (`durationFrames`).

## Annotate step (baked flow)

1. **Project annotations.** The bbox registry lives in
   [`phenotype-journeys/data/shot-annotations.yaml`](https://github.com/KooshaPari/phenotype-journeys/blob/main/data/shot-annotations.yaml)
   keyed by `<journey_id>.<frame_index>` (1-based, matching the
   `frame-NNN.png` suffix). The Rust CLI projects these entries onto
   `steps[].annotations` in every matching `manifest.verified.json`:

   ```bash
   hwledger-journey-render project-annotations \
     --yaml /path/to/phenotype-journeys/data/shot-annotations.yaml \
     --manifest apps/cli-journeys/manifests/<id>/manifest.verified.json \
     --manifest docs-site/public/cli-journeys/manifests/<id>/manifest.verified.json
   ```

2. **Bake keyframes.** `src/annotate.ts` (invoked via the `annotate`
   subcommand, or automatically by `one`/`all` when the manifest has
   annotations) composites `steps[].annotations` onto each keyframe PNG
   via `sharp`, writing `<frame>.annotated.png` next to the source.

3. **Bake into the rich video.** The Remotion `FrameStill` prefers
   `<name>.annotated.png` over `<name>.png`, so bboxes render directly
   into the MP4 bitstream — no runtime SVG overlay required.

4. **Viewer toggle.** `@phenotype/journey-viewer` ≥0.1.0 respects the
   same baked PNG. The lightbox toolbar shows an
   "Annotations baked: on/off" toggle (persisted to
   `localStorage['phenotype-journey:annotations-baked-on']`, default on)
   that swaps between the baked PNG (with the live SVG overlay hidden)
   and the raw PNG + live SVG overlay. Gallery thumbnails follow the
   same preference.

If no `annotations[]` are set on any step, annotate is a no-op and
returns success — this is the normal state for journeys that haven't
been annotated yet.

## Voiceover

Two backends:

| Backend  | Behaviour                                                                 |
|----------|---------------------------------------------------------------------------|
| `silent` | default; no `<Audio>` track                                                |
| `piper`  | synthesise per-step WAV via [`piper-tts`](https://github.com/OHF-Voice/piper1-gpl) (`pip install piper-tts`); concatenate with `ffmpeg`; mix into Remotion as `<Audio src={staticFile(...)} />` |

`synthesise_voiceover_piper()` writes per-step WAVs under
`tools/journey-remotion/public/audio/<journey>/` (transient), then
concatenates them into `public/audio/<journey>.voiceover.wav` and sets
`manifest.voiceover.audio = "audio/<journey>.voiceover.wav"` on the
rich manifest. The composition sees `voiceover.backend === "piper"` and
mounts an `<Audio>` element.

The authoritative WAV is also dropped at `recordings/<journey>.voiceover.wav`
next to the rich MP4, and recorded on the canonical
`manifest.verified.json` as `recording_audio_voiceover` so downstream
tooling can play or re-mix it without reaching into the Remotion working
directory.

**Voice model:** `en_US-lessac-medium` by default; override via
`HWLEDGER_PIPER_VOICE=/path/to/voice.onnx`.

If Piper is not installed the `--voiceover piper` branch hard-fails
(no silent fallback) — you want to know that the audio track you
asked for didn't materialise, not discover it in a play-through.
Omit the flag (or pass `--voiceover silent`) to stay silent.

## Idempotency / canonicalisation

The batch driver canonicalises both `<root>` and `--remotion-root` to
absolute paths before spawning subprocesses. This is required because
the Remotion render subprocess runs with `current_dir = remotion_root`;
relative paths would otherwise resolve against the wrong directory and
produce output under `tools/journey-remotion/<relative-path>/...`.

## Nightly CI

`.github/workflows/journey-rich-render.yml` is configured to run at
`cron: "0 10 * * *"` (03:00 PT) on a self-hosted Linux runner. It:

1. Pulls `main`.
2. Installs Bun + caches `tools/journey-remotion/node_modules`.
3. Runs `hwledger-journey-render all docs-site/public`.
4. Commits any updated `*.rich.mp4` + `manifest.verified.json` if the
   hash of an underlying manifest changed (PR back to `main`).

See the workflow file for details.
