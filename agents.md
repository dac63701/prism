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

## Active Issues & Plans

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

### Plan: quality-hotkey-ui (NOT STARTED)
Resolution/bitrate dropdown + click-to-capture hotkey inputs + hotkey re-registration.

### Plan: thumbnails-monitor-fix
- **Part A (thumbnails)**: ✅ DONE — `preview_frame` in ClipData, `generate_thumbnail()` in
  `commands/recording.rs` at 320px JPEG Q75 after clip save. Frontend `<img>` loading in
  ClipDetailPage.tsx.
- **Part B (monitor switching)**: 🟡 IN PROGRESS — B-1 (encoder reset on stop) is done;
  B-2 (always emit state event) and B-3 (frontend error surfacing) are pending.
