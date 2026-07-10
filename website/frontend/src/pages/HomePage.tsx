import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { Film, HardDrive, Upload, ArrowRight, Key } from "lucide-react";
import { useAuthStore } from "@/stores/auth";
import { api } from "@/lib/api";

interface ClipSummary {
  id: string;
  title: string;
  duration_secs: number;
  size_bytes: number;
  created_at: string;
  thumbnail_path?: string;
}

interface Stats {
  total_clips: number;
  total_storage_bytes: number;
}

function formatSize(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return (bytes / Math.pow(1024, i)).toFixed(i > 0 ? 1 : 0) + " " + units[i];
}

export default function HomePage() {
  const navigate = useNavigate();
  const user = useAuthStore((s) => s.user);
  const [recentClips, setRecentClips] = useState<ClipSummary[]>([]);
  const [stats, setStats] = useState<Stats>({ total_clips: 0, total_storage_bytes: 0 });
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function load() {
      try {
        const data = await api.get<{
          clips: ClipSummary[];
          total: number;
        }>("/api/clips?per_page=5");

        setRecentClips(data.clips || []);
        setStats({
          total_clips: data.total || 0,
          total_storage_bytes: data.clips?.reduce((s, c) => s + c.size_bytes, 0) || 0,
        });
      } catch (err) {
        console.error("Failed to load dashboard data:", err);
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  const storagePercent = user
    ? Math.min(100, (user.storage_used_bytes / user.max_storage_bytes) * 100)
    : 0;

  return (
    <div className="h-full px-6 py-6 overflow-y-auto">
      <div className="max-w-4xl">
        <div className="mb-6">
          <h1 className="text-xl font-semibold text-zinc-100">
            Welcome{user?.display_name ? `, ${user.display_name}` : ""}
          </h1>
          <p className="text-sm text-zinc-500 mt-1">Your clip sharing dashboard</p>
        </div>

        {/* Stats Cards */}
        <div className="grid grid-cols-3 gap-4 mb-8">
          <div className="rounded-xl bg-zinc-900 border border-zinc-800 p-4">
            <div className="flex items-center gap-2 text-zinc-500 mb-2">
              <Film className="size-4" />
              <span className="text-xs font-medium">Total Clips</span>
            </div>
            <p className="text-2xl font-semibold text-zinc-100">
              {loading ? "..." : stats.total_clips}
            </p>
          </div>

          <div className="rounded-xl bg-zinc-900 border border-zinc-800 p-4">
            <div className="flex items-center gap-2 text-zinc-500 mb-2">
              <HardDrive className="size-4" />
              <span className="text-xs font-medium">Storage Used</span>
            </div>
            <p className="text-2xl font-semibold text-zinc-100">
              {loading ? "..." : formatSize(user?.storage_used_bytes || 0)}
            </p>
          </div>

          <div className="rounded-xl bg-zinc-900 border border-zinc-800 p-4">
            <div className="flex items-center gap-2 text-zinc-500 mb-2">
              <Upload className="size-4" />
              <span className="text-xs font-medium">Uploads</span>
            </div>
            <p className="text-2xl font-semibold text-zinc-100">
              {loading ? "..." : recentClips.length > 0 ? "Active" : "No uploads yet"}
            </p>
          </div>
        </div>

        {/* Storage Meter */}
        <div className="rounded-xl bg-zinc-900 border border-zinc-800 p-4 mb-6">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-medium text-zinc-500">Storage</span>
            <span className="text-xs text-zinc-600">
              {formatSize(user?.storage_used_bytes || 0)} /{" "}
              {formatSize(user?.max_storage_bytes || 0)}
            </span>
          </div>
          <div className="h-2 bg-zinc-800 rounded-full overflow-hidden">
            <div
              className="h-full bg-zinc-100 rounded-full transition-all duration-500"
              style={{ width: `${storagePercent}%` }}
            />
          </div>
        </div>

        {/* Quick Actions */}
        <div className="flex gap-3 mb-8">
          <button
            onClick={() => navigate("/library")}
            className="flex items-center gap-2 px-4 py-2 bg-zinc-900 border border-zinc-800 rounded-lg text-sm text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 transition-colors"
          >
            <Film className="size-4" />
            Open Library
            <ArrowRight className="size-3.5" />
          </button>
          <button
            onClick={() => navigate("/settings")}
            className="flex items-center gap-2 px-4 py-2 bg-zinc-900 border border-zinc-800 rounded-lg text-sm text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 transition-colors"
          >
            <Key className="size-4" />
            Create API Key
            <ArrowRight className="size-3.5" />
          </button>
        </div>

        {/* Recent Clips */}
        <div>
          <h2 className="text-sm font-medium text-zinc-300 mb-3">Recent Clips</h2>
          {loading ? (
            <div className="grid grid-cols-5 gap-3">
              {[...Array(5)].map((_, i) => (
                <div
                  key={i}
                  className="aspect-video rounded-lg bg-zinc-900 border border-zinc-800 animate-pulse"
                />
              ))}
            </div>
          ) : recentClips.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-12 text-zinc-600">
              <Film className="size-10 mb-3 text-zinc-700" />
              <p className="text-sm">No clips yet</p>
              <p className="text-xs text-zinc-700 mt-1">
                Upload clips from the Prism desktop app to see them here.
            </p>
            </div>
          ) : (
            <div className="grid grid-cols-5 gap-3">
              {recentClips.map((clip) => (
                <button
                  key={clip.id}
                  onClick={() => navigate(`/clip/${clip.id}`)}
                  className="group aspect-video rounded-lg bg-zinc-900 border border-zinc-800 overflow-hidden relative"
                >
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
                      <Film className="size-6 text-zinc-700" />
                    </div>
                  )}
                  <div className="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/80 to-transparent px-2 pb-1.5 pt-4">
                    <p className="text-[10px] text-zinc-300 truncate">
                      {clip.title || "Untitled"}
                    </p>
                    <p className="text-[9px] text-zinc-500">
                      {Math.round(clip.duration_secs)}s · {formatSize(clip.size_bytes)}
                    </p>
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
