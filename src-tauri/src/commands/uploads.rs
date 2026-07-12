use std::path::PathBuf;

use tauri::{AppHandle, Emitter, State};

use crate::recording::chrono_now_formatted;
use crate::settings::SettingsManager;
use crate::upload::queue::{UploadMetadata, UploadQueue};

/// Enqueue a clip for upload to the configured server.
#[tauri::command]
pub async fn upload_clip(
    app: AppHandle,
    queue: State<'_, UploadQueue>,
    settings_mgr: State<'_, SettingsManager>,
    path: String,
    filename: String,
    game: String,
) -> Result<(), String> {
    let settings = settings_mgr.get();
    if settings.cloud.access_token.is_empty() {
        return Err("Not authenticated — sign in first".into());
    }
    if settings.cloud.server_url.is_empty() {
        return Err("Server URL not configured".into());
    }

    let clip_path = PathBuf::from(&path);
    if !clip_path.exists() {
        return Err(format!("Clip not found: {path}"));
    }

    let size_bytes = std::fs::metadata(&clip_path).map(|m| m.len()).unwrap_or(0);

    let duration_secs = read_mp4_duration(&clip_path).unwrap_or(0);
    let (width, height) = read_mp4_resolution(&clip_path).unwrap_or((0, 0));

    let codec = "h264".to_string();
    let title = filename
        .strip_suffix(".mp4")
        .unwrap_or(&filename)
        .to_string();

    let clip_path_for_event = path.clone();
    let task_id = format!("upload_{}", chrono_now_formatted());
    queue.enqueue_with_meta(
        task_id.clone(),
        path,
        settings.cloud.server_url.clone(),
        settings.cloud.api_key.clone(),
        UploadMetadata {
            title,
            game,
            duration_secs: duration_secs as f64,
            width,
            height,
            codec,
            size_bytes,
        },
    );

    let _ = app.emit(
        "upload-progress",
        serde_json::json!({
            "id": task_id,
            "status": "Pending",
            "progress": 0.0,
            "clip_path": clip_path_for_event,
        }),
    );

    Ok(())
}

/// Get the current state of the upload queue.
#[tauri::command]
pub async fn upload_queue_status(
    queue: State<'_, UploadQueue>,
) -> Result<Vec<crate::upload::queue::UploadTask>, String> {
    let all = queue.all();
    Ok(all)
}

/// Cancel a pending upload.
#[tauri::command]
pub async fn cancel_upload(queue: State<'_, UploadQueue>, task_id: String) -> Result<(), String> {
    queue.cancel(&task_id);
    Ok(())
}

/// Retry a failed upload.
#[tauri::command]
pub async fn retry_upload(
    app: AppHandle,
    queue: State<'_, UploadQueue>,
    task_id: String,
) -> Result<(), String> {
    queue.retry(&task_id);
    let _ = app.emit(
        "upload-progress",
        serde_json::json!({
            "id": task_id,
            "status": "Pending",
            "progress": 0.0,
        }),
    );
    Ok(())
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn read_mp4_duration(path: &PathBuf) -> Option<u32> {
    use std::fs::File;
    use std::io::BufReader;

    let file = File::open(path).ok()?;
    let size = file.metadata().ok()?.len();
    let reader = BufReader::new(file);
    let mp4 = mp4::Mp4Reader::read_header(reader, size).ok()?;
    Some(mp4.duration().as_secs() as u32)
}

fn read_mp4_resolution(path: &PathBuf) -> Option<(u32, u32)> {
    use std::fs::File;
    use std::io::BufReader;

    let file = File::open(path).ok()?;
    let size = file.metadata().ok()?.len();
    let reader = BufReader::new(file);
    let mp4 = mp4::Mp4Reader::read_header(reader, size).ok()?;
    let track = mp4.tracks().values().next()?;
    Some((track.width() as u32, track.height() as u32))
}
