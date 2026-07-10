import { useState, useEffect } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { ArrowLeft, Shield, Ban, Trash2 } from "lucide-react";
import { api } from "@/lib/api";

interface AdminUser {
  id: string;
  email: string;
  display_name: string;
  role: string;
  storage_used_bytes: number;
  max_storage_bytes: number;
  is_banned: boolean;
  clip_count: number;
  created_at: string;
}

function formatSize(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return (bytes / Math.pow(1024, i)).toFixed(i > 0 ? 1 : 0) + " " + units[i];
}

export default function AdminUserDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [user, setUser] = useState<AdminUser | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!id) return;
    api
      .get<AdminUser>(`/api/admin/users/${id}`)
      .then(setUser)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [id]);

  const updateField = async (field: string, value: unknown) => {
    setSaving(true);
    try {
      await api.patch(`/api/admin/users/${id}`, { [field]: value });
      setUser((prev) => (prev ? { ...prev, [field]: value } : prev));
    } catch (err: any) {
      alert(err.message);
    } finally {
      setSaving(false);
    }
  };

  const deleteUser = async () => {
    if (!confirm("Delete this user and all their clips? This cannot be undone.")) return;
    try {
      await api.delete(`/api/admin/users/${id}`);
      navigate("/admin/users");
    } catch (err: any) {
      alert(err.message);
    }
  };

  if (loading || !user) {
    return (
      <div className="h-full flex items-center justify-center text-zinc-500 text-sm">
        {loading ? "Loading..." : "User not found"}
      </div>
    );
  }

  const storagePercent = Math.min(
    100,
    (user.storage_used_bytes / user.max_storage_bytes) * 100
  );

  return (
    <div className="h-full px-6 py-6 overflow-y-auto">
      <button
        onClick={() => navigate("/admin/users")}
        className="flex items-center gap-1.5 text-sm text-zinc-500 hover:text-zinc-200 mb-4 transition-colors"
      >
        <ArrowLeft className="size-4" />
        Back to users
      </button>

      <div className="max-w-2xl space-y-4">
        <div className="rounded-xl bg-zinc-900 border border-zinc-800 p-4">
          <h2 className="text-sm font-medium text-zinc-100 mb-1">{user.email}</h2>
          <p className="text-xs text-zinc-500">
            {user.display_name || "No display name"} · Joined{" "}
            {new Date(user.created_at).toLocaleDateString()}
          </p>

          <div className="mt-4 space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-xs text-zinc-500">Role</span>
              <select
                value={user.role}
                onChange={(e) => updateField("role", e.target.value)}
                disabled={saving}
                className="bg-zinc-950 border border-zinc-800 rounded-lg px-2.5 py-1 text-xs text-zinc-100 focus:outline-none focus:ring-1 focus:ring-zinc-600"
              >
                <option value="user">User</option>
                <option value="admin">Admin</option>
              </select>
            </div>

            <div className="flex items-center justify-between">
              <span className="text-xs text-zinc-500">Status</span>
              <button
                onClick={() => updateField("is_banned", !user.is_banned)}
                disabled={saving}
                className={`flex items-center gap-1.5 px-3 py-1 text-xs rounded-lg transition-colors ${
                  user.is_banned
                    ? "bg-red-950/60 text-red-300 hover:bg-red-900/80"
                    : "bg-zinc-800 text-zinc-400 hover:bg-zinc-700"
                }`}
              >
                <Ban className="size-3" />
                {user.is_banned ? "Unban" : "Ban"}
              </button>
            </div>

            <div className="flex items-center justify-between">
              <span className="text-xs text-zinc-500">Clips</span>
              <span className="text-sm text-zinc-100">{user.clip_count}</span>
            </div>

            <div>
              <div className="flex items-center justify-between mb-1">
                <span className="text-xs text-zinc-500">Storage</span>
                <span className="text-xs text-zinc-600">
                  {formatSize(user.storage_used_bytes)} /{" "}
                  {formatSize(user.max_storage_bytes)}
                </span>
              </div>
              <div className="h-1.5 bg-zinc-800 rounded-full overflow-hidden">
                <div
                  className="h-full bg-zinc-100 rounded-full"
                  style={{ width: `${storagePercent}%` }}
                />
              </div>
            </div>

            <div className="pt-2 border-t border-zinc-800/50">
              <button
                onClick={deleteUser}
                className="flex items-center gap-1.5 text-xs text-red-400 hover:text-red-300 transition-colors"
              >
                <Trash2 className="size-3.5" />
                Delete User
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
