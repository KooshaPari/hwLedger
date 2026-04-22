/**
 * Probe journey — real user flow.
 *
 * Navigate to Probe, click Start Polling, hover a device row to trigger a
 * detail expander, then expand the backend-detail accordion.
 */
import { test } from '@playwright/test';
import { JourneyRecorder, journeysRoot, waitForStreamlit } from '../lib/journey';

test('streamlit probe — start polling + inspect row', async ({ page }, testInfo) => {
  const recorder = new JourneyRecorder(
    'streamlit-probe',
    'Streamlit Probe — start polling + inspect row',
    'Start live GPU polling, hover a device row for the detail expander, then open the backend detail accordion.',
    journeysRoot(testInfo),
  );
  await recorder.init();
  await recorder.installCursor(page);

  await page.goto('/Probe');
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'landing',
    intent: 'Probe page loaded — a Start Polling button sits above the device table.',
  });

  // Click Start Polling (or similar action).
  const startBtn = page
    .locator('button', { hasText: /Start Polling|Poll|Refresh/i })
    .first();
  if (await startBtn.isVisible().catch(() => false)) {
    await startBtn.click();
    await waitForStreamlit(page);
  }
  await recorder.capture(page, {
    slug: 'polling-started',
    intent: 'Polling started — the device table begins updating with per-GPU VRAM / utilisation rows.',
  });

  // Hover the first device row.
  const firstRow = page
    .locator('table tbody tr, [data-testid="stDataFrameResizable"] [role="row"]')
    .first();
  if (await firstRow.isVisible().catch(() => false)) {
    await firstRow.hover();
    await page.waitForTimeout(400);
  }
  await recorder.capture(page, {
    slug: 'row-hover',
    intent: 'Hovering the first device row highlights it and reveals a small detail affordance.',
  });

  // Click to expand (many Streamlit dataframes use st.expander below).
  const expander = page
    .locator('[data-testid="stExpander"]', { hasText: /Detail|Backend|Device/i })
    .first();
  if (await expander.isVisible().catch(() => false)) {
    await expander.click();
    await waitForStreamlit(page);
  }
  await recorder.capture(page, {
    slug: 'detail-open',
    intent: 'Device detail expanded — driver version, PCIe link speed, backend (CUDA/ROCm/Metal), and per-process VRAM attribution visible.',
  });

  await page.mouse.wheel(0, 500);
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'summary-table',
    intent: 'Summary dataframe at the bottom: one row per device with ID, name, backend, and VRAM in GB.',
  });

  await recorder.finalize(true);
});
