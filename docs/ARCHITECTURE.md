# Prism — Architecture

This document describes how Prism actually fits together today: the desktop
recording pipeline, the cloud server, and the data flow between them. For the
user-facing UI see [UI.md](UI.md); for the original feature spec see
[FEATURES.md](FEATURES.md).

## High-level data flow

```
  Display ──▶ DXGI / ScreenCaptureKit ──▶ BGRA frame
                                              │  bgra_to_nv12() (2.7× memory savings)
                                              ▼
                                      NV12 frame
                                     /          \
                        H.264 encoder         fallback (no encoder / encode error)
                            │                       │
              EncodedPacket (compressed)   raw NV12 frame
                            │                       │
                            ▼                       ▼
              Shadow buffer (ring buffer, 256 MB budget, byte-accounted)
                            │
                    trigger clip (hotkey / tray / auto-clip)
                            ▼
                    save_clip → mux H.264 AVCC → MP4 (mp4 crate, no re-encode)
                            │
                            ├── NV12→RGB→JPEG thumbnail (clipname_thumb.jpg)
                            └── upload queue → Prism Cloud API → PostgreSQL + /data
```

## Desktop — Rust backend (`src-tauri/src`)

### Capture (`capture/`)

Platform capture backends behind a common `Capture` trait (`capture/mod.rs`).

- **Windows** (`capture/windows/mod.rs`) — `IDXGIOutputDuplication` with
  `AcquireNextFrame(0)` non-blocking polling. A D3D11 staging texture is reused
  across frames; a fast-path single `memcpy` is used when GPU row pitch matches
  the destination stride. Frames are converted **BGRA→NV12** immediately after
  the GPU→CPU copy via `bgra_to_nv12()`, giving hardware encoders native input.
- **macOS** (`capture/macos/mod.rs`) — ScreenCaptureKit `SCStream`. The callback
  is **FPS-limited** to the configured capture rate (30/60) so high-refresh
  displays (144Hz) don't churn memory. A `LatestFrame` slot holds the most recent
  frame; the polling consumer drains it.
- **Linux** (`capture/linux/mod.rs`) — PipeWire stub (not yet wired).

### Encoding (`encoder/`)

- **Windows** (`encoder/windows/mf_encoder.rs`) — Media Foundation H.264 encoder
  MFT. Supports **sync hardware** (Intel QSV, MS software) and **async hardware**
  (NVIDIA/AMD) MFTs driven Chromium-style through `IMFMediaEventGenerator` with
  D3D11 texture input (`MFCreateDXGISurfaceBuffer` + a recycled NV12 texture
  pool). Encoder selection enumerates hardware MFT candidates, matches each MFT's
  `MFT_ENUM_ADAPTER_LUID` to a DXGI adapter, and falls back to the MS software
  encoder when hardware can't negotiate (e.g. AMD on some drivers).
- **macOS** (`encoder/macos/vt_encoder.rs`) — VideoToolbox `VtH264Encoder` with
  BGRA IOSurface input; NV12→BGRA conversion + resize happens inside
  `encode_frame()`. SPS/PPS are extracted from the VT format description on the
  first keyframe.
- **Software fallback** — `encoder/codecs/` and the encoder factory in
  `encoder/mod.rs`; the recorder also has a raw-NV12 ring path if encoding
  fails entirely.

### Shadow buffer (`buffer/`)

- Byte-accounted `VecDeque` with a **256 MB budget** (`buffer/ring.rs`). Oldest
  frames auto-evict when the budget is exceeded.
  - Compressed H.264 path: ~10 KB/packet → roughly **7 minutes** of 1080p.
  - Raw NV12 fallback path: ~3 MB/frame at 1080p → ~82 frames before eviction.
- Stored frames are `StoredFrame` with `PixelFormat::H264` (or NV12) and an
  `is_sync` keyframe flag plus per-packet timestamps.

### Recording pipeline (`recording/mod.rs`)

`Recorder` owns `Mutex<Option<RecorderInner>>` (a `parking_lot::Mutex`). The
polling loop (`poll_and_push`) runs in **three phases** to avoid holding the lock
during encode:

1. **Phase 1 (brief lock)** — read the latest frame, take the encoder via
   `Option::take()`, clone metadata, drop the lock.
2. **Phase 2 (no lock)** — H.264 encode (the expensive part) runs without
   blocking the async runtime.
3. **Phase 3 (brief lock)** — restore the encoder state and push the encoded
   packets into the ring buffer.

Clip save extracts frames from the ring buffer under lock, then **encodes to MP4
outside the lock** (~0.1s). SPS/PPS captured from the encoder's first keyframe
are cached in `RecorderInner` and prepended if the original keyframe was evicted.

### Audio (`audio/`)

- **Windows WASAPI** loopback capture of system audio → AAC for clips
  (`audio/mod.rs`, `audio/aac.rs`). Gated by the "System audio" setting.

### Thumbnails

Server-side at clip-save time: a representative frame is converted
NV12→RGB→JPEG and written as `clipname_thumb.jpg` next to the MP4. The frontend
loads it with `<img>` and only falls back to a `<video>` capture on image error.

### Game detection & auto-clip (`games/`)

- **`GameDetector`** polls every 5s; on Windows it enumerates visible windows and
  matches titles against the `GameRegistry` database (`games/database/`). Emits
  `game-detected` / `game-lost` events.
- **CS2 (`games/cs2/`)** — listens on the official Valve **Game State
  Integration** API. `ensure_gsi_config` writes
  `gamestate_integration_prism.cfg` into the CS2 cfg directory (if found) and a
  tiny HTTP listener parses round/kill/death/headshot/win events.
- **Rust (`games/rust/`)** — Windows-only; captures the game's **final process
  audio via WASAPI**, runs FFT-based analysis (`analyzer.rs`) to detect gunshots,
  headshot dings, explosions, and combat, and triggers clips accordingly.
- **`trigger.rs` (`AutoClipTrigger`)** — decides when a detected event becomes a
  saved clip (cooldown, per-game clip durations) and reuses the same save path as
  a manual clip.

All detection is **read-only and anti-cheat-safe**: no process injection, no
memory reading — documented OS APIs and localhost HTTP only.

### Upload (`upload/`)

- `queue.rs` — persistent upload queue (`UploadQueue`) with retry/backoff (2
  retries), resume after restart, cleanup of completed tasks.
- `client.rs` — multipart upload to `POST /api/clips/upload` using the cloud JWT
  `access_token` (switched from API-key auth). Manual multipart body construction
  for axum 0.8 / multer 3.x compatibility.
- A background tokio task (`start_upload_processor`) drains the queue; progress
  is surfaced to the frontend via events.

### Auth (`auth/`)

OAuth sign-in via browser + `prism://` deep-link. `AuthManager` handles
`prism://auth/callback?code=…` (cold-start argv, single-instance plugin routing,
or the `deep-link` event), exchanges the code for JWT tokens, and persists them.
`cloud_verify_auth` checks server-side that the stored key is valid; after a
fresh sign-in the auth state is driven by the `auth-state-changed` event.

### Settings (`settings/`)

- `config.rs` — typed `AppSettings` (recording, hotkeys, general, storage, cloud,
  auto_clip) with serde defaults.
- `store.rs` — JSON file in the platform app-data directory; writes are
  immediate. Settings changes emit `settings-changed` for the frontend.

### Tray & hotkeys

- `tray/mod.rs` — tray icon + menu (Save Clip / Open Library / Settings / Quit);
  left-click shows/focuses the window; closing the window hides to tray when
  `minimize_to_tray` is on.
- `hotkey/mod.rs` — wraps `tauri-plugin-global-shortcut`; registers
  save-clip / toggle-recording / open-library bindings from settings and emits
  `hotkey-pressed`. Bindings re-register immediately on change.

### Notifications (`notification.rs`)

Windows toast AUMID self-healing registration so native notifications render
regardless of launch path; macOS requests Notification Center permission at
startup.

### Runtime & packaging

- `mimalloc` global allocator on Windows for the per-frame allocation churn.
- Single-instance plugin routes `prism://` deep links to the running instance.
- Tauri window: frameless custom title bar (Windows/Linux), overlay traffic
  lights on macOS. Asset protocol scoped to `$HOME/Videos/…` for thumbnails.

## Desktop — frontend

See [UI.md](UI.md). Key points: React + Zustand, MemoryRouter routes, typed
`invoke()` IPC, and Tauri event listeners (`menu-action`, `hotkey-pressed`,
`settings-changed`, `recording-state-changed`, `game-detected`, upload events).

## Cloud server (`website/`)

```
Browser ──HTTPS──▶ Nginx ──▶ web (Next.js, port 3000)
                        │
                        └── /api/* ──▶ api (Axum, port 8080) ──▶ PostgreSQL
                                                                   │
                                                                   └── /data/clips/ (local storage)
```

- **API (`website/src`)** — Axum 0.8 + SQLx (Postgres). Auth (register/login/
  refresh, JWT 15m access + 30d refresh, Google OAuth, API keys hashed with
  SHA-256 for the desktop app), clip CRUD + upload (multipart, multer), tags,
  share-ID regeneration, media serving, admin endpoints (users, stats, config),
  rate limiting via token bucket.
- **Web (`website/frontend`)** — Next.js 15. Dashboard (clip grid, detail,
  settings), admin panel, public player pages (`/s/[shareId]`), public profiles
  (`/u/[username]`), landing/features/docs/download pages, lazy-loaded with
  skeleton loading states.
- **Storage** — local filesystem under `STORAGE_PATH` (`/data`), with per-user
  quota enforcement (`DEFAULT_MAX_STORAGE_GB`).
- **Deploy** — Docker Compose (`docker-compose.prod.yml` uses pinned `@sha256:`
  image digests for Portainer), nginx reverse proxy, GitHub Actions CI that
  builds/publishes images. Details in [website/README.md](../website/README.md).

### Cloud API surface (abridged)

| Group | Endpoints |
|-------|-----------|
| Auth | `POST /api/auth/register` `login` `refresh`, `GET /api/auth/me`, `change-password`, `update-profile` |
| API keys | `GET/POST /api/auth/api-keys`, `DELETE /api/auth/api-keys/{id}` |
| Clips | `POST /api/clips/upload`, `GET /api/clips`, `GET/PATCH/DELETE /api/clips/{id}`, `POST /api/clips/{id}/regenerate-share`, tags routes |
| Public | `GET /s/{shareId}` (HTML + OG), `GET /api/s/{shareId}/meta`, `GET /api/media/{*path}` |
| Admin | `GET /api/admin/users`, `users/{id}`, `stats`, `clips`, `logs`, `config` (GET/PUT) |
| Health | `GET /api/health` |

## Key invariants

- **Recording cost must stay low**: the shadow buffer holds compressed H.264
  packets, not raw frames; encode runs off the async runtime's critical path.
- **Clip save never re-encodes** — packets are muxed directly; SPS/PPS are cached
  so an evicted keyframe can't break a clip.
- **Settings persist immediately** and are validated server-side on upload.
- **Game detection is external to game processes** — no injection, no memory
  reads.