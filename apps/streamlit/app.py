"""
hwLedger Streamlit Web Client

Multipage app entry point. Provides:
- Planner: real-time memory planning with live slider updates
- Probe: device detection and telemetry
- Fleet: remote server audit (HTTP)
- Ledger: event log timeline
- Settings: config and HF token input
"""

import streamlit as st
from lib.ffi import is_available

# Page configuration
st.set_page_config(
    page_title="hwLedger",
    page_icon="🚀",
    layout="wide",
    initial_sidebar_state="expanded",
)

# Initialize session state
if "ffi_available" not in st.session_state:
    st.session_state.ffi_available = is_available()

if "server_url" not in st.session_state:
    st.session_state.server_url = "http://localhost:8080"

if "hf_token" not in st.session_state:
    st.session_state.hf_token = ""

# Banner alert if FFI not available
if not st.session_state.ffi_available:
    st.warning(
        "**FFI Library Not Found**: The native hwledger-ffi library is not loaded. "
        "To enable the Planner and Probe pages, run:\n\n"
        "`cargo build --release -p hwledger-ffi`\n\n"
        "Fleet and Ledger pages will still work if you configure a server URL below."
    )

# Sidebar
with st.sidebar:
    st.title("hwLedger")
    st.markdown("---")

    st.subheader("Configuration")
    st.session_state.server_url = st.text_input(
        "Server URL",
        value=st.session_state.server_url,
        help="API base URL for Fleet audit and Ledger queries (e.g., http://localhost:8080)",
    )

    st.session_state.hf_token = st.text_input(
        "HF Token (optional)",
        value=st.session_state.hf_token,
        type="password",
        help="Hugging Face token for model ingestion",
    )

    st.markdown("---")
    st.markdown("**About**")
    st.caption("hwLedger: GPU memory planner for LLM inference. Built with Rust + Streamlit.")

# Main content
st.title("hwLedger")
st.markdown("Real-time GPU memory planning and fleet audit for LLM inference.")

# Navigation — mirrors the SwiftUI sidebar order, with the three net-new
# pages (HF Search, What-If, Export) appended.
pages = {
    "Planner":    "pages/01_Planner.py",
    "Probe":      "pages/02_Probe.py",
    "Fleet":      "pages/03_Fleet.py",
    "Ledger":     "pages/04_Ledger.py",
    "Settings":   "pages/05_Settings.py",
    "HF Search":  "pages/06_HF_Search.py",
    "What-If":    "pages/07_WhatIf.py",
    "Export":     "pages/08_Export.py",
}

# Multipage routing is handled by Streamlit's built-in Pages directory structure
st.info("Use the sidebar to navigate to different tools.")
st.caption(
    "Parity map: `apps/streamlit/PARITY.md` — tracks feature gaps between "
    "this client and the macOS SwiftUI desktop app."
)
