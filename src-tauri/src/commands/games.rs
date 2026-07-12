//! Game detection status IPC.

use tauri::State;

use crate::games::{DetectedGame, GameDetector};

#[tauri::command]
pub async fn get_detected_game(
    detector: State<'_, GameDetector>,
) -> Result<Option<DetectedGame>, String> {
    Ok(detector.active_game())
}
