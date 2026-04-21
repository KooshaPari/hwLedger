"""
Settings Page: parity with SwiftUI SettingsScreen.

Covers:
- Server URL + test connection
- Bootstrap / mTLS token (session-only, masked)
- mTLS client cert generation (shells out to `hwledger cert-gen` when available)
- HF token (session-only, masked, never logged)
- Log level
- Core version via FFI
"""

from __future__ import annotations

import ctypes
import shutil
import subprocess

import httpx
import streamlit as st

from lib.ffi import is_available, lib


st.set_page_config(page_title="Settings - hwLedger", layout="wide")
st.title("Settings")
st.markdown("Configure hwLedger Streamlit client. All tokens stay in session memory.")


# --- System ---
st.subheader("System")
c_core1, c_core2 = st.columns(2)
with c_core1:
    st.markdown(f"**FFI library:** {'loaded' if is_available() else 'NOT FOUND'}")
with c_core2:
    version = "unknown"
    if lib is not None and hasattr(lib, "hwledger_core_version"):
        try:
            lib.hwledger_core_version.restype = ctypes.c_char_p
            raw = lib.hwledger_core_version()
            version = raw.decode("utf-8") if raw else "unknown"
        except Exception:
            pass
    st.markdown(f"**Core version:** `{version}`")

st.divider()

# --- Server ---
st.subheader("Fleet server")
col1, col2 = st.columns([3, 1])
with col1:
    new_server_url = st.text_input(
        "API server URL",
        value=st.session_state.get("server_url", "http://localhost:8080"),
        help="Base URL used by Fleet and Ledger pages.",
    )
    st.session_state.server_url = new_server_url
with col2:
    if st.button("Test connection", use_container_width=True):
        try:
            with httpx.Client(timeout=3.0) as c:
                r = c.get(f"{new_server_url}/v1/health")
                if r.status_code == 200:
                    st.success("Reachable")
                else:
                    st.warning(f"HTTP {r.status_code}")
        except Exception as e:
            st.error(f"Unreachable: {e}")

bootstrap = st.text_input(
    "Bootstrap / mTLS admin token",
    value=st.session_state.get("bootstrap_token", ""),
    type="password",
    help="Session-only. Never persisted, never logged.",
)
st.session_state.bootstrap_token = bootstrap

st.divider()

# --- mTLS cert generation ---
st.subheader("mTLS client certificate")
cert_col1, cert_col2 = st.columns([1, 2])
with cert_col1:
    name = st.text_input("Common name", value="streamlit-client")
    if st.button("Generate cert", use_container_width=True):
        cert_bin = shutil.which("hwledger")
        if cert_bin:
            try:
                res = subprocess.run(
                    [cert_bin, "cert-gen", "--cn", name],
                    capture_output=True, text=True, timeout=15,
                )
                if res.returncode == 0:
                    st.session_state.mtls_cert = res.stdout
                    st.success("Cert generated")
                else:
                    st.error(res.stderr or "cert-gen failed")
            except Exception as e:
                st.error(f"cert-gen error: {e}")
        else:
            # Fallback: emit a placeholder PEM block so the UI is honest about
            # what would happen, without faking trust material.
            st.session_state.mtls_cert = (
                "-----BEGIN CERTIFICATE-----\n"
                "(install `hwledger` CLI to enable real generation)\n"
                "-----END CERTIFICATE-----\n"
            )
            st.warning("`hwledger` CLI not on PATH; placeholder emitted.")
with cert_col2:
    cert = st.session_state.get("mtls_cert", "")
    if cert:
        st.code(cert, language="text")
        # Streamlit has no native clipboard; offer download instead.
        st.download_button("Download cert.pem", cert,
                           file_name="hwledger-client.pem",
                           mime="application/x-pem-file")

st.divider()

# --- HF ---
st.subheader("Hugging Face")
hf_token = st.text_input(
    "HF API token",
    value=st.session_state.get("hf_token", ""),
    type="password",
    help="Optional. Needed for gated models and to relieve anon rate limits.",
)
st.session_state.hf_token = hf_token
if hf_token:
    st.caption("Stored in session memory only.")

st.divider()

# --- Logging ---
st.subheader("Logging")
log_level = st.selectbox(
    "Level", ["trace", "debug", "info", "warn", "error"],
    index=["trace", "debug", "info", "warn", "error"].index(
        st.session_state.get("log_level", "info")
    ),
)
st.session_state.log_level = log_level

st.divider()

# --- About ---
st.subheader("About")
st.markdown(
    "- Repo: [KooshaPari/hwLedger](https://github.com/KooshaPari/hwLedger)\n"
    "- License: Apache-2.0\n"
    "- Streamlit client: parity with macOS SwiftUI app (see `PARITY.md`)"
)
