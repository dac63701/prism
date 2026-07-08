import { useEffect, useRef, useState } from "react";
import { Film } from "lucide-react";
import { convertFileSrc } from "@tauri-apps/api/core";

interface ClipThumbnailProps {
  path: string;
  filename: string;
}

const PREVIEW_TIME = 0.15;

export default function ClipThumbnail({ path, filename }: ClipThumbnailProps) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const [thumbnail, setThumbnail] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    setThumbnail(null);
    setFailed(false);
  }, [path]);

  const captureFrame = () => {
    const video = videoRef.current;
    if (!video || video.videoWidth === 0 || video.videoHeight === 0) {
      return;
    }

    const canvas = document.createElement("canvas");
    canvas.width = video.videoWidth;
    canvas.height = video.videoHeight;

    const ctx = canvas.getContext("2d");
    if (!ctx) {
      setFailed(true);
      return;
    }

    try {
      ctx.drawImage(video, 0, 0, canvas.width, canvas.height);
      setThumbnail(canvas.toDataURL("image/jpeg", 0.82));
    } catch {
      setFailed(true);
    }
  };

  const seekToPreview = () => {
    const video = videoRef.current;
    if (!video) return;

    const target = Number.isFinite(video.duration) && video.duration > 0
      ? Math.min(PREVIEW_TIME, Math.max(0, video.duration - 0.05))
      : PREVIEW_TIME;

    try {
      video.currentTime = target;
    } catch {
      captureFrame();
    }
  };

  return (
    <div className="relative h-full w-full overflow-hidden bg-gradient-to-br from-zinc-800 to-zinc-950">
      {thumbnail ? (
        <img
          src={thumbnail}
          alt={`${filename} thumbnail`}
          className="h-full w-full object-cover"
        />
      ) : failed ? (
        <div className="absolute inset-0 flex flex-col items-center justify-center p-3 text-center">
          <Film className="size-8 text-zinc-700 mb-2" />
          <p className="text-xs text-zinc-500 line-clamp-2">{filename}</p>
        </div>
      ) : (
        <>
          <video
            ref={videoRef}
            src={convertFileSrc(path)}
            muted
            playsInline
            preload="metadata"
            className="absolute inset-0 h-full w-full object-cover opacity-0 pointer-events-none"
            onLoadedMetadata={seekToPreview}
            onSeeked={captureFrame}
            onError={() => setFailed(true)}
          />
          <div className="absolute inset-0 flex items-center justify-center">
            <Film className="size-8 text-zinc-700" />
          </div>
        </>
      )}
    </div>
  );
}
