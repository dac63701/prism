# Prism Desktop UI Rewrite — Feature Specification

**Version:** 1.0 · **Status:** Draft · **App version:** 0.2.5 (Windows-first, macOS in CI)

## 1. Overview

Rewrite the Prism desktop frontend (`src/`) for a modern, premium feel while keeping all Rust IPC contracts, stores, and event flows unchanged. Refine the existing blue-on-deep-navy identity rather than rebranding.

### 1.1 Goals
- **Modern/premium** — frameless custom title bar, cohesive tokens, consistent motion.
- **Snappy** — fewer re-renders, lazy thumbnails, content-visibility, GPU-only animation, reduced IPC chatter.
- **Responsive** — usable at the 800×600 default and at any larger size; Settings becomes tabbed instead of one 750-line scroll page.
- **Consistent** — one shared design system across desktop and the existing website (`website/frontend`).

### 1.2 Non-goals
- No changes to Rust recording/capture/upload logic.
- No new IPC commands (except none needed) — reuse existing `invoke("...")` handlers.
- No light mode, no clip multi-select, no in-app trimming, no command palette in v1.0 (stretch).

---

## 2. Design System

### 2.1 Tokens (`src/index.css`, `@theme`)
Keep existing brand tokens and add the missing semantic set so the shadcn base-nova `Button` variants resolve correctly.

| Token | Value | Notes |
|---|---|---|
| `--color-bg` | `#050816` | page background |
| `--color-surface` | `#0b1222` | panel background |
| `--color-surface-2` | `#10192e` | elevated panel / active |
| `--color-border` | `#1f2a44` | borders |
| `--color-accent` | `#4f8cff` | primary action |
| `--color-accent-2` | `#77a8ff` | gradient end |
| `--color-primary` | accent-derived | for shadcn variants |
| `--color-muted` | surface-based | hover fills |
| `--color-destructive` | red `#ef4444`-family | errors/danger |
| `--color-input` / `--color-ring` | border / accent | focus rings |

### 2.2 Typography
- Font: **Geist Variable** (`@fontsource-variable/geist`) — already wired.
- Scale: page titles `text-xl font-semibold tracking-tight`, section titles `text-lg`, body `text-sm`, meta `text-xs`, eyebrows `text-xs uppercase tracking-[0.28em]`.
- Tabular numerals (`tabular-nums`) for all timers, durations, sizes.

### 2.3 Motion (`@theme` keyframes)
- `fade-up` (already on website) — panel/route entrance, 250ms ease-out.
- `scale-in` — dialogs/toasts.
- `pulse-ring` — expanding ring on the active record button.
- `shimmer` — skeleton loading.
- All animations limited to `transform`/`opacity` (GPU-friendly).
- Global `@media (prefers-reduced-motion: no-preference)` guard — motion disabled otherwise.

### 2.4 Base layer
- `color-scheme: dark`.
- Thin, dark custom scrollbars.
- Global focus ring: `focus-visible:ring-2 ring-blue-500/20 border-blue-400/70`.
- `::selection` accent-tinted (existing).

---

## 3. Custom Frameless Title Bar

### 3.1 Rust / config
- `src-tauri/tauri.conf.json` → `app.windows[0]`: add `"decorations": false`, `"minWidth": 720`, `"minHeight": 520` (default stays 800×600).
- `src-tauri/src/lib.rs` `setup`: on macOS (`cfg(target_os = "macos")`), call `window.set_decorations(true)` + overlay `titleBarStyle` so native traffic lights render over content. Windows/Linux: keep `decorations: false`.
- Existing `CloseRequested → hide-to-tray` handler is untouched; `window.close()` from the custom close button still triggers it.

### 3.2 Component — `src/components/layout/TitleBar.tsx`
- Height `h-10` (40px), full-width, `border-b border-border`, subtle translucent bg `bg-[#050816]/85 backdrop-blur`.
- Root carries `data-tauri-drag-region` (native drag + double-click maximize on Windows).
- **Left:** mini `PrismLogo` (size-5) + current page label from route (Home / Library / Clip details / Settings).
- **Right (Windows/Linux only):** window controls via `@tauri-apps/api/window`:
  - Minimize (`minimize()`) — hover `bg-white/10`.
  - Maximize/Restore (`toggleMaximize()`) — icon swaps on fullscreenchange.
  - Close (`close()`) — hover `bg-red-500/90 text-white`.
  - Each `size-10`, `transition`, `active:scale-90`, `focus-visible:ring`.
- **macOS:** render nothing on the right (native lights handle it); reserve a `pl-20` safe area on the left so content clears the traffic lights.

### 3.3 Layout integration
- `AppLayout.tsx`: root becomes `flex flex-col h-screen` → `<TitleBar />` then a `flex flex-1 min-h-0` row of `<Sidebar />` + `<main>`.

---

## 4. App Layout & Sidebar

### 4.1 `src/components/layout/AppLayout.tsx`
- Keep: contextmenu suppression, cloud auth check on settings load, 1s recording-status poll while recording, menu/hotkey listeners, `ClipNotification`.
- Replace inline glow orbs with a dedicated `AmbientBackground` (absolute, `pointer-events-none`, two blurred `bg-accent` orbs).
- Keep `ClipNotification` (toast) in the tree; new global `<Toaster />` added too.

### 4.2 `src/components/layout/Sidebar.tsx`
- Width `w-56`; collapses to icon rail `w-14` below 768px window width (labels hidden, tooltips shown).
- **Brand block:** logo + "Prism" wordmark + "Game clipping" tagline (existing).
- **Nav:** Home / Library / Settings (`NavLink`), each item:
  - Active: `bg-surface` pill + accent left indicator bar + white text.
  - Idle: `text-zinc-400 hover:text-white hover:bg-white/5`.
  - Icons `size-4 shrink-0 text-blue-300` (existing), press feedback `active:scale-[0.98]`.
- **Recording status:** keep `RecordingIndicator` (pulsing red dot + elapsed + buffered), restyle chip.
- **Cloud status:** `CloudStatus` — emerald check "Connected"/`email`, zinc "Cloud off", pending-upload count badge (blue).
- **Version footer:** derive via `getVersion()` from `@tauri-apps/api/app` — remove hardcoded `v0.2.3`.

---

## 5. Home Page (`src/pages/HomePage.tsx`)

### 5.1 Layout
- Responsive: `flex flex-col lg:flex-row gap-5 px-6 pb-5`.
- **Main column (flex-1, min-w-0):** Preview → error toast → controls → info chips.
- **Source column:** `w-full lg:w-64` — below the preview on narrow windows, right rail on wide.

### 5.2 Preview (`src/components/common/ScreenPreview.tsx`)
- Panel: `aspect-video` in a `rounded-2xl border border-border` frame with a subtle gradient ring on hover.
- Behavior unchanged: poll `get_preview_frame` at ~1fps only while recording, exponential backoff to 10s on errors.
- Add `decoding="async"` to the `<img>`.
- Idle state: `Monitor` icon + "Start recording to see preview" + subtle secondary text.
- LIVE badge (top-left, black/50 blur pill, red pulsing dot) + resolution chip (`Monitor` + `resolution`).

### 5.3 RecordingControls (`src/components/common/RecordingControls.tsx`)
- Keep logic (`startRecording`/`stopRecording`/`saveClip`, starting/saving states).
- **Record button** (`size-16 rounded-full`):
  - Idle: `bg-surface border-2 border-border`, `Play` icon.
  - Recording: `bg-red-600 border-red-500`, `Square` icon, `pulse-ring` animation (keyframe ring), glow `shadow-[0_0_24px_rgba(239,68,68,0.4)]`.
  - Starting: `Loader2 animate-spin`, `cursor-wait`.
- **Save-clip button** (`size-11`): `Scissors` icon, hidden/shrunk when not recording, `Kbd` tooltip showing the save hotkey from settings.
- Error state renders via global toast + inline if error present.

### 5.4 Info bar
- Replace with `Badge` chips: target label (`Monitor`), `{bufferDurationSecs}s clip` (`HardDrive`), `{resolution} · {bitrate} Mbps · {fps} FPS` (`Film`). Truncate-safe (`min-w-0`).

### 5.5 SourceSelector (`src/components/common/SourceSelector.tsx`)
- Keep data flow (`get_capture_sources`, `set_capture_target`, JSON target parsing).
- Row polish: resolution badge (`width×height`), "Main display" gets an accent dot; selected row `bg-surface-2 border-accent/20` + `Check`.
- Refresh button: optimistic — spinner while loading; keeps current selection.
- Panel header "Capture Source" + tab segmented control (Screen / App) restyled per design system.

---

## 6. Library Page (`src/pages/LibraryPage.tsx`)

### 6.1 Header
- Title "Clip Library" + count badge (`{n} clips`).
- Actions row: **Open Folder** (`FolderOpen`, `variant="secondary"`), **search input** (existing behavior), working **sort** and **filter** menus (replaces dead `onClick={() => {}}` Filter button):
  - Sort: Newest / Oldest / Name A–Z / Size (desc) / Duration (desc).
  - Filter: All / Uploaded / Uploading / Failed / By game (from clip data).
- Upload error banner → replaced by toast (keep store error for retry UX).

### 6.2 Clip grid
- Container: `grid gap-4 grid-cols-[repeat(auto-fill,minmax(220px,1fr))]`.
- Item class: `content-visibility: auto` (via inline utility or CSS layer) + `contain-intrinsic-size`.
- **ClipCard** (memoized, existing):
  - `ClipThumbnail` images get `loading="lazy"` + `decoding="async"`.
  - Duration badge top-right (black/60 blur pill).
  - Status pill top-left: Uploaded (emerald + `Check`) / Uploading (`Loader2 spin`, accent) / Failed (red, tooltip with `task.error`).
  - Upload progress bar (existing inline, keep).
  - Bottom gradient overlay: title, game, duration · size, date.
  - Hover overlay: Play (nav) + Upload or Copy-share-link (conditional on `cloudAuthed`/`share_url`) + Delete (top-right trash).
  - Delete confirm: **Dialog** component (not inline overlay).

### 6.3 States
- Loading: skeleton grid (mirror website `SkeletonClipsGrid` — 6–8 `aspect-video` shimmer cards).
- Empty (no clips): `Film` icon, "No clips yet", hint text.
- Empty (search/filter): "No clips match your search" + "Clear filters" action.

---

## 7. Clip Detail Page (`src/pages/ClipDetailPage.tsx`)

### 7.1 Header
- Back button (`ArrowLeft`, `icon-sm` ghost), truncated title + filename sub-label, size badge on right.

### 7.2 VideoPlayer polish (`src/components/common/VideoPlayer.tsx`)
- Keep rAF time updates, seek, PiP, fullscreen, auto-hide (3s), poster.
- Add keyboard shortcuts when focused: `Space` play/pause, `←`/`→` ±5s, `M` mute, `F` fullscreen.
- Seekbar: custom gradient fill (played = accent, rest = white/15), `h-1.5 rounded-full`.
- Controls bar: consistent `active:scale-90` hover fills; time `tabular-nums text-xs`.

### 7.3 Metadata card
- Convert to stat tiles in a 2-col grid:
  - Game (`Gamepad2`, blue), Captured (date), Duration (`Clock`), Size (`HardDrive`), Format (MP4).
  - Description block spans full width.
- Edit mode (existing data flow): `Input`/`textarea` primitives, Save (`variant="brand"`) / Cancel, inline error text, `Saving...` state.

---

## 8. Settings Page (`src/pages/SettingsPage.tsx`) — Restructured

### 8.1 Layout
- Two-pane: left **section rail** (`w-44 shrink-0`, sticky) + content pane (flex-1, max-w-2xl).
- Sections (tabs): Recording · Hotkeys · General · Auto-clip · Cloud · Storage.
- Rail collapses to horizontal chips on narrow widths.
- Keep "Changes are saved automatically" hint.

### 8.2 Code structure
- `SettingsPage.tsx` → thin shell owning: `loadSettings`, section state, `useSettingsActions`.
- New `src/hooks/useSettingsActions.ts`: `setField`, `save`, `debouncedSave` (300ms), `resetHotkeys`, `updateAutoClipGame` — extracted verbatim.
- New section components in `src/components/settings/sections/`:
  - `RecordingSection.tsx` — clip length slider, FPS select, resolution select, bitrate `PresetSlider`, output directory input (debounced), always-on toggle.
  - `HotkeysSection.tsx` — three `HotkeyCaptureInput` rows + "Reset to defaults".
  - `GeneralSection.tsx` — launch-at-startup, minimize-to-tray, show-clip-notification, game-detection, CS2 GSI port + restart hint.
  - `AutoClipSection.tsx` — enable toggle, cooldown slider, Rust audio-sensitivity slider, per-game cards (detection badge, event chips, duration inputs, audio toggle) + footnote.
  - `CloudSection.tsx` — server URL, account card (sign in/out, manual auth-code expander), auto-upload, concurrent uploads.
  - `StorageSection.tsx` — max clips GB + auto-prune days.
- New shared components:
  - `SettingRow` (label left w/ help text, control right), `SettingCard` (grouped `rounded-2xl border-border bg-surface/70`).
  - `ui/switch.tsx` replaces inline `ToggleSwitch` (animated knob, `transition`).
  - `ui/input.tsx`, `ui/select.tsx` consolidate the ~6 repeated input class strings.

---

## 9. Component Library (`src/components/ui/`)

New primitives (all follow AGENTS.md conventions: `transition`, `active:scale-[0.98]`, focus rings, lucide `size-4 shrink-0`):

| File | Purpose |
|---|---|
| `input.tsx` | text/number input base styles |
| `select.tsx` | styled native `<select>` |
| `switch.tsx` | animated toggle |
| `slider.tsx` | range with filled track + value label |
| `tabs.tsx` | section navigation (accessible, keyboard) |
| `dialog.tsx` | modal (delete confirm, share) with scale-in + backdrop blur |
| `toast.tsx` + `src/stores/toast.ts` | global toast queue + `<Toaster />` |
| `skeleton.tsx` | shimmer primitives |
| `empty-state.tsx` | icon + title + hint + optional action |
| `kbd.tsx` | hotkey keycap display |
| `tooltip.tsx` | hover tooltip for icon buttons |

**Toast triggers:** clip saved, share link copied, upload started / completed / failed, clip deleted, sign-in/out, settings saved.

---

## 10. Performance Requirements ("Snappy")

1. **Lazy images:** thumbnail `<img loading="lazy" decoding="async">`; preview base64 `decoding="async"`.
2. **Content-visibility:** `content-visibility: auto` on clip-grid cards with `contain-intrinsic-size` (e.g. 220×124).
3. **Memoization:** keep `MemoClipCard`; ensure upload-progress events only change the matching card's `task` identity (map by `clip_path`).
4. **Polling discipline:** `get_buffer_info` 1s while recording only (existing); `get_preview_frame` 1fps while recording only (existing); `uploadQueueStatus` on Library mount only (existing).
5. **Re-render isolation:** timer state consumed via narrow selectors; toast rendering isolated from page content.
6. **Motion budget:** transform/opacity only; no width/height/box-shadow animation on hot paths; `will-change` on the two glow orbs only.
7. **Settings:** sliders update local UI instantly, persist debounced 300ms.
8. **Search/filter:** `useMemo` + `startTransition` for filtering.

---

## 11. Responsive Breakpoints

| Width | Behavior |
|---|---|
| ≥ 1024 | Sidebar full (w-56), Home two-column, settings rail vertical |
| 768–1023 | Sidebar full; Home source column still right rail |
| < 768 | Sidebar icon rail (w-14, tooltips); Home stacks source below preview; settings rail → horizontal chips; clip grid 1–2 cols |
| Min window | 720×520 (via tauri.conf.json `minWidth`/`minHeight`) |

All text that can overflow uses `truncate`/`min-w-0`; no horizontal scroll at min width.

---

## 12. Accessibility
- Focus ring everywhere (no `outline-none` without a ring).
- Button/link `aria-label`s on icon-only controls.
- Toasts/dialogs keyboard-dismissible; dialogs trap focus.
- `prefers-reduced-motion` honored.
- Color contrast maintained (zinc-400+ on navy for body text).

---

## 13. Files Touched / Created

**Rust (2):** `src-tauri/tauri.conf.json`, `src-tauri/src/lib.rs`.

**Frontend — created (~15):**
- `src/components/layout/TitleBar.tsx`
- `src/components/layout/AmbientBackground.tsx`
- `src/components/ui/{input,select,switch,slider,tabs,dialog,toast,skeleton,empty-state,kbd,tooltip}.tsx`
- `src/stores/toast.ts`
- `src/hooks/{useSettingsActions,useRouteTitle}.ts`
- `src/components/settings/sections/{Recording,Hotkeys,General,AutoClip,Cloud,Storage}Section.tsx`

**Frontend — modified (~15):**
- `src/index.css`, `src/components/ui/{button,brand}.tsx`
- `src/components/layout/{AppLayout,Sidebar}.tsx`
- `src/pages/{HomePage,LibraryPage,ClipDetailPage,SettingsPage}.tsx`
- `src/components/common/{RecordingControls,ScreenPreview,SourceSelector,ClipThumbnail,ClipNotification,RecordingIndicator,VideoPlayer}.tsx`
- `src/components/settings/{PresetSlider,HotkeyCaptureInput}.tsx`

---

## 14. Verification Checklist (per AGENTS.md)

1. `cargo check` — clean (title bar/config changes).
2. `npx tsc --noEmit` — clean.
3. `npm run build` — clean.
4. `npm run tauri build` — **must succeed** (msi + setup.exe).
5. Manual: drag/maximize/restore/close via custom bar; close → hide to tray; record → save → toast + library refresh; share-link copy toast; upload progress pill; delete confirm dialog; 800×600 layouts; settings tabs; search/filter/sort; macOS traffic-light overlap (if macOS lane available).

## 15. Execution Order
Phase 0 tokens → Phase 1 title bar → Phase 2 component library → Phase 3 layout/sidebar → Home → Library → Clip Detail → Settings (largest) → Phase 4 performance pass interleaved → Phase 5 verification + `tauri build`.