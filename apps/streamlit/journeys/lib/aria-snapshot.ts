/**
 * Tier 0 structural-capture (Streamlit family).
 *
 * Produces a sibling `<frame-id>.structural.json` next to every Playwright
 * screenshot. The payload captures three complementary views of the page:
 *
 *   - `aria`      — Playwright's `ariaSnapshot({ref: true})` accessibility
 *                   tree (YAML-ish string; the canonical a11y view).
 *   - `html`      — raw `page.content()` HTML; useful when aria falls
 *                   short on non-semantic Streamlit widgets.
 *   - `url`       — `page.url()` at snapshot time.
 *   - `title`     — `page.title()` at snapshot time.
 *   - `viewport`  — page.viewportSize() or null.
 *
 * Usage from a spec:
 *
 *   import { captureStructural } from '../lib/aria-snapshot';
 *   await captureStructural(page, '/out/journey/frame_003');
 *
 * When wired through `JourneyRecorder.capture(...)`, the helper is called
 * automatically at every screenshot.
 *
 * Traces to: Tier 0 structural-capture (Streamlit family).
 */

import type { Page } from '@playwright/test';
import fs from 'node:fs/promises';
import path from 'node:path';

export interface StructuralSnapshot {
  family: 'streamlit';
  url: string;
  title: string;
  viewport: { width: number; height: number } | null;
  aria: string;
  html: string;
}

/**
 * Minimal page shape we need. Permissive to stay forward-compatible with
 * Playwright's evolving `ariaSnapshot` options bag (1.49→1.59 changed the
 * option names; `ref` was the 1.49 form, later releases added `mode`/`depth`).
 */
export interface PageLike {
  locator(selector: string): {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    ariaSnapshot(options?: any): Promise<string>;
  };
  content(): Promise<string>;
  url(): string;
  title(): Promise<string>;
  viewportSize(): { width: number; height: number } | null;
}

/**
 * Build a structural snapshot from the live page. Keeps the return value
 * as a structured JS object so callers can inspect it inline; caller is
 * responsible for persistence.
 */
export async function snapshotPage(page: PageLike): Promise<StructuralSnapshot> {
  // `ref: true` in Playwright 1.49; current releases accept an object but
  // silently ignore unknown keys — we stay permissive here so either version
  // works without per-release branching.
  const aria = await page.locator('body').ariaSnapshot({ ref: true });
  const [html, title] = await Promise.all([page.content(), page.title()]);
  return {
    family: 'streamlit',
    url: page.url(),
    title,
    viewport: page.viewportSize(),
    aria,
    html,
  };
}

/**
 * Capture + write the structural snapshot alongside a keyframe.
 *
 * `baseNoExt` is the keyframe path *without* extension, e.g.
 * `/out/recordings/planner/frame_003`. The sibling is written to
 * `<baseNoExt>.structural.json`.
 *
 * Returns the absolute path that was written.
 */
export async function captureStructural(
  page: PageLike,
  baseNoExt: string,
): Promise<string> {
  const snap = await snapshotPage(page);
  const outPath = `${baseNoExt}.structural.json`;
  await fs.mkdir(path.dirname(outPath), { recursive: true });
  await fs.writeFile(outPath, JSON.stringify(snap, null, 2), 'utf8');
  return outPath;
}

/**
 * Derive the structural sibling path from a screenshot path by swapping the
 * extension (`.png`/`.jpg`/`.jpeg` → `.structural.json`).
 */
export function structuralPathFor(screenshotPath: string): string {
  return screenshotPath.replace(/\.(png|jpe?g|webp)$/i, '.structural.json');
}
