# ADR 0026 — Internal web framework: Streamlit

Constrains: FR-DASH-001..003

Date: 2026-04-19
Status: Accepted

## Context

hwLedger ships internal Python dashboards (fleet cost, journey run summaries, telemetry explorations). These are single-user-ish ops tools that must be authored by data-minded contributors in hours, not days, and render Plotly charts (ADR-0027) alongside tabular data pulled from SQLite (ADR-0019) or the fleet API (ADR-0018).

## Options

| Framework | Author speed | Chart integration | Reactive model | Auth | Deploy footprint |
|---|---|---|---|---|---|
| Streamlit | Fast | Native Plotly/Altair | Top-to-bottom rerun | Via reverse proxy | Docker/systemd |
| Gradio | Fast | Good for ML demos | Event-based | Built-in share links | Docker |
| Panel (HoloViz) | Medium | Native Bokeh/Plotly | Param-based | Via tornado | Docker |
| Taipy | Medium | Good | State-machine | Built-in | Docker |
| NiceGUI | Medium | Good | Reactive | External | Docker |
| Reflex (ex Pynecone) | Slow (React-ish) | OK | React model | External | Heavier |
| Dash (Plotly) | Slow | Native (Plotly) | Callback graph | External | Docker |

## Decision

Use **Streamlit 1.x** for all internal dashboards in `apps/dashboards/` under `uv` (ADR-0028). Auth via an nginx reverse-proxy with basic auth on the self-hosted host.

## Rationale

- Fastest time-to-usable of any Python dash framework. A new dashboard is ~100 LOC.
- Plotly support is native (ADR-0027), matching our chart standard.
- No JS build step; no React mental model; onboarding is trivial.
- Streamlit's "rerun on input change" model fits ops tooling where state is ephemeral.
- Reflex/Dash are more powerful but require committing to a component/callback architecture we don't need for ops UIs.

## Consequences

- Streamlit is not suitable for multi-tenant production apps — no RBAC, weak session isolation. Fine, since dashboards are internal behind auth.
- Reruns can be slow on large dataframes; we cache heavy reads with `@st.cache_data` and pull pre-aggregated summaries from the fleet API.
- End users cannot customize components without a React/JS detour.

## Revisit when

- Dashboards need real multi-user RBAC or embedding in the customer product.
- Streamlit's maintainership/licensing changes materially (Snowflake acquired; stable as of 2026-04).

## References

- Streamlit: https://streamlit.io
- ADR-0027 (Plotly), ADR-0028 (uv).
