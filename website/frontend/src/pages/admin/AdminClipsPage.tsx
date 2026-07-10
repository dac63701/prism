import { useState, useEffect } from "react";
import { Search, Film, Trash2 } from "lucide-react";
import { api } from "@/lib/api";

interface AdminClip {
  id: string;
  title: string;
  game: string;
  duration_secs: number;
  size_bytes: number;
  visibility: string;
  created_at: string;
  user_email?: string;
  user_display_name?: string;
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

export default function AdminClipsPage() {
  const [clips, setClips] = useState<AdminClip[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(1);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const perPage = 50;

  const loadClips = async () => {
    setLoading(true);
    try {
      const data = await api.get<{
        clips: AdminClip[];
        total: number;
      }>(
        `/api/admin/clips?page=${page}&per_page=${perPage}&search=${encodeURIComponent(search)}`
      );
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
  }, [page, search]);

  const handleBulkDelete = async () => {
    if (!confirm(`Delete ${selected.size} clip(s)?`)) return;
    for (const id of selected) {
      try {
        await api.delete(`/api/admin/clips/${id}`);
      } catch (err) {
        console.error("Failed to delete clip:", err);
      }
    }
    setSelected(new Set());
    loadClips();
  };

  return (
    <div className="h-full px-6 py-6 overflow-y-auto">
      <div className="flex items-center justify-between mb-4">
        <div>
          <h1 className="text-xl font-semibold text-zinc-100">All Clips</h1>
          <p className="text-sm text-zinc-500 mt-1">{total} total clips</p>
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

      <div className="relative max-w-xs mb-4">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-zinc-500" />
        <input
          type="text"
          placeholder="Search clips..."
          value={search}
          onChange={(e) => {
            setSearch(e.target.value);
            setPage(1);
          }}
          className="w-full pl-9 pr-3 py-1.5 text-sm bg-zinc-900 border border-zinc-800 rounded-lg text-zinc-100 placeholder-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-600"
        />
      </div>

      {loading ? (
        <p className="text-sm text-zinc-600">Loading...</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-xs text-zinc-500 border-b border-zinc-800">
                <th className="w-8 px-2 py-2">
                  <input
                    type="checkbox"
                    onChange={(e) => {
                      if (e.target.checked) {
                        setSelected(new Set(clips.map((c) => c.id)));
                      } else {
                        setSelected(new Set());
                      }
                    }}
                    className="size-4 accent-zinc-100"
                  />
                </th>
                <th className="text-left font-medium px-3 py-2">Title</th>
                <th className="text-left font-medium px-3 py-2">User</th>
                <th className="text-left font-medium px-3 py-2">Game</th>
                <th className="text-right font-medium px-3 py-2">Duration</th>
                <th className="text-right font-medium px-3 py-2">Size</th>
                <th className="text-center font-medium px-3 py-2">Visibility</th>
                <th className="text-right font-medium px-3 py-2">Created</th>
              </tr>
            </thead>
            <tbody>
              {clips.map((clip) => (
                <tr
                  key={clip.id}
                  className="border-b border-zinc-800/50 hover:bg-zinc-900/50 transition-colors"
                >
                  <td className="px-2 py-2.5">
                    <input
                      type="checkbox"
                      checked={selected.has(clip.id)}
                      onChange={() => {
                        const next = new Set(selected);
                        if (next.has(clip.id)) next.delete(clip.id);
                        else next.add(clip.id);
                        setSelected(next);
                      }}
                      className="size-4 accent-zinc-100"
                    />
                  </td>
                  <td className="px-3 py-2.5 text-zinc-200">{clip.title || "Untitled"}</td>
                  <td className="px-3 py-2.5 text-zinc-400">
                    {clip.user_display_name || clip.user_email || "Unknown"}
                  </td>
                  <td className="px-3 py-2.5 text-zinc-400">{clip.game || "—"}</td>
                  <td className="px-3 py-2.5 text-right text-zinc-400">
                    {formatDuration(clip.duration_secs)}
                  </td>
                  <td className="px-3 py-2.5 text-right text-zinc-400">
                    {formatSize(clip.size_bytes)}
                  </td>
                  <td className="px-3 py-2.5 text-center">
                    <span
                      className={`text-xs ${
                        clip.visibility === "public"
                          ? "text-green-400"
                          : clip.visibility === "unlisted"
                          ? "text-zinc-400"
                          : "text-zinc-600"
                      }`}
                    >
                      {clip.visibility}
                    </span>
                  </td>
                  <td className="px-3 py-2.5 text-right text-zinc-500 text-xs">
                    {new Date(clip.created_at).toLocaleDateString()}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
