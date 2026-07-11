/**
 * Generates PNG assets from the brand logo SVG for both the website and desktop app.
 *
 * Usage:  node scripts/generate-assets.mjs
 *
 * Reads the SVG from ../lib/brand.ts (LOGO_SVG export), renders it at
 * multiple sizes, and writes the output to:
 *   - ../public/              (website favicon, og:image)
 *   - ../../src-tauri/icons/  (Tauri app icons)
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "..");

// ── Parse the SVG from lib/brand.ts ──────────────────────────────────
const brandPath = join(ROOT, "lib", "brand.ts");
const brandSrc = readFileSync(brandPath, "utf-8");

// Extract the SVG string between backtick quotes after LOGO_SVG =
const match = brandSrc.match(/LOGO_SVG\s*=\s*`([\s\S]*?)`/);
if (!match) throw new Error("Could not extract LOGO_SVG from lib/brand.ts");
const svgContent = match[1];

// Wrap in a full SVG document with explicit size for rendering
function wrapSVG(viewBox) {
  return `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="${viewBox}" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
${svgContent.replace(/<svg[^>]*>/, "").replace(/<\/svg>/, "")}
</svg>`;
}

const SVG_SOURCE = wrapSVG("0 0 24 24");

// ── Rendering helper ─────────────────────────────────────────────────
async function renderSVG(size, pad = 0) {
  const total = size + pad * 2;
  const svg = wrapSVG(`${-pad} ${-pad} ${total} ${total}`);
  return sharp(Buffer.from(svg))
    .resize(size, size)
    .png()
    .toBuffer();
}

async function renderJPEG(size) {
  const bg = await sharp({
    create: {
      width: size,
      height: Math.round(size * 0.525),
      channels: 3,
      background: { r: 5, g: 8, b: 22 },
    },
  })
    .jpeg()
    .toBuffer();

  const logo = await sharp(Buffer.from(SVG_SOURCE))
    .resize(Math.round(size * 0.4), Math.round(size * 0.4))
    .png()
    .toBuffer();

  return sharp(bg)
    .composite([
      {
        input: logo,
        top: Math.round(size * 0.12),
        left: Math.round(size * 0.3),
      },
    ])
    .jpeg({ quality: 90 })
    .toBuffer();
}

// ── Output directories ───────────────────────────────────────────────
const PUBLIC = join(ROOT, "public");
const TAURI_ICONS = join(ROOT, "..", "..", "src-tauri", "icons");

for (const dir of [PUBLIC, TAURI_ICONS]) {
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true });
}

// ── Generate all assets ──────────────────────────────────────────────
async function main() {
  // Website favicon (32x32 PNG — modern favicon)
  writeFileSync(join(PUBLIC, "favicon.ico"), await renderSVG(32));

  // Website OG image (1200x630)
  const ogBuffer = await sharp({
    create: {
      width: 1200,
      height: 630,
      channels: 3,
      background: { r: 5, g: 8, b: 22 },
    },
  })
    .jpeg()
    .toBuffer();

  const logoLarge = await sharp(Buffer.from(SVG_SOURCE))
    .resize(400, 400)
    .png()
    .toBuffer();

  const ogComposite = await sharp(ogBuffer)
    .composite([
      {
        input: logoLarge,
        top: 115,
        left: 400,
      },
    ])
    .jpeg({ quality: 92 })
    .toBuffer();

  writeFileSync(join(PUBLIC, "og-image.jpg"), ogComposite);

  // Tauri desktop app icons
  writeFileSync(join(TAURI_ICONS, "32x32.png"), await renderSVG(32));
  writeFileSync(join(TAURI_ICONS, "128x128.png"), await renderSVG(128));
  writeFileSync(join(TAURI_ICONS, "128x128@2x.png"), await renderSVG(256));

  // icon.png (larger version for Tauri)
  writeFileSync(join(TAURI_ICONS, "icon.png"), await renderSVG(512));

  // icon.ico (multi-size ICO — sharp can't write .ico, so use PNG as fallback)
  writeFileSync(join(TAURI_ICONS, "icon.ico"), await renderSVG(256));

  // macOS icns — skip, we keep the existing one since .icns requires special tooling

  console.log("✅ Assets generated");
  console.log(`  → ${PUBLIC}/favicon.ico`);
  console.log(`  → ${PUBLIC}/og-image.jpg`);
  console.log(`  → ${TAURI_ICONS}/32x32.png`);
  console.log(`  → ${TAURI_ICONS}/128x128.png`);
  console.log(`  → ${TAURI_ICONS}/128x128@2x.png`);
  console.log(`  → ${TAURI_ICONS}/icon.png`);
  console.log(`  → ${TAURI_ICONS}/icon.ico`);
}

main().catch((err) => {
  console.error("Asset generation failed:", err);
  process.exit(1);
});
