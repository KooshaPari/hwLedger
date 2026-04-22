/**
 * JourneySlideshow — keyframe-driven slideshow composition for GUI journeys
 * where the raw MP4 recording is missing or < 3s (e.g. TCC-blocked XCUITest
 * capture on the user's Mac).
 *
 * Per step:
 *   - Ken-Burns slow zoom over the step's `screenshot_path` keyframe.
 *   - Cross-fade with the previous step over ~300ms.
 *   - Caption bar overlaying `step.intent`.
 *   - CalloutBox for each `step.annotations[]` entry, positioned by its
 *     `position` hint (top-right, center-bottom, bottom-left, ...).
 *   - Animated cursor that travels from a soft start to the first
 *     annotation bbox mid-step, with a click-pulse on arrival.
 *
 * Timeline:
 *   [0, intro)                     TitleCard (journey intent)
 *   [intro, intro+step*N+overlap)  N × step slides (cross-faded)
 *   [... , ... + outro)            Outro TitleCard (PASS/FAIL + step count)
 *
 * Voiceover: `manifest.voiceover.audio`, when present, is mounted as a
 * single `<Audio>` track spanning the whole composition — Piper already
 * concatenates one utterance per step into a single WAV (see
 * `synthesise_voiceover_piper` in `hwledger-journey-render`).
 */
import React from "react";
import {
  AbsoluteFill,
  Audio,
  interpolate,
  Sequence,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";
import { CalloutBox } from "../components/CalloutBox";
import { CaptionBar } from "../components/CaptionBar";
import { ClickPulse } from "../components/ClickPulse";
import { Cursor } from "../components/Cursor";
import { TitleCard } from "../components/TitleCard";
import type {
  Annotation,
  CalloutPosition,
  JourneyStep,
  RichManifest,
} from "../types";

export interface JourneySlideshowProps {
  journeyId: string;
  manifest: RichManifest;
  keyframeBase: string;
}

const INTRO_SECONDS = 1.5;
const OUTRO_SECONDS = 2.0;
const STEP_SECONDS = 5.0;
const CROSSFADE_FRAMES = 9; // ~300ms @ 30fps

function pickPosition(bbox: [number, number, number, number]): CalloutPosition {
  const [x, y, w, h] = bbox;
  const cx = x + w / 2;
  const cy = y + h / 2;
  // 1440x900 frame heuristic:
  const leftHalf = cx < 720;
  const topHalf = cy < 450;
  if (topHalf && !leftHalf) return "bottom-left";
  if (topHalf && leftHalf) return "bottom-right";
  if (!topHalf && leftHalf) return "top-right";
  return "top-left";
}

function bboxCenter(bbox: [number, number, number, number]): [number, number] {
  return [bbox[0] + bbox[2] / 2, bbox[1] + bbox[3] / 2];
}

/**
 * KenBurnsStill — slow zoom-and-pan over a still keyframe. Unlike FrameStill
 * the pan offset biases toward any annotation bbox so the camera drifts
 * toward the action rather than the image centre.
 */
const KenBurnsStill: React.FC<{
  src: string;
  durationFrames: number;
  focus?: [number, number];
  canvas: { w: number; h: number };
}> = ({ src, durationFrames, focus, canvas }) => {
  const frame = useCurrentFrame();
  const scale = interpolate(frame, [0, durationFrames], [1.0, 1.06], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  // Pan: 0 -> drift 2% of canvas toward focus point.
  const [fx, fy] = focus ?? [canvas.w / 2, canvas.h / 2];
  const dx = ((fx - canvas.w / 2) / canvas.w) * 2; // -1..1 -> -2%..+2%
  const dy = ((fy - canvas.h / 2) / canvas.h) * 2;
  const tx = interpolate(frame, [0, durationFrames], [0, -dx * 20], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const ty = interpolate(frame, [0, durationFrames], [0, -dy * 20], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  return (
    <AbsoluteFill style={{ background: "#000" }}>
      <img
        src={src}
        style={{
          width: "100%",
          height: "100%",
          objectFit: "cover",
          transform: `translate(${tx}px, ${ty}px) scale(${scale})`,
          transformOrigin: "center center",
        }}
        alt=""
      />
    </AbsoluteFill>
  );
};

/**
 * StepSlide — a single step rendered into its containing <Sequence>. Handles
 * its own cross-fade (fades in for the first CROSSFADE_FRAMES frames).
 */
const StepSlide: React.FC<{
  step: JourneyStep;
  keyframeBase: string;
  durationFrames: number;
  isFirst: boolean;
  canvas: { w: number; h: number };
}> = ({ step, keyframeBase, durationFrames, isFirst, canvas }) => {
  const frame = useCurrentFrame();
  const fade = isFirst
    ? 1
    : interpolate(frame, [0, CROSSFADE_FRAMES], [0, 1], {
        extrapolateLeft: "clamp",
        extrapolateRight: "clamp",
      });

  const annotations: Annotation[] = step.annotations ?? [];
  const primary = annotations[0];
  const focus = primary ? bboxCenter(primary.bbox) : undefined;

  // Cursor path: start somewhere neutral, arrive at primary bbox centre
  // ~60% through the slide. Click-pulse fires at arrival.
  const cursorFrom = Math.round(durationFrames * 0.15);
  const cursorTo = Math.round(durationFrames * 0.6);
  const startPt: [number, number] = [canvas.w * 0.15, canvas.h * 0.85];
  const endPt: [number, number] = focus ?? [canvas.w * 0.5, canvas.h * 0.5];

  const rawName = step.screenshot_path;
  const src = rawName ? staticFile(`${keyframeBase}/${rawName.split("/").pop()}`) : null;

  return (
    <AbsoluteFill style={{ opacity: fade }}>
      {src && (
        <KenBurnsStill
          src={src}
          durationFrames={durationFrames}
          focus={focus}
          canvas={canvas}
        />
      )}
      {annotations.map((ann, i) => {
        const pos: CalloutPosition =
          ann.position && ann.position !== "auto"
            ? ann.position
            : pickPosition(ann.bbox);
        return (
          <CalloutBox
            key={i}
            text={ann.label}
            subText={ann.note ?? undefined}
            color={ann.color ?? "#34d399"}
            startFrame={Math.max(8, cursorFrom - 4)}
            at={pos}
            bbox={ann.bbox}
          />
        );
      })}
      {primary && (
        <>
          <Cursor
            from={cursorFrom}
            to={cursorTo}
            path={[startPt, endPt]}
            color="#f9e2af"
          />
          <ClickPulse at={endPt} frame={cursorTo} color="#f9e2af" />
        </>
      )}
      <CaptionBar text={step.intent} />
    </AbsoluteFill>
  );
};

export const JourneySlideshow: React.FC<JourneySlideshowProps> = ({
  journeyId,
  manifest,
  keyframeBase,
}) => {
  const { fps, width, height } = useVideoConfig();
  const introFrames = Math.round(fps * INTRO_SECONDS);
  const outroFrames = Math.round(fps * OUTRO_SECONDS);
  const stepFrames = Math.round(fps * STEP_SECONDS);
  const steps = (manifest.steps || []).filter((s) => !!s.screenshot_path);

  const voiceoverSrc =
    manifest.voiceover?.backend === "piper" && manifest.voiceover?.audio
      ? staticFile(manifest.voiceover.audio)
      : null;

  // Layout: intro, then steps with CROSSFADE_FRAMES of overlap between
  // consecutive slides so transitions visibly cross-fade.
  const scenes = steps.map((step, i) => {
    const from = introFrames + i * (stepFrames - CROSSFADE_FRAMES);
    return { step, from, duration: stepFrames };
  });
  const lastScene = scenes[scenes.length - 1];
  const endOfScenes = lastScene ? lastScene.from + lastScene.duration : introFrames;

  return (
    <AbsoluteFill style={{ background: "#000" }}>
      {voiceoverSrc && <Audio src={voiceoverSrc} />}

      <Sequence from={0} durationInFrames={introFrames}>
        <TitleCard title={journeyId} subtitle={manifest.intent} />
      </Sequence>

      {scenes.map(({ step, from, duration }, idx) => (
        <Sequence key={idx} from={from} durationInFrames={duration}>
          <StepSlide
            step={step}
            keyframeBase={keyframeBase}
            durationFrames={duration}
            isFirst={idx === 0}
            canvas={{ w: width, h: height }}
          />
        </Sequence>
      ))}

      <Sequence from={endOfScenes} durationInFrames={outroFrames}>
        <TitleCard
          title={manifest.passed ? "PASS" : "FAIL"}
          subtitle={`${journeyId} · ${steps.length} steps`}
        />
      </Sequence>
    </AbsoluteFill>
  );
};

/** Total-frames calculator used by Remotion's `calculateMetadata`. */
export function slideshowTotalFrames(
  manifest: RichManifest,
  fps: number,
): number {
  const introFrames = Math.round(fps * INTRO_SECONDS);
  const outroFrames = Math.round(fps * OUTRO_SECONDS);
  const stepFrames = Math.round(fps * STEP_SECONDS);
  const steps = (manifest.steps || []).filter((s) => !!s.screenshot_path);
  const n = steps.length;
  if (n === 0) return introFrames + outroFrames;
  // N slides, each `stepFrames` long, with (N-1) overlaps of CROSSFADE_FRAMES.
  const stepsTotal = n * stepFrames - (n - 1) * CROSSFADE_FRAMES;
  return introFrames + stepsTotal + outroFrames;
}
