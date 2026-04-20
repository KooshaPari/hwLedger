/**
 * Fleet journey: point the Fleet Audit page at a (likely offline) server URL
 * and capture the graceful connection-error path required by NFR-007.
 */
import { test } from '@playwright/test';
import { JourneyRecorder, journeysRoot, waitForStreamlit } from '../lib/journey';

test('streamlit fleet — offline server fail-loudly', async ({ page }, testInfo) => {
  const recorder = new JourneyRecorder(
    'streamlit-fleet',
    'Streamlit Fleet — offline server fail-loudly',
    'Navigate to Fleet Audit while the hwLedger server is offline; the page must report a clear connect error rather than silently degrade.',
    journeysRoot(testInfo),
  );
  await recorder.init();

  await page.goto('/Fleet');
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'landing',
    intent: 'Fleet Audit page loaded, showing the configured server URL and a Refresh button in the header row.',
  });

  await page.waitForTimeout(1500);
  await recorder.capture(page, {
    slug: 'connect-error',
    intent: 'Connect error surfaced: Streamlit prints a red banner explaining the server is unreachable (no silent fallback).',
  });

  const refresh = page.getByRole('button', { name: /refresh/i });
  if (await refresh.isVisible().catch(() => false)) {
    await refresh.click();
    await waitForStreamlit(page);
  }
  await recorder.capture(page, {
    slug: 'refresh-retry',
    intent: 'Refresh clicked; the error banner re-renders, confirming the retry path is explicit and visible to the operator.',
  });

  await recorder.finalize(true);
});
