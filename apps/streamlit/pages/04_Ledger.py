"""
Ledger Page: audit-event timeline, hash-chain verify, retention policy.

Parity with SwiftUI LedgerScreen:
- Load /v1/audit?limit=N
- Verify hash chain via /v1/audit/verify
- Retention policy via /v1/audit/retention (graceful fallback)
- Timeline (Plotly scatter) + detailed event JSON viewer
"""

from __future__ import annotations

from datetime import datetime
from typing import Any

import httpx
import plotly.graph_objects as go
import streamlit as st


st.set_page_config(page_title="Ledger - hwLedger", layout="wide")
st.title("Audit Ledger")
st.markdown("Event timeline, hash-chain verification, and retention policy.")

server_url = st.session_state.get("server_url", "http://localhost:8080")
if not server_url:
    st.warning("Configure a server URL in Settings to use this page.")
    st.stop()

# --- Controls ---
c1, c2, c3, c4 = st.columns([1, 2, 1, 1])
with c1:
    limit = st.slider("Limit", 10, 1000, 100, key="ledger_limit")
with c2:
    type_filter = st.text_input("Type filter (optional)",
                                placeholder="e.g. plan, probe, ingest")
with c3:
    load = st.button("Load", use_container_width=True)
with c4:
    verify = st.button("Verify chain", use_container_width=True)


def _fetch_events() -> list[dict[str, Any]]:
    params: dict[str, Any] = {"limit": limit}
    if type_filter:
        params["type"] = type_filter
    with httpx.Client(timeout=10.0) as c:
        r = c.get(f"{server_url}/v1/audit", params=params)
        r.raise_for_status()
        body = r.json()
    if isinstance(body, list):
        return body
    return body.get("events", [])


def _verify_chain() -> dict[str, Any]:
    with httpx.Client(timeout=10.0) as c:
        r = c.get(f"{server_url}/v1/audit/verify")
        r.raise_for_status()
        return r.json()


def _retention_policy() -> dict[str, Any]:
    try:
        with httpx.Client(timeout=5.0) as c:
            r = c.get(f"{server_url}/v1/audit/retention")
            if r.status_code == 404:
                return {
                    "max_age_days": None,
                    "max_events": None,
                    "note": "Server does not expose retention policy (endpoint 404).",
                }
            r.raise_for_status()
            return r.json()
    except Exception as e:
        return {"error": str(e)}


# --- Retention section ---
with st.expander("Retention policy", expanded=False):
    policy = _retention_policy()
    st.json(policy)


# --- Verify ---
if verify:
    try:
        res = _verify_chain()
        if res.get("is_valid", res.get("valid", False)):
            st.success(f"Hash chain verified · head={res.get('head_hash', '—')[:12]}…")
        else:
            st.error(f"Hash chain BROKEN at seq {res.get('broken_at_seq', '?')}")
        st.json(res)
    except Exception as e:
        st.error(f"Verify failed: {e}")


# --- Events ---
try:
    events = _fetch_events()
except httpx.ConnectError:
    st.error(f"Cannot reach server at {server_url}.")
    st.stop()
except Exception as e:
    st.error(f"Error: {e}")
    st.stop()

if not events:
    st.info("No audit events found.")
    st.stop()

st.success(f"Retrieved {len(events)} event(s)")

# Timeline scatter
xs, ys, texts, types = [], [], [], []
type_palette: dict[str, str] = {}
palette_pool = ["#4ECDC4", "#BB77DD", "#45B7D1", "#FFA07A", "#FF6B6B", "#8FD694"]
for i, evt in enumerate(events):
    ts = evt.get("appended_at") or evt.get("timestamp") or ""
    try:
        x = datetime.fromisoformat(ts.replace("Z", "+00:00"))
    except Exception:
        x = datetime.utcnow()
    t = evt.get("event_type") or evt.get("type") or "unknown"
    if t not in type_palette:
        type_palette[t] = palette_pool[len(type_palette) % len(palette_pool)]
    xs.append(x)
    ys.append(t)
    types.append(t)
    texts.append(f"{t} by {evt.get('actor', 'system')}")

fig = go.Figure()
for t, color in type_palette.items():
    mask = [i for i, tt in enumerate(types) if tt == t]
    fig.add_trace(go.Scatter(
        x=[xs[i] for i in mask], y=[ys[i] for i in mask],
        mode="markers", marker=dict(size=12, color=color),
        text=[texts[i] for i in mask], name=t,
    ))
fig.update_layout(height=260, margin=dict(l=0, r=0, t=10, b=0),
                  xaxis_title="time", yaxis_title="event type",
                  hovermode="closest")
st.plotly_chart(fig, use_container_width=True, key="ledger_timeline")

# Event list with detail expander
st.subheader("Events (newest first)")
for evt in reversed(events):
    ts = evt.get("appended_at") or evt.get("timestamp") or "?"
    t = evt.get("event_type") or evt.get("type") or "unknown"
    actor = evt.get("actor", "system")
    hsh = evt.get("hash", "")[:8]
    with st.expander(f"{ts} · {t} · {actor} · hash {hsh}"):
        st.json(evt)
