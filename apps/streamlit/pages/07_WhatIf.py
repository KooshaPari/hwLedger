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

# Dismissible mock banner with "why" expander + "Try live FFI" button.
if "whatif_banner_dismissed" not in st.session_state:
    st.session_state.whatif_banner_dismissed = False
if "whatif_ffi_probe" not in st.session_state:
    st.session_state.whatif_ffi_probe = None  # None | "ok" | error string

if not status.ffi_predict and not st.session_state.whatif_banner_dismissed:
    banner = st.container()
    with banner:
        bcol1, bcol2, bcol3 = st.columns([5, 2, 1])
        with bcol1:
            st.info(
                "Using deterministic-mock predictions. The sibling "
                "`hwledger_predict_whatif` FFI symbol is not currently "
                "exported by `libhwledger_ffi`; citations and deltas shown "
                "below come from published multipliers, not the live crate."
            )
        with bcol2:
            if st.button("Try live FFI", use_container_width=True, key="whatif_try_ffi"):
                # Attempt to dlopen / probe the symbol.
                try:
                    import ctypes
                    from lib.ffi import lib as _ffi_lib  # type: ignore
                    if _ffi_lib is None:
                        raise RuntimeError(
                            "libhwledger_ffi is not loaded. "
                            "Build it with `cargo build --release -p hwledger-ffi`."
                        )
                    sym_names = [
                        "hwledger_predict_whatif",
                        "hwledger_predict",
                    ]
                    found = None
                    last_err = None
                    for name in sym_names:
                        try:
                            fn = getattr(_ffi_lib, name)
                            # Ensure it looks like a real extern C function.
                            if isinstance(fn, ctypes._CFuncPtr) or callable(fn):
                                found = name
                                break
                        except AttributeError as e:
                            last_err = e
                    if found:
                        st.session_state.whatif_ffi_probe = f"ok:{found}"
                    else:
                        raise AttributeError(
                            f"symbol not exported (tried {sym_names}): {last_err}"
                        )
                except Exception as e:
                    st.session_state.whatif_ffi_probe = (
                        f"{type(e).__name__}: {e}"
                    )
                st.rerun()
        with bcol3:
            if st.button("Dismiss", use_container_width=True, key="whatif_dismiss"):
                st.session_state.whatif_banner_dismissed = True
                st.rerun()

        probe = st.session_state.whatif_ffi_probe
        if probe:
            if probe.startswith("ok:"):
                st.success(f"Live FFI available via symbol `{probe[3:]}`.")
            else:
                st.error(f"Live FFI dlopen failed — exact error:\n\n`{probe}`")

        with st.expander("Why am I seeing mocks?"):
            st.markdown(
                "- The Streamlit app loads `libhwledger_ffi` via `ctypes` at "
                "import time (see `apps/streamlit/lib/ffi.py`).\n"
                "- The sibling **predict** crate (`crates/hwledger-predict`) "
                "exports `hwledger_predict`/`hwledger_predict_whatif` behind a "
                "feature that has not been wired into the default FFI build.\n"
                "- Until the symbol is present, this page falls back to a "
                "deterministic-mock that applies published technique "
                "multipliers (GPTQ, KIVI, LoRA, REAP, FlashAttn-3…) and "
                "shows the real citations.\n"
                "- To switch to live:\n"
                "    1. `cargo build --release -p hwledger-ffi "
                "--features predict`\n"
                "    2. Confirm `nm target/release/libhwledger_ffi.{so,dylib} "
                "| grep predict` shows the exported symbol.\n"
                "    3. Restart Streamlit and click **Try live FFI** to "
                "verify the dlopen."
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


# ---------------------------------------------------------------------------
# Architecture-swap BUFFET
# ---------------------------------------------------------------------------
# Stack multiple candidate changes and see composed predictions. Each option
# is a card with: VRAM delta, TPS delta, quality delta, cost-to-achieve, time.
# Sections:
#   3a  Quantization swap
#   3b  KV cache swap
#   3c  Attention architecture swap
#   3d  Window / context-length swap
#   3e  Composed forecast
#
# Formulas & citations:
#   - Quantization quality: QLoRA (Dettmers 2023) arxiv:2305.14314
#   - GQA/MQA: GQA paper (Ainslie 2023) arxiv:2305.13245
#   - MLA: DeepSeek-V2 (Dai 2024) arxiv:2405.04434
#   - YaRN: Peng 2023 arxiv:2309.00071
#   - LongRoPE: Ding 2024 arxiv:2402.13753
#   - Paged attention: vLLM (Kwon 2023) arxiv:2309.06180
#   - MLA KV footprint: DeepSeek-V2 reports KV compression factor ~7x vs MHA

from lib.cost_model import retraining_cost, data_needed_tokens, gpu_price_per_hour  # noqa: E402


st.divider()
st.header("Architecture Swap Buffet")
st.caption(
    "Stack mechanical (quantization) and architectural (MHA→MLA) changes. "
    "Each card shows deltas + cost-to-achieve. Check the boxes to include a "
    "change in the composed forecast below."
)

latest = st.session_state.get("latest_plan") or {}
_baseline_total_mb = latest.get("adjusted_total_mb") or (
    baseline.weights_mb + baseline.kv_mb + baseline.prefill_mb + baseline.runtime_mb
)
_baseline_tps = latest.get("tps") or 0.0
_attention = (latest.get("attention_kind") or "mha").lower()
# Rough parameter-count estimate from weights assuming BF16 (2 bytes/param).
_params_est = int((latest.get("weights_mb") or baseline.weights_mb) * 1024 * 1024 / 2)

# -- 3a · Quantization --
with st.container(border=True):
    st.subheader("3a · Quantization Swap")
    st.caption(
        "Low-cost, mechanical. Runs on-device in minutes. "
        "Quality delta proxied by QLoRA PPL tables (arxiv:2305.14314)."
    )
    qopts = {
        "FP16 (baseline)": (1.00, 0.00),
        "Q8 / INT8": (0.50, 0.01),
        "Q5_K_M": (0.32, 0.03),
        "Q4_K_M": (0.25, 0.05),
        "Q3_K": (0.19, 0.10),
        "Q2_K": (0.13, 0.25),
        "AWQ INT4": (0.25, 0.02),
        "GPTQ INT4": (0.25, 0.03),
    }
    _q_pick = st.radio("Target quant", list(qopts.keys()), index=3,
                       horizontal=True, key="buf_quant")
    q_factor, q_ppl = qopts[_q_pick]
    _q_delta_mb = (latest.get("weights_mb") or baseline.weights_mb) * (q_factor - 1.0)
    c1, c2, c3 = st.columns(3)
    c1.metric("Weights VRAM Δ", f"{_q_delta_mb / 1024:+.2f} GB")
    c2.metric("Quality Δ (PPL)", f"+{q_ppl * 100:.1f}%")
    c3.metric("Cost to achieve", "0 USD · <1 h")
    q_enable = st.checkbox("Include in composed forecast", value=True, key="buf_quant_en")

# -- 3b · KV cache swap --
with st.container(border=True):
    st.subheader("3b · KV Cache Swap")
    st.caption(
        "Inference-engine config changes. Paged/MLA are near-zero quality "
        "cost; MQA/GQA KV need retrained base model."
    )
    kv_opts = {
        "Full KV (baseline)": (1.00, 0.00, "inference config"),
        "Paged KV (vLLM)":    (0.95, 0.00, "inference config"),
        "MLA KV":             (0.14, 0.02, "retrain / distill"),
        "MQA KV":             (0.13, 0.10, "retrain / distill"),
        "GQA KV (8:1)":       (0.25, 0.03, "retrain / distill"),
    }
    _kv_pick = st.radio("Target KV scheme", list(kv_opts.keys()), index=1,
                        horizontal=True, key="buf_kv")
    kv_factor, kv_quality, kv_path = kv_opts[_kv_pick]
    _kv_delta_mb = (latest.get("kv_mb") or baseline.kv_mb) * (kv_factor - 1.0)
    c1, c2, c3 = st.columns(3)
    c1.metric("KV VRAM Δ", f"{_kv_delta_mb / 1024:+.2f} GB")
    c2.metric("Quality Δ", f"+{kv_quality * 100:.1f}% PPL")
    c3.metric("Cost-path", kv_path)
    kv_enable = st.checkbox("Include in composed forecast", value=True, key="buf_kv_en")

# -- 3c · Attention architecture swap --
with st.container(border=True):
    st.subheader("3c · Attention Architecture Swap")
    st.caption(
        "Big cost — retraining required. See DeepSeek-V2 for MLA "
        "(arxiv:2405.04434), Ainslie 2023 for GQA (arxiv:2305.13245)."
    )
    att_opts = {
        "MHA (baseline)":     (_attention, "mha", 1.00, 0.00, "none"),
        "MHA → GQA (8:1)":    ("mha", "gqa", 1.18, 0.02, "gqa_distill"),
        "MHA → MQA":          ("mha", "mqa", 1.35, 0.10, "mqa_distill"),
        "GQA → MLA":          ("gqa", "mla", 1.20, 0.01, "mla_retrain"),
        "MHA → MLA":          ("mha", "mla", 1.42, 0.02, "mla_retrain"),
    }
    _att_pick = st.radio("Target attention", list(att_opts.keys()), index=3,
                         horizontal=True, key="buf_att")
    src, dst, tps_mult, att_quality, change_kind = att_opts[_att_pick]
    retrain = retraining_cost(parameter_count=_params_est, gpu_kind="H100")
    tokens_needed = data_needed_tokens(_params_est, change_kind)
    c1, c2, c3 = st.columns(3)
    c1.metric("TPS Δ", f"{(tps_mult - 1.0) * 100:+.0f}%")
    c2.metric("Quality Δ", f"+{att_quality * 100:.2f}% PPL")
    c3.metric(
        "Retraining est.",
        f"${retrain.usd_cost:,.0f}",
        help=(
            f"{retrain.gpu_hours:,.0f} H100-hours @ ${gpu_price_per_hour('H100'):.2f}/h; "
            f"~{tokens_needed / 1e9:.1f}B tokens; "
            f"~{retrain.wall_clock_days_at_8gpu:.1f} days on 8xH100. "
            f"{retrain.notes}"
        ),
    )
    with st.expander("Why this cost?"):
        st.markdown(
            f"- Compute = 6 × params × tokens = "
            f"6 × {_params_est / 1e9:.1f}B × {tokens_needed / 1e9:.1f}B FLOPs.\n"
            f"- H100 BF16 dense: 989 TFLOPS; MFU 45% typical distributed.\n"
            f"- **Risks:** catastrophic forgetting, eval drift, numerical "
            f"instability during re-init of collapsed KV projections (MLA).\n"
            f"- Papers: "
            f"[DeepSeek-V2](https://arxiv.org/abs/2405.04434) · "
            f"[GQA](https://arxiv.org/abs/2305.13245) · "
            f"[Chinchilla](https://arxiv.org/abs/2203.15556)"
        )
    att_enable = st.checkbox("Include in composed forecast", value=False, key="buf_att_en")

# -- 3d · Context-length swap --
with st.container(border=True):
    st.subheader("3d · Context-Length Swap")
    st.caption(
        "Sliding-window, ring-attention, rope-scaling, YaRN, LongRoPE. "
        "Cost ranges from inference-only config to full-pretrain extension."
    )
    ctx_opts = {
        "Sliding Window":  (0.60, 0.02, "config"),
        "Ring Attention":  (0.40, 0.00, "engine support"),
        "Rope-scaling 2x": (1.00, 0.05, "config"),
        "YaRN 4x":         (1.00, 0.02, "fine-tune ~1B tokens"),
        "LongRoPE 8x":     (1.00, 0.03, "fine-tune ~1B tokens"),
    }
    _ctx_pick = st.radio("Target scheme", list(ctx_opts.keys()), index=3,
                         horizontal=True, key="buf_ctx")
    ctx_factor, ctx_quality, ctx_path = ctx_opts[_ctx_pick]
    _ctx_delta_mb = ((latest.get("kv_mb") or baseline.kv_mb) +
                     (latest.get("prefill_mb") or baseline.prefill_mb)) * (ctx_factor - 1.0)
    c1, c2, c3 = st.columns(3)
    c1.metric("KV+Prefill VRAM Δ", f"{_ctx_delta_mb / 1024:+.2f} GB")
    c2.metric("Quality Δ", f"+{ctx_quality * 100:.1f}% PPL")
    c3.metric("Cost-path", ctx_path)
    ctx_enable = st.checkbox("Include in composed forecast", value=False, key="buf_ctx_en")

# -- 3e · Composed forecast --
with st.container(border=True):
    st.subheader("3e · Composed Forecast")
    st.caption(
        "Multiplicatively compose the checked cards. VRAM/TPS/quality deltas "
        "are combined; retraining cost is the max over retraining-required "
        "items (they share the same tokens budget)."
    )

    total_vram_delta_mb = 0.0
    total_tps_mult = 1.0
    total_quality_pct = 0.0
    total_retrain_usd = 0.0
    retrain_days = 0.0
    notes: list[str] = []

    if q_enable:
        total_vram_delta_mb += _q_delta_mb
        total_quality_pct += q_ppl * 100
        notes.append(f"Quant: {_q_pick}")
    if kv_enable:
        total_vram_delta_mb += _kv_delta_mb
        total_quality_pct += kv_quality * 100
        notes.append(f"KV: {_kv_pick}")
    if att_enable and tps_mult != 1.0:
        total_tps_mult *= tps_mult
        total_quality_pct += att_quality * 100
        total_retrain_usd = max(total_retrain_usd, retrain.usd_cost)
        retrain_days = max(retrain_days, retrain.wall_clock_days_at_8gpu)
        notes.append(f"Attn: {_att_pick}")
    if ctx_enable:
        total_vram_delta_mb += _ctx_delta_mb
        total_quality_pct += ctx_quality * 100
        notes.append(f"Ctx: {_ctx_pick}")

    new_total_gb = max(0.0, (_baseline_total_mb + total_vram_delta_mb) / 1024.0)
    baseline_gb = _baseline_total_mb / 1024.0
    pct_change = (new_total_gb - baseline_gb) / max(0.01, baseline_gb) * 100.0

    c1, c2, c3, c4 = st.columns(4)
    c1.metric("Total VRAM", f"{new_total_gb:.2f} GB", f"{pct_change:+.1f}%")
    c2.metric("TPS Δ", f"{(total_tps_mult - 1.0) * 100:+.0f}%")
    c3.metric("Quality Δ (cumulative PPL)", f"+{total_quality_pct:.1f}%")
    c4.metric(
        "Retraining cost",
        f"${total_retrain_usd:,.0f}",
        help=f"~{retrain_days:.1f} days on 8xH100" if retrain_days else "no retraining",
    )

    if notes:
        st.caption(" · ".join(notes))

    # Copy-as-config — emit a JSON diff the user can paste into hf config.
    import json as _json
    diff = {
        "quantization": {
            "weights": _q_pick if q_enable else None,
            "kv_cache": _kv_pick if kv_enable else None,
        },
        "attention": _att_pick if att_enable else None,
        "context": _ctx_pick if ctx_enable else None,
        "expected": {
            "total_vram_gb": new_total_gb,
            "tps_delta_pct": (total_tps_mult - 1.0) * 100,
            "quality_delta_ppl_pct": total_quality_pct,
            "retraining_usd": total_retrain_usd,
            "retraining_days_8xh100": retrain_days,
        },
    }
    with st.expander("Copy-as-config JSON"):
        st.code(_json.dumps(diff, indent=2), language="json")
