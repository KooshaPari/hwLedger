# UI Journeys

Interactive recordings of user workflows in the hwLedger GUI. Each journey captures intent, keyframes, and verification scores.

## How to Record Journeys

Run the journey recorder on macOS:

```bash
./apps/macos/HwLedgerUITests/scripts/run-journeys.sh
```

Outputs are saved to `apps/macos/build/journeys/` and synced to this site at build time.

## Available Journeys

### Planner with Qwen2-7B (32K context)

The first journey demonstrates the planner UI with a small, fast model.

<JourneyViewer manifest="/journeys/planner-qwen2-7b-32k/manifest.json" />

### More Journeys Coming

Additional journeys will be recorded as the GUI is developed:

- Planner with Mixtral-8x7B (cost/speed tradeoff)
- Fleet dispatch workflow
- Device registration and provisioning
- Event log viewer and drill-down

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
