import { Composition, registerRoot } from "remotion";
import React from "react";
import { JourneyRich, totalFrames, type JourneyRichProps } from "./compositions/JourneyRich";
import type { RichManifest } from "./types";

// Placeholder manifest — real values come in via --props at render time or
// via the Studio when authoring. This shape must satisfy RichManifest.
const PLACEHOLDER_MANIFEST: RichManifest = {
  id: "placeholder",
  intent: "Set --props={...} with a RichManifest to render",
  keyframe_count: 0,
  passed: false,
  steps: [],
  scenes: [],
};

const FPS = 30;
const WIDTH = 1280;
const HEIGHT = 800;

const RemotionRoot: React.FC = () => (
  <>
    <Composition<Record<string, unknown>, JourneyRichProps>
      id="JourneyRich"
      component={JourneyRich as React.ComponentType<Record<string, unknown>>}
      defaultProps={{
        journeyId: "placeholder",
        manifest: PLACEHOLDER_MANIFEST,
        keyframeBase: "keyframes/placeholder",
      }}
      durationInFrames={FPS * 4} // overridden by calculateMetadata
      fps={FPS}
      width={WIDTH}
      height={HEIGHT}
      calculateMetadata={({ props }) => {
        const m = (props as unknown as JourneyRichProps).manifest;
        return { durationInFrames: totalFrames(m, FPS) };
      }}
    />
  </>
);

registerRoot(RemotionRoot);
