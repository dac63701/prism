//! Recording IPC commands — start, stop, save clip, status.

use tauri::{AppHandle, Emitter, Manager, State};

use std::path::Path;

use crate::capture::{enumerate_capture_sources, CaptureSources, CaptureTarget, PixelFormat};
use crate::encoder::codecs::{Codec, EncoderConfig};
use crate::encoder::create_encoder;
use crate::recording::{chrono_now_formatted, Recorder};
use crate::settings::SettingsManager;

/// Start the ring-buffer recording and spawn the polling task.
#[tauri::command]
pub async fn start_recording(
    app: AppHandle,
    recorder: State<'_, Recorder>,
) -> Result<String, String> {
    eprintln!("[recording] start_recording command invoked");

    recorder.start_recording().map_err(|e| {
        eprintln!("[recording] start_recording failed: {e}");
        e
    })?;

    recorder.start_polling(app.clone());
    // Always emit the event — polling spawn status is internal
    let _ = app.emit("recording-state-changed", true);

    eprintln!("[recording] start_recording command succeeded");
    Ok("started".to_string())
}

/// Stop recording.
#[tauri::command]
pub async fn stop_recording(app: AppHandle, recorder: State<'_, Recorder>) -> Result<(), String> {
    eprintln!("[recording] stop_recording command invoked");

    recorder.stop_recording()?;

    let _ = app.emit("recording-state-changed", false);
    eprintln!("[recording] stop_recording command succeeded");
    Ok(())
}

/// Check whether recording is active.
#[tauri::command]
pub async fn is_recording(recorder: State<'_, Recorder>) -> Result<bool, String> {
    Ok(recorder.is_recording())
}

/// Trigger a clip save — extracts frames under the recorder lock (briefly),
/// then encodes to MP4 OUTSIDE the lock so the polling task keeps running.
#[tauri::command]
pub async fn save_clip(
    app: AppHandle,
    _recorder: State<'_, Recorder>,
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
    let worker_app = app.clone();
    tokio::task::spawn_blocking(move || {
        let recorder = worker_app.state::<Recorder>();
        save_clip_internal(&worker_app, &recorder, &settings, duration, filename)
    })
    .await
    .map_err(|e| format!("Clip save worker failed: {e}"))?
}

/// Save a clip from an internal event source while retaining the same safe
/// extract-then-encode behavior used by the manual Tauri command.
pub(crate) fn save_clip_internal(
    app: &AppHandle,
    recorder: &Recorder,
    settings: &crate::settings::config::AppSettings,
    duration: u32,
    filename: String,
) -> Result<String, String> {
    let prof = std::env::var("PRISM_PROF").as_deref() == Ok("1");
    let t_start = if prof {
        Some(std::time::Instant::now())
    } else {
        None
    };

    // Step 1: Extract frames under the recorder's brief internal lock.
    let clip_data = recorder.extract_clip_data(duration)?;

    if clip_data.frames.is_empty() {
        return Err("No frames available to clip".into());
    }

    // Step 2: Build encoder config from settings
    let rs = &settings.recording;
    let first = &clip_data.frames[0];
    // Buffered H.264 packets are already encoded. Their actual dimensions,
    // rather than a potentially different current setting, define the MP4.
    let (target_width, target_height) = (first.width, first.height);
    let enc_config = EncoderConfig {
        codec: Codec::H264,
        bitrate_kbps: rs.bitrate_kbps,
        fps: recorder.cached_fps(),
        keyframe_interval: recorder.cached_fps(),
        target_width,
        target_height,
        audio: None,
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
    // If no keyframe is buffered yet (first ~GOP of recording, or a stale sync
    // frame evicted from the byte budget), request a fresh keyframe from the
    // capture loop, wait for it to land, and re-extract before giving up.
    eprintln!(
        "[recording] save_clip: {} frames, sps={} pps={}",
        clip_data.frames.len(),
        clip_data.sps.len(),
        clip_data.pps.len()
    );
    let mut frames = clip_data.frames;
    let mut sps = clip_data.sps;
    let mut pps = clip_data.pps;
    let mut frames_with_sps: Option<Vec<crate::buffer::StoredFrame>> = None;
    let mut last_keyframe_err: Option<String> = None;
    for attempt in 0..5u32 {
        match prepare_h264_clip_frames(std::mem::take(&mut frames), &sps, &pps) {
            Ok(prepared) => {
                frames_with_sps = Some(prepared);
                break;
            }
            Err(e) if e.contains("keyframe") && attempt < 4 => {
                eprintln!(
                    "[recording] no H.264 keyframe in buffer (attempt {}); requesting one and retrying",
                    attempt + 1
                );
                last_keyframe_err = Some(e);
                recorder.request_keyframe();
                std::thread::sleep(std::time::Duration::from_millis(150));
                let next = recorder.extract_clip_data(duration)?;
                frames = next.frames;
                sps = next.sps;
                pps = next.pps;
            }
            Err(e) => return Err(e),
        }
    }
    let frames_with_sps = frames_with_sps.ok_or_else(|| {
        last_keyframe_err.unwrap_or_else(|| {
            "No H.264 keyframe is available yet. Keep recording for a moment and try again."
                .to_string()
        })
    })?;
    let t_extract = if prof {
        Some(std::time::Instant::now())
    } else {
        None
    };

    // Step 6: Extract system audio for the exact video window and AAC-encode
    // it outside the recorder lock. Missing/disabled audio is not fatal — the
    // clip simply saves without an audio track.
    let enc_config = {
        let mut cfg = enc_config;
        cfg.audio = extract_and_encode_audio(recorder, &frames_with_sps, &cfg)?;
        cfg
    };
    let t_audio = if prof {
        Some(std::time::Instant::now())
    } else {
        None
    };

    // Step 7: Encode (NO lock held — polling continues)
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

    if let (Some(t_start), Some(t_extract), Some(t_audio)) = (t_start, t_extract, t_audio) {
        let extract = t_extract.duration_since(t_start);
        let audio = t_audio.duration_since(t_extract);
        let mux = t_audio.elapsed();
        eprintln!("[prof] clip: extract={extract:?} audio={audio:?} mux={mux:?}");
    }

    let output_str = output_path.to_string_lossy().to_string();
    let _ = app.emit("clip-saved", &output_str);

    Ok(output_str)
}

/// Extract the system-audio window matching `frames` and AAC-encode it.
///
/// The window is `[first.timestamp, last.timestamp + frame_duration)` so the
/// audio track lines up with the video timeline. Returns `None` when audio is
/// disabled, unavailable, or encoding fails (the clip saves video-only).
fn extract_and_encode_audio(
    recorder: &Recorder,
    frames: &[crate::buffer::StoredFrame],
    cfg: &EncoderConfig,
) -> Result<Option<crate::encoder::codecs::AudioClip>, String> {
    #[cfg(target_os = "windows")]
    {
        let Some(first) = frames.first() else {
            return Ok(None);
        };
        let Some(last) = frames.last() else {
            return Ok(None);
        };
        let frame_dur = std::time::Duration::from_secs_f64(1.0 / cfg.fps.max(1) as f64);
        let start = first.timestamp;
        let end = last.timestamp + frame_dur;

        let Some(pcm) = recorder.extract_clip_audio(start, end) else {
            eprintln!("[recording] no system audio available for clip window");
            return Ok(None);
        };
        if pcm.is_empty() {
            return Ok(None);
        }

        let bitrate = if cfg.bitrate_kbps >= 160 { 128 } else { 96 };
        eprintln!(
            "[recording] encoding {} KB of system audio ({:.2}s)",
            pcm.len() / 1024,
            pcm.len() as f64
                / (crate::audio::SAMPLE_RATE as f64 * crate::audio::BYTES_PER_FRAME as f64)
        );
        match crate::audio::aac::encode_clip_audio(
            &pcm,
            crate::audio::SAMPLE_RATE,
            crate::audio::CHANNELS,
            bitrate,
        ) {
            Ok(audio) if !audio.frames.is_empty() => {
                eprintln!(
                    "[recording] audio encoded: {} AAC frames @ {} kbps",
                    audio.frames.len(),
                    audio.bitrate_kbps
                );
                Ok(Some(audio))
            }
            Ok(_) => {
                eprintln!("[recording] AAC encoder produced no frames");
                Ok(None)
            }
            Err(e) => {
                eprintln!("[recording] AAC encoding failed: {e}");
                Ok(None)
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (recorder, frames, cfg);
        Ok(None)
    }
}

/// Whether a frame is a decodable H.264 sync point.
///
/// Prefers the encoder's `is_sync` flag, but falls back to scanning the AVCC
/// data for an IDR slice (NAL type 5) or SPS (NAL type 7) — some hardware
/// encoders omit the CleanPoint attribute on keyframe output samples, and the
/// first VideoToolbox keyframe is never flagged on macOS.
fn frame_is_h264_sync(frame: &crate::buffer::StoredFrame) -> bool {
    if frame.pixel_format != PixelFormat::H264 {
        return false;
    }
    if frame.is_sync {
        return true;
    }
    let data = &*frame.data;
    let mut offset = 0;
    while offset + 4 <= data.len() {
        let nal_len =
            u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap_or_default()) as usize;
        let nal_start = offset + 4;
        let nal_end = nal_start + nal_len;
        if nal_len == 0 || nal_end > data.len() {
            return false;
        }
        if let Some(5 | 7) = data.get(nal_start).map(|b| b & 0x1F) {
            return true;
        }
        offset = nal_end;
    }
    false
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
    let first_sync = frames.iter().position(frame_is_h264_sync).ok_or_else(|| {
        let has_h264 = frames
            .iter()
            .any(|frame| frame.pixel_format == PixelFormat::H264);
        if has_h264 {
            "No H.264 keyframe is available yet. Keep recording for a moment and try again."
                .to_string()
        } else {
            "No H.264 keyframe is available — the buffer holds only raw fallback frames (H.264 encoder unavailable). Check the encoder and try again."
                .to_string()
        }
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

    // The muxer uses `is_sync` to build the MP4 sync-sample table. A frame may
    // have been selected via content inspection (IDR/SPS NALs) even when the
    // encoder failed to flag it, so make sure the first sample is marked sync.
    if let Some(first) = h264_frames.first_mut() {
        first.is_sync = true;
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
pub async fn get_preview_frame(recorder: State<'_, Recorder>) -> Result<Option<String>, String> {
    Ok(recorder.get_preview_frame())
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

    #[test]
    fn clip_preparation_detects_keyframes_by_idr_content_when_flag_missing() {
        // is_sync is false on every frame, but the second packet contains an
        // IDR NAL (type 5) — some encoders omit the CleanPoint attribute.
        let idr_sample = vec![0, 0, 0, 3, 0x65, 0x88, 0x84];
        let next_sample = vec![0, 0, 0, 2, 0x41, 0x9A];
        let frames = vec![
            frame(PixelFormat::H264, false, next_sample.clone()),
            frame(PixelFormat::H264, false, idr_sample.clone()),
            frame(PixelFormat::H264, false, next_sample.clone()),
        ];

        let prepared = prepare_h264_clip_frames(frames, &[], &[]).unwrap();

        assert_eq!(prepared.len(), 2);
        assert!(
            prepared[0].is_sync,
            "content-detected keyframe must be flagged sync"
        );
        assert_eq!(prepared[0].data.as_slice(), idr_sample);
        assert_eq!(prepared[1].data.as_slice(), next_sample);
    }

    #[test]
    fn clip_preparation_reports_encoder_unavailable_when_only_raw_frames() {
        let frames = vec![
            frame(PixelFormat::Nv12, true, vec![0; 8]),
            frame(PixelFormat::Nv12, true, vec![0; 8]),
        ];

        let error = prepare_h264_clip_frames(frames, &[], &[]).unwrap_err();

        assert!(error.contains("keyframe"));
        assert!(error.contains("raw fallback"));
    }
}

/// Get the current frame count in the ring buffer.
#[tauri::command]
pub async fn get_buffer_info(recorder: State<'_, Recorder>) -> Result<serde_json::Value, String> {
    let fc = recorder.frame_count();
    let fr = recorder.total_frames_received();
    let fps = recorder.cached_fps();
    let clip_len = recorder.available_clip_secs();
    let actual_buffer_time = recorder.buffer_time_secs();
    let elapsed = recorder.recording_elapsed_secs();
    Ok(serde_json::json!({
        "frame_count": fc,
        "buffer_time_seconds": clip_len,
        "clip_length_seconds": clip_len,
        "actual_buffer_time_seconds": actual_buffer_time,
        "configured_duration_seconds": recorder.buffer_duration_secs(),
        "is_recording": recorder.is_recording(),
        "frames_received": fr,
        "preview_available": recorder.preview_available(),
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

/// Refresh rate of the main display in Hz (0 if undetectable).
#[tauri::command]
pub async fn get_display_refresh_rate() -> Result<u32, String> {
    Ok(crate::capture::primary_display_refresh_rate())
}

/// Set the capture target (display, window, or application).
/// Saves to settings and reconfigures the recorder.
/// Accepts target as a JSON string matching serde externally-tagged format,
/// e.g. `"display"` for main display or `{"display_id":5}` for a specific display.
#[tauri::command]
pub async fn set_capture_target(
    app: AppHandle,
    recorder: State<'_, Recorder>,
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
    let was_recording = recorder.is_recording();
    if was_recording {
        recorder.stop_recording().ok();
        recorder.reconfigure_target(target);
        let started = recorder.start_recording();
        if started.is_ok() {
            recorder.start_polling(app.clone());
        }
        let _ = app.emit("recording-state-changed", started.is_ok());
    } else {
        recorder.reconfigure_target(target);
        let _ = app.emit("recording-state-changed", false);
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
