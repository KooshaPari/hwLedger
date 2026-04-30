#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const docsRoot = process.cwd();
const publicRoot = path.join(docsRoot, "public");
const failures = [];
const warnings = [];
const referencedPublicAssets = new Set();

function existsNonEmpty(file) {
  try {
    return fs.statSync(file).size > 0;
  } catch {
    return false;
  }
}

function walk(dir, predicate = () => true) {
  const out = [];
  if (!fs.existsSync(dir)) return out;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...walk(full, predicate));
    else if (predicate(full)) out.push(full);
  }
  return out;
}

function rel(file) {
  return path.relative(docsRoot, file);
}

function publicAsset(assetPath) {
  const clean = assetPath.split(/[?#]/, 1)[0];
  if (!clean.startsWith("/")) return null;
  return path.join(publicRoot, clean.slice(1));
}

function fail(message) {
  failures.push(message);
}

function warn(message) {
  warnings.push(message);
}

function checkMarkdownReferences() {
  const markdown = walk(docsRoot, (file) => {
    if (!file.endsWith(".md")) return false;
    const parts = file.split(path.sep);
    return !parts.includes("node_modules") && !parts.includes(".vitepress");
  });
  const assetPattern =
    /(?:src|manifest)=["']([^"']+)["']|["'](\/(?:cli|streamlit|gui)-journeys\/[^"']+\.(?:png|gif|mp4|json))["']/g;
  let refs = 0;
  for (const file of markdown) {
    const text = fs.readFileSync(file, "utf8");
    for (const match of text.matchAll(assetPattern)) {
      const asset = match[1] || match[2];
      if (!asset) continue;
      const target = publicAsset(asset);
      if (!target) continue;
      referencedPublicAssets.add(path.relative(publicRoot, target).split(path.sep).join("/"));
      refs += 1;
      if (!existsNonEmpty(target)) {
        fail(`${rel(file)} references missing or empty public asset ${asset}`);
      }
    }
  }
  return refs;
}

function readJson(file) {
  try {
    return JSON.parse(fs.readFileSync(file, "utf8"));
  } catch (error) {
    fail(`${rel(file)} is not valid JSON: ${error.message}`);
    return null;
  }
}

function pngCount(dir) {
  return walk(dir, (file) => file.endsWith(".png")).length;
}

function manifestTargets(family, id, manifestFile) {
  const publicRel = path.relative(publicRoot, manifestFile).split(path.sep);
  if (family === "cli") {
    return {
      keyframesDir: path.join(publicRoot, "cli-journeys", "keyframes", id),
      rich: path.join(publicRoot, "cli-journeys", "recordings", id, `${id}.rich.mp4`),
      baseForScreenshots: path.join(publicRoot, "cli-journeys", "keyframes", id),
    };
  }
  if (family === "streamlit") {
    return {
      keyframesDir: path.join(publicRoot, "streamlit-journeys", "recordings", id),
      rich: path.join(publicRoot, "streamlit-journeys", "recordings", id, `${id}.rich.mp4`),
      baseForScreenshots: path.join(publicRoot, "streamlit-journeys", "recordings", id),
    };
  }
  const journeyDir = path.join(publicRoot, publicRel[0], publicRel[1]);
  return {
    keyframesDir: path.join(journeyDir, "keyframes"),
    rich: path.join(journeyDir, `${id}.rich.mp4`),
    baseForScreenshots: journeyDir,
  };
}

function checkManifest(family, manifestFile) {
  const manifest = readJson(manifestFile);
  if (!manifest) return;
  const id = manifest.id || path.basename(path.dirname(manifestFile));
  const targets = manifestTargets(family, id, manifestFile);
  const manifestRel = path.relative(publicRoot, manifestFile).split(path.sep).join("/");
  const journeyPrefix =
    family === "gui" ? `gui-journeys/${id}/` : `${family === "cli" ? "cli" : "streamlit"}-journeys/`;
  const directlyReferenced =
    referencedPublicAssets.has(manifestRel) ||
    [...referencedPublicAssets].some((asset) => asset.startsWith(journeyPrefix) && asset.includes(id));
  const frames = pngCount(targets.keyframesDir);
  const declared = Number(manifest.keyframe_count || manifest.steps?.length || 0);

  if (!existsNonEmpty(manifestFile)) {
    fail(`${rel(manifestFile)} is empty`);
  }
  if (!fs.existsSync(targets.keyframesDir)) {
    fail(`${rel(manifestFile)} has no keyframe directory at ${rel(targets.keyframesDir)}`);
  } else if (frames === 0) {
    fail(`${rel(manifestFile)} has zero keyframe PNGs`);
  }
  if (declared > 0 && frames > 0 && frames < Math.min(declared, 2)) {
    fail(`${rel(manifestFile)} declares ${declared} keyframes but only ${frames} PNGs exist`);
  }
  if (!existsNonEmpty(targets.rich)) {
    const message = `${rel(manifestFile)} is missing rich Remotion MP4 at ${rel(targets.rich)}`;
    if (family === "gui" && !directlyReferenced) {
      warn(message);
    } else {
      fail(message);
    }
  }

  const manifestRich = manifest.recording_rich;
  if (manifestRich) {
    const richFromManifest = path.join(path.dirname(manifestFile), manifestRich);
    const fallbackRich = path.join(publicRoot, manifestRich);
    if (!existsNonEmpty(richFromManifest) && !existsNonEmpty(fallbackRich) && !existsNonEmpty(targets.rich)) {
      const message = `${rel(manifestFile)} recording_rich points to missing ${manifestRich}`;
      if (family === "gui" && !directlyReferenced) {
        warn(message);
      } else {
        fail(message);
      }
    }
  } else {
    warn(`${rel(manifestFile)} has no recording_rich enrichment`);
  }

  for (const step of manifest.steps || []) {
    const shot = step.screenshot_path;
    if (!shot) {
      fail(`${rel(manifestFile)} step ${step.index ?? "?"} has no screenshot_path`);
      continue;
    }
    const candidates = [
      path.join(targets.baseForScreenshots, shot),
      path.join(targets.keyframesDir, shot),
      path.join(path.dirname(manifestFile), shot),
    ];
    if (!candidates.some(existsNonEmpty)) {
      fail(`${rel(manifestFile)} step ${step.index ?? "?"} screenshot_path missing: ${shot}`);
    }
  }
}

function checkJourneyFamily(family, manifestRoot) {
  if (!fs.existsSync(manifestRoot)) {
    fail(`missing ${family} manifest root ${rel(manifestRoot)}`);
    return 0;
  }
  const verified = walk(manifestRoot, (file) => file.endsWith("manifest.verified.json"));
  const raw = walk(manifestRoot, (file) => file.endsWith("manifest.json"));
  const verifiedSet = new Set(verified.map((file) => path.dirname(file)));
  for (const file of raw) {
    if (!verifiedSet.has(path.dirname(file))) {
      fail(`${rel(file)} has no sibling manifest.verified.json`);
    }
  }
  for (const file of verified) checkManifest(family, file);
  return verified.length;
}

function main() {
  const referenceCount = checkMarkdownReferences();
  const counts = {
    cli: checkJourneyFamily("cli", path.join(publicRoot, "cli-journeys", "manifests")),
    streamlit: checkJourneyFamily(
      "streamlit",
      path.join(publicRoot, "streamlit-journeys", "manifests"),
    ),
    gui: checkJourneyFamily("gui", path.join(publicRoot, "gui-journeys")),
  };

  for (const message of warnings) console.warn(`[media-audit:warn] ${message}`);
  if (failures.length) {
    for (const message of failures) console.error(`[media-audit:fail] ${message}`);
    console.error(
      `[media-audit] failed: ${failures.length} issue(s), refs=${referenceCount}, manifests=${JSON.stringify(counts)}`,
    );
    process.exit(1);
  }
  console.log(
    `[media-audit] ok: refs=${referenceCount}, manifests=${JSON.stringify(counts)}, warnings=${warnings.length}`,
  );
}

main();
