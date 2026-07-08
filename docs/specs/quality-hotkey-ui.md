# Spec: quality-hotkey-ui

Scope: feature

# Quality & Hotkey UI — Feature Spec

## 1. Goals

Replace the current text-based quality/hotkey settings with a usable UI:
- Resolution selector + bitrate slider instead of a vague "quality" dropdown
- Key-capture hotkey input instead of typing modifier strings
- Hotkeys re-register immediately on change

## 2. Data Model Changes

### RecordingSettings (Rust & TS)

```diff
- quality: String        // "performance" | "balanced" | "quality"
+ bitrate_kbps: u32      // e.g. 8000 = 8 Mbps
+ resolution: String      // "720p" | "1080p" | "1440p" | "2160p"
```

- Default bitrate: `8000` (8 Mbps — good for 1080p H.264)
- Default resolution: `"1080p"`

### EncoderConfig (Rust)

```diff
- quality: Quality
- bitrate: Option<u64>
+ bitrate_kbps: u32
+ target_width: u32
+ target_height: u32
```

- Remove `Quality` enum, `parse_quality()`, `effective_bitrate()`

### Hotkeys (no change to data model)

`HotkeySettings` stays `{ save_clip: String, toggle_recording: String, open_library: String }`. Only the UI changes from text input to key capture.

## 3. Resolution → Dimensions Map

| Setting | Width | Height |
|---------|-------|--------|
| 720p    | 1280  | 720    |
| 1080p   | 1920  | 1080   |
| 1440p   | 2560  | 1440   |
| 2160p   | 3840  | 2160   |

## 4. Bitrate Presets (Slider Values)

The slider snaps to these values (in kbps):

```
1000, 2500, 5000, 8000, 12000, 16000, 25000, 40000, 60000
```

Corresponding labels: "1 Mbps", "2.5", "5", "8", "12", "16", "25", "40", "60 Mbps"

Default position: `8000` (index 3)

## 5. Resolution Scaling Strategy

**VT encoder handles scaling** — the capture backend always captures at the full display resolution (whatever SCK provides). The target resolution is passed to `EncoderConfig` as `target_width` / `target_height`. The macOS encoder sets these as the compression session dimensions, and VideoToolbox scales internally.

Why: no stream restart needed, zero capture interruption, VT's hardware scaler is fast, and it works transparently with our BGRA pipeline.

## 6. Hotkey Capture UX

Each hotkey row gets a `HotkeyCaptureInput` component that:

1. Shows the current shortcut in a styled button (e.g. `[Cmd+Shift+X]`)
2. On click, enters **capture mode** — button highlights, shows "Press keys..."
3. On `keydown`:
   - Captures modifier keys (Meta/Ctrl/Alt/Shift) + the key
   - Formats as `"Mod1+Mod2+Key"` (matching `Shortcut::from_str`)
   - Validates via `Shortcut::from_str` (trivially via IPC or a local parse)
   - On success: updates settings
   - On failure: shows validation error
4. On `Escape`: cancels capture mode, reverts to previous value
5. On blur (click outside): cancels capture mode

Key mapping:
- `Meta` → `"Cmd"` on macOS, `"Win"` on others
- `Control` → `"Ctrl"`
- `Alt` → `"Option"` on macOS, `"Alt"` on others
- `Shift` → `"Shift"`

## 7. Hotkey Re-registration

In `commands/settings.rs`:
- After `settings_mgr.set()` succeeds, call `hotkey::register_hotkeys(app.handle(), &new_settings.hotkeys)`
- `register_hotkeys()` must first **unregister** existing hotkeys (call `app.global_shortcut().unregister_all()` or track registered shortcuts)

## 8. Files to Change

### Rust
- `src-tauri/src/settings/config.rs` — new fields, resolutions map, bitrate default
- `src-tauri/src/encoder/codecs/mod.rs` — remove Quality, update EncoderConfig
- `src-tauri/src/encoder/macos/mod.rs` — use target_width/target_height + raw bitrate
- `src-tauri/src/recording/mod.rs` — remove parse_quality, update save_clip
- `src-tauri/src/commands/recording.rs` — update save_clip
- `src-tauri/src/hotkey/mod.rs` — make register_hotkeys callable from anywhere, add unregister step
- `src-tauri/src/commands/settings.rs` — trigger hotkey re-register after update
- `src-tauri/src/encoder/windows/mod.rs` — no change (stub)
- `src-tauri/src/encoder/linux/mod.rs` — no change (stub)

### TypeScript
- `src/types/settings.ts` — update RecordingSettings
- `src/stores/settings.ts` — new defaults
- `src/pages/SettingsPage.tsx` — replace quality select, replace hotkey inputs
- `src/components/settings/HotkeyCaptureInput.tsx` — new component
- `src/components/settings/SliderInput.tsx` — new component (if extracted)

## 9. Acceptance Criteria

- [ ] Settings page shows Resolution dropdown with 4 options
- [ ] Resolution dropdown changes encoder output size (VT scales)
- [ ] Bitrate slider shows current value, snaps to presets
- [ ] Changing resolution/bitrate saves immediately
- [ ] Each hotkey has a capture button that works on click
- [ ] Captured hotkeys display correctly and are saved to settings
- [ ] Changed hotkeys take effect immediately (no restart needed)
- [ ] `npm run tauri build` succeeds
- [ ] All warnings remain pre-existing only (no new warnings or errors)