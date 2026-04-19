"""
Fleet Page: Remote server audit via HTTP API.

Connects to hwLedger server /v1/agents endpoint.
"""

import streamlit as st
import httpx
import json


st.set_page_config(page_title="Fleet - hwLedger", layout="wide")

st.title("Fleet Audit")
st.markdown("Monitor remote hwLedger servers and agent status.")

# Get server URL from session state
server_url = st.session_state.get("server_url", "http://localhost:8080")

if not server_url:
    st.warning("Configure a server URL in Settings to use this page.")
    st.stop()

# Fetch agents
col1, col2 = st.columns([3, 1])

with col1:
    st.subheader("Connected Servers")
    st.code(server_url, language="text")

with col2:
    if st.button("Refresh"):
        st.rerun()

try:
    async def fetch_agents():
        async with httpx.AsyncClient() as client:
            resp = await client.get(f"{server_url}/v1/agents", timeout=5.0)
            return resp.json()

    # Use sync client for simplicity
    with httpx.Client(timeout=5.0) as client:
        resp = client.get(f"{server_url}/v1/agents")
        agents = resp.json()

    if not agents:
        st.info("No agents registered on this server.")
    else:
        st.success(f"Found {len(agents)} agent(s)")

        for agent in agents:
            with st.expander(f"Agent: {agent.get('id', 'unknown')}", expanded=True):
                st.json(agent)

except httpx.ConnectError:
    st.error(f"Cannot reach server at {server_url}. Check URL and network.")
except httpx.TimeoutException:
    st.error(f"Request timeout to {server_url}. Server may be offline.")
except json.JSONDecodeError:
    st.error(f"Invalid JSON response from {server_url}.")
except Exception as e:
    st.error(f"Error: {e}")
