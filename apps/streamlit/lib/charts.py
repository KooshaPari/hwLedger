"""
Chart builders for hwLedger Streamlit UI.

Provides:
- Stacked bar chart (weights/KV/prefill/runtime)
- Gauge chart (fit status)
"""

import plotly.graph_objects as go
from plotly.subplots import make_subplots


def stacked_bar_chart(plan_result) -> go.Figure:
    """
    Build stacked bar chart showing memory breakdown.

    Args:
        plan_result: PlanResult dataclass

    Returns:
        Plotly figure
    """
    fig = go.Figure()

    categories = ["Memory Breakdown"]
    colors = ["#FF6B6B", "#4ECDC4", "#45B7D1", "#FFA07A"]
    labels = [
        f"Weights: {plan_result.weights_mb:.1f} MB",
        f"KV Cache: {plan_result.kv_mb:.1f} MB",
        f"Prefill: {plan_result.prefill_mb:.1f} MB",
        f"Runtime: {plan_result.runtime_mb:.1f} MB",
    ]
    values = [
        plan_result.weights_mb,
        plan_result.kv_mb,
        plan_result.prefill_mb,
        plan_result.runtime_mb,
    ]

    for i, (label, value, color) in enumerate(zip(labels, values, colors)):
        fig.add_trace(go.Bar(
            y=categories,
            x=[value],
            name=label,
            orientation="h",
            marker=dict(color=color),
            showlegend=True,
        ))

    fig.update_layout(
        title=f"Total: {plan_result.total_gb:.2f} GB | Attention: {plan_result.attention_kind}",
        barmode="stack",
        height=250,
        margin=dict(l=20, r=20, t=40, b=20),
        hovermode="x unified",
        xaxis_title="Memory (MB)",
    )

    return fig


def gauge_chart(utilization_percent: float) -> go.Figure:
    """
    Build gauge chart for fit status.

    Args:
        utilization_percent: Percentage of available memory used (0-100)

    Returns:
        Plotly figure
    """
    fig = go.Figure(go.Indicator(
        mode="gauge+number+delta",
        value=utilization_percent,
        title={"text": "GPU Memory Fit"},
        domain={"x": [0, 1], "y": [0, 1]},
        gauge={
            "axis": {"range": [0, 100]},
            "bar": {"color": "#1f77b4"},
            "steps": [
                {"range": [0, 33], "color": "#d4edda"},
                {"range": [33, 67], "color": "#fff3cd"},
                {"range": [67, 100], "color": "#f8d7da"},
            ],
            "threshold": {
                "line": {"color": "red", "width": 4},
                "thickness": 0.75,
                "value": 100,
            },
        },
    ))

    fig.update_layout(
        height=300,
        margin=dict(l=20, r=20, t=40, b=20),
    )

    return fig


def device_telemetry_chart(samples: list) -> go.Figure:
    """
    Build line chart for device telemetry over time.

    Args:
        samples: List of Telemetry dataclass instances

    Returns:
        Plotly figure
    """
    if not samples:
        fig = go.Figure()
        fig.add_annotation(text="No telemetry data", showarrow=False)
        return fig

    timestamps = [s.captured_at_ms / 1000 for s in samples]
    util = [s.util_percent for s in samples]
    temps = [s.temperature_c for s in samples]
    power = [s.power_watts for s in samples]

    fig = make_subplots(
        rows=1, cols=3,
        subplot_titles=("GPU Utilization %", "Temperature C", "Power W"),
        specs=[[{"secondary_y": False}, {"secondary_y": False}, {"secondary_y": False}]],
    )

    fig.add_trace(go.Scatter(x=timestamps, y=util, name="Util %", mode="lines"), row=1, col=1)
    fig.add_trace(go.Scatter(x=timestamps, y=temps, name="Temp C", mode="lines"), row=1, col=2)
    fig.add_trace(go.Scatter(x=timestamps, y=power, name="Power W", mode="lines"), row=1, col=3)

    fig.update_layout(height=300, showlegend=False, hovermode="x unified")
    return fig
