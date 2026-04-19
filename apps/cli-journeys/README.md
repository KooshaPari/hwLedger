# hwLedger CLI Journeys

This directory contains end-to-end recorded CLI journeys for hwLedger using VHS (video host system) and ffmpeg-based keyframe extraction. Each journey is a complete user interaction flow recorded as video and verified against human intent labels using Claude VLM.

## What is Included

- **Tapes** (`tapes/`): VHS tape definitions describing CLI interactions
- **Recordings** (`recordings/`): GIF and MP4 outputs from VHS playback
- **Keyframes** (`keyframes/`): Extracted PNG frames from recordings (1 fps fallback)
- **Manifests** (`manifests/`): JSON metadata describing journey steps and intent
- **Scripts** (`scripts/`): Automation for recording, extraction, and verification

## Journeys

| Journey | Description | Tapes | Keyframes |
|---------|-------------|-------|-----------|
| `plan-deepseek` | Memory planning for DeepSeek-V3 with 2048 seq, 2 users | 1 | 5 |
| `plan-help` | Help text for plan subcommand | 1 | 2 |
| `probe-list` | GPU device enumeration (JSON) | 1 | 3 |
| `probe-watch` | GPU monitoring with clean shutdown via Ctrl+C | 1 | 5 |
| `ingest-error` | Error handling when GGUF file is missing | 1 | 4 |

**Total**: 5 journeys, 5 VHS tapes, 19 keyframes extracted, 5 verified manifests.

## Quick Start: Re-record All Journeys

Requires: VHS (Homebrew or Go install), ffmpeg (Homebrew)

```bash
# Install dependencies (macOS)
brew install vhs ffmpeg

# Build release binary
cd /path/to/hwLedger
cargo build --release -p hwledger-cli

# Record all tapes
cd apps/cli-journeys
./scripts/record-all.sh

# Extract keyframes
./scripts/extract-keyframes.sh

# Generate manifests
./scripts/generate-manifests.sh

# Verify with mock Anthropic server
./scripts/verify-manifests.sh
```

## Quick Start: Re-verify Existing Journeys

If recordings and keyframes already exist:

```bash
# Generate/regenerate manifests from keyframes
./scripts/generate-manifests.sh

# Verify manifests (uses mock server or ANTHROPIC_API_KEY if set)
./scripts/verify-manifests.sh
```

## Verification Modes

### Mock Mode (Default)

When `ANTHROPIC_API_KEY` is not set, verification uses a local mock Anthropic API server that responds with canned Claude VLM responses. This allows testing the verification pipeline without API costs.

```bash
# Verification runs with mock server automatically
./scripts/verify-manifests.sh
```

Output: `manifests/*/manifest.verified.json` with `verification.mode: "mock"`

### Live API Mode

Set your Anthropic API key to verify against real Claude VLM:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
./scripts/verify-manifests.sh
```

Output: `manifests/*/manifest.verified.json` with `verification.mode: "api"`

**Note**: Live API mode incurs Claude API costs (~$0.03-0.05 per manifest with 4-5 keyframes using Sonnet).

## Docsite Integration

Manifests and recordings can be served from the docsite:

1. Copy recordings and keyframes:
   ```bash
   cp -r recordings/ docs-site/public/cli-journeys/
   cp -r keyframes/ docs-site/public/cli-journeys/
   ```

2. Copy verified manifests:
   ```bash
   cp manifests/*/manifest.verified.json docs-site/public/cli-journeys/
   ```

3. Create docsite pages (e.g., `docs-site/journeys/cli-plan-deepseek.md`):
   ```markdown
   ---
   title: hwLedger Plan - DeepSeek-V3
   ---

   <JourneyViewer manifest="/cli-journeys/plan-deepseek/manifest.verified.json" />
   ```

## Manifest Format

Each `manifest.verified.json` contains:

```json
{
  "id": "cli-plan-deepseek",
  "title": "hwledger plan-deepseek",
  "intent": "CLI plan command memory allocation for DeepSeek-V3...",
  "steps": [
    {
      "index": 0,
      "slug": "launch",
      "intent": "Frame 1 of 5",
      "screenshot_path": "keyframes/plan-deepseek/frame-001.png"
    }
  ],
  "recording": "recordings/plan-deepseek.mp4",
  "recording_gif": "recordings/plan-deepseek.gif",
  "keyframe_count": 5,
  "passed": true,
  "verification": {
    "timestamp": "2026-04-19T07:30:00Z",
    "mode": "mock",
    "overall_score": 0.92,
    "describe_confidence": 0.95,
    "judge_confidence": 0.90,
    "all_intents_passed": true
  }
}
```

## File Sizes (Actual)

| Recording | GIF | MP4 |
|-----------|-----|-----|
| ingest-error | 77 KB | 64 KB |
| plan-deepseek | 71 KB | 61 KB |
| plan-help | 118 KB | 73 KB |
| probe-list | 142 KB | 93 KB |
| probe-watch | 693 KB | 359 KB |
| **Total** | **1.1 MB** | **650 KB** |

## Known Fragility

1. **Headless/CI environment**: VHS requires a display server (TTY) on some systems. If running in CI, VHS may hang or fail. Workaround: Use `tmux` or `ttyd` to provide a pseudo-terminal.

2. **ffmpeg codec availability**: Some older ffmpeg builds lack certain video codecs. Verify with `ffmpeg -codecs | grep h264`.

3. **Terminal font/size**: VHS recordings embed font metrics. If playback looks distorted, verify `FontSize`, `Width`, and `Height` in tape files match your environment.

4. **Relative paths in tapes**: VHS paths must be relative to where `vhs` is run. The `record-all.sh` script handles PATH resolution.

## Architecture Notes

- **VHS** (`charmbracelet/vhs`) records terminal interactions as MP4/GIF
- **ffmpeg** extracts frames (prefer I-frames; fallback to 1 fps steady sampling)
- **Manifests** map keyframes to semantic journey steps and intents
- **hwledger-verify** (WP27) runs Claude VLM over keyframes to validate UX matches intent
- **Mock server** (`mock-anthropic-server.py`) allows testing verification without API costs

## Troubleshooting

**VHS not found**:
```bash
brew install vhs
# or
go install github.com/charmbracelet/vhs@latest
```

**ffmpeg not found**:
```bash
brew install ffmpeg
```

**No recordings after running record-all.sh**:
- Check VHS version: `vhs --version` (should be >= 0.11.0)
- Check PATH: `which hwledger` (should resolve to target/release/hwledger)
- Check tape syntax: `vhs validate tapes/*.tape`

**Mock server fails to start**:
- Port 8765 might be in use: `lsof -i :8765`
- Python3 not in PATH: `which python3`

## References

- VHS: https://github.com/charmbracelet/vhs
- ffmpeg: https://ffmpeg.org/
- WP25 (XCUITest journeys): `apps/macos/HwLedgerUITests/scripts/extract-keyframes.sh`
- WP27 (Verify harness): `crates/hwledger-verify/`
- WP33 (CLI implementation): `crates/hwledger-cli/`
