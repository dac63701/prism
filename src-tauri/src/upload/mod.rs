//! Upload module — client communication, queue management, and background processing.

pub mod client;
pub mod queue;

use std::path::PathBuf;

use queue::{UploadMetadata, UploadQueue, UploadStatus};
use tauri::{AppHandle, Emitter, Manager};

use crate::settings::SettingsManager;

/// Start the background upload processor.
/// Spawns a task that polls the queue and processes pending uploads.
pub fn start_upload_processor(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            let queue = app.state::<UploadQueue>();
            let settings = app.state::<SettingsManager>().get();

            // Check concurrent upload limit
            let active = queue
                .all()
                .iter()
                .filter(|t| t.status == UploadStatus::Uploading)
                .count() as u32;
            if active >= settings.cloud.max_concurrent_uploads {
                continue;
            }

            let task = match queue.next_pending() {
                Some(t) => t,
                None => continue,
            };

            let base_url = match &task.server_url {
                Some(u) => u.trim_end_matches('/').to_string(),
                None => continue,
            };
            let upload_url = format!("{base_url}/api/clips/upload");
            let api_key = task.api_key.clone().unwrap_or_default();
            let clip_path = PathBuf::from(&task.clip_path);
            let task_id = task.id.clone();

            eprintln!(
                "[upload] uploading {} to {}",
                clip_path.display(),
                upload_url
            );

            let _ = app.emit(
                "upload-progress",
                serde_json::json!({
                    "id": task_id,
                    "status": "Uploading",
                    "progress": 0.0,
                    "clip_path": task.clip_path,
                }),
            );

            match client::upload_clip(
                &upload_url,
                &clip_path,
                Some(&api_key),
                &UploadMetadata {
                    title: task.title.clone(),
                    game: task.game.clone(),
                    duration_secs: task.duration_secs,
                    width: task.width,
                    height: task.height,
                    codec: task.codec.clone(),
                    size_bytes: task.size_bytes,
                },
            )
            .await
            {
                Ok(response) => {
                    queue.mark_completed(&task_id);
                    queue.set_share_url(&task_id, response.share_url.clone());

                    eprintln!("[upload] completed {}: {}", task_id, response.share_url);

                    let full_share_url = format!("{}{}", base_url, response.share_url);
                    let _ = app.emit(
                        "upload-progress",
                        serde_json::json!({
                            "id": task_id,
                            "status": "Completed",
                            "progress": 1.0,
                            "share_url": full_share_url,
                            "clip_path": task.clip_path,
                        }),
                    );

                    // Share URL is included in the event payload for frontend to copy
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    eprintln!("[upload] failed {}: {}", task_id, err_msg);
                    queue.mark_failed(&task_id, err_msg.clone());

                    let _ = app.emit(
                        "upload-progress",
                        serde_json::json!({
                            "id": task_id,
                            "status": "Failed",
                            "progress": 0.0,
                            "error": err_msg,
                            "clip_path": task.clip_path,
                        }),
                    );
                }
            }
        }
    });
}
