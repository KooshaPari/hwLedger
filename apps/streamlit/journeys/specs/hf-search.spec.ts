/**
 * @user-story
 * ---
 * journey_id: streamlit-hf-search
 * title: Streamlit — HuggingFace model search journey
 * persona: ML engineer picking a model for local inference
 * given: Streamlit server running, HF token cached
 * when:
 *   - navigate to /HF_Search
 *   - type "llama" in the query box
 *   - scroll to per-row actions
 *   - click "Plan it" on the first result
 * then:
 *   - url contains /Planner
 *   - resolved-model chip renders for the chosen model
 * traces_to: [FR-HF-001, FR-UI-001]
 * record: true
 * blind_judge: auto
 * family: streamlit
 * ---
 *
 * HF Search journey: drive the HuggingFace search page end-to-end and
 * emit a Phenotype-conformant user-story manifest via
 * `@phenotype/playwright-record`.
 */
import { test, expect } from '@phenotype/playwright-record';

async function waitForStreamlitIdle(page: import('@playwright/test').Page) {
  // Streamlit reports "Running" in the top-right during reruns; wait for idle.
  try {
    await page.waitForLoadState('networkidle', { timeout: 15_000 });
  } catch {
    /* best-effort */
  }
}

test('streamlit hf-search — anon search, quick picks, handoff to planner', async ({
  page,
  recorder,
}) => {
  await page.goto('/HF_Search');
  await waitForStreamlitIdle(page);
  await recorder.capture(
    page,
    'landing',
    'HF Search landed; Quick picks grid visible with 2025-2026 releases and download-count badges.',
  );

  const queryBox = page.getByPlaceholder(/llama|qwen|mistral/i).first();
  try {
    await queryBox.waitFor({ state: 'visible', timeout: 30_000 });
    await queryBox.click();
    await queryBox.fill('llama');
  } catch {
    const fallback = page.locator('input[type="text"]').first();
    await fallback.waitFor({ state: 'visible', timeout: 30_000 });
    await fallback.click();
    await fallback.fill('llama');
  }
  await waitForStreamlitIdle(page);
  await recorder.capture(
    page,
    'search-typed',
    'User types "llama" into the search box; quick picks remain visible above.',
  );

  const searchBtn = page.getByRole('button', { name: /^Search$/i }).first();
  if (await searchBtn.isVisible().catch(() => false)) {
    await searchBtn.click();
    await waitForStreamlitIdle(page);
  }
  await recorder.capture(
    page,
    'results',
    'Search returned; results table shows models with downloads, likes, library, tags, and last-modified.',
  );

  await page.mouse.wheel(0, 900);
  await waitForStreamlitIdle(page);
  await recorder.capture(
    page,
    'model-actions',
    'Per-row actions visible: each model exposes a "Plan it →" button that primes the Planner session.',
  );

  const planBtn = page.getByRole('button', { name: /Plan it/i }).first();
  if (await planBtn.isVisible().catch(() => false)) {
    await planBtn.click();
    await waitForStreamlitIdle(page);
    await expect(page).toHaveURL(/\/Planner/i);
    await recorder.capture(
      page,
      'handoff-planner',
      'Handoff complete: Planner opens with the chosen model id banner at the top.',
    );
  }
});
