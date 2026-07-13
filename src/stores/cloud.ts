import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useSettingsStore } from "@/stores/settings";

interface CloudState {
  authenticated: boolean;
  email: string;
  uploads: UploadTask[];
  loading: boolean;
  uploadError: string | null;

  login: () => Promise<void>;
  logout: () => Promise<void>;
  checkStatus: () => Promise<void>;
  uploadClip: (path: string, filename: string, game?: string) => Promise<void>;
  uploadQueueStatus: () => Promise<UploadTask[]>;
  copyShareUrl: (url: string) => Promise<void>;
  handleAuthCode: (code: string) => Promise<void>;
  clearUploadError: () => void;
}

export interface UploadTask {
  id: string;
  clip_path: string;
  status: "Pending" | "Uploading" | "Completed" | "Failed" | "Cancelled" | string;
  progress: number;
  started_at_secs: number | null;
  server_url: string | null;
  share_url?: string;
  error?: string | null;
}

let unlistenAuth: (() => void) | null = null;
let unlistenUploadProgress: (() => void) | null = null;
let unlistenAuthError: (() => void) | null = null;
let unlistenAuthInvalid: (() => void) | null = null;

export const useCloudStore = create<CloudState>((set) => {
  const setupListeners = async () => {
    if (!unlistenAuth) {
      unlistenAuth = await listen<boolean>("auth-state-changed", (event) => {
        // The event fires after a successful OAuth callback or logout.
        // authenticated=true means the backend just created a fresh API key
        // and saved it to settings — no need to re-verify immediately.
        set({ authenticated: event.payload });
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
    if (!unlistenAuthInvalid) {
      unlistenAuthInvalid = await listen<undefined>("auth-invalid", () => {
        set({ authenticated: false, uploadError: "Session expired — please sign in again" });
      });
    }
  };
  setupListeners();

  return {
    authenticated: false,
    email: "",
    uploads: [],
    loading: false,
    uploadError: null,

    login: async () => {
      const settings = useSettingsStore.getState().settings;
      if (!settings.cloud.server_url) {
        set({ uploadError: "Server URL not configured" });
        return;
      }
      set({ loading: true, uploadError: null });
      try {
        await invoke("cloud_login");
      } catch (err) {
        const msg = typeof err === "string" ? err : "Sign in failed";
        set({ uploadError: msg });
        console.error("[cloud] login failed:", err);
      } finally {
        set({ loading: false });
      }
    },

    logout: async () => {
      set({ loading: true });
      try {
        await invoke("cloud_logout");
        set({ authenticated: false, email: "", loading: false, uploadError: null });
      } catch (err) {
        const msg = typeof err === "string" ? err : "Sign out failed";
        set({ uploadError: msg });
        console.error("[cloud] logout failed:", err);
        set({ loading: false });
      }
    },

    checkStatus: async () => {
      try {
        const settings = useSettingsStore.getState().settings;
        const valid =
          !!settings.cloud.access_token &&
          (await invoke<boolean>("cloud_verify_auth"));
        set({
          authenticated: valid,
          email: valid ? settings.cloud.account_email : "",
        });
      } catch (err) {
        console.error("[cloud] checkStatus failed:", err);
      }
    },

    uploadClip: async (path: string, filename: string, game?: string) => {
      try {
        set({ loading: true, uploadError: null });
        await invoke("upload_clip", { path, filename, game: game ?? "" });
      } catch (err) {
        const msg = typeof err === "string" ? err : "Upload failed";
        set({ uploadError: msg });
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

    handleAuthCode: async (code: string) => {
      try {
        set({ loading: true, uploadError: null });
        await invoke("cloud_handle_auth_code", { code });
      } catch (err) {
        const msg = typeof err === "string" ? err : "Auth failed";
        set({ uploadError: msg });
        console.error("[cloud] handleAuthCode failed:", err);
      } finally {
        set({ loading: false });
      }
    },

    clearUploadError: () => set({ uploadError: null }),
  };
});
