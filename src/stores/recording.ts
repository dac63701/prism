import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useClipsStore } from "@/stores/clips";

/// IPC timeout for save_clip (seconds).
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
  bufferTimeSeconds: number;
  recordingElapsedSeconds: number;

  startRecording: () => Promise<void>;
  stopRecording: () => Promise<void>;
  saveClip: (durationSecs?: number) => Promise<string>;
  checkStatus: () => Promise<void>;
  clearLastClipPath: () => void;
  clearError: () => void;
}

let unlistenStateChanged: (() => void) | null = null;
let unlistenClipSaved: (() => void) | null = null;

export const useRecordingStore = create<RecordingState>((set) => {
  const setupListeners = async () => {
    // Only register once
    if (!unlistenStateChanged) {
      unlistenStateChanged = await listen<boolean>("recording-state-changed", (event) => {
        console.log("[recording] event: recording-state-changed =", event.payload);
        set({ isRecording: event.payload, starting: false });
      });
    }
    if (!unlistenClipSaved) {
      unlistenClipSaved = await listen<string>("clip-saved", (event) => {
        console.log("[recording] event: clip-saved =", event.payload);
        set({ lastClipPath: event.payload, saving: false });
        void useClipsStore.getState().loadClips();
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

    clearLastClipPath: () => {
      set({ lastClipPath: null });
    },

    clearError: () => {
      set({ error: null });
    },
  };
});
