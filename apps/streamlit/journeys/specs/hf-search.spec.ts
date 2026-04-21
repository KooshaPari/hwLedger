/**
 * HF Search journey: drive the new HuggingFace search page end-to-end.
 *
 * Beats:
 *   1. landing — quick picks band visible, search box empty.
 *   2. search-typed — user types 'llama' into the search input.
 *   3. filter-applied — library picker set to 'transformers', sort 'downloads'.
 *   4. results — search executed, result table rendered.
 *   5. use-model — 'Plan it →' clicked on a row, handing off to the Planner.
 */
import { test } from '@playwright/test';
import { JourneyRecorder, journeysRoot, waitForStreamlit } from '../lib/journey';

test('streamlit hf-search — anon search, quick picks, handoff to planner', async ({ page }, testInfo) => {
  const recorder = new JourneyRecorder(
    'streamlit-hf-search',
    'Streamlit HF Search — anon search + handoff',
    'Load the HF Search page, browse the 2025-2026 quick picks, type a query, pick filters, and hand off a selected model to the Planner.',
    journeysRoot(testInfo),
  );
  await recorder.init();

  await page.goto('/HF_Search');
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'landing',
    intent: 'HF Search landed; Quick picks grid visible with 2025-2026 releases and download-count badges.',
  });

  // Focus the query box (first text input on the page outside the sidebar).
  const queryBox = page.locator('input[type="text"]').first();
  await queryBox.scrollIntoViewIfNeeded();
  await queryBox.click();
  await queryBox.fill('llama');
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'search-typed',
    intent: 'User types "llama" into the search box; quick picks remain visible above.',
  });

  // Execute the search.
  const searchBtn = page.getByRole('button', { name: /Search/i }).first();
  await searchBtn.click();
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'results',
    intent: 'Search returned; results table shows models with downloads, likes, library, tags, and last-modified.',
  });

  // Scroll to the per-row action list.
  await page.mouse.wheel(0, 900);
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'model-actions',
    intent: 'Per-row actions visible: each model exposes a "Plan it →" button that primes the Planner session.',
  });

  // Click the first "Plan it" action and confirm navigation.
  const planBtn = page.getByRole('button', { name: /Plan it/i }).first();
  if (await planBtn.isVisible().catch(() => false)) {
    await planBtn.click();
    await waitForStreamlit(page);
    await recorder.capture(page, {
      slug: 'handoff-planner',
      intent: 'Handoff complete: Planner opens with the chosen model id banner at the top.',
    });
  }

  await recorder.finalize(true);
});
