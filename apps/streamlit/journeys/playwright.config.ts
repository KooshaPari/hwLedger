import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright config for Streamlit journey recording.
 *
 * Captures real pixels via Chromium headed mode. Each spec writes its own
 * keyframe PNGs + manifest.json under `recordings/<slug>/`. The record-all.sh
 * wrapper is responsible for booting Streamlit, waiting for /healthz, and
 * converting Playwright's video.webm into recording.mp4 + recording.gif.
 */
export default defineConfig({
  testDir: './specs',
  timeout: 120_000,
  expect: { timeout: 15_000 },
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [['list']],
  outputDir: './playwright-output',
  use: {
    baseURL: process.env.STREAMLIT_URL ?? 'http://127.0.0.1:8599',
    viewport: { width: 1280, height: 800 },
    video: 'on',
    trace: 'on',
    screenshot: 'off',
    actionTimeout: 15_000,
    navigationTimeout: 30_000,
  },
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        // Headed so we capture real pixels (Streamlit uses canvas/webgl for plots).
        headless: process.env.HEADLESS === '1',
        launchOptions: {
          args: ['--no-sandbox', '--disable-dev-shm-usage'],
        },
      },
    },
  ],
});
