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

export interface CursorSample {
  /** Frame index (derived from screenshot order; 0-based). */
  frame: number;
  x: number;
  y: number;
  click?: boolean;
}

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
  private readonly cursor: CursorSample[] = [];
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

  /**
   * Inject a DOM cursor that tracks `mousemove` / `mousedown` and stores
   * samples on `window.__hwledgerCursor`. Playwright's headless browser
   * does NOT render the OS cursor in video, so we render one in the page
   * itself; the accompanying `cursor_track` JSON is then consumed by the
   * Remotion CursorOverlay for the rich render.
   */
  async installCursor(page: Page): Promise<void> {
    await page.addInitScript(() => {
      const style = document.createElement('style');
      style.textContent = `
        .__hwl-cursor {
          position: fixed;
          width: 18px;
          height: 18px;
          border-radius: 50%;
          background: rgba(249, 226, 175, 0.92);
          box-shadow: 0 0 0 2px rgba(0,0,0,0.4), 0 0 10px rgba(249,226,175,0.8);
          pointer-events: none;
          transform: translate(-50%, -50%);
          z-index: 2147483647;
          transition: transform 60ms linear;
        }
        .__hwl-click {
          position: fixed;
          width: 22px; height: 22px; border-radius: 50%;
          border: 2px solid #f9e2af; pointer-events: none;
          transform: translate(-50%, -50%) scale(1);
          animation: __hwl-ripple 520ms ease-out forwards;
          z-index: 2147483646;
        }
        @keyframes __hwl-ripple {
          to { transform: translate(-50%, -50%) scale(3.2); opacity: 0; }
        }
      `;
      document.documentElement.appendChild(style);
      const dot = document.createElement('div');
      dot.className = '__hwl-cursor';
      dot.style.left = '-100px';
      dot.style.top = '-100px';
      document.documentElement.appendChild(dot);
      const samples: Array<{ t: number; x: number; y: number; click?: boolean }> = [];
      (window as unknown as { __hwledgerCursor: typeof samples }).__hwledgerCursor = samples;
      document.addEventListener('mousemove', (e) => {
        dot.style.left = `${e.clientX}px`;
        dot.style.top = `${e.clientY}px`;
        samples.push({ t: performance.now(), x: e.clientX, y: e.clientY });
      }, { passive: true, capture: true });
      document.addEventListener('mousedown', (e) => {
        const r = document.createElement('div');
        r.className = '__hwl-click';
        r.style.left = `${e.clientX}px`;
        r.style.top = `${e.clientY}px`;
        document.documentElement.appendChild(r);
        setTimeout(() => r.remove(), 600);
        samples.push({ t: performance.now(), x: e.clientX, y: e.clientY, click: true });
      }, { passive: true, capture: true });
    });
  }

  /** Capture a numbered keyframe and record its intent. */
  async capture(page: Page, step: JourneyStepInit): Promise<void> {
    this.stepIndex += 1;
    const frameName = `frame-${String(this.stepIndex).padStart(3, '0')}.png`;
    const absPath = path.join(this.outDir, frameName);
    await page.screenshot({ path: absPath, fullPage: false });
    // Snapshot cursor position at capture time; frame index = stepIndex - 1.
    type Sample = { t: number; x: number; y: number; click?: boolean };
    const recent: Sample | null = await page
      .evaluate(() => {
        const w = window as unknown as { __hwledgerCursor?: Sample[] };
        const arr = w.__hwledgerCursor ?? [];
        return arr.length > 0 ? arr[arr.length - 1] : null;
      })
      .catch(() => null);
    if (recent) {
      this.cursor.push({
        frame: this.stepIndex - 1,
        x: recent.x,
        y: recent.y,
        click: recent.click,
      });
    }
    this.steps.push({
      index: this.stepIndex - 1,
      slug: step.slug,
      intent: step.intent,
      screenshot_path: frameName,
    });
  }

  async finalize(passed: boolean): Promise<JourneyManifest> {
    const manifest: JourneyManifest & { cursor_track?: CursorSample[] } = {
      id: this.id,
      title: this.title,
      intent: this.intent,
      recording: `recordings/${this.id}.mp4`,
      recording_gif: `recordings/${this.id}.gif`,
      keyframe_count: this.steps.length,
      passed,
      steps: this.steps,
    };
    if (this.cursor.length > 0) manifest.cursor_track = this.cursor;
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
