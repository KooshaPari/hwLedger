# Web: Streamlit Fleet — offline server fail-loudly

The Fleet Audit page points at an hwLedger API server (default `http://localhost:8080`) and lists registered agents. This journey captures the **fail-loudly path**: what happens when the server is offline.

## What you'll see

Narrative beats:

1. Fleet Audit page loaded; header row shows the configured server URL and a Refresh button.
2. After the initial fetch times out, Streamlit prints a red `Cannot reach server…` banner — no silent degradation, no empty state masquerading as success.
3. Refresh clicked; the error re-renders, confirming the retry path is explicit.

This is deliberate: the governance rule (NFR-007, and the global "Optionality and Failure Behavior" policy) is that connectivity failures must be **clear and loud**, never silently fall back to stale data.

<Shot src="/cli-journeys/keyframes/fleet-audit/frame-003.png"
      caption="CLI counterpart — hwledger fleet audit invocation"
      size="small" align="right" />

<Shot src="/cli-journeys/keyframes/fleet-register/frame-011.png"
      caption="hwledger fleet status — same summary the Streamlit page consumes"
      size="small" align="left" />

<JourneyViewer manifest="/streamlit-journeys/manifests/streamlit-fleet/manifest.verified.json" />

## Reproduce

```bash
cd apps/streamlit/journeys
bun install
bash scripts/record-all.sh
STREAMLIT_URL=http://127.0.0.1:8599 bunx playwright test specs/fleet.spec.ts
```
