import { Composition, registerRoot } from "remotion";
import React from "react";
import { JourneyRich, totalFrames, type JourneyRichProps } from "./compositions/JourneyRich";
import {
  JourneySlideshow,
  slideshowTotalFrames,
  type JourneySlideshowProps,
} from "./compositions/JourneySlideshow";
import {
  JourneyCli,
  cliTotalFrames,
  type JourneyCliProps,
} from "./compositions/JourneyCli";
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
    <Composition<Record<string, unknown>, JourneyCliProps>
      id="JourneyCli"
      component={JourneyCli as React.ComponentType<Record<string, unknown>>}
      defaultProps={{
        journeyId: "placeholder",
        manifest: PLACEHOLDER_MANIFEST,
        keyframeBase: "keyframes/placeholder",
      }}
      durationInFrames={FPS * 4}
      fps={FPS}
      width={WIDTH}
      height={HEIGHT}
      calculateMetadata={({ props }) => {
        const m = (props as unknown as JourneyCliProps).manifest;
        return { durationInFrames: cliTotalFrames(m, FPS) };
      }}
    />
    <Composition<Record<string, unknown>, JourneySlideshowProps>
      id="JourneySlideshow"
      component={JourneySlideshow as React.ComponentType<Record<string, unknown>>}
      defaultProps={{
        journeyId: "placeholder",
        manifest: PLACEHOLDER_MANIFEST,
        keyframeBase: "keyframes/placeholder",
      }}
      durationInFrames={FPS * 4}
      fps={FPS}
      width={1440}
      height={900}
      calculateMetadata={({ props }) => {
        const m = (props as unknown as JourneySlideshowProps).manifest;
        return { durationInFrames: slideshowTotalFrames(m, FPS) };
      }}
    />
  </>
);

registerRoot(RemotionRoot);
