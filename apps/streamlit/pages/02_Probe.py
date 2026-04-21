"""
Probe Page: Device detection + live telemetry via FFI.

Parity gap closed: matches FleetScreen.swift polling loop (2s on macOS; 1s here
for snappier web feel). Each detected device gets a row with:
- VRAM used / total progress bar
- Utilization, temperature, power metric trio
- Rolling 30-sample sparkline (Plotly) for util / power

Traces to: FR-TEL-002 (GPU Telemetry), FR-PROBE-001 (Device Detection).
"""

from __future__ import annotations

import time
from collections import deque
from typing import Deque, Dict

import plotly.graph_objects as go
import streamlit as st

from lib.ffi import detect_devices, is_available, lib, Telemetry, TelemetrySample
import ctypes


st.set_page_config(page_title="Probe - hwLedger", layout="wide")
st.title("Device Probe")
st.markdown("Detect GPUs and stream live utilization / thermal / power telemetry.")

if not is_available():
    st.error(
        "FFI library not loaded. Build it with:\n\n"
        "`cargo build --release -p hwledger-ffi`"
    )
    st.stop()


def _sample_device(device_id: int, backend: str) -> Telemetry | None:
    """Call hwledger_probe_sample via ctypes."""
    try:
        lib.hwledger_probe_sample.argtypes = [ctypes.c_uint32, ctypes.c_char_p]
        lib.hwledger_probe_sample.restype = ctypes.POINTER(TelemetrySample)
        lib.hwledger_probe_sample_free.argtypes = [ctypes.POINTER(TelemetrySample)]
        ptr = lib.hwledger_probe_sample(device_id, backend.encode("utf-8"))
        if not ptr:
            return None
        s = ptr.contents
        out = Telemetry(
            device_id=s.device_id,
            free_vram_gb=s.free_vram_bytes / (1024**3),
            util_percent=float(s.util_percent),
            temperature_c=float(s.temperature_c),
            power_watts=float(s.power_watts),
            captured_at_ms=int(s.captured_at_ms),
        )
        lib.hwledger_probe_sample_free(ptr)
        return out
    except Exception:
        return None


# Session-level rolling buffers (30 samples ≈ 30s at 1 Hz)
if "probe_history" not in st.session_state:
    st.session_state.probe_history = {}
if "probe_running" not in st.session_state:
    st.session_state.probe_running = False

history: Dict[int, Deque[Telemetry]] = st.session_state.probe_history

# Controls
col_a, col_b, col_c = st.columns([1, 1, 4])
with col_a:
    if st.button(
        "Stop" if st.session_state.probe_running else "Start live",
        use_container_width=True,
    ):
        st.session_state.probe_running = not st.session_state.probe_running
        st.rerun()
with col_b:
    if st.button("Refresh devices", use_container_width=True):
        st.rerun()
with col_c:
    st.caption(
        f"Polling {'ACTIVE (1 Hz)' if st.session_state.probe_running else 'paused'}"
    )

devices = detect_devices()
if not devices:
    st.warning("No GPU devices detected. Check driver installation and FFI library.")
    st.stop()
st.success(f"Detected {len(devices)} device(s)")


def _sparkline(samples: Deque[Telemetry], field: str, label: str) -> go.Figure:
    xs = [s.captured_at_ms / 1000 for s in samples]
    ys = [getattr(s, field) for s in samples]
    fig = go.Figure(go.Scatter(x=xs, y=ys, mode="lines",
                               line=dict(width=2, color="#BB77DD")))
    fig.update_layout(
        height=80, margin=dict(l=4, r=4, t=20, b=4),
        showlegend=False, title=dict(text=label, font=dict(size=11)),
        xaxis=dict(visible=False),
        yaxis=dict(showticklabels=False, showgrid=False),
    )
    return fig


for dev in devices:
    buf = history.setdefault(dev.id, deque(maxlen=30))
    if st.session_state.probe_running:
        s = _sample_device(dev.id, dev.backend)
        if s is not None:
            buf.append(s)

    with st.expander(f"Device {dev.id}: {dev.name} ({dev.backend})", expanded=True):
        col1, col2, col3, col4 = st.columns(4)
        latest = buf[-1] if buf else None
        total = dev.vram_gb
        used = total - latest.free_vram_gb if latest else 0

        with col1:
            st.metric("VRAM", f"{used:.1f}/{total:.1f} GB")
            st.progress(min(1.0, max(0.0, used / total if total else 0)))
        # Sentinel values: the Rust FFI writes f32::NAN for UnsupportedMetric
        # readings, and negative for hard errors. The IOKit backend surfaces
        # `ProbeError::UnsupportedMetric { chip, macos_version, metric }` which
        # we render inline as "Not supported on <chip>" per FR-TEL-002.
        def _fmt(v: float, unit: str) -> str:
            import math

            if latest is None:
                return "—"
            if v is None or (isinstance(v, float) and math.isnan(v)):
                return f"Not supported on {dev.name}"
            if v < 0:
                return "error"
            return f"{v:.0f}{unit}"

        with col2:
            st.metric("Util", _fmt(latest.util_percent if latest else None, "%"))
        with col3:
            st.metric("Temp", _fmt(latest.temperature_c if latest else None, "°C"))
        with col4:
            st.metric("Power", _fmt(latest.power_watts if latest else None, " W"))

        if len(buf) >= 2:
            spc1, spc2 = st.columns(2)
            with spc1:
                st.plotly_chart(_sparkline(buf, "util_percent", "Util %"),
                                use_container_width=True,
                                key=f"util_spark_{dev.id}")
            with spc2:
                st.plotly_chart(_sparkline(buf, "power_watts", "Power W"),
                                use_container_width=True,
                                key=f"power_spark_{dev.id}")

# Summary table
st.subheader("Summary")
st.dataframe(
    {
        "ID": [d.id for d in devices],
        "Name": [d.name for d in devices],
        "Backend": [d.backend for d in devices],
        "VRAM (GB)": [f"{d.vram_gb:.1f}" for d in devices],
    },
    use_container_width=True,
    hide_index=True,
)

# Auto-refresh every 1s while running
if st.session_state.probe_running:
    time.sleep(1.0)
    st.rerun()
