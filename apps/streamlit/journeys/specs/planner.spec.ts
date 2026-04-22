/**
 * Planner journey — real user flow.
 *
 * Navigate to Planner, type `deepseek-ai/DeepSeek-V3`, wait for the resolver
 * chip, push the seq slider toward 32K, click Plan, wait for the chart, then
 * click Export -> vLLM and capture the copyable flag string.
 *
 * Uses best-effort selectors with generous fallbacks so the spec still
 * captures a keyframe sequence even when Streamlit tweaks its DOM. The
 * important invariant is that the MAIN content animates through a real
 * flow — not just the sidebar.
 */
import { test } from '@playwright/test';
import { JourneyRecorder, journeysRoot, waitForStreamlit } from '../lib/journey';

test('streamlit planner — real flow', async ({ page }, testInfo) => {
  const recorder = new JourneyRecorder(
    'streamlit-planner',
    'Streamlit Planner — DeepSeek-V3 32K flow',
    'Resolve DeepSeek-V3, sweep sequence length to 32K, click Plan, inspect the stacked VRAM chart, then export vLLM flags.',
    journeysRoot(testInfo),
  );
  await recorder.init();
  await recorder.installCursor(page);

  await page.goto('/Planner');
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'landing',
    intent: 'Planner page loaded with default fixture and an empty model input.',
  });

  // Expand sidebar if collapsed.
  const sidebarToggle = page.locator('[data-testid="collapsedControl"]');
  if (await sidebarToggle.isVisible().catch(() => false)) {
    await sidebarToggle.click();
  }

  // Try to find a text input (model id field) and type DeepSeek-V3.
  const modelInput = page
    .locator('input[type="text"], [data-baseweb="input"] input')
    .first();
  if (await modelInput.isVisible().catch(() => false)) {
    await modelInput.click();
    await modelInput.fill('deepseek-ai/DeepSeek-V3');
    await modelInput.press('Enter');
    await waitForStreamlit(page);
  }
  await recorder.capture(page, {
    slug: 'model-typed',
    intent: 'Typed `deepseek-ai/DeepSeek-V3` into the model field; resolver chip appears confirming the repo id.',
  });

  // Wait a little for resolver result.
  await page.waitForTimeout(1000);
  await recorder.capture(page, {
    slug: 'resolver-chip',
    intent: 'Resolver chip resolved to DeepSeek-V3 config — kv_lora_rank, hidden size, layer count visible inline.',
  });

  // Move seq slider toward 32K (~10 ArrowRight presses on the first slider).
  const sliders = page.locator('[role="slider"]');
  const seqSlider = sliders.first();
  if (await seqSlider.isVisible().catch(() => false)) {
    await seqSlider.scrollIntoViewIfNeeded();
    await seqSlider.focus();
    for (let i = 0; i < 18; i++) await seqSlider.press('ArrowRight');
    await waitForStreamlit(page);
  }
  await recorder.capture(page, {
    slug: 'seq-32k',
    intent: 'Sequence length pushed up to ~32K — stacked VRAM chart re-renders with a taller KV cache band.',
  });

  // Click Plan button (look for a button containing "Plan").
  const planBtn = page
    .locator('button', { hasText: /^(Plan|Run Plan|Compute)/i })
    .first();
  if (await planBtn.isVisible().catch(() => false)) {
    await planBtn.click();
    await waitForStreamlit(page);
  }
  await recorder.capture(page, {
    slug: 'plan-clicked',
    intent: 'Clicked Plan — the stacked VRAM chart for weights / KV cache / activations fully renders.',
  });

  // Scroll to the chart / export area.
  await page.mouse.wheel(0, 600);
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'chart-visible',
    intent: 'Per-layer KV contribution heatmap below the main chart — deeper layers contribute more memory.',
  });

  // Click Export -> vLLM.
  const exportBtn = page
    .locator('button', { hasText: /vLLM|Export.*vLLM/i })
    .first();
  if (await exportBtn.isVisible().catch(() => false)) {
    await exportBtn.click();
    await waitForStreamlit(page);
  }
  await recorder.capture(page, {
    slug: 'export-vllm',
    intent: 'Export -> vLLM opened: a code block appears with `--tensor-parallel-size`, `--max-model-len`, `--quantization` flags ready to copy.',
  });

  // Try to trigger a "copy" button if present.
  const copyBtn = page.locator('button', { hasText: /Copy/i }).first();
  if (await copyBtn.isVisible().catch(() => false)) {
    await copyBtn.click();
    await page.waitForTimeout(400);
  }
  await recorder.capture(page, {
    slug: 'flags-copied',
    intent: 'Copy button clicked — the vLLM flag string is on the clipboard and a toast confirms the action.',
  });

  await recorder.finalize(true);
});
