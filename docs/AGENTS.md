# Project Architecture & Guidelines

> **Note:** The canonical agent instructions (build commands, completed work,
> pending work, and conventions) live in the **root [`AGENTS.md`](../AGENTS.md)**.
> This file is the architecture + guidelines reference. For the user-facing UI
> see [`UI.md`](UI.md); for the full pipeline see
> [`ARCHITECTURE.md`](ARCHITECTURE.md).

## Project Overview

Cross-platform game clipping desktop app (like Medal / Outplayed) with a
self-hostable cloud. A **Tauri 2** (Rust backend) + **React** (TypeScript)
desktop app continuously records the screen into a compressed H.264 shadow
buffer; the user triggers a clip to save the last N seconds to MP4 and optionally
uploads it to the cloud for sharing. Includes game detection, auto-clipping for
CS2/Rust, system tray, global hotkeys, and a web dashboard.

## Tech Stack

| Layer | Technology | Why |
|-------|-----------|-----|
| Desktop Framework | **Tauri 2** | Lightweight, Rust backend, webview frontend |
| Desktop Frontend | **React 19 + TypeScript + Vite** | Fast dev, strong typing |
| Desktop UI | **TailwindCSS v4 + shadcn-style primitives + lucide-react** | Utility-first, dark-only |
| State | **Zustand** | Minimal boilerplate, good with Tauri IPC |
| Backend | **Rust** | Performance, safety, native API access |
| Screen Capture | **DXGI** (Win) / **ScreenCaptureKit** (Mac) / PipeWire stub (Linux) | Native, minimal overhead |
| HW Encoding | **Media Foundation** (Win, incl. async MFTs) / **VideoToolbox** (Mac) | GPU-accelerated |
| Muxing | **mp4** crate (H.264 AVCC) | No re-encode on clip save |
| Audio | **WASAPI** loopback → AAC (Win) | System audio in clips |
| Hotkeys | **tauri-plugin-global-shortcut** | Cross-platform registration |
| CI | **GitHub Actions** | Desktop bundles + cloud Docker images |
| Cloud API | **Axum 0.8 + SQLx (Postgres)** | JWT auth, clips, admin |
| Cloud Web | **Next.js 15** | Dashboard + player pages + docs |

## Project Structure

```
/
├── README.md                 # Project overview + quickstart
├── AGENTS.md                 # Agent instructions (canonical)
├── docs/
│   ├── UI.md                 # Desktop app UI reference
│   ├── ARCHITECTURE.md       # Pipeline + cloud architecture
│   ├── FEATURES.md           # Original feature specs
│   ├── PLAN.md               # Implementation plan & status
│   ├── CLOUD-VISION.md       # Cloud roadmap
│   ├── plans/                # Per-feature implementation plans
│   └── specs/                # Feature specs
│
├── src/                      # Desktop React frontend
│   ├── App.tsx               # MemoryRouter + routes + ErrorBoundary
│   ├── components/           # layout/ common/ settings/ auth/ ui/ upload/
│   ├── pages/                # Home, Library, ClipDetail, Settings
│   ├── stores/               # Zustand stores (settings, recording, clips, cloud, toast)
│   ├── hooks/                # useSettingsActions, useDisplayRefreshRate, ...
│   ├── lib/                  # utils, presets, constants
│   └── types/                # shared types
│
├── src-tauri/                # Desktop Rust backend
│   ├── src/
│   │   ├── main.rs           # entry point
│   │   ├── lib.rs            # builder, plugins, setup, invoke handler
│   │   ├── commands/         # #[tauri::command] IPC handlers
│   │   ├── capture/          # windows/ macos/ linux/ backends + trait
│   │   ├── encoder/          # windows/mf_encoder.rs, macos/vt_encoder.rs, codecs/
│   │   ├── buffer/           # ring buffer (byte-accounted, 256 MB)
│   │   ├── recording/        # poll_and_push 3-phase loop, clip save
│   │   ├── games/            # database/ cs2/ rust/ moment/ + trigger.rs
│   │   ├── upload/           # queue.rs + client.rs
│   │   ├── auth/             # OAuth deep-link auth
│   │   ├── settings/         # config.rs + store.rs
│   │   ├── hotkey/           # global shortcut registration
│   │   ├── tray/             # system tray menu & events
│   │   ├── audio/            # WASAPI loopback → AAC (Windows)
│   │   └── notification.rs   # toast AUMID self-heal (Win) / permissions (Mac)
│   └── Cargo.toml
│
└── website/                  # Prism Cloud
    ├── src/                  # Axum API (auth, clips, admin, media, config)
    ├── frontend/             # Next.js app (dashboard, admin, player, docs)
    ├── Dockerfile*           # api + web images
    └── docker-compose*.yml   # dev/prod stacks
```

## Code Conventions

### Rust
- Run `cargo fmt` and `cargo check` — keep clean
- Async with `tokio` where beneficial (file I/O, network)
- Capture/encoder backends: trait-based with `cfg` platform gating
- Locks: `parking_lot::Mutex` (no `std::sync::Mutex` in the recorder)
- Error handling: `thiserror` + `Result<T, String>` for Tauri commands
- IPC commands: typed params, return `Result<T, String>`
- No `unsafe` unless absolutely required (with safety comment)

### TypeScript / React
- `strict: true` — no `any`
- One component per file, PascalCase, functional with hooks
- State logic in Zustand stores; side effects in custom hooks
- IPC calls wrapped in typed store/hook actions
- Named exports preferred

## IPC Communication

Rust commands defined with `#[tauri::command]`, called from frontend via
`invoke()` from `@tauri-apps/api/core`. Events flow Rust → frontend through
`app.emit()` and `listen()` (e.g. `hotkey-pressed`, `settings-changed`,
`recording-state-changed`, `game-detected`, upload progress).

## Constraints

- **CPU overhead <2%** target during background recording; encode runs off the
  runtime's critical path (3-phase lock-free encode)
- **Memory buffer capped** at 256 MB budget (byte-accounted)
- **No Electron** — Tauri webview only
- **Privacy-first** — no telemetry; recording is local until explicit upload
- **Game detection is read-only** — window title matching, documented APIs,
  localhost HTTP (CS2 GSI), audio analysis (Rust). No injection, no memory reads
- **Clip save never re-encodes** — direct H.264 AVCC muxing with cached SPS/PPS

## Build & Run

See the root [`AGENTS.md`](../AGENTS.md) for the canonical commands:

```bash
npm run tauri dev        # Dev mode with hot reload
npm run tauri build      # Production bundle (MSI + NSIS)
cargo test               # Rust tests
npx tsc --noEmit         # Frontend type-check
cd website/frontend && npm run build   # Website build
```