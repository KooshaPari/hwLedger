/**
 * Tier 0 structural-capture — Streamlit family unit tests.
 *
 * Uses a fake `PageLike` (no real browser) so the walker + file writer run
 * entirely in-process. Fixture is a static HTML string mimicking Streamlit's
 * planner page (heading + form + result panel).
 *
 * Runner: `bun test apps/streamlit/journeys/tests/aria-snapshot.test.ts`
 */

import { describe, expect, test } from 'bun:test';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {
  captureStructural,
  snapshotPage,
  structuralPathFor,
  type PageLike,
} from '../lib/aria-snapshot';

const FIXTURE_HTML = `<!doctype html>
<html lang="en">
<head><title>hwLedger Planner</title></head>
<body>
  <main>
    <h1>Planner</h1>
    <form>
      <label for="seqlen">Sequence length</label>
      <input id="seqlen" type="number" value="4096" />
      <button type="submit">Run plan</button>
    </form>
    <section aria-label="Memory breakdown">
      <p>Weights: 22.4 GB</p>
    </section>
  </main>
</body>
</html>`;

function fakePage(): PageLike {
  return {
    locator(_selector: string) {
      return {
        async ariaSnapshot(_opts?: { ref?: boolean }) {
          // Synthetic ARIA tree — shape mirrors what Playwright returns
          // (YAML-ish indented). The test asserts on stable tokens.
          return [
            '- main:',
            '  - heading "Planner" [level=1]',
            '  - textbox "Sequence length" [value="4096"]',
            '  - button "Run plan"',
            '  - region "Memory breakdown":',
            '    - paragraph: Weights: 22.4 GB',
          ].join('\n');
        },
      };
    },
    async content() {
      return FIXTURE_HTML;
    },
    url() {
      return 'http://localhost:8501/Planner';
    },
    async title() {
      return 'hwLedger Planner';
    },
    viewportSize() {
      return { width: 1280, height: 800 };
    },
  };
}

describe('aria-snapshot', () => {
  test('snapshotPage captures aria + html + url + title + viewport', async () => {
    const snap = await snapshotPage(fakePage());
    expect(snap.family).toBe('streamlit');
    expect(snap.url).toBe('http://localhost:8501/Planner');
    expect(snap.title).toBe('hwLedger Planner');
    expect(snap.viewport).toEqual({ width: 1280, height: 800 });
    expect(snap.aria).toContain('heading "Planner"');
    expect(snap.aria).toContain('button "Run plan"');
    expect(snap.html).toContain('<h1>Planner</h1>');
    expect(snap.html).toContain('Memory breakdown');
  });

  test('captureStructural writes a parseable sibling JSON next to the frame', async () => {
    const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'aria-snap-'));
    const base = path.join(dir, 'frame-001');
    const outPath = await captureStructural(fakePage(), base);
    expect(outPath).toBe(`${base}.structural.json`);
    const raw = await fs.readFile(outPath, 'utf8');
    const parsed = JSON.parse(raw);
    expect(parsed.family).toBe('streamlit');
    expect(parsed.title).toBe('hwLedger Planner');
    expect(parsed.aria).toContain('textbox "Sequence length"');
    await fs.rm(dir, { recursive: true, force: true });
  });

  test('structuralPathFor swaps image extensions correctly', () => {
    expect(structuralPathFor('/tmp/frame-003.png')).toBe('/tmp/frame-003.structural.json');
    expect(structuralPathFor('/tmp/a/b/kf.jpg')).toBe('/tmp/a/b/kf.structural.json');
    expect(structuralPathFor('/tmp/kf.webp')).toBe('/tmp/kf.structural.json');
  });
});
