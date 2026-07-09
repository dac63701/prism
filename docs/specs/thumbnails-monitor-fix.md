# Spec: thumbnails-monitor-fix

Scope: feature

# Server-side Thumbnails & Monitor Switching Fix

## Part A: Server-side JPEG Thumbnails

### Goal
Eliminate slow video-decode-in-browser by generating JPEG thumbnails during clip save.

### Data Changes

**ClipData struct** (`recording/mod.rs`):
```diff
+ preview_frame: Option<CapturedFrame>
```

**`extract_clip_data()`**: Clone `latest_frame` into `preview_frame` under the recorder lock.

### Thumbnail Generation (`commands/recording.rs`)

After `encoder.encode_clip()` succeeds, generate JPEG from the preview frame:

Match on `preview_frame.pixel_format`:
- **Nv12**: Convert directly to RGB buffer (use existing `nv12_to_rgb` logic adapted for `image::RgbImage`). Point-sample downscale to 320px wide while converting (single pass).
- **Bgra**: Same approach — downscale to 320px wide during conversion.

Encode to JPEG at quality 75 via `image::codecs::jpeg::JpegEncoder`. Save as `clipname_thumb.jpg` alongside the MP4.

```rust
fn generate_thumbnail(
    frame: &CapturedFrame,
    output_dir: &Path,
    mp4_stem: &str,
) -> Result<(), String> {
    let thumb_path = output_dir.join(format!("{mp4_stem}_thumb.jpg"));
    let max_w = 320u32;
    // convert + downscale + encode JPEG
}
```

### Frontend (`ClipThumbnail.tsx`)

```tsx
const thumbSrc = convertFileSrc(path.replace(/\.mp4$/, "_thumb.jpg"));
```

Try loading the JPEG first:
```
<img src={thumbSrc} onError={fallbackToVideo} />
```

If the `<img>` fails (`onError`), fall back to the current `<video>` element approach (for clips saved before this change). The `onError` fires once and the component switches to video mode.

Remove IntersectionObserver (JPEG loading is fast enough without it).

---

## Part B: Monitor Switching Fix

### B-1: Reset H.264 encoder on stop (`recording/mod.rs`)

In `stop_recording()` macOS block, add:
```rust
inner.h264_encoder = None; // forces re-creation with new display dimensions
```

This ensures the next `poll_and_push()` with the new display creates a fresh `VtH264Encoder` at the correct dimensions.

### B-2: Always emit state event (`commands/recording.rs`)

Restructure `set_capture_target` to emit `recording-state-changed` even when `start_recording()` fails:

```rust
let rec = recorder.lock()?;
let was_recording = rec.is_recording();
if was_recording {
    rec.stop_recording().ok();
    rec.reconfigure_target(target);
    let start_ok = rec.start_recording();
    if start_ok.is_ok() {
        rec.start_polling(app.clone());
    }
    let _ = app.emit("recording-state-changed", start_ok.is_ok());
} else {
    rec.reconfigure_target(target);
}
```

### B-3: Surface errors to frontend (`HomePage.tsx`)

`handleSourceChange` should write errors into the recording store's `error` field so the error banner in HomePage.tsx shows them:

```tsx
const setError = useRecordingStore((s) => s.setError);
const handleSourceChange = async (targetJson: string) => {
    try {
        await invoke("set_capture_target", { targetJson });
        await loadSettings();
    } catch (err) {
        const msg = typeof err === "string" ? err : "Failed to switch source";
        setError(msg);
    }
};
```

Add `setError` action to the recording store:
```tsx
setError: (err: string) => set({ error: err }),
```