"""Python entrypoint wrapper that prebuilds FFI (or explicitly opts out) then
runs `streamlit run app.py`.

Intentionally small — most logic lives in `lib/ffi.py`. This exists so the
`hwledger-dev-harness` binary (tools/dev-harness) can shell to `uv run python
-m apps.streamlit.run` for a single idempotent command and users who prefer a
one-liner get the same behavior without inventing shell glue (brief §5 Rust-
only mandate still holds — this is Python glue for the Python client, not a
shared dev-loop script).
"""

from __future__ import annotations

import os
import sys
from pathlib import Path


def main() -> int:
    here = Path(__file__).resolve().parent
    # Touching the FFI module runs the auto-build path. If it fails, we still
    # want Streamlit to start so users see the in-page error rather than an
    # exit-1 from the launcher.
    sys.path.insert(0, str(here))
    try:
        from lib import ffi  # noqa: F401  — side-effect: load+build
    except Exception as e:  # pragma: no cover — defensive
        print(f"[run.py] ffi import failed (continuing): {e}", file=sys.stderr)

    port = os.environ.get("HWLEDGER_STREAMLIT_PORT", "8511")
    import streamlit.web.cli as stcli  # type: ignore

    sys.argv = [
        "streamlit",
        "run",
        str(here / "app.py"),
        "--server.port",
        port,
        "--server.headless",
        os.environ.get("HWLEDGER_STREAMLIT_HEADLESS", "true"),
    ]
    return stcli.main()  # type: ignore[no-any-return]


if __name__ == "__main__":
    raise SystemExit(main())
