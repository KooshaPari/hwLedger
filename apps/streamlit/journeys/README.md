# Streamlit Journey Recorder

Playwright-driven recordings of the hwLedger Streamlit web client (`apps/streamlit`).
Mirrors the CLI journey pipeline in `apps/cli-journeys/` so docs-site can embed both
families of journeys through the same `<JourneyViewer>` component.

## Layout

```
journeys/
  playwright.config.ts     # Chromium headed, video: on, trace: on
  specs/
    planner.spec.ts
    probe.spec.ts
    fleet.spec.ts
    exports.spec.ts
  lib/journey.ts           # JourneyRecorder (frames + manifest.json)
  scripts/
    record-all.sh          # boot streamlit, run playwright, convert video
    verify-manifests.sh    # produce manifest.verified.json (mock or API)
  recordings/<slug>/       # frame-NNN.png + manifest.json + .mp4 + .gif
  manifests/<slug>/        # manifest.json + manifest.verified.json
```

## Record

```bash
cd apps/streamlit/journeys
bun install
bash scripts/record-all.sh
bash scripts/verify-manifests.sh
```

`record-all.sh` boots Streamlit on a free port (`STREAMLIT_PORT`, default `8599`),
waits for `/_stcore/health`, runs Playwright, then converts each test's
`video.webm` to `<slug>.mp4` + `<slug>.gif` (800 px wide, fps 10, <5 MB).

`verify-manifests.sh` reuses the mock Anthropic server from
`apps/cli-journeys/scripts/mock-anthropic-server.py` when `ANTHROPIC_API_KEY` is
not set, so the pipeline is fully offline-green.

## Docs-site integration

`docs-site/scripts/sync-streamlit-journeys.sh` copies `recordings/` + `manifests/`
into `docs-site/public/streamlit-journeys/`; it runs as part of `bun run sync`.
Each page under `docs-site/journeys/streamlit-*.md` embeds the manifest via
`<JourneyViewer manifest="/streamlit-journeys/manifests/<slug>/manifest.verified.json" />`.
