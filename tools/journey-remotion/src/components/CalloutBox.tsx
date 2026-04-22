// Callout component with position variants.
//
// Inspired by phenotype-journeys/remotion/borrowed/dino-components/CalloutBox.tsx
// but extended with an `at` prop so callouts can be placed at any of nine
// anchors (top-left/right, bottom-left/right, center, center-top/bottom,
// custom pixel coords, or `auto` — which defers to the caller's bbox anchor).
import React from "react";
import { useCurrentFrame, useVideoConfig, spring } from "remotion";
import type { CalloutProps } from "../types";

type Placement = {
  top?: number | string;
  right?: number | string;
  bottom?: number | string;
  left?: number | string;
  transformOrigin: string;
};

function placementFor(
  at: CalloutProps["at"] | undefined,
  custom?: CalloutProps["custom"],
): Placement {
  switch (at ?? "top-right") {
    case "top-left":
      return { top: 96, left: 48, transformOrigin: "top left" };
    case "top-right":
    case "auto":
      return { top: 96, right: 48, transformOrigin: "top right" };
    case "bottom-left":
      return { bottom: 96, left: 48, transformOrigin: "bottom left" };
    case "bottom-right":
      return { bottom: 96, right: 48, transformOrigin: "bottom right" };
    case "center":
      return {
        top: "50%",
        left: "50%",
        transformOrigin: "center",
      };
    case "center-top":
      return { top: 48, left: "50%", transformOrigin: "top center" };
    case "center-bottom":
      return { bottom: 48, left: "50%", transformOrigin: "bottom center" };
    case "custom": {
      const { x = 96, y = 96 } = custom ?? {};
      return { top: y, left: x, transformOrigin: "top left" };
    }
  }
}

export const CalloutBox: React.FC<CalloutProps> = ({
  text,
  subText,
  color,
  startFrame,
  at,
  custom,
  bbox,
  endFrame,
}) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const relativeFrame = Math.max(0, frame - startFrame);
  const scale = spring({
    frame: relativeFrame,
    fps,
    config: { damping: 12, stiffness: 180 },
  });

  if (endFrame !== undefined && frame > endFrame) return null;

  const place = placementFor(at, custom);
  const isCenter = at === "center";
  const centerTranslate = isCenter ? "translate(-50%, -50%)" : "";
  const centerTopBottomTranslate =
    at === "center-top" || at === "center-bottom" ? "translateX(-50%)" : "";

  return (
    <>
      {bbox && (
        <div
          style={{
            position: "absolute",
            left: bbox[0],
            top: bbox[1],
            width: bbox[2],
            height: bbox[3],
            border: `3px solid ${color}`,
            borderRadius: 6,
            boxShadow: `0 0 0 4px ${color}33`,
            opacity: scale,
            pointerEvents: "none",
          }}
          aria-hidden
        />
      )}
      <div
        style={{
          position: "absolute",
          ...place,
          transform: `${centerTranslate} ${centerTopBottomTranslate} scale(${scale})`.trim(),
          background: "rgba(10,10,15,0.82)",
          border: `2px solid ${color}`,
          borderRadius: 10,
          padding: "14px 22px",
          minWidth: 300,
          maxWidth: 460,
          boxShadow: "0 10px 30px rgba(0,0,0,0.5)",
        }}
      >
        <div
          style={{
            color,
            fontSize: 26,
            fontWeight: 700,
            fontFamily: "Inter, system-ui, Arial, sans-serif",
            letterSpacing: 0.3,
          }}
        >
          {text}
        </div>
        {subText && (
          <div
            style={{
              color: "white",
              fontSize: 15,
              fontFamily: "Inter, system-ui, Arial, sans-serif",
              marginTop: 6,
              opacity: 0.9,
              lineHeight: 1.4,
            }}
          >
            {subText}
          </div>
        )}
      </div>
    </>
  );
};
