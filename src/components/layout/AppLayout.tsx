import { useEffect } from "react";
import { Outlet } from "react-router-dom";
import { listen } from "@tauri-apps/api/event";
import Sidebar from "./Sidebar";
import ClipNotification from "@/components/common/ClipNotification";
import { useRecordingStore } from "@/stores/recording";
import { useCloudStore } from "@/stores/cloud";
import { useSettingsStore } from "@/stores/settings";

export default function AppLayout() {
  // Suppress the default browser right-click context menu
  useEffect(() => {
    const handler = (e: MouseEvent) => e.preventDefault();
    document.addEventListener("contextmenu", handler);
    return () => document.removeEventListener("contextmenu", handler);
  }, []);
  const saveClip = useRecordingStore((s) => s.saveClip);
  const checkCloudStatus = useCloudStore((s) => s.checkStatus);
  const settings = useSettingsStore((s) => s.settings);

  const isRecording = useRecordingStore((s) => s.isRecording);
  const checkRecordingStatus = useRecordingStore((s) => s.checkStatus);

  // Re-derive cloud auth state whenever settings load or change.
  // This avoids a race where checkCloudStatus() reads default
  // settings (api_key = "") before loadSettings() finishes.
  useEffect(() => {
    checkCloudStatus();
  }, [settings, checkCloudStatus]);

  // Poll recording status every 1s while recording (keeps timer live on all pages)
  useEffect(() => {
    let interval: ReturnType<typeof setInterval> | null = null;
    if (isRecording) {
      checkRecordingStatus();
      interval = setInterval(checkRecordingStatus, 1000);
    }
    return () => {
      if (interval) clearInterval(interval);
    };
  }, [isRecording, checkRecordingStatus]);

  useEffect(() => {
    const unlistenMenu = listen<string>("menu-action", (event) => {
      if (event.payload === "save_clip") {
        saveClip();
      }
    });

    const unlistenHotkey = listen<string>("hotkey-pressed", (event) => {
      const action = event.payload;
      if (action === "save_clip") {
        saveClip();
      } else if (action === "toggle_recording") {
        const state = useRecordingStore.getState();
        if (state.isRecording) {
          state.stopRecording();
        } else {
          state.startRecording();
        }
      }
    });

    return () => {
      unlistenMenu.then((fn) => fn());
      unlistenHotkey.then((fn) => fn());
    };
  }, [saveClip]);

  return (
    <div className="relative flex h-screen w-screen overflow-hidden bg-[#050816] text-[#e5eefc]">
      <div className="pointer-events-none fixed inset-0 overflow-hidden">
        <div className="absolute -left-40 -top-40 h-[500px] w-[500px] rounded-full bg-blue-500/[0.07] blur-[120px]" />
        <div className="absolute -bottom-40 -right-40 h-[500px] w-[500px] rounded-full bg-blue-600/5 blur-[120px]" />
      </div>
      <div className="relative z-10 flex h-full w-full">
        <Sidebar />
        <main className="flex-1 overflow-y-auto">
          <Outlet />
        </main>
      </div>
      <ClipNotification />
    </div>
  );
}
