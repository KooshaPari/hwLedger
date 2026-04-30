#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const docsRoot = process.cwd();
const publicRoot = path.join(docsRoot, "public");
const failures = [];
const warnings = [];
const referencedPublicAssets = new Set();
const requiredMediaPages = [
  "clients/index.md",
  "predict/index.md",
  "predict/techniques.md",
  "predict/benchmarks.md",
  "predict/methodology.md",
  "fleet/overview.md",
  "fleet/server.md",
  "fleet/agent.md",
  "fleet/audit-log.md",
  "fleet/cloud-rentals.md",
  "fleet/placement.md",
  "fleet/tailscale.md",
  "fleet/ssh-fallback.md",
];

function existsNonEmpty(file) {
  try {
    return fs.statSync(file).size > 0;
  } catch {
    return false;
  }
}

function sizeOf(file) {
  try {
    return fs.statSync(file).size;
  } catch {
    return 0;
  }
}

function hasMuxedAudioStream(file) {
  let buffer;
  try {
    buffer = fs.readFileSync(file);
  } catch {
    return false;
  }

  const containerBoxes = new Set([
    "moov",
    "trak",
    "mdia",
    "minf",
    "stbl",
    "edts",
    "udta",
    "meta",
    "ilst",
  ]);

  function walk(start, end) {
    let offset = start;
    while (offset + 8 <= end) {
      let size = buffer.readUInt32BE(offset);
      const type = buffer.toString("ascii", offset + 4, offset + 8);
      let headerSize = 8;
      if (size === 1) {
        if (offset + 16 > end) return false;
        size = Number(buffer.readBigUInt64BE(offset + 8));
        headerSize = 16;
      } else if (size === 0) {
        size = end - offset;
      }

      const boxEnd = offset + size;
      if (size < headerSize || boxEnd > end) return false;

      const contentStart = offset + headerSize;
      if (type === "hdlr" && contentStart + 12 <= boxEnd) {
        const handlerType = buffer.toString("ascii", contentStart + 8, contentStart + 12);
        if (handlerType === "soun") return true;
      }

      const childStart = type === "meta" ? contentStart + 4 : contentStart;
      if (containerBoxes.has(type) && childStart < boxEnd && walk(childStart, boxEnd)) {
        return true;
      }

      offset = boxEnd;
    }
    return false;
  }

  return walk(0, buffer.length);
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
  const recordingEmbedPattern = /<RecordingEmbed\b([^>]*)>/g;
  const attrPattern = /([:\w-]+)=["']([^"']+)["']/g;
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
    for (const match of text.matchAll(recordingEmbedPattern)) {
      const attrs = {};
      for (const attr of match[1].matchAll(attrPattern)) {
        attrs[attr[1]] = attr[2];
      }
      const tape = attrs.tape;
      const kind = attrs.kind;
      if (!tape && !kind) {
        continue;
      }
      if (!tape || !kind) {
        fail(`${rel(file)} has malformed RecordingEmbed; both tape and kind are required`);
        continue;
      }
      refs += 1;
      for (const target of recordingEmbedTargets(kind, tape)) {
        if (!existsNonEmpty(target)) {
          fail(`${rel(file)} embeds missing ${kind} tape asset ${tape}: ${rel(target)}`);
        } else if (target.endsWith(".mp4") && sizeOf(target) < 1024) {
          fail(`${rel(file)} embeds invalid tiny ${kind} rich MP4 ${tape}: ${rel(target)}`);
        }
      }
    }
  }
  return refs;
}

function recordingEmbedTargets(kind, tape) {
  if (kind === "cli") {
    return [
      path.join(publicRoot, "cli-journeys", "manifests", tape, "manifest.verified.json"),
      path.join(publicRoot, "cli-journeys", "recordings", tape, `${tape}.rich.mp4`),
    ];
  }
  if (kind === "streamlit") {
    return [
      path.join(publicRoot, "streamlit-journeys", "manifests", tape, "manifest.verified.json"),
      path.join(publicRoot, "streamlit-journeys", "recordings", tape, `${tape}.rich.mp4`),
    ];
  }
  if (kind === "gui") {
    return [
      path.join(publicRoot, "gui-journeys", tape, "manifest.verified.json"),
      path.join(publicRoot, "gui-journeys", tape, `${tape}.rich.mp4`),
    ];
  }
  fail(`unknown RecordingEmbed kind ${kind} for tape ${tape}`);
  return [];
}

function checkRequiredMediaPages() {
  const mediaPattern = /<(?:RecordingEmbed|Shot|ShotGallery|JourneyViewer)\b|<video\b/;
  for (const page of requiredMediaPages) {
    const file = path.join(docsRoot, page);
    if (!existsNonEmpty(file)) {
      fail(`required media page is missing: ${page}`);
      continue;
    }
    const text = fs.readFileSync(file, "utf8");
    if (!mediaPattern.test(text)) {
      fail(`${page} must include at least one generated recording, keyframe, or verified journey viewer`);
    }
  }
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

function audioCandidates(family, manifestFile, audio) {
  const familyRoot =
    family === "gui"
      ? path.join(publicRoot, "gui-journeys")
      : path.join(publicRoot, `${family === "cli" ? "cli" : "streamlit"}-journeys`);
  const baseCandidates = [
    path.join(path.dirname(manifestFile), audio),
    path.join(familyRoot, audio),
    path.join(publicRoot, audio),
    path.join(publicRoot, "audio", path.basename(audio)),
  ];
  const expanded = new Set(baseCandidates);
  for (const candidate of baseCandidates) {
    const ext = path.extname(candidate);
    if (ext === ".wav" || ext === ".mp3") {
      expanded.add(candidate.slice(0, -ext.length) + (ext === ".wav" ? ".mp3" : ".wav"));
    }
  }
  return [...expanded];
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
  const screenshotSteps = (manifest.steps || []).filter((step) => step.screenshot_path).length;
  const declared = Number(manifest.keyframe_count || 0);

  if (!existsNonEmpty(manifestFile)) {
    fail(`${rel(manifestFile)} is empty`);
  }
  if (!fs.existsSync(targets.keyframesDir)) {
    fail(`${rel(manifestFile)} has no keyframe directory at ${rel(targets.keyframesDir)}`);
  } else if (frames === 0) {
    fail(`${rel(manifestFile)} has zero keyframe PNGs`);
  }
  if (screenshotSteps === 0) {
    fail(`${rel(manifestFile)} has no keyframed steps`);
  }
  if (manifest.passed === false || manifest.failure) {
    warn(
      `${rel(manifestFile)} has media but the source journey did not pass: ${manifest.failure || "passed=false"}`
    );
  }
  if (declared !== screenshotSteps) {
    fail(`${rel(manifestFile)} declares ${declared} keyframes but has ${screenshotSteps} screenshot-backed steps`);
  }
  if (frames < screenshotSteps) {
    fail(`${rel(manifestFile)} has ${screenshotSteps} screenshot-backed steps but only ${frames} PNGs exist`);
  }
  if (!existsNonEmpty(targets.rich)) {
    const message = `${rel(manifestFile)} is missing rich Remotion MP4 at ${rel(targets.rich)}`;
    if (family === "gui" && !directlyReferenced) {
      warn(message);
    } else {
      fail(message);
    }
  } else if (sizeOf(targets.rich) < 1024) {
    fail(`${rel(manifestFile)} rich Remotion MP4 is too small to be valid media: ${rel(targets.rich)}`);
  }
  if (!manifest.recording_rich_sha256) {
    fail(`${rel(manifestFile)} is missing recording_rich_sha256`);
  }
  if (!manifest.recording_rich_manifest_sha256) {
    fail(`${rel(manifestFile)} is missing recording_rich_manifest_sha256`);
  }
  if (manifest.voiceover?.audio || manifest.recording_audio_voiceover) {
    const audio = manifest.voiceover?.audio || manifest.recording_audio_voiceover;
    const candidates = audioCandidates(family, manifestFile, audio);
    if (!candidates.some(existsNonEmpty)) {
      fail(`${rel(manifestFile)} voiceover audio is missing: ${audio}`);
    }
    if (existsNonEmpty(targets.rich) && !hasMuxedAudioStream(targets.rich)) {
      fail(`${rel(manifestFile)} declares voiceover audio but rich MP4 has no muxed audio stream: ${rel(targets.rich)}`);
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
  checkRequiredMediaPages();
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
