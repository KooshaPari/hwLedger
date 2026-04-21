/**
 * What-If journey: compare a baseline plan to a candidate plan under a set
 * of transformation techniques, read the verdict, and inspect the citations
 * that back each technique's memory multipliers.
 *
 * Beats:
 *   1. landing — page loaded, baseline defaulting to "Use latest Planner result".
 *   2. manual-baseline — switch to "Enter manually" to show the numeric inputs.
 *   3. pick-techniques — multi-select opened with INT4 + KV-FP8 chosen.
 *   4. side-by-side — Plotly bars rendered with baseline vs candidate.
 *   5. verdict — verdict banner and citations table visible after scrolling.
 */
import { test } from '@playwright/test';
import { JourneyRecorder, journeysRoot, waitForStreamlit } from '../lib/journey';

test('streamlit what-if — technique sweep + citations', async ({ page }, testInfo) => {
  const recorder = new JourneyRecorder(
    'streamlit-what-if',
    'Streamlit What-If — technique sweep',
    'Open the What-If page, set a manual baseline, pick INT4 + KV-FP8, and inspect the side-by-side bars, verdict, and citations table.',
    journeysRoot(testInfo),
  );
  await recorder.init();

  await page.goto('/WhatIf');
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'landing',
    intent: 'What-If page loaded; baseline defaults to latest Planner result with a manual-entry fallback.',
  });

  // Flip the baseline source to "Enter manually" for determinism.
  const manual = page.getByText(/Enter manually/i).first();
  if (await manual.isVisible().catch(() => false)) {
    await manual.click();
  }
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'manual-baseline',
    intent: 'Switched to manual baseline; four numeric inputs expose weights/KV/prefill/runtime MB.',
  });

  // Open the multi-select and accept the default INT4+KV-FP8.
  const multiselect = page.locator('[data-baseweb="select"]').first();
  if (await multiselect.isVisible().catch(() => false)) {
    await multiselect.click();
    await page.keyboard.press('Escape');
  }
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'pick-techniques',
    intent: 'Technique picker opened: INT4, INT8, KV-FP8, KV-INT4, LoRA, REAP, SpecDecode, FlashAttn3 available.',
  });

  // Scroll to side-by-side bars.
  await page.mouse.wheel(0, 500);
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'side-by-side',
    intent: 'Plotly grouped bar chart shows baseline vs candidate per band (Weights, KV, Prefill, Runtime).',
  });

  // Scroll further for the verdict + citations table.
  await page.mouse.wheel(0, 600);
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'verdict-citations',
    intent: 'Verdict banner summarises the delta; citations table lists arXiv papers backing each technique.',
  });

  await recorder.finalize(true);
});
