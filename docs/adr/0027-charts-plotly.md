# ADR 0027 — Charting library: Plotly (single standard)

Constrains: FR-DOCS-005, FR-DASH-002

Date: 2026-04-19
Status: Accepted

## Context

Charts appear in three places: Streamlit dashboards (ADR-0026), docs-site `<Shot>` / interactive blocks, and static PNG renders for reports. Using one chart lib everywhere lets us share theming, accessibility tokens (ADR-0021 color policy), and maintenance.

## Options

| Lib | Python | JS/TS | Static PNG | Theming | Interactivity | License |
|---|---|---|---|---|---|---|
| Plotly | plotly.py | plotly.js | Yes (`kaleido`) | Full | Zoom/pan/hover | MIT |
| Altair | Yes (Vega-Lite) | Via `vega-embed` | Via node | Good | Limited | BSD |
| Bokeh | Yes | `@bokeh/bokehjs` | Yes | Good | Good | BSD |
| matplotlib | Yes | No | Yes | Manual | Minimal | PSF |
| D3 | No | Yes | Manual | Everything | Total | BSD |
| ECharts | pyecharts | Yes (Baidu) | Yes | Good | Good | Apache 2 |
| Chart.js | No | Yes | Via canvas | Good | Good | MIT |
| Recharts | No | React | Via node | Good | Good | MIT |

## Decision

**Plotly** (`plotly.py` + `plotly.js`) is the single charting standard across dashboards, docs, and reports. Theme tokens are defined once in `crates/hwledger-ui/src/plotly_theme.json` and consumed everywhere.

## Rationale

- Plotly is the only mature lib with first-class parity across Python + JS + static export — critical since our ecosystem spans all three.
- The JSON spec is a natural handoff: Python builds a figure → JSON → TS renders; no rebuild required to port charts.
- Interactivity (zoom/pan/hover) ships in both Streamlit and docs-site with zero effort.
- Altair is elegant but its JSON spec is Vega-Lite, which doubles the learning surface.
- ECharts is excellent but its docs lean Chinese-first; onboarding friction for our team.

## Consequences

- Plotly's bundle is ~3 MB gzipped when used in full. Docs-site uses `plotly.min.js` with a partial bundle (`plotly-basic-dist-min`) to keep pages light.
- Some advanced viz types (Sankey, Parcoords) are in Plotly but slow; we avoid them for runtime charts.
- Recharts/D3 are off the table; contributors will need to learn Plotly.

## Revisit when

- A chart lib demonstrably beats Plotly on bundle size + theming + cross-runtime parity.
- Plotly licensing or maintainership changes.

## References

- Plotly: https://plotly.com/python/
- ADR-0026 (Streamlit).
