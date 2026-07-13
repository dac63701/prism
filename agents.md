# Prism — Agent Instructions

## Build Reminder

**Always run `npm run tauri build` after completing the full todo list.**

The build produces `Prism_0.1.0_x64_en-US.msi` and `Prism_0.1.0_x64-setup.exe`.
Confirm the build succeeds before reporting completion.

## Development Commands

- `cargo check` — Rust type-check (fast)
- `cargo test` — Run Rust tests (including ring buffer unit tests)
- `npx tsc --noEmit` — TypeScript type-check
- `npm run build` — Vite frontend build
- `npm run tauri build` — Production build (both bundles)

## Architecture

- Rust backend: 10 modules in `src-tauri/src/`
- Frontend: React 18 + TypeScript + Tailwind v4 + Zustand in `src/`
- Routing: MemoryRouter (Tauri-appropriate)
- IPC: `invoke("command_name")` + event listeners via `@tauri-apps/api`
- Windows capture: `IDXGIOutputDuplication::AcquireNextFrame(0)` non-blocking polling;
  D3D11 staging texture reused across frames; fast-path single `memcpy` when
  GPU row pitch matches destination stride.
  Frames converted BGRA→NV12 immediately after GPU→CPU copy (`bgra_to_nv12()`),
  providing native input for hardware encoders and 2.7× memory savings vs BGRA.
- macOS capture: ScreenCaptureKit with SCStream; FPS-limited in callback to
  reduce memory churn on high-refresh displays. `LatestFrame` slot holds the
  most recent frame; polling consumer drains it.
- macOS encoding: VideoToolbox `VtH264Encoder` with BGRA IOSurface, NV12→BGRA
  conversion + resize inside `encode_frame()`. SPS/PPS extracted from VT format
  description on first keyframe.
- H.264 encoding (Windows): Media Foundation H.264 Video Encoder MFT (`MfH264Encoder`)
  produces compressed H.264 packets stored in ring buffer as `StoredFrame`
  with `PixelFormat::H264` and `is_sync` flag.
- Clip saving (both platforms): mux H.264 AVCC packets directly via `mp4` crate
  — no re-encoding during clip save (~0.1 s).
- Thumbnails: NV12→RGB→JPEG generated server-side at clip-save time, saved as
  `clipname_thumb.jpg` alongside MP4. Frontend loads `<img>` directly.

## Key Conventions

- All settings persist immediately (no save button)
- System tray + global hotkeys emit events that frontend handlers pick up
- Recording pipeline: capture → NV12 → H.264 shadow buffer (ring buffer of compressed packets) → MP4
- Shadow buffer: byte-accounted `VecDeque` with 256 MB budget. Auto-evicts oldest frames when exceeded.
  - Compressed H.264 path: ~10 KB/packet, holds ~7 min
  - Raw NV12 fallback path: ~3 MB/frame at 1080p, holds ~82 frames before eviction
- Preview: NV12→RGB→JPEG→base64 data URL, polled at ~1fps
- Clip save: frames extracted from ring buffer under lock, encoded to MP4 outside lock
- SPS/PPS: captured from first keyframe output of MF H.264 encoder, cached in `RecorderInner`,
  prepended to clip data if the original keyframe was evicted from the ring buffer

## Completed Work

### OAuth & Authentication
- Full OAuth sign-in flow with deep-link routing (`prism://` scheme) for Windows
- Auto-retrieve OAuth credentials via session-based polling (no button press needed)
- Robust auth verification with server-side API key validity checks
- Bring app window to foreground after deep-link OAuth sign-in
- Stop re-verifying auth after fresh sign-in; use cloud store for Settings display
- Switch upload auth from API key to JWT `access_token`
- Redirect browser to signin success page after OAuth + suppress webview right-click
- Bold logo + wordmark hero on sign-in/register pages

### Cloud Sync & Clip Management
- Upload queue with clip upload UI (`useUploadQueue` hook, upload processor)
- Live settings API key in upload processor, skip retries on missing file
- Manual multipart body construction for axum 0.8 / multer 3.x compatibility
- Reduce upload retries to 2 (from 3)
- Clip deletion from dashboard list and detail pages
- Fix dashboard: replace admin-only stats with user-scoped data
- Fix: cast `SUM(size_bytes)` to bigint for `NUMERIC/INT8` type mismatch in `get_server_stats`
- Fix: `::text` cast for visibility ENUM in all SELECT/RETURNING queries + activity_logs action/level
- Fix: alter `clips.duration_secs` from REAL to DOUBLE PRECISION to match Rust f64 mapping
- Fix: run `duration_secs` ALTER as raw SQL after migrations (avoids VersionMismatch error)

### MP4 & Thumbnail Fixes
- Compute MP4 timescale from actual frame timestamps
- Stop generating XOR placeholder thumbnails — proper NV12→RGB→JPEG thumbnails now work

### Settings & Recording
- Settings changes no longer start/stop recording
- Add console logging to upload processor for visibility

### Desktop App UI
- UI consistency pass across entire desktop app
- Decorative polish: background glow orbs and clip card hover overlay
- Improve clip library and playback
- Unify app and website styling — replace white borders with `border-border` (#1f2a44), differentiate sidebar background

### Website Frontend
- Add skeleton loading states for all data-fetching routes
- Match skeleton dimensions exactly to real page layouts
- Add `VideoPlayer` component
- Remove redundant chevron icon from download button
- Enhance download page with animated tabs, install guide, and nav links
- Remove expensive blur and layered-gradient effects (website render cleanup)
- Skeleton primitives system: `SkeletonCard`, `SkeletonPanel`, `SkeletonStatCard`, `SkeletonClipsGrid`, `SkeletonTable`, etc.
- Lazy loading with `loading.tsx` for every route segment

### CI/CD & Infrastructure
- Docker images published for Portainer (ci: publish docker images for portainer)
- Rust cache via Swatinem/rust-cache (auto-invalidates on Cargo.lock change)
- Fix: route `/api/*` directly through nginx instead of Next.js rewrite
- Fix: Dockerfile.web — correct `outputFileTracingRoot`, restore manual COPY of `.next/static` and public
- Fix: frontend dist path, remove unused mut, fix standalone server.js path
- Fix CI: per-platform bundle types instead of invalid `--bundles all`, Ubuntu system deps, macOS lane on macos-26

### Code Quality
- `cargo fmt` compliance across all Rust source files (multiple passes)
- `#[allow(dead_code)]` for macOS-specific, Linux-specific, and cross-platform unused variants
- Clippy: `div_ceil`, `clamp`, `is_multiple_of`, `too_many_arguments`, `unwrap_or_default`, `unnecessary_unwrap`, `field_reassign_with_default`
- Replace `gen_random_bytes` with `gen_random_uuid` to drop pgcrypto dependency
- SPS/PPS encoding fallback + brand logo + cargo fmt
- Optimize, consolidate, and cleanup project-wide (tagged v0.1.0)

### 4K 144Hz Monitor Buffering (FIXED)
**Root cause**: The ScreenCaptureKit SCStream fires at the display's native refresh rate (144Hz),
allocating 33 MB BGRA frames (3840×2160×4) in the callback at 144 fps = 4.7 GB/s memory churn.
Combined with slow `FilterType::Triangle` resize and fallback paths using wrong dimensions
(original 4K instead of the resized 1080p), the pipeline could stall silently.

**Fixes applied**:
1. **FPS limiter** in `capture/macos/mod.rs::start()` — SCStream callback now skips frames
   when the time since the last processed frame is < 1/fps, capping allocation rate to the
   configured capture FPS (30 or 60).
2. **Correct NV12 fallback dimensions** in `recording/mod.rs::poll_and_push()` — both the
   encode-error and no-encoder fallback paths now use `nv12_width`/`nv12_height` instead of
   `frame.width`/`frame.height`, so raw NV12 pushes have correct metadata.
3. **Correct H.264 StoredFrame dimensions** — encoded packets now store `nv12_width`/`nv12_height`.
4. **Diagnostic logging** on first frame: capture vs target resolution, native flag, FPS.

### Plan: quality-hotkey-ui ✅ DONE
Resolution/bitrate dropdown, HotkeyCaptureInput, hotkey re-registration, all wired.

### Plan: thumbnails-monitor-fix ✅ DONE
- **Part A (thumbnails)**: ✅ DONE — server-side JPEG thumbnails at clip save.
- **Part B (monitor switching)**: ✅ DONE — encoder reset on stop (Win + Mac), emit on error,
  frontend error surfacing.

### Thumbnail Reliability
- Generate thumbnail from clip data frames when `preview_frame` is unavailable
- Remove unreliable `naturalWidth < 960` check in `ClipThumbnail` component
- Simplify fallback: trust JPEG thumbnail, only fall back to video capture on img error

### Smooth Video Progress
- Replace `onTimeUpdate` (~4Hz) with `requestAnimationFrame` loop (~60fps)
- Show tenths of seconds in time display to eliminate stepping
- Update `currentTime` state immediately on seek

## Pending Work

### High CPU Usage
Investigate and fix high CPU usage in both the desktop app (recording/capture pipeline) and potentially the upload processor. Profile with Windows Task Manager/PerfMon and Rust `perf`-like tooling. Likely suspects: busy-wait polling in capture loop, excessive IPC events, or SQLite write amplification.

### Clip Sharing
Add ability to toggle clip visibility (public/private) via a "Share" button on the website. Generate shareable links that let others watch the clip without authentication. Requires:
- Backend: `PATCH /api/clips/:id/visibility` endpoint
- Frontend: Share button + share modal with link copy
- Public view page at `/s/<share_id>` (existing skeleton loading already in place)

### Remote Clip Name Editing
Allow renaming clips from the website dashboard. Changes sync back to the desktop client via periodic pull mechanism (already in place). Requires:
- Backend: `PATCH /api/clips/:id/name` endpoint
- Frontend: inline edit or rename dialog on clip detail page
- Desktop: honor updated name on next pull

### App UI Full Rewrite / Fix
The desktop app UI needs significant work — potentially a full rewrite. Current state has inconsistencies, missing polish, and suboptimal component architecture. Goals:
- Consistent dark theme application
- Proper component hierarchy (no inline styles, use Tailwind classes)
- Responsive layout that works at small window sizes
- Proper loading/empty/error states everywhere
- Reusable component library aligned with website conventions

### Passive File Update Checks
Add passive checks that verify files have actually been written/updated after operations (clip save, settings persist, upload complete). Use file modification timestamps or content hashing to detect stale state. Surface warnings in the UI if an expected update didn't occur.

### General Cleanup
- Remove dead code and unused imports across Rust and TypeScript codebases
- Consolidate duplicate types between frontend and backend
- Standardize error handling patterns
- Audit all unwraps/expects in Rust code and replace with proper error propagation
- Revisit Tauri commands for consistent return types
- Clean up Zustand store — remove stale fields, normalize shape
- Ensure all IPC event listeners are properly cleaned up on unmount

## Styling Conventions

- All interactive elements get `transition` (not `transition-colors`) to enable transform/opacity/filter animations on hover/active
- Buttons use `<Button>` component from `@/components/ui/button` wherever semantically appropriate (7 variants: `default`, `outline`, `secondary`, `ghost`, `destructive`, `link`, `brand`)
- Cards use `<Card>` from `@/components/ui/brand` for large content sections (rounded-3xl with gradient bg + shadow)
- Interactive elements have `active:scale-[0.98]` or `active:scale-95` for press feedback; clip cards have `hover:scale-[1.02]`
- Focus rings are always `focus-visible:ring-2 focus-visible:ring-blue-500/20 focus-visible:border-blue-400/70`
- Icons from `lucide-react`, consistently `size-4` or `size-3.5`, wrapped with `shrink-0`
- `transition-colors` reserved for passive elements (inputs, toggles) that only animate color/border changes

## Website Frontend Conventions (`website/frontend/`)

### Lazy Loading & Skeletons

Every route segment that fetches async data **must** have a sibling `loading.tsx` file.
This creates an automatic `<Suspense>` boundary so the layout shell renders instantly
while the page's data loads.

**Where to place loading.tsx:**

| Segment | Data fetched by page | Skeleton covers |
|---|---|---|
| `app/dashboard/loading.tsx` | `currentUser()` + `listClips()` | Stat cards, recent clips grid, quick-actions panel |
| `app/dashboard/clips/loading.tsx` | `listClips()` | 6 clip-card skeletons |
| `app/dashboard/clips/[id]/loading.tsx` | `getClip(id)` | Video player area + metadata panel |
| `app/admin/loading.tsx` | `getDashboardStats()` | Stat cards, 3 action cards |
| `app/admin/users/loading.tsx` | `listAdminUsers()` | 5 user-row skeletons |
| `app/admin/users/[id]/loading.tsx` | `getAdminUser(id)` | 4 info-panel skeletons |
| `app/s/[shareId]/loading.tsx` | `getShareMeta(shareId)` | Video player + metadata (wraps in SiteShell) |
| `app/u/[username]/loading.tsx` | `getProfile(username)` | Profile header + 6 clip cards (wraps in SiteShell) |
| `app/download/loading.tsx` | GitHub releases API | Title + 3 platform download cards (wraps in SiteShell) |

When creating a **new page** with async data:
1. Create a `loading.tsx` sibling in the same route segment directory
2. Import skeleton primitives from `@/components/skeleton`
3. Mirror the real page's container div classes so the skeleton matches layout dimensions
4. For public pages that self-wrap in `<SiteShell>` (download, share, profile), the loading
   file must also wrap in `<SiteShell>` so the nav is visible during loading. Dashboard and
   admin loading files must NOT wrap in SiteShell — the layout already provides DashboardShell.

### Skeleton Primitives

Defined in `components/skeleton.tsx`. All use Tailwind `animate-pulse` + dark-theme colors:

- `<Skeleton className="..." />` — base pulsing placeholder
- `<SkeletonCard />` — matches `<Card>` (rounded-3xl, gradient bg)
- `<SkeletonPanel />` — matches `<Panel>` (rounded-2xl, subtle bg)
- `<SkeletonStatCard />` — label + value + hint placeholder
- `<SkeletonSectionHeading />` — eyebrow + title + description placeholders
- `<SkeletonVideoPlayer />` — aspect-video block
- `<SkeletonClipsGrid count={6} />` — grid of thumbnail card skeletons
- `<SkeletonTable rows={5} />` — list of row skeletons with name/email/details
- `<SkeletonUserDetail />` — 2-column grid of 4 info panels
- `<SkeletonDashboardClips />` — recent clips section with 4 thumbnail cards
- `<SkeletonDownloadCards />` — 3 platform download card skeletons

### Rules

- Do NOT add `"use client"` to loading.tsx files — they are server components.
- Do NOT add `export const metadata` to loading.tsx — it is ignored and causes lint warnings.
- Use `Array.from({ length: N })` for repeated skeleton items rather than manual duplication.
- If a page is entirely static (no `await`), it does not need a loading.tsx.
- For future component-level code splitting, use `next/dynamic` from `next/dynamic` with a
  skeleton fallback: `dynamic(() => import("./heavy-component"), { loading: () => <Skeleton className="..." /> })`.

### Build

```bash
cd website/frontend && npm run build
```

## Future Options

- **Custom title bar**: Consider frameless window with custom title bar for premium feel
  (`decorations: false` in tauri.conf.json, drag region in AppLayout)
- **Light mode**: Currently dark-only; would need `@media (prefers-color-scheme: light)`
  overrides for all color tokens
