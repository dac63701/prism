import { useState, useEffect, useCallback, useMemo, memo, useTransition } from "react";
import { useNavigate } from "react-router-dom";
import { Film, FolderOpen, Play, Upload, Check, Link2, Loader2, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Select } from "@/components/ui/select";
import { Dialog } from "@/components/ui/dialog";
import { EmptyState } from "@/components/ui/empty-state";
import { SkeletonClipsGrid } from "@/components/ui/skeleton";
import { useClipsStore, formatSize, formatDuration, formatDate, type Clip } from "@/stores/clips";
import { useCloudStore } from "@/stores/cloud";
import { toast } from "@/stores/toast";
import ClipThumbnail from "@/components/common/ClipThumbnail";

type SortKey = "newest" | "oldest" | "name" | "size" | "duration";
type StatusFilter = "all" | "uploaded" | "uploading" | "failed";

function ClipCard({ clip, task, confirming, uploadingClip, onDelete, onUpload, onNavigate, onCopyShare, cloudAuthed }: {
  clip: Clip;
  task: { status: string; progress: number; share_url?: string; clip_path: string; error?: string | null } | undefined;
  confirming: boolean;
  uploadingClip: string | null;
  onDelete: (filename: string) => void;
  onUpload: (path: string, filename: string, game: string) => void;
  onNavigate: (filename: string) => void;
  onCopyShare: (url: string) => void;
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
      className="group content-visibility-auto relative aspect-video cursor-pointer overflow-hidden rounded-2xl border border-border bg-surface transition hover:scale-[1.02]"
    >
      <ClipThumbnail path={clip.path} filename={clip.filename} />

      {isUploaded ? (
        <div className="absolute left-2 top-2 flex items-center gap-1 rounded-full bg-emerald-600/80 px-2 py-0.5 text-[10px] font-medium text-white">
          <Check className="size-3" />
          Uploaded
        </div>
      ) : isFailed ? (
        <div
          className="absolute left-2 top-2 flex items-center gap-1 rounded-full bg-red-600/80 px-2 py-0.5 text-[10px] font-medium text-white"
          title={task?.error || "Upload failed"}
        >
          Failed
        </div>
      ) : isUploading ? (
        <div className="absolute left-2 top-2 flex items-center gap-1 rounded-full bg-accent/80 px-2 py-0.5 text-[10px] font-medium text-white">
          <Loader2 className="size-3 animate-spin" />
          Uploading
        </div>
      ) : null}

      {isUploading && task && (
        <div className="absolute bottom-9 left-2 right-2 h-1 overflow-hidden rounded-full bg-white/10">
          <div
            className="h-full rounded-full bg-accent transition-all duration-300"
            style={{ width: `${task.progress * 100}%` }}
          />
        </div>
      )}

      <div className="absolute right-2 top-2 rounded-full bg-black/60 px-2 py-0.5 text-[10px] font-medium text-white/90 backdrop-blur-sm">
        {formatDuration(clip.duration_secs)}
      </div>

      <div className="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/80 to-transparent px-3 pb-2 pt-6">
        <p className="truncate text-xs font-medium text-white">{displayName}</p>
        {clip.game && <p className="mt-0.5 truncate text-[11px] text-blue-200/80">{clip.game}</p>}
        <div className="flex items-center justify-between text-[11px] text-zinc-400">
          <span>{formatSize(clip.size_bytes)}</span>
          <span>{formatDate(clip.created_at)}</span>
        </div>
      </div>

      <div className="absolute inset-0 opacity-0 transition group-hover:opacity-100">
        <div className="absolute inset-0 bg-[linear-gradient(135deg,rgba(79,140,255,0.12),rgba(119,168,255,0.04))]" />
        <div className="absolute inset-0 flex items-center justify-center gap-3 bg-black/40">
          <button
            onClick={(e) => { e.stopPropagation(); onNavigate(clip.filename); }}
            className="rounded-full bg-white/15 p-3 text-white transition hover:bg-white/25 active:scale-90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20"
            title="Play in app"
            aria-label="Play in app"
          >
            <Play className="ml-0.5 size-5 fill-white" />
          </button>

          {isUploaded && shareUrl ? (
            <button
              onClick={(e) => { e.stopPropagation(); onCopyShare(shareUrl); }}
              className="rounded-full bg-emerald-600/40 p-3 text-white transition hover:bg-emerald-600/60 active:scale-90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20"
              title="Copy share link"
              aria-label="Copy share link"
            >
              <Link2 className="size-5" />
            </button>
          ) : cloudAuthed ? (
            <button
              onClick={(e) => { e.stopPropagation(); onUpload(clip.path, clip.filename, clip.game); }}
              disabled={isUploading}
              className="rounded-full bg-accent/40 p-3 text-white transition hover:bg-accent/60 active:scale-90 disabled:opacity-40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20"
              title={isUploading ? "Uploading..." : "Upload to cloud"}
              aria-label={isUploading ? "Uploading..." : "Upload to cloud"}
            >
              {isUploading ? (
                <Loader2 className="size-5 animate-spin" />
              ) : (
                <Upload className="size-5" />
              )}
            </button>
          ) : null}
        </div>

        <button
          onClick={(e) => { e.stopPropagation(); onDelete(clip.filename); }}
          className="absolute right-2 top-2 rounded-lg bg-zinc-800/80 p-2 text-zinc-300 transition hover:bg-red-900/60 hover:text-red-300 active:scale-90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20"
          title="Delete clip"
          aria-label="Delete clip"
        >
          <Trash2 className="size-4" />
        </button>
      </div>

      <div className="pointer-events-none absolute inset-0 rounded-2xl ring-1 ring-inset ring-white/0 transition group-hover:ring-white/10" />

      {confirming && (
        <div className="pointer-events-none absolute inset-0 rounded-2xl ring-2 ring-red-500/50" />
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

  const [search, setSearch] = useState("");
  const [sort, setSort] = useState<SortKey>("newest");
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("all");
  const [gameFilter, setGameFilter] = useState("all");
  const [deleteTarget, setDeleteTarget] = useState<Clip | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [uploadingClip, setUploadingClip] = useState<string | null>(null);
  const [isPending, startTransition] = useTransition();

  useEffect(() => {
    loadClips();
    useCloudStore.getState().uploadQueueStatus();
  }, [loadClips]);

  const uploadMap = useMemo(
    () => new Map(uploads.map((t) => [t.clip_path, t])),
    [uploads],
  );

  const games = useMemo(
    () => Array.from(new Set(clips.map((c) => c.game).filter(Boolean))).sort(),
    [clips],
  );

  const filtered = useMemo(() => {
    const query = search.toLowerCase().trim();
    let result = clips.filter((clip) => {
      if (query) {
        const haystack = [clip.filename, clip.title, clip.description, clip.game]
          .join(" ")
          .toLowerCase();
        if (!haystack.includes(query)) return false;
      }
      if (gameFilter !== "all" && clip.game !== gameFilter) return false;
      if (statusFilter !== "all") {
        const status = uploadMap.get(clip.path)?.status;
        if (statusFilter === "uploaded" && status !== "Completed") return false;
        if (statusFilter === "uploading" && status !== "Uploading") return false;
        if (statusFilter === "failed" && !(status === "Failed" || status?.startsWith("Failed"))) return false;
      }
      return true;
    });

    result = [...result].sort((a, b) => {
      switch (sort) {
        case "oldest":
          return new Date(a.created_at).getTime() - new Date(b.created_at).getTime();
        case "name":
          return a.filename.localeCompare(b.filename);
        case "size":
          return b.size_bytes - a.size_bytes;
        case "duration":
          return b.duration_secs - a.duration_secs;
        default:
          return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
      }
    });
    return result;
  }, [clips, search, sort, statusFilter, gameFilter, uploadMap]);

  const handleSearch = useCallback((value: string) => {
    startTransition(() => setSearch(value));
  }, []);

  const handleDelete = useCallback(async (clip: Clip) => {
    setConfirming(true);
    try {
      await deleteClip(clip.filename);
      toast({ title: "Clip deleted", variant: "success" });
    } finally {
      setConfirming(false);
      setDeleteTarget(null);
    }
  }, [deleteClip]);

  const handleUpload = useCallback(async (path: string, filename: string, game: string) => {
    setUploadingClip(filename);
    try {
      await uploadClip(path, filename, game || undefined);
      toast({ title: "Upload started", description: filename });
    } finally {
      setUploadingClip(null);
    }
  }, [uploadClip]);

  const handleCopyShare = useCallback(async (url: string) => {
    await copyShareUrl(url);
    toast({ title: "Share link copied", variant: "success" });
  }, [copyShareUrl]);

  const hasActiveFilter = search.trim() !== "" || gameFilter !== "all" || statusFilter !== "all";

  const clearFilters = useCallback(() => {
    setSearch("");
    setGameFilter("all");
    setStatusFilter("all");
  }, []);

  return (
    <div className="flex h-full flex-col">
      <header className="px-6 pb-4 pt-6">
        <div className="flex items-center justify-between gap-4">
          <div className="min-w-0">
            <h1 className="flex items-center gap-2 text-xl font-semibold tracking-tight text-white">
              Clip Library
              <span className="rounded-full bg-white/5 px-2 py-0.5 text-xs font-medium text-zinc-400">
                {clips.length}
              </span>
            </h1>
            <p className="mt-1 text-sm text-zinc-500">
              {filtered.length} of {clips.length} clip{clips.length !== 1 ? "s" : ""}
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

        <div className="mt-4 flex flex-wrap items-center gap-2">
          <div className="relative min-w-0 flex-1 max-w-xs">
            <svg
              className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-zinc-500"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <circle cx="11" cy="11" r="8" />
              <path d="m21 21-4.3-4.3" />
            </svg>
            <input
              type="text"
              placeholder="Search clips..."
              value={search}
              onChange={(e) => handleSearch(e.target.value)}
              className="w-full rounded-xl border border-border bg-surface py-1.5 pl-9 pr-3 text-sm text-white placeholder-zinc-500 transition-colors outline-none focus:border-blue-400/70 focus:ring-2 focus:ring-blue-500/20"
            />
          </div>

          <Select
            value={sort}
            onChange={(e) => setSort(e.target.value as SortKey)}
            aria-label="Sort clips"
            className="w-32"
          >
            <option value="newest">Newest</option>
            <option value="oldest">Oldest</option>
            <option value="name">Name A–Z</option>
            <option value="size">Largest</option>
            <option value="duration">Longest</option>
          </Select>

          <Select
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value as StatusFilter)}
            aria-label="Filter by upload status"
            className="w-32"
          >
            <option value="all">All statuses</option>
            <option value="uploaded">Uploaded</option>
            <option value="uploading">Uploading</option>
            <option value="failed">Failed</option>
          </Select>

          {games.length > 0 && (
            <Select
              value={gameFilter}
              onChange={(e) => setGameFilter(e.target.value)}
              aria-label="Filter by game"
              className="max-w-36"
            >
              <option value="all">All games</option>
              {games.map((game) => (
                <option key={game} value={game}>{game}</option>
              ))}
            </Select>
          )}

          {hasActiveFilter && (
            <Button variant="ghost" size="sm" onClick={clearFilters}>
              Clear filters
            </Button>
          )}
        </div>
      </header>

      <div className="flex-1 overflow-y-auto px-6 pb-6">
        {loading ? (
          <SkeletonClipsGrid count={6} />
        ) : filtered.length === 0 ? (
          <div className="flex h-48 items-center justify-center">
            <EmptyState
              icon={Film}
              title={clips.length === 0 ? "No clips yet" : "No clips match your search"}
              description={
                clips.length === 0
                  ? "Clips will appear here once you save them."
                  : "Try a different search or clear your filters."
              }
              action={
                hasActiveFilter && clips.length > 0 ? (
                  <Button variant="outline" size="sm" onClick={clearFilters}>
                    Clear filters
                  </Button>
                ) : undefined
              }
            />
          </div>
        ) : (
          <div className="grid gap-4 grid-cols-[repeat(auto-fill,minmax(220px,1fr))]">
            {filtered.map((clip) => (
              <MemoClipCard
                key={clip.id}
                clip={clip}
                task={uploadMap.get(clip.path)}
                confirming={deleteTarget?.id === clip.id && confirming}
                uploadingClip={uploadingClip}
                onDelete={(filename) => {
                  const target = clips.find((c) => c.filename === filename);
                  if (target) setDeleteTarget(target);
                }}
                onUpload={handleUpload}
                onNavigate={(filename) => navigate(`/clip/${filename}`)}
                onCopyShare={handleCopyShare}
                cloudAuthed={cloudAuthed}
              />
            ))}
          </div>
        )}
        {isPending && <p className="sr-only">Filtering clips…</p>}
      </div>

      <Dialog
        open={deleteTarget !== null}
        onClose={() => setDeleteTarget(null)}
        title="Delete this clip?"
        description={
          deleteTarget
            ? `"${deleteTarget.title || deleteTarget.filename.replace(/\.mp4$/, "")}" will be permanently deleted from your library.`
            : undefined
        }
        footer={
          <>
            <Button variant="ghost" size="sm" onClick={() => setDeleteTarget(null)} disabled={confirming}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              size="sm"
              onClick={() => {
                if (deleteTarget) void handleDelete(deleteTarget);
              }}
              disabled={confirming}
            >
              {confirming ? "Deleting..." : "Delete"}
            </Button>
          </>
        }
      />
    </div>
  );
}