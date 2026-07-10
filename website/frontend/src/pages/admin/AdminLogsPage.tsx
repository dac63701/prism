import { useState, useEffect } from "react";
import { api } from "@/lib/api";

interface LogEntry {
  id: string;
  user_id: string | null;
  action: string;
  level: string;
  ip_address: string | null;
  details: Record<string, unknown> | null;
  created_at: string;
}

export default function AdminLogsPage() {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [page, setPage] = useState(1);
  const [filter, setFilter] = useState("");
  const perPage = 100;

  const loadLogs = async () => {
    setLoading(true);
    try {
      const params = `page=${page}&per_page=${perPage}${filter ? `&action=${filter}` : ""}`;
      const data = await api.get<{ logs: LogEntry[]; total: number }>(
        `/api/admin/logs?${params}`
      );
      setLogs(data.logs);
      setTotal(data.total);
    } catch (err) {
      console.error("Failed to load logs:", err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadLogs();
  }, [page, filter]);

  const levelColor = (level: string) => {
    switch (level) {
      case "error":
        return "text-red-400";
      case "warn":
        return "text-amber-400";
      default:
        return "text-zinc-400";
    }
  };

  return (
    <div className="h-full px-6 py-6 overflow-y-auto">
      <div className="mb-4">
        <h1 className="text-xl font-semibold text-zinc-100">Activity Log</h1>
        <p className="text-sm text-zinc-500 mt-1">{total} total entries</p>
      </div>

      <div className="flex gap-2 mb-4">
        {["", "clip_uploaded", "user_registered", "admin_user_banned"].map(
          (a) => (
            <button
              key={a}
              onClick={() => {
                setFilter(a);
                setPage(1);
              }}
              className={`px-3 py-1 text-xs rounded-lg transition-colors ${
                filter === a
                  ? "bg-zinc-100 text-zinc-950"
                  : "bg-zinc-900 text-zinc-500 hover:text-zinc-300"
              }`}
            >
              {a || "All"}
            </button>
          )
        )}
      </div>

      {loading ? (
        <p className="text-sm text-zinc-600">Loading...</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-xs text-zinc-500 border-b border-zinc-800">
                <th className="text-left font-medium px-3 py-2">Timestamp</th>
                <th className="text-left font-medium px-3 py-2">Action</th>
                <th className="text-center font-medium px-3 py-2">Level</th>
                <th className="text-left font-medium px-3 py-2">IP</th>
              </tr>
            </thead>
            <tbody>
              {logs.map((log) => (
                <tr
                  key={log.id}
                  className="border-b border-zinc-800/50 hover:bg-zinc-900/50"
                >
                  <td className="px-3 py-2 text-zinc-500 text-xs">
                    {new Date(log.created_at).toLocaleString()}
                  </td>
                  <td className="px-3 py-2 text-zinc-300">{log.action}</td>
                  <td className={`px-3 py-2 text-center text-xs ${levelColor(log.level)}`}>
                    {log.level}
                  </td>
                  <td className="px-3 py-2 text-zinc-600 text-xs font-mono">
                    {log.ip_address || "—"}
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
