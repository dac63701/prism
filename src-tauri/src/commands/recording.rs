//! Recording IPC commands — start, stop, save clip, status.

use parking_lot::Mutex;

use tauri::{AppHandle, Emitter, State};

use std::path::Path;

use crate::capture::{enumerate_capture_sources, CaptureSources, CaptureTarget, PixelFormat};
use crate::encoder::codecs::{Codec, EncoderConfig};
use crate::encoder::create_encoder;
use crate::recording::{chrono_now_formatted, Recorder};
use crate::settings::config::resolution_dimensions;
use crate::settings::SettingsManager;

/// Start the ring-buffer recording and spawn the polling task.
#[tauri::command]
pub async fn start_recording(
    app: AppHandle,
    recorder: State<'_, Mutex<Recorder>>,
) -> Result<String, String> {
    eprintln!("[recording] start_recording command invoked");

    let rec = recorder.lock();

    rec.start_recording().map_err(|e| {
        eprintln!("[recording] start_recording failed: {e}");
        e
    })?;

    rec.start_polling(app.clone());
    // Always emit the event — polling spawn status is internal
    let _ = app.emit("recording-state-changed", true);

    eprintln!("[recording] start_recording command succeeded");
    Ok("started".to_string())
}

/// Stop recording.
#[tauri::command]
pub async fn stop_recording(
    app: AppHandle,
    recorder: State<'_, Mutex<Recorder>>,
) -> Result<(), String> {
    eprintln!("[recording] stop_recording command invoked");

    {
        let rec = recorder.lock();
        rec.stop_recording()?;
    }

    let _ = app.emit("recording-state-changed", false);
    eprintln!("[recording] stop_recording command succeeded");
    Ok(())
}

/// Check whether recording is active.
#[tauri::command]
pub async fn is_recording(recorder: State<'_, Mutex<Recorder>>) -> Result<bool, String> {
    let rec = recorder.lock();
    Ok(rec.is_recording())
}

/// Trigger a clip save — extracts frames under the recorder lock (briefly),
/// then encodes to MP4 OUTSIDE the lock so the polling task keeps running.
#[tauri::command]
pub async fn save_clip(
    app: AppHandle,
    recorder: State<'_, Mutex<Recorder>>,
    settings_mgr: State<'_, SettingsManager>,
    duration_secs: u32,
) -> Result<String, String> {
    let settings = settings_mgr.get();
    let duration = if duration_secs > 0 {
        duration_secs
    } else {
        settings.recording.buffer_duration_secs
    };

    let filename = format!("clip_{}.mp4", chrono_now_formatted());
    save_clip_internal(&app, &recorder, &settings, duration, filename)
}

/// Save a clip from an internal event source while retaining the same safe
/// extract-then-encode behavior used by the manual Tauri command.
pub(crate) fn save_clip_internal(
    app: &AppHandle,
    recorder: &Mutex<Recorder>,
    settings: &crate::settings::config::AppSettings,
    duration: u32,
    filename: String,
) -> Result<String, String> {
    // Step 1: Extract frames under lock (brief — frame copy only)
    let clip_data = {
        let rec = recorder.lock();
        rec.extract_clip_data(duration)?
    };
    // LOCK RELEASED — polling and other commands can proceed

    if clip_data.frames.is_empty() {
        return Err("No frames available to clip".into());
    }

    // Step 2: Build encoder config from settings
    let rs = &settings.recording;
    let first = &clip_data.frames[0];
    let dims = resolution_dimensions(&rs.resolution);
    let (target_width, target_height) = if dims == (0, 0) {
        (first.width, first.height)
    } else {
        dims
    };
    let enc_config = EncoderConfig {
        codec: Codec::H264,
        bitrate_kbps: rs.bitrate_kbps,
        fps: rs.fps,
        keyframe_interval: rs.fps,
        target_width,
        target_height,
    };

    // Step 3: Generate output path
    let output_path = clip_data.output_dir.join(&filename);

    // Ensure output directory exists
    std::fs::create_dir_all(&clip_data.output_dir)
        .map_err(|e| format!("Failed to create output directory: {e}"))?;

    // Step 4: Generate server-side thumbnail (before frames are moved into
    // prepare_h264_clip_frames). Try preview_frame first, then fall back to
    // extracting a usable frame from the clip data.
    let thumb_stem = output_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let thumb_path = output_path.with_file_name(format!("{}_thumb.jpg", thumb_stem));
    let thumb_result = match clip_data.preview_frame.as_ref() {
        Some(preview) => generate_thumbnail(preview, &thumb_path)
            .or_else(|_| extract_thumbnail_from_clip_frames(&clip_data.frames, &thumb_path)),
        None => extract_thumbnail_from_clip_frames(&clip_data.frames, &thumb_path),
    };
    if let Err(e) = thumb_result {
        eprintln!("[recording] thumbnail generation failed: {e}");
    }

    // Step 5: Keep only a decodable H.264 sequence. Raw fallback frames cannot
    // be mixed into an H.264 MP4 track, and decoding must begin at a sync frame.
    eprintln!(
        "[recording] save_clip: {} frames, sps={} pps={}",
        clip_data.frames.len(),
        clip_data.sps.len(),
        clip_data.pps.len()
    );
    let frames_with_sps =
        prepare_h264_clip_frames(clip_data.frames, &clip_data.sps, &clip_data.pps)?;

    // Step 6: Encode (NO lock held — polling continues)
    eprintln!(
        "[recording] save_clip encoding {} frames to {}",
        frames_with_sps.len(),
        output_path.display()
    );
    let mut encoder = create_encoder();
    encoder
        .encode_clip(&frames_with_sps, &output_path, &enc_config)
        .map_err(|e| format!("Encoding failed: {e}"))?;
    eprintln!("[recording] save_clip encoding complete");

    let output_str = output_path.to_string_lossy().to_string();
    let _ = app.emit("clip-saved", &output_str);

    Ok(output_str)
}

/// Select a decodable H.264 sequence for MP4 muxing.
///
/// Raw NV12 frames are a capture fallback and cannot be written to an AVC
/// track. Starting at a sync frame prevents clips that begin with undecodable
/// P-frames. Cached parameter sets are AVCC-formatted and are attached to that
/// sync sample so the muxer can build the AVC configuration.
fn prepare_h264_clip_frames(
    frames: Vec<crate::buffer::StoredFrame>,
    sps: &[u8],
    pps: &[u8],
) -> Result<Vec<crate::buffer::StoredFrame>, String> {
    let total = frames.len();
    let first_sync = frames
        .iter()
        .position(|frame| frame.pixel_format == PixelFormat::H264 && frame.is_sync)
        .ok_or_else(|| {
            "No H.264 keyframe is available yet. Keep recording for a moment and try again."
                .to_string()
        })?;

    let dropped_before_sync = first_sync;
    let mut h264_frames: Vec<_> = frames
        .into_iter()
        .skip(first_sync)
        .filter(|frame| frame.pixel_format == PixelFormat::H264)
        .collect();

    let dropped_non_h264 = total
        .saturating_sub(first_sync)
        .saturating_sub(h264_frames.len());
    if dropped_before_sync > 0 || dropped_non_h264 > 0 {
        eprintln!(
            "[recording] prepare_h264_clip_frames: {total} total, \
             {dropped_before_sync} dropped before sync, {dropped_non_h264} non-H.264 dropped, \
             {} H.264 frames kept",
            h264_frames.len()
        );
    }

    if !sps.is_empty() && !pps.is_empty() {
        let first = h264_frames.first_mut().ok_or_else(|| {
            "No H.264 frames available after filtering — clip may contain only raw NV12 fallback data".to_string()
        })?;
        let mut data = Vec::with_capacity(sps.len() + pps.len() + first.data.len());
        data.extend_from_slice(sps);
        data.extend_from_slice(pps);
        data.extend_from_slice(&first.data);
        first.data = std::sync::Arc::new(data);
    }

    Ok(h264_frames)
}

/// Get a live preview frame as a JPEG base64 data URL.
/// Returns `null` when not recording or no frame available.
#[tauri::command]
pub async fn get_preview_frame(
    recorder: State<'_, Mutex<Recorder>>,
) -> Result<Option<String>, String> {
    let rec = recorder.lock();
    Ok(rec.get_preview_frame())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Instant;

    fn frame(
        pixel_format: PixelFormat,
        is_sync: bool,
        data: Vec<u8>,
    ) -> crate::buffer::StoredFrame {
        crate::buffer::StoredFrame {
            data: Arc::new(data),
            width: 1920,
            height: 1080,
            stride: 0,
            pixel_format,
            timestamp: Instant::now(),
            is_sync,
        }
    }

    #[test]
    fn clip_preparation_attaches_parameters_to_first_h264_sync_frame() {
        let sps = [0, 0, 0, 2, 0x67, 0x42];
        let pps = [0, 0, 0, 2, 0x68, 0xCE];
        let sync_sample = vec![0, 0, 0, 2, 0x65, 0x88];
        let next_sample = vec![0, 0, 0, 2, 0x41, 0x9A];
        let frames = vec![
            frame(PixelFormat::Nv12, true, vec![0; 8]),
            frame(PixelFormat::H264, false, next_sample.clone()),
            frame(PixelFormat::H264, true, sync_sample.clone()),
            frame(PixelFormat::H264, false, next_sample.clone()),
        ];

        let prepared = prepare_h264_clip_frames(frames, &sps, &pps).unwrap();

        assert_eq!(prepared.len(), 2);
        assert!(prepared[0].is_sync);
        let expected = [sps.as_slice(), pps.as_slice(), sync_sample.as_slice()].concat();
        assert_eq!(prepared[0].data.as_slice(), expected);
        assert_eq!(prepared[1].data.as_slice(), next_sample);
    }

    #[test]
    fn clip_preparation_requires_an_h264_sync_frame() {
        let frames = vec![
            frame(PixelFormat::Nv12, true, vec![0; 8]),
            frame(PixelFormat::H264, false, vec![0, 0, 0, 2, 0x41, 0x9A]),
        ];

        let error = prepare_h264_clip_frames(frames, &[], &[]).unwrap_err();

        assert!(error.contains("keyframe"));
    }
}

/// Get the current frame count in the ring buffer.
#[tauri::command]
pub async fn get_buffer_info(
    recorder: State<'_, Mutex<Recorder>>,
) -> Result<serde_json::Value, String> {
    let rec = recorder.lock();
    let fc = rec.frame_count();
    let fr = rec.total_frames_received();
    let fps = rec.cached_fps();
    let clip_len = rec.available_clip_secs();
    let actual_buffer_time = rec.buffer_time_secs();
    let elapsed = rec.recording_elapsed_secs();
    Ok(serde_json::json!({
        "frame_count": fc,
        "buffer_time_seconds": clip_len,
        "clip_length_seconds": clip_len,
        "actual_buffer_time_seconds": actual_buffer_time,
        "configured_duration_seconds": rec.buffer_duration_secs(),
        "is_recording": rec.is_recording(),
        "frames_received": fr,
        "preview_available": rec.preview_available(),
        "polling_active": true,
        "recording_elapsed_seconds": elapsed,
        "fps": fps,
    }))
}

/// List available displays and applications for the source selector UI.
#[tauri::command]
pub async fn get_capture_sources() -> Result<CaptureSources, String> {
    Ok(enumerate_capture_sources())
}

/// Set the capture target (display, window, or application).
/// Saves to settings and reconfigures the recorder.
/// Accepts target as a JSON string matching serde externally-tagged format,
/// e.g. `"display"` for main display or `{"display_id":5}` for a specific display.
#[tauri::command]
pub async fn set_capture_target(
    app: AppHandle,
    recorder: State<'_, Mutex<Recorder>>,
    settings_mgr: State<'_, SettingsManager>,
    target_json: String,
) -> Result<(), String> {
    let target: CaptureTarget = serde_json::from_str(&target_json)
        .map_err(|e| format!("Invalid capture target JSON: {e}"))?;

    let mut settings = settings_mgr.get();

    // Update settings
    settings.recording.capture_target = target_json;
    settings_mgr
        .set(&app, settings)
        .map_err(|e| format!("Failed to save settings: {e}"))?;

    // Reconfigure recorder with new target
    {
        let rec = recorder.lock();
        let was_recording = rec.is_recording();
        if was_recording {
            rec.stop_recording().ok();
            rec.reconfigure_target(target);
            let started = rec.start_recording();
            if started.is_ok() {
                rec.start_polling(app.clone());
            }
            let _ = app.emit("recording-state-changed", started.is_ok());
        } else {
            rec.reconfigure_target(target);
            let _ = app.emit("recording-state-changed", false);
        }
    }

    Ok(())
}

/// Generate a high-quality JPEG thumbnail from a captured frame and save it
/// alongside the MP4. The image fits within a 1280×720 box for crisp library
/// cards and a useful poster in the clip editor.
fn generate_thumbnail(
    frame: &crate::capture::CapturedFrame,
    thumb_path: &Path,
) -> Result<(), String> {
    use image::imageops::FilterType;

    let w = frame.width;
    let h = frame.height;
    let (thumb_w, thumb_h) = thumbnail_dimensions(w, h);

    let rgb = match frame.pixel_format {
        PixelFormat::Nv12 => crate::capture::nv12_to_rgb(&frame.data, w, h),
        PixelFormat::Bgra => {
            let mut rgb = vec![0u8; (w * h * 3) as usize];
            for y in 0..h {
                for x in 0..w {
                    let off = (y * frame.stride + x * 4) as usize;
                    let dst = (y * w + x) as usize * 3;
                    rgb[dst] = frame.data[off + 2];
                    rgb[dst + 1] = frame.data[off + 1];
                    rgb[dst + 2] = frame.data[off];
                }
            }
            rgb
        }
        PixelFormat::H264 => return Err("Cannot generate thumbnail from H.264 data".into()),
    };

    let img =
        image::RgbImage::from_raw(w, h, rgb).ok_or("Failed to create RGB image from frame data")?;
    let resized = image::imageops::resize(&img, thumb_w, thumb_h, FilterType::Triangle);

    let file = std::fs::File::create(thumb_path)
        .map_err(|e| format!("Failed to create thumbnail file: {e}"))?;
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, 90);
    encoder
        .encode(&resized, thumb_w, thumb_h, image::ExtendedColorType::Rgb8)
        .map_err(|e| format!("JPEG encode failed: {e}"))?;

    Ok(())
}

/// Fallback: extract a thumbnail from the first usable NV12/BGRA frame in
/// the clip data when no preview_frame was available.
fn extract_thumbnail_from_clip_frames(
    frames: &[crate::buffer::StoredFrame],
    thumb_path: &Path,
) -> Result<(), String> {
    let frame = frames
        .iter()
        .find(|f| f.pixel_format == PixelFormat::Nv12 || f.pixel_format == PixelFormat::Bgra)
        .ok_or_else(|| "No NV12 or BGRA frame in clip data for thumbnail generation".to_string())?;
    let captured = crate::capture::CapturedFrame {
        data: frame.data.clone(),
        width: frame.width,
        height: frame.height,
        stride: frame.stride,
        pixel_format: frame.pixel_format,
        timestamp: frame.timestamp,
    };
    generate_thumbnail(&captured, thumb_path)
}

fn thumbnail_dimensions(width: u32, height: u32) -> (u32, u32) {
    const MAX_WIDTH: u32 = 1280;
    const MAX_HEIGHT: u32 = 720;

    if width == 0 || height == 0 {
        return (1, 1);
    }

    let scale = (MAX_WIDTH as f64 / width as f64)
        .min(MAX_HEIGHT as f64 / height as f64)
        .min(1.0);
    (
        (width as f64 * scale).round().max(1.0) as u32,
        (height as f64 * scale).round().max(1.0) as u32,
    )
}

#[cfg(test)]
mod thumbnail_tests {
    use super::thumbnail_dimensions;

    #[test]
    fn thumbnail_dimensions_preserve_720p_landscape() {
        assert_eq!(thumbnail_dimensions(1920, 1080), (1280, 720));
    }

    #[test]
    fn thumbnail_dimensions_fit_tall_sources() {
        assert_eq!(thumbnail_dimensions(1080, 1920), (405, 720));
    }
}
