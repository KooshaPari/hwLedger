/**
 * Single-image scene used when a journey has PNG keyframes but no MP4 source
 * tape (or when we want to slow-zoom over a still).
 */
import React from "react";
import { AbsoluteFill, Img, interpolate, useCurrentFrame } from "remotion";

export const FrameStill: React.FC<{ src: string; durationFrames: number }> = ({
  src,
  durationFrames,
}) => {
  const frame = useCurrentFrame();
  // Subtle Ken-Burns: 1.0 -> 1.04 over the scene.
  const scale = interpolate(frame, [0, durationFrames], [1.0, 1.04], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  return (
    <AbsoluteFill style={{ background: "#000" }}>
      <Img
        src={src}
        style={{
          width: "100%",
          height: "100%",
          objectFit: "contain",
          transform: `scale(${scale})`,
          transformOrigin: "center center",
        }}
      />
    </AbsoluteFill>
  );
};
