"""
Settings Page: Configuration and preferences.
"""

import streamlit as st


st.set_page_config(page_title="Settings - hwLedger", layout="wide")

st.title("Settings")
st.markdown("Configure hwLedger Streamlit client.")

# Server configuration
st.subheader("Server")
col1, col2 = st.columns([3, 1])

with col1:
    new_server_url = st.text_input(
        "API Server URL",
        value=st.session_state.get("server_url", "http://localhost:8080"),
        help="Base URL for remote hwLedger server (used by Fleet and Ledger pages)",
    )
    st.session_state.server_url = new_server_url

with col2:
    if st.button("Test Connection"):
        import httpx
        try:
            with httpx.Client(timeout=3.0) as client:
                resp = client.get(f"{new_server_url}/health")
                if resp.status_code == 200:
                    st.success("Connected!")
                else:
                    st.warning(f"Status: {resp.status_code}")
        except Exception as e:
            st.error(f"Connection failed: {e}")

st.divider()

# HF Token
st.subheader("Authentication")
new_hf_token = st.text_input(
    "Hugging Face Token",
    value=st.session_state.get("hf_token", ""),
    type="password",
    help="Token for HF model authentication (not persisted)",
)
st.session_state.hf_token = new_hf_token

if new_hf_token:
    st.caption("Token is stored in session memory only and not persisted to disk.")

st.divider()

# System info
st.subheader("System Info")

from lib.ffi import is_available

st.markdown(f"**FFI Library**: {'Available' if is_available() else 'Not Found'}")
st.markdown("**Python Version**: 3.11+")
st.markdown("**Streamlit Client**: 1.40+")

st.divider()

# Help
st.subheader("Help")
st.markdown("""
### Pages

- **Planner**: Real-time memory planning with slider controls. Requires FFI library.
- **Probe**: Device detection and VRAM inventory. Requires FFI library.
- **Fleet**: Remote server audit via HTTP API.
- **Ledger**: Event timeline from remote server.

### Build FFI Library

```bash
cd hwLedger
cargo build --release -p hwledger-ffi
```

The native library will be loaded from `target/release/`.

### Run Streamlit

```bash
cd apps/streamlit
./scripts/run-streamlit.sh
```

Launches on port 8501.
""")
