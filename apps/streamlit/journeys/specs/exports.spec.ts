/**
 * Exports journey: drive the Planner page's export row to produce
 * vLLM / llama.cpp / MLX deploy configs for the active plan.
 */
import { test } from '@playwright/test';
import { JourneyRecorder, journeysRoot, waitForStreamlit } from '../lib/journey';

test('streamlit exports — vLLM/llama.cpp/MLX', async ({ page }, testInfo) => {
  const recorder = new JourneyRecorder(
    'streamlit-exports',
    'Streamlit Exports — vLLM, llama.cpp, MLX',
    'From the Planner page, click each export button in turn and capture the generated deployment config for vLLM, llama.cpp, and MLX.',
    journeysRoot(testInfo),
  );
  await recorder.init();
  await recorder.installCursor(page);

  await page.goto('/Planner');
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'planner-ready',
    intent: 'Planner page ready with a concrete plan already computed; we scroll down to the Export Configuration row.',
  });

  // Scroll to the bottom where the export row lives.
  await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
  await waitForStreamlit(page);
  await recorder.capture(page, {
    slug: 'export-row',
    intent: 'Export Configuration row in view: three buttons — Export as vLLM, Export as llama.cpp, Export as MLX.',
  });

  const clickIfPresent = async (name: RegExp, slug: string, intent: string) => {
    const btn = page.getByRole('button', { name });
    if (await btn.isVisible().catch(() => false)) {
      await btn.click();
      await waitForStreamlit(page);
    }
    await recorder.capture(page, { slug, intent });
    // Click the "Copy" icon button the st.code block renders (if any).
    const copy = page.locator('button[title="Copy"], button', { hasText: /^Copy$/ }).first();
    if (await copy.isVisible().catch(() => false)) {
      await copy.click();
      await page.waitForTimeout(300);
      await recorder.capture(page, {
        slug: `${slug}-copied`,
        intent: `${slug} flag string copied to clipboard — a small toast confirms the action.`,
      });
    }
  };

  await clickIfPresent(
    /export as vllm/i,
    'vllm-config',
    'vLLM click: JSON payload with --model, --max-model-len, --max-num-seqs rendered in a code block.',
  );

  await clickIfPresent(
    /export as llama\.cpp/i,
    'llama-config',
    'llama.cpp click: CLI arg string (-m, -c, -ngl) emitted for the same plan parameters.',
  );

  await clickIfPresent(
    /export as mlx/i,
    'mlx-config',
    'MLX click: Apple Silicon deploy config serialised as JSON, completing the export triple.',
  );

  await recorder.finalize(true);
});
