/**
 * CursorOverlay — renders a visible cursor dot + click-pulse ripple synced to
 * a `CursorPoint[]` track emitted by the Playwright / GUI recorder.
 *
 * Traces to: rich-journey-renders.md — "cursor visibility" requirement.
 *
 * Implementation notes:
 * - Playwright natively does NOT render the mouse cursor in headless video.
 *   We inject a DOM cursor in the spec via `page.addInitScript()` (see
 *   `apps/streamlit/journeys/lib/journey.ts`), which produces a cursor track
 *   JSON alongside each recording. This component post-renders that track on
 *   top of the composed keyframe in Remotion so the cursor is always visible
 *   regardless of whether the source recording had one.
 * - Click points emit a radial ripple (6 frames, spring-scale 1 -> 2.4).
 */
import React from "react";
import { useCurrentFrame } from "remotion";
import type { CursorOverlayProps } from "../types";

export const CursorOverlay: React.FC<CursorOverlayProps> = ({
  track,
  radius = 10,
  color = "#f9e2af",
}) => {
  const frame = useCurrentFrame();
  if (!track || track.length === 0) return null;

  // Find the closest cursor sample at or before the current frame.
  const sample =
    track.filter((p) => p.frame <= frame).at(-1) ?? track[0];
  if (!sample) return null;

  // Any click within the last 6 frames triggers a fading ripple.
  const recentClick = track.find(
    (p) => p.click && p.frame <= frame && frame - p.frame < 6,
  );
  const rippleAge = recentClick ? frame - recentClick.frame : null;

  return (
    <>
      {rippleAge !== null && recentClick && (
        <div
          style={{
            position: "absolute",
            left: recentClick.x - radius,
            top: recentClick.y - radius,
            width: radius * 2,
            height: radius * 2,
            borderRadius: "50%",
            border: `2px solid ${color}`,
            opacity: 1 - rippleAge / 6,
            transform: `scale(${1 + rippleAge * 0.25})`,
            pointerEvents: "none",
          }}
          aria-hidden
        />
      )}
      <div
        style={{
          position: "absolute",
          left: sample.x - radius / 2,
          top: sample.y - radius / 2,
          width: radius,
          height: radius,
          borderRadius: "50%",
          background: color,
          boxShadow: `0 0 0 2px rgba(0,0,0,0.35), 0 0 8px ${color}`,
          pointerEvents: "none",
          zIndex: 100,
        }}
        aria-hidden
      />
    </>
  );
};
