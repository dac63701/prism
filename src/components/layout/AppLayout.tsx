import { useEffect } from "react";
import { Outlet } from "react-router-dom";
import { listen } from "@tauri-apps/api/event";
import Sidebar from "./Sidebar";
import ClipNotification from "@/components/common/ClipNotification";
import { useRecordingStore } from "@/stores/recording";
import { useCloudStore } from "@/stores/cloud";

export default function AppLayout() {
  const saveClip = useRecordingStore((s) => s.saveClip);
  const checkCloudStatus = useCloudStore((s) => s.checkStatus);

  useEffect(() => {
    checkCloudStatus();
  }, [checkCloudStatus]);

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
    <div className="flex h-screen w-screen overflow-hidden bg-[#050816] text-[#e5eefc]">
      <Sidebar />
      <main className="flex-1 overflow-y-auto">
        <Outlet />
      </main>
      <ClipNotification />
    </div>
  );
}
