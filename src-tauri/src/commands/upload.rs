//! Upload IPC commands — trigger uploads and query queue status.

use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{Emitter, State};

use crate::settings::SettingsManager;
use crate::upload::client::upload_clip;
use crate::upload::queue::UploadQueue;

fn generate_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("up_{:x}", nanos)
}

/// Upload a saved clip to the configured Prism server.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn upload_clip_to_server(
    app: tauri::AppHandle,
    settings_mgr: State<'_, SettingsManager>,
    queue: State<'_, UploadQueue>,
    clip_path: String,
    title: String,
    game: String,
    duration_secs: f64,
    width: u32,
    height: u32,
) -> Result<String, String> {
    let settings = settings_mgr.get();
    let upload_cfg = settings.upload;

    if upload_cfg.server_url.is_empty() || upload_cfg.api_key.is_empty() {
        return Err("Server URL and API key must be configured in Settings".into());
    }

    let task_id = generate_id();
    queue.enqueue(task_id.clone(), clip_path.clone());

    let path = std::path::PathBuf::from(&clip_path);
    queue.mark_started(&task_id);

    let result = upload_clip(
        &upload_cfg.server_url,
        &upload_cfg.api_key,
        &path,
        &title,
        &game,
        duration_secs,
        width,
        height,
    )
    .await;

    match result {
        Ok(response) => {
            queue.mark_completed(&task_id);
            let _ = app.emit(
                "upload-completed",
                serde_json::json!({
                    "task_id": task_id,
                    "clip_path": clip_path,
                }),
            );
            Ok(response)
        }
        Err(e) => {
            let err_msg = e.to_string();
            queue.mark_failed(&task_id, err_msg.clone());
            let _ = app.emit(
                "upload-failed",
                serde_json::json!({
                    "task_id": task_id,
                    "clip_path": clip_path,
                    "error": err_msg,
                }),
            );
            Err(err_msg)
        }
    }
}

/// Get the current upload queue status.
#[tauri::command]
pub async fn get_upload_queue(
    queue: State<'_, UploadQueue>,
) -> Result<Vec<crate::upload::queue::UploadTask>, String> {
    Ok(queue.all())
}

/// Clear completed uploads from the queue.
#[tauri::command]
pub async fn clear_upload_queue(queue: State<'_, UploadQueue>) -> Result<(), String> {
    queue.clear_completed();
    Ok(())
}
