// Borrowed from: dino/scripts/video/src/components/CaptionBar.tsx (text prop added).
// Extended with optional `at` anchor + `from`/`to` frame-window props so CLI
// renders can place the caption explicitly at the center-bottom of each step
// and gate visibility to the step's frame window.
import React from "react";
import { useCurrentFrame } from "remotion";

export type CaptionAnchor = "center-bottom" | "top-right" | "bottom-left";

export interface CaptionBarProps {
  text: string;
  /** Anchor for the caption. Defaults to `center-bottom` (spans full width). */
  at?: CaptionAnchor;
  /** If set, hide the caption until this absolute composition frame. */
  from?: number;
  /** If set, hide the caption after this absolute composition frame. */
  to?: number;
}

export const CaptionBar: React.FC<CaptionBarProps> = ({
  text,
  at = "center-bottom",
  from,
  to,
}) => {
  const frame = useCurrentFrame();
  if (from !== undefined && frame < from) return null;
  if (to !== undefined && frame > to) return null;

  const style: React.CSSProperties = {
    position: "absolute",
    background: "rgba(0,0,0,0.82)",
    padding: "12px 28px",
    fontFamily: "Inter, system-ui, Arial, sans-serif",
    fontSize: 18,
    color: "white",
    letterSpacing: 0.2,
    textAlign: "center",
    boxShadow: "0 -2px 12px rgba(0,0,0,0.55)",
  };

  if (at === "center-bottom") {
    Object.assign(style, { bottom: 0, left: 0, right: 0 });
  } else if (at === "top-right") {
    Object.assign(style, { top: 16, right: 16, maxWidth: 560, borderRadius: 8 });
  } else {
    // bottom-left
    Object.assign(style, { bottom: 16, left: 16, maxWidth: 560, borderRadius: 8 });
  }

  return <div style={style}>{text}</div>;
};
