import { useCallback, useEffect, useRef, useState } from "react";
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
 *
 * Two stacked `<img>` layers cross-fade on each new frame so there is never a
 * blank gap while the next JPEG decodes. A plain `src` swap makes Chromium
 * clear the old frame before painting the new one, which reads as a periodic
 * flicker. The previous frame stays fully visible underneath while the new one
 * fades in, then is dropped once the transition finishes.
 */
export default function ScreenPreview({ recording }: ScreenPreviewProps) {
  const [src, setSrc] = useState<string | null>(null);
  const [prevSrc, setPrevSrc] = useState<string | null>(null);
  const [srcOpacity, setSrcOpacity] = useState(1);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const fadeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const errorCountRef = useRef(0);
  const srcRef = useRef<string | null>(null);
  const resolution = useSettingsStore((s) => s.settings.recording.resolution);

  const applyFrame = useCallback((url: string) => {
    // Demote the current frame to the bottom layer (stays visible), start the
    // new one transparent on top, then fade it in over 200 ms. The old layer is
    // dropped once the transition completes.
    setPrevSrc(srcRef.current);
    srcRef.current = url;
    setSrc(url);
    setSrcOpacity(0);
    requestAnimationFrame(() => setSrcOpacity(1));
    if (fadeTimerRef.current) clearTimeout(fadeTimerRef.current);
    fadeTimerRef.current = setTimeout(() => setPrevSrc(null), 200);
  }, []);

  useEffect(() => {
    if (!recording) {
      setSrc(null);
      setPrevSrc(null);
      srcRef.current = null;
      return;
    }

    let active = true;
    // Reset transient-error backoff from a previous session.
    errorCountRef.current = 0;

    const poll = async () => {
      try {
        const dataUrl = await invoke<string | null>("get_preview_frame");
        if (active && dataUrl) {
          // Pre-decode so the fade starts from a ready image; ignore results
          // that are no longer the newest frame (slow decodes must not paint
          // stale data over newer frames).
          const mine = dataUrl;
          const preload = new Image();
          preload.onload = () => {
            if (active && srcRef.current !== mine) applyFrame(mine);
          };
          preload.onerror = () => {
            if (active) errorCountRef.current++;
          };
          preload.src = mine;
        }
        errorCountRef.current = 0;
      } catch {
        errorCountRef.current++;
      }

      if (active) {
        const backoff = Math.min(1000 * Math.pow(2, errorCountRef.current), 10000);
        timerRef.current = setTimeout(poll, backoff);
      }
    };

    poll();

    return () => {
      active = false;
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
      if (fadeTimerRef.current) {
        clearTimeout(fadeTimerRef.current);
      }
    };
  }, [recording, applyFrame]);

  const showPlaceholder = !src && !prevSrc;

  return (
    <div className="relative flex h-full min-h-0 w-full items-center justify-center">
      <div className="relative aspect-video w-full max-h-full overflow-hidden rounded-2xl border border-border bg-surface shadow-lg shadow-black/30">
        <div className="absolute inset-0">
          {prevSrc && (
            <img
              src={prevSrc}
              alt=""
              aria-hidden
              decoding="async"
              className="absolute inset-0 h-full w-full object-contain"
            />
          )}
          {src && (
            <img
              src={src}
              alt="Screen preview"
              decoding="async"
              className="absolute inset-0 h-full w-full object-contain transition-opacity duration-200"
              style={{ opacity: srcOpacity }}
            />
          )}
        </div>

        {showPlaceholder && (
          <div className="absolute inset-0 flex flex-col items-center justify-center gap-3 text-zinc-600">
            <Monitor className="size-10" />
            <span className="text-xs font-medium">
              {recording ? "Waiting for frame…" : "Start recording to see preview"}
            </span>
            {recording && <Loader2 className="size-4 animate-spin text-zinc-700" />}
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