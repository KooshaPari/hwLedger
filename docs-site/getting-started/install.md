# Installation

Get hwLedger running on your system.

## Requirements

- **Rust**: 1.70+ (MSRV)
- **OS**: macOS 12+, Windows 10+, Linux (Ubuntu 20.04+)
- **RAM**: 8 GB minimum (16 GB recommended)
- **GPU** (optional): NVIDIA, AMD, or Apple Silicon for inference

## Build from Source

<RecordingEmbed tape="install-from-source" caption="Clone, build, and verify installation" />

<RecordingEmbed tape="install-cargo" caption="cargo install: the same path the GitHub Actions matrix uses, run locally" />

<Shot src="/cli-journeys/keyframes/install-cargo/frame-005.png"
      caption="Final link step before the binary lands on $PATH"
      size="small" align="right" />

### 1. Clone the repository

<Shot src="/cli-journeys/keyframes/install-cargo/frame-003.png"
      caption="cargo install — download"
      size="small" align="right" />

```bash
git clone https://github.com/KooshaPari/hwLedger.git
cd hwLedger
```

<Shot src="/cli-journeys/keyframes/install-cargo/frame-002.png"
      caption="Compile progress"
      size="small" align="left" />

<Shot src="/cli-journeys/keyframes/install-cargo/frame-004.png"
      caption="Binary installed and on PATH"
      size="small" align="right" />

### 2. Install Rust (if not already installed)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 3. Build the CLI

```bash
cargo build --release
```

The binary will be at `target/release/hwledger-cli`.

### 4. (Optional) Build native GUI

#### macOS (SwiftUI)

```bash
cd apps/macos
xcodebuild -scheme HwLedger -configuration Release
```

#### Windows (WinUI 3)

```bash
cd apps/windows
dotnet publish -c Release
```

#### Linux (Qt 6)

```bash
cd apps/linux-qt
cargo build --release
```

## Quick Test

Run the planner to verify installation:

```bash
./target/release/hwledger-cli plan --model llama-2-70b
```

Expected output (example):

```
hwLedger Capacity Planner

Model: llama-2-70b
├─ Attention: MHA
├─ Num layers: 80
├─ Hidden size: 4096
└─ Num heads: 64

KV Cache (batch_size=1, seq_length=4096, dtype=float16):
├─ Per layer: 67 MB
├─ Total: 5.3 GB

Model Weights (dtype=float16):
├─ Parameters: 70 B
├─ Size: 140 GB

Total VRAM needed: ~146 GB
```

## Installation via Homebrew (coming soon)

Once released, install via Homebrew:

```bash
brew install hwledger
```

## Docker (coming soon)

Run in Docker:

```bash
docker run -it --gpus all ghcr.io/kooshapari/hwledger:latest
```

## Next Steps

- [Plan a model](/math/kv-cache) — understand VRAM and throughput
- [View architecture](/architecture/) — system design overview
- [Fleet setup](/fleet/overview) — configure heterogeneous hardware
- [Build from source](https://github.com/KooshaPari/hwLedger#development) — contribute to hwLedger

## Troubleshooting

### Rust compilation fails

Ensure you have the latest Rust:

```bash
rustup update
```

### macOS: Xcode not found

Install Xcode command-line tools:

```bash
xcode-select --install
```

### Windows: .NET 9 not found

Install [.NET 9 SDK](https://dotnet.microsoft.com/download).

### Linux: Qt 6 not found

```bash
# Ubuntu/Debian
sudo apt-get install qt6-base-dev qt6-qml-dev

# Fedora
sudo dnf install qt6-qtbase-devel qt6-qtdeclarative-devel
```

## Support

- [GitHub Issues](https://github.com/KooshaPari/hwLedger/issues)
- [Documentation](/architecture/)
- [Research](/research/)
