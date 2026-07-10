export interface RecordingSettings {
  buffer_duration_secs: number;
  fps: number;
  bitrate_kbps: number;
  resolution: string;
  output_directory: string;
  always_on_recording: boolean;
  capture_target: string;
}

export interface HotkeySettings {
  save_clip: string;
  toggle_recording: string;
  open_library: string;
}

export interface GeneralSettings {
  launch_at_startup: boolean;
  minimize_to_tray: boolean;
  show_clip_notification: boolean;
  game_detection_enabled: boolean;
}

export interface StorageSettings {
  max_clips_gb: number;
  auto_prune_days: number | null;
}

export interface AppSettings {
  recording: RecordingSettings;
  hotkeys: HotkeySettings;
  general: GeneralSettings;
  storage: StorageSettings;
}
