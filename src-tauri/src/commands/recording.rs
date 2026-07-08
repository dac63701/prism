//! Recording IPC commands — start, stop, save clip, status.

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, State};

use crate::capture::{enumerate_capture_sources, CaptureSources, CaptureTarget};
use crate::encoder::codecs::{Codec, EncoderConfig};
use crate::encoder::create_encoder;
use crate::recording::{Recorder, chrono_now_formatted};
use crate::settings::config::resolution_dimensions;
use crate::settings::SettingsManager;

/// Start the ring-buffer recording and spawn the polling task.
#[tauri::command]
pub async fn start_recording(
    app: AppHandle,
    recorder: State<'_, Mutex<Recorder>>,
) -> Result<String, String> {
    eprintln!("[recording] start_recording command invoked");

    let rec = recorder.lock().map_err(|e| {
        eprintln!("[recording] lock error: {e}");
        format!("Lock error: {e}")
    })?;

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
        let rec = recorder.lock().map_err(|e| {
            eprintln!("[recording] lock error: {e}");
            format!("Lock error: {e}")
        })?;
        rec.stop_recording()?;
    }

    let _ = app.emit("recording-state-changed", false);
    eprintln!("[recording] stop_recording command succeeded");
    Ok(())
}

/// Check whether recording is active.
#[tauri::command]
pub async fn is_recording(
    recorder: State<'_, Mutex<Recorder>>,
) -> Result<bool, String> {
    let rec = recorder.lock().map_err(|e| e.to_string())?;
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

    // Step 1: Extract frames under lock (brief — frame copy only)
    let clip_data = {
        let rec = recorder.lock().map_err(|e| {
            eprintln!("[recording] save_clip lock error: {e}");
            format!("Lock error: {e}")
        })?;
        rec.extract_clip_data(duration)?
    };
    // LOCK RELEASED — polling and other commands can proceed

    if clip_data.frames.is_empty() {
        return Err("No frames available to clip".into());
    }

    // Step 2: Build encoder config from settings
    let rs = &settings.recording;
    let (target_width, target_height) = resolution_dimensions(&rs.resolution);
    let enc_config = EncoderConfig {
        codec: Codec::H264,
        bitrate_kbps: rs.bitrate_kbps,
        fps: rs.fps,
        keyframe_interval: rs.fps.saturating_mul(2),
        target_width,
        target_height,
    };

    // Step 3: Generate output path
    let timestamp = chrono_now_formatted();
    let filename = format!("clip_{timestamp}.mp4");
    let output_path = clip_data.output_dir.join(&filename);

    // Ensure output directory exists
    std::fs::create_dir_all(&clip_data.output_dir)
        .map_err(|e| format!("Failed to create output directory: {e}"))?;

    // Step 4: Encode (NO lock held — polling continues)
    eprintln!("[recording] save_clip encoding {} frames to {}", clip_data.frames.len(), output_path.display());
    let mut encoder = create_encoder();
    encoder
        .encode_clip(&clip_data.frames, &output_path, &enc_config)
        .map_err(|e| format!("Encoding failed: {e}"))?;
    eprintln!("[recording] save_clip encoding complete");

    let _ = app.emit("clip-saved", &output_path.to_string_lossy().to_string());

    Ok(output_path.to_string_lossy().to_string())
}

/// Get a live preview frame as a JPEG base64 data URL.
/// Returns `null` when not recording or no frame available.
#[tauri::command]
pub async fn get_preview_frame(
    recorder: State<'_, Mutex<Recorder>>,
) -> Result<Option<String>, String> {
    let rec = recorder.lock().map_err(|e| e.to_string())?;
    Ok(rec.get_preview_frame())
}

/// Get the current frame count in the ring buffer.
#[tauri::command]
pub async fn get_buffer_info(
    recorder: State<'_, Mutex<Recorder>>,
) -> Result<serde_json::Value, String> {
    let rec = recorder.lock().map_err(|e| e.to_string())?;
    let fc = rec.frame_count();
    let fr = rec.total_frames_received();
    let clip_len = rec.buffer_duration_secs();
    Ok(serde_json::json!({
        "frame_count": fc,
        "clip_length_seconds": clip_len,
        "is_recording": rec.is_recording(),
        "frames_received": fr,
        "preview_available": rec.preview_available(),
        "polling_active": true,
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
    settings_mgr.set(&app, settings)
        .map_err(|e| format!("Failed to save settings: {e}"))?;

    // Reconfigure recorder with new target
    {
        let rec = recorder.lock().map_err(|e| e.to_string())?;
        if rec.is_recording() {
            rec.stop_recording().ok();
            rec.reconfigure_target(target);
            rec.start_recording()?;
        } else {
            rec.reconfigure_target(target);
        }

        let _ = app.emit("recording-state-changed", rec.is_recording());
    }

    Ok(())
}
