"""
Probe Page: Device detection and telemetry via FFI.

Traces to: FR-TEL-002 (GPU Telemetry)
"""

import streamlit as st
from lib.ffi import detect_devices, is_available


st.set_page_config(page_title="Probe - hwLedger", layout="wide")

st.title("Device Probe")
st.markdown("Detect and inspect available GPU devices on this system.")

if not is_available():
    st.error(
        "FFI library not loaded. To enable this page, build the native library:\n\n"
        "`cargo build --release -p hwledger-ffi`"
    )
    st.stop()

# Detect devices
devices = detect_devices()

if not devices:
    st.warning("No GPU devices detected. Check driver installation and FFI library.")
    st.stop()

st.success(f"Detected {len(devices)} device(s)")

# Display each device
for dev in devices:
    with st.expander(f"Device {dev.id}: {dev.name} ({dev.backend})", expanded=True):
        col1, col2, col3 = st.columns(3)

        with col1:
            st.metric("Backend", dev.backend)
            st.metric("Total VRAM", f"{dev.vram_gb:.1f} GB")

        with col2:
            st.metric("Device ID", dev.id)
            st.metric("UUID", dev.uuid[:12] + "..." if len(dev.uuid) > 12 else dev.uuid)

        with col3:
            st.info(f"**Device Name**: {dev.name}")

# Summary table
st.subheader("Summary")
devices_data = {
    "ID": [d.id for d in devices],
    "Name": [d.name for d in devices],
    "Backend": [d.backend for d in devices],
    "VRAM (GB)": [f"{d.vram_gb:.1f}" for d in devices],
}

st.dataframe(devices_data, use_container_width=True, hide_index=True)
