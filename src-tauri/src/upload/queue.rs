//! Upload queue — in-memory tracking of pending/completed/failed uploads.

use std::sync::Mutex;

use serde::Serialize;

/// Status of a single upload task.
#[derive(Debug, Clone, Serialize)]
pub enum UploadStatus {
    Pending,
    Uploading,
    Completed,
    Failed(String),
}

/// An upload task in the queue.
#[derive(Debug, Clone, Serialize)]
pub struct UploadTask {
    pub id: String,
    pub clip_path: String,
    pub status: UploadStatus,
    pub progress: f32,  // 0.0 – 1.0
    pub started_at_secs: Option<u64>,
    pub server_url: Option<String>,
}

/// Manages the upload queue in memory.
pub struct UploadQueue {
    inner: Mutex<Vec<UploadTask>>,
}

impl UploadQueue {
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(Vec::new()),
        }
    }

    /// Add a clip to the upload queue.
    pub fn enqueue(&self, id: String, clip_path: String) {
        if let Ok(mut queue) = self.inner.lock() {
            queue.push(UploadTask {
                id,
                clip_path,
                status: UploadStatus::Pending,
                progress: 0.0,
                started_at_secs: None,
                server_url: None,
            });
        }
    }

    /// Get all tasks (for UI display).
    pub fn all(&self) -> Vec<UploadTask> {
        self.inner
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    /// Mark a task as started.
    pub fn mark_started(&self, id: &str) {
        if let Ok(mut queue) = self.inner.lock() {
            if let Some(task) = queue.iter_mut().find(|t| t.id == id) {
                task.status = UploadStatus::Uploading;
                task.started_at_secs = Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                );
            }
        }
    }

    /// Mark a task as completed.
    pub fn mark_completed(&self, id: &str) {
        if let Ok(mut queue) = self.inner.lock() {
            if let Some(task) = queue.iter_mut().find(|t| t.id == id) {
                task.status = UploadStatus::Completed;
                task.progress = 1.0;
            }
        }
    }

    /// Mark a task as failed.
    pub fn mark_failed(&self, id: &str, error: String) {
        if let Ok(mut queue) = self.inner.lock() {
            if let Some(task) = queue.iter_mut().find(|t| t.id == id) {
                task.status = UploadStatus::Failed(error);
            }
        }
    }

    /// Update progress for a task.
    pub fn update_progress(&self, id: &str, progress: f32) {
        if let Ok(mut queue) = self.inner.lock() {
            if let Some(task) = queue.iter_mut().find(|t| t.id == id) {
                task.progress = progress;
            }
        }
    }
}
