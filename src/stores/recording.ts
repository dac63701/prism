import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useClipsStore } from "@/stores/clips";
import { useSettingsStore } from "@/stores/settings";

const SAVE_CLIP_TIMEOUT_SECS = 15;

export interface UploadTask {
  id: string;
  clip_path: string;
  status: "Pending" | "Uploading" | "Completed" | "Failed";
  progress: number;
  started_at_secs: number | null;
  server_url: string | null;
}

interface RecordingState {
  isRecording: boolean;
  frameCount: number;
  framesReceived: number;
  previewAvailable: boolean;
  saving: boolean;
  starting: boolean;
  lastClipPath: string | null;
  error: string | null;
  bufferTimeSeconds: number;
  recordingElapsedSeconds: number;
  uploads: UploadTask[];

  startRecording: () => Promise<void>;
  stopRecording: () => Promise<void>;
  saveClip: (durationSecs?: number) => Promise<string>;
  checkStatus: () => Promise<void>;
  clearLastClipPath: () => void;
  setError: (err: string) => void;
  clearError: () => void;
  refreshUploadQueue: () => Promise<void>;
  clearCompletedUploads: () => Promise<void>;
}

let unlistenStateChanged: (() => void) | null = null;
let unlistenClipSaved: (() => void) | null = null;
let unlistenUploadCompleted: (() => void) | null = null;
let unlistenUploadFailed: (() => void) | null = null;

export const useRecordingStore = create<RecordingState>((set, get) => {
  const setupListeners = async () => {
    if (!unlistenStateChanged) {
      unlistenStateChanged = await listen<boolean>("recording-state-changed", (event) => {
        console.log("[recording] event: recording-state-changed =", event.payload);
        set({ isRecording: event.payload, starting: false });
      });
    }
    if (!unlistenClipSaved) {
      unlistenClipSaved = await listen<string>("clip-saved", async (event) => {
        console.log("[recording] event: clip-saved =", event.payload);
        set({ lastClipPath: event.payload, saving: false });
        void useClipsStore.getState().loadClips();

        const settings = useSettingsStore.getState().settings;
        if (settings?.upload.auto_upload && settings?.upload.server_url && settings?.upload.api_key) {
          try {
            await invoke("upload_clip_to_server", {
              clipPath: event.payload,
              title: "",
              game: "",
              durationSecs: 0,
              width: 0,
              height: 0,
            });
          } catch (err) {
            console.error("[recording] auto-upload failed:", err);
          }
          void get().refreshUploadQueue();
        }
      });
    }
    if (!unlistenUploadCompleted) {
      unlistenUploadCompleted = await listen("upload-completed", () => {
        void get().refreshUploadQueue();
      });
    }
    if (!unlistenUploadFailed) {
      unlistenUploadFailed = await listen("upload-failed", () => {
        void get().refreshUploadQueue();
      });
    }
  };
  setupListeners().catch((err) => {
    console.error("[recording] Failed to set up event listeners:", err);
  });

  return {
    isRecording: false,
    frameCount: 0,
    framesReceived: 0,
    previewAvailable: false,
    saving: false,
    starting: false,
    lastClipPath: null,
    error: null,
    bufferTimeSeconds: 0,
    recordingElapsedSeconds: 0,
    uploads: [],

    startRecording: async () => {
      console.log("[recording] startRecording() called");
      set({ error: null, starting: true });
      try {
        await invoke<string | null>("start_recording");
      } catch (err) {
        const msg = typeof err === "string" ? err : "Failed to start recording";
        console.error("[recording] start_recording failed:", err);
        set({ error: msg, isRecording: false, starting: false });
      }
    },

    stopRecording: async () => {
      console.log("[recording] stopRecording() called");
      try {
        await invoke("stop_recording");
      } catch (err) {
        console.error("[recording] stop_recording failed:", err);
      }
    },

    saveClip: async (durationSecs?: number) => {
      set({ saving: true, error: null });
      try {
        const path = await Promise.race([
          invoke<string>("save_clip", {
            durationSecs: durationSecs ?? 30,
          }),
          new Promise<string>((_, reject) =>
            setTimeout(
              () => reject(new Error(`Clip save timed out after ${SAVE_CLIP_TIMEOUT_SECS}s`)),
              SAVE_CLIP_TIMEOUT_SECS * 1000,
            ),
          ),
        ]);
        return path;
      } catch (err) {
        const msg = typeof err === "string" ? err : err instanceof Error ? err.message : "Clip save failed";
        console.error("[recording] save_clip failed:", msg);
        set({ saving: false, error: msg });
        throw err;
      }
    },

    checkStatus: async () => {
      try {
        const info = await invoke<{
          frame_count: number;
          buffer_time_seconds: number;
          is_recording: boolean;
          frames_received: number;
          preview_available: boolean;
          recording_elapsed_seconds: number;
        }>("get_buffer_info");
        set({
          frameCount: info.frame_count,
          isRecording: info.is_recording,
          framesReceived: info.frames_received,
          previewAvailable: info.preview_available,
          bufferTimeSeconds: info.buffer_time_seconds,
          recordingElapsedSeconds: info.recording_elapsed_seconds,
        });
      } catch (err) {
        console.error("[recording] checkStatus failed:", err);
      }
    },

    refreshUploadQueue: async () => {
      try {
        const uploads = await invoke<UploadTask[]>("get_upload_queue");
        set({ uploads });
      } catch (err) {
        console.error("[recording] refreshUploadQueue failed:", err);
      }
    },

    clearCompletedUploads: async () => {
      try {
        await invoke("clear_upload_queue");
        set({ uploads: [] });
      } catch (err) {
        console.error("[recording] clearCompletedUploads failed:", err);
      }
    },

    clearLastClipPath: () => {
      set({ lastClipPath: null });
    },

    setError: (err: string) => set({ error: err }),

    clearError: () => {
      set({ error: null });
    },
  };
});
