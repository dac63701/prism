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
  cs2_gsi_port: number;
}

export interface PerGameAutoClip {
  game_name: string;
  enabled: boolean;
  kill_clip_duration: number;
  death_clip_duration: number;
  combat_event_duration: number;
  events: string[];
  audio_enabled: boolean;
  audio_sensitivity: number | null;
}

export interface AutoClipSettings {
  enabled: boolean;
  cooldown_secs: number;
  audio_sensitivity: number;
  games: PerGameAutoClip[];
}

export interface StorageSettings {
  max_clips_gb: number;
  auto_prune_days: number | null;
}

export interface CloudSettings {
  server_url: string;
  api_key: string;
  access_token: string;
  refresh_token: string;
  auto_upload: boolean;
  max_concurrent_uploads: number;
  account_display_name: string;
  account_email: string;
}

export interface AppSettings {
  recording: RecordingSettings;
  hotkeys: HotkeySettings;
  general: GeneralSettings;
  storage: StorageSettings;
  cloud: CloudSettings;
  auto_clip: AutoClipSettings;
}
