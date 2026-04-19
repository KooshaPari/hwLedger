"""
Ledger Page: Event log timeline from remote server.

Fetches from /v1/audit endpoint.
"""

import streamlit as st
import httpx
import json
from datetime import datetime


st.set_page_config(page_title="Ledger - hwLedger", layout="wide")

st.title("Audit Ledger")
st.markdown("View event timeline from hwLedger servers.")

server_url = st.session_state.get("server_url", "http://localhost:8080")

if not server_url:
    st.warning("Configure a server URL in Settings to use this page.")
    st.stop()

# Filters
col1, col2, col3 = st.columns(3)
with col1:
    limit = st.slider("Event limit", min_value=10, max_value=1000, value=100)
with col2:
    event_type = st.text_input("Filter by type (optional)", placeholder="e.g., plan, probe")
with col3:
    if st.button("Fetch Events"):
        st.rerun()

# Fetch events
try:
    params = {"limit": limit}
    if event_type:
        params["type"] = event_type

    with httpx.Client(timeout=10.0) as client:
        resp = client.get(f"{server_url}/v1/audit", params=params)
        events = resp.json()

    if not events:
        st.info("No audit events found.")
    else:
        st.success(f"Retrieved {len(events)} event(s)")

        # Timeline view
        for event in reversed(events):  # Newest first
            timestamp = event.get("timestamp", "unknown")
            event_type_val = event.get("type", "unknown")
            actor = event.get("actor", "system")

            with st.container():
                col1, col2 = st.columns([1, 5])
                with col1:
                    st.caption(timestamp)
                with col2:
                    st.markdown(f"**{event_type_val}** by {actor}")
                    if "details" in event:
                        st.json(event["details"])
            st.divider()

except httpx.ConnectError:
    st.error(f"Cannot reach server at {server_url}.")
except Exception as e:
    st.error(f"Error: {e}")
