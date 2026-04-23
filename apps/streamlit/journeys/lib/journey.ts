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
import { captureStructural } from './aria-snapshot';

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
  /** Tier 0 structural-capture sibling (ARIA + HTML + URL + title). */
  structural_path?: string;
  /** Native pixel dimensions of the captured PNG (viewport size). Feeds
   *  Remotion's AnnotationOverlay SVG viewBox — see Problem 1(b). */
  native_width?: number;
  native_height?: number;
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

/**
 * Raw cursor event emitted by the in-page listener (`page.addInitScript`).
 * One line per event in the output JSONL.
 *
 * - `ts_ms`  — monotonic ms from the recorder's start (drawn from
 *              `performance.now()` in-page, rebased to the recorder clock).
 * - `action` — `"move"` for every mousemove sample, `"click"` at the moment
 *              of `mousedown`, `"release"` at `mouseup`.
 *
 * Traces to: `feat/annotations-cursor-visible` — Deliverable 3 JSONL shape.
 */
export interface CursorTrackEvent {
  ts_ms: number;
  x: number;
  y: number;
  action: 'move' | 'click' | 'release';
}

export class JourneyRecorder {
  private readonly steps: JourneyManifestStep[] = [];
  private readonly cursor: CursorSample[] = [];
  /** Raw, per-event cursor log for the JSONL sibling (Deliverable 3). */
  private readonly cursorLog: CursorTrackEvent[] = [];
  private readonly outDir: string;
  private stepIndex = 0;
  private readonly recorderStartMs: number = Date.now();

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
      type Sample = {
        t: number;
        x: number;
        y: number;
        action: 'move' | 'click' | 'release';
      };
      const samples: Sample[] = [];
      (window as unknown as { __hwledgerCursor: Sample[] }).__hwledgerCursor = samples;
      document.addEventListener('mousemove', (e) => {
        dot.style.left = `${e.clientX}px`;
        dot.style.top = `${e.clientY}px`;
        samples.push({ t: performance.now(), x: e.clientX, y: e.clientY, action: 'move' });
      }, { passive: true, capture: true });
      document.addEventListener('mousedown', (e) => {
        const r = document.createElement('div');
        r.className = '__hwl-click';
        r.style.left = `${e.clientX}px`;
        r.style.top = `${e.clientY}px`;
        document.documentElement.appendChild(r);
        setTimeout(() => r.remove(), 600);
        samples.push({ t: performance.now(), x: e.clientX, y: e.clientY, action: 'click' });
      }, { passive: true, capture: true });
      document.addEventListener('mouseup', (e) => {
        samples.push({ t: performance.now(), x: e.clientX, y: e.clientY, action: 'release' });
      }, { passive: true, capture: true });
    });
  }

  /** Capture a numbered keyframe and record its intent. */
  async capture(page: Page, step: JourneyStepInit): Promise<void> {
    this.stepIndex += 1;
    const frameName = `frame-${String(this.stepIndex).padStart(3, '0')}.png`;
    const absPath = path.join(this.outDir, frameName);
    await page.screenshot({ path: absPath, fullPage: false });
    // Drain all cursor events since the last capture. Two sinks:
    //   1. `this.cursor` — one entry per frame (most-recent position) for
    //      the manifest's inlined `cursor_track` (used by Remotion directly
    //      when it doesn't want to load the JSONL sibling).
    //   2. `this.cursorLog` — every raw event for the JSONL sibling.
    type Sample = {
      t: number;
      x: number;
      y: number;
      action: 'move' | 'click' | 'release';
    };
    const drained: Sample[] = await page
      .evaluate(() => {
        type InPageSample = {
          t: number;
          x: number;
          y: number;
          action: 'move' | 'click' | 'release';
        };
        const w = window as unknown as { __hwledgerCursor?: InPageSample[] };
        const arr = w.__hwledgerCursor ?? [];
        const copy = arr.slice();
        // Reset so subsequent captures only see new events.
        w.__hwledgerCursor = [];
        return copy;
      })
      .catch(() => [] as Sample[]);
    const recorderStart = this.recorderStartMs;
    for (const ev of drained) {
      this.cursorLog.push({
        // Rebase to recorder clock: `performance.now()` is epoch-ish-relative
        // in-page; we can't trivially align it, so use monotonic Date.now()
        // at drain time as an approximation (events captured between the
        // previous capture and this one are all attributed to the drain
        // window, which is accurate to <1 frame at 30fps for user-paced UI).
        ts_ms: Math.max(0, Date.now() - recorderStart),
        x: ev.x,
        y: ev.y,
        action: ev.action,
      });
    }
    // Per-frame sample: most-recent position in the drained window, or fall
    // back to the previous capture's last recorded point.
    const recent = drained[drained.length - 1];
    if (recent) {
      this.cursor.push({
        frame: this.stepIndex - 1,
        x: recent.x,
        y: recent.y,
        click: recent.action === 'click',
      });
    }
    // Tier 0 structural-capture: write ARIA tree + raw HTML + url/title
    // sibling next to the PNG. Failure is logged but not fatal — the PNG
    // remains the primary capture (Streamlit's highly-dynamic DOM can trip
    // accessibility-tree extraction for certain custom components).
    let structuralName: string | undefined;
    try {
      const baseNoExt = path.join(this.outDir, frameName.replace(/\.png$/, ''));
      const written = await captureStructural(page, baseNoExt);
      structuralName = path.basename(written);
    } catch (err) {
      // eslint-disable-next-line no-console
      console.warn(`structural-capture failed for ${frameName}: ${String(err)}`);
    }
    // Record the viewport size at capture time so downstream Remotion
    // renders place bboxes in the right coordinate system regardless of
    // the eventual composition canvas size.
    const viewport = page.viewportSize();
    this.steps.push({
      index: this.stepIndex - 1,
      slug: step.slug,
      intent: step.intent,
      screenshot_path: frameName,
      structural_path: structuralName,
      native_width: viewport?.width,
      native_height: viewport?.height,
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
    // Emit cursor-track.jsonl — one event per line, ts_ms relative to the
    // recording's start. This is the Remotion-agnostic cursor source (sibling
    // to `manifest.cursor_track` which is the already-frame-indexed variant).
    // Consumed by downstream tooling (bbox-ground, VLM auditors, Remotion
    // loader when the manifest does not carry an inlined track).
    //
    // Traces to: `feat/annotations-cursor-visible` — Deliverable 3.
    if (this.cursorLog.length > 0) {
      const jsonl = this.cursorLog
        .map((ev) => JSON.stringify(ev))
        .join('\n');
      await fs.writeFile(
        path.join(this.outDir, 'cursor-track.jsonl'),
        jsonl + '\n',
        'utf8',
      );
    }
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
