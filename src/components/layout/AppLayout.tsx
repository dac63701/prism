import { useCallback, useEffect, useState } from "react";
import { Outlet } from "react-router-dom";
import { listen } from "@tauri-apps/api/event";
import TitleBar from "./TitleBar";
import Sidebar from "./Sidebar";
import AmbientBackground from "./AmbientBackground";
import ClipNotification from "@/components/common/ClipNotification";
import SignInPrompt from "@/components/auth/SignInPrompt";
import { Toaster } from "@/components/ui/toast";
import { useRecordingStore } from "@/stores/recording";
import { useCloudStore } from "@/stores/cloud";
import { useSettingsStore } from "@/stores/settings";

const SIGN_IN_PROMPT_COOLDOWN_SECS = 3 * 24 * 60 * 60;

export default function AppLayout() {
  const [showSignInPrompt, setShowSignInPrompt] = useState(false);
  // Suppress the default browser right-click context menu
  useEffect(() => {
    const handler = (e: MouseEvent) => e.preventDefault();
    document.addEventListener("contextmenu", handler);
    return () => document.removeEventListener("contextmenu", handler);
  }, []);
  const saveClip = useRecordingStore((s) => s.saveClip);
  const checkCloudStatus = useCloudStore((s) => s.checkStatus);
  const cloudStatusChecked = useCloudStore((s) => s.statusChecked);
  const cloudAuthenticated = useCloudStore((s) => s.authenticated);
  const settingsLoaded = useSettingsStore((s) => s.loaded);
  const settings = useSettingsStore((s) => s.settings);

  const isRecording = useRecordingStore((s) => s.isRecording);
  const checkRecordingStatus = useRecordingStore((s) => s.checkStatus);

  // Verify cloud auth once after settings finish loading on startup.
  // This catches stale/invalid API keys from a previous install.
  // After that, auth state is driven by the auth-state-changed event
  // (OAuth callback / logout) so re-verification on every settings
  // change doesn't clobber a freshly-created API key.
  useEffect(() => {
    if (settingsLoaded) {
      checkCloudStatus();
      void useCloudStore.getState().uploadQueueStatus();
    }
  }, [settingsLoaded, checkCloudStatus]);

  // Show the first-boot sign-in prompt after auth has been verified.
  // Re-asks after a cooldown if the user dismissed it but never signed in.
  useEffect(() => {
    if (!settingsLoaded || !cloudStatusChecked || cloudAuthenticated) return;
    const dismissedAt = settings.general.sign_in_prompt_dismissed_at;
    if (dismissedAt == null) {
      setShowSignInPrompt(true);
      return;
    }
    const now = Math.floor(Date.now() / 1000);
    if (now - dismissedAt >= SIGN_IN_PROMPT_COOLDOWN_SECS) {
      setShowSignInPrompt(true);
    }
  }, [
    settingsLoaded,
    cloudStatusChecked,
    cloudAuthenticated,
    settings.general.sign_in_prompt_dismissed_at,
  ]);

  const closeSignInPrompt = useCallback(() => setShowSignInPrompt(false), []);
  const openSignInPrompt = useCallback(() => setShowSignInPrompt(true), []);

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
    <div className="relative flex h-screen w-screen flex-col overflow-hidden bg-[#050816] text-[#e5eefc]">
      <AmbientBackground />
      <TitleBar />
      <div className="relative z-10 flex h-full min-h-0 w-full">
        <Sidebar onSignInClick={openSignInPrompt} />
        <main className="min-w-0 flex-1 overflow-y-auto">
          <Outlet />
        </main>
      </div>
      <ClipNotification />
      <SignInPrompt open={showSignInPrompt} onClose={closeSignInPrompt} />
      <Toaster />
    </div>
  );
}