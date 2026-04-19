# oMlx fork patch series

Divergences from `jundot/omlx` upstream (tracked as remote `upstream` in the submodule at `../omlx-fork/`).

Numbered patch files follow `NNNN-kebab-title.patch`. Patches are applied in order on top of a clean upstream checkout; they may be rebased during weekly upstream-sync passes.

## Policy

- **Do not edit upstream files directly** without a matching numbered patch here. Reason: future re-forks or upstream rebases need a deterministic replay.
- **hwLedger-specific features** (JSON-RPC stdio protocol, layer-wise memory RPCs, deterministic benchmark hooks) live under `omlx/hwledger_rpc.py` and related files; their introductions are captured as patches here.
- **Non-hwLedger-specific improvements** should be upstreamed first; cherry-pick back if merged, drop the local patch once included upstream.

## Current series

### [0001-hwledger-rpc.md](0001-hwledger-rpc.md)
JSON-RPC 2.0 stdio server for hwLedger inference sidecar (WP20, FR-INF-002).
Files: `omlx/hwledger_rpc.py`, `omlx/__main_hwledger__.py`.
