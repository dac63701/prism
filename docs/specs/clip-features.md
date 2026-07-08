# Spec: clip-features

Scope: feature

# Clip App — Feature Specifications

## 1. Recording Engine

### Ring Buffer
- Circular memory buffer storing raw frames from screen capture
- Configurable max duration (10s to 30min, default 60s)
- Framerate-aware frame dropping when system under load
- Buffer is always writing; when clip is triggered, last N seconds are saved
- **Critical:** Zero-copy or minimal-copy design — frames written directly to GPU memory where possible

### Platform Screen Capture
| Platform | API | Requirements |
|----------|-----|-------------|
| Windows | DXGI Desktop Duplication API | Windows 8+, recommended GPU |
| macOS | ScreenCaptureKit | macOS 10.15+ (Ventura better) |
| Linux | PipeWire + portal | Recent compositor (GNOME/KDE/Wayland) |

### Hardware Encoding
| Codec | Windows | macOS | Linux |
|-------|---------|-------|-------|
| H.264 | NVENC/Media Foundation | VideoToolbox | VAAPI/NVENC |
| H.265 | NVENC/Media Foundation | VideoToolbox | VAAPI/NVENC |
| AV1 | NVENC (RTX 40+) | — | Intel Arc + VAAPI |

- Codec selection in settings, filtered by available hardware
- Fallback to software encoding if no HW encoder available (show warning)
- Encoding quality presets: Performance → Balanced → Quality
- Configurable bitrate (auto/manual)

### Recording Modes
- **Always-on buffer:** Starts with app, minimal overhead (~0.5-2% CPU target)
- **Manual recording:** User can also start/stop recording on demand
- **Clip-only:** Just the buffer, no full recording (default mode)

## 2. System Tray
- Icon in system tray while recording
- Context menu:
  - "Clip Now" (trigger clip)
  - "Open Library"
  - "Settings"
  - "Pause/Resume Recording"
  - "Quit"
- Minimize to tray on window close (configurable)
- Visual indicator (icon change) when recording/paused
- Upload progress notification in tray

## 3. Global Hotkeys
- **Default:** `Ctrl+Shift+X` (clip now)
- All hotkeys rebindable in settings with conflict detection
- Hotkey detection: press a key chord, app captures it
- System must handle:
  - Hotkey registration across all platforms
  - Showing error if hotkey conflicts with another app
  - Multiple hotkey actions (Clip, Toggle Recording, Push-to-Talk placeholder)

## 4. Settings UI
Structured in categories:

### Recording
- Buffer duration (slider: 10s–30min, step 5s)
- Recording FPS (24/30/60)
- Video codec (H.264 / H.265 / AV1)
- Quality preset (Performance / Balanced / Quality)
- Output directory (folder picker)
- Output format (MP4 default)

### Hotkeys
- Clip Now hotkey (rebindable)
- toggle Recording hotkey
- Reset to defaults button

### General
- Launch on startup (toggle)
- Minimize to tray on close (toggle)
- Show clip notification after saving (toggle)
- Game detection enabled (toggle)

### Storage
- Max disk usage (auto-cleanup oldest clips)
- Auto-delete clips older than N days (optional)
- Upload bandwidth limit (placeholder)

### Advanced
- Debug logging
- Hardware encoder selection (auto/manual pick)
- Frame dropping aggressiveness

## 5. Clip Library
- Thumbnail grid view (screenshot at clip mid-point)
- Sort by: date (default), game, duration, size
- Filter by game, date range
- Search by filename
- Clip actions:
  - Play in external player (default OS player)
  - Open file location (reveal in Finder/Explorer)
  - Rename
  - Delete (with confirmation, or move to trash)
  - Upload / Share (button → triggers upload flow)
- Multi-select for batch operations
- Clip metadata: filename, game name, duration, file size, date recorded, codec info

## 6. Game Detection Plugin System

### Plugin Interface (Rust trait)
```rust
#[async_trait]
pub trait GameDetector: Send + Sync {
    fn name(&self) -> &'static str;
    fn supported_games(&self) -> Vec<&'static str>;
    async fn detect(&self) -> Result<Option<GameEvent>>;
    async fn initialize(&self) -> Result<()>;
}
```

### Detection Methods (per plugin)
- **Process name match:** Check foreground window process name
- **Window title match:** Regex-based window title matching
- **OCR-based:** (future) Screen OCR for kill feeds, death screens
- **Audio trigger:** (future) Sound signature detection

### Auto-Clipping Rules
- Per-game: "When Rust is detected, auto-clip on kill feed detected"
- Configurable cooldown (prevent spam: min N seconds between auto-clips)
- Rules stored in settings alongside game detector config
- Default: off, user enables per-game

## 7. Rust Game Detection (Phase 1)
- Process name: `RustClient.exe` (Win) / `Rust` (Mac) / `rust` (Linux)
- Window title contains "Rust"
- No memory reading, no injection
- Future: OCR-based kill feed detection
- When Rust is detected AND auto-clip enabled, register additional hotkeys or triggers

## 8. Upload & Sharing (Desktop Client)
- Upload queue (sequential, parallel configurable)
- Upload progress bar in library & tray
- Retry on failure with backoff
- Offline queue — saves upload task, retries when online
- After upload: copy share link to clipboard automatically
- Configurable server URL in settings
- Auth token management

## 9. Performance Requirements
- **Background buffer recording:** <2% CPU on modern CPU (2020+), <1% on GPU
- **Memory:** Buffer of 60s @ 1080p60 ≈ ~1-2GB VRAM/system RAM (configurable)
- **Encoding:** Should not increase game frame times by more than 1-2ms
- **Startup:** Load to tray in <3 seconds
- **Clip export:** Saving a 60s clip should take <2 seconds (HW encode)
- **App cold start:** <5 seconds to tray-ready state

## 10. Distribution
- Windows: .msi installer (Tauri bundler)
- macOS: .dmg with notarization
- Linux: .AppImage + .deb
- Auto-updater (Tauri built-in)
- Code signing for Windows and macOS