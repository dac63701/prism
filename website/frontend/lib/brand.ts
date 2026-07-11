/**
 * Single source of truth for the Prism brand logo.
 *
 * To change the logo everywhere (website + desktop app):
 *   1. Update LOGO_SVG below with your new SVG markup.
 *   2. Run:  npm run generate-assets
 *   3. Rebuild and redeploy.
 *
 * The SVG should use viewBox="0 0 24 24" and currentColor for stroke/fill
 * so it picks up the parent's text colour automatically.
 */

export const LOGO_SVG = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
  <path d="M12 3 5 7.2v9.6L12 21l7-4.2V7.2L12 3Z" />
  <path d="M12 7.2v13.8" />
  <path d="M5 7.2 12 12l7-4.8" />
</svg>`;

export const BRAND_NAME = "Prism";
export const BRAND_TAGLINE = "Clip-based screen recording";
