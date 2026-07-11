import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AppSettings } from "@/types/settings";

function isMacPlatform() {
  return typeof navigator !== "undefined" && /mac/i.test(navigator.platform);
}

export function getDefaultHotkeys() {
  const mod = isMacPlatform() ? "Cmd" : "Ctrl";
  return {
    save_clip: `${mod}+Shift+X`,
    toggle_recording: `${mod}+Shift+R`,
    open_library: `${mod}+Shift+L`,
  };
}

export function getDefaultSettings(): AppSettings {
  return {
    recording: {
      buffer_duration_secs: 60,
      fps: 60,
      bitrate_kbps: 8000,
      resolution: "1080p",
      output_directory: "",
      always_on_recording: true,
      capture_target: "",
    },
    hotkeys: getDefaultHotkeys(),
    general: {
      launch_at_startup: false,
      minimize_to_tray: true,
      show_clip_notification: true,
      game_detection_enabled: false,
    },
    storage: {
      max_clips_gb: 50,
      auto_prune_days: null,
    },
    cloud: {
      server_url: "https://goprism.studio",
      api_key: "",
      auto_upload: false,
      max_concurrent_uploads: 1,
      account_display_name: "",
      account_email: "",
    },
  };
}

interface SettingsState {
  settings: AppSettings;
  loading: boolean;
  loaded: boolean;
  saving: boolean;
  loadSettings: () => Promise<void>;
  updateSettings: (settings: AppSettings) => Promise<void>;
  resetSettings: () => Promise<void>;
}

let unlistenSettings: (() => void) | null = null;

export const useSettingsStore = create<SettingsState>((set) => {
  // Register event listener once; store unlisten for cleanup
  (async () => {
    if (unlistenSettings) return; // already registered
    const unlisten = await listen<AppSettings>("settings-changed", (event) => {
      set({ settings: event.payload });
    });
    unlistenSettings = unlisten;
  })();

  return {
    settings: getDefaultSettings(),
    loading: false,
    loaded: false,
    saving: false,

    loadSettings: async () => {
      set({ loading: true });
      try {
        const settings = await invoke<AppSettings>("get_settings");
        set({ settings, loaded: true, loading: false });
      } catch (err) {
        console.error("Failed to load settings:", err);
        set({ loading: false });
      }
    },

    updateSettings: async (settings: AppSettings) => {
      set({ saving: true });
      try {
        const updated = await invoke<AppSettings>("update_settings", {
          settings,
        });
        set({ settings: updated, saving: false });
      } catch (err) {
        console.error("Failed to update settings:", err);
        set({ saving: false });
        throw err;
      }
    },

    resetSettings: async () => {
      set({ saving: true });
      try {
        const settings = await invoke<AppSettings>("reset_settings");
        set({ settings, saving: false });
      } catch (err) {
        console.error("Failed to reset settings:", err);
        set({ saving: false });
      }
    },
  };
});
