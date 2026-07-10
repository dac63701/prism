import { useState, useEffect, useMemo } from "react";
import { useNavigate } from "react-router-dom";
import { Search, Film, Trash2 } from "lucide-react";
import { api } from "@/lib/api";

interface Clip {
  id: string;
  title: string;
  game: string;
  duration_secs: number;
  size_bytes: number;
  width: number;
  height: number;
  visibility: string;
  thumbnail_path?: string;
  created_at: string;
}

function formatSize(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return (bytes / Math.pow(1024, i)).toFixed(i > 0 ? 1 : 0) + " " + units[i];
}

function formatDuration(secs: number): string {
  const m = Math.floor(secs / 60);
  const s = Math.floor(secs % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

function formatDate(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return iso;
  }
}

export default function LibraryPage() {
  const navigate = useNavigate();
  const [clips, setClips] = useState<Clip[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(1);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const perPage = 24;

  const loadClips = async () => {
    setLoading(true);
    try {
      const data = await api.get<{
        clips: Clip[];
        total: number;
        page: number;
        total_pages: number;
      }>(`/api/clips?page=${page}&per_page=${perPage}&search=${encodeURIComponent(search)}`);
      setClips(data.clips);
      setTotal(data.total);
    } catch (err) {
      console.error("Failed to load clips:", err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadClips();
  }, [page]);

  useEffect(() => {
    setPage(1);
    loadClips();
  }, [search]);

  const totalPages = Math.max(1, Math.ceil(total / perPage));

  const toggleSelect = (id: string) => {
    const next = new Set(selected);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    setSelected(next);
  };

  const handleBulkDelete = async () => {
    if (!confirm(`Delete ${selected.size} clip(s)?`)) return;
    for (const id of selected) {
      try {
        await api.delete(`/api/clips/${id}`);
      } catch (err) {
        console.error("Failed to delete clip:", err);
      }
    }
    setSelected(new Set());
    loadClips();
  };

  return (
    <div className="h-full flex flex-col">
      <header className="px-6 pt-6 pb-4">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-xl font-semibold text-zinc-100">Clip Library</h1>
            <p className="text-sm text-zinc-500 mt-1">
              {total} clip{total !== 1 ? "s" : ""}
            </p>
          </div>
          {selected.size > 0 && (
            <button
              onClick={handleBulkDelete}
              className="flex items-center gap-1.5 px-3 py-1.5 text-sm text-red-300 bg-red-950/60 border border-red-900/60 rounded-lg hover:bg-red-900/80 transition-colors"
            >
              <Trash2 className="size-4" />
              Delete {selected.size}
            </button>
          )}
        </div>

        <div className="mt-4 relative max-w-xs">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-zinc-500" />
          <input
            type="text"
            placeholder="Search clips..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="w-full pl-9 pr-3 py-1.5 text-sm bg-zinc-900 border border-zinc-800 rounded-lg text-zinc-100 placeholder-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-600"
          />
        </div>
      </header>

      <div className="flex-1 px-6 pb-6 overflow-y-auto">
        {loading ? (
          <div className="grid grid-cols-4 gap-4">
            {[...Array(8)].map((_, i) => (
              <div
                key={i}
                className="aspect-video rounded-lg bg-zinc-900 border border-zinc-800 animate-pulse"
              />
            ))}
          </div>
        ) : clips.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-48 text-zinc-600">
            <Film className="size-10 text-zinc-700 mb-3" />
            <p className="text-sm">{total === 0 ? "No clips yet" : "No clips match your search"}</p>
            <p className="text-xs text-zinc-700 mt-1">
              {total === 0
                ? "Upload clips from the Prism desktop app."
                : "Try a different search term."}
            </p>
          </div>
        ) : (
          <>
            <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
              {clips.map((clip) => (
                <div
                  key={clip.id}
                  className="group aspect-video bg-zinc-900 rounded-lg border border-zinc-800 overflow-hidden relative cursor-pointer"
                  onClick={() => navigate(`/clip/${clip.id}`)}
                >
                  <input
                    type="checkbox"
                    checked={selected.has(clip.id)}
                    onChange={(e) => {
                      e.stopPropagation();
                      toggleSelect(clip.id);
                    }}
                    className="absolute top-2 left-2 z-10 size-4 accent-zinc-100 opacity-0 group-hover:opacity-100 transition-opacity"
                  />

                  {clip.thumbnail_path ? (
                    <img
                      src={`/api/media/${clip.thumbnail_path}`}
                      alt=""
                      className="w-full h-full object-cover"
                      onError={(e) => {
                        (e.target as HTMLImageElement).style.display = "none";
                      }}
                    />
                  ) : (
                    <div className="w-full h-full flex items-center justify-center">
                      <Film className="size-8 text-zinc-700" />
                    </div>
                  )}

                  <div className="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/80 to-transparent px-3 pb-2 pt-6">
                    <div className="flex items-center justify-between text-[11px] text-zinc-400">
                      <span>{formatDuration(clip.duration_secs)}</span>
                      <span>{formatSize(clip.size_bytes)}</span>
                    </div>
                    <p className="text-[11px] text-zinc-500 mt-0.5 truncate">
                      {clip.title || "Untitled"}
                    </p>
                    <p className="text-[10px] text-zinc-600">
                      {formatDate(clip.created_at)}
                    </p>
                  </div>
                </div>
              ))}
            </div>

            {totalPages > 1 && (
              <div className="flex items-center justify-center gap-2 mt-6">
                {Array.from({ length: totalPages }, (_, i) => i + 1).map((p) => (
                  <button
                    key={p}
                    onClick={() => setPage(p)}
                    className={`px-3 py-1 text-xs rounded-md transition-colors ${
                      p === page
                        ? "bg-zinc-100 text-zinc-950"
                        : "bg-zinc-900 text-zinc-500 hover:text-zinc-300"
                    }`}
                  >
                    {p}
                  </button>
                ))}
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
