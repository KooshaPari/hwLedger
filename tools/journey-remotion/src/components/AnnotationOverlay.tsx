/**
 * AnnotationOverlay — renders every `Annotation.bbox` from a step as an SVG
 * rect inside a `viewBox` keyed to the *native* image dimensions
 * (`native_width` × `native_height`).
 *
 * Why SVG + viewBox?
 * ---
 * The keyframes are captured at the native screen resolution (Streamlit:
 * 1440×900, XCUITest: 1920×1200, CLI: terminal cells × font metrics) but
 * the Remotion composition renders at 1280×720 / 1280×800 / 1440×900.
 * `<FrameStill>` uses `objectFit: contain`, which letterboxes the image.
 *
 * Previous implementations placed bbox overlays with absolute CSS `left/top`
 * in *native* pixels against a *composition-sized* viewport, producing
 * overlays drifted off the image. This component sidesteps the mismatch by:
 *
 *   1. Wrapping the SVG in the same `objectFit: contain` letterbox envelope
 *      so its computed size matches the image's on-screen footprint.
 *   2. Declaring `viewBox="0 0 native_width native_height"` so bbox
 *      coordinates stay in image-pixel space — the browser scales them.
 *   3. `preserveAspectRatio="xMidYMid meet"` matches `objectFit: contain`.
 *
 * Fade behaviour
 * ---
 * Opacity ramps 0 → 1 over the first `fadeIn` frames, holds through the
 * scene, then ramps 1 → 0 over the last `fadeOut` frames. Previous
 * implementations (spring-scale-as-opacity) bottomed at 0 and often never
 * visibly ramped because the spring's early derivative was tiny at low
 * stiffness — this is an explicit piecewise interpolate.
 *
 * Traces to: `feat/annotations-cursor-visible` — Problem 1(a,b,c).
 */
import React from "react";
import { interpolate, useCurrentFrame } from "remotion";
import type { Annotation } from "../types";

export interface AnnotationOverlayProps {
  annotations: Annotation[];
  /** Scene duration in frames — used for the fade-out timing. */
  durationFrames: number;
  /**
   * Native pixel dimensions of the underlying screenshot. Defaults to the
   * historical hwLedger capture canvas (1440×900) when omitted — this
   * keeps legacy manifests rendering approximately correctly.
   */
  nativeWidth?: number;
  nativeHeight?: number;
  /** Frames to fade in / out over. */
  fadeIn?: number;
  fadeOut?: number;
}

const DEFAULT_NATIVE_W = 1440;
const DEFAULT_NATIVE_H = 900;

export const AnnotationOverlay: React.FC<AnnotationOverlayProps> = ({
  annotations,
  durationFrames,
  nativeWidth = DEFAULT_NATIVE_W,
  nativeHeight = DEFAULT_NATIVE_H,
  fadeIn = 10,
  fadeOut = 10,
}) => {
  const frame = useCurrentFrame();
  if (!annotations || annotations.length === 0) return null;

  // Piecewise opacity: ramp in, hold, ramp out. Never starts at 0 and stays
  // there — the "invisible overlay" bug was rooted in using spring-as-opacity
  // whose early values round-tripped to ~0.
  const fadeInEnd = Math.min(fadeIn, durationFrames);
  const fadeOutStart = Math.max(fadeInEnd, durationFrames - fadeOut);
  const opacity = interpolate(
    frame,
    [0, fadeInEnd, fadeOutStart, durationFrames],
    [0, 1, 1, 0],
    { extrapolateLeft: "clamp", extrapolateRight: "clamp" },
  );

  return (
    <svg
      // Fill the composition; `preserveAspectRatio="xMidYMid meet"` matches
      // `<FrameStill>`'s `objectFit: contain`, so the SVG's inner coordinate
      // system letterboxes identically to the image underneath.
      viewBox={`0 0 ${nativeWidth} ${nativeHeight}`}
      preserveAspectRatio="xMidYMid meet"
      style={{
        position: "absolute",
        inset: 0,
        width: "100%",
        height: "100%",
        pointerEvents: "none",
        opacity,
      }}
      aria-hidden
    >
      {annotations.map((ann, i) => {
        const [x, y, w, h] = ann.bbox;
        const color = ann.color ?? "#34d399";
        const dash = ann.style === "dashed" ? "10 6" : undefined;
        return (
          <g key={i}>
            {/* Soft shadow halo — fills a 4-native-pixel border. */}
            <rect
              x={x - 4}
              y={y - 4}
              width={w + 8}
              height={h + 8}
              rx={8}
              ry={8}
              fill="none"
              stroke={color}
              strokeOpacity={0.25}
              strokeWidth={10}
            />
            <rect
              x={x}
              y={y}
              width={w}
              height={h}
              rx={4}
              ry={4}
              fill="none"
              stroke={color}
              strokeWidth={4}
              strokeDasharray={dash}
            />
            {ann.label && (
              <text
                x={x}
                y={Math.max(18, y - 8)}
                fill={color}
                fontFamily="Inter, system-ui, sans-serif"
                fontSize={20}
                fontWeight={700}
                stroke="#000"
                strokeWidth={0.5}
                paintOrder="stroke"
              >
                {ann.label}
              </text>
            )}
          </g>
        );
      })}
    </svg>
  );
};
