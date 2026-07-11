use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

const PERSIST_FILE: &str = "upload_queue.json";

/// Status of a single upload task.
#[derive(Debug, Clone, PartialEq)]
pub enum UploadStatus {
    Pending,
    Uploading,
    Completed,
    Failed(String),
    Cancelled,
}

impl Serialize for UploadStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match self {
            UploadStatus::Pending => "Pending",
            UploadStatus::Uploading => "Uploading",
            UploadStatus::Completed => "Completed",
            UploadStatus::Failed(_) => "Failed",
            UploadStatus::Cancelled => "Cancelled",
        };
        serializer.serialize_str(s)
    }
}

impl<'de> Deserialize<'de> for UploadStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "Pending" => Ok(UploadStatus::Pending),
            "Uploading" => Ok(UploadStatus::Uploading),
            "Completed" => Ok(UploadStatus::Completed),
            "Failed" => Ok(UploadStatus::Failed("Unknown error".into())),
            "Cancelled" => Ok(UploadStatus::Cancelled),
            _ => Ok(UploadStatus::Failed(format!("Unknown status: {s}"))),
        }
    }
}

/// An upload task in the queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadTask {
    pub id: String,
    pub clip_path: String,
    pub status: UploadStatus,
    pub progress: f32,
    pub started_at_secs: Option<u64>,
    pub server_url: Option<String>,
    pub api_key: Option<String>,
    pub title: String,
    pub game: String,
    pub duration_secs: f64,
    pub width: u32,
    pub height: u32,
    pub codec: String,
    pub size_bytes: u64,
    pub retry_count: u32,
    pub share_url: Option<String>,
    pub error: Option<String>,
}

/// Metadata collected from a clip before it is queued for upload.
#[derive(Debug, Clone)]
pub struct UploadMetadata {
    pub title: String,
    pub game: String,
    pub duration_secs: f64,
    pub width: u32,
    pub height: u32,
    pub codec: String,
    pub size_bytes: u64,
}

/// Manages the upload queue in memory with optional disk persistence.
pub struct UploadQueue {
    inner: Mutex<Vec<UploadTask>>,
    persist_path: Mutex<Option<PathBuf>>,
}

impl UploadQueue {
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(Vec::new()),
            persist_path: Mutex::new(None),
        }
    }

    /// Set the persist path and load existing tasks.
    pub fn set_persist_path(&self, app_data: PathBuf) {
        let path = app_data.join(PERSIST_FILE);
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(tasks) = serde_json::from_str::<Vec<UploadTask>>(&content) {
                if let Ok(mut queue) = self.inner.lock() {
                    // Only load pending/failed tasks (resume incomplete ones)
                    for task in tasks {
                        if matches!(
                            task.status,
                            UploadStatus::Pending
                                | UploadStatus::Uploading
                                | UploadStatus::Failed(_)
                        ) {
                            queue.push(UploadTask {
                                status: UploadStatus::Pending,
                                progress: 0.0,
                                started_at_secs: None,
                                error: None,
                                ..task
                            });
                        }
                    }
                }
            }
        }
        if let Ok(mut pp) = self.persist_path.lock() {
            *pp = Some(path);
        }
    }

    fn persist(&self) {
        if let Ok(pp) = self.persist_path.lock() {
            if let Some(path) = pp.as_ref() {
                if let Ok(queue) = self.inner.lock() {
                    let pending: Vec<&UploadTask> = queue
                        .iter()
                        .filter(|t| {
                            matches!(
                                t.status,
                                UploadStatus::Pending
                                    | UploadStatus::Uploading
                                    | UploadStatus::Failed(_)
                            )
                        })
                        .collect();
                    if let Ok(json) = serde_json::to_string(&pending) {
                        let _ = std::fs::write(path, &json);
                    }
                }
            }
        }
    }

    /// Add a clip to the upload queue with full metadata.
    pub fn enqueue_with_meta(
        &self,
        id: String,
        clip_path: String,
        server_url: String,
        api_key: String,
        metadata: UploadMetadata,
    ) {
        if let Ok(mut queue) = self.inner.lock() {
            queue.push(UploadTask {
                id,
                clip_path,
                status: UploadStatus::Pending,
                progress: 0.0,
                started_at_secs: None,
                server_url: Some(server_url),
                api_key: Some(api_key),
                title: metadata.title,
                game: metadata.game,
                duration_secs: metadata.duration_secs,
                width: metadata.width,
                height: metadata.height,
                codec: metadata.codec,
                size_bytes: metadata.size_bytes,
                retry_count: 0,
                share_url: None,
                error: None,
            });
        }
        self.persist();
    }

    /// Get all tasks (for UI display).
    pub fn all(&self) -> Vec<UploadTask> {
        self.inner.lock().map(|g| g.clone()).unwrap_or_default()
    }

    /// Get the next pending task.
    pub fn next_pending(&self) -> Option<UploadTask> {
        self.inner.lock().ok().and_then(|mut queue| {
            let idx = queue.iter().position(|t| t.status == UploadStatus::Pending);
            idx.map(|i| {
                queue[i].status = UploadStatus::Uploading;
                queue[i].started_at_secs = Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                );
                queue[i].clone()
            })
        })
    }

    /// Mark a task as completed.
    pub fn mark_completed(&self, id: &str) {
        if let Ok(mut queue) = self.inner.lock() {
            if let Some(task) = queue.iter_mut().find(|t| t.id == id) {
                task.status = UploadStatus::Completed;
                task.progress = 1.0;
            }
        }
        self.persist();
    }

    /// Mark a task as failed with optional retry.
    pub fn mark_failed(&self, id: &str, error: String) {
        if let Ok(mut queue) = self.inner.lock() {
            if let Some(task) = queue.iter_mut().find(|t| t.id == id) {
                if task.retry_count < 3 {
                    task.retry_count += 1;
                    task.status = UploadStatus::Pending;
                    task.progress = 0.0;
                    task.started_at_secs = None;
                    task.error = None;
                } else {
                    task.status = UploadStatus::Failed(error.clone());
                    task.error = Some(error);
                }
            }
        }
        self.persist();
    }

    /// Update progress for a task.
    pub fn update_progress(&self, id: &str, progress: f32) {
        if let Ok(mut queue) = self.inner.lock() {
            if let Some(task) = queue.iter_mut().find(|t| t.id == id) {
                task.progress = progress;
            }
        }
    }

    /// Cancel a pending or in-progress upload.
    pub fn cancel(&self, id: &str) {
        if let Ok(mut queue) = self.inner.lock() {
            if let Some(task) = queue.iter_mut().find(|t| t.id == id) {
                task.status = UploadStatus::Cancelled;
            }
        }
        self.persist();
    }

    /// Retry a failed upload.
    pub fn retry(&self, id: &str) {
        if let Ok(mut queue) = self.inner.lock() {
            if let Some(task) = queue.iter_mut().find(|t| t.id == id) {
                if matches!(task.status, UploadStatus::Failed(_)) {
                    task.status = UploadStatus::Pending;
                    task.progress = 0.0;
                    task.started_at_secs = None;
                    task.retry_count = 0;
                    task.error = None;
                }
            }
        }
        self.persist();
    }

    /// Set the share URL for a completed upload.
    pub fn set_share_url(&self, id: &str, url: String) {
        if let Ok(mut queue) = self.inner.lock() {
            if let Some(task) = queue.iter_mut().find(|t| t.id == id) {
                task.share_url = Some(url);
            }
        }
    }

    /// Clean up completed tasks older than a given duration.
    pub fn cleanup_completed(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let one_day_secs = 86400u64;

        if let Ok(mut queue) = self.inner.lock() {
            queue.retain(|t| {
                if matches!(t.status, UploadStatus::Completed) {
                    if let Some(started) = t.started_at_secs {
                        if now.saturating_sub(started) > one_day_secs {
                            return false;
                        }
                    }
                }
                true
            });
        }
        self.persist();
    }
}
