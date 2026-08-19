# Prism

Clip-based screen recording for gamers. A lightweight **Tauri 2** desktop app that
continuously records your screen into a compressed H.264 shadow buffer, saves the
last N seconds the moment you trigger a clip, and uploads it to a self-hostable
cloud service with shareable links, public profiles, and a web dashboard.

Think Medal / Outplayed, but privacy-first and self-hosted.

```
┌─────────────────────────┐         ┌──────────────────────────────┐
│   Prism Desktop (Tauri) │         │   Prism Cloud (Docker)       │
│                         │ upload  │                              │
│  capture → NV12 → H.264 ┼────────▶│  Axum API ──▶ PostgreSQL     │
│  shadow buffer (256 MB) │         │       │                      │
│  hotkey / auto clip     │         │       └─▶ local /data clips  │
│  clip library           │         │  Next.js web dashboard       │
│  system tray + hotkeys  │         │  public player pages (/s/id) │
└─────────────────────────┘         └──────────────────────────────┘
```

## Features

- **Always-on shadow buffer** — records continuously at native resolution;
  save the last 10s–30min on demand with zero setup. ~2.7× memory savings by
  converting frames BGRA→NV12 before encoding.
- **Hardware-accelerated H.264** — Media Foundation (Windows) and VideoToolbox
  (macOS) encoders with async MFT support for NVIDIA/AMD GPUs, plus software fallback.
- **Instant clip export** — compressed H.264 packets are muxed straight to MP4
  (no re-encoding), so clip save takes ~0.1s. Server-generated JPEG thumbnails
  appear alongside each clip.
- **Global hotkeys & system tray** — save clips from anywhere (`Ctrl+Shift+X`),
  rebindable in settings; tray menu with quick actions; minimize-to-tray.
- **Auto-clipping (game-aware)** — built-in game detection plus per-game moment
  triggers: **CS2** via Valve's official Game State Integration API and **Rust**
  via Windows WASAPI audio analysis (read-only, anti-cheat-safe).
- **Cloud upload & sharing** — one-click upload from the library, upload queue
  with retry + persistence, auto-copied share links, public profiles.
- **Clip library** — thumbnail grid with search, sort, game/status filters,
  inline rename, metadata editing, and delete.
- **Web dashboard & public pages** — manage clips, rename, set visibility
  (public/unlisted/private), and share via polished player pages with Open Graph
  previews.
- **Self-hostable cloud** — full Docker Compose stack (Postgres + Axum API +
  Next.js web + optional nginx), admin panel, usage limits, OAuth (Google) login.

## Repository layout

```
├── src/                  # Desktop app frontend (React + TypeScript + Vite)
├── src-tauri/            # Desktop app backend (Rust / Tauri 2)
│   └── src/
│       ├── capture/      # Screen capture (DXGI / ScreenCaptureKit / PipeWire)
│       ├── encoder/      # H.264 encoders (MF / VideoToolbox / software)
│       ├── buffer/       # Ring buffer (byte-accounted VecDeque, 256 MB)
│       ├── recording/    # Recording pipeline + clip save
│       ├── games/        # Game detection + CS2 GSI + Rust audio + auto-clip
│       ├── upload/       # Upload queue + API client
│       ├── auth/         # OAuth deep-link auth (prism://)
│       ├── settings/     # Settings manager + config store
│       ├── hotkey/       # Global hotkey registration
│       ├── tray/         # System tray
│       ├── audio/        # Windows WASAPI system-audio capture → AAC
│       └── commands/     # #[tauri::command] IPC handlers
├── website/              # Prism Cloud (Rust server + Next.js frontend)
│   ├── src/              # Axum API (auth, clips, admin, media)
│   └── frontend/         # Next.js app (dashboard, player, docs, landing)
└── docs/                 # Project documentation (see below)
```

## Tech stack

| Layer | Technology |
|-------|-----------|
| Desktop framework | **Tauri 2** (Rust backend + webview frontend) |
| Desktop frontend | **React 18 + TypeScript + Vite** |
| Desktop UI | **Tailwind CSS v4 + shadcn-style primitives + lucide-react** |
| State | **Zustand** |
| Rust | **tokio**, **parking_lot**, **serde**, **thiserror** |
| Screen capture | **DXGI Desktop Duplication** (Win), **ScreenCaptureKit** (macOS) |
| Encoding | **Media Foundation H.264 MFT** (Win), **VideoToolbox** (macOS) |
| Muxing | **mp4** crate (H.264 AVCC packets, no re-encode) |
| Audio capture | **WASAPI** (Win) |
| Cloud API | **Axum 0.8 + SQLx (Postgres)** |
| Cloud web | **Next.js 15** |
| Deploy | **Docker Compose**, GitHub Actions CI |

## Getting started (development)

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [Node.js](https://nodejs.org/) 20+ and npm
- Tauri prerequisites for your platform
  ([Tauri docs](https://v2.tauri.app/start/prerequisites/)):
  - Windows: WebView2 + MSVC Build Tools
  - macOS: Xcode Command Line Tools
  - Linux: webkit2gtk, etc.

### Desktop app

```bash
npm install

# Dev mode with hot reload (launches the Tauri window)
npm run tauri dev

# Production bundle (MSI + NSIS on Windows, DMG on macOS)
npm run tauri build
```

### Cloud server

See [website/README.md](website/README.md) for the full setup guide.

```bash
cd website

# 1. Configure environment
cp .env.example .env   # set JWT_SECRET (32+ chars) and DATABASE_URL

# 2. Full stack (Postgres + API + web) with Docker
docker compose up --build -d

# 3. Or a bundled local run without Docker
make serve

# 4. Open the dashboard
open http://localhost:3000
```

The first registered user is not automatically admin. Promote one with:

```bash
docker compose exec postgres psql -U prism -d prism \
  -c "UPDATE users SET role = 'admin' WHERE email = 'admin@example.com';"
```

## Testing & validation

| Command | Purpose |
|---------|---------|
| `cargo check` | Rust type-check (fast) |
| `cargo test` | Rust unit tests (incl. ring-buffer + encoder tests) |
| `npx tsc --noEmit` | TypeScript type-check |
| `npm run build` | Vite frontend build |
| `npm run tauri build` | Production build (both installers) |
| `cd website/frontend && npm run build` | Website (Next.js) build |

## Configuration

Desktop settings are persisted immediately (no save button) to a JSON config in
the platform app-data directory. The full settings surface is documented in
[docs/UI.md](docs/UI.md#settings). Cloud server configuration variables are in
[website/README.md](website/README.md#configuration).

## Documentation

- **[docs/UI.md](docs/UI.md)** — desktop app UI: pages, settings, components, design system
- **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)** — how the recording pipeline and cloud fit together
- **[docs/AGENTS.md](docs/AGENTS.md)** — agent instructions: commands, conventions, completed work
- **[docs/FEATURES.md](docs/FEATURES.md)** — original feature specifications
- **[docs/PLAN.md](docs/PLAN.md)** — implementation plan & status
- **[docs/CLOUD-VISION.md](docs/CLOUD-VISION.md)** — cloud sharing roadmap
- **[website/README.md](website/README.md)** — cloud server setup, API reference, production deployment
- **Website docs & wiki** — end-user articles at `goprism.studio/docs`

## License

Proprietary. All rights reserved.