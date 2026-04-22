/**
 * ClickPulse — radial ripple emitted at a single frame/location.
 *
 * Used by JourneySlideshow when the cursor "arrives" at an annotation bbox
 * and we want to signal a click, independent of a full `CursorPoint[]`
 * track. Traces to: GUI slideshow fallback, rich-journey-renders.md.
 */
import React from "react";
import { useCurrentFrame } from "remotion";

export interface ClickPulseProps {
  /** Pixel-space [x, y] of the pulse origin. */
  at: [number, number];
  /** Absolute composition frame at which the pulse fires. */
  frame: number;
  /** Hex colour of the ring. Defaults to the canonical hwLedger accent. */
  color?: string;
  /** Initial ring radius in px. Grows with age. */
  radius?: number;
  /** Ring lifetime in frames. */
  lifetime?: number;
}

export const ClickPulse: React.FC<ClickPulseProps> = ({
  at,
  frame: fireFrame,
  color = "#f9e2af",
  radius = 14,
  lifetime = 10,
}) => {
  const now = useCurrentFrame();
  const age = now - fireFrame;
  if (age < 0 || age > lifetime) return null;
  const t = age / lifetime;
  const [x, y] = at;
  return (
    <div
      style={{
        position: "absolute",
        left: x - radius,
        top: y - radius,
        width: radius * 2,
        height: radius * 2,
        borderRadius: "50%",
        border: `3px solid ${color}`,
        opacity: 1 - t,
        transform: `scale(${1 + t * 1.6})`,
        pointerEvents: "none",
        zIndex: 120,
      }}
      aria-hidden
    />
  );
};
