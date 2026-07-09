import { useState, useRef, useEffect, useMemo } from "react";
import { useNavigate, useParams, useLocation } from "react-router-dom";
import { ArrowLeft, Play, Pause, PictureInPicture2, Edit3, Check, X } from "lucide-react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useClipsStore, formatSize, formatDuration, formatDate } from "@/stores/clips";
import type { Clip } from "@/stores/clips";

export default function ClipDetailPage() {
  const { filename } = useParams<{ filename: string }>();
  const navigate = useNavigate();
  const location = useLocation();
  const clips = useClipsStore((s) => s.clips);
  const renameClip = useClipsStore((s) => s.renameClip);
  const loadClips = useClipsStore((s) => s.loadClips);
  const loaded = useClipsStore((s) => s.loaded);
  const loading = useClipsStore((s) => s.loading);

  // Prefer clip from location state, fall back to store lookup
  const clip: Clip | undefined = useMemo(
    () =>
      (location.state as { clip?: Clip })?.clip ??
      clips.find((c) => c.filename === filename),
    [location.state, clips, filename],
  );

  const [playing, setPlaying] = useState(false);
  const [renaming, setRenaming] = useState(false);
  const [newName, setNewName] = useState("");
  const [renameError, setRenameError] = useState("");
  const [videoError, setVideoError] = useState("");
  const videoRef = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    if (clip) setNewName(clip.filename.replace(/\.mp4$/, ""));
  }, [clip]);

  useEffect(() => {
    if (!clip && !loading && !loaded) {
      void loadClips();
    }
  }, [clip, loaded, loadClips, loading]);

  if (!clip) {
    if (loading || !loaded) {
      return (
        <div className="h-full flex items-center justify-center text-zinc-500 text-sm">
          Loading clip...
        </div>
      );
    }

    return (
      <div className="h-full flex flex-col items-center justify-center text-zinc-500 gap-3">
        <p className="text-sm">Clip not found</p>
        <button
          onClick={() => navigate("/library")}
          className="text-xs text-zinc-400 hover:text-zinc-200 transition-colors"
        >
          Back to library
        </button>
      </div>
    );
  }

  const videoSrc = convertFileSrc(clip.path);

  const togglePlay = () => {
    if (!videoRef.current) return;
    if (videoRef.current.paused) {
      videoRef.current.play();
      setPlaying(true);
    } else {
      videoRef.current.pause();
      setPlaying(false);
    }
  };

  const togglePip = async () => {
    if (!videoRef.current) return;
    try {
      if (document.pictureInPictureElement) {
        await document.exitPictureInPicture();
      } else {
        await videoRef.current.requestPictureInPicture();
      }
    } catch (err) {
      console.error("PiP failed:", err);
    }
  };

  const handleRename = async () => {
    const trimmed = newName.trim();
    if (!trimmed) {
      setRenameError("Name cannot be empty");
      return;
    }
    if (trimmed.includes("/") || trimmed.includes("\\")) {
      setRenameError("No path separators allowed");
      return;
    }
    try {
      await renameClip(clip.filename, trimmed);
      setRenaming(false);
      setRenameError("");
    } catch (err: any) {
      setRenameError(typeof err === "string" ? err : "Rename failed");
    }
  };

  const cancelRename = () => {
    setRenaming(false);
    setNewName(clip.filename.replace(/\.mp4$/, ""));
    setRenameError("");
  };

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <header className="flex items-center gap-3 px-6 pt-5 pb-3">
        <button
          onClick={() => navigate("/library")}
          className="p-1.5 rounded-lg text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800 transition-colors"
        >
          <ArrowLeft className="size-5" />
        </button>
        <div className="flex-1 min-w-0">
          {renaming ? (
            <div className="flex items-center gap-2">
              <input
                type="text"
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleRename();
                  if (e.key === "Escape") cancelRename();
                }}
                autoFocus
                className="bg-zinc-900 border border-zinc-700 rounded-lg px-3 py-1 text-sm text-zinc-100 font-mono focus:outline-none focus:ring-1 focus:ring-zinc-600"
              />
              <button
                onClick={handleRename}
                className="p-1 rounded text-emerald-400 hover:text-emerald-300 transition-colors"
              >
                <Check className="size-4" />
              </button>
              <button
                onClick={cancelRename}
                className="p-1 rounded text-zinc-500 hover:text-zinc-300 transition-colors"
              >
                <X className="size-4" />
              </button>
              {renameError && (
                <span className="text-xs text-red-400">{renameError}</span>
              )}
            </div>
          ) : (
            <div className="flex items-center gap-2">
              <h1 className="text-sm font-medium text-zinc-100 truncate">
                {clip.filename.replace(/\.mp4$/, "")}
              </h1>
              <button
                onClick={() => setRenaming(true)}
                className="p-1 rounded text-zinc-500 hover:text-zinc-300 transition-colors"
                title="Rename"
              >
                <Edit3 className="size-3.5" />
              </button>
            </div>
          )}
        </div>
        <span className="text-xs text-zinc-500">{formatSize(clip.size_bytes)}</span>
      </header>

      {/* Video Player */}
      <div className="flex-1 flex flex-col items-center justify-center px-6 pb-4">
        <div className="relative w-full max-w-3xl aspect-video bg-black rounded-xl overflow-hidden group">
          <video
            ref={videoRef}
            src={videoSrc}
            className="w-full h-full"
            onClick={togglePlay}
            onEnded={() => setPlaying(false)}
            onPlay={() => setPlaying(true)}
            onPause={() => setPlaying(false)}
            onError={() => setVideoError("This clip could not be loaded in the app.")}
            onLoadedData={() => setVideoError("")}
            controls={false}
            preload="metadata"
            playsInline
          />

          {videoError && (
            <div className="absolute inset-0 flex items-center justify-center bg-black/70 p-4 text-center">
              <p className="text-sm text-zinc-200">{videoError}</p>
            </div>
          )}

          {/* Play overlay (hidden when playing) */}
          {!playing && (
            <div className="absolute inset-0 flex items-center justify-center">
              <button
                onClick={togglePlay}
                className="size-16 rounded-full bg-white/10 backdrop-blur-sm flex items-center justify-center hover:bg-white/20 transition-colors"
              >
                <Play className="size-7 text-white fill-white ml-1" />
              </button>
            </div>
          )}

          {/* Bottom controls bar */}
          <div className="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/80 to-transparent px-4 pb-3 pt-8 opacity-0 group-hover:opacity-100 transition-opacity">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <button
                  onClick={togglePlay}
                  className="text-white/80 hover:text-white transition-colors"
                >
                  {playing ? <Pause className="size-5" /> : <Play className="size-5" />}
                </button>
                <button
                  onClick={togglePip}
                  className="text-white/60 hover:text-white transition-colors"
                  title="Picture-in-Picture"
                >
                  <PictureInPicture2 className="size-4" />
                </button>
              </div>
              <span className="text-[11px] text-white/60 tabular-nums">
                {formatDuration(clip.duration_secs)}
              </span>
            </div>
          </div>
        </div>

        {/* Metadata */}
        <div className="mt-4 w-full max-w-3xl flex items-center gap-4 text-xs text-zinc-500">
          <span>{formatDate(clip.created_at)}</span>
          <span className="text-zinc-700">·</span>
          <span>{formatDuration(clip.duration_secs)}</span>
          <span className="text-zinc-700">·</span>
          <span className="truncate">{clip.path}</span>
        </div>
      </div>
    </div>
  );
}
