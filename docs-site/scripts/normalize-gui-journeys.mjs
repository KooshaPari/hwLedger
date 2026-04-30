#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import crypto from "node:crypto";

const docsRoot = path.resolve(process.argv[2] || process.cwd());
const guiRoot = path.join(docsRoot, "public", "gui-journeys");

function copyFile(from, to) {
  fs.mkdirSync(path.dirname(to), { recursive: true });
  fs.copyFileSync(from, to);
}

function stableStringify(value) {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function sha256File(file) {
  return crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex");
}

function sha256Text(text) {
  return crypto.createHash("sha256").update(text).digest("hex");
}

function normalizeJourney(dir) {
  const manifestPath = path.join(dir, "manifest.json");
  if (!fs.existsSync(manifestPath)) return false;
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  const previousVerifiedPath = path.join(dir, "manifest.verified.json");
  const previousVerified = fs.existsSync(previousVerifiedPath)
    ? JSON.parse(fs.readFileSync(previousVerifiedPath, "utf8"))
    : {};
  const steps = Array.isArray(manifest.steps) ? manifest.steps : [];
  const fallbackScreenshots = fs
    .readdirSync(dir)
    .filter((entry) => entry.endsWith(".png"))
    .sort();
  if (
    !steps.some((step) => step.screenshot_path && fs.existsSync(path.join(dir, step.screenshot_path))) &&
    fallbackScreenshots.length === 0
  ) {
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
    } else if (!lastScreenshot && fallbackScreenshots[index]) {
      lastScreenshot = fallbackScreenshots[index];
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

  const richFile = path.join(dir, `${manifest.id}.rich.mp4`);
  const verified = {
    ...manifest,
    steps: normalizedSteps,
    keyframe_count: normalizedSteps.filter((step) => step.screenshot_path).length,
    recording_audio_voiceover: previousVerified.recording_audio_voiceover,
    recording_rich: previousVerified.recording_rich || `${manifest.id}.rich.mp4`,
    recording_rich_manifest_sha256: previousVerified.recording_rich_manifest_sha256,
    recording_rich_sha256: previousVerified.recording_rich_sha256,
    voiceover_sha256: previousVerified.voiceover_sha256,
    media_validation: {
      mode: "local-gui-keyframe-normalization",
      keyframes: normalizedSteps.filter((step) => step.screenshot_path).length,
      rich_required: true,
    },
  };
  if (fs.existsSync(richFile) && fs.statSync(richFile).size > 0) {
    verified.recording_rich_sha256 = sha256File(richFile);
    verified.recording_rich_manifest_sha256 = sha256Text(
      stableStringify({
        ...verified,
        recording_rich_sha256: undefined,
        recording_rich_manifest_sha256: undefined,
      }),
    );
  }
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
