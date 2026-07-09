---
plan name: thumbnails-monitor-fix
plan description: Server-side thumbnails + monitor fix
plan status: active
---

## Idea
Two parts: (A) Generate JPEG thumbnails on Rust side during clip save to eliminate slow video-decode-in-browser approach. Capture latest_frame in ClipData, convert NV12→JPEG via image crate, save alongside MP4. Frontend loads <img> directly — instant. (B) Fix monitor switching: reset H.264 encoder in stop_recording(), always emit recording-state-changed event, surface capture target errors to frontend.

## Implementation
- Part A-1: Add preview_frame field to ClipData struct in recording/mod.rs — clone latest_frame in extract_clip_data() under the recorder lock
- Part A-2: Add generate_thumbnail() helper in capture/mod.rs — converts NV12/BGRA to JPEG via image crate
- Part A-3: In commands/recording.rs save_clip, after MP4 encode, convert preview frame to JPEG and save as clipname_thumb.jpg
- Part A-4: Update ClipThumbnail.tsx — first try loading <img src=thumb.jpg>, fall back to current video element for old clips without .jpg sibling
- Part B-1: In recording/mod.rs stop_recording() macOS block, set h264_encoder = None to force re-creation with new display dimensions
- Part B-2: In commands/recording.rs set_capture_target, restructure to emit recording-state-changed event even on error
- Part B-3: In HomePage.tsx handleSourceChange, surface errors into recording store error field instead of just console.log
- Verify: cargo check + tsc + build

## Required Specs
<!-- SPECS_START -->
- agents-guidelines
- thumbnails-monitor-fix
<!-- SPECS_END -->