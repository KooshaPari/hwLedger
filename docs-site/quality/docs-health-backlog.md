# docs-health backlog

Triage snapshot from `hwledger-docs-health --root docs-site` — initial landing
of the gate. Each row is a concrete finding. Fix the path, remove the
reference, or regenerate the missing artifact.

Run locally to refresh:

```bash
cargo run -q -p hwledger-docs-health -- --root docs-site --json \
  > docs-site/quality/docs-health-triage.json
```

| Path | Line | Check | Severity | Suggestion |
|---|---|---|---|---|
| `docs-site/research/12-ui-journey-harness-2026.md` | 122 | assets | error | regenerate journey artifact or drop the `<video>` until available — missing `...mp4` |
| `docs-site/research/12-ui-journey-harness-2026.md` | 123 | assets | error | regenerate journey artifact or drop the `<img>` until available — missing `...gif` |
| `docs-site/journeys/gui-planner-launch.md` | 59 | assets | error | regenerate journey recording — `/gui-journeys/planner-gui-launch/recording.mp4` |
| `docs-site/quality/gap-backlog-2026-04-21.md` | 23 | links | error | `./cli-*.md` glob is not a link target — inline the list or drop |
| `docs-site/journeys/gui-export-vllm.md` | 57-59 | links | error | regenerate `export-gui-vllm` manifest / recording / preview under `docs-site/public/gui-journeys/` |
| `docs-site/journeys/gui-fleet-map.md` | 56-58 | links | error | regenerate `fleet-gui-map` manifest / recording / preview under `docs-site/public/gui-journeys/` |
| `docs-site/journeys/gui-journeys-status.md` | 269-271 | links | error | links reach outside `docs-site/` into `apps/macos/...` — use a GitHub link or drop |

Additional categories (not enumerated here):

- **placeholders (warn, resolved 2026-04-22):** scans for the placeholder
  marker set defined in `tools/docs-health/src/lib.rs` (the constant near the
  top of the `check_placeholders` function). Each finding should either
  resolve or migrate to a tracked issue. Batch resolved in
  `fix/docs-health-placeholder-warns`.
- **video (warn, 8):** `<100KB` mp4 files under `docs-site/public/` — likely
  truncated or stubbed; regenerate via `phenotype-journey record`.
- **journey (warn, 1):** `phenotype-journey` missing on PATH in current env.
  Install it or set `PHENOTYPE_JOURNEYS_ROOT`.

## Gate policy

Pre-push gate currently runs with `--fail-on error`. When the error list is
drained below a comfortable floor, escalate to `--fail-on warning` to also
block on placeholders and tiny videos.
