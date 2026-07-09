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
- Frames converted BGRA→NV12 immediately after GPU→CPU copy (`bgra_to_nv12()`),
  providing native input for hardware encoders and 2.7× memory savings vs BGRA.
- H.264 encoding: Media Foundation H.264 Video Encoder MFT (`MfH264Encoder`)
  produces compressed H.264 packets stored in ring buffer as `StoredFrame`
  with `PixelFormat::H264` and `is_sync` flag.
- Windows clip muxing: `WindowsEncoder::encode_clip()` muxes H.264 AVCC packets
  directly via the `mp4` crate — no re-encoding during clip save (~0.1 s).

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
