import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface Clip {
  id: string;
  path: string;
  filename: string;
  duration_secs: number;
  created_at: string;
  size_bytes: number;
}

interface ClipsState {
  clips: Clip[];
  loading: boolean;
  loaded: boolean;
  loadClips: () => Promise<void>;
  deleteClip: (filename: string) => Promise<void>;
  renameClip: (filename: string, newName: string) => Promise<void>;
  openClipLocation: () => Promise<void>;
}

export const useClipsStore = create<ClipsState>((set) => ({
  clips: [],
  loading: false,
  loaded: false,

  loadClips: async () => {
    set({ loading: true });
    try {
      const clips = await invoke<Clip[]>("list_clips");
      set({ clips, loaded: true, loading: false });
    } catch (err) {
      console.error("Failed to load clips:", err);
      set({ loading: false });
    }
  },

  deleteClip: async (filename: string) => {
    try {
      await invoke("delete_clip", { filename });
      // Refresh list after deletion
      const clips = await invoke<Clip[]>("list_clips");
      set({ clips });
    } catch (err) {
      console.error("Failed to delete clip:", err);
      throw err;
    }
  },

  renameClip: async (filename: string, newName: string) => {
    try {
      const updated = await invoke<Clip>("rename_clip", { filename, newName });
      set((state) => ({
        clips: state.clips.map((c) =>
          c.id === filename ? updated : c
        ),
      }));
    } catch (err) {
      console.error("Failed to rename clip:", err);
      throw err;
    }
  },

  openClipLocation: async () => {
    try {
      await invoke("open_clip_location");
    } catch (err) {
      console.error("Failed to open clip location:", err);
    }
  },
}));

/// Format bytes into a human-readable string.
export function formatSize(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  const size = (bytes / Math.pow(1024, i)).toFixed(i > 0 ? 1 : 0);
  return `${size} ${units[i]}`;
}

/// Format duration in seconds to mm:ss.
export function formatDuration(secs: number): string {
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}

/// Format an ISO timestamp to a locale-friendly string.
export function formatDate(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return iso;
  }
}
