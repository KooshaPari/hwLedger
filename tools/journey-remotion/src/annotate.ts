/**
 * annotate.ts — draw bbox/pointer/highlight annotations on keyframe PNGs via
 * `sharp` (composite with SVG overlay). Reads annotations from the enriched
 * manifest; writes `<frame>.annotated.png` next to each source frame.
 *
 * Usage:
 *   bun run src/annotate.ts --manifest <path> --keyframes-dir <dir>
 *
 * Exits 0 if all frames wrote successfully (or if no annotations found —
 * this is a no-op case). Non-zero on IO / sharp failure.
 */
import { promises as fs } from "node:fs";
import * as path from "node:path";
import sharp from "sharp";
import type { Annotation, RichManifest } from "./types";

interface Args {
  manifest: string;
  keyframesDir: string;
}

function parseArgs(argv: string[]): Args {
  const a: Partial<Args> = {};
  for (let i = 0; i < argv.length; i++) {
    const cur = argv[i];
    const next = argv[i + 1];
    if (cur === "--manifest" && next) { a.manifest = next; i++; }
    else if (cur === "--keyframes-dir" && next) { a.keyframesDir = next; i++; }
  }
  if (!a.manifest || !a.keyframesDir) {
    throw new Error("usage: annotate.ts --manifest <path> --keyframes-dir <dir>");
  }
  return a as Args;
}

function annotationsToSvg(width: number, height: number, ann: Annotation[]): string {
  const esc = (s: string) =>
    s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  const rects = ann.map((a) => {
    const [x, y, w, h] = a.bbox;
    const color = a.color ?? "#34d399";
    const dash = a.style === "dashed" ? 'stroke-dasharray="6,4"' : "";
    const fill =
      a.kind === "highlight" ? `fill="${color}" fill-opacity="0.15"` : 'fill="none"';
    return `<rect x="${x}" y="${y}" width="${w}" height="${h}" ${fill} stroke="${color}" stroke-width="3" ${dash}/>`;
  });
  const labels = ann.map((a) => {
    const [x, y] = a.bbox;
    const color = a.color ?? "#34d399";
    const text = esc(a.label);
    const labelY = Math.max(0, y - 6);
    return `<g>
      <rect x="${x}" y="${labelY - 20}" width="${Math.max(60, text.length * 8 + 14)}" height="22" fill="${color}" rx="3"/>
      <text x="${x + 7}" y="${labelY - 5}" font-family="Inter, Arial, sans-serif" font-size="14" font-weight="700" fill="#0a0a0f">${text}</text>
    </g>`;
  });
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">
    ${rects.join("\n")}
    ${labels.join("\n")}
  </svg>`;
}

async function annotateOne(
  framePath: string,
  outPath: string,
  annotations: Annotation[],
): Promise<void> {
  const img = sharp(framePath);
  const meta = await img.metadata();
  const width = meta.width ?? 1280;
  const height = meta.height ?? 800;
  const svg = annotationsToSvg(width, height, annotations);
  await img
    .composite([{ input: Buffer.from(svg), top: 0, left: 0 }])
    .png()
    .toFile(outPath);
}

async function main(): Promise<void> {
  const { manifest: manifestPath, keyframesDir } = parseArgs(process.argv.slice(2));
  const raw = await fs.readFile(manifestPath, "utf-8");
  const manifest: RichManifest = JSON.parse(raw);
  const annotated: string[] = [];
  for (const step of manifest.steps ?? []) {
    const ann = step.annotations ?? [];
    if (!ann.length) continue;
    // GUI manifests emit `screenshot_path: "keyframes/frame_NNN.png"`
    // (path is relative to the journey dir, not the staged keyframes dir).
    // The staged `keyframesDir` already IS the keyframes tree, so prefer the
    // basename to avoid doubling `keyframes/keyframes/`.
    const basename = path.basename(step.screenshot_path);
    const src = path.join(keyframesDir, basename);
    const outName = basename.replace(/\.png$/i, ".annotated.png");
    const out = path.join(keyframesDir, outName);
    try {
      await annotateOne(src, out, ann);
      annotated.push(outName);
      console.log(`annotated ${outName} (${ann.length} bbox)`);
    } catch (err) {
      console.error(`annotate failed for ${src}: ${(err as Error).message}`);
      throw err;
    }
  }
  // Write the updated manifest (with annotated_keyframes field) alongside.
  manifest.annotated_keyframes = annotated;
  await fs.writeFile(manifestPath, JSON.stringify(manifest, null, 2));
  console.log(`wrote ${annotated.length} annotated keyframes; manifest updated`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
