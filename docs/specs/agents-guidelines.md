# Spec: agents-guidelines

Scope: repo

# AGENTS.md — General Guidelines

## Project Overview
Cross-platform game clipping desktop app built with **Tauri 2** (Rust backend) + **React** (TypeScript frontend). Lightweight ring-buffer recorder using platform-native screen capture APIs with hardware-accelerated encoding. System tray background operation, global hotkeys, clip library, game detection plugins.

## Tech Stack
- **Desktop Framework:** Tauri 2
- **Frontend:** React 18+ / TypeScript / Vite
- **Backend Language:** Rust
- **Screen Capture:** DXGI (Win), ScreenCaptureKit (Mac), PipeWire (Linux)
- **Hardware Encoding:** NVENC / Media Foundation (Win), VideoToolbox (Mac), VAAPI (Linux)
- **Video Muxing:** FFmpeg (muxing only, not capture)
- **State Management:** Zustand (lightweight)
- **UI Library:** TailwindCSS + shadcn/ui
- **CI:** GitHub Actions — build/test/publish for all 3 platforms

## Build & Run
- `npm run tauri dev` — dev mode with hot reload
- `npm run tauri build` — production bundle
- `cargo test` — Rust unit/integration tests
- `npm run test` — frontend tests

## Code Conventions
- Rust: `cargo fmt`, clippy clean, async with tokio where beneficial
- TypeScript: strict mode, no `any`, named exports preferred
- Components: one component per file, PascalCase
- State: Zustand stores in `src/stores/`, slices pattern
- IPC: Rust commands in `src-tauri/src/commands/`, typed with serde
- Recording backend: trait-based platform abstraction in `src-tauri/src/capture/`
- Plugin system: trait-based `GameDetector` in `src-tauri/src/games/`

## Project Structure
```
/
├── src/                    # React frontend
│   ├── components/         # UI components
│   ├── pages/             # Route pages
│   ├── stores/            # Zustand stores
│   ├── hooks/             # Custom hooks
│   └── lib/               # Utilities
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── commands/      # Tauri IPC commands
│   │   ├── capture/       # Screen capture backends
│   │   │   ├── windows/   # DXGI implementation
│   │   │   ├── macos/     # ScreenCaptureKit implementation
│   │   │   └── linux/     # PipeWire implementation
│   │   ├── encoder/       # Hardware encoding
│   │   ├── buffer/        # Ring buffer
│   │   ├── hotkey/        # Global hotkey registration
│   │   ├── tray/          # System tray
│   │   ├── games/         # Game detection plugins
│   │   ├── settings/      # Configuration system
│   │   └── upload/        # Upload API client
│   └── Cargo.toml
└── package.json
```

## Important Constraints
- **CPU overhead must be <2%** during background recording
- **Memory buffer capped** at configurable max duration
- **No Electron** — Tauri webview only, keep JS minimal
- **Privacy-first** — no telemetry, all recording local until explicit upload
- **Game detection plugins** must not use invasive memory scanning; process name + window title matching preferred

## Key Dependencies (Rust)
- `tauri` v2
- `cap` / `dxgi-rs` / `screenpipe` platform capture crates
- `ffmpeg-next` for muxing
- `rdev` or `global-hotkey` for hotkeys
- `tray-icon` for system tray
- `serde` / `serde_json` for config
- `reqwest` for upload API calls
- `tracing` for logging

## Key Dependencies (Frontend)
- `react` + `react-dom` 18
- `@tauri-apps/api` v2
- `zustand`
- `tailwindcss` + `shadcn/ui`
- `lucide-react` (icons)
- `react-router-dom` (if multi-page)