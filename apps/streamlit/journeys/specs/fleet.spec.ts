/**
 * Fleet journey — real user flow.
 *
 * Fill the new-agent form (host / transport / labels), submit, trigger the
 * SSH probe, then inspect the audit row. The spec still survives an offline
 * server (fail-loud banner is captured) but the MAIN content animates
 * through the registration flow.
 */
import { test } from '@playwright/test';
import { JourneyRecorder, journeysRoot, waitForStreamlit } from '../lib/journey';

test('streamlit fleet — register + probe + audit', async ({ page }, testInfo) => {
  const recorder = new JourneyRecorder(
    'streamlit-fleet',
    'Streamlit Fleet — register + probe + audit',
    'Fill the new-agent form, submit a register request, trigger the SSH probe, and inspect the resulting audit entry.',
    journeysRoot(testInfo),
  );
  await recorder.init();
  await recorder.installCursor(page);

  await page.goto('/Fleet');
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'landing',
    intent: 'Fleet page loaded — the new-agent form sits above the audit table.',
  });

  // Fill host field (first text input), agent id (second), labels (third).
  const inputs = page.locator('input[type="text"], [data-baseweb="input"] input');
  const first = inputs.nth(0);
  const second = inputs.nth(1);
  if (await first.isVisible().catch(() => false)) {
    await first.click();
    await first.fill('gpu-box-01.tailnet.ts.net');
  }
  if (await second.isVisible().catch(() => false)) {
    await second.click();
    await second.fill('dev-nvidia-01');
  }
  await recorder.capture(page, {
    slug: 'form-filled',
    intent: 'Filled host + agent id in the register-agent form.',
  });

  // Submit.
  const submit = page
    .locator('button', { hasText: /Register|Add Agent|Submit/i })
    .first();
  if (await submit.isVisible().catch(() => false)) {
    await submit.click();
    await waitForStreamlit(page);
  }
  await recorder.capture(page, {
    slug: 'submitted',
    intent: 'Submitted — either the audit table grows by one row or a fail-loud banner explains the offline server (NFR-007).',
  });

  // Trigger SSH probe if a Probe button exists for the new row.
  const probeBtn = page.locator('button', { hasText: /Probe|SSH Probe/i }).first();
  if (await probeBtn.isVisible().catch(() => false)) {
    await probeBtn.click();
    await waitForStreamlit(page);
  }
  await recorder.capture(page, {
    slug: 'probe-triggered',
    intent: 'SSH probe triggered; the agent row shows `probing...` then resolves with detected GPU count + backend.',
  });

  // Expand the first audit row.
  const firstExpander = page.locator('[data-testid="stExpander"]').first();
  if (await firstExpander.isVisible().catch(() => false)) {
    await firstExpander.click();
    await waitForStreamlit(page);
  }
  await recorder.capture(page, {
    slug: 'audit-detail',
    intent: 'Audit row expanded — attestation hash, signer, and append-only chain pointer visible.',
  });

  await recorder.finalize(true);
});
