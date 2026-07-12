import { useEffect, useMemo, useState } from "react";
import { useLocation, useNavigate, useParams } from "react-router-dom";
import { ArrowLeft, Edit3, Gamepad2, Save } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/brand";
import { convertFileSrc } from "@tauri-apps/api/core";
import { formatDate, formatDuration, formatSize, useClipsStore } from "@/stores/clips";
import VideoPlayer from "@/components/common/VideoPlayer";
import type { Clip } from "@/stores/clips";

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

  const [editing, setEditing] = useState(false);
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [game, setGame] = useState("");
  const [saving, setSaving] = useState(false);
  const [editorError, setEditorError] = useState("");

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
  const displayTitle = clip.title || clip.filename.replace(/\.mp4$/, "");

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
        <section className="relative overflow-hidden rounded-2xl border border-border bg-black shadow-2xl shadow-black/30">
          <VideoPlayer src={videoSrc} poster={posterSrc} />
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
                <input value={title} maxLength={120} onChange={(event) => setTitle(event.target.value)} className="rounded-xl border border-border bg-black/20 px-3 py-2 text-sm text-white outline-none transition-colors focus:border-blue-400/70 focus:ring-2 focus:ring-blue-500/20" />
              </label>
              <label className="grid gap-1.5 text-xs font-medium text-zinc-400">Game
                <input value={game} maxLength={120} onChange={(event) => setGame(event.target.value)} placeholder="e.g. Counter-Strike 2" className="rounded-xl border border-border bg-black/20 px-3 py-2 text-sm text-white placeholder:text-zinc-600 outline-none transition-colors focus:border-blue-400/70 focus:ring-2 focus:ring-blue-500/20" />
              </label>
              <label className="grid gap-1.5 text-xs font-medium text-zinc-400">Description
                <textarea value={description} maxLength={2000} onChange={(event) => setDescription(event.target.value)} placeholder="What happened in this clip?" rows={4} className="resize-y rounded-xl border border-border bg-black/20 px-3 py-2 text-sm text-white placeholder:text-zinc-600 outline-none transition-colors focus:border-blue-400/70 focus:ring-2 focus:ring-blue-500/20" />
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
