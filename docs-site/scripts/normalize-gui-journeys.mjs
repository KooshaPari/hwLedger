#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const docsRoot = path.resolve(process.argv[2] || process.cwd());
const guiRoot = path.join(docsRoot, "public", "gui-journeys");

function copyFile(from, to) {
  fs.mkdirSync(path.dirname(to), { recursive: true });
  fs.copyFileSync(from, to);
}

function stableStringify(value) {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function normalizeJourney(dir) {
  const manifestPath = path.join(dir, "manifest.json");
  if (!fs.existsSync(manifestPath)) return false;
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  const steps = Array.isArray(manifest.steps) ? manifest.steps : [];
  if (!steps.some((step) => step.screenshot_path && fs.existsSync(path.join(dir, step.screenshot_path)))) {
    fs.rmSync(dir, { recursive: true, force: true });
    return true;
  }
  const keyframesDir = path.join(dir, "keyframes");
  fs.rmSync(keyframesDir, { recursive: true, force: true });
  fs.mkdirSync(keyframesDir, { recursive: true });

  let lastScreenshot = null;
  const normalizedSteps = steps.map((step, index) => {
    if (step.screenshot_path) {
      lastScreenshot = step.screenshot_path;
    }
    if (!lastScreenshot) {
      return { ...step, screenshot_path: null };
    }
    const source = path.join(dir, lastScreenshot);
    const normalizedName = `frame_${String(index + 1).padStart(3, "0")}.png`;
    const target = path.join(keyframesDir, normalizedName);
    if (fs.existsSync(source)) {
      copyFile(source, target);
    }
    return {
      ...step,
      screenshot_path: `keyframes/${normalizedName}`,
    };
  });

  const verified = {
    ...manifest,
    steps: normalizedSteps,
    keyframe_count: normalizedSteps.filter((step) => step.screenshot_path).length,
    recording_rich: `${manifest.id}.rich.mp4`,
    media_validation: {
      mode: "local-gui-keyframe-normalization",
      keyframes: normalizedSteps.filter((step) => step.screenshot_path).length,
      rich_required: true,
    },
  };
  fs.writeFileSync(path.join(dir, "manifest.verified.json"), stableStringify(verified));
  return true;
}

function main() {
  if (!fs.existsSync(guiRoot)) {
    console.log(`[normalize-gui] no gui journey root at ${guiRoot}`);
    return;
  }
  let count = 0;
  for (const entry of fs.readdirSync(guiRoot, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    if (normalizeJourney(path.join(guiRoot, entry.name))) count += 1;
  }
  console.log(`[normalize-gui] normalized ${count} GUI journey manifest(s)`);
}

main();
