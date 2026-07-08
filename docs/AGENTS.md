# AGENTS.md — Project Architecture & Guidelines

## Project Overview
Cross-platform game clipping desktop app (like Medal / Outplayed). Built with **Tauri 2** (Rust backend) + **React** (TypeScript frontend). Lightweight ring-buffer recorder using platform-native screen capture APIs with hardware-accelerated encoding. System tray background operation, global hotkeys, clip library, game detection for clip metadata, and an optional moment detection plugin system.

## Tech Stack

| Layer | Technology | Why |
|-------|-----------|-----|
| Desktop Framework | **Tauri 2** | Lightweight (vs Electron), Rust backend, webview frontend |
| Frontend | **React 18 + TypeScript + Vite** | Fast dev, strong typing |
| UI | **TailwindCSS + shadcn/ui** | Utility-first, accessible components |
| State | **Zustand** | Minimal boilerplate, good with Tauri IPC |
| Backend | **Rust** | Performance, safety, native API access |
| Screen Capture | **DXGI** (Win) / **ScreenCaptureKit** (Mac) / **PipeWire** (Linux) | Native, minimal overhead |
| HW Encoding | **NVENC** / **Media Foundation** / **VideoToolbox** / **VAAPI** | GPU-accelerated, gameplay impact <1-2ms |
| Video Muxing | **FFmpeg** (via `ffmpeg-next`) | Muxing only, not capture |
| Hotkeys | **rdev** or **global-hotkey** crate | Cross-platform global hotkey registration |
| CI | **GitHub Actions** | Build/test/publish for all 3 platforms |

## Project Structure

```
/
├── docs/
│   ├── PLAN.md           # Implementation plan (14 steps)
│   ├── AGENTS.md         # This file — architecture & guidelines
│   ├── FEATURES.md       # Full feature specifications
│   └── CLOUD-VISION.md   # Server & cloud sharing vision
│
├── src/                    # React frontend
│   ├── components/         # UI components (one per file, PascalCase)
│   │   ├── layout/        # App shell, sidebar, headers
│   │   ├── settings/      # Settings form components
│   │   ├── library/       # Clip grid, clip card, player
│   │   ├── upload/        # Upload progress, queue
│   │   └── common/        # Buttons, inputs, modals
│   ├── pages/              # Route pages
│   ├── stores/             # Zustand stores (slices pattern)
│   ├── hooks/              # Custom hooks (useClip, useSettings, etc.)
│   ├── lib/                # Utilities, constants, types
│   ├── App.tsx
│   └── main.tsx
│
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── main.rs        # Tauri entry point
│   │   ├── commands/      # #[tauri::command] IPC handlers
│   │   ├── capture/       # Screen capture backends
│   │   │   ├── mod.rs     # Common Capture trait + factory
│   │   │   ├── windows/   # DXGI implementation
│   │   │   ├── macos/     # ScreenCaptureKit implementation
│   │   │   └── linux/     # PipeWire implementation
│   │   ├── encoder/       # Hardware encoding wrappers
│   │   │   ├── mod.rs     # Encoder trait + factory
│   │   │   └── codecs/    # H.264, H.265, AV1
│   │   ├── buffer/        # Ring buffer
│   │   │   ├── ring.rs    # Circular buffer implementation
│   │   │   └── pool.rs    # Memory pool for frames
│   │   ├── hotkey/        # Global hotkey registration
│   │   ├── tray/          # System tray menu & events
│   │   ├── games/         # Game detection & metadata
│   │   │   ├── mod.rs     # Process watcher + clip tagging
│   │   │   ├── database/  # Known games DB (JSON patterns)
│   │   │   │   └── games.json
│   │   │   └── moment/    # Moment detection plugins (optional)
│   │   │       ├── mod.rs     # MomentDetector trait + registry
│   │   │       ├── ocr/       # OCR-based detectors (future)
│   │   │       └── audio/     # Audio-based detectors (future)
│   │   ├── settings/      # Configuration system
│   │   │   ├── config.rs  # Config struct + serde
│   │   │   └── store.rs   # Read/write config file
│   │   └── upload/        # Upload API client
│   │       ├── client.rs  # reqwest-based API client
│   │       └── queue.rs   # Upload queue + retry
│   └── Cargo.toml
│
├── package.json
├── tsconfig.json
├── tailwind.config.ts
└── tauri.conf.json
```

## Code Conventions

### Rust
- Run `cargo fmt` and `cargo clippy` — zero warnings
- Async with `tokio` where beneficial (file I/O, network requests)
- Capture/encoder backends: trait-based with `cfg` platform gating
- Error handling: custom error enum with `thiserror`
- Logging: `tracing` crate with structured events
- IPC commands: typed params, return `Result<T, String>` for Tauri
- No `unsafe` unless absolutely required (with safety comment)

### TypeScript
- `strict: true` in tsconfig — no `any`
- Named exports preferred over default exports
- Types/interfaces in co-located `.types.ts` files
- IPC calls wrapped in typed service functions (e.g., `clipService.ts`)

### React
- One component per file, PascalCase
- Components are functional with hooks
- State logic in Zustand stores (not in components)
- Side effects in custom hooks (not directly in components)

## IPC Communication

Tauri commands are defined in Rust with `#[tauri::command]` and called from frontend via `@tauri-apps/api/core` `invoke()`.

Pattern:
```rust
// Rust side
#[tauri::command]
async fn trigger_clip(state: State<'_, AppState>) -> Result<ClipResult, String> {
    state.recorder.trigger_clip().await.map_err(|e| e.to_string())
}
```

```typescript
// Frontend side
import { invoke } from '@tauri-apps/api/core';

const clip = await invoke<ClipResult>('trigger_clip');
```

## Platform Gating (Rust)

```rust
#[cfg(target_os = "windows")]
mod capture {
    pub use self::windows::*;
    // ...
}

#[cfg(target_os = "macos")]
mod capture {
    pub use self::macos::*;
    // ...
}
```

## Constraints
- **CPU overhead must be <2%** during background recording
- **Memory buffer capped** at configurable max duration
- **No Electron** — Tauri webview only, keep JS bundle small
- **Privacy-first** — no telemetry, all recording local until explicit upload
- **Game detection** (core): process name + window title matching only — no memory scanning, no injection
- **Moment detection** (optional): read-only approaches preferred (OCR, audio) — no game memory injection
- **Moment detection is opt-in** — disabled by default, zero CPU cost until user enables a game plugin

## Build & Run Commands
```bash
npm run tauri dev        # Dev mode with hot reload
npm run tauri build      # Production bundle
cargo test               # Rust tests
npm run test             # Frontend tests
cargo clippy             # Rust lint
```

## Key Rust Dependencies
- `tauri` v2
- Platform capture: platform-specific crates (gated by cfg)
- `ffmpeg-next` for muxing
- `rdev` or `global-hotkey` for hotkeys
- `tray-icon` for system tray
- `serde` + `serde_json` for config
- `reqwest` for HTTP uploads
- `tracing` + `tracing-subscriber` for logging

## Key Frontend Dependencies
- `react` + `react-dom` 18
- `@tauri-apps/api` v2
- `zustand`
- `tailwindcss` + `shadcn/ui`
- `lucide-react` (icons)
- `react-router-dom` (if multi-page)
