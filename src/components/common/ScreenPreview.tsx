import { useEffect, useRef, useState } from "react";
import { Monitor } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";

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
        const backoff = Math.min(800 * Math.pow(2, errorCountRef.current), 10000);
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
    <div className="relative w-full flex-1 min-h-0 flex items-center justify-center">
      {/* Constrained 16:9 box that scales with available space */}
      <div className="relative w-full max-h-full aspect-video bg-surface rounded-2xl overflow-hidden border border-white/10">
        {src ? (
          <img
            src={src}
            alt="Screen preview"
            className="w-full h-full object-contain"
          />
        ) : (
          <div className="absolute inset-0 flex flex-col items-center justify-center text-zinc-600 gap-3">
            <Monitor className="size-10" />
            <span className="text-xs font-medium">
              {recording
                ? "Waiting for frame\u2026"
                : "Start recording to see preview"}
            </span>
          </div>
        )}

        {/* Recording badge */}
        {recording && (
          <div className="absolute top-3 left-3 flex items-center gap-1.5 px-2 py-1 rounded-md bg-black/50 backdrop-blur-sm">
            <span className="size-2 rounded-full bg-red-500 animate-pulse shadow-[0_0_6px_rgba(239,68,68,0.6)]" />
            <span className="text-[11px] font-medium text-white/80">LIVE</span>
          </div>
        )}
      </div>
    </div>
  );
}
