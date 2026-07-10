import { useState, useEffect } from "react";
import { Users, Film, HardDrive, Upload, Activity } from "lucide-react";
import { api } from "@/lib/api";

interface Stats {
  total_users: number;
  total_clips: number;
  total_storage_bytes: number;
  total_storage_gb: number;
  uploads_today: number;
  uploads_this_week: number;
}

function formatSize(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return (bytes / Math.pow(1024, i)).toFixed(i > 0 ? 1 : 0) + " " + units[i];
}

export default function AdminDashboard() {
  const [stats, setStats] = useState<Stats | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api
      .get<Stats>("/api/admin/stats")
      .then(setStats)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  const cards = stats
    ? [
        { label: "Total Users", value: stats.total_users, icon: Users },
        { label: "Total Clips", value: stats.total_clips, icon: Film },
        {
          label: "Storage Used",
          value: formatSize(stats.total_storage_bytes),
          icon: HardDrive,
        },
        { label: "Uploads Today", value: stats.uploads_today, icon: Upload },
        {
          label: "Uploads This Week",
          value: stats.uploads_this_week,
          icon: Activity,
        },
      ]
    : [];

  return (
    <div className="h-full px-6 py-6 overflow-y-auto">
      <div className="mb-6">
        <h1 className="text-xl font-semibold text-zinc-100">Admin Dashboard</h1>
        <p className="text-sm text-zinc-500 mt-1">Server overview and statistics</p>
      </div>

      {loading ? (
        <div className="grid grid-cols-3 gap-4">
          {[...Array(5)].map((_, i) => (
            <div
              key={i}
              className="h-24 rounded-xl bg-zinc-900 border border-zinc-800 animate-pulse"
            />
          ))}
        </div>
      ) : (
        <div className="grid grid-cols-3 gap-4">
          {cards.map((card) => (
            <div
              key={card.label}
              className="rounded-xl bg-zinc-900 border border-zinc-800 p-4"
            >
              <div className="flex items-center gap-2 text-zinc-500 mb-2">
                <card.icon className="size-4" />
                <span className="text-xs font-medium">{card.label}</span>
              </div>
              <p className="text-2xl font-semibold text-zinc-100">{card.value}</p>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
