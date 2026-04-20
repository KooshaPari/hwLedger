/**
 * Journey recording helper for Streamlit Playwright specs.
 *
 * Produces per-journey output shaped to match `apps/cli-journeys/manifests/*`:
 *   recordings/<slug>/frame-NNN.png    (keyframes)
 *   recordings/<slug>/manifest.json     (intent + steps)
 *
 * record-all.sh handles video.webm -> recording.mp4 + recording.gif afterward
 * and copies the manifest into the `manifests/<slug>/` tree.
 */
import { Page, TestInfo } from '@playwright/test';
import fs from 'node:fs/promises';
import path from 'node:path';

export interface JourneyStepInit {
  slug: string;
  intent: string;
}

export interface JourneyManifestStep {
  index: number;
  slug: string;
  intent: string;
  screenshot_path: string;
}

export interface JourneyManifest {
  id: string;
  title: string;
  intent: string;
  recording: string;
  recording_gif: string;
  keyframe_count: number;
  passed: boolean;
  steps: JourneyManifestStep[];
}

export class JourneyRecorder {
  private readonly steps: JourneyManifestStep[] = [];
  private readonly outDir: string;
  private stepIndex = 0;

  constructor(
    public readonly id: string,
    public readonly title: string,
    public readonly intent: string,
    journeysRoot: string,
  ) {
    this.outDir = path.join(journeysRoot, 'recordings', id);
  }

  async init(): Promise<void> {
    await fs.mkdir(this.outDir, { recursive: true });
  }

  /** Capture a numbered keyframe and record its intent. */
  async capture(page: Page, step: JourneyStepInit): Promise<void> {
    this.stepIndex += 1;
    const frameName = `frame-${String(this.stepIndex).padStart(3, '0')}.png`;
    const absPath = path.join(this.outDir, frameName);
    await page.screenshot({ path: absPath, fullPage: false });
    this.steps.push({
      index: this.stepIndex - 1,
      slug: step.slug,
      intent: step.intent,
      screenshot_path: frameName,
    });
  }

  async finalize(passed: boolean): Promise<JourneyManifest> {
    const manifest: JourneyManifest = {
      id: this.id,
      title: this.title,
      intent: this.intent,
      recording: `recordings/${this.id}.mp4`,
      recording_gif: `recordings/${this.id}.gif`,
      keyframe_count: this.steps.length,
      passed,
      steps: this.steps,
    };
    await fs.writeFile(
      path.join(this.outDir, 'manifest.json'),
      JSON.stringify(manifest, null, 2) + '\n',
      'utf8',
    );
    return manifest;
  }
}

/** Resolve journeys root (this file lives in journeys/lib/). */
export function journeysRoot(testInfo: TestInfo): string {
  // testInfo.project.testDir is journeys/specs; go one level up.
  return path.resolve(testInfo.project.testDir, '..');
}

/** Wait for the Streamlit app iframe to become interactive. */
export async function waitForStreamlit(page: Page): Promise<void> {
  // Streamlit's "running" indicator disappears when the server finishes.
  await page.waitForLoadState('networkidle');
  // Settle animations.
  await page.waitForTimeout(500);
}
