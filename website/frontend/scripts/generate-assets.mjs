/**
 * Generates PNG assets from the brand logo SVG for both the website and desktop app.
 *
 * Usage:  node scripts/generate-assets.mjs
 *
 * Reads the SVG from ../public/brand/logo.svg, renders it at
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

// ── Parse the SVG from public/brand/logo.svg ─────────────────────────
const svgPath = join(ROOT, "public", "brand", "logo.svg");
const svgContent = readFileSync(svgPath, "utf-8");

// The source SVG uses a fixed viewBox (e.g. "0 0 290 290")
const SVG_SOURCE = svgContent;

// ── Rendering helper ─────────────────────────────────────────────────
async function renderSVG(size) {
  return sharp(Buffer.from(SVG_SOURCE))
    .resize(size, size)
    .png()
    .toBuffer();
}

// ── ICO writer ───────────────────────────────────────────────────────
// ICO format supports embedded PNG data. We write a proper ICO header
// wrapping one or more PNG images.
function writeIco(filePath, pngBuffers) {
  const count = pngBuffers.length;
  const headerSize = 6 + count * 16;
  const header = Buffer.alloc(headerSize);

  let offset = 0;
  // Reserved
  header.writeUInt16LE(0, offset); offset += 2;
  // Type: 1 = ICO
  header.writeUInt16LE(1, offset); offset += 2;
  // Count
  header.writeUInt16LE(count, offset); offset += 2;

  let dataOffset = headerSize;
  for (let i = 0; i < count; i++) {
    const buf = pngBuffers[i];
    const w = i === 0 ? 0 : Math.min(pngBuffers[i].width || 256, 255);

    // Directory entry
    // Width (0 = 256)
    header.writeUInt8(Math.min(pngBuffers[i].width || 0, 255) || 0, offset); offset += 1;
    // Height (0 = 256)
    header.writeUInt8(Math.min(pngBuffers[i].height || 0, 255) || 0, offset); offset += 1;
    // Color palette count
    header.writeUInt8(0, offset); offset += 1;
    // Reserved
    header.writeUInt8(0, offset); offset += 1;
    // Color planes
    header.writeUInt16LE(1, offset); offset += 2;
    // Bits per pixel
    header.writeUInt16LE(32, offset); offset += 2;
    // Image data size
    header.writeUInt32LE(buf.length, offset); offset += 4;
    // Image data offset
    header.writeUInt32LE(dataOffset, offset); offset += 4;

    dataOffset += buf.length;
  }

  // Write header + all PNG data
  const ico = Buffer.concat([header, ...pngBuffers]);
  writeFileSync(filePath, ico);
}

// ── Output directories ───────────────────────────────────────────────
const PUBLIC = join(ROOT, "public");
const TAURI_ICONS = join(ROOT, "..", "..", "src-tauri", "icons");

for (const dir of [PUBLIC, TAURI_ICONS]) {
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true });
}

// ── Generate all assets ──────────────────────────────────────────────
async function main() {
  // Render PNGs at various sizes
  const png32 = await renderSVG(32);
  const png128 = await renderSVG(128);
  const png256 = await renderSVG(256);
  const png512 = await renderSVG(512);

  // Attach width/height metadata for ICO writer
  const withMeta = (buf, w, h) => { buf.width = w; buf.height = h; return buf; };

  // Website favicon (multi-size ICO: 32 + 256)
  writeIco(join(PUBLIC, "favicon.ico"), [
    withMeta(png32, 32, 32),
    withMeta(png256, 256, 256),
  ]);

  // Website OG image (1200x630)
  const ogBg = await sharp({
    create: { width: 1200, height: 630, channels: 3, background: { r: 5, g: 8, b: 22 } },
  }).jpeg().toBuffer();

  const logoLarge = await sharp(Buffer.from(SVG_SOURCE)).resize(400, 400).png().toBuffer();

  const ogComposite = await sharp(ogBg)
    .composite([{ input: logoLarge, top: 115, left: 400 }])
    .jpeg({ quality: 92 })
    .toBuffer();

  writeFileSync(join(PUBLIC, "og-image.jpg"), ogComposite);

  // Tauri app icons
  writeFileSync(join(TAURI_ICONS, "32x32.png"), png32);
  writeFileSync(join(TAURI_ICONS, "128x128.png"), png128);
  writeFileSync(join(TAURI_ICONS, "128x128@2x.png"), png256);
  writeFileSync(join(TAURI_ICONS, "icon.png"), png512);

  // icon.ico — proper ICO format with 32 + 256 entries
  writeIco(join(TAURI_ICONS, "icon.ico"), [
    withMeta(png32, 32, 32),
    withMeta(png256, 256, 256),
  ]);

  console.log("✅ Assets generated");
  console.log("  → public/favicon.ico  (ICO: 32, 256)");
  console.log("  → public/og-image.jpg (1200×630)");
  console.log("  → src-tauri/icons/32x32.png");
  console.log("  → src-tauri/icons/128x128.png");
  console.log("  → src-tauri/icons/128x128@2x.png");
  console.log("  → src-tauri/icons/icon.png");
  console.log("  → src-tauri/icons/icon.ico  (ICO: 32, 256)");
}

main().catch((err) => {
  console.error("Asset generation failed:", err);
  process.exit(1);
});
