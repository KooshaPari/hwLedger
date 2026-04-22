/**
 * JourneyRich — enriched render driven by an input manifest.
 *
 * Input (via Remotion `inputProps`): { journeyId, manifest, fps, width, height }.
 * Scene layout:
 *   [0, fps*2)                  TitleCard (journey intent)
 *   [fps*2, fps*2 + stepDur*N)  Per-step FrameStill + CalloutBox + CaptionBar
 *   Final fps*1                 Fade-to-black outro (just TitleCard with pass/fail)
 */
import React from "react";
import {
  AbsoluteFill,
  Audio,
  Sequence,
  staticFile,
  useVideoConfig,
} from "remotion";
import { TitleCard } from "../components/TitleCard";
import { CalloutBox } from "../components/CalloutBox";
import { CaptionBar } from "../components/CaptionBar";
import { FrameStill } from "../components/FrameStill";
import { CursorOverlay } from "../components/CursorOverlay";
import { StepBadge } from "../components/StepBadge";
import type { CalloutPosition, RichManifest, SceneSpec } from "../types";

export interface JourneyRichProps {
  journeyId: string;
  manifest: RichManifest;
  /** Base URL (via staticFile prefix) where annotated keyframes live.
   *  e.g. "keyframes/plan-deepseek" -> staticFile("keyframes/plan-deepseek/frame-001.png") */
  keyframeBase: string;
}

const DEFAULT_SCENE_FRAMES = 90; // 3s at 30fps

export const JourneyRich: React.FC<JourneyRichProps> = ({
  journeyId,
  manifest,
  keyframeBase,
}) => {
  const { fps } = useVideoConfig();
  const titleFrames = fps * 2;
  const outroFrames = fps * 1;

  const steps = manifest.steps || [];
  const scenes: SceneSpec[] =
    manifest.scenes && manifest.scenes.length > 0
      ? manifest.scenes
      : steps.map((s, i) => ({
          step: i,
          calloutText: `Step ${i + 1}`,
          calloutSubText: s.intent,
          calloutColor: "#34d399",
          durationFrames: DEFAULT_SCENE_FRAMES,
        }));

  let cursor = titleFrames;
  const sceneFrames = scenes.map((sc) => {
    const d = sc.durationFrames ?? DEFAULT_SCENE_FRAMES;
    const from = cursor;
    cursor += d;
    return { spec: sc, from, duration: d };
  });
  const endOfScenes = cursor;

  const voiceoverBackend = manifest.voiceover?.backend;
  const voiceoverSrc =
    (voiceoverBackend === "piper" ||
      voiceoverBackend === "edge-tts" ||
      voiceoverBackend === "edge") &&
    manifest.voiceover?.audio
      ? staticFile(manifest.voiceover.audio)
      : null;

  return (
    <AbsoluteFill style={{ background: "#000" }}>
      {voiceoverSrc && <Audio src={voiceoverSrc} />}

      <Sequence from={0} durationInFrames={titleFrames}>
        <TitleCard
          title={journeyId}
          subtitle={manifest.intent}
        />
      </Sequence>

      {sceneFrames.map(({ spec, from, duration }, idx) => {
        const step = steps[spec.step];
        if (!step) return null;
        // Use annotated PNG if produced; fall back to raw frame.
        const annotatedName = `frame-${String(spec.step + 1).padStart(3, "0")}.annotated.png`;
        const rawName = step.screenshot_path;
        const annotatedPath = `${keyframeBase}/${annotatedName}`;
        const rawPath = `${keyframeBase}/${rawName}`;
        const hasAnnotated = (manifest.annotated_keyframes || []).includes(annotatedName);
        const src = staticFile(hasAnnotated ? annotatedPath : rawPath);

        // Derive callout position from the first annotation's `position`
        // hint (if any). Varied placement across scenes avoids the "all
        // callouts stuck top-right" bug flagged in the dispersal review.
        const firstAnn = (step.annotations ?? [])[0];
        const position: CalloutPosition =
          (firstAnn?.position as CalloutPosition | undefined) ?? "auto";

        // Scene-local cursor track — filter global track to this window.
        const sceneCursor = (manifest.cursor_track ?? []).filter(
          (p) => p.frame >= from && p.frame < from + duration,
        );

        return (
          <Sequence key={idx} from={from} durationInFrames={duration}>
            <AbsoluteFill>
              <FrameStill src={src} durationFrames={duration} />
              <CalloutBox
                text={spec.calloutText}
                subText={spec.calloutSubText}
                color={spec.calloutColor ?? "#34d399"}
                startFrame={8}
                at={position}
                custom={firstAnn?.custom}
                bbox={firstAnn?.bbox}
              />
              {sceneCursor.length > 0 && (
                <CursorOverlay track={sceneCursor.map((p) => ({
                  ...p,
                  frame: p.frame - from,
                }))} />
              )}
              <StepBadge step={spec.step + 1} total={steps.length} />
              <CaptionBar text={step.intent} />
            </AbsoluteFill>
          </Sequence>
        );
      })}

      <Sequence from={endOfScenes} durationInFrames={outroFrames}>
        <TitleCard
          title={manifest.passed ? "PASS" : "FAIL"}
          subtitle={`${journeyId} · ${manifest.keyframe_count} keyframes`}
        />
      </Sequence>
    </AbsoluteFill>
  );
};

/** Compute total frames for the given manifest — used by Remotion's
 *  `calculateMetadata` so durationInFrames is data-driven. */
export function totalFrames(manifest: RichManifest, fps: number): number {
  const titleFrames = fps * 2;
  const outroFrames = fps * 1;
  const steps = manifest.steps || [];
  const scenes: SceneSpec[] =
    manifest.scenes && manifest.scenes.length > 0
      ? manifest.scenes
      : steps.map((_s, i) => ({
          step: i,
          calloutText: `Step ${i + 1}`,
          durationFrames: DEFAULT_SCENE_FRAMES,
        }));
  const sceneTotal = scenes.reduce(
    (acc, sc) => acc + (sc.durationFrames ?? DEFAULT_SCENE_FRAMES),
    0,
  );
  return titleFrames + sceneTotal + outroFrames;
}
