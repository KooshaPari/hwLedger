"""
Planner Page: Real-time memory planning with live slider updates via FFI.

Traces to: FR-PLAN-003 (Memory Planning)
"""

import json
import streamlit as st
from pathlib import Path
from lib.ffi import plan, is_available, PlanResult
from lib.charts import stacked_bar_chart, gauge_chart


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

    st.markdown("---")
    st.subheader("Runtime Config")

    seq_len = st.slider(
        "Sequence Length (tokens)",
        min_value=128,
        max_value=32768,
        value=4096,
        step=256,
        help="Context window size",
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

with col2:
    # Metrics
    st.metric("Total Memory", f"{result.total_gb:.2f} GB")
    st.metric("Effective Batch", result.effective_batch)
    st.metric("Attention", result.attention_kind)

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

# Config preview
st.subheader("Model Config (JSON)")
st.json(config_json)
