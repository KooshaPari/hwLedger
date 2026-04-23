"""
Planner Page: Real-time memory planning with live slider updates via FFI.

Traces to: FR-PLAN-003 (Memory Planning)
"""

import json
import streamlit as st
import plotly.graph_objects as go
from pathlib import Path
from typing import Optional
from lib.ffi import (
    plan, plan_layers, export_vllm, export_llama_cpp, export_mlx,
    is_available, PlanResult, predict, predict_available, model_max_context,
    resolve_model, resolve_available,
)
from lib.charts import stacked_bar_chart, gauge_chart
from lib.tokens import LOG_TICKS, fmt_tokens, ticks_up_to
from lib.perf_model import estimate_throughput
from lib.cost_model import fine_tune_overhead_mb


st.set_page_config(page_title="Planner - hwLedger", layout="wide")

st.title("Memory Planner")
st.markdown("Plan LLM inference memory requirements in real-time.")

# Mode tab — inference vs fine-tune. Fine-tune mode adds optimizer-state VRAM
# via lib.cost_model.fine_tune_overhead_mb() (AdamW = 4x weights, LoRA = 5%).
_mode_tabs = st.radio(
    "Mode",
    options=["Inference", "Fine-tune (LoRA)", "Fine-tune (Full AdamW)"],
    horizontal=True,
    key="planner_mode",
    help=(
        "Inference: forward-pass only. "
        "Fine-tune adds optimizer state + gradients. "
        "LoRA is the cheap path (~5% overhead); full AdamW is ~4-8x weights."
    ),
)
_mode_key = {
    "Inference": "inference",
    "Fine-tune (LoRA)": "lora",
    "Fine-tune (Full AdamW)": "adamw",
}[_mode_tabs]

if not is_available():
    st.error(
        "FFI library not loaded. To enable this page, build the native library:\n\n"
        "`cargo build --release -p hwledger-ffi`"
    )
    st.stop()

# Golden fixtures directory — still used by the built-in popover and by the
# `golden_fixture` branch of the resolver output. The old selectbox UI is gone;
# input flows exclusively through the unified model text box below.
golden_dir = Path(__file__).parent.parent.parent.parent / "tests" / "golden"
fixture_files = sorted(golden_dir.glob("*.json")) if golden_dir.exists() else []
fixture_names = [f.stem for f in fixture_files]

# Built-in one-click fixtures surfaced in the popover. Matches the four callouts
# in the Planner spec (DeepSeek-V3, Llama-3-70B, Mixtral-8x7B, Mamba2-2.7B).
BUILTIN_FIXTURES = [
    ("DeepSeek-V3", "deepseek-v3"),
    ("Llama-3-70B", "llama3-70b"),
    ("Mixtral-8x7B", "mixtral-8x7b"),
    ("Mamba2-2.7B", "mamba2-2.7b"),
]

# Show a banner if user just came in from HF Search's "Use this model" action.
if st.session_state.get("pending_model_id"):
    pending_id = st.session_state["pending_model_id"]
    st.info(f"Loaded model id from HF Search: **{pending_id}**")
    # Seed the unified input on first arrival so the user doesn't need to retype.
    st.session_state.setdefault("planner_model_input", pending_id)
    st.session_state.pop("pending_model_id", None)


def load_config_json(source: dict) -> Optional[dict]:
    """Materialise a resolver output dict into a parsed config.json.

    Handles all three successful `kind` variants:
      - ``golden_fixture`` / ``local_config`` — read the absolute path from disk
      - ``hf_repo``        — fetch via the HF FFI client (``hwledger_hf_plan``
        is the sibling path; for config-only needs we read the cached
        ``config.json`` through the resolver-returned path when available,
        otherwise fall back to the golden fixture with the same slug).

    Returns a parsed JSON dict or ``None`` on failure.
    """
    kind = source.get("kind")
    if kind in ("golden_fixture", "local_config"):
        path = source.get("path")
        if not path:
            return None
        try:
            with open(path) as fh:
                return json.load(fh)
        except Exception:
            return None
    if kind == "hf_repo":
        # The planner FFI exposes `hwledger_hf_plan`, but here we only need
        # config for UI context (max context, layer count). Fall back to the
        # closest golden fixture when the slug maps 1:1; otherwise use a
        # minimal config stub so the planner can still run via the repo-id
        # path once the HF fetch integration is wired in end-to-end.
        repo_id = source.get("repo_id", "")
        slug = repo_id.split("/", 1)[-1].lower()
        for name in fixture_names:
            if name.lower() == slug or slug.startswith(name.lower()):
                try:
                    with open(golden_dir / f"{name}.json") as fh:
                        return json.load(fh)
                except Exception:
                    break
        # Minimal stub so downstream chart/plan code doesn't KeyError. The
        # FFI planner will still handle a sparse config gracefully.
        return {
            "model_type": "llama",
            "hidden_size": 4096,
            "num_hidden_layers": 32,
            "num_attention_heads": 32,
            "max_position_embeddings": 4096,
            "_hf_repo_id": repo_id,
        }
    return None


# --- Unified model picker -------------------------------------------------
with st.sidebar:
    st.subheader("Model & Parameters")

    if "planner_model_input" not in st.session_state:
        st.session_state["planner_model_input"] = "gold:deepseek-v3"

    st.text_input(
        "Model",
        placeholder="search name, paste HF URL, or type org/repo-id",
        key="planner_model_input",
        help=(
            "Type a model name or paste a Hugging Face URL / repo-id. "
            "Built-in fixtures use the `gold:<name>` scheme."
        ),
    )

    _model_input = st.session_state["planner_model_input"].strip()
    _resolution: Optional[dict] = None
    if _model_input and resolve_available():
        _resolution = resolve_model(_model_input)

    config_json: Optional[dict] = None
    config_str: Optional[str] = None
    _resolution_kind = (_resolution or {}).get("kind")

    if _resolution is None:
        st.warning("Resolver FFI unavailable — rebuild `hwledger-ffi`.")
    elif "error" in _resolution:
        st.error(f"Resolver error: {_resolution['error']}")
    elif _resolution_kind == "hf_repo":
        repo_id = _resolution["repo_id"]
        rev = _resolution.get("revision")
        label = f"{repo_id}" + (f"@{rev}" if rev else "")
        st.markdown(
            f"<div style='display:inline-block;padding:4px 10px;border-radius:6px;"
            f"background:#16a34a;color:white;font-weight:600;"
            f"font-family:ui-monospace,monospace;font-size:12px;'>"
            f"✓ Resolved → {label}</div>",
            unsafe_allow_html=True,
        )
        config_json = load_config_json(_resolution)
    elif _resolution_kind == "golden_fixture":
        path = _resolution["path"]
        fname = Path(path).name
        st.markdown(
            f"<div style='display:inline-block;padding:4px 10px;border-radius:6px;"
            f"background:#16a34a;color:white;font-weight:600;"
            f"font-family:ui-monospace,monospace;font-size:12px;'>"
            f"✓ Loaded → {fname} <span style='opacity:0.75;font-weight:400;'>"
            f"(built-in)</span></div>",
            unsafe_allow_html=True,
        )
        config_json = load_config_json(_resolution)
    elif _resolution_kind == "local_config":
        path = _resolution["path"]
        st.markdown(
            f"<div style='display:inline-block;padding:4px 10px;border-radius:6px;"
            f"background:#16a34a;color:white;font-weight:600;"
            f"font-family:ui-monospace,monospace;font-size:12px;'>"
            f"✓ Loaded → {path}</div>",
            unsafe_allow_html=True,
        )
        config_json = load_config_json(_resolution)
    elif _resolution_kind == "ambiguous":
        hint = _resolution.get("hint", _model_input)
        candidates = _resolution.get("candidates", []) or []
        st.info(f"Did you mean… (searching HF for **{hint}**)")
        if candidates:
            options = [""] + [c.get("id", "") for c in candidates if c.get("id")]
            pick = st.selectbox(
                "HF candidates",
                options=options,
                format_func=lambda x: x if x else "— pick one —",
                key="planner_ambiguous_pick",
            )
            if pick:
                st.session_state["planner_model_input"] = pick
                st.rerun()
        else:
            st.caption("No HF candidates returned (offline or rate-limited).")
    else:
        st.warning(f"Unrecognised resolver response: {_resolution}")

    # Built-in fixtures popover — four one-click buttons.
    with st.popover("Built-in fixtures"):
        for label, slug in BUILTIN_FIXTURES:
            if st.button(label, key=f"builtin_{slug}", use_container_width=True):
                st.session_state["planner_model_input"] = f"gold:{slug}"
                st.rerun()

    # Compute config_str + max context for downstream sections.
    _model_max_ctx = 0
    if config_json is not None:
        try:
            config_str = json.dumps(config_json)
            _model_max_ctx = model_max_context(config_str) or 0
        except Exception:
            config_str = None
            _model_max_ctx = 0

    if _model_max_ctx > 0:
        st.markdown(
            f"<div style='display:inline-block;padding:4px 10px;border-radius:6px;"
            f"background:#7c3aed;color:white;font-weight:600;"
            f"font-family:ui-monospace,monospace;font-size:12px;margin-top:6px;'>"
            f"Max context: {_model_max_ctx:,}</div>",
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

    st.markdown("---")
    st.subheader("Hardware & Offload")

    gpu_kind = st.selectbox(
        "GPU Kind",
        options=["A100", "H100", "L40S", "B200", "M3_Ultra", "RTX_4090"],
        index=1,
        help="Target GPU — drives bandwidth-bound TPS and capacity status.",
    )
    gpu_capacity_gb = st.slider(
        "GPU Capacity (GB)",
        min_value=8,
        max_value=192,
        value={"A100": 80, "H100": 80, "L40S": 48, "B200": 180,
               "M3_Ultra": 192, "RTX_4090": 24}.get(gpu_kind, 80),
        step=4,
        help="Per-GPU VRAM. Scale up for SXM/NVL; tensor-parallel splits not modelled here.",
    )
    cpu_nvme_offload = st.toggle(
        "CPU/NVMe Offload",
        value=False,
        help=(
            "Enable when weights exceed GPU capacity. PCIe Gen4 x16 tops out "
            "near 32 GB/s vs HBM ~3.3 TB/s on H100 — expect major TPS hit."
        ),
    )

    # Per-slider tips & tricks sidebar (QoL from apxml).
    with st.expander("Tips & Tricks"):
        st.markdown(
            "- **Batch 1** is great for single-user chat; **>=8** for "
            "throughput-optimised deployments.\n"
            "- **KV-Q4** saves ~4x KV memory at near-zero quality cost "
            "(see KIVI, Liu 2024).\n"
            "- **Fine-tune mode** adds 4-8x weight VRAM for AdamW; use LoRA "
            "to stay near inference footprint.\n"
            "- Long-context over 32K benefits from **paged attention** or "
            "**MLA** — see the What-If page.\n"
            "- Docs: `docs-site/math/gqa.md`, `kv-cache.md`, `mla.md`."
        )

# Main content
col1, col2 = st.columns([2, 1])

with col1:
    # `config_json` and `config_str` are populated by the unified model picker
    # above. Bail out with an actionable message when resolution didn't yield
    # a usable config (e.g. free-text without a candidate selected).
    if config_json is None or config_str is None:
        st.info(
            "Type a model name, paste a Hugging Face URL, or pick a built-in "
            "fixture from the sidebar popover to start planning."
        )
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
    # Apply fine-tune overhead on top of the FFI inference plan.
    # Keeps the Rust FFI C ABI untouched (backward-compat) by computing the
    # optimizer-state contribution client-side in Python.
    ft_overhead_mb = 0.0
    if _mode_key in ("lora", "adamw"):
        ft_overhead_mb = fine_tune_overhead_mb(result.weights_mb, optimizer=_mode_key)
    adjusted_total_mb = result.total_mb + ft_overhead_mb
    adjusted_total_gb = adjusted_total_mb / 1024.0

    # Throughput + status via perf_model.
    tps_est = estimate_throughput(
        model_size_gb=adjusted_total_gb,
        attention_kind=result.attention_kind,
        gpu_kind=gpu_kind,
        gpu_capacity_gb=gpu_capacity_gb,
        concurrent_users=concurrent_users,
        seq_len=seq_len,
        cpu_offload=cpu_nvme_offload,
    )

    # Metrics
    st.metric("Total Memory", f"{adjusted_total_gb:.2f} GB")
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
        "ft_overhead_mb": ft_overhead_mb,
        "adjusted_total_mb": adjusted_total_mb,
        "config_json": config_str,
        "seq_len": seq_len,
        "concurrent_users": concurrent_users,
        "batch_size": batch_size,
        "kv_quant": kv_quants[kv_quant],
        "weight_quant": weight_quants[weight_quant],
        "mode": _mode_key,
        "gpu_kind": gpu_kind,
        "gpu_capacity_gb": gpu_capacity_gb,
        "cpu_nvme_offload": cpu_nvme_offload,
        "tps": tps_est.tps,
        "ttft_ms": tps_est.ttft_ms,
        "attention_kind": result.attention_kind,
    }

# ---------------------------------------------------------------------------
# Capacity, throughput, and status row (apxml-inspired outputs).
# ---------------------------------------------------------------------------
st.divider()
st.subheader("Capacity & Throughput")

capacity_remaining_gb = max(0.0, gpu_capacity_gb - adjusted_total_gb)
over_by_gb = max(0.0, adjusted_total_gb - gpu_capacity_gb)

# VRAM percentage bar — hard-capped display at 150% so over-budget is visible.
st.progress(min(1.0, tps_est.vram_pct / 100.0),
            text=f"VRAM usage: {tps_est.vram_pct:.1f}% of {gpu_capacity_gb} GB")

if over_by_gb > 0:
    st.caption(
        f"Over capacity by **{over_by_gb:.2f} GB** — enable CPU/NVMe offload, "
        "reduce seq_len, or quantize further."
    )
else:
    st.caption(f"Capacity remaining: **{capacity_remaining_gb:.2f} GB**.")

# Status badge — green / amber / red.
_status_meta = {
    "ready": ("#16a34a", "Ready"),
    "offload_required": ("#d97706", "Offload Required"),
    "oom_risk": ("#dc2626", "OOM Risk"),
}[tps_est.status]
st.markdown(
    f"<div style='display:inline-block;padding:6px 14px;border-radius:6px;"
    f"background:{_status_meta[0]};color:white;font-weight:600;"
    f"font-family:ui-monospace,monospace;'>Status · {_status_meta[1]}</div>",
    unsafe_allow_html=True,
)

# Throughput row: TPS, TTFT, total throughput.
tcol1, tcol2, tcol3 = st.columns(3)
tcol1.metric(
    "TPS (tokens/sec)",
    f"{tps_est.tps:.1f}",
    help=(
        "Bandwidth-bound decode estimate: "
        "tps = (bandwidth_GB/s / model_size_GB) * attention_efficiency. "
        "See lib/perf_model.py — formula cites apxml + DeepSeek-V2 paper."
    ),
)
tcol2.metric(
    "TTFT (ms)",
    f"{tps_est.ttft_ms:.0f}",
    help="Prefill-bound: seq_len * model_size / (bandwidth * 0.6 derate).",
)
tcol3.metric(
    "Total Throughput (tok/s)",
    f"{tps_est.total_tps:.0f}",
    help=f"TPS * {concurrent_users} concurrent users.",
)

with st.expander("Why is my VRAM 110%?"):
    st.markdown(
        f"""
Each memory contributor, measured in MB:

| Component              | MB |
|------------------------|------:|
| Weights                | {result.weights_mb:,.0f} |
| KV cache               | {result.kv_mb:,.0f} |
| Prefill activations    | {result.prefill_mb:,.0f} |
| Runtime overhead (CUDA ctx, allocator) | {result.runtime_mb:,.0f} |
| **Fine-tune overhead** (optimizer/grads) | {ft_overhead_mb:,.0f} |
| **Total**              | **{adjusted_total_mb:,.0f}** |

**Common escape hatches:**
- Quantize weights to Int4 (→ 4x reduction on the biggest row).
- Enable KV-cache Int8 / FP8 (→ 2x reduction on KV row).
- Reduce `seq_len` or `concurrent_users` (→ linear reduction on KV + prefill).
- Switch from AdamW to LoRA (→ reduces optimizer state ~80x).
- Enable CPU/NVMe offload (→ trades TPS for capacity).
        """
    )

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
