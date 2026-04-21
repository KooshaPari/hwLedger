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

## Annotate step

`src/annotate.ts` composites `steps[].annotations` (bbox/label/colour)
onto each keyframe PNG via `sharp`, writing
`<frame>.annotated.png` next to the source. The composition prefers
`<name>.annotated.png` over `<name>.png` when present.

If no `annotations[]` are set on any step, annotate is a no-op and
returns success — this is the normal state for journeys that haven't
been annotated yet.

## Voiceover

Two backends:

| Backend  | Behaviour                                                                 |
|----------|---------------------------------------------------------------------------|
| `silent` | default; no `<Audio>` track                                                |
| `piper`  | synthesise per-step WAV via [`piper`](https://github.com/rhasspy/piper); mix into Remotion as `<Audio src=...>` |

Piper must be on `PATH`. If missing, the pipeline falls through to
silent with a warning log (`voiceover=piper requested but piper not
found; continuing silent`) — this is a soft failure, not a hard one.

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
