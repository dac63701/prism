import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useSettingsStore } from "@/stores/settings";

interface CloudState {
  authenticated: boolean;
  displayName: string;
  email: string;
  serverUrl: string;
  uploads: UploadTask[];
  loading: boolean;

  login: () => Promise<void>;
  logout: () => Promise<void>;
  checkStatus: () => Promise<void>;
  uploadClip: (path: string, filename: string, game?: string) => Promise<void>;
  uploadQueueStatus: () => Promise<UploadTask[]>;
  copyShareUrl: (url: string) => Promise<void>;
}

export interface UploadTask {
  id: string;
  clip_path: string;
  status: "Pending" | "Uploading" | "Completed" | string;
  progress: number;
  started_at_secs: number | null;
  server_url: string | null;
  share_url?: string;
}

let unlistenAuth: (() => void) | null = null;
let unlistenUploadProgress: (() => void) | null = null;
let unlistenAuthError: (() => void) | null = null;

export const useCloudStore = create<CloudState>((set, get) => {
  const setupListeners = async () => {
    if (!unlistenAuth) {
      unlistenAuth = await listen<boolean>("auth-state-changed", (event) => {
        set({ authenticated: event.payload });
        if (event.payload) {
          get().checkStatus();
        }
      });
    }
    if (!unlistenAuthError) {
      unlistenAuthError = await listen<string>("auth-error", (event) => {
        console.error("[cloud] auth error:", event.payload);
      });
    }
    if (!unlistenUploadProgress) {
      unlistenUploadProgress = await listen<Record<string, unknown>>(
        "upload-progress",
        (event) => {
          const payload = event.payload as unknown as UploadTask;
          set((state) => {
            const idx = state.uploads.findIndex((t) => t.id === payload.id);
            const uploads = [...state.uploads];
            if (idx >= 0) {
              uploads[idx] = { ...uploads[idx], ...payload };
            } else {
              uploads.push(payload);
            }
            return { uploads };
          });
        },
      );
    }
  };
  setupListeners();

  return {
    authenticated: false,
    displayName: "",
    email: "",
    serverUrl: "",
    uploads: [],
    loading: false,

    login: async () => {
      const settings = useSettingsStore.getState().settings;
      if (!settings.cloud.server_url) {
        console.error("[cloud] Server URL not configured");
        return;
      }
      set({ loading: true });
      await invoke("cloud_login");
      set({ loading: false });
    },

    logout: async () => {
      set({ loading: true });
      await invoke("cloud_logout");
      set({ authenticated: false, displayName: "", email: "", loading: false });
    },

    checkStatus: async () => {
      try {
        const settings = useSettingsStore.getState().settings;
        set({
          serverUrl: settings.cloud.server_url,
          authenticated: !!settings.cloud.api_key,
          displayName: settings.cloud.account_display_name,
          email: settings.cloud.account_email,
        });
      } catch (err) {
        console.error("[cloud] checkStatus failed:", err);
      }
    },

    uploadClip: async (path: string, filename: string, game?: string) => {
      try {
        set({ loading: true });
        await invoke("upload_clip", { path, filename, game: game ?? "" });
      } catch (err) {
        console.error("[cloud] uploadClip failed:", err);
      } finally {
        set({ loading: false });
      }
    },

    uploadQueueStatus: async () => {
      try {
        const tasks = await invoke<UploadTask[]>("upload_queue_status");
        set({ uploads: tasks });
        return tasks;
      } catch (err) {
        console.error("[cloud] uploadQueueStatus failed:", err);
        return [];
      }
    },

    copyShareUrl: async (url: string) => {
      try {
        await navigator.clipboard.writeText(url);
      } catch {
        console.error("[cloud] Failed to copy share URL");
      }
    },
  };
});
