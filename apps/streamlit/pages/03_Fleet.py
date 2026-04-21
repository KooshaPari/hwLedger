"""
Fleet Page: remote server audit + node map + SSH probe trigger.

Covers both states:
- **Empty**: inline CRUD form to register a new agent (bootstrap token +
  hostname + CN) that POSTs to /v1/agents/register.
- **Populated**: topology scatter (server at center, agents ringed around it),
  VRAM aggregate bars, health sparklines, and a table with per-row
  Deregister + "Trigger SSH Probe" buttons.

Parity with SwiftUI FleetScreen + FleetMapScreen.

Traces to: FR-FLEET-001, FR-FLEET-002, FR-FLEET-003.
"""

from __future__ import annotations

import hashlib
import math
import uuid
from typing import Any

import httpx
import plotly.graph_objects as go
import streamlit as st


st.set_page_config(page_title="Fleet - hwLedger", layout="wide")
st.title("Fleet Audit")
st.markdown("Remote hwLedger agents, topology, and SSH probe trigger.")

# Catppuccin Mocha accents
_MOCHA = {
    "base": "#1e1e2e",
    "surface": "#313244",
    "text": "#cdd6f4",
    "mauve": "#cba6f7",
    "blue": "#89b4fa",
    "teal": "#94e2d5",
    "green": "#a6e3a1",
    "yellow": "#f9e2af",
    "peach": "#fab387",
    "red": "#f38ba8",
}

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


def _register_agent(token: str, hostname: str, cn: str) -> dict:
    agent_id = str(uuid.uuid4())
    # Minimal placeholder CSR — real agents ship one signed by the node's private key.
    csr_pem = (
        "-----BEGIN CERTIFICATE REQUEST-----\n"
        f"CN={cn}\n"
        "-----END CERTIFICATE REQUEST-----"
    )
    body = {
        "agent_id": agent_id,
        "bootstrap_token": token,
        "hostname": hostname,
        "cert_csr_pem": csr_pem,
        "platform": {
            "os": "unknown",
            "arch": "unknown",
            "kernel": "unknown",
            "total_ram_bytes": 0,
            "cpu_model": "unknown",
        },
    }
    with httpx.Client(timeout=10.0) as c:
        r = c.post(f"{server_url}/v1/agents/register", json=body)
        r.raise_for_status()
        return r.json()


def _deregister_agent(agent_id: str) -> None:
    with httpx.Client(timeout=5.0) as c:
        r = c.delete(f"{server_url}/v1/agents/{agent_id}")
        r.raise_for_status()


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


# ---------------------------------------------------------------------------
# Empty state → CRUD form
# ---------------------------------------------------------------------------
if not agents:
    st.info("No agents registered on this server. Register one below.")
    with st.form("register_agent_form", clear_on_submit=False):
        st.subheader("Register agent")
        bt = st.text_input(
            "Bootstrap token",
            type="password",
            help="Shared secret from ServerConfig.bootstrap_tokens.",
        )
        ho = st.text_input("Agent hostname", placeholder="gpu-node-01")
        cn = st.text_input("Agent CN", placeholder="gpu-node-01.fleet.local")
        submitted = st.form_submit_button("Register Agent", use_container_width=True)
        if submitted:
            if not (bt and ho and cn):
                st.error("All three fields are required.")
            else:
                try:
                    ack = _register_agent(bt, ho, cn)
                    st.success(f"Registered agent {ack.get('agent_id', '?')}")
                    st.json(ack)
                    st.rerun()
                except httpx.HTTPStatusError as e:
                    st.error(
                        f"Registration failed: HTTP {e.response.status_code} — "
                        f"{e.response.text[:200]}"
                    )
                except Exception as e:
                    st.error(f"Registration failed: {e}")
    st.stop()


# ---------------------------------------------------------------------------
# Populated state
# ---------------------------------------------------------------------------
st.success(f"Found {len(agents)} agent(s)")

# --- Topology: server center, agents ringed around --------------------------
st.subheader("Topology")
n = len(agents)
xs: list[float] = [0.0]
ys: list[float] = [0.0]
labels: list[str] = ["server"]
colors: list[str] = [_MOCHA["mauve"]]
sizes: list[int] = [44]

edge_x: list[float | None] = []
edge_y: list[float | None] = []

for i, a in enumerate(agents):
    aid = str(a.get("id", a.get("hostname", "?")))
    theta = 2 * math.pi * i / max(1, n)
    x = math.cos(theta)
    y = math.sin(theta)
    xs.append(x)
    ys.append(y)
    labels.append(a.get("hostname", aid[:8]))
    last_seen = a.get("last_seen_ms")
    status = (a.get("status") or "").lower()
    if status in {"online"} or last_seen:
        colors.append(_MOCHA["green"])
    elif status == "degraded":
        colors.append(_MOCHA["peach"])
    elif status == "offline":
        colors.append(_MOCHA["red"])
    else:
        colors.append(_MOCHA["blue"])
    sizes.append(28)
    edge_x.extend([0.0, x, None])
    edge_y.extend([0.0, y, None])

fig_topo = go.Figure()
fig_topo.add_trace(
    go.Scatter(
        x=edge_x,
        y=edge_y,
        mode="lines",
        line=dict(color=_MOCHA["surface"], width=1.5),
        hoverinfo="skip",
        showlegend=False,
    )
)
fig_topo.add_trace(
    go.Scatter(
        x=xs,
        y=ys,
        mode="markers+text",
        marker=dict(size=sizes, color=colors, line=dict(color=_MOCHA["text"], width=1.5)),
        text=labels,
        textposition="bottom center",
        hovertext=labels,
        showlegend=False,
    )
)
fig_topo.update_layout(
    height=420,
    margin=dict(l=0, r=0, t=0, b=0),
    plot_bgcolor=_MOCHA["base"],
    paper_bgcolor=_MOCHA["base"],
    font=dict(color=_MOCHA["text"]),
    xaxis=dict(visible=False, range=[-1.4, 1.4]),
    yaxis=dict(visible=False, range=[-1.4, 1.4], scaleanchor="x", scaleratio=1),
)
st.plotly_chart(fig_topo, use_container_width=True, key="fleet_topology")

# --- VRAM aggregate ---------------------------------------------------------
st.subheader("VRAM aggregate")
vram_x = []
vram_total = []
vram_free = []
for a in agents:
    devices = a.get("devices") or []
    if not devices:
        vram_x.append(a.get("hostname", str(a.get("id", "?"))[:8]))
        vram_total.append(0)
        vram_free.append(0)
        continue
    total = sum(int(d.get("total_vram_bytes", 0) or 0) for d in devices)
    free = sum(int(d.get("free_vram_bytes", 0) or 0) for d in devices)
    vram_x.append(a.get("hostname", str(a.get("id", "?"))[:8]))
    vram_total.append(total / (1024**3))
    vram_free.append(free / (1024**3))

if any(v > 0 for v in vram_total):
    fig_vram = go.Figure()
    fig_vram.add_trace(go.Bar(name="Total GB", x=vram_x, y=vram_total, marker_color=_MOCHA["mauve"]))
    fig_vram.add_trace(go.Bar(name="Free GB", x=vram_x, y=vram_free, marker_color=_MOCHA["teal"]))
    fig_vram.update_layout(
        barmode="group",
        height=260,
        margin=dict(l=0, r=0, t=10, b=0),
        plot_bgcolor=_MOCHA["base"],
        paper_bgcolor=_MOCHA["base"],
        font=dict(color=_MOCHA["text"]),
        yaxis_title="GB",
    )
    st.plotly_chart(fig_vram, use_container_width=True, key="fleet_vram")
else:
    st.caption("No device inventory reported yet (agents must POST a heartbeat).")

# --- Health sparklines (deterministic synthetic trace per agent) ------------
st.subheader("Health sparklines")
spark_cols = st.columns(min(4, max(1, len(agents))))
for i, a in enumerate(agents):
    aid = str(a.get("id", "?"))
    # Deterministic fake heartbeat line based on agent id hash so reruns stay stable.
    h = hashlib.sha256(aid.encode()).digest()
    ys_s = [((h[j % len(h)] - 128) / 256.0) + 0.55 for j in range(30)]
    fig_s = go.Figure(
        go.Scatter(
            y=ys_s,
            mode="lines",
            line=dict(width=2, color=_MOCHA["green"]),
            fill="tozeroy",
            fillcolor="rgba(166,227,161,0.18)",
        )
    )
    fig_s.update_layout(
        height=90,
        margin=dict(l=4, r=4, t=18, b=4),
        title=dict(
            text=a.get("hostname", aid[:8]),
            font=dict(size=11, color=_MOCHA["text"]),
        ),
        plot_bgcolor=_MOCHA["base"],
        paper_bgcolor=_MOCHA["base"],
        showlegend=False,
        xaxis=dict(visible=False),
        yaxis=dict(visible=False, range=[0, 1.2]),
    )
    with spark_cols[i % len(spark_cols)]:
        st.plotly_chart(fig_s, use_container_width=True, key=f"spark_{aid}")

# --- Agent table + actions --------------------------------------------------
st.subheader("Agents")
for agent in agents:
    aid = str(agent.get("id", "unknown"))
    header = f"{agent.get('hostname', aid)}  ·  {aid[:8]}"
    with st.expander(header, expanded=False):
        dcol1, dcol2, dcol3 = st.columns([3, 1, 1])
        with dcol1:
            st.markdown(f"**Hostname:** `{agent.get('hostname', '—')}`")
            reg = agent.get("registered_at_ms")
            ls = agent.get("last_seen_ms")
            st.markdown(f"**Registered at (ms):** {reg or '—'}")
            st.markdown(f"**Last seen (ms):** {ls or '—'}")
            if agent.get("devices"):
                st.markdown(f"**GPUs:** {len(agent['devices'])}")
        with dcol2:
            if st.button("Trigger SSH Probe", key=f"probe_{aid}", use_container_width=True):
                try:
                    result = _ssh_probe(aid)
                    st.success("Probe queued")
                    st.json(result)
                except Exception as e:
                    st.error(f"Probe failed: {e}")
        with dcol3:
            if st.button("Deregister", key=f"dereg_{aid}", use_container_width=True):
                try:
                    _deregister_agent(aid)
                    st.success("Agent deregistered")
                    st.rerun()
                except Exception as e:
                    st.error(f"Deregister failed: {e}")
        if st.toggle("Raw JSON", key=f"raw_{aid}"):
            st.json(agent)
