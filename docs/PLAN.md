# Clip App — Implementation Plan

**Cross-platform game clipping app** — Tauri 2 + React, platform-native screen capture, hardware encoding, ring-buffer recording, system tray, hotkeys, game detection for metadata, optional moment detection plugins.

---

## 14 Implementation Steps

### Phase 1: Foundation

#### 1. Scaffold Tauri 2 + React Project
- Initialize Tauri 2 + React + TypeScript + Vite project
- Set up Rust project structure with workspace
- Configure TailwindCSS + shadcn/ui for UI
- Set up GitHub Actions CI for Windows, macOS, Linux
- Verify dev build works on all three platforms

#### 2. Platform-Native Screen Capture Modules
Implement as Rust plugins per platform:
- **Windows:** DXGI Desktop Duplication API via `dxgi-rs` or `cap`
- **macOS:** ScreenCaptureKit via `screenpipe` or raw bindings
- **Linux:** PipeWire via `pipewire-rs`
- Common trait interface for frame delivery
- Frame format conversion to RGBA for buffer pipeline

#### 3. Ring-Buffer Recording Engine
- Circular memory buffer (GPU memory where possible)
- Configurable max duration (10s–30min, user setting)
- Framerate-aware frame dropping under load
- Hotkey trigger → flush last N seconds to disk
- Memory pool to avoid allocations during recording

#### 4. Hardware Encoding Integration
Per-platform HW encoder wrappers with codec selection:
- **Windows:** NVENC (NVIDIA) / Media Foundation (AMD/Intel)
- **macOS:** VideoToolbox
- **Linux:** VAAPI / NVENC
- Codec selection: H.264, H.265, AV1 (filtered by available hardware)
- Fallback to software encoding with warning
- Quality presets: Performance → Balanced → Quality
- Configurable bitrate (auto/manual)

### Phase 2: Core App

#### 5. Settings System
- JSON config file stored in platform app data directory
- Rust-side validation (schemas via `serde` + custom validation)
- React settings UI with categories:
  - Recording: buffer duration, FPS, codec, quality, output dir
  - Hotkeys: bindable key chords
  - General: launch on startup, minimize to tray
  - Storage: max disk usage, auto-delete rules
  - Advanced: encoder selection, debug logging

#### 6. System Tray Integration
- Tray icon with recording state indicator
- Context menu: Clip Now, Open Library, Settings, Pause/Resume, Quit
- Minimize to tray on window close (configurable)
- Upload progress notifications in tray
- Background recording indicator (icon badge/color change)

#### 7. Global Hotkey System
- Platform hotkey registration via `rdev`/`global-hotkey` crate
- Default: `Ctrl+Shift+X` (Clip Now)
- Rebindable in settings with conflict detection
- Hotkey capture UI: press chord → app registers it
- Multiple hotkey actions: Clip, Toggle Recording

#### 8. Clip Library UI
- Thumbnail grid (screenshot at mid-point)
- Sort by: date, game, duration, file size
- Filter by game, date range
- Search by filename
- Clip actions:
  - Play in OS default player
  - Reveal in File Explorer / Finder
  - Rename, Delete (confirm/move to trash)
  - Upload / Share button (placeholder)
- Multi-select for batch operations
- Metadata display: game, duration, size, date, codec

### Phase 3: Game Detection & Clip Metadata

#### 9. Foreground Game Detection (Process Watcher)
- Background service that polls the active foreground window
- Cross-platform: platform-native API to get foreground process info
- Returns: process name, window title, timestamp
- Lightweight polling (every 1–2s), <0.1% CPU cost
- No game hooks, no injection, no memory reading — purely read-only OS queries
- Runs automatically when app is active; no user config needed for basic detection

#### 10. Known Games Database + Clip Tagging
- Pre-populated database of common games with process name + window title patterns:
  - Rust (`RustClient.exe`, `Rust`), CS2 (`cs2.exe`), Valorant (`VALORANT.exe`)
  - Minecraft, Fortnite, Apex Legends, Call of Duty, League of Legends, Elden Ring, etc.
  - Community-contributed pattern additions (JSON file, easy to edit)
- When foreground game matches a known game → tag clip metadata with game name
- Game name stored alongside clip (file metadata or sidecar DB)
- Enables: sort/filter clips by game in library, display game icon/name
- Unknown games get tagged as "Unknown" — still recorded, just not identified
- Database is extendable without recompiling (plain JSON/TOML config file)

### Phase 4: Upload & Polish

#### 11. Upload & Share Foundation
- API client module in Rust (`reqwest`-based)
- Upload queue with retry + backoff
- Offline queue: saves task, retries when online
- Upload progress UI (library + tray)
- After upload: copy share link to clipboard
- Configurable server URL in settings
- Auth token management (login/logout)

#### 12. Cross-Platform Performance Profiling
- Measure CPU/GPU overhead during background recording
- Memory usage tracking: buffer sizing optimization
- Disk I/O under game load
- Frame time impact on foreground game
- Optimize ring-buffer and encoding pipeline based on data
- Validate <2% CPU, <1% GPU overhead targets

#### 13. Packaging & Distribution
- Windows: .msi installer (Tauri bundler) + code signing
- macOS: .dmg with notarization
- Linux: .AppImage + .deb
- Auto-updater integration (Tauri built-in)
- App icon, branding, installer customization

### Phase 5: Moment Detection (Optional — Last Priority)

#### 14. Moment Detection Plugin System
- **Optional feature** — disabled by default, user must opt in per game
- Plugin system for game-specific "important moment" detection
- Each game plugin detects events like kills, deaths, objectives, match end
- **Detection methods (per plugin capability):**
  - OCR screen region monitoring (kill feed, death screen text)
  - Audio signature detection (gunshots, explosions)
  - Network packet inspection (future, advanced)
  - No game memory injection — read-only approaches preferred
- **Plugin interface:**
  ```rust
  pub trait MomentDetector: Send + Sync {
      fn game(&self) -> &'static str;
      fn detection_method(&self) -> DetectionMethod;
      async fn check_for_moment(&self, frame: &Frame) -> Result<Option<MomentEvent>>;
  }
  ```
- When moment detected → auto-clip buffer (same as hotkey trigger)
- Configurable cooldown per game (default: 30s between auto-clips)
- Configurable pre-roll / post-roll duration per game
- Ships with **no moment plugins** initially — framework only
- Users/community can author plugins as separate Rust crates or WASM modules
- First-party plugins (Rust OCR, Rust audio) considered post-launch

---

## Architecture Sketch

```
┌────────────────────────────────────────────────────┐
│                  React Frontend                     │
│  ┌─────────┐  ┌──────────┐  ┌──────────────────┐  │
│  │Settings │  │Library   │  │Tray (via Tauri)  │  │
│  │  UI     │  │  UI      │  │ Context Menu     │  │
│  └────┬────┘  └────┬─────┘  └──────────────────┘  │
│       │            │                                │
├───────┴────────────┴────────────────────────────────┤
│                 Tauri IPC Bridge                     │
├───────┬────────────┬────────────────────────────────┤
│       │            │                                │
│  ┌────▼────┐  ┌────▼─────┐  ┌──────────────────┐  │
│  │Settings │  │Clip      │  │Upload API Client │  │
│  │Manager  │  │Manager   │  │ (reqwest)        │  │
│  └─────────┘  └────┬─────┘  └──────────────────┘  │
│                     │                                │
│  ┌──────────────────▼────────────────────────────┐  │
│  │           Recording Engine                     │  │
│  │  ┌────────┐  ┌──────────┐  ┌──────────────┐  │  │
│  │  │Capture │─▶│Ring      │─▶│HW Encoder    │  │  │
│  │  │ Backend│  │Buffer    │  │ + Mux to MP4 │  │  │
│  │  └───┬────┘  └──────────┘  └──────────────┘  │  │
│  │      │                                         │  │
│  │  ┌───▼────┐  ┌──────────┐                     │  │
│  │  │DXGI    │  │Hotkey    │                     │  │
│  │  │Metal   │  │System    │                     │  │
│  │  │PipeWire│  │(rdev)    │                     │  │
│  │  └────────┘  └──────────┘                     │  │
│  └────────────────────────────────────────────────┘  │
│                                                      │
│  ┌──────────────────────────────────────────────┐   │
│  │       Game Detection & Metadata              │   │
│  │  ┌────────────┐  ┌──────────────────────┐   │   │
│  │  │Foreground  │  │Known Games DB        │   │   │
│  │  │Process     │──▶Process names + titles │   │   │
│  │  │Watcher     │  │→ Clip Tagging        │   │   │
│  │  └────────────┘  └──────────────────────┘   │   │
│  └──────────────────────────────────────────────┘   │
│                                                      │
│  Optional: Moment Detection Plugins                  │
│  (step 14, disabled by default)                      │
│                        Rust Backend                  │
└──────────────────────────────────────────────────────┘
```

## Timeline Estimates (Rough)

| Phase | Steps | Est. Time |
|-------|-------|-----------|
| Foundation | 1–4 | 2–3 weeks |
| Core App | 5–8 | 2–3 weeks |
| Game Detection & Metadata | 9–10 | ~1 week |
| Upload + Polish | 11–13 | 1–2 weeks |
| Moment Detection (Optional) | 14 | When ready |
| **Core Total** | **1–13** | **~6–9 weeks** |

## Status Tracking

- [ ] **Step 1** — Scaffold project
- [ ] **Step 2** — Screen capture modules
- [ ] **Step 3** — Ring-buffer engine
- [ ] **Step 4** — Hardware encoding
- [ ] **Step 5** — Settings system
- [ ] **Step 6** — System tray
- [ ] **Step 7** — Global hotkeys
- [ ] **Step 8** — Clip library UI
- [ ] **Step 9** — Foreground game detection (process watcher)
- [ ] **Step 10** — Known games database + clip tagging
- [ ] **Step 11** — Upload & share foundation
- [ ] **Step 12** — Performance profiling
- [ ] **Step 13** — Packaging & distribution
- [ ] **Step 14** *(Optional)* — Moment detection plugin system
