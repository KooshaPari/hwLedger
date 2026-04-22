/**
 * JourneyCli — polished CLI terminal composition.
 *
 * Why a dedicated composition?
 *   Terminal recordings have no per-button bboxes, so CursorOverlay,
 *   ClickPulse, and CalloutBox-on-bbox don't apply. What CLI journeys *do*
 *   benefit from is:
 *     - TitleCard intro (1.5s) naming the journey
 *     - Per-step CaptionBar at center-bottom showing `step.intent`
 *     - Per-step StepBadge at top-left showing `Step N/M`
 *     - Voiceover audio spanning the whole composition
 *     - TitleCard outro (2s) showing PASS/FAIL + keyframe count
 *
 * Timeline:
 *   [0, intro)                         TitleCard (journey intent)
 *   [intro, intro + Σ stepDur)         Per-step FrameStill + StepBadge + CaptionBar
 *   [..., ... + outro)                 Outro TitleCard (PASS/FAIL)
 *
 * Voiceover:
 *   `manifest.voiceover.audio` is mounted once as an <Audio> track when the
 *   backend is piper / edge-tts / edge. The Rust driver concatenates one
 *   utterance per step into a single WAV so timing roughly aligns with the
 *   step durations (which are synthesised from `target_content_seconds`).
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
import { CaptionBar } from "../components/CaptionBar";
import { FrameStill } from "../components/FrameStill";
import { StepBadge } from "../components/StepBadge";
import type { RichManifest, SceneSpec } from "../types";

export interface JourneyCliProps {
  journeyId: string;
  manifest: RichManifest;
  /** Base URL (via staticFile prefix) where keyframe PNGs live. */
  keyframeBase: string;
}

const INTRO_SECONDS = 1.5;
const OUTRO_SECONDS = 2.0;
const DEFAULT_STEP_FRAMES = 90; // 3s @ 30fps — only used when no scene spec

export const JourneyCli: React.FC<JourneyCliProps> = ({
  journeyId,
  manifest,
  keyframeBase,
}) => {
  const { fps } = useVideoConfig();
  const introFrames = Math.round(fps * INTRO_SECONDS);
  const outroFrames = Math.round(fps * OUTRO_SECONDS);

  const steps = manifest.steps || [];
  const scenes: SceneSpec[] =
    manifest.scenes && manifest.scenes.length > 0
      ? manifest.scenes
      : steps.map((s, i) => ({
          step: i,
          calloutText: `Step ${i + 1}`,
          calloutSubText: s.intent,
          durationFrames: DEFAULT_STEP_FRAMES,
        }));

  let cursor = introFrames;
  const sceneFrames = scenes.map((sc) => {
    const d = sc.durationFrames ?? DEFAULT_STEP_FRAMES;
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

      <Sequence from={0} durationInFrames={introFrames}>
        <TitleCard title={journeyId} subtitle={manifest.intent} />
      </Sequence>

      {sceneFrames.map(({ spec, from, duration }, idx) => {
        const step = steps[spec.step];
        if (!step) return null;
        // CLI keyframes are raw terminal screenshots — no annotated variant
        // is produced, so always load the raw frame.
        const rawName = step.screenshot_path;
        const src = staticFile(`${keyframeBase}/${rawName}`);
        // CaptionBar visibility is gated by this <Sequence>; inside the
        // sequence the <CaptionBar> sees a relative frame starting at 0, so
        // we don't pass `from`/`to` (which are absolute-composition frames).
        return (
          <Sequence key={idx} from={from} durationInFrames={duration}>
            <AbsoluteFill>
              <FrameStill src={src} durationFrames={duration} />
              <StepBadge step={spec.step + 1} total={steps.length} />
              <CaptionBar at="center-bottom" text={step.intent} />
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

/** Compute total frames — used by Remotion's `calculateMetadata`. */
export function cliTotalFrames(manifest: RichManifest, fps: number): number {
  const introFrames = Math.round(fps * INTRO_SECONDS);
  const outroFrames = Math.round(fps * OUTRO_SECONDS);
  const steps = manifest.steps || [];
  const scenes: SceneSpec[] =
    manifest.scenes && manifest.scenes.length > 0
      ? manifest.scenes
      : steps.map((_s, i) => ({
          step: i,
          calloutText: `Step ${i + 1}`,
          durationFrames: DEFAULT_STEP_FRAMES,
        }));
  const sceneTotal = scenes.reduce(
    (acc, sc) => acc + (sc.durationFrames ?? DEFAULT_STEP_FRAMES),
    0,
  );
  return introFrames + sceneTotal + outroFrames;
}
