# Dev Setup

One-liner for local development:

```bash
cargo run -p hwledger-dev-harness -- up
```

That single command:

1. Verifies your toolchain (`cargo`, `bun`, `uv`; warns on missing `vhs` / `ffmpeg` / `tesseract`).
2. Builds `hwledger-ffi`, `hwledger-cli`, and `hwledger-server` in `--release` mode in parallel.
3. Launches three background services, writes their PIDs to `~/.hwledger/dev-harness.pid`, and tails a colorized combined log.

## Services started

| Service    | Default port | Log file                             |
|------------|--------------|--------------------------------------|
| `server`   | `8080` (HTTP, dev-plain) | `~/.hwledger/logs/server.log` |
| `docs-site` | `5173` (VitePress)       | `~/.hwledger/logs/docs-site.log` |
| `streamlit` | `8511`                    | `~/.hwledger/logs/streamlit.log` |

Override the base with `--port-base 9000` (server becomes `9080`, streamlit `9511`; docs-site stays on `5173`).

Pick a subset:

```bash
cargo run -p hwledger-dev-harness -- up --clients cli,streamlit
```

Stop everything:

```bash
cargo run -p hwledger-dev-harness -- down
```

Status:

```bash
cargo run -p hwledger-dev-harness -- status
```

## Sample combined log

```
[   server] listening on 127.0.0.1:8080
[docs-site] vite v5.4.0  dev server running at:
[docs-site]   ➜  Local:   http://localhost:5173/
[streamlit] [hwledger-ffi] auto-building: cargo build --release -p hwledger-ffi
[streamlit]    Compiling hwledger-ffi v0.0.1
[streamlit] You can now view your Streamlit app in your browser.
[streamlit]   Local URL: http://localhost:8511
```

Prefix colors: `server`=cyan, `docs-site`=green, `streamlit`=magenta, rotating for additional services.

## Troubleshooting

### FFI missing — Streamlit shows "hwledger-ffi build failed"

By default Streamlit auto-builds the FFI dylib on first boot. If the build fails you get the `cargo` log inline. Fix the Rust error, then refresh the page — the lock at `~/.hwledger/ffi-build.lock` ensures concurrent Streamlit pages won't race.

CI should set `HWLEDGER_SKIP_FFI_AUTOBUILD=1` so the artifact must be pre-built:

```bash
HWLEDGER_SKIP_FFI_AUTOBUILD=1 streamlit run apps/streamlit/app.py
```

### Server cert error ("alert unknown ca")

The dev harness starts `hwledger-server` with `--dev` (plain HTTP) on purpose to sidestep the mTLS bootstrap. For full mTLS testing, run the server manually via `cargo run -p hwledger-server` (it will emit `ca.crt` / `ca.key` in the repo root on first boot).

### Streamlit hot-reload

Streamlit hot-reloads Python edits automatically. The harness's `streamlit` child also honours `HWLEDGER_FFI_PATH` so you can point it at a freshly-built dylib without restarting:

```bash
HWLEDGER_FFI_PATH=/tmp/libhwledger_ffi.dylib cargo run -p hwledger-dev-harness -- up --clients streamlit
```

For Rust-side hot-reload (rebuild FFI on `.rs` save), use the sibling watcher:

```bash
cargo run -p hwledger-devtools --bin hwledger-streamlit-dev
```

### Swift client: "hwLedger engine missing" sheet

The macOS app calls `HwLedger.ffiAvailability()` on launch. When `libhwledger_ffi.dylib` is absent it shows a sheet with the exact command: `cargo build --release -p hwledger-ffi`. Or just run the harness once — the XCFramework produced by `scripts/build-xcframework.sh` embeds the dylib for packaged builds.
