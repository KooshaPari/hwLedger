"""
Ledger Page: audit-event timeline, hash-chain verify, retention policy.

Surfaces:
- GET /v1/audit?limit=N&type=<filter>  → paginated event timeline
- GET /v1/audit/verify-chain           → boolean + broken-link index
- GET /v1/audit/policy                 → current retention policy

Visualizations (Plotly, Catppuccin Mocha):
- Timeline scatter colored by event type
- Event-rate histogram (per hour)
- Event-type pie
- Hash-chain badge (green/red)
- Retention-policy display
- Row click → event detail drawer

Traces to: FR-FLEET-006.
"""

from __future__ import annotations

from collections import Counter
from datetime import datetime, timezone
from typing import Any

import httpx
import plotly.graph_objects as go
import streamlit as st


st.set_page_config(page_title="Ledger - hwLedger", layout="wide")
st.title("Audit Ledger")
st.markdown("Event timeline, hash-chain verification, and retention policy.")

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
    "pink": "#f5c2e7",
    "sapphire": "#74c7ec",
}
_PALETTE = [
    _MOCHA["mauve"], _MOCHA["teal"], _MOCHA["blue"], _MOCHA["peach"],
    _MOCHA["pink"], _MOCHA["yellow"], _MOCHA["sapphire"], _MOCHA["green"],
]

server_url = st.session_state.get("server_url", "http://localhost:8080")
if not server_url:
    st.warning("Configure a server URL in Settings to use this page.")
    st.stop()

# --- Controls ---------------------------------------------------------------
c1, c2, c3, c4 = st.columns([1, 2, 1, 1])
with c1:
    limit = st.slider("Limit", 10, 1000, 100, key="ledger_limit")
with c2:
    type_filter = st.text_input(
        "Type filter (optional)",
        placeholder="e.g. agent_registered, ssh_probe_triggered",
    )
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
        r = c.get(f"{server_url}/v1/audit/verify-chain")
        r.raise_for_status()
        return r.json()


def _retention_policy() -> dict[str, Any]:
    try:
        with httpx.Client(timeout=5.0) as c:
            r = c.get(f"{server_url}/v1/audit/policy")
            if r.status_code == 404:
                return {"note": "Server does not expose /v1/audit/policy (404)."}
            r.raise_for_status()
            return r.json()
    except Exception as e:
        return {"error": str(e)}


# --- Chain badge + retention ------------------------------------------------
badge_col, retention_col = st.columns([1, 2])
with badge_col:
    st.markdown("**Hash chain**")
    try:
        res = _verify_chain()
        ok = bool(res.get("is_valid", res.get("valid", False)))
        head = (res.get("head_hash") or "")[:12]
        color = _MOCHA["green"] if ok else _MOCHA["red"]
        text = "VALID" if ok else f"BROKEN @ seq {res.get('broken_at_seq', '?')}"
        st.markdown(
            f"<div style='padding:10px 14px;border-radius:8px;"
            f"background:{color};color:{_MOCHA['base']};font-weight:600;"
            f"text-align:center'>{text}<br>"
            f"<span style='font-weight:400;font-family:monospace;font-size:11px;"
            f"opacity:0.85'>head {head}…</span></div>",
            unsafe_allow_html=True,
        )
    except Exception as e:
        st.markdown(
            f"<div style='padding:10px 14px;border-radius:8px;"
            f"background:{_MOCHA['surface']};color:{_MOCHA['text']};"
            f"text-align:center'>chain state unknown<br>"
            f"<span style='font-size:11px;opacity:0.75'>{e}</span></div>",
            unsafe_allow_html=True,
        )
with retention_col:
    with st.expander("Retention policy", expanded=True):
        policy = _retention_policy()
        if isinstance(policy, dict):
            m1, m2, m3 = st.columns(3)
            m1.metric(
                "Max events",
                policy.get("max_events") if policy.get("max_events") is not None else "∞",
            )
            m2.metric(
                "Max age (days)",
                policy.get("max_age_days") if policy.get("max_age_days") is not None else "∞",
            )
            m3.metric(
                "Snapshot every",
                policy.get("snapshot_every_n") if policy.get("snapshot_every_n") is not None else "—",
            )
        st.json(policy)

if verify:
    try:
        st.json(_verify_chain())
    except Exception as e:
        st.error(f"Verify failed: {e}")


# --- Events fetch -----------------------------------------------------------
try:
    events = _fetch_events()
except httpx.ConnectError:
    st.error(f"Cannot reach server at {server_url}.")
    st.stop()
except httpx.HTTPStatusError as e:
    st.error(f"Server returned {e.response.status_code}: {e.response.text[:200]}")
    st.stop()
except Exception as e:
    st.error(f"Error: {e}")
    st.stop()

if not events:
    st.info("No audit events found.")
    st.stop()

st.success(f"Retrieved {len(events)} event(s)")


def _parse_ts(evt: dict[str, Any]) -> datetime:
    ms = evt.get("appended_at_ms")
    if isinstance(ms, (int, float)):
        return datetime.fromtimestamp(ms / 1000.0, tz=timezone.utc)
    ts = evt.get("appended_at") or evt.get("timestamp") or ""
    try:
        return datetime.fromisoformat(str(ts).replace("Z", "+00:00"))
    except Exception:
        return datetime.now(tz=timezone.utc)


def _type_of(evt: dict[str, Any]) -> str:
    return str(evt.get("event_type") or evt.get("type") or "unknown")


xs: list[datetime] = []
ys: list[str] = []
types: list[str] = []
texts: list[str] = []
type_palette: dict[str, str] = {}
for evt in events:
    t = _type_of(evt)
    if t not in type_palette:
        type_palette[t] = _PALETTE[len(type_palette) % len(_PALETTE)]
    xs.append(_parse_ts(evt))
    ys.append(t)
    types.append(t)
    texts.append(f"{t} · seq {evt.get('seq', '?')} · {evt.get('actor', 'system')}")

# --- Timeline scatter -------------------------------------------------------
fig_tl = go.Figure()
for t, color in type_palette.items():
    mask = [i for i, tt in enumerate(types) if tt == t]
    fig_tl.add_trace(
        go.Scatter(
            x=[xs[i] for i in mask],
            y=[ys[i] for i in mask],
            mode="markers",
            marker=dict(size=12, color=color, line=dict(color=_MOCHA["base"], width=1)),
            text=[texts[i] for i in mask],
            name=t,
        )
    )
fig_tl.update_layout(
    height=280,
    margin=dict(l=0, r=0, t=10, b=0),
    plot_bgcolor=_MOCHA["base"],
    paper_bgcolor=_MOCHA["base"],
    font=dict(color=_MOCHA["text"]),
    xaxis_title="time",
    yaxis_title="event type",
    hovermode="closest",
    legend=dict(bgcolor=_MOCHA["surface"]),
)
st.plotly_chart(fig_tl, use_container_width=True, key="ledger_timeline")

# --- Rate + type pie --------------------------------------------------------
rate_col, pie_col = st.columns(2)
with rate_col:
    hour_buckets = Counter(x.replace(minute=0, second=0, microsecond=0) for x in xs)
    bx = sorted(hour_buckets.keys())
    by = [hour_buckets[b] for b in bx]
    fig_rate = go.Figure(
        go.Bar(x=bx, y=by, marker_color=_MOCHA["sapphire"])
    )
    fig_rate.update_layout(
        title=dict(text="Events per hour", font=dict(color=_MOCHA["text"])),
        height=260,
        margin=dict(l=0, r=0, t=40, b=0),
        plot_bgcolor=_MOCHA["base"],
        paper_bgcolor=_MOCHA["base"],
        font=dict(color=_MOCHA["text"]),
        yaxis_title="count",
    )
    st.plotly_chart(fig_rate, use_container_width=True, key="ledger_rate")
with pie_col:
    type_counter = Counter(types)
    fig_pie = go.Figure(
        go.Pie(
            labels=list(type_counter.keys()),
            values=list(type_counter.values()),
            marker=dict(colors=[type_palette[t] for t in type_counter.keys()]),
            textinfo="label+percent",
        )
    )
    fig_pie.update_layout(
        title=dict(text="Event type mix", font=dict(color=_MOCHA["text"])),
        height=260,
        margin=dict(l=0, r=0, t=40, b=0),
        paper_bgcolor=_MOCHA["base"],
        font=dict(color=_MOCHA["text"]),
        showlegend=False,
    )
    st.plotly_chart(fig_pie, use_container_width=True, key="ledger_pie")

# --- Event detail drawer ----------------------------------------------------
st.subheader("Events (newest first)")

# Build a lightweight table view
table_rows: list[dict[str, Any]] = []
for evt in events:
    ts_dt = _parse_ts(evt)
    table_rows.append(
        {
            "seq": evt.get("seq"),
            "appended_at": ts_dt.isoformat(),
            "event_type": _type_of(evt),
            "actor": evt.get("actor", "system"),
            "hash": (evt.get("hash") or "")[:12],
        }
    )
st.dataframe(table_rows, use_container_width=True, hide_index=True)

seq_options = [e.get("seq") for e in events if e.get("seq") is not None]
selected = st.selectbox(
    "Inspect event (by seq)",
    options=["—"] + [str(s) for s in seq_options],
    index=0,
    key="ledger_drawer_select",
)
if selected and selected != "—":
    match = next((e for e in events if str(e.get("seq")) == selected), None)
    if match is not None:
        with st.expander(f"Event seq {selected}", expanded=True):
            meta_col, payload_col = st.columns([1, 2])
            with meta_col:
                st.markdown(f"**Type:** `{_type_of(match)}`")
                st.markdown(f"**Appended:** {_parse_ts(match).isoformat()}")
                st.markdown(f"**Hash:** `{match.get('hash', '')[:16]}…`")
                st.markdown(f"**Prev hash:** `{(match.get('previous_hash') or '')[:16]}…`")
                st.markdown(f"**Actor:** `{match.get('actor', 'system')}`")
            with payload_col:
                st.json(match.get("event", match))
