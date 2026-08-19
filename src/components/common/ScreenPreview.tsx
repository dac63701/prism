import { useEffect, useRef, useState } from "react";
import { Monitor, Loader2 } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "@/stores/settings";

interface ScreenPreviewProps {
  /** Whether recording is active — only poll when true */
  recording: boolean;
}

/**
 * Live screen capture preview — polls the Rust backend for JPEG frames
 * at ~1 fps when recording is active. Fills available space while keeping
 * 16:9 aspect ratio.
 */
export default function ScreenPreview({ recording }: ScreenPreviewProps) {
  const [src, setSrc] = useState<string | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const errorCountRef = useRef(0);
  const resolution = useSettingsStore((s) => s.settings.recording.resolution);

  useEffect(() => {
    if (!recording) {
      setSrc(null);
      return;
    }

    let active = true;

    const poll = async () => {
      try {
        const dataUrl = await invoke<string | null>("get_preview_frame");
        if (active && dataUrl) {
          setSrc(dataUrl);
        }
        errorCountRef.current = 0;
      } catch {
        errorCountRef.current++;
      }

      if (active) {
        const backoff = Math.min(2000 * Math.pow(2, errorCountRef.current), 10000);
        timerRef.current = setTimeout(poll, backoff);
      }
    };

    poll();

    return () => {
      active = false;
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
    };
  }, [recording]);

  return (
    <div className="relative flex h-full min-h-0 w-full items-center justify-center">
      <div className="relative aspect-video w-full max-h-full overflow-hidden rounded-2xl border border-border bg-surface shadow-lg shadow-black/30">
        {src ? (
          <img
            src={src}
            alt="Screen preview"
            decoding="async"
            className="h-full w-full object-contain"
          />
        ) : (
          <div className="absolute inset-0 flex flex-col items-center justify-center gap-3 text-zinc-600">
            <Monitor className="size-10" />
            <span className="text-xs font-medium">
              {recording ? "Waiting for frame…" : "Start recording to see preview"}
            </span>
            {recording && (
              <Loader2 className="size-4 animate-spin text-zinc-700" />
            )}
          </div>
        )}

        {recording && (
          <div className="absolute left-3 top-3 flex items-center gap-2">
            <div className="flex items-center gap-1.5 rounded-md bg-black/50 px-2 py-1 backdrop-blur-sm">
              <span className="relative flex size-2">
                <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-red-500 opacity-60" />
                <span className="relative inline-flex size-2 rounded-full bg-red-500 shadow-[0_0_6px_rgba(239,68,68,0.6)]" />
              </span>
              <span className="text-[11px] font-medium text-white/80">LIVE</span>
            </div>
            <div className="flex items-center gap-1.5 rounded-md bg-black/50 px-2 py-1 text-[11px] font-medium text-white/80 backdrop-blur-sm">
              <Monitor className="size-3" />
              {resolution}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}