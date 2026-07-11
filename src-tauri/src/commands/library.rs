//! Clip library IPC commands — list, delete, open file location.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::settings::SettingsManager;

/// A clip entry returned to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct ClipInfo {
    pub id: String,
    pub path: String,
    pub filename: String,
    pub duration_secs: u32,
    pub created_at: String,
    pub size_bytes: u64,
    pub title: String,
    pub description: String,
    pub game: String,
}

/// User-authored clip fields are kept separately from the MP4 so future clip
/// edits can update metadata without rewriting video data.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ClipMetadata {
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    game: String,
}

/// List all clips in the output directory, sorted newest-first.
#[tauri::command]
pub async fn list_clips(settings_mgr: State<'_, SettingsManager>) -> Result<Vec<ClipInfo>, String> {
    let settings = settings_mgr.get();
    let output_dir = resolve_output_dir(&settings.recording.output_directory);

    if !output_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries: Vec<ClipInfo> = Vec::new();

    let mut dir = tokio::fs::read_dir(&output_dir)
        .await
        .map_err(|e| format!("Failed to read output directory: {e}"))?;

    while let Some(entry) = dir.next_entry().await.map_err(|e| e.to_string())? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("mp4") {
            continue;
        }

        if let Ok(meta) = entry.metadata().await {
            let size_bytes = meta.len();
            let modified = meta.modified().ok();

            let filename = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Parse timestamp from filename: clip_YYYYMMDD_HHMMSS.mp4
            let created_at = parse_clip_timestamp(&filename)
                .or_else(|| modified.map(format_unix_timestamp))
                .unwrap_or_default();

            // Try to read MP4 duration from file header
            let duration_secs = read_mp4_duration(&path).unwrap_or(0);
            let clip_metadata = read_clip_metadata(&path);

            entries.push(ClipInfo {
                id: filename.clone(),
                path: path.to_string_lossy().to_string(),
                filename,
                duration_secs,
                created_at,
                size_bytes,
                title: clip_metadata.title,
                description: clip_metadata.description,
                game: clip_metadata.game,
            });
        }
    }

    // Sort newest-first by created_at (parsed timestamps sort lexically as YYYYMMDD_HHMMSS)
    entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(entries)
}

/// Delete a clip file by its filename.
#[tauri::command]
pub async fn delete_clip(
    settings_mgr: State<'_, SettingsManager>,
    filename: String,
) -> Result<(), String> {
    let settings = settings_mgr.get();
    let output_dir = resolve_output_dir(&settings.recording.output_directory);
    let path = output_dir.join(&filename);

    // Safety: only allow deleting .mp4 files in the output directory
    if !path.starts_with(&output_dir) {
        return Err("Invalid file path".into());
    }
    if path.extension().and_then(|s| s.to_str()) != Some("mp4") {
        return Err("Not a clip file".into());
    }

    tokio::fs::remove_file(&path)
        .await
        .map_err(|e| format!("Failed to delete clip: {e}"))?;

    // These are derived/user sidecars, so a missing file must not prevent
    // deletion of the actual clip.
    let _ = tokio::fs::remove_file(thumbnail_path(&path)).await;
    let _ = tokio::fs::remove_file(metadata_path(&path)).await;

    Ok(())
}

/// Rename a clip file. Returns the updated ClipInfo.
#[tauri::command]
pub async fn rename_clip(
    settings_mgr: State<'_, SettingsManager>,
    filename: String,
    new_name: String,
) -> Result<ClipInfo, String> {
    let settings = settings_mgr.get();
    let output_dir = resolve_output_dir(&settings.recording.output_directory);

    // Validate new name: no path separators, no empty
    if new_name.trim().is_empty() {
        return Err("Name cannot be empty".into());
    }
    if new_name.contains('/') || new_name.contains('\\') {
        return Err("Name cannot contain path separators".into());
    }

    let old_path = output_dir.join(&filename);
    let new_filename = format!("{}.mp4", new_name.trim());
    let new_path = output_dir.join(&new_filename);

    // Safety: only operate on .mp4 files in the output directory
    if !old_path.starts_with(&output_dir) || !new_path.starts_with(&output_dir) {
        return Err("Invalid file path".into());
    }
    if old_path.extension().and_then(|s| s.to_str()) != Some("mp4") {
        return Err("Not a clip file".into());
    }
    if new_path.exists() {
        return Err("A clip with that name already exists".into());
    }

    tokio::fs::rename(&old_path, &new_path)
        .await
        .map_err(|e| format!("Failed to rename clip: {e}"))?;

    let old_thumbnail = thumbnail_path(&old_path);
    if old_thumbnail.exists() {
        tokio::fs::rename(&old_thumbnail, thumbnail_path(&new_path))
            .await
            .map_err(|e| format!("Failed to rename thumbnail: {e}"))?;
    }

    let old_metadata = metadata_path(&old_path);
    if old_metadata.exists() {
        tokio::fs::rename(&old_metadata, metadata_path(&new_path))
            .await
            .map_err(|e| format!("Failed to rename clip metadata: {e}"))?;
    }

    // Build response
    let meta = tokio::fs::metadata(&new_path)
        .await
        .map_err(|e| format!("Failed to read clip metadata: {e}"))?;

    let modified = meta.modified().ok();
    let created_at = parse_clip_timestamp(&new_filename)
        .or_else(|| modified.map(format_unix_timestamp))
        .unwrap_or_default();

    let duration_secs = read_mp4_duration(&new_path).unwrap_or(0);
    let clip_metadata = read_clip_metadata(&new_path);

    Ok(ClipInfo {
        id: new_filename.clone(),
        path: new_path.to_string_lossy().to_string(),
        filename: new_filename,
        duration_secs,
        created_at,
        size_bytes: meta.len(),
        title: clip_metadata.title,
        description: clip_metadata.description,
        game: clip_metadata.game,
    })
}

/// Save editable clip fields without modifying the MP4 itself.
#[tauri::command]
pub async fn update_clip_metadata(
    settings_mgr: State<'_, SettingsManager>,
    filename: String,
    title: String,
    description: String,
    game: String,
) -> Result<ClipInfo, String> {
    let settings = settings_mgr.get();
    let output_dir = resolve_output_dir(&settings.recording.output_directory);
    let path = output_dir.join(&filename);

    if !path.starts_with(&output_dir)
        || path.extension().and_then(|s| s.to_str()) != Some("mp4")
        || !path.exists()
    {
        return Err("Invalid clip file".into());
    }

    let metadata = ClipMetadata {
        title: validate_metadata_field("Title", title, 120)?,
        description: validate_metadata_field("Description", description, 2_000)?,
        game: validate_metadata_field("Game", game, 120)?,
    };
    let json = serde_json::to_vec_pretty(&metadata)
        .map_err(|e| format!("Failed to serialize clip metadata: {e}"))?;
    tokio::fs::write(metadata_path(&path), json)
        .await
        .map_err(|e| format!("Failed to save clip metadata: {e}"))?;

    let meta = tokio::fs::metadata(&path)
        .await
        .map_err(|e| format!("Failed to read clip metadata: {e}"))?;
    let created_at = parse_clip_timestamp(&filename)
        .or_else(|| meta.modified().ok().map(format_unix_timestamp))
        .unwrap_or_default();

    Ok(ClipInfo {
        id: filename.clone(),
        path: path.to_string_lossy().to_string(),
        filename,
        duration_secs: read_mp4_duration(&path).unwrap_or(0),
        created_at,
        size_bytes: meta.len(),
        title: metadata.title,
        description: metadata.description,
        game: metadata.game,
    })
}

/// Open the clip library location in the system file manager.
#[tauri::command]
pub async fn open_clip_location(settings_mgr: State<'_, SettingsManager>) -> Result<(), String> {
    let settings = settings_mgr.get();
    let output_dir = resolve_output_dir(&settings.recording.output_directory);

    tauri_plugin_opener::open_path(&output_dir, None::<&str>)
        .map_err(|e| format!("Failed to open file location: {e}"))
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn resolve_output_dir(configured: &str) -> PathBuf {
    if !configured.is_empty() {
        return PathBuf::from(configured);
    }
    dirs::video_dir()
        .map(|d| d.join("Prism"))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn metadata_path(clip_path: &Path) -> PathBuf {
    clip_path.with_extension("mp4.json")
}

fn thumbnail_path(clip_path: &Path) -> PathBuf {
    let stem = clip_path.file_stem().unwrap_or_default().to_string_lossy();
    clip_path.with_file_name(format!("{stem}_thumb.jpg"))
}

fn read_clip_metadata(clip_path: &Path) -> ClipMetadata {
    std::fs::read_to_string(metadata_path(clip_path))
        .ok()
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

fn validate_metadata_field(name: &str, value: String, max_len: usize) -> Result<String, String> {
    let value = value.trim().to_string();
    if value.chars().count() > max_len {
        return Err(format!("{name} must be {max_len} characters or fewer"));
    }
    Ok(value)
}

/// Parse a clip filename like `clip_20260708_143022.mp4` into an ISO timestamp.
fn parse_clip_timestamp(filename: &str) -> Option<String> {
    // strip .mp4
    let stem = filename.strip_suffix(".mp4")?;
    // strip "clip_" prefix
    let ts_part = stem.strip_prefix("clip_")?;
    if ts_part.len() != 15 {
        // YYYYMMDD_HHMMSS = 15 chars
        return None;
    }
    let (date_part, time_part) = ts_part.split_once('_')?;
    if date_part.len() != 8 || time_part.len() != 6 {
        return None;
    }
    Some(format!(
        "{}-{}-{}T{}:{}:{}Z",
        &date_part[0..4],
        &date_part[4..6],
        &date_part[6..8],
        &time_part[0..2],
        &time_part[2..4],
        &time_part[4..6],
    ))
}

/// Format a SystemTime as an ISO-like timestamp string.
fn format_unix_timestamp(time: std::time::SystemTime) -> String {
    use std::time::UNIX_EPOCH;
    let d = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = d.as_secs();
    // Quick approximate conversion (good enough for filenames/sorting)
    const SECS_PER_DAY: u64 = 86400;
    let days = secs / SECS_PER_DAY;
    let rem = secs % SECS_PER_DAY;
    let hours = rem / 3600;
    let mins = (rem % 3600) / 60;
    let secs_rem = rem % 60;
    let year = 1970 + (days as f64 / 365.25) as u64;
    let remaining = days - ((year - 1970) * 365 + ((year - 1969) / 4));
    let month = 1 + remaining / 28;
    let day = 1 + remaining % 28;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year.min(9999),
        month.min(12),
        day.min(31),
        hours,
        mins,
        secs_rem
    )
}

/// Best-effort read MP4 duration from the file header using the mp4 crate.
fn read_mp4_duration(path: &PathBuf) -> Option<u32> {
    // The mp4 crate's Mp4Reader reads the header and gives duration in seconds
    use std::fs::File;
    use std::io::BufReader;

    let file = File::open(path).ok()?;
    let size = file.metadata().ok()?.len();
    let reader = BufReader::new(file);
    let mp4 = mp4::Mp4Reader::read_header(reader, size).ok()?;
    // duration() returns a Duration
    let duration = mp4.duration();
    Some(duration.as_secs() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_sidecars_follow_the_clip_filename() {
        let clip = PathBuf::from("C:/clips/highlight.mp4");

        assert_eq!(
            metadata_path(&clip),
            PathBuf::from("C:/clips/highlight.mp4.json")
        );
        assert_eq!(
            thumbnail_path(&clip),
            PathBuf::from("C:/clips/highlight_thumb.jpg")
        );
    }

    #[test]
    fn metadata_fields_are_trimmed_and_bounded() {
        assert_eq!(
            validate_metadata_field("Game", "  Prism Arena  ".into(), 120).unwrap(),
            "Prism Arena"
        );
        assert!(validate_metadata_field("Title", "x".repeat(121), 120).is_err());
    }
}
