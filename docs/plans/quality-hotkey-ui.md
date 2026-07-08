---
plan name: quality-hotkey-ui
plan description: Resolution/bitrate + hotkey capture
plan status: active
---

## Idea
Replace the current quality preset dropdown with proper Resolution + Bitrate settings, and replace the text-input hotkey fields with key-capture inputs. This is about making the settings UI useful and discoverable rather than hacky.

For resolution: add a dropdown with 720p / 1080p / 1440p / 2160p (4K). The selected resolution scales the captured display output to that size before encoding (or can crop). This affects the width/height in EncoderConfig.

For bitrate: replace the quality text field with a slider that maps to a list of preset bitrate values. The list can be long (maybe 6-10 values from 1 Mbps up to 50 Mbps). The slider snaps to the nearest preset. The selected bitrate in kbps is stored in settings and passed directly to EncoderConfig instead of going through effective_bitrate().

For hotkeys: replace the three text inputs with "Click to set hotkey" buttons. When clicked, the app listens for the next keydown event and captures the modifier+key combination, then validates it and saves it. The existing tauri-plugin-global-shortcut Shortcut::from_str is the validation function.

Also needs: re-register hotkeys when settings change (currently only done at startup).

## Implementation
- 1. Rust: Replace `quality: String` in `RecordingSettings` with `bitrate_kbps: u32` and `resolution: String`. Add `default_resolution()` and `default_bitrate()` helpers in config.rs. Remove `parse_quality()` and related dead code. Update `save_clip()` in both recording/mod.rs and commands/recording.rs to pass bitrate directly.
- 2. Rust: Add resolution scaling to the capture pipeline. When recording starts, scale capture frames to the configured resolution (720p/1080p/etc) before pushing to the ring buffer, OR pass the target resolution to EncoderConfig and let the encoder prep handle it.
- 3. Rust: Update `EncoderConfig` — replace `quality: Quality` with `bitrate_kbps: u32` and `target_width/target_height: u32`. Remove `effective_bitrate()`. Update macos encoder to use the raw bitrate value.
- 4. Rust: Expose `register_hotkeys()` as a public function callable from `commands/settings.rs`. Call it after `SettingsManager::set()` in `update_settings()` so hotkeys re-register on change.
- 5. Frontend types: Update `RecordingSettings` TS interface — replace `quality: string` with `bitrate_kbps: number` and `resolution: string`.
- 6. Frontend settings store: Update DEFAULT_SETTINGS with new shape.
- 7. Frontend SettingsPage: Replace quality <select> with a Resolution <select> (720p, 1080p, 1440p, 4K) and a Bitrate <input type=range> that snaps through a list of predefined kbps values with labels. Build a reusable SliderInput component.
- 8. Frontend SettingsPage: Replace the three hotkey text inputs with a `HotkeyCapture` component — an inert-looking button that enters capture mode on click, listens for keydown, captures modifier+key combo, validates the shortcut format, and emits the result. Exit capture mode on Escape.
- 9. Frontend: Ensure clip list auto-refreshes after a save by using the clip-saved event listener (already wired from previous session, verify it still works).
- 10. Build + verify: `npm run tauri build` succeeds with no errors.

## Required Specs
<!-- SPECS_START -->
- quality-hotkey-ui
<!-- SPECS_END -->