# hwLedger — AGENTS.md

## Project Overview

Hardware wallet ledger companion app — bridges hardware security devices with the Ledger ecosystem.

## Stack

- Language: Rust (per GitHub language detection)
- Platform: Desktop / Embedded
- Build system: Cargo (verify `Cargo.toml`)

## Key Commands

```bash
# Verify project structure
ls -la Cargo.toml Cargo.lock rust-toolchain.toml 2>/dev/null

# Build
cargo build --release

# Test
cargo test

# Lint
cargo clippy
```

## Notes

- **Active** — verify language and build system locally before running commands
- Hardware wallet integration — may require device/USB access for full testing
