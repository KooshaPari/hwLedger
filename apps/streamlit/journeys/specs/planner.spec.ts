/**
 * Planner journey: load a golden fixture, exercise the sequence-length slider,
 * and capture narrative keyframes as the stacked VRAM chart re-renders.
 */
import { test } from '@playwright/test';
import { JourneyRecorder, journeysRoot, waitForStreamlit } from '../lib/journey';

test('streamlit planner — seq length sweep', async ({ page }, testInfo) => {
  const recorder = new JourneyRecorder(
    'streamlit-planner',
    'Streamlit Planner — seq length sweep',
    'Load the DeepSeek-V3 golden fixture, sweep sequence length from 2K to 32K, and watch the KV cache bar dominate the stacked VRAM chart.',
    journeysRoot(testInfo),
  );
  await recorder.init();

  await page.goto('/Planner');
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'landing',
    intent: 'Planner loaded with the default fixture; sidebar exposes seq length, concurrent users, and quantisation controls.',
  });

  // The sidebar may be collapsed by default in newer Streamlit builds.
  const sidebarToggle = page.locator('[data-testid="collapsedControl"]');
  if (await sidebarToggle.isVisible().catch(() => false)) {
    await sidebarToggle.click();
  }

  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'sidebar-open',
    intent: 'Sidebar expanded, showing the sequence length slider at its default 4096 tokens.',
  });

  // Drive the first slider by keyboard: focus then arrow-right to bump seq len.
  const sliders = page.locator('[role="slider"]');
  const firstSlider = sliders.first();
  await firstSlider.scrollIntoViewIfNeeded();
  await firstSlider.focus();
  for (let i = 0; i < 10; i++) {
    await firstSlider.press('ArrowRight');
  }
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'seq-bumped',
    intent: 'Sequence length bumped upward; the stacked VRAM chart re-renders with a taller KV cache band.',
  });

  // Scroll down to reveal the per-layer heatmap + export buttons.
  await page.mouse.wheel(0, 800);
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'layer-heatmap',
    intent: 'Per-layer KV contribution heatmap visible below the main chart; deeper layers contribute more memory.',
  });

  // One more scroll to see the export row.
  await page.mouse.wheel(0, 600);
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'export-row',
    intent: 'Export row in view: vLLM, llama.cpp, and MLX buttons ready to emit deploy-ready configs for the current plan.',
  });

  await recorder.finalize(true);
});
