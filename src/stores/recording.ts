import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useClipsStore } from "@/stores/clips";

/// IPC timeout for save_clip (seconds). VT encoding can hang indefinitely
/// if the pixel buffer format or resolution is incompatible, so we protect
/// the frontend from freezing by timing out and resetting the saving state.
const SAVE_CLIP_TIMEOUT_SECS = 15;

interface RecordingState {
  isRecording: boolean;
  frameCount: number;
  framesReceived: number;
  previewAvailable: boolean;
  saving: boolean;
  starting: boolean;
  lastClipPath: string | null;
  error: string | null;

  startRecording: () => Promise<void>;
  stopRecording: () => Promise<void>;
  saveClip: (durationSecs?: number) => Promise<string>;
  checkStatus: () => Promise<void>;
  clearLastClipPath: () => void;
  clearError: () => void;
}

export const useRecordingStore = create<RecordingState>((set) => {
  // Set up event listeners
  const setupListeners = async () => {
    try {
      await listen<boolean>("recording-state-changed", (event) => {
        console.log("[recording] event: recording-state-changed =", event.payload);
        set({ isRecording: event.payload, starting: false });
      });

      await listen<string>("clip-saved", (event) => {
        console.log("[recording] event: clip-saved =", event.payload);
        set({ lastClipPath: event.payload, saving: false });
        void useClipsStore.getState().loadClips();
      });
    } catch (err) {
      console.error("[recording] Failed to set up event listeners:", err);
    }
  };
  setupListeners();

  return {
    isRecording: false,
    frameCount: 0,
    framesReceived: 0,
    previewAvailable: false,
    saving: false,
    starting: false,
    lastClipPath: null,
    error: null,

    startRecording: async () => {
      console.log("[recording] startRecording() called");
      set({ error: null, starting: true });
      try {
        const result = await invoke<string | null>("start_recording");
        console.log("[recording] start_recording invoke returned:", result);
        // The event listener should update isRecording.
        // But in case the event doesn't arrive (e.g. command succeeded
        // but emit failed), also check state after a short delay.
        setTimeout(async () => {
          try {
            const info = await invoke<{ is_recording: boolean }>("get_buffer_info");
            if (info.is_recording) {
              set({ isRecording: true, starting: false });
            }
          } catch {}
        }, 500);
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
        console.log("[recording] stop_recording invoke succeeded");
      } catch (err) {
        console.error("[recording] stop_recording failed:", err);
      }
    },

    saveClip: async (durationSecs?: number) => {
      set({ saving: true, error: null });
      try {
        // Race the IPC against a timeout to prevent UI freeze if VT hangs
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
        console.log("[recording] save_clip succeeded:", path);
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
          is_recording: boolean;
          frames_received: number;
          preview_available: boolean;
        }>("get_buffer_info");
        set({
          frameCount: info.frame_count,
          isRecording: info.is_recording,
          framesReceived: info.frames_received,
          previewAvailable: info.preview_available,
        });
      } catch (err) {
        console.error("[recording] checkStatus failed:", err);
      }
    },

    clearLastClipPath: () => {
      set({ lastClipPath: null });
    },

    clearError: () => {
      set({ error: null });
    },
  };
});
