/**
 * StepBadge — top-left "Step N/M" marker for CLI (and other) journeys where
 * there's no per-button bbox to target a CalloutBox at. Pairs with the
 * bottom CaptionBar to provide "rich editing" without a cursor/annotation
 * track.
 */
import React from "react";

export const StepBadge: React.FC<{ step: number; total: number; label?: string }> = ({
  step,
  total,
  label,
}) => (
  <div
    style={{
      position: "absolute",
      top: 16,
      left: 16,
      display: "inline-flex",
      alignItems: "center",
      gap: 8,
      background: "rgba(15, 23, 42, 0.85)",
      border: "1px solid rgba(148, 163, 184, 0.5)",
      borderRadius: 8,
      padding: "6px 12px",
      fontFamily:
        "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
      fontSize: 13,
      color: "#e2e8f0",
      letterSpacing: 0.3,
      boxShadow: "0 2px 8px rgba(0,0,0,0.45)",
    }}
  >
    <span style={{ color: "#34d399", fontWeight: 700 }}>●</span>
    <span style={{ fontWeight: 600 }}>
      Step {step}/{total}
    </span>
    {label ? (
      <span style={{ color: "#94a3b8", fontWeight: 400 }}>· {label}</span>
    ) : null}
  </div>
);
