import { useEffect, useMemo, useRef, useState } from "react";
import { useLocation, useNavigate, useParams } from "react-router-dom";
import {
  ArrowLeft,
  Edit3,
  Gamepad2,
  Pause,
  PictureInPicture2,
  Play,
  Save,
  SkipBack,
  SkipForward,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/brand";
import { convertFileSrc } from "@tauri-apps/api/core";
import { formatDate, formatDuration, formatSize, useClipsStore } from "@/stores/clips";
import type { Clip } from "@/stores/clips";

const SKIP_SECONDS = 5;

export default function ClipDetailPage() {
  const { filename } = useParams<{ filename: string }>();
  const navigate = useNavigate();
  const location = useLocation();
  const clips = useClipsStore((state) => state.clips);
  const updateClipMetadata = useClipsStore((state) => state.updateClipMetadata);
  const loadClips = useClipsStore((state) => state.loadClips);
  const loaded = useClipsStore((state) => state.loaded);
  const loading = useClipsStore((state) => state.loading);

  const clip: Clip | undefined = useMemo(
    () => clips.find((item) => item.filename === filename) ?? (location.state as { clip?: Clip })?.clip,
    [clips, filename, location.state],
  );

  const [playing, setPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [videoDuration, setVideoDuration] = useState(0);
  const [editing, setEditing] = useState(false);
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [game, setGame] = useState("");
  const [saving, setSaving] = useState(false);
  const [editorError, setEditorError] = useState("");
  const [videoError, setVideoError] = useState("");
  const videoRef = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    if (clip) {
      setTitle(clip.title || clip.filename.replace(/\.mp4$/, ""));
      setDescription(clip.description);
      setGame(clip.game);
    }
  }, [clip]);

  useEffect(() => {
    if (!clip && !loading && !loaded) {
      void loadClips();
    }
  }, [clip, loaded, loadClips, loading]);

  if (!clip) {
    if (loading || !loaded) {
      return <div className="h-full flex items-center justify-center text-zinc-500 text-sm">Loading clip...</div>;
    }
    return (
      <div className="h-full flex flex-col items-center justify-center text-zinc-500 gap-3">
        <p className="text-sm">Clip not found</p>
        <button onClick={() => navigate("/library")} className="text-xs text-zinc-400 hover:text-zinc-200 transition">
          Back to library
        </button>
      </div>
    );
  }

  const videoSrc = convertFileSrc(clip.path);
  const posterSrc = convertFileSrc(clip.path.replace(/\.mp4$/, "_thumb.jpg"));
  const totalDuration = videoDuration || clip.duration_secs;
  const displayTitle = clip.title || clip.filename.replace(/\.mp4$/, "");

  const togglePlay = async () => {
    const video = videoRef.current;
    if (!video) return;
    try {
      if (video.paused) await video.play();
      else video.pause();
    } catch {
      setVideoError("Playback could not be started for this clip.");
    }
  };

  const seekTo = (time: number) => {
    const video = videoRef.current;
    if (!video) return;
    const nextTime = Math.max(0, Math.min(time, Number.isFinite(video.duration) ? video.duration : totalDuration));
    video.currentTime = nextTime;
    setCurrentTime(nextTime);
  };

  const togglePip = async () => {
    if (!videoRef.current) return;
    try {
      if (document.pictureInPictureElement) await document.exitPictureInPicture();
      else await videoRef.current.requestPictureInPicture();
    } catch (error) {
      console.error("Picture-in-Picture failed:", error);
    }
  };

  const saveMetadata = async () => {
    const trimmedTitle = title.trim();
    if (!trimmedTitle) {
      setEditorError("A clip name is required.");
      return;
    }
    setSaving(true);
    try {
      await updateClipMetadata(clip.filename, {
        title: trimmedTitle,
        description: description.trim(),
        game: game.trim(),
      });
      setEditing(false);
      setEditorError("");
    } catch (error) {
      setEditorError(typeof error === "string" ? error : "Could not save clip details.");
    } finally {
      setSaving(false);
    }
  };

  const cancelEdit = () => {
    setTitle(clip.title || clip.filename.replace(/\.mp4$/, ""));
    setDescription(clip.description);
    setGame(clip.game);
    setEditorError("");
    setEditing(false);
  };

  return (
    <div className="h-full overflow-y-auto">
      <header className="flex items-center gap-3 px-6 pt-5 pb-3">
        <button onClick={() => navigate("/library")} className="p-1.5 rounded-xl text-zinc-500 hover:text-zinc-200 hover:bg-white/5 transition active:scale-90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20" aria-label="Back to library">
          <ArrowLeft className="size-5" />
        </button>
        <div className="min-w-0 flex-1">
          <h1 className="truncate text-base font-semibold text-white">{displayTitle}</h1>
          <p className="mt-0.5 truncate text-xs text-zinc-500">{clip.filename}</p>
        </div>
        <span className="text-xs text-zinc-500">{formatSize(clip.size_bytes)}</span>
      </header>

      <main className="mx-auto flex w-full max-w-5xl flex-col gap-5 px-6 pb-8">
        <section className="relative aspect-video overflow-hidden rounded-2xl border border-white/10 bg-black shadow-2xl shadow-black/30">
          {!playing && (
            <img src={posterSrc} alt="" className="absolute inset-0 h-full w-full object-cover" onError={(event) => { event.currentTarget.style.display = "none"; }} />
          )}
          <video
            ref={videoRef}
            src={videoSrc}
            className="h-full w-full"
            onClick={() => void togglePlay()}
            onEnded={() => setPlaying(false)}
            onPlay={() => setPlaying(true)}
            onPause={() => setPlaying(false)}
            onTimeUpdate={(event) => setCurrentTime(event.currentTarget.currentTime)}
            onLoadedMetadata={(event) => {
              setVideoDuration(Number.isFinite(event.currentTarget.duration) ? event.currentTarget.duration : 0);
              setVideoError("");
            }}
            onError={() => setVideoError("This clip could not be loaded in the app.")}
            controls={false}
            preload="metadata"
            playsInline
          />

          {videoError && <div className="absolute inset-0 flex items-center justify-center bg-black/75 p-4 text-center text-sm text-zinc-200">{videoError}</div>}

          {!playing && !videoError && (
            <div className="pointer-events-none absolute inset-0 flex items-center justify-center">
              <button onClick={() => void togglePlay()} className="pointer-events-auto flex size-16 items-center justify-center rounded-full bg-white/15 text-white backdrop-blur-sm transition active:scale-95 hover:bg-white/25 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20" aria-label="Play clip">
                <Play className="ml-1 size-7 fill-white" />
              </button>
            </div>
          )}

          <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black via-black/80 to-transparent px-4 pb-3 pt-12">
            <input
              aria-label="Clip timeline"
              type="range"
              min="0"
              max={Math.max(totalDuration, 1)}
              step="0.01"
              value={Math.min(currentTime, totalDuration || 0)}
              onChange={(event) => seekTo(Number(event.target.value))}
              className="h-1.5 w-full cursor-pointer accent-white"
            />
            <div className="mt-3 flex items-center justify-between">
              <div className="flex items-center gap-2 text-white">
                <button onClick={() => void togglePlay()} className="rounded-lg p-1.5 transition active:scale-90 hover:bg-white/15 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20" aria-label={playing ? "Pause clip" : "Play clip"}>
                  {playing ? <Pause className="size-5" /> : <Play className="size-5 fill-white" />}
                </button>
                <button onClick={() => seekTo(currentTime - SKIP_SECONDS)} className="rounded-lg p-1.5 transition active:scale-90 hover:bg-white/15 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20" aria-label="Skip back 5 seconds">
                  <SkipBack className="size-5" />
                </button>
                <button onClick={() => seekTo(currentTime + SKIP_SECONDS)} className="rounded-lg p-1.5 transition active:scale-90 hover:bg-white/15 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20" aria-label="Skip forward 5 seconds">
                  <SkipForward className="size-5" />
                </button>
                <button onClick={() => void togglePip()} className="rounded-lg p-1.5 text-white/70 transition active:scale-90 hover:bg-white/15 hover:text-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20" aria-label="Picture in picture">
                  <PictureInPicture2 className="size-4" />
                </button>
              </div>
              <span className="text-xs tabular-nums text-white/75">{formatDuration(Math.floor(currentTime))} / {formatDuration(Math.floor(totalDuration))}</span>
            </div>
          </div>
        </section>

        <Card><div className="p-4 sm:p-5">
          <div className="mb-4 flex items-center justify-between gap-4">
            <div>
              <h2 className="text-sm font-semibold text-white">Clip details</h2>
              <p className="mt-1 text-xs text-zinc-500">Saved separately from the video, ready for future clip editing.</p>
            </div>
            {!editing && <Button variant="secondary" size="sm" onClick={() => setEditing(true)}><Edit3 className="size-3.5" /> Edit</Button>}
          </div>

          {editing ? (
            <div className="grid gap-4">
              <label className="grid gap-1.5 text-xs font-medium text-zinc-400">Name
                <input value={title} maxLength={120} onChange={(event) => setTitle(event.target.value)} className="rounded-xl border border-white/10 bg-black/20 px-3 py-2 text-sm text-white outline-none transition-colors focus:border-blue-400/70 focus:ring-2 focus:ring-blue-500/20" />
              </label>
              <label className="grid gap-1.5 text-xs font-medium text-zinc-400">Game
                <input value={game} maxLength={120} onChange={(event) => setGame(event.target.value)} placeholder="e.g. Counter-Strike 2" className="rounded-xl border border-white/10 bg-black/20 px-3 py-2 text-sm text-white placeholder:text-zinc-600 outline-none transition-colors focus:border-blue-400/70 focus:ring-2 focus:ring-blue-500/20" />
              </label>
              <label className="grid gap-1.5 text-xs font-medium text-zinc-400">Description
                <textarea value={description} maxLength={2000} onChange={(event) => setDescription(event.target.value)} placeholder="What happened in this clip?" rows={4} className="resize-y rounded-xl border border-white/10 bg-black/20 px-3 py-2 text-sm text-white placeholder:text-zinc-600 outline-none transition-colors focus:border-blue-400/70 focus:ring-2 focus:ring-blue-500/20" />
              </label>
              {editorError && <p className="text-xs text-red-400">{editorError}</p>}
              <div className="flex justify-end gap-2">
                <Button variant="ghost" size="xs" onClick={cancelEdit} disabled={saving}>Cancel</Button>
                <Button variant="brand" size="sm" onClick={() => void saveMetadata()} disabled={saving}><Save className="size-3.5" /> {saving ? "Saving..." : "Save details"}</Button>
              </div>
            </div>
          ) : (
            <div className="grid gap-4 text-sm sm:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
              <div className="min-w-0"><p className="text-[11px] font-medium uppercase tracking-wider text-zinc-600">Game</p><p className="mt-1 flex items-center gap-1.5 text-zinc-200"><Gamepad2 className="size-4 text-blue-300" /> {clip.game || "Not set"}</p></div>
              <div><p className="text-[11px] font-medium uppercase tracking-wider text-zinc-600">Captured</p><p className="mt-1 text-zinc-200">{formatDate(clip.created_at)} · {formatDuration(clip.duration_secs)}</p></div>
              <div className="sm:col-span-2"><p className="text-[11px] font-medium uppercase tracking-wider text-zinc-600">Description</p><p className="mt-1 whitespace-pre-wrap leading-6 text-zinc-300">{clip.description || "No description yet."}</p></div>
            </div>
          )}
        </div></Card>
      </main>
    </div>
  );
}
