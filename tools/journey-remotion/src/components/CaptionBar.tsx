// Borrowed from: dino/scripts/video/src/components/CaptionBar.tsx (text prop added).
import React from "react";

export const CaptionBar: React.FC<{ text: string }> = ({ text }) => (
  <div
    style={{
      position: "absolute",
      bottom: 0,
      left: 0,
      right: 0,
      background: "rgba(0,0,0,0.78)",
      padding: "10px 24px",
      textAlign: "center",
      fontFamily: "Inter, system-ui, Arial, sans-serif",
      fontSize: 16,
      color: "white",
      letterSpacing: 0.2,
    }}
  >
    {text}
  </div>
);
