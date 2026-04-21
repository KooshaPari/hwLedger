"""
Planner Page: Real-time memory planning with live slider updates via FFI.

Traces to: FR-PLAN-003 (Memory Planning)
"""

import json
import streamlit as st
import plotly.graph_objects as go
from pathlib import Path
from lib.ffi import plan, plan_layers, export_vllm, export_llama_cpp, export_mlx, is_available, PlanResult, predict, predict_available, model_max_context
from lib.charts import stacked_bar_chart, gauge_chart
from lib.tokens import LOG_TICKS, fmt_tokens, ticks_up_to


st.set_page_config(page_title="Planner - hwLedger", layout="wide")

st.title("Memory Planner")
st.markdown("Plan LLM inference memory requirements in real-time.")

if not is_available():
    st.error(
        "FFI library not loaded. To enable this page, build the native library:\n\n"
        "`cargo build --release -p hwledger-ffi`"
    )
    st.stop()

# Load golden fixture models
golden_dir = Path(__file__).parent.parent.parent.parent / "tests" / "golden"
fixture_files = sorted(golden_dir.glob("*.json")) if golden_dir.exists() else []
fixture_names = [f.stem for f in fixture_files]

# Show a banner if user just came in from HF Search's "Use this model" action.
if st.session_state.get("pending_model_id"):
    st.info(
        f"Loaded model id from HF Search: "
        f"**{st.session_state['pending_model_id']}** "
        f"(using closest golden fixture until live HF configs land)."
    )

if not fixture_names:
    st.warning("No golden fixture models found in tests/golden/. Add *.json files there.")
    st.stop()

# Sidebar controls
with st.sidebar:
    st.subheader("Model & Parameters")

    selected_fixture = st.selectbox(
        "Golden Fixture",
        options=fixture_names,
        help="Pre-configured model from tests/golden/",
    )

    # Resolve model max context BEFORE building the slider so options are bounded.
    # Traces to: FR-PLAN-003
    _fixture_path_preview = golden_dir / f"{selected_fixture}.json"
    try:
        with open(_fixture_path_preview) as _f:
            _preview_config_str = json.dumps(json.load(_f))
        _model_max_ctx = model_max_context(_preview_config_str) or 0
    except Exception:
        _preview_config_str = None
        _model_max_ctx = 0

    if _model_max_ctx > 0:
        st.markdown(
            f"<div style='display:inline-block;padding:4px 10px;border-radius:6px;"
            f"background:#7c3aed;color:white;font-weight:600;"
            f"font-family:ui-monospace,monospace;font-size:12px;'>"
            f"Max context: {_model_max_ctx:,} ({selected_fixture})</div>",
            unsafe_allow_html=True,
        )

    st.markdown("---")
    st.subheader("Runtime Config")

    # Log-spaced sequence-length slider. Capped at resolved model max when known;
    # otherwise exposes the full 128 → 10M range.
    _available_ticks = ticks_up_to(_model_max_ctx if _model_max_ctx > 0 else None)
    _default_seq = 4096 if 4096 in _available_ticks else _available_ticks[0]
    seq_len = st.select_slider(
        "Sequence Length (tokens)",
        options=_available_ticks,
        value=_default_seq,
        format_func=fmt_tokens,
        help="Log-scale context window size (128 → 10M). Capped at the model's declared max when known.",
    )
    if _model_max_ctx == 0 and seq_len > 131_072:
        st.warning(
            f"Requesting {fmt_tokens(seq_len)} without a resolved model — most deployed "
            "models cap at 128K. Pick a long-context model or expect failure at runtime."
        )

    concurrent_users = st.slider(
        "Concurrent Users",
        min_value=1,
        max_value=256,
        value=4,
        step=1,
        help="Number of simultaneous inference sessions",
    )

    batch_size = st.slider(
        "Batch Size",
        min_value=1,
        max_value=256,
        value=1,
        step=1,
        help="Tokens per batch during prefill",
    )

    st.markdown("---")
    st.subheader("Quantization")

    kv_quants = {
        "Fp16": 0,
        "Fp8": 1,
        "Int8": 2,
        "Int4": 3,
        "ThreeBit": 4,
    }
    kv_quant = st.selectbox(
        "KV Cache Quantization",
        options=list(kv_quants.keys()),
        index=0,
        help="Quantization for KV cache",
    )

    weight_quants = {
        "Fp16": 0,
        "Bf16": 1,
        "Int8": 2,
        "Int4": 3,
        "ThreeBit": 4,
    }
    weight_quant = st.selectbox(
        "Weight Quantization",
        options=list(weight_quants.keys()),
        index=0,
        help="Quantization for model weights",
    )

# Main content
col1, col2 = st.columns([2, 1])

with col1:
    # Load fixture
    fixture_path = golden_dir / f"{selected_fixture}.json"
    try:
        with open(fixture_path) as f:
            config_json = json.load(f)
        config_str = json.dumps(config_json)
    except Exception as e:
        st.error(f"Failed to load fixture: {e}")
        st.stop()

    # Call FFI planner
    result = plan(
        config_json=config_str,
        seq_len=seq_len,
        concurrent_users=concurrent_users,
        batch_size=batch_size,
        kv_quant=kv_quants[kv_quant],
        weight_quant=weight_quants[weight_quant],
    )

    if result is None:
        st.error("Planning failed. Check FFI library and config.")
        st.stop()

    # Chart
    fig = stacked_bar_chart(result)
    st.plotly_chart(fig, use_container_width=True, key="plan_chart")

    # Layer heatmap
    st.subheader("Per-Layer KV Contributions")
    layers = plan_layers(
        config_json=config_str,
        seq_len=seq_len,
        kv_quant=kv_quants[kv_quant],
    )

    if layers:
        max_val = max(layers) if layers else 1
        min_val = min(layers) if layers else 0
        normalized = [(x - min_val) / max(1, max_val - min_val) for x in layers]

        # Heatmap row: 1 row x N columns (one per layer). Palette matches the
        # SwiftUI `Color.interpolate` used in PlannerScreen.layerHeatmapSection —
        # pale lavender → magenta → deep purple.
        planner_scale = [
            [0.00, "rgb(217,217,217)"],
            [0.50, "rgb(255,217,255)"],
            [1.00, "rgb(127,51,76)"],
        ]
        fig_heat = go.Figure(data=go.Heatmap(
            z=[[norm for norm in normalized]],
            colorscale=planner_scale,
            showscale=True,
            colorbar=dict(title="Relative KV Bytes"),
        ))
        fig_heat.update_layout(
            height=150,
            margin=dict(l=0, r=0, t=0, b=0),
            xaxis_title="Layer Index",
            yaxis=dict(showticklabels=False),
        )
        st.plotly_chart(fig_heat, use_container_width=True, key="layer_heatmap")

with col2:
    # Metrics
    st.metric("Total Memory", f"{result.total_gb:.2f} GB")
    st.metric("Effective Batch", result.effective_batch)
    # Attention-kind badge (closes P1 parity gap).
    badge_color = {
        "mha": "#4ECDC4",
        "gqa": "#45B7D1",
        "mla": "#BB77DD",
    }.get(result.attention_kind.lower(), "#888")
    st.markdown(
        f"<div style='display:inline-block;padding:6px 12px;border-radius:6px;"
        f"background:{badge_color};color:white;font-weight:600;"
        f"font-family:ui-monospace,monospace;'>Attention · {result.attention_kind}</div>",
        unsafe_allow_html=True,
    )

    # Stash latest breakdown so WhatIf/Export pages can read it.
    st.session_state["latest_plan"] = {
        "weights_mb": result.weights_mb,
        "kv_mb": result.kv_mb,
        "prefill_mb": result.prefill_mb,
        "runtime_mb": result.runtime_mb,
        "config_json": config_str,
        "seq_len": seq_len,
        "concurrent_users": concurrent_users,
        "batch_size": batch_size,
        "kv_quant": kv_quants[kv_quant],
        "weight_quant": weight_quants[weight_quant],
    }

# Detailed breakdown table
st.subheader("Memory Breakdown")
breakdown_data = {
    "Component": ["Weights", "KV Cache", "Prefill Activations", "Runtime Overhead"],
    "Memory (MB)": [
        f"{result.weights_mb:.2f}",
        f"{result.kv_mb:.2f}",
        f"{result.prefill_mb:.2f}",
        f"{result.runtime_mb:.2f}",
    ],
    "Percent": [
        f"{100 * result.weights_mb / result.total_mb:.1f}%",
        f"{100 * result.kv_mb / result.total_mb:.1f}%",
        f"{100 * result.prefill_mb / result.total_mb:.1f}%",
        f"{100 * result.runtime_mb / result.total_mb:.1f}%",
    ],
}

st.dataframe(breakdown_data, use_container_width=True, hide_index=True)

# Export section
st.subheader("Export Configuration")

col_export1, col_export2, col_export3 = st.columns(3)

with col_export1:
    if st.button("Export as vLLM", use_container_width=True, key="export_vllm_btn"):
        vllm_args = export_vllm(
            config_json=config_str,
            seq_len=seq_len,
            concurrent_users=concurrent_users,
            batch_size=batch_size,
            kv_quant=kv_quants[kv_quant],
            weight_quant=weight_quants[weight_quant],
        )
        if vllm_args:
            st.code(vllm_args, language="bash")
        else:
            st.error("Export failed")

with col_export2:
    if st.button("Export as llama.cpp", use_container_width=True, key="export_llama_btn"):
        llama_args = export_llama_cpp(
            config_json=config_str,
            seq_len=seq_len,
            concurrent_users=concurrent_users,
            batch_size=batch_size,
            kv_quant=kv_quants[kv_quant],
            weight_quant=weight_quants[weight_quant],
        )
        if llama_args:
            st.code(llama_args, language="bash")
        else:
            st.error("Export failed")

with col_export3:
    if st.button("Export as MLX", use_container_width=True, key="export_mlx_btn"):
        mlx_config = export_mlx(
            config_json=config_str,
            seq_len=seq_len,
            concurrent_users=concurrent_users,
            batch_size=batch_size,
            kv_quant=kv_quants[kv_quant],
            weight_quant=weight_quants[weight_quant],
        )
        if mlx_config:
            st.code(mlx_config, language="json")
        else:
            st.error("Export failed")

# Config preview
st.subheader("Model Config (JSON)")
st.json(config_json)


# =============================================================================
# What-If panel — prediction buffet (Traces to: FR-PREDICT-001)
# =============================================================================
st.divider()
st.subheader("What-If — Prediction Buffet")
st.caption(
    "Before you swap a model or enable a compression technique, see the delta: "
    "memory, decode tok/s, TTFT, transformation cost, and citations."
)

if not predict_available():
    st.info(
        "Prediction FFI not detected in the currently loaded hwledger-ffi. "
        "Rebuild with `cargo build --release -p hwledger-ffi` to enable this panel."
    )
else:
    col_cand, col_tech = st.columns([1, 1])
    with col_cand:
        candidate_fixture = st.selectbox(
            "Candidate model (golden fixture)",
            options=fixture_names,
            index=min(1, len(fixture_names) - 1),
            help="Compare against baseline selected above. HF search integration uses the sibling crate.",
            key="whatif_candidate",
        )
        hardware = st.selectbox(
            "Target hardware",
            options=["A100-80G", "H100-80G", "B200-180G", "L40S", "M3-Max-128G", "M3-Ultra-192G"],
            key="whatif_hw",
        )
    with col_tech:
        all_techniques = [
            "int8", "int4", "fp8", "int4_awq", "int4_gptq", "int4_gptq_v2", "quarot",
            "smooth_quant", "kv_cache_int8", "kv_cache_int4", "sparse_gpt", "wanda", "reap",
            "lora", "qlora", "dora", "speculative_decoding", "medusa", "eagle",
            "lookahead_decoding", "flash_attention2", "flash_attention3", "paged_attention",
            "continuous_batching", "kv_cache_offload", "tensor_parallel", "pipeline_parallel",
            "expert_parallel", "context_parallel",
        ]
        picked_techniques = st.multiselect(
            "Techniques to stack", all_techniques, default=[], key="whatif_tech",
            help="Applied multiplicatively on the candidate. Every technique cites a paper.",
        )

    candidate_path = golden_dir / f"{candidate_fixture}.json"
    try:
        with open(candidate_path) as cf:
            candidate_json = json.dumps(json.load(cf))
    except Exception as exc:
        st.error(f"Candidate fixture load failed: {exc}")
        candidate_json = None

    if candidate_json:
        pred = predict(
            baseline_config_json=config_str,
            candidate_config_json=candidate_json,
            techniques=picked_techniques,
            prefill_tokens=seq_len,
            decode_tokens=max(128, seq_len // 4),
            batch=batch_size,
            seq_len=seq_len,
            hardware=hardware,
        )
        if pred is None:
            st.error("Prediction call failed — check FFI library rebuild.")
        else:
            mcol1, mcol2, mcol3 = st.columns(3)
            mcol1.metric("Memory Δ", f"{pred['mem_delta_bytes'] / 1e9:+.2f} GB")
            tps = pred["decode_tps"]
            mcol2.metric(
                "Decode tok/s",
                f"{tps['mid']:.1f}",
                help=f"95% CI: {tps['low']:.1f} - {tps['high']:.1f} ({tps['provenance']})",
            )
            ttft = pred["ttft_ms"]
            mcol3.metric(
                "TTFT (ms)",
                f"{ttft['mid']:.0f}",
                help=f"95% CI: {ttft['low']:.0f} - {ttft['high']:.0f} ({ttft['provenance']})",
            )

            baseline_total_gb = result.total_gb if result else 0.0
            candidate_total_gb = baseline_total_gb + pred["mem_delta_bytes"] / 1e9
            bar = go.Figure(data=[
                go.Bar(name="Baseline", x=["Total"], y=[baseline_total_gb], marker_color="#a78bfa"),
                go.Bar(name="Candidate (after techniques)", x=["Total"], y=[candidate_total_gb], marker_color="#7c3aed"),
            ])
            bar.update_layout(barmode="group", height=260, yaxis_title="GB")
            st.plotly_chart(bar, use_container_width=True, key="whatif_bar")

            tp = pred.get("throughput_at_batch", {})
            if tp:
                st.write("**Throughput vs batch**")
                bdata = sorted((int(k), v) for k, v in tp.items())
                fig_tp = go.Figure(data=go.Scatter(
                    x=[b for b, _ in bdata],
                    y=[v["mid"] for _, v in bdata],
                    error_y=dict(
                        type="data",
                        array=[v["high"] - v["mid"] for _, v in bdata],
                        arrayminus=[v["mid"] - v["low"] for _, v in bdata],
                    ),
                    mode="lines+markers",
                    line=dict(color="#7c3aed"),
                ))
                fig_tp.update_layout(height=260, xaxis_title="Batch", yaxis_title="tok/s")
                st.plotly_chart(fig_tp, use_container_width=True, key="whatif_tp")

            tr = pred["transformation"]
            verdict_kind = tr.get("kind", "none")
            verdict_color = {
                "none": "green",
                "lora_required": "orange",
                "fine_tune_required": "orange",
                "retrain_required": "red",
                "incompatible": "red",
            }.get(verdict_kind, "gray")
            st.markdown(
                f"**Transformation verdict:** :{verdict_color}[{verdict_kind.replace('_', ' ')}]"
            )
            st.json(tr)

            if pred.get("warnings"):
                st.warning("\n".join(f"- {w}" for w in pred["warnings"]))
            if pred.get("citations"):
                with st.expander("Citations"):
                    for c in pred["citations"]:
                        label = c.get("label", "")
                        src = c.get("source", "")
                        url = c.get("url")
                        if url:
                            st.markdown(f"- [{src}]({url}) - {label}")
                        else:
                            st.markdown(f"- **{src}** - {label}")
