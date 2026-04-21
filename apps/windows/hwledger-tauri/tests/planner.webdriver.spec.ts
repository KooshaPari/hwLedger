/**
 * Planner webdriver integration test.
 *
 * Run against a release build driven by `tauri-driver` + WebDriverIO. This
 * spec is shipped as a skeleton — wire in CI once a Windows runner is
 * available. See `../scripts/webdriver.md`.
 *
 * Traces to: FR-PLAN-003, FR-UI-001.
 */

// @ts-expect-error — wdio types aren't in devDependencies until CI is wired.
import { remote } from "webdriverio";

async function driver() {
  return remote({
    hostname: "127.0.0.1",
    port: 4444,
    capabilities: {
      "tauri:options": { application: process.env.TAURI_BIN ?? "" },
    } as unknown as WebdriverIO.Capabilities,
  });
}

describe("Planner", () => {
  it("renders a breakdown after Plan memory", async () => {
    const d = await driver();
    try {
      await d.$("#tab-planner").click();
      await d.$('button.primary').click();
      const total = await d.$(".stat-value").getText();
      if (!/[0-9]/.test(total)) {
        throw new Error(`expected numeric total, got: ${total}`);
      }
    } finally {
      await d.deleteSession();
    }
  });
});
