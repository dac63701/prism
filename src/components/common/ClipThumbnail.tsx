import { useState, useCallback, useEffect, useRef } from "react";
import { Film } from "lucide-react";
import { convertFileSrc } from "@tauri-apps/api/core";

interface ClipThumbnailProps {
  path: string;
  filename: string;
}

const PREVIEW_TIME = 0.15;
const thumbnailCache = new Map<string, string>();

export default function ClipThumbnail({ path, filename }: ClipThumbnailProps) {
  const thumbSrc = convertFileSrc(path.replace(/\.mp4$/, "_thumb.jpg"));
  const videoSrc = convertFileSrc(path);
  const cacheKey = `${path}-thumb`;
  const [mode, setMode] = useState<"img" | "video" | "failed">(() =>
    thumbnailCache.has(cacheKey) ? "video" : "img"
  );
  const [videoThumb, setVideoThumb] = useState<string | null>(
    () => thumbnailCache.get(cacheKey) ?? null
  );
  const mountedRef = useRef(true);

  useEffect(() => {
    return () => { mountedRef.current = false; };
  }, []);

  const handleImgError = useCallback(() => {
    setMode("video");
  }, []);

  const captureFrame = useCallback(() => {
    const video = document.createElement("video");
    video.src = videoSrc;
    video.muted = true;
    video.playsInline = true;
    video.preload = "metadata";
    video.currentTime = PREVIEW_TIME;
    video.onloadedmetadata = () => {
      const target = Number.isFinite(video.duration) && video.duration > 0
        ? Math.min(PREVIEW_TIME, Math.max(0, video.duration - 0.05))
        : PREVIEW_TIME;
      try { video.currentTime = target; } catch { /* ignore */ }
    };
    video.onseeked = () => {
      if (!mountedRef.current) return;
      if (video.videoWidth === 0 || video.videoHeight === 0) {
        setMode("failed");
        return;
      }
      const canvas = document.createElement("canvas");
      canvas.width = video.videoWidth;
      canvas.height = video.videoHeight;
      const ctx = canvas.getContext("2d");
      if (!ctx) { setMode("failed"); return; }
      try {
        ctx.drawImage(video, 0, 0, canvas.width, canvas.height);
        const dataUrl = canvas.toDataURL("image/jpeg", 0.82);
        thumbnailCache.set(cacheKey, dataUrl);
        setVideoThumb(dataUrl);
      } catch {
        setMode("failed");
      }
    };
    video.onerror = () => {
      if (mountedRef.current) setMode("failed");
    };
  }, [videoSrc, cacheKey]);

  // Auto-trigger video fallback when mode switches to "video" and no cached thumb
  useEffect(() => {
    if (mode === "video" && !videoThumb && !thumbnailCache.has(cacheKey)) {
      captureFrame();
    }
  }, [mode, videoThumb, captureFrame, cacheKey]);

  return (
    <div className="relative h-full w-full overflow-hidden bg-gradient-to-br from-surface to-bg">
      {mode === "img" ? (
        <img
          src={thumbSrc}
          alt={`${filename} thumbnail`}
          className="h-full w-full object-cover"
          onError={handleImgError}
        />
      ) : mode === "video" ? (
        videoThumb ? (
          <img
            src={videoThumb}
            alt={`${filename} thumbnail`}
            className="h-full w-full object-cover"
          />
        ) : (
          <div className="absolute inset-0 flex items-center justify-center">
            <Film className="size-8 text-zinc-700" />
          </div>
        )
      ) : (
        <div className="absolute inset-0 flex flex-col items-center justify-center p-3 text-center">
          <Film className="size-8 text-zinc-700 mb-2" />
          <p className="text-xs text-zinc-500 line-clamp-2">{filename}</p>
        </div>
      )}
    </div>
  );
}
