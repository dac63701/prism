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
| H.264 | NVENC / Media Foundation | VideoToolbox | VAAPI / NVENC |
| H.265/HEVC | NVENC / Media Foundation | VideoToolbox | VAAPI / NVENC |
| AV1 | NVENC (RTX 40+ series) | — | Intel Arc + VAAPI |

- Codec selection in settings, filtered by available hardware on startup
- Fallback to software encoding if no HW encoder available (show warning badge)
- Encoding quality presets: **Performance** → **Balanced** → **Quality**
- Configurable bitrate: Auto (recommended) or Manual (kbps slider)

### Recording Modes
- **Always-on buffer:** Starts with app, minimal overhead (~0.5–2% CPU target)
- **Manual recording:** User can also start/stop full recording on demand
- **Clip-only:** Just the buffer, no full recording (default mode)

---

## 2. System Tray

- Icon in system tray while app is running
- **Context menu:**
  - "Clip Now" — triggers clip with current buffer
  - "Open Library" — shows clip library window
  - "Settings" — opens settings
  - "Pause / Resume Recording" — toggles buffer recording
  - "Quit" — exits app
- Minimize to tray on window close (configurable in settings)
- Visual indicator when recording is paused (icon change)
- Upload progress badge/notification in tray
- Click tray icon to toggle window visibility

---

## 3. Global Hotkeys

| Action | Default Binding |
|--------|----------------|
| Clip Now | `Ctrl + Shift + X` |
| Toggle Recording | (unbound by default) |

- All hotkeys rebindable in settings
- **Rebinding UX:** press a key chord → app captures and displays it
- **Conflict detection:** warn if binding matches system or another app's hotkey
- Per-action: can be unbound (set to "None")

---

## 4. Settings UI

Structured in categorized tabs or sections:

### Recording
| Setting | Type | Default | Range |
|---------|------|---------|-------|
| Buffer Duration | Slider | 60s | 10s – 30min (step 5s) |
| Recording FPS | Select | 60 | 24 / 30 / 60 |
| Video Codec | Select | H.264 | H.264 / H.265 / AV1 |
| Quality Preset | Select | Balanced | Performance / Balanced / Quality |
| Output Directory | Folder Picker | `~/Videos/Clips/` | — |
| Output Format | Select | MP4 | MP4 |

### Hotkeys
- "Clip Now" — rebind button
- "Toggle Recording" — rebind button
- "Reset to Defaults" button

### General
| Setting | Type | Default |
|---------|------|---------|
| Launch on startup | Toggle | Off |
| Minimize to tray on close | Toggle | On |
| Show notification after clip | Toggle | On |
| Game detection enabled | Toggle | Off |

### Storage
| Setting | Type | Default |
|---------|------|---------|
| Max disk usage | Number (GB) | 50 GB |
| Auto-delete clips older than | Select | Never / 7d / 30d / 90d |
| Upload bandwidth limit | Number (Mbps) | 0 (unlimited) |

### Advanced
| Setting | Type | Default |
|---------|------|---------|
| Debug logging | Toggle | Off |
| Hardware encoder | Select | Auto / Manual pick |
| Frame dropping aggressiveness | Slider (1-5) | 3 |

---

## 5. Clip Library

### Views
- **Thumbnail grid** — screenshot at clip mid-point
- **List view** — compact rows with metadata

### Sorting & Filtering
- Sort by: Date recorded (default), Game, Duration, File Size
- Filter by game (dropdown of detected games)
- Filter by date range (from/to picker)
- Search by filename (text input)

### Clip Actions
| Action | Behavior |
|--------|----------|
| Play | Opens clip in OS default video player |
| Open File Location | Reveals in Finder / File Explorer |
| Rename | Inline rename, updates file on disk |
| Delete | Moves to trash (confirm dialog) |
| Upload | Triggers upload to configured server |
| Copy Share Link | Available after upload |

### Multi-Select
- Shift/Cmd+click to select multiple clips
- Batch delete, batch upload
- Select all / deselect all

### Clip Metadata Display
- Filename
- Game name (if detected)
- Duration (mm:ss)
- File size (MB)
- Date recorded
- Codec used
- Upload status (Not Uploaded / Uploading / Uploaded ✅)

---

## 6. Game Detection & Clip Metadata

### Foreground Process Watcher
- Background service that constantly checks which window is in the foreground
- Platform APIs:
  - **Windows:** `GetForegroundWindow()` + `GetWindowThreadProcessId()` → process name
  - **macOS:** `NSWorkspace.shared.frontmostApplication` → bundle/process name
  - **Linux:** `XGetInputFocus()` / `kwindowinfo` (Wayland) → window class
- Polling interval: every 1–2 seconds, very lightweight
- Returns: process name + window title string
- Runs automatically from app start — no user interaction needed

### Known Games Database
- Pre-populated JSON/TOML file bundled with app:

```json
{
  "games": [
    {
      "name": "Rust",
      "process_names": ["RustClient.exe", "Rust", "rust"],
      "window_title_patterns": ["Rust"],
      "icon": "rust"
    },
    {
      "name": "CS2",
      "process_names": ["cs2.exe", "cs2"],
      "window_title_patterns": ["Counter-Strike 2"],
      "icon": "cs2"
    },
    {
      "name": "Valorant",
      "process_names": ["VALORANT.exe", "VALORANT"],
      "window_title_patterns": ["VALORANT"],
      "icon": "valorant"
    },
    {
      "name": "Apex Legends",
      "process_names": ["r5apex.exe", "Apex Legends"],
      "window_title_patterns": ["Apex Legends"],
      "icon": "apex"
    }
  ]
}
```

- Target: **50+ common games** pre-configured at launch
- Community-contributed additions via PR to the games JSON
- Database is extendable without recompiling — user can edit the JSON file

### Game Database (Small Extract)

| Game | Process Names | Window Title Match |
|------|--------------|-------------------|
| Rust | `RustClient.exe`, `Rust`, `rust` | "Rust" |
| CS2 | `cs2.exe`, `cs2` | "Counter-Strike 2" |
| Valorant | `VALORANT.exe`, `VALORANT` | "VALORANT" |
| Apex Legends | `r5apex.exe`, `Apex Legends` | "Apex Legends" |
| Minecraft | `javaw.exe` | "Minecraft" |
| Fortnite | `FortniteClient-Win64-Shipping.exe` | "Fortnite" |
| Call of Duty | `cod.exe` | "Call of Duty" |
| League of Legends | `LeagueClient.exe` | "League of Legends" |
| Elden Ring | `eldenring.exe` | "ELDEN RING" |
| GTA V | `GTA5.exe` | "Grand Theft Auto V" |
| Overwatch 2 | `Overwatch.exe` | "Overwatch" |
| Rainbow Six Siege | `RainbowSix.exe` | "Rainbow Six" |

### Clip Tagging Flow
1. Foreground watcher detects active game → matches against games database
2. On clip trigger (hotkey or manual record):
   - Current game name is embedded in clip metadata
   - Saved alongside clip: `{ file, game, timestamp, duration, codec }`
   - File naming convention: `{GameName}_{YYYY-MM-DD}_{HHmmss}.mp4`
3. Clip library uses game tag for sorting, filtering, displaying game icon

### Settings
| Setting | Type | Default | Notes |
|---------|------|---------|-------|
| Game detection enabled | Toggle | On | Lightweight, recommended on |
| Custom game patterns | Text area | — | Add your own process/title patterns |

---

## 7. Moment Detection Plugin System *(Optional)*

> **Status:** Phase 5 — last priority, disabled by default, opt-in per game.

Moment detection is a separate plugin system that can detect specific in-game events (kills, deaths, objectives) and auto-save clips. It is **not required** for the core clipping experience.

### Design Philosophy
- **Opt-in:** User must enable per game — nothing runs without consent
- **Framework-first:** Ships with the plugin system interface only, zero moment plugins included initially
- **Game-specific:** Each game needs its own plugin tailored to that game's UI
- **Read-only approaches preferred:** OCR over memory reading, audio signatures over packet inspection

### Plugin Interface
```rust
/// Plugin that detects "important moments" in a specific game.
/// Runs alongside the recording engine and can trigger auto-clips.
pub trait MomentDetector: Send + Sync {
    /// Game this detector targets (must match a game in the games database)
    fn game(&self) -> &'static str;

    /// What detection method this plugin uses
    fn detection_method(&self) -> DetectionMethod;

    /// Check the current frame/buffer for a moment event.
    /// Called on a configurable interval (not every frame).
    async fn check_for_moment(&self, state: &MomentDetectionState) -> Result<Option<MomentEvent>>;
}

pub enum DetectionMethod {
    Ocr,         // Screen region OCR (kill feed, death screen)
    Audio,       // Audio signature matching (gunshots, explosions)
    Network,     // Packet inspection (advanced, future)
    Hybrid,      // Combination of methods
}

pub struct MomentEvent {
    pub event_type: MomentEventType,
    pub confidence: f32,        // 0.0–1.0
    pub timestamp: std::time::Instant,
}

pub enum MomentEventType {
    Kill,
    Death,
    Headshot,
    ObjectiveCaptured,
    MatchStart,
    MatchEnd,
    MultiKill(u32),       // 2 for double, 3 for triple, etc.
}

/// Lightweight state passed to detectors
pub struct MomentDetectionState {
    pub latest_frame: Arc<Vec<u8>>,       // Current screen capture frame
    pub frame_metadata: FrameMetadata,     // Resolution, timestamp
    pub audio_buffer: Option<Arc<Vec<f32>>>, // Recent audio samples (if enabled)
}
```

### Auto-Clip Behavior
- When a moment is detected → same as hotkey trigger: save last N seconds from ring buffer
- **Cooldown:** Min seconds between auto-clips (default: 30s, configurable)
- **Pre-roll:** How much buffer before the moment to include (default: 5s)
- **Post-roll:** How much recording after the moment to include (default: 3s)
- Clips tagged with `auto: true` + `moment_type` in metadata

### Detection Methods (Per Plugin)

| Method | Accuracy | CPU Cost | Example Use |
|--------|----------|----------|-------------|
| **OCR** (screen region) | High | Low-moderate | Reading kill feed text "You killed PlayerX" |
| **Pixel color change** | Medium | Very low | Detecting death screen overlay |
| **Audio signature** | Medium | Low | Gunshot sound detection |
| **Network** (future) | High | Moderate | Reading game packets (advanced) |

### Plugin Loading
- Plugins as separate Rust crates compiled alongside app
- Future: WASM-based plugins for third-party/community plugins (sandboxed)
- Plugin manifest declares which game it targets + detection method
- Registry: built-in list of available plugins, user enables per game

### First-Party Plugin Roadmap
| Game | Detection Method | Complexity |
|------|-----------------|------------|
| Rust | OCR (kill feed region) | Medium |
| CS2 | OCR (kill feed) + audio (gunshots) | Medium |
| Valorant | OCR (kill feed) | Medium |
| Apex Legends | OCR (kill feed + banner) | Medium-High |
| Call of Duty | OCR (kill feed) | Medium |

### Performance Considerations
- **Disabled by default** — zero CPU cost until user enables a plugin
- OCR runs on a timer (every 200–500ms), not every frame
- Audio detection runs on a separate low-priority thread
- If CPU impact exceeds threshold → auto-pause detection, notify user

---

## 8. Upload & Sharing (Desktop Client)

### Upload Queue
- Sequential upload (one at a time) by default
- Parallel upload configurable (up to 3)
- Progress bar per clip in library view
- Tray progress indicator

### Reliability
- **Retry on failure** — exponential backoff (3 retries)
- **Offline queue** — saves pending uploads to disk, retries when connection resumes
- Detection of network availability

### Share Flow
1. User clicks Upload on a clip
2. Clip is queued → encoded/compressed if needed → uploaded
3. On completion: share link is copied to clipboard automatically
4. Toast notification: "Uploaded! Link copied to clipboard"
5. Clip metadata in library updates to show share link

### Configuration
- Server URL (text input in settings)
- Auth token / API key management (login/logout in settings)

---

## 9. Performance Requirements

| Metric | Target |
|--------|--------|
| Background buffer recording CPU | <2% on modern CPU (2020+) |
| Background GPU overhead | <1% |
| 60s @ 1080p60 buffer memory | ~1–2 GB (configurable) |
| Encoding latency impact | <1–2 ms added to game frame time |
| Startup to tray-ready | <3 seconds |
| Clip export (60s → MP4) | <2 seconds (HW encode) |
| App cold start | <5 seconds to tray-ready |
| Clip library load (1000 clips) | <1 second |
| Installer size | <100 MB |

---

## 10. Distribution

| Platform | Format | Extra |
|----------|--------|-------|
| Windows | `.msi` installer | Code signing (EV cert) |
| macOS | `.dmg` | Notarization, Apple Developer Program |
| Linux | `.AppImage` + `.deb` | FUSE-free AppImage preferred |

### Auto-Updater
- Tauri built-in updater
- Checks on startup + periodic (configurable)
- Delta updates where possible
- Release notes shown before update

---

## Future Features (Post-MVP)
- [ ] Moment detection plugins (Rust OCR, CS2, Valorant, etc.)
- [ ] Video trimmer (in-app clip trimming)
- [ ] Clip merging / compilation
- [ ] GIF export
- [ ] Push-to-Talk overlay
- [ ] Replay buffer (death cam style)
- [ ] Share to Discord / Twitter directly
- [ ] Multi-monitor selection
- [ ] Vulkan capture support (Linux)
- [ ] WASM-based third-party moment detector plugins
