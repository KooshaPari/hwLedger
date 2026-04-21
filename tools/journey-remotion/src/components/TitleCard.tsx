// Borrowed from: dino/scripts/video/src/components/TitleCard.tsx.
import React from "react";
import { AbsoluteFill, Audio, interpolate, useCurrentFrame } from "remotion";

interface TitleCardProps {
  title: string;
  subtitle?: string;
  audioSrc?: string;
}

export const TitleCard: React.FC<TitleCardProps> = ({ title, subtitle, audioSrc }) => {
  const frame = useCurrentFrame();
  const opacity = interpolate(frame, [0, 10, 50, 60], [0, 1, 1, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  return (
    <AbsoluteFill
      style={{
        background: "linear-gradient(135deg, #0a0a0f 0%, #1a1a2e 100%)",
        justifyContent: "center",
        alignItems: "center",
        flexDirection: "column",
        gap: 18,
        opacity,
      }}
    >
      {audioSrc && <Audio src={audioSrc} />}
      <div
        style={{
          color: "white",
          fontSize: 60,
          fontWeight: 800,
          fontFamily: "Inter, system-ui, Arial, sans-serif",
          textAlign: "center",
          letterSpacing: 1.5,
        }}
      >
        {title}
      </div>
      {subtitle && (
        <div
          style={{
            color: "#94a3b8",
            fontSize: 26,
            fontFamily: "Inter, system-ui, Arial, sans-serif",
            textAlign: "center",
            maxWidth: 900,
            padding: "0 40px",
            lineHeight: 1.35,
          }}
        >
          {subtitle}
        </div>
      )}
    </AbsoluteFill>
  );
};
