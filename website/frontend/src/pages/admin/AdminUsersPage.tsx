import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { Search, Shield, ShieldOff } from "lucide-react";
import { api } from "@/lib/api";

interface UserItem {
  id: string;
  email: string;
  display_name: string;
  role: string;
  clip_count: number;
  storage_used_bytes: number;
  created_at: string;
  is_banned: boolean;
}

function formatSize(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return (bytes / Math.pow(1024, i)).toFixed(i > 0 ? 1 : 0) + " " + units[i];
}

export default function AdminUsersPage() {
  const navigate = useNavigate();
  const [users, setUsers] = useState<UserItem[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(1);
  const perPage = 50;

  const loadUsers = async () => {
    setLoading(true);
    try {
      const data = await api.get<{
        users: UserItem[];
        total: number;
      }>(
        `/api/admin/users?page=${page}&per_page=${perPage}&search=${encodeURIComponent(search)}`
      );
      setUsers(data.users);
      setTotal(data.total);
    } catch (err) {
      console.error("Failed to load users:", err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadUsers();
  }, [page, search]);

  return (
    <div className="h-full px-6 py-6 overflow-y-auto">
      <div className="mb-4">
        <h1 className="text-xl font-semibold text-zinc-100">Users</h1>
        <p className="text-sm text-zinc-500 mt-1">{total} total users</p>
      </div>

      <div className="relative max-w-xs mb-4">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-zinc-500" />
        <input
          type="text"
          placeholder="Search users..."
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
                <th className="text-left font-medium px-3 py-2">Email</th>
                <th className="text-left font-medium px-3 py-2">Name</th>
                <th className="text-left font-medium px-3 py-2">Role</th>
                <th className="text-right font-medium px-3 py-2">Clips</th>
                <th className="text-right font-medium px-3 py-2">Storage</th>
                <th className="text-center font-medium px-3 py-2">Status</th>
              </tr>
            </thead>
            <tbody>
              {users.map((user) => (
                <tr
                  key={user.id}
                  onClick={() => navigate(`/admin/users/${user.id}`)}
                  className="border-b border-zinc-800/50 hover:bg-zinc-900/50 cursor-pointer transition-colors"
                >
                  <td className="px-3 py-2.5 text-zinc-200">{user.email}</td>
                  <td className="px-3 py-2.5 text-zinc-400">
                    {user.display_name || "—"}
                  </td>
                  <td className="px-3 py-2.5">
                    <span
                      className={`inline-flex items-center gap-1 text-xs ${
                        user.role === "admin" ? "text-amber-400" : "text-zinc-500"
                      }`}
                    >
                      {user.role === "admin" ? (
                        <Shield className="size-3" />
                      ) : (
                        <ShieldOff className="size-3" />
                      )}
                      {user.role}
                    </span>
                  </td>
                  <td className="px-3 py-2.5 text-right text-zinc-400">
                    {user.clip_count}
                  </td>
                  <td className="px-3 py-2.5 text-right text-zinc-400">
                    {formatSize(user.storage_used_bytes)}
                  </td>
                  <td className="px-3 py-2.5 text-center">
                    {user.is_banned ? (
                      <span className="text-xs text-red-400">Banned</span>
                    ) : (
                      <span className="text-xs text-green-400">Active</span>
                    )}
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
