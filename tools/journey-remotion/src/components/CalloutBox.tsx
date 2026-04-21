// Re-export from borrowed dino pattern; see
// phenotype-journeys/remotion/borrowed/dino-components/CalloutBox.tsx
// Inlined here rather than via a workspace path dep because hwLedger's
// docs-site build is self-contained.
import React from "react";
import { useCurrentFrame, useVideoConfig, spring } from "remotion";
import type { CalloutProps } from "../types";

export const CalloutBox: React.FC<CalloutProps> = ({
  text,
  subText,
  color,
  startFrame,
}) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const relativeFrame = Math.max(0, frame - startFrame);
  const scale = spring({
    frame: relativeFrame,
    fps,
    config: { damping: 12, stiffness: 180 },
  });

  return (
    <div
      style={{
        position: "absolute",
        top: 96,
        right: 48,
        transform: `scale(${scale})`,
        transformOrigin: "top right",
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
  );
};
