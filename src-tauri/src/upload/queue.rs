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
    /// Only resumes Pending/Uploading tasks whose clip files still exist on disk.
    /// Permanently failed tasks are never resurrected.
    pub fn set_persist_path(&self, app_data: PathBuf) {
        let path = app_data.join(PERSIST_FILE);
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(tasks) = serde_json::from_str::<Vec<UploadTask>>(&content) {
                if let Ok(mut queue) = self.inner.lock() {
                    for task in tasks {
                        if matches!(task.status, UploadStatus::Pending | UploadStatus::Uploading) {
                            let clip_path = PathBuf::from(&task.clip_path);
                            if !clip_path.exists() {
                                eprintln!(
                                    "[upload] skipping deleted clip on reload: {}",
                                    clip_path.display()
                                );
                                continue;
                            }
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
                if task.retry_count < 2 {
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
    #[allow(dead_code)]
    pub fn update_progress(&self, id: &str, progress: f32) {
        if let Ok(mut queue) = self.inner.lock() {
            if let Some(task) = queue.iter_mut().find(|t| t.id == id) {
                task.progress = progress;
            }
        }
    }

    /// Mark a task as permanently failed (no retry).
    pub fn mark_permanent_failure(&self, id: &str, error: String) {
        if let Ok(mut queue) = self.inner.lock() {
            if let Some(task) = queue.iter_mut().find(|t| t.id == id) {
                task.status = UploadStatus::Failed(error.clone());
                task.retry_count = 99;
                task.error = Some(error);
            }
        }
        self.persist();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_metadata() -> UploadMetadata {
        UploadMetadata {
            title: "test_clip".into(),
            game: "TestGame".into(),
            duration_secs: 30.0,
            width: 1920,
            height: 1080,
            codec: "h264".into(),
            size_bytes: 1_000_000,
        }
    }

    fn make_queue() -> UploadQueue {
        let q = UploadQueue::new();
        q.enqueue_with_meta(
            "task_1".into(),
            "/clips/clip_1.mp4".into(),
            "https://example.com".into(),
            "prism_test_key".into(),
            make_metadata(),
        );
        q
    }

    #[test]
    fn test_enqueue_and_all() {
        let q = make_queue();
        let all = q.all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "task_1");
        assert_eq!(all[0].status, UploadStatus::Pending);
    }

    #[test]
    fn test_next_pending_marks_uploading() {
        let q = make_queue();
        let task = q.next_pending();
        assert!(task.is_some());
        assert_eq!(task.unwrap().id, "task_1");

        let all = q.all();
        assert_eq!(all[0].status, UploadStatus::Uploading);
        assert!(all[0].started_at_secs.is_some());
    }

    #[test]
    fn test_next_pending_empty() {
        let q = UploadQueue::new();
        assert!(q.next_pending().is_none());
    }

    #[test]
    fn test_next_pending_skips_non_pending() {
        let q = UploadQueue::new();
        q.enqueue_with_meta(
            "task_1".into(),
            "/clips/clip_1.mp4".into(),
            "https://example.com".into(),
            "key".into(),
            make_metadata(),
        );
        q.mark_completed("task_1");
        assert!(q.next_pending().is_none());
    }

    #[test]
    fn test_mark_completed() {
        let q = make_queue();
        let _ = q.next_pending();
        q.mark_completed("task_1");

        let all = q.all();
        assert_eq!(all[0].status, UploadStatus::Completed);
        assert!((all[0].progress - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_mark_failed_retries_then_permanent() {
        let q = make_queue();
        q.mark_failed("task_1", "error 1".into());

        let all = q.all();
        assert_eq!(
            all[0].status,
            UploadStatus::Pending,
            "should retry on 1st failure"
        );
        assert_eq!(all[0].retry_count, 1);

        q.mark_failed("task_1", "error 2".into());
        let all = q.all();
        assert_eq!(
            all[0].status,
            UploadStatus::Pending,
            "should retry on 2nd failure"
        );
        assert_eq!(all[0].retry_count, 2);

        q.mark_failed("task_1", "final error".into());
        let all = q.all();
        assert_eq!(all[0].status, UploadStatus::Failed("final error".into()));
        assert_eq!(all[0].error.as_deref(), Some("final error"));
    }

    #[test]
    fn test_cancel() {
        let q = make_queue();
        q.cancel("task_1");
        let all = q.all();
        assert_eq!(all[0].status, UploadStatus::Cancelled);
    }

    #[test]
    fn test_retry_resets_failed() {
        let q = make_queue();
        q.mark_failed("task_1", "err".into());
        q.mark_failed("task_1", "err".into());
        q.mark_failed("task_1", "err".into());
        q.mark_failed("task_1", "permanent".into());

        let all = q.all();
        assert_eq!(all[0].status, UploadStatus::Failed("permanent".into()));

        q.retry("task_1");
        let all = q.all();
        assert_eq!(all[0].status, UploadStatus::Pending);
        assert_eq!(all[0].retry_count, 0);
        assert!(all[0].error.is_none());
    }

    #[test]
    fn test_set_share_url() {
        let q = make_queue();
        q.set_share_url("task_1", "/s/abc123".into());
        let all = q.all();
        assert_eq!(all[0].share_url.as_deref(), Some("/s/abc123"));
    }

    #[test]
    fn test_update_progress() {
        let q = make_queue();
        q.update_progress("task_1", 0.5);
        let all = q.all();
        assert!((all[0].progress - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_persistence_roundtrip() {
        let dir = std::env::temp_dir().join("prism_upload_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("upload_queue.json");
        let clip_path = dir.join("persist.mp4");
        let _ = std::fs::write(&clip_path, b"dummy mp4 content");

        let q = UploadQueue::new();
        q.enqueue_with_meta(
            "persist_1".into(),
            clip_path.to_string_lossy().to_string(),
            "https://example.com".into(),
            "key".into(),
            make_metadata(),
        );

        // Manually persist by calling persist with the path set
        if let Ok(mut pp) = q.persist_path.lock() {
            *pp = Some(path.clone());
        }
        q.persist();

        // Load into a new queue
        let q2 = UploadQueue::new();
        q2.set_persist_path(dir.clone());

        let all = q2.all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "persist_1");
        assert_eq!(all[0].status, UploadStatus::Pending);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_cleanup_completed_removes_old() {
        let q = UploadQueue::new();
        q.enqueue_with_meta(
            "old".into(),
            "/clips/old.mp4".into(),
            "https://example.com".into(),
            "key".into(),
            make_metadata(),
        );
        // Artificially set started_at to 2 days ago
        let two_days_ago = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
            .saturating_sub(172800);
        if let Ok(mut queue) = q.inner.lock() {
            if let Some(task) = queue.iter_mut().find(|t| t.id == "old") {
                task.status = UploadStatus::Completed;
                task.started_at_secs = Some(two_days_ago);
            }
        }
        q.cleanup_completed();
        let all = q.all();
        assert!(all.is_empty(), "old completed tasks should be cleaned up");
    }

    #[test]
    fn test_cleanup_completed_keeps_recent() {
        let q = make_queue();
        if let Ok(mut queue) = q.inner.lock() {
            if let Some(task) = queue.iter_mut().find(|t| t.id == "task_1") {
                task.status = UploadStatus::Completed;
            }
        }
        q.cleanup_completed();
        let all = q.all();
        assert_eq!(all.len(), 1, "recent completed tasks should be kept");
    }

    #[test]
    fn test_set_persist_path_skips_missing_files() {
        let dir = std::env::temp_dir().join("prism_upload_test_missing");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("upload_queue.json");
        // Clip file intentionally NOT created — simulates deleted clip

        let q = UploadQueue::new();
        q.enqueue_with_meta(
            "missing_clip".into(),
            dir.join("nonexistent.mp4").to_string_lossy().to_string(),
            "https://example.com".into(),
            "key".into(),
            make_metadata(),
        );

        if let Ok(mut pp) = q.persist_path.lock() {
            *pp = Some(path.clone());
        }
        q.persist();

        let q2 = UploadQueue::new();
        q2.set_persist_path(dir.clone());

        let all = q2.all();
        assert_eq!(all.len(), 0, "deleted clips should be skipped on reload");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
