"""
Fleet Page: remote server audit + node map + SSH probe trigger.

Parity with SwiftUI FleetScreen + FleetMapScreen (inferred):
- List agents via /v1/agents
- Plotly scatter "node map" (topological placement by agent id hash)
- Per-node detail panel with structured view + raw JSON toggle
- SSH-probe trigger that POSTs /v1/agents/{id}/probe
"""

from __future__ import annotations

import hashlib
import json

import httpx
import plotly.graph_objects as go
import streamlit as st


st.set_page_config(page_title="Fleet - hwLedger", layout="wide")
st.title("Fleet Audit")
st.markdown("Remote hwLedger agents, topology, and SSH probe trigger.")

server_url = st.session_state.get("server_url", "http://localhost:8080")
if not server_url:
    st.warning("Configure a server URL in Settings to use this page.")
    st.stop()

col_head1, col_head2 = st.columns([4, 1])
with col_head1:
    st.code(server_url, language="text")
with col_head2:
    if st.button("Refresh", use_container_width=True):
        st.rerun()


def _fetch_agents() -> list[dict]:
    with httpx.Client(timeout=5.0) as c:
        r = c.get(f"{server_url}/v1/agents")
        r.raise_for_status()
        data = r.json()
    return data if isinstance(data, list) else data.get("agents", [])


def _ssh_probe(agent_id: str) -> dict:
    with httpx.Client(timeout=10.0) as c:
        r = c.post(f"{server_url}/v1/agents/{agent_id}/probe")
        r.raise_for_status()
        return r.json()


try:
    agents = _fetch_agents()
except httpx.ConnectError:
    st.error(f"Cannot reach server at {server_url}. Check URL and network.")
    st.stop()
except httpx.HTTPStatusError as e:
    st.error(f"Server returned {e.response.status_code}.")
    st.stop()
except Exception as e:
    st.error(f"Error: {e}")
    st.stop()

if not agents:
    st.info("No agents registered on this server.")
    st.stop()

st.success(f"Found {len(agents)} agent(s)")

# --- Node map ---
st.subheader("Node map")
xs, ys, labels, colors = [], [], [], []
for a in agents:
    aid = str(a.get("id", a.get("hostname", "?")))
    # Deterministic placement via hash → unit disk-ish
    h = hashlib.sha256(aid.encode()).digest()
    xs.append((h[0] - 128) / 128.0)
    ys.append((h[1] - 128) / 128.0)
    labels.append(aid)
    status = (a.get("status") or "unknown").lower()
    colors.append({"online": "#4ECDC4", "offline": "#FF6B6B",
                   "degraded": "#FFA07A"}.get(status, "#888"))

fig = go.Figure(go.Scatter(
    x=xs, y=ys, mode="markers+text",
    marker=dict(size=28, color=colors, line=dict(color="white", width=2),
                symbol="hexagon"),
    text=labels, textposition="bottom center",
))
fig.update_layout(
    height=400, showlegend=False, margin=dict(l=0, r=0, t=0, b=0),
    xaxis=dict(visible=False, range=[-1.3, 1.3]),
    yaxis=dict(visible=False, range=[-1.3, 1.3]),
    plot_bgcolor="rgba(245,245,250,1)",
)
st.plotly_chart(fig, use_container_width=True, key="fleet_map")

# --- Detail panels ---
st.subheader("Agents")
for agent in agents:
    aid = str(agent.get("id", "unknown"))
    with st.expander(f"Agent · {aid}", expanded=False):
        dcol1, dcol2 = st.columns([2, 1])
        with dcol1:
            st.markdown(f"**Hostname:** `{agent.get('hostname', '—')}`")
            st.markdown(f"**Status:** {agent.get('status', 'unknown')}")
            st.markdown(f"**Last seen:** {agent.get('last_seen', '—')}")
            st.markdown(f"**GPUs:** {len(agent.get('devices', []))}")
        with dcol2:
            if st.button("SSH probe", key=f"probe_{aid}",
                         use_container_width=True):
                try:
                    result = _ssh_probe(aid)
                    st.success("Probe queued")
                    st.json(result)
                except Exception as e:
                    st.error(f"Probe failed: {e}")

        if st.toggle("Raw JSON", key=f"raw_{aid}"):
            st.json(agent)
