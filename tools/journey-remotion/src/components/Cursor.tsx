/**
 * Cursor — a visible cursor dot that interpolates along a polyline path
 * between two frames. Used by JourneySlideshow to fake the flow from a
 * step's description toward the first annotation bbox when no real
 * recorder-emitted `CursorPoint[]` track exists.
 *
 * Traces to: rich-journey-renders.md — "cursor visibility" requirement.
 */
import React from "react";
import { interpolate, useCurrentFrame } from "remotion";

export interface CursorProps {
  /** Absolute composition frame the cursor enters at. */
  from: number;
  /** Absolute composition frame the cursor arrives at its final waypoint. */
  to: number;
  /** Polyline waypoints in pixel space; first is start, last is end. */
  path: Array<[number, number]>;
  /** Dot radius in px. */
  radius?: number;
  /** Hex colour. */
  color?: string;
}

export const Cursor: React.FC<CursorProps> = ({
  from,
  to,
  path,
  radius = 10,
  color = "#f9e2af",
}) => {
  const now = useCurrentFrame();
  if (now < from || path.length === 0) return null;
  const clamped = Math.min(Math.max(now, from), to);
  // Parametric position along the polyline. t in [0,1], segment index =
  // floor(t * (N-1)), local s = fractional part of the segment.
  const n = path.length;
  if (n === 1) {
    const [x, y] = path[0];
    return <Dot x={x} y={y} radius={radius} color={color} />;
  }
  const t = interpolate(clamped, [from, to], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const scaled = t * (n - 1);
  const i = Math.min(Math.floor(scaled), n - 2);
  const s = scaled - i;
  const [x0, y0] = path[i];
  const [x1, y1] = path[i + 1];
  const x = x0 + (x1 - x0) * s;
  const y = y0 + (y1 - y0) * s;
  return <Dot x={x} y={y} radius={radius} color={color} />;
};

const Dot: React.FC<{ x: number; y: number; radius: number; color: string }> = ({
  x,
  y,
  radius,
  color,
}) => (
  <div
    style={{
      position: "absolute",
      left: x - radius / 2,
      top: y - radius / 2,
      width: radius,
      height: radius,
      borderRadius: "50%",
      background: color,
      boxShadow: `0 0 0 2px rgba(0,0,0,0.35), 0 0 8px ${color}`,
      pointerEvents: "none",
      zIndex: 110,
    }}
    aria-hidden
  />
);
