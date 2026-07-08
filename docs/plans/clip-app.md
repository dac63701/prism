---
plan name: clip-app
plan description: Cross-platform game clipping app
plan status: active
---

## Idea
A cross-platform game clipping desktop app (Tauri 2 + React) with a ring-buffer recording engine using platform-native capture APIs (DXGI/Metal/PipeWire) and hardware encoding. Features: global hotkeys for clipping, configurable buffer/codec/quality settings, system tray, clip library, game detection plugin system for auto-clipping (starting with Rust), and a foundation for future cloud upload/sharing.

## Implementation
- Scaffold Tauri 2 + React project with Rust backend structure, CI config for Win/Mac/Linux
- Implement platform-native screen capture modules: DXGI (Win), ScreenCaptureKit (Mac), PipeWire (Linux) as Rust plugins
- Build ring-buffer recording engine: circular memory buffer, configurable duration, framerate-aware dropping
- Integrate hardware encoding per platform: NVENC/Media Foundation (Win), VideoToolbox (Mac), VAAPI (Linux), with codec selection (H.264/H.265/AV1)
- Create settings system: JSON/config file, Rust-side validation, React settings UI with all recording parameters (buffer duration, codec, quality, fps, output dir, hotkeys)
- Build system tray integration: tray icon, minimize-to-tray, context menu (clip now, open, quit), background recording indicator
- Implement global hotkey system: platform hotkey registration (rdev/global-hotkey crate), rebindable in settings, conflict detection
- Build clip library UI: thumbnail grid, search/filter, delete/rename, trim (basic), open file location, upload button placeholder
- Create game detection plugin system: trait-based GameDetector plugin interface, process watcher that detects foreground game, triggers auto-clip rules
- Implement Rust game detector: process name detection for configurable games, shared state with recording engine for auto-clip triggers
- Add auto-clipping rules system: per-game rules (detect event → auto-save buffer), threshold config, cooldown to avoid spam
- Build upload/share foundation: API client module in Rust, upload queue, share link copy, upload progress UI, designed for future server integration
- Cross-platform performance profiling: measure CPU/GPU overhead, memory usage, disk I/O under game load, optimize ring-buffer and encoding pipeline
- Package and distribution setup: Tauri bundler config for MSI/DMG/AppImage, auto-updater, code signing setup

## Required Specs
<!-- SPECS_START -->
- agents-guidelines
- clip-features
<!-- SPECS_END -->