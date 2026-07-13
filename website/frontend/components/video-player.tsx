"use client";

import { useRef, useState, useCallback, useEffect } from "react";
import {
  Play,
  Pause,
  SkipBack,
  SkipForward,
  PictureInPicture2,
  Maximize,
  Minimize,
} from "lucide-react";

interface VideoPlayerProps {
  src: string;
  poster?: string | null;
  onError?: (msg: string) => void;
}

function formatTime(secs: number): string {
  if (!Number.isFinite(secs) || secs < 0) return "0:00";
  const m = Math.floor(secs / 60);
  const s = Math.floor(secs % 60);
  const tenths = Math.floor((secs - Math.floor(secs)) * 10);
  return `${m}:${s.toString().padStart(2, "0")}.${tenths}`;
}

const SKIP_SECONDS = 5;

export default function VideoPlayer({ src, poster, onError }: VideoPlayerProps) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [playing, setPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [fullscreen, setFullscreen] = useState(false);
  const [showControls, setShowControls] = useState(true);
  const [videoError, setVideoError] = useState("");
  const hideTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const rafRef = useRef<number | null>(null);

  const updateTime = useCallback(() => {
    const video = videoRef.current;
    if (video && !video.paused && Number.isFinite(video.currentTime)) {
      setCurrentTime(video.currentTime);
      rafRef.current = requestAnimationFrame(updateTime);
    }
  }, []);

  useEffect(() => {
    if (playing) {
      rafRef.current = requestAnimationFrame(updateTime);
    } else {
      if (rafRef.current !== null) cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
    }
    return () => {
      if (rafRef.current !== null) cancelAnimationFrame(rafRef.current);
    };
  }, [playing, updateTime]);

  const togglePlay = useCallback(async () => {
    const video = videoRef.current;
    if (!video) return;
    try {
      if (video.paused) await video.play();
      else video.pause();
    } catch {
      // autoplay blocked
    }
  }, []);

  const seekTo = useCallback((time: number) => {
    const video = videoRef.current;
    if (!video) return;
    video.currentTime = Math.max(0, Math.min(time, video.duration || 0));
    setCurrentTime(video.currentTime);
  }, []);

  const toggleFullscreen = useCallback(async () => {
    const el = containerRef.current;
    if (!el) return;
    try {
      if (document.fullscreenElement) {
        await document.exitFullscreen();
      } else {
        await el.requestFullscreen();
      }
    } catch {
      // fullscreen blocked
    }
  }, []);

  const togglePip = useCallback(async () => {
    if (!videoRef.current) return;
    try {
      if (document.pictureInPictureElement) {
        await document.exitPictureInPicture();
      } else {
        await videoRef.current.requestPictureInPicture();
      }
    } catch {
      // PiP not supported
    }
  }, []);

  useEffect(() => {
    const onFsChange = () => setFullscreen(!!document.fullscreenElement);
    document.addEventListener("fullscreenchange", onFsChange);
    return () => document.removeEventListener("fullscreenchange", onFsChange);
  }, []);

  const handleMouseMove = useCallback(() => {
    setShowControls(true);
    if (hideTimerRef.current) clearTimeout(hideTimerRef.current);
    hideTimerRef.current = setTimeout(() => {
      if (videoRef.current && !videoRef.current.paused) {
        setShowControls(false);
      }
    }, 3000);
  }, []);

  const handleVideoError = () => {
    const msg = "This clip could not be loaded.";
    setVideoError(msg);
    onError?.(msg);
  };

  return (
    <div
      ref={containerRef}
      className="group relative aspect-video overflow-hidden rounded-2xl bg-black shadow-2xl shadow-black/30"
      onMouseMove={handleMouseMove}
      onMouseLeave={() => { if (playing) setShowControls(false); }}
    >
      {!playing && poster ? (
        // eslint-disable-next-line @next/next/no-img-element
        <img src={poster} alt="" className="absolute inset-0 h-full w-full object-cover" />
      ) : null}

      <video
        ref={videoRef}
        src={src}
        className="h-full w-full cursor-pointer"
        onClick={() => void togglePlay()}
        onPlay={() => setPlaying(true)}
        onPause={() => setPlaying(false)}
        onEnded={() => setPlaying(false)}
        onLoadedMetadata={(e) => {
          const d = e.currentTarget.duration;
          setDuration(Number.isFinite(d) ? d : 0);
          setVideoError("");
        }}
        onError={handleVideoError}
        playsInline
        preload="metadata"
      />

      {videoError && (
        <div className="absolute inset-0 flex items-center justify-center bg-black/75 p-4 text-center text-sm text-zinc-200">
          {videoError}
        </div>
      )}

      {!playing && !videoError ? (
        <div className="pointer-events-none absolute inset-0 flex items-center justify-center">
          <button
            onClick={() => void togglePlay()}
            className="pointer-events-auto flex size-16 items-center justify-center rounded-full bg-white/15 text-white backdrop-blur-sm transition active:scale-95 hover:bg-white/25 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20"
            aria-label="Play"
          >
            <Play className="ml-1 size-7 fill-white" />
          </button>
        </div>
      ) : null}

      <div
        className={`absolute inset-x-0 bottom-0 bg-gradient-to-t from-black via-black/80 to-transparent px-4 pb-3 pt-12 transition-opacity duration-300 ${
          showControls ? "opacity-100" : "opacity-0"
        }`}
      >
        <input
          aria-label="Seek"
          type="range"
          min={0}
          max={Math.max(duration, 1)}
          step="0.01"
          value={Math.min(currentTime, duration || 0)}
          onChange={(e) => seekTo(Number(e.target.value))}
          className="h-1.5 w-full cursor-pointer accent-white"
        />
        <div className="mt-3 flex items-center justify-between">
          <div className="flex items-center gap-2 text-white">
            <button
              onClick={() => void togglePlay()}
              className="rounded-lg p-1.5 transition active:scale-90 hover:bg-white/15 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20"
              aria-label={playing ? "Pause" : "Play"}
            >
              {playing ? <Pause className="size-5" /> : <Play className="size-5 fill-white" />}
            </button>
            <button
              onClick={() => seekTo(currentTime - SKIP_SECONDS)}
              className="rounded-lg p-1.5 transition active:scale-90 hover:bg-white/15 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20"
              aria-label="Skip back 5 seconds"
            >
              <SkipBack className="size-5" />
            </button>
            <button
              onClick={() => seekTo(currentTime + SKIP_SECONDS)}
              className="rounded-lg p-1.5 transition active:scale-90 hover:bg-white/15 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20"
              aria-label="Skip forward 5 seconds"
            >
              <SkipForward className="size-5" />
            </button>
            <button
              onClick={() => void togglePip()}
              className="rounded-lg p-1.5 text-white/70 transition active:scale-90 hover:bg-white/15 hover:text-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20"
              aria-label="Picture in picture"
            >
              <PictureInPicture2 className="size-4" />
            </button>
          </div>
          <div className="flex items-center gap-3">
            <span className="text-xs tabular-nums text-white/75">
              {formatTime(currentTime)} / {formatTime(duration)}
            </span>
            <button
              onClick={() => void toggleFullscreen()}
              className="rounded-lg p-1.5 text-white/70 transition active:scale-90 hover:bg-white/15 hover:text-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20"
              aria-label={fullscreen ? "Exit fullscreen" : "Fullscreen"}
            >
              {fullscreen ? <Minimize className="size-4" /> : <Maximize className="size-4" />}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
