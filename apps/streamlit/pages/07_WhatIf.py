"""
What-If / Predict page.

Let the operator pick a baseline plan, a candidate plan, and a set of
transformation techniques (INT4, KV-FP8, LoRA, REAP, SpecDecode, FlashAttn3, …)
then see a side-by-side Plotly bar comparison, a verdict, and the citations
backing each technique's claimed effect.

Traces to: brief §3 (what-if / predict wiring).
"""

from __future__ import annotations

import plotly.graph_objects as go
import streamlit as st

from lib.ffi_ext import (
    PredictBreakdown,
    WhatIfResult,
    backend_status,
    list_techniques,
    predict_available,
    whatif,
)


st.set_page_config(page_title="What-If - hwLedger", layout="wide")
st.title("What-If · Memory Prediction")
st.markdown(
    "Compare a baseline plan against a candidate under transformation "
    "techniques (quantization, KV compression, REAP pruning, LoRA, "
    "speculative decoding, FlashAttn-3). All deltas come from the Rust "
    "`hwledger_predict_*` FFI; if the sibling crate isn't built yet, a "
    "deterministic mock keeps the UI honest."
)

status = backend_status()
if not status.ffi_predict:
    st.info(
        "Sibling `hwledger_predict_whatif` FFI symbol is not yet exported; "
        "showing deterministic-mock deltas with real published citations."
    )


# --- Baseline ---
st.subheader("1 · Baseline")
base_source = st.radio(
    "Source",
    ["Use latest Planner result", "Enter manually"],
    horizontal=True,
)

latest = st.session_state.get("latest_plan")
if base_source == "Use latest Planner result":
    if latest is None:
        st.warning("No planner result yet. Visit the Planner page once, or "
                   "pick 'Enter manually'.")
        st.stop()
    baseline = PredictBreakdown(
        weights_mb=latest["weights_mb"],
        kv_mb=latest["kv_mb"],
        prefill_mb=latest["prefill_mb"],
        runtime_mb=latest["runtime_mb"],
    )
    st.caption(
        f"Baseline: weights {baseline.weights_mb:.0f} MB · "
        f"KV {baseline.kv_mb:.0f} MB · "
        f"prefill {baseline.prefill_mb:.0f} MB · "
        f"runtime {baseline.runtime_mb:.0f} MB"
    )
else:
    mcol = st.columns(4)
    weights = mcol[0].number_input("Weights MB", value=14000.0, step=100.0)
    kvmb = mcol[1].number_input("KV MB", value=2000.0, step=100.0)
    prefmb = mcol[2].number_input("Prefill MB", value=400.0, step=50.0)
    runmb = mcol[3].number_input("Runtime MB", value=800.0, step=50.0)
    baseline = PredictBreakdown(weights, kvmb, prefmb, runmb)


# --- Techniques ---
st.subheader("2 · Techniques to apply")
techniques = st.multiselect(
    "Pick one or more",
    options=list_techniques(),
    default=["INT4", "KV-FP8"],
    help="Each technique applies a published multiplier to the baseline bands.",
)


# --- Run ---
st.subheader("3 · Result")
result: WhatIfResult = whatif(baseline, techniques)

# Side-by-side bars
bands = ["Weights", "KV", "Prefill", "Runtime"]
baseline_vals = [result.baseline.weights_mb, result.baseline.kv_mb,
                 result.baseline.prefill_mb, result.baseline.runtime_mb]
candidate_vals = [result.candidate.weights_mb, result.candidate.kv_mb,
                  result.candidate.prefill_mb, result.candidate.runtime_mb]

fig = go.Figure()
fig.add_trace(go.Bar(name="Baseline", x=bands, y=baseline_vals,
                     marker_color="#888"))
fig.add_trace(go.Bar(name="Candidate", x=bands, y=candidate_vals,
                     marker_color="#BB77DD"))
fig.update_layout(
    barmode="group", height=320,
    yaxis_title="Memory (MB)",
    margin=dict(l=0, r=0, t=20, b=0),
)
st.plotly_chart(fig, use_container_width=True, key="whatif_bars")

# Verdict
delta = result.delta_pct
verdict_color = ("#28a745" if delta <= -10 else
                 ("#ffc107" if delta <= 0 else "#dc3545"))
st.markdown(
    f"<div style='padding:14px;border-left:4px solid {verdict_color};"
    f"background:rgba(0,0,0,0.03);margin:16px 0;'>"
    f"<strong>Verdict · Δ {delta:+.1f}%</strong><br>{result.verdict}"
    "</div>",
    unsafe_allow_html=True,
)

# Totals
tc1, tc2, tc3 = st.columns(3)
tc1.metric("Baseline total", f"{result.baseline.total_mb:.0f} MB")
tc2.metric("Candidate total", f"{result.candidate.total_mb:.0f} MB",
           f"{delta:+.1f}%")
tc3.metric("Techniques applied", len(result.techniques))

# Citations
if result.citations:
    st.subheader("Citations")
    cite_rows = [{
        "Technique": c.technique,
        "Title": c.title,
        "arXiv": c.arxiv_id or "",
        "URL": c.url,
    } for c in result.citations]
    st.dataframe(cite_rows, use_container_width=True, hide_index=True)
