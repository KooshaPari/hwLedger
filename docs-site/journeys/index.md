# User Journeys

Interactive recordings of hwLedger workflows, end-to-end. Each journey captures an intent label per step, a keyframe gallery, and a Claude-verified blackbox description with a per-step judge score.

Three families of journeys:

1. **CLI** — recorded via [VHS](https://github.com/charmbracelet/vhs); run in any terminal.
2. **GUI** — recorded via XCUITest + ScreenCaptureKit on macOS.
3. **Web (Streamlit)** — recorded via headed Chromium + [Playwright](https://playwright.dev) against the `apps/streamlit` client.

## Web (Streamlit) journeys

Real-pixel Playwright recordings of the Streamlit web client. Each spec boots Streamlit on a loopback port, drives the page through a realistic interaction, and snapshots numbered keyframes alongside a narrated manifest.

| Journey | What it demonstrates |
|---|---|
| [Planner — seq length sweep](./streamlit-planner.md) | Live memory planning; KV cache grows with context |
| [Probe — device inventory](./streamlit-probe.md) | GPU enumeration via FFI in the browser |
| [Fleet — offline fail-loudly](./streamlit-fleet.md) | Connect-error UX; no silent fallback (NFR-007) |
| [Exports — vLLM / llama.cpp / MLX](./streamlit-exports.md) | One-click deploy configs from any plan |

Recorded and verified via `apps/streamlit/journeys/scripts/record-all.sh` + `verify-manifests.sh`.

## CLI journeys (live)

### Core Workflows

| Journey | What it demonstrates |
|---|---|
| [plan — DeepSeek-V3](./cli-plan-deepseek.md) | Memory planner with MLA classification |
| [plan --help](./cli-plan-help.md) | Help output for the planner subcommand |
| [probe list](./cli-probe-list.md) | GPU device enumeration, JSON output |
| [probe watch (Ctrl+C)](./cli-probe-watch.md) | Streaming telemetry + clean shutdown |
| [ingest error UX](./cli-ingest-error.md) | Fail-loudly error path (NFR-007) |

### Installation & Setup

| Journey | What it demonstrates |
|---|---|
| Install from Source | Clone, build, and verify hwLedger installation |
| First Plan | Live memory planning with colored VRAM breakdown |

### Fleet Management

| Journey | What it demonstrates |
|---|---|
| Fleet Register | Register a device to the fleet network |
| Fleet Audit | Audit fleet health and device status |

### Model Ingestion

| Journey | What it demonstrates |
|---|---|
| Ingest (Hugging Face) | Fetch model metadata from Hugging Face hub |
| Ingest (Ollama) | Query local Ollama server for model info |

### Releases & Quality

| Journey | What it demonstrates |
|---|---|
| Release (Signed DMG) | Create signed macOS releases with notarization |
| Traceability Report | Generate spec -> test -> code traceability report |

Recorded and verified via `apps/cli-journeys/scripts/record-all.sh` + `verify-manifests.sh`. Without an `ANTHROPIC_API_KEY`, verification runs against a local mock server so the pipeline exercises end-to-end offline.

## GUI journeys (macOS)

Requires a built `.app` bundle (`apps/macos/HwLedgerUITests/scripts/bundle-app.sh`) and optional ScreenCaptureKit permission.

```bash
./apps/macos/HwLedgerUITests/scripts/run-journeys.sh
```

### Planner with Qwen2-7B (32K context)

<JourneyViewer manifest="/journeys/planner-qwen2-7b-32k/manifest.json" />

### Core GUI journeys

| Journey | What it demonstrates |
|---|---|
| [Planner launch & slider](./gui-planner-launch.md) | App launch, Accessibility nav, seq-len slider drag |
| [Probe live telemetry watch](./gui-probe-watch.md) | Telemetry subscription, rolling sparkline, expand-to-detail |
| [Fleet Map agent discovery](./gui-fleet-map.md) | Fleet Map canvas, agent node spawn, host detail side panel |
| [Settings mTLS admin cert](./gui-settings-mtls.md) | Generate admin client cert, Copy PEM, "Copied" toast |
| [Planner export to vLLM flags](./gui-export-vllm.md) | Load fixture, Export > vLLM flags, copy generated flag string |

### More journeys to come

- Planner with Mixtral-8x7B (cost/speed tradeoff)
- Fleet dispatch workflow
- Device registration and provisioning
- Ledger viewer with hash-chain verification

## Journey Format

Each journey is a JSON manifest with:

```json
{
  "title": "Planner with Qwen2-7B",
  "intent": "Demonstrate KV cache calculation and live breakdown",
  "pass": true,
  "recording": true,
  "keyframes": [
    {
      "path": "/journeys/planner-qwen2-7b-32k/frame-001.png",
      "caption": "Launch planner, select Qwen2-7B"
    }
  ],
  "steps": [
    {
      "slug": "select_model",
      "intent": "Choose model from HF hub",
      "screenshot": "/journeys/planner-qwen2-7b-32k/thumb-select.png",
      "description": "User clicks model selector and searches for 'qwen2'",
      "judge_score": 0.95
    }
  ]
}
```

## Contributing Journeys

If you'd like to contribute a journey:

1. Run the recorder with your workflow
2. Verify the manifest is valid JSON
3. Submit a PR to add the journey folder to `apps/macos/build/journeys/`

See [CONTRIBUTING.md](https://github.com/KooshaPari/hwLedger/blob/main/CONTRIBUTING.md) for details.
