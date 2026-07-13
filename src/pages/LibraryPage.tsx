import { useState, useEffect, useCallback, useMemo, memo } from "react";
import { useNavigate } from "react-router-dom";
import { Search, Filter, Film, Trash2, FolderOpen, Play, Upload, Check, Link2, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useClipsStore, formatSize, formatDuration, formatDate, type Clip } from "@/stores/clips";
import { useCloudStore } from "@/stores/cloud";
import ClipThumbnail from "@/components/common/ClipThumbnail";

function ClipCard({ clip, task, showConfirm, confirming, uploadingClip, onDelete, onUpload, onNavigate, onCopyShare, onShowConfirm, onHideConfirm, cloudAuthed }: {
  clip: Clip;
  task: { status: string; progress: number; share_url?: string; clip_path: string; error?: string | null } | undefined;
  showConfirm: string | null;
  confirming: boolean;
  uploadingClip: string | null;
  onDelete: (filename: string) => void;
  onUpload: (path: string, filename: string, game: string) => void;
  onNavigate: (filename: string) => void;
  onCopyShare: (url: string) => void;
  onShowConfirm: (filename: string) => void;
  onHideConfirm: () => void;
  cloudAuthed: boolean;
}) {
  const status = task?.status;
  const isUploaded = status === "Completed";
  const isUploading = (status === "Uploading" || uploadingClip === clip.filename);
  const isFailed = typeof status === "string" && (status === "Failed" || status.startsWith("Failed"));
  const shareUrl = task?.share_url;
  const displayName = clip.title || clip.filename.replace(/\.mp4$/, "");

  return (
    <div
      onClick={() => onNavigate(clip.filename)}
      className="group aspect-video bg-surface rounded-2xl border border-border overflow-hidden relative cursor-pointer transition hover:scale-[1.02]"
    >
      <ClipThumbnail path={clip.path} filename={clip.filename} />

      {isUploaded ? (
        <div className="absolute top-2 left-2 flex items-center gap-1 px-2 py-0.5 rounded-full bg-emerald-600/80 text-[10px] text-white font-medium">
          <Check className="size-3" />
          Uploaded
        </div>
      ) : isFailed ? (
        <div
          className="absolute top-2 left-2 flex items-center gap-1 px-2 py-0.5 rounded-full bg-red-600/80 text-[10px] text-white font-medium"
          title={task?.error || "Upload failed"}
        >
          Failed
        </div>
      ) : isUploading ? (
        <div className="absolute top-2 left-2 flex items-center gap-1 px-2 py-0.5 rounded-full bg-accent/80 text-[10px] text-white font-medium">
          <Loader2 className="size-3 animate-spin" />
          Uploading
        </div>
      ) : null}

      {isUploading && task && (
        <div className="absolute bottom-9 left-2 right-2 h-1 rounded-full bg-white/10 overflow-hidden">
          <div
            className="h-full bg-accent transition-all duration-300"
            style={{ width: `${task.progress * 100}%` }}
          />
        </div>
      )}

      <div className="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/80 to-transparent px-3 pb-2 pt-6">
        <p className="truncate text-xs font-medium text-white">{displayName}</p>
        {clip.game && <p className="truncate text-[11px] text-blue-200/80 mt-0.5">{clip.game}</p>}
        <div className="flex items-center justify-between text-[11px] text-zinc-400">
          <span>{formatDuration(clip.duration_secs)}</span>
          <span>{formatSize(clip.size_bytes)}</span>
        </div>
        <p className="text-[11px] text-zinc-500 mt-0.5">
          {formatDate(clip.created_at)}
        </p>
      </div>

      <div className="absolute inset-0 opacity-0 group-hover:opacity-100 transition">
        <div className="absolute inset-0 bg-[linear-gradient(135deg,rgba(79,140,255,0.12),rgba(119,168,255,0.04))]" />
        <div className="absolute inset-0 bg-black/40 flex items-center justify-center gap-3">
        <button
          onClick={(e) => { e.stopPropagation(); onNavigate(clip.filename); }}
          className="p-3 rounded-full bg-white/15 hover:bg-white/25 text-white transition active:scale-90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20"
          title="Play in app"
        >
          <Play className="size-5 fill-white ml-0.5" />
        </button>

        {isUploaded && shareUrl ? (
          <button
            onClick={(e) => { e.stopPropagation(); onCopyShare(shareUrl); }}
            className="p-3 rounded-full bg-emerald-600/40 hover:bg-emerald-600/60 text-white transition active:scale-90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20"
            title="Copy share link"
          >
            <Link2 className="size-5" />
          </button>
        ) : cloudAuthed ? (
          <button
            onClick={(e) => { e.stopPropagation(); onUpload(clip.path, clip.filename, clip.game); }}
            disabled={isUploading}
            className="p-3 rounded-full bg-accent/40 hover:bg-accent/60 text-white transition active:scale-90 disabled:opacity-40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20"
            title={isUploading ? "Uploading..." : "Upload to cloud"}
          >
            {isUploading ? (
              <Loader2 className="size-5 animate-spin" />
            ) : (
              <Upload className="size-5" />
            )}
          </button>
        ) : null}

        <button
          onClick={(e) => { e.stopPropagation(); onShowConfirm(clip.filename); }}
          className="p-2 rounded-lg bg-zinc-800/80 hover:bg-red-900/60 text-zinc-300 hover:text-red-300 transition active:scale-90 absolute top-2 right-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20"
          title="Delete clip"
        >
          <Trash2 className="size-4" />
        </button>
      </div>
      </div>

      {showConfirm === clip.filename && (
        <div
          className="absolute inset-0 bg-[#050816]/90 flex flex-col items-center justify-center p-4 gap-3"
          onClick={(e) => e.stopPropagation()}
        >
          <p className="text-sm text-zinc-300 text-center">Delete this clip?</p>
          <div className="flex gap-2">
            <Button
              variant="destructive"
              size="xs"
              onClick={() => onDelete(clip.filename)}
              disabled={confirming}
            >
              {confirming ? "Deleting..." : "Delete"}
            </Button>
            <Button
              variant="ghost"
              size="xs"
              onClick={onHideConfirm}
            >
              Cancel
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}

const MemoClipCard = memo(ClipCard);

export default function LibraryPage() {
  const navigate = useNavigate();
  const clips = useClipsStore((s) => s.clips);
  const loading = useClipsStore((s) => s.loading);
  const loadClips = useClipsStore((s) => s.loadClips);
  const deleteClip = useClipsStore((s) => s.deleteClip);
  const openClipLocation = useClipsStore((s) => s.openClipLocation);

  const uploads = useCloudStore((s) => s.uploads);
  const uploadClip = useCloudStore((s) => s.uploadClip);
  const copyShareUrl = useCloudStore((s) => s.copyShareUrl);
  const cloudAuthed = useCloudStore((s) => s.authenticated);
  const uploadError = useCloudStore((s) => s.uploadError);
  const clearUploadError = useCloudStore((s) => s.clearUploadError);

  const [search, setSearch] = useState("");
  const [showConfirm, setShowConfirm] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [uploadingClip, setUploadingClip] = useState<string | null>(null);

  useEffect(() => {
    loadClips();
    useCloudStore.getState().uploadQueueStatus();
  }, [loadClips]);

  const filtered = useMemo(
    () => clips.filter((clip) => {
      const query = search.toLowerCase();
      return [clip.filename, clip.title, clip.description, clip.game]
        .some((value) => value.toLowerCase().includes(query));
    }),
    [clips, search],
  );

  const uploadMap = useMemo(
    () => new Map(uploads.map((t) => [t.clip_path, t])),
    [uploads],
  );

  const handleDelete = useCallback(async (filename: string) => {
    setConfirming(true);
    try {
      await deleteClip(filename);
    } finally {
      setConfirming(false);
      setShowConfirm(null);
    }
  }, [deleteClip]);

  const handleUpload = useCallback(async (path: string, filename: string, game: string) => {
    setUploadingClip(filename);
    try {
      await uploadClip(path, filename, game || undefined);
    } finally {
      setUploadingClip(null);
    }
  }, [uploadClip]);

  return (
    <div className="h-full flex flex-col">
      <header className="px-6 pt-6 pb-4">
        <div className="flex items-center justify-between">
          <div>
                <h1 className="text-xl font-semibold text-white">Clip Library</h1>
            <p className="text-sm text-zinc-500 mt-1">
              {clips.length} clip{clips.length !== 1 ? "s" : ""}
            </p>
          </div>
          <Button
            variant="secondary"
            size="sm"
            onClick={openClipLocation}
          >
            <FolderOpen className="size-4" />
            Open Folder
          </Button>
        </div>

        <div className="mt-4 flex items-center gap-3">
          <div className="relative flex-1 max-w-xs">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-zinc-500" />
            <input
              type="text"
              placeholder="Search clips..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="w-full pl-9 pr-3 py-1.5 text-sm bg-surface border border-border rounded-xl text-white placeholder-zinc-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-400/70"
            />
          </div>
          <Button variant="secondary" size="sm" onClick={() => {}}>
            <Filter className="size-4" />
            Filter
          </Button>
        </div>

        {uploadError && (
          <div className="mt-3 flex items-start gap-2 px-4 py-2.5 rounded-lg bg-red-950/60 border border-red-900/60">
            <p className="text-xs text-red-300 flex-1">{uploadError}</p>
            <button
              onClick={clearUploadError}
              className="p-0.5 rounded text-red-400 hover:text-red-200 transition active:scale-90 shrink-0"
            >
              <span className="text-xs font-medium">Dismiss</span>
            </button>
          </div>
        )}
      </header>

      <div className="flex-1 px-6 pb-6 overflow-y-auto">
        {loading ? (
          <div className="flex items-center justify-center h-48 text-sm text-zinc-600">
            Loading clips...
          </div>
        ) : filtered.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-48 text-sm text-zinc-600">
            <Film className="size-10 text-zinc-700 mb-3" />
            <p>{clips.length === 0 ? "No clips yet" : "No clips match your search"}</p>
            <p className="text-xs text-zinc-700 mt-1">
              {clips.length === 0
                ? "Clips will appear here once you save them."
                : "Try a different search term."}
            </p>
          </div>
        ) : (
          <div className="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
            {filtered.map((clip) => (
              <MemoClipCard
                key={clip.id}
                clip={clip}
                task={uploadMap.get(clip.path)}
                showConfirm={showConfirm}
                confirming={confirming}
                uploadingClip={uploadingClip}
                onDelete={handleDelete}
                onUpload={handleUpload}
                onNavigate={(filename) => navigate(`/clip/${filename}`)}
                onCopyShare={copyShareUrl}
                onShowConfirm={setShowConfirm}
                onHideConfirm={() => setShowConfirm(null)}
                cloudAuthed={cloudAuthed}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
