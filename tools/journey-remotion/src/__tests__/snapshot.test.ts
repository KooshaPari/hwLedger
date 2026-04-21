/**
 * Minimum Remotion snapshot: the JourneyRich composition renders when fed a
 * trivial manifest with zero steps (title + outro only). Run via:
 *   bun test
 * We don't exercise the full render here (that requires Chromium); we only
 * assert the composition types + totalFrames calculation stay stable.
 */
import { describe, expect, test } from "bun:test";
import { totalFrames } from "../compositions/JourneyRich";
import type { RichManifest } from "../types";

describe("JourneyRich.totalFrames", () => {
  test("zero-step manifest: title(60) + outro(30) = 90 frames at 30fps", () => {
    const m: RichManifest = {
      id: "empty",
      intent: "empty",
      keyframe_count: 0,
      passed: true,
      steps: [],
    };
    expect(totalFrames(m, 30)).toBe(90);
  });

  test("three-step default scenes: title(60) + 3*90 + outro(30) = 360", () => {
    const m: RichManifest = {
      id: "three",
      intent: "three",
      keyframe_count: 3,
      passed: true,
      steps: [
        { index: 0, slug: "a", intent: "a", screenshot_path: "a.png" },
        { index: 1, slug: "b", intent: "b", screenshot_path: "b.png" },
        { index: 2, slug: "c", intent: "c", screenshot_path: "c.png" },
      ],
    };
    expect(totalFrames(m, 30)).toBe(360);
  });

  test("explicit scene durations override defaults", () => {
    const m: RichManifest = {
      id: "custom",
      intent: "custom",
      keyframe_count: 2,
      passed: true,
      steps: [
        { index: 0, slug: "a", intent: "a", screenshot_path: "a.png" },
        { index: 1, slug: "b", intent: "b", screenshot_path: "b.png" },
      ],
      scenes: [
        { step: 0, calloutText: "A", durationFrames: 120 },
        { step: 1, calloutText: "B", durationFrames: 60 },
      ],
    };
    // title(60) + 120 + 60 + outro(30) = 270
    expect(totalFrames(m, 30)).toBe(270);
  });
});
