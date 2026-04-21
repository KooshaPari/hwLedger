"""
Export page: emit deploy-ready configs for the latest Planner result.

Consolidates the three inline Planner export buttons into a single
screen with copy + download actions for each target:
- vLLM CLI args
- llama.cpp CLI args
- MLX JSON config
"""

from __future__ import annotations

import json

import streamlit as st

from lib.ffi import export_llama_cpp, export_mlx, export_vllm, is_available


st.set_page_config(page_title="Export - hwLedger", layout="wide")
st.title("Export")
st.markdown("Emit deploy-ready configs for the latest Planner result.")

if not is_available():
    st.error("FFI library not loaded. Run `cargo build --release -p hwledger-ffi`.")
    st.stop()

latest = st.session_state.get("latest_plan")
if latest is None:
    st.info("Open the Planner page first, then return here to export.")
    st.stop()

st.caption(
    f"Current plan: seq={latest['seq_len']} · "
    f"users={latest['concurrent_users']} · "
    f"batch={latest['batch_size']} · "
    f"kv_q={latest['kv_quant']} · w_q={latest['weight_quant']}"
)


def _call(fn) -> str:
    val = fn(
        config_json=latest["config_json"],
        seq_len=latest["seq_len"],
        concurrent_users=latest["concurrent_users"],
        batch_size=latest["batch_size"],
        kv_quant=latest["kv_quant"],
        weight_quant=latest["weight_quant"],
    )
    return val or ""


tabs = st.tabs(["vLLM", "llama.cpp", "MLX"])

with tabs[0]:
    out = _call(export_vllm)
    st.code(out or "(no output)", language="bash")
    if out:
        st.download_button("Download vllm-args.sh", f"#!/bin/bash\nvllm serve {out}\n",
                           file_name="vllm-args.sh", mime="text/x-shellscript")

with tabs[1]:
    out = _call(export_llama_cpp)
    st.code(out or "(no output)", language="bash")
    if out:
        st.download_button("Download llama-args.sh",
                           f"#!/bin/bash\n./llama-server {out}\n",
                           file_name="llama-args.sh", mime="text/x-shellscript")

with tabs[2]:
    out = _call(export_mlx)
    st.code(out or "(no output)", language="json")
    if out:
        try:
            json.loads(out)  # validate
            st.download_button("Download mlx-config.json", out,
                               file_name="mlx-config.json",
                               mime="application/json")
        except Exception:
            st.warning("MLX output is not valid JSON; download disabled.")
