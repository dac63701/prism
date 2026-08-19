# Prism Desktop — UI Documentation

This document describes the desktop application UI: the app shell, every page,
every settings section, the component library, and the design system conventions.
It reflects the current implementation (v0.2.x). For the recording pipeline
behind the UI, see [ARCHITECTURE.md](ARCHITECTURE.md).

## Overview

Prism is a **Tauri 2** app: a Rust backend exposed through typed IPC commands and
a React frontend rendered in the system webview. The frontend lives in `src/`,
the backend in `src-tauri/src/`.

```
src/
├── App.tsx                      # MemoryRouter + routes + ErrorBoundary
├── pages/                       # Home, Library, ClipDetail, Settings
├── components/
│   ├── layout/                  # AppLayout, TitleBar, Sidebar, AmbientBackground
│   ├── common/                  # RecordingControls, ScreenPreview, VideoPlayer, ...
│   ├── settings/                # section components + form primitives
│   ├── auth/                    # SignInPrompt
│   ├── ui/                      # button, dialog, input, switch, slider, tabs, ...
│   └── upload/                  # (upload UI components)
├── stores/                      # Zustand stores (recording, settings, clips, cloud)
├── hooks/                       # useSettingsActions, useDisplayRefreshRate, ...
├── lib/                         # utils, presets, constants
└── types/                       # shared TypeScript types (AppSettings, Clip, ...)
```

## App shell

The shell is `AppLayout` (`src/components/layout/AppLayout.tsx`), rendered by the
root route. It composes:

- **AmbientBackground** — fixed decorative gradient orbs behind everything.
- **TitleBar** — custom window chrome (`decorations: false` on Windows/Linux).
  Shows the Prism logo, current page title (from `useRouteTitle`), and native
  window controls (minimize / maximize / close). On macOS the window keeps native
  decorations with overlay traffic lights and the bar is inset (`pl-20`) to
  avoid the lights. The bar is a drag region (`data-tauri-drag-region="deep"`).
- **Sidebar** — app navigation with `Home`, `Library`, `Settings` items; a live
  `RecordingIndicator`; and an `AccountCard` that shows cloud sign-in state,
  upload queue badge, an account dialog (open dashboard / sign out), and the app
  version. Collapses to a 56px rail below `768px` via a matchMedia hook.
- **ClipNotification** — toast banner after a clip is saved.
- **SignInPrompt** — first-boot modal prompting cloud sign-in; re-asks after a
  3-day cooldown if dismissed.
- **Toaster** — global toast stack (`stores/toast.ts`).

### Global event handling

`AppLayout` registers two Tauri event listeners:

- `menu-action` — tray menu clicks (e.g. `save_clip`, `open_library`,
  `open_settings`).
- `hotkey-pressed` — global hotkey triggers (`save_clip`, `toggle_recording`).

`toggle_recording` toggles the recording store's start/stop. The right-click
context menu is suppressed app-wide.

### Recording status polling

While recording, the layout polls `is_recording` / `get_buffer_info` every 1s so
the buffered-seconds counter stays live on every page.

## Pages

### Home (`/`)

The capture dashboard. Left column: **ScreenPreview** (live NV12→JPEG preview at
~1fps), **RecordingControls**, a status line (elapsed time · buffered seconds or
"Idle"), and badges showing the active capture source, clip length, and
resolution/bitrate/FPS. Right column: **SourceSelector** for picking the capture
target (main display, a specific display, or — on macOS — an application window).

`RecordingControls` has a large circular record/stop button and a "save clip"
button showing the current hotkey in a `<Kbd>` chip. While recording, the record
button glows red with a pulse ring.

### Clip Library (`/library`)

Thumbnail grid of saved clips. Features:

- **Search** (debounced, over filename/title/description/game) with a clear button.
- **Sort** — newest / oldest / name A–Z / largest / longest.
- **Status filter** — all / uploaded / uploading / failed (from the upload queue).
- **Game filter** — dropdown of games present in the library.
- **Open Folder** — reveals the output directory in the OS file manager.

Each `ClipCard` shows a JPEG thumbnail, duration badge, title, game tag, size,
and date. Hovering reveals overlay actions: play, upload (or copy share link once
uploaded), and delete. Upload state is surfaced as a pill badge (Uploaded /
Uploading / Failed) plus an inline progress bar. Deleting opens a confirm dialog.

### Clip Detail (`/clip/:filename`)

- In-app `VideoPlayer` (custom `<video>` element with poster thumbnail).
- A **Clip details** card with editable **name**, **game**, and **description**
  (max lengths enforced). The read view shows stat tiles: game, captured date,
  duration, size, and description.
- Back navigation to the library; header shows size badge and filename.

### Settings (`/settings`)

Settings are persisted **immediately on change** (no save button) — see
[Settings sections](#settings-sections) below. The page uses a tab rail
(vertical on desktop, horizontal chips on mobile) with six sections:
**Recording**, **Hotkeys**, **General**, **Auto-clip**, **Cloud**, **Storage**.

## Settings sections

### Recording

- **Quality preset** — segmented control (`Fast` / `Balanced` / `High`), with
  presets defined in `lib/presets.ts`. Tuning any individual option below flips
  the preset to `Custom`.
- **Video quality** — Resolution (Native / 720p / 1080p / 1440p / 4K), FPS
  (Auto matches display refresh rate, or 24/30/60/120/144), Bitrate
  (1–60 Mbps). A live "Output" summary shows `resolution · fps · Mbps · MB/min`.
- **Clip length** — shadow-buffer duration slider (10s–30min, step 5s), with a
  formatted clock readout.
- **Capture** — Always-on recording toggle; System audio toggle (Windows WASAPI).
- **Storage** — Output directory text input (debounced) + "Open folder" button.
- **Advanced** — collapsible; shows the current capture source.
- **Danger zone** — Reset recording settings (confirm dialog).

### Hotkeys

Three rebindable shortcuts using `HotkeyCaptureInput` (press the chord; the input
records it live): **Save clip**, **Toggle recording**, **Open library**. A "Reset
to defaults" button restores platform defaults (`Cmd/Ctrl+Shift+X/R/L`). Changes
re-register the global hotkeys immediately via `validate_hotkey` +
`update_settings`.

### General

- Launch at startup
- Minimize to tray (closing the window hides to tray instead of quitting)
- Show clip notification (native toast after a clip is saved)
- Game detection (powers auto-clipping + library game tags)
- CS2 GSI port (restart required; writes CS2's `gamestate_integration_prism.cfg`)

### Auto-clip

Automatic highlight capture driven by game events. Master toggle + clip cooldown
(5–120s) + Rust audio sensitivity slider. Per-game cards:

- **Counter-Strike 2** — events: kills, deaths, headshots, round wins. Detection
  uses Valve's official **Game State Integration** (localhost HTTP).
- **Rust** — events: gunfights, headshot dings, rockets/C4 (explosions). Detection
  reads final process audio through Windows **WASAPI**.

Each game card has per-event toggles, clip durations (kill/death/combat, 5–120s),
and an audio toggle for Rust. A "Detected / Waiting" badge reflects the live game
detector. Clips triggered by events appear in the library like manual clips.

### Cloud

- **Server URL** — where clips are uploaded (default `https://goprism.studio`).
- **Account** — sign in with Google via browser OAuth + `prism://` deep-link
  callback; manual auth-code paste fallback; sign out.
- **Auto-upload** — automatically upload every saved clip.
- **Concurrent uploads** — 1 (sequential) / 2 / 3 parallel uploads.

### Storage

- **Max clips (GB)** — local library cap (`0` = unlimited).
- **Auto-prune (days)** — automatically delete clips older than N days
  (empty = disabled).

## Component library (`src/components/ui`)

Primitives following shadcn conventions (dark-only, Tailwind):

| Component | Purpose |
|-----------|---------|
| `button` | 7 variants: `default`, `outline`, `secondary`, `ghost`, `destructive`, `link`, `brand` |
| `dialog` | Modal with title/description/footer |
| `input` / `select` / `slider` / `switch` | Form controls |
| `tabs` | Vertical + horizontal tab rails |
| `toast` | Global toast stack + `Toaster` |
| `tooltip` | Hover tooltips |
| `kbd` | Keyboard-key chip |
| `skeleton` | `SkeletonClipsGrid` etc. loading states |
| `empty-state` | Empty-state illustration + action |
| `brand` | `Card` (rounded-3xl gradient) and `Badge` |

## Design system

- **Dark theme only.** App background `#050816`, surfaces `#0b1424`/`#07101f`,
  borders use `border-border` (`#1f2a44`).
- Text scale: `zinc-100`/`white` headings, `zinc-400/500` body/muted,
  `zinc-600` for hints; accent blue (`blue-300`/`#4f8cff`).
- **Interactions**: interactive elements use `transition` (not
  `transition-colors`) so transforms/filters animate; press feedback via
  `active:scale-[0.98]` / `active:scale-95`; clip cards `hover:scale-[1.02]`.
- **Focus rings**: always `focus-visible:ring-2 focus-visible:ring-blue-500/20
  focus-visible:border-blue-400/70`.
- **Icons**: `lucide-react`, consistently `size-4`/`size-3.5`, wrapped `shrink-0`.
- **Passive elements** (inputs, toggles) use `transition-colors`.
- **Cards**: use `<Card>` from `@/components/ui/brand` for large content sections;
  `<Panel>` / `SettingCard` for compact sections.
- **Typography**: Geist variable font; headings `tracking-tight`, uppercase
  eyebrow labels with `tracking-[0.28em]`.
- **Motion**: `animate-fade-up` on settings section swaps, `animate-pulse-ring`
  while recording, `tw-animate-css` powers keyframes.

## Stores (Zustand)

| Store | Holds | Notes |
|-------|-------|-------|
| `settings` | `AppSettings` + load/update/reset | Listens to `settings-changed` events |
| `recording` | recording state, buffer info, save actions | Polls buffer info |
| `clips` | clip library + list/detail operations | CRUD via IPC |
| `cloud` | auth status, uploads queue, upload actions | `useUploadQueue` processor |
| `toast` | toast stack | UI only |

## IPC surface

All backend commands are registered in `src-tauri/src/lib.rs` via
`tauri::generate_handler!`. Frontend calls them through `invoke()` from
`@tauri-apps/api/core`. Key groups:

- **Recording** — `start_recording`, `stop_recording`, `is_recording`,
  `save_clip`, `get_preview_frame`, `get_buffer_info`, `get_capture_sources`,
  `get_display_refresh_rate`, `set_capture_target`
- **Library** — `list_clips`, `delete_clip`, `rename_clip`, `update_clip_metadata`,
  `open_clip_location`
- **Settings** — `get_settings`, `update_settings`, `reset_settings`,
  `validate_hotkey`
- **Auth** — `cloud_login`, `cloud_logout`, `get_auth_status`,
  `cloud_handle_auth_code`, `cloud_verify_auth`
- **Uploads** — `upload_clip`, `upload_queue_status`, `cancel_upload`,
  `retry_upload`
- **Games** — `get_detected_game`

Events emitted from Rust to the frontend: `settings-changed`,
`recording-state-changed`, `menu-action`, `hotkey-pressed`, `game-detected`,
`game-lost`, `clip-saved`, `auth-error`, upload progress events.