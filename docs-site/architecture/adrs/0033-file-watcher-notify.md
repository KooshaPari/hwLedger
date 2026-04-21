# ADR 0033 — File-watcher: `notify` (Rust)

Constrains: FR-DEV-001, FR-DOCS-006

Date: 2026-04-19
Status: Accepted

## Context

Hot-reload surfaces across the workspace: docs-site dev server, `hwledger-journey watch`, and the dashboard auto-sync between `hwledger-server` and the Streamlit frontend. We need a file-watcher that is Rust-native (scripting policy tier-1 language), cross-platform, and debounced.

## Options

| Option | Language | Platforms | Debounce | Deep-watch | Maintainer |
|---|---|---|---|---|---|
| notify 6 | Rust | mac (fsevents), linux (inotify), win (ReadDirectoryChangesW), bsd (kqueue) | Yes (notify-debouncer-mini) | Yes | Active |
| watchexec | Rust binary | Same as notify | Yes | Yes | Active |
| chokidar | Node | Cross | Yes | Yes | Active |
| watchman | C++ daemon | Cross | Yes | Yes | Meta |
| inotifywait | Linux C | Linux only | External | No | stable |

## Decision

Use **notify 6** with `notify-debouncer-mini 0.4` embedded directly into binaries that need watching. `watchexec` is the canonical CLI wrapper for one-off shell-invocation workflows (e.g., `watchexec -w src -- cargo run`).

## Rationale

- Rust-native aligns with the scripting policy: watchers embed into Rust binaries without shelling out or adding a non-Rust runtime.
- notify + debouncer-mini covers all four supported OS kernel APIs behind one event enum; no OS-specific code paths in user land.
- watchman is excellent but is an external daemon (Meta-maintained) — adds a process to manage.
- chokidar is Node-only; wrong dep graph for Rust tools.

## Consequences

- notify on macOS uses FSEvents which has ~1 s coalescing by default; we debounce to 250 ms to smooth out.
- Deep symlink traversal is opt-in; our use cases never need it.
- Watching very large trees (>100k files) can hit inotify watch limits on Linux; we document the sysctl bump in `CONTRIBUTING.md`.

## Revisit when

- notify 7 ships with breaking API.
- A kernel-level watcher (e.g., fanotify, macOS's new Endpoint Security watchers) supersedes current primitives.

## References

- notify: https://github.com/notify-rs/notify
- watchexec: https://github.com/watchexec/watchexec
