import { useState, useEffect, useCallback, useMemo } from "react";
import { useNavigate } from "react-router-dom";
import { Search, Filter, Film, Trash2, FolderOpen, Play, Upload, Check, Link2, Loader2 } from "lucide-react";
import { useClipsStore, formatSize, formatDuration, formatDate } from "@/stores/clips";
import { useCloudStore } from "@/stores/cloud";
import ClipThumbnail from "@/components/common/ClipThumbnail";

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
  const [showConfirm, setShowConfirm] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [uploadingClip, setUploadingClip] = useState<string | null>(null);

  useEffect(() => {
    loadClips();
    useCloudStore.getState().uploadQueueStatus();
  }, [loadClips]);

  const filtered = useMemo(
    () => clips.filter((c) => c.filename.toLowerCase().includes(search.toLowerCase())),
    [clips, search],
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

  const handleUpload = useCallback(async (path: string, filename: string) => {
    setUploadingClip(filename);
    try {
      await uploadClip(path, filename);
    } finally {
      setUploadingClip(null);
    }
  }, [uploadClip]);

  const taskForClip = useCallback((path: string) => {
    return uploads.find((t: { clip_path: string }) => t.clip_path === path);
  }, [uploads]);

  return (
    <div className="h-full flex flex-col">
      <header className="px-6 pt-6 pb-4">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-xl font-semibold text-zinc-100">Clip Library</h1>
            <p className="text-sm text-zinc-500 mt-1">
              {clips.length} clip{clips.length !== 1 ? "s" : ""}
            </p>
          </div>
          <button
            onClick={openClipLocation}
            className="flex items-center gap-1.5 px-3 py-1.5 text-sm text-zinc-400 bg-zinc-900 border border-zinc-800 rounded-lg hover:text-zinc-200 hover:bg-zinc-800 transition-colors"
          >
            <FolderOpen className="size-4" />
            Open Folder
          </button>
        </div>

        <div className="mt-4 flex items-center gap-3">
          <div className="relative flex-1 max-w-xs">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-zinc-500" />
            <input
              type="text"
              placeholder="Search clips..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="w-full pl-9 pr-3 py-1.5 text-sm bg-zinc-900 border border-zinc-800 rounded-lg text-zinc-100 placeholder-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-600"
            />
          </div>
          <button className="flex items-center gap-1.5 px-3 py-1.5 text-sm text-zinc-400 bg-zinc-900 border border-zinc-800 rounded-lg hover:text-zinc-200 hover:bg-zinc-800 transition-colors">
            <Filter className="size-4" />
            Filter
          </button>
        </div>
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
            {filtered.map((clip) => {
              const task = taskForClip(clip.path);
              const isUploaded = task?.status === "Completed";
              const isUploading = task?.status === "Uploading" || uploadingClip === clip.filename;
              const isFailed = task?.status?.startsWith("Failed");
              const shareUrl = task?.share_url;

              return (
                <div
                  key={clip.id}
                  onClick={() => navigate(`/clip/${clip.filename}`, { state: { clip } })}
                  className="group aspect-video bg-zinc-900 rounded-lg border border-zinc-800 overflow-hidden relative cursor-pointer"
                >
                  <ClipThumbnail path={clip.path} filename={clip.filename} />

                  {/* Upload status badge */}
                  {isUploaded ? (
                    <div className="absolute top-2 left-2 flex items-center gap-1 px-2 py-0.5 rounded-full bg-emerald-600/80 text-[10px] text-white font-medium">
                      <Check className="size-3" />
                      Uploaded
                    </div>
                  ) : isFailed ? (
                    <div className="absolute top-2 left-2 flex items-center gap-1 px-2 py-0.5 rounded-full bg-red-600/80 text-[10px] text-white font-medium">
                      Failed
                    </div>
                  ) : isUploading ? (
                    <div className="absolute top-2 left-2 flex items-center gap-1 px-2 py-0.5 rounded-full bg-blue-600/80 text-[10px] text-white font-medium">
                      <Loader2 className="size-3 animate-spin" />
                      Uploading
                    </div>
                  ) : null}

                  {/* Progress bar for uploading */}
                  {isUploading && task && (
                    <div className="absolute bottom-9 left-2 right-2 h-1 rounded-full bg-zinc-700 overflow-hidden">
                      <div
                        className="h-full bg-blue-500 transition-all duration-300"
                        style={{ width: `${task.progress * 100}%` }}
                      />
                    </div>
                  )}

                  {/* Metadata overlay at bottom */}
                  <div className="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/80 to-transparent px-3 pb-2 pt-6">
                    <div className="flex items-center justify-between text-[11px] text-zinc-400">
                      <span>{formatDuration(clip.duration_secs)}</span>
                      <span>{formatSize(clip.size_bytes)}</span>
                    </div>
                    <p className="text-[11px] text-zinc-500 mt-0.5">
                      {formatDate(clip.created_at)}
                    </p>
                  </div>

                  {/* Hover actions */}
                  <div className="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center gap-3">
                    <button
                      onClick={(e) => { e.stopPropagation(); navigate(`/clip/${clip.filename}`, { state: { clip } }); }}
                      className="p-3 rounded-full bg-white/15 hover:bg-white/25 text-white transition-colors"
                      title="Play in app"
                    >
                      <Play className="size-5 fill-white ml-0.5" />
                    </button>

                    {isUploaded && shareUrl ? (
                      <button
                        onClick={(e) => { e.stopPropagation(); copyShareUrl(shareUrl); }}
                        className="p-3 rounded-full bg-emerald-600/40 hover:bg-emerald-600/60 text-white transition-colors"
                        title="Copy share link"
                      >
                        <Link2 className="size-5" />
                      </button>
                    ) : cloudAuthed ? (
                      <button
                        onClick={(e) => { e.stopPropagation(); handleUpload(clip.path, clip.filename); }}
                        disabled={isUploading}
                        className="p-3 rounded-full bg-blue-600/40 hover:bg-blue-600/60 text-white transition-colors disabled:opacity-40"
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
                      onClick={(e) => { e.stopPropagation(); setShowConfirm(clip.filename); }}
                      className="p-2 rounded-lg bg-zinc-800/80 hover:bg-red-900/60 text-zinc-300 hover:text-red-300 transition-colors absolute top-2 right-2"
                      title="Delete clip"
                    >
                      <Trash2 className="size-4" />
                    </button>
                  </div>

                  {/* Delete confirmation overlay */}
                  {showConfirm === clip.filename && (
                    <div
                      className="absolute inset-0 bg-zinc-950/90 flex flex-col items-center justify-center p-4 gap-3"
                      onClick={(e) => e.stopPropagation()}
                    >
                      <p className="text-sm text-zinc-300 text-center">Delete this clip?</p>
                      <div className="flex gap-2">
                        <button
                          onClick={() => handleDelete(clip.filename)}
                          disabled={confirming}
                          className="px-3 py-1.5 text-xs font-medium bg-red-600 hover:bg-red-500 text-white rounded-md transition-colors disabled:opacity-50"
                        >
                          {confirming ? "Deleting..." : "Delete"}
                        </button>
                        <button
                          onClick={() => setShowConfirm(null)}
                          className="px-3 py-1.5 text-xs font-medium bg-zinc-800 hover:bg-zinc-700 text-zinc-300 rounded-md transition-colors"
                        >
                          Cancel
                        </button>
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
