//! Upload module — client communication, queue management, and background processing.

pub mod client;
pub mod queue;

use std::path::PathBuf;

use queue::{UploadMetadata, UploadQueue, UploadStatus};
use tauri::{AppHandle, Emitter, Manager};

use crate::auth::AuthManager;
use crate::settings::SettingsManager;

/// Start the background upload processor.
/// Spawns a task that polls the queue and processes pending uploads.
pub fn start_upload_processor(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut refreshed_this_session = false;

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            let settings = app.state::<SettingsManager>().get();
            let queue = app.state::<UploadQueue>();

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

            let clip_path = PathBuf::from(&task.clip_path);
            let task_id = task.id.clone();

            // Use live settings for auth — re-login updates the key without
            // invalidating already-enqueued upload tasks.
            let base_url = settings.cloud.server_url.trim_end_matches('/').to_string();
            if base_url.is_empty() {
                eprintln!("[upload] server_url not configured — skipping {}", task_id);
                queue.mark_failed(&task_id, "Server URL not configured".into());
                continue;
            }
            let upload_url = format!("{base_url}/api/clips/upload");
            let access_token = settings.cloud.access_token.clone();
            if access_token.is_empty() {
                eprintln!("[upload] access_token is empty — skipping {}", task_id);
                queue.mark_failed(&task_id, "Not authenticated — sign in first".into());
                continue;
            }

            if !clip_path.exists() {
                eprintln!(
                    "[upload] clip file not found on disk: {} — permanent failure",
                    clip_path.display()
                );
                queue.mark_permanent_failure(&task_id, "Clip file was deleted".into());
                continue;
            }

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

            let result = client::upload_clip(
                &upload_url,
                &clip_path,
                Some(&access_token),
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
            .await;

            match result {
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
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    eprintln!("[upload] failed {}: {}", task_id, err_msg);

                    // HTTP 401 may mean the JWT expired — try one refresh
                    if err_msg.contains("401") && !refreshed_this_session {
                        eprintln!("[upload] trying token refresh...");
                        match AuthManager::refresh_access_token(&app).await {
                            Ok(_) => {
                                eprintln!("[upload] token refreshed, will retry");
                                refreshed_this_session = true;
                                // Re-enqueue as pending so next loop picks it up
                                queue.mark_failed(&task_id, "Token expired, will retry".into());
                                // Force a retry by resetting the task to pending
                                // (mark_failed with retry_count < 3 will do this)
                                continue;
                            }
                            Err(refresh_err) => {
                                eprintln!("[upload] token refresh failed: {refresh_err}");
                                queue.mark_permanent_failure(
                                    &task_id,
                                    "Session expired — please sign in again".into(),
                                );
                                let _ = app.emit("auth-invalid", ());
                            }
                        }
                    } else if err_msg.contains("401") {
                        queue.mark_permanent_failure(
                            &task_id,
                            "Session expired — please sign in again".into(),
                        );
                        let _ = app.emit("auth-invalid", ());
                    } else {
                        queue.mark_failed(&task_id, err_msg.clone());
                    }

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
