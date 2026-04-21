"""
HF Search page.

Real search against HuggingFace Hub via sibling-agent FFI. Works anonymously
(token optional). Offers a "Quick picks" band of recent 2025-2026 models with
download-count badges, a faceted search box (library + tags + sort), a result
table, and a "Use this model" action that hands off to the Planner.

Traces to: brief §3 (HF search), §4 (real HF search UI).
"""

from __future__ import annotations

import streamlit as st

from lib.ffi_ext import (
    HfModel,
    HfSearchResult,
    backend_status,
    quick_picks,
    search_available,
    search_hf,
)


st.set_page_config(page_title="HF Search - hwLedger", layout="wide")
st.title("HuggingFace Search")
st.markdown(
    "Search the HuggingFace Hub for models. "
    "Anonymous works; add an HF token in **Settings** for higher rate limits "
    "and gated-model access."
)

status = backend_status()
if not status.ffi_search:
    st.info(
        "Sibling `hwledger_search_*` FFI symbols are not yet available; "
        "returning curated quick-picks + local filter. Wire-up will go live "
        "automatically once the sibling crate builds into the shared dylib."
    )


def _format_downloads(n: int) -> str:
    if n >= 1_000_000:
        return f"{n / 1_000_000:.1f}M"
    if n >= 1_000:
        return f"{n / 1_000:.1f}K"
    return str(n)


# --- Quick picks band ---
st.subheader("Quick picks · 2025-2026 releases")
qp = quick_picks()[:12]
qcols = st.columns(4)
for i, m in enumerate(qp):
    with qcols[i % 4]:
        with st.container(border=True):
            st.markdown(f"**{m.id.split('/')[-1]}**")
            st.caption(f"{m.author} · {_format_downloads(m.downloads)} downloads")
            st.markdown(
                " ".join(f"<span style='font-size:10px;background:#EEE;"
                         f"padding:2px 6px;border-radius:4px;margin-right:2px;'>"
                         f"{t}</span>" for t in m.tags[:3]),
                unsafe_allow_html=True,
            )
            if st.button("Use this model", key=f"qp_use_{m.id}",
                         use_container_width=True):
                st.session_state.pending_model_id = m.id
                st.switch_page("pages/01_Planner.py")

st.divider()

# --- Search ---
st.subheader("Search")
c1, c2, c3, c4 = st.columns([3, 2, 2, 1])
with c1:
    q = st.text_input("Query", placeholder="llama, qwen, mistral-nemo, ...",
                      label_visibility="collapsed")
with c2:
    library = st.selectbox("Library", ["(any)", "transformers", "gguf",
                                        "mlx", "diffusers", "sentence-transformers"])
    library = None if library == "(any)" else library
with c3:
    sort = st.selectbox("Sort", ["downloads", "likes", "updated"])
with c4:
    limit = st.number_input("Limit", min_value=5, max_value=100,
                            value=25, step=5)

token = st.session_state.get("hf_token") or None

if st.button("Search", type="primary"):
    with st.spinner("Querying HF Hub..."):
        result: HfSearchResult = search_hf(q or "", library=library,
                                            sort=sort, limit=int(limit),
                                            token=token)
    st.session_state.hf_last_result = result

result: HfSearchResult | None = st.session_state.get("hf_last_result")
if result is not None:
    # Rate-limit surfacing
    if result.rate_limited:
        st.error(
            f"HF Hub rate-limited this client (HTTP 429). "
            f"Retry in ~{result.next_retry_after_s or 60}s, "
            f"or paste a token in Settings to raise the cap."
        )
    elif result.rate_limit_remaining is not None and result.rate_limit_remaining < 20:
        st.warning(
            f"Low HF rate budget: {result.rate_limit_remaining} calls "
            f"remaining this window."
        )

    st.caption(f"{len(result.models)} of {result.total} results")

    # Table
    rows = [{
        "Model": m.id,
        "Author": m.author,
        "Downloads": _format_downloads(m.downloads),
        "Likes": m.likes,
        "Library": m.library,
        "Tags": ", ".join(m.tags[:4]),
        "Updated": m.last_modified,
        "Gated": "yes" if m.gated else "",
    } for m in result.models]
    st.dataframe(rows, use_container_width=True, hide_index=True)

    # Per-row action
    st.subheader("Model actions")
    for m in result.models[:20]:
        ac1, ac2, ac3 = st.columns([4, 2, 1])
        with ac1:
            st.markdown(f"**{m.id}** — {_format_downloads(m.downloads)} downloads")
        with ac2:
            st.caption(", ".join(m.tags[:5]))
        with ac3:
            if st.button("Plan it →", key=f"use_{m.id}"):
                st.session_state.pending_model_id = m.id
                st.switch_page("pages/01_Planner.py")
