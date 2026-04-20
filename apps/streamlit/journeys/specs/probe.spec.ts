/**
 * Probe journey: open the device probe page and capture the detected GPU
 * inventory (or a clean empty-state when no devices are present).
 */
import { test } from '@playwright/test';
import { JourneyRecorder, journeysRoot, waitForStreamlit } from '../lib/journey';

test('streamlit probe — device inventory', async ({ page }, testInfo) => {
  const recorder = new JourneyRecorder(
    'streamlit-probe',
    'Streamlit Probe — device inventory',
    'Visit the Probe page to enumerate GPUs detected by the FFI shim and inspect their backend + VRAM.',
    journeysRoot(testInfo),
  );
  await recorder.init();

  await page.goto('/Probe');
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'landing',
    intent: 'Probe page loaded; a banner reports whether any GPUs were detected via the hwledger-ffi shim.',
  });

  // Either the success banner or an FFI-missing warning will be visible.
  await page.waitForTimeout(750);
  await recorder.capture(page, {
    slug: 'device-panel',
    intent: 'Primary device panel expanded (or warning shown when FFI is unavailable), listing backend and VRAM per device.',
  });

  await page.mouse.wheel(0, 500);
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'summary-table',
    intent: 'Summary dataframe at the bottom: one row per device with ID, name, backend, and VRAM in GB.',
  });

  await recorder.finalize(true);
});
